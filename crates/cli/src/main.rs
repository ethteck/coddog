#[cfg(feature = "db")]
mod db;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};
use coddog_core::cluster::get_clusters;
use coddog_core::{
    self as core, Binary, Platform, Symbol, get_submatches,
    ingest::{read_elf, read_map},
};

use colored::*;
use decomp_settings::{config::Version, read_config, scan_for_config};
use dotenvy::dotenv;
use glob::glob;
use inquire::Select;
use std::collections::HashMap;
use std::{
    fs,
    path::{Path, PathBuf},
};

const BINARY_COLORS: [Color; 6] = [
    Color::BrightGreen,
    Color::BrightYellow,
    Color::BrightBlue,
    Color::BrightMagenta,
    Color::BrightCyan,
    Color::BrightRed,
];

/// Find cod
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]

enum Commands {
    /// Find functions similar to the query function
    /// Uses project in the current directory
    Match {
        /// Name of the query function
        query: String,

        /// Similarity threshold
        #[arg(short, long, default_value = "0.985")]
        threshold: f32,
    },

    /// Cluster functions by similarity, showing possible duplicates
    /// Uses project in the current directory
    Cluster {
        /// Similarity threshold
        #[arg(short, long, default_value = "0.985")]
        threshold: f32,

        /// Minimum length of functions (in number of instructions) to consider
        #[arg(short, long, default_value = "5")]
        min_len: usize,
    },

    /// Find chunks of code similar to those in the query function
    /// Uses project in the current directory
    Submatch {
        /// Name of the query function
        query: String,

        /// Window size (smaller values will find more matches but take longer)
        window_size: usize,
    },

    /// Compare two binaries, showing the functions in common between them
    Compare2 {
        /// Path to the first decomp.yaml
        yaml1: PathBuf,

        /// Version to compare from the first yaml
        version1: String,

        /// Path to the second decomp.yaml
        yaml2: PathBuf,

        /// Version to compare from the second yaml
        version2: String,

        /// Similarity threshold
        #[arg(short, long, default_value = "0.985")]
        threshold: f32,

        /// Minimum length of functions (in number of instructions) to consider
        #[arg(short, long, default_value = "5")]
        min_len: usize,
    },

    /// Compare a binary in one project to one or more others, showing the functions in common between them
    CompareN {
        /// Path to the main decomp.yaml
        main_yaml: PathBuf,

        /// Version to compare from the main yaml
        main_version: String,

        /// Path to other projects' decomp.yaml files
        other_yamls: Vec<PathBuf>,
    },

    /// Compare one raw binary to one or more projects' binaries, showing the functions in common between them
    CompareRaw {
        /// Path to the main binary
        query_bin: PathBuf,

        /// Path to other projects' decomp.yaml files
        yamls: Vec<PathBuf>,
    },
    /// Database management commands
    #[cfg(feature = "db")]
    #[command(subcommand)]
    Db(DbCommands),
}

#[cfg(feature = "db")]
#[derive(Subcommand)]
enum DbCommands {
    /// Add a new project to the database, given a path to a repo
    AddProject {
        /// Path to the project's repo
        repo: PathBuf,
    },
    /// Delete a project from the database, removing its sources, symbols, and hashes
    DeleteProject {
        /// Name of the project to delete
        name: String,
    },
    /// Remove orphaned binary files on disk that no longer appear in the database
    CleanBins {},
    /// Search the database for matches of a given symbol
    Match {
        /// Name of the query function
        query: String,
        /// Specificity of match
        match_type: MatchType,
    },
    /// Search the database for submatches of a given symbol
    Submatch {
        /// Name of the query function
        query: String,
        /// Window size (smaller values will find more matches but take longer)
        window_size: usize,
    },
    /// Import data from a locally-loaded decomp.me database
    ImportDecompme {},
}

#[derive(ValueEnum, Clone, PartialEq)]
enum MatchType {
    /// Only opcodes are compared
    Opcode,
    /// Opcodes and some operands are compared
    Equivalent,
    /// Exact bytes are compared
    Exact,
}

fn cli_fullname(sym: &Symbol) -> String {
    format!(
        "{}{}",
        sym.name.clone(),
        if sym.is_decompiled {
            " (decompiled)".green()
        } else {
            "".normal()
        }
    )
}

fn cli_name_colored(sym: &Symbol, color: Color) -> String {
    format!("{}", sym.name.clone().color(color))
}

fn do_match(query: &str, symbols: &[Symbol], threshold: f32) {
    struct FunctionMatch<'a> {
        symbol: &'a Symbol,
        score: f32,
    }

    let Some(query_sym) = symbols.iter().find(|s| s.name == query) else {
        println!("Symbol {query:} not found");
        return;
    };

    let mut matches: Vec<FunctionMatch> = symbols
        .iter()
        .filter(|s| s.name != query_sym.name)
        .map(|s| FunctionMatch {
            symbol: s,
            score: core::diff_symbols(query_sym, s, threshold),
        })
        .filter(|m| m.score > threshold)
        .collect();

    // sort by score descending
    matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    for m in matches {
        println!("{:.2}% - {}", m.score * 100.0, cli_fullname(m.symbol));
    }
}

fn do_submatch(query: &str, symbols: &[Symbol], window_size: usize) {
    let Some(query_sym) = symbols.iter().find(|s| s.name == query) else {
        println!("Symbol {query:} not found");
        return;
    };

    let query_hashes = query_sym.get_opcode_hashes(window_size);

    for s in symbols {
        if s == query_sym {
            continue;
        }

        if query_sym.opcodes == s.opcodes {
            let match_pct = if query_sym.bytes == s.bytes {
                "100%"
            } else {
                "99%"
            };
            println!("{} matches {}", cli_fullname(s), match_pct);
            continue;
        }

        let hashes = s.get_opcode_hashes(window_size);

        let pair_matches = get_submatches(&query_hashes, &hashes, window_size);

        if pair_matches.is_empty() {
            continue;
        }

        println!("{}:", cli_fullname(s));

        for m in pair_matches {
            let query_str = format!("query [{}-{}]", m.offset1, m.offset1 + m.length);
            let target_str = format!(
                "{} [insn {}-{}] ({} total)",
                s.name,
                m.offset2,
                m.offset2 + m.length,
                m.length
            );
            println!("\t{query_str} matches {target_str}");
        }
    }
}

pub fn do_cluster(symbols: &[Symbol], threshold: f32, min_len: usize) {
    let clusters = get_clusters(symbols, threshold, min_len);

    // Print clusters
    for cluster in clusters.iter().filter(|c| c.size() > 1) {
        println!(
            "Cluster {} has {} symbols",
            cluster.syms[0].name,
            cluster.size()
        );
    }
}

fn get_full_path(base_dir: &Path, config_path: Option<PathBuf>) -> Option<PathBuf> {
    config_path.map(|path| {
        if path.is_relative() {
            base_dir.join(path)
        } else {
            path.clone()
        }
    })
}

fn get_unmatched_funcs(base_dir: &Path, config: &Version) -> Option<Vec<String>> {
    get_full_path(base_dir, config.paths.asm.clone()).map(|asm_dir| {
        let mut unmatched_funcs = Vec::new();

        for s_file in glob(asm_dir.join("**/*.s").to_str().unwrap()).unwrap() {
            // add filename minus extension to vec
            let s_file = s_file.unwrap();
            let s_file_stem = s_file.file_stem().unwrap().to_str().unwrap();
            unmatched_funcs.push(s_file_stem.to_string());
        }
        unmatched_funcs
    })
}

fn collect_symbols(config: &Version, base_dir: &Path, platform: &str) -> Result<Vec<Symbol>> {
    let unmatched_funcs = get_unmatched_funcs(base_dir, config);
    let platform =
        Platform::from_name(platform).unwrap_or_else(|| panic!("Invalid platform: {platform}"));

    if let Some(elf_path) = get_full_path(base_dir, config.paths.elf.clone()) {
        let elf_data = fs::read(elf_path)?;
        return read_elf(platform, &unmatched_funcs, &elf_data);
    }

    if let (Some(target), Some(map_path)) = (
        get_full_path(base_dir, Some(config.paths.target.clone())),
        get_full_path(base_dir, Some(config.paths.map.clone())),
    ) {
        let target_bytes = fs::read(target)?;
        let map_str = fs::read_to_string(map_path)?;
        return read_map(platform, unmatched_funcs, target_bytes, &map_str);
    }

    Err(anyhow!("No elf or mapfile found"))
}

fn do_compare_binaries(bin1: &Binary, bin2: &Binary, threshold: f32, min_len: usize) {
    let mut matched_syms: Vec<(&Symbol, &Symbol, f32)> = Vec::new();

    bin1.symbols
        .iter()
        .filter(|s| s.opcodes.len() >= min_len)
        .for_each(|sym| {
            let mut best_match: Option<(&Symbol, f32)> = None;

            for sym2 in bin2.symbols.iter().filter(|s| s.opcodes.len() >= min_len) {
                let score = core::diff_symbols(sym, sym2, threshold);
                if score > threshold {
                    if let Some((_, best_score)) = best_match {
                        if score > best_score {
                            best_match = Some((sym2, score));
                        }
                    } else {
                        best_match = Some((sym2, score));
                    }
                }
            }

            if let Some((best_sym, score)) = best_match {
                matched_syms.push((sym, best_sym, score));
            }
        });

    match matched_syms.len() {
        0 => {
            println!("No matches found");
        }
        _ => {
            let mut both_decompiled: Vec<(&Symbol, &Symbol, f32)> = vec![];
            let mut only1_decompiled: Vec<(&Symbol, &Symbol, f32)> = vec![];
            let mut only2_decompiled: Vec<(&Symbol, &Symbol, f32)> = vec![];
            let mut both_undecompiled: Vec<(&Symbol, &Symbol, f32)> = vec![];

            for (sym1, sym2, score) in matched_syms {
                if sym1.is_decompiled && sym2.is_decompiled {
                    both_decompiled.push((sym1, sym2, score));
                } else if sym1.is_decompiled {
                    only1_decompiled.push((sym1, sym2, score));
                } else if sym2.is_decompiled {
                    only2_decompiled.push((sym1, sym2, score));
                } else {
                    both_undecompiled.push((sym1, sym2, score));
                }
            }

            if !both_decompiled.is_empty() {
                println!(
                    "\nDecompiled in {} and {}:",
                    bin1.name.color(BINARY_COLORS[0]),
                    bin2.name.color(BINARY_COLORS[1])
                );
                for (sym1, sym2, score) in both_decompiled {
                    println!(
                        "{} - {} ({:.2}%)",
                        cli_name_colored(sym1, BINARY_COLORS[0]),
                        cli_name_colored(sym2, BINARY_COLORS[1]),
                        score * 100.0
                    );
                }
            }

            if !only1_decompiled.is_empty() {
                println!(
                    "\nOnly decompiled in {}:",
                    bin1.name.color(BINARY_COLORS[0])
                );
                for (sym1, sym2, score) in only1_decompiled {
                    println!(
                        "{} - {} ({:.2}%)",
                        cli_name_colored(sym1, BINARY_COLORS[0]),
                        cli_name_colored(sym2, BINARY_COLORS[1]),
                        score * 100.0
                    );
                }
            }

            if !only2_decompiled.is_empty() {
                println!(
                    "\nOnly decompiled in {}:",
                    bin2.name.color(BINARY_COLORS[1])
                );
                for (sym1, sym2, score) in only2_decompiled {
                    println!(
                        "{} - {} ({:.2}%)",
                        cli_name_colored(sym1, BINARY_COLORS[0]),
                        cli_name_colored(sym2, BINARY_COLORS[1]),
                        score * 100.0
                    );
                }
            }

            if !both_undecompiled.is_empty() {
                println!("\nDecompiled in neither:");
                for (sym1, sym2, score) in both_undecompiled {
                    println!(
                        "{} - {} ({:.2}%)",
                        cli_name_colored(sym1, BINARY_COLORS[0]),
                        cli_name_colored(sym2, BINARY_COLORS[1]),
                        score * 100.0
                    );
                }
            }
        }
    }
}

fn get_cwd_symbols() -> Result<Vec<Symbol>> {
    let config = scan_for_config()?;

    let version = if config.versions.len() > 1 {
        let res = Select::new("Which version do you want to use?", config.versions).prompt();
        res?
    } else {
        config.versions.first().unwrap().clone()
    };

    collect_symbols(&version, &std::env::current_dir()?, &config.platform)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let cli: Cli = Cli::parse();

    match &cli.command {
        Commands::Match { query, threshold } => {
            let symbols = get_cwd_symbols()?;
            do_match(query, &symbols, *threshold);
        }
        Commands::Submatch { query, window_size } => {
            let symbols = get_cwd_symbols()?;
            do_submatch(query, &symbols, *window_size);
        }
        Commands::Cluster { threshold, min_len } => {
            let symbols = get_cwd_symbols()?;
            do_cluster(&symbols, *threshold, *min_len);
        }
        Commands::Compare2 {
            yaml1,
            version1,
            yaml2,
            version2,
            threshold,
            min_len,
        } => {
            let config1 = read_config(yaml1.clone())?;
            let config2 = read_config(yaml2.clone())?;

            let version1 = config1.get_version_by_name(version1).unwrap();
            let version2 = config2.get_version_by_name(version2).unwrap();

            let symbols1 = collect_symbols(&version1, yaml1.parent().unwrap(), &config1.platform)?;
            let symbols2 = collect_symbols(&version2, yaml2.parent().unwrap(), &config2.platform)?;

            let bin1 = Binary {
                name: config1.name,
                symbols: symbols1,
            };

            let bin2 = Binary {
                name: config2.name,
                symbols: symbols2,
            };

            do_compare_binaries(&bin1, &bin2, *threshold, *min_len);
        }
        Commands::CompareN {
            main_yaml,
            main_version,
            other_yamls,
        } => {
            let main_config = read_config(main_yaml.clone())?;
            let main_version = main_config.get_version_by_name(main_version).unwrap();
            let main_symbols = collect_symbols(
                &main_version,
                main_yaml.parent().unwrap(),
                &main_config.platform,
            )?;

            let main_bin: Binary = Binary {
                name: main_config.name.clone(),
                symbols: main_symbols,
            };

            for other_yaml in other_yamls {
                let other_config = read_config(other_yaml.clone())?;

                for other_version in &other_config.versions {
                    let other_symbols = collect_symbols(
                        other_version,
                        other_yaml.parent().unwrap(),
                        &other_config.platform.clone(),
                    )?;

                    let other_bin = Binary {
                        name: other_config.name.clone(),
                        symbols: other_symbols,
                    };

                    println!(
                        "Comparing {} {} to {} {}:",
                        main_config.name.color(BINARY_COLORS[0]),
                        main_version.fullname.color(BINARY_COLORS[0]),
                        other_config.name.color(BINARY_COLORS[1]),
                        other_version.fullname.color(BINARY_COLORS[1])
                    );

                    do_compare_binaries(&main_bin, &other_bin, 0.99, 5);
                    println!();
                }
            }
        }
        Commands::CompareRaw { query_bin, yamls } => {
            let query_bin_data = fs::read(query_bin)?;
            let mut symbol_hashes = HashMap::new();
            let mut platform = None;
            let window_size = 20;

            for yaml in yamls {
                let config = read_config(yaml.clone())?;
                let cur_platform = Platform::from_name(&config.platform).unwrap();

                if platform.is_none() {
                    platform = Some(cur_platform);
                } else if platform.unwrap() != cur_platform {
                    return Err(anyhow!("All projects must be for the same platform"));
                }

                for version in &config.versions {
                    let symbols =
                        collect_symbols(version, yaml.parent().unwrap(), &config.platform)?;
                    for sym in symbols {
                        if sym.opcodes.len() < window_size {
                            continue;
                        }
                        let hashes = sym.get_opcode_hashes(window_size);
                        let first_hash = *hashes.first().unwrap();
                        symbol_hashes.insert(
                            first_hash,
                            (config.name.clone(), version.fullname.clone(), sym.clone()),
                        );
                    }
                }
            }

            let platform =
                platform.ok_or_else(|| anyhow!("No platform found in provided configs"))?;

            let opcodes = core::arch::get_opcodes(&query_bin_data, platform);
            for (i, hash) in core::get_hashes(&opcodes, window_size).iter().enumerate() {
                if let Some((project_name, version_name, symbol)) = symbol_hashes.get(hash)
                    && opcodes[i..i + symbol.opcodes.len()] == symbol.opcodes
                {
                    println!(
                        "0x{:X} - {} {}: {}",
                        i * platform.arch().insn_length(),
                        project_name.color(BINARY_COLORS[0]),
                        version_name.color(BINARY_COLORS[0]),
                        cli_fullname(symbol)
                    );
                }
            }
        }
        #[cfg(feature = "db")]
        Commands::Db(cmd) => db::handle_db_command(cmd).await?,
    }

    Ok(())
}

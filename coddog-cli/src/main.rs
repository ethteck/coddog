use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};
use colored::*;
use decomp_settings::{config::Version, read_config, scan_for_config};
use glob::glob;
use std::{
    fs,
    path::{Path, PathBuf},
};

use coddog_core::{
    self as core, cluster, get_hashes, get_submatches,
    ingest::{read_elf, read_map},
    Binary, Symbol,
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
    Match {
        /// Name of the query function
        query: String,

        /// Similarity threshold
        #[arg(short, long, default_value = "0.985")]
        threshold: f32,
    },

    /// Cluster functions by similarity, showing possible duplicates
    Cluster {
        /// Similarity threshold
        #[arg(short, long, default_value = "0.985")]
        threshold: f32,

        /// Minimum length of functions (in number of instructions) to consider
        #[arg(short, long, default_value = "5")]
        min_len: usize,
    },

    /// Find chunks of code similar to those in the query function
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

    /// Compare one binary to one or more others, showing the functions in common between them
    CompareN {
        /// Path to the main decomp.yaml
        main_yaml: PathBuf,

        /// Version to compare from the main yaml
        main_version: String,

        /// Path to other projects' decomp.yaml files
        other_yamls: Vec<PathBuf>,
    },
}

struct FunctionMatch<'a> {
    symbol: &'a Symbol,
    score: f32,
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

    let query_hashes = get_hashes(query_sym, window_size);

    for s in symbols {
        if s == query_sym {
            continue;
        }

        if query_sym.insns == s.insns {
            let match_pct = if query_sym.bytes == s.bytes {
                "100%"
            } else {
                "99%"
            };
            println!("{} matches {}", cli_fullname(s), match_pct);
            continue;
        }

        let hashes = get_hashes(s, window_size);

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

fn get_full_path(settings_dir: &Path, config: &Version, name: &str) -> Option<PathBuf> {
    config.paths.get(name).map(|path| {
        if path.is_relative() {
            settings_dir.join(path)
        } else {
            path.clone()
        }
    })
}

fn get_unmatched_funcs(settings_dir: &Path, config: &Version) -> Option<Vec<String>> {
    get_full_path(settings_dir, config, "asm").map(|asm_dir| {
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

fn collect_symbols(config: &Version, settings_dir: &Path, platform: String) -> Result<Vec<Symbol>> {
    let unmatched_funcs = get_unmatched_funcs(settings_dir, config);

    if let Some(elf_path) = get_full_path(settings_dir, config, "elf") {
        let elf_data = fs::read(elf_path)?;
        return read_elf(&platform, &unmatched_funcs, elf_data);
    }

    if let (Some(baserom_path), Some(map_path)) = (
        get_full_path(settings_dir, config, "baserom"),
        get_full_path(settings_dir, config, "map"),
    ) {
        let rom_bytes = std::fs::read(baserom_path)?;
        return read_map(platform, unmatched_funcs, rom_bytes, map_path);
    }

    panic!("No elf or mapfile found");
}

fn do_compare_binaries(bin1: &Binary, bin2: &Binary, threshold: f32, min_len: usize) {
    let mut matched_syms: Vec<(&Symbol, &Symbol, f32)> = Vec::new();

    bin1.symbols
        .iter()
        .filter(|s| s.insns.len() >= min_len)
        .for_each(|sym| {
            let mut best_match: Option<(&Symbol, f32)> = None;

            for sym2 in bin2.symbols.iter().filter(|s| s.insns.len() >= min_len) {
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
    let version = &config.versions[0]; // TODO: allow specifying
    Ok(collect_symbols(
        version,
        &std::env::current_dir()?,
        config.platform,
    )?)
}

fn main() {
    let cli: Cli = Cli::parse();

    match &cli.command {
        Commands::Match { query, threshold } => {
            let symbols = get_cwd_symbols().unwrap();
            do_match(query, &symbols, *threshold);
        }
        Commands::Submatch { query, window_size } => {
            let symbols = get_cwd_symbols().unwrap();
            do_submatch(query, &symbols, *window_size);
        }
        Commands::Cluster { threshold, min_len } => {
            let symbols = get_cwd_symbols().unwrap();
            cluster::do_cluster(&symbols, *threshold, *min_len);
        }
        Commands::Compare2 {
            yaml1,
            version1,
            yaml2,
            version2,
            threshold,
            min_len,
        } => {
            let config1 = read_config(yaml1.to_path_buf()).unwrap();
            let config2 = read_config(yaml2.to_path_buf()).unwrap();

            let version1 = config1.get_version_by_name(version1).unwrap();
            let version2 = config2.get_version_by_name(version2).unwrap();

            let symbols1 =
                collect_symbols(&version1, yaml1.parent().unwrap(), config1.platform).unwrap();
            let symbols2 =
                collect_symbols(&version2, yaml2.parent().unwrap(), config2.platform).unwrap();

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
            let main_config = read_config(main_yaml.to_path_buf()).unwrap();
            let main_version = main_config.get_version_by_name(main_version).unwrap();
            let main_symbols = collect_symbols(
                &main_version,
                main_yaml.parent().unwrap(),
                main_config.platform,
            )
            .unwrap();

            let main_bin: Binary = Binary {
                name: main_config.name.clone(),
                symbols: main_symbols,
            };

            for other_yaml in other_yamls {
                let other_config = read_config(other_yaml.to_path_buf()).unwrap();

                for other_version in &other_config.versions {
                    let other_symbols = collect_symbols(
                        other_version,
                        other_yaml.parent().unwrap(),
                        other_config.platform.clone(),
                    )
                    .unwrap();

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
    }
}

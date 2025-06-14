use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
use coddog_core::cluster::get_clusters;
use coddog_core::{
    self as core, get_submatches, ingest::{read_elf, read_map}, Binary, Platform,
    Symbol,
};
use coddog_db::projects::CreateProjectRequest;
use coddog_db::symbols::QuerySymbolsRequest;
use coddog_db::{DBSymbol, DBWindow};
use colored::*;
use decomp_settings::{config::Version, read_config, scan_for_config};
use dotenvy::dotenv;
use glob::glob;
use inquire::Select;
use itertools::Itertools;
use pbr::ProgressBar;
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use std::time::SystemTime;
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
    #[command(subcommand)]
    Db(DbCommands),
}

#[derive(Subcommand)]
enum DbCommands {
    /// Add a new project to the database, given a path to a repo
    AddProject {
        /// Path to the project's repo
        repo: PathBuf,
    },
    /// Delete a project from the database, removing its objects, symbols, and hashes
    DeleteProject {
        /// Name of the project to delete
        name: String,
    },
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
        Platform::of(platform).unwrap_or_else(|| panic!("Invalid platform: {}", platform));

    if let Some(elf_path) = get_full_path(base_dir, config.paths.elf.clone()) {
        let elf_data = fs::read(elf_path)?;
        return read_elf(platform, &unmatched_funcs, elf_data);
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

async fn db_search_symbol_by_name(conn: Pool<Postgres>, name: &str) -> Result<DBSymbol> {
    let symbols = coddog_db::symbols::query_by_name(
        conn,
        &QuerySymbolsRequest {
            name: name.to_string(),
        },
    )
    .await?;

    if symbols.is_empty() {
        return Err(anyhow!("No symbols found with the name '{}'", name));
    }

    if symbols.len() > 1 {
        let res = Select::new("Which symbol do you want to check?", symbols).prompt();
        Ok(res?)
    } else {
        Ok(symbols.first().unwrap().clone())
    }
}

async fn db_search_project_by_name(conn: Pool<Postgres>, name: &str) -> Result<i64> {
    let projects = coddog_db::projects::query_by_name(conn, name).await?;

    if projects.is_empty() {
        return Err(anyhow!("No projects found with the name '{}'", name));
    }

    if projects.len() > 1 {
        let res = Select::new("Which project do you want to check?", projects).prompt();
        Ok(res?.id)
    } else {
        Ok(projects.first().unwrap().id)
    }
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
                let cur_platform = Platform::of(&config.platform).unwrap();

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
                if let Some((project_name, version_name, symbol)) = symbol_hashes.get(hash) {
                    if opcodes[i..i + symbol.opcodes.len()] == symbol.opcodes {
                        println!(
                            "0x{:X} - {} {}: {}",
                            i * 4,
                            project_name.color(BINARY_COLORS[0]),
                            version_name.color(BINARY_COLORS[0]),
                            cli_fullname(symbol)
                        );
                    }
                }
            }
        }
        Commands::Db(DbCommands::AddProject { repo }) => {
            let yaml = repo.join("decomp.yaml");
            let config = read_config(yaml.clone())?;
            let platform = Platform::of(&config.platform).unwrap();
            let window_size = std::env::var("DB_WINDOW_SIZE")
                .expect("DB_WINDOW_SIZE must be set")
                .parse::<usize>()?;

            let pool = coddog_db::init().await?;

            let project_id = coddog_db::projects::create(
                pool.clone(),
                &CreateProjectRequest {
                    name: config.name.clone(),
                    platform: platform as i32,
                    repo: config.repo,
                },
            )
            .await?;

            let mut tx = pool.begin().await?;

            for version in &config.versions {
                let target_path =
                    get_full_path(yaml.parent().unwrap(), Some(version.paths.target.clone()))
                        .unwrap();
                let object_id =
                    coddog_db::create_object(&mut tx, project_id, &version.fullname, &target_path)
                        .await?;

                let symbols = collect_symbols(version, yaml.parent().unwrap(), &config.platform)?;

                let symbol_ids = coddog_db::symbols::create(&mut tx, object_id, &symbols).await;

                let mut pb = ProgressBar::new(symbols.len() as u64);

                pb.format("[=>-]");

                if config.versions.len() == 1 {
                    pb.message("Importing hashes ");
                } else {
                    pb.message(format!("Importing hashes ({}) ", version.fullname).as_str());
                }

                for (symbol, id) in symbols.iter().zip(symbol_ids) {
                    pb.inc();

                    let opcode_hashes = symbol.get_opcode_hashes(window_size);

                    coddog_db::create_symbol_window_hashes(&mut tx, &opcode_hashes, id).await?;
                }
                println!();
            }
            tx.commit().await?;
            println!("Imported project {} ", config.name);
        }
        Commands::Db(DbCommands::DeleteProject { name }) => {
            let pool = coddog_db::init().await?;

            let project = db_search_project_by_name(pool.clone(), name).await?;

            coddog_db::projects::delete(pool.clone(), project).await?;
            println!("Deleted project {name}");
        }
        Commands::Db(DbCommands::Match { query, match_type }) => {
            let pool = coddog_db::init().await?;

            let symbol = db_search_symbol_by_name(pool.clone(), query).await?;

            let matches = match match_type {
                MatchType::Opcode => {
                    coddog_db::symbols::query_by_opcode_hash(pool.clone(), &symbol).await?
                }
                MatchType::Equivalent => {
                    coddog_db::symbols::query_by_equiv_hash(pool.clone(), &symbol).await?
                }
                MatchType::Exact => {
                    coddog_db::symbols::query_by_exact_hash(pool.clone(), &symbol).await?
                }
            };

            if matches.is_empty() {
                println!("No matches found");
            } else {
                for sym in matches {
                    println!("{} - {} {}", sym.name, sym.project_name, sym.object_name);
                }
            }
        }
        Commands::Db(DbCommands::Submatch { query, window_size }) => {
            let db_window_size = std::env::var("DB_WINDOW_SIZE")
                .expect("DB_WINDOW_SIZE must be set")
                .parse::<usize>()?;

            if *window_size < db_window_size {
                return Err(anyhow!("Window size must be at least {}", db_window_size));
            }

            let pool = coddog_db::init().await?;

            let symbol = db_search_symbol_by_name(pool.clone(), query).await?;

            let before_time = SystemTime::now();
            let matching_hashes = coddog_db::query_windows_by_symbol_id(
                pool.clone(),
                symbol.id,
                (window_size - db_window_size) as i64,
            )
            .await?;

            match before_time.elapsed() {
                Ok(elapsed) => {
                    println!("Big query took {}ms", elapsed.as_millis());
                }
                Err(e) => {
                    println!("Error: {e:?}");
                }
            }

            if matching_hashes.is_empty() {
                println!("No matches found");
                return Ok(());
            }

            let mut project_map: HashMap<i64, String> = HashMap::new();
            let mut object_map: HashMap<i64, String> = HashMap::new();
            let mut symbol_map: HashMap<i64, String> = HashMap::new();

            let results = SubmatchResults::from_db_hashes(
                &matching_hashes,
                &mut project_map,
                &mut object_map,
                &mut symbol_map,
            );

            println!(
                "{}",
                results.to_string(*window_size, &project_map, &object_map, &symbol_map)
            );
        }
    }

    Ok(())
}

struct SubmatchResults {
    projects: Vec<SubmatchProjectResults>,
}

impl SubmatchResults {
    fn from_db_hashes(
        hashes: &[DBWindow],
        project_map: &mut HashMap<i64, String>,
        object_map: &mut HashMap<i64, String>,
        symbol_map: &mut HashMap<i64, String>,
    ) -> Self {
        let mut results = SubmatchResults { projects: vec![] };

        for (project_id, project_rows) in &hashes.iter().chunk_by(|h| h.project_id) {
            let project_rows = project_rows.collect_vec();
            let project_name = &project_rows.first().unwrap().project_name;
            project_map.insert(project_id, project_name.to_string());

            let mut project_results = SubmatchProjectResults {
                id: project_id,
                objects: vec![],
            };

            for (object_id, object_rows) in &project_rows.iter().chunk_by(|h| h.object_id) {
                let object_rows = object_rows.collect_vec();
                let object_name = &object_rows.first().unwrap().object_name;
                object_map.insert(object_id, object_name.to_string());

                let mut object_results = SubmatchobjectResults {
                    id: object_id,
                    symbols: vec![],
                };

                for (symbol_id, symbol_rows) in &object_rows.into_iter().chunk_by(|h| h.symbol_id) {
                    let symbol_rows = symbol_rows.collect_vec();
                    let sym_name = &symbol_rows.first().unwrap().symbol_name;
                    symbol_map.insert(symbol_id, sym_name.clone());

                    let sym_results = SubmatchSymbolResults {
                        id: symbol_id,
                        slices: symbol_rows
                            .into_iter()
                            .map(|h| SubmatchSliceResults {
                                query_start: h.query_start,
                                match_start: h.match_start,
                                length: h.length,
                            })
                            .collect(),
                    };
                    object_results.symbols.push(sym_results);
                }
                project_results.objects.push(object_results);
            }
            results.projects.push(project_results);
        }

        results
    }

    fn to_string(
        &self,
        window_size: usize,
        project_map: &HashMap<i64, String>,
        object_map: &HashMap<i64, String>,
        symbol_map: &HashMap<i64, String>,
    ) -> String {
        let mut result = String::new();
        for project in &self.projects {
            result.push_str(&format!("{}:\n", project_map.get(&project.id).unwrap()));
            for object in &project.objects {
                result.push_str(&format!(
                    "\tVersion {}:\n",
                    object_map.get(&object.id).unwrap()
                ));
                for symbol in &object.symbols {
                    result.push_str(&format!("\t\t{}:\n", symbol_map.get(&symbol.id).unwrap()));
                    for slice in &symbol.slices {
                        result.push_str(&format!(
                            "\t\t\t[{}/{}] ({} insns)\n",
                            slice.query_start,
                            slice.match_start,
                            slice.length as usize + window_size - 1
                        ));
                    }
                }
            }
        }
        result
    }
}

struct SubmatchProjectResults {
    id: i64,
    objects: Vec<SubmatchobjectResults>,
}

struct SubmatchobjectResults {
    id: i64,
    symbols: Vec<SubmatchSymbolResults>,
}

struct SubmatchSymbolResults {
    id: i64,
    slices: Vec<SubmatchSliceResults>,
}

struct SubmatchSliceResults {
    query_start: i32,
    match_start: i32,
    length: i64,
}

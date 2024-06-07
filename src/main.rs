use anyhow::{Ok, Result};
use bincode::serialize_into;
use clap::{Parser, Subcommand};
use colored::*;
use config::Config;
use dashmap::DashMap;
use editdistancek::edit_distance_bounded;
use glob::glob;
use mapfile_parser::MapFile;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fs::File,
    hash::{Hash, Hasher},
    io::BufWriter,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum Endianness {
    Little,
    Big,
}

#[derive(Clone, Serialize, Deserialize)]
struct ProjectConfig {
    name: String,
    shortname: String,
    asm_dir: PathBuf,
    map_path: PathBuf,
    rom_path: PathBuf,
    endianness: Endianness,
}

#[derive(Clone, Serialize, Deserialize)]
struct ConfigSettings {
    db_path: PathBuf,
    projects: Vec<ProjectConfig>,
}

impl ConfigSettings {
    fn project_config_by_shortname(&self, shortname: &str) -> Option<&ProjectConfig> {
        self.projects.iter().find(|x| x.shortname == shortname)
    }
}

/// Find cod
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Create {
        /// Name of the project
        project_name: String,

        /// Short name of the project
        project_shortname: String,

        /// Path to the directory containing the .s files
        asm_dir: String,

        /// Path to the map file
        map_path: String,

        /// Path to the ROM file
        rom_path: String,

        /// Endianness of the ROM file
        /// #[arg(default_value = "little", possible_values = &["little", "big"])]
        endianness: String,
    },
    Submatch {
        /// The project (shortname) to search in
        project: String,

        /// Name of the query function
        query: String,

        /// Window size
        window_size: usize,
    },
    Match {
        /// The project (shortname) to search in
        project: String,

        /// Name of the query function
        query: String,

        /// Similarity threshold
        #[arg(default_value = "0.985")]
        threshold: f32,
    },
    Cross {
        /// The project (shortname) to search in
        project: String,

        /// Similarity threshold
        #[arg(default_value = "0.985")]
        threshold: f32,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct Project {
    name: String,
    short_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Symbol {
    // the project this symbol belongs to
    pub project_id: u64,
    // the name of the symbol
    pub name: String,
    // a hash of the raw bytes of the symbol
    pub byte_hash: u64,
    // a hash of the normalized instructions of the symbol
    pub insn_hash: u64,
    // the symbol's instructions, normalized to essentially just opcodes
    pub insns: Vec<u8>,
    // whether the symbol is decompiled
    pub is_decompiled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SymbolSlice {
    // the slice hash
    hash: u64,
    // the symbol this slice belongs to
    symbol_id: u64,
    // the offset of the slice into the symbol
    offset: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct CodDogDB {
    projects: HashMap<u64, Project>,
    symbols: HashMap<u64, Symbol>,
    slices: HashMap<u64, SmallVec<[SymbolSlice; 1]>>,
}

fn load_config() -> ConfigSettings {
    let settings = Config::builder()
        .add_source(config::File::with_name("config"))
        .build()
        .unwrap();

    let settings_map = settings.try_deserialize::<ConfigSettings>().unwrap();

    // let root_dir: &Path = Path::new(settings_map.get("root_dir").unwrap());
    // let asm_dir = root_dir.join(settings_map.get("asm_dir").unwrap());
    // let map_path = root_dir.join(settings_map.get("map_path").unwrap());
    // let rom_path = root_dir.join(settings_map.get("rom_path").unwrap());
    // let endianness = match settings_map.get("endianness").unwrap().as_str() {
    //     "little" => Endianness::Little,
    //     "big" => Endianness::Big,
    //     _ => panic!("Invalid endianness"),
    // };

    settings_map
}

fn get_hashes(bytes: &Symbol, window_size: usize) -> Vec<u64> {
    let ret: Vec<u64> = bytes
        .insns
        .windows(window_size)
        .map(|x| {
            let mut hasher = DefaultHasher::new();
            (*x).hash(&mut hasher);
            hasher.finish()
        })
        .collect();
    ret
}

#[derive(Debug, Clone, Copy)]
struct InsnSeqMatch {
    offset1: usize,
    offset2: usize,
    length: usize,
}

fn get_submatches(hashes_1: &[u64], hashes_2: &[u64], window_size: usize) -> Vec<InsnSeqMatch> {
    let mut matches = Vec::new();

    let matching_hashes = hashes_1
        .iter()
        .enumerate()
        .filter(|(_, h)| hashes_2.contains(h))
        .map(|(i, h)| InsnSeqMatch {
            offset1: i,
            offset2: hashes_2.iter().position(|x| x == h).unwrap(),
            length: 1,
        })
        .collect::<Vec<InsnSeqMatch>>();

    if matching_hashes.is_empty() {
        return matches;
    }

    let mut match_groups: Vec<Vec<InsnSeqMatch>> = Vec::new();
    let mut cur_pos = matching_hashes[0].offset1;
    for mh in matching_hashes {
        if mh.offset1 == cur_pos + 1 {
            match_groups.last_mut().unwrap().push(mh);
        } else {
            match_groups.push(vec![mh]);
        }
        cur_pos = mh.offset1;
    }

    for group in match_groups {
        matches.push(InsnSeqMatch {
            offset1: group[0].offset1,
            offset2: group[0].offset2,
            length: group.len() + window_size,
        });
    }

    matches
}

struct FunctionMatch<'a> {
    symbol: &'a Symbol,
    score: f32,
}

fn do_match(query: &str, threshold: f32, symbols: &[Symbol]) {
    let Some(query_sym) = symbols.iter().find(|s| s.name == query) else {
        println!("Symbol {query:} not found");
        return;
    };

    let mut matches: Vec<FunctionMatch> = symbols
        .iter()
        .filter(|s| s.name != query_sym.name)
        .map(|s| FunctionMatch {
            symbol: s,
            score: diff_symbols(query_sym, s, threshold),
        })
        .filter(|m| m.score > threshold)
        .collect();

    // sort by score descending
    matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    for m in matches {
        let decompiled_str = if m.symbol.is_decompiled {
            " (decompiled)"
        } else {
            ""
        };
        println!(
            "{:.2}% - {}{}",
            m.score * 100.0,
            m.symbol.name,
            decompiled_str.green(),
        );
    }
}

fn do_submatch(query: &str, window_size: usize, symbols: &[Symbol]) {
    let Some(query_sym) = symbols.iter().find(|s| s.name == query) else {
        println!("Symbol {query:} not found");
        return;
    };

    let query_hashes = get_hashes(query_sym, window_size);

    for s in symbols {
        if s == query_sym {
            continue;
        }

        let decompiled_str = if s.is_decompiled { " (decompiled)" } else { "" };

        if query_sym.insns == s.insns {
            let match_pct = if query_sym.byte_hash == s.byte_hash {
                "100%"
            } else {
                "99%"
            };
            println!("{}{} matches {}", s.name, decompiled_str.green(), match_pct);
            continue;
        }

        let hashes = get_hashes(s, window_size);

        let pair_matches = get_submatches(&query_hashes, &hashes, window_size);

        if pair_matches.is_empty() {
            continue;
        }

        println!("{}{}:", s.name, decompiled_str.green());

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

fn do_crossmatch(threshold: f32, symbols: &[Symbol]) {
    let mut clusters: Vec<Vec<&Symbol>> = Vec::new();

    for symbol in symbols {
        let mut cluster_match = false;

        for cluster in &mut clusters {
            let cluster_score = diff_symbols(symbol, cluster[0], threshold);
            if cluster_score > threshold {
                cluster_match = true;
                cluster.push(symbol);
                break;
            }
        }

        // Add this symbol to a new cluster if it didn't match any existing clusters
        if !cluster_match {
            clusters.push(vec![symbol]);
        }
    }

    // Sort clusters by size
    clusters.sort_by_key(|c| std::cmp::Reverse(c.len()));

    // Print clusters
    for cluster in clusters.iter().filter(|x| x.len() > 1) {
        println!("Cluster {} has {} symbols", cluster[0].name, cluster.len());
    }
}

fn do_crossmatch2(threshold: f32, db: &CodDogDB) {
    let mut clusters: Vec<Vec<Symbol>> = Vec::new();

    for (_, symbol) in db.symbols.iter() {
        let mut cluster_match = false;

        for cluster in &mut clusters {
            let cluster_score = diff_symbols(&symbol, &cluster[0], threshold);
            if cluster_score > threshold {
                cluster_match = true;
                cluster.push(symbol.clone());
                break;
            }
        }

        // Add this symbol to a new cluster if it didn't match any existing clusters
        if !cluster_match {
            clusters.push(vec![symbol.clone()]);
        }
    }

    // Sort clusters by size
    clusters.sort_by_key(|c| std::cmp::Reverse(c.len()));

    // Print clusters
    for cluster in clusters.iter().filter(|x| x.len() > 1) {
        println!("Cluster {} has {} symbols", cluster[0].name, cluster.len());
    }
}

fn diff_symbols(sym1: &Symbol, sym2: &Symbol, threshold: f32) -> f32 {
    // The minimum edit distance for two strings of different lengths is `abs(l1 - l2)`
    // Quickly check if it's impossible to beat the threshold. If it is, then return 0
    let l1 = sym1.insns.len();
    let l2: usize = sym2.insns.len();
    let max_edit_dist = (l1 + l2) as f32;
    if (l1.abs_diff(l2) as f32 / max_edit_dist) > (1.0 - threshold) {
        return 0.0;
    }

    let bound = (max_edit_dist - (max_edit_dist * threshold)) as usize;
    if let Some(edit_distance) = edit_distance_bounded(&sym1.insns, &sym2.insns, bound) {
        let edit_dist = edit_distance as f32;
        let normalized_edit_dist = (max_edit_dist - edit_dist) / max_edit_dist;

        if normalized_edit_dist == 1.0 && sym1.byte_hash != sym2.byte_hash {
            return 0.9999;
        }
        normalized_edit_dist
    } else {
        0.0
    }
}

fn get_symbol_bytes(
    start: usize,
    end: usize,
    rom_bytes: &[u8],
    endianness: Endianness,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let raw = rom_bytes[start..end].to_vec();

    // Remove trailing nops
    let mut bs = raw.clone();
    while !bs.is_empty() && bs[bs.len() - 1] == 0 {
        bs.pop();
    }

    let skip_amt = match endianness {
        Endianness::Little => 3,
        Endianness::Big => 0,
    };

    let insns: Vec<u8> = bs
        .iter()
        .skip(skip_amt)
        .step_by(4)
        .map(|x| x >> 2) // normalize to just opcodes
        .collect();

    Ok((raw, insns))
}

fn get_unmatched_funcs(asm_dir: &Path) -> Result<Vec<String>> {
    let mut unmatched_funcs = Vec::new();

    for s_file in glob(asm_dir.join("**/*.s").to_str().unwrap()).unwrap() {
        // add filename minus extension to vec
        let s_file = s_file?;
        let s_file_stem = s_file.file_stem().unwrap().to_str().unwrap();
        unmatched_funcs.push(s_file_stem.to_string());
    }
    Ok(unmatched_funcs)
}

fn collect_symbols(config: &ProjectConfig) -> Result<Vec<Symbol>> {
    let rom_bytes = std::fs::read(&config.rom_path).unwrap();
    let unmatched_funcs = get_unmatched_funcs(&config.asm_dir).unwrap();
    let mut mapfile = MapFile::new();
    mapfile.parse_map_contents(std::fs::read_to_string(&config.map_path).unwrap());

    let symbol_bytes: Vec<Symbol> = mapfile
        .segments_list
        .iter()
        .flat_map(|x| x.files_list.iter())
        .filter(|x| x.section_type == ".text")
        .flat_map(|x| x.symbols.iter())
        .filter(|x| x.vrom.is_some() && x.size.is_some())
        .map(|x| {
            let start = x.vrom.unwrap() as usize;
            let end = start + x.size.unwrap() as usize;
            let (bytes, insns) =
                get_symbol_bytes(start, end, &rom_bytes, config.endianness).unwrap();
            let byte_hash = {
                let mut hasher = DefaultHasher::new();
                bytes.hash(&mut hasher);
                hasher.finish()
            };
            let insn_hash = {
                let mut hasher = DefaultHasher::new();
                insns.hash(&mut hasher);
                hasher.finish()
            };
            Symbol {
                project_id: 0, // TODO address
                name: x.name.clone(),
                byte_hash,
                insn_hash,
                insns,
                is_decompiled: !unmatched_funcs.contains(&x.name),
            }
        })
        .collect();

    Ok(symbol_bytes)
}

fn main() {
    let config = load_config();

    let cli = Cli::parse();

    let mut db: CodDogDB = match cli.command {
        Commands::Create { .. } => {
            if config.db_path.exists() {
                let f = File::open(config.db_path.clone()).unwrap();
                bincode::deserialize_from(f).unwrap()
            } else {
                CodDogDB {
                    projects: HashMap::new(),
                    symbols: HashMap::new(),
                    slices: HashMap::new(),
                }
            }
        }
        _ => {
            if config.db_path.exists() {
                let f: File = File::open(config.db_path.clone()).unwrap();
                bincode::deserialize_from(f).unwrap()
            } else {
                panic!("Database not found; please run create first");
            }
        }
    };

    println!(
        "Loaded {} projects, {} symbols, and {} slices from disk",
        db.projects.len(),
        db.symbols.len(),
        db.slices.len()
    );

    match &cli.command {
        Commands::Create {
            project_name,
            project_shortname,
            asm_dir,
            map_path,
            rom_path,
            endianness,
        } => {
            let pjconfig = ProjectConfig {
                name: project_name.clone(),
                shortname: project_shortname.clone(),
                asm_dir: PathBuf::from(asm_dir),
                map_path: PathBuf::from(map_path),
                rom_path: PathBuf::from(rom_path),
                endianness: match endianness.as_str() {
                    "little" => Endianness::Little,
                    "big" => Endianness::Big,
                    _ => panic!("Invalid endianness"),
                },
            };

            let window_size = 8;

            let symbols = collect_symbols(&pjconfig).unwrap();

            let project_id = db.projects.len() as u64;
            db.projects.insert(
                project_id,
                Project {
                    name: project_name.clone(),
                    short_name: project_shortname.clone(),
                },
            );

            for (i, symbol) in symbols.iter().enumerate() {
                db.symbols.insert(i as u64, symbol.clone());
            }

            for (s, symbol) in symbols.iter().enumerate() {
                for (o, slice) in get_hashes(symbol, window_size).iter().enumerate() {
                    match db.slices.contains_key(slice) {
                        true => {
                            db.slices.get_mut(slice).unwrap().push(SymbolSlice {
                                hash: *slice,
                                symbol_id: s as u64,
                                offset: o,
                            });
                        }
                        false => {
                            db.slices.insert(
                                *slice,
                                SmallVec::from_buf([SymbolSlice {
                                    hash: *slice,
                                    symbol_id: s as u64,
                                    offset: o,
                                }]),
                            );
                        }
                    }
                }
            }

            // Write to disk
            let mut f = BufWriter::new(File::create(config.db_path).unwrap());
            serialize_into(&mut f, &db).unwrap();

            println!(
                "Wrote {} symbols and {} slices (length {}) to disk",
                db.symbols.len(),
                db.slices.len(),
                window_size
            );
        }
        Commands::Match {
            project,
            query,
            threshold,
        } => {
            let pconfig = config.project_config_by_shortname(project).unwrap();
            let symbols = collect_symbols(pconfig).unwrap();
            do_match(query, *threshold, &symbols);
        }
        Commands::Submatch {
            project,
            query,
            window_size,
        } => {
            let pconfig = config.project_config_by_shortname(project).unwrap();
            let symbols = collect_symbols(pconfig).unwrap();
            do_submatch(query, *window_size, &symbols);
        }
        Commands::Cross { project, threshold } => {
            do_crossmatch2(*threshold, &db);
        }
    }
}

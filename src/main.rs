use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};
use colored::*;
use config::Config;
use glob::glob;
use mapfile_parser::{MapFile, Symbol};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
use triple_accel::levenshtein::levenshtein;

#[derive(Clone)]
struct ConfigSettings {
    asm_dir: PathBuf,
    map_path: PathBuf,
    rom_path: PathBuf,
    endianness: Endianness,
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
    Match {
        /// Name of the query function
        query: String,

        /// Window size
        window_size: usize,
    },
    Cross,
}

#[derive(Debug, Clone, Copy)]
enum Endianness {
    Little,
    Big,
}

struct SymbolBytes {
    // the raw bytes of the symbol
    raw: Vec<u8>,
    // the symbol's instructions, normalized to essentially just opcodes
    insns: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct InsnMatch {
    offset1: usize,
    offset2: usize,
    length: usize,
}

fn load_config() -> ConfigSettings {
    let settings = Config::builder()
        .add_source(config::File::with_name("config"))
        .build()
        .unwrap();

    let settings_map = settings
        .try_deserialize::<HashMap<String, String>>()
        .unwrap();

    let root_dir: &Path = Path::new(settings_map.get("root_dir").unwrap());
    let asm_dir = root_dir.join(settings_map.get("asm_dir").unwrap());
    let map_path = root_dir.join(settings_map.get("map_path").unwrap());
    let rom_path = root_dir.join(settings_map.get("rom_path").unwrap());
    let endianness = match settings_map.get("endianness").unwrap().as_str() {
        "little" => Endianness::Little,
        "big" => Endianness::Big,
        _ => panic!("Invalid endianness"),
    };

    ConfigSettings {
        asm_dir,
        map_path,
        rom_path,
        endianness,
    }
}

fn get_unmatched_funcs(asm_dir: &PathBuf) -> Result<Vec<String>> {
    let mut unmatched_funcs = Vec::new();

    for s_file in glob(asm_dir.join("**/*.s").to_str().unwrap()).unwrap() {
        // add filename minus extension to vec
        let s_file = s_file?;
        let s_file_stem = s_file.file_stem().unwrap().to_str().unwrap();
        unmatched_funcs.push(s_file_stem.to_string());
    }
    Ok(unmatched_funcs)
}

fn get_symbol_bytes(
    symbol: &Symbol,
    rom_bytes: &Vec<u8>,
    endianness: &Endianness,
) -> Result<SymbolBytes> {
    if symbol.vrom.is_none() || symbol.size.is_none() {
        return Err(anyhow::anyhow!("Symbol {:?} has no vrom or size", symbol));
    }
    let start = symbol.vrom.unwrap() as usize;
    let end = start + symbol.size.unwrap() as usize;
    let raw = rom_bytes[start..end].to_vec();

    // Remove trailing nops
    let mut bs = raw.clone();
    while bs.len() > 0 && bs[bs.len() - 1] == 0 {
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
        .map(|x| x >> 2)
        .collect();

    let ret = SymbolBytes { raw, insns };

    Ok(ret)
}

fn get_hashes(bytes: &SymbolBytes, window_size: usize) -> Vec<u64> {
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

fn get_pair_matches(
    hashes_1: &Vec<u64>,
    hashes_2: &Vec<u64>,
    window_size: usize,
) -> Vec<InsnMatch> {
    let mut matches = Vec::new();

    let matching_hashes = hashes_1
        .iter()
        .enumerate()
        .filter(|(_, h)| hashes_2.contains(h))
        .map(|(i, h)| InsnMatch {
            offset1: i,
            offset2: hashes_2.iter().position(|x| x == h).unwrap(),
            length: 1,
        })
        .collect::<Vec<InsnMatch>>();

    if matching_hashes.len() == 0 {
        return matches;
    }

    let mut match_groups: Vec<Vec<InsnMatch>> = Vec::new();
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
        matches.push(InsnMatch {
            offset1: group[0].offset1,
            offset2: group[0].offset2,
            length: group.len() + window_size,
        })
    }

    matches
}

fn do_match(query: &str, window_size: usize, config: &ConfigSettings) {
    let rom_bytes = std::fs::read(&config.rom_path).unwrap();
    let unmatched_funcs = get_unmatched_funcs(&config.asm_dir).unwrap();
    let mut mapfile = MapFile::new();
    mapfile.parse_map_contents(std::fs::read_to_string(&config.map_path).unwrap());

    let query_sym_info = match mapfile.find_symbol_by_name(query) {
        Some(x) => x,
        None => {
            println!("Symbol {:?} not found", query);
            return;
        }
    };

    let query_bytes =
        get_symbol_bytes(&query_sym_info.symbol, &rom_bytes, &config.endianness).unwrap();

    let query_hashes = get_hashes(&query_bytes, window_size);

    for segment in &mapfile.segments_list {
        for file in &segment.files_list {
            if file.section_type != ".text" {
                continue;
            }

            for s in &file.symbols {
                if s == &query_sym_info.symbol {
                    continue;
                }

                let decompiled_str = match unmatched_funcs.contains(&s.name) {
                    true => "",
                    false => " (decompiled)",
                };

                let sb = get_symbol_bytes(&s, &rom_bytes, &config.endianness);
                if sb.is_ok() {
                    let bytes = sb.unwrap();

                    if query_bytes.insns == bytes.insns {
                        let match_pct = match query_bytes.raw == bytes.raw {
                            true => "100%",
                            false => "99%",
                        };
                        println!("{}{} matches {}", s.name, decompiled_str.green(), match_pct);
                        continue;
                    }

                    let hashes = get_hashes(&bytes, window_size);

                    let pair_matches = get_pair_matches(&query_hashes, &hashes, window_size);

                    if pair_matches.len() == 0 {
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
                        println!("\t{} matches {}", query_str, target_str)
                    }
                }
            }
        }
    }
}

fn do_crossmatch(config: &ConfigSettings) {
    let rom_bytes = std::fs::read(&config.rom_path).unwrap();
    let mut mapfile = MapFile::new();
    mapfile.parse_map_contents(std::fs::read_to_string(&config.map_path).unwrap());
    //let mut unmatched_funcs = get_unmatched_funcs(&config.asm_dir).unwrap();
    let mut clusters: Vec<Vec<&Symbol>> = Vec::new();

    let threshold = 0.985;

    // Collect all symbol bytes
    let symbol_bytes: HashMap<&Symbol, SymbolBytes> = mapfile
        .segments_list
        .iter()
        .flat_map(|x| x.files_list.iter())
        .filter(|x| x.section_type == ".text")
        .flat_map(|x| x.symbols.iter())
        .filter(|x| x.vrom.is_some() && x.size.is_some())
        .map(|x| {
            let sb = get_symbol_bytes(&x, &rom_bytes, &config.endianness);
            (x, sb.unwrap())
        })
        .collect();

    for (symbol, bytes) in &symbol_bytes {
        let mut cluster_match = false;

        for cluster in clusters.iter_mut() {
            let cluster_score = diff_syms(&bytes, &symbol_bytes[cluster[0]], threshold);
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
    clusters.sort_by(|a, b| b.len().cmp(&a.len()));

    // Print clusters
    for cluster in clusters.iter().filter(|x| x.len() > 1) {
        println!("Cluster {} has {} symbols", cluster[0].name, cluster.len());
    }
}

fn diff_syms(sym1: &SymbolBytes, sym2: &SymbolBytes, threshold: f32) -> f32 {
    // The minimum edit distance for two strings of different lengths is `abs(l1 - l2)`
    // Quickly check if it's impossible to beat the threshold. If it is, then return 0
    let l1 = sym1.insns.len();
    let l2: usize = sym2.insns.len();
    let max_edit_dist = (l1 + l2) as f32;
    if (l1.abs_diff(l2) as f32 / max_edit_dist) > (1.0 - threshold) {
        return 0.0;
    }

    let edit_dist = levenshtein(&sym1.insns, &sym2.insns) as f32;
    let normalized_edit_dist = (max_edit_dist - edit_dist) / max_edit_dist;

    if normalized_edit_dist == 1.0 && sym1.raw != sym2.raw {
        return 0.99;
    }
    normalized_edit_dist
}

fn main() {
    let config = load_config();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Match { query, window_size } => {
            do_match(query, *window_size, &config);
        }
        Commands::Cross => {
            do_crossmatch(&config);
        }
    }
}

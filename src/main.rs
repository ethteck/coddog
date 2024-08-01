use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};
use colored::*;
use decomp_settings::{config::Config, scan_for_config};
use editdistancek::edit_distance_bounded;
use glob::glob;
use mapfile_parser::{MapFile, Symbol};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::Path,
};

#[derive(Debug, Clone, Copy)]
enum Endianness {
    Little,
    Big,
}

impl Endianness {
    fn from_platform(platform: &str) -> Self {
        match platform {
            "n64" => Endianness::Big,
            "ps2" => Endianness::Little,
            _ => panic!("Unknown platform {}", platform),
        }
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
    Submatch {
        /// Name of the query function
        query: String,

        /// Window size
        window_size: usize,
    },
    Match {
        /// Name of the query function
        query: String,

        /// Similarity threshold
        #[arg(default_value = "0.985")]
        threshold: f32,
    },
    Cross {
        /// Similarity threshold
        #[arg(default_value = "0.985")]
        threshold: f32,
    },
}

#[derive(Debug, PartialEq, Eq)]
struct CodDogSym {
    // the name of the symbol
    pub name: String,
    // the raw bytes of the symbol
    pub bytes: Vec<u8>,
    // the symbol's instructions, normalized to essentially just opcodes
    pub insns: Vec<u8>,
    // whether the symbol is decompiled
    pub is_decompiled: bool,
}

fn get_hashes(bytes: &CodDogSym, window_size: usize) -> Vec<u64> {
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
    symbol: &'a CodDogSym,
    score: f32,
}

fn do_match(query: &str, threshold: f32, symbols: &[CodDogSym]) {
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

fn do_submatch(query: &str, window_size: usize, symbols: &[CodDogSym]) {
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
            let match_pct = if query_sym.bytes == s.bytes {
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

fn do_crossmatch(threshold: f32, symbols: &[CodDogSym]) {
    let mut clusters: Vec<Vec<&CodDogSym>> = Vec::new();

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

fn diff_symbols(sym1: &CodDogSym, sym2: &CodDogSym, threshold: f32) -> f32 {
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

        if normalized_edit_dist == 1.0 && sym1.bytes != sym2.bytes {
            return 0.9999;
        }
        normalized_edit_dist
    } else {
        0.0
    }
}

fn get_symbol_bytes(
    symbol: &Symbol,
    rom_bytes: &[u8],
    endianness: Endianness,
) -> Result<(Vec<u8>, Vec<u8>)> {
    if symbol.vrom.is_none() || symbol.size.is_none() {
        return Err(anyhow::anyhow!("Symbol {:?} has no vrom or size", symbol));
    }
    let start = symbol.vrom.unwrap() as usize;
    let end = start + symbol.size.unwrap() as usize;
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

fn collect_symbols(config: &Config) -> Result<Vec<CodDogSym>> {
    let version = config.get_default_version()?;

    let baserom_path = version.paths.baserom.unwrap();
    let asm_dir = version.paths.asm.unwrap();
    let map_path = version.paths.map.unwrap();

    let baserom_path = Path::new(&baserom_path);
    let asm_dir = Path::new(&asm_dir);
    let map_path = Path::new(&map_path);

    let rom_bytes = std::fs::read(baserom_path).unwrap();
    let unmatched_funcs = get_unmatched_funcs(asm_dir).unwrap();
    let mut mapfile = MapFile::new();
    mapfile.parse_map_contents(std::fs::read_to_string(map_path).unwrap());

    let symbol_bytes: Vec<CodDogSym> = mapfile
        .segments_list
        .iter()
        .flat_map(|x| x.files_list.iter())
        .filter(|x| x.section_type == ".text")
        .flat_map(|x| x.symbols.iter())
        .filter(|x| x.vrom.is_some() && x.size.is_some())
        .map(|x| {
            let (bytes, insns) =
                get_symbol_bytes(x, &rom_bytes, Endianness::from_platform(&config.platform))
                    .unwrap();
            CodDogSym {
                name: x.name.clone(),
                bytes,
                insns,
                is_decompiled: !unmatched_funcs.contains(&x.name),
            }
        })
        .collect();

    Ok(symbol_bytes)
}

fn main() {
    let config = scan_for_config().unwrap();

    let cli = Cli::parse();

    let symbols = collect_symbols(&config).unwrap();

    match &cli.command {
        Commands::Match { query, threshold } => {
            do_match(query, *threshold, &symbols);
        }
        Commands::Submatch { query, window_size } => {
            do_submatch(query, *window_size, &symbols);
        }
        Commands::Cross { threshold } => {
            do_crossmatch(*threshold, &symbols);
        }
    }
}

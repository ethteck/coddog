use anyhow::{Ok, Result};
use clap::Parser;
use config::Config;
use glob::glob;
use mapfile_parser::{MapFile, Symbol};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

/// Find cod
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the query function
    query: String,

    /// Window size
    window_size: usize,

    /// min match length
    min: Option<usize>,

    /// max match length
    max: Option<usize>,
}

enum Endianness {
    Little,
    Big,
}

struct SymbolBytes {
    raw: Vec<u8>,
    insns: Vec<u8>,
}

// TODO FIX
fn get_unmatched_funcs(asm_dir: PathBuf) -> Result<Vec<String>> {
    let mut unmatched_funcs = Vec::new();

    for s_file in glob(asm_dir.join("../*.s").to_str().unwrap())? {
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

fn main() {
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

    let args = Args::parse();

    let rom_bytes = std::fs::read(rom_path).unwrap();
    let unmatched_funcs = get_unmatched_funcs(asm_dir).unwrap();
    let mut mapfile = MapFile::new();
    mapfile.parse_map_contents(std::fs::read_to_string(map_path).unwrap());

    let query_sym_info = mapfile.find_symbol_by_name(&args.query);

    if query_sym_info.is_none() {
        println!("Symbol {:?} not found", args.query);
        return;
    }

    let query_sym_info = query_sym_info.unwrap();

    let query_bytes = get_symbol_bytes(&query_sym_info.symbol, &rom_bytes, &endianness).unwrap();

    let query_hashes = get_hashes(&query_bytes, args.window_size);

    for segment in &mapfile.segments_list {
        for file in &segment.files_list {
            if file.section_type != ".text" {
                continue;
            }

            for s in &file.symbols {
                if unmatched_funcs.contains(&s.name) {
                    continue;
                }
                if s == &query_sym_info.symbol {
                    continue;
                }

                let sb = get_symbol_bytes(&s, &rom_bytes, &endianness);
                if sb.is_ok() {
                    let bytes = sb.unwrap();
                    let hashes = get_hashes(&bytes, args.window_size);

                    let mut matches = Vec::new();
                    for (i, h) in hashes.iter().enumerate() {
                        if query_hashes.contains(h) {
                            matches.push(i);
                        }
                    }

                    if matches.len() > 0 {
                        println!("{}: {:?}", s.name, matches);
                    }
                }
            }
        }
    }
}

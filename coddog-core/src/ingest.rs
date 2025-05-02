use std::path::PathBuf;

use anyhow::Result;
use mapfile_parser::MapFile;
use object::{Object, ObjectSection, ObjectSymbol};

use crate::{Endianness, Symbol};

pub fn read_elf(
    platform: &str,
    unmatched_funcs: &Option<Vec<String>>,
    elf_data: Vec<u8>,
) -> Result<Vec<Symbol>> {
    let file = object::File::parse(&*elf_data)?;
    let ret: Vec<Symbol> = file
        .symbols()
        .filter(|s| s.kind() == object::SymbolKind::Text)
        .filter_map(|symbol| {
            symbol.section_index().and_then(|i| {
                file.section_by_index(i)
                    .ok()
                    .map(|section| (symbol, section))
            })
        })
        .filter_map(|(symbol, section)| {
            section
                .data_range(symbol.address(), symbol.size())
                .ok()
                .flatten()
                .map(|data| (symbol, data))
        })
        .map(|(symbol, data)| {
            let insns: Vec<u8> = get_mips_insns(data, Endianness::from_platform(platform));
            Symbol {
                id: 0,
                name: symbol.name().unwrap().to_string(),
                bytes: data.to_vec(),
                insns,
                is_decompiled: unmatched_funcs
                    .as_ref()
                    .is_some_and(|fs| !fs.contains(&symbol.name().unwrap().to_string())),
            }
        })
        .collect();
    Ok(ret)
}

fn get_mips_insns(bytes: &[u8], endianness: Endianness) -> Vec<u8> {
    // Remove trailing nops
    let mut bs = bytes.to_vec();
    while !bs.is_empty() && bs[bs.len() - 1] == 0 {
        bs.pop();
    }

    match endianness {
        Endianness::Little => bs.iter().step_by(4).map(|x| x >> 2).collect(),
        Endianness::Big => bs.iter().skip(3).step_by(4).map(|x| x >> 2).collect(),
    }
}

pub fn read_map(
    platform: String,
    unmatched_funcs: Option<Vec<String>>,
    rom_bytes: Vec<u8>,
    map_path: PathBuf,
) -> Result<Vec<Symbol>> {
    let mut mapfile = MapFile::new();
    mapfile.parse_map_contents(std::fs::read_to_string(map_path)?.as_str());
    let ret: Vec<Symbol> = mapfile
        .segments_list
        .iter()
        .flat_map(|x| x.files_list.iter())
        .filter(|x| x.section_type == ".text")
        .flat_map(|x| x.symbols.iter())
        .filter(|x| x.vrom.is_some() && x.size.is_some())
        .enumerate()
        .map(|(id, x)| {
            let start = x.vrom.unwrap() as usize;
            let end = start + x.size.unwrap() as usize;
            let raw = &rom_bytes[start..end];
            let insns = get_mips_insns(raw, Endianness::from_platform(&platform));

            Symbol {
                id,
                name: x.name.clone(),
                bytes: raw.to_vec(),
                insns,
                is_decompiled: unmatched_funcs
                    .as_ref()
                    .is_some_and(|fs| !fs.contains(&x.name)),
            }
        })
        .collect();
    Ok(ret)
}

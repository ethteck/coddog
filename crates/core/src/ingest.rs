use std::path::PathBuf;

use anyhow::Result;
use mapfile_parser::MapFile;
use object::{Object, ObjectSection, ObjectSymbol};

use crate::arch::get_opcodes;
use crate::{Platform, Symbol};

pub fn read_elf(
    platform: Platform,
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
                .map(|data| {
                    (
                        symbol,
                        data,
                        section.address(),
                        section.file_range().unwrap().0,
                    )
                })
        })
        .map(|(symbol, data, section_address, section_offset)| {
            let opcodes = get_opcodes(data, platform);
            let offset = symbol.address() - section_address + section_offset;

            Symbol::new(
                0,
                symbol.name().unwrap().to_string(),
                data.to_vec(),
                opcodes,
                offset as usize,
                unmatched_funcs
                    .as_ref()
                    .is_some_and(|fs| !fs.contains(&symbol.name().unwrap().to_string())),
                platform,
            )
        })
        .collect();
    Ok(ret)
}

pub fn read_map(
    platform: Platform,
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
            let opcodes = get_opcodes(raw, platform);

            Symbol::new(
                id,
                x.name.clone(),
                raw.to_vec(),
                opcodes,
                start,
                unmatched_funcs
                    .as_ref()
                    .is_some_and(|fs| !fs.contains(&x.name)),
                platform,
            )
        })
        .collect();
    Ok(ret)
}

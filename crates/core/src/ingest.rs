use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;
use mapfile_parser::MapFile;
use object::{
    elf, Endian, File, Object, ObjectSection, ObjectSymbol, Relocation,
    RelocationFlags, RelocationTarget,
};

use crate::ingest::CoddogRel::SymbolTarget;
use crate::{Platform, Symbol};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CoddogRel {
    SymbolTarget(String, i64),
}

fn get_reloc_target(elf: &File, addr: u64, reloc: &Relocation, addend: i64) -> CoddogRel {
    match reloc.target() {
        RelocationTarget::Symbol(s) => {
            let symbol = elf
                .symbol_by_index(s)
                .ok()
                .and_then(|s| s.name().ok())
                .unwrap_or_default();
            SymbolTarget(symbol.to_string(), addend)
        }
        RelocationTarget::Absolute => {
            panic!("Absolute reloc: {:#x} => ({})?", addr, addend);
        }
        _ => {
            panic!("Unsupported reloc: {:#x} => ({})?", addr, addend);
        }
    }
}

pub fn read_elf(
    platform: Platform,
    unmatched_funcs: &Option<Vec<String>>,
    elf_data: Vec<u8>,
) -> Result<Vec<Symbol>> {
    let file = File::parse(&*elf_data)?;

    let relocation_data = get_reloc_data(platform, &file)?;

    let ret: Vec<Symbol> = file
        .symbols()
        .filter(|s| s.kind() == object::SymbolKind::Text)
        .filter_map(|symbol| {
            symbol.section_index().and_then(|i| {
                file.section_by_index(i)
                    .ok()
                    .map(|section| (symbol, section, &relocation_data[i.0]))
            })
        })
        .filter_map(|(symbol, section, relocation_data)| {
            section
                .data_range(symbol.address(), symbol.size())
                .ok()
                .flatten()
                .map(|data| {
                    (
                        symbol,
                        relocation_data,
                        data,
                        section.address(),
                        section.file_range().unwrap().0,
                    )
                })
        })
        .map(
            |(symbol, section_relocations, data, section_address, section_offset)| {
                let offset = symbol.address() - section_address + section_offset;

                Symbol::new(
                    symbol.name().unwrap().to_string(),
                    data.to_vec(),
                    symbol.address() as usize,
                    offset as usize,
                    unmatched_funcs
                        .as_ref()
                        .is_some_and(|fs| !fs.contains(&symbol.name().unwrap().to_string())),
                    platform,
                    section_relocations,
                )
            },
        )
        .collect();
    Ok(ret)
}

fn get_reloc_data(platform: Platform, file: &File) -> Result<Vec<BTreeMap<u64, CoddogRel>>> {
    // Copied & modified from objdiff - thx
    // Parse all relocations to pair R_MIPS_HI16 and R_MIPS_LO16. Since the instructions only
    // have 16-bit immediate fields, the 32-bit addend is split across the two relocations.
    // R_MIPS_LO16 relocations without an immediately preceding R_MIPS_HI16 use the last seen
    // R_MIPS_HI16 addend.
    // See https://refspecs.linuxfoundation.org/elf/mipsabi.pdf pages 4-17 and 4-18

    let mut relocation_data = Vec::with_capacity(file.sections().count() + 1);
    for obj_section in file.sections() {
        let data = obj_section.data().unwrap_or_default();
        let mut last_hi = None;
        let mut last_hi_addend = 0;
        let mut section_relocs = BTreeMap::new();

        for (addr, reloc) in obj_section.relocations() {
            if !reloc.has_implicit_addend() {
                continue;
            }
            match reloc.flags() {
                RelocationFlags::Elf {
                    r_type: elf::R_MIPS_HI16,
                } => {
                    let code = data[addr as usize..addr as usize + 4].try_into()?;
                    let addend =
                        ((platform.endianness().read_u32_bytes(code) & 0x0000FFFF) << 16) as i32;
                    last_hi = Some(addr);
                    last_hi_addend = addend;
                }
                RelocationFlags::Elf {
                    r_type: elf::R_MIPS_LO16,
                } => {
                    let code = data[addr as usize..addr as usize + 4].try_into()?;
                    let addend =
                        (platform.endianness().read_u32_bytes(code) & 0x0000FFFF) as i16 as i32;
                    let full_addend = (last_hi_addend + addend) as i64;

                    let reloc_target = get_reloc_target(file, addr, &reloc, full_addend);

                    if let Some(hi_addr) = last_hi.take() {
                        section_relocs.insert(hi_addr, reloc_target.clone());
                    }
                    section_relocs.insert(addr, reloc_target);
                }
                RelocationFlags::Elf {
                    r_type: elf::R_MIPS_26,
                } => {
                    section_relocs
                        .insert(addr, get_reloc_target(file, addr, &reloc, reloc.addend()));
                }
                _ => {
                    last_hi = None;
                }
            }
        }
        let section_index = obj_section.index().0;
        if section_index >= relocation_data.len() {
            relocation_data.resize_with(section_index + 1, Default::default);
        }
        relocation_data[section_index] = section_relocs;
    }
    Ok(relocation_data)
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
        .map(|x| {
            let start = x.vrom.unwrap() as usize;
            let end = start + x.size.unwrap() as usize;
            let raw = &rom_bytes[start..end];

            Symbol::new(
                x.name.clone(),
                raw.to_vec(),
                x.vram as usize,
                start,
                unmatched_funcs
                    .as_ref()
                    .is_some_and(|fs| !fs.contains(&x.name)),
                platform,
                &BTreeMap::default(),
            )
        })
        .collect();
    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_elf() {
        let elf_data = include_bytes!("../../../test/test_mips.o").to_vec();
        let unmatched_funcs = None;
        let platform = Platform::N64;
        let symbols = read_elf(platform, &unmatched_funcs, elf_data).unwrap();
        assert!(!symbols.is_empty());
        assert_eq!(symbols[0].equiv_hash, symbols[3].equiv_hash);
    }
}

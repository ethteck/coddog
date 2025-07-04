use std::collections::BTreeMap;

use anyhow::Result;
use mapfile_parser::MapFile;
use object::{
    Endian, File, Object, ObjectSection, ObjectSymbol, Relocation, RelocationFlags,
    RelocationTarget, elf,
};

use crate::ingest::CoddogRel::SymbolTarget;
use crate::{Arch, Platform, Symbol, SymbolDef};

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
            panic!("Absolute reloc: {addr:#x} => ({addend})?");
        }
        _ => {
            panic!("Unsupported reloc: {addr:#x} => ({addend})?");
        }
    }
}

pub fn read_elf(
    platform: Platform,
    unmatched_funcs: &Option<Vec<String>>,
    elf_data: &[u8],
) -> Result<Vec<Symbol>> {
    let file = File::parse(elf_data)?;

    let relocation_data = get_reloc_data(platform, &file)?;

    let ret: Vec<Symbol> = file
        .symbols()
        .enumerate()
        .filter(|elem| elem.1.kind() == object::SymbolKind::Text)
        .filter_map(|elem| {
            let symbol = elem.1;
            symbol.section_index().and_then(|i| {
                file.section_by_index(i)
                    .ok()
                    .map(|section| (symbol, elem.0, section, &relocation_data[i.0]))
            })
        })
        .filter_map(|(symbol, symbol_idx, section, relocation_data)| {
            section
                .data_range(symbol.address(), symbol.size())
                .ok()
                .flatten()
                .map(|data| {
                    (
                        symbol,
                        symbol_idx,
                        relocation_data,
                        data,
                        section.address(),
                        section.file_range().unwrap().0,
                    )
                })
        })
        .map(
            |(symbol, symbol_idx, section_relocations, data, section_address, section_offset)| {
                let offset = symbol.address() - section_address + section_offset;

                Symbol::new(SymbolDef {
                    name: symbol.name().unwrap().to_string(),
                    bytes: data.to_vec(),
                    vram: symbol.address() as usize,
                    offset: offset as usize,
                    is_decompiled: unmatched_funcs
                        .as_ref()
                        .is_some_and(|fs| !fs.contains(&symbol.name().unwrap().to_string())),
                    platform,
                    relocations: section_relocations.clone(),
                    symbol_idx,
                })
            },
        )
        .collect();
    Ok(ret)
}

fn get_reloc_data(platform: Platform, file: &File) -> Result<Vec<BTreeMap<u64, CoddogRel>>> {
    // Copied & modified from objdiff - thx

    let mut relocation_data = Vec::with_capacity(file.sections().count() + 1);

    let insn_length = platform.arch().insn_length();

    match platform.arch() {
        Arch::Mips => {
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
                            let code =
                                data[addr as usize..addr as usize + insn_length].try_into()?;
                            let addend = ((platform.endianness().read_u32_bytes(code) & 0x0000FFFF)
                                << 16) as i32;
                            last_hi = Some(addr);
                            last_hi_addend = addend;
                        }
                        RelocationFlags::Elf {
                            r_type: elf::R_MIPS_LO16,
                        } => {
                            let code =
                                data[addr as usize..addr as usize + insn_length].try_into()?;
                            let addend = (platform.endianness().read_u32_bytes(code) & 0x0000FFFF)
                                as i16 as i32;
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
        }
        Arch::Ppc => {
            for obj_section in file.sections() {
                let mut section_relocs = BTreeMap::new();
                for (addr, reloc) in obj_section.relocations() {
                    match reloc.flags() {
                        RelocationFlags::Elf {
                            r_type: elf::R_PPC_EMB_SDA21,
                        } => {
                            section_relocs
                                .insert(addr, get_reloc_target(file, addr, &reloc, reloc.addend()));
                        }
                        RelocationFlags::Elf {
                            r_type: elf::R_PPC_REL24,
                        } => {
                            section_relocs
                                .insert(addr, get_reloc_target(file, addr, &reloc, reloc.addend()));
                        }
                        RelocationFlags::Elf {
                            r_type: elf::R_PPC_ADDR32,
                        } => {
                            section_relocs
                                .insert(addr, get_reloc_target(file, addr, &reloc, reloc.addend()));
                        }
                        _ => todo!("Unsupported relocation type: {:?}", reloc.flags()),
                    }
                }
                let section_index = obj_section.index().0;
                if section_index >= relocation_data.len() {
                    relocation_data.resize_with(section_index + 1, Default::default);
                }
                relocation_data[section_index] = section_relocs;
            }
        }
    }

    Ok(relocation_data)
}

pub fn read_map(
    platform: Platform,
    unmatched_funcs: Option<Vec<String>>,
    rom_bytes: Vec<u8>,
    map_str: &str,
) -> Result<Vec<Symbol>> {
    let mapfile = MapFile::new_from_map_str(map_str);

    let ret: Vec<Symbol> = mapfile
        .segments_list
        .iter()
        .flat_map(|x| x.sections_list.iter())
        .filter(|x| x.section_type == ".text")
        .flat_map(|x| x.symbols.iter())
        .filter(|x| x.vrom.is_some())
        .map(|x| {
            let start = x.vrom.unwrap() as usize;
            let end = start + x.size as usize;
            let raw = &rom_bytes[start..end];

            Symbol::new(SymbolDef {
                name: x.name.clone(),
                bytes: raw.to_vec(),
                vram: x.vram as usize,
                offset: start,
                is_decompiled: unmatched_funcs
                    .as_ref()
                    .is_some_and(|fs| !fs.contains(&x.name)),
                platform,
                relocations: BTreeMap::default(),
                symbol_idx: 0, // No symbol index in plain binaries
            })
        })
        .collect();
    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_mips() {
        let elf_data = include_bytes!("../../../test/simple_mips.o").to_vec();
        let symbols = read_elf(Platform::N64, &None, &elf_data).unwrap();
        assert!(!symbols.is_empty());

        let tf1 = symbols.iter().find(|s| s.name == "test_1").unwrap();
        let tf2 = symbols.iter().find(|s| s.name == "test_2").unwrap();
        let tf3 = symbols.iter().find(|s| s.name == "test_3").unwrap();

        assert_eq!(tf1.opcode_hash, tf2.opcode_hash);
        assert_eq!(tf1.equiv_hash, tf2.equiv_hash);
        assert_ne!(tf1.exact_hash, tf2.exact_hash);

        assert_eq!(tf1.opcode_hash, tf3.opcode_hash);
        assert_ne!(tf1.equiv_hash, tf3.equiv_hash);
        assert_ne!(tf1.exact_hash, tf3.exact_hash);

        let math_op_1 = symbols.iter().find(|s| s.name == "math_op_1").unwrap();
        let math_op_1_dup = symbols.iter().find(|s| s.name == "math_op_1_dup").unwrap();
        assert_eq!(math_op_1.opcode_hash, math_op_1_dup.opcode_hash);
        assert_eq!(math_op_1.equiv_hash, math_op_1_dup.equiv_hash);
        assert_eq!(math_op_1.exact_hash, math_op_1_dup.exact_hash);
    }

    #[test]
    fn test_simple_mips_linked() {
        let elf_data = include_bytes!("../../../test/simple_mips_linked.o").to_vec();
        let symbols = read_elf(Platform::N64, &None, &elf_data).unwrap();
        assert!(!symbols.is_empty());

        let tf1 = symbols.iter().find(|s| s.name == "test_1").unwrap();
        let tf2 = symbols.iter().find(|s| s.name == "test_2").unwrap();
        let tf3 = symbols.iter().find(|s| s.name == "test_3").unwrap();

        assert_eq!(tf1.opcode_hash, tf2.opcode_hash);
        // TODO need to figure out what to do when we have no relocations
        //assert_eq!(tf1.equiv_hash, tf2.equiv_hash);
        assert_ne!(tf1.exact_hash, tf2.exact_hash);

        assert_eq!(tf1.opcode_hash, tf3.opcode_hash);
        assert_ne!(tf1.equiv_hash, tf3.equiv_hash);
        assert_ne!(tf1.exact_hash, tf3.exact_hash);

        let math_op_1 = symbols.iter().find(|s| s.name == "math_op_1").unwrap();
        let math_op_1_dup = symbols.iter().find(|s| s.name == "math_op_1_dup").unwrap();
        assert_eq!(math_op_1.opcode_hash, math_op_1_dup.opcode_hash);
        assert_eq!(math_op_1.equiv_hash, math_op_1_dup.equiv_hash);
        assert_eq!(math_op_1.exact_hash, math_op_1_dup.exact_hash);
    }

    #[test]
    fn test_simple_mips_map() {
        let rom_bytes = include_bytes!("../../../test/simple_mips_raw.bin").to_vec();
        let map_str = include_str!("../../../test/simple_mips.map");
        let symbols = read_map(Platform::N64, None, rom_bytes, map_str).unwrap();
        assert!(!symbols.is_empty());

        let tf1 = symbols.iter().find(|s| s.name == "test_1").unwrap();
        let tf2 = symbols.iter().find(|s| s.name == "test_2").unwrap();
        let tf3 = symbols.iter().find(|s| s.name == "test_3").unwrap();

        assert_eq!(tf1.opcode_hash, tf2.opcode_hash);
        // TODO need to figure out what to do when we have no relocations
        //assert_eq!(tf1.equiv_hash, tf2.equiv_hash);
        assert_ne!(tf1.exact_hash, tf2.exact_hash);

        assert_eq!(tf1.opcode_hash, tf3.opcode_hash);
        assert_ne!(tf1.equiv_hash, tf3.equiv_hash);
        assert_ne!(tf1.exact_hash, tf3.exact_hash);

        let math_op_1 = symbols.iter().find(|s| s.name == "math_op_1").unwrap();
        let math_op_1_dup = symbols.iter().find(|s| s.name == "math_op_1_dup").unwrap();
        assert_eq!(math_op_1.opcode_hash, math_op_1_dup.opcode_hash);
        assert_eq!(math_op_1.equiv_hash, math_op_1_dup.equiv_hash);
        assert_eq!(math_op_1.exact_hash, math_op_1_dup.exact_hash);
    }

    #[test]
    fn test_simple_ppc() {
        let elf_data = include_bytes!("../../../test/simple_ppc.o").to_vec();
        let symbols = read_elf(Platform::Gc, &None, &elf_data).unwrap();
        assert!(!symbols.is_empty());

        let tf1 = symbols.iter().find(|s| s.name == "test_1").unwrap();
        let tf2 = symbols.iter().find(|s| s.name == "test_2").unwrap();
        let tf3 = symbols.iter().find(|s| s.name == "test_3").unwrap();

        assert_eq!(tf1.opcode_hash, tf2.opcode_hash);
        assert_eq!(tf1.equiv_hash, tf2.equiv_hash);
        assert_eq!(tf1.exact_hash, tf2.exact_hash);

        assert_eq!(tf1.opcode_hash, tf3.opcode_hash);
        assert_ne!(tf1.equiv_hash, tf3.equiv_hash);
        assert_eq!(tf1.exact_hash, tf3.exact_hash);

        let math_op_1 = symbols.iter().find(|s| s.name == "math_op_1").unwrap();
        let math_op_1_dup = symbols.iter().find(|s| s.name == "math_op_1_dup").unwrap();
        assert_eq!(math_op_1.opcode_hash, math_op_1_dup.opcode_hash);
        assert_eq!(math_op_1.equiv_hash, math_op_1_dup.equiv_hash);
        assert_eq!(math_op_1.exact_hash, math_op_1_dup.exact_hash);
    }

    #[test]
    fn test_simple_ppc_linked() {
        let elf_data = include_bytes!("../../../test/simple_ppc_linked.o").to_vec();
        let symbols = read_elf(Platform::Gc, &None, &elf_data).unwrap();
        assert!(!symbols.is_empty());

        let tf1 = symbols.iter().find(|s| s.name == "test_1").unwrap();
        let tf2 = symbols.iter().find(|s| s.name == "test_2").unwrap();
        let tf3 = symbols.iter().find(|s| s.name == "test_3").unwrap();

        assert_eq!(tf1.opcode_hash, tf2.opcode_hash);
        assert_eq!(tf1.equiv_hash, tf2.equiv_hash);
        assert_eq!(tf1.exact_hash, tf2.exact_hash);

        assert_eq!(tf1.opcode_hash, tf3.opcode_hash);
        assert_ne!(tf1.equiv_hash, tf3.equiv_hash);
        assert_eq!(tf1.exact_hash, tf3.exact_hash);

        let math_op_1 = symbols.iter().find(|s| s.name == "math_op_1").unwrap();
        let math_op_1_dup = symbols.iter().find(|s| s.name == "math_op_1_dup").unwrap();
        assert_eq!(math_op_1.opcode_hash, math_op_1_dup.opcode_hash);
        assert_eq!(math_op_1.equiv_hash, math_op_1_dup.equiv_hash);
        assert_eq!(math_op_1.exact_hash, math_op_1_dup.exact_hash);
    }
}

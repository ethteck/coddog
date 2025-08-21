use std::collections::BTreeMap;

use crate::{Platform, Symbol, SymbolDef};
use anyhow::{Result, anyhow};
use mapfile_parser::MapFile;
use objdiff_core::diff::DiffObjConfig;
use objdiff_core::obj::SymbolFlag;

pub fn read_elf(
    platform: Platform,
    unmatched_funcs: &Option<Vec<String>>,
    elf_data: &[u8],
) -> Result<Vec<Symbol>> {
    let config = DiffObjConfig::default();

    let objdiff_obj = objdiff_core::obj::read::parse(elf_data, &config)
        .map_err(|e| anyhow!("Failed to parse ELF object: {}", e))?;

    let symbols = objdiff_obj
        .symbols
        .iter()
        .filter(|s| {
            s.size > 0
                && s.section.is_some() // not extern
                && s.kind == objdiff_core::obj::SymbolKind::Function
                && !s.flags.contains(SymbolFlag::Hidden)
                && !s.flags.contains(SymbolFlag::Ignored)
        })
        .cloned()
        .collect::<Vec<_>>();

    let ret: Vec<Symbol> = symbols
        .iter()
        .enumerate()
        .filter_map(|(idx, symbol)| {
            let section_index = symbol
                .section
                .ok_or_else(|| anyhow!("Missing section for symbol"));
            if let Err(e) = section_index {
                eprintln!(
                    "Error getting section index for symbol {}: {}",
                    symbol.name, e
                );
                return None; // Skip this symbol if section is missing
            }
            let section_index = section_index.unwrap();

            let section = &objdiff_obj.sections[section_index];

            // Get symbol data from the section
            let data = section
                .data_range(symbol.address, symbol.size as usize)
                .ok_or_else(|| {
                    anyhow!(
                        "Symbol data out of bounds: {:#x}..{:#x}",
                        symbol.address,
                        symbol.address + symbol.size
                    )
                });

            if let Err(e) = data {
                eprintln!("Error getting symbol data: {}", e);
                return None; // Skip this symbol if data is out of bounds
            }
            let data = data.unwrap();

            let sect_relocations = section
                .relocations
                .iter()
                .map(|r| (r.address, r.clone()))
                .collect();

            Some(Symbol::new(
                SymbolDef {
                    name: symbol.name.clone(),
                    bytes: data.to_vec(),
                    vram: symbol.address as usize,
                    is_decompiled: unmatched_funcs
                        .as_ref()
                        .is_none_or(|fs| !fs.contains(&symbol.name)),
                    platform,
                    symbol_idx: idx,
                },
                &sect_relocations,
            ))
        })
        .collect();

    Ok(ret)
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

            Symbol::new(
                SymbolDef {
                    name: x.name.clone(),
                    bytes: raw.to_vec(),
                    vram: x.vram as usize,
                    is_decompiled: unmatched_funcs
                        .as_ref()
                        .is_some_and(|fs| !fs.contains(&x.name)),
                    platform,
                    symbol_idx: 0, // No symbol index in plain binaries
                },
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

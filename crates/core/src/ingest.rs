use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{OBJDIFF_CONFIG, Platform, Symbol, arch};
use anyhow::{Result, anyhow};
use mapfile_parser::MapFile;
use objdiff_core::obj::{ResolvedSymbol, SymbolFlag};

pub fn read_elf(
    platform: Platform,
    unmatched_funcs: &Option<Vec<String>>,
    elf_data: &[u8],
) -> Result<Vec<Symbol>> {
    let objdiff_obj = objdiff_core::obj::read::parse(elf_data, &OBJDIFF_CONFIG)
        .map_err(|e| anyhow!("Failed to parse ELF object: {}", e))?;

    let symbols = objdiff_obj
        .symbols
        .iter()
        .enumerate()
        .filter_map(|(idx, s)| {
            if s.size > 0
                && s.section.is_some() // not extern
                && s.kind == objdiff_core::obj::SymbolKind::Function
                && !s.flags.contains(SymbolFlag::Hidden)
                && !s.flags.contains(SymbolFlag::Ignored)
            {
                Some((idx, s))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let ret: Vec<Symbol> = symbols
        .iter()
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
            let bytes: Vec<u8> = data.unwrap().to_vec();

            let insn_refs = objdiff_obj
                .arch
                .scan_instructions(
                    ResolvedSymbol {
                        obj: &objdiff_obj,
                        symbol_index: *idx,
                        symbol,
                        section_index,
                        section,
                        data: &bytes,
                    },
                    &OBJDIFF_CONFIG,
                )
                .unwrap();

            let vram = symbol.address as usize;

            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            let exact_hash = hasher.finish();

            let equiv_hash =
                arch::get_equivalence_hash(&bytes, platform, &objdiff_obj, section, &insn_refs);

            let opcodes: Vec<u16> = insn_refs.iter().map(|r| r.opcode).collect();
            let mut hasher = DefaultHasher::new();
            opcodes.hash(&mut hasher);
            let opcode_hash = hasher.finish();

            Some(Symbol {
                name: symbol.name.clone(),
                bytes,
                opcodes,
                vram,
                is_decompiled: unmatched_funcs
                    .as_ref()
                    .is_none_or(|fs| !fs.contains(&symbol.name)),
                exact_hash,
                equiv_hash,
                opcode_hash,
                symbol_idx: *idx,
            })
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
        .enumerate()
        .map(|(symbol_idx, x)| {
            let start = x.vrom.unwrap() as usize;
            let end = start + x.size as usize;
            let raw = &rom_bytes[start..end];
            let vram = x.vram as usize;

            let mut bytes = raw.to_vec();

            let insn_length = platform.arch().standard_insn_length();

            // trim trailing nops
            while bytes.len() >= insn_length
                && bytes[bytes.len() - insn_length..] == vec![0; insn_length]
            {
                bytes.truncate(bytes.len() - insn_length);
            }
            let opcodes: Vec<u16> = arch::get_opcodes_raw(&bytes, platform);

            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            let exact_hash = hasher.finish();

            let equiv_hash = arch::get_equivalence_hash_raw(&bytes, vram, platform);

            let mut hasher = DefaultHasher::new();
            opcodes.hash(&mut hasher);
            let opcode_hash = hasher.finish();

            Symbol {
                name: x.name.clone(),
                bytes,
                opcodes,
                vram,
                is_decompiled: unmatched_funcs
                    .as_ref()
                    .is_some_and(|fs| !fs.contains(&x.name)),
                exact_hash,
                equiv_hash,
                opcode_hash,
                symbol_idx,
            }
        })
        .collect();
    Ok(ret)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;

    #[test]
    fn test_simple_mips() {
        let d: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let elf_data = fs::read(d.join("../../test/simple_mips.o")).unwrap();
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
        let d: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let elf_data = fs::read(d.join("../../test/simple_mips_linked.o")).unwrap();
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
        let d: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let rom_bytes = fs::read(d.join("../../test/simple_mips_raw.bin")).unwrap();
        let map_str = fs::read_to_string(d.join("../../test/simple_mips.map")).unwrap();
        let symbols = read_map(Platform::N64, None, rom_bytes, &map_str).unwrap();
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
        let d: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let elf_data = fs::read(d.join("../../test/simple_ppc.o")).unwrap();
        let symbols = read_elf(Platform::GcWii, &None, &elf_data).unwrap();
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
        let d: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let elf_data = fs::read(d.join("../../test/simple_ppc_linked.o")).unwrap();
        let symbols = read_elf(Platform::GcWii, &None, &elf_data).unwrap();
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
    fn test_simple_gba() {
        let d: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let elf_data = fs::read(d.join("../../test/simple_gba.o")).unwrap();
        let symbols = read_elf(Platform::Gba, &None, &elf_data).unwrap();
        assert!(!symbols.is_empty());

        let tf1 = symbols.iter().find(|s| s.name == "test_1").unwrap();
        let tf2 = symbols.iter().find(|s| s.name == "test_2").unwrap();
        let tf3 = symbols.iter().find(|s| s.name == "test_3").unwrap();

        assert_eq!(tf1.opcode_hash, tf2.opcode_hash);
        assert_eq!(tf1.equiv_hash, tf2.equiv_hash);
        assert_ne!(tf1.exact_hash, tf2.exact_hash); // has data inside the code, so the exact hash differs

        assert_eq!(tf1.opcode_hash, tf3.opcode_hash);
        assert_ne!(tf1.equiv_hash, tf3.equiv_hash);
        assert_ne!(tf1.exact_hash, tf3.exact_hash); // has data inside the code, so the exact hash differs

        let math_op_1 = symbols.iter().find(|s| s.name == "math_op_1").unwrap();
        let math_op_1_dup = symbols.iter().find(|s| s.name == "math_op_1_dup").unwrap();
        assert_eq!(math_op_1.opcode_hash, math_op_1_dup.opcode_hash);
        assert_eq!(math_op_1.equiv_hash, math_op_1_dup.equiv_hash);
        assert_eq!(math_op_1.exact_hash, math_op_1_dup.exact_hash);
    }
}

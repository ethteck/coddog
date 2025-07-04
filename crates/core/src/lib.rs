pub mod arch;
pub mod cluster;
pub mod ingest;

use crate::arch::get_opcodes;
use crate::ingest::CoddogRel;
use anyhow::Result;
use editdistancek::edit_distance_bounded;
use objdiff_core::diff::DiffObjConfig;
use objdiff_core::diff::display::DiffText;
use object::Endianness;
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Arch {
    Mips,
    Ppc,
}

impl Arch {
    pub fn insn_length(&self) -> usize {
        match self {
            Arch::Mips => 4,
            Arch::Ppc => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Platform {
    N64,
    Psx,
    Ps2,
    Gc,
    Wii,
}

impl Platform {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "n64" => Some(Platform::N64),
            "psx" => Some(Platform::Psx),
            "ps2" => Some(Platform::Ps2),
            "gc" => Some(Platform::Gc),
            "wii" => Some(Platform::Wii),
            _ => None,
        }
    }

    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Platform::N64),
            1 => Some(Platform::Psx),
            2 => Some(Platform::Ps2),
            3 => Some(Platform::Gc),
            4 => Some(Platform::Wii),
            _ => None,
        }
    }

    pub fn endianness(&self) -> Endianness {
        match self {
            Platform::N64 => Endianness::Big,
            Platform::Psx => Endianness::Little,
            Platform::Ps2 => Endianness::Little,
            Platform::Gc => Endianness::Big,
            Platform::Wii => Endianness::Big,
        }
    }

    pub fn arch(&self) -> Arch {
        match self {
            Platform::N64 => Arch::Mips,
            Platform::Psx => Arch::Mips,
            Platform::Ps2 => Arch::Mips,
            Platform::Gc => Arch::Ppc,
            Platform::Wii => Arch::Ppc,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    /// the name of the symbol
    pub name: String,
    /// the raw bytes of the symbol
    pub bytes: Vec<u8>,
    /// the symbol's opcodes
    pub opcodes: Vec<u16>,
    /// the symbol's memory address
    pub vram: usize,
    /// the file offset of the symbol
    pub offset: usize,
    /// whether the symbol is decompiled
    pub is_decompiled: bool,
    /// the opcode hash for the symbol
    pub opcode_hash: u64,
    /// the equivalent hash for the symbol
    pub equiv_hash: u64,
    /// the exact hash for the symbol
    pub exact_hash: u64,
    /// the symbol_idx of the symbol in the object
    pub symbol_idx: usize,
}

#[derive(Debug)]
pub struct Binary {
    pub name: String,
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone, Copy)]
pub struct InsnSeqMatch {
    pub offset1: usize,
    pub offset2: usize,
    pub length: usize,
}

pub struct SymbolDef {
    name: String,
    bytes: Vec<u8>,
    vram: usize,
    offset: usize,
    is_decompiled: bool,
    platform: Platform,
    relocations: BTreeMap<u64, CoddogRel>,
    symbol_idx: usize,
}

impl Symbol {
    pub fn new(def: SymbolDef) -> Symbol {
        let mut bytes = def.bytes;

        let insn_length = def.platform.arch().insn_length();
        while bytes.len() >= insn_length
            && bytes[bytes.len() - insn_length..] == vec![0; insn_length]
        {
            bytes.truncate(bytes.len() - insn_length);
        }

        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let exact_hash = hasher.finish();

        let equiv_hash =
            arch::get_equivalence_hash(&bytes, def.vram, def.platform, &def.relocations);

        let opcodes = get_opcodes(&bytes, def.platform);
        let mut hasher = DefaultHasher::new();
        opcodes.hash(&mut hasher);
        let opcode_hash = hasher.finish();

        Symbol {
            name: def.name,
            bytes,
            opcodes,
            vram: def.vram,
            offset: def.offset,
            is_decompiled: def.is_decompiled,
            exact_hash,
            equiv_hash,
            opcode_hash,
            symbol_idx: def.symbol_idx,
        }
    }

    pub fn get_exact_hashes(&self, window_size: usize) -> Vec<u64> {
        get_hashes(&self.bytes, window_size)
    }

    pub fn get_opcode_hashes(&self, window_size: usize) -> Vec<u64> {
        get_hashes(&self.opcodes, window_size)
    }
}
pub fn get_hashes<T: Clone + Default + Hash>(data: &[T], window_size: usize) -> Vec<u64> {
    let mut data = data.to_vec();

    if data.len() < window_size {
        data.resize(window_size, Default::default());
    }

    data.windows(window_size)
        .map(|x| {
            let mut hasher = DefaultHasher::new();
            (*x).hash(&mut hasher);
            hasher.finish()
        })
        .collect()
}

pub fn get_submatches(hashes_1: &[u64], hashes_2: &[u64], window_size: usize) -> Vec<InsnSeqMatch> {
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

pub fn diff_symbols(sym1: &Symbol, sym2: &Symbol, threshold: f32) -> f32 {
    // The minimum edit distance for two strings of different lengths is `abs(l1 - l2)`
    // Quickly check if it's possible to beat the threshold. If it isn't, return 0
    let l1 = sym1.opcodes.len();
    let l2 = sym2.opcodes.len();

    let max_edit_dist = (l1 + l2) as f32;
    if (l1.abs_diff(l2) as f32 / max_edit_dist) > (1.0 - threshold) {
        return 0.0;
    }

    let sym1_insns_u8: Vec<u8> = sym1.opcodes.iter().flat_map(|&x| x.to_be_bytes()).collect();
    let sym2_insns_u8: Vec<u8> = sym2.opcodes.iter().flat_map(|&x| x.to_be_bytes()).collect();

    let bound = (max_edit_dist - (max_edit_dist * threshold)) as usize;
    if let Some(edit_distance) = edit_distance_bounded(&sym1_insns_u8, &sym2_insns_u8, bound) {
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

pub fn get_asm_for_symbol(object_path: &str, symbol_idx: i32) -> Result<Vec<String>> {
    let object_bytes = std::fs::read(object_path).expect("Failed to read object file");

    let diff_config = DiffObjConfig {
        analyze_data_flow: false,
        ppc_calculate_pool_relocations: false,
        ..Default::default()
    };
    let object = objdiff_core::obj::read::parse(&object_bytes, &diff_config)?;

    let diff = objdiff_core::diff::code::no_diff_code(&object, symbol_idx as usize, &diff_config)?;

    let mut ret = Vec::new();

    for row in &diff.instruction_rows {
        let mut line = String::new();

        objdiff_core::diff::display::display_row(
            &object,
            symbol_idx as usize,
            row,
            &diff_config,
            |segment| {
                match segment.text {
                    DiffText::Eol => {
                        ret.push(line.clone());
                        line = String::new();
                        return Ok(());
                    }
                    DiffText::Basic(s) => line.push_str(s),
                    DiffText::Line(_) => {}
                    DiffText::Address(_) => {}
                    DiffText::Opcode(m, _) => line.push_str(format!("{m:} ").as_str()),
                    DiffText::Argument(a) => line.push_str(&a.to_string()),
                    DiffText::BranchDest(d) => line.push_str(&d.to_string()),
                    DiffText::Symbol(s) => {
                        line.push_str(&s.demangled_name.clone().unwrap_or(s.name.clone()))
                    }
                    DiffText::Addend(a) => line.push_str(&a.to_string()),
                    DiffText::Spacing(s) => {
                        line.push_str(&std::iter::repeat_n(" ", s as usize).collect::<String>())
                    }
                }
                Ok(())
            },
        )?;
    }

    Ok(ret)
}

// #[cfg(test)]
// mod tests {
//     use crate::*;
//     #[test]
//     fn test_get_asm() {
//         let path = "/home/ethteck/repos/papermario/ver/us/build/papermario.elf";
//         let asm = get_asm_for_symbol(path, 469993).unwrap();
//         for line in asm {
//             println!("{}", line);
//         }
//     }
// }

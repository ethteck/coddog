pub mod arch;
pub mod cluster;
pub mod ingest;

use crate::arch::get_opcodes;
use anyhow::Result;
use editdistancek::edit_distance_bounded;
use objdiff_core::diff::DiffObjConfig;
use objdiff_core::diff::display::DiffText;
use objdiff_core::obj::Relocation;
use object::Endianness;
use serde::Serialize;
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Arch {
    Mips,
    Ppc,
    Aarch64,
}

impl Arch {
    pub fn insn_length(&self) -> usize {
        match self {
            Arch::Mips => 4,
            Arch::Ppc => 4,
            Arch::Aarch64 => 4,
        }
    }
}

// thanks https://stackoverflow.com/a/57578431
macro_rules! back_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl std::convert::TryFrom<i32> for $name {
            type Error = ();

            fn try_from(v: i32) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as i32 => Ok($name::$vname),)*
                    _ => Err(()),
                }
            }
        }
    }
}

back_to_enum! {
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Platform {
    N64,
    Psx,
    Ps2,
    GcWii,
    Psp,
    //Switch,
}
}

impl Platform {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "n64" => Some(Platform::N64),
            "psx" => Some(Platform::Psx),
            "ps2" => Some(Platform::Ps2),
            "gc_wii" => Some(Platform::GcWii),
            "psp" => Some(Platform::Psp),
            //"switch" => Some(Platform::Switch),
            _ => None,
        }
    }

    pub fn from_decompme_name(name: &str) -> Option<Self> {
        match name {
            "n64" => Some(Platform::N64),
            "ps1" => Some(Platform::Psx),
            "ps2" => Some(Platform::Ps2),
            "psp" => Some(Platform::Psp),
            "gc_wii" => Some(Platform::GcWii),
            "gba" => None,      // TODO: needs arm support
            "nds_arm9" => None, // TODO: needs arm support
            "n3ds" => None,     // TODO: needs arm support
            "irix" => None,     // TODO: not sure
            "switch" => None,   //"switch" => Some(Platform::Switch),
            "win32" => None,    // :frull:
            "msdos" => None,    // :frull:
            "saturn" => None,   // TODO: needs sh2 support
            "macosx" => None,   // :frull:
            "macos9" => None,   // :frull:
            _ => None,
        }
    }

    pub fn endianness(&self) -> Endianness {
        match self {
            Platform::N64 => Endianness::Big,
            Platform::Psx => Endianness::Little,
            Platform::Ps2 => Endianness::Little,
            Platform::GcWii => Endianness::Big,
            Platform::Psp => Endianness::Little,
            //Platform::Switch => Endianness::Little,
        }
    }

    pub fn arch(&self) -> Arch {
        match self {
            Platform::N64 => Arch::Mips,
            Platform::Psx => Arch::Mips,
            Platform::Ps2 => Arch::Mips,
            Platform::GcWii => Arch::Ppc,
            Platform::Psp => Arch::Mips,
            //Platform::Switch => Arch::Aarch64,
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
    is_decompiled: bool,
    platform: Platform,
    symbol_idx: usize,
}

impl Symbol {
    pub fn new(def: SymbolDef, relocations: &BTreeMap<u64, Relocation>) -> Symbol {
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

        let equiv_hash = arch::get_equivalence_hash(&bytes, def.vram, def.platform, relocations);

        let opcodes = get_opcodes(&bytes, def.platform);
        let mut hasher = DefaultHasher::new();
        opcodes.hash(&mut hasher);
        let opcode_hash = hasher.finish();

        Symbol {
            name: def.name,
            bytes,
            opcodes,
            vram: def.vram,
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

#[derive(Debug, Clone, Serialize)]
pub struct AsmInsn {
    pub opcode: String,
    pub address: Option<String>,
    pub arguments: Vec<String>,
    pub branch_dest: Option<String>,
    pub symbol: Option<String>,
    pub addend: Option<String>,
}

impl AsmInsn {
    fn new(opcode: String) -> Self {
        Self {
            opcode,
            address: None,
            arguments: Vec::new(),
            branch_dest: None,
            symbol: None,
            addend: None,
        }
    }
}

pub fn get_asm_for_symbol(object_path: &str, symbol_idx: i32) -> Result<Vec<AsmInsn>> {
    let object_bytes = std::fs::read(object_path)
        .map_err(|e| anyhow::anyhow!("Failed to read object file at {}: {}", object_path, e))?;

    let diff_config = DiffObjConfig {
        analyze_data_flow: false,
        ppc_calculate_pool_relocations: false,
        ..Default::default()
    };
    let object = objdiff_core::obj::read::parse(&object_bytes, &diff_config)?;

    let diff = objdiff_core::diff::code::no_diff_code(&object, symbol_idx as usize, &diff_config)?;

    let mut ret = Vec::new();
    let mut current_insn: Option<AsmInsn> = None;

    for row in &diff.instruction_rows {
        objdiff_core::diff::display::display_row(
            &object,
            symbol_idx as usize,
            row,
            &diff_config,
            |segment| {
                match segment.text {
                    DiffText::Eol => {
                        if let Some(insn) = current_insn.take() {
                            ret.push(insn);
                        }
                        return Ok(());
                    }
                    DiffText::Basic(_) | DiffText::Line(_) | DiffText::Spacing(_) => {
                        // Ignore these variants as requested
                    }
                    DiffText::Address(addr) => {
                        if let Some(ref mut insn) = current_insn {
                            insn.address = Some(addr.to_string());
                        }
                    }
                    DiffText::Opcode(opcode, _) => {
                        // End current instruction and start new one
                        if let Some(insn) = current_insn.take() {
                            ret.push(insn);
                        }
                        current_insn = Some(AsmInsn::new(opcode.to_string()));
                    }
                    DiffText::Argument(arg) => {
                        if let Some(ref mut insn) = current_insn {
                            let arg = arg.to_string();
                            if !insn.arguments.is_empty() && arg.to_lowercase() == "sp" {
                                if let Some(mut last_arg) = insn.arguments.pop() {
                                    last_arg.push_str("(sp)");
                                    insn.arguments.push(last_arg);
                                }
                            } else {
                                insn.arguments.push(arg);
                            }
                        }
                    }
                    DiffText::BranchDest(dest) => {
                        if let Some(ref mut insn) = current_insn {
                            insn.branch_dest = Some(format!("{dest:X}"));
                        }
                    }
                    DiffText::Symbol(sym) => {
                        if let Some(ref mut insn) = current_insn {
                            insn.symbol =
                                Some(sym.demangled_name.clone().unwrap_or(sym.name.clone()));
                        }
                    }
                    DiffText::Addend(add) => {
                        if let Some(ref mut insn) = current_insn {
                            insn.addend = Some(add.to_string());
                        }
                    }
                }
                Ok(())
            },
        )?;
    }

    // Push any remaining instruction
    if let Some(insn) = current_insn {
        ret.push(insn);
    }

    if ret.is_empty() {
        return Err(anyhow::anyhow!("No assembly found for symbol"));
    }

    Ok(ret)
}

pub mod cluster;
pub mod ingest;
pub mod instructions;

use std::hash::{DefaultHasher, Hash, Hasher};

use editdistancek::edit_distance_bounded;
use object::Endianness;

#[derive(Debug, Clone, Copy)]
pub enum Arch {
    Unknown,
    Mips,
    Ppc,
}

#[derive(Debug, Clone, Copy)]
pub enum Platform {
    N64,
    Psx,
    Ps2,
    Gc,
    Wii,
}

impl Platform {
    pub fn of(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "n64" => Some(Platform::N64),
            "psx" => Some(Platform::Psx),
            "ps2" => Some(Platform::Ps2),
            "gc" => Some(Platform::Gc),
            "wii" => Some(Platform::Wii),
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

#[derive(Debug, PartialEq, Eq)]
pub struct Symbol {
    /// internal id for the symbol
    pub id: usize,
    /// the name of the symbol
    pub name: String,
    /// the raw bytes of the symbol
    pub bytes: Vec<u8>,
    /// the symbol's instructions, normalized to essentially just opcodes
    pub insns: Vec<u16>,
    /// the file offset of the symbol
    pub offset: usize,
    /// whether the symbol is decompiled
    pub is_decompiled: bool,
    /// the exact hash for the symbol
    pub exact_hash: u64,
    /// the fuzzy hash for the symbol
    pub fuzzy_hash: u64,
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

impl Symbol {
    pub fn new(
        id: usize,
        name: String,
        bytes: Vec<u8>,
        insns: Vec<u16>,
        offset: usize,
        is_decompiled: bool,
    ) -> Symbol {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let exact_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        insns.hash(&mut hasher);
        let fuzzy_hash = hasher.finish();

        Symbol {
            id,
            name,
            bytes,
            insns,
            offset,
            is_decompiled,
            exact_hash,
            fuzzy_hash,
        }
    }

    pub fn get_exact_hashes(&self, window_size: usize) -> Vec<u64> {
        self.bytes
            .windows(window_size)
            .map(|x| {
                let mut hasher = DefaultHasher::new();
                (*x).hash(&mut hasher);
                hasher.finish()
            })
            .collect()
    }

    pub fn get_fuzzy_hashes(&self, window_size: usize) -> Vec<u64> {
        self.insns
            .windows(window_size)
            .map(|x| {
                let mut hasher = DefaultHasher::new();
                (*x).hash(&mut hasher);
                hasher.finish()
            })
            .collect()
    }
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
    let l1 = sym1.insns.len();
    let l2 = sym2.insns.len();

    let max_edit_dist = (l1 + l2) as f32;
    if (l1.abs_diff(l2) as f32 / max_edit_dist) > (1.0 - threshold) {
        return 0.0;
    }

    let sym1_insns_u8: Vec<u8> = sym1.insns.iter().flat_map(|&x| x.to_be_bytes()).collect();
    let sym2_insns_u8: Vec<u8> = sym2.insns.iter().flat_map(|&x| x.to_be_bytes()).collect();

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

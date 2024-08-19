use crate::*;

#[derive(Debug, Clone, Copy)]
pub enum Endianness {
    Little,
    Big,
}

impl Endianness {
    pub fn from_platform(platform: &str) -> Self {
        match platform {
            "n64" => Endianness::Big,
            "ps2" => Endianness::Little,
            _ => panic!("Unknown platform {}", platform),
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
    pub insns: Vec<u8>,
    /// whether the symbol is decompiled
    pub is_decompiled: bool,
}

impl Symbol {
    pub fn cli_name(&self) -> String {
        format!(
            "{}{}",
            self.name.clone(),
            if self.is_decompiled {
                " (decompiled)".green()
            } else {
                "".normal()
            }
        )
    }

    pub fn cli_name_colored(&self, color: Color) -> String {
        format!(
            "{}{}",
            self.name.clone().color(color),
            if self.is_decompiled {
                " (decompiled)".green()
            } else {
                "".normal()
            }
        )
    }
}

#[derive(Debug)]
pub struct Binary {
    pub symbols: Vec<Symbol>,
    pub cli_color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct InsnSeqMatch {
    pub offset1: usize,
    pub offset2: usize,
    pub length: usize,
}

pub fn get_hashes(bytes: &Symbol, window_size: usize) -> Vec<u64> {
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

    let bound = (max_edit_dist - (max_edit_dist * threshold)) as usize;
    if let Some(edit_distance) = edit_distance_bounded(&sym1.insns, &sym2.insns, bound) {
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

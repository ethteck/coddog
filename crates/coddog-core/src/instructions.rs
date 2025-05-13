use crate::Platform;
use object::{Endian, Endianness};
use ppc750cl::Opcode;

pub(crate) fn get_insns(bytes: &[u8], platform: Platform) -> Vec<u16> {
    let mut insns: Vec<u16> = match platform {
        Platform::N64 | Platform::Psx | Platform::Ps2 => bytes
            .iter()
            .skip(match platform.endianness() {
                Endianness::Little => 0,
                Endianness::Big => 3,
            })
            .step_by(4)
            .map(|x| (x >> 2) as u16)
            .collect(),
        Platform::Gc | Platform::Wii => bytes
            .chunks_exact(4)
            .map(|c| {
                Opcode::_detect(platform.endianness().read_u32_bytes(c.try_into().unwrap())) as u16
            })
            .collect(),
    };

    // Remove trailing nops
    while !insns.is_empty() && insns[insns.len() - 1] == 0 {
        insns.pop();
    }
    insns
}

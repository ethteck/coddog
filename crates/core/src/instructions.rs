use crate::Platform;
use object::Endian;
use ppc750cl::Opcode;
use rabbitizer;
use rabbitizer::IsaExtension::{R3000GTE, R5900EE};
use rabbitizer::IsaVersion::MIPS_III;
use rabbitizer::Vram;

pub(crate) fn get_insns(bytes: &[u8], platform: Platform) -> Vec<u16> {
    let mut insns: Vec<u16> = match platform {
        Platform::N64 | Platform::Psx | Platform::Ps2 => bytes
            .chunks_exact(4)
            .map(|chunk| {
                let code = platform
                    .endianness()
                    .read_u32_bytes(chunk.try_into().unwrap());
                let instruction = rabbitizer::Instruction::new(
                    code,
                    Vram::new(0),
                    match platform {
                        Platform::N64 => rabbitizer::InstructionFlags::new(MIPS_III),
                        Platform::Psx => rabbitizer::InstructionFlags::new_extension(R3000GTE),
                        Platform::Ps2 => rabbitizer::InstructionFlags::new_extension(R5900EE),
                        _ => unreachable!(),
                    },
                );
                instruction.opcode() as u16
            })
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

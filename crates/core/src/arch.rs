use crate::Platform;
use object::Endian;
use ppc750cl::Opcode;
use rabbitizer::IsaExtension::{R3000GTE, R5900EE};
use rabbitizer::IsaVersion::MIPS_III;
use rabbitizer::Vram;
use std::hash::{DefaultHasher, Hash, Hasher};

fn get_rabbitizer_instruction(word: u32, platform: Platform) -> rabbitizer::Instruction {
    rabbitizer::Instruction::new(
        word,
        Vram::new(0),
        match platform {
            Platform::N64 => rabbitizer::InstructionFlags::new(MIPS_III),
            Platform::Psx => rabbitizer::InstructionFlags::new_extension(R3000GTE),
            Platform::Ps2 => rabbitizer::InstructionFlags::new_extension(R5900EE),
            _ => unreachable!(),
        },
    )
}

pub(crate) fn get_opcodes(bytes: &[u8], platform: Platform) -> Vec<u16> {
    let mut insns: Vec<u16> = match platform {
        Platform::N64 | Platform::Psx | Platform::Ps2 => bytes
            .chunks_exact(4)
            .map(|chunk| {
                let code = platform
                    .endianness()
                    .read_u32_bytes(chunk.try_into().unwrap());
                let instruction = get_rabbitizer_instruction(code, platform);
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

pub(crate) fn get_equivalence_hash(bytes: &[u8], platform: Platform) -> u64 {
    let mut hasher = DefaultHasher::new();

    match platform {
        Platform::N64 | Platform::Psx | Platform::Ps2 => {
            for (i, chunk) in bytes.chunks_exact(4).enumerate() {
                let code = platform
                    .endianness()
                    .read_u32_bytes(chunk.try_into().unwrap());
                let instruction = get_rabbitizer_instruction(code, platform);

                // hash opcode
                instruction.opcode().hash(&mut hasher);

                // hash operands
                for a in instruction.operands_iter() {
                    // TODO only want to do the right ones
                    //a.hash(&mut hasher);
                }
            }
        }
        Platform::Gc | Platform::Wii => {
            for (i, chunk) in bytes.chunks_exact(4).enumerate() {
                let code = platform
                    .endianness()
                    .read_u32_bytes(chunk.try_into().unwrap());
                let instruction = ppc750cl::Ins::new(code);
                let opcode = instruction.op as u16;

                // hash opcode
                opcode.hash(&mut hasher);

                // hash operands
                for a in instruction.defs() {
                    // TODO only want to do the right ones
                    //a.hash(&mut hasher);
                }
            }
        }
    }

    hasher.finish()
}

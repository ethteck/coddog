use crate::{Arch, Platform};
use objdiff_core::obj::{InstructionRef, Object, Section};
use object::Endian;
use rabbitizer::IsaExtension::{R3000GTE, R4000ALLEGREX, R5900EE};
use rabbitizer::IsaVersion::MIPS_III;
use rabbitizer::operands::ValuedOperand;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

enum Insn {
    Mips(rabbitizer::Instruction),
    Ppc(powerpc::Ins),
    Thumb(unarm::thumb::Ins),
}

fn get_rabbitizer_instruction(word: u32, vram: u32, platform: Platform) -> rabbitizer::Instruction {
    rabbitizer::Instruction::new(
        word,
        rabbitizer::Vram::new(vram),
        match platform {
            Platform::N64 => rabbitizer::InstructionFlags::new(MIPS_III),
            Platform::Psx => rabbitizer::InstructionFlags::new_extension(R3000GTE),
            Platform::Ps2 => rabbitizer::InstructionFlags::new_extension(R5900EE),
            Platform::Psp => rabbitizer::InstructionFlags::new_extension(R4000ALLEGREX),
            _ => unreachable!(),
        },
    )
}

// Given raw bytes, attempt to get opcodes for the bytes
pub fn get_opcodes_raw(bytes: &[u8], platform: Platform) -> Vec<u16> {
    let insn_length = platform.arch().standard_insn_length();

    match platform.arch() {
        Arch::Mips => bytes
            .chunks_exact(insn_length)
            .map(|chunk| {
                let code = platform
                    .endianness()
                    .read_u32_bytes(chunk.try_into().unwrap());
                let instruction = get_rabbitizer_instruction(code, 0, platform);
                instruction.opcode() as u16
            })
            .collect(),
        Arch::Ppc => bytes
            .chunks_exact(insn_length)
            .map(|c| {
                powerpc::Opcode::detect(
                    platform.endianness().read_u32_bytes(c.try_into().unwrap()),
                    powerpc::Extensions::gekko_broadway(),
                ) as u16
            })
            .collect(),
        Arch::Thumb => bytes
            .chunks_exact(insn_length)
            .map(|chunk| {
                let code = platform
                    .endianness()
                    .read_u16_bytes(chunk.try_into().unwrap());
                let ins = unarm::thumb::Ins::new(
                    code as u32,
                    &unarm::ParseFlags {
                        ual: true,
                        version: platform.arm_version(),
                    },
                );
                ins.op as u16
            })
            .collect(),
    }
}

fn decode_instruction(
    insn_bytes: &[u8],
    platform: Platform,
    insn_ref: &InstructionRef,
) -> Result<Insn, anyhow::Error> {
    match platform.arch() {
        Arch::Mips => {
            let code = platform
                .endianness()
                .read_u32_bytes(insn_bytes.try_into().unwrap());

            Ok(Insn::Mips(get_rabbitizer_instruction(
                code,
                insn_ref.address as u32,
                platform,
            )))
        }
        Arch::Ppc => Ok(Insn::Ppc(powerpc::Ins::new(
            platform
                .endianness()
                .read_u32_bytes(insn_bytes.try_into().unwrap()),
            powerpc::Extensions::gekko_broadway(),
        ))),
        Arch::Thumb => match insn_ref.size {
            2 => Ok(Insn::Thumb(unarm::thumb::Ins::new(
                platform
                    .endianness()
                    .read_u16_bytes(insn_bytes.try_into().unwrap()) as u32,
                &unarm::ParseFlags {
                    ual: true,
                    version: platform.arm_version(),
                },
            ))),
            4 => Ok(Insn::Thumb(unarm::thumb::Ins::new(
                platform
                    .endianness()
                    .read_u32_bytes(insn_bytes.try_into().unwrap()),
                &unarm::ParseFlags {
                    ual: true,
                    version: platform.arm_version(),
                },
            ))),
            _ => Err(anyhow::anyhow!(
                "Unexpected instruction size {} for Thumb",
                insn_ref.size
            )),
        },
    }
}

pub(crate) fn get_equivalence_hash(
    bytes: &[u8],
    platform: Platform,
    object: &Object,
    section: &Section,
    insn_refs: &Vec<InstructionRef>,
) -> u64 {
    let mut hasher = DefaultHasher::new();

    let mut reloc_ids = HashMap::new();

    let mut hashed_reloc;

    let start_address = insn_refs.first().map(|r| r.address as usize).unwrap_or(0);

    for insn_ref in insn_refs {
        // Replace with constant when new objdiff is out
        if insn_ref.opcode == u16::MAX || insn_ref.opcode == u16::MAX - 1 {
            continue;
        }

        // Hash the unique id for the relocation entry rather than the specifics
        if let Some(reloc) = section.relocation_at(object, *insn_ref) {
            let next_id = reloc_ids.len();
            let hash_id = *reloc_ids
                .entry((
                    reloc.relocation.target_symbol,
                    reloc.relocation.addend,
                    reloc.relocation.flags,
                ))
                .or_insert(next_id);
            hash_id.hash(&mut hasher);
            hashed_reloc = true;
        } else {
            hashed_reloc = false;
        }

        let offset = insn_ref.address as usize - start_address;
        let insn_length = insn_ref.size as usize;
        let insn_bytes = &bytes[offset..offset + insn_length];

        let instruction = match decode_instruction(insn_bytes, platform, insn_ref) {
            Ok(insn) => insn,
            Err(_) => {
                eprintln!(
                    "Warning: Failed to read instruction at {:#X}",
                    insn_ref.address
                );
                continue;
            }
        };

        hash_args_for_insn(instruction, &mut hasher, hashed_reloc);
    }

    hasher.finish()
}

pub(crate) fn get_equivalence_hash_raw(bytes: &[u8], vram: usize, platform: Platform) -> u64 {
    let mut hasher: DefaultHasher = DefaultHasher::new();

    let insn_length = platform.arch().standard_insn_length();

    for (i, chunk) in bytes.chunks_exact(insn_length).enumerate() {
        let cur_vram = vram + i * insn_length;

        let insn = decode_instruction(
            chunk,
            platform,
            &InstructionRef {
                address: cur_vram as u64,
                size: insn_length as u8,
                opcode: 0,
                branch_dest: None,
            },
        );

        let insn = match insn {
            Ok(insn) => insn,
            Err(_) => {
                eprintln!("Warning: Failed to read instruction at {:#X}", cur_vram);
                continue;
            }
        };

        hash_args_for_insn(insn, &mut hasher, false);
    }

    hasher.finish()
}

fn hash_args_for_insn(insn: Insn, hasher: &mut DefaultHasher, hashed_reloc: bool) {
    match insn {
        Insn::Mips(insn) => hash_mips_args(insn, hasher, hashed_reloc),
        Insn::Ppc(insn) => hash_ppc_args(insn, hasher, hashed_reloc),
        Insn::Thumb(insn) => hash_thumb_args(insn, hasher, hashed_reloc),
    }
}

fn hash_mips_args(insn: rabbitizer::Instruction, hasher: &mut DefaultHasher, hashed_reloc: bool) {
    // hash opcode
    insn.opcode().hash(hasher);

    // hash operands
    for vo in insn.valued_operands_iter() {
        match vo {
            ValuedOperand::ALL_EMPTY() => vo.hash(hasher),
            ValuedOperand::core_rs(_) => vo.hash(hasher),
            ValuedOperand::core_rt(_) => vo.hash(hasher),
            ValuedOperand::core_rd(_) => vo.hash(hasher),
            ValuedOperand::core_sa(_) => vo.hash(hasher),
            ValuedOperand::core_zero() => vo.hash(hasher),
            ValuedOperand::core_cop0d(_) => vo.hash(hasher),
            ValuedOperand::core_cop0cd(_) => vo.hash(hasher),
            ValuedOperand::core_fs(_) => vo.hash(hasher),
            ValuedOperand::core_ft(_) => vo.hash(hasher),
            ValuedOperand::core_fd(_) => vo.hash(hasher),
            // ValuedOperand::core_cop1cs(_) => {}
            // ValuedOperand::core_cop2t(_) => {}
            // ValuedOperand::core_cop2d(_) => {}
            // ValuedOperand::core_cop2cd(_) => {}
            // ValuedOperand::core_op(_) => {}
            // ValuedOperand::core_hint(_) => {}
            // ValuedOperand::core_code(_, _) => {}
            // ValuedOperand::core_code_lower(_) => {}
            // ValuedOperand::core_copraw(_) => {}
            ValuedOperand::core_label(_) => {
                if !hashed_reloc {
                    vo.hash(hasher);
                }
            }
            ValuedOperand::core_imm_i16(_) => {
                if !hashed_reloc {
                    vo.hash(hasher);
                }
            }
            ValuedOperand::core_imm_u16(_) => {
                if !hashed_reloc {
                    vo.hash(hasher);
                }
            }
            ValuedOperand::core_branch_target_label(_) => {
                vo.hash(hasher);
            }
            ValuedOperand::core_imm_rs(_, gpr) => {
                if !hashed_reloc {
                    vo.hash(hasher);
                } else {
                    gpr.hash(hasher);
                }
            }
            // ValuedOperand::core_maybe_rd_rs(_, _) => {}
            // ValuedOperand::core_maybe_zero_rs(_, _) => {}
            // ValuedOperand::rsp_cop0d(_) => {}
            // ValuedOperand::rsp_cop2cd(_) => {}
            // ValuedOperand::rsp_vs(_) => {}
            // ValuedOperand::rsp_vd(_) => {}
            // ValuedOperand::rsp_vt_elementhigh(_, _) => {}
            // ValuedOperand::rsp_vt_elementlow(_, _) => {}
            // ValuedOperand::rsp_vd_de(_, _) => {}
            // ValuedOperand::rsp_vs_index(_, _) => {}
            // ValuedOperand::rsp_offset_rs(_, _) => {}
            // ValuedOperand::r3000gte_sf(_) => {}
            // ValuedOperand::r3000gte_mx(_) => {}
            // ValuedOperand::r3000gte_v(_) => {}
            // ValuedOperand::r3000gte_cv(_) => {}
            // ValuedOperand::r3000gte_lm(_) => {}
            // ValuedOperand::r4000allegrex_s_vs(_) => {}
            // ValuedOperand::r4000allegrex_s_vt(_) => {}
            // ValuedOperand::r4000allegrex_s_vd(_) => {}
            // ValuedOperand::r4000allegrex_s_vt_imm(_) => {}
            // ValuedOperand::r4000allegrex_s_vd_imm(_) => {}
            // ValuedOperand::r4000allegrex_p_vs(_) => {}
            // ValuedOperand::r4000allegrex_p_vt(_) => {}
            // ValuedOperand::r4000allegrex_p_vd(_) => {}
            // ValuedOperand::r4000allegrex_t_vs(_) => {}
            // ValuedOperand::r4000allegrex_t_vt(_) => {}
            // ValuedOperand::r4000allegrex_t_vd(_) => {}
            // ValuedOperand::r4000allegrex_q_vs(_) => {}
            // ValuedOperand::r4000allegrex_q_vt(_) => {}
            // ValuedOperand::r4000allegrex_q_vd(_) => {}
            // ValuedOperand::r4000allegrex_q_vt_imm(_) => {}
            // ValuedOperand::r4000allegrex_mp_vs(_) => {}
            // ValuedOperand::r4000allegrex_mp_vt(_) => {}
            // ValuedOperand::r4000allegrex_mp_vd(_) => {}
            // ValuedOperand::r4000allegrex_mp_vs_transpose(_) => {}
            // ValuedOperand::r4000allegrex_mt_vs(_) => {}
            // ValuedOperand::r4000allegrex_mt_vt(_) => {}
            // ValuedOperand::r4000allegrex_mt_vd(_) => {}
            // ValuedOperand::r4000allegrex_mt_vs_transpose(_) => {}
            // ValuedOperand::r4000allegrex_mq_vs(_) => {}
            // ValuedOperand::r4000allegrex_mq_vt(_) => {}
            // ValuedOperand::r4000allegrex_mq_vd(_) => {}
            // ValuedOperand::r4000allegrex_mq_vs_transpose(_) => {}
            // ValuedOperand::r4000allegrex_cop2cs(_) => {}
            // ValuedOperand::r4000allegrex_cop2cd(_) => {}
            // ValuedOperand::r4000allegrex_pos(_) => {}
            // ValuedOperand::r4000allegrex_size(_) => {}
            // ValuedOperand::r4000allegrex_size_plus_pos(_) => {}
            // ValuedOperand::r4000allegrex_imm3(_) => {}
            // ValuedOperand::r4000allegrex_offset14_base(_, _) => {}
            // ValuedOperand::r4000allegrex_offset14_base_maybe_wb(_, _, _) => {}
            // ValuedOperand::r4000allegrex_vcmp_cond_s_maybe_vs_maybe_vt(_, _, _) => {}
            // ValuedOperand::r4000allegrex_vcmp_cond_p_maybe_vs_maybe_vt(_, _, _) => {}
            // ValuedOperand::r4000allegrex_vcmp_cond_t_maybe_vs_maybe_vt(_, _, _) => {}
            // ValuedOperand::r4000allegrex_vcmp_cond_q_maybe_vs_maybe_vt(_, _, _) => {}
            // ValuedOperand::r4000allegrex_vconstant(_) => {}
            // ValuedOperand::r4000allegrex_power_of_two(_) => {}
            // ValuedOperand::r4000allegrex_vfpu_cc_bit(_) => {}
            // ValuedOperand::r4000allegrex_bn(_) => {}
            // ValuedOperand::r4000allegrex_int16(_) => {}
            // ValuedOperand::r4000allegrex_float16(_) => {}
            // ValuedOperand::r4000allegrex_p_vrot_code(_) => {}
            // ValuedOperand::r4000allegrex_t_vrot_code(_) => {}
            // ValuedOperand::r4000allegrex_q_vrot_code(_) => {}
            // ValuedOperand::r4000allegrex_wpx(_) => {}
            // ValuedOperand::r4000allegrex_wpy(_) => {}
            // ValuedOperand::r4000allegrex_wpz(_) => {}
            // ValuedOperand::r4000allegrex_wpw(_) => {}
            // ValuedOperand::r4000allegrex_rpx(_) => {}
            // ValuedOperand::r4000allegrex_rpy(_) => {}
            // ValuedOperand::r4000allegrex_rpz(_) => {}
            // ValuedOperand::r4000allegrex_rpw(_) => {}
            // ValuedOperand::r5900ee_I() => {}
            // ValuedOperand::r5900ee_Q() => {}
            // ValuedOperand::r5900ee_R() => {}
            // ValuedOperand::r5900ee_ACC() => {}
            // ValuedOperand::r5900ee_immediate5(_) => {}
            // ValuedOperand::r5900ee_immediate15(_) => {}
            // ValuedOperand::r5900ee_vfs(_) => {}
            // ValuedOperand::r5900ee_vft(_) => {}
            // ValuedOperand::r5900ee_vfd(_) => {}
            // ValuedOperand::r5900ee_vis(_) => {}
            // ValuedOperand::r5900ee_vit(_) => {}
            // ValuedOperand::r5900ee_vid(_) => {}
            // ValuedOperand::r5900ee_ACCxyzw(_, _, _, _) => {}
            // ValuedOperand::r5900ee_vfsxyzw(_, _, _, _, _) => {}
            // ValuedOperand::r5900ee_vftxyzw(_, _, _, _, _) => {}
            // ValuedOperand::r5900ee_vfdxyzw(_, _, _, _, _) => {}
            // ValuedOperand::r5900ee_vftn(_, _) => {}
            // ValuedOperand::r5900ee_vfsl(_, _) => {}
            // ValuedOperand::r5900ee_vftm(_, _) => {}
            // ValuedOperand::r5900ee_vis_predecr(_, _) => {}
            // ValuedOperand::r5900ee_vit_predecr(_, _) => {}
            // ValuedOperand::r5900ee_vis_postincr(_, _) => {}
            // ValuedOperand::r5900ee_vit_postincr(_, _) => {}
            // ValuedOperand::r5900ee_vis_parenthesis(_) => {}
            _ => vo.hash(hasher),
        }
    }
}

fn hash_ppc_args(insn: powerpc::Ins, hasher: &mut DefaultHasher, hashed_reloc: bool) {
    // hash opcode
    insn.op.hash(hasher);

    // hash operands
    for a in insn.basic().args {
        match a {
            powerpc::Argument::None => {}
            powerpc::Argument::Simm(_)
            | powerpc::Argument::Uimm(_)
            | powerpc::Argument::Offset(_)
            | powerpc::Argument::BranchDest(_)
            | powerpc::Argument::OpaqueU(_) => {
                if !hashed_reloc {
                    a.hash(hasher);
                }
            }
            _ => a.hash(hasher),
        }
    }
}

fn hash_thumb_args(insn: unarm::thumb::Ins, hasher: &mut DefaultHasher, hashed_reloc: bool) {
    // hash opcode
    (insn.op as u16).hash(hasher);

    // hash operands
    for a in insn
        .parse(&unarm::ParseFlags {
            ual: true,
            version: unarm::ArmVersion::V4T,
        })
        .args_iter()
    {
        match a {
            unarm::args::Argument::None => {}
            unarm::args::Argument::Reg(_) => a.hash(hasher),
            unarm::args::Argument::RegList(_) => a.hash(hasher),
            unarm::args::Argument::CoReg(_) => a.hash(hasher),
            unarm::args::Argument::StatusReg(_) => a.hash(hasher),
            unarm::args::Argument::StatusMask(_) => a.hash(hasher),
            unarm::args::Argument::Shift(_) => a.hash(hasher),
            unarm::args::Argument::ShiftImm(_)
            | unarm::args::Argument::ShiftReg(_)
            | unarm::args::Argument::UImm(_)
            | unarm::args::Argument::SatImm(_)
            | unarm::args::Argument::SImm(_)
            | unarm::args::Argument::OffsetImm(_)
            | unarm::args::Argument::OffsetReg(_)
            | unarm::args::Argument::BranchDest(_) => {
                if !hashed_reloc {
                    a.hash(hasher);
                }
            }
            unarm::args::Argument::CoOption(_) => a.hash(hasher),
            unarm::args::Argument::CoOpcode(_) => a.hash(hasher),
            unarm::args::Argument::CoprocNum(_) => a.hash(hasher),
            unarm::args::Argument::CpsrMode(_) => a.hash(hasher),
            unarm::args::Argument::CpsrFlags(_) => a.hash(hasher),
            unarm::args::Argument::Endian(_) => a.hash(hasher),
        }
    }
}

use crate::{Arch, Platform};
use objdiff_core::obj::Relocation;
use object::Endian;
use object::elf::{R_ARM_ABS8, R_ARM_ABS16, R_ARM_ABS32, R_ARM_REL32, R_ARM_SBREL32};
use rabbitizer::IsaExtension::{R3000GTE, R4000ALLEGREX, R5900EE};
use rabbitizer::IsaVersion::MIPS_III;
use rabbitizer::operands::ValuedOperand;
use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};

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

fn arm_should_ignore_instruction(vram: usize, relocations: &BTreeMap<u64, Relocation>) -> bool {
    let mut reloc = relocations.get(&(vram as u64));
    if reloc.is_none() {
        reloc = relocations.get(&((vram + 2) as u64));
    }

    if let Some(reloc) = reloc {
        match reloc.flags {
            objdiff_core::obj::RelocationFlags::Elf(flags) => {
                if flags == R_ARM_ABS32
                    || flags == R_ARM_REL32
                    || flags == R_ARM_ABS16
                    || flags == R_ARM_ABS8
                    || flags == R_ARM_SBREL32
                {
                    return true;
                }
            }
            objdiff_core::obj::RelocationFlags::Coff(_) => {
                unimplemented!("COFF relocations not yet implemented")
            }
        }
    }

    false
}

pub fn get_opcodes(
    bytes: &[u8],
    platform: Platform,
    vram: usize,
    relocations: &BTreeMap<u64, Relocation>,
) -> Vec<u16> {
    let insn_length = platform.arch().insn_length();

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
            .enumerate()
            .map(|(i, chunk)| {
                let addr = vram + (i * insn_length);

                // Ignore data
                if arm_should_ignore_instruction(addr, relocations) {
                    return 0;
                }

                let code = platform
                    .endianness()
                    .read_u16_bytes(chunk.try_into().unwrap());
                let ins = unarm::thumb::Ins::new(
                    code as u32,
                    &unarm::ParseFlags {
                        ual: true,
                        version: unarm::ArmVersion::V4T,
                    },
                );
                ins.op as u16
            })
            .collect(),
    }
}

pub(crate) fn get_equivalence_hash(
    bytes: &[u8],
    vram: usize,
    platform: Platform,
    relocations: &BTreeMap<u64, Relocation>,
) -> u64 {
    let mut hasher = DefaultHasher::new();

    let mut reloc_ids = HashMap::new();

    let insn_length = platform.arch().insn_length();

    let mut hashed_reloc;

    for (i, chunk) in bytes.chunks_exact(insn_length).enumerate() {
        let cur_vram = vram + i * insn_length;

        // Hash the unique id for the relocation entry rather than the specifics
        if let Some(reloc) = relocations.get(&(cur_vram as u64)) {
            let next_id = reloc_ids.len();
            let hash_id = *reloc_ids
                .entry((reloc.target_symbol, reloc.addend, reloc.flags))
                .or_insert(next_id);
            hash_id.hash(&mut hasher);
            hashed_reloc = true;
        } else {
            hashed_reloc = false;
        }

        match platform.arch() {
            Arch::Mips | Arch::Ppc => {
                let code = platform
                    .endianness()
                    .read_u32_bytes(chunk.try_into().unwrap());

                match platform.arch() {
                    Arch::Mips => {
                        let instruction =
                            get_rabbitizer_instruction(code, cur_vram as u32, platform);

                        // hash opcode
                        instruction.opcode().hash(&mut hasher);

                        // hash operands
                        for vo in instruction.valued_operands_iter() {
                            match vo {
                                ValuedOperand::ALL_EMPTY() => vo.hash(&mut hasher),
                                ValuedOperand::core_rs(_) => vo.hash(&mut hasher),
                                ValuedOperand::core_rt(_) => vo.hash(&mut hasher),
                                ValuedOperand::core_rd(_) => vo.hash(&mut hasher),
                                ValuedOperand::core_sa(_) => vo.hash(&mut hasher),
                                ValuedOperand::core_zero() => vo.hash(&mut hasher),
                                ValuedOperand::core_cop0d(_) => vo.hash(&mut hasher),
                                ValuedOperand::core_cop0cd(_) => vo.hash(&mut hasher),
                                ValuedOperand::core_fs(_) => vo.hash(&mut hasher),
                                ValuedOperand::core_ft(_) => vo.hash(&mut hasher),
                                ValuedOperand::core_fd(_) => vo.hash(&mut hasher),
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
                                        vo.hash(&mut hasher);
                                    }
                                }
                                ValuedOperand::core_imm_i16(_) => {
                                    if !hashed_reloc {
                                        vo.hash(&mut hasher);
                                    }
                                }
                                ValuedOperand::core_imm_u16(_) => {
                                    if !hashed_reloc {
                                        vo.hash(&mut hasher);
                                    }
                                }
                                ValuedOperand::core_branch_target_label(_) => {
                                    // assert!(
                                    //     !hashed_reloc,
                                    //     "Relocation and branch target label at the same time"
                                    // );
                                    vo.hash(&mut hasher);
                                }
                                ValuedOperand::core_imm_rs(_, gpr) => {
                                    if !hashed_reloc {
                                        vo.hash(&mut hasher);
                                    } else {
                                        gpr.hash(&mut hasher);
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
                                _ => vo.hash(&mut hasher),
                            }
                        }
                    }
                    Arch::Ppc => {
                        let instruction =
                            powerpc::Ins::new(code, powerpc::Extensions::gekko_broadway());

                        // hash opcode
                        instruction.op.hash(&mut hasher);

                        // hash operands
                        for a in instruction.basic().args {
                            match a {
                                powerpc::Argument::None => {}
                                powerpc::Argument::Simm(_)
                                | powerpc::Argument::Uimm(_)
                                | powerpc::Argument::Offset(_)
                                | powerpc::Argument::BranchDest(_)
                                | powerpc::Argument::OpaqueU(_) => {
                                    if !hashed_reloc {
                                        a.hash(&mut hasher);
                                    }
                                }
                                _ => a.hash(&mut hasher),
                            }
                        }
                    }
                    Arch::Thumb => unreachable!(),
                }
            }
            Arch::Thumb => {
                if arm_should_ignore_instruction(cur_vram, relocations) {
                    continue;
                }

                let code = platform
                    .endianness()
                    .read_u16_bytes(chunk.try_into().unwrap());
                let instruction = unarm::thumb::Ins::new(
                    code as u32,
                    &unarm::ParseFlags {
                        ual: true,
                        version: unarm::ArmVersion::V4T,
                    },
                );

                // hash opcode
                (instruction.op as u16).hash(&mut hasher);

                // hash operands
                for a in instruction
                    .parse(&unarm::ParseFlags {
                        ual: true,
                        version: unarm::ArmVersion::V4T,
                    })
                    .args_iter()
                {
                    match a {
                        unarm::args::Argument::None => {}
                        unarm::args::Argument::Reg(_) => a.hash(&mut hasher),
                        unarm::args::Argument::RegList(_) => a.hash(&mut hasher),
                        unarm::args::Argument::CoReg(_) => a.hash(&mut hasher),
                        unarm::args::Argument::StatusReg(_) => a.hash(&mut hasher),
                        unarm::args::Argument::StatusMask(_) => a.hash(&mut hasher),
                        unarm::args::Argument::Shift(_) => a.hash(&mut hasher),
                        unarm::args::Argument::ShiftImm(_)
                        | unarm::args::Argument::ShiftReg(_)
                        | unarm::args::Argument::UImm(_)
                        | unarm::args::Argument::SatImm(_)
                        | unarm::args::Argument::SImm(_)
                        | unarm::args::Argument::OffsetImm(_)
                        | unarm::args::Argument::OffsetReg(_)
                        | unarm::args::Argument::BranchDest(_) => {
                            if !hashed_reloc {
                                a.hash(&mut hasher);
                            }
                        }

                        unarm::args::Argument::CoOption(_) => a.hash(&mut hasher),
                        unarm::args::Argument::CoOpcode(_) => a.hash(&mut hasher),
                        unarm::args::Argument::CoprocNum(_) => a.hash(&mut hasher),
                        unarm::args::Argument::CpsrMode(_) => a.hash(&mut hasher),
                        unarm::args::Argument::CpsrFlags(_) => a.hash(&mut hasher),
                        unarm::args::Argument::Endian(_) => a.hash(&mut hasher),
                    }
                }
            }
        }
    }

    hasher.finish()
}

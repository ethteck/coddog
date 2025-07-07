use crate::Platform;
use crate::ingest::CoddogRel;
use object::Endian;
use rabbitizer::IsaExtension::{R3000GTE, R5900EE};
use rabbitizer::IsaVersion::MIPS_III;
use rabbitizer::Vram;
use rabbitizer::operands::ValuedOperand;
use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};

fn get_rabbitizer_instruction(word: u32, vram: u32, platform: Platform) -> rabbitizer::Instruction {
    rabbitizer::Instruction::new(
        word,
        Vram::new(vram),
        match platform {
            Platform::N64 => rabbitizer::InstructionFlags::new(MIPS_III),
            Platform::Psx => rabbitizer::InstructionFlags::new_extension(R3000GTE),
            Platform::Ps2 => rabbitizer::InstructionFlags::new_extension(R5900EE),
            _ => unreachable!(),
        },
    )
}

pub fn get_opcodes(bytes: &[u8], platform: Platform) -> Vec<u16> {
    let insn_length = platform.arch().insn_length();

    match platform {
        Platform::N64 | Platform::Psx | Platform::Ps2 => bytes
            .chunks_exact(insn_length)
            .map(|chunk| {
                let code = platform
                    .endianness()
                    .read_u32_bytes(chunk.try_into().unwrap());
                let instruction = get_rabbitizer_instruction(code, 0, platform);
                instruction.opcode() as u16
            })
            .collect(),
        Platform::Gc | Platform::Wii => bytes
            .chunks_exact(insn_length)
            .map(|c| {
                powerpc::Opcode::detect(
                    platform.endianness().read_u32_bytes(c.try_into().unwrap()),
                    powerpc::Extensions::none(),
                ) as u16
            })
            .collect(),
    }
}

pub(crate) fn get_equivalence_hash(
    bytes: &[u8],
    vram: usize,
    platform: Platform,
    relocations: &BTreeMap<u64, CoddogRel>,
) -> u64 {
    let mut hasher = DefaultHasher::new();

    let mut reloc_ids = HashMap::new();

    let insn_length = platform.arch().insn_length();

    match platform {
        Platform::N64 | Platform::Psx | Platform::Ps2 | Platform::Wii | Platform::Gc => {
            let mut hashed_reloc;

            for (i, chunk) in bytes.chunks_exact(insn_length).enumerate() {
                let code = platform
                    .endianness()
                    .read_u32_bytes(chunk.try_into().unwrap());
                let cur_vram = vram + i * insn_length;

                // Hash the unique id for the relocation entry rather than the specifics
                if let Some(reloc) = relocations.get(&(cur_vram as u64)) {
                    let next_id = reloc_ids.len();
                    let hash_id = *reloc_ids.entry(reloc).or_insert(next_id);
                    hash_id.hash(&mut hasher);
                    hashed_reloc = true;
                } else {
                    hashed_reloc = false;
                }

                match platform {
                    Platform::N64 | Platform::Psx | Platform::Ps2 => {
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
                                ValuedOperand::core_immediate(_) => {
                                    if !hashed_reloc {
                                        vo.hash(&mut hasher);
                                    }
                                }
                                ValuedOperand::core_branch_target_label(_) => {
                                    assert!(
                                        !hashed_reloc,
                                        "Relocation and branch target label at the same time"
                                    );
                                    vo.hash(&mut hasher);
                                }
                                ValuedOperand::core_immediate_base(_, gpr) => {
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
                    Platform::Gc | Platform::Wii => {
                        let instruction = powerpc::Ins::new(code, powerpc::Extensions::none());

                        // hash opcode
                        let opcode = instruction.op as u16;
                        opcode.hash(&mut hasher);

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
                }
            }
        }
    }

    hasher.finish()
}

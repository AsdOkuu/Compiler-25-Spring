/* Uses */
use std::collections::HashMap;
use crate::ast;
use koopa::ir::*;

/* Parse Binary into risc32 text (instruction text, final register) */
fn parse_binary(op: BinaryOp) -> String {
    let mut final_str = String::new();

    let lhs = "t0".to_string();
    let rhs = "t1".to_string();
    let new_reg = "t2".to_string();
    match op {
        BinaryOp::NotEq => {
            final_str += &("sub ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
            final_str += &("snez ".to_string() + &new_reg + ", " + &new_reg + "\n");
        },
        BinaryOp::Eq => {
            final_str += &("sub ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
            final_str += &("seqz ".to_string() + &new_reg + ", " + &new_reg + "\n");
        },
        BinaryOp::Gt => {
            final_str += &("sub ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
            final_str += &("sgtz ".to_string() + &new_reg + ", " + &new_reg + "\n");
        },
        BinaryOp::Lt => {
            final_str += &("sub ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
            final_str += &("sltz ".to_string() + &new_reg + ", " + &new_reg + "\n");
        },
        BinaryOp::Ge => {
            final_str += &("sub ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
            final_str += &("sltz ".to_string() + &new_reg + ", " + &new_reg + "\n");
            final_str += &("seqz ".to_string() + &new_reg + ", " + &new_reg + "\n");
        },
        BinaryOp::Le => {
            final_str += &("sub ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
            final_str += &("sgtz ".to_string() + &new_reg + ", " + &new_reg + "\n");
            final_str += &("seqz ".to_string() + &new_reg + ", " + &new_reg + "\n");
        },
        BinaryOp::Add => {
            final_str += &("add ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
        },
        BinaryOp::Sub => {
            final_str += &("sub ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
        },
        BinaryOp::Mul => {
            final_str += &("mul ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
        },
        BinaryOp::Div => {
            final_str += &("div ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
        },
        BinaryOp::Mod => {
            final_str += &("rem ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
        },
        BinaryOp::And => {
            final_str += &("and ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
        },
        BinaryOp::Or => {
            final_str += &("or ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
        },
        BinaryOp::Xor => {
            final_str += &("xor ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
        },
        BinaryOp::Shl => {
            final_str += &("sll ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
        },
        BinaryOp::Shr => {
            final_str += &("srl ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
        },
        BinaryOp::Sar => {
            final_str += &("sra ".to_string() + &new_reg + ", " + &lhs + ", " + &rhs + "\n");
        },
    }
    
    final_str
}

/* Get offset */
fn get_offset(offset: usize, text: &mut String) -> String {
    if offset <= 2048 {
        offset.to_string() + "(sp)"
    }else {
        *text += &("li t3, ".to_string() + &offset.to_string() + "\n");
        *text += "add t3, sp, t3\n";
        "0(t3)".to_string()
    }
}

fn check_for_bb(bb: BasicBlock, func_data: &FunctionData, sp_delta: usize, pos: &HashMap<Value, usize>, bb_count: &mut usize, check: &mut HashMap<BasicBlock, usize>) -> (usize, String) {
    match check.get(&bb) {
        None => {
            *bb_count += 1;
            check.insert(bb, *bb_count);
            (*bb_count, bb_gen_riscv32(bb, func_data, sp_delta, pos, bb_count, check))
        },
        Some(&id) => {
            (id, String::new())
        }
    }
}

fn bb_gen_riscv32(bb: BasicBlock, func_data: &FunctionData, sp_delta: usize, pos: &HashMap<Value, usize>, bb_count: &mut usize, check: &mut HashMap<BasicBlock, usize>) -> String {
    let prefix = &".L".to_string();
    let mut text = prefix.to_string() + &bb_count.to_string() + ":\n";
    let node = func_data.layout().bbs().node(&bb).unwrap();

    for &inst in node.insts().keys() {
        let value_data = func_data.dfg().value(inst);
        // Only visit return
        match value_data.kind() {
            ValueKind::Return(ret) => {
                let value = ret.value().unwrap();
                let value_data = func_data.dfg().value(value);
                match value_data.kind() {
                    ValueKind::Integer(int) => {
                        text += &("li a0, ".to_string() + &int.value().to_string() + "\n");
                    },
                    _ => {
                        let offset = get_offset(pos[&value], &mut text);
                        text += &("lw a0, ".to_string() + &offset + "\n");
                    }
                }
                text += &("li t0, ".to_string() + &sp_delta.to_string() + "\n");
                text += "add sp, sp, t0\n";
                text += "ret\n";
            },
            ValueKind::Alloc(_) => {
                // do nothing
            },
            ValueKind::Store(store) => {
                let value = store.value();
                let value_data = func_data.dfg().value(value);
                match value_data.kind() {
                    ValueKind::Integer(int) => {
                        text += &("li t0, ".to_string() + &int.value().to_string() + "\n");
                    },
                    _ => {
                        let offset = get_offset(pos[&value], &mut text);
                        text += &("lw t0, ".to_string() + &offset + "\n");
                    }
                }
                let offset = get_offset(pos[&store.dest()], &mut text);
                text += &("sw t0, ".to_string() + &offset + "\n");
            },
            ValueKind::Load(load) => {
                let src = load.src();
                if let Some(&offset) = pos.get(&src) {
                    let offset = get_offset(offset, &mut text);
                    text += &("lw t0, ".to_string() + &offset + "\n");
                    let offset = get_offset(pos[&inst], &mut text);
                    text += &("sw t0, ".to_string() + &offset + "\n");
                }else {
                    panic!("Load not found");
                }
            },
            ValueKind::Binary(bin) => {
                let lhs = bin.lhs();
                let rhs = bin.rhs();
                let lhs_data = func_data.dfg().value(lhs);
                let rhs_data = func_data.dfg().value(rhs);

                match lhs_data.kind() {
                    ValueKind::Integer(int) => {
                        text += &("li t0, ".to_string() + &int.value().to_string() + "\n");
                    },
                    _ => {
                        let offset = get_offset(pos[&lhs], &mut text);
                        text += &("lw t0, ".to_string() + &offset + "\n");
                    }
                }
                match rhs_data.kind() {
                    ValueKind::Integer(int) => {
                        text += &("li t1, ".to_string() + &int.value().to_string() + "\n");
                    },
                    _ => {
                        let offset = get_offset(pos[&rhs], &mut text);
                        text += &("lw t1, ".to_string() + &offset + "\n");
                    }
                }

                text += &parse_binary(bin.op());

                let offset = get_offset(pos[&inst], &mut text);
                text += &("sw t2, ".to_string() + &offset + "\n");
            },
            ValueKind::Jump(jump) => {
                let target = jump.target();
                let (target_id, new_text) = check_for_bb(target, func_data, sp_delta, pos, bb_count, check);
                text += &("j ".to_string() + prefix + &target_id.to_string() + "\n");
                text += &new_text;
            },
            ValueKind::Branch(branch) => {
                let cond = branch.cond();

                match func_data.dfg().value(cond).kind() {
                    ValueKind::Integer(int) => {
                        text += &("li t0, ".to_string() + &int.value().to_string() + "\n");
                    },
                    _ => {
                        let offset = get_offset(pos[&cond], &mut text);
                        text += &("lw t0, ".to_string() + &offset + "\n");
                    }
                }

                let true_bb = branch.true_bb();
                let false_bb = branch.false_bb();
                
                let (true_id, true_text) = check_for_bb(true_bb, func_data, sp_delta, pos, bb_count, check);
                let (false_id, false_text) = check_for_bb(false_bb, func_data, sp_delta, pos, bb_count, check);

                

                text += &("bnez t0, ".to_string() + prefix + &true_id.to_string() + "\n");
                text += &("j ".to_string() + prefix + &false_id.to_string() + "\n");
                text += &true_text;
                text += &false_text;
            },
            _ => {
                panic!("Unknown inst value kind");
            }
        }
    }
    text
}

/* Generate riscv32 code */
pub fn gen_riscv32(ast: ast::Program) -> String {
    let mut text = ".text\n".to_string();
    text += ".globl main\n";
    let program = ast.dump();
    for &func in program.func_layout() {
        let func_data = program.func(func);
        text += &func_data.name()[1..];
        text += ":\n";

        // Calc stack placement
        let mut sp_delta = 0 as usize;
        let mut pos = HashMap::new();
        for (&_bb, node) in func_data.layout().bbs() {
            for &inst in node.insts().keys() {
                let value_data = func_data.dfg().value(inst);
                // Only visit alloc, load, binary
                if !value_data.ty().is_unit() {
                    pos.insert(inst, sp_delta);
                    sp_delta += 4;
                }
            }
        }
        println!("sp_delta: {}", sp_delta);
        sp_delta = (sp_delta + 15) / 16 * 16;
        text += &("li t0, -".to_string() + &sp_delta.to_string() + "\n");
        text += "add sp, sp, t0\n";

        // Start from entry
        let mut check = HashMap::new();
        let mut bb_count = 0 as usize;
        text += &bb_gen_riscv32(func_data.layout().entry_bb().unwrap(), func_data, sp_delta, &pos, &mut bb_count, &mut check);
    }
    text
}

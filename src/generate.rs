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

    *text += &("li t4, ".to_string() + &offset.to_string() + "\n");
    *text += "add t4, t3, t4\n";
    "0(t4)".to_string()
}

fn load_value(reg: String, value: Value, func_data: &FunctionData, sp_delta: usize, pos: &HashMap<Value, usize>) -> String {
    match func_data.dfg().value(value).kind() {
        ValueKind::Integer(int) => {
            "li ".to_string() + &reg + ", " + &int.value().to_string() + "\n"
        },
        ValueKind::FuncArgRef(arg) => {
            let i = arg.index();
            if i < 8 {
                "mv ".to_string() + &reg + ", a" + &i.to_string() + "\n"
            }else {
                let offset = sp_delta + (i - 8) * 4;
                "lw ".to_string() + &reg + ", " + &offset.to_string() + "(sp)\n"
            }
        },
        _ => {
            let mut text = String::new();
            println!("{:?}", func_data.dfg().value(value).kind());
            let offset = get_offset(pos[&value], &mut text);
            text + "lw " + &reg + ", " + &offset + "\n"
        }
    }
}

fn check_for_bb(bb: BasicBlock, program: &Program, func_data: &FunctionData, sp_delta: usize, pos: &HashMap<Value, usize>, global_var: &HashMap<Value, i32>, bb_count: &mut usize, check: &mut HashMap<BasicBlock, usize>) -> (usize, String) {
    match check.get(&bb) {
        None => {
            *bb_count += 1;
            check.insert(bb, *bb_count);
            (*bb_count, bb_gen_riscv32(bb, &program, func_data, sp_delta, pos, global_var, bb_count, check))
        },
        Some(&id) => {
            (id, String::new())
        }
    }
}

fn bb_gen_riscv32(bb: BasicBlock, program: &Program, func_data: &FunctionData, sp_delta: usize, pos: &HashMap<Value, usize>, global_var: &HashMap<Value, i32>, bb_count: &mut usize, check: &mut HashMap<BasicBlock, usize>) -> String {
    let prefix = &(".L".to_string() + &func_data.name()[1..]);
    let mut text = prefix.to_string() + &bb_count.to_string() + ":\n";
    let node = func_data.layout().bbs().node(&bb).unwrap();

    for &inst in node.insts().keys() {
        let value_data = func_data.dfg().value(inst);
        // Only visit return
        match value_data.kind() {
            ValueKind::Return(ret) => {
                if let Some(ret_value) = ret.value() {
                    text += &load_value("a0".to_string(), ret_value, func_data, sp_delta, pos);
                }
                text += "lw ra, -4(t3)\n";
                text += &("li t0, ".to_string() + &sp_delta.to_string() + "\n");
                text += "add sp, sp, t0\n";
                text += "ret\n";
            },
            ValueKind::Alloc(_) => {
                // do nothing
            },
            ValueKind::Store(store) => {
                text += &load_value("t0".to_string(), store.value(), func_data, sp_delta, pos);
                if let Some(&offset) = pos.get(&store.dest()) {
                    let offset = get_offset(offset, &mut text);
                    text += &("sw t0, ".to_string() + &offset + "\n");
                }else {
                    // Global
                    text += &("la t5, gvar".to_string() + &global_var[&store.dest()].to_string() + "\n");
                    text += "sw t0, 0(t5)\n";
                }
            },
            ValueKind::Load(load) => {
                let src = load.src();
                if let Some(&offset) = pos.get(&src) {
                    let offset = get_offset(offset, &mut text);
                    text += &("lw t0, ".to_string() + &offset + "\n");
                }else {
                    // panic!("Load not found");
                    // Global
                    text += &("la t5, gvar".to_string() + &global_var[&src].to_string() + "\n");
                    text += "lw t0, 0(t5)\n";
                }
                let offset = get_offset(pos[&inst], &mut text);
                text += &("sw t0, ".to_string() + &offset + "\n");
            },
            ValueKind::Binary(bin) => {
                text += &load_value("t0".to_string(), bin.lhs(), func_data, sp_delta, pos);
                text += &load_value("t1".to_string(), bin.rhs(), func_data, sp_delta, pos);
                text += &parse_binary(bin.op());
                let offset = get_offset(pos[&inst], &mut text);
                text += &("sw t2, ".to_string() + &offset + "\n");
            },
            ValueKind::Jump(jump) => {
                let target = jump.target();
                let (target_id, new_text) = check_for_bb(target, program, func_data, sp_delta, pos, global_var, bb_count, check);
                text += &("j ".to_string() + prefix + &target_id.to_string() + "\n");
                text += &new_text;
            },
            ValueKind::Branch(branch) => {
                text += &load_value("t0".to_string(), branch.cond(), func_data, sp_delta, pos);

                let true_bb = branch.true_bb();
                let false_bb = branch.false_bb();
                let (true_id, true_text) = check_for_bb(true_bb, program, func_data, sp_delta, pos, global_var, bb_count, check);
                let (false_id, false_text) = check_for_bb(false_bb, program, func_data, sp_delta, pos, global_var, bb_count, check);

                text += &("bnez t0, ".to_string() + prefix + &true_id.to_string() + "\n");
                text += &("j ".to_string() + prefix + &false_id.to_string() + "\n");
                text += &true_text;
                text += &false_text;
            },
            ValueKind::Call(call) => {
                // Bind args to a0 - a7
                let mut reg_len = call.args().len();
                if reg_len > 8 {
                    reg_len = 8;
                }
                for i in 0..reg_len {
                    text += &load_value("a".to_string() + &i.to_string(), call.args()[i], func_data, sp_delta, pos);
                }
                // Store args above [sp]
                let len = call.args().len();
                if len > 8 {
                    for i in 8..len {
                        let offset = (i - 8) * 4;
                        text += &load_value("t0".to_string(), call.args()[i], func_data, sp_delta, pos);
                        text += "sw t0, ";
                        text += &(offset.to_string() + "(sp)\n");
                    }
                }
                // Store t0 - t6 (actually only t3 need)
                text += "sw t3, ";
                text += &(sp_delta - 4).to_string();
                text += "(sp)\n";
                for i in 0..=6 {
                    let offset = -(i + 2) * 4;
                    text += "sw t";
                    text += &(i.to_string() + ", ");
                    text += &(offset.to_string() + "(t3)\n");
                }
                // Call
                text += "call ";
                text += &program.func(call.callee()).name()[1..];
                text += "\n";
                // Recover t0 - t6
                text += "lw t3, ";
                text += &(sp_delta - 4).to_string();
                text += "(sp)\n";
                for i in 0..=6 {
                    let offset = -(i + 2) * 4;
                    text += "lw t";
                    text += &(i.to_string() + ", ");
                    text += &(offset.to_string() + "(t3)\n");
                }
                // Store return value
                if let Some(offset) = pos.get(&inst) {
                    let offset = get_offset(*offset, &mut text);
                    text += "sw a0, ";
                    text += &offset;
                    text += "\n";
                }
            }
            _ => {
                panic!("Unknown inst value kind");
            }
        }
    }
    text
}

/* Generate riscv32 code */
pub fn gen_riscv32(ast: ast::Program) -> String {
    let mut text = String::new();
    let program = ast.dump();
    let mut global_count = 0;
    let mut global_var = HashMap::new();
    for &inst in program.inst_layout() {
        global_var.insert(inst, global_count);
        text += ".data\n";
        text += ".globl gvar";
        text += &global_count.to_string();
        text += "\n";
        text += "gvar";
        text += &global_count.to_string();
        text += ":\n";
        if let ValueKind::GlobalAlloc(global) = program.borrow_value(inst).kind() {
            match program.borrow_value(global.init()).kind() {
                ValueKind::Integer(integer) => {
                    text += ".word ";
                    text += &integer.value().to_string();
                    text += "\n";
                }
                ValueKind::ZeroInit(_) => {
                    text += ".zero 4\n";
                }
                _  => unreachable!()
            }
        }
        global_count += 1;
    }
    for &func in program.func_layout() {
        let func_data = program.func(func);
        if let None = func_data.layout().entry_bb() {
            continue;
        }

        text += ".text\n";
        text += &(".globl ".to_string() + &func_data.name()[1..] + "\n");
        text += &func_data.name()[1..];
        text += ":\n";

        // Calc stack placement
        let mut sp_delta = 0 as usize;
        let mut call_delta = 0 as usize;
        let mut pos = HashMap::new();
        for (&_bb, node) in func_data.layout().bbs() {
            for &inst in node.insts().keys() {
                let value_data = func_data.dfg().value(inst);
                // Only visit alloc, load, binary
                if !value_data.ty().is_unit() {
                    pos.insert(inst, sp_delta);
                    sp_delta += 4;
                }else if let ValueKind::Call(call) = value_data.kind() {
                    let val = call.args().len() * 4;
                    if val > call_delta {
                        call_delta = val;
                    }
                }
            }
        }
        println!("sp_delta: {}", sp_delta);
        sp_delta = (sp_delta + 4 + 15) / 16 * 16;
        call_delta = (call_delta + 15) / 16 * 16;
        let delta = sp_delta + call_delta + 64;
        text += &("li t3, -".to_string() + &sp_delta.to_string() + "\n");
        text += "add t3, sp, t3\n";
        text += &("li t0, -".to_string() + &delta.to_string() + "\n");
        text += "add sp, sp, t0\n";
        // store ra
        text += "sw ra, -4(t3)\n";
        

        // Start from entry
        let mut check = HashMap::new();
        let mut bb_count = 0 as usize;
        text += &bb_gen_riscv32(func_data.layout().entry_bb().unwrap(), &program, func_data, delta, &pos, &global_var, &mut bb_count, &mut check);
    }
    text
}

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

fn load_value(reg: String, value: Value, func_data: &FunctionData, sp_delta: usize, pos: &HashMap<Value, usize>, _program: &Program) -> String {
    let in_func = func_data.dfg().values().get(&value).is_some();
    let kind = if in_func {
        func_data.dfg().value(value).kind().clone()
    }else {
        // program.borrow_value(value).kind().clone()
        panic!()
    };
    match kind {
        ValueKind::Integer(int) => {
            "li ".to_string() + &reg + ", " + &int.value().to_string() + "\n"
        },
        ValueKind::FuncArgRef(arg) => {
            let i = arg.index();
            if i < 8 {
                "mv ".to_string() + &reg + ", a" + &i.to_string() + "\n"
            }else {
                let offset = sp_delta + (i - 8) * 4;
                "li t4, ".to_string() + &offset.to_string() + "\n" + 
                "add t4, sp, t4\n" +
                &"lw ".to_string() + &reg + ", 0(t4)\n"
            }
        },
        _ => {
            let mut text = String::new();
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
                    text += &load_value("a0".to_string(), ret_value, func_data, sp_delta, pos, program);
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
                let dest = store.dest();
                text += &load_value("t0".to_string(), store.value(), func_data, sp_delta, pos, program);
                if let Some(&offset) = pos.get(&dest) {
                    let get_ptr = matches!(func_data.dfg().value(dest).kind(), ValueKind::GetElemPtr(_)) || matches!(func_data.dfg().value(dest).kind(), ValueKind::GetPtr(_));
                    let offset = get_offset(offset, &mut text);
                    if get_ptr {
                        text += "lw t4, 0(t4)\n";
                    }
                    text += &("sw t0, ".to_string() + &offset + "\n");
                }else {
                    // Global
                    text += &("la t5, gvar".to_string() + &global_var[&dest].to_string() + "\n");
                    text += "sw t0, 0(t5)\n";
                }
            },
            ValueKind::Load(load) => {
                let src = load.src();
                if let Some(&offset) = pos.get(&src) {
                    let get_ptr = matches!(func_data.dfg().value(src).kind(), ValueKind::GetElemPtr(_)) || matches!(func_data.dfg().value(src).kind(), ValueKind::GetPtr(_));
                    let offset = get_offset(offset, &mut text);
                    text += &("lw t0, ".to_string() + &offset + "\n");
                    if get_ptr {
                        text += "lw t0, 0(t0)\n";
                    }
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
                text += &load_value("t0".to_string(), bin.lhs(), func_data, sp_delta, pos, program);
                text += &load_value("t1".to_string(), bin.rhs(), func_data, sp_delta, pos, program);
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
                text += &load_value("t0".to_string(), branch.cond(), func_data, sp_delta, pos, program);

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
                    text += &load_value("a".to_string() + &i.to_string(), call.args()[i], func_data, sp_delta, pos, program);
                }
                // Store args above [sp]
                let len = call.args().len();
                if len > 8 {
                    for i in 8..len {
                        let offset = (i - 8) * 4;
                        text += &load_value("t0".to_string(), call.args()[i], func_data, sp_delta, pos, program);
                        text += "sw t0, ";
                        text += &(offset.to_string() + "(sp)\n");
                    }
                }
                // Store t0 - t6 (actually only t3 need)
                text += &("li t4, ".to_string() + &(sp_delta - 4).to_string() + "\n");
                text += "add t4, sp, t4\n";
                text += "sw t3, 0(t4)\n";
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
                text += &("li t4, ".to_string() + &(sp_delta - 4).to_string() + "\n");
                text += "add t4, sp, t4\n";
                text += "lw t3, 0(t4)\n";
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
            ValueKind::GetElemPtr(gep) => {
                // 1. Calc offset
                let src = gep.src();
                let in_func = func_data.dfg().values().get(&src).is_some();
                let mut alloc = false;
                let ty = if in_func {
                    // println!("src_kind: {:?}", func_data.dfg().value(src).kind());
                    if let ValueKind::Alloc(_) = func_data.dfg().value(src).kind() {
                        alloc = true;
                    }
                    func_data.dfg().value(src).ty().clone()
                }else {
                    program.borrow_value(src).ty().clone()
                };
                let src_size = if let TypeKind::Pointer(ptr) = ty.kind() {
                    if let TypeKind::Array(elem, _) = ptr.kind() {
                        elem.size()
                    }else {
                        panic!()
                    }
                }else {
                    panic!()
                };
                // Todo: bad calling dfg().value(...) in load_value
                // println!("src_size: {}", src_size);
                text += &load_value("t0".to_string(), gep.index(), func_data, sp_delta, pos, program);
                text += "li t1, ";
                text += &src_size.to_string();
                text += "\n";
                text += "mul t0, t0, t1\n";
                // 2. Position array
                // text += &load_value("t1".to_string(), gep.src(), func_data, sp_delta, pos, program);
                if in_func {
                    text += &("li t1, ".to_string() + &pos[&src].to_string() + "\n");
                    text += "add t1, t3, t1\n";
                    if !alloc {
                        text += "lw t1, 0(t1)\n";
                    }
                }else {
                    text += &("la t1, gvar".to_string() + &global_var[&src].to_string() + "\n");
                }
                // 3. Calc absolute addr
                text += "add t1, t1, t0\n";
                // 4. Save
                let offset = get_offset(pos[&inst], &mut text);
                text += "sw t1, ";
                text += &offset;
                text += "\n";
            }
            ValueKind::GetPtr(gp) => {
                // 1. Calc offset
                let src = gp.src();
                let in_func = func_data.dfg().values().get(&src).is_some();
                let mut alloc = false;
                let ty = if in_func {
                    println!("src_kind: {:?}", func_data.dfg().value(src).kind());
                    if let ValueKind::Alloc(_) = func_data.dfg().value(src).kind() {
                        alloc = true;
                    }
                    func_data.dfg().value(src).ty().clone()
                }else {
                    program.borrow_value(src).ty().clone()
                };
                println!("gp ty: {:#?}", ty);
                let src_size = if let TypeKind::Pointer(ptr) = ty.kind() {
                    ptr.size()
                }else {
                    panic!()
                };
                // Todo: bad calling dfg().value(...) in load_value
                println!("src_size: {}", src_size);
                text += &load_value("t0".to_string(), gp.index(), func_data, sp_delta, pos, program);
                text += "li t1, ";
                text += &src_size.to_string();
                text += "\n";
                text += "mul t0, t0, t1\n";
                // 2. Position array
                // text += &load_value("t1".to_string(), gep.src(), func_data, sp_delta, pos, program);
                if in_func {
                    text += &("li t1, ".to_string() + &pos[&src].to_string() + "\n");
                    text += "add t1, t3, t1\n";
                    if !alloc {
                        text += "lw t1, 0(t1)\n";
                    }
                }else {
                    text += &("la t1, gvar".to_string() + &global_var[&src].to_string() + "\n");
                }
                // 3. Calc absolute addr
                text += "add t1, t1, t0\n";
                // 4. Save
                let offset = get_offset(pos[&inst], &mut text);
                text += "sw t1, ";
                text += &offset;
                text += "\n";
            }
            _ => {
                panic!("Unknown inst value kind");
            }
        }
    }
    text
}

fn gen_global_alloc(value: Value, program: &Program, text: &mut String) {
    match program.borrow_value(value).kind() {
        ValueKind::GlobalAlloc(alloc) => {
            gen_global_alloc(alloc.init(), program, text);
        }
        ValueKind::Integer(int) => {
            *text += ".word ";
            *text += &int.value().to_string();
            *text += "\n";
        }
        ValueKind::Aggregate(agg) => {
            for v in agg.elems() {
                gen_global_alloc(*v, program, text);
            }
        }
        _ => unreachable!()
    }
}

/* Generate riscv32 code */
pub fn gen_riscv32(ast: ast::Program) -> String {
    let mut text = String::new();
    let program = ast.dump();

    Type::set_ptr_size(4);
    let mut global_count = 0;
    let mut global_var = HashMap::new();
    // Global alloc
    for &inst in program.inst_layout() {
        global_var.insert(inst, global_count);
        text += ".data\n";
        text += ".globl gvar";
        text += &global_count.to_string();
        text += "\n";
        text += "gvar";
        text += &global_count.to_string();
        text += ":\n";
        gen_global_alloc(inst, &program, &mut text);
        global_count += 1;
    }
    // Function
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
                    let ty = value_data.ty();
                    if let TypeKind::Pointer(ptr) = ty.kind() {
                        sp_delta += ptr.size();
                        println!("ty: {:#?}", ptr);
                        println!("{}", ptr.size());
                    }else {
                        sp_delta += ty.size();
                    }
                }else if let ValueKind::Call(call) = value_data.kind() {
                    let val = call.args().len() * 4;
                    if val > call_delta {
                        call_delta = val;
                    }
                }
            }
        }
        println!("sp_delta: {}", sp_delta);
        sp_delta = (sp_delta + 4 + 15) / 16 * 128;
        call_delta = (call_delta + 15) / 16 * 128;
        // sp_delta = 1536;
        // call_delta = 512;
        let delta = sp_delta + call_delta + 128;
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

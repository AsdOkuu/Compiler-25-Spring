/* Uses */
use std::collections::HashMap;
use crate::ast;
use koopa::ir::*;
use koopa::ir::builder::*;
use koopa::back::KoopaGenerator;

enum DefType {
    Const,
    Var,
}

impl ast::Exp {
    fn dump(self, bb: BasicBlock, data: &mut FunctionData, symbol_table: &HashMap<String, (Value, DefType)>) -> Value {
        match *self.core {
            ast::ExpCore::Binary(e0, op, e1) => {
                let v0 = e0.dump(bb, data, symbol_table);
                let v1 = e1.dump(bb, data, symbol_table);
                let v = data.dfg_mut().new_value().binary(op, v0, v1);
                data.layout_mut().bb_mut(bb).insts_mut().push_key_back(v).unwrap();
                v
            },
            ast::ExpCore::Single(i) => {
                data.dfg_mut().new_value().integer(i)
            },
            ast::ExpCore::Ident(id) => {
                let (v, t) = symbol_table.get(&id).unwrap();
                match t {
                    DefType::Const => {
                        v.clone()
                    },
                    DefType::Var => {
                        let load = data.dfg_mut().new_value().load(v.clone());
                        data.layout_mut().bb_mut(bb).insts_mut().push_key_back(load).unwrap();
                        load
                    },
                }
            }
        }
    }
}

impl ast::Program {
    /* Dump prog into koopa */
    fn dump(self) -> Program {
        // Now original version
        let mut program = Program::new();
        let main = program.new_func(
            FunctionData::new(("@".to_owned() + &self.func.id).into(), Vec::new(), Type::get_i32()),
        );
        let main_data = program.func_mut(main);
        let entry = main_data.dfg_mut().new_bb().basic_block(Some("%entry".into()));
        main_data.layout_mut().bbs_mut().push_key_back(entry).unwrap();

        // Maintain a symbol table (key, value)
        let mut symbol_table = HashMap::new();
        for item in self.func.block.block_item_list {
            match item {
                ast::BlockItem::Decl(decl) => {
                    match decl {
                        ast::Decl::Const(const_decl) => {
                            for const_def in const_decl.const_def_list {
                                let const_val = const_def.const_init_val.dump(entry, main_data, &symbol_table);
                                let const_name = const_def.id;
                                // Add to symbol table
                                symbol_table.insert(const_name, (const_val, DefType::Const));
                            }
                        },
                        ast::Decl::Var(var_decl) => {
                            for var_def in var_decl.var_def_list {
                                let var_val = main_data.dfg_mut().new_value().alloc(Type::get_i32());
                                let var_name = var_def.id;
                                main_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(var_val).unwrap();
                                // Add to symbol table
                                symbol_table.insert(var_name, (var_val, DefType::Var));
                                if let Some(init_val) = var_def.init_val {
                                    let init_val = init_val.dump(entry, main_data, &symbol_table);
                                    let store = main_data.dfg_mut().new_value().store(init_val, var_val);
                                    main_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(store).unwrap();
                                }
                            }
                        },
                    }
                },
                ast::BlockItem::Stmt(stmt) => {
                    match stmt {
                        ast::Stmt::Assign(lval, exp) => {
                            let val = symbol_table[&lval].0.clone();
                            let exp_val = exp.dump(entry, main_data, &symbol_table);
                            let store = main_data.dfg_mut().new_value().store(exp_val, val);
                            main_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(store).unwrap();
                        },
                        ast::Stmt::Ret(ret) => {
                            let ret_val = ret.dump(entry, main_data, &symbol_table);
                            let ret = main_data.dfg_mut().new_value().ret(Some(ret_val));
                            main_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret).unwrap();
                            // Exit
                            break;
                        },
                    }
                },
            }
        }

        program
    }
}

/* Generate koopa text */
pub fn gen_text_koopa(ast: ast::Program) -> String {
    // Dump, then call koopa lib
    let program = ast.dump();
    let mut gen = KoopaGenerator::new(Vec::new());
    gen.generate_on(&program).unwrap();

    std::str::from_utf8(&gen.writer()).unwrap().to_string()
}

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

/* Generate riscv32 code */
pub fn gen_riscv32(ast: ast::Program) -> String {
    let mut text = ".text\n".to_string();
    text += ".globl main\n";
    let program = ast.dump();
    for &func in program.func_layout() {
        let func_data = program.func(func);
        text += &func_data.name()[1..];
        text += ":\n";

        for (&_bb, node) in func_data.layout().bbs() {
            // Init for bb
            let mut sp_delta = 0 as usize;
            let mut pos = HashMap::new();
            for &inst in node.insts().keys() {
                let value_data = func_data.dfg().value(inst);
                // Only visit alloc, load, binary
                if !value_data.ty().is_unit() {
                    pos.insert(inst, sp_delta);
                    sp_delta += 4;
                }
            }

            println!("sp_delta: {}", sp_delta);
            sp_delta = (sp_delta + 15) / 16 * 16;
            text += &("li t0, -".to_string() + &sp_delta.to_string() + "\n");
            text += "add sp, sp, t0\n";
            
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
                    _ => {
                        panic!("Unknown inst value kind");
                    }
                }
            }
        }
    }
    text
}

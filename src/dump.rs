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
    pub fn dump(self) -> Program {
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

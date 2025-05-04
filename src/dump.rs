/* Uses */
use std::collections::HashMap;
use std::rc::Rc;
use koopa::ir::*;
use koopa::ir::builder::*;
use koopa::back::KoopaGenerator;
use crate::ast;

enum DefType {
    Const,
    Var,
}

struct SymbolTable {
    table: HashMap<String, (Value, DefType)>,
    old: Option<Rc<SymbolTable>>,
}

impl SymbolTable {
    fn new() -> SymbolTable{
        SymbolTable {table: HashMap::new(), old: None}
    }

    fn find(&self, s: &String) -> Option<&(Value, DefType)> {
        match self.table.get(s) {
            Some(tup) => Some(tup),
            None => match &self.old {
                Some(old_table) => old_table.find(s),
                None => None,
            }
        }
    }
}

impl ast::Exp {
    fn dump(self, bb: BasicBlock, data: &mut FunctionData, symbol_table: &Rc<SymbolTable>) -> Value {
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
                let (v, t) = symbol_table.find(&id).unwrap();
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

impl ast::Block {
    fn dump(self, bb: BasicBlock, func_data: &mut FunctionData, mut symbol_table: Rc<SymbolTable>) -> Option<()> {
        for item in self.block_item_list {
            match item {
                ast::BlockItem::Decl(decl) => {
                    match decl {
                        ast::Decl::Const(const_decl) => {
                            for const_def in const_decl.const_def_list {
                                let const_val = const_def.const_init_val.dump(bb, func_data, &symbol_table);
                                let const_name = const_def.id;
                                // Add to symbol table
                                Rc::get_mut(&mut symbol_table).unwrap().table.insert(const_name, (const_val, DefType::Const));
                            }
                        }
                        ast::Decl::Var(var_decl) => {
                            for var_def in var_decl.var_def_list {
                                let var_val = func_data.dfg_mut().new_value().alloc(Type::get_i32());
                                let var_name = var_def.id;
                                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(var_val).unwrap();
                                // Add to symbol table
                                Rc::get_mut(&mut symbol_table).unwrap().table.insert(var_name, (var_val, DefType::Var));
                                if let Some(init_val) = var_def.init_val {
                                    let init_val = init_val.dump(bb, func_data, &symbol_table);
                                    let store = func_data.dfg_mut().new_value().store(init_val, var_val);
                                    func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(store).unwrap();
                                }
                            }
                        }
                    }
                }
                ast::BlockItem::Stmt(stmt) => {
                    match stmt {
                        ast::Stmt::Assign(lval, exp) => {
                            let val = symbol_table.find(&lval).unwrap().0.clone();
                            let exp_val = exp.dump(bb, func_data, &symbol_table);
                            let store = func_data.dfg_mut().new_value().store(exp_val, val);
                            func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(store).unwrap();
                        }
                        ast::Stmt::Block(block) => {
                            let mut new_table = Rc::new(SymbolTable::new());
                            Rc::get_mut(&mut new_table).unwrap().old = Some(Rc::clone(&symbol_table));
                            if let None = block.dump(bb, func_data, new_table) {
                                return None;
                            }
                        }
                        ast::Stmt::Ret(ret) => {
                            let ret_val = match ret {
                                Some(exp) => exp.dump(bb, func_data, &symbol_table),
                                None => func_data.dfg_mut().new_value().integer(0),
                            };
                            let ret = func_data.dfg_mut().new_value().ret(Some(ret_val));
                            func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(ret).unwrap();
                            // Exit
                            return None;
                        }
                        _ => {
                            //do nothing
                        }
                    }
                }
            }
        }
        Some(())
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

        let symbol_table = Rc::new(SymbolTable::new());
        self.func.block.dump(entry, main_data, symbol_table);

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

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
    fn dump(self, mut bb: BasicBlock, func_data: &mut FunctionData, symbol_table: Rc<SymbolTable>) -> (Value, BasicBlock) {
        match *self.core {
            ast::ExpCore::Binary(e0, op, e1) => {
                match op {
                    op @ (BinaryOp::And | BinaryOp::Or) => {
                        // parse e0
                        let zero = func_data.dfg_mut().new_value().integer(0);
                        let (v0, new_bb) = e0.dump(bb, func_data, Rc::clone(&symbol_table));
                        bb = new_bb;

                        // assign value
                        let value = func_data.dfg_mut().new_value().alloc(Type::get_i32());
                        func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(value).unwrap();
                        let assign1 = func_data.dfg_mut().new_value().store(v0, value);
                        func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(assign1).unwrap();
                        // let load = func_data.dfg_mut().new_value().load(value);
                        // func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(load).unwrap();
                        
                        // calc cond
                        let cond = match op {
                            BinaryOp::Or => {
                                let v = func_data.dfg_mut().new_value().binary(BinaryOp::Eq, v0, zero);
                                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(v).unwrap();
                                v
                            }
                            BinaryOp::And => {
                                v0
                            }
                            _ => unreachable!()
                        };
                        

                        // New then bb
                        let then_bb = func_data.dfg_mut().new_bb().basic_block(None);
                        func_data.layout_mut().bbs_mut().push_key_back(then_bb).unwrap();

                        // parse e1
                        let (v1, then_last_bb) = e1.dump(then_bb, func_data, Rc::clone(&symbol_table));

                        // assign value
                        let assign2 = func_data.dfg_mut().new_value().store(v1, value);
                        func_data.layout_mut().bb_mut(then_last_bb).insts_mut().push_key_back(assign2).unwrap();
                        
                        // New end bb
                        let end_bb = func_data.dfg_mut().new_bb().basic_block(None);
                        func_data.layout_mut().bbs_mut().push_key_back(end_bb).unwrap();
                        
                        // br & jump
                        let br = func_data.dfg_mut().new_value().branch(cond, then_bb, end_bb);
                        func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(br).unwrap();
                        let jump = func_data.dfg_mut().new_value().jump(end_bb);
                        func_data.layout_mut().bb_mut(then_last_bb).insts_mut().push_key_back(jump).unwrap();
                        
                        let load = func_data.dfg_mut().new_value().load(value);
                        func_data.layout_mut().bb_mut(end_bb).insts_mut().push_key_back(load).unwrap();
                        (load, end_bb)
                    }
                    op => {
                        let (v0, new_bb) = e0.dump(bb, func_data, Rc::clone(&symbol_table));
                        let (v1, new_bb) = e1.dump(new_bb, func_data, Rc::clone(&symbol_table));
                        let v = func_data.dfg_mut().new_value().binary(op, v0, v1);
                        func_data.layout_mut().bb_mut(new_bb).insts_mut().push_key_back(v).unwrap();
                        (v, new_bb)
                    }
                }
            },
            ast::ExpCore::Single(i) => {
                (func_data.dfg_mut().new_value().integer(i), bb)
            },
            ast::ExpCore::Ident(id) => {
                let (v, t) = symbol_table.find(&id).unwrap();
                match t {
                    DefType::Const => {
                        (v.clone(), bb)
                    },
                    DefType::Var => {
                        let load = func_data.dfg_mut().new_value().load(v.clone());
                        func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(load).unwrap();
                        (load, bb)
                    },
                }
            }
        }
    }
}

impl ast::Stmt {
    fn dump(self, mut bb: BasicBlock, func_data: &mut FunctionData, symbol_table: Rc<SymbolTable>) -> BasicBlock {
        match self {
            ast::Stmt::Assign(lval, exp) => {
                let val = symbol_table.find(&lval).unwrap().0.clone();
                let (exp_val, new_bb) = exp.dump(bb, func_data, Rc::clone(&symbol_table));
                bb = new_bb;
                let store = func_data.dfg_mut().new_value().store(exp_val, val);
                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(store).unwrap();
            }
            ast::Stmt::Block(block) => {
                let mut new_table = Rc::new(SymbolTable::new());
                Rc::get_mut(&mut new_table).unwrap().old = Some(Rc::clone(&symbol_table));
                bb = block.dump(bb, func_data, new_table);
            }
            ast::Stmt::Ret(ret) => {
                let ret = match ret {
                    Some(exp) => {
                        let (ret_value, new_bb) = exp.dump(bb, func_data, Rc::clone(&symbol_table));
                        bb = new_bb;
                        func_data.dfg_mut().new_value().ret(Some(ret_value))
                    }
                    None => func_data.dfg_mut().new_value().ret(None)
                };
                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(ret).unwrap();
                // New a bb
                bb = func_data.dfg_mut().new_bb().basic_block(None);
                func_data.layout_mut().bbs_mut().push_key_back(bb).unwrap();
            }
            ast::Stmt::If(if_stmt) => {
                let (cond, new_bb) = if_stmt.exp.dump(bb, func_data, Rc::clone(&symbol_table));
                bb = new_bb;
                // New then bb
                let then_bb = func_data.dfg_mut().new_bb().basic_block(None);
                func_data.layout_mut().bbs_mut().push_key_back(then_bb).unwrap();
                let then_last_bb = if_stmt.then_stmt.dump(then_bb, func_data, Rc::clone(&symbol_table));
                // New end bb
                let end_bb = func_data.dfg_mut().new_bb().basic_block(None);
                func_data.layout_mut().bbs_mut().push_key_back(end_bb).unwrap();
                
                match if_stmt.else_stmt {
                    Some(else_stmt) => {
                        // New else bb
                        let else_bb = func_data.dfg_mut().new_bb().basic_block(None);
                        func_data.layout_mut().bbs_mut().push_key_back(else_bb).unwrap();
                        let else_last_bb = else_stmt.dump(else_bb, func_data, Rc::clone(&symbol_table));
                        
                        // bb -> then_bb | else_bb
                        let br_then = func_data.dfg_mut().new_value().branch(cond, then_bb, else_bb);
                        func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(br_then).unwrap();

                        // else_last_bb -> end_bb
                        let jump_end = func_data.dfg_mut().new_value().jump(end_bb);
                        func_data.layout_mut().bb_mut(else_last_bb).insts_mut().push_key_back(jump_end).unwrap();
                    },
                    None => {
                        // bb -> then_bb | end_bb
                        let br_then = func_data.dfg_mut().new_value().branch(cond, then_bb, end_bb);
                        func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(br_then).unwrap();
                    }
                }
                // then_last_bb -> end_bb
                let jump_end = func_data.dfg_mut().new_value().jump(end_bb);
                func_data.layout_mut().bb_mut(then_last_bb).insts_mut().push_key_back(jump_end).unwrap();

                bb = end_bb;
            }
            ast::Stmt::While(exp, stmt) => {
                // new exp_bb & body_bb & end_bb
                let exp_bb = func_data.dfg_mut().new_bb().basic_block(None);
                let body_bb = func_data.dfg_mut().new_bb().basic_block(None);
                let end_bb = func_data.dfg_mut().new_bb().basic_block(None);
                func_data.layout_mut().bbs_mut().push_key_back(exp_bb).unwrap();
                func_data.layout_mut().bbs_mut().push_key_back(body_bb).unwrap();
                func_data.layout_mut().bbs_mut().push_key_back(end_bb).unwrap();

                let jump0 = func_data.dfg_mut().new_value().jump(exp_bb);
                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(jump0).unwrap();
                
                let (exp_value, exp_last_bb) = exp.dump(exp_bb, func_data, Rc::clone(&symbol_table));
                let br = func_data.dfg_mut().new_value().branch(exp_value, body_bb, end_bb);
                func_data.layout_mut().bb_mut(exp_last_bb).insts_mut().push_key_back(br).unwrap();

                let body_last_bb = stmt.dump(body_bb, func_data, Rc::clone(&symbol_table));
                let jump = func_data.dfg_mut().new_value().jump(exp_bb);
                func_data.layout_mut().bb_mut(body_last_bb).insts_mut().push_key_back(jump).unwrap();

                bb = end_bb;
            }
            _ => {
                //do nothing
            }
        }
        bb
    }
}

impl ast::Block {
    fn dump(self, mut bb: BasicBlock, func_data: &mut FunctionData, mut symbol_table: Rc<SymbolTable>) -> BasicBlock {

        for item in self.block_item_list {
            match item {
                ast::BlockItem::Decl(decl) => {
                    match decl {
                        ast::Decl::Const(const_decl) => {
                            for const_def in const_decl.const_def_list {
                                let (const_val, new_bb) = const_def.const_init_val.dump(bb, func_data, Rc::clone(&symbol_table));
                                bb = new_bb;
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
                                    let (init_val, new_bb) = init_val.dump(bb, func_data, Rc::clone(&symbol_table));
                                    bb = new_bb;
                                    let store = func_data.dfg_mut().new_value().store(init_val, var_val);
                                    func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(store).unwrap();
                                }
                            }
                        }
                    }
                }
                ast::BlockItem::Stmt(stmt) => {
                    bb = stmt.dump(bb, func_data, Rc::clone(&symbol_table));
                }
            }
        }
        bb
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
        
        let symbol_table = Rc::new(SymbolTable::new());
        let entry = main_data.dfg_mut().new_bb().basic_block(Some("%entry".to_string()));
        main_data.layout_mut().bbs_mut().push_key_back(entry).unwrap();
        let last_bb = self.func.block.dump(entry, main_data, symbol_table);
        let ret = main_data.dfg_mut().new_value().ret(None);
        main_data.layout_mut().bb_mut(last_bb).insts_mut().push_key_back(ret).unwrap();

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

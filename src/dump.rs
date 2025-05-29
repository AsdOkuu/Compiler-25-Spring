/* Uses */
use std::collections::HashMap;
use std::iter::zip;
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
    fn dump(self, mut bb: BasicBlock, func_data: &mut FunctionData, symbol_table: Rc<SymbolTable>, func_table: &HashMap<String, Function>) -> (Value, BasicBlock) {
        match *self.core {
            ast::ExpCore::Binary(e0, op, e1) => {
                match op {
                    op @ (BinaryOp::And | BinaryOp::Or) => {
                        // parse e0
                        let zero = func_data.dfg_mut().new_value().integer(0);
                        let (v0, new_bb) = e0.dump(bb, func_data, Rc::clone(&symbol_table), func_table);
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
                        let (v1, then_last_bb) = e1.dump(then_bb, func_data, Rc::clone(&symbol_table), func_table);

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
                        let (v0, new_bb) = e0.dump(bb, func_data, Rc::clone(&symbol_table), func_table);
                        let (v1, new_bb) = e1.dump(new_bb, func_data, Rc::clone(&symbol_table), func_table);
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
            },
            ast::ExpCore::Call(id, param_list) => {
                let mut bb = bb;
                let mut params = vec![];
                for exp in param_list.into_iter() {
                    let (value, new_bb) = exp.dump(bb, func_data, Rc::clone(&symbol_table), func_table);
                    bb = new_bb;
                    params.push(value);
                }

                let func = func_table[&id];
                let call = func_data.dfg_mut().new_value().call(func, params);
                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(call).unwrap();

                (call, bb)
            }
        }
    }
}

#[derive(Clone, Copy)]
struct WhileInfo {
    exp_bb: BasicBlock,
    end_bb: BasicBlock,
}

impl ast::Stmt {
    fn dump(self, mut bb: BasicBlock, func_data: &mut FunctionData, symbol_table: Rc<SymbolTable>, while_info: Option<WhileInfo>, func_table: &HashMap<String, Function>) -> BasicBlock {
        match self {
            ast::Stmt::Exp(exp) => {
                let (_, new_bb) = exp.dump(bb, func_data, symbol_table, func_table);
                bb = new_bb;
            }
            ast::Stmt::Assign(lval, exp) => {
                let val = symbol_table.find(&lval).unwrap().0.clone();
                let (exp_val, new_bb) = exp.dump(bb, func_data, Rc::clone(&symbol_table), func_table);
                bb = new_bb;
                let store = func_data.dfg_mut().new_value().store(exp_val, val);
                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(store).unwrap();
            }
            ast::Stmt::Block(block) => {
                let mut new_table = Rc::new(SymbolTable::new());
                Rc::get_mut(&mut new_table).unwrap().old = Some(Rc::clone(&symbol_table));
                bb = block.dump(bb, func_data, new_table, while_info, func_table);
            }
            ast::Stmt::Ret(ret) => {
                let ret = match ret {
                    Some(exp) => {
                        let (ret_value, new_bb) = exp.dump(bb, func_data, Rc::clone(&symbol_table), func_table);
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
                let (cond, new_bb) = if_stmt.exp.dump(bb, func_data, Rc::clone(&symbol_table), func_table);
                bb = new_bb;
                // New then bb
                let then_bb = func_data.dfg_mut().new_bb().basic_block(None);
                func_data.layout_mut().bbs_mut().push_key_back(then_bb).unwrap();
                let then_last_bb = if_stmt.then_stmt.dump(then_bb, func_data, Rc::clone(&symbol_table), while_info, func_table);
                // New end bb
                let end_bb = func_data.dfg_mut().new_bb().basic_block(None);
                func_data.layout_mut().bbs_mut().push_key_back(end_bb).unwrap();
                
                match if_stmt.else_stmt {
                    Some(else_stmt) => {
                        // New else bb
                        let else_bb = func_data.dfg_mut().new_bb().basic_block(None);
                        func_data.layout_mut().bbs_mut().push_key_back(else_bb).unwrap();
                        let else_last_bb = else_stmt.dump(else_bb, func_data, Rc::clone(&symbol_table), while_info, func_table);
                        
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
                
                let (exp_value, exp_last_bb) = exp.dump(exp_bb, func_data, Rc::clone(&symbol_table), func_table);
                let br = func_data.dfg_mut().new_value().branch(exp_value, body_bb, end_bb);
                func_data.layout_mut().bb_mut(exp_last_bb).insts_mut().push_key_back(br).unwrap();

                let body_last_bb = stmt.dump(body_bb, func_data, Rc::clone(&symbol_table), Some(WhileInfo { exp_bb, end_bb }), func_table);
                let jump = func_data.dfg_mut().new_value().jump(exp_bb);
                func_data.layout_mut().bb_mut(body_last_bb).insts_mut().push_key_back(jump).unwrap();

                bb = end_bb;
            }
            ast::Stmt::Continue => {
                match while_info {
                    Some(while_info) => {
                        let jump = func_data.dfg_mut().new_value().jump(while_info.exp_bb);
                        func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(jump).unwrap();

                        bb = func_data.dfg_mut().new_bb().basic_block(None);
                        func_data.layout_mut().bbs_mut().push_key_back(bb).unwrap();
                    }
                    None => panic!()
                }
            }
            ast::Stmt::Break => {
                match while_info {
                    Some(while_info) => {
                        let jump = func_data.dfg_mut().new_value().jump(while_info.end_bb);
                        func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(jump).unwrap();

                        bb = func_data.dfg_mut().new_bb().basic_block(None);
                        func_data.layout_mut().bbs_mut().push_key_back(bb).unwrap();
                    }
                    None => panic!()
                }
            }
            _ => {
                //do nothing
            }
        }
        bb
    }
}

impl ast::Block {
    fn dump(self, mut bb: BasicBlock, func_data: &mut FunctionData, mut symbol_table: Rc<SymbolTable>, while_info: Option<WhileInfo>, func_table: &HashMap<String, Function>) -> BasicBlock {

        for item in self.block_item_list {
            match item {
                ast::BlockItem::Decl(decl) => {
                    match decl {
                        ast::Decl::Const(const_decl) => {
                            for const_def in const_decl.const_def_list {
                                let (const_val, new_bb) = const_def.const_init_val.dump(bb, func_data, Rc::clone(&symbol_table), func_table);
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
                                    let (init_val, new_bb) = init_val.dump(bb, func_data, Rc::clone(&symbol_table), func_table);
                                    bb = new_bb;
                                    let store = func_data.dfg_mut().new_value().store(init_val, var_val);
                                    func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(store).unwrap();
                                }
                            }
                        }
                    }
                }
                ast::BlockItem::Stmt(stmt) => {
                    bb = stmt.dump(bb, func_data, Rc::clone(&symbol_table), while_info, func_table);
                }
            }
        }
        bb
    }
}

impl ast::FuncDef {
    pub fn dump(self, func_data: &mut FunctionData, func_table: &HashMap<String, Function>) {
        let mut symbol_table = Rc::new(SymbolTable::new());
        let entry = func_data.dfg_mut().new_bb().basic_block(Some("%entry".to_string()));
        func_data.layout_mut().bbs_mut().push_key_back(entry).unwrap();

        let params = func_data.params().to_vec();
        for (func_param, value) in zip(self.func_param_list, params) {
            let alloc = func_data.dfg_mut().new_value().alloc(Type::get_i32());
            func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(alloc).unwrap();
            let assign = func_data.dfg_mut().new_value().store(value, alloc);
            func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(assign).unwrap();
            Rc::get_mut(&mut symbol_table).unwrap().table.insert(func_param.0, (alloc, DefType::Var));
        }

        let last_bb = self.block.dump(entry, func_data, symbol_table, None, func_table);
        let zero = func_data.dfg_mut().new_value().integer(0);
        let ret = func_data.dfg_mut().new_value().ret(Some(zero));
        func_data.layout_mut().bb_mut(last_bb).insts_mut().push_key_back(ret).unwrap();
    }
}

fn push_runtime_func(program: &mut Program, func_table: &mut HashMap<String, Function>) {
    let getint = program.new_func(FunctionData::new(
        "@getint".to_string(),
        vec![],
        Type::get_i32()
    ));
    let getch = program.new_func(FunctionData::new(
        "@getch".to_string(),
        vec![],
        Type::get_i32()
    ));
    let getarray = program.new_func(FunctionData::new(
        "@getarray".to_string(),
        vec![Type::get_pointer(Type::get_i32())],
        Type::get_i32()
    ));
    let putint = program.new_func(FunctionData::new(
        "@putint".to_string(),
        vec![Type::get_i32()],
        Type::get_unit()
    ));
    let putch = program.new_func(FunctionData::new(
        "@putch".to_string(),
        vec![Type::get_i32()],
        Type::get_unit()
    ));
    let putarray = program.new_func(FunctionData::new(
        "@putarray".to_string(),
        vec![Type::get_i32(), Type::get_pointer(Type::get_i32())],
        Type::get_unit()
    ));
    let starttime = program.new_func(FunctionData::new(
        "@starttime".to_string(),
        vec![],
        Type::get_unit()
    ));
    let stoptime = program.new_func(FunctionData::new(
        "@stoptime".to_string(),
        vec![],
        Type::get_unit()
    ));
    
    func_table.insert("getint".to_string(), getint);
    func_table.insert("getch".to_string(), getch);
    func_table.insert("getarray".to_string(), getarray);
    func_table.insert("putint".to_string(), putint);
    func_table.insert("putch".to_string(), putch);
    func_table.insert("putarray".to_string(), putarray);
    func_table.insert("starttime".to_string(), starttime);
    func_table.insert("stoptime".to_string(), stoptime);
}

impl ast::Program {
    /* Dump prog into koopa */
    pub fn dump(self) -> Program {
        let mut program = Program::new();
        let mut func_table = HashMap::new();

        push_runtime_func(&mut program, &mut func_table);
        
        for func_def in self.func_list.iter() {
            
            let mut param_ty = Vec::new();
            for param in func_def.func_param_list.iter() {
                param_ty.push((Some("@".to_owned() + &param.0), Type::get_i32()));
            }
            let ret_ty = match func_def.func_type {
                ast::FuncType::Int => Type::get_i32(),
                ast::FuncType::Void => Type::get_unit(),
            };
            let func = program.new_func(
                FunctionData::with_param_names(("@".to_owned() + &func_def.id).into(), param_ty, ret_ty),
            );
            

            func_table.insert(func_def.id.clone(), func);
        }

        for func_def in self.func_list.into_iter() {
            let name = func_def.id.clone();
            println!("{} dumping", name);
            func_def.dump(program.func_mut(func_table[&name]), &func_table);
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

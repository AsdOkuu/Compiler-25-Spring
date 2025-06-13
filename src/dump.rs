/* Uses */
use std::collections::HashMap;
use std::iter::zip;
use std::rc::Rc;
use koopa::ir::*;
use koopa::ir::builder::*;
use koopa::back::KoopaGenerator;
use crate::ast;

struct SymbolTable {
    table: HashMap<String, (Value, usize, bool)>,
    old: Option<Rc<SymbolTable>>,
    const_table: HashMap<String, i32>
}

impl SymbolTable {
    fn new() -> SymbolTable{
        SymbolTable {table: HashMap::new(), old: None, const_table: HashMap::new()}
    }

    fn find(&self, s: &String) -> Option<(Value, usize, bool)> {
        match self.table.get(s) {
            Some(res) => Some(*res),
            None => match &self.old {
                Some(old_table) => old_table.find(s),
                None => None,
            }
        }
    }

    fn find_const(&self, s: &String) -> Option<i32> {
        match self.const_table.get(s) {
            Some(i) => Some(*i),
            None => match &self.old {
                Some(old_table) => old_table.find_const(s),
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
            ast::ExpCore::Ident(lval) => {
                let (v, dim, is_ptr) = symbol_table.find(&lval.id).unwrap();

                let is_partial = lval.is_array.len() < dim;
                let to_get = is_ptr && lval.is_array.len() == 0;

                println!("{}", is_ptr);
                // func_data.dfg_mut().values().get(&v).unwrap();
                let (ptr, new_bb) = get_array_ptr(v, is_ptr, lval.is_array, bb, func_data, Rc::clone(&symbol_table), func_table);
                bb = new_bb;

                // func_data.dfg_mut().values().get(&ptr).unwrap();
                // if let TypeKind::Pointer(ty) = func_data.dfg_mut().value(ptr).ty().kind() {
                //     if let TypeKind::Array(_, _) = ty.kind() {
                //         return (ptr, bb);
                //     }
                // }

                if is_partial {
                    let zero = func_data.dfg_mut().new_value().integer(0);
                    let value = if to_get {
                        func_data.dfg_mut().new_value().get_ptr(ptr, zero)
                    }else {
                        func_data.dfg_mut().new_value().get_elem_ptr(ptr, zero)
                    };
                    func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(value).unwrap();
                    return (value, bb);
                }
                

                let load = func_data.dfg_mut().new_value().load(ptr);
                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(load).unwrap();

                (load, bb)
            },
            ast::ExpCore::Call(id, param_list) => {
                let mut bb = bb;
                let mut params = vec![];
                for exp in param_list.into_iter() {
                    let (value, new_bb) = exp.dump(bb, func_data, Rc::clone(&symbol_table), func_table);
                    bb = new_bb;

                    // func_data.dfg_mut().values().get(&value).unwrap();
                    // if let TypeKind::Pointer(_) = func_data.dfg_mut().value(value).ty().kind() {
                    //     let zero = func_data.dfg_mut().new_value().integer(0);
                    //     value = func_data.dfg_mut().new_value().get_elem_ptr(value, zero);
                    //     func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(value).unwrap();
                    // }
                    
                    params.push(value);
                }

                let func = func_table[&id];
                let call = func_data.dfg_mut().new_value().call(func, params);
                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(call).unwrap();

                (call, bb)
            }
        }
    }

    fn dump_const(self, symbol_table: Rc<SymbolTable>) -> i32 {
        match *self.core {
            ast::ExpCore::Single(i) => i,
            ast::ExpCore::Ident(id) => {
                symbol_table.find_const(&id.id).unwrap()
            },
            ast::ExpCore::Binary(e0, op, e1) => {
                let x = e0.dump_const(Rc::clone(&symbol_table));
                let y = e1.dump_const(Rc::clone(&symbol_table));
                match op {
                    BinaryOp::Add => x + y,
                    BinaryOp::Sub => x - y,
                    BinaryOp::Mul => x * y,
                    BinaryOp::Div => x / y,
                    BinaryOp::Mod => x % y,
                    BinaryOp::And => {
                        if x & y == 0 {
                            0
                        }else {
                            1
                        }
                    }
                    BinaryOp::Or => {
                        if x | y == 0 {
                            0
                        }else {
                            1
                        }
                    }
                    BinaryOp::Eq => {
                        if x == y {
                            1
                        }else {
                            0
                        }
                    }
                    BinaryOp::NotEq => {
                        if x == y {
                            0
                        }else {
                            1
                        }
                    }
                    BinaryOp::Ge => {
                        if x >= y {
                            1
                        }else {
                            0
                        }
                    }
                    BinaryOp::Le => {
                        if x <= y {
                            1
                        }else {
                            0
                        }
                    }
                    BinaryOp::Gt => {
                        if x > y {
                            1
                        }else {
                            0
                        }
                    }
                    BinaryOp::Lt => {
                        if x < y {
                            1
                        }else {
                            0
                        }
                    }
                    _ => unreachable!()
                }
            }
            _ => unreachable!()
        }
    }
}

#[derive(Clone, Copy)]
struct WhileInfo {
    exp_bb: BasicBlock,
    end_bb: BasicBlock,
}

fn get_array_ptr(mut value: Value, mut is_ptr: bool, list: Vec<ast::Exp>, mut bb: BasicBlock, func_data: &mut FunctionData, symbol_table: Rc<SymbolTable>, func_table: &HashMap<String, Function>) -> (Value, BasicBlock) {
    if is_ptr {
        value = func_data.dfg_mut().new_value().load(value);
        func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(value).unwrap();
    }
    for exp in list {
        let (index, new_bb) = exp.dump(bb, func_data, Rc::clone(&symbol_table), &func_table);
        bb = new_bb;

        if is_ptr {
            value = func_data.dfg_mut().new_value().get_ptr(value, index);
            is_ptr = false;
        }else {
            value = func_data.dfg_mut().new_value().get_elem_ptr(value, index);
        }
        func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(value).unwrap();
    }
    (value, bb)
}

impl ast::Stmt {
    fn dump(self, mut bb: BasicBlock, func_data: &mut FunctionData, symbol_table: Rc<SymbolTable>, while_info: Option<WhileInfo>, func_table: &HashMap<String, Function>) -> BasicBlock {
        match self {
            ast::Stmt::Exp(exp) => {
                let (_, new_bb) = exp.dump(bb, func_data, symbol_table, func_table);
                bb = new_bb;
            }
            ast::Stmt::Assign(lval, exp) => {
                let (dest, _, is_ptr) = symbol_table.find(&lval.id).unwrap();
                let (exp_val, new_bb) = exp.dump(bb, func_data, Rc::clone(&symbol_table), func_table);
                bb = new_bb;
                // func_data.dfg_mut().values().get(&dest).unwrap();
                let (ptr, new_bb) = get_array_ptr(dest, is_ptr, lval.is_array, bb, func_data, Rc::clone(&symbol_table), func_table);
                bb = new_bb;

                let store = func_data.dfg_mut().new_value().store(exp_val, ptr);
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

fn build_value_local(vree: &Vree, ptr: Value, func_data: &mut FunctionData, bb: BasicBlock) {
    match vree {
        Vree::Leaf(Some(value)) => {
            let store = func_data.dfg_mut().new_value().store(*value, ptr);
            func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(store).unwrap();
        }
        Vree::Sub(list) => {
            for i in 0..list.len() {
                let index = func_data.dfg_mut().new_value().integer(i as i32);
                let get = func_data.dfg_mut().new_value().get_elem_ptr(ptr, index);
                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(get).unwrap();
                build_value_local(&list[i], get, func_data, bb);
            }
        }
        _ => {
            // donothing
        }
    }
}

fn adj_tree(arr: &Tree, func_data: &mut FunctionData) -> Vree {
    match arr {
        Tree::Leaf(i) => {
            let v = func_data.dfg_mut().new_value().integer(*i);
            Vree::Leaf(Some(v))
        },
        Tree::Sub(list) => {
            let mut vlist = vec![];
            for t in list {
                vlist.push(adj_tree(t, func_data));
            }
            Vree::Sub(vlist)
        }
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
                                let mut index = vec![];
                                for exp in const_def.is_array {
                                    let i = exp.dump_const(Rc::clone(&symbol_table)) as usize;
                                    index.push(i);
                                }

                                let len = index.len();

                                let mut ty = Type::get_i32();
                                for i in index.iter().rev() {
                                    ty = Type::get_array(ty, *i);
                                }

                                let arr = const_def.const_init_val.dump_global(index, Rc::clone(&symbol_table));

                                let alloc = func_data.dfg_mut().new_value().alloc(ty);
                                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(alloc).unwrap();

                                let vree = adj_tree(&arr, func_data);
                                build_value_local(&vree, alloc, func_data, bb);
                                Rc::get_mut(&mut symbol_table).unwrap().table.insert(const_def.id.clone(), (alloc, len, false));

                                if let Tree::Leaf(i) = arr {
                                    Rc::get_mut(&mut symbol_table).unwrap().const_table.insert(const_def.id.clone(), i);
                                }
                            }
                        }
                        ast::Decl::Var(var_decl) => {
                            for var_def in var_decl.var_def_list {
                                let mut index = vec![];
                                for exp in var_def.is_array {
                                    let i = exp.dump_const(Rc::clone(&symbol_table)) as usize;
                                    index.push(i);
                                }

                                let len = index.len();

                                let mut ty = Type::get_i32();
                                for i in index.iter().rev() {
                                    ty = Type::get_array(ty, *i);
                                }

                                let alloc = func_data.dfg_mut().new_value().alloc(ty);
                                func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(alloc).unwrap();

                                if let Some(init_val) = var_def.init_val {
                                    let (vree, new_bb) = init_val.dump_local(index, bb, Rc::clone(&symbol_table), func_data, func_table);
                                    bb = new_bb;

                                    build_value_local(&vree, alloc, func_data, bb);
                                }
                                Rc::get_mut(&mut symbol_table).unwrap().table.insert(var_def.id.clone(), (alloc, len, false));
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
    fn dump(self, func_data: &mut FunctionData, func_table: &HashMap<String, Function>, old_symbol_table: Rc<SymbolTable>) {
        let mut symbol_table = Rc::new(SymbolTable::new());
        Rc::get_mut(&mut symbol_table).unwrap().old = Some(old_symbol_table);
        let entry = func_data.dfg_mut().new_bb().basic_block(Some("%entry".to_string()));
        func_data.layout_mut().bbs_mut().push_key_back(entry).unwrap();

        let params = func_data.params().to_vec();
        let param_kind = if let TypeKind::Function(p, _) = func_data.ty().kind() {
            p.clone()
        }else {
            panic!()
        };
        for (func_param, (value, ty)) in zip(self.func_param_list, zip(params, param_kind)) {
            let alloc = func_data.dfg_mut().new_value().alloc(ty);
            func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(alloc).unwrap();
            let assign = func_data.dfg_mut().new_value().store(value, alloc);
            func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(assign).unwrap();

            let len = func_param.1.len();
            if len == 0 {
                println!("{}", 1);
                Rc::get_mut(&mut symbol_table).unwrap().table.insert(func_param.0, (alloc, 0, false));
            }else {
                println!("{}", 2);
                Rc::get_mut(&mut symbol_table).unwrap().table.insert(func_param.0, (alloc, len, true));
            }
        }

        let last_bb = self.block.dump(entry, func_data, symbol_table, None, func_table);
        
        let ret = match func_data.ty().kind() {
            TypeKind::Function(_, ret_type) => {
                if ret_type.is_unit() {
                    func_data.dfg_mut().new_value().ret(None)
                }else {
                    let zero = func_data.dfg_mut().new_value().integer(0);
                    func_data.dfg_mut().new_value().ret(Some(zero))
                }
            }
            _ => unreachable!()
        };
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

fn union_set(size: &Vec<usize>, arr_set: &mut Vec<Vec<Tree>>) {
    for i in 0..size.len() {
        let len = size[size.len() - i - 1];
        while arr_set[i].len() >= len {
            // println!("!{} {} {}", size.len(), i, len);
            let mut set = vec![];
            for _ in 0..len {
                set.push(arr_set[i].remove(0));
            }
            let arr = Tree::Sub(set);
            arr_set[i + 1].push(arr);
        }
    }
}

fn union_set_local(size: &Vec<usize>, arr_set: &mut Vec<Vec<Vree>>) {
    for i in 0..size.len() {
        let len = size[size.len() - i - 1];
        while arr_set[i].len() >= len {
            let mut set = vec![];
            for _ in 0..len {
                set.push(arr_set[i].remove(0));
            }
            let arr = Vree::Sub(set);
            arr_set[i + 1].push(arr);
        }
    }
}

#[derive(Clone, Debug)]
enum Tree {
    Sub(Vec<Tree>),
    Leaf(i32)
}

#[derive(Clone)]
enum Vree {
    Sub(Vec<Vree>),
    Leaf(Option<Value>)
}

impl ast::InitVal {
    fn dump_local(self, size: Vec<usize>, mut bb: BasicBlock, symbol_table: Rc<SymbolTable>, func_data: &mut FunctionData, func_table: &HashMap<String, Function>) -> (Vree, BasicBlock) {
        match self {
            ast::InitVal::Exp(exp) => {
                // ArrayD::from_elem(vec![1], exp.dump_const(Rc::clone(&symbol_table)))
                let (value, new_bb) = exp.dump(bb, func_data, Rc::clone(&symbol_table), func_table);
                (Vree::Leaf(Some(value)), new_bb)
            },
            ast::InitVal::List(list) => {
                let mut arr_set = vec![vec![]; size.len() + 1];
                for init_val in list {
                    // Union arr set
                    union_set_local(&size, &mut arr_set);

                    // Divide
                    match &*init_val {
                        ast::InitVal::Exp(_) => {
                            let (vree, new_bb) = init_val.dump_local(vec![], bb, Rc::clone(&symbol_table), func_data, func_table);
                            arr_set[0].push(vree);
                            bb = new_bb;
                        },
                        ast::InitVal::List(_) => {
                            let mut dim = size.len() - 1;
                            for i in 0..arr_set.len() {
                                if !arr_set[i].is_empty() {
                                    dim = i;
                                    break;
                                }
                            }
                            let start = size.len() - dim;
                            let back = size[start..].to_vec();
                            let (vree, new_bb) = init_val.dump_local(back, bb, Rc::clone(&symbol_table), func_data, func_table);
                            arr_set[dim].push(vree);
                            bb = new_bb;
                        }
                    }
                }
                // Count & Push
                let mut s = 1 as usize;
                let mut count = 0 as usize;
                for i in 0..arr_set.len() {
                    if i > 0 {
                        s *= size[size.len() - i];
                    }
                    count += s * arr_set[i].len();
                }
                if count < s {
                    for _ in 0..s-count {
                        let zero = func_data.dfg_mut().new_value().integer(0);
                        arr_set[0].push(Vree::Leaf(Some(zero)));
                    }
                }
                // Union
                union_set_local(&size, &mut arr_set);

                (arr_set[size.len()].remove(0), bb)
            },
        }
    }

    fn dump_global(self, size: Vec<usize>, symbol_table: Rc<SymbolTable>) -> Tree {
        // println!("{}", size.len());
        match self {
            ast::InitVal::Exp(exp) => {
                // ArrayD::from_elem(vec![1], exp.dump_const(Rc::clone(&symbol_table)))
                let i = exp.dump_const(Rc::clone(&symbol_table));
                println!("= {}", i);
                Tree::Leaf(i)
            },
            ast::InitVal::List(list) => {
                let mut arr_set = vec![vec![]; size.len() + 1];
                for init_val in list {
                    // Union arr set
                    union_set(&size, &mut arr_set);

                    // Divide
                    match &*init_val {
                        ast::InitVal::Exp(_) => {
                            arr_set[0].push(init_val.dump_global(vec![], Rc::clone(&symbol_table)));
                        },
                        ast::InitVal::List(_) => {
                            let mut dim = size.len() - 1;
                            println!("??{}", dim);
                            for i in 0..arr_set.len() {
                                if !arr_set[i].is_empty() {
                                    dim = i;
                                    break;
                                }
                            }
                            println!("?{}", dim);
                            let start = size.len() - dim;
                            let back = size[start..].to_vec();
                            let arr = init_val.dump_global(back, Rc::clone(&symbol_table));
                            arr_set[dim].push(arr);
                        }
                    }
                }
                // Count & Push
                let mut s = 1 as usize;
                let mut count = 0 as usize;
                for i in 0..arr_set.len() {
                    if i > 0 {
                        s *= size[size.len() - i];
                    }
                    count += s * arr_set[i].len();
                }
                println!("size: {}, count: {}, s: {}", size.len(), count, s);
                if count < s {
                    for _ in 0..s-count {
                        arr_set[0].push(Tree::Leaf(0));
                    }
                }
                // Union
                union_set(&size, &mut arr_set);

                arr_set[size.len()].remove(0)
            },
        }
    }
}

fn build_value(arr: Tree, program: &mut Program) -> Value {
    match arr {
        Tree::Leaf(i) => {
            program.new_value().integer(i)
            // program.new_value().global_alloc(int)
        },
        Tree::Sub(list) => {
            let mut value_list = vec![];
            for t in list {
                let v = build_value(t, program);
                value_list.push(v);
            }
            program.new_value().aggregate(value_list)
            // program.new_value().global_alloc(agg)
        }
    }
}

impl ast::Program {
    /* Dump prog into koopa */
    pub fn dump(self) -> Program {
        let mut program = Program::new();
        let mut func_table = HashMap::new();
        let mut symbol_table = Rc::new(SymbolTable::new());

        push_runtime_func(&mut program, &mut func_table);
        
        let mut func_list = vec![];

        for def in self.list {
            match def {
                Ok(mut func_def) => {
                    let mut param_ty = Vec::new();
                    let param_list = func_def.func_param_list;
                    func_def.func_param_list = vec![];
                    for param in param_list {
                        let fake_exp_list = vec![ast::Exp::single(0); param.1.len()];
                        func_def.func_param_list.push(ast::FuncParam(param.0, fake_exp_list));
                        // param_ty.push((Some("@".to_owned() + &param.0), Type::get_i32()));
                        let mut ty = Type::get_i32();
                        let mut list = param.1;
                        if list.len() > 0 {
                            list.reverse();
                            list.pop().unwrap();
                            for exp in list {
                                let i = exp.dump_const(Rc::clone(&symbol_table));
                                ty = Type::get_array(ty, i as usize);
                            }
                            ty = Type::get_pointer(ty);
                        }
                        param_ty.push((None, ty));
                    }
                    let ret_ty = match func_def.func_type {
                        ast::FuncType::Int => Type::get_i32(),
                        ast::FuncType::Void => Type::get_unit(),
                    };
                    let func = program.new_func(
                        FunctionData::with_param_names(("@".to_owned() + &func_def.id).into(), param_ty, ret_ty),
                    );
                    func_table.insert(func_def.id.clone(), func);
                    func_list.push(func_def);
                },
                Err(decl) => {
                    match decl {
                        ast::Decl::Const(const_decl) => {
                            for const_def in const_decl.const_def_list {
                                let mut index = vec![];
                                for exp in const_def.is_array {
                                    let i = exp.dump_const(Rc::clone(&symbol_table)) as usize;
                                    index.push(i);
                                }

                                let len = index.len();

                                let arr = const_def.const_init_val.dump_global(index, Rc::clone(&symbol_table));

                                if let Tree::Leaf(i) = arr {
                                    Rc::get_mut(&mut symbol_table).unwrap().const_table.insert(const_def.id.clone(), i);
                                }

                                let value = build_value(arr, &mut program);
                                let alloc = program.new_value().global_alloc(value);
                                Rc::get_mut(&mut symbol_table).unwrap().table.insert(const_def.id.clone(), (alloc, len, false));
                            }
                        }
                        ast::Decl::Var(var_decl) => {
                            for var_def in var_decl.var_def_list {
                                let mut index = vec![];
                                for exp in var_def.is_array {
                                    let i = exp.dump_const(Rc::clone(&symbol_table)) as usize;
                                    index.push(i);
                                }

                                let len = index.len();

                                println!("len: {}", len);

                                let arr = match var_def.init_val {
                                    Some(init_val) => init_val.dump_global(index, Rc::clone(&symbol_table)),
                                    None => {
                                        if index.len() == 0 {
                                            Tree::Leaf(0)
                                        }else {
                                            ast::InitVal::List(vec![]).dump_global(index, Rc::clone(&symbol_table))
                                        }
                                    }
                                };

                                
                                
                                if let Tree::Leaf(i) = arr {
                                    Rc::get_mut(&mut symbol_table).unwrap().const_table.insert(var_def.id.clone(), i);
                                }

                                let value = build_value(arr, &mut program);
                                let alloc = program.new_value().global_alloc(value);
                                Rc::get_mut(&mut symbol_table).unwrap().table.insert(var_def.id.clone(), (alloc, len, false));
                            }
                        }
                    }
                }
            }
        }

        for func_def in func_list {
            let name = func_def.id.clone();
            // println!("{} dumping", name);
            func_def.dump(program.func_mut(func_table[&name]), &func_table, Rc::clone(&symbol_table));
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

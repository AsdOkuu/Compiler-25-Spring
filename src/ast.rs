use koopa::ir::BinaryOp;

#[derive(Debug)]
pub struct Program {
    pub func: FuncDef,
}

#[derive(Debug)]
pub struct FuncDef {
    pub func_type: FuncType,
    pub id: String,
    pub block: Block,
}

#[derive(Debug)]
pub struct Block {
    pub block_item_list: Vec<BlockItem>,
}

#[derive(Debug)]
pub enum BlockItem {
    Decl(Decl),
    Stmt(Stmt),
}

#[derive(Debug)]
pub enum Stmt {
    Assign(String, Exp),
    Exp(Exp),
    Block(Block),
    Ret(Option<Exp>),
    If(If),
    While(Exp, Box<Stmt>),
    Blank,
}

#[derive(Debug)]
pub struct If {
    pub exp: Exp,
    pub then_stmt: Box<Stmt>,
    pub else_stmt: Option<Box<Stmt>>,
}

impl If {
    pub fn single(exp: Exp, stmt: Stmt) -> If {
        If { exp, then_stmt: Box::new(stmt), else_stmt: None }
    }

    pub fn complex(exp: Exp, then_stmt: Stmt, else_stmt: Stmt) -> If {
        If { exp, then_stmt: Box::new(then_stmt), else_stmt: Some(Box::new(else_stmt)) }
    }
}

#[derive(Debug)]
pub enum FuncType {
    Int,
}

#[derive(Debug)]
pub enum Decl {
    Const(ConstDecl),
    Var(VarDecl),
}

#[derive(Debug)]
pub struct VarDecl {
    pub btype: BType,
    pub var_def_list: Vec<VarDef>,
}

#[derive(Debug)]
pub struct VarDef {
    pub id: String,
    pub init_val: Option<Exp>,
}

#[derive(Debug)]
pub struct ConstDecl {
    pub btype: BType,
    pub const_def_list: Vec<ConstDef>,
}

#[derive(Debug)]
pub enum BType {
    Int,
}

#[derive(Debug)]
pub struct ConstDef {
    pub id: String,
    pub const_init_val: Exp,
}

#[derive(Debug)]
pub struct Exp {
    pub core: Box<ExpCore>,
}

#[derive(Debug)]
pub enum ExpCore {
    Binary(Exp, BinaryOp, Exp),
    Single(i32),
    Ident(String),
}

impl Exp {
    pub fn binary(e0: Exp, op: BinaryOp, e1: Exp) -> Exp {
        Exp { core: Box::new(ExpCore::Binary(e0, op, e1)), }
    }

    pub fn single(num: i32) -> Exp {
        Exp { core: Box::new(ExpCore::Single(num)), }
    }

    pub fn ident(id: String) -> Exp {
        Exp { core: Box::new(ExpCore::Ident(id)), }
    }
}

/*
pub fn to_unary(op: Op, e: Exp) -> Exp {
    let c = match op {
        Op::Add | Op::Sub => 0,
        Op::Lxor => 1,
        _ => unreachable!(),
    };
    Exp::binary(Exp::single(c), op, e)
}
*/

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
    pub stmt: Stmt,
}

#[derive(Debug)]
pub enum Stmt {
    Ret(Exp),
}

#[derive(Debug)]
pub enum FuncType {
    Int,
}

#[derive(Debug)]
pub struct Exp {
    pub core: Box<ExpCore>,
}

#[derive(Debug)]
pub enum ExpCore {
    Binary(Exp, BinaryOp, Exp),
    Single(i32),
}

impl Exp {
    pub fn binary(e0: Exp, op: BinaryOp, e1: Exp) -> Exp {
        Exp { core: Box::new(ExpCore::Binary(e0, op, e1)), }
    }

    pub fn single(num: i32) -> Exp {
        Exp { core: Box::new(ExpCore::Single(num)), }
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

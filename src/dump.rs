/* Uses */
use std::collections::HashSet;
use crate::ast;
use koopa::ir::*;
use koopa::ir::builder::*;
use koopa::back::KoopaGenerator;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

impl ast::Exp {
    fn dump(self, bb: BasicBlock, data: &mut FunctionData) -> Value {
        match *self.core {
            ast::ExpCore::Binary(e0, op, e1) => {
                let v0 = e0.dump(bb, data);
                let v1 = e1.dump(bb, data);
                let v = data.dfg_mut().new_value().binary(op, v0, v1);
                data.layout_mut().bb_mut(bb).insts_mut().push_key_back(v).unwrap();
                v
            },
            ast::ExpCore::Single(i) => {
                data.dfg_mut().new_value().integer(i)
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
        let ret_exp = match self.func.block.stmt {
            ast::Stmt::Ret(exp) => exp,
        };
        let ret_val = ret_exp.dump(entry, main_data);
        let ret = main_data.dfg_mut().new_value().ret(Some(ret_val));
        main_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret).unwrap();

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
enum Reg {
    T0, T1, T2, T3, T4, T5, T6,
    A0, A1, A2, A3, A4, A5, A6, A7,
}

impl Reg {
    fn to_string(self) -> String {
        match self {
            T0 => "t0",
            T1 => "t1",
            T2 => "t2",
            T3 => "t3",
            T4 => "t4",
            T5 => "t5",
            T6 => "t6",
            A0 => "a0",
            A1 => "a1",
            A2 => "a2",
            A3 => "a3",
            A4 => "a4",
            A5 => "a5",
            A6 => "a6",
            A7 => "a7",
        }.to_string()
    }
}

type RegMap = HashSet<Reg>;

fn next_reg(regs: &mut RegMap) -> Option<Reg> {
    for reg in Reg::iter() {
        let mut flag = true;
        for val in regs.iter() {
            if reg == val {
                flag = false;
                break;
            }
        }
        if flag {
            regs.insert(reg);
            return Some(reg);
        }
    }
    None
}


/* Parse Binary into risc32 text */
fn parse_binary(bin: &values::Binary, func_data: &FunctionData, regs: &mut RegMap) -> (String, String) {
    let mut final_str = String::new();
    let match_deeper = |value| {
        match func_data.dfg().value(value).kind() {
            ValueKind::Integer(int) => parse_integer(int),
            ValueKind::Binary(bin) => {
                let (s, v) = parse_binary(bin, func_data, regs);
                final_str += &s;
                v
            }
        }
    };
    let lhs = match_deeper(bin.lhs());
    let rhs = match_deeper(bin.rhs());
    let new_reg = next_reg(regs).unwrap().to_string();
    match bin.op() {
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
    };
    (final_str, new_reg)
}

fn parse_integer(int: &values::Integer) -> String {
    int.value().to_string()
}

fn parse_return(ret: &values::Return, func_data: &FunctionData, regs: &mut RegMap) -> String {
    match ret.value() {
        Some(value) => match func_data.dfg().value(value).kind() {
            ValueKind::Integer(int) => "li a0, ".to_string() + &parse_integer(int) + "\nret\n",
            ValueKind::Binary(bin) => {
                let (insts, reg) = parse_binary(bin, func_data, regs);
                insts + "li a0, " + &reg + "\nret\n"
            },
        },
        None => "ret\n".to_string()
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
            for &inst in node.insts().keys() {
                let value_data = func_data.dfg().value(inst);
                if let ValueKind::Return(ret) = value_data.kind() {
                    let mut map = RegMap::new();
                    text += &parse_return(ret, func_data, &mut map);
                }
            }
        }
    }

    text
}

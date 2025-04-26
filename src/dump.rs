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
            },
            ast::ExpCore::Ident(_id) => unreachable!(),
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
        let ret_stmt = match self.func.block.block_item_list.into_iter().next().unwrap() {
            ast::BlockItem::Stmt(stmt) => stmt,
            ast::BlockItem::Decl(_) => unreachable!(),
        };
        let ret_exp = match ret_stmt {
            ast::Stmt::Ret(ret) => ret,
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
            Reg::T0 => "t0",
            Reg::T1 => "t1",
            Reg::T2 => "t2",
            Reg::T3 => "t3",
            Reg::T4 => "t4",
            Reg::T5 => "t5",
            Reg::T6 => "t6",
            Reg::A0 => "a0",
            Reg::A1 => "a1",
            Reg::A2 => "a2",
            Reg::A3 => "a3",
            Reg::A4 => "a4",
            Reg::A5 => "a5",
            Reg::A6 => "a6",
            Reg::A7 => "a7",
        }.to_string()
    }
}

type RegMap = HashSet<Reg>;

/* Find next unused register */
fn next_reg(regs: &mut RegMap) -> Option<Reg> {
    for reg in Reg::iter() {
        let mut flag = true;
        for val in regs.iter() {
            if reg == *val {
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

/* Parse Binary into risc32 text (instruction text, final register) */
fn parse_binary(bin: &values::Binary, func_data: &FunctionData, regs: &mut RegMap) -> (String, Reg) {
    let mut final_str = String::new();
    // Forwarding recursive return value
    let mut match_deeper = |value| {
        match func_data.dfg().value(value).kind() {
            ValueKind::Integer(int) => {
                // Check if int = 0
                match int.value() {
                    0 => Ok("x0".to_string()),
                    _ => {
                        let reg = next_reg(regs).unwrap();
                        final_str += &("li ".to_string() + &reg.to_string() + ", " + &parse_integer(int) + "\n");
                        Err(reg)
                    }
                }
            }
            ValueKind::Binary(bin) => {
                let (s, reg) = parse_binary(bin, func_data, regs);
                final_str += &s;
                Err(reg)
            },
            _ => unreachable!(),
        }
    };
    let lhs_o = match_deeper(bin.lhs());
    let rhs_o = match_deeper(bin.rhs());
    // Reuse registers
    let mut match_res = |res: Result<String, Reg>| {
        match res {
            Ok(s) => s,
            Err(r) => {
                regs.remove(&r);
                r.to_string()
            }
        }
    };
    let lhs = match_res(lhs_o);
    let rhs = match_res(rhs_o);
    let new_reg_name = next_reg(regs).unwrap();
    let new_reg = new_reg_name.to_string();
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
    };
    (final_str, new_reg_name)
}

/* Parse Integer into risc32 text */
fn parse_integer(int: &values::Integer) -> String {
    int.value().to_string()
}

/* Parse Return into risc32 text */
fn parse_return(ret: &values::Return, func_data: &FunctionData, regs: &mut RegMap) -> String {
    match ret.value() {
        Some(value) => match func_data.dfg().value(value).kind() {
            ValueKind::Integer(int) => "li a0, ".to_string() + &parse_integer(int) + "\nret\n",
            ValueKind::Binary(bin) => {
                let (insts, reg) = parse_binary(bin, func_data, regs);
                insts + "mv a0, " + &reg.to_string() + "\nret\n"
            },
            _ => unreachable!(),
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
                // Only visit return
                if let ValueKind::Return(ret) = value_data.kind() {
                    // Actually map shall be defined over the func
                    let mut map = RegMap::new();
                    text += &parse_return(ret, func_data, &mut map);
                }
            }
        }
    }
    text
}

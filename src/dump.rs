/* Uses */
use crate::ast;
use koopa::ir::*;
use koopa::ir::entities::ValueData;
use koopa::ir::builder::*;
use koopa::back::KoopaGenerator;

/* Dump prog into koopa */
fn dump(prog: ast::Program) -> Program {
    // Now original version
    let mut program = Program::new();
    let main = program.new_func(
        FunctionData::new(("@".to_owned() + &prog.func.id).into(), Vec::new(), Type::get_i32()),
    );
    let main_data = program.func_mut(main);
    let entry = main_data.dfg_mut().new_bb().basic_block(Some("%entry".into()));
    main_data.layout_mut().bbs_mut().push_key_back(entry).unwrap();
    let ret_val_i32 = match prog.func.block.stmt {
        ast::Stmt::Ret(num) => num,
    };
    let ret_val = main_data.dfg_mut().new_value().integer(ret_val_i32);
    let ret = main_data.dfg_mut().new_value().ret(Some(ret_val));
    main_data.layout_mut().bb_mut(entry).insts_mut().extend([ret]);

    program
}

/* Generate koopa text */
pub fn gen_text_koopa(ast: ast::Program) -> String {
    // Dump, then call koopa lib
    let program = dump(ast);
    let mut gen = KoopaGenerator::new(Vec::new());
    gen.generate_on(&program).unwrap();

    std::str::from_utf8(&gen.writer()).unwrap().to_string()
}

/* Parse Value into text */
fn parse_value(value_data: &ValueData, func_data: &FunctionData) -> String {
    match value_data.kind() {
        ValueKind::Integer(int) => {
            int.value().to_string()
        },
        ValueKind::Return(ret) => {
            match ret.value() {
                Some(int) => {
                    "li a0, ".to_string() + &parse_value(func_data.dfg().value(int), func_data) + "\nret\n"
                }
                None => {
                    "ret\n".to_string()
                }
            }
        }
        _ => unreachable!(),
    }
}

/* Generate riscv32 code */
pub fn gen_riscv32(ast: ast::Program) -> String {
    let mut text = ".text\n".to_string();
    text += ".globl main\n";
    let program = dump(ast);
    for &func in program.func_layout() {
        let func_data = program.func(func);
        text += &func_data.name()[1..];
        text += ":\n";
        for (&bb, node) in func_data.layout().bbs() {
            // Init for bb
            for &inst in node.insts().keys() {
                let value_data = func_data.dfg().value(inst);
                text += &parse_value(value_data, func_data);
            }
        }
    }

    text
}

use crate::ast;
use koopa::ir::*;
use koopa::ir::builder::*;
use koopa::back::KoopaGenerator;

fn dump_prog(prog: ast::Program) -> Program {
    let mut program = Program::new();
    let main = program.new_func(
        FunctionData::new(("@".to_owned() + &prog.func.id).into(), Vec::new(), Type::get_i32()),
    );
    let main_data = program.func_mut(main);
    let entry = main_data.dfg_mut().new_bb().basic_block(Some("%entry".into()));
    main_data.layout_mut().bbs_mut().push_key_back(entry).unwrap();
    let ret_val = main_data.dfg_mut().new_value().integer(0);
    let ret = main_data.dfg_mut().new_value().ret(Some(ret_val));
    main_data.layout_mut().bb_mut(entry).insts_mut().extend([ret]);

    program
}

pub fn gen_text_koopa(ast: ast::Program) -> String {
    let program = dump_prog(ast);
    let mut gen = KoopaGenerator::new(Vec::new());
    gen.generate_on(&program).unwrap();
    std::str::from_utf8(&gen.writer()).unwrap().to_string()
}

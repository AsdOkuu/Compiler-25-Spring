use lalrpop_util::lalrpop_mod;
use std::env::args;
use std::fs::read_to_string;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;

lalrpop_mod!(sysy);
pub mod ast;
pub mod dump;

fn main() {
    let mut args = args();
    args.next();
    let _mode = args.next().unwrap();
    let input = args.next().unwrap();
    args.next();
    let output = args.next().unwrap();

    let input = read_to_string(input).unwrap();

    let ast = sysy::ProgramParser::new().parse(&input).unwrap();

    println!("{:#?}", ast);

    // println!("{}", dump::gen_text_koopa(ast));

    let text_ir = dump::gen_text_koopa(ast);
    let path = Path::new(&output);
    let mut file = File::create(&path).unwrap(); 
    file.write_all(text_ir.as_bytes()).unwrap();
}

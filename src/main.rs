/* Uses */
use lalrpop_util::lalrpop_mod;
use std::env::args;
use std::fs::read_to_string;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;

/* Lalrpop Generate */
lalrpop_mod!(sysy);

/* Module (Extern) */
pub mod ast;
pub mod dump;
pub mod generate;

/* Main */
fn main() {
    /* Args Process */
    let mut args = args();
    args.next();
    // Mode: -koopa / -riscv32
    let mode = args.next().unwrap();
    // Input file path
    let input = args.next().unwrap();
    args.next();
    // Output file path
    let output = args.next().unwrap();

    /* Read */
    let input = read_to_string(input).unwrap();

    /* Compile */
    // Use lalrpop generated parser
    let ast = sysy::ProgramParser::new().parse(&input).unwrap();
    // Output log
    // println!("{:#?}", ast);

    // Select mode
    let text = match mode.as_str() {
        "-koopa" => {
            // Koopa - output text
            dump::gen_text_koopa(ast) // Gen from dump mod
        },
        "-riscv" => {
            // Riscv32 - output assemble code
            generate::gen_riscv32(ast)
        },
        _ => unreachable!(),
    };

    /* Output */
    let path = Path::new(&output);
    let mut file = File::create(&path).unwrap(); 
    file.write_all(text.as_bytes()).unwrap();
}

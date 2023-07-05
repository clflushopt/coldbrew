use coldbrew::jvm::{read_class_file, JVMParser};
use coldbrew::program::Program;
use coldbrew::runtime::Runtime;
use std::path::Path;

fn main() {
    // What are the program components ?
    // 1. Reads and parse Java class file.
    let path = Path::new("./support/Factorial.class");
    let class_file_bytes = read_class_file(path);
    let class_file = JVMParser::parse(&class_file_bytes)
        .expect("JVMParser failed with some error");
    // 2. Build abstract program from class file to run in the interpreter.
    let program = Program::new(&class_file);
    // 3. Interepreter executes bytecode and records a trace.
    //  When trace is hot it is compiled to assembly
    //  Interpreter/Handler executes assembly and returns value
    // Runtime takes ownership of program.
    let mut runtime = Runtime::new(program);
    match runtime.run() {
        Ok(()) => println!("Success !"),
        Err(err) => println!("Error : {err}"),
    }
}

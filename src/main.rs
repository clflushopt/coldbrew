use coldbrew::jvm::{JVMParser, read_class_file};
use std::env;
use std::path::Path;

fn main() {
    // What are the program components ?
    // 1. Reads a Java class file.
    let path = Path::new("./support/SingleFuncCall.class");
    let class_file_bytes = read_class_file(&path);
    let result = JVMParser::parse(&class_file_bytes);
    assert!(result.is_ok());
    let class_file = result.unwrap();
    // 2. Passes bytecode to an Interpreter class
    // 3. Interepreter executes bytecode and records a trace.
    // 4. When trace is hot it is compiled to assembly
    // 5. Interpreter/Handler executes assembly and returns value
    // 6. Repeat
    println!("{:?}", class_file);
}

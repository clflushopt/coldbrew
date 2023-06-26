use coldbrew::jvm;

fn main() {
    // What are the program components ?
    // 1. Reads a Java class file.
    // 2. Passes bytecode to an Interpreter class
    // 3. Interepreter executes bytecode and records a trace.
    // 4. When trace is hot it is compiled to assembly
    // 5. Interpreter/Handler executes assembly and returns value
    // 6. Repeat
    println!("Hello, world!");
}

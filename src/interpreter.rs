//! Interpreter for JVM bytecode.

/// `Interpreter` for a stack based virtual machine for JVM bytecode.
pub struct Interpreter {
    // Actual stack used to execute bytecode instructions.
    stack:Vec<u64>,
    // Instruction stream.
    instructions:Vec<u8>,
    // Constants pool.
    constants_pool:Vec<u64>,
}

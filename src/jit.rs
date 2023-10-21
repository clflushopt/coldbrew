//! JIT compiler for coldrew targeting x86_64.
use std::collections::{HashMap, VecDeque};

use crate::bytecode::OPCode;
use crate::runtime::ProgramCounter;
use crate::runtime::Value;
use crate::trace::Recording;

use dynasmrt::components::LitPool;
use dynasmrt::dynasm;
use dynasmrt::x64::Assembler;
use dynasmrt::{AssemblyOffset, DynasmApi, ExecutableBuffer};

/// Intel x86-64 registers, ordered by their syntactic order in the Intel
/// manuals. The usage of the registers follows the System ADM64 ABI.
///
/// Arguments 1 to 6 go into Rdi, Rsi, Rdx, Rcx, R8 and R9.
/// Excess arguments are pushed to the stack, but since the Jit calling
/// convention restrics the `execute` function to two arguments we want be
/// using any registers besides Rdi and Rsi.
///
/// Registers Rbx, Rsp, Rbp and R12 to R15 must be callee preserved if they
/// are to be used, the other registers can be clobbered and caller must
/// preserve them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Register {
    Rax,
    Rbx,
    Rcx,
    Rdx,
    Rdi,
    Rsi,
    Rbp,
    Rcsp,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

/// Operands.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Operand {
    // Register operands.
    Register(Register),
    // Immediate operands.
    Immediate(i32),
    // Memory operands represent memory addresses as a pair of base register
    // and immediate offset often seen as `[bp, offset]`.
    Memory(Register, i32),
    // Label operands.
    Label(ProgramCounter),
}

/// x86_64 functioe prologue, allocates `max_locals` space on the stack even
/// though they might not be all used.
#[cfg(target_arch = "x86_64")]
macro_rules! prologue {
    ($ops:ident) => {{
        let start = $ops.offset();
        dynasm!($ops
            ; push rbp
            ; mov rbp, rsp
            ; mov QWORD [rbp-8], rdi
            ; mov QWORD [rbp-16], rsi
        );
        start
    }};
}

/// aarch64 function epilogue.
#[cfg(target_arch = "x86_64")]
macro_rules! epilogue {
    ($ops:ident) => {dynasm!($ops
        ; pop rbp
        ; ret
    );};
}

/// `NativeTrace` is a pair of `usize` and `Assembler` that represents an entry
/// point in the `Assembler` buffer.
#[derive(Debug)]
pub struct NativeTrace(AssemblyOffset, ExecutableBuffer);

/// `JitCache` is responsible for compiling and caching recorded native traces.
pub struct JitCache {
    // Internal cache of available registers.
    registers: VecDeque<Register>,
    // Operand stack.
    operands: Vec<Operand>,
    // Cache of native traces.
    traces: HashMap<ProgramCounter, NativeTrace>,
}

impl Default for JitCache {
    fn default() -> Self {
        Self::new()
    }
}

impl JitCache {
    // Create a new JIT compilation cache.
    pub fn new() -> Self {
        let registers = vec![];
        JitCache {
            registers: VecDeque::from(registers),
            traces: HashMap::new(),
            operands: Vec::new(),
        }
    }

    // Execute the trace at `pc` and return the mutated locals for the frame
    // and the program counter where the runtime should continue execution.
    pub fn execute(&mut self, pc: ProgramCounter) -> ProgramCounter {
        if self.traces.contains_key(&pc) {
            // execute the assembled trace.
            if let trace = self.traces.get_mut(&pc).expect("Expected a native trace @ {pc}") {
                println!("Found a native trace @ {pc}");
                let entry = trace.0;
                let buf = &trace.1;
                let execute: fn() = unsafe { std::mem::transmute(buf.ptr(entry)) };

                println!("Executing native trace");
                execute();
                println!("Done executing native trace");
            }
        }
        pc
    }

    /// Checks if a native trace exists at this `pc`.
    pub fn has_native_trace(&self, pc: ProgramCounter) -> bool {
        self.traces.contains_key(&pc)
    }

    // Compile the trace given as argument and prepare a native trace
    // for execution.
    //
    // This is the tracelet JIT version where we only compile basic blocks
    // and exit skip all control flow opcodes.
    //
    // Labels:
    //
    // ```
    // let label = new_dynamic_label()
    // labels.insert(pc, label) ; binds a label to a program counter.
    //
    // if labels.contains(pc):
    //  // Bind the label to the current offset
    //  define_dynamic(labels.get(pc), ops.offset())
    // ```
    // continue compiling
    //
    // Compile works as follows :
    // 1. Build a dynasmrt Assembler object.
    // 2. Emits a static prologue for the jitted code.
    // 3. For each instruction in the trace generate its equivalent arm64
    // 4. Emits a static epilogue for the jitted code.
    // 5. When a trace recording is looked, assemble and run the jitted code.
    //
    // There are a few details we want to fix before hand :
    // - We need to define a calling convention for our JIT i.e where do
    // arguments go and what are the scratch space registers.
    // - We need to keep track of the traces we record and when we stitch them
    // i.e book-keeping `pc`, offsets and other stuff.
    //
    // When we run the trace we need to return PC at which the interpreter
    // will continue execution (`reentry_pc`)
    //
    // We need to load local variables into an array let's call it `local_vars`
    // speaking of calling convention, when we load a local we need a way to
    // to translate the existing locals load from JVM bytecode to a load in
    // `local_var` if we assume that r10 will be the base register where we
    // set `local_vars` and we want to access local at index 3 then the we
    // can setup a memory load then store using `r13 + 3 * 8`.
    pub fn compile(
        &mut self,
        recording: &Recording,
    )  {
        let pc = recording.start;
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();
        // Prologue for dynamically compiled code.
        let offset = prologue!(ops);
        // Trace compilation :
        // For now we compile only the prologue and epilogue and ensure that
        // entering the Jit executing the assembled code and leaving the Jit
        // works correct.
        /*
        for trace in &recording.trace {
            match trace.instruction().get_mnemonic() {
                _ => todo!(),
            }
        }
        */


        // Epilogue for dynamically compiled code.
        epilogue!(ops);

        println!("Compiled trace @ {pc}");
        let buf = ops.finalize().unwrap();
        let native_trace = NativeTrace(offset, buf);
        self.traces.insert(pc, native_trace);
        println!("Added trace to native traces");
    }

    // Emit a load operation, where `dst` must be a register and `src` a memory
    // address.
    fn emit_load(&mut self, ops: &mut Assembler, dst: &Operand, src: &Operand) {
    }

    // Emit a move operation, this includes all data movement operations
    // register to register and immediate to register.
    // For memory accesses we follow the aarch64 story of generating all
    // necessary stores and loads.
    fn emit_mov(&mut self, ops: &mut Assembler, dst: &Operand, src: &Operand) {}

    // Emit an arithmetic operation, covers all simple instructions such as
    // `add`, `mul` and `sub`.
    fn emit_arithmetic(&mut self, ops: &mut Assembler) {}

    // Returns the first available register.
    fn first_available_register(&mut self) -> Operand {
        println!("Get available register => queue : {:?}", self.registers);
        if !self.registers.is_empty() {
            let reg = self.registers.pop_front().unwrap();
            Operand::Register(reg)
        } else {
            panic!("no available registers")
        }
    }

    // Free the top most register in the operand stack.
    fn free_register(&mut self) -> Option<Operand> {
        let op = self.operands.pop();

        if let Some(op) = op {
            match op {
                Operand::Register(reg) => self.registers.push_back(reg),
                _ => (),
            }
        }
        op
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use dynasmrt::dynasm;
    use dynasmrt::{DynasmApi, ExecutableBuffer};
    use std::slice;

    #[test]
    fn can_jit_load_and_store_opcodes() {}
}

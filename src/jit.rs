//! JIT compiler for coldrew targeting x86_64.
use std::collections::{HashMap, VecDeque};

use crate::bytecode::OPCode;
use crate::runtime::{Frame, ProgramCounter, Value};
use crate::trace::Recording;

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
    Rsp,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

/// Generic representation of assembly operands that allows for supporting
/// both x86 and ARM64.
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
macro_rules! prologue {
    ($ops:ident) => {{
        #[cfg(target_arch = "x86_64")]
        {
        let start = $ops.offset();
        dynasm!($ops
            ; push rbp
            ; mov rbp, rsp
            ; mov QWORD [rbp-8], rdi
            ; mov QWORD [rbp-16], rsi
        );
        start
        }
        #[cfg(target_arch = "aarch64")]
        {
        let start = $ops.offset();
        dynasm!($ops
            ; sub sp, sp, #32
            ; str x0, [sp, 8]
            ; str x1, [sp]
        );
        start
        }
    }};
}

/// aarch64 function epilogue.
macro_rules! epilogue {
    ($ops:ident) => {
        #[cfg(target_arch = "x86_64")]
        dynasm!($ops
            ; pop rbp
            ; ret
        );

#[cfg(target_arch = "aarch64")]
    dynasm!($ops
        // Increment stack pointer to go back to where we were
        // before the function call.
        ; add sp, sp, #32
        ; ret
    );
    };
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
    /// Create a new JIT compilation cache.
    pub fn new() -> Self {
        let registers = vec![
            Register::Rax,
            Register::Rcx,
            Register::R8,
            Register::R9,
            Register::R10,
            Register::R11,
            Register::Rbx,
            Register::R12,
            Register::R13,
            Register::R14,
            Register::R15,
        ];
        JitCache {
            registers: VecDeque::from(registers),
            traces: HashMap::new(),
            operands: Vec::new(),
        }
    }

    /// Reset Jit state.
    fn reset(&mut self) {
        let registers = vec![
            Register::Rax,
            Register::Rcx,
            Register::R8,
            Register::R9,
            Register::R10,
            Register::R11,
            Register::Rbx,
            Register::R12,
            Register::R13,
            Register::R14,
            Register::R15,
        ];
        self.registers.clear();
        self.registers = VecDeque::from(registers);
        self.operands.clear();
    }

    /// Execute the trace at `pc` and return the mutated locals for the frame
    /// and the program counter where the runtime should continue execution.
    ///
    /// Ideally we can just return the update `locals` and exit but for now
    /// let's take in the entire execution frame of VM and update it.
    ///
    /// Following the x86-64 convention the locals are passed in `rdi`, exit
    /// information is passed in `rsi`.
    pub fn execute(
        &mut self,
        pc: ProgramCounter,
        frame: &mut Frame,
    ) -> ProgramCounter {
        if self.traces.contains_key(&pc) {
            // execute the assembled trace.
            let trace = self
                .traces
                .get_mut(&pc)
                .expect("Expected a native trace @ {pc}");

            // Flatten the locals `HashMap` into a `i32` slice.
            let mut locals = vec![0i32; 4096];
            // Exit information, for now is empty.
            let exits = vec![0i32; 4096];

            for (key, val) in frame.locals.iter() {
                locals[*key] = match val {
                    Value::Int(x) => *x,
                    Value::Long(x) => *x as i32,
                    Value::Float(x) => *x as i32,
                    Value::Double(x) => *x as i32,
                };
            }

            // println!("Found a native trace @ {pc}");
            let entry = trace.0;
            let buf = &trace.1;
            let execute: fn(*mut i32, *const i32) =
                unsafe { std::mem::transmute(buf.ptr(entry)) };

            println!("Executing native trace");
            unsafe {
                execute(locals.as_mut_ptr(), exits.as_ptr());
            }
            println!("Done executing native trace");
        }
        pc
    }

    /// Checks if a native trace exists at this `pc`.
    pub fn has_native_trace(&self, pc: ProgramCounter) -> bool {
        self.traces.contains_key(&pc)
    }

    /// Compile the trace given as argument and prepare a native trace
    /// for execution.
    ///
    /// This is the tracelet JIT version where we only compile basic blocks
    /// and exit skip all control flow opcodes.
    ///
    /// Compile works as follows :
    /// 1. Build a dynasmrt Assembler object.
    /// 2. Emits a static prologue for the jitted code.
    /// 3. For each instruction in the trace generate its equivalent arm64
    /// 4. Emits a static epilogue for the jitted code.
    /// 5. When a trace recording is looked, assemble and run the jitted code.
    ///
    /// When we run the trace we need to return PC at which the interpreter
    /// will continue execution (`reentry_pc`)
    ///
    /// Solving the exit problem :
    /// 1. At each trace.instruction()
    ///     1.1 Create a DynasmLabel `inst_label_{pc}`
    ///     1.2 Append the new label to the `global_jump_table`
    /// 2. If the trace.instruction() is a branch:
    ///     1.1 Check if we have an existing entry in the `global_jump_table`.
    ///     1.2 If an entry exists it means we've compiled a trace for this block.
    ///         1.2.1 Fetch the label and mark the native trace with this label
    ///         the trace will either be stitched if the jump is outside this trace
    ///         or it will be local if it is inside this trace.
    ///     1.3 If an entry doesn't exists it means we're exiting the JIT so we
    ///     preserve the target `pc` in `rax` and return, when calling `execute`
    ///     we will either jump to another trace and continue executing or exit
    ///     the JIT where we update the `pc` and transfer control back to the JIT.
    pub fn compile(&mut self, recording: &Recording) {
        // Reset Jit state.
        self.reset();
        let pc = recording.start;
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();
        // Prologue for dynamically compiled code.
        let offset = prologue!(ops);
        // Trace compilation :
        // For now we compile only the prologue and epilogue and ensure that
        // entering the Jit executing the assembled code and leaving the Jit
        // works correct.
        for trace in &recording.trace {
            match trace.instruction().get_mnemonic() {
                // Load operation loads a constant from the locals array at
                // the position given by the opcode's operand.
                OPCode::ILoad
                | OPCode::ILoad0
                | OPCode::ILoad1
                | OPCode::ILoad2
                | OPCode::ILoad3 => {
                    // println!("Compiling an ILoad");
                    let value = match trace.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                        _ => todo!(),
                    };
                    let dst = self.first_available_register();
                    match dst {
                        Operand::Register(dst) => {
                            // println!("Using {:?} as destination register", dst);
                            #[cfg(target_arch = "x86_64")]
                            dynasm!(ops
                                ; mov Rq(dst as u8), [rdi + 8 * value]
                            );
                            #[cfg(target_arch = "aarch64")]
                            dynasm!(ops);
                        }
                        _ => todo!(),
                    }
                    self.operands.push(dst);
                }
                _ => (),
            }
        }

        // Epilogue for dynamically compiled code.
        epilogue!(ops);

        // println!("Compiled trace @ {pc}");
        let buf = ops.finalize().unwrap();
        let native_trace = NativeTrace(offset, buf);
        self.traces.insert(pc, native_trace);
        // println!("Added trace to native traces");
    }

    /// Emit a load operation, where `dst` must be a register and `src` a memory
    /// address.
    fn emit_load(&mut self, ops: &mut Assembler, dst: &Operand, src: &Operand) {
    }

    /// Emit a move operation, this includes all data movement operations
    /// register to register and immediate to register.
    fn emit_mov(&mut self, ops: &mut Assembler, dst: &Operand, src: &Operand) {}

    /// Emit an arithmetic operation, covers all simple instructions such as
    /// `add`, `mul` and `sub`.
    fn emit_arithmetic(&mut self, ops: &mut Assembler) {}

    /// Returns the first available register.
    fn first_available_register(&mut self) -> Operand {
        if !self.registers.is_empty() {
            let reg = self.registers.pop_front().unwrap();
            Operand::Register(reg)
        } else {
            panic!("no available registers")
        }
    }

    /// Free the top most register in the operand stack.
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
    #[test]
    fn can_jit_load_and_store_opcodes() {}
}

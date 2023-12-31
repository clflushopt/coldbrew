//! JIT compiler for coldrew targeting x86_64.
use std::collections::{HashMap, VecDeque};

use crate::bytecode::OPCode;
use crate::runtime::{Frame, ProgramCounter, Value};
use crate::trace::Trace;

use dynasmrt::x64::Assembler;
use dynasmrt::{dynasm, DynamicLabel};
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
    Rcx,
    Rdx,
    Rbx,
    Rsp,
    Rbp,
    Rsi,
    Rdi,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

/// Intel x86-64 shorthand for arithmetic opcodes.
#[derive(Debug, Clone, Copy)]
enum Op {
    Add,
    Sub,
    IMul,
    IDiv,
    IRem,
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
            ; mov QWORD [rbp-24], rdi
            ; mov QWORD [rbp-32], rsi
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
    ($ops:ident) => {{
        let epilogue = $ops.offset();
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
        epilogue
    }};
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
    // Cache of `pc` entries to labels.
    labels: HashMap<ProgramCounter, DynamicLabel>,
}

impl Default for JitCache {
    fn default() -> Self {
        Self::new()
    }
}

impl JitCache {
    /// Create a new JIT cache.
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
            labels: HashMap::new(),
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
        self.labels.clear();
    }

    /// Execute the trace at `pc` and return the mutated locals for the frame
    /// and the program counter where the runtime should continue execution.
    ///
    /// Ideally we can just return the updated `locals` and exit but for now
    /// let's take in the entire execution frame of VM and update it.
    ///
    /// Following the x86-64 convention the locals are passed in `rdi`, exit
    /// information is passed in `rsi`.
    pub fn execute(&mut self, pc: ProgramCounter, frame: &mut Frame) -> usize {
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
            let execute: fn(*mut i32, *const i32) -> i32 =
                unsafe { std::mem::transmute(buf.ptr(entry)) };

            println!("Executing native trace");
            let exit_pc = execute(locals.as_mut_ptr(), exits.as_ptr()) as usize;
            println!("Done executing native trace");
            exit_pc
        } else {
            pc.get_instruction_index()
        }
    }

    /// Checks if a native trace exists at this `pc`.
    pub fn has_native_trace(&self, pc: ProgramCounter) -> bool {
        self.traces.contains_key(&pc)
    }

    /// Compile the trace given as argument and prepare a native trace
    /// for execution.
    ///
    ///
    /// Compile works as follows :
    /// 1. Build a dynasmrt Assembler object.
    /// 2. Emits a static prologue for the jitted code.
    /// 3. For each recorded instruction generate its equivalent x86 or arm64
    ///    instruction and create a label for it.
    ///   3.1 If the instruction is a jump i.e `Goto` check if we have a label
    ///   for it, since all recorded traces are straight lines with backward
    ///   jumps we must have one, then emit the equivalent jump with the label
    ///   as the target.
    /// 4. Emits a static epilogue for the jitted code.
    /// 5. When a trace recording is looked, run the jitted code.
    ///
    /// When we run the trace we need to return PC at which the interpreter
    /// will continue execution (`reentry_pc`)
    ///
    /// How jumps are handled (in more details) :
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
    ///     the assumption is that we will always exit back to the interpreter
    ///     since we currently don't support trace stitching.
    pub fn compile(&mut self, recording: &Trace) {
        self.reset();
        // Reset Jit state.
        let pc = recording.start;
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();
        // Prologue for dynamically compiled code.
        let offset = prologue!(ops);
        let mut exit_pc = 0usize;
        // Trace compilation :
        // For now we compile only the prologue and epilogue and ensure that
        // entering the Jit executing the assembled code and leaving the Jit
        // works correct.
        for entry in &recording.trace {
            println!("Trace dump: {:}", entry);
            // Record the instruction program counter to a new label.
            let inst_label = ops.new_dynamic_label();
            let _ = self.labels.insert(entry.pc(), inst_label);
            println!("Record label {} @ {}", inst_label.get_id(), entry.pc());
            match entry.instruction().get_mnemonic() {
                // Load operation loads a constant from the locals array at
                // the position given by the opcode's operand.
                //
                // Since the locals array is the first argument to our JIT
                // `execute` function the value can be fetched from memory
                // using base addressing.
                // We assume (for now) locals are 8 bytes long.
                OPCode::ILoad
                | OPCode::ILoad0
                | OPCode::ILoad1
                | OPCode::ILoad2
                | OPCode::ILoad3 => {
                    println!("Compiling an iload");
                    let value = match entry.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                        _ => unreachable!("Operand to iload (index in locals) must be int in current implementation")
                    };
                    let dst = self.first_available_register();
                    Self::emit_mov(
                        &mut ops,
                        &dst,
                        &Operand::Memory(Register::Rdi, 8 * value),
                    );
                    self.operands.push(dst);
                    exit_pc = entry.pc().get_instruction_index() + 1;
                }
                OPCode::IStore
                | OPCode::IStore0
                | OPCode::IStore1
                | OPCode::IStore2
                | OPCode::IStore3 => {
                    println!("Compiling an istore");
                    let value = match entry.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                            _ => unreachable!("Operand to istore (index in locals) must be int in current implementation")
                    };
                    if let Some(src) = self.free_register() {
                        Self::emit_mov(
                            &mut ops,
                            &Operand::Memory(Register::Rdi, 8 * value),
                            &src,
                        );
                    }
                }
                OPCode::BiPush | OPCode::SiPush | OPCode::Ldc => {
                    let imm = match entry.instruction().nth(0) {
                        Some(Value::Int(imm)) => imm,
                        _ => unreachable!("Operand to {} must be an int in current implementation", entry.instruction().get_mnemonic())
                    };
                    self.operands.push(Operand::Immediate(imm));
                }
                OPCode::IAdd => {
                    println!("Compiling an iadd");
                    self.emit_arithmetic(&mut ops, Op::Add);
                }
                OPCode::ISub => {
                    println!("Compiling an isub");
                    self.emit_arithmetic(&mut ops, Op::Sub);
                }
                OPCode::IMul => {
                    println!("Compiling an imul");
                    self.emit_arithmetic(&mut ops, Op::IMul);
                }
                OPCode::IDiv => {
                    println!("Compiling an idiv");
                    self.emit_arithmetic(&mut ops, Op::IDiv);
                }
                OPCode::IRem => {
                    println!("Compiling an iadd");
                    self.emit_arithmetic(&mut ops, Op::IRem);
                }
                OPCode::IInc => {
                    println!("Compiling an iinc");
                    let index = match entry.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                            _ => unreachable!("First operand to iinc (index in locals) must be int")
                    };
                    let constant = match entry.instruction().nth(1) {
                        Some(Value::Int(x)) => x,
                        _ => unreachable!("Second operand to iinc (constant for increment) must be int in current implementation")
                    };
                    #[cfg(target_arch = "x86_64")]
                    dynasm!(ops
                        ; add [Rq(Register::Rdi as u8) + 8 * index], constant as _
                    );
                }
                OPCode::Goto => {
                    // let target = match ...
                    // if let Some(pc) = trace.contains(target) {
                    // the target jump is inside the trace
                    // is it before or after ?
                    // if pc < entry.pc {
                    //  The target PC is before the current instruction
                    //  do we have a label for it ?
                    //  self.labels.get(pc)
                    //  emit a jmp .label
                    // } else if pc > entry.pc {
                    //  The target PC is forward (think a break statement) so emit a jump
                    //  instruction to a new label and add this to labels map.
                    // }
                    // If the Goto target is outside then abondon this trace.
                    //
                }
                _ => println!(
                    "Found opcode : {:}",
                    entry.instruction().get_mnemonic()
                ),
            }
        }
        #[cfg(target_arch = "x86_64")]
        dynasm!(ops
            ; mov rax, exit_pc as i32
        );
        // Epilogue for dynamically compiled code.
        epilogue!(ops);

        println!("Exit PC {}", exit_pc);
        // println!("Compiled trace @ {pc}");
        let buf = ops.finalize().unwrap();
        let native_trace = NativeTrace(offset, buf);
        self.traces.insert(pc, native_trace);
        // println!("Added trace to native traces");
    }

    /// Emit a move operation, this includes all data movement operations
    /// register to register and immediate to register.
    fn emit_mov(ops: &mut Assembler, dst: &Operand, src: &Operand) {
        match (dst, src) {
            (Operand::Register(dst), Operand::Register(src)) => {
                #[cfg(target_arch = "x86_64")]
                dynasm!(ops
                    ;mov Rq(*dst as u8), Rq(*src as u8)
                );
            }
            (Operand::Register(dst), Operand::Immediate(imm)) => {
                #[cfg(target_arch = "x86_64")]
                dynasm!(ops
                        ;mov Rq(*dst as u8), *imm
                );
            }
            (Operand::Register(dst), Operand::Memory(base, offset)) => {
                #[cfg(target_arch = "x86_64")]
                dynasm!(ops
                    ;mov Rq(*dst as u8), [Rq(*base as u8) + *offset]
                );
            }
            (Operand::Memory(base, offset), Operand::Register(src)) => {
                #[cfg(target_arch = "x86_64")]
                dynasm!(ops
                    ; mov [Rq(*base as u8) + *offset], Rq(*src as u8)
                );
            }
            (Operand::Memory(base, offset), Operand::Immediate(imm)) => {
                #[cfg(target_arch = "x86_64")]
                dynasm!(ops
                        ; mov DWORD [Rq(*base as u8) + *offset], *imm as _
                );
            }
            _ => unreachable!(
                "Unexpected operands for `mov` `dst`={:?}, `src`={:?})",
                dst, src
            ),
        }
    }

    /// Emit an arithmetic operation, covers all simple instructions such as
    /// `add`, `mul` and `sub`.
    fn emit_arithmetic(&mut self, ops: &mut Assembler, op: Op) {
        let rhs = match self.operands.pop() {
            Some(rhs) => rhs,
            None => panic!("expected operand found None"),
        };
        let lhs = match self.operands.pop() {
            Some(lhs) => lhs,
            None => panic!("expected operand found None"),
        };

        let dst = match &lhs {
            &Operand::Register(reg) => Operand::Register(reg),
            // TODO: need to mov lhs operand to the first free register.
            _ => {
                let dst = self.first_available_register();
                JitCache::emit_mov(ops, &dst, &lhs);
                dst
            }
        };

        match &rhs {
            Operand::Register(reg) => self.registers.push_back(*reg),
            _ => (),
        }

        self.operands.push(dst);

        let Operand::Register(dst) = dst else {
            unreachable!("Unexpected enum variant for `Operand` expected `Register` got {:?}", dst)
        };
        let Operand::Register(src) = rhs else {
            unreachable!("Unexpected enum variant for `Operand` expected `Register` got {:?}", dst)
        };
        match op {
            Op::Add => {
                #[cfg(target_arch = "x86_64")]
                dynasm!(ops
                        ; add Rq(dst as u8), Rq(src as u8)
                );
            }
            Op::Sub => {
                #[cfg(target_arch = "x86_64")]
                dynasm!(ops
                        ; sub Rq(dst as u8), Rq(src as u8)
                );
            }
            Op::IMul => {
                #[cfg(target_arch = "x86_64")]
                dynasm!(ops
                        ; imul Rd(dst as u8), Rd(src as u8)
                );
            }
            _ => todo!(),
        }
    }

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
    use std::fs::File;
    use std::io::Write;

    use dynasmrt::dynasm;
    use dynasmrt::DynasmApi;
    use std::env;
    use std::path::Path;

    use super::JitCache;
    use super::Operand;
    use super::Register;
    use crate::jvm::read_class_file;
    use crate::jvm::JVMParser;
    use crate::program::Program;
    use crate::runtime::{Runtime, Value};

    macro_rules! run_jit_test_case {
        ($name: ident, $test_file:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
                let path = Path::new(&env_var).join($test_file);
                let class_file_bytes =
                    read_class_file(&path).unwrap_or_else(|_| {
                        panic!("Failed to parse file : {:?}", path.as_os_str())
                    });
                let class_file = JVMParser::parse(&class_file_bytes);
                assert!(class_file.is_ok());
                let program = Program::new(&class_file.unwrap());
                let mut runtime = Runtime::new(program);
                assert!(runtime.run().is_ok());
                assert_eq!(runtime.top_return_value(), $expected);
            }
        };
    }

    #[test]
    fn can_emit_loads() {
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();
        let _prologue = prologue!(ops);
        JitCache::emit_mov(
            &mut ops,
            &Operand::Register(Register::Rax),
            &Operand::Memory(Register::Rdi, 1),
        );
        let _epilogue = epilogue!(ops);

        let _ = ops.finalize().and_then(|buf| {
            let mut testfile = File::create("test.bin").unwrap();
            let _ = testfile.write_all(&buf.to_vec());
            Ok(buf)
        });
    }
    run_jit_test_case!(
        loops,
        "support/tests/Loop.class",
        Some(Value::Int(1000))
    );
}

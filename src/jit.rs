//! JIT compiler for coldrew targeting x86_64.
use std::collections::{HashMap, VecDeque};

use crate::bytecode::OPCode;
use crate::runtime::{Frame, ProgramCounter, Value};
use crate::trace::Trace;

use dynasmrt::x64::Assembler;
use dynasmrt::{
    dynasm, AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi,
    ExecutableBuffer,
};

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
#[allow(dead_code)]
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

/// Intel x86-64 shorthand for instructions.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum Inst {
    Add,
    Sub,
    IMul,
    IDiv,
    IRem,
    Jge,
    Jg,
    Jle,
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
}

/// x86_64 function prologue, allocates `max_locals` space on the stack even
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

/// `JitCache` is responsible for compiling, caching and executing the native
/// traces.
///
/// The calling convention for our Jit is the following :
///
/// - Rdi & Rsi are used to pass input arguments which are the local variables
/// in the current frame and a guard program counter which is the entry point
/// of our native trace.
///
/// - Rax, Rbx, Rcx and R9-R15 are used for intermediate operations.
///
/// Since every trace is self contained all register allocation is local and
/// done with a simple queue based scheme.
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
            let mut locals = vec![0i32; frame.max_locals as usize * 8];
            // Exit information, for now is empty.
            let exits = [0i32; 0];

            for (key, val) in frame.locals.iter() {
                locals[*key] = match val {
                    Value::Int(x) => *x,
                    Value::Long(x) => *x as i32,
                    Value::Float(x) => *x as i32,
                    Value::Double(x) => *x as i32,
                };
            }

            let entry = trace.0;
            let buf = &trace.1;
            let execute: fn(*mut i32, *const i32) -> i32 =
                unsafe { std::mem::transmute(buf.ptr(entry)) };

            let exit_pc = execute(locals.as_mut_ptr(), exits.as_ptr()) as usize;
            frame.locals.clear();
            for (index, value) in locals.iter().enumerate() {
                frame.locals.insert(index, Value::Int(*value));
            }

            frame.pc.instruction_index = exit_pc as usize;
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
        // Reset Jit state.
        let pc = recording.start;
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();
        // Prologue for dynamically compiled code.
        let offset = prologue!(ops);
        let mut exit_pc = 0i32;
        // Trace compilation :
        // For now we compile only the prologue and epilogue and ensure that
        // entering the Jit executing the assembled code and leaving the Jit
        // works correct.
        for entry in &recording.trace {
            // Record the instruction program counter to a new label.
            let inst_label = ops.new_dynamic_label();
            let _ = self.labels.insert(entry.pc(), inst_label);
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
                    let value = match entry.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                        _ => unreachable!("Operand to iload (index in locals) must be int in current implementation")
                    };
                    let dst = self.first_available_register();

                    #[cfg(target_arch = "x86_64")]
                    dynasm!(ops
                        ; =>inst_label
                    );
                    Self::emit_mov(
                        &mut ops,
                        &dst,
                        &Operand::Memory(Register::Rdi, 4 * value),
                    );
                    self.operands.push(dst);
                }
                OPCode::IStore
                | OPCode::IStore0
                | OPCode::IStore1
                | OPCode::IStore2
                | OPCode::IStore3 => {
                    let value = match entry.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                            _ => unreachable!("Operand to istore (index in locals) must be int in current implementation")
                    };
                    if let Some(src) = self.free_register() {
                        dynasm!(ops
                            ; =>inst_label
                        );
                        Self::emit_mov(
                            &mut ops,
                            &Operand::Memory(Register::Rdi, 4 * value),
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
                    #[cfg(target_arch = "x86_64")]
                    dynasm!(ops
                        ; =>inst_label
                    );
                    self.emit_arithmetic(&mut ops, Inst::Add);
                }
                OPCode::ISub => {
                    #[cfg(target_arch = "x86_64")]
                    dynasm!(ops
                        ; =>inst_label
                    );
                    self.emit_arithmetic(&mut ops, Inst::Sub);
                }
                OPCode::IMul => {
                    #[cfg(target_arch = "x86_64")]
                    dynasm!(ops
                        ; =>inst_label
                    );
                    self.emit_arithmetic(&mut ops, Inst::IMul);
                }
                OPCode::IDiv => {
                    #[cfg(target_arch = "x86_64")]
                    dynasm!(ops
                        ; =>inst_label
                    );
                    self.emit_div(&mut ops, Inst::IDiv);
                }
                OPCode::IRem => {
                    #[cfg(target_arch = "x86_64")]
                    dynasm!(ops
                        ; =>inst_label
                    );
                    self.emit_div(&mut ops, Inst::IRem);
                }
                OPCode::IInc => {
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
                        ; =>inst_label
                    );
                    dynasm!(ops
                        ; add [Rq(Register::Rdi as u8) + 4* index], constant as _
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
                    let target = match entry.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                            _ => unreachable!("First operand to goto (relative offset) must be int")
                    };
                    if let Some(label) = self.labels.get(&ProgramCounter::new(
                        entry.pc().get_method_index(),
                        (entry.pc().get_instruction_index() as isize
                            + target as isize) as usize,
                    )) {
                        #[cfg(target_arch = "x86_64")]
                        dynasm!(ops
                            ; jmp =>*label
                        );
                    }
                }
                // if_icmp{cond} compares the top two values on the stack
                // and branches to the target offset given as an operand
                // if the comparison is not true.
                // Since our traces are self contained to the loop code
                // the target offset will be the exit pc value at which
                // the interpreter should continue execution.
                OPCode::IfICmpGe
                | OPCode::IfICmpGt
                | OPCode::IfICmpLe
                | OPCode::IfICmpEq => {
                    let target = match entry.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                            _ => unreachable!("First operand to if_icmpge (relative offset) must be int")
                    };
                    let mnemonic = entry.instruction().get_mnemonic();
                    exit_pc = (entry.pc().get_instruction_index() as isize
                        + target as isize) as i32;

                    self.emit_cond_branch(&mut ops, mnemonic);
                }
                OPCode::IfEq => {
                    let operand = self.free_register();
                    match operand {
                        Some(Operand::Register(reg)) => {
                            #[cfg(target_arch = "x86_64")]
                            dynasm!(ops
                                ; cmp Rq(reg as u8), 0
                                ; je ->abort_guard
                            );
                        }
                        Some(Operand::Memory(base, offset)) => {
                            #[cfg(target_arch = "x86_64")]
                            dynasm!(ops
                                ; cmp [Rq(base as u8) + offset], 0
                                ; je ->abort_guard
                            );
                        }
                        _ => unreachable!("expected operand for if_eq to be either `Operand::Memory` or `Operand::Register`"),
                    }
                }
                OPCode::IfNe => {
                    let operand = self.free_register();
                    match operand {
                        Some(Operand::Register(reg)) => {
                            #[cfg(target_arch = "x86_64")]
                            dynasm!(ops
                                ; cmp Rq(reg as u8), 0
                                ; jz ->abort_guard
                            );
                        }
                        Some(Operand::Memory(base, offset)) => {
                            #[cfg(target_arch = "x86_64")]
                            dynasm!(ops
                                ; cmp [Rq(base as u8) + offset], 0
                                ; jz ->abort_guard
                            );
                        }
                        _ => unreachable!("expected operand for if_eq to be either `Operand::Memory` or `Operand::Register`"),
                    }
                }
                _ => (),
            }
        }
        #[cfg(target_arch = "x86_64")]
        dynasm!(ops
            ; ->abort_guard:
            ; mov rax, exit_pc as _
        );
        // Epilogue for dynamically compiled code.
        epilogue!(ops);

        let buf = ops.finalize().unwrap();

        let native_trace = NativeTrace(offset, buf);
        self.traces.insert(pc, native_trace);
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

    /// Emit an arithmetic operation, covers only simple instructions such as
    /// `add`, `mul` and `sub`.
    fn emit_arithmetic(&mut self, ops: &mut Assembler, op: Inst) {
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
        if let Operand::Register(reg) = &rhs {
            self.registers.push_back(*reg)
        }

        self.operands.push(dst);

        match op {
            Inst::Add => {
                let Operand::Register(dst) = dst else {
                    unreachable!("Unexpected enum variant for `Operand` expected `Register` got {:?}", dst)
                };

                match rhs {
                    Operand::Register(src) => {
                        #[cfg(target_arch = "x86_64")]
                        dynasm!(ops
                                ; add Rq(dst as u8), Rq(src as u8)
                        );
                    },
                    Operand::Immediate(val) => {
                        #[cfg(target_arch = "x86_64")]
                        dynasm!(ops
                                ; add Rq(dst as u8), val as _
                        );
                    },
                    Operand::Memory(base, offset) => {
                        #[cfg(target_arch = "x86_64")]
                        dynasm!(ops
                                ; add Rq(dst as u8), [Rq(base as u8) + offset]
                        );
                    },
                }
            }
            Inst::Sub => {
                let Operand::Register(dst) = dst else {
                    unreachable!("Unexpected enum variant for `Operand` expected `Register` got {:?}", dst)
                };

                match rhs {
                    Operand::Register(src) => {
                        #[cfg(target_arch = "x86_64")]
                        dynasm!(ops
                                ; sub Rq(dst as u8), Rq(src as u8)
                        );
                    },
                    Operand::Immediate(val) => {
                        #[cfg(target_arch = "x86_64")]
                        dynasm!(ops
                                ; sub Rq(dst as u8), val as _
                        );
                    },
                    Operand::Memory(base, offset) => {
                        #[cfg(target_arch = "x86_64")]
                        dynasm!(ops
                                ; sub Rq(dst as u8), [Rq(base as u8) + offset]
                        );
                    },
                }
            }
            Inst::IMul => {
                let Operand::Register(dst) = dst else {
                    unreachable!("Unexpected enum variant for `Operand` expected `Register` got {:?}", dst)
                };
                match rhs {
                    Operand::Register(src) => {
                        #[cfg(target_arch = "x86_64")]
                        dynasm!(ops
                                ; imul Rq(dst as u8), Rq(src as u8)
                        );
                    },
                    Operand::Immediate(val) => {
                        #[cfg(target_arch = "x86_64")]
                        dynasm!(ops
                                ; imul Rq(dst as u8), Rq(dst as u8), val as _
                        );
                    },
                    Operand::Memory(base, offset) => {
                        #[cfg(target_arch = "x86_64")]
                        dynasm!(ops
                                ; imul Rq(dst as u8), [Rq(base as u8) + offset]
                        );
                    },
                }
            }
            _ => unreachable!("emit_arithmetic only supports simple x86-64 arithmetic (add, sub and mul).)"),
        }
    }

    /// Emit division operation.
    fn emit_div(&mut self, ops: &mut Assembler, op: Inst) {
        let rdx = Register::Rdx;
        let rax = Register::Rax;

        let denom = match self.operands.pop() {
            Some(operand) => operand,
            _ => {
                unreachable!("Expected operand for `idiv` and `irem` got None")
            }
        };

        if let Some(nom) = self.free_register() {
            JitCache::emit_mov(ops, &Operand::Register(Register::Rax), &nom);
        }
        let dst = match denom {
            Operand::Register(reg) => Operand::Register(reg),
            _ => {
                let reg = self.first_available_register();
                JitCache::emit_mov(ops, &reg, &denom);
                reg
            }
        };

        let src = match op {
            // x86 division rax holds divident rdx holds modulo.
            Inst::IDiv => rax,
            Inst::IRem => rdx,
            _ => unreachable!("emit_div expected op to be idiv or irem"),
        };

        #[cfg(target_arch = "x86_64")]
        let Operand::Register(dst_reg) = dst
        else {
            unreachable!("Unexpected enum variant for `Operand` expected `Register` got {:?}", dst)
        };
        dynasm!(ops
            ; mov Rq(rdx as u8), 0
            ; div Rq(dst_reg as u8)
        );
        JitCache::emit_mov(ops, &dst, &Operand::Register(src));
        self.operands.push(dst);
    }

    /// Emit conditional branch for the given instruction.
    fn emit_cond_branch(&mut self, ops: &mut Assembler, cond: OPCode) {
        let rhs = match self.free_register() {
            Some(operand) => operand,
            None => panic!("expected operand found None"),
        };
        let lhs = match self.free_register() {
            Some(operand) => operand,
            None => todo!("Expected register in operand stack found None"),
        };

        match (lhs, rhs) {
            (Operand::Register(lhs), Operand::Register(rhs)) => {
                dynasm!(ops
                    ; cmp Rq(lhs as u8), Rq(rhs as u8)
                );
            }
            (Operand::Register(lhs), Operand::Memory(base, offset)) => {
                dynasm!(ops
                    ; cmp Rq(lhs as u8), [Rq(base as u8) + offset]
                );
            }
            (Operand::Register(lhs), Operand::Immediate(imm)) => {
                dynasm!(ops
                    ; cmp Rq(lhs as u8), imm as _
                );
            }
            (Operand::Memory(base, offset), Operand::Register(rhs)) => {
                dynasm!(ops
                    ; cmp [Rq(base as u8) + offset], Rq(rhs as u8)
                );
            }
            (Operand::Memory(base, offset), Operand::Immediate(imm)) => {
                dynasm!(ops
                    ; cmp [Rq(base as u8) + offset], imm as _
                );
            }
            _ => unreachable!(
                "unsupported comparison between operands {:?} and {:?}",
                lhs, rhs
            ),
        }

        match cond {
            OPCode::IfICmpGt => {
                dynasm!(ops
                    ; jg ->abort_guard
                );
            }
            OPCode::IfICmpGe => {
                dynasm!(ops
                    ; jge ->abort_guard
                );
            }
            OPCode::IfICmpLe => {
                dynasm!(ops
                    ; jle -> abort_guard
                );
            }
            OPCode::IfICmpEq => {
                dynasm!(ops
                    ; je -> abort_guard
                );
            }
            _ => unreachable!("Expected instruction for conditional branch to be a if_icmp<cond> {:?}", cond)
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
        if let Some(Operand::Register(reg)) = op {
            self.registers.push_back(reg)
        }
        op
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::Path;

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
                assert!(runtime.run(true).is_ok());
                assert_eq!(runtime.top_return_value(), $expected);
            }
        };
    }

    run_jit_test_case!(
        loops,
        "support/tests/HotLoop.class",
        Some(Value::Int(55))
    );
}

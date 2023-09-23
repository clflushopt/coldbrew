//! JIT compiler for coldrew targeting arm64.
//!
//! # Notes about arm64
//!
//! `dynasm` has two ways of dynamically assigning registers :
//!
//! - X(d) register family where  X31 is XZR (Zero Register)
//! - XSP(d) register family where register 31 is SP (Stack Pointer)
//!
//! On ARM64 registers are X0~X31 and are 64-bit wide with their 32-bit
//! sub quantities addresses using W0~W31, ARM64 is exclusively little
//! endian so when we say something like `w4[0..8] = 0xA8` imagine something
//! like this (assume the remaining bit fields are populated by 0's or 1's):
//!
//! | | | | | | | | | | | | | | | | | | | | | | | |1|0|1|0|1|0|0|0|
//!
//! # Moves
//!
//! The `mov` instruction writes an immediate value to a register, it comes in
//! different flavors :
//! - (bitmask immediate): `mov <Xd|SP>, #<imm>`
//!
//! - (inverted wide immediate): `mov <Xd>, #imm[16], LSL#<shift>`
//! where shift = {0,16,32,48} is the amount of the left shift
//!
//! - (register copy): `mov <Xd>, <Xm>` (copy Xm into Xd)
//!
//! - (stack pointer copy): `mov <Xd|SP>, <Xn|SP>`
//!
//! - (wide immediate): `mov <Xd>, #imm`
//!
//! - (move with keep): `movk <Xd>, #imm{,LSL #<shift>}`
//!
//! The above are useful for moving immediates into registers.
//!
//! # Loads and Stores
//!
//! ARM is a load and store oriented architecture with no direct memory access
//! like in x86. There are two main load and store operations :
//!
//! - `LDR <Dst>, [<address>]` : Load from value @ `<address>` to `<Dst>`
//! - `STR <Src>, [<address>]` : Store value @ `<address>` from `<Src>`
//!
//! The size of the load and store is determined from the register
//! for example Xd registers are 64-bit wide, but we can still use Wd
//! as a short hand to only populate the low 32 bits e.g
//!
//! `ldr w0, [sp, #16]` : Load 32 bit value from SP + 16 into w0
//! `ldr x0, [sp, #16]` : Load 64 bit value from SP + 16 into x0
//!
//! There are variants to LDR/STR that are suffixed by a <size> quantity
//! - `strb w0, [sp, #16]` : Load bottom byte of w0 to [address] (u8/char)
//! - `strh w0, [sp, #16]` : Load bottom 2 bytes of w0 to [address] (u16,short)
//! - `strw w0, [sp, #16]` : Load bottom 4 bytes of w0 to [address] (u32, int)
//!
//! The above instructions respect zero and sign extension for example in case
//!
//! `ldr w0, [sp, #16]` the top 32 bits of x0 are zeroed by default.
//! `ldrb w4, <addr>` Will read one byte from <addr> e.g 0x8A into w4[0..8]
//! the remaining bits w[8..31] will be zero extended.
//!
//! The second variants to LDR/STR are suffixed by `S` to mean sign extension
//! the range of sign extension will depend on the target register.
//!
//! `ldrsb w4, <addr>` will load 0xA8 from <addr> into w4[0..8] and w4[8..31] = 0xff
//!
//! `ldrsb x4, <addr>` will load 0xA8 from <addr> into x4[0..8] and x4[8..63] = 0xff
//!
//! Another example :
//!
//! `ldrsb x4, [0x8000]` assume [0x8000] = 0x1F then x4[0..8] = 0x1F and x[8..63] = 0x00.
//!
//! The reason is that 0x1F in binary is '0b00011111' where the MSB is 0 therefore sign
//! extension will fill the rest of the registers by 0x00 = 0b00000000
//!
//! On the other hand 0xA8 in binary is '0b10101000' where the MSB is 1 therefore sign
//! extension will fill the rest of the registers by 0xFF = 0b11111111
//!
//! # Addressing modes in Loads and Stores
//!
//! Generally all assembly for loads and store will look something like this :
//!
//! `ldr W0, [X]` where `X` is the address within bracket, now how the address
//! is accessed is what we call addressing modes.
//!
//! - Base Register Mode: `X` is a register that contains the virtual address to access.
//!
//! - Offset mode: uses a combination of a base address and an offset such `X = [X1, #12]`
//! which says to access virtual address that's stored in `X1 + 12` this mode always
//! assume the offset is a byte offset, the offset itself can also be a register e.g `X1 + X0`.
//!
//! - Pre-index mode: Pre-indexing is shown with `!` example `ldr W0, [X1, #12]!` this is
//! similar to the offset mode except that we update `X1` to store `X1 + 12`.
//!
//! - Post-index mode: Post indexing looks like this `ldr W0, [X1], #12` and it stays to acess
//! the virtual address @ X1 *AND AFTER* update `X1` to `X1 + 12`.
//!
//! Post indexing mode is weird but useful for popping off the stack since it basically moves a
//! value pointed at by the stack pointer and the updates the stack pointer to point to the next
//! value.
//!
//! # Branching and Control Flow
//!
//! There are two unconditional branch instructions `B` for PC-relative branches and `BR` for
//! register indirect branches.
//!
//! - `B <label>` : Branches to label, the offset from PC to `label` is encoded in the instruction
//! as an immediate and can be in the range +/-128MB, to see this consider that instructions in ARM
//! are 32-bit wide (fixed) the branch instruction is encoded as [0,0,0,1,0,1,IMM[26..0]] where we
//! use 26-bits to encode an immediate, to get the offset multiply the immediate by 4.
//!
//! So 2**26 * 4 is 256MB since relative addressing can be in both directions that's +/-128MB
//!
//! - `BR <Xd>` : Branch indirectly to memory address stored @ `<Xd>`
//!
//! There is a conditional variant of the branch instruction `B.<cond> <label>` which jumps
//! to `<label>` if `<cond>` is true. The `<cond>` is set in the ALU flags in the Program State
//! also called `PSTATE`.
//!
use std::collections::VecDeque;

use crate::bytecode::OPCode;

use crate::runtime::ProgramCounter;
use crate::runtime::Value;
use crate::trace::Recording;
use dynasmrt::aarch64::Assembler;
use dynasmrt::components::LitPool;
use dynasmrt::dynasm;
use dynasmrt::AssemblyOffset;
use dynasmrt::DynasmApi;

/// ARM64 (aarch64) registers, mainly used to keep track of available
/// and used registers during compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Register {
    // Arguments and return values.
    X0 = 0x0,
    X1 = 0x1,
    X2 = 0x2,
    X3 = 0x3,
    X4 = 0x4,
    X5 = 0x5,
    X6 = 0x6,
    X7 = 0x7,
    // Indirect result.
    X8 = 0x8,
    // Temporary.
    X9 = 0x9,
    X10 = 0xA,
    X11 = 0xB,
    X12 = 0xC,
    X13 = 0xD,
    X14 = 0xE,
    X15 = 0xF,
    // Intra-procedure call temporaries.
    X16 = 0x10,
    X17 = 0x11,
    // Platform defined usage.
    X18 = 0x12,
    // Temporary (must be preserved).
    X19 = 0x13,
    X20 = 0x14,
    X21 = 0x15,
    X22 = 0x16,
    X23 = 0x17,
    X24 = 0x18,
    X25 = 0x19,
    X26 = 0x1A,
    X27 = 0x1B,
    X28 = 0x1C,
    // Stack/Frame pointer (must be preserved).
    X29 = 0x1D,
    // Link Register/Return address.
    X30 = 0x1E,
    // Zero register.
    X31 = 0x1F,
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

/// aarch64 function prologue.
macro_rules! prologue {
    ($ops:ident) => {{
        let start = $ops.offset();
        dynasm!($ops
            ; str x30, [sp, #-16]!
            ; stp x0, x1, [sp, #-16]!
            ; stp x2, x3, [sp, #-32]!
        );
        start
    }};
}

/// aarch64 function epilogue.
macro_rules! epilogue {
    ($ops:ident) => {dynasm!($ops
        // Load return value that we assume
        // is the third stack variable.
        ; ldr w0, [sp, #12]
        // Increment stack pointer to go back to where we were
        // before the function call.
        ; add sp, sp, #32
        ; ldr x30, [sp], #16
        ; ret
    );};
}
/// `JitCache` is responsible for compiling and caching recorded native traces.
pub struct JitCache {
    // Internal cache of available registers.
    registers: VecDeque<Register>,
    // Operand stack.
    operands: Vec<Operand>,
}

impl Default for JitCache {
    fn default() -> Self {
        Self::new()
    }
}

impl JitCache {
    // Create a new JIT compilation cache.
    pub fn new() -> Self {
        let registers = vec![
            Register::X9,
            Register::X10,
            Register::X11,
            Register::X12,
            Register::X13,
            Register::X14,
            Register::X15,
            Register::X19,
            Register::X20,
            Register::X21,
            Register::X22,
            Register::X23,
            Register::X24,
            Register::X25,
            Register::X26,
            Register::X27,
            Register::X28,
        ];
        JitCache {
            registers: VecDeque::from(registers),
            operands: Vec::new(),
        }
    }

    // Compile the trace given as argument and prepare a native trace
    // for execution.
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
    ) -> (AssemblyOffset, Assembler) {
        let mut ops = dynasmrt::aarch64::Assembler::new().unwrap();
        // Prologue for dynamically compiled code.
        let offset = prologue!(ops);
        // Trace compilation
        for trace in &recording.trace {
            match trace.instruction().get_mnemonic() {
                OPCode::ILoad0
                | OPCode::ILoad1
                | OPCode::ILoad2
                | OPCode::ILoad3 => {
                    let value = match trace.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                        _ => todo!(),
                    };
                    let dst = self.first_available_register();
                    // Emit a load.
                    self.emit_mov(
                        &mut ops,
                        &dst,
                        &Operand::Memory(Register::X14, 8 * value),
                    );
                    self.operands.push(dst);
                }
                OPCode::IStore0
                | OPCode::IStore1
                | OPCode::IStore2
                | OPCode::IStore3 => {
                    let value = match trace.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                        _ => todo!(),
                    };
                    let dst = self.free_register().unwrap();
                    // Emit a store.
                    self.emit_mov(
                        &mut ops,
                        &Operand::Memory(Register::X14, 8 * value),
                        &dst,
                    );
                }
                OPCode::BiPush | OPCode::SiPush | OPCode::Ldc => {
                    let value = match trace.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                        _ => todo!(),
                    };

                    self.operands.push(Operand::Immediate(value))
                }
                OPCode::IInc => {
                    let value = match trace.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                        _ => todo!(),
                    };
                    let imm = match trace.instruction().nth(0) {
                        Some(Value::Int(x)) => x,
                        _ => todo!(),
                    };
                    // let dst = self.first_available_register();
                    // Load immediate into register
                    // self.emit_mov(&mut ops, &dst, &Operand::Immediate(imm));

                    // Load memory into a register
                    let dst = self.first_available_register();
                    self.emit_mov(
                        &mut ops,
                        &dst,
                        &Operand::Memory(Register::X14, 8 * value),
                    );
                    let dst = match dst {
                        Operand::Register(reg) => reg,
                        _ => panic!("Wanted register"),
                    };
                    // Add dst, dst, #imm
                    let mut litpool = LitPool::new();
                    let x = 0xffffffffffffffff;
                    let offset = litpool.push_u64(x);
                    dynasm!(ops
                        ; ldr x0, offset
                        ; add XSP(dst as u32), XSP(dst as u32), imm as u32
                    );
                    // Store dst back into memory
                    self.emit_mov(
                        &mut ops,
                        &Operand::Memory(Register::X14, 8 * value),
                        &Operand::Register(dst),
                    );
                    self.free_register().unwrap();
                }
                _ => (),
            }
        }

        // Epilogue for dynamically compiled code.
        epilogue!(ops);

        (offset, ops)
    }

    // Emit a load operation, where `dst` must be a register and `src` a memory
    // address.
    fn emit_load(&mut self, ops: &mut Assembler, dst: &Operand, src: &Operand) {
        // Following the ARM convention we expect `dst` to be a register
        match (dst, src) {
            (Operand::Register(dst), Operand::Memory(base, offset)) => {
                dynasm!(ops
                    ; ldr X(*dst as u32), [X(*base as u32), *offset as u32]
                )
            }
            _ => panic!(
                "Expected load instruction argument to be (register, address)"
            ),
        }
    }

    // Emit a move operation, this includes all data movement operations
    // register to register and immediate to register.
    // For memory accesses we follow the aarch64 story of generating all
    // necessary stores and loads.
    fn emit_mov(&mut self, ops: &mut Assembler, dst: &Operand, src: &Operand) {
        // Early return if the move is considered useless.
        if dst == src {
            return;
        }
        // If the destination operand is a register and the source operand is
        // either a register or an immediate we emit `mov dst, src`.
        match (dst, src) {
            // Direct register copies.
            (Operand::Register(dst), Operand::Register(src)) => {
                dynasm!(ops
                    ; mov X(*dst as u32), X(*src as u32)
                );
            }
            // Direct immediate copies.
            (Operand::Register(dst), Operand::Immediate(imm)) => {
                dynasm!(ops
                    ; mov X(*dst as u32), #*imm as u64
                );
            }
            // Indirect memory load into a destination register.
            (Operand::Register(dst), Operand::Memory(base, offset)) => {
                dynasm!(ops
                    ; ldr X(*dst as u32), [X(*base as u32), *offset as u32]
                );
            }
            // Indirect memory store into a destination register
            (Operand::Memory(base, offset), Operand::Register(dst)) => {
                dynasm!(ops
                    ; str X(*dst as u32), [X(*base as u32), *offset as u32]
                );
            }
            _ => todo!(),
        }
    }

    // Emit an arithmetic operation, covers all simple instructions such as
    // `add`, `mul` and `sub`.
    fn emit_arithmetic(&mut self, ops: &mut Assembler) {
        let op2 = match self.operands.pop() {
            Some(operand) => operand,
            None => panic!("expected operand found None"),
        };
        let op1 = match self.operands.pop() {
            Some(operand) => operand,
            None => panic!("expected operand found None"),
        };
        let dst = match op1 {
            Operand::Register(_) => op1,
            _ => {
                let _dst = self.first_available_register();
                // Generate a mov dst, op1
                self.emit_mov(ops, &_dst, &op1);
                _dst
            }
        };

        if let Operand::Register(reg) = op2 {
            self.registers.push_back(reg)
        }

        match (dst, op2, dst) {
            (
                Operand::Register(r1),
                Operand::Register(r2),
                Operand::Register(r3),
            ) => {
                dynasm!(
                    ops
                    ; add X(r3 as u32), X(r1 as u32), X(r2 as u32)
                )
            }
            _ => todo!(),
        }
    }

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

    use std::slice;

    use crate::arm64::{self};

    use super::*;
    use dynasmrt::dynasm;
    use dynasmrt::{DynasmApi, ExecutableBuffer};

    fn prebuilt_test_fn_aarch64(
        buffer: &mut ExecutableBuffer,
    ) -> dynasmrt::AssemblyOffset {
        let mut ops = dynasmrt::aarch64::Assembler::new().unwrap();

        let start = prologue!(ops);
        let target = Register::X8 as u32;
        let addr = 16;
        dynasm!(ops
            // int c = a + b;
            ; ldr X(target), [sp, #24]
            ; ldr X(9), [sp, #addr]
            ; add X(8), x8, x9
            ; str w8, [sp, #12]
        );
        epilogue!(ops);
        *buffer = ops.finalize().unwrap();
        return start
    }

    fn build_test_fn_x86(
        buffer: &mut ExecutableBuffer,
    ) -> dynasmrt::AssemblyOffset {
        let mut builder = dynasmrt::x64::Assembler::new();

        dynasm!(builder.as_mut().expect("REASON")
            ; movz x0, 42
            ; movz x1, 13//, lsl 16
            ; add x0, x0, x1
            ; ret
        );
        let _offset = builder.as_ref().expect("REASON").offset();
        *buffer = builder.expect("REASON").finalize().unwrap();
        return dynasmrt::AssemblyOffset(0)
    }

    fn build_test_fn_imm(
        buffer: &mut ExecutableBuffer,
    ) -> dynasmrt::AssemblyOffset {
        let mut builder = dynasmrt::x64::Assembler::new();
        let mut litpool = LitPool::new();
        let _offset = litpool.push_u64(0xcafebabe);
        let a = 0xcafebabe;
        let _b = 0x0;

        use arm64;
        

        // let lo = a & mask(16, 0);
        let (hi, lo) = arm64::split(a);
        println!("{:#x}", lo);
        // let hi = a >> 16 as u32;

        println!("{:#x}", hi);

        dynasm!(builder.as_mut().expect("REASON")
            ; movz x0, lo
            ; movk x0, hi, LSL #16
            ; movz x1, #0
            ; add x0, x0, x1
            ; ret
        );
        litpool.emit(builder.as_mut().unwrap());
        let _offset = builder.as_ref().expect("REASON").offset();
        *buffer = builder.expect("REASON").finalize().unwrap();
        return dynasmrt::AssemblyOffset(0)
    }

    fn build_test_fn_aarch64(
        buffer: &mut ExecutableBuffer,
    ) -> dynasmrt::AssemblyOffset {
        let mut builder = dynasmrt::aarch64::Assembler::new();
        dynasm!(builder.as_mut().expect("expected builder to be mutable")
            // Prologue call stack preparation <> add(sp...)
            ; sub     sp, sp, #32
            ; str     x0, [sp, #24]
            ; str     x1, [sp, #16]
            // int c = a + b;
            ; ldr x8, [sp, #24]
            ; ldr x9, [sp, #16]
            ; add x8, x8, x9
            ; str w8, [sp, #12]
            // Epilogue call stack cleanup return c
            ; ldr W(0), [sp, #12]
            ; add sp, sp, #32
            ; ret
        );
        let _offset = builder
            .as_ref()
            .expect("expected valid reference to builder")
            .offset();
        *buffer = builder.expect("expected builder").finalize().unwrap();
        return dynasmrt::AssemblyOffset(0)
    }

    #[ignore = "ignore until unsafe segfault bug is fixed"]
    #[test]
    fn test_dynasm_buffer() {
        // Create a buffer to hold the generated machine code
        let mut buffer = ExecutableBuffer::new(8012).unwrap();

        // Build the function using Dynasm
        let code_offset = build_test_fn_x86(&mut buffer);

        let code_offset_aarch64 = build_test_fn_aarch64(&mut buffer);

        let prebuilt_code_offset_aarch64 =
            prebuilt_test_fn_aarch64(&mut buffer);

        // Execute the generated machine code
        let add_fn: extern "C" fn(u64, u64) -> u64 =
            unsafe { std::mem::transmute(buffer.ptr(code_offset)) };

        let add_fn_aarch64: extern "C" fn(u64, u64) -> u64 =
            unsafe { std::mem::transmute(buffer.ptr(code_offset_aarch64)) };

        let prebuilt_add_fn_aarch64: extern "C" fn(u64, u64) -> u64 = unsafe {
            std::mem::transmute(buffer.ptr(prebuilt_code_offset_aarch64))
        };

        let mut buffer = ExecutableBuffer::new(8012).unwrap();
        let code_offset_imm = build_test_fn_imm(&mut buffer);
        let add_fn_imm: fn(u64, u64) -> u64 =
            unsafe { std::mem::transmute(buffer.ptr(code_offset_imm)) };

        let result_imm = add_fn_imm(0, 0);
        // Call the generated function and print the result
        let result = add_fn(42, 13);
        let result_aarch64 = add_fn_aarch64(42, 13);
        let result_prebuilt_aarch64 = prebuilt_add_fn_aarch64(42, 13);
        assert_eq!(result, 55);
        assert_eq!(result_aarch64, 55);
        assert_eq!(result_prebuilt_aarch64, 55);
        assert_eq!(result_imm, 0xcafebabe);
    }

    #[ignore = "ignore until JIT is fully implemented"]
    #[test]
    fn test_jit_traces() {
        use crate::jvm::{read_class_file, JVMParser};
        use crate::program::Program;
        use crate::runtime::Runtime;
        use std::env;
        use std::path::Path;
        let test_file = "support/tests/HotLoop.class";
        let env_var = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(&env_var).join(test_file);
        let class_file_bytes = read_class_file(&path).unwrap_or_else(|_| {
            panic!("Failed to parse file : {:?}", path.as_os_str())
        });
        let class_file = JVMParser::parse(&class_file_bytes);
        assert!(class_file.is_ok());
        let program = Program::new(&class_file.unwrap());
        let mut runtime = Runtime::new(program);
        runtime.run().unwrap();
        assert_eq!(runtime.top_return_value(), Some(Value::Int(55)));
        let trace = runtime.recorder.recording();
        let mut jit = JitCache::new();
        let (offset, asm) = jit.compile(&trace);
        let code = asm.finalize().unwrap();
        let buf =
            unsafe { slice::from_raw_parts(code.ptr(offset), code.len()) };
        use std::fs::File;
        use std::io::prelude::*;
        let mut out = File::create("test.asm").unwrap();
        out.write_all(buf);

        let dummy: fn(u64, u64) -> u64 =
            unsafe { std::mem::transmute(code.ptr(offset)) };
        let res = dummy(0, 0);
        println!("Result : {res}");
    }

    #[test]
    fn test_inline_asm() {
        use std::arch::asm;

        struct MyStruct {
            registers: [u64; 8],
        }

        struct MyVecStruct {
            registers: Vec<u64>,
        }

        let mut my_struct = MyStruct { registers: [0; 8] };

        let _my_vec = MyVecStruct {
            registers: vec![0u64; 8],
        };

        unsafe {
            asm!(
                "mov x0, 1",
                "mov x1, 2",
                "mov x2, 3",
                "mov x3, 4",
                "mov x4, 5",
                "mov x5, 6",
                "mov x6, 7",
                "mov x7, 8",
                "str x0, [{0}]", // Store x0 in registers[0]
                "str x1, [{0}, #8]", // Store x1 in registers[1]
                "str x2, [{0}, #16]", // Store x2 in registers[2]
                "str x3, [{0}, #24]", // Store x3 in registers[3]
                "str x4, [{0}, #32]", // Store x4 in registers[4]
                "str x5, [{0}, #40]", // Store x5 in registers[5]
                "str x6, [{0}, #48]", // Store x6 in registers[6]
                "str x7, [{0}, #56]", // Store x7 in registers[7]"
                inout(reg) &mut my_struct.registers => _,
                // inout(reg)  my_vec.registers.as_mut_ptr() as *mut u64 => _,
            );
        };

        // Now, my_struct.registers contains [1, 2, 3, 4, 5, 6, 7, 8]
        println!("{:?}", my_struct.registers);
    }
}

//! JIT compiler for coldrew.
use std::collections::VecDeque;

use crate::bytecode::OPCode;
use crate::runtime::ProgramCounter;
use crate::trace::Recording;
use dynasmrt::aarch64::Assembler;
use dynasmrt::dynasm;
use dynasmrt::AssemblyOffset;
use dynasmrt::DynasmApi;

/// aarch64 registers, mainly used to keep track of available
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
    X10 = 0x10,
    X11 = 0x11,
    X12 = 0x12,
    X13 = 0x13,
    X14 = 0x14,
    X15 = 0x15,
    // Intra-procedure call temporaries.
    X16 = 0x16,
    X17 = 0x17,
    // Platform defined usage.
    X18 = 0x18,
    // Temporary (must be preserved).
    X19 = 0x19,
    X20 = 0x20,
    X21 = 0x21,
    X22 = 0x22,
    X23 = 0x23,
    X24 = 0x24,
    X25 = 0x25,
    X26 = 0x26,
    X27 = 0x27,
    X28 = 0x28,
    // Frame pointer (must be preserved).
    X29 = 0x29,
    // Return address.
    X30 = 0x30,
    // Zero register.
    X31 = 0x31,
}

/// Operands.
enum Operand {
    // Register operands.
    Register(Register),
    // Immediate operands.
    Immediate(i32),
    // Memory operands.
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
            ; stp x2, x3, [sp, #-16]!
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
        JitCache {
            registers: VecDeque::new(),
            operands: Vec::new(),
        }
    }

    // Compile the trace given as argument and prepare a native trace
    // for execution.
    fn compile(recording: &Recording) -> AssemblyOffset {
        let mut ops = dynasmrt::aarch64::Assembler::new().unwrap();
        // Prologue for dynamically compiled code.
        let offset = prologue!(ops);
        // Trace compilation
        for trace in &recording.trace {
            match trace.instruction().get_mnemonic() {
                _ => (),
            }
        }

        // Epilogue for dynamically compiled code.
        epilogue!(ops);

        offset
    }

    // Emit an arithmetic operation.
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
                _dst
            }
        };

        if let Operand::Register(reg) = op2 {
            self.registers.push_back(reg)
        }
        dynasm!(ops
            ; add X(8), X(8), X(9)
        );
    }

    // Returns the first available register.
    fn first_available_register(&mut self) -> Operand {
        Operand::Register(Register::X0)
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::E;
    use std::os::raw::c_void;

    use super::*;
    use dynasmrt::dynasm;
    use dynasmrt::{DynasmApi, ExecutableBuffer};

    fn prebuilt_test_fn_aarch64(
        buffer: &mut ExecutableBuffer,
    ) -> dynasmrt::AssemblyOffset {
        let mut ops = dynasmrt::aarch64::Assembler::new().unwrap();

        let start = prologue!(ops);
        let target = Register::X8 as u32;
        dynasm!(ops
            // int c = a + b;
            ; ldr X(target), [sp, #24]
            ; ldr X(9), [sp, #16]
            ; add X(8), x8, x9
            ; str w8, [sp, #12]
        );
        epilogue!(ops);
        *buffer = ops.finalize().unwrap();
        start
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
        dynasmrt::AssemblyOffset(0)
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
        dynasmrt::AssemblyOffset(0)
    }

    #[test]
    fn test_dynasm_buffer() {
        // Create a buffer to hold the generated machine code
        let mut buffer = ExecutableBuffer::new(4096).unwrap();

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

        // Call the generated function and print the result
        let result = add_fn(42, 13);
        let result_aarch64 = add_fn_aarch64(42, 13);
        let result_prebuilt_aarch64 = prebuilt_add_fn_aarch64(42, 13);
        assert_eq!(result, 55);
        assert_eq!(result_aarch64, 55);
        assert_eq!(result_prebuilt_aarch64, 55);
    }
}

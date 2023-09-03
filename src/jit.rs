//! JIT compiler for coldrew.
use crate::trace::Recording;
use dynasmrt::dynasm;

/// Macros that are reusable when building traces.
///
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
    ($ops:ident, $e:expr) => {dynasm!($ops
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

/// Cache for managing JIT compiled traces.
pub struct JitCache {}

impl Default for JitCache {
    fn default() -> Self {
        Self::new()
    }
}

impl JitCache {
    // Create a new JIT compilation cache.
    pub fn new() -> Self {
        JitCache {}
    }

    // Compile the trace given as argument and prepare a native trace
    // for execution.
    fn compile(_recording: &Recording) {}
}

#[cfg(test)]
mod tests {
    use dynasmrt::dynasm;
    use dynasmrt::{DynasmApi, ExecutableBuffer};

    fn prebuilt_test_fn_aarch64(
        buffer: &mut ExecutableBuffer,
    ) -> dynasmrt::AssemblyOffset {
        let mut ops = dynasmrt::aarch64::Assembler::new().unwrap();

        let start = prologue!(ops);
        dynasm!(ops
            // int c = a + b;
            ; ldr x8, [sp, #24]
            ; ldr x9, [sp, #16]
            ; add x8, x8, x9
            ; str w8, [sp, #12]
        );
        epilogue!(ops, 0);
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
            ; ldr w0, [sp, #12]
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

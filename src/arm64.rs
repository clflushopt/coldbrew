//! Minimal ARM64 assembly module useful for doing ARM64 codegen.

/// ARM64 (aarch64) registers, mainly used to keep track of available
/// and used registers during compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(target_arch = "aarch64")]
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

/// Create a mask to extract n-bits of a given value from start.
pub fn mask(len: u64, start: u64) -> u64 {
    ((1 << len) - 1) << start
}

/// Split a u64 into two chunks of high and low bits.
pub fn split(x: u64) -> (u32, u32) {
    return ((x >> 16) as u32, (x & mask(16, 0)) as u32);
}

#[cfg(test)]
#[cfg(target = "aarch64")]
mod tests {
    use super::*;
    #[test]
    fn immediate_from_i32() {
        // Given the following immediate break it to separate bits to fit
        // an ARM64 instruction.
        // This is useful when loading immediates that don't fit the fixed
        // size instruction width of 32-bits.
        let x = 0x48f0d0i32;
        // To read x in x0 we can use a movz movk pair
        // movz x0, #0xf0d0
        // movk x0, #0x48, lsl #16
        // Which is equivalent to the following.
        let lo = x as u64 & mask(16, 0);
        let hi = x as u64 >> 16;
        assert_eq!((hi << 16 | lo) as i32, x);
        // Another example with an even bigger integer
        let v = 0x1122334455667788u64;
        let lo_1 = v & mask(16, 0);
        let lo_2 = v & mask(16, 16);
        let lo_3 = v & mask(16, 32);
        let lo_4 = v & mask(16, 48);
        assert_eq!(lo_4 | lo_3 | lo_2 | lo_1, v);
    }
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
        return start;
    }
}

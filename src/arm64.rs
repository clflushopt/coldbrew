//! Minimal ARM64 assembly module useful for doing ARM64 codegen.

/// Create a mask to extract n-bits of a given value from start.
pub fn mask(len: u64, start: u64) -> u64 {
    ((1 << len) - 1) << start
}

/// Split a u64 into two chunks of high and low bits.
pub fn split(x: u64) -> (u32, u32) {
    return ((x >> 16) as u32, (x & mask(16, 0)) as u32)
}

#[cfg(test)]
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
}

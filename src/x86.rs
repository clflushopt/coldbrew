//! Functions used for the x86_64 target.

/// Reads the current value of the CPU timestamp counter.
#[cfg(target = "x86_64")]
pub fn rdtsc() -> u64 {
    unsafe { std::arch::x86_64::_rdtsc() }
}

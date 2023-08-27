//! JIT compilation engine for coldrew.
use dynasmrt::aarch64::RX;
use dynasmrt::Register;

/// The `JitCache` is the core component of the compilation pipeline, given
/// a recorded trace it prepares and returns a native trace. Unlike traces
/// recorded by the `TraceRecoder` native traces are raw assembly opcodes
/// that can be executed by the `dynasmrt` runtime.
pub struct JitCache {}

impl JitCache {
    // Create a new JIT compilation cache.
    pub fn new() -> Self {
        JitCache {}
    }

    // Compile the trace given as argument and prepapre a native trace
    // for execution.
    fn compile() {}
}

# coldbrew

coldbrew is a tracing JIT compiler for the Java Virtual Machine, it currently
supports mainly primitive numeric types (`int`, `long`, `float`, `double`) and
serves as a demo project for how JIT compilers work in genenral.

## How it works

coldbrew bundles a traditional bytecode interpreter with a runtime for the JVM
as per the Java SE7 specification described in the link [below][1], during the
execution the bytecode is profiled, when an opcode jumps backwards (such as in
a loop) a trace is recorded. The trace contains all the information needed to
compile the bytecode to native assembly, such as the loop start, exit and body.

On further executions each time we encounter the loop entry we dispatch the JIT
to run the native code and capture its return value redirecting execution back
to the interpreter.

The following diagram gives an architecture guide to how coldbrew works :

[INSERT DIAGRAM HERE]

<p align=center><img src="assets/logo.png" alt="coldbrew logo" width="55%"></p>

## References

[1] [Java SE7 Spec](https://docs.oracle.com/javase/specs/jvms/se7/html/)

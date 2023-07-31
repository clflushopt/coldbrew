# coldbrew

`coldbrew` is a tracing JIT compiler for the Java Virtual Machine, it currently
supports mainly primitive numeric types (`int`, `long`, `float`, `double`) and
serves as a demo project for how JIT compilers work in genenral.

`coldbrew` is inspired by [TigerShrimp][2] and [Higgs][3]. I started the project
as a Rust port of [TigerShrimp C++ implementation][4] my goal was to understand
how tracing JITs work and how they are architectured but then it quickly become
obvious that porting C++ to Rust isn't feasible without introducing a few
architectural changes to ensure program safety.

I tried to remain as close as to the TigerShrimp implementation to have a sort
of baseline to test against, some architectural changes include breaking down
removing the object oriented approach in the original implementation to a more
flattened approach. In `coldbrew` there's no explicit object separation between
the interpreter, compiler and trace collection. Instead everything happens at
the same time everywhere at once.

## How it works

`coldbrew` bundles a traditional bytecode interpreter with a runtime for the JVM
as per the Java SE7 specification described in the link [below][1], during the
execution the bytecode is profiled, when an opcode jumps backwards (such as in
a loop) a trace is recorded. The trace contains all the information needed to
compile the bytecode to native assembly, such as the loop start, exit and body.

On further executions each time we encounter the loop entry we dispatch the JIT
to run the native code and capture its return value redirecting execution back
to the interpreter.

## References

[1] [Java SE7 Spec](https://docs.oracle.com/javase/specs/jvms/se7/html/)

[2] [TigerShrimp: An Understandable Tracing JIT
Compiler](https://odr.chalmers.se/server/api/core/bitstreams/87898837-623a-46f0-bcdc-06d2bf10805d/content)

[3] [Higgs: A New Tracing JIT for
JavaScript](https://pointersgonewild.com/2012/12/08/higgs-my-new-tracing-jit-for-javascript/)

[4] [Github/TigerShrimp](https://github.com/TigerShrimp/TracingJITCompiler)

## Acknowledgments

I would like to thank the authors of the TigerShrimp work and for providing
their implementation. The thesis is exellent overall and is clearly a must
read to anyone who wishes to understand the overall architecture and behavior
of tracing JIT compilers.


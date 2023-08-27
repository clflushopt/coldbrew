# coldbrew

`coldbrew` is a tracing JIT compiler for the Java Virtual Machine, it currently
supports mainly primitive numeric types (`int`, `long`, `float`, `double`) and
serves as a demo project for how JIT compilers work in genenral.

`coldbrew` is inspired by TigerShrimp[^2] and Higgs[^3]. I started the project
as a Rust port of TigerShrimp C++ implementation[^4] my goal was to understand
how tracing JITs work and how they are architectured, while I tried to remain
as close as TigerShrimp I had to change the implementation to be more Rust
friendly.

I tried to remain as close as to the TigerShrimp implementation to have a sort
of baseline to test against, unfortunately I didn't have much success building
TigerShrimp (probably due to the fact of the little patience I have these days
for building C++ code.)

## How it works

`coldbrew` bundles a traditional bytecode interpreter with a runtime for the JVM
as per the Java SE7 specification described in the link below[^1], during the
execution the bytecode is profiled and hotspots are recorded. The trace contains
all the information needed to compile the bytecode to native assembly, such as
the loop start, exit and body.

## Acknowledgments

I would like to thank the authors of the TigerShrimp work and for providing
their implementation. The thesis is an exellent introduction to Tracing JITs
and is a must read to anyone who wishes to understand the overall architecture
and details of how tracing JIT interpreters work.

[^1]: [Java SE7 Spec](https://docs.oracle.com/javase/specs/jvms/se7/html/)

[^2]: [TigerShrimp: An Understandable Tracing JIT
Compiler](https://odr.chalmers.se/server/api/core/bitstreams/87898837-623a-46f0-bcdc-06d2bf10805d/content)

[^3]: [Higgs: A New Tracing JIT for
JavaScript](https://pointersgonewild.com/2012/12/08/higgs-my-new-tracing-jit-for-javascript/)

[^4]: [Github/TigerShrimp](https://github.com/TigerShrimp/TracingJITCompiler)


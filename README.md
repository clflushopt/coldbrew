# coldbrew

`coldbrew` is a tracing JIT compiler for the Java Virtual Machine, it currently
supports mainly primitive numeric types (`int`, `long`, `float`, `double`) and
serves as a demo project for how JIT compilers work in genenral.

`coldbrew` is inspired by TigerShrimp[^1] and Higgs[^2], the TigerShrimp C++
implementation[^3] is very readable and was of huge help to debug some issues
along the line. Other implementations I've found useful is LuaJIT 2.0 and Mike
Pall's email about the LuaJIT internals which you can in the mailing list[^4].

While I tried to remain as close as the TigerShrimp implementation as possible,
there are some changes in the overall structure since we are using Rust.

Other notable changes is that we target ARM64[^5] primarly instead of x86.

I was originally planning to use the C++ implementation as a baseline to test
against but I didn't have much success building it.

## How it works

`coldbrew` bundles a traditional bytecode interpreter with a runtime for the JVM
as per the Java SE7 specification described in the link below[^6], during the
execution the bytecode is profiled and execution traces are recorded.

The trace contains all the information needed to compile the bytecode to native
such as the entry, exit codes and the bytecode of the core loop. Once a trace
is ready we pipeline it to the JIT cache for compilation and cachine, when we
reach that code path again, execution leaves the VM and executes the compiled
native trace before returning control to the VM.

## Acknowledgments

I would like to thank the authors of the TigerShrimp work and for providing
their implementation. The thesis is an exellent introduction to Tracing JITs
and is a must read to anyone who wishes to understand the overall architecture
and details of tracing JIT interpreters.


[^1]: [TigerShrimp: An Understandable Tracing JIT
Compiler](https://odr.chalmers.se/server/api/core/bitstreams/87898837-623a-46f0-bcdc-06d2bf10805d/content)

[^2]: [Higgs: A New Tracing JIT for
JavaScript](https://pointersgonewild.com/2012/12/08/higgs-my-new-tracing-jit-for-javascript/)

[^3]: [Github/TigerShrimp](https://github.com/TigerShrimp/TracingJITCompiler)

[^4]: [Archive: On LuaJIT 2.0](https://gist.github.com/jmpnz/fb8a1f2c9c0e70b4d2b0cc6cb5ddec25)

[^5]: [It's called arm64](https://lore.kernel.org/lkml/CA+55aFxL6uEre-c=JrhPfts=7BGmhb2Js1c2ZGkTH8F=+rEWDg@mail.gmail.com/)

[^6]: [Java SE7 Spec](https://docs.oracle.com/javase/specs/jvms/se7/html/)


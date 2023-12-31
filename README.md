# coldbrew

`coldbrew` is a tracing JIT compiler for the Java Virtual Machine, with
support for primitive numeric types (`int`, `long`, `float`, `double`) and
serves as a demo project for how JIT compilers work in genenral.

Currently `coldbrew` is able to successfully interpret, record, compile and
execute native code on x86-64 for some very simple demo programs e.g `support/jit`.

`coldbrew` is inspired primarly by TigerShrimp[^1] and some ideas from Higgs[^2]
the TigerShrimp C++ implementation[^3] is very readable and was of huge help. 

Other implementations I've found useful is LuaJIT 2.0 and Mike Pall's email
about the LuaJIT internals which you can in the mailing list[^4].

While I tried to remain as close as the TigerShrimp implementation as possible,
there are some changes such as (trace recording logic is different, we don't
support trace stitching and we want to maybe add support for inlining calls).

It's possible support for ARM64 will be added in the future.

I was originally planning to use the C++ implementation as a baseline to test
against but I didn't have much success building it.

## How it works

`coldbrew` bundles a traditional bytecode interpreter with a runtime for the JVM
as per the Java SE7 specification described in the link below[^6], during the
execution the bytecode is profiled and execution traces are recorded.

The recorded traces are self-contained with backwards branches and inner branches
only. Ideally we want to make this even more self-contained by recording just
basic blocks.

The trace contains all the information needed to compile the bytecode to native
such as the entry, exit codes and the bytecode of the core loop. 

Once a trace is ready we pipeline it to the JIT cache for compilation and when we
reach that code path again (loop entry) execution leaves the interpreter and executes
the compiled native trace.

When a compiled trace finishes executing we overwrite the interpreter current stack
frame to record all mutations that happened in native code then execution is returned
to the interpreter again.

### Trace recording and execution

To identify hotpaths we use heuristics that target *loop headers* which we can
identify when we encounter a *backwards branch* instruction. Once such branch
is identified we record the *program counter* where we branch as the start of
the loop.

But it's not sufficient to track *backwards branches* we need to calculate
their execution frequency to identify if they are *hot*, the invocation frequency
threshold (currently set to 1) triggewrs the start of recording.

An example of a trace would be a sequence of bytecode like the one below, the
format is `Inst(opcode, operands) @ PC`:

```asm

Inst(iload, Some([Int(2)])) @ Instruction Index 6 @ Method Index: 11
Inst(bipush, Some([Int(10)])) @ Instruction Index 7 @ Method Index: 11
Inst(if_icmpgt, Some([Int(13)])) @ Instruction Index 9 @ Method Index: 11
Inst(iload, Some([Int(1)])) @ Instruction Index 12 @ Method Index: 11
Inst(iload, Some([Int(2)])) @ Instruction Index 13 @ Method Index: 11
Inst(iadd, None) @ Instruction Index 14 @ Method Index: 11
Inst(istore, Some([Int(1)])) @ Instruction Index 15 @ Method Index: 11
Inst(iinc, Some([Int(2), Int(1)])) @ Instruction Index 16 @ Method Index: 11
Inst(goto, Some([Int(-13)])) @ Instruction Index 19 @ Method Index: 11

```

Ideally in a tracing JIT you might want to replace the comparison instruction by
speculatively executing under the assumption that the condition is true. This is
done in many production tracing JITs were guard clauses are introduced to assert
the condition.

In our case we don't do any of that we simply compile the code as is, the above
bytecode results in the following assembly (comments added for clarification). 


```asm

; epilogue
00000000  55                push rbp
00000001  4889E5            mov rbp,rsp
; this is not needed since we don't clobber rdi or rsi
00000004  48897DE8          mov [rbp-0x18],rdi
00000008  488975E0          mov [rbp-0x20],rsi
; loop entry
0000000C  488B842710000000  mov rax,[rdi+0x10]
00000014  4881F80A000000    cmp rax,0xa
; loop condition i <= 10
0000001B  0F8F29000000      jg near 0x4a
; loop code.
00000021  488B8C2708000000  mov rcx,[rdi+0x8]
00000029  4C8B842710000000  mov r8,[rdi+0x10]
00000031  4C01C1            add rcx,r8
00000034  48898C2708000000  mov [rdi+0x8],rcx
0000003C  4080842710000000  add byte [rdi+0x10],0x1
         -01
; go back to the loop entry (equivalent to goto)
00000045  E9C2FFFFFF        jmp 0xc
; 0xd is the program counter of the target instruction of if_icmpge above
; this is preloaded and known at compile time and we don't need to inject
; it.
0000004A  48C7C00D000000    mov rax,0xd
00000051  5D                pop rbp
00000052  C3                ret

```

The above is the relocation free version, emitted by [dynasm-rs](https://github.com/CensoredUsername/dynasm-rs).

*Note*: Special thanks to `dynasm-rs` author for an exellent and pleasent to use dynamic
assembler.

When it comes to executing the trace we assemble the native trace using `dynasm`
and record it as a pointer to a function with the following signature.

## Going Further

I might possibly keep working on this but if you would like a challenge
here are some ideas :

- Handle nested loops.
- Inline invoked functions (currently we abort traces that do function calls
  but under certain heuristics we can pretty much compile simple functions)
- Add an IR then compile and optimize the IR before compiling to assembly
  this offers you the opportunity for DCE, Algebraic Simplification, Constant
  Folding, Loop Unrolling (the list goes on really).
- Rewrite the tracer to build tracelets instead (basic blocks) then do trace
  splatting with branch flipping to really speed up things.
- Add support for trace stitching
- Add ARM64 support

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

[^7]: [HotpathVM: An Effective JIT Compiler for Resource-constrained Devices](https://www.usenix.org/legacy/events/vee06/full_papers/p144-gal.pdf)


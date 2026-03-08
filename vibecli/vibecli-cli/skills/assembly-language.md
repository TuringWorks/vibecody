---
triggers: ["assembly language", "assembly", "ASM", "x86 assembly", "ARM assembly", "RISC-V assembly", "NASM", "MASM", "GAS", "inline assembly", "machine code"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["nasm"]
category: assembly
---

# Assembly Language

When writing assembly code (x86-64, ARM, RISC-V):

1. Choose the right assembler: NASM (Intel syntax, portable) for x86-64 on Linux/macOS; MASM for Windows x86-64; GAS (GNU Assembler, AT&T syntax) for GCC integration; `as` for ARM/RISC-V — Intel syntax: `mov rax, rbx`; AT&T syntax: `movq %rbx, %rax`.
2. Understand calling conventions: x86-64 System V (Linux/macOS): args in `rdi, rsi, rdx, rcx, r8, r9`, return in `rax`, callee-saves `rbx, rbp, r12-r15`; Windows x64: args in `rcx, rdx, r8, r9` — violating conventions corrupts the call stack.
3. Use the stack correctly: `push rbp; mov rbp, rsp; sub rsp, 32` for frame setup; align stack to 16 bytes before `call` (required by System V ABI); `leave; ret` for cleanup — stack misalignment causes segfaults in SSE/AVX instructions.
4. For x86-64 SIMD: use SSE/AVX for data parallelism — `movaps xmm0, [data]; addps xmm0, xmm1; movaps [result], xmm0` processes 4 floats simultaneously; AVX-512 processes 16 floats; align data to 16/32/64-byte boundaries.
5. For ARM (AArch64): registers `x0-x7` for args/return, `x8` for indirect result, `x9-x15` temp, `x19-x28` callee-saved; use `ldr/str` for memory, `add/sub/mul` for arithmetic; conditional execution via flags (`b.eq`, `b.ne`, `b.gt`).
6. For RISC-V: registers `a0-a7` for args, `s0-s11` callee-saved, `t0-t6` temp; `la t0, label` for address loading; `lw/sw` for 32-bit load/store; `ecall` for system calls — RISC-V has a clean, minimal ISA ideal for learning.
7. Use inline assembly in C/Rust sparingly: GCC `asm volatile("cpuid" : "=a"(eax) : "a"(leaf) : "ebx","ecx","edx");` — specify inputs, outputs, and clobbers correctly; incorrect clobber lists cause silent corruption.
8. Optimize memory access: keep hot data in cache lines (64 bytes on x86); avoid false sharing in multi-threaded code; use prefetch (`prefetcht0 [addr+64]`) for sequential access patterns; align structures to cache line boundaries.
9. Debug with GDB: `layout asm` for disassembly view; `info registers` to inspect registers; `x/16xg $rsp` to examine stack; `stepi`/`nexti` for single instruction stepping; `display/i $pc` to auto-show current instruction.
10. System calls on Linux x86-64: syscall number in `rax`, args in `rdi, rsi, rdx, r10, r8, r9`, invoke with `syscall` instruction — e.g., write: `mov rax, 1; mov rdi, 1; lea rsi, [msg]; mov rdx, len; syscall`.
11. Use NASM macros for repeated patterns: `%macro pushall 0; push rax; push rbx; ...; %endmacro` — organize code with sections `.text` (code), `.data` (initialized data), `.bss` (uninitialized data), `.rodata` (constants).
12. Profile with `perf stat ./program` and `perf record ./program; perf annotate` for instruction-level profiling — look for cache misses, branch mispredictions, and pipeline stalls; optimize the innermost loops first.

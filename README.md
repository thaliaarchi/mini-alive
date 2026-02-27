# Mini-Alive

A minimal re-implementation of [Alive2](https://alive2.llvm.org/ce/), intended
as a test bed for research in improving it. Alive2 is a translation validator
for LLVM and its de facto formal semantics.

Mini-Alive supports a subset of LLVM IR:
- Values: `i16`, `ptr`, struct (inductive), array (inductive)
- Second-class values: `i1`
- Instructions: arithmetic, `extractvalue`/`insertvalue`, `alloca`,
  `load`/`store`, `icmp`, `phi`, `call`, `br`, `ret`
- Intrinsics: `@malloc`
- Control flow: sequence, branch, call

To simplify its memory model, the only memory access granularity in Mini-Alive
is 16-bit words. This eliminates large SMT case splits for alignment and
provenance split between accesses. You can think of this as the memory model of
B on a PDP-11 (and the project's name is an oblique reference to Mini-UNIX).

Mini-Alive emits SMT solver queries via an SMT IR, which can lower to Z3.

License: MPL-2.0

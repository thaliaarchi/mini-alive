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

Mini-Alive has its own IR, separate from LLVM's or Alive's, so I am free to
experiment with representations to suit research needs, instead of sticking to
existing design decisions. For compatibility, I match LLVM's parsing behavior,
even when strange.

Mini-Alive emits SMT solver queries via an SMT IR, which can lower to SMT-LIB2
text or Z3.

## Demo

At the time of this demo, the parser, parse diagnostics, IR, SMT IR, Z3
lowering, and SMT-LIB lowering are complete.

Examples and tests:
- [LLVM IR parsing and pretty-printing](https://github.com/thaliaarchi/mini-alive/tree/main/demo)
- [LLVM IR parse diagnostics](https://github.com/thaliaarchi/mini-alive/blob/main/src/syntax/tests.rs#L302)
- [SMT IR lowering to SMT-LIB](https://github.com/thaliaarchi/mini-alive/blob/main/src/smt/smtlib/lower.rs#L149)
- [SMT-LIB pretty-printing](https://github.com/thaliaarchi/mini-alive/blob/main/src/smt/smtlib/pretty.rs#L189)

Run the tests with `cargo test`.

Run the parser / pretty-printer with `cargo run FILENAME`.

Install Rust with [rustup](https://rustup.rs).

## License

License: MPL-2.0

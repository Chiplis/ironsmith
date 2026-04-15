# `ironsmith-compiler` Migration Checklist

This is the blocker list for finishing the compiler extraction. The concrete
legacy backend now lives physically under
`crates/ironsmith-compiler/src/runtime_backend`, with runtime loading it
through a path bridge while the remaining type coupling is removed.

## Checklist

- [x] Move compiler-owned diagnostics and source spans into `ironsmith-compiler`.
- [x] Move compiler-owned lexer/token view infrastructure into `ironsmith-compiler`.
- [x] Move sentence splitting and followup-intro detection into `ironsmith-compiler`.
- [x] Move token rewrite/query helpers into `ironsmith-compiler`.
- [x] Move parse/preprocess source model types into `ironsmith-compiler`.
- [x] Move metadata parsing and preprocess helpers into `ironsmith-compiler`.
- [x] Move pure CST line-shape structs/enums into `ironsmith-compiler`.
- [x] Move AST-bearing CST/document parser structures into `ironsmith-compiler`.
- [x] Move parse-only semantic helper types into `ironsmith-compiler`.
- [x] Move semantic/IR model types into `ironsmith-compiler`.
- [x] Move reference-model and parse-only context types into `ironsmith-compiler`.
- [x] Move lowering, effect-pipeline, and postpasses into `ironsmith-compiler`.
- [x] Replace runtime-owned `CardDefinitionBuilder` parse entrypoints with compiler-owned entrypoints.
- [x] Rewrite registry/tools/cli callers to use compiler-owned APIs directly.
- [x] Delete the runtime-owned compiler tree once no callers remain.

## Current Hard Blockers

- The relocated backend still compiles against runtime-owned domain payload
  types through `crate::...` imports when it is built inside runtime.
- The relocated backend still needs those runtime-only domain structures
  extracted or bridged before it can compile natively inside
  `ironsmith-compiler`.
- `CardDefinitionBuilder` in `crates/ironsmith-runtime/src/cards/builders.rs`
  is still the public runtime wrapper, but its parse methods now delegate to
  compiler-owned canonical entrypoints rather than owning parse logic directly.
- The remaining runtime-backed parse calls in adapters now live behind intentional
  `CompilerBackend` bridge implementations rather than direct ad hoc builder
  calls, but the public parse entrypoint is still runtime-owned.

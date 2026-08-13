# Parser final ownership

The compiler crate owns the complete oracle-text pipeline. Its tracked source is divided into three physical ownership roots:

- `front_end/` owns lossless document preparation, grammar, structured recognition outcomes, and semantic-document construction.
- `model/` owns compiler facts, provenance, typed symbols, references, control flow, the canonical compiler AST, and shared traversal.
- `lowering/` owns the single materialization boundary from validated compiler AST nodes to runtime objects.

Runtime crates may call the compiler facade and consume lowered output. They do not recognize oracle text. The former `runtime_backend` namespace, its compatibility re-exports, thread-local source context, unbounded parse cache, and parser stack-growth guards are not tracked ownership boundaries.

The machine-readable authority is `architecture/parser-ownership-manifest.json`. Its ordered phase edges are the complete dependency allowlist, and its bridge, exception, and legacy-path arrays are empty after PR-33. The parser architecture, manual-parser, and module-size audits enumerate Git-tracked production sources so untracked local work and test-only modules cannot alter an architecture result.

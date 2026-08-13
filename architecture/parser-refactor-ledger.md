# Parser refactor migration ledger

- Specification: `SPEC.md`
- Starting checkout: `40e392d58b73b229fbf9b9df071299e99b202979` (`main`, tracking `origin/main`)
- Baseline captured: 2026-08-13 from the pre-existing working tree, which already contained unrelated user changes
- Validation policy: PR-01 through PR-33 are source-only implementation checkpoints; compiled validation, test edits, audits, corpus synchronization, generated artifacts, and WASM work are deferred to PR-34
- Ownership manifest: `architecture/parser-ownership-manifest.json`

## Ordered checkpoints

| PR | Commit | Production files | New bridge | Removed bridge | Static evidence | Deferred validation risk |
| --- | --- | --- | --- | --- | --- | --- |
| PR-01 — Ledger and architecture manifest | pending | `architecture/parser-refactor-ledger.md`; `architecture/parser-ownership-manifest.json`; `crates/ironsmith-tools/src/bin/audit_parser_architecture.rs`; `crates/ironsmith-tools/Cargo.toml` | Manifest registers `BRIDGE-PARSE-CONTEXT-TLS`, `BRIDGE-PARSE-OUTCOME-OPTION`, `BRIDGE-PARSE-OUTCOME-RESULT`, `BRIDGE-LEGACY-REGISTRY`, `BRIDGE-TAGKEY-SYMBOL`, `BRIDGE-CANONICAL-MODEL-REEXPORT`, and `BRIDGE-RUNTIME-PARSER-OUTPUT` | none | Baseline source counts and read-only corpus metrics recorded below; every manifest exception has an expiry PR | New audit source is intentionally uncompiled until PR-34; schema or Rust errors may surface then |

## Baseline smells

Counts exclude standalone `tests.rs`, `*_tests.rs`, and `tests/` paths unless a row states otherwise. Inline `#[cfg(test)]` modules remain part of simple source scans. The exact audit binaries were not executed because the implementation-checkpoint contract forbids compiling or running Rust; the manual-parser row is therefore a source-pattern proxy and the module-size count is a read-only reproduction of the checked-in budget table.

| Smell | Baseline | Source-only measurement |
| --- | ---: | --- |
| Parser/compiler backend production Rust | 434,838 LOC | `rg --files crates/ironsmith-compiler/src/runtime_backend -g '*.rs'` filtered as above, then `wc -l` |
| Typed grammar and parser implementation | 189,117 LOC | Same LOC scan under `runtime_backend/front_end/grammar` |
| Legacy sentence parsing | 98,454 LOC | Same LOC scan under `runtime_backend/sentences` |
| Family parsers | 46,245 LOC | Same LOC scan under `runtime_backend/families` |
| Lowering | 48,497 LOC | Same LOC scan under `runtime_backend/lowering` |
| Reference implementation | 9,924 LOC | Same LOC scan under `runtime_backend/references` |
| Semantic line parsing | 10,100 LOC | Same LOC scan under `runtime_backend/front_end/semantic_line_parsing` |
| `line_lowering.rs` | 11,616 LOC | `wc -l runtime_backend/lowering/lower/line_lowering.rs` |
| Post-lowering repair functions | 78 | Function names in `line_lowering.rs` beginning `bind_`, `reconcile_`, `preserve_`, `normalize_`, `rewrite_`, `restore_`, `transport_`, `dedupe_`, `attach_`, or `fuse_` |
| Raw/source-token references in line lowering | 300 | Whole-word occurrences of `raw`, `raw_text`, `source`, `source_text`, `token(s)`, and `word(s)` |
| `.ok()` conversions in parser production files | 1,301 | Literal `.ok()` occurrences under `runtime_backend`, excluding standalone test paths |
| Option-returning grammar signatures | 2,574 | Same-line `-> Option<` signatures under `runtime_backend/front_end/grammar`, excluding standalone test paths |
| Bundle/sequence recipe registrations | 179 | 146 `SequenceRuleDef` registrations plus 33 named bundle parser functions |
| Literal `TagKey::from(...)` uses | 311 uses / 101 strings | Literal-string construction under `runtime_backend`, excluding standalone test paths |
| Manual-parser source-pattern candidates | 980 lines | Source lines matching the checked-in audit's phrase/scan/raw-string families; PR-34 records the authoritative audit finding count |
| Parser module-size budget failures | 166 | Read-only comparison of current file lengths with `audit_parser_module_sizes.rs` budgets |

## Corpus/status baseline

Read from `reports/engine-status.sqlite3` with SQLite read-only mode; no compiler-backed status tool or synchronization ran.

| Metric | Baseline |
| --- | ---: |
| Latest cards | 31,922 |
| `parse_failed` | 3,762 |
| `strict_compiled` | 28,160 |
| Cards with unimplemented output | 24 |
| Global average semantic similarity | 0.881079993950201 |

## PR-34 regression log

No entries yet. PR-34 owns compilation/test/audit/corpus failures and closes each entry before completion.

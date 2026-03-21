# Rewrite Runtime Dependency Status

The rewrite runtime is now independent of the old copied `ported_*` module
surface. The stable runtime parser/lowering stack is:

- [effect_pipeline.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/effect_pipeline.rs)
- [parse.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/parse.rs)
- [lower.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/lower.rs)
- [effect_sentences](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/effect_sentences)
- [activation_and_restrictions.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/activation_and_restrictions.rs)
- [keyword_static](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/keyword_static)
- [object_filters.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/object_filters.rs)

## Runtime Boundary Checks

- [x] Runtime entrypoints use rewrite parsing/lowering.
- [x] Rewrite owns preprocessing, lexing, CST construction, semantic IR, and lowering.
- [x] Runtime parser/lowering entrypoints and helpers do not import `ported_*`.
- [x] `legacy_helpers` is gone from runtime code.
- [x] Lowering does not reparse semantic line text after rewrite IR creation.
- [x] Stable module names replace the former `ported_*` runtime surface.

## Compatibility Notes

- The compatibility `Token` view in
  [util.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/util.rs)
  intentionally preserves historical token semantics for the stable supported
  slice, but it is produced from rewrite `logos` tokens rather than from the
  deleted legacy tokenizer.
- Unsupported diagnostics are expected runtime behavior and remain explicit.
- Verification lives in `cargo test --lib -q`, generated-registry checks, and
  corpus/tooling audits.

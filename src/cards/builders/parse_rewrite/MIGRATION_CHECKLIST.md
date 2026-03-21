# `winnow` + `logos` Rewrite Status

## Landed Runtime Contract

- Rewrite parsing/lowering is the runtime path behind `parse_text(...)` and
  `parse_text_allow_unsupported(...)`.
- Lexing is rewrite-owned in
  [lexer.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/lexer.rs)
  via `logos`.
- Rewrite leaf parsing is rewrite-owned in
  [leaf.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/leaf.rs)
  via `winnow`.
- Sentence, trigger, static, keyword, restriction, and filter parsing now live
  under stable rewrite-owned modules:
  [effect_sentences](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/effect_sentences),
  [activation_and_restrictions.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/activation_and_restrictions.rs),
  [keyword_static](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/keyword_static),
  and [object_filters.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/object_filters.rs).
- Runtime parser/lowering entrypoints no longer import `ported_*` modules or
  `legacy_helpers`.
- Rewrite semantic IR carries parsed runtime payloads for keywords,
  activations, triggers, statics, statements, modal modes, level items, and
  saga chapters.
- Lowering consumes those parsed payloads directly and does not reparse
  semantic line text after IR creation.
- Unsupported-card behavior remains first-class: strict parsing still errors,
  `allow_unsupported` still threads explicit unsupported markers/diagnostics.

## Notes

- The compatibility `Token` view in
  [util.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/util.rs)
  is now derived from the rewrite `logos` lexer so the runtime lexer boundary is
  still rewrite-owned.
- Supported-card behavior is verified by the library test suite and corpus/tool
  audits rather than by preserving the deleted `ported_*` module tree.

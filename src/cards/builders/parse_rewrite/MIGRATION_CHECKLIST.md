# `winnow` + `logos` Migration Checklist

## Completed
- [x] Add `logos` and `winnow` dependencies.
- [x] Create a parallel `parse_rewrite` module tree.
- [x] Introduce rewrite preprocessing, CST, semantic IR, and an internal rewrite entrypoint.
- [x] Add a `logos` lexer with span-carrying rewrite tokens.
- [x] Implement first `winnow` leaf parsers for mana symbols/costs, count words, type lines, and a subset of activation costs.
- [x] Add rewrite-focused unit tests and differential checks against legacy parsers for the stable leaf slices.
- [x] Preserve metadata lines as first-class rewrite items through preprocessing, CST, and semantic IR.
- [x] Add stronger document-level differential tests for rewrite-vs-legacy behavior on the currently supported slice.
- [x] Expand activation-cost leaf coverage to include untap, energy, mill, exile-self / exile-from-hand, and source counter add/remove costs.
- [x] Recognize modal bullet blocks, level headers, and saga chapter prefixes as first-class rewrite structure.
- [x] Parse document structure beyond ordinary lines: bullets, modal blocks, level headers, and saga chapter prefixes.
- [x] Port triggered-line parsing into rewrite CST/IR.
- [x] Port static-line parsing into rewrite CST/IR.
- [x] Port statement/effect-line parsing into rewrite CST/IR.
- [x] Add a rewrite-to-legacy-AST lowering bridge for currently supported line families.
- [x] Keep rewrite-native activated lowering for inline-restriction lines by attaching `ParsedRestrictions` instead of falling back to whole-line legacy parsing.
- [x] Add a controlled cutover seam in `effect_pipeline::parse_text_with_annotations`.

## Remaining
- [ ] Continue expanding leaf grammars toward near-full activation-cost coverage.
- [x] Build rewrite-native unsupported-reason classification instead of the current generic placeholder.
- [ ] Rebuild semantic lowering from rewrite IR into the existing runtime card model.
- [ ] Rebuild reference resolution/import-export threading on top of rewrite IR.
- [x] Add document-level differential tests comparing legacy and rewrite parse classes/annotations.
- [x] Add corpus-driven audit wiring so report binaries can compare rewrite and legacy behavior.
- [ ] Remove legacy parser internals after rewrite parity is proven.

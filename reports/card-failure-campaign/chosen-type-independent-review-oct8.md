# Independent source re-review

Reviewed exact final anchor `0e11a69886f93516d16b5204175ce2cd7d95df67`, correction atop `33ce2773f1cdebeabeb406b33c0544bd3f387a68`, baseline `1dd81cd84c62f272479f26e16d74719fff24b97b`.

## Disposition

Previously reported independent-arm scope blocker is closed at source level. No further material production or test/API blocker identified in this bounded review. This is not execution clearance.

`complete_characteristic_extended_subject` now returns constructed filters. Its union path recursively consumes both complete nominal arms and stores them unchanged under an unrestricted outer `any_of`, recording only the authored connective. It no longer delegates independently scoped arms to broad-reader modifier propagation. Nested nonbattlefield selectors remain nested rather than unsafely flattened. The single nonbattlefield suffix path remains separately bounded and uses the established seven-zone expansion only after complete nominal and explicit-zone checks.

New grammar assertions independently inspect branch fields for both orders of nontoken, tapped, and other versus Slivers, and both orders of battlefield-control/nonbattlefield-ownership domains. They no longer compare the result against the problematic broad parser. Both-arm invalid suffix/unknown-word/quote/dangling-conjunction tests are authored at grammar and whole-body admission boundaries. Direct and artifact-materialized runtime witnesses cover both nontoken/tapped orders and cross-zone ownership/control orders. Canonical-text reparse now checks actual chosen-type filter equality. The added core ObjectFilter export and GameState::tap API uses match source.

Prior fixture verification remains valid: all three full Oracle bodies match the completed refresh; fixture mana costs/type lines and Rukarumel's 3/3 match its frozen cards-current.json. Runtime addition continues to use AddSubtypes. The exact actual-metadata whole-body fixtures qualify for authored source coverage, not synthetic-only credit.

## Remaining limits

- All builds, tests, compiler probes, corpus sweeps, and codegen remain UNRUN; no source files were edited during review.
- Leyline whole-body zero-loss depends on separately integrating and validating speculative-probe patch `39ad55d76d741a560d732de07536a11a74d5faa0`.
- Runtime witnesses still omit Ante, although grammar checks the seven-domain selector; choice is set before observing the grant rather than changed between established grants.
- `other` is independently checked in grammar, not with a runtime self-exclusion witness. Untapped-specific, longer mixed-connective, and arbitrary generalized grammar claims are outside this bounded clearance.
- No measured support improvement, successful artifact regeneration, or complete gameplay validation is established.

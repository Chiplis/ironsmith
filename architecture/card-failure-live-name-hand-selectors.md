# Live name comparison sets: implementation-first, UNVALIDATED

Frozen identities (stack07):

- Key to the Side-Door: `76c036df-44f5-4021-b7fc-656147351341`
- Hint of Insanity: `4db59824-6bcb-4707-977b-0ff699d8662b`

Together with Jandor's Ring (`c586dabe`) and Circu/Godsend (`cec1e069`), this is the approved five-card name/history cohort. No other same-name, copying, land-play, morph or history family is claimed.

## Representation and execution

The existing `ObjectCharacteristicRelation` receives an appended `Name` characteristic, plus a serde-default false `exclude_candidate` field. Old inclusive comparison sets retain their meaning. "Another" excludes the compared object's exact incarnation, not the ability source. The ordinary recursive visitors already walk `comparison`; the additional latest-drawn player field is also included in core iterated-player detection and contextual resolution.

The outer Key selector is a legendary hand card. Its comparison is a live legendary battlefield permanent controlled by the payer, regardless of that permanent's owner. Existing cost preflight and actual discard payment both evaluate the complete filter; pending/disclosed payment continues through the reviewed transaction machinery. No prospective hand identities or unconditional payment permissions are added.

Hint's outer selector retains the nonland restriction, while its comparison includes every *other* card in that candidate owner's hand, including lands. This requires reading the complete card phrase before the old discard qualifier/tail split. The count and card filter are lowered as the same complete set, with the affected player's bound reference. Existing discard execution freezes this set before committing any moves, so removing the first duplicate does not save the second. Prior reveal-hand execution stays intact.

Runtime comparisons use the existing shared-name operations, including split/multiple names and nameless exclusion. The name characteristic is covered by layer dependencies, the cost-reduction intersection visitor, and prior-result shared-characteristic evaluation. Compact prior-result memory retains both split names when no live object remains. A three-object pairwise-only overlap is not mistaken for a common name across all three objects.

The grammar accepts explicit live comparison phrases and leaves source-linked exile references to their existing reader. Recognized hand comparisons with unknown trailing tokens are errors. The now-implemented Hint phrase is removed from the unsupported tables only after receiving a typed parse/lower path.

## Authored checks (not executed)

- `cargo test -p ironsmith-compiler-runtime --test live_name_hand_selectors -- --nocapture`
- `cargo test -p ironsmith-compiler-runtime --test latest_drawn_hand_cost -- --nocapture`
- Grammar unit tests in `grammar/filters/live_name_relations.rs` cover both facades, independent qualifier scopes, exact candidate exclusion, unknown tails and linked-exile routing.
- Engine unit test `result_name_relation_requires_one_common_name_and_retains_split_names` covers compact split-name memories and nameless/common-set semantics.

The public integration target uses the exact full printed payloads both directly and after artifact JSON round-trip. It exercises both Key abilities, true mana/discard payment and stack resolution, donor theft/departure, legendary/nameless negatives, alternate split names, and Hint's complete multiplayer duplicate-discard set. Older relation JSON defaults to inclusive semantics.

Only source review, rustfmt parsing, and diff whitespace checks were performed. No compilation, runtime tests, replay, or measured coverage claim accompanies this change.

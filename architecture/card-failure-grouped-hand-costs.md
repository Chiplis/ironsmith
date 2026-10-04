# Group-constrained hand payments

Status: **UNVALIDATED, SOURCE REVIEW ADMITTED**. These are source-only proposals; no verified correctness claim. No compilation or test execution; source review and authored regressions only.

The three frozen stack07 identities are source-proposed after coordinated review: Illuminated Folio, Sphinx of the Chimes, and Ormos, Archive Keeper (`fixtures/grouped_hand_costs.json.fixture`). Key to the Side-Door and Jandor's Ring remain separate reference/history blockers.

## Shared typed mechanism

Two default-false fields extend the existing ObjectFilter selection-set metadata: `shares_name` and `shares_color`. Like the existing `distinct_names` and `shares_land_type`, these describe the selected set, never membership of an individual candidate. Older JSON payloads default them off; no enum ordinals change. Descriptions, hidden-identity classification, and characteristic dependency inspection retain the new metadata.

An exact grouped hand cost lowers to an ordinary ChooseObjectsEffect plus tagged discard or reveal. The selection is payer-owned Hand with the complete printed object filter, including Sphinx's nonland predicate. A shared relational validator checks a common name/color or pairwise distinct names. Its subset finder supports preflight and default deciders without mutating state, choosing card names, or capping a search by an arbitrary heuristic.

Native decision contexts carry the relation. The drafted actual cost selection uses the existing public selected-card reveal policy so peer replay sees the chosen identities before validating their joint relationship. The transaction follow-up retains the exact disclosure across failed commands and recovery, and validates submitted groups before publication; see the reviewed boundary below. Names account for split-card multiple names and nameless objects; colors require a nonempty intersection, not merely pairwise overlap. Default deciders find valid groups rather than assuming the first N candidates qualify.

Representative tags use a feasible group. Tagged discard/reveal payment requires the currently selected object incarnation, preserving stable-ID historical facts elsewhere. Missing tags, cards that changed zones, insufficient groups and reserved simultaneous-entry objects fail closed. Existing whole-cost cancellation checkpoints remain in use, but they cannot undo information already delivered to opponents.

## Ormos body

A separate grammar fact admits “If you would draw a card while your library has no cards in it, instead <effect>.” It lowers through the existing ConditionalDrawReplacement payload and effect parser with a CardsInLibrary(You)==0 condition. The existing empty-library win/skip and no-cards-in-hand draw readers are preserved. The Phial marker guard (`af8574ba`, integrated centrally) remains a companion compatibility dependency: exactly one leading OR trailing `instead` there; the new empty-library body form requires its single leading marker.

## Authored evidence

- Exact strict/artifact transport and old-payload default checks.
- Folio ownership, common color, colorless exclusion, reveal-without-discard, real mana/tap payment and draw.
- Sphinx nonland restriction, same-name selection, foreign-card exclusion and real discard/draw.
- Ormos three different names, owner/controller separation, partial actual draws followed by individual empty-library replacements.
- Native default choices, cancellation, nonmutating unpayability, common-versus-pairwise relation negatives, split names, nameless objects, and leave/return incarnation negatives.
- Grammar full-consumption and replacement marker controls.

Deferred command: `cargo test -p ironsmith-compiler-runtime --test grouped_hand_costs -- --nocapture`.


## Reviewed peer-disclosure boundary

The historical hold correctly identified information that GameState rollback cannot retract, but its initial claim about a later mana-cancel prompt was too broad. Printed mana is prepared before these nonmana costs. The proven common gaps were completed-action Undo and legitimate interrupted/error/recovery paths spanning more than one command. See `architecture/card-failure-payment-disclosure-boundary.md` for that corrected source trace.

The transaction implementation in `architecture/card-failure-payment-disclosure-transactions.md` now routes the group Public selection through the same native commitment, durable signed-attempt journal, and replay/resync recovery as the earlier four discard and five SNC cards. Group relation and complete individual filters are checked on the selected opened identities before the native metadata query can authorize publication or latch a retry. An invalid color/name group is rejected while leaving the cost prompt and a later valid group available. Unknown peer placeholders remain availability-only; actual cost execution requires opened identities.

The group choice still has Public proof semantics. No proof checks, cost filters, actual movements, or transaction guards are suppressed. Names/colors remain typed filter metadata, and existing dynamic quantity and per-player filter code is unchanged. Ormos's empty-library effect-body reader remains separate from the existing Phial leading/trailing marker reader.

Coordinated source review admitted the complete transaction and group follow-up in stage40. All regressions remain authored and unrun; full runtime and regression validation is mandatory.

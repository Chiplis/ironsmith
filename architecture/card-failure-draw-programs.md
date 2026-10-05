# Owned draw-replacement programs and hand visibility

UNVALIDATED source work. Seven proposed complete identities: Tomorrow, Azami's
Familiar; Obstinate Familiar; Possessed Portal; Forbidden Crypt; Out of the
Tombs; Chains of Mephistopheles; and Enduring Renewal. Exact complete frozen
metadata, Oracle text and IDs are in `fixtures/draw_replacement_programs.json.fixture`.
No build, compilation, test, formatter or replay was executed. The only executed
code check is `git diff --check`. Measured recovery remains 40, with 3,193
unresolved unique cards; these source proposals are not verified recoveries.

## Program ownership, scope and matching

A named token grammar owns the draw header, its optional first-draw-step or
empty-library gate, the optional replacement choice, and one outer `instead`
marker. That marker can precede the program or end its first sentence. Following
sentences stay inside the same replacement program, so look/selection/remainder,
return/failure, discard/draw/mill and reveal/conditional-draw references are not
split into unrelated statements. Existing specialized draw-count, redirect,
empty-library skip and reveal/search grammars retain ownership when they match.
The complete source label remains available for presentation.

Both generic and conditional replacement bodies lower through one shared
reference/result frame. The former conditional payload's generic child mapper
created a fresh context for every effect; that lost the previous action needed
by `If you can't` and other continuations. The frame explicitly binds the
affected drawer as its iterated player, separately from the replacement host's
controller (`you`). Existing native replacement execution already supplies that
binding and carries the selected event's application history into nested draws.
Thus a nested draw cannot reapply the same occurrence indefinitely, while
independent replacement sources still participate in ordinary affected-player
ordering.

Skipping *this* draw is an empty replacement program that consumes only the
current proposal. It does not schedule a future skipped draw or draw step. An
optional cancellation is a real optional replacement/decline choice before
executing the program, with the existing atomic pending-choice rollback.
Unsupported optional choosers and combined gates remain rejected rather than
becoming an always-replaced draw containing an optional inner effect.

Out of the Tombs's empty-library predicate is checked independently for each
draw. The official [Warhammer 40,000 release notes](https://magic.wizards.com/en/news/feature/warhammer-40000-commander-release-notes-2022-09-19)
confirm the per-card behavior when an effect draws multiple cards. The retained
first-of-draw-step matcher for Chains uses the engine's draw-step event flag,
not a once-per-turn approximation. Result continuations retain the engine's
actual/replaced action receipts; a replaced return is not silently treated as
an impossible instruction merely because its original destination changed.

## Real public-hand visibility

Enduring Renewal's first line now has a native self-hand visibility ID; the
corresponding all-player form also has a native ID. Both are appended for
ordinal compatibility and mapped through compiled-model materialization. They
are enforced in the live sync visibility views and incremental UI snapshot
cache, including control changes, phase-out and departure. They reveal hands,
not libraries. The existing opponent-only form now uses actual opponent/team
relationships in both paths, rather than all players with a different ID.
This also supplies a real visibility primitive for Zur's Weirding, but does not
claim that card's separate multiplayer payment/replacement body.

## Deferred checks and partials

Three grammar scenarios, eleven full direct/restored-artifact runtime scenarios
and one cached/sync visibility matrix are authored and unrun. Coverage includes
complete frozen bodies, actual draw events versus non-draw card movement,
optional decline/replay, no scheduled future skip, return success/failure,
wrong-player controls, Chains's original discard result across both branches,
separate draw steps, Enduring Renewal's graveyard trigger, the Tombs upkeep
counter/mill sequence, Portal's end-step per-player alternative, and team-aware
visibility through source lifetime changes.

Alms Collector needs a whole-instruction draw replacement rather than this
per-card root. Ormos retains its distinct-name discard-cost gap. Uba Mask and
Shared Fate still require explicit replacement-card/exile-link and permission
ownership, Parallel Thoughts needs its ordered private pile, and Unpredictable
Cyclone needs captured cycling-ability identity. Island Sanctuary has its own
draw-step gate plus temporary attack restriction. They are not counted here.
The earlier token-family silent 500-token truncation remains a mandatory final
correctness gap tracked separately; nothing in this draw batch closes it.

### Cryptographic opening path (source-reviewed, unrun)

The visibility IDs feed `hand_revealed_by_static_ability` in WASM `lib.rs`, then
`append_static_visibility_views`, which includes every current hand object and
stable ID in a `public: true` view. `update_crypto_requirements_from` emits both
`public_view_window` and commitment-bound `public_open` requirements from those
views. The normal snapshot path runs this audit after a captured action, so
entry, control transfer, phasing and departure re-evaluate the current scope.

`web/ui/src/hooks/peer-lobby/audit-material.js` builds/collects these requirements'
openings and `verifyAuditSatisfiesCryptoRequirements` rejects missing public
openings. `revealAuditOpenings` calls `verifyAuditOpeningsAgainstManifests` before
installing identities; that verifies the deck-manifest opening and applicable
Ziffle cryptographic proof. It then uses the existing commitment/position-aware
`revealHiddenObject`, `revealHiddenSlot` or `revealHiddenPosition` routes. WASM
validates the physical binding, hydrates/rebinds with
`reveal_hidden_card_with_definition`, and updates any live continuation
checkpoint before preserving/recomputing the decision. A repeated known-card
opening must preserve current characteristics and identity; stopping public
visibility does not make already disclosed information unknown again.

One additional authored, unrun WASM scenario asserts actual public window/opening
requirements for self/global scopes across all source transitions, excludes a
hidden library card, and checks known-card rehydration retains the commitment,
stable identity and current state. This extends the local view matrix; it does
not claim a newly executed peer/cryptographic validation run.

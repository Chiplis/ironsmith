# Payment disclosure and Undo: required runtime correction

Status: **SOURCE-PROVEN GAP, UNVALIDATED REMEDIATION**. No compilation or tests have run. This is defensive correctness within the card campaign, not an unrelated protocol audit.

Exact frozen identities and printed programs are in `fixtures/payment_disclosure_partials.json.fixture`.

## Affected identities

Earlier source-complete proposals requiring partial status pending correction:

- Knollspine Invocation
- Krovikan Sorcerer
- Sanctum Spirit
- Kozilek, the Great Distortion (joint cost and draw-difference work)

Three additional drafts remain held and partial:

- Illuminated Folio
- Sphinx of the Chimes
- Ormos, Archive Keeper

The grouped implementation `0b3738e6` and its hold note `088a38b4` remain isolated. Do not integrate them as complete-card coverage.

## Corrected ordinary-flow trace

It is **not** correct to attribute all seven to a later printed mana prompt. Knollspine's activation first prepares/Confirms mana; its X-relative discard then normally finishes, commits the prepared mana, and finalizes in the same command. Krovikan pays tap plus discard; Sanctum and Kozilek pay only a discard. Their normal printed costs have no subsequent interactive mana prompt.

The demonstrated route is completed-action **Undo before the ability resolves**, using a normal game with unchanged libraries and no random outcomes:

1. `ironsmith-engine/src/effects/cards/discard.rs` and `special_actions.rs` use Public selected-card reveal policy for ordinary discard costs. The held grouped choice also uses it; Folio's reveal additionally calls RevealTaggedEffect.
2. `web/ui/src/hooks/peer-lobby/shared.js::collectCommandObjectIds` includes Public selected-card IDs in opening requirements. `optimistic-state.js::commandClaimIds` includes the same identities. `usePeerLobby.js` builds/stages/signs the selection command with its public openings. `ironsmith-wasm/src/lib.rs::ReplayDecisionMaker::view_cards` also merges active and audit viewed-card records.
3. `ironsmith-engine/src/game_loop/priority_cast.rs::activation_stage_after_targets` chooses `ReadyToFinalize` once costs finish. The `ReadyToFinalize` arm pushes the ability on the stack, clears the engine action checkpoint, and returns priority. The draw/damage/counter effect has not resolved; libraries remain unchanged.
4. `ironsmith-wasm/src/wasm_game_impl/runtime_flow.rs::record_completed_live_priority_action_for_undo` marks these nonmana activation roots undoable at the priority epoch. Its special committed lock covers irreversible mana activations, not hand-cost disclosures.
5. `wasm_game_impl/undo.rs::is_cancelable` and `is_replay_chain_cancelable` check mana, library, random, and land-play boundaries. They do not check public hand identities disclosed by discarded/revealed costs. `has_irreversible_library_change_since` cannot catch hand-to-graveyard discards or a reveal that leaves the card in hand.
6. `wasm_game_impl/dispatch.rs::cancel_decision` accepts that Undo, restores the priority/action checkpoint, and clears viewed-card buffers. A previously published opening remains known to opponents. `peer-lobby/connections.js::handleActionIntentCancelMessage` expressly keeps disclosure locks; those protect one command from substitution, not confidentiality across a later Undo command.

Thus each of the four earlier cards reaches the gap without a later mana prompt, and each of the three held group cards has the same completed-action path. Ordinary mana-only Undo must remain available.

## Distinct, not-yet-classified precommit routes

A published intermediate decision followed by another cost/replacement decision, an engine payment failure, or cancellation needs separate tracing. `priority_mana.rs::commit_prepared_activation_mana_payment` has failure rollback paths; `apply_mana_payment_plan_response_inner` has explicit Cancel rollback; WASM replay Undo can also restore an in-flight action. None of those locations alone proves a given printed card reaches a post-disclosure pending state in a legitimate flow. The normal trace above must not be replaced by a blanket assertion that every card permits a late mana cancel.

A disclosure-aware completed-action Undo latch is the first bounded fix. It must recognize the actual information boundary, preserve mana-only Undo, and retain peer proof validation. Any genuinely reachable precommit flow still needs transaction-scoped disclosure handling; late GameState restoration cannot erase knowledge. Availability probes and crypto previews must remain nondisclosing and must not permanently latch speculative state.

No game-state-only cancellation test establishes this property. Authored follow-up tests must inspect Undo availability, the peer-facing reveal metadata/audit buffers, and normal/failed/pending payment paths.

## Bounded completed-action Undo guard (authored, unrun)

`ReplayCheckpoint` now captures the set of already-public hand identity observations at the boundary. `undo.rs` compares against current cost-discard events, hand-reveal events, tracked public hand openings, and public active/audit views (including a captured pending decision game). Every ordinary action, epoch, and replay-chain Undo eligibility path applies the same test. A completed cost's event history keeps the boundary after active view buffers are cleared. No spell/ability names are recognized by the guard.

The comparison is read-only. It does not add a monotonically increasing speculative counter or alter the existing public proof policy. `preview_crypto_requirements` already restores the game, pending decision game, and active/audit buffers, so hypothetical observations disappear with the preview. Existing checkpoint clones and runtime savepoints carry the added knowledge baseline through their derived clone paths. A later checkpoint treats disclosures already known then as part of its safe baseline.

Authored regressions are included from `crates/ironsmith-wasm/src/wasm_game_impl/payment_disclosure_undo_tests.rs`: exact four-card live WASM dispatch to completed activation, reveal-without-zone-change, preselection cancellation availability, private-view/mana-only controls, audit-view observations, and speculative restoration. Direct rejected `cancelDecision` calls are additionally asserted on wasm32, where JS error values are available.

Deferred native command: `cargo test -p ironsmith-web-session payment_disclosure -- --nocapture`.

This guard closes only the demonstrated Undo decision route. It does not stage publications until a whole payment commits, and it does not by itself establish correctness of an engine-level failure/Cancel path that already emitted external material. No partial identity is promoted back to complete by this document alone; the independent precommit analysis remains open.


## Separate precommit opening-preview correction (authored, unrun)

The peer progress path contained another concrete earlier disclosure: `usePeerLobby.js::previewBuiltLocalOpening` placed `cardName` and `openingPreview` into `broadcastLocalActionProgress`. The callback runs during `build_local_openings_pre`, before `applySyncedCommand`. A later ordinary apply/proof/quorum failure can restore `localSubmissionSnapshot` and cancel the intent, but cannot undo that already-broadcast preview.

The bounded correction keeps the actor's local inspector preview and sends only a whitelisted generic operation plus numeric progress counts to peers. It does not remove or weaken opening requirements, local opening application, signed action payloads, quorum verification, or peer replay validation. Successful action publication still carries the actual openings through its normal payload. It does not add an early public preview after local apply, because quorum/publication can still fail then.

`web/ui/tests/opening-preparation-progress.test.js` authors identity-redaction and source-wiring contracts. Deferred command: `cd web/ui && node --test tests/opening-preparation-progress.test.js`.

This closes the ancillary progress-packet disclosure only. Actual cryptographic-material requests may carry openings before command publication, and a valid payment can span multiple accepted decision commands. Those are distinct from progress previews and remain subject to the transaction-boundary analysis above. This narrow correction alone does not promote any partial identity.


## Legitimate interrupted payment scenario (authored, unrun)

A focused Knollspine scenario now selects a hidden-tracked Fiery Temper while Rest in Peace is on the battlefield. Madness and Rest in Peace offer competing replacements after the public discard selection. The selected card has not yet left the authoritative hand, and the activated ability is not yet on the stack, but the public selection has been disclosed. The regression asserts Undo is unavailable during this replacement prompt and after choosing Rest in Peace; normal preselection cancellation remains available. It also asserts Knollspine's ManaPayment prompt occurs before disclosure, not after it. This exercises the existing guard against a captured pending-decision game rather than relying only on completed discard events.

The core's legitimate payment semantics already treat a legally started discard cost as paid even if replacement/prevention changes its destination (CR 118.11); this scenario must continue through the replacement, not invalidate the cost because the card was exiled. It uses existing final-rule primitives and makes no card-name production exception. The same deferred native test command above includes this regression. It has not run.

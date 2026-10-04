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

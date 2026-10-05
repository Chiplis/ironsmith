# Triggered-object characteristic frames (UNVALIDATED)

Reserved exact cohort: Sigil Captain; Unstoppable Slasher; Wilhelt, the Rotcleaver; Jason Bright, Glowing Prophet; Dawn Evangel; Tom, Bert, and William; Fyndhorn Druid. Full Oracle metadata and frozen failure identities are in `fixtures/triggered_object_predicates.json.fixture`. Burning-Eye Zubera and Rushing-Tide Zubera were transferred to the damage-history worker and are excluded from this cohort.

## First six: current versus past characteristic frames

Grammar reuses ItMatches and ItMatchedLastKnown for literal P/T, untyped absent counters, past keyword possession, singular object-role pronouns and a historical attachment. No runtime Oracle parsing is introduced. Effective power versus the same object's base power uses a typed comparison operator on ObjectFilter; both operands come from the same live or snapshot frame. Negative historical keyword/counter predicates require a real snapshot, rather than succeeding because an absent positive lookup returned false.

A current triggering-object condition now uses the exact live ObjectId at resolution. If that incarnation left, it uses its exact departure LKI, then the event snapshot when no later departure exists. An old successful event snapshot cannot override a failed current recheck, and no StableId lookup can follow a blink. The external trigger-time check prefers the captured completed-event frame. The cost/history worker owns completed ETB snapshots and deduplication, which supplies the entry frame for Sigil Captain.

Historical `with_attached_object` and `without_attached_object` filters use captured attachment snapshots, including their old controllers. They do not read an Aura's later controller or require an Aura to survive its host's death. An incomplete attachment snapshot cannot prove a negative attachment predicate.

Native and grammar regressions are authored for current 1/1 becoming 2/2, departure and blink, greater/less/equal base-power frames, Aura/host departure with controller separation, and the exact six predicate surfaces. Public direct/artifact full-card scenarios and Fyndhorn's directional blocked-history closure are still being authored; no complete identities are proposed yet.

No builds, tests, compilation or CLI probes have run. Only source reads, Python fixture generation, Rustfmt syntax parsing and whitespace checks were used.

## Seven complete-body source proposals

All seven exact fixtures are now proposed complete, with direct and JSON-restored artifact scenarios in `triggered_object_predicates.rs`. No compilation or execution has verified them.

Fyndhorn uses a separate passive `was_blocked_this_turn` filter, querying completed CreatureBecameBlockedEvent / attacking side of CreatureBlockedEvent receipts for the exact ObjectId. It does not confuse having blocked with having been blocked, does not depend on the current combat still existing, does not follow a blink, and expires with the turn history. Native coverage also includes becoming blocked without a blocker. No damage-history fields were changed.

Public scenarios cover Sigil's two-stat requirement and a real intervening change; Slasher's combat half-life body, counterless return, tapped/two-stun entry and negative second death; Wilhelt's decayed-token negative and end-step sacrifice/draw; Jason's positive/zero/negative effective-versus-base deltas plus paid sacrifice/counter/flying cleanup; Dawn's stolen Aura owner/controller separation and Aura/host departure before resolution; Tom's return as an artifact and actual sacrifice/draw-power/discard activation; and Fyndhorn's actual attack/block declaration, postcombat death, opposite-role negative and blink negative.

Deferred command target after the user opens validation: `cargo test -p ironsmith-compiler-runtime --test triggered_object_predicates`, plus native `referenced_characteristic_frame_tests` / `passive_blocked_history_tests` and grammar `referenced_characteristics_keep_current_and_historical_frames_distinct` / `passive_was_blocked_history_does_not_mean_the_object_declared_a_block`.

## Review correction: completed entry receipt dependency

Sigil's event-time frame is the completed battlefield destination, not ZoneChangeEvent.snapshot (which remains pre-move origin LKI). The condition consumer now selects destination_objects()[0] and its exact destination_snapshot receipt for ETB trigger-time checks. Resolution rechecks that destination incarnation, then its departure LKI; the original hand/stack incarnation is never substituted. LTB and explicitly past-tense predicates retain origin snapshots. Missing completed ETB evidence is not reconstructed from a possibly later live state.

This consumer depends on the cost/history worker's ZoneChangeEvent.destination_snapshots field and destination_snapshot(ObjectId) accessor, whose producer captures the completed original batch before replacement-added programs. A real casting regression uses a printed 0/0 entering with a +1/+1 counter, then optionally changes the completed 1/1 to 2/2 before resolution. Hand-authored native ETB fixtures now explicitly carry completed receipts.

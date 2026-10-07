# Chittering Rats and Chimney Imp frozen-body gate

Status: authored source-only candidate; all new checks UNRUN. Base: 94e075587a07b4ba659bcca84ded1d98605ccb82.

## Provenance and scope

The exact names, Oracle IDs, mana costs, creature types, power/toughness and complete Oracle bodies were read from the inherited fixtures/card-failure-campaign/cards-20261003.json.xz. They are frozen in fixtures/opponent_hand_library_top_bodies.json.fixture. No live card service or mutable cards.json is required by the authored tests.

- Chittering Rats: 08dfe42e-35c0-4be0-abba-57269792ff3d; {1}{B}{B}; Creature — Rat; 2/2; complete ETB instruction.
- Chimney Imp: 3901bf30-b7c1-4977-a7b1-fcdafcc266cd; {4}{B}; Creature — Imp; 1/2; Flying plus complete dies instruction.

The supplied unadmitted-measured-recoveries inventory marks both bodies unaddressed. Historical measured-main compilation does not establish current source/gameplay admission. No inventory, accounting, coverage status, or measurement credit is changed here.

## Authored gates

crates/ironsmith-tools/tests/opponent_hand_library_top_bodies.rs independently compiles the complete input through the strict registry runtime route and strict compiled-artifact route. The latter validates, serializes, deserializes and materializes its own artifact. Neither route serializes the other's runtime definition. A separate strict snapshot assertion rejects lossy, unimplemented and fallback results. Exact mana pips, types, subtype, power/toughness, ability cardinality, opponent target cardinality and Imp flying are asserted.

The gameplay matrix uses real entry and death zone changes, drains their native events, announces targets through the native trigger stack path, and resolves the retained stack entry. It never extracts and executes a shortened effect list.

Authored expectations include:

- Three players with either opponent selected; source controller chooses the opponent, that opponent chooses their own hand card. Imp's source owner B differs from its controller A.
- Empty, singleton and three-card hands; land and nonland eligibility, deliberate non-first selection, other players' hands and all graveyard/exile decoys excluded. The generic hand-card filter has no identity-dependent quality: the native helper auto-selects a forced singleton without an object-choice prompt, while still invoking the private visibility callback. The prompt oracle expects zero for singleton/empty/fizzled cases, one for surplus and two for surplus pending/resume. This test-only correction was confirmed by source inspection during independent review; no runtime result is claimed.
- Exact complete library order preserved with the chosen card appended at the engine's top; empty-library control and every other player's library unchanged.
- Hidden hand references attached; no public selection policy, no public view callback, no views to the source controller or uninvolved player, no public-reveal marker on the moved or remaining cards. These are engine-level privacy expectations, not a cryptographic protocol or UI audit.
- Source departure and Rats control change followed by departure after announcement; trigger controller remains the original controller.
- Hexproof acquired after announcement fizzles before the hand decision; preexisting hexproof excludes that player at announcement; self and object targets excluded.
- Native pending-choice rollback retains the stack entry, all hands and library order; resumption moves exactly one card and completes the entry.
- Unrelated entry, Rats death, Imp exile and Imp entry are negative trigger controls.

Initial mana-free board objects and zone transitions are fixtures; these tests do not certify paid casting, combat flying restrictions, network replay, or cryptographic hidden-card opening. They cover printed body structure, matching real ETB/death events and native trigger resolution. No production owner edit was needed based on static inspection: inherited targeted-subject lexical binding and MoveToZone actor routing already express the relevant ownership boundary.

## Verification boundary

Only source/data reads, source authoring, and local git operations were performed. Builds, tests, probes, formatters, corpus runs, code generation and remote writes were not run. New tests have no execution result. Any compiler/API or behavioral issue first exposed by future authorized validation remains a blocker to measured or final source/gameplay admission. No recovery count is claimed.

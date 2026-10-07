# Independent source review: opponent hand to library top

Reviewed commit `6fb101b9d443858d4a5eecf27ec07538def1108c` against `94e075587`. Source/data inspection only. No builds, tests, probes, formatters, corpus execution, code generation, or remote writes. No source edited by reviewer.

## Finding requiring test-only correction

**P1: singleton scenario incorrectly requires a card-selection callback.** `crates/ironsmith-tools/tests/opponent_hand_library_top_bodies.rs:228` expects one `decide_objects` call for every nonempty successful move, including `count == 1`.

The relevant native route is `MoveToZoneEffect` → `resolve_zone_move_objects` → `resolve_objects_for_effect_with_choice_description`. The final helper returns the sole candidate directly when `min == max == 1` and `!hidden_hand_choice` (`crates/ironsmith-engine/src/effects/helpers.rs`, around 3158). `hidden_hand_choice_for_filter` requires an identity-dependent filter, not merely attached `HiddenCardInfo` (`game_state/hidden_hand_choices.rs:541`). Generic card-from-hand selection has no stated card quality: zone, owner and controller are excluded from that determination (`ironsmith-core/src/filter_model.rs:3361`). The singleton still receives the native private view and moves automatically, but never reaches the decision callback. The later general decision-dispatch guard against auto-picking hidden candidates cannot override this earlier helper return.

Requested correction: zero selection callbacks for singleton, empty and fizzled cases; one for successful three-card selection; two for its pending/resume case. Keep exact singleton movement/library/privacy assertions. Sent the finding to parent and author and requested a local test/report correction, without production edits or execution.

## Source checks otherwise supported

- Frozen fixture names, Oracle IDs, mana costs, type lines, power/toughness and complete Oracle text match the two records read directly from inherited `fixtures/card-failure-campaign/cards-20261003.json.xz`.
- Direct and artifact cases independently invoke strict registry entrypoints with `allow_unsupported=false`. The artifact route does not serialize the direct route's runtime result; it validates and round-trips its independently compiled artifact before materialization. A separate strict snapshot rejects fallback, parse loss, errors and unimplemented content.
- Both full bodies retain required mana pips, subtype, power/toughness, exact ability cardinality and Imp Flying assertions. Trigger target cardinality is checked both structurally and through native announcement requirements.
- Native ETB processing and native battlefield-to-graveyard movement produce event queues; trigger stacking and resolution use ordinary game-loop APIs, not extracted effect lists. Initial direct battlefield creation is an assembly fixture and does not itself queue ETB triggers.
- Three-player target checks exclude self and objects using exact cardinality plus required membership. Opponent C is selected as well as B. Imp ownership B versus controller A distinguishes controller targeting from source-owner targeting.
- Surplus hand selection deliberately chooses the final candidate, checks all and only the intended hand, and includes land eligibility alongside instants and wrong-zone/other-seat decoys.
- Library order oracle agrees with engine bottom-to-top storage: selected card is appended at the top and the complete preexisting vector retained; all other libraries are compared exactly. Hand snapshots are captured after Rats leaves its own hand and after any requested source departure, avoiding the reversed-snapshot defect seen elsewhere.
- Privacy checks reject public choice policy and any public view or view to another seat; moved and retained cards must lack public-reveal markers. Hidden metadata is attached to every hand fixture. This is an engine-level privacy gate, not a cryptographic/network/UI proof.
- Source movement is tracked correctly: Rats uses ETB receipt's new ID; Imp uses the post-death graveyard ID. Post-announcement Rats controller change plus exile and Imp graveyard-to-exile departure retain the captured trigger controller. Selection callback verifies the chosen opponent remains decision owner.
- Post-announcement hexproof expects no hand prompt or move; preexisting hexproof checks exact announcement target exclusion. Other-object entry, Rats death, Imp exile and Imp entry are negative controls.
- Pending choice uses native resolver checkpoint rollback; exact hands/libraries and retained stack length are asserted before resumption. Successful resumption must complete the stack entry and move the selected physical card, tracked through stable identity.
- Called APIs and field types inspected in engine, registry/compiler bridge, and tools source; no additional concrete signature/type mismatch found. No new serialized JSON-field oracle is used here.

## Admission boundary

Original reviewed commit needs the singleton assertion correction before source-proposal clearance. Subject to that correction and re-review, the packet is suitable as authored, independently reviewed, UNRUN evidence for source-proposal completeness. This does not require execution before source-proposal admission and confers no executable gameplay certification, measured recovery, residual reduction, inventory change, or recovery count.

## Correction re-review

Reviewed follow-on commit `25e628c833f406743df4c142af3bb4858c12b225`. It makes exactly the requested prompt-count correction (`moved && count > 1`) and documents the native singleton behavior, without changing production code or weakening movement, ordering, visibility, target or rollback assertions. The sole concrete finding is resolved.

Additional source inspection confirmed ordinary zone changes replace object IDs but retain the physical stable ID (`zones_and_characteristics.rs:1291`); only combined permanent representations take a replacement stable ID, irrelevant to these fixtures. Stack resolution preserves captured ability controller and source snapshot and checks all-illegal targets before executing effects (`stack_resolution.rs:1126`, `1266` onward). Thus source departure and post-announcement hexproof expectations are appropriate source-level gates, not evidence of current runtime success.

**Final disposition:** source-proposal packet clear at `25e628c833f406743df4c142af3bb4858c12b225`, with authored gates remaining UNRUN. No additional concrete source/API/oracle defect found in reviewed scope. No execution or measurement credit.

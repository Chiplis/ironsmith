# Loss of unspent mana and conversion

Status: **UNVALIDATED source proposal. No build, compilation, test or engine probe was run.**

## Exact scope

The frozen fixture contains six complete Oracle rows; five are proposed complete at source level:

- Horizon Stone — add46292-470e-47fa-a261-4884d02e65fe
- Kruphix, God of Horizons — b2cefcd6-4b81-479c-86ff-1695b836972c
- Ozai, the Phoenix King — 7472af8e-2e93-43b2-bc32-a3741f6d7502
- Omnath, Locus of Mana — 9da896e1-2256-425b-b801-1ae6f0470559
- Omnath, Locus of the Void — 579225b5-50e2-4891-8871-0bf6dbc07e33

Omnath, Locus of All (26445dc7-8363-4205-aee8-1cafeb4ba4c0) remains **partial, not counted**. Its black conversion has the shared owner, but the optional top-card reveal qualified by three colored cost symbols, conditional mana from that card's colors, and unconditional final move to hand still need a complete verified program. Drain Power, Pygmy Hippo, Yurlok, Leyline Tyrant and instruction-local combat-retention bodies are separate prerequisites; this does not count them.

Fixture `fixtures/mana_retention_conversion.json.fixture` SHA256: `0a73a6ed97fe570fe8aa1d0e4558bc184360fe9636758f22080ee24dd0a00cfa`.

## Actual operation

The appended typed static payload generates a live source-controlled `ManaLossMatcher`, with normal source zone, control, condition and phase-out discovery. `ManaLostEvent` is distinct from mana production. One proposal freezes exactly the existing units due to be lost, excluding global color retention and unexpired individual retention at a step boundary. Forced loss ignores those boundary-only permissions.

The affected player orders applicable replacements using the existing checked replacement engine. Conversion changes the selected old units' actual type and ends the would-lose event; it never credits fresh mana or emits ManaAdded. Already-red or already-colorless mana is still retained. Another converter cannot reapply to a loss that no longer occurs. Empty proposals ask no question.

Before recoloring, every restricted unit is paired with its original production snapshot and restriction. After conversion, the two vectors are rebuilt in the same canonical order, preserving the existing exact payable-unit index pairing when formerly distinct colors collapse to the same type and producer. Snow, source identity and last-known characteristics, chosen types, original producer controller, on-spend riders and individual retention metadata are preserved. There is no new payment-price or constrained-X owner. Converted mana pays according to its actual new type; as-though permissions remain independent.

All players' proposed losses are prepared in APNAP order against the pre-commit world; all original pools commit before added replacement programs run. The batch uses the existing simultaneous action and trigger-receipt owners. Pending choices, malformed results, stale unit identities and typed resource exhaustion roll back the entire pool operation. Checked per-type conversion totals reject a representational overflow rather than truncating mana.

## Real turn boundaries

TurnRunner has a private replay owner for mana-loss choices. It collects exact responses against an unpublished game clone and resumes the same boundary. Untap has a separate terminal state so answering a mana-loss question cannot untap or begin a turn twice. A skipped final step retains the actual phase-ending continuation: skipping end combat does not leave Firebending units alive indefinitely.

CR 500.5 expires end-of-combat effects before loss. The helper handles this before replacement discovery; individual markers also expire before deciding which units would be lost. The end-combat and end-turn procedures preserve their original post-SBA continuations. Cleanup's public `execute_cleanup_step` now performs CR 514.2 only; the runner empties mana when the cleanup step actually ends, after 514.3 checks or its priority window. This preserves mana during cleanup priority and prevents a just-expired turn retention rule from retaining it into a later turn.

The compatibility `GameState::empty_mana_pools` now returns a Result and rejects an unresolved player choice rather than choosing a converter silently. Production runner calls use `empty_mana_pools_with_dm` through the replay owner.

## Other bodies and recovery

The existing green-only unspent-mana anthem is retained. The appended total-unspent count supports Omnath of the Void; the typed live threshold supports Ozai's conditional flying/indestructible. Existing mana-sensitive continuous-state invalidation recomputes both after production, payment and conversion. Kruphix's devotion/type, indestructible and hand-size bodies, Void's real landfall CC, and Ozai's actual attack-triggered Firebending have authored direct/artifact gameplay gates.

The preceding shared checkpoint correction rejects live mana provenance rather than exporting only aggregate counts. This proposal additionally rejects unencoded pending mana-loss answers and skipped-phase continuations, including multiplayer lanes. Native savepoints retain those exact owners; network recovery remains full accepted signed-transcript replay with existing authenticated material/opening checks. There is no hidden snapshot export or host-asserted recovery shortcut.

## Authored gates and limits

Unrun engine tests cover exact restricted/snow pairing after colliding-color conversion and actual payment, overflow rollback, affected-player pause, simultaneous added-program ordering and duration expiry. Unrun compiler/runtime tests cover all five full frozen bodies in direct/artifact modes, source leave/control/phasing, green retention versus forced loss, actual planner/payment and live mana statistics, Kruphix's other abilities, Void's entry trigger, Ozai's declared attack, skipped/end-combat procedures and cleanup ordering. Named grammar tests reject unsupported colors, scoped losses and trailing instructions. A Wasm scenario retains the new continuation in a native savepoint and rejects lossy export.

No new measured recovery is asserted. This is not a proof of arbitrary numerical ranges for the engine's pre-existing signed scalar/P/T domain; full deferred regression and resource-boundary execution remain necessary.

Rules: [current Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt), particularly 106.4, 106.6, 500.3, 500.5, 514.2–514.3 and 616. [Official Avatar release notes](https://magic.wizards.com/en/news/feature/avatar-the-last-airbender-release-notes) confirm Ozai retains the mana's restrictions/riders and Firebending's combat boundary.

### Direct-owner review correction

The complete mana-loss operation now opens `execute_resource_transaction_atomically`, sharing one work meter and typed-failure latch across every player's replacement-added work, including the public direct boundary and TurnRunner. Generic effect owners reuse an enclosing meter. Authored cases cover two one-token additions at allowances one (whole-operation rollback) and two (exact success).

Arbitrary `Instead` executable programs are rejected during preparation, before any original pool commits. They require their own simultaneous original/completion contract; running a whole program during one player's commit could alter another player's prepared pool or observers. The printed conversion cohort uses `Modified` events and remains supported. Added programs retain their existing deferred completion path. An authored rejection case asserts both pools and life remain unchanged.

Preventing the loss also commits already-expired unit-duration metadata while retaining the pool. Prevention cannot revive a Firebending/end-of-turn marker in a later step. Both deadlines have authored prevented-loss then next-boundary regressions (unrun).

### Checked scalar follow-up

Unspent-mana reads now sum in a wide domain and narrow with an explicit checked conversion. Direct execution values and resolution count predicates return typed resource errors. Authoritative continuous discovery and cached query admission reject a combined pool count beyond the engine's existing signed scalar representation, before an infallible filter/condition adapter could report a wrapped value or false predicate. This is an explicit representation boundary, not an arbitrary gameplay truncation.

Mana-derived anthems preflight their multiplied/capped modifiers before native emission. Discovery validates the complete P/T layer for worlds with those anthems. Native P/T additions and counter deltas retain an explicit provisional numeric-range failure; checked characteristic consumers reject that result rather than publishing a wrapped or clamped power/toughness. Ordinary boards without a mana anthem do not acquire the extra final P/T discovery pass. Mana-sensitive cache invalidation remains authoritative.

A mana credit also checks the count domain and validates the new board before reporting success. If a representable count would make the base P/T plus mana anthem exceed the signed domain, the original credit is restored and the checked discovery error propagates. This preserves production-time source snapshots, exact unit pairing, price metadata, replacement witnesses and payment scopes.

Additional unrun cases cover the largest representable positive P/T, a one-mana credit beyond that boundary with exact rollback, counter addition beyond it, raw count overflow across colors and players, typed legality/execution failures rather than negative memoized answers, and a subsequent ordinary 501-mana query after the invalid state is corrected. There is no arbitrary-precision claim.

Final range admission also covers every ordinary P/T-layer effect and native P/T counter, not just mana anthems. Otherwise the new marker could make a legacy characteristic lookup return absent inside an allegedly complete ordinary query (for example skipping a fight). Worlds with neither a P/T modifier nor P/T counters retain the cheap path. Authored no-mana-anthem effect/counter cases require the control boundary, query snapshot and Fight instruction to return typed failure with no damage.

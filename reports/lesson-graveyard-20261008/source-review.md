# Lesson graveyard existential regression repair — source only

Base: `8ac862bff90d9ffbde89195187369d5ccf283a9e`.

Status: proposed source repair; all newly authored and inherited tests are **UNRUN**. No builds, probes, corpus runs, code generation, formatters, or remote writes were performed. No measured support, runtime correctness, passing-gate, or original-majority credit is claimed.

## Evidence and scope

The retained `current-refresh-20261008/analysis/current-unresolved-entries.json` reports complete-body parser failures for:

- Dragonfly Swarm, `83abf1d0-04e4-49e5-bf63-59f77829ffd1`, content hash `e96329e842ac2a47b2b567e2f5dafad5cf5e545f41e86c6eb9eba1b84604f7cf`.
- Walltop Sentries, `fb3a0910-a582-41ef-b5b9-3cda1f1cd5ce`, content hash `5c0d2b7823811c5804d29526de64d7683e1d07b9c0a301bffd1934ba345088a1`.

Both failures identify the contracted Lesson graveyard intervening-if. The historical `fixtures/intervening_predicate_cohort.json.fixture` supplies the full raw bodies and actual mana cost, creature subtypes, and printed power/toughness. These cards were already historical source proposals. Repairing their current regression does not constitute two new original-majority completions.

## Source finding and repair

`OwnedLexToken` preserves apostrophes in its token spelling, while `TokenWordView` strips them through `push_normalized_token_words`. Thus a token keyword can match `there's`, but a `WinnowSequence` capture and `surface::exact_any` consume `theres`. The two graveyard predicate readers used the token spelling in their word-view alternatives. The existing anthem existential head operates at token level and already admits contracted forms; changing that unrelated owner would not fix the full predicate route.

The repair centralizes the graveyard existential word-view alternatives and uses the normalized `theres` head. Both independently articled conjunctions and single-card graveyard conditions retain their existing complete captures, descriptor/filter parsing, and location validation. Their token guards also cover the normalized contraction spelling, so symbol tails cannot disappear in a word-only capture. No global string rewrite, card-name exception, new fallback, or runtime interpretation was introduced. Existing owner, zone, Lesson subtype, trigger-time check, and resolution-time recheck lowering remain unchanged.

## Authored coverage, not execution results

New grammar regressions explicitly inspect the word view for straight apostrophe, typographic apostrophe, and normalized contraction; require the exact owned-Lesson-graveyard filter; and reject trailing mana/operator tokens, unknown qualifications, incomplete conjunction/disjunction, and extra location. Existing expanded `there is` controls remain.

New full-body runtime tests compile direct definitions and separately validate, serialize, deserialize, and materialize artifacts. For straight/typographic apostrophes and expanded `there is`, they require the exact lowered condition and test false-at-death/true-later, true-at-death/false-at-resolution, and replacement-Lesson-at-resolution behavior. A stolen source makes the last controller's graveyard and reward recipient explicit; wrong-owner, hand, exile, and non-Lesson cards are decoys. Rewards are exactly one draw or two life, with all other players unchanged. The same complete bodies retain Dragonfly flying/ward and dynamic graveyard power, and Sentries reach/deathtouch. Full-body invalid-tail variants must fail direct and artifact compilation.

The tools regression identifies both oracle IDs and uses actual metadata for complete raw and reminder-stripped bodies, rejecting lossy/fallback or unimplemented results. Existing full-body tests additionally cover real ward payment, teammate exclusion, printed metadata, noncreature/nonland power exclusions, source controller changes, and cloned-game resolution. None of these existing tests is represented as having passed.

## Successor boundary required

Inherited artifact16/audit30 outputs remain immutable. There is no artifact schema or runtime condition model change, but compiler-source provenance has changed. The publication/integration coordinator must include this source commit in a fresh successor boundary, run the focused grammar/runtime/tools suites and required aggregate checks when authorized, then regenerate and audit complete-body results on that exact integrated head. Preserve the fresh baseline failures and historical proposal accounting; do not copy support counts, artifacts, audit results, or execution credit from this source-only patch.

## Review correction: reward baselines

The contraction matrix intentionally places a Lesson decoy in B's hand. Its original call to the older empty-hand reward helper was therefore invalid. The corrected scenario snapshots every player's hand, library and life before the death trigger, preserves all existing hand identities, checks the exact one-card delta and drawn stable identity for Dragonfly, and checks exact library/life deltas for all four players. Walltop adds no card. The separate older zero-baseline scenarios keep their existing helper. This is an authored source correction only; no executable gate was run.

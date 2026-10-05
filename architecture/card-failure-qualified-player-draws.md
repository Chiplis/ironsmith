# Qualified draw/life participants (UNVALIDATED)

Frozen stack07 cohort: exactly four identities in `fixtures/qualified_player_draws.json.fixture`, preserving complete Oracle text, type/mana metadata, Oracle IDs, content hashes and diagnostics.

Proposed complete: Psychic Possession; The Watcher in the Water; Wiretapping (three). Partial: Wedding Ring (one), pending capture of event-time qualifications before replacement-added programs in every actual combat/cost producer path. Grammar acceptance alone does not close that runtime seam.

## Typed ownership

- Grammar reads singular participant, optional `who controls <object filter>`, and exact event/turn tail. Unknown subjects and compound tails fail closed.
- Positive draw-turn and first-card-of-each-own-draw-step events have separate appended core variants. First-of-step reads captured physical draw ordinal, not total draws this turn. Per-card events retain their multiplicity.
- Enchanted opponent is an authenticated attached-player tag. Native draw matchers use the common strict player-filter evaluator, including opponent/team and unresolved-tag behavior.
- `who controls` uses the existing typed event qualification, not an intervening-if. Its triggering participant is independent from the ability controller and from the active player. Qualification wrappers forward per-event count, amount, lookback and snapshot capabilities.
- Qualified life events retain the actual life-gain amount through existing typed life-event binding. Explicit life-body producers keep precedence.
- `a2619604` is the draw-step ordinal/checkpoint prerequisite, with no separate card count.

## Authored scenarios

Direct and restored-artifact scenarios retain all four full cards. They cover a real paid Psychic Possession cast/attachment, other-opponent exclusion, optional refusal, and its draw-step skip; Watcher's real entry with nine stun counters, teammate versus opponent turns, per-card Tentacles, and both death-trigger targets; Wiretapping's real Hideaway entry, zero-mana play of the exiled nine-mana artifact, upkeep versus draw-step ordinals, bonus draw nonrecursion, and an adjacent extra draw step; and Wedding Ring's paid cast, nonrecursive token copy, own-turn/participant qualifications, actual gained amount, per-card count, and removal of the partner's artifact after trigger capture.

Grammar/native negatives cover unknown participants/tags, incomplete who-clauses, unsupported timing/compound tails, zero draws and captured multi-card first-step batches. Native scheduler and checkpoint regressions are in the prerequisite.

No builds, compilation, test execution or compiler probes were run. Source formatting/parsing and whitespace checks only. Deferred validation targets: compiler-grammar qualified_player_events; engine qualified_player_draw and draw_step_ordinal tests; compiler-runtime `qualified_player_draws`; WASM draw_step_ordinal_transport tests.

# Aggregate scalar damage quantities

Status: **UNVALIDATED source proposal. No compilation or tests were run.**
This is seven exact frozen baseline identities, not a claim for the whole
missing-damage-amount diagnostic family. The fixture retains each complete
Oracle body, mana cost, types, P/T and (for Ral) loyalty.

## Membership and source-level boundaries

| Card | Quantity and repair |
| --- | --- |
| Barret Wallace | Count equipped creature hosts controlled by the ability's controller at resolution. A plural equipped group uses a has-Equipment attachment filter, not the source Equipment's `equipped` object tag. Multiple Equipment on one host count once; Equipment ownership does not restrict the host. |
| Beacon Bolt | Total **number** of owned instant-or-sorcery cards in the union of exile and that owner's graveyard. The typed equal-to cardinality prefix accepts `total`; the existing disjoint-zone union and type-OR filters are retained. |
| Ral, Izzet Viceroy | The same owned-zone cardinality, with the existing loyalty ability and all other printed abilities preserved. |
| Runebound Wolf | Existing complete typed count of Wolves-or-Werewolves under the controller scope. The amount-before-target handler now stops after a complete typed reading instead of eagerly running incompatible permissive probes. |
| Summon: Esper Ramuh | Existing complete count with conjunctive noncreature and nonland exclusions in the controller's graveyard. Preserve its comma-separated operand and both later Saga chapters; use the same bounded dispatch correction. |
| Triumphant Chomp | A composable greatest-power operand inside the existing `A or B, whichever is greater` expression. The interpreter recognizes its exact existing `A+B-min(A,B)` structure before an intermediate signed overflow; surface hints do not supply semantics. |
| Burn at the Stake | Existing integer scaling of a typed prior Tapped action count binds to the exact imported paid tap group when there is no local producer. Spell additional costs and activated costs retain their actual, different tag namespaces. |

All seven had missing-damage-amount errors in the frozen stack07 snapshot.
Some errors exposed truncated count operands. Earlier shared quantity and
zone-union changes are prerequisites; this patch also adds whole-sentence
regressions so a successful isolated operand is not treated as complete-card
proof. Before/after expectation for each fixture is failure -> strict compiled
and non-lossy; this remains an expectation until the deferred build phase.

## Semantic constraints

- The aggregate reader remains the shared typed reader: historical aggregates,
  prior-action results and source-linked exile keep their dedicated values.
  Nested operands remove only the accidental enclosing `EqualTo` hint.
- Maximum recognition requires structural equality of both repeated operands.
  It cannot turn a mismatched expression into a maximum based on its renderer
  hint. No serialized variants are introduced.
- Tap cost fallback is limited to typed Tapped object cardinality with no player
  or counter metric. It preserves the authored object predicate and counts stored
  snapshots, removing only the current-zone constraint. Untapping, a later control
  change, death or leave/return cannot change the paid group. A compatible local
  tap effect wins; unrelated effects do not become its producer. Missing imports
  and other action/metric domains still reject.
- Singular `equipped creature` remains the source-linked attachment reference.
  A plural has-Equipment condition must not overwrite an existing Aura condition;
  the existing nested filter conjunction represents both independently.
- Permissive fallback probes are not invoked after a complete typed damage
  amount has won. Diagnostics themselves are not suppressed or discarded.

## Authored, unrun regressions

- Normal tools integration target `aggregate_damage_quantities`: all seven exact
  metadata-bearing payloads require strict, non-lossy results.
- Normal compiler-runtime target of the same name: direct and JSON-round-tripped
  artifact materialization; actual casts, Jump-start discard/exile, loyalty payment,
  tap payment (including choosing zero), real attacker declaration and Saga lore
  actions. Responses change zones, controller, source presence, P/T and tap state.
- Grammar regressions inspect typed count/maximum operands and run the ordinary
  full sentence entry point without lossy suffix recovery. Filter tests distinguish
  plural equipped hosts from the singular source link and preserve an Aura predicate.
- Reference-resolution regressions preserve the exact tap tag/filter, check local
  producer priority and reject missing or inapplicable metric domains.
- Shared interpreter tests cover canonical maxima across signed integer extremes
  in both execution and continuous evaluation, plus a mismatched hinted expression.

## Still outside this proposal

Personal Incarnation remains open: its owner-only activation permission and
owner damage redirect require an actual legality/runtime correction before its
quantity can be counted. Mathemagics and the four multi-source simultaneous damage
cards remain explicitly unresolved as documented in their separate design notes.

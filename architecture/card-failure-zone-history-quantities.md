# Owned-zone counts and opponent turn totals

Status: **UNVALIDATED implementation-first source proposal** for four exact frozen
cards: Florian, Voldaren Scion; Notorious Throng; Serpentine Curve; Slime Against
Humanity. Fixtures retain full metadata, Oracle text, and stack07 diagnostics.
No build, compiler, CLI replay, or tests have been executed.

## Shared boundaries

- Opponent life loss and damage are separate existing values:
  `LifeLostThisTurn(Opponent)` and `DamageDealtToPlayersThisTurn(Opponent)`.
  The shared expression reader accepts their authored quantities and existing
  renderer wording. It does not turn life gained/net change into life loss, count
  the number of affected opponents, or treat damage dealt *by* opponents as damage
  dealt *to* them. Existing turn-history evaluators retain those distinctions.
- The optional word "total" before "number of" now enters the existing complete
  count/filter reader. Ordinary arithmetic composes the fixed offset with that
  count; the counted domains remain actual card objects currently in those zones.
- The existing owned cross-zone union reader retains its compact type/subtype
  representation wherever possible. A subtype-or-name selector cannot flatten
  into that representation, so a bounded fallback retains it nested within each
  domain. The result is `(exile AND (Ooze OR named card)) OR (graveyard AND (Ooze OR
  named card))`, with shared ownership preserved. A card matching both properties
  is visited once, not counted once per branch. Explicitly scoped selector tails
  are not erased by the inferred-battlefield-default cleanup.

No new runtime evaluator, serialized variant, fallback route, or loss suppression
is added. The existing union/count/history machinery executes these quantities.

## Authored verification

The tools/runtime integration targets are both `zone_history_quantities`.

- Four strict metadata inputs without lossy fallback, artifact JSON round trip,
  and direct/materialized typed programs
- Distinct history metrics and directed recipient/actor negative grammar cases
- Typed fixed-plus-count expressions retaining both owned card zones
- Nested subtype/name and location disjunctions remain independent
- Public damage, prevention, life gain, life loss, and life-payment actions in a
  three-player game; Infect damage adds to damage but not ordinary life loss
- Florian sees 14 life lost, despite life gained and only 10 opponent damage,
  looks at exactly that many cards, grants the actual exiled card's cast
  permission, and sees a zero total after the turn resets
- Notorious Throng creates ten tokens from actual opponent damage and grants an
  extra turn only through actual eligible paid Prowl casting
- Serpentine Curve counts owned instants/sorceries across exile and graveyard,
  excludes other owners/zones/types, counts a dual-type card once, and observes a
  qualifying card's zone move while the spell waits
- Slime Against Humanity preserves subtype OR exact name inside the owned-zone
  union, counts a card matching both once, excludes foreign/other-zone/nonmatching
  cards, and produces the correct counter total/trample token

All coverage and test outcomes remain proposals until deferred execution.

# Referenced spell and player-action damage quantities

Status: **UNVALIDATED**, source proposals only. No builds, compiler executions, or tests were run.

## First bounded checkpoint: four proposed complete identities

Exact full frozen Oracle text and metadata are in `fixtures/prior_damage_quantities.json.fixture`:

- Cerebral Vortex: the referenced player's completed draw history this turn, including the current draws, excluding other players and replaced draws.
- Essence Backlash: the exact targeted creature spell's power and controller, with live data if countering fails and exact stack-object departure LKI if it succeeds.
- Malignant Growth: the exact preceding draw instruction's completed count for that player, rather than its requested count or all draws this turn. The full fixture retains cumulative upkeep and both upkeep/draw-step abilities.
- Rumbling Aftershocks: actual kicker and multikicker payments on the triggering spell, distinct from the resolving enchantment's optional costs.

The frozen baseline diagnostics all reported missing damage amounts for these exact quantities. Expected future status is strict, metadata-bearing, non-lossy full-card compilation plus these gameplay semantics.

## Implementation boundary

The existing `TurnHistoryCount::CardsDrawn`, typed `PendingPriorEffectMetric` for `Drawn`, and exact `PowerOf` object references are reused. New lexical forms preserve participant identity and the difference between this-turn history and this-way result memory.

`Value::KicksPaidOf(ChooseSpec)` is appended at the end of the serialized enum. It uses the existing object-number reader, which selects an exact live object or its departure LKI. Snapshot and object cast metadata already retain optional costs paid. Reference resolution, tag inspection/binding, dependency classification, and renderers recognize the new node; ordinary `Value::KickCount` semantics remain unchanged. Only Kicker and Multikicker payments count. The new reader uses checked `i32` conversion/addition and reports unresolvable out-of-range values instead of wrapping. No new continuous-anthem admission is added.

## Authored regressions

Normal compiler-runtime target `prior_damage_quantities`: seven tests, with full-card direct/artifact-JSON definitions, public real casts/payment, draw replacements, turn reset, successful/uncounterable creature spells and calculated stack-power versus destination-graveyard CDA, actual cumulative-upkeep/growth triggers, source-controller changes, paid kicker/multikicker casts, and an exact departure snapshot versus the same stable card's new incarnation.

Normal tools target `prior_damage_quantities`: all four metadata-bearing strict/non-lossy candidates aggregated. Grammar tests distinguish prior result from turn history and verify referenced spell quantities do not become source quantities. Engine unit coverage pins typed kicker sums and numerical overflow rejection. All are authored and unrun.

## Remaining two identities in the reserved six

Friendly Fire and Volcanic Eruption remain **partial and uncounted** in this checkpoint. Their single-source damage goes to multiple object/player recipients with a shared prior-result amount. Existing compound fan-out lowers to sequential player/object effects; simply parsing their suffix quantity would not establish simultaneous damage, shared prevention allocation, or a single lifelink event. A bounded one-source recipient-set effect can gather the amount and recipient set before entering the existing simultaneous damage executor. That mechanism, exact prior revealed-object / destroyed-Mountain result binding, and tests must precede any full-card claim.

Whipkeeper and Impact Resonance remain separate damage-history primitives. No new coverage is claimed for them.

The later `card-failure-single-source-damage-set.md` checkpoint implements the two remaining recipient-set bodies and their exact fixtures. This document's checkpoint remains four identities.

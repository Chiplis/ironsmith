# Temporary damage-prevention recipients and sources

## Status

UNVALIDATED, source-only implementation. No build, compilation, test or corpus
replay was executed. The measured baseline remains 40 strict recoveries and
3,193 unresolved unique cards. The ten exact frozen identities in
`fixtures/temporary_damage_prevention.json.fixture` are additional proposed
candidates, not measured recoveries. They use the frozen dataset SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.

## Shared changes

- Combat prevention carries a typed `combat_only` flag for object, target,
  self-reference and player/object-union recipients. It stays distinct from
  all-damage prevention through lowering and the artifact's existing executable
  shield models.
- Source-before-duration passive clauses and active “source would deal” clauses
  retain complete source phrases. Typed object filters describe untargeted source
  sets; explicit targets retain choices, counts and subsequent reference tags.
- Unlimited shields are installed separately for every selected source. The
  previous executor retained only the first selected source. Optional zero-target
  selections do not install a global shield.
- Plural antecedents retain their existing typed target/reference path. They
  do not turn into an unconstrained object set.
- Unrecognized recipient tails, conditions, source exclusions and multiple
  sentences cannot disappear behind a partially recognized source clause.
  The combat flag is not discarded when considering counter follow-up rewrites.

The bounded complete candidates are Al-abara's Carpet, Azorius Ploy, Energy Arc,
Ethereal Haze, Fleeting Flight, Hidden Retreat, Pack Leader, Repel the Abominable,
Shieldmage Elder and Soul Parry. Their other clauses use existing activated-cost,
anthem, attack-trigger, counter, flying, untap and target-reference paths.
The exact full-card fixture test is authored to challenge that source assessment;
none is established as compiling or executing successfully yet.

## Deferred scenarios

Authored grammar cases retain combat/turn scope, count explicit source targets,
and reject unsupported tails. Direct and restored-artifact runtime scenarios use
real spell target requirements, target assignments and the stack resolver. They
cover live recipient filters, later creatures, changing control, both selected
sources, combat versus noncombat damage, unpreventable damage and end-turn expiry.
A plural-reference scenario deliberately starts with already-untapped creatures
to ensure the subsequent protection refers to all declared targets, rather than
only objects changed by the preceding untap instruction.

## Boundaries

“This combat” now retains `Until::EndOfCombat`, backed by the combat-boundary
lifecycle described below. No additional full-card identity is counted solely
from that duration capability. Chosen sources/colors, finite shared budgets, prevention-triggered/reflexive
follow-ups, conditional copy programs and independent source/recipient target
pairs are not added by this tranche. Cards such as Cover of Winter, Vindicator,
Comeuppance, Channel Harm and Forcefield require their complete distinct semantics.
No new unsupported fallbacks, placeholders or diagnostic suppression were added.

## Combat-boundary follow-up

The shield manager now expires EndOfCombat shields from the shared
`GameState::cleanup_effects_end_of_combat` hook. Ordinary combat completion,
ending combat, ending the turn during combat and phase-skip transitions use that
hook. The skipped-end-combat-step path also closes combat and invokes it before
scheduling the next phase. An additional combat therefore begins without the
previous combat's shields. End-turn cleanup is a backstop for any combat-duration
shield retained outside normal advancement; the live-duration predicate also
rejects a combat shield outside its creation turn's combat phase.

Actual prevented totals survive only when still owned by an unexpired delayed
trigger, matching existing turn-cleanup retention. Pending additional actions
are not discarded. Authored unrun tests cover restored manager state, normal
combat followed by an extra combat, skipping the end-combat step, executing the
real end-combat/end-turn effects and advancing TurnRunner, retained metrics,
the cleanup backstop and direct/restored-artifact duration behavior. The ten
proposed identities and all measured campaign counts remain unchanged.

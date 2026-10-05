# Pack-tactics intervening-if predicates

## Frozen scope

The unsupported intervening-if diagnostic group of 59 cards contains six
pack-tactics cards: Gnoll Hunter, Hobgoblin Captain, Intrepid Outlander, Minion
of the Mighty, Targ Nar, Demon-Fang Gnoll, and Werewolf Pack Leader. Battle Cry
Goblin and Tiger-Tribe Hunter have the same predicate but their apostrophes
split the diagnostic signature into separate groups. This change addresses
that eight-card grammatical family, not all intervening-if failures.

`fixtures/card-failure-campaign/predicates/pack-tactics.json` contains exact
name, Oracle ID, mana cost, type, P/T, and Oracle-text fields from the campaign's
frozen `cards-20261003.json.xz` corpus. No card-name dispatch, diagnostic
suppression, fallback relaxation, or validator change is involved.

## Rules and semantic choice

The official [Adventures in the Forgotten Realms release notes, July 9,
2021](https://magic.wizards.com/en/news/feature/adventures-forgotten-realms-release-notes-2021-07-09),
under **New Ability Word: Pack Tactics**, say:

> Once a pack tactics ability triggers, it doesn't matter what happens to the attacking creatures.

The immediately following ruling specifies declaration-time power. Static
bonuses that apply while attacking contribute; later attack-trigger effects
do not. The preceding ruling also requires the source creature to attack.
Consequently, checking current battlefield power during resolution would be
incorrect, even though the printed ability has an intervening-if clause.

CR 603.4 still requires both trigger-time and resolution-time checks. Both
checks read the historical fact of the declaration. CR 508.4 distinguishes a
creature put onto the battlefield attacking from one declared as an attacker;
the former contributes no attack-declaration event. Rules reference:
[official Comprehensive Rules, June 19,
2026](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf).

## Typed pipeline

- The existing named `combat-turn-predicate` reading delegates the complete
  phrase to a bounded leaf parser. The numeric threshold remains variable.
- `TurnEventPredicateAst::YouAttackedWithTotalPowerAtLeastThisCombat` resolves
  to `Condition::AttackedWithTotalPowerAtLeastThisCombat`. The latter is
  appended to the serialized enum, preserving older variant indices.
- Ordinary intervening-if lowering, trigger matching, and stack resolution
  retain the condition. It is not converted to an effect-only conditional.
- Canonical text renders the historical subject and combat scope explicitly.
- `CreatureAttackedEvent` records its turn-local combat phase. Its existing
  projected history snapshot records power and controller; no parallel
  power-history map is introduced.
- Attack history previously captured raw object P/T. Only attack events now
  use the cached calculated-characteristics snapshot. Production declaration
  finishes marking all attackers and refreshes continuous state first; all
  simultaneous attack records are captured before matching any triggers.
- The history query filters by combat and historical controller, deduplicates
  attacker object IDs, and sums signed power. It neither reads current
  characteristics nor requires the original creature or token to survive.
  Other combats, other players, and creatures entering attacking cannot leak
  into the sum. The condition also requires an active combat phase.

## Regression coverage

`ironsmith-compiler-runtime/tests/pack_tactics.rs` exercises JSON artifact
round-trip and materialization for every frozen source plus renamed variants,
then real declarations at total power five and six, and a six-power attack
without the source. The queued and stacked abilities must retain the typed
intervening-if condition. Further scenarios cover:

- power loss, creature removal, token disappearance, and source departure;
- controller changes after declaration;
- growth and tokens entering attacking after a below-threshold attack;
- negative-power contributors;
- additional combats, player isolation, and turn-history reset;
- ordinary anthems and static bonuses conditional on attacking;
- later attack-trigger counter gains, which must not retroactively trigger;
- a composed historical/current-state intervening-if whose current-state
  clause becomes false before resolution.

The grammar leaf test checks numeric variation and rejects changes to the
player, action tense, characteristic, comparator, time window, and trailing
unparsed text.

## Validation commands and limits

Run serially under the campaign's shared build coordination:

```sh
cargo test -p ironsmith-compiler-runtime --test pack_tactics
cargo test -p ironsmith-compiler-grammar pack_tactics_power_is_a_typed_historical_combat_threshold
cargo test -p ironsmith-tools --test pack_tactics_snapshot
```

At implementation handoff, Cargo validation is pending because the coordinator
reserved the build slot for the optimized full-corpus audit. `rustfmt` on new
Rust files and `git diff --check` pass. Do not infer an eight-card authoritative
audit recovery, complete effect correctness, or a whole-corpus pass from the
existence of these tests; record their actual results and the authoritative
per-card audit separately.

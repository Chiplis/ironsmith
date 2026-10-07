# Combat participant predicates (UNVALIDATED)

Source candidates are the six exact frozen remaining identities in
`fixtures/combat_participant_conditions.json.fixture`: Kazuul, Tyrant of the
Cliffs; Septic Rats; Ever-Watching Threshold; Mirkwood Trapper; Norn's Decree;
Scourge of the Throne. Each fixture retains its complete Oracle body, metadata,
Oracle identity and baseline error/content hash. These are not verified
recoveries or admitted coverage counts. Independent review is still required.

Only source/data reads, source edits and Git operations were used. All tests
below are authored and UNRUN. No builds, compiler probes, tests, formatters,
browser/replay execution or compiled-artifact generation was performed.

## Shared ownership

The grammar's complete combat-participant predicate owner recognizes the whole
condition, including its tense and target category. It rejects additional
unconsumed qualifications. Semantic predicates lower to the appended
`Condition::CombatParticipant` vocabulary. Conditions are not card-name cases,
opaque text, runtime parsing or permissive fallbacks.

The appended `PlayerAttackGrouping::AttackerAnyTarget` represents unqualified
“a player attacks,” including an attack solely against a planeswalker or Battle.
The existing directly-attacked-player groups remain direct-only. One completed
declaration creates one occurrence per declaring player, even when they attack
multiple players or both a player and a planeswalker. Attacking teammates are
separate actors. Creatures merely put onto the battlefield attacking create no
declaration occurrence.

Player-attack events retain an immutable complete declaration: each exact
attacking incarnation, its declaring controller, typed target and target's
defending player. Capture happens after attack costs and surviving declarations
are established, before any attack-trigger publication. Preparation retains
the exact target defender before costs, admitting missing evidence before any
tapping. A target that survives costs is observed at completion; a target that
left uses that target's prepared last-known defender. This adds no new fallible
step after mutation to the ordinary no-cost declaration path.

“They attacked you and/or a planeswalker you control” reads this retained
declaration at both trigger and resolution. Source loss, attacker departure,
planeswalker departure or later control changes cannot alter the earlier fact.
Battles do not satisfy this player-or-planeswalker condition.

“They aren't attacking you” retains the declaring player and checks that
player's current direct attacks. “Players being attacked are poisoned” checks
current directly attacked players and their current poison. Scourge retains
the exact attacking tenure, requires that tenure still to be attacking a player,
and checks current life totals including ties. Neither another creature nor a
later attack by the same ObjectId can substitute for that tenure.

“You're the defending player” uses the existing exact current-or-last defender
role, including planeswalker/Battle defenders. Septic's already shared numeric
poison grammar uses the same retained role; this batch closes the missing
evidence path in external numeric comparison. An inferred live player cannot
repair a missing event role. Incomplete evidence remains a typed error through
negation and discovery rollback.

## Complete bodies

- Kazuul keeps one trigger per attacker, the current-or-last exact defender,
  actual optional `{3}` payment by that creature's current/last-known controller,
  and the real red 3/3 Ogre token when payment is declined. Controller references
  for attacking incarnations now prefer the live object, then its exact
  departure receipt. They cannot follow a blink or substitute the earlier
  declaration snapshot if departure evidence is missing.
- Septic retains infect and the end-of-turn +1/+1 modifier, rechecking poison
  on resolution. Its existing generic poison parsing was a prerequisite, not a
  newly invented second parser.
- Ever-Watching retains the full opponent declaration, tests all that opponent's
  targets, and draws exactly one card for the captured ability controller.
- Mirkwood keeps its first targeted -2/-0 body and its second non-targeted
  attacking-creature choice. The captured attacking player chooses; the
  ability controller is not substituted. The chosen incarnation gets +2/+0
  until end of turn, with native pending-choice rollback.
- Norn keeps combat-damage grouping, the source controller's poison counter,
  and its independent per-attacking-player draw. Current poison can invalidate
  the second ability before resolution. Its first poison program and second
  draw program retain separate participant roles. The consuming damage grammar
  retains a `per_source_controller` cardinality bit: “an opponent controls”
  groups each damage-time opponent, whereas “your opponents control” remains
  one aggregate group. Matching and actor binding read the completed source
  snapshot, never a later controller. Grouped source references and amounts
  merge only within the same actor/recipient key, with checked amount overflow.
- Scourge retains flying, dethrone, the unqualified first-attack history gate,
  current most-life/tie check, untapping all attacking creatures and the
  additional combat phase. Failing the intervening condition on the first
  attack does not permit the ability to trigger on a later attack that turn.

Breena, Suppressor Skyguard and Seraphic Greatsword are explicitly excluded.
Their remaining opponent-life, un-attacked-opponent and Equipment-specific
complete bodies were not broadened into this candidate set.

## Consumers and recovery

`predicate_conditions.rs` lowers the semantic condition, and the existing
`trigger_support.rs` preserves the player-attack grouping. The core typed
condition/grouping travel through ordinary compiled-artifact serde and
materialization; the native model interpreter uses the same condition owner.
Compiled text renders every new condition and the bare player-attack wording.
Layer-dependency and text-change traversals recognize the new condition, and
the runtime contract inventory requires its actual triggering-event evidence.

The native event receipts are `Arc`-retained immutable values inside the
existing event/queue/history owners. `RuntimeSavepoint` captures them through
GameState, pending decision/replay roots and inactive host-lane trigger queues.
No gameplay serializer or public-audit importer is introduced.

Serialized/public compatibility: the appended Condition and grouping variants
extend compiled definitions after the published864 artifact10/public6/protocol23
boundary. A fresh centrally coordinated successor gate and artifact regeneration
are required; this cohort does not inherit864 compatibility. The native event
struct has two new fields but no wire codec or public-audit field was added.
Concrete public and protocol reachability is being assessed by the coordinator;
prior action semantics cannot be presumed replay-compatible. No version
constants, descriptor or campaign ledger were edited here.

The Norn correction also appends default-false `per_source_controller` fields
to `TriggerKind::DealsCombatDamageToPlayer` and the corresponding grouped
`DelayedTriggerSpec`. All direct/delayed lowering and native/model/text-change
bridges preserve the field. The new well-known damage-controller tag carries
the completed source's player, not a choice of a current opponent. These fields
are included in the coordinator's successor-boundary assessment.

## Deferred regression surface

`combat_participant_conditions.rs` compiles each complete fixture directly and
through JSON-restored artifacts, checks rendered semantic markers, then exercises
payment/token/infect/draw/choice/dethrone/extra-combat bodies, multiplayer
grouping, trigger-versus-resolution rechecks, pending native restore and the
failed-first-attack history case. Native condition scenarios cover lost target
and attacker evidence, current attack/poison/life changes, Battle exclusion,
same-object later tenure, branch retention and typed missing-evidence negatives.
`combat_participant_savepoint_tests.rs` covers actual root capture/exchange,
pending replay roots and retained inactive host queues. All remain UNRUN.

Independent-review corrections add six full-body missing-event stack rollback
scenarios, complete token consumption (including mana/symbol tails), a completed
declaration whose attacked permanent disappeared after preparation, exact
attacker departure before stacking, and a missing-before-stacking controller
receipt. Norn's full B/B/C simultaneous-damage scenario retains exactly one
poison trigger per B/C actor across source control changes and native restore;
a paired singular-versus-plural regression protects the grammar distinction.

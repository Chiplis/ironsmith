# Persistent prevention source/recipient relations

Source proposal only. Compilation, builds, parser/engine probes, tests, formatters,
and corpus execution are deferred under the implementation-first campaign gate.
No measured recovery or coverage-ledger claim is made here.

## Frozen complete bodies

The exact corpus rows are retained in `fixtures/persistent_prevention_relations.json.fixture`:

| Card | Oracle identity | Entire body obligations |
| --- | --- | --- |
| Artifact Ward | 9bd3a4bb-cc12-4e5f-a33f-77ab0c7788db | Enchant creature; artifact-creature blocking prohibition; artifact-source damage prevention; ability-only targeting prohibition from artifact sources, including friendly abilities |
| Energy Field | fbf7a7f4-9915-4a53-84b3-e55e9f487323 | Prevention only for its controller from sources they do not control; sacrifice on a card entering that player's graveyard from any zone |
| Energy Storm | 567d3de7-8d56-4b6d-a59a-f8674172f595 | Cumulative upkeep {1}; instant/sorcery spell damage prevention to all recipients; flying creatures of every controller do not untap during their controllers' untap steps |
| Gideon's Intervention | 02fe8ed4-3c5d-4875-b584-b33b819c47be | Actual as-enters name choice; opponents' matching spell prohibition; matching-source damage prevention to controller and controlled permanents |
| Goblin Furrier | 3f086b72-6e7c-40d4-b995-d5f94c494462 | Prevent only its own damage to current snow creatures |
| Indentured Oaf | d82bde29-1576-4736-a755-df9c37765202 | Prevent only its own damage to current red creatures |
| Light of Sanction | c34cf404-729d-46ef-8661-1095e5581766 | Independently match controlled source and controlled creature recipient; any source zone; no player protection |
| Prismatic Ward | ad5e2cb0-00d7-4dad-b20e-23bd41ada398 | Enchant creature; actual as-enters color choice; matching damage to current enchanted creature, using the Aura's choice |
| Wall of Vapor | 7d08d128-863c-4cca-837f-847ff44acef5 | Defender; prevent combat and noncombat damage from current creatures the Wall is blocking, preserving the direction of that relationship |

## Root cause and ownership

The existing permanent prevention production recognized only passive sentences
with an explicit recipient, and its semantic reader accepted only a self
recipient. This increment extends the local syntax production to represent
active voice (source would deal to recipient) and source-only passive voice
(would be dealt by source). An empty recipient in that syntax production means
all damage recipients; the semantic reader explicitly lowers it to any player
or permanent. It never means a missing required operand.

`parse_persistent_filtered_damage_prevention_line` owns non-self source/recipient
relations. Existing self and unqualified attached-object rules remain disjoint.
`parse_permanent_self_damage_prevention_line` uses the same shape and shared
source mapper for the directional blocking relationship. Source-noun scope,
controller exclusion, chosen source color/name, and exact source identity become
existing typed `ObjectFilter` fields. Durations, conditionals, optional or
additional effects cannot be discarded by these full-line readers.

Artifact Ward's separate last sentence uses the new local
`TargetRestrictionEnvelope::AbilitiesFrom` syntax case and lowers to the existing
`Restriction::BeTargetedFrom` with `StackObjectKind::Ability`. The runtime matcher
already distinguishes the targeting action's kind from its source's current
characteristics and exact last-known information. This retains artifact spells'
ability to target and includes abilities controlled by the enchanted creature's
controller.

All prevention lowers to the existing `PreventMatchingDamageSpec` and runtime
`DamageAmountReplacementMatcher` / `PreventDamageByRule` owners. Source and
recipient filters are reevaluated; each event is prevention, so unpreventable
damage bypasses the rule and prevention notices retain their established owner.
Source disappearance requires exact retained evidence. Missing evidence uses the
existing checked failure latch and native effect savepoint rollback.

No runtime/card-name/Debug/display dispatch, runtime data fields, serialized
variants, protocol versions, audit digests, or gameplay restoration paths are
introduced. Artifact7/public digest3/signed protocol20 remain unchanged. Existing
typed artifact codecs carry the full filters. Native savepoints and transcript
replay remain the gameplay recovery owners; an artifact round trip is only a
compiled-definition check.

## Authored validation, not executed

`persistent_prevention_relations.rs` contains whole frozen body compilation
through direct and artifact-materialized routes, strict parse-loss checks,
typed owner assertions and meaningful behavior scenarios. Scenarios include
source/recipient direction and independent controller matching, active source
versus unrelated source, any-zone sources versus instant/sorcery spells on the
stack, repeatable combat/noncombat prevention, unpreventable bypass, current
attachment changes, source and ability loss, phasing, exact source LKI, real
entry choices, name-based casting restriction, Energy Field's complete trigger,
Energy Storm's escalating cumulative upkeep and each player's flying untap
restriction, Artifact Ward's blocking/targeting companions, Wall's defender and
current blocking relation, and native checked rollback before retry with exact
evidence. Pure grammar tests also preserve rejecting unmatched tails and timed
or conditional instructions.

All these scenarios are authored source and have not been run. Independent
source review and the eventual authorized broad validation gate determine
whether each proposed complete body earns verified recovery.

# Activation, combat participants and blind exile compatibility

UNVALIDATED source staging after published PR864. Builds, compilation, tests,
probes, formatting, artifact generation and browser/replay execution are deferred.
Full-body source admission is recorded separately in
`card-failure-stage99-source-admission.md`. The integrated corrections and boundary
have bounded source review; actual runtime verification remains deferred.

This historical published865 successor is compiled artifact **11**, public audit digest **7**, and signed
audit protocol **24**. Manabrew remains **3**. The first current-main recovered-body successor is recorded
in `card-failure-next-series-01-compatibility.md`. These numbers describe separate
surfaces and do not imply executed compatibility or a gameplay serializer.

## Compiled semantics

The exact UTF-8 fingerprint descriptor is
`activation-combat-blind-schema.descriptor`, without a trailing newline. Its
SHA-256 is `cf9f06e2cea9c4facdfe9b4aad19eaa4bca1062e4e9c9f28d22de18920cbd401`,
recorded in `ENGINE_SCHEMA_HASH`; it parents published schema
`e27b521de2a44a2c1c3349a8b8cbf5392da6882c14270112de24faecced0adc9`.
The descriptor is an explicit contract, not generated exhaustive Rust schema.

Preserve published enum positions and use one combined append order:

- ActivatedAbilityKeyword: Equip0, PowerUp1, ClassLevel2, Cycling3, Ninjutsu4,
  Boast5, Exhaust6. ClassLevel retains its u32 level payload.
- ActivatedAbilityCostCondition: TargetsExactly0, EquipAbility1, ThisAbility2,
  All3, Keyword4, NonManaAbility5, LoyaltyAbility6, Activator7.
- Condition retains positions0–201, with CombatParticipant appended at202.
- PlayerAttackGrouping retains Attacker0, Defender1, Pair2 and appends
  AttackerAnyTarget3.

Combat participant conditions distinguish event-time declaration categories and
actors from current life/poison/attacking-state reads. The appended fields
`TriggerKind::DealsCombatDamageToPlayer.per_source_controller` and
`DelayedTriggerSpec::DealsCombatDamageToPlayerOneOrMore.per_source_controller`
default absent input to false, and serialize explicit false/true. Their existing
variant positions do not change. Reserializing an older model adds false and is
not byte-preserving migration; historical signature-only verification preserves
original bytes without this model conversion. The named damage-controller tag
and native grouping key retain actual event actors; the key has no wire codec.
The final body/checked-condition correction is source-reviewed separately from
this field contract; all runtime scenarios remain unrun.

GrantSpec.linked_exile_class_level is optional, defaults to None and is omitted
when absent. Non-Class bytes keep their previous shape. Both materializers retain
nondefault Class metadata and the existing native legacy-marker interpretation;
that internal fallback never permits artifact10 admission under artifact11.
Named JSON variants are the supported artifact contract. Ordinal preservation
avoids repurposing old variants without promising a new binary codec.

Artifact10 is rejected even with a correct refreshed checksum. Relabeled
artifact11 with the old e27 fingerprint is independently rejected. Regenerate
definitions from their source: replacing format/hash/checksum labels cannot
reconstruct missing selected-ability or participant meaning.

## Public typed evidence and commands

RestrictedManaUnit<WireEffect> carries PaymentTransaction predicates, including
ManaPaymentPredicate::ActivatedAbilityKeyword. New keyword variants are directly
publicly reachable. Its on-spend programs can grant typed static/triggered
abilities, exposing the new cost conditions, combat conditions/grouping and
Class/grant metadata even without new outer audit fields.

Blind exile also adds exact public evidence: hiddenIncarnationHighWater, unknown
versus known hidden generations, root/inactive-lane openedExilePlay and
exileFaceDown receipts, their kinds and accepted declaredKind, captured public
source IDs, and blindExileOrigin in retained blind cast-claim digest rows.
Ordinary claim rows retain their old encoding. A missing exact source public
receipt fails export rather than looking up a later incarnation. Native action
authority remains exact ObjectId plus hidden generation; public stable IDs only
name evidence. The Rust inner claim digest is independently normalized, not
repaired by later JavaScript outer-checkpoint normalization.

The new explicit open_exiled_card_for_play and cast_exiled_card_face_down command
refs carry original card, generation and complete permission selection. Tracked
commands require a complete paired hidden identity even when local ObjectId
matches; partial private/public pairs cannot be combined. Raw signed input is
preserved for validation, and index-only opaque commands are refused before
hydration/material release. Accepted declarations survive their owned payment
rollback; unaccepted failed declarations do not enter the accepted prefix.

## Historical evidence and unchanged recovery

Protocol24 is required on transcript and match before current-engine work.
Historical23 is explicitly retained in the signature-only supported set, alongside
previous versions. Original23/6 signed bytes, nested ledger digests and hash
normalization/domain stay intact. No new generation or declaration is invented
for historical commands; old versions are not relabeled or signatures rewritten.
A replay callback requires current versions even with requireEngineReplay=false.
Initial/per-action digest checks reject6, absent and string-valued versions before
mutation. All three live peer routes derive the same current protocol.

Exact native root/inactive-lane savepoints and authenticated full-genesis replay
remain gameplay recovery. Pending captured activation costs, combat receipts,
blind declarations/queues and Manabrew bindings are native Clone/Arc state. Public
audit is redacted evidence with no gameplay importer.

## Manabrew3 and deferred verification

No Manabrew wire shape changes: opaque play is advertised using existing
AvailableActionKind::Cast with cardId, staticAlternative mode and label; the
response remains ChooseActionOutput::Act with actionId. Exact action refs and
paired origins are retained in native prompt bindings, not serialized as new
protocol fields. Response ownership rechecks deciding player, decision hash,
current pair and exact permission before dispatch. Existing strict protocol3
config admission stays unchanged. Actual prompt/answer serde roundtrips precede
the authored response-owner regression; stale pair/reentry cases remain covered.

Authored validation includes old-format/schema refusal, historical23/6 signature
and mutation checks, mixed24/23 replay rejection, per-action digest checks, all
three live peer gates, typed public mana programs, old/new serde variants, exact
opaque pair/material/accepted-prefix paths, real emitted inner claim digests and
native root/lane/branch recovery. All remain unrun. The completed body and combined-owner source reviews are recorded in the
admission note; version changes alone establish no runtime correctness.

After execution is authorized, regenerate real current artifacts/catalogs,
canonical text/checksums, fingerprints, WASM/UI outputs and digest expectations
from the integrated source. The prior missing historical fixtures/v5.json golden
reference remains a provisioning prerequisite. Generate a correctly named current
golden together with its reference; never invent or overwrite historical evidence.

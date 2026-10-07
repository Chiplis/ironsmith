# Second current-main recovery compatibility boundary

Status: **source-authored, UNVALIDATED; every executable scenario UNRUN**.
This successor is **compiled artifact 13 / public digest 9 / signed audit 26**.
**Manabrew 3 and exact local-image format 1 remain unchanged.** The parent is the
published first-series 12/8/25 schema, not one of its unpublished proposals.
No build, compiler/parser/engine/browser/replay probe, test, formatter, corpus
run, code generation, generated fixture or generated asset belongs to this work.
The source/API pass used integrated semantic anchor `3734113a4000b7bbd7f9cea114fa6a4829e2854c`.
The final combined tree still requires independent source review.

## Exact schema contract

`next-series-02-schema.descriptor` is UTF-8 with no trailing newline. Its SHA-256
is `cbaf3a819cee97d5351ddc85c7789a85508caace7573a7f8aa3fe7af0b87854c`.
The declared parent is the exact published descriptor hash
`fbc604c03fa9eed8de6567576052324b9c8bee8d9ddc319f2ae25bc0a62b9c5d`.
The first-series descriptor remains byte-for-byte unchanged and has a separate
authored identity assertion. This is a reviewed semantic source contract, not
an exhaustive generated schema, compiler output, or exact WASM build identity.

The structural additions are:

- `ConditionalEffect<E>.capture_condition_result: bool`, appended and default
  false. Current serialization emits the field, including false.
- `RepeatProcessPromptEffect.decider: Option<PlayerFilter>`, appended and default
  None. Current serialization emits null for None.
- `EffectPredicate::AffectedObjectsShare { required_count, characteristic }`,
  appended at ordinal 14; the prior 0–13 variants retain their positions.
- `AnthemCountExpression::PlayerCounters(PlayerFilter, CounterType)`, appended
  at ordinal 21; the prior 0–20 variants retain their positions.
- `Until::UntilControllersNextUntapStep { object }` at ordinal 16 and
  `Until::PlayersNextUntapStep { player }` at ordinal 17. Prior Until 0–15 remain
  unchanged. These variants have different effect owners and are not synonyms.
- `DelayedTriggerSpec::PlayerAttackDeclaration { attacker, defender, grouping }`
  at ordinal 48. The initial insertion before Attacks was corrected before this
  boundary: Attacks stays 21, PlayerDiscardsCard stays 47, and all prior 0–47
  positions remain unchanged. PlayerAttackGrouping stays 0–3.

Core tests pin each appended position, original tail and representative old
positions, and retain the earlier keyword/cost/Condition/grouping assertions.
They observe Serde variant calls without promising any binary codec. Named JSON
tests retain complete fields and children, absent/default behavior, nondefault
players/counters/objects/groupings, and malformed-field refusal. Decoding an old
model fragment with defaults does not preserve its original bytes and grants no
artifact or signed-replay admission.

## Semantic owners included in the fingerprint

Repeat processing retains each complete initial/additional program, fresh choices,
exact producer collections, original target binding and the complete repeat-once
sequence receipt. Distinct members from one actual result collection satisfy a
sharing predicate; ambient or accumulated collections do not. Captured gates
sample before branch mutations but publish through the original prepared
completion owner. Added programs, participant receipts, pending neutrality,
errors and rollback remain separate. Unsupported simultaneous branches and
concealed captured identity guards fail closed. Explicit prompt actors override
the old implicit iterated-player/controller choice only when present.

Delayed declarations retain all four grouping modes, attacker and defender
filters, complete simultaneous declaration evidence, and registration-local
coalescing. Ordinary abilities are queued before delayed matching; separate
registrations and later combats remain distinct. One-shot consumption follows
grouping. Captured Jaya references name the exact original incarnation rather
than another same-name object, while event-relative defenders bind at the future
declaration. Dalkovan's nested token-sacrifice programs retain their own owners.

Player-counter anthems evaluate current ability-controller-relative live players.
Shared Two-Headed Giant poison pools count once per team; experience remains
individual. Live recipients, source-controller changes, signed multipliers,
caps and checked arithmetic are part of the semantics, not additional wire fields.

Source admission limits next-step durations to represented effect owners: named-player untap
occurrence belongs to the supported CantEffect restriction owner, and exact
controller-beginning expiry belongs to supported continuous-effect owners.
Unsupported source combinations fail closed rather than compiling a forever or
always-active rule. This does not claim an unimplemented validator for arbitrary
handwritten current-schema model fragments. A named player freezes to Specific, while the affected
creature set remains live. The exact-controller variant retains an object
incarnation. Lane-local actual occurrence receipts, original beginning-expiry
IDs, registration cutoffs, skipped steps, optional-choice prefixes and submitted
answers survive retries and native branch/savepoint operations. Restrictions
expire after untap actions, before the separate mana-emptying boundary.

Conditional-damage compilation binds an amount replacement to one preceding
complete damage instruction, keeps its original source/recipient/target assignment,
and selects one existing self-replacement branch. It does not invent cast-time
facts or an event-relative amount. The residual static cohort preserves distinct
ObjectId attack incarnations, exact source-mana-cost current/LKI payment evidence,
and per-ability command/battlefield functional-zone filtering in cost inventories.
These changes warrant audit 26 even when accepted command vocabulary is unchanged.

Native TurnStore/TurnRunner continuation fields are not new artifact fields or a
public gameplay-state format. No generic gameplay importer is introduced.

## Artifact admission and source regeneration

All format-12 artifacts fail validation, JSON admission and actual materialization,
including when their checksums are refreshed. A format-13 envelope carrying the
published `fbc604...` parent schema is also rejected with a refreshed checksum.
The earlier `cf9f06...` published schema and unpublished `9d0e16...` / `2b6fde...`
proposals remain rejected. The old format-11 and earlier rejection cases remain.
The new gate compiles the complete 28 candidate bodies independently through
direct and artifact routes, checks parse loss, validates/roundtrips the current
envelope and exercises actual materialization and direct-definition re-encoding.
This is authored coverage, not 28 measured or validated recoveries.

Regenerate from complete original source and metadata when that execution is
authorized. Checksums authenticate envelope consistency, not a claim that old
semantics were reconstructed. Writing current metadata over old payloads is not
an admitted migration. Loading stored model fragments must not infer dropped
instructions, new chooser ownership, live gates or exact source evidence.

Only `ironsmith-compiled-artifact/fixtures/v3.json` is currently present. The
current golden gate explicitly requires genuinely regenerated `v13.json`; it
does not rewrite an older fixture. The existing original historical `v5.json`
provisioning gate remains. No original generated v12 fixture or historical game
capture is invented. Synthetic envelope and signed-model cases are labeled as
such and do not establish possession of missing historical bytes.

## Real public projection and historical signatures

The actual `sync_restricted_mana` PaymentTransaction/on-spend carrier contains
typed WireEffect programs and granted WireAbility values. It therefore exposes
the new repeat fields and delayed/anthem/duration vocabulary even though the
outer checkpoint fields have not changed. Digest 9 is an independent boundary.
Authored projection tests use fresh native repeat/gate/prompt, supported duration
and granted-anthem owners and roundtrip the real carrier, preserving nested
fields. A materialized delayed schedule projects its retained canonical model.
A genuinely fresh native ScheduleDelayedTriggerEffect has no complete reverse
encoder here and is required to fail projection, never omit or invent fields.

Protocol 25 is explicitly retained in the signature-only supported set with all
previously supported protocols. Original 25/8 and older payloads, nested claims,
canonical normalization, signatures and the `ironsmith-public-audit-checkpoint-v1`
hash domain remain unchanged. Verification does not decode and reserialize model
payloads or insert default fields. New synthetic signed 25/8 tests retain exact
canonical before/after bytes, refuse default injection and relabeling, and never
invoke an engine callback. Existing 24/7 signed cases retain their original
payloads; only current-admission expectations advance.

Engine replay requires numeric 26 on both transcript and match. Protocol 25,
mixed 26/25 pairs, absent and string-valued versions fail before reading engine
state. A callback supplied with `requireEngineReplay=false` still requires current
admission. Initial, per-action and supplied final checkpoint gates require numeric
9; old 8, missing and string versions fail before dependent engine mutation or
callback. All three live peer callbacks inherit current 26 and reject old peers
for match-start, action, resync, recovery and crypto-material routes.

Signed genesis commits protocol and initial public digest, not an exact engine
build identity. Historical genesis signature validity alone never grants current
engine replay. No exact-build check is invented for signed genesis.

## Preserved recovery architecture and required later gates

RuntimeSavepoint owns native game, root and inactive host lanes, runner, pending
prompts, prepared actions/payments, accepted answer prefixes, exact IDs, trigger
history and branch analysis. ReplayCheckpoint retains native game/queue/priority
state but does not own the runner. Existing root/exchange/clone/inactive-lane
combat and newly restored next-step tests remain required, alongside repeat
pending rollback and captured-condition prepared-completion tests. They are all
UNRUN; neither public projection nor model roundtrip validates native recovery.

Trusted same-session `captureLocal`/`restoreLocal` images retain memory, globals,
reference tables, exact root, build/layout and function-table checks under the
foreground worker queue. They keep their deliberate no-integrity-hash analysis
path. Persisted exact-instance images retain integrity, match/seat and accepted
prefix checks. Schema 13 is not a substitute build ID; old-build images cannot be
reused after a genuine build change simply by changing metadata.

Authenticated recovery still verifies accepted genesis, signatures and resync
envelopes, refuses prefix rewinds/forks, optionally restores a verified local
anchor, replays its signed suffix and verifies the signed/public head. Invalid
anchors fall back to accepted-genesis/full replay. A remote transcript never
authorizes installation of a trusted remote analysis image. Required later
scenarios include pending-choice/failed-retry retention, valid local suffix,
corrupt/incompatible anchor fallback, wrong match/seat/prefix refusal and a failed
final signed/public head. No recovery implementation is replaced by this gate.

The checkout still has no genuine generated `web/wasm_demo/pkg/engine.js` supplying
`exactSnapshotBuildId`, `exactSnapshotLayout`, `replaceEngineInstance` and
`attachExactBuildGame`; tracked ironsmith.js/d.ts are not that proof. Matching
custom glue/WASM/layout provenance, current v13 artifacts/catalogs/fingerprints,
public digest expectations, all focused/aggregate suites and authenticated replay
remain deferred execution prerequisites. No substitute output is fabricated.

## Explicit holds

Farmer's disputed controller-binding rule and Savor the Moment's exact extra-turn
owner stay held; fingerprinting shipped Until variants grants neither card credit.
Sin, Spira's Punishment stays partial. Flame Discharge, Surtland Flinger, Slaying
Fire and Summary Judgment remain outside the seven damage candidates. Crown of
Convergence, Mul Daya Channelers, The Ur-Sphinx, Torrent of Lava and Volrath's
Shapeshifter remain outside the three residual statics. Existing source-candidate,
source-review and measured-recovery tiers remain distinct and unchanged.

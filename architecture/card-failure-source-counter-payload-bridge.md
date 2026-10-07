# Source-counter payload bridge: reviewed source proposal, UNVALIDATED

This patch repairs the shared bridge for `RemoveAnyCountersFromSourceEffect`.
It does not claim that any card now compiles or works correctly. All authored
regressions are **UNRUN**. No compiler, parser, runtime, test, formatter,
build, or code generation command was executed for this patch.

## Measurement and cohort ownership

The completed October 7 refresh measured clean main
`5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1`. The original executor recorded
`runtime-bridge-blockers.json` with SHA-256
`84f14e400bb2b1715c0b0dcb54d44b14aaa078eb2615ee69663aa17e54500dac`.
The committed compact [bridge evidence](../reports/card-failure-campaign/refresh-20261007-main5cc46c1/runtime-bridge-blockers.json.gz)
has its own compressed-byte SHA-256
`e7e3930dceaf5f51d2fe74041226ef129b11b40a2b009044f7887b9b18fbdd30`,
pinned by the immutable measurement packet manifest.
The source patch starts at `44e501aad0c0834c6f09e36adcc1c7c5e917f532`,
which includes the separately saved shared-owner corrections.

The measured family has 61 Oracle identities and 62 retained entry names:

- 37 identities fail compiler-to-runtime conversion.
- 24 identities fail authored-definition graph normalization.
- 56 identities were formerly supported and are measured regressions.
- Five identities were original failures with prior-source holds; the fresh
  complete-body dependency review now restores those source proposals.

The five original failures are:

| Card | Oracle ID | Existing source proposal family |
| --- | --- | --- |
| Essence Bottle | `76f839b0-1d4c-4ad9-b44f-a8a13d6d109f` | paid-counter-and-actual-sacrifice-quantities |
| Jar of Eyeballs | `3075dadd-240f-4455-9286-9f1d48f53a3f` | activation-counter-payment-receipts |
| Sorin, Grim Nemesis | `0a01dd05-e289-489f-b3ad-88e15e157bd0` | dynamic-token-quantities |
| The Astonishing Ant-Man | `6c7e4e51-099a-4398-a4b7-e2f0ddd36429` | activation-counter-payment-receipts |
| Vish Kal, Blood Arbiter | `ba3bfe0c-2bd6-469c-802c-440b7caf7fb5` | paid-counter-and-actual-sacrifice-quantities |

The exact source snapshot is checked into
`fixtures/remove_any_source_counter_payloads.json.fixture`. It preserves full
card text, printed metadata, measured names, cohort status, and prior-source
family ownership. Chandra Nalaar's retained duplicate-name entry is exercised
separately without counting it as another Oracle identity. The fixture contains
no admission flags. The separate [combined admission](card-failure-next-series-01-source-admission.md)
and source ledgers record the reviewed tiers. Independent review of isolated
`4e0c792674d9f6498d5b384ad3a339a29cc084f8` cleared the shared repair and found no
remaining concrete dependency in the five prior full-body notes beyond the
corrected owner. The 56 regressions carry **shared blocker repair proposed**;
the five originals carry **restored full-body source proposal**. Neither tier is
a measured compile or runtime recovery.

## Root cause and repair

The compiler already authored the canonical core payload, but the engine
defined an unrelated same-named Rust struct. The shared interpreter lacked
conversion for the core type; the artifact decoder lacked family dispatch,
typed decode, and typed authored-graph remapping; and the native encoder lacked
the corresponding type registration.

The runtime now re-exports the existing core payload. Its constructors and
display method move to core; game-dependent availability stays in the engine.
The interpreter, resource decoder, normalization registry, and native encoder
all use that one canonical type. The disabled monolithic decoder reference
is kept consistent with the active family decoder.

Source, amount, counter-type selection, X announcement, all-counter payment,
replacement handling, choice suspension, and result ownership remain with
the existing counter executor. No card-name conditions or fallback suppression
were introduced.

A separate shared-owner correction preserves the requesting cause in prepared
counter payments. The old capture reconstructed a cost cause from the payment
source and payer, discarding the requesting effect's source, controller and
resolving spell. Capture now uses the existing `payment_event_cause` helper,
with the captured payment reason and cause. Ordinary costs keep that helper's
existing source/payer fallback. The existing scoped original/completion
adapters still restore their caller's cause on success, error, and suspension.

Independent source review also found that the prepared counter-cost projection
reported the nominal cost as its physical count. For captured counter-quantity
owners, the projection now sums the checked original child removal counts,
retaining nominal quantity in the existing requested-amount receipt for X.
Accepted payment owns its successful
status and acceptance fact; physical child prevention/replacement facts stay
in the complete observations and original child packets. Consequently, an
accepted payment whose removal is prevented still pays its cost and permits
an "if you do" follow-up, while "removed this way" reads zero. The zero-quantity
acknowledgement receives the same explicit acceptance fact. Pending, declined,
impossible, and failed paths are not marked accepted. Fallible projection uses
the existing transaction owner, including later completion reprojection.

The two direct callers that previously compared physical count to the price,
loyalty-paid crew and the legacy staged counter activation, now check the
successful acknowledgement and nominal requested amount. These legacy direct
callers retain their existing nominal aggregate result and separate physical
child receipts. Fixed/chosen energy uses the same legacy result contract:
aggregate count, requested quantity, and `ChosenNumber` stay nominal, so
`EffectValue` and typed chosen-number consumers remain consistent. A native,
nonserialized projection flag distinguishes captured counter-quantity owners
from those existing payment owners, through original and completion phases.
Captured fixed-energy proposals also retain the nominal contract. No energy
production code is changed; replacement scenarios cover both direct and
captured energy payment with physical counts retained in child packets.

## Compatibility

No serialized vocabulary or field meaning is added. The existing effect kind
remains `RemoveAnyCountersFromSourceEffect`, with exactly these fields:

```json
{
  "counter_type": null,
  "display_x": false,
  "remove_all": false
}
```

`counter_type` still accepts the existing optional `CounterType` encoding;
`display_x` still owns the existing X behavior; `remove_all` still takes
precedence when selecting all matching counters. The payload kind is derived
from the final type-name component, so re-exporting the core type does not
rename it. Existing `Cost::RemoveAnyCountersFromSource` models are unchanged.
The corrected requesting-cause and physical-payment behavior uses existing
execution-context, event-cause, and receipt fields. Local artifact
format/schema constants are unchanged. This does **not** make cross-version
runtime replay safe: cause/receipt corrections can change execution of old
signed commands. These runtime changes require the parent-owned compatibility
boundary before release, alongside artifact 12 / digest 8 / protocol 25;
old 24/7 commands retain historical signature-only handling rather than replay
under the corrected rules. Ordinary artifact version, schema, and checksum
validation remains mandatory. The combined compatibility source review is now
clear at `a363ea924f6ef98d160060960228688fede0c6a0`, including the canonical
generation registry row, with descriptor SHA-256
`fbc604c03fa9eed8de6567576052324b9c8bee8d9ddc319f2ae25bc0a62b9c5d`.
All generation, codec, native recovery and replay execution gates remain UNRUN.

## Authored verification, not executed

- Decoder tests cover absent/named/built-in counter types, both boolean fields,
  nested cost/WithId normalization, identity remapping, and malformed typed
  payload rejection.
- Payload route tests independently exercise compiler core conversion, JSON
  artifact materialization, and native encoding from an effect with no retained
  serialized model.
- Actual payment tests cover any-number/X/all, selected counter types and
  mixed counters, source isolation, payer choice ownership, ordinary and
  effect-requested causes, full unsigned and zero quantities, prevention versus
  nominal X, unpayable X, wrong-zone source, and suspended choice replay.
- Original receipt assertions keep physical removal outcomes separate from
  nominal payment quantities. A completion seam test retains the captured
  requesting cause and owned output packet through observation, a retained
  boundary, success, error, and pending choice, restoring the caller's scope.
- Actual mixed-counter payment tests commit the first group's physical removal,
  then fail or suspend a later replacement program after it mutates life. They
  require rollback of counters, life, replacement availability, prior receipts,
  trigger/history publication, and caller cause. Modified and replaced groups
  keep physical counts separate from replacement-added life counts; prevented
  all/X costs still accept If-you-do and skip UnlessPays consequences.
- Full-card tests independently invoke direct compilation and artifact
  compilation, round-trip the artifact JSON, materialize it, then encode and
  materialize each resulting native definition. The 56 regression identities
  and five original-failure holds are separate test cohorts.

Before any card can receive verified recovery credit, authorized future
validation must establish that the source builds, these scenarios pass, full
card bodies and all compiler routes remain lossless, and runtime/rules behavior
is correct. The 56 regression IDs receive only the shared-blocker proposal tier,
with no original-majority credit. The five original proposals are restored by
the substantive shared-owner correction, independent review of their prior
whole-body dependencies, and the reviewed combined compatibility gate. Those
source admissions do not establish execution results or remove validation gates.

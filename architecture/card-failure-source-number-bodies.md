# Chosen-number bodies: source candidate, UNVALIDATED

The frozen five candidates are Sanctum Prelate, Scrying Glass, Void, Talion, the
Kindly Lord, and Shapeshifter. All remain `partial_not_counted` pending bounded
independent source review and the separately authorized execution gate. Liquid
Fire is excluded: its announcement-time additional-cost owner is separate.

`fixtures/source_number_bodies.json.fixture` retains each exact corpus record,
including printing ID, oracle ID, full body, mana cost, types, P/T and metadata.
Input is `fixtures/card-failure-campaign/cards-20261003.json.xz`; the decompressed
SHA256 is `9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.
The reference baseline is `e8740178a7f7367ffa3147e7642607042079237c`, in
`baseline-e8740178.snapshot.json.gz`. All five baseline records are parser failures.

## Local choices and exact receipts

Numeric producers retain their authored lower and optional upper bounds. The
positive lower bound of Scrying Glass is explicit. The existing host u32 response
representation is separate from an absent authored upper bound; there is no
silent 100-choice cap.

Local number, color, and hand-reveal producers have independent compiler result
histories. Count comparisons bind to exact producers instead of the most recent
unrelated outcome. A color-count query carries its exact color producer ID and
its exact reveal producer ID. Whole-hand reveal emits `RevealedCards`, including
an empty vector for a completed empty reveal. Missing or ambiguous receipts are
errors. Memory is captured after the public-opening/hydration operation, and
later source departure, card movement, or unrelated choices cannot replace it.

Void's destruction and hand discard refer to the original number producer.
Target player/opponent discovery remains in the ordinary announcement owner;
all-illegal targets stop the complete resolution before any choices. These
contracts have authored whole-body scenarios but have not been executed.

## Persistent number ownership

The source-wide experimental numeric slot has been removed. A source number is
owned by `(host incarnation, immutable executable pair, rules-text acquisition)`.
The pair reuses the existing immutable definition/pair vocabulary. The native
acquisition reuses the existing printed/effect/borrowed/temporary/counter/level
algebra without reading or changing the linked-exile member store.

A typed finalizer stamps the single entry number producer, its explicit readers,
and a compatible upkeep reselection. An upkeep is not linked solely because its
bounds happen to match: an explicit last-chosen-number CDA is required. Runtime
pair identities remain frozen through copy and text rewriting. Card names,
presentation labels, Debug strings, and coverage results never select semantics.

Entry admission resolves the actual prospective ability origin. An ordinary
entry copy uses its new printed acquisition; a duration copy reserves one native
continuous-effect registration before choices. Prospective clones and final
commit use that same registration, so its CDA, restrictions and upkeep read the
number actually selected while entering. Reservations live only in native game
state and restore with failed/pending entry transactions. Trigger and activation
admission retain the actual current ability origin. The separate
numeric owner travels through pending/native contexts, stack entries, copies,
reflexive/delayed lanes, and context savepoints. Source-owned number effects
require that admission before prompting. Ordinary local choices cannot modify a
persistent acquisition. Optional upkeep declines leave the last completed choice
untouched, including after controller changes.

Static cast restrictions retain their exact acquisition. Talion uses one spell
filter with a mana-value OR power OR toughness disjunction and a completed cast
snapshot; matching several axes still fires once. CDA evaluation receives the
originating ability. A copied acquisition starts without its own number even if
the same permanent had another choice; its upkeep can establish a separate
number. Copy expiry restores the original acquisition's retained number.
Preserving copy effects retain equal abilities with distinct native origins,
including multiple simultaneous linked acquisitions on one host. A delayed
matcher retains its admitted numeric host separately from its watched object.
The current exact host wins; after departure its latest true departure receipt
(including leaving the game) wins over an older admitted snapshot. No lookup
follows a blinked or otherwise replaced incarnation.
Completed choices invalidate both characteristic and object-snapshot caches,
including when their noncopiable memory changes without any layer descriptor.

Native snapshots retain an optional acquisition-memory map. `Some(empty)` is
checked never-chosen evidence; `None` is unavailable historical evidence. Public
snapshot serialization deliberately omits both executable acquisition memory and
ability origins. A public or legacy snapshot cannot reconstruct either owner.
Only a genuinely never-made CDA choice receives its authored zero default;
missing history fails. Checked CDA evaluation records a discovery error instead
of publishing a provisional zero or panicking on missing numeric evidence.

## Canonical public proof and recovery

Public audit v4 is staged separately from the source implementation. It records:

- Completed choice groups, ordered by per-source public group ordinal, with the
  immutable definition/pair and last chosen value
- Current numeric ability-slot bindings to a chosen group, or explicit no-choice

The first completed choice for a native acquisition assigns its public ordinal.
Reselection keeps it; a new copied acquisition receives a new group. Dormant copy
choices remain in the group list, so expiry can restore their original binding.
Pending choices and failed transactions cannot consume ordinals. No CardId,
static-instance ID, native effect-registration ID, or executable acquisition is
included in the proof. Equivalent gameplay choices with different internal
allocation orders must produce equal proofs. Hidden identities suppress both
groups and definition/binding metadata. Public proof rejects source-owned entry,
triggered and activated producers missing their program pair, including omitted
legacy fields; a missing producer slot cannot silently disappear from proof. These are audit records, never a recipe
for restoring gameplay owners.

The published coordinator boundary is artifact7 / public digest3 / signed
protocol20. This source branch began before that boundary. New numeric model and
public-proof bytes must join the text correction in one later coordinated gate;
this branch does not independently set or publish that successor. Historical
signature verification remains distinct from current executable replay. Native
root/inactive-lane savepoints and verified transcript replay remain the recovery
owners; no serialized gameplay checkpoint is introduced.

## Authored verification, never run

Independent direct compilation and JSON artifact materialization use every full
frozen body. Scenarios cover zero/large endpoints, positive-only choices, actual
announcement targets, all-illegal resolution, cost/tap payment, destruction and
hand discard, exact color/multicolor count, source removal, optional upkeep and
controller changes, token copies, same-definition acquired copies and expiry,
completed cast snapshots and one OR trigger, selected split faces and payable X,
missing evidence, rollback, and native/model codecs.

Native receipt tests vary unrelated later number/color/reveal outcomes and
source/card departures. Acquisition tests distinguish current versus dormant
choices, legacy/public snapshots versus native history, pending/failed group
allocation (including failure after a successful allocation), native restore, and different internal allocation order with equal
public proof. The copy regression is an ordinary acceptance test, not ignored.

No build, compilation, formatter, test, compiler probe, engine/corpus replay, or
browser execution has run for these increments. No coverage ledger changed and
no card has received a full-card completion proposal.

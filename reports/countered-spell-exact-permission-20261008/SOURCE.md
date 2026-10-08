# Exact counter-to-exile priced permission: source implementation

Base: `041e1d0b1af01ad82612359d55e8614a97756926`.
Branch: `repair/exact-counter-exile-grants-20261008`.
Scope: Spelljack `7687b2a7-816d-4416-979b-675e35e235fc`, Thranduil's Decree `d7cba934-02ad-4677-bb4d-50808b01b4f9`, Kheru Spellsnatcher `c01411e0-77b2-4e65-a369-5dbe13745769`.

**Source-authored only. Every build, parser/lowering test, artifact test, native/runtime test, corpus measurement, generator, formatter and release/recovery gate is UNRUN. Measured recoveries: 0. The three IDs remain admission-gated; this is not a supported-count increase or executable/release clearance.**

The exact original complete metadata and raw/normalized bodies remain in `../countered-spell-durable-permission-20261008/frozen-inputs.json`; tests consume that committed file rather than reconstructed names, costs, types or truncated bodies. The earlier HOLD remains historical diagnosis, not a statement that this patch was executed.

## Ownership

`CounterEffect` now has an optional typed `CounterExilePermission` containing a required `CounterExileGate::{AnySpell,PermanentSpell}` and required play-versus-cast boolean. The rest of this narrowly scoped owner's meaning is intrinsic: effect controller, free mana price, ordinary casting/land timing, exact committed counter arrival in Exile, and lifetime only while that same object incarnation remains there. There is no separable free-cost consumer, tag, stable-card fallback or globally sourced exile collection.

The complete three-sentence grammar production consumes counter, fully specified destination replacement, and free-cost/while-exiled permission together. It does not add generic cast/play verbs or globally enable the broken tagged lifetime-tail composition. Related malformed full programs fail closed instead of reaching the lossy broad destination reader. Statement partitioning and the correlated turned-face-up trigger path preserve the complete body. Ordinary counters retain their existing constructor/lowering path. Kheru retains its complete morph ability and trigger.

Lowering emits one counter effect, with the initial unrestricted spell target. `PermanentSpell` gates only the self-replacement against calculated **pre-move stack characteristics** captured through the checked snapshot API (failure propagates through rollback rather than panicking), not a battlefield-only permanent selector and not the legal target. A countered instant or sorcery under that gate uses its ordinary destination and receives no permission even if another effect independently exiles it.

The native owner adds a temporary self-replacement only to this counter's zone-change operation. Only `receipt.original = Proceed(change)` with final Exile and its exact `new_object_id` can install a grant. Protected/invalid/prevented actions, other final destinations, and replacement-owned `Replaced` bodies without that original receipt do not invent a grant from global movement results. The grant is installed inside the existing counter transaction before deferred receipt additions; pending/error rollback restores the counter and grants. A later addition moving the exile object away invalidates the original ID. Existing published counter/movement receipts and source-exile links remain intact.

The registered free-price AlternativeCast and, for play only, land-filtered PlayFrom both use exact object ID, Exile, effect controller and existing source-independent effect lifetime. No ordinary-price spell PlayFrom is added. Neither grant binds stable ID, so the existing Adventure stable-grant exception cannot revive this price after exile-to-stack-to-exile. Ordinary zone departure already revoked ordinary stable grants before this repair; no generic reentry defect is claimed and that cleanup is unchanged.

Rider target admission is checked in core, compiler-model interpretation, artifact decode/card-graph traversal and native execution: one explicit stack spell or exact native object ID, not plural/tagged/source-pool/ability selectors. Presentation-only target hints are retained. All ordinary CounterEffect constructors default to absent rider; source search found no external production CounterEffect struct literals requiring a new initializer.

## Priced-path prerequisites

Source tracing identified three concrete prerequisites in the shared existing zero-component `FromZone { zone: Exile, ... }` shape:

1. Other-face spell actions had depended on an ordinary PlayFrom permission, absent for an exclusively priced grant. A dedicated proposed-face alternative route now considers eligible free-exile grants without opening an ordinary-price route.
2. The old menu path treated an absent mana component as skipping the affordability block, hiding mandatory additional mana and taxes. The free-exile path now uses the complete selected casting-method price calculation, including mandatory costs.
3. The ordinary priority path did not apply the printed-X zero rule to a selected empty FromZone price. The same free-exile shape now fixes **printed mana-cost X** to zero while preserving independently announced X in an additional cost when printed mana cost has no X.

These changes are restricted to the identical free-exile semantic shape, which can have other legitimate producers. Existing FromZone carries no owner-provenance discriminator; card names or display labels are not used as one. Paid/nonzero/X-price alternatives, other zones, ordinary Adventure permissions, and generic zone-lifetime cleanup are not redefined. Authored controls cover paid/X-price paths, mandatory mana and life payment, eligible other faces, competing alternatives, ordinary timing, land-face domain, and real Adventure resolution followed by a new normally priced permission.

## Compatibility and admission

- Native/core: `CounterEffect` grows an optional typed field; clone/PartialEq/TagKeyWalk and compiler-model conversion carry it. Existing native transaction savepoints clone the whole game/context. New grant storage reuses existing grant representations.
- Wire: existing CounterEffect typed serde/materializer route carries the nested rider. Absence defaults to None and is omitted again; required nested fields reject a dropped/malformed gate and unknown nested fields. This preserves legacy ordinary payload shape, **not** semantic boundary compatibility for a new binary or cache.
- Integrity: a stale-checksum rider deletion is rejected. An attacker/editor that deletes the entire optional rider and recomputes the checksum has created a syntactically valid legacy plain counter; without source-semantic provenance no decoder can distinguish it from that legitimate old payload. Full frozen-input typed/canonical comparisons and later boundary admission must detect such semantic loss. The atomic representation makes a dangling separate consumer unrepresentable.
- Codec generation: the source generator must retain the validated CounterEffect path; no generator was run. Inherited broader generator/facade/card-graph drift is a separate genuine prerequisite before any full regeneration. See the artifact impact report for exact paths.
- Cache/build/image: no cached definitions, generated catalogs, WASM/glue, fingerprints, artifacts or executable outputs were regenerated. A later authorized boundary must invalidate old semantic caches and build genuine final-source artifacts together. A serde default is not evidence for accepting old generated meaning.
- Public/audit/recovery: canonical counter text changes and `wasm_game_impl/public_audit.rs` projects nested effects via `encode_runtime_effect`/WireEffect in restricted-mana restrictions. Therefore preserving public digest10 by convenience is not justified. The later independent boundary must review public digest, signed audit, artifact schema/cache and recovery projections together, including exact-build/snapshot/continuation cases.
- Inherited artifact16/audit30/digest10 declarations, descriptors, goldens and web audit gates are untouched in this source component. They are not silently reinterpreted as admitting this new meaning.

## Authored evidence (all UNRUN)

- Grammar: complete typed owner with gate/actor/play-cast assertions, malformed/missing/wrong-gate/suffix/producer/copy/pile/global-exile controls and unrelated-family controls.
- Lowering: all three complete frozen metadata/raw and normalized inputs, strict non-lossy compilation, unrestricted spell targets and single atomic owner, Kheru's full morph/3-3/face-up trigger, renamed-card control, ordinary counter control.
- Native counter: successful exact receipt, old source-linked exiles and stale tags ignored, opponent ownership, source departure/later turn and no off-turn creature casting; permanent and nonpermanent gates including a face-down permanent spell that reveals an instant in exile; uncounterable/absent target; prevented/redirected destination; ordinary departure/reentry and stale arrival; independent Adventure stable-grant control; receipt addition departure/reentry; error/pending rollback/replay; malformed plural/tagged native targets.
- Artifact: grammar-independent native/wire/materialized equality and runtime matrix; required nested gate/target rejection and card-ID graph mapping; legacy absent rider; original integrity rejection without fallback; native clone/lifetime and actual zero-mana cast with mandatory payment. Synthetic direct definitions establish no Oracle recovery credit.
- Price/payment: direct priority-action availability and actual casts/payments for free and paid controls, other faces, land play versus cast, additional costs, X, source departure and real Adventure continuation.
- Full Kheru runtime: separate frozen-input face-down/morph/trigger witness, authored without execution.

No authored assertion is reported as passing. Independent source review plus authorized focused direct/artifact/native tests must precede a new exact-ID corpus measurement. Every original recovery and aggregate supported-count claim remains unchanged.

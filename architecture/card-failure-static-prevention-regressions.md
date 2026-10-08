# Current-main static prevention ownership regressions (UNVALIDATED)

This packet starts at `44e501aad0c0834c6f09e36adcc1c7c5e917f532`, the current
campaign integration checkpoint over main `5cc46c1`. It is **source-only**:
no build, test, compiler, parser, engine, browser, formatter, or code generation
was run. The one-time current-main execution allowance was already consumed by
the parent audit. Measured recovery counts and catalogs remain unchanged.
Fresh whole-body/source review cleared isolated
`babecaff62cabb50bf311d3536b7c17a706d99a8` plus
`3f02ae226b65099fcdf86eac95c9ea5c4f3b1ae7`, integrated through
`c06bf3f4564822ba20e5e3b721d1fac1bb812d8e`. The [combined admission](card-failure-next-series-01-source-admission.md)
records eleven regression full-body source proposals and five restored original
Aura proposals after the independent compatibility review cleared. The eleven
regressions add no original-majority credit; all scenarios remain UNRUN.

## Exact bounded inputs

The first subset is the join of `regressed` in the parent audit's
`reports/current-refresh-20261007/original-identity-comparison.json` with
`current-failures.json`: eleven formerly supported identities whose current
diagnostic routes end in `could not find verb in effect clause` at the fixed
`prevent N of that damage` tail. The retained compact evidence is
[baseline-and-proposal-comparison.json.gz](../reports/card-failure-campaign/refresh-20261007-main5cc46c1/baseline-and-proposal-comparison.json.gz)
(`changed_or_unresolved_identity_outcomes.regressed`) joined by Oracle ID with
[current-failures.json.gz](../reports/card-failure-campaign/refresh-20261007-main5cc46c1/current-failures.json.gz).
Their source/loader records were unchanged in that audit. A second explicit
subset contains five earlier Aura source holds whose diagnostics directly report
overlapping, non-equivalent registry rules.

`fixtures/static_prevention_regressions.json.fixture` contains each complete
Oracle body and its compiler-facing header fields from the pinned original
`fixtures/card-failure-campaign/cards-20261003.json.xz`. The compressed input's
SHA-256 is `a38a53fb6da122f0f7240800d203386fcaa437190414221476719cb46086e1f0`.
Read-only JSON analysis checked all sixteen Oracle bodies against the fresh
audit's raw Oracle text. No filtered or shortened card body is substituted.

Fixed-to-you regressions:

| Oracle ID | Card |
| --- | --- |
| fe9106e3-c637-4995-9dc1-cae0be31f23b | Guardian Seraph |
| 20418984-bc6d-42e5-ada6-30b5281925c0 | Heart-Shaped Herb |
| c420d9c8-dc54-46b8-bd22-39c21019c3d7 | Orbs of Warding |
| c5f611c9-9e6e-4719-ad71-8686327a8564 | Protection of the Hekma |
| c1612eef-5059-46fd-a5fd-c8ebf6779a6e | Sphere of Duty |
| e45de1f7-ad45-494d-855e-3e11f109909d | Sphere of Grace |
| a8da7c57-c60f-42b1-b611-64a0a43295f2 | Sphere of Law |
| 790d3ee2-8b93-464c-9b16-afe62db0048c | Sphere of Purity |
| de85c961-622d-4d70-8cd7-f722c1ac5ed1 | Sphere of Reason |
| e6325604-cdce-4872-af03-e436631004de | Sphere of Truth |
| e5fa232a-c8f2-4532-88bf-60d1223181b0 | Urza's Armor |

Attached-source holds:

| Oracle ID | Card |
| --- | --- |
| 089788ce-d06c-4a85-915c-82e2cb5b0103 | Candletrap |
| 2172b724-9004-488e-88d3-a5fc48c50e41 | Demonic Torment |
| 5a6c81b8-71f0-468b-85ad-d87e9a712ecf | Defang |
| bdf497b5-166f-46a7-8e18-ae0b8f97768c | Muzzle |
| cb5ab978-d9b1-4f2f-bf3a-5e28e96b9e2e | Temporal Isolation |

These are proposed source repairs, not sixteen measured recoveries. The unrelated
temporary-prevention packet is not copied or included in this membership.

## Shared root cause and canonical ownership

Both `parse_filtered_damage_prevention_line` and the established
`parse_prevent_damage_to_you_from_source_filter_line` recognize complete fixed
reductions to you, but produce different static payloads. The static registry
correctly rejects their non-equivalent ASTs. For an effect-statement head such as
`If`, `document_parser/line_recognition.rs::recognize_static_line` declines a
static error and continues to statement recognition, hiding the registry cause
behind the eventual missing-verb error. Adding `prevent` to the ordinary verb
fallback would not repair that ownership problem.

The general reader now declines only a successful complete fixed-to-you
specialist parse. This preserves the existing
`PreventDamageToYouFromSourceFilter { amount, source_filter, display }` payload,
its exact amount and source/controller filters, and its true
`PreventDamageAmount` runtime action. Combat/noncombat, threshold, different
recipient, all-but, and richer source-filter productions retain the general
reader. This is an explicit grammar-domain partition, not a registry first-match
override or a diagnostic suppression.

The Aura overlap was between the persistent source/recipient relation reader and
the complete attached `dealt by` specialists. Merely excluding one reading would
retain a semantic defect in the old specialist: an `AttachedStaticAbilityGrant`
places the prevention on the creature and attributes its source/controller to
that creature. It also makes losing the creature's abilities disable a rule that
belongs to the Aura. An unquoted `Prevent ... by enchanted creature` instruction
does not grant an ability.

Both complete attached-source specialists now reuse the exact canonical
`PreventMatchingDamage` payload from the persistent reader. Registry alternatives
therefore agree structurally, preserving the ambiguity check. The Aura owns the
prevention; the source filter selects its live enchanted creature. All recipients
remain covered, the two combat-only cards stay combat-only, and real prevention
still respects unpreventable damage and emits the actual prevented amount.
The existing conditional `Otherwise, prevent ... by enchanted creature` caller
wraps the canonical ability in its same typed negated condition. Genuine quoted
ability grants retain recipient ownership.

`DamageAmountReplacementMatcher::source_matches` now checks a source-relative
attachment against the prevention source's current attachment before reading
source characteristics or LKI. A historical creature snapshot may still contain
an Aura that has moved. That snapshot cannot reestablish an obsolete attachment,
select a new incarnation, or turn a detached Aura into an active rule. Remaining
source characteristics still use the shared live-or-exact-LKI matcher. This
check applies wherever that shared matcher receives a top-level positive
`Enchanted`/`Equipped` identity-tag constraint or a `with_attached_object` filter
requiring the replacement source; other source filters retain their existing
behavior. The five singular Aura source phrases use the former canonical
`Enchanted`/`IsTaggedObject` representation, not the inverse attachment field.
The live owner must be on the battlefield and not phased out; its current
attachment is authoritative independently of its Aura/Equipment subtype.
This early check also rejects an absent former host before missing source LKI
can be treated as incomplete evidence.

Unqualified `dealt to` and combined `dealt to and dealt by` attached productions
are outside this bounded packet. No new prevention mechanic or broad fallback is
introduced, and no additional candidate is credited for the conditional helper.

## Complete-body inspection and authored scenarios

No further reader gap was identified by source inspection of these sixteen
bodies. That is not executed evidence of full correctness. Every body remains
behind strict compilation and runtime validation, including the following
secondary clauses:

- Guardian Seraph's flying and Orbs of Warding's player hexproof.
- Heart-Shaped Herb's mana/tap/self-sacrifice cost, optional creature sacrifice,
  `if you do` branch, return of that exact card under its owner's control with
  three +1/+1 counters, and the activating player becoming monarch. Existing
  return-with-counters and conditional program owners are retained; authored
  scenarios distinguish accepting and declining, with an opponent-owned creature
  temporarily controlled by the activating player.
- Candletrap's enchant restriction, defender grant, and coven activation. Existing
  distinct-power predicates and the exact departed source's attachment receipt
  remain the owners. Its authored scenario rejects three creatures with only two
  distinct powers, activates after adding the third power, sacrifices the Aura,
  removes that third power before resolution, and still exiles the originally
  enchanted creature. This checks activation-only timing and departed Aura scope.
- Demonic Torment's attack restriction and Temporal Isolation's flash and shadow.
- Defang and Muzzle's complete enchant restriction.

`static_prevention_regressions.rs` authors every full body through four routes:
strict `compile_to_runtime_definition`, strict artifact compilation's runtime
definition, validated artifact JSON decode/materialization, and fresh runtime
encoding into native wire JSON followed by materialization. The scenarios reject
lossy parsing and unimplemented content, compare full rendered bodies across
routes, inspect the typed prevention payloads, and retain secondary clauses.

Authored live scenarios cover all eleven fixed reductions; player versus object
recipients; source type, color and controller; prevention ability controller
changes; both damage kinds; amounts below and above the reduction; actual
prevention events; unpreventable damage; live properties overriding stale LKI;
exact departed source snapshots; and ability source departure.

All five Auras have authored source/controller attribution, combat versus
noncombat, all-recipient coverage, unpreventable damage, reverse-direction
nonmatches, creature ability loss, Aura ability loss, retargeting, creature blink,
and stale snapshot scenarios. Additional cases cover direct and indirect phasing,
detachment, attachment-owner subtype changes, departed non-host damage with and
without stale LKI, Aura departure/return, genuine granted prevention, and the conditional
`Otherwise` caller. Grammar scenarios compare complete specialist/general ASTs
through the registry and reject truncated, conditional, temporal, and effect tails.

Deferred commands, **not executed**:

- `cargo test -p ironsmith-compiler-grammar prevention`
- `cargo test -p ironsmith-compiler-runtime --test static_prevention_regressions`
- The supported-card regression gate and the eventual authorized corpus audit.

## Compatibility and integration boundary

There are no new model fields, enum variants, wire layouts, or version edits.
The eleven fixed-to-you cards retain their established canonical payloads.
Newly compiled attached-source prevention changes from recipient-owned
`AttachedStaticAbilityGrant` to Aura-owned `PreventMatchingDamage`, including the
typed conditional caller. Existing old grant artifacts are not silently rewritten
by this source patch. The shared live attachment/LKI gate is also an observable
runtime semantics correction, including source/controller attribution and
ability-loss behavior.

This packet now joins the independently reviewed **format 12 / public digest 8 /
audit protocol 25** integration boundary, with Manabrew3 unchanged, alongside the
counter cause/receipt correction and the 27 restored copular, destination-reference
and temporary-prevention full bodies. The final combined source anchor is
`a363ea924f6ef98d160060960228688fede0c6a0`; descriptor SHA-256 is
`fbc604c03fa9eed8de6567576052324b9c8bee8d9ddc319f2ae25bc0a62b9c5d`.
Catalog regeneration remains deferred. The absence of new wire fields does not
make the semantic change backward equivalent. Only after execution is authorized,
rebuild the affected catalog from full original inputs and run the direct,
artifact and native assertions against that same engine build before claiming
recoveries.

The current signed replay format has no exact engine-build gate. Old protocol
24 / semantics 7 history must remain **signature-only**; it must not be
reinterpreted under these changed runtime semantics or claimed as a replay match.
Historical signed bytes and measured-main counts remain unchanged. Source
admission records the reviewed semantics and exact identities only; generation,
original-v5/current-v12, custom WASM/glue/layout, native recovery and replay gates
remain explicit and UNRUN.

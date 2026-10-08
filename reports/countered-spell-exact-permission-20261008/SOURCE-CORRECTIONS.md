# Corrections following independent review of d0c214f6

Original component preserved: `d0c214f6de9d8256627804e10812dd698cff7d4f`.
The corrections below are source-authored follow-up work, not execution results.
All builds, tests, type checks, probes, corpus measurement, formatters and code
or artifact generation remain **UNRUN**. Three-card recovery credit remains 0.
Independent re-review and later admission are required.

## R1: forced-zero X must satisfy the authored minimum

`decision/mana.rs` now checks an empty Exile FromZone price's printed X against
stack-functional `ThisSpellXMinimum` abilities in the selected-face stack
payment proposal. A contradictory positive minimum removes the menu route;
unknown/unresolvable minimum evidence is propagated through the existing failed
calculation mechanism rather than accepted as zero.

`game_loop/priority_cast.rs` checks the same condition at the forced-zero phase.
At that point proposal may already have moved the card and consumed a shared
permission. Contradiction or calculation failure explicitly rolls back the
pre-cast action transaction before returning an error. It does not choose a
positive X, clamp the bound, pay costs, or preserve a partially spent permission.
The authoritative check also repairs the existing effect-waived-base branch of
the shared forced-zero path; its menu addition remains scoped to empty Exile
FromZone prices. Paid/nonzero/X-bearing alternatives keep their existing paths.

Authored UNRUN controls include minimum-positive versus minimum-zero, forged and
stale actions, direct post-proposal rejection, shared-budget/resource/identity
restoration, selected-face/source-relative minimum values, the older
resolution-effect waiver, additional-only X and paid/X-bearing alternatives.

## R2: admit a complete target vocabulary

The core rider predicate now accepts only the complete canonical
`ObjectFilter::spell()` (optionally as a single Target) or a literal native
SpecificObject. Presentation-only SurfaceHinted wrappers remain permitted.
This deliberately narrower vocabulary makes source flags, nested any_of/tag
constraints, contextual collections and future extra filter predicates fail
closed instead of trying to maintain a partial recursive blacklist. All three
printed bodies author this unrestricted spell target. The permanent gate
continues to qualify the replacement, not the target.

The same predicate was already reached by direct compiler-model interpretation,
artifact decoding, card-graph walks and native execution. New controls exercise
source-qualified and nested-tag filters in all those routes, with a genuinely
live source spell/native stale-tag setup, legacy None controls and a
presentation-only-hint control. No generator file was changed in this follow-up.

## Missing final permission / direct-versus-artifact evidence

Deleting Thranduil's entire final sentence now commits to an incomplete typed
permanent-gated program and fails closed. The legacy marker recognizer also
independently declines that permanent-only destination gate. Ordinary
unconditional two-sentence counter/exile forms retain their old owner. The
actual frozen Thranduil metadata/body deletion is an authored negative.

The original Kheru test incorrectly called the second result of
`compile_to_artifact` a native direct route: that function already materializes
its artifact. The corrected witness independently invokes genuine direct
compilation/conversion and, separately, artifact generation, serialization,
deserialization and materialization. No previous direct-route execution is
claimed; neither the original nor corrected tests was run.

New complete frozen Spelljack and Thranduil tests drive actual casting of the
counter card and target, resolution, source departure, appropriate free recast,
mandatory mana/life costs, ordinary timing, owner/controller distinctions and
permanent versus nonpermanent destinations. Modal land-face controls allow
Spelljack's play permission and reject Decree's cast-only land action. Full
three-card artifact mutations cover malformed nested fields/targets, stale
checksums, and well-typed rechecksummed changes to gate, play domain or entire
rider. The latter are compared with the independently compiled source model;
they are **not** falsely described as structurally invalid legacy payloads.

The official [The Hobbit release notes](https://magic.wizards.com/en/news/feature/the-hobbit-release-notes)
(Thranduil's Decree section, verified by the coordinator) support normal timing,
printed X=0, required additional costs, optional additional costs, and no
combination with another alternative cost such as morph. Full-card controls now
reject morph, an intrinsic alternative and ordinary-price origins while
preserving the legal face-up free cast. Kheru's initial ordinary face-down
casting remains a separate valid path.

## Receipt edges: provenance evidence, not an invented rules conclusion

1. A nonpermanent spell under Decree's gate can be independently redirected to
   exile by another destination replacement. Its original counter succeeds,
   but the gate did not qualify; no Decree price is installed. The witness
   asserts the counter event and actual exile destination separately.
2. A synthetic `ReplacementAction::Instead` program exiles an unrelated card
   and the original target through separate movement instructions. Source
   trace: `events/processing/application.rs` maps Instead to Replaced;
   `prepare_zone_change_with_context_inner` in `events/processing/mod.rs`
   executes that payload during preparation and returns EventOutcome::Replaced.
   The original counter has no Proceed/new-object receipt to bind. The witness
   checks both actual exiles, absence of a committed counter event in this
   constructed case, and no grant for either object. It does not treat last
   movement results, shared source links or the separate exile actions as a
   successful original counter arrival.
3. A nonconverging static-ability fixture proves checked characteristic capture
   returns ContinuousDiscovery failure. Direct counter execution must report
   the typed error, restore the original stack/object/provenance/ID/zone and
   resource state, and publish no counter result or price. The test is authored
   from the existing bounded-discovery fixture pattern; it has not been run.

The release notes do not settle unusual replacement-owned-exile rules cases.
These tests establish the intended native receipt/provenance boundary for the
synthetic instructions, not a general external rules assertion that every
replacement-owned exile must lose a permission. A broader valid replacement
family without a successful original receipt requires an explicit proven
semantic bridge; none was invented here.

## Boundaries and remaining work

Original complete metadata, inherited artifact16/audit30/digest10 evidence and
all historical counts remain untouched. Generator reconciliation (`45bce2d`,
coordinator-owned) and the new semantic/public/signed boundary are separate
components and were not edited or assumed integrated here. The original
ARTIFACT-IMPACT report records the earlier generator hold; this follow-up does
not re-run or claim generated parity for that subsequently separate repair.

Re-review this exact follow-up source before admission. All runtime and
recovery claims remain unestablished until authorized focused and aggregate
execution on the final integrated SHA. Well-typed rechecksummed whole-rider
loss still requires source-semantic provenance comparison, not envelope
integrity alone.

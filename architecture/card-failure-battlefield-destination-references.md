# Battlefield destination and source references (source proposal, UNRUN)

This family is based on published commit
`f6876fd284afbdef37d16db37d5c73b182194426`. The eight exact records in
`fixtures/battlefield_destination_references.json.fixture` come from the pinned
`cards-20261003.json.xz`, and are all unaddressed identities in the frozen
source-coverage inventory. Their original diagnostic bodies are retained in
`baseline-e8740178.snapshot.json.gz`. No campaign ledger is changed here.

## Recovery provenance (2026-10-07)

The original unpublished source head was
`6a8700d171b0fa3bfb251bf61b034b8fc602523a`. After the execution filesystem was
replaced, its exact authored patches and review corrections were reconstructed
on published PR 865 commit `489e6561d8b90ca088f2ba2ee481fec875cc3a46`, without
overwriting current owner files. This does not assert that the old Git objects
survived or that their bytes could be compared with the reconstructed commits.
The recovered 18-path source inventory totals 888 insertions and 21 deletions,
matching the retained final patch inventory before this provenance addition.
All eight fixture records were compared again with the restored frozen corpus;
every recorded field matches. A fresh independent source review is required
on the reconstructed base. All executable validation remains UNRUN.

## Latest-main reconstruction

The corrected destination-only source through
`dcb023bda12de0326a7ebd85aff67f097c99377b` was reconstructed again on latest main
`5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1`. The user changes already present on
that main commit are retained. Scoped Git comparison found no post-865 changes
in the existing destination-family paths; no production adaptation was needed.

The 18 reconstructed file blobs match the retained destination manifest, with
the corrected `part_2.rs` and full-body scenario file matching their final
stage-100 manifest hashes. Two files in the combined stage-100 manifest also
contain other unpublished family changes: `subject_verb_early.rs` and
`ast/queries.rs`. Their destination-only versions deliberately retain the
original destination manifest hashes, rather than importing those other
changes. This note is the sole documentation adaptation after blob comparison.

The corrected guards reject unsupported per-card-owner partitions, all/each
attacking or attachment destinations, and non-word residue in relative tagged
source references. All five partial bodies remain outside the complete-candidate
set. Fresh independent review of this latest-main reconstruction is required;
no compile, test, gameplay, schema, version or coverage result is inferred from
the prior source reviews.

The latest-main source review additionally found raw source-reference and
collection-controller bypasses. Fixed Source/Tagged references, including
source-zone filters, now validate their complete raw tokens in both newly
admitted destination routes. Ordinary object-filter numeric/mana grammar is
unchanged. Looked and exiled collection readers reject contextual controller
suffixes at their full-clause boundaries and shared shape reader, rather than
falling back to the default controller. The bounded frozen card/face inventory
found only the already-partial Dubious Challenge containing both `from among`
and the contextual controller spellings; no previously strict-compiled or
source-admitted frozen identity matched. Additional direct/artifact and shape
negative scenarios are authored and UNRUN. These are reviewed-source follow-up
changes to the reconstructed blob comparison above, not extra body admissions.

## Complete-candidate scope

- **Charmed Griffin:** the complete Flying/entry body. A bounded source
  location after the battlefield destination is parsed as the same source
  filter as a location before it. Other players retain their own optional
  artifact-or-enchantment selection from their own hands.
- **Trove Warden:** Vigilance, the entire landfall exile ability, and the entire
  death ability. “Under the control of that card's owner” is a per-object owner
  reference. It uses the existing linked-exile identity and batched owner-entry
  path; it is not the source's owner or last controller.
- **Endless Whispers:** the complete granted ability, including death,
  selection of an opponent of the dying creature's controller, the source's
  own graveyard, and the next end step. The delayed trigger retains the exact
  death-arrival incarnation and selected player. A card moved independently
  after registration cannot be found again by its stable identity.

These are source proposals, not measured compile or gameplay recoveries. Every
listed body and secondary ability is authored into direct-runtime and
artifact-serialization/materialization scenarios. None has been executed.

## Shared implementation

The destination grammar distinguishes contextual “their/that player's
control” from each moved object's owner. Contextual entries lower to the
existing `PutOntoBattlefieldEffect` and its `PlayerFilter`; owner entries use
the existing native zone-movement/return-all owners. The grammar does not
substitute an owner for a selected player. Plain/tapped contextual entries
currently admit only fixed source/tagged references and real announced object
targets. Resolution-time selections need their own actor-owned selection
program and are not admitted by this new reading. Combined partition,
attacking, face-down and attachment destinations also need their complete
owners; acceptance of a controller prefix does not discard their tails.

An explicit source-zone filter survives both AST-to-spec and reference
resolution. Native source filtering uses the existing exact source/death
arrival resolver, applies the retained zone and owner predicates, and never
asks a chooser to select a same-name or same-stable-ID substitute. The native
prepared Put and Move owners retain all existing selection, preparation,
original movement, entry, observation and completion phases. There is no
new executor or post-entry controller mutation. Result tagging preserves
every arrival for a fixed plural set.

Compiled text includes non-default entry controllers and source-owned
graveyard eligibility. The default imperative still denotes the effect's
controller. Parser guards keep unknown words and non-word tokens, including
mana symbols, from disappearing from newly accepted source/destination
phrases.

No serialized/public field, variant, schema, protocol or version is added.
The compiler now emits existing qualified source-filter vocabulary instead of
discarding the source-zone qualification, and the native selector honors it.
The coordinator owns any resulting compatibility boundary and version gate.

## Five retained partial bodies

The fixture preserves these entire frozen bodies and metadata, but this
family does not claim complete coverage for them:

- **The Beamtown Bullies:** “target opponent whose turn it is” still needs an
  enforced target restriction. A destination change must not erase it.
- **Prince of Thralls:** “unless that opponent pays 3 life” requires the
  proper payer production and retained former-controller reference.
- **Tempting Wurm:** the optional participant clause still needs its complete
  comma-separated artifact/creature/enchantment/land selection, including
  any-number cardinality.
- **Dubious Challenge:** the whole looked/exiled pool, optional opponent
  selection and controller-split remainder need a complete collection program.
- **Verdant Mastery:** the alternate-cost branch, chosen opponent, one-card
  and two-card battlefield partitions and remaining searched cards need a
  complete search-pool partition program.

All eight records are normal-layout cards; there are no linked source faces
to omit. The fixture includes complete Oracle text, all printed header
fields present for these cards, layout, colors, color identity and keywords.

## Authored scenarios and checks performed

`crates/ironsmith-compiler-runtime/tests/battlefield_destination_references.rs`
contains independent strict-direct and artifact/JSON/decode/materialize routes,
secondary-body and text assertions, Charmed Griffin's actual cast/ETB and
optional opponent hands, Trove Warden's real landfall exiles across control
changes, and Endless Whispers' real death/grant/selected-opponent/delayed
sequence. Both linked exile and delayed return include a changed-incarnation
negative. Real target legality excludes wrong mana value, type and owner.
Malformed relative destinations and unowned resolution choices are negative
cases, not coverage.

Native Put scenarios retain a controller distinct from both the object owner
and effect controller across prepared entry, addition observation, pending
rollback/retry, late errors, exact arrival bindings and checked shared resource
failure/retry. Source-reference scenarios distinguish wrong zones and
same-name decoys without a choice prompt. Missing selected players and missing
required public producer evidence return checked errors without movement.
Grammar cases distinguish owner/player phrases and retained unknown residue.

Performed checks are source inspection, exact JSON field comparison against
the frozen archive, and `git diff --check`. Builds, compilers, tests, engine
probes, replays, browser probes, formatters and generators remain UNRUN.

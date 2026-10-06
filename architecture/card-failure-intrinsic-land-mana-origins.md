# Intrinsic basic-land mana: source-only prerequisite

This is an unvalidated ownership correction, not a completed card claim. No
build, compilation, compiler probe, test, formatter, browser, engine, or corpus
execution was performed. The artifact 7 / audit 20 boundary below is integrated
with exact selected play-permission actions in stage93, with independent source
review at `06cd8b8def619bc2f7064a5af16b57036bb6ce01`. This prerequisite is not
published independently; see `card-failure-stage93-compatibility.md`.

## Definition and rule owners

CR 305.6 mana is supplied from the current basic land subtypes, including on
nonbasic lands. The five handwritten basic definitions no longer manufacture
printed mana activations. Handwritten Scrubland's mana line is explicitly its
parenthesized reminder; Godless Shrine already has that authored surface. Its
actual entry ability remains. Ordinary unparenthesized mana on any card,
including a land with the matching basic subtype, remains a printed ability.

The front end recognizes only the complete standalone parenthesized tap/add
basic-color alternative surface. It carries the resulting subtype list in
`LineSemanticFacts`, not diagnostic annotations. Document recognition validates
that fact against final Land and subtype metadata before excluding the reminder
from executable recognition. Metadata order therefore does not assign origin.
Contradictory metadata is explicitly unsupported. The existing whole-document
CST path already discards generic fully parenthesized lines before the legacy
unwrap helper; recognized intrinsic reminders now bypass that early discard
only to preserve and validate typed source evidence. The old unwrap helper alone
is not proof that every prior compiled typed land contained a printed activation.
Original source/CST and Oracle lines remain available to diagnostics; no runtime display, card name,
Debug text, ability equality, or rules reparse decides provenance.

Both calculated and fast derived routes supply intrinsic mana at the existing
ability-layer boundary. The calculated owner suppresses only an already present
`IntrinsicBasicLandMana(subtype)` origin. A genuine equal printed activation or
independent grant keeps its own occurrence and index. The fast path appends the
same rule occurrences after object definitions. Existing layer-six ability loss
and CR 305.7 removal owners still apply. Copy values and text-box overlays carry
authored abilities; rule mana is supplied from the recipient's resulting types.

The typed text-change worker must integrate this prerequisite and the admission
gate together before removing its old ambiguous-mana hold. Forest to Island
then supplies blue intrinsic mana without stale printed green. A separately
authored or granted green activation remains green: mana symbols are not color
words. Activated-program rewriting remains that worker's responsibility.

## Compatibility and required regeneration

Artifact 6 erased the old reminder-versus-printed distinction. A default field
cannot recover it. The staged format 7 gate rejects all older artifacts at
`CompiledCardArtifact::validate`, including checksum-valid artifact 6, and thus
at catalog registration and artifact materialization. It does not guess a
migration by card name or matching mana effect. Wire field shapes and ordinals
are unchanged, so the engine schema-shape hash is retained; the format version
records the changed definition contract. New native in-process definitions are
authored by source owners. There is no serialized gameplay recovery migration.

All compiled artifacts must be regenerated from their original compilation
inputs after the execution gate opens. Merely relabeling 6 as 7 or refreshing a
checksum is not migration. No artifact regeneration was performed. Source
checksums for unchanged inputs may remain equal. Artifact payload checksums
change with the new format, and affected definition ability bytes, ability
labels, and rendered canonical text change. Pure rule reminders render no
authored activation; unrelated genuinely printed mana remains canonical text.

`ironsmith-tools` has two different hashes: `registry_card_content_hash` hashes
source payload/raw metadata and does not prove a compiled cache is current;
`CompileSnapshot::compute_content_hash` includes compiled text/definition and
therefore changes for affected results. Existing source-only cache keys cannot
justify skipping recompilation. Public audit object identity includes
`compiled_card_text` as `oracleText`; changed rendered identity can change its
checkpoint digest even though checkpoint structure 3 is unchanged. Native
basics previously built with empty canonical metadata may keep that displayed
identity, but their ability definitions still change. No numeric before/after
hashes were computed by executing the engine.

Current ability indices also change where an old dual-color printed activation
becomes separate subtype rule activations, or where a real printed activation
previously suppressed an equal intrinsic occurrence. Existing action JSON
shapes alone therefore cannot establish replay compatibility. The staged audit
20 gate admits only matching current peers and current-engine replay. Audit 19
joins 14/16/17/18 in historical signature-only verification with original bytes;
it is rejected before current-engine state is read or mutated. Public checkpoint
3 remains a digest input, not a gameplay restore format. In-memory root/inactive
lane savepoints continue to preserve their actual native state within the same
running engine; they are not a cross-version migration mechanism.

The exact selected play-permission action work shares this proposed boundary.
The coordinator must publish both semantic changes and their gate atomically,
or allocate distinct later versions if they are ever released separately.

## Authored scenarios and remaining validation

Native scenarios cover all five basic definitions, current-origin identity,
equal printed versus intrinsic occurrence indices, fast/sparse/current dispatch,
idempotent rule supply, copy values versus layer-three types, text-box replacement,
and independent grant identity. Existing basic/Scrubland/Godless Shrine checks
now inspect the actual rule-derived abilities instead of a fabricated printed
activation. Direct and artifact scenarios cover basic/nonbasic/dual typed lands,
metadata order, Forest to Island, genuine printed mana, independent green grants,
canonical defaults, and explicitly contradictory reminder metadata. Grammar
negatives retain authored, quoted, colorless, multi-mana, extra-clause, malformed,
and restricted activations outside the intrinsic reminder recognition.

Compatibility scenarios explicitly reject checksum-valid artifact 6, preserve
audit 19 signature bytes, and reject historical/mismatched replay before engine
access. Every scenario is unrun. Rejection scenarios establish admission holds,
not functionality. Before validation, the existing compiled-artifact golden
test needs its missing `fixtures/v5.json` source prerequisite resolved: the base
checkout contains only `fixtures/v3.json`. This pre-existing source issue was
observed, not executed or silently treated as a passing check.

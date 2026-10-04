# Spell copy prohibitions

Implementation-first checkpoint: UNVALIDATED. No compilation or tests were run.

Exact frozen payloads for Choreographed Sparks, Display of Power and See Double
are retained in `fixtures/spell_copy_prohibitions.json.fixture` with Oracle IDs.
The complete bodies remain in the tests; accepting only the first restriction
line would not constitute source coverage.

A new appended `CantBeCopied` static capability uses spell-functional zones and
has a typed compiler/runtime mapping. The complete restriction grammar accepts
only the authored production. Runtime copy creation checks the live spell's
abilities before allocating an object or announcing any copy/target events.
Captured departed spells use their old stack incarnation. Abilities of a
protected spell remain independently copyable. Epic, casualty and per-target
copy owners all use the same optional creation result; a prohibited copy is a
legitimate prevented action, not an execution error or a fake spell object.

Existing copy targets retain every announced target and each legal selected
spell is copied once. An explicitly optional zero-target set resolves without
an invalid-target error. Mixed protected/unprotected targets preserve copies of
the latter. Ordinary copies retain original modes, targets, X and payment
metadata through the existing copying primitive.

Whole-body source review also follows the existing modal choice and copy grant
paths. Choreographed Sparks grants haste plus a real triggered ability to the
copied stack object through typed continuous modifications; the existing
CR 400.7a retargeting carries those effects onto its resolved permanent, whose
end-step ability sacrifices itself. See Double's conditional modal limit uses
the maximum graveyard count of a matching opponent, not the sum of opponents'
graveyards, and modes remain frozen after announcement. Both bodies still
require the campaign's eventual token/resource and full runtime gates.

Authored, unrun regressions include all three direct/artifact full payloads,
real casting and target selection, mixed/zero copy sets, opponent graveyard
thresholds and response changes, creature-copy haste and end-step sacrifice,
protected departed LKI, ordinary copy metadata and independent ability copies.
Future validation targets are the `spell_copy_prohibitions` tools/runtime
integration targets and the native stack copy tests, followed by the frozen
full-corpus and supported-card comparisons.

# Ordinary face-down permanent transitions

Source proposal only. Builds, compiler probes, formatters, tests, and corpus
execution remain deferred by the implementation-first campaign policy.

The exact seven Oracle identity/name pairs in
`fixtures/ordinary_face_down_bodies.json.fixture` are extracted from the pinned
2026-10-03 corpus (uncompressed SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`). All seven are
`parser_failure` entries in `baseline-e8740178.snapshot.json.gz`. Each full body
includes mana cost, type, power/toughness where present, all Oracle sentences,
and its source URL; no clipped reminder/companion text is a replacement body.

## Shared operation and boundaries

A named `turn-face-down` clause reading consumes `turn <object phrase> face
down` through the sentence end. `PermanentStateActionAst::TurnFaceDown` retains
the existing target/source/count/filter model. Lowering produces one tagged
`TurnFaceDownEffect`, with artifact decoder/card-graph traversal, native
materialization and compiled-text rendering support. There is no card-name
branch and no new whole-card recipe.

The executor refreshes current characteristics, resolves and locks the complete
legal selected set, applies the existing reversible face-down overlay, and only
then refreshes dependent continuous characteristics. It uses an execution and
context checkpoint for pending choices or discovery errors. The action changes
no object incarnation or zone, counters, attachments, owner, controller or tap
state. It does not create ETB/face-up events, grant a turn-face-up method, or
invent Manifest, Cloak or Disguise evidence. The original Morph/Megamorph body
remains in the underlying reversible state for the normal paid face-up action.

Ordinary turning down has its own eligibility query. A face-up transforming or
modal DFC (`TransformLike`) and a merged permanent containing one are ineligible;
phased-out, already-face-down and nonbattlefield objects do nothing. This gate
is not added to the low-level `set_face_down`, because entry/casting through
Manifest/Cloak may legitimately install a face-down DFC. A flip card is not a DFC.

The typed Morph predicate reads current layer-derived static abilities.
Megamorph is a Morph variant; Disguise is separate. Printed alternative-casting
permissions and hidden face-up restoration data cannot make a currently
abilityless creature qualify. Current granted Morph does qualify. Target
legality is rechecked at resolution, then locked for that entire instruction.

## Complete bodies

- Backslide: targeted current Morph/Megamorph creature; Cycling {U}, discard,
  and draw remain native activated costs/effects.
- Master of the Veil: Morph {2}{U}, real face-up trigger, target declaration,
  and optional resolution. It may target itself or another player's creature.
- Mischievous Quanar: {3}{U}{U} self transition; Morph {1}{U}{U}; face-up
  instant/sorcery copy with the separate permission to choose new copy targets.
- Obscuring Aether: {1}{G} self transition and face-down creature spell discount.
  Cost matching now sees the selected prospective face-down view before public
  announcement, scoped to that candidate. This composes with the separate mana
  payment proposal projection; it neither changes that proposal's root/cache
  admission nor introduces a payment protocol/checkpoint/artifact migration.
- Skittish Valesk: own-upkeep called coin, only the losing branch turns it down;
  Morph {5}{R} survives underneath.
- Wall of Deceit: Defender, {3} self transition, and Morph {U} restoration.
- Weaver of Lies: Morph {4}{U}, face-up trigger, zero or more distinct targets
  with current Morph/Megamorph, excluding the source; one completed transition
  covers the entire selected set even if its first member grants Morph to others.

Cyber Conversion, Illithid Harvester, Ixidron, Yedora and Vesuvan Shapeshifter
remain separate owners for custom face-down characteristics, entry/return
semantics, or copy lifetime. These bodies are not claimed by this proposal.

## Authored validation

Grammar scenarios exercise source versus target phrases, real Morph ability
nouns, source exclusion, cardinality, and terminal-tail rejection. Full-body
integration scenarios compile direct and serialized/materialized routes, assert
companion rendering, and exercise paid activations/cycling/Morph, retargeted
spell copying, coin outcomes, Aether caster/face-down discount scope, declined
and accepted optional transitions, zero/subset/all Weaver targets, layer-derived
ability filtering, Defender loss/restoration, grant-loss batch locking, standalone
and buried-DFC restrictions with a single-faced merged positive control, preserved object
state, and pending-choice rollback/resume. These scenarios have not been run.

Final frozen full-corpus, linked-face, strict round-trip, native runtime, and
regression gates remain required before measured coverage can increase.

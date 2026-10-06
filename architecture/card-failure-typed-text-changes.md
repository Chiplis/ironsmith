# Typed text-changing effects: bounded owner increment

The frozen bodies of Alter Reality, Artificial Evolution, Balduvian Shaman,
Crystal Spray, Glamerdye, Magical Hack, Mind Bend, New Blood, Sleight of Mind,
Spectral Shift, Trait Doctoring and Whim of Volrath are retained exactly in
`fixtures/typed_text_changes.json.fixture`. **All twelve remain held; this
increment claims no complete card.** No parser body is routed to the old
`Modification::ChangeText { from: String, to: String }` no-op.

## Implemented owner boundary

`ironsmith_core::TextChange` checks same-family, distinct color/basic-land/
creature-type words, including during deserialization. A source word need not
occur. `RewriteText` is appended to the runtime and compiler continuous schemas;
the existing artifact 6/checkpoint 3/audit 19 boundaries keep their versions and
all earlier fields/ordinals/defaults. The core conversion and payload codecs
preserve the typed replacement. Existing string ChangeText values keep their
legacy interpretation; no existing body is admitted through the new primitive.

Layer 3, text-box inspection and dependency application share one atomic owner.
Current type-line words, literal protection qualities, landwalk subtype words,
hexproof predicates and bounded typed filters are transformed. Names, mana
symbols, object colors/indicators, P/T, selected runtime colors/types and rules
implied by wordless keywords remain unchanged. Residual filter validation is
structural, retaining names and mana predicates and refusing unknown nondefault
fields. No Oracle text, label, rendered name, Debug string or reparse supplies
execution semantics.

The definition/acquisition distinction is explicit: printed text, layer-1
copied text and exchanged text boxes can change. Counters, temporary grants,
borrowed abilities and intrinsic abilities cannot. Transformations preserve
`AbilityOrigin` and `StaticAbilityInstanceId`; previously captured immutable
clones retain their old definition. Copy reads stop at layer 1. Basic land
mana is still supplied later from the current type line by the existing
intrinsic-mana owner. Incomplete transformations produce a typed checked-state
error before any partial type or ability edit is published; ordinary effect
execution rolls back the registered effect and context receipts.

## Required next owners and admission holds

The current `CalculatedCharacteristics` has no spell resolution program;
`Object::spell_effect` and stack entries own it separately. Stack targets are
explicitly held until a current layer-3 program view, target legality rereads,
permanent-spell zone-transition retention and captured ability semantics agree.
Activated/triggered costs, choices, predicates and programs are also held;
rewriting only static text would incorrectly admit otherwise legal targets.
Attachment metadata, recursive/static model families, nested filters, token
creation/type-derived names, inferred keyword payloads and their authored-word
provenance remain outside the bounded visitor. Linked-exile/reveal definition
identities must be preserved, never recalculated from transformed programs.

No card-choice parser or resolution selector is admitted yet. Full body work
must include the following, beyond complete target/model traversal:

- Alter Reality: indefinite replacement and Flashback {1}{U}.
- Artificial Evolution: any source creature type, another destination type
  except Wall; Wall remains valid for the generic primitive and as a source.
- Balduvian Shaman: tap cost, white enchantment/controller/cumulative-upkeep
  target recheck, permanent text edit and independent Cumulative upkeep {1}.
- Crystal Spray: one selected family, end-of-turn expiry and draw-one tail.
- Glamerdye: indefinite replacement and complete Retrace land-discard cost.
- Magical Hack: indefinite basic-land replacement, including landwalk words
  and basic land type-line/intrinsic mana changes.
- Mind Bend: permanent-only target and indefinite selected-family edit.
- New Blood: additional untapped-Vampire cost, creature target, control change
  and same-object source-type choice with fixed Vampire destination.
- Sleight of Mind: indefinite color-word edit across spell/permanent domains.
- Spectral Shift: independently targeted land/color modes, both with Entwine
  {2}, independent choices and ordered resolution.
- Trait Doctoring: permanent-only edit through end of turn and full Cipher.
- Whim of Volrath: permanent-only edit through end of turn and Buyback {2}.

The eventual full-body scenarios must pair direct and artifact definitions with
native witnesses, absent-source choices, illegal cross-family/same-word/Wall
choices, target departure and changed target quality, stacked replacements,
expiry, copied text versus copied values, earlier captured versus later current
abilities, permanent-spell transitions and pending-choice rollback/retry. A
rejected domain is not a successful body, scenario, or coverage claim.

## Source evidence and deferred verification

Primary rules: Wizards' Comprehensive Rules dated 2026-06-19,
https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf,
rules 612.1–612.4, 613.1c and 613.2c: typed word role, names excluded, ordinary
grants excluded, token characteristics eligible, text layer and copiable layer
boundary. The exact frozen reminder text supplies each duration and secondary
body; no later Oracle substitution was used.

Authored native tests cover role boundaries, immutable captures and occurrence
identity, copy/source copies, independent grants, expiration, filter negation,
name/mana preservation and atomic holds. Independent direct/artifact target
scenarios exercise native/wire substitutions, timestamps, target recheck and
transaction rollback. Core tests cover typed invalid choices and serde. Every
scenario is unrun. No build, compilation, compiler probe, engine/corpus run,
test or formatter was invoked.

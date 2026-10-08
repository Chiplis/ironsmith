# Bounded copular characteristic statics

Status: **UNVALIDATED source proposal**, reconstructed on merged latest main
`5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1` for the new campaign stack.
The prior unpublished reconstruction `cfc952e6` and earlier `c820d97c7` Git
objects were lost after workspace replacement. Their exact edit/heredoc payloads
were retained. The twelve-file reconstruction was compared with the retained
old per-file blob manifest before main-relative adaptations. All unchanged
files match those hashes; the costs reader deliberately preserves the user's
foretell `token.is_word("costs")` fix. This document updates provenance and
review status. No claim is made that the old files or Git objects survived.

No build, compilation, test, formatter, compiler/engine/browser/replay probe, or
artifact generation ran. Only source/Git inspection, source edits, and frozen
JSON inventory comparison were performed. The authored scenarios below are
UNRUN, not evidence of a measured coverage increase.

The exact metadata fixture contains seventeen unique frozen Oracle IDs, all
copied from `cards-20261003.json.xz`. Thirteen complete bodies are proposed;
four bodies remain explicitly partial. The coordinator owns coverage records,
versions, publication, and the eventual validation decision.

## Complete proposed subset and full-body ownership

| Frozen card | Characteristic owner and retained secondary body |
| --- | --- |
| Aerial Modification | Conditional unsized creature addition to the enchanted Vehicle; existing creature-or-Vehicle enchant filter and +2/+2/flying grant retained. |
| Siege Modification | Same attached conditional recipient; +3/+0/first strike grant retained. |
| Ambush Commander | Whole Forest subject, 1/1, green, Elf, creature, and still-land tail; existing `{1}{G}`, sacrifice-an-Elf cost and targeted +3/+3 effect retained. |
| Kormus Bell | Whole all-Swamps subject, 1/1, black, creature, still-land tail. This is the complete one-line body. |
| Ashes of the Fallen | Existing choose-creature-type entry owner plus `has the chosen creature type`; explicit creature-card/graveyard/owner scope is retained. |
| Nylea's Presence | The existing every-basic-land-type addition now reaches the indexed registry from an enchanted-land subject; enchant-land and ETB draw retain their existing owners. |
| Leyline of Singularity | Existing supertype assertion and typed opening-hand pregame permission. The latter is consumed by `wasm_game_impl/pregame.rs::available_pregame_actions` and its opening-action handler; no fabricated permanent-entry spell effect. |
| Rimefeather Owl | Existing ice-counter supertype assertion, snow-permanent characteristic P/T, flying, and real `{1}{S}` ice-counter activation. |
| Rusted Relic | Complete labeled suffix condition; 5/5 Golem artifact creature under the live three-artifact threshold. |
| War Balloon | Complete source fire-counter condition and unsized artifact-creature descriptor; printed 4/3, flying, `{1}` fire-counter activation, and crew 3 retained. |
| Shifting Sky | Existing choose-color entry owner plus nonland-permanent chosen-color setter. |
| Stonework Packbeast | Unconditional additive source subtype CDA plus existing `{2}` mana filtering. |
| Veteran Adventurer | Same subtype CDA plus existing distinct-party cost reduction and vigilance. |

The first-prioritized line was not taken as proof of the rest of a card. The
frozen fixture keeps these four cards partial, despite their characteristic
clauses being supported by the generic changes:

- Lifecraft Engine: its chosen-type anthem excluding the source and crew body
  still need a dedicated complete-body audit.
- Shimmerwilds Growth: the extra-mana trigger must use the enchanted land's
  controller and the Aura's chosen color. The isolated color setter proves
  neither binding.
- Burakos, Party Leader: the attack's one party-size X must feed both defending
  player life loss and Treasure creation, with Choose a Background intact.
- Tajuru Paragon: the kicked consult must bind creature-type sharing to the
  source, preserve optional selection and random-order remainder.

These are withheld pending separate full-body work, not assertions that those
secondary owners necessarily fail today. No partial body is counted complete.

## Grammar boundary

The land-animation fact now carries the entire descriptor slice between the
fixed P/T and `that ... still lands`. The semantic owner consumes every color,
creature subtype and creature noun, then emits the existing AddCardTypes,
AddSubtypes, SetColors and SetBasePowerToughness payloads. Unrecognized words,
quotes, durations, grants and extra tails do not become ignored descriptors.
Land card types, supertypes, old land subtypes and existing abilities survive.
The existing land-animation registry already accepted arbitrary nominal heads;
only the separate land-type-addition registry needed its new whole-line head
exception for `Enchanted land`.

Chosen-type descriptors accept `chosen type` and `chosen creature type`;
`has/have` is admitted only for that complete chosen-type possession. Chosen
color and bare `is also` subtype assertions have their own complete predicates.
New subjects use complete source/attached/simple nominal productions rather
than suffix recovery. Source aliases are normalized through the card's existing
contextual static dispatch, so no printed card names are hard-coded.
Replacement words, dangling connectives and quoted text are not discarded.

The new conditional creature owner handles both leading `as long as` and a
complete trailing condition, with an optional fixed P/T. It requires a complete
creature descriptor and either an additive tail or artifact-creature wording.
CR 205.1b's artifact-creature exception preserves existing card types/subtypes.
Each emitted sibling carries the same existing typed condition. A missing P/T
emits no size-setting effect, so a Vehicle retains its printed size.

An attached antecedent binds `it's` to the attached object. A compound
attachment condition whose recipient is not unambiguous is rejected, rather
than silently using the Aura as the recipient. Explicit source-zone conditions
remain with their specialized owners; after recognizing a descriptor this
bounded battlefield owner returns a committed diagnostic for unsupported zone
conditions, including negated battlefield conditions. This prevents fallback
to a vacuous conditional card-type success. Quoted grants and compound grant
tails remain with their existing complete animation/grant readers.

The established source-only chosen-color production retains its canonical
display. The established complete fixed-size additive animation bundle retains
its ordering and condition representation. The new conditional owner is routed
before the older explicit-`it is` generic card-type identity wrapper, so
artifact-creature retention does not depend on contraction spelling.

## CDA, layer order, and lifetime

No new continuous modification, static payload, enum variant or serialized field
is added. Core and runtime expose a structural
`characteristic_defining_subtypes` query for nonempty, unconditional,
source-only AddSubtypes. Construction gives only this exact shape all-zone
defaults. Conditional wrappers, recipient filters, explicit source-zone filters,
and chosen-type dependencies keep the existing battlefield default.

The existing static-effect processor's rules-text provenance check already
classifies self AddSubtypes effects as characteristic-defining. Printed/copied
origins therefore use CDA ordering before ordinary effects in layer four;
temporary ordinary grants keep their acquisition timestamp and the host's
ordinary functional zone. The new subtype query joins the existing color query
at that temporary-grant boundary. It does not expand all conditional or granted
statics to all zones. Ordinary later subtype replacement and ability loss remain
owned by the current layer machinery.

Chosen color/type is read from the current source incarnation by the existing
native generators. Controller-relative affected sets remain live. Phasing,
source departure, attachment changes and blink use the existing exact source
and recipient identity; a new incarnation cannot inherit an old choice or old
attachment. Missing choice state produces no invented color/type.

Compatibility is a coordinator decision. Although vocabulary is unchanged,
new construction serializes different functional_zones for qualifying source
AddSubtypes. Old artifacts retain their previously serialized zone list because
zone defaults are intentionally not rerun on load. A semantic/artifact cache
invalidation may therefore be needed independently of any earlier version
proposal. This patch does not edit versions or recovery boundary manifests.

## Authored validation, all UNRUN

- Grammar/core tests: exact descriptors and recipient domains, Ashes possession,
  Vehicle creature scope, four source subtypes, all-zone versus conditional and
  chosen defaults, attached no-size binding, suffix conditions, registry routing,
  and negative incomplete/quoted/extra-tail/unresolved-antecedent cases.
- Tools aggregate: all thirteen exact metadata-bearing bodies must compile
  strictly with no parse loss or metadata fallback. All seventeen fixture rows
  remain visible, and only thirteen are selected.
- Runtime direct and JSON-artifact paths: validate transport, reconstruct native
  definitions independently, render complete canonical text and reparse it.
- All-zone source subtype cases, original subtype retention, older ordinary
  replacement versus printed/copied CDA order, later ordinary grants, and native
  clone retention of exact grant origins. Temporary grants outside the host's
  ordinary zone must not inherit CDA defaults.
- Both land animations: controller-relative versus global recipient sets, hand
  exclusion, additive types, color/subtype/P/T, counters, controller changes,
  phasing and departure. Ambush also pays the real Elf-sacrifice activation.
- Aura animations: opponent-owned artifact-land Vehicle recipients, printed
  size plus complete P/T/keyword grants, reattachment, phasing and host blink.
- Ashes: actual entry choice, explicit graveyard domain, controller changes,
  phasing, fresh choice after blink, an uncommitted pending entry and independent
  native clone continuation. Shifting Sky uses a real color choice and excludes
  lands while retaining ordinary later color effects.
- Rusted threshold transitions, source-zone exclusions, retained land type and
  counters. War Balloon pays for three fire-counter activations and separately
  crews with a real creature while retaining printed size and flying.
- Nylea retains ETB draw and five intrinsic basic-land mana abilities; Leyline
  retains its typed opening permission and live legendary scope; Rimefeather
  pays real snow mana and updates its snow-count CDA through ice counters;
  Packbeast filters mana; Veteran counts distinct party members and keeps
  vigilance.
- Checked native static discovery: a deliberately insufficient effect budget
  must return the existing explicit EffectLimit without publishing partial
  characteristics; the same unchanged input with a sufficient budget must retain
  the full animation. Missing choice and conditional non-CDA cases are separate.
- A checked missing-controller source must produce the existing typed error
  before a new animation can publish a partial result; removing the invalid
  retained effect permits ordinary recovery.

Source inspection and JSON comparison establish authored scope only. Deferred
validation must run the authored suite, existing static/color/basic-land tests,
and the coordinator's exact native/Wasm retained-state compatibility boundary.
The clone/pending scenarios are not a claim of a completed browser or public
recovery round trip. No measured coverage delta is reported here.

## Reconstruction review

A fresh independent source reviewer checked the restored production owners and
identified the source-zone fallback gap above. The correction changes a
recognized unsupported descriptor from NoMatch to a committed diagnostic, with
whole-parser explicit-`it is` and contracted negative cases. The original
pre-outage source review is historical evidence only; it is not substituted for
this reconstruction review. All authored executable validation remains UNRUN.

The renewed source reviewer also identified a scenario-only phasing assertion:
current characteristic queries return None for a phased battlefield object.
The restored Shifting Sky scenario now expects None while phased, the chosen
red after phase-in, and its printed blue after departure. This correction was
made from source inspection; it was not executed. The compatibility document
was restored after the reviewer's bounded production pass and must be included
in the coordinator's final integration review.

## Latest-main integration review

The unpublished copular delta is reconstructed independently on the exact main
commit above. The user's merged build/test fixes are retained; no earlier
coverage ledger or version patch is replayed. A fresh independent read-only
review through `c032297b2a14ee0661a2eefd59883aab8fef9cb6` covered production,
all original 535 runtime lines plus the 18-line choice-family scenario,
grammar/core and aggregate fixture cases, and this compatibility document.
No remaining material source defect was identified. Prior review findings
above are historical context; executable validation remains UNRUN. The
coordinator owns publication, version decisions and coverage credit.

The fresh latest-main source review found that explicit `chosen creature type`
wording could inherit the old recipient-based land-family inference for land
or land/creature-union subjects. The explicit descriptor family now selects
AddChosenCreatureType; bare `chosen type` preserves the existing land inference.
Authored typed grammar cases and direct/artifact entry-choice scenarios keep
creature-choice and basic-land-choice storage distinct. These additional source
corrections deliberately differ from the retained old blobs. The thirteen-card
proposal and four withheld partials are unchanged; execution remains UNRUN.

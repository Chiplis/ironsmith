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
earlier modification ordinals. Retained copy values add an omitted-by-default
spell-program field with explicit unavailable, known-absent and present states. The core conversion and payload codecs
preserve the typed replacement. Existing string ChangeText values keep their
legacy interpretation; no existing body is admitted through the new primitive.

Layer 3, text-box inspection and dependency application share one atomic owner.
Current type-line words, literal protection qualities, landwalk subtype words,
hexproof/enchant predicates, attachment metadata and typed recursive predicates
are transformed. Value/condition/target/history visitors follow authored
subexpressions, including a color-word mana-symbol count and its derived event
quantity, while leaving actual mana symbols intact. Names, mana
symbols, object colors/indicators, P/T, selected runtime colors/types and rules
implied by wordless keywords remain unchanged. Residual filter validation is
structural, retaining names and refusing unknown nondefault fields. Ambiguous
symbol-versus-word values and opaque ability markers remain checked holds. No Oracle text, label, rendered name, Debug string or reparse supplies
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

Current `CalculatedCharacteristics` now owns a transient spell program and its
ordered text substitutions. Raw Object programs remain the copiable baseline;
StackEntry ability programs remain earlier immutable captures. Spell resolution,
announced target-spec revalidation and retarget proposals read current text.
Declaration ranges, choices, modes, link/result/tag identities and controller
bindings remain intact. Retarget and choose-new-target owners propagate missing
program evidence; boolean effect filters record it on the checked-action latch
so negation or zero matches cannot commit a successful answer. Live spell copies read the layer-1 envelope, and departed-spell LKI records
that same pre-text envelope before the old object is removed. Spell-specific
capture retains Bestow cast overlays and their attachment program; the exact
metadata-owned enchant occurrence does not become a lasting raw ability. Ability copies
continue to use their captured stack program. The existing CR 400.7a owner carries a resolved text
effect to the entering permanent with its original duration.

`CopiableValues` freezes the program when the copy effect begins. Its retained
codec preserves exact programs and explicit known absence. Omitted historical
payloads become **unavailable**, never known-absent and never a request to read
a donor's later object or definition. Reading such a copied spell program is a
checked completeness boundary. Raw object materialization retains missing
copy-program evidence in the typed ResolutionProgram completeness field rather
than manufacturing an executable empty body. Complete older programs omit that
new flag and retain their ordinary meaning. Program execution and definition
admission reject the incomplete state; map/append operations preserve it. This is the necessary admission change for old
retained spell-copy payloads; ordinary old permanent-copy characteristics are
unchanged when executable spell evidence is not required. Public claim snapshots
strip the executable program just as they strip executable abilities.

The native program visitor currently admits 67 complete elementary/composition
models, activated costs, choices and restrictions. Twenty-seven added native
codec conversions retain changed programs; every admitted effect has a typed
codec path. Memoized immutable transformations preserve repeated effect-node
identity and old captures. Triggered owners and the rest of the static/effect
model families are still held. Fresh native ApplyContinuousEffect serialization
is a separate pre-existing codec boundary; native application and independently
constructed wire application are tested separately, not misreported as a native
round trip.

Basic-land intrinsic-mana provenance is a further explicit hold. Some historical
native/compiler definitions store rules-granted mana as an ordinary printed
activation. Changing its type line must not leave that obsolete activation and
also add the new intrinsic mana ability. An affected basic land with ambiguous
raw mana activations is held until the definition owner records the distinction;
no mana symbol or label is used to infer an authored word. Token creation and
type-derived names, inferred keyword payloads, and remaining semantic owners
also need complete provenance. Linked-exile/reveal identities are preserved,
never recalculated from transformed programs.

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
transaction rollback. Core tests cover typed invalid choices and serde. Current-program tests also
cover independent captured activations, changed target legality, frozen donor
programs, spell copies, permanent-spell transitions, repeated reads, and missing
historical program evidence. Codec fixtures rewrite native, materialize wire,
then rewrite again to detect stale execution rather than merely retained JSON. Every
scenario is unrun. No build, compilation, compiler probe, engine/corpus run,
test or formatter was invoked.

## Reviewed additive program checkpoint

The current/captured program chain through `75e059ed5` is independently source-
reviewed. Integration at `546b7bb9f` retains linked-exile pairs, exact activation
definitions and incomplete copied-program evidence together. The exhaustive
condition visitor preserves activation thresholds unchanged; both combined
metadata/serde and predicate scenarios are authored but unrun.

`CopiableValues::spell_effect` is explicitly skipped by public-claim snapshot
serialization, and claim normalization clears it before ledger hashing. Current
public-audit object/stack and hidden-metadata carriers are explicit projections.
The separate retained-copy codec carries known program evidence but has no
production public-audit/hidden-metadata caller; restricted-mana on-spend
encoding does not use that codec. The bounded source check found no implicit
new hash-shape field on audit 3/protocol 19 from this increment. This is not an
executed replay-equivalence claim.

All twelve full text-changing cards remain partial and receive zero credit.
The intrinsic basic-land mana and exact selected-permission migrations require
a separate, coordinated compatibility boundary; its tentative versions are
not activated by this checkpoint.

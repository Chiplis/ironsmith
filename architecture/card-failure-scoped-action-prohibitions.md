# Scoped action prohibitions

Status: source-authored, UNVALIDATED. No build, compiler, test, formatter or corpus
replay was run. These are fourteen proposed full-card source closures, not
measured recoveries. The campaign's last measured result remains 40 recoveries
and 3,193 unresolved unique Oracle identities.

## Source proposals and retained bodies

The exact frozen Oracle identities and texts are in
`fixtures/scoped_action_prohibitions.json.fixture`:

- Aggressive Mining: controller-only land prohibition; real land-sacrifice cost,
  draw-two program and once-each-turn activation restriction retained.
- Ashes of the Abhorrent: graveyard-origin casting and graveyard-card activation
  are separate prohibitions, including mana abilities; its creature-death
  life-gain trigger remains a real trigger.
- Cornered Market: source-independent current nontoken battlefield names;
  separate matching spell and nonbasic-land prohibitions.
- Exclusion Ritual and Ixalan's Binding: actual source-linked exile, live current
  exiled incarnation names, and global versus opposing-player prohibition scope.
  Binding retains its until-source-leaves return instruction.
- Experimental Frenzy: both hand-origin prohibitions share the authored origin;
  existing top-library look/play/cast permissions and paid self-destruction stay
  present. Its restriction does not supply permission to play another zone.
- Iona, Shield of Emeria: real entry color choice and source-relative chosen-color
  prohibition; current controller and phased-out host rules remain authoritative.
- Llawan, Cephalid Empress: intersecting blue/creature spell filter; the actual
  blue-opposing-creature return trigger remains separate.
- Pardic Miner, Turf Wound and Solfatara: announced player target, retained exact
  player identity and turn-end duration. Miner pays its self-sacrifice cost;
  Turf Wound draws now; Solfatara schedules its next-turn-upkeep draw.
- Territorial Dispute: global land prohibition and actual upkeep sacrifice unless
  a land is sacrificed.
- The Immortal Sun: only loyalty abilities of planeswalkers are prohibited;
  the same planeswalker may still have other activated abilities. Its additional
  draw, generic spell discount and controlled-creature anthem remain present.
- Tomik, Distinguished Advokist: opponent graveyard-origin land plays, plus the
  existing source-filtered targeting restriction on battlefield/graveyard lands.

Tidal Barracuda is **partial**: its opponent-turn prohibition uses the existing
conditional restriction owner, but the global flash sentence requires a genuine
all-origin casting-timing permission. The existing hand-only grant is not a
whole-card implementation. Phyrexian Censor is **partial**: the existing filtered
one-spell-per-turn restriction does not close its negative-subtype ETB dispatch
failure. Neither identity is counted by this patch.

## Executable ownership

`Restriction::PlayLandsMatching` and `ActivateLoyaltyAbilitiesOf` are appended to
preserve older serialized variant ordinals. No card-name dispatch or acceptance
marker is introduced. Core tag walks, reference resolution, lowering's player
scope check, text rendering and target folding retain both new payloads.

Land plays use the actual special-action legality check, after selecting the
chosen MDFC land face and before considering zone permission. Direct checks use
`continuous_query_snapshot` and preserve discovery errors. Every land enumeration
owner (hand, library, public zones, Adventure exile and the separate back-face
option) propagates those errors through `special_action_is_legal`; an unknown
selected-face result cannot become a completed action list with that land absent. Matching retains the
restriction's controller, source, iterated player and frozen object tags. Playing
a land and casting a land card as a spell are not conflated. Source-relative
cast filters now also retain that context, including whose hand is named by
`your hand`; current source-linked exile tags are added without replacing it.

Graveyard-card activation rules materialize matching object identities in their
explicit zones instead of scanning only the battlefield. The normal and mana
activation checks both enforce the same current-incarnation prohibition. Default
permanent-only activation bans retain their battlefield scope. Loyalty-only
prohibitions are checked in the shared normal activation precheck and leave
nonloyalty abilities of a planeswalker usable.

A resolving player land-play prohibition freezes its announced player while
retaining the future land filter. It is not a frozen list of current land cards.
The source may leave after resolution; cleanup owns expiry. Static rules instead
follow the live host's control and phase/zone status. Both use the existing
restriction layer, not the replacement manager.

The complete mixed-action grammar owns the final shared origin before generic
`or` splitting. Unknown tails remain unrecognized. Targeted durational player
clauses decline the static object-rule route and retain effect/target ownership.

## Recovery and deferred evidence

Active resolved restrictions are not encoded by the ordinary sync carrier.
The earlier `b63ab96e` completeness guard rejects such exports and missing import
proof fields; local UI analysis uses the exact native branch, and verified
network recovery replays the full accepted signed transcript. These prerequisites
are source-reviewed only, with runtime validation deferred.

Authored scenarios cover direct/restored full artifacts, global/controller and
source-phase/control lifetimes, exact player target retention after self-sacrifice,
turn cleanup, future land acquisition, immediate/delayed draws, actual graveyard
casting and activation routes, mana-ability sources, mixed-action hand/library
scope, current exiled identities, chosen colors, same-name nontoken matching,
loyalty-only enforcement, secondary bodies, and chosen MDFC face names.
All scenarios remain unrun. Full corpus replay, runtime semantics, native/Wasm
wire behavior and broad regressions remain deferred campaign gates.

An additional unrun native scenario keeps front-face discovery finite and makes
only the selected back-face query nonconvergent. Both whole-player and per-source
action enumeration must retain the typed failure across six land-origin routes,
without committing the prospective face or publishing partial actions.

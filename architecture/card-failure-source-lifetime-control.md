# Source-lifetime control proposals

Status: UNVALIDATED. No compiler probes, builds, or tests were run. The frozen
fixture contains five complete-body source proposals and four explicit partials.

## Shared semantic correction

The complete control-duration reader retains the authored source reference and
lowers “for as long as this [source] remains on the battlefield” to the existing
latched `ForAsLongAs(ObjectOnBattlefield(Source))` predicate. Resolution captures
the exact source incarnation; an absent source prevents registration, source
control changes do not change the effect controller, and departure/phasing ends
the effect permanently. An unknown conjunct/tail is rejected rather than dropped.

The same authored source-lifetime meaning is now shared by gain-ability, leading
chain-carry, and generic suffix readers through one typed `Until` constructor.
Literal “until this leaves the battlefield” stays a distinct departure-event
lifetime, including the already proposed Gaea's Liege/Graceful Antelope bodies.

Rules: CR 611.2b and 702.26f in the pinned 2026-09-25 Comprehensive Rules:
https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt
The former prevents a duration from starting after its required condition has
ended; the latter ends source-tracking for-as-long-as effects on phasing. Neither
permits chasing a blinked source by card identity.

## Five source proposals

- Charisma: real enchanted-source damage captures the other damaged creature;
  the Aura owns the duration even after it moves to another host.
- Cytoplast Manipulator: full Graft entry/transfer plus paid blue/tap activation;
  the target's +1/+1 counter is an announcement/resolution restriction, not a
  continuing control condition.
- Giant's Grasp: Giant-you-control Aura attachment and nonland ETB target are
  separate announcements; indirect Aura phasing ends the control effect.
- Scarwood Bandits: opponent payment keeps the artifact, nonpayment creates the
  source-bound effect; mana/tap costs and the complete Forestwalk body remain.
- Sower of Temptation: real cast/ETB target, Flying, departure-before-resolution,
  exact blink identity, control changes, phase-out, and non-restart are covered.

The fixture also pins The Akroan War, The Super Hero Civil War, Infernal Denizen,
and The Horus Heresy as partial until their complete additional chapter/upkeep
programs and choices are reviewed and have dedicated scenarios.

## Authored validation

Grammar positives cover creature/Aura/Saga/permanent source phrases. Negatives
cover unrelated references and trailing extra predicates. Full frozen definitions
are checked for strictness and artifact JSON materialization in authored tests.
Runtime scenarios exercise actual casts, target selection, activated payment,
Graft counters, replacement-compatible damage receipts, control changes, source
incarnations and phase transitions. A paired supported-grant regression covers
both leading and suffix durations and preserves literal until-leaves behavior.

Source inspection is provisional. No measured recovery, no executed semantic
credit, and no exhaustiveness claim are made by this proposal.

## The Akroan War closure: zipped source/recipient damage

The quantified reflexive damage reader now lowers both direct quantified and
explicit ForEach AST shapes to one `DealDamageBySourcesEffect` with
`DamageRecipientSetBinding::EachSource`. Each captured tapped source contributes
only `(its exact id, the same recipient id, its pre-damage power)`. The default
`SharedSet` retains ordinary Cartesian damage. Recipient binding and the
unpreventability flag are appended, defaulted payload fields; no existing enum
ordinal changes. Native codec/interpreter paths preserve the typed payload.

The shared damage owner freezes all assignments before replacements or original
results, so lifelink and additional pump/tap programs cannot change a later
source's power or enroll a new tapped member. Empty and negative-power sets still
produce no positive damage. Canonical text renders the reflexive relationship;
Alpha's structural compactor rejects a different recipient mode.

Full exact metadata/direct/artifact scenarios now propose The Akroan War:
chapter I uses the exact visible Saga lifetime; chapter II grants the opponent
attack requirement through the caster's next turn independently of Saga
existence; chapter III captures tapped sources, assigns only self damage in one
batch, and the final Saga sacrifice releases chapter I's control effect. The
scenario includes life-derived power, lifelink, and replacement-added pumping
and tapping. Native scenarios cover empty/nonpositive members and positive
members' distinct receipt identities. All scenarios remain authored/unrun.

### Chapter II rule correction

The explicit timed `attack each combat if able` set clause is now a positive
`Restriction::MustAttack`, using the existing registered rule-effect owner and
its duration. It is not a layer-six ability grant. The live controller/type
filter is re-evaluated, later creatures qualify, and ability removal cannot
clear it. The derived combat tracker retains the number of independent rule
requirements per creature, consumed by both attacker choices and the declaration
optimizer. Source departure does not end a separately timed rule. Actual quoted
granted abilities retain their ability representation.

The additional full-card scenario uses later entry, control changes, ability
removal and source exile, and rejects an empty attack declaration while the rule
is active. It expires at the original controller's next turn. Existing native
savepoints clone registered restrictions; authoritative wire checkpoints already
refuse every active registered restriction and use accepted-transcript replay.
No new wire carrier or silent recovery fallback is introduced. Unrun.
## Targeted Saga follow-up (source review pending)

The Super Hero Civil War has two additional authored direct/artifact full-chapter
scenarios. They assert one aggregate mana-value target budget (4+2 accepted,
4+3 rejected), zero optional control targets, the locked chapter-II pump and
Vigilance set excluding later entrants, two distinct fight targets or no optional
opponent, Saga sacrifice returning the stolen creatures, and end-turn expiry of
the independent pump. The frozen row remains partial pending bounded review;
no compiler or runtime execution was performed.

## Aggregate target revalidation follow-up

Source review found that chapter I's total-mana-value restriction was checked
only at announcement. The resolution owner now rechecks each retained assignment
as one group, with current calculated characteristics and a wide total. It never
selects an arbitrary affordable subset. Independent target slots remain separate;
an empty optional assignment remains empty. Unknown dynamic limits or missing
retained evidence propagate typed errors through both ordinary stack resolution
and immediate trigger resolution rather than becoming a default bound.

A single legacy aggregate group can be reconstructed; ambiguous legacy multiple
groups without retained boundaries fail explicitly. Present targets retain their
contribution even if another condition makes them illegal. Exact departure or
phasing LKI is used for other original members, never a new incarnation. This
last choice is inferred from the same legality purpose as the official Run Away
Together controller-comparison ruling, rather than an explicit aggregate-specific
example. It remains part of the bounded source review and eventual runtime gate.

Primary references:
- [Reunion of the House, Tarkir Dragonstorm release notes](https://magic.wizards.com/en/news/feature/tarkir-dragonstorm-release-notes): exceeding the collective bound invalidates the whole selection.
- [Run Away Together, Bloomburrow release notes](https://magic.wizards.com/en/news/feature/bloomburrow-release-notes): independently illegal members still provide comparison information while checking the other target's legality.

Additional unrun scenarios cover same-incarnation copy changes to MV7, collective
6-to-7 inflation, departure, shroud, and unknown dynamic-bound propagation. The
Super Hero Civil War remains partial until this delta and the separate Fight
owner correction are source-reviewed and integrated together.

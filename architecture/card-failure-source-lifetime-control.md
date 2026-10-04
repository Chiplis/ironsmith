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

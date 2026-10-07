# Plural-controller untap restrictions

## Current NEXT03 disposition

Independent final source review cleared `92168f50060c362c88d5c1ecf6477dbbffa11839`. The [NEXT03 admission](card-failure-next-series-03-source-admission.md) records only the bounded source proposals: predicate 5, copy 4 IDs / 5 entries, suspended 4, numeric 4, plural untap 5. All held neighbors remain excluded. All executable scenarios are **UNRUN**, the source remains **UNVALIDATED**, and no new measured recovery is claimed. The historical scoped-work notes below describe their original stages; coordinated compatibility is now artifact 14 / digest 9 / audit 27.


Status: **UNVALIDATED source proposals**. Independent bounded review clears six full frozen bodies through `0692003a7`, with additive integration reviewed at `020896ba8`. Builds, tests, compiler probes, formatters and corpus execution remain deferred.

The exact inputs are in `fixtures/plural_controller_untap.json.fixture`: Breaching Leviathan, Cone of Cold, Dragon Turtle, Lorthos, Sudden Storm and Code of Constraint. The complete plural duration now parses through the shared leaf. Conditional imperative bodies use the existing typed effect discriminator, preserving Code's tap-and-freeze sequence instead of treating its suffix as a static ability.

`RestrictionEffectInstance` retains an optional exact affected object for controller-relative untap restrictions. Its live controller owns the next real step; fixed native owners such as Exert retain their original player. The appended `Until::YourNextUntapStep` distinguishes explicit fixed-player wording without changing prior enum order. Both forms lock the resolving object set; known empty registers nothing. Checked native Cant execution shares resource and rollback ownership. The turn owner freezes which existing restriction timestamps this step consumes before untap replacement additions. Phasing preserves the exact ID, new zone incarnations do not inherit it, and departure of the original spell controller does not rewrite an object-owned duration.

The official Sudden Storm ruling requires a controller change before the old controller's next step to move the restriction to the new controller's next step. Source: https://magic.wizards.com/en/news/feature/release-notes-2014-01-22 (verified October 5, 2026).

Positive tagged subject membership becomes exact object identity at registration. Untargeted Tap references likewise retain exact tagged identities while preserving explicit target slots, wrappers and authored filters. Code's real draw-replacement blink regression checks that neither its later tap nor freeze transfers to the returned incarnation. A direct retained-set scenario separately pins the native Cant boundary. This does not add a generic Tap replacement feature.

`plural_controller_untap.rs` authors independent direct/artifact/native scenarios for all printed bodies: hand-cast qualification and blue exclusion; optional targets/payment and actual attack/entry; dice branches and timed future entries; zero/partial/all-illegal targets and Scry; cast-time Addendum despite a changed resolution phase; already-tapped objects; controller changes; native clone, source departure, blink and phasing; fixed-player/Exert contrast; and an actual TurnRunner skipped untap followed by the next two real untaps. None ran.

Named fixed-player cohorts (Sleep, Misstep, Blinding Beam, Icebreaker Kraken and Imaginary Threats), Telekinesis's two-step count and Orcish Farmer's until-the-step characteristic duration remain separate unclaimed work. All broader runtime validation remains gated by the campaign workflow.

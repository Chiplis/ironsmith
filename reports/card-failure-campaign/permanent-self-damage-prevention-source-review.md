# Permanent self damage prevention: six complete bodies

Source-only checkpoint based on published stage 76 (`69a946ec767deda59927d63f08dd17fabff470f3`). No builds, compiler probes, tests, formatting, or corpus execution were performed. All scenarios below are authored and unexecuted; no verified compile-recovery claim is made.

## Bounded identities

The frozen `baseline-e8740178.snapshot.json.gz` classifies all six as parser failures. Their entries in `source-coverage.json` were unaddressed at this checkout. `fixtures/permanent_self_damage_prevention.json.fixture` preserves each complete body, metadata, and Oracle identity directly from `cards-20261003.json.xz`.

- Argothian Pixies: artifact-creature prevention and its independent artifact-creature blocking restriction.
- Argothian Treefolk: artifact-source prevention in any zone, without broadening artifact creatures to any artifact or narrowing sources to creatures.
- Dawn Elemental: existing all-damage self-prevention plus flying; this checkpoint supplies complete-body native/artifact/runtime coverage for the previously unaddressed identity.
- Desert Nomads: Desert-source prevention plus a typed Desertwalk keyword, including landwalk's live defending-land condition.
- Tresserhorn Skyknight: flying and prevention from creatures that currently have first strike; double strike alone does not qualify, and prevention is not restricted to first-strike combat damage.
- Wall of Putrid Flesh: defender, protection from white, and prevention from creatures with an active Aura, including exact attachment characteristics in source LKI.

Wall of Vapor, Aura-recipient prevention, conditional prevention, and more complicated follow-up bodies are not claimed by this checkpoint. Their coverage dispositions are unchanged.

## Grammar and runtime path

A named grammar produces a complete imperative prevention shape and typed source-domain/controller facts. Bare subtype nouns such as Deserts keep the permanent domain, while explicit sources remain any-zone and explicit card/zone qualifiers are preserved. The front end maps these through the existing object-filter mapper into `PreventMatchingDamageSpec`. The registry owns only source-qualified, all-damage, self-recipient instructions; existing unqualified, bare-creature, and combat-only productions remain disjoint. No card name participates in recognition or execution. Durations, conditions, target instructions, inner punctuation, and extra sentences cannot disappear into the recognized instruction.

The existing `DamageAmountReplacementMatcher` supplies live replacement context, complete continuous-effect discovery, current source characteristics, and exact event-source LKI for a departed/phased source. The exact self recipient is retained across type/controller changes. A missing or wrong filtered source snapshot remains an execution error rather than an apparent successful nonmatch. Prevention remains prevention: it emits actual prevented-damage events and cannot stop unpreventable damage.

Two related source-inspection defects are corrected: calculated attachment snapshots omit phased-out attachments and derive enchanted state from the captured active Aura characteristics; landwalk ignores phased-out defending lands. These changes are exercised by complete-body scenarios.

## Authored verification

`crates/ironsmith-compiler-runtime/tests/permanent_self_damage_prevention.rs` independently authors strict native compilation and artifact validation, JSON round-trip, materialization, and behavior for every full frozen body. Its cases cover repeated combat and noncombat damage, exact recipients, no shared or consumable budget, unpreventable damage and prevention provenance, artifact/source zone distinctions, current type/subtype/first-strike changes, Aura versus Equipment, attachment movement and phasing, exact source LKI versus a new incarnation, missing/wrong LKI, source departure/phasing/ability loss, and each companion keyword/restriction. An unlisted-name case demonstrates the generic route.

Parser-local scenarios assert source domain, controller, exact self recipient, typed amount, first-strike/Desert/Aura filters, full-tail rejection, and static-registry reachability. Execution remains deferred under the campaign instruction. Central coverage files were not changed.

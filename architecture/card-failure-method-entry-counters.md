# Selected-method entry counters (UNVALIDATED)

Exact frozen identity: Worldheart Phoenix,
`67c50af9-bb8e-40b6-8b25-c209784b6865`, in
`fixtures/intrinsic_zone_alternative_costs.json.fixture`. Its previous partial
was the method-specific entry rider after the represented WUBRG graveyard cost.
Independent bounded source review cleared the complete identity through `19edf4ab4`. It is proposed/unvalidated in stage53; every authored scenario remains unrun.

`AlternativeCastingMethod::FromZone` now retains a default-empty entry counter
list. Empty lists are omitted from its JSON shape and old payloads read as
having no rider. The existing total-cost/condition mapper preserves that list.
The full intrinsic reader delegates the complete entry-counter clause to the
existing typed counter grammar, then accepts fixed counter amounts only. Unknown
extra effects and unrepresented dynamic amounts still produce a parse error.

Permanent-spell resolution reads only the retained selected method (or the
explicit intrinsic alternative index), then supplies its counters to the normal
`BattlefieldEntryOptions.initial_counters` owner. Thus entry replacements,
doublers, prevention, original-event completion and ETB observers all share the
existing entry receipt. No counters are added later and no card names are read.
An independent ordinary-price graveyard permission does not select this rider.

CR 707.10 copies an alternative-cost decision, so a copy of the paid spell
inherits the rider. `Object::spell_copy_of` already retains that exact method.
Copying the resulting permanent does not copy paid-method state, and a later
blink is a new incarnation. This is different from actual mana-spent receipts,
which correctly become zero on a spell copy.

Primary references:
- [Modern Masters 2015 release notes](https://magic.wizards.com/en/news/feature/modern-masters-2015-edition-release-notes-2015-05-12): Worldheart's alternative does not change timing.
- [March of the Machine release notes](https://magic.wizards.com/en/news/feature/march-of-the-machine-release-notes): copied spells retain effects based on paid alternative/additional costs.
- Frozen official CR 2026-09-25: 601.2b, 614.12, 707.2 and 707.10.

Six added public runtime scenarios cover exact artifact payload, actual WUBRG
payment versus the ordinary hand price, forbidden origins/ownership, real
counter doubling and entry observers, spell/permanent copy and blink boundaries,
another graveyard permission, and legacy/new wire round trips.
No builds, compilation, tests or runtime probes have been performed.
Deferred command:
`cargo test -p ironsmith-compiler-runtime --test intrinsic_zone_alternative_costs -- --nocapture`

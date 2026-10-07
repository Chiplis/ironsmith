# Bounded next-time damage redirection

Status: independent actual-source review cleared all seven complete frozen bodies at `ca130346ee8be51e150599f183c8f66aed6d53e1`, including the paid Hazduhr scenario in `d9494e6b8` and the independently reviewed combat survivor correction in `f65569b89`. Implementation and source scenarios are authored; execution deliberately not performed. This is not compiler or runtime validation.

Base: stage78, `000fa2000ee0df25aad526618cc47191d3a09c88`.

## Frozen full-body scope

The fixture copies the complete Oracle body, type line, costs, power/toughness, keywords and stable Oracle identity from `fixtures/card-failure-campaign/cards-20261003.json.xz`:

- Aegis of Honor — `ea93acc8-0c1f-42a2-bed3-f385d210d58f`
- Glarecaster — `9c058107-b2a1-4300-8d97-c697c248df96`
- Goblin Psychopath — `9369f131-3356-4926-9a9f-88a636305bd0`
- Hazduhr the Abbot — `35844d8b-8f68-4248-b921-e766757a9e26`
- Mirrorwood Treefolk — `fec51ab9-484f-46a8-b2b0-772a61d89e41`
- Shield Dancer — `0ca9c400-c62a-49d8-8dce-5caca7f92ed9`
- Soltari Guerrillas — `37da03c2-c03e-453c-8fbf-e1039faceb8c`

No candidate body is shortened. Goblin Psychopath retains both attack/block triggering and the losing-coin condition; Glarecaster retains flying and Soltari Guerrillas retains shadow. No card-name dispatch was added.

## Semantic owners

The named redirection clause production reads passive and active next-time clauses, optional combat restriction, source pronouns, damage-source-controller and damage-source-itself destinations, and both supported next-N duration placements. Semantic fields retain the combat restriction. The existing exact source target variant is lowered as an identity, rather than widened to an object filter. The existing object-or-player vocabulary expresses the protected source-or-controller union. Recipient-less next-time clauses protect all recipients; player filters such as an opponent remain filters, with no new choice of opponent.

The runtime matcher uses `DamageSourceConstraint::Specific` for locked sources and `Filter` for descriptive sources. It retains a complete protected-recipient union and accepts player or permanent destinations. `DamageSource` maps to the shared event source destination; the ability-source destination remains a captured object ID. Source resolution and next-N redirection share the exact target-assignment resolver. Hazduhr registers a decrementing next-N damage budget and source destination; it is not converted to all damage in one occurrence. The shared live-proposal scheduler allocates that budget by the affected player’s choice when the replacement is selected; it does not choose sources by serial input order.

`ReplacementEffectManager` has a separate next-damage-occurrence scope. Every matching sibling and split remainder in one successful simultaneous damage action can apply a registration once to its own history. It is consumed at the completed proposal boundary. Nested replacement-created damage uses a nested frame and cannot reuse a registration already claimed by its parent occurrence. Existing execution checkpoints roll back frame membership and consumption on failed or pending work. ETB batch one-shots keep their separate lifetime. The shared owner publishes original per-assignment source and cause and completed receipts once; no serial replay of the original damage is introduced.

Rules reference: [Wizards Comprehensive Rules, effective September 25, 2026](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt), rules 614.5–614.9. They require one application per event history, no redirection of zero damage, and a valid original and replacement recipient.

The shared redirection resolver preserves the original event when the destination is absent, phased out, not a damageable permanent, or a departed player. Source-controller redirection uses current controller or actual departure LKI, rather than the registration's controller. An ability can still install its captured source-destination identity after that source departed; shared event handling decides whether redirection is possible.

## Next-N allocation rule and shared owner

The [Wizards Time Spiral Remastered release notes, Outrider en-Kor](https://magic.wizards.com/en/news/feature/time-spiral-remastered-release-notes-2021-03-23) explicitly establish player selection of the source whose next 1 damage is redirected when multiple sources deal damage simultaneously; multiple shields permit independent selections. Applying that same next-N redirection rule to Hazduhr requires preserving which simultaneous source spends each part of its X budget. The corrected shared owner retains each evolving proposal and its application history, applies the affected player’s selected replacement, and allocates finite redirection or prevention using current amounts and recipients. This preserves ordering when a multiplier crosses the original budget threshold, when another redirect changes eligibility, and when multiple shields compete. Recipient-dependent shield counters are refreshed with stable structural identities. Quotas are spent only on actual application, and pending/invalid choices restore the whole damage checkpoint. CR 615.7 is a prevention rule and is not the authority used to claim redirection allocation.

The combat owner also commits every surviving assignment when a different split branch was prevented. Its aggregate prevention flag does not discard surviving damage, exclude it from recipient capacity sampling, or suppress the original source’s lifelink. That narrow correction was reviewed independently of the wider allocator.

## Compatibility

This branch starts from artifact format 5 and adopts the coordinator's current format-6 boundary. Within format 6 the additional core `combat_only` field defaults to false, matching every earlier admitted unqualified redirection payload. Existing field names and destination enum cases remain intact; `DamageSource` is an additive destination. A source-authored decode scenario removes the new flag and checks that the original payload restores unchanged. No public checkpoint or protocol payload changes are required: damage occurrence registration and frames are internal clone-backed replacement-manager state.

## Authored checks

- `crates/ironsmith-compiler-runtime/tests/bounded_damage_redirection.rs`: all seven strict frozen bodies, artifact round trips, paid activation and tap/X consumption, real attack and block triggers with both coin outcomes, exact source identity after combat changes, player/object targets, protected union with controller changes, combat and opponent filters, expiration, unavailable destination, simultaneous siblings, next occurrence, legacy payload default, structured rendering/reparse, and paid Hazduhr source allocation with changed affected controller and exact source receipts.
- `crates/ironsmith-compiler-grammar/src/grammar/effects/clause_pattern_shapes/damage_inline_tests.rs`: active/passive forms, combat qualifiers, pronoun destinations, both next-N duration positions and complete-clause rejection boundaries.
- `crates/ironsmith-engine/src/events/processing/next_damage_occurrence_tests.rs`: actual committed damage, same/different sources, original combat history and source/cause receipts, split remainder history, nested occurrence isolation, pending/error rollback, absent destination, source-controller LKI, manager cloning and ETB isolation; plus finite budget selection across either unpreventable source, multiplier ordering/threshold crossing, changed recipients, multiple shields, partial budgets, invalid allocation, prevention competition and redirected shield counters.
- `crates/ironsmith-engine/src/game_loop/combat_damage.rs`: actual combat split into prevented and surviving branches, preserving original-source damage and lifelink.

All scenarios are unrun. Only source inspection and Git whitespace checking were performed.

## Deliberately excluded

Martyrdom and Personal Incarnation still require ownership-gated activation work. Harm's Way, Kor Chant, Reflect Damage and Shining Shoal require source-choice-domain work. No coverage claim is made for those full bodies. Filtered chosen-source redirection remains explicit unsupported behavior rather than discarding the chooser filter. They must stay partial until their own complete bodies are implemented and reviewed.

## Integrated combat correction

The previously published six-card packet includes independently source-reviewed correction `f65569b89478686c20d76b0a61d0fffb7a0a2da3`. Combat now uses surviving damage assignments for excess-capacity capture and commitment even when another branch was prevented. Its actual-combat regression checks the surviving damage, original source, lifelink and consumed shield. The narrow combat correction is separate from the live Hazduhr allocation scheduler now included above. The regression remains unrun.

# Pending multi-source power damage

Status: investigation only; **zero proposed recoveries**. No build or test execution.

Exact frozen candidates:
- Coordinated Clobbering
- Friendly Rivalry
- Tandem Takedown
- Terrific Team-Up

The first, third, and fourth introduce a target set, then say “They each deal damage equal to their power.” Friendly Rivalry declares one mandatory creature target and an optional, different legendary creature target in the damage instruction itself. A flat filter allowing two ordinary creatures would be incorrect.

The existing parser `grammar/effects/clause_primitive_shapes.rs::parse_power_damage_shape` recognizes singular possessives but does not preserve a trailing distributive `each` on the source subject. The reusable semantic construction would be a source-set iteration with an individual source-relative power, not total power or one source dealing all damage.

Existing `try_compile_for_each_object_as_damage_source` lowers a single damage recipient outside the source loop, preventing the target pronoun from becoming the current source. Its runtime `ForEachObject` opens a shared simultaneous-action scope. This currently shares event provenance, but its execution loop evaluates and applies each damage effect in succession. `DealDamageEffect::prepare_simultaneous_player_action` also defers the entire effect, rather than freezing its amount. Therefore a first source's lifelink can change a later source's power, and separate processing calls cannot correctly allocate one prevention shield over the complete incoming batch.

A complete bounded repair should gather source bindings, calculated powers, recipient identities, source keyword/controller information and replacement context before application; call the existing simultaneous-damage processing function once for all sources; then apply all consequences and delayed replacement additions at the proper whole-batch boundary. `events/processing/mod.rs::SimultaneousDamageEvent` already carries per-source information and its simultaneous processor already supports shared prevention allocation. `effects/damage/deal_damage.rs::apply_processed_damage_results` currently assumes one source and immediately performs that source's lifelink and replacement additions, so it needs a deliberate multi-source owner before this family can be counted.

Authored tests should include unequal current powers, lifelink changing a second source's dynamic power, deathtouch on only one source, a shared prevention shield allocated across sources, one/more trigger grouping, legal versus illegal source targets, zero optional sources, different source controllers after responses, exact departed-source LKI without following a blink, temporary pump expiry, and Friendly Rivalry's distinct mandatory/optional target groups. Parser-only acceptance is insufficient.

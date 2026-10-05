# Aminatou, Veil Piercer: remaining complete-body dependency

This card remains unaddressed by the compound-static checkpoints. Its frozen raw body contains the upkeep surveil-2 trigger and the complete two-sentence hand grant: each enchantment card in your hand has miracle, with its own mana cost reduced by four. Removing only the registry ambiguity would not establish the card's behavior.

Source inspection identifies a concrete lifecycle gap:

1. `keyword_static/anthem_grant_lines.rs::parse_granted_alternative_cast_static` recognizes the complete miracle-cost continuation and creates `Grantable::MiracleFromCardManaCostReducedBy` through the existing grant specification. It returns a static grant only.
2. `ironsmith-engine/src/grant.rs::DerivedAlternativeCastRuntimeExt::materialize_for` materializes that grant into a reduced `AlternativeCastingMethod::Miracle` for a card in hand. It does not create the linked hand-functioning draw trigger. The native card builder's `miracle` method explicitly installs both an alternative casting method and that trigger; the grant currently supplies only the former.
3. `ironsmith-engine/src/effects/player/may_cast_miracle.rs` reads the drawn object's intrinsic `alternative_casts` for both its miracle price and the alternative-cast index. It does not consume the derived grant registry. Adding a trigger alone would therefore still fail to find the derived price.
4. The complete solution must retain the correct linked miracle price/permission through trigger resolution, including loss of the granting permanent after the trigger is created, multiple intrinsic/granted miracle instances, a card that leaves hand and returns as a new object, first-versus-later draws, reveal decline, and interruption/resume during casting/payment. It must use the native cast proposal/payment path, preserve colored symbols while reducing generic mana, avoid persistent unauthorized cast permission, and respect the exact drawn incarnation.

A coherent next checkpoint should implement the granted miracle package and its linked casting context, then make the broad subject/grant reader decline the fully proven two-sentence owner. The full-body scenarios must also exercise the upkeep surveil operation, own-controller enchantment/hand scope, first-draw reveal and casting, non-enchantment/opponent/later-draw negative cases, exact reduced payment, source-loss/multiple-cost choice and stale-incarnation gates.

No partial grammar-only success is claimed here. No builds, tests, compiler probes, formatters, corpus execution, coverage matrix edits, or publication were performed for this dependency investigation.

//! Replacement ability processor.
//!
//! This module converts static abilities into replacement effects that can be
//! registered with the `ReplacementEffectManager`.
//!
//! Per MTG rules, replacement effects can function outside the battlefield
//! when the source text says so (for example, "from anywhere" effects like
//! Darksteel Colossus). We therefore scan all objects and respect each
//! ability's functional zones instead of assuming every replacement effect
//! comes only from battlefield permanents.

use crate::ability::AbilityKind;
use crate::game_state::GameState;
use crate::replacement::ReplacementEffect;

/// Generate all replacement effects from static abilities in zones where they function.
///
/// This scans all objects for static abilities that generate replacement effects
/// and returns the corresponding `ReplacementEffect` structs.
///
/// This function is called during game state refresh to ensure that static ability
/// replacement effects are properly registered.
pub fn generate_replacement_effects_from_abilities(
    game: &GameState,
) -> Result<Vec<ReplacementEffect>, crate::static_ability_processor::StaticEffectDiscoveryError> {
    // Validate discovery before allowing the shared layer cache to serve any
    // object. Every zone remains in scope, including hidden-zone replacements.
    let mut effects = Vec::new();

    let object_ids = game.object_ids_in_deterministic_order();
    let mut characteristics = game.try_current_characteristics_batch(&object_ids)?;

    // Iterate over all objects and apply static abilities only in zones where they function.
    for object_id in object_ids {
        if let Some(object) = game.object(object_id) {
            if object.zone == crate::zone::Zone::Battlefield && game.is_phased_out(object_id) {
                continue;
            }
            let Some(chars) = characteristics.remove(&object_id) else {
                continue;
            };
            let controller = chars.controller;
            let zone = object.zone;

            // These payment methods replace every subsequent departure from
            // the stack (CR 702.34, 702.133, 702.180), including counter/bounce.
            if zone == crate::zone::Zone::Stack && object.cast_alternative_method.as_deref().is_some_and(|method|
                matches!(method, crate::alternative_cast::AlternativeCastingMethod::Flashback { .. }
                    | crate::alternative_cast::AlternativeCastingMethod::JumpStart { .. }
                    | crate::alternative_cast::AlternativeCastingMethod::Harmonize { .. })) {
                effects.push(crate::replacement::ZoneReplacementSpec::new(
                    crate::target::ObjectFilter::specific(object_id), crate::zone::Zone::Exile)
                    .from_zone(crate::zone::Zone::Stack).build(object_id, controller));
            } else if zone == crate::zone::Zone::Stack
                && object.cast_alternative_method.as_deref().is_some_and(|method| {
                    matches!(
                        method,
                        crate::alternative_cast::AlternativeCastingMethod::FromZone {
                            exiles_after_resolution: true,
                            ..
                        }
                    )
                })
            {
                // Aftermath (CR 702.127a) and "if a spell cast this way would be
                // put into your graveyard, exile it instead" grants: a countered
                // or fizzled spell is exiled too, not only a resolved one.
                // Aftermath exiles the half "instead of putting it anywhere
                // else any time it would leave the stack" (a bounce too); the
                // grants only replace the move to the graveyard.
                let has_aftermath = object.abilities.iter().any(|ability| {
                    matches!(
                        &ability.kind,
                        AbilityKind::Static(static_ability)
                            if static_ability
                                .compiled_model()
                                .is_some_and(|model| model.label == "Aftermath")
                    )
                });
                let spec = crate::replacement::ZoneReplacementSpec::new(
                    crate::target::ObjectFilter::specific(object_id), crate::zone::Zone::Exile)
                    .from_zone(crate::zone::Zone::Stack);
                let spec = if has_aftermath {
                    spec
                } else {
                    spec.to_zone(crate::zone::Zone::Graveyard)
                };
                effects.push(spec.build(object_id, controller));
            }

            // Carry calculated occurrence origins, not just static value ids.
            // Printed, layer-granted and registered temporary abilities can
            // contain equal/cloned values while remaining independent effects.
            for (slot, ability) in chars.abilities.iter().enumerate() {
                let AbilityKind::Static(static_ability) = &ability.kind else {
                    continue;
                };
                let Some(origin) = chars.abilities.origin(slot).cloned() else {
                    continue;
                };
                let temporary = if let crate::continuous::AbilityOrigin::Temporary(token) = &origin
                {
                    if !object
                        .temporary_static_ability_grants
                        .iter()
                        .enumerate()
                        .any(|(index, grant)| {
                            object.temporary_static_ability_grants.origin(index) == Some(token)
                                && !grant.is_expired(game.turn.turn_number)
                        })
                    {
                        continue;
                    }
                    true
                } else {
                    false
                };
                // Stack cast-this-way riders are registered temporary abilities;
                // their event matcher defines scope, not printed battlefield zones.
                if !temporary && !ability.functions_in(&zone) {
                    continue;
                }
                if !static_ability.is_active(game, object_id) {
                    continue;
                }
                let face = matches!(&origin, crate::continuous::AbilityOrigin::Printed(_))
                    .then_some(object.card)
                    .flatten();
                if let Some(effect) =
                    static_ability.generate_replacement_effect(object_id, controller)
                {
                    effects.push(effect.with_ability_origin(origin.clone(), face, 0));
                }
                // Ability-grant instructions are not replacement sources.
                // Existing recipients are scanned through calculated abilities;
                // an entrant's self-entry ability is read by the prospective
                // entry driver. Projecting the grant itself duplicates those
                // occurrences and incorrectly activates general replacements
                // before any recipient is on the battlefield (CR 614.12).
            }
        }
    }

    Ok(effects)
}

#[cfg(test)]
mod tests {
    use super::generate_replacement_effects_from_abilities;
    use crate::cards::CardDefinitionBuilder;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::cards::basic_island;
    use crate::continuous::Modification;
    use crate::effect::{Until, Value};
    use crate::effects::{ApplyContinuousEffect, EffectExecutor, ExecutionContext};
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::ids::{ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::CounterType;
    use crate::replacement::ReplacementAction;
    use crate::static_abilities::StaticAbility;
    use crate::target::{ChooseSpec, ObjectFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn test_enters_tapped_generates_replacement() {
        let ability = StaticAbility::enters_tapped_ability();
        let effect =
            ability.generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0));

        assert!(effect.is_some());
        let effect = effect.unwrap();
        assert_eq!(effect.priority_override, None);
        // Now using trait-based matcher instead of ReplacementCondition enum
        assert!(
            effect.matcher.is_some(),
            "EntersTapped should use a trait-based matcher"
        );
        assert!(matches!(effect.replacement, ReplacementAction::EnterTapped));
    }

    #[test]
    fn test_flying_does_not_generate_replacement() {
        let ability = StaticAbility::flying();
        let effect =
            ability.generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0));

        assert!(effect.is_none());
    }

    #[test]
    fn counter_removal_prevention_activates_only_while_source_has_counter() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::prevent_damage_to_self_remove_counter(
                CounterType::PlusOnePlusOne,
                crate::effect::Value::EventValue(crate::effect::EventValueSpec::Amount),
            )
            .with_condition(crate::effect::Condition::SourceHasCounterAtLeast {
                counter_type: CounterType::PlusOnePlusOne,
                count: 1,
                surface: ironsmith_core::SourceCounterThresholdSurface::SourceHas,
            });
        let definition = CardDefinitionBuilder::new(CardId::new(), "Conditional Counter Shield")
            .card_types(vec![CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(
                StaticAbility::from_model(model),
            ))
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        // Exercise the validated shared cache, then invalidate it with counter
        // changes; a cached ability must neither survive nor miss its condition.
        game.prewarm_calculated_characteristics(&[source]);

        assert!(
            generate_replacement_effects_from_abilities(&game)
                .unwrap()
                .iter()
                .all(|effect| effect.source != source),
            "the prevention replacement must be inactive without the required counter"
        );

        game.add_counters(source, CounterType::PlusOnePlusOne, 1);
        game.refresh_continuous_state().unwrap();
        let replacements = generate_replacement_effects_from_abilities(&game).unwrap();
        let replacement = replacements
            .iter()
            .find(|effect| effect.source == source)
            .expect("the prevention replacement should activate once the source has a counter");
        assert!(matches!(
            replacement.replacement,
            ReplacementAction::PreventDamageThenFromProposedAmount(_)
        ));

        game.remove_counters(source, CounterType::PlusOnePlusOne, 1, None, None);
        game.refresh_continuous_state().unwrap();
        assert!(
            generate_replacement_effects_from_abilities(&game)
                .unwrap()
                .iter()
                .all(|effect| effect.source != source),
            "the prevention replacement must deactivate after the last counter is removed"
        );
    }

    #[test]
    fn dirty_replacement_batch_matches_individual_queries_and_refresh() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let conditional = ironsmith_core::StaticAbility::prevent_damage_to_self_remove_counter(
            CounterType::PlusOnePlusOne,
            crate::effect::Value::EventValue(crate::effect::EventValueSpec::Amount),
        )
        .with_condition(crate::effect::Condition::SourceHasCounterAtLeast {
            counter_type: CounterType::PlusOnePlusOne,
            count: 1,
            surface: ironsmith_core::SourceCounterThresholdSurface::SourceHas,
        });
        let definition = CardDefinitionBuilder::new(CardId::new(), "Conditional batch recipient")
            .card_types(vec![CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(
                StaticAbility::from_model(conditional),
            ))
            .with_ability(crate::ability::Ability::static_ability(
                StaticAbility::changeling(),
            ))
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for zone in [Zone::Hand, Zone::Library, Zone::Graveyard, Zone::Exile] {
            game.create_object_from_definition(&definition, alice, zone);
        }
        game.refresh_continuous_state().unwrap();
        let check = |game: &GameState| {
            let mut ids = game.object_ids_in_deterministic_order();
            ids.push(ObjectId::from_raw(999999));
            let batch = game.try_current_characteristics_batch(&ids).unwrap();
            for id in ids {
                let individual = game.try_current_characteristics(id).unwrap();
                assert_eq!(
                    format!("{:?}", batch.get(&id)),
                    format!("{:?}", individual.as_ref()),
                    "batch must preserve fallible single-object characteristics for {id:?}"
                );
            }
            let dirty = generate_replacement_effects_from_abilities(game).unwrap();
            let mut refreshed = game.clone();
            refreshed.refresh_continuous_state().unwrap();
            let clean = generate_replacement_effects_from_abilities(&refreshed).unwrap();
            assert_eq!(format!("{dirty:?}"), format!("{clean:?}"));
            dirty.iter().any(|effect| effect.source == source)
        };
        assert!(!check(&game));
        game.add_counters(source, CounterType::PlusOnePlusOne, 1);
        assert!(!game.continuous_state_is_clean());
        assert!(check(&game));
        let active_branch = game.clone();
        game.remove_counters(source, CounterType::PlusOnePlusOne, 1, None, None);
        assert!(!check(&game));
        assert!(check(&active_branch));
        game.add_counters(source, CounterType::PlusOnePlusOne, 1);
        game.phase_out(source);
        assert!(!check(&game));
    }

    #[test]
    fn test_shuffle_into_library_generates_replacement() {
        let ability = StaticAbility::shuffle_into_library_from_graveyard();
        let effect =
            ability.generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0));

        assert!(effect.is_some());
        let effect = effect.unwrap();
        assert_eq!(effect.priority_override, None);
        // Now using trait-based matcher instead of ReplacementCondition enum
        assert!(
            effect.matcher.is_some(),
            "ShuffleIntoLibraryFromGraveyard should use a trait-based matcher"
        );
        assert!(matches!(
            effect.replacement,
            ReplacementAction::ChangeDestination(Zone::Library)
        ));
    }

    fn grant_dynamic_entry_counters(
        game: &mut GameState,
        source: ObjectId,
        controller: PlayerId,
        target: ObjectId,
        count: Value,
    ) {
        let model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::enters_with_counters_and_subtypes_for_filter(
                ObjectFilter::source(),
                CounterType::PlusOnePlusOne,
                count,
                Vec::new(),
            );
        let granted = StaticAbility::from_model(model);
        let apply = ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(target),
            Modification::AddAbility(granted),
            Until::Forever,
        );
        let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, controller, &mut decision_maker);
        apply
            .execute(game, &mut ctx)
            .expect("the entry-counter ability grant should resolve");
    }

    #[test]
    fn resolution_granted_entry_counter_ability_uses_two_dynamic_mana_values() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let source = CardDefinitionBuilder::new(CardId::new(), "Entry Counter Grant Source")
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);

        for (mana_value, expected_counters) in [(6_u8, 2_u32), (9_u8, 5_u32)] {
            let creature = CardDefinitionBuilder::new(
                CardId::new(),
                format!("Dynamic Entry Creature {mana_value}"),
            )
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
                mana_value,
            )]]))
            .build();
            let creature_id = game.create_object_from_definition(&creature, alice, Zone::Stack);

            let count = Value::Add(
                Box::new(Value::ManaValueOf(Box::new(
                    ChooseSpec::Source.with_surface_hint(
                        ironsmith_core::ChooseSpecSurfaceHint::SourceReference(
                            ironsmith_core::SourceReferenceSurface::ThisPermanentType(
                                "it".to_string(),
                            ),
                        ),
                    ),
                ))),
                Box::new(Value::Fixed(-4)),
            );
            grant_dynamic_entry_counters(&mut game, source_id, alice, creature_id, count);

            let entered = game
                .move_object_with_etb_processing(creature_id, Zone::Battlefield)
                .expect("replacement operation must execute successfully in this scenario")
                .assert_completed_without_additions()
                .expect("creature with a resolution-granted ETB ability should enter")
                .new_id;
            assert_eq!(
                game.counter_count(entered, CounterType::PlusOnePlusOne),
                expected_counters,
                "mana value {mana_value} should produce X = {expected_counters}, not a fixed one"
            );
        }
    }

    #[test]
    fn resolution_granted_entry_counter_ability_reads_outer_source_counters_at_two_values() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let source = CardDefinitionBuilder::new(CardId::new(), "Ingredient Source")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
        let ingredient = CounterType::Named("ingredient".into());

        for (additional_source_counters, expected_counters) in [(2_u32, 2_u32), (3_u32, 5_u32)] {
            game.add_counters(source_id, ingredient, additional_source_counters);
            let creature = CardDefinitionBuilder::new(
                CardId::new(),
                format!("Ingredient Entry Creature {expected_counters}"),
            )
            .card_types(vec![CardType::Creature])
            .build();
            let creature_id = game.create_object_from_definition(&creature, alice, Zone::Stack);
            grant_dynamic_entry_counters(
                &mut game,
                source_id,
                alice,
                creature_id,
                Value::CountersOnSource(ingredient),
            );

            let entered = game
                .move_object_with_etb_processing(creature_id, Zone::Battlefield)
                .expect("replacement operation must execute successfully in this scenario")
                .assert_completed_without_additions()
                .expect("creature with a source-counter-based ETB ability should enter")
                .new_id;
            assert_eq!(
                game.counter_count(entered, CounterType::PlusOnePlusOne),
                expected_counters,
                "the granted ability should read {expected_counters} counters from its outer source"
            );
        }
    }

    #[test]
    fn resolution_granted_entry_counter_ability_counts_two_distinct_color_totals() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let source = CardDefinitionBuilder::new(CardId::new(), "Color Entry Grant Source")
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);

        for colors in [2_u32, 4_u32] {
            let creature =
                CardDefinitionBuilder::new(CardId::new(), format!("Color Entry Creature {colors}"))
                    .card_types(vec![CardType::Creature])
                    .build();
            let creature_id = game.create_object_from_definition(&creature, alice, Zone::Stack);
            let mut spent = crate::player::ManaPool::new();
            for symbol in [
                ManaSymbol::White,
                ManaSymbol::Blue,
                ManaSymbol::Black,
                ManaSymbol::Red,
            ]
            .into_iter()
            .take(colors as usize)
            {
                spent.add(symbol, 1);
            }
            game.object_mut(creature_id)
                .expect("creature spell should exist")
                .mana_spent_to_cast = spent;
            grant_dynamic_entry_counters(
                &mut game,
                source_id,
                alice,
                creature_id,
                Value::ColorsOfManaSpentToCastThisSpell,
            );

            let entered = game
                .move_object_with_etb_processing(creature_id, Zone::Battlefield)
                .expect("replacement operation must execute successfully in this scenario")
                .assert_completed_without_additions()
                .expect("creature with a color-count-based ETB ability should enter")
                .new_id;
            assert_eq!(
                game.counter_count(entered, CounterType::PlusOnePlusOne),
                colors,
                "{colors} colors of spent mana should produce {colors} counters"
            );
        }
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_generate_replacements_respects_nonbattlefield_functional_zones() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

        let darksteel = CardDefinitionBuilder::new(CardId::new(), "Darksteel Test")
            .card_types(vec![
                crate::types::CardType::Artifact,
                crate::types::CardType::Creature,
            ])
            .shuffle_into_library_from_graveyard()
            .build();
        let island = basic_island();

        let darksteel_id = game.create_object_from_definition(&darksteel, alice, Zone::Hand);
        game.create_object_from_definition(&island, alice, Zone::Battlefield);

        let effects = generate_replacement_effects_from_abilities(&game).unwrap();
        assert!(
            effects.iter().any(|effect| {
                effect.source == darksteel_id
                    && matches!(
                        effect.replacement,
                        ReplacementAction::ChangeDestination(Zone::Library)
                    )
            }),
            "expected nonbattlefield shuffle replacement to be generated from hand"
        );
    }

    #[test]
    fn continuous_grants_create_replacements_only_on_actual_ability_recipients() {
        let mut observations = Vec::new();
        for self_only in [false, true] {
            for source_is_creature in [false, true] {
                for existing_recipients in [0, 1] {
                    let alice = PlayerId::from_index(0);
                    let mut game = GameState::new(vec!["Alice".into()], 20);
                    let grant = if self_only {
                        StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, 1)
                    } else {
                        StaticAbility::enters_with_counters_for_filter(
                            ObjectFilter::creature(),
                            CounterType::PlusOnePlusOne,
                            1,
                        )
                    };
                    let source = CardDefinitionBuilder::new(CardId::new(), "Ability grant source")
                        .card_types(vec![if source_is_creature {
                            CardType::Creature
                        } else {
                            CardType::Enchantment
                        }])
                        .with_ability(crate::ability::Ability::static_ability(
                            StaticAbility::grant_ability(ObjectFilter::creature(), grant),
                        ))
                        .build();
                    game.create_object_from_definition(&source, alice, Zone::Battlefield);
                    let creature = CardDefinitionBuilder::new(CardId::new(), "Ability recipient")
                        .card_types(vec![CardType::Creature])
                        .build();
                    for _ in 0..existing_recipients {
                        game.create_object_from_definition(&creature, alice, Zone::Battlefield);
                    }
                    let entrant = game.create_object_from_definition(&creature, alice, Zone::Hand);
                    let entered = crate::tests::test_helpers::enter_fixture(
                        &mut game,
                        entrant,
                        "granted ability recipient should enter",
                    );
                    let actual = game.counter_count(entered.new_id, CounterType::PlusOnePlusOne);
                    let expected = if self_only {
                        1
                    } else {
                        u32::from(source_is_creature) + existing_recipients
                    };
                    observations.push((
                        self_only,
                        source_is_creature,
                        existing_recipients,
                        actual,
                        expected,
                    ));
                }
            }
        }
        assert!(
            observations
                .iter()
                .all(|(_, _, _, actual, expected)| actual == expected),
            "self-entry abilities apply on the entrant; global replacements apply once per existing recipient: {observations:?}"
        );
    }

    #[test]
    fn entering_source_global_counter_ability_does_not_modify_its_own_entry() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Global entry grant")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let entering = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let model = ironsmith_core::StaticAbility::enters_with_counters_and_subtypes_for_filter(
            ObjectFilter::creature(),
            CounterType::PlusOnePlusOne,
            Value::Fixed(2),
            Vec::new(),
        );
        let apply = ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(entering),
            Modification::AddAbility(StaticAbility::from_model(model)),
            Until::Forever,
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        apply.execute(&mut game, &mut ctx).unwrap();
        let entered = game
            .move_object_with_etb_processing(entering, Zone::Battlefield)
            .expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions()
            .unwrap()
            .new_id;
        assert_eq!(
            game.counter_count(entered, CounterType::PlusOnePlusOne),
            0,
            "CR 614.12: a general subset replacement on the entrant cannot modify its own entry"
        );
        let later = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let later = game
            .move_object_with_etb_processing(later, Zone::Battlefield)
            .expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions()
            .unwrap()
            .new_id;
        assert_eq!(
            game.counter_count(later, CounterType::PlusOnePlusOne),
            2,
            "the same granted ability must affect another creature once its source is on the battlefield"
        );
    }

    #[test]
    fn resolved_spell_characteristic_effects_survive_resolution_but_not_bounce() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Modified spell")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut ctx = ExecutionContext::new_default(source, alice);
        for modification in [
            Modification::SetPower {
                value: Value::Fixed(7),
                sublayer: crate::continuous::PtSublayer::Setting,
            },
            Modification::ChangeController(bob),
            Modification::AddAbility(StaticAbility::flying()),
        ] {
            ApplyContinuousEffect::with_spec(
                ChooseSpec::SpecificObject(spell),
                modification,
                Until::Forever,
            )
            .execute(&mut game, &mut ctx)
            .unwrap();
        }
        let permanent = game
            .move_object_with_etb_processing(spell, Zone::Battlefield)
            .expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions()
            .unwrap()
            .new_id;
        assert_eq!(game.calculated_power(permanent), Some(7));
        assert_eq!(game.current_controller(permanent), Some(bob));
        assert!(game.object_has_static_ability_id(
            permanent,
            crate::static_abilities::StaticAbilityId::Flying
        ));
        let hand = game.move_object_by_effect(permanent, Zone::Hand).unwrap();
        let returned = game
            .move_object_with_etb_processing(hand, Zone::Battlefield)
            .expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions()
            .unwrap()
            .new_id;
        assert_eq!(game.calculated_power(returned), Some(2));
        assert_eq!(game.current_controller(returned), Some(alice));
        assert!(!game.object_has_static_ability_id(
            returned,
            crate::static_abilities::StaticAbilityId::Flying
        ));
    }
}

#[cfg(test)]
mod independent_occurrence_gameplay_tests {
    use super::*;
    use crate::ability::Ability;
    use crate::effects::EffectExecutor;
    #[test]
    fn cloned_replacement_occurrences_survive_refresh_expiry_and_control_changes() {
        for temporary in [false, true] {
            for layered in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let bob = game.players[1].id;
                let card = crate::card::CardBuilder::new(
                    crate::ids::CardId::new(),
                    "Independent ability source",
                )
                .card_types(vec![crate::types::CardType::Artifact])
                .build();
                let source =
                    game.create_object_from_card(&card, alice, crate::zone::Zone::Battlefield);
                let haste = crate::static_abilities::StaticAbility::haste();
                let doubling =
                    crate::static_abilities::StaticAbility::double_life_change_replacement(
                        crate::target::PlayerFilter::You,
                        false,
                        "Double life gain",
                    );
                let turn = game.turn.turn_number;
                if temporary {
                    for (ability, expiry) in [
                        (haste.clone(), turn),
                        (doubling.clone(), turn + 1),
                        (doubling.clone(), turn + 1),
                    ] {
                        game.object_mut(source)
                            .unwrap()
                            .temporary_static_ability_grants
                            .push(crate::object::TemporaryStaticAbilityGrant {
                                ability: ability.id(),
                                ability_payload: Some(ability),
                                expires_end_of_turn: Some(expiry),
                            });
                    }
                } else {
                    for ability in [haste, doubling.clone(), doubling.clone()] {
                        game.object_mut(source)
                            .unwrap()
                            .abilities_mut()
                            .push(Ability::static_ability(ability));
                    }
                }
                if layered {
                    game.effect_store.continuous_effects.add_effect(
                        crate::continuous::ContinuousEffect::new(
                            source,
                            alice,
                            crate::continuous::EffectTarget::Specific(source),
                            crate::continuous::Modification::RemoveStaticAbilityFamily(
                                crate::static_abilities::StaticAbilityId::Haste,
                            ),
                        )
                        .until(crate::effect::Until::Forever),
                    );
                }
                let keys = |game: &GameState| {
                    generate_replacement_effects_from_abilities(game)
                        .unwrap()
                        .into_iter()
                        .map(|effect| effect.application_key())
                        .collect::<std::collections::HashSet<_>>()
                };
                let before = keys(&game);
                assert_eq!(before.len(), 2);
                game.refresh_continuous_state();
                assert_eq!(keys(&game), before);
                let mut ctx = crate::effects::EffectContext::new_default(source, alice);
                let outcome = crate::effects::GainLifeEffect::you(1)
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                assert_eq!(game.player(alice).unwrap().life, 24);
                assert_eq!(outcome.count_or_zero(), 4);
                assert_eq!(outcome.events.len(), 1);
                game.cleanup_temporary_object_static_ability_grants_end_of_turn();
                assert_eq!(
                    keys(&game),
                    before,
                    "expiry of unrelated older grant cannot renumber survivors"
                );
                assert_eq!(keys(&game.clone()), before);
                game.set_current_controller(source, bob)
                    .expect("finite controller fixture must refresh successfully");
                assert_eq!(
                    keys(&game),
                    before,
                    "controller changes keep the same occurrences"
                );
                let mut ctx = crate::effects::EffectContext::new_default(source, bob);
                let outcome = crate::effects::GainLifeEffect::you(1)
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                assert_eq!(game.player(bob).unwrap().life, 24);
                assert_eq!(outcome.count_or_zero(), 4);
                assert_eq!(outcome.events.len(), 1);
            }
        }
    }
    #[test]
    fn cloned_entry_replacement_occurrences_each_add_their_counter() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Independent entry source")
                .card_types(vec![crate::types::CardType::Artifact])
                .build();
        let object = game.create_object_from_card(&card, alice, crate::zone::Zone::Hand);
        let stable = game.object(object).unwrap().stable_id;
        let ability = crate::static_abilities::StaticAbility::enters_with_counters(
            crate::object::CounterType::PlusOnePlusOne,
            1,
        );
        for _ in 0..2 {
            game.object_mut(object)
                .unwrap()
                .abilities_mut()
                .push(Ability::static_ability(ability.clone()));
        }
        let mut ctx = crate::effects::EffectContext::new_default(object, alice);
        crate::effects::MoveToZoneEffect::new(
            crate::target::ChooseSpec::SpecificObject(object),
            crate::zone::Zone::Battlefield,
            false,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        let object = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(object).unwrap().zone,
            crate::zone::Zone::Battlefield
        );
        assert_eq!(
            game.counter_count(object, crate::object::CounterType::PlusOnePlusOne),
            2
        );
    }
}

#[cfg(test)]
mod continuous_parent_occurrence_gameplay_tests {
    use super::*;
    use crate::ability::Ability;
    use crate::effects::EffectExecutor;
    #[test]
    fn independent_conditional_grant_parents_survive_nested_materialization_and_refresh() {
        for (depth, temporary) in [1, 2, 3]
            .into_iter()
            .flat_map(|depth| [false, true].map(|temporary| (depth, temporary)))
        {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let card = crate::card::CardBuilder::new(
                crate::ids::CardId::new(),
                "Nested replacement grant",
            )
            .card_types(vec![crate::types::CardType::Artifact])
            .build();
            let source = game.create_object_from_card(&card, alice, crate::zone::Zone::Battlefield);
            let mut model: crate::static_abilities::CompiledStaticAbility =
                ironsmith_core::StaticAbility::double_life_change_replacement(
                    crate::target::PlayerFilter::You,
                    false,
                    "Double life gain",
                );
            for _ in 0..depth {
                model = ironsmith_core::StaticAbility::grant_object_ability_for_filter(
                    crate::target::ObjectFilter::source(),
                    ironsmith_core::Ability::static_ability(model),
                    "Source has the granted ability",
                );
            }
            model = model.with_condition(crate::effect::Condition::SourceHasCounterAtLeast {
                counter_type: crate::object::CounterType::PlusOnePlusOne,
                count: 1,
                surface: ironsmith_core::SourceCounterThresholdSurface::SourceHas,
            });
            let parent = crate::static_abilities::StaticAbility::from_model(model);
            for _ in 0..2 {
                if temporary {
                    game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
                        source,
                        parent.id(),
                        Some(parent.clone()),
                    );
                } else {
                    game.object_mut(source)
                        .unwrap()
                        .abilities_mut()
                        .push(Ability::static_ability(parent.clone()));
                }
            }
            let keys = |game: &GameState| {
                generate_replacement_effects_from_abilities(game)
                    .unwrap()
                    .into_iter()
                    .map(|effect| effect.application_key())
                    .collect::<std::collections::HashSet<_>>()
            };
            assert!(keys(&game).is_empty());
            game.add_counters(source, crate::object::CounterType::PlusOnePlusOne, 1);
            let before = keys(&game);
            assert_eq!(before.len(), 2, "depth={depth}");
            for _ in 0..3 {
                game.refresh_continuous_state();
                assert_eq!(keys(&game), before);
            }
            assert_eq!(keys(&game.clone()), before);
            let mut ctx = crate::effects::EffectContext::new_default(source, alice);
            let outcome = crate::effects::GainLifeEffect::you(1)
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(game.player(alice).unwrap().life, 24);
            assert_eq!(outcome.count_or_zero(), 4);
            assert_eq!(outcome.events.len(), 1);
            game.set_current_controller(source, bob)
                .expect("finite controller fixture must refresh successfully");
            assert_eq!(
                keys(&game),
                before,
                "controller binding cannot rename parent occurrence"
            );
            let mut ctx = crate::effects::EffectContext::new_default(source, bob);
            let outcome = crate::effects::GainLifeEffect::you(1)
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(game.player(bob).unwrap().life, 24);
            assert_eq!(outcome.count_or_zero(), 4);
            assert_eq!(outcome.events.len(), 1);
            game.remove_counters(
                source,
                crate::object::CounterType::PlusOnePlusOne,
                1,
                None,
                None,
            );
            assert!(keys(&game).is_empty());
            game.add_counters(source, crate::object::CounterType::PlusOnePlusOne, 1);
            assert_eq!(
                keys(&game),
                before,
                "reactivating parent keeps its occurrence"
            );
            if temporary {
                game.cleanup_temporary_object_static_ability_grants_end_of_turn();
                assert!(
                    keys(&game).is_empty(),
                    "expired generating grants leave no descendant replacement"
                );
            }
        }
    }
}

#[cfg(test)]
mod registered_continuous_occurrence_gameplay_tests {
    use super::*;
    use crate::effects::EffectExecutor;
    #[test]
    fn registered_grants_keep_distinct_keys_through_expiry_and_reregistration() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Registered grant source")
                .card_types(vec![crate::types::CardType::Artifact])
                .build();
        let source = game.create_object_from_card(&card, alice, crate::zone::Zone::Battlefield);
        let older = crate::continuous::ContinuousEffect::from_resolution(
            source,
            alice,
            vec![source],
            crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::haste(),
            ),
        )
        .until(crate::effect::Until::EndOfTurn);
        game.effect_store.continuous_effects.add_effect(older);
        let doubling = crate::static_abilities::StaticAbility::double_life_change_replacement(
            crate::target::PlayerFilter::You,
            false,
            "Double life gain",
        );
        let mut descriptor = crate::continuous::ContinuousEffect::from_resolution(
            source,
            alice,
            vec![source],
            crate::continuous::Modification::AddAbility(doubling),
        );
        descriptor.timestamp = 7;
        // Retained generation metadata cannot merge new registrations.
        descriptor.originating_ability =
            Some(Box::new(crate::continuous::ContinuousAbilityOrigin {
                host: source,
                ability: crate::continuous::AbilityOrigin::Printed(0),
                printed_face: game.object(source).unwrap().card,
                branch: 0,
            }));
        let first = game
            .effect_store
            .continuous_effects
            .add_effect(descriptor.clone());
        let second = game.effect_store.continuous_effects.add_effect(descriptor);
        let keys = |game: &GameState| {
            generate_replacement_effects_from_abilities(game)
                .unwrap()
                .into_iter()
                .map(|effect| effect.application_key())
                .collect::<std::collections::HashSet<_>>()
        };
        let before = keys(&game);
        assert_eq!(before.len(), 2);
        game.refresh_continuous_state();
        assert_eq!(keys(&game), before);
        assert_eq!(keys(&game.clone()), before);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        let outcome = crate::effects::GainLifeEffect::you(1)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 24);
        assert_eq!(outcome.count_or_zero(), 4);
        assert_eq!(outcome.events.len(), 1);
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        assert_eq!(
            keys(&game),
            before,
            "removing an older unrelated registration cannot renumber survivors"
        );
        let checkpoint = game.clone();
        game.effect_store.continuous_effects.remove_effect(first);
        let survivor = keys(&game);
        assert_eq!(survivor.len(), 1);
        assert!(survivor.is_subset(&before));
        assert_eq!(keys(&checkpoint), before);
        let outcome = crate::effects::GainLifeEffect::you(1)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 26);
        assert_eq!(outcome.count_or_zero(), 2);
        assert_eq!(outcome.events.len(), 1);
        let clone = game
            .effect_store
            .continuous_effects
            .effects()
            .iter()
            .find(|effect| effect.id == second)
            .unwrap()
            .clone();
        let third = game.effect_store.continuous_effects.add_effect(clone);
        assert_ne!(first, third);
        assert_ne!(second, third);
        let restored = keys(&game);
        assert_eq!(restored.len(), 2);
        assert_eq!(
            restored.intersection(&before).count(),
            1,
            "re-registration creates a fresh effect"
        );
        let outcome = crate::effects::GainLifeEffect::you(1)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 30);
        assert_eq!(outcome.count_or_zero(), 4);
        assert_eq!(outcome.events.len(), 1);
    }
}

#[cfg(test)]
mod surviving_level_parent_gameplay_tests {
    use super::*;
    use crate::{
        ability::{Ability, LevelAbility},
        card::{CardBuilder, PowerToughness},
        continuous::{ContinuousEffect, Modification, TextBoxOverlay},
        effects::EffectExecutor,
        ids::{CardId, ObjectId, PlayerId},
        object::CounterType,
        static_abilities::{CompiledStaticAbility, StaticAbility},
        target::PlayerFilter,
        types::CardType,
        zone::Zone,
    };
    use std::collections::HashSet;

    fn fixture() -> (GameState, ObjectId, PlayerId, StaticAbility) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::new(), "Level replacement recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .counters
            .insert(CounterType::Level, 1);
        let leaf =
            StaticAbility::from_model(CompiledStaticAbility::double_life_change_replacement(
                PlayerFilter::You,
                false,
                "Double life gain",
            ));
        let parent = StaticAbility::with_level_abilities(vec![LevelAbility {
            min_level: 1,
            max_level: None,
            power_toughness: Some((7, 7)),
            abilities: vec![leaf],
        }]);
        (game, source, alice, parent)
    }

    fn keys(game: &GameState) -> HashSet<crate::replacement::ReplacementEffectKey> {
        generate_replacement_effects_from_abilities(game)
            .unwrap()
            .into_iter()
            .map(|effect| effect.application_key())
            .collect()
    }

    #[test]
    fn selective_loss_overwrite_and_regrant_use_surviving_level_description() {
        for mode in 0..7 {
            let (mut game, source, alice, parent) = fixture();
            game.object_mut(source)
                .unwrap()
                .abilities_mut()
                .push(Ability::static_ability(parent.clone()));
            let modification = match mode {
                0 => None,
                1 | 6 => Some(Modification::RemoveAllAbilities),
                2 => Some(Modification::RemoveAbility(parent.clone())),
                3 => Some(Modification::RemoveStaticAbilityFamily(parent.id())),
                4 => Some(Modification::SetAbilities(vec![Ability::static_ability(
                    StaticAbility::haste(),
                )])),
                5 => Some(Modification::SetTextBox(TextBoxOverlay::new(
                    "Haste",
                    vec![Ability::static_ability(StaticAbility::haste())],
                ))),
                _ => unreachable!(),
            };
            if let Some(modification) = modification {
                game.effect_store
                    .continuous_effects
                    .add_effect(ContinuousEffect::from_resolution(
                        source,
                        alice,
                        vec![source],
                        modification,
                    ));
            }
            if mode == 6 {
                game.effect_store
                    .continuous_effects
                    .add_effect(ContinuousEffect::from_resolution(
                        source,
                        alice,
                        vec![source],
                        Modification::AddAbility(parent),
                    ));
            }
            let active = mode == 0 || mode == 6;
            let expected_pt = if active { 7 } else { 2 };
            let before = keys(&game);
            assert_eq!(before.len(), usize::from(active), "mode={mode}");
            for refreshed in [false, true] {
                if refreshed {
                    game.refresh_continuous_state();
                }
                let chars = game.current_characteristics(source).unwrap();
                assert_eq!(
                    (chars.power, chars.toughness),
                    (Some(expected_pt), Some(expected_pt)),
                    "mode={mode},refreshed={refreshed}"
                );
                assert_eq!(keys(&game), before);
            }
            assert_eq!(keys(&game.clone()), before);
            let mut ctx = crate::effects::EffectContext::new_default(source, alice);
            let outcome = crate::effects::GainLifeEffect::you(1)
                .execute(&mut game, &mut ctx)
                .unwrap();
            let gain = if active { 2 } else { 1 };
            assert_eq!(game.player(alice).unwrap().life, 20 + gain, "mode={mode}");
            assert_eq!(outcome.count_or_zero(), i64::from(gain));
            assert_eq!(outcome.events.len(), 1);
        }
    }

    #[test]
    fn independent_level_description_parents_preserve_keys_through_refresh_and_lifetime() {
        for temporary in [false, true] {
            let (mut game, source, alice, parent) = fixture();
            let bob = game.players[1].id;
            if temporary {
                let haste = StaticAbility::haste();
                game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
                    source,
                    haste.id(),
                    Some(haste),
                );
            }
            for _ in 0..2 {
                if temporary {
                    game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
                        source,
                        parent.id(),
                        Some(parent.clone()),
                    );
                } else {
                    game.object_mut(source)
                        .unwrap()
                        .abilities_mut()
                        .push(Ability::static_ability(parent.clone()));
                }
            }
            let before = keys(&game);
            assert_eq!(before.len(), 2, "temporary={temporary}");
            for _ in 0..3 {
                game.refresh_continuous_state();
                assert_eq!(keys(&game), before);
            }
            assert_eq!(keys(&game.clone()), before);
            if temporary {
                game.object_mut(source)
                    .unwrap()
                    .temporary_static_ability_grants
                    .retain(|grant| grant.ability != StaticAbility::haste().id());
                assert_eq!(
                    keys(&game),
                    before,
                    "removing an earlier grant cannot renumber level parents"
                );
            }
            let mut ctx = crate::effects::EffectContext::new_default(source, alice);
            let outcome = crate::effects::GainLifeEffect::you(1)
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(game.player(alice).unwrap().life, 24);
            assert_eq!(outcome.count_or_zero(), 4);
            assert_eq!(outcome.events.len(), 1);
            game.set_current_controller(source, bob)
                .expect("finite controller fixture must refresh successfully");
            assert_eq!(keys(&game), before);
            let mut ctx = crate::effects::EffectContext::new_default(source, bob);
            let outcome = crate::effects::GainLifeEffect::you(1)
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(game.player(bob).unwrap().life, 24);
            assert_eq!(outcome.count_or_zero(), 4);
            assert_eq!(outcome.events.len(), 1);
            if temporary {
                game.cleanup_temporary_object_static_ability_grants_end_of_turn();
                assert!(keys(&game).is_empty());
            }
        }
    }
}

#[cfg(test)]
mod independent_static_copy_gameplay_tests {
    use super::generate_replacement_effects_from_abilities;
    use crate::{
        ability::Ability,
        card::CardBuilder,
        continuous::{ContinuousEffect, Modification},
        effects::EffectExecutor,
        game_state::GameState,
        ids::CardId,
        static_abilities::{
            CompiledStaticAbility, CopyStaticAbilityVariants, StaticAbility, StaticAbilityId,
        },
        target::{ObjectFilter, PlayerFilter},
        types::CardType,
        zone::Zone,
    };

    #[test]
    fn independent_static_copy_instructions_preserve_variant_occurrences_and_lifetime() {
        for route in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let donor_card = CardBuilder::new(CardId::new(), "Variant donor")
                .card_types(vec![CardType::Artifact])
                .build();
            let donor = game.create_object_from_card(&donor_card, alice, Zone::Graveyard);
            let recipient_card = CardBuilder::new(CardId::new(), "Variant recipient")
                .card_types(vec![CardType::Artifact])
                .build();
            let recipient = game.create_object_from_card(&recipient_card, alice, Zone::Battlefield);
            let leaf =
                StaticAbility::from_model(CompiledStaticAbility::double_life_change_replacement(
                    PlayerFilter::You,
                    false,
                    "Double life gain",
                ));
            // Repeated donor aliases remain one selected variant per instruction.
            // Multiplicity comes from the two independent copy instructions.
            for _ in 0..2 {
                game.object_mut(donor)
                    .unwrap()
                    .abilities_mut()
                    .push(Ability::static_ability(leaf.clone()));
            }
            let filter = ObjectFilter::default().in_zone(Zone::Graveyard);
            let selectors = vec![ironsmith_core::StaticAbilityVariantSelector::Any(
                StaticAbilityId::DoubleLifeChangeReplacement,
            )];
            let copy = StaticAbility::copy_static_ability_variants(CopyStaticAbilityVariants::new(
                filter.clone(),
                selectors.clone(),
                "Copy selected variants".into(),
            ));
            let mut registrations = Vec::new();
            for _ in 0..2 {
                match route {
                    0 => registrations.push(game.effect_store.continuous_effects.add_effect(
                        ContinuousEffect::from_resolution(
                            recipient,
                            alice,
                            vec![recipient],
                            Modification::CopyStaticAbilityVariants {
                                filter: filter.clone(),
                                selectors: selectors.clone(),
                                exclude_source_id: true,
                            },
                        ),
                    )),
                    1 => game
                        .object_mut(recipient)
                        .unwrap()
                        .abilities_mut()
                        .push(Ability::static_ability(copy.clone())),
                    _ => game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
                        recipient,
                        copy.id(),
                        Some(copy.clone()),
                    ),
                }
            }
            let keys = |game: &GameState| {
                generate_replacement_effects_from_abilities(game)
                    .unwrap()
                    .into_iter()
                    .map(|effect| effect.application_key())
                    .collect::<std::collections::HashSet<_>>()
            };
            let before = keys(&game);
            assert_eq!(before.len(), 2, "route={route}");
            for _ in 0..3 {
                game.refresh_continuous_state();
                assert_eq!(keys(&game), before);
            }
            assert_eq!(keys(&game.clone()), before);
            let mut ctx = crate::effects::EffectContext::new_default(recipient, alice);
            let outcome = crate::effects::GainLifeEffect::you(1)
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(outcome.count_or_zero(), 4);
            assert_eq!(outcome.events.len(), 1);
            assert_eq!(game.player(alice).unwrap().life, 24);
            game.set_current_controller(recipient, bob)
                .expect("finite controller fixture must refresh successfully");
            assert_eq!(
                keys(&game),
                before,
                "binding cannot merge or rename copy occurrences"
            );
            let mut ctx = crate::effects::EffectContext::new_default(recipient, bob);
            let outcome = crate::effects::GainLifeEffect::you(1)
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(outcome.count_or_zero(), 4);
            assert_eq!(outcome.events.len(), 1);
            assert_eq!(game.player(bob).unwrap().life, 24);
            game.object_mut(donor).unwrap().abilities_mut().clear();
            assert!(keys(&game).is_empty(), "variant no longer available");
            game.refresh_continuous_state();
            assert!(keys(&game).is_empty());
            for _ in 0..2 {
                game.object_mut(donor)
                    .unwrap()
                    .abilities_mut()
                    .push(Ability::static_ability(leaf.clone()));
            }
            assert_eq!(
                keys(&game),
                before,
                "restoring the same donor origins retains copy identity"
            );
            if route == 0 {
                game.effect_store
                    .continuous_effects
                    .remove_effect(registrations[0]);
                let survivor = keys(&game);
                assert_eq!(survivor.len(), 1);
                assert!(survivor.is_subset(&before));
                game.refresh_continuous_state();
                assert_eq!(keys(&game), survivor);
                let mut ctx = crate::effects::EffectContext::new_default(recipient, bob);
                let outcome = crate::effects::GainLifeEffect::you(1)
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                assert_eq!(outcome.count_or_zero(), 2);
                assert_eq!(outcome.events.len(), 1);
                assert_eq!(game.player(bob).unwrap().life, 26);
            } else if route == 2 {
                game.cleanup_temporary_object_static_ability_grants_end_of_turn();
                assert!(keys(&game).is_empty());
            } else {
                let loss = game.effect_store.continuous_effects.add_effect(
                    ContinuousEffect::from_resolution(
                        recipient,
                        bob,
                        vec![recipient],
                        Modification::RemoveAllAbilities,
                    ),
                );
                assert!(keys(&game).is_empty());
                game.refresh_continuous_state();
                assert!(keys(&game).is_empty());
                game.effect_store.continuous_effects.remove_effect(loss);
                assert_eq!(keys(&game), before);
            }
        }
    }
}

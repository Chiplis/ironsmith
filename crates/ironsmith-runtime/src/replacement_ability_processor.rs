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
use crate::continuous::{EffectTarget, Modification};
use crate::events::context::EventContext;
use crate::events::traits::{GameEventType, ReplacementMatcher, ReplacementPriority};
use crate::events::zones::matchers::{
    ThisWouldEnterBattlefieldMatcher, WouldEnterBattlefieldMatcher,
};
use crate::game_state::GameState;
use crate::replacement::ReplacementEffect;

#[derive(Debug)]
struct GrantedReplacementMatcher {
    grant_target: Box<dyn ReplacementMatcher>,
    granted_ability: Box<dyn ReplacementMatcher>,
}

impl Clone for GrantedReplacementMatcher {
    fn clone(&self) -> Self {
        Self {
            grant_target: self.grant_target.clone_box(),
            granted_ability: self.granted_ability.clone_box(),
        }
    }
}

impl ReplacementMatcher for GrantedReplacementMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        self.grant_target.matches_event(event, ctx)
            && self.granted_ability.matches_event(event, ctx)
    }

    fn priority(&self) -> ReplacementPriority {
        self.granted_ability.priority()
    }

    fn display(&self) -> String {
        format!(
            "{} and {}",
            self.grant_target.display(),
            self.granted_ability.display()
        )
    }
}

fn replacement_matcher_for_effect_target(
    target: &EffectTarget,
) -> Option<Box<dyn ReplacementMatcher>> {
    match target {
        EffectTarget::Filter(filter) => {
            Some(Box::new(WouldEnterBattlefieldMatcher::new(filter.clone())))
        }
        EffectTarget::AllPermanents => Some(Box::new(WouldEnterBattlefieldMatcher::any())),
        EffectTarget::AllCreatures => Some(Box::new(WouldEnterBattlefieldMatcher::creature())),
        EffectTarget::Source => Some(Box::new(ThisWouldEnterBattlefieldMatcher)),
        EffectTarget::Specific(object_id) => {
            let filter = crate::target::ObjectFilter {
                specific: Some(*object_id),
                ..crate::target::ObjectFilter::permanent()
            };
            Some(Box::new(WouldEnterBattlefieldMatcher::new(filter)))
        }
        EffectTarget::AttachedTo(_) => None,
    }
}

fn replacement_effects_from_granted_abilities(
    game: &GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    static_ability: &crate::static_abilities::StaticAbility,
) -> Vec<ReplacementEffect> {
    static_ability
        .generate_effects(source, controller, game)
        .into_iter()
        .filter(|effect| {
            crate::continuous::continuous_effect_duration_and_condition_are_active(effect, game)
        })
        .filter_map(|effect| {
            let Modification::AddAbility(granted_ability) = effect.modification else {
                return None;
            };
            let mut replacement =
                granted_ability.generate_replacement_effect(source, controller)?;
            // A source-only grant is how the model interpreter preserves a
            // condition around a static ability that has no native conditional
            // runtime form. The granted ability's replacement matcher already
            // refers to `source`, which is also the object receiving the grant.
            // Wrapping it in an enter-the-battlefield matcher would incorrectly
            // restrict every such replacement to zone-entry events (and makes
            // conditional damage prevention impossible to apply).
            if matches!(effect.applies_to, EffectTarget::Source) {
                return Some(replacement);
            }
            let grant_target = replacement_matcher_for_effect_target(&effect.applies_to)?;
            let granted_ability = replacement.matcher.take()?;
            replacement.matcher = Some(Box::new(GrantedReplacementMatcher {
                grant_target,
                granted_ability,
            }));
            Some(replacement)
        })
        .collect()
}

/// Generate all replacement effects from static abilities in zones where they function.
///
/// This scans all objects for static abilities that generate replacement effects
/// and returns the corresponding `ReplacementEffect` structs.
///
/// This function is called during game state refresh to ensure that static ability
/// replacement effects are properly registered.
pub fn generate_replacement_effects_from_abilities(game: &GameState) -> Vec<ReplacementEffect> {
    let mut effects = Vec::new();

    let object_ids = game.object_ids_in_deterministic_order();

    // Iterate over all objects and apply static abilities only in zones where they function.
    for object_id in object_ids {
        if let Some(object) = game.object(object_id) {
            if object.zone == crate::zone::Zone::Battlefield && game.is_phased_out(object_id) {
                continue;
            }
            let controller = game.controller_of(object);
            let zone = object.zone;

            // Process each static ability on the object.
            for ability in object.abilities.iter() {
                if let AbilityKind::Static(static_ability) = &ability.kind {
                    if !ability.functions_in(&zone) {
                        continue;
                    }
                    if let Some(effect) =
                        static_ability.generate_replacement_effect(object_id, controller)
                    {
                        effects.push(effect);
                    }
                    effects.extend(replacement_effects_from_granted_abilities(
                        game,
                        object_id,
                        controller,
                        static_ability,
                    ));
                }
            }

            for grant in &object.temporary_static_ability_grants {
                if grant.is_expired(game.turn.turn_number) {
                    continue;
                }
                let Some(static_ability) = grant.materialize() else {
                    continue;
                };
                if let Some(effect) =
                    static_ability.generate_replacement_effect(object_id, controller)
                {
                    effects.push(effect);
                }
            }
        }
    }

    effects
}

#[cfg(test)]
mod tests {
    use super::generate_replacement_effects_from_abilities;
    use crate::cards::CardDefinitionBuilder;
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

        assert!(
            generate_replacement_effects_from_abilities(&game)
                .iter()
                .all(|effect| effect.source != source),
            "the prevention replacement must be inactive without the required counter"
        );

        game.add_counters(source, CounterType::PlusOnePlusOne, 1);
        let replacements = generate_replacement_effects_from_abilities(&game);
        let replacement = replacements
            .iter()
            .find(|effect| effect.source == source)
            .expect("the prevention replacement should activate once the source has a counter");
        assert!(matches!(
            replacement.replacement,
            ReplacementAction::Instead(_)
        ));

        game.remove_counters(source, CounterType::PlusOnePlusOne, 1, None, None);
        assert!(
            generate_replacement_effects_from_abilities(&game)
                .iter()
                .all(|effect| effect.source != source),
            "the prevention replacement must deactivate after the last counter is removed"
        );
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
                ObjectFilter::creature(),
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
        let ingredient = CounterType::Named("ingredient");

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

        let effects = generate_replacement_effects_from_abilities(&game);
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
}

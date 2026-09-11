use crate::effect::EffectOutcome;
use crate::effects::{
    ApplyReplacementEffect, EffectExecutionCategory, EffectExecutor, ExecutionContext,
    ExecutionError,
};
use crate::events::zones::matchers::WouldEnterBattlefieldMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};

pub type RegisterEnterWithCountersReplacementEffect =
    ironsmith_core::RegisterEnterWithCountersReplacementEffect;

impl EffectExecutor for RegisterEnterWithCountersReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let filters = if let Some(objects) = &self.objects {
            let selected =
                match crate::effects::helpers::resolve_objects_from_spec(game, objects, ctx) {
                    Ok(objects) => objects,
                    // A referenced spell can leave the stack before this trigger
                    // resolves. That leaves no future entry to modify.
                    Err(ExecutionError::InvalidTarget) if !objects.is_target() => Vec::new(),
                    Err(error) => return Err(error),
                };
            selected
                .into_iter()
                .map(|id| {
                    let mut filter = self.filter.clone();
                    filter.specific = Some(id);
                    filter
                })
                .collect::<Vec<_>>()
        } else {
            vec![self.filter.clone()]
        };
        for filter in filters {
            let replacement = ReplacementEffect::with_matcher(
                ctx.source,
                ctx.controller,
                WouldEnterBattlefieldMatcher::new(filter),
                ReplacementAction::EnterWithCounters {
                    counter_type: self.counter_type,
                    count: self.count.clone(),
                    count_condition: None,
                    otherwise_count: None,
                    added_subtypes: Vec::new(),
                    added_abilities: Vec::new(),
                },
            );
            ApplyReplacementEffect {
                effect: replacement,
                mode: self.mode,
            }
            .execute(game, ctx)?;
        }
        Ok(EffectOutcome::resolved())
    }
    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::ReplacementRegistration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::ReplacementApplyMode;
    use crate::ids::CardId;
    use crate::object::CounterType;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn ongoing_entry_counters_survive_source_and_intervening_turns() {
        for schedule in 0..4 {
            for remove_source in [false, true] {
                let mut game =
                    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                let alice = game.players[0].id;
                let bob = game.players[1].id;
                let card = CardBuilder::new(CardId::new(), "Creature")
                    .card_types(vec![CardType::Creature])
                    .build();
                let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
                let effect = RegisterEnterWithCountersReplacementEffect::new(
                    ObjectFilter::creature()
                        .you_control()
                        .in_zone(Zone::Battlefield),
                    CounterType::PlusOnePlusOne,
                    crate::Value::Fixed(1),
                    ReplacementApplyMode::UntilYourNextTurn,
                );
                let mut dm = SelectFirstDecisionMaker;
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                effect.execute(&mut game, &mut ctx).unwrap();
                if remove_source {
                    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
                }
                match schedule {
                    1 => game.turn_store.extra_turns.push(alice),
                    2 => game.turn_store.extra_turns.push(bob),
                    3 => {
                        game.turn_store.skip_next_turn.insert(alice);
                    }
                    _ => {}
                }
                let mut expired = false;
                for turn in 0..8 {
                    if turn > 0 {
                        game.next_turn();
                        expired |= game.is_active_player(alice);
                    }
                    for owner in [alice, bob] {
                        for origin in [Zone::Hand, Zone::Graveyard, Zone::Exile] {
                            let object = game.create_object_from_card(&card, owner, origin);
                            let entered = game
                                .move_object_with_etb_processing_with_dm(
                                    object,
                                    Zone::Battlefield,
                                    &mut dm,
                                )
                                .unwrap()
                                .new_id;
                            assert_eq!(
                                game.counter_count(entered, CounterType::PlusOnePlusOne),
                                u32::from(owner == alice && !expired),
                                "schedule={schedule} removed={remove_source} turn={turn} origin={origin:?}"
                            );
                        }
                    }
                    game.effect_store
                        .replacement_effects
                        .clear_until_end_of_turn_effects();
                }
            }
        }
    }

    #[test]
    fn next_turn_replacement_expires_at_departed_players_would_be_turn() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let card = CardBuilder::new(CardId::new(), "Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let boundary = game.next_turn_number_if_player_stayed(alice);
        let effect = RegisterEnterWithCountersReplacementEffect::new(
            ObjectFilter::creature().in_zone(Zone::Battlefield),
            CounterType::PlusOnePlusOne,
            crate::Value::Fixed(2),
            ReplacementApplyMode::UntilYourNextTurn,
        );
        let mut dm = SelectFirstDecisionMaker;
        effect
            .execute(
                &mut game,
                &mut ExecutionContext::new(source, alice, &mut dm),
            )
            .unwrap();
        assert!(game.leave_game(alice));
        for _ in 0..6 {
            game.next_turn();
            let object = game.create_object_from_card(&card, bob, Zone::Hand);
            let entered = game
                .move_object_with_etb_processing_with_dm(object, Zone::Battlefield, &mut dm)
                .unwrap()
                .new_id;
            assert_eq!(
                game.counter_count(entered, CounterType::PlusOnePlusOne),
                if game.turn.turn_number < boundary {
                    2
                } else {
                    0
                }
            );
        }
    }
    #[test]
    fn bound_entry_replacement_ignores_unrelated_and_later_zone_objects() {
        for counter_timing in [0, 1, 2] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let card = CardBuilder::new(CardId::new(), "Creature")
                .card_types(vec![CardType::Creature])
                .build();
            let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
            let spell = game.create_object_from_card(&card, alice, Zone::Stack);
            game.stack
                .push(crate::game_state::StackEntry::new(spell, alice));
            let unrelated = game.create_object_from_card(&card, alice, Zone::Hand);
            let snapshot =
                crate::snapshot::ObjectSnapshot::from_object(game.object(spell).unwrap(), &game);
            let mut current = spell;
            if counter_timing == 1 {
                current = game
                    .move_object_by_effect(current, Zone::Graveyard)
                    .unwrap();
            }
            let mut ctx = ExecutionContext::new_default(source, alice);
            ctx.tagged_objects.insert("spell".into(), vec![snapshot]);
            let effect = RegisterEnterWithCountersReplacementEffect::new(
                ObjectFilter::permanent(),
                CounterType::PlusOnePlusOne,
                crate::Value::Fixed(1),
                ReplacementApplyMode::OneShot,
            )
            .with_objects(crate::target::ChooseSpec::Object(
                ObjectFilter::exact_tagged("spell").in_zone(Zone::Stack),
            ));
            effect.execute(&mut game, &mut ctx).unwrap();
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            let mut dm = SelectFirstDecisionMaker;
            let unrelated = game
                .move_object_with_etb_processing_with_dm(unrelated, Zone::Battlefield, &mut dm)
                .unwrap()
                .new_id;
            assert_eq!(
                game.counter_count(unrelated, CounterType::PlusOnePlusOne),
                0
            );
            if counter_timing == 2 {
                current = game
                    .move_object_by_effect(current, Zone::Graveyard)
                    .unwrap();
            }
            let entered = game
                .move_object_with_etb_processing_with_dm(current, Zone::Battlefield, &mut dm)
                .unwrap()
                .new_id;
            assert_eq!(
                game.counter_count(entered, CounterType::PlusOnePlusOne),
                u32::from(counter_timing == 0)
            );
            let graveyard = game
                .move_object_by_effect(entered, Zone::Graveyard)
                .unwrap();
            let returned = game
                .move_object_with_etb_processing_with_dm(graveyard, Zone::Battlefield, &mut dm)
                .unwrap()
                .new_id;
            assert_eq!(game.counter_count(returned, CounterType::PlusOnePlusOne), 0);
        }
    }
}

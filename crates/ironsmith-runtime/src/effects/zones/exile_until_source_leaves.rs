//! Exile-until effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::EventOutcome;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::zone::Zone;

use super::apply_zone_change_with_additional_effects;

/// Duration for "exile ... until ..." effects.
pub type ExileUntilDuration = ironsmith_core::ExileUntilDuration;

/// Exile objects with an associated duration.
pub type ExileUntilEffect = ironsmith_core::ExileUntilEffect;

fn object_known_and_not_on_battlefield(
    game: &GameState,
    object_id: ObjectId,
    snapshot: Option<&ObjectSnapshot>,
) -> bool {
    if let Some(object) = game.object(object_id) {
        return object.zone != Zone::Battlefield;
    }

    let Some(snapshot) = snapshot else {
        return false;
    };

    game.find_object_by_stable_id(snapshot.stable_id)
        .and_then(|current_id| game.object(current_id))
        .is_none_or(|object| object.zone != Zone::Battlefield)
}

impl EffectExecutor for ExileUntilEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let leave_watcher = if self.duration == ExileUntilDuration::SourceLeavesBattlefield {
            if let Some(watcher_spec) = &self.leave_watcher {
                let watchers = match resolve_objects_for_effect(game, ctx, watcher_spec) {
                    Ok(watchers) => watchers,
                    Err(ExecutionError::InvalidTarget) => {
                        return Ok(EffectOutcome::count(0));
                    }
                    Err(error) => return Err(error),
                };
                let Some(&watcher) = watchers.first() else {
                    return Ok(EffectOutcome::count(0));
                };
                if object_known_and_not_on_battlefield(
                    game,
                    watcher,
                    ctx.target_snapshots.get(&watcher),
                ) {
                    return Ok(EffectOutcome::count(0));
                }
                watcher
            } else {
                if object_known_and_not_on_battlefield(
                    game,
                    ctx.source,
                    ctx.source_snapshot.as_ref(),
                ) {
                    return Ok(EffectOutcome::count(0));
                }
                ctx.source
            }
        } else {
            ctx.source
        };

        let objects = resolve_objects_for_effect(game, ctx, &self.spec)?;
        let mut exiled_count = 0_i32;
        let mut monarch_duration_stable_ids = Vec::new();
        for object_id in objects {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let from_zone = obj.zone;
            let additional_effects = ctx.additional_replacement_effects_snapshot();

            let result = apply_zone_change_with_additional_effects(
                game,
                object_id,
                from_zone,
                Zone::Exile,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                &additional_effects,
            );

            if let EventOutcome::Proceed(result) = result
                && result.final_zone == Zone::Exile
            {
                for &new_id in &result.new_object_ids {
                    if self.face_down {
                        game.set_face_down(new_id);
                    }
                    if self.duration == ExileUntilDuration::SourceLeavesBattlefield {
                        game.add_exiled_with_source_link_returning_to(
                            leave_watcher,
                            new_id,
                            from_zone,
                        );
                    } else {
                        game.add_exiled_with_source_link(ctx.source, new_id);
                    }
                    if self.duration == ExileUntilDuration::OpponentBecomesMonarch
                        && let Some(exiled) = game.object(new_id)
                    {
                        monarch_duration_stable_ids.push(exiled.stable_id);
                    }
                    exiled_count += 1;
                }
            }
        }

        if exiled_count > 0 && self.duration == ExileUntilDuration::SourceLeavesBattlefield {
            game.mark_return_exiled_when_source_leaves(leave_watcher);
        }
        if !monarch_duration_stable_ids.is_empty()
            && self.duration == ExileUntilDuration::OpponentBecomesMonarch
        {
            game.track_exiled_until_opponent_becomes_monarch(
                ctx.controller,
                monarch_duration_stable_ids,
                self.return_zone,
            );
        }
        Ok(EffectOutcome::count(exiled_count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "target to exile"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::zones::matchers::WouldBeExiledMatcher;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_on_battlefield(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_enchantment_on_battlefield(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Enchantment])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_exile_until_respects_destination_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature_id = create_creature_on_battlefield(&mut game, "Elite Vanguard", alice);

        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                WouldBeExiledMatcher::new(ObjectFilter::permanent()),
                ReplacementAction::ChangeDestination(Zone::Hand),
            ),
        );

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(creature_id));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert!(game.exile.is_empty());
        assert_eq!(game.get_exiled_with_source_links(source).len(), 0);
        assert_eq!(game.players[0].hand.len(), 1);
        assert!(game.battlefield.is_empty());
    }

    #[test]
    fn source_leaves_duration_noops_if_source_already_left_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature_on_battlefield(&mut game, "Banisher Priest", alice);
        let creature_id = create_creature_on_battlefield(&mut game, "Elite Vanguard", alice);
        let source_snapshot = ObjectSnapshot::from_object(
            game.object(source)
                .expect("source should exist before it leaves"),
            &game,
        );
        game.move_object_by_effect(source, Zone::Graveyard);

        let mut ctx =
            ExecutionContext::new_default(source, alice).with_source_snapshot(source_snapshot);
        let effect = ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(creature_id));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert!(game.exile.is_empty());
        assert!(game.battlefield.contains(&creature_id));
    }

    #[test]
    fn source_leaves_duration_returns_exiled_card_without_stack_trigger() {
        let mut game = setup_game();
        let mut trigger_queue = crate::triggers::TriggerQueue::new();
        let alice = PlayerId::from_index(0);
        let source = create_creature_on_battlefield(&mut game, "Banisher Priest", alice);
        let creature_id = create_creature_on_battlefield(&mut game, "Elite Vanguard", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(creature_id));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.get_exiled_with_source_links(source).len(), 1);
        assert!(game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Elite Vanguard")
        }));

        game.move_object_by_effect(source, Zone::Graveyard);
        crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);

        assert!(trigger_queue.entries.is_empty());
        assert!(game.exile.is_empty());
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Elite Vanguard")
        }));
    }

    #[test]
    fn distinct_leave_watcher_owns_the_return_link() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature_on_battlefield(&mut game, "Ability Source", alice);
        let watcher = create_enchantment_on_battlefield(&mut game, "Watched Enchantment", alice);
        let unrelated = create_creature_on_battlefield(&mut game, "Unrelated Permanent", alice);
        let creature_id = create_creature_on_battlefield(&mut game, "Exiled Creature", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(creature_id))
            .with_leave_watcher(ChooseSpec::SpecificObject(watcher));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.get_exiled_with_source_links(source).len(), 0);
        assert_eq!(game.get_exiled_with_source_links(watcher).len(), 1);

        let mut trigger_queue = crate::triggers::TriggerQueue::new();
        game.move_object_by_effect(source, Zone::Graveyard);
        game.move_object_by_effect(unrelated, Zone::Graveyard);
        crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);
        assert!(game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Exiled Creature")
        }));

        game.move_object_by_effect(watcher, Zone::Graveyard);
        crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);
        assert!(game.exile.is_empty());
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Exiled Creature")
        }));
    }

    #[test]
    fn distinct_leave_watcher_lki_noops_after_watcher_already_left() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature_on_battlefield(&mut game, "Ability Source", alice);
        let watcher = create_enchantment_on_battlefield(&mut game, "Watched Enchantment", alice);
        let creature_id = create_creature_on_battlefield(&mut game, "Exiled Creature", alice);
        let watcher_snapshot =
            ObjectSnapshot::from_object(game.object(watcher).expect("watcher should exist"), &game);
        game.move_object_by_effect(watcher, Zone::Graveyard);

        let watcher_spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::enchantment()));
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(watcher)]);
        ctx.target_snapshots.insert(watcher, watcher_snapshot);
        let effect = ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(creature_id))
            .with_leave_watcher(watcher_spec);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert!(game.exile.is_empty());
        assert!(game.battlefield.contains(&creature_id));
    }

    #[test]
    fn source_leaves_duration_can_exile_card_from_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature_on_battlefield(&mut game, "Brain Maggot", alice);
        let card = make_creature_card(9911, "Bloodflow Connoisseur");
        let hand_card = game.create_object_from_card(&card, bob, Zone::Hand);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(hand_card));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert!(game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Bloodflow Connoisseur")
        }));
        assert_eq!(game.get_exiled_with_source_links(source).len(), 1);

        game.move_object_by_effect(source, Zone::Graveyard);
        crate::game_loop::drain_pending_trigger_events(
            &mut game,
            &mut crate::triggers::TriggerQueue::new(),
        );

        assert!(game.exile.is_empty());
        assert!(game.player(bob).expect("bob exists").hand.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Bloodflow Connoisseur")
        }));
    }

    #[test]
    fn opponent_becomes_monarch_duration_survives_source_leaving_and_returns_for_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature_on_battlefield(&mut game, "Palace Jailer", alice);
        let creature_id = create_creature_on_battlefield(&mut game, "Exiled Creature", bob);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExileUntilEffect::new(
            ChooseSpec::SpecificObject(creature_id),
            ExileUntilDuration::OpponentBecomesMonarch,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert!(game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Exiled Creature")
        }));

        game.move_object_by_effect(source, Zone::Graveyard);
        crate::game_loop::drain_pending_trigger_events(
            &mut game,
            &mut crate::triggers::TriggerQueue::new(),
        );
        assert!(
            game.exile.iter().any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Exiled Creature")),
            "leaving Palace Jailer must not end its monarch-event duration"
        );

        game.set_monarch(Some(alice));
        assert!(
            game.exile.iter().any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Exiled Creature")),
            "the effect controller becoming monarch is not the duration event"
        );

        game.set_monarch(Some(bob));
        assert!(game.exile.is_empty());
        let returned = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name == "Exiled Creature")
            })
            .expect("the exiled creature should return under its owner's control");
        assert_eq!(game.controller_of_id(returned), Some(bob));
    }
}

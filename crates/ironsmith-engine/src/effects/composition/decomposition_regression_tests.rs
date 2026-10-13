//! Integration contracts for the shared choice and continuation programs.
use crate::effect::Effect;
use crate::effects::{EffectExecutor, ExecutionContext};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::{
    card::{CardBuilder, PowerToughness},
    ids::{CardId, ObjectId, PlayerId},
    types::CardType,
    zone::Zone,
};
fn creature(game: &mut crate::GameState, player: PlayerId) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        player,
        Zone::Battlefield,
    )
}
#[test]
fn backup_prevented_counter_still_grants_and_preserves_producer_result() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let player = PlayerId(0);
    let source = creature(&mut game, player);
    let target = creature(&mut game, player);
    game.effect_store.replacement_effects.add_one_shot_effect(
        crate::replacement::ReplacementEffect::with_matcher(
            source,
            player,
            crate::events::counters::matchers::WouldPutCountersMatcher::new(
                ObjectFilter::specific(target),
                Some(crate::object::CounterType::PlusOnePlusOne),
            ),
            crate::replacement::ReplacementAction::Prevent,
        ),
    );
    let mut ctx = ExecutionContext::new_default(source, player)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    let output = super::mechanic_actions::BackupEffect::new(
        2,
        vec![crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::flying(),
        )],
    )
    .execute_with_outputs(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(output.outcome.count_or_zero(), 0);
    assert_eq!(
        game.counter_count(target, crate::object::CounterType::PlusOnePlusOne),
        0
    );
    assert!(
        game.object_has_static_ability_id(target, crate::static_abilities::StaticAbilityId::Flying)
    );
}
#[test]
fn exact_presence_guard_does_not_follow_a_moved_card() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let player = PlayerId(0);
    let source = creature(&mut game, player);
    let mut ctx = ExecutionContext::new_default(source, player);
    Effect::move_to_zone(ChooseSpec::SpecificObject(source), Zone::Hand, false)
        .0
        .execute_child_with_outputs(&mut game, &mut ctx)
        .unwrap();
    super::resolved_program::IfPresentProgram {
        object: source,
        zone: Zone::Battlefield,
        effects: vec![Effect::gain_life(4)],
    }
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(game.player(player).unwrap().life, 20);
}
#[test]
fn melee_queries_current_controller_and_current_combat_and_preserves_count() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId(0);
    let bob = PlayerId(1);
    let source = creature(&mut game, bob);
    game.combat = Some(crate::combat_state::CombatState::default());
    let combat = game.turn_store.combat_phases_started_this_turn;
    game.turn_store
        .turn_history
        .players_attacked_in_combat
        .insert((combat, bob), [alice, bob].into_iter().collect());
    game.turn_store
        .turn_history
        .players_attacked_in_combat
        .insert((combat + 1, bob), [alice, bob].into_iter().collect());
    let mut ctx = ExecutionContext::new_default(source, alice);
    let result = crate::effects::MeleeEffect::new()
        .execute_with_outputs(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(result.outcome.count_or_zero(), 1);
    assert_eq!(game.current_power(source), Some(3));
    let value = crate::effect::Value::TurnHistoryCount(
        crate::effect::TurnHistoryCount::PlayersAttackedThisCombat(PlayerFilter::Specific(bob)),
    );
    assert_eq!(
        crate::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(),
        1
    );
    game.combat = None;
    assert_eq!(
        crate::effects::MeleeEffect::new()
            .execute(&mut game, &mut ctx)
            .unwrap()
            .count_or_zero(),
        0
    );
}
struct Decline;
impl crate::decision::DecisionMaker for Decline {
    fn decide_boolean(
        &mut self,
        _: &crate::GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        false
    }
}
#[test]
fn resolved_optional_movement_declines_without_moving_or_rebinding_iteration() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let player = PlayerId(0);
    let source = creature(&mut game, player);
    let mut dm = Decline;
    let mut ctx = ExecutionContext::new(source, player, &mut dm);
    ctx.iteration.iterated_player = Some(player);
    let output = crate::effects::MayMoveToZoneEffect::new(
        ChooseSpec::SpecificObject(source),
        Zone::Hand,
        PlayerFilter::Specific(PlayerId(1)),
    )
    .execute_with_outputs(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
    assert_eq!(ctx.iteration.iterated_player, Some(player));
    assert!(
        output
            .outcome
            .execution_facts
            .iter()
            .any(|fact| matches!(fact, crate::effect::ExecutionFact::Declined))
    );
}
#[test]
fn stored_copy_decline_cleans_up_and_preserves_prior_binding() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let player = PlayerId(0);
    let source = creature(&mut game, player);
    let card = game.object(source).unwrap().clone();
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(&card, &game);
    let mut dm = Decline;
    let mut ctx = ExecutionContext::new(source, player, &mut dm);
    ctx.set_tagged_objects("__stored_card_copy__", vec![snapshot]);
    crate::effects::stack::CastStoredCardCopyEffect::new(&card)
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(game.exile.is_empty());
    assert_eq!(
        ctx.get_tagged_all("__stored_card_copy__").unwrap()[0].object_id,
        source
    );
}

#[test]
fn backup_does_not_grant_after_a_counter_addition_moves_the_exact_target() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let player = PlayerId(0);
    let source = creature(&mut game, player);
    let target = creature(&mut game, player);
    game.effect_store.replacement_effects.add_one_shot_effect(
        crate::replacement::ReplacementEffect::with_matcher(
            source,
            player,
            crate::events::counters::matchers::WouldPutCountersMatcher::new(
                ObjectFilter::specific(target),
                Some(crate::object::CounterType::PlusOnePlusOne),
            ),
            crate::replacement::ReplacementAction::Additionally(vec![Effect::move_to_zone(
                ChooseSpec::SpecificObject(target),
                Zone::Hand,
                false,
            )]),
        ),
    );
    let mut ctx = ExecutionContext::new_default(source, player)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    let output = super::mechanic_actions::BackupEffect::new(
        1,
        vec![crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::flying(),
        )],
    )
    .execute_with_outputs(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(output.outcome.count_or_zero(), 1);
    assert!(game.object(target).is_none());
    let moved = *game.player(player).unwrap().hand.last().unwrap();
    assert!(
        !game.object_has_static_ability_id(moved, crate::static_abilities::StaticAbilityId::Flying)
    );
}

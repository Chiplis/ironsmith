//! Authored, UNRUN regressions for player hexproof's retained controller.
use super::*;
use crate::card::CardBuilder;
use crate::color::ColorSet;
use crate::effect::{Restriction, RestrictionExt};
use crate::effects::{ExecutionContext, ResolvedTarget};
use crate::game_state::{CantEffectTracker, PlayerCantBeTargetedFrom, TargetingAsThoughOverride};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbilityId;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const D: PlayerId = PlayerId(3);

fn fixture() -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Red ability source")
        .card_types(vec![CardType::Enchantment])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .build();
    let source = game.create_object_from_card(&card, A, Zone::Battlefield);
    (game, source)
}

fn hexproof(game: &mut GameState, filter: PlayerFilter, qualities: ObjectFilter) {
    let mut tracker = CantEffectTracker::default();
    Restriction::player_hexproof_from(filter, qualities)
        .apply(game, &mut tracker, A, None, None);
    game.effect_store.cant_effects.merge(tracker);
}

fn specs() -> Vec<ChooseSpec> {
    vec![
        ChooseSpec::Player(PlayerFilter::Any),
        ChooseSpec::ObjectOrPlayer(ObjectFilter::creature(), PlayerFilter::Any),
        ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any),
        ChooseSpec::AnyTarget,
        ChooseSpec::AnyOtherTarget,
    ]
}

#[test]
fn every_player_target_domain_retains_ability_controller_after_source_theft_or_absence() {
    for state in 0..4 {
        let (mut game, source) = fixture();
        game.set_current_controller(source, B).unwrap();
        // Deliberately retain a snapshot controlled by B: it must supply
        // source qualities, never replace the ability's controller A.
        let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        match state {
            1 | 2 => { game.remove_object(source); }
            3 => game.phase_out(source),
            _ => {}
        }
        hexproof(&mut game, PlayerFilter::Specific(B), ObjectFilter::default());
        let mut ctx = ExecutionContext::new_default(source, A);
        if state != 2 { ctx.source_snapshot = Some(snapshot); }
        for spec in specs() {
            let spec = ChooseSpec::target(spec);
            assert!(!compute_legal_targets_with_execution_context(&game, &spec, &ctx)
                .contains(&Target::Player(B)), "state {state}: {spec:?}");
            assert!(!crate::effects::validate_target(&game, &ResolvedTarget::Player(B), &spec, &ctx));
        }
        assert!(!compute_legal_targets(&game, &ChooseSpec::AnyTarget, A, None)
            .contains(&Target::Player(B)), "unqualified hexproof needs no physical source");
        ctx.controller = B;
        assert!(compute_legal_targets_with_execution_context(&game, &ChooseSpec::AnyTarget, &ctx)
            .contains(&Target::Player(B)), "a player's own ability can target that player");
    }
}

#[test]
fn all_players_hexproof_uses_each_recipient_and_current_teams() {
    let (mut game, source) = fixture();
    game.set_teams(vec![vec![A, C], vec![B, D]]).unwrap();
    game.set_current_controller(source, B).unwrap();
    hexproof(&mut game, PlayerFilter::Any, ObjectFilter::default());
    for controller in [A, B, C, D] {
        let targets = compute_legal_targets(&game, &ChooseSpec::AnyTarget, controller, Some(source));
        for player in [A, B, C, D] {
            assert_eq!(targets.contains(&Target::Player(player)),
                !game.are_opponents(controller, player));
        }
    }
    game.effect_store.cant_effects.clear();
    assert!(game.effect_store.cant_effects.player_hexproof_from.is_empty());
}

#[test]
fn qualified_hexproof_uses_live_then_retained_color_without_borrowing_source_controller() {
    let (mut game, source) = fixture();
    game.set_current_controller(source, B).unwrap();
    let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    game.object_mut(source).unwrap().color_override = Some(ColorSet::GREEN);
    hexproof(&mut game, PlayerFilter::Specific(B), ObjectFilter::default().with_colors(ColorSet::RED));
    assert!(game.can_target_player_from_source_or_snapshot(B, Some(source), Some(&snapshot), A));
    game.remove_object(source);
    assert!(!game.can_target_player_from_source_or_snapshot(B, Some(source), Some(&snapshot), A));
    assert!(game.can_target_player_from_source_or_snapshot(B, Some(source), None, A));
}

#[test]
fn protection_retains_actual_source_controller_and_survives_ignore_hexproof_permission() {
    let (mut game, source) = fixture();
    let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    game.set_current_controller(source, B).unwrap();
    game.effect_store.cant_effects.cant_target_players_from.push(PlayerCantBeTargetedFrom {
        player: B,
        source_filter: ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
        controller: B,
    });
    assert!(game.can_target_player_from_source_or_snapshot(B, Some(source), Some(&snapshot), A),
        "protection from opponents evaluates the actual source now controlled by B");
    game.remove_object(source);
    assert!(!game.can_target_player_from_source_or_snapshot(B, Some(source), Some(&snapshot), B),
        "retained protection qualities still see the absent source's controller A");
    game.effect_store.cant_effects.targeting_as_though_overrides.push(TargetingAsThoughOverride {
        objects: None, players: Some(PlayerFilter::Any),
        allowed_source_controller: Some(B), ignored_ability: StaticAbilityId::Hexproof,
        controller: B, source,
    });
    assert!(!game.can_target_player_from_source_or_snapshot(B, Some(source), Some(&snapshot), B),
        "permission to ignore hexproof cannot ignore protection");
}

#[test]
fn ignore_hexproof_permission_is_owned_by_retained_controller_and_keeps_shroud() {
    let (mut game, source) = fixture();
    game.set_current_controller(source, B).unwrap();
    let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    game.remove_object(source);
    hexproof(&mut game, PlayerFilter::Specific(B), ObjectFilter::default());
    for allowed in [A, B] {
        game.effect_store.cant_effects.targeting_as_though_overrides = vec![TargetingAsThoughOverride {
            objects: None, players: Some(PlayerFilter::Any), allowed_source_controller: Some(allowed),
            ignored_ability: StaticAbilityId::Hexproof, controller: A, source,
        }];
        assert_eq!(game.can_target_player_from_source_or_snapshot(B, Some(source), Some(&snapshot), A),
            allowed == A);
    }
    game.effect_store.cant_effects.cant_target_players.insert(B);
    game.effect_store.cant_effects.targeting_as_though_overrides[0].allowed_source_controller = Some(A);
    assert!(!game.can_target_player_from_source_or_snapshot(B, Some(source), Some(&snapshot), A));
}

#[test]
fn temporary_player_hexproof_retains_resolved_recipient_and_missing_target_error() {
    use crate::effects::{CantEffect, EffectExecutor, ExecutionError};
    let (mut game, source) = fixture();
    let effect = CantEffect::until_end_of_turn(Restriction::player_hexproof_from(
        PlayerFilter::Target(Box::new(PlayerFilter::Any)), ObjectFilter::default(),
    ));
    let mut ctx = ExecutionContext::new_default(source, A);
    assert!(matches!(effect.execute(&mut game, &mut ctx), Err(ExecutionError::InvalidTarget)));
    assert!(game.effect_store.restriction_effects.is_empty());
    ctx.targets = vec![ResolvedTarget::Player(B)];
    effect.execute(&mut game, &mut ctx).unwrap();
    ctx.targets = vec![ResolvedTarget::Player(C)];
    game.set_current_controller(source, B).unwrap();
    game.refresh_continuous_state().unwrap();
    assert!(!game.can_target_player_from_source_or_snapshot(B, Some(source), None, A));
    assert!(game.can_target_player_from_source_or_snapshot(C, Some(source), None, A));
}

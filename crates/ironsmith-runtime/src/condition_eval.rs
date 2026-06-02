use crate::effect::Condition;
use crate::effect::Value;
use crate::effects::helpers::resolve_value;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::{ObjectFilterExt as _, player_filter_matches_game};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::object_query::candidate_ids_for_zone;
use crate::target::PlayerFilter;
use crate::zone::Zone;

use crate::triggers::{TriggerEvent, TriggerIdentity};

fn source_is_face_down_or_alternate_face(game: &GameState, source: ObjectId) -> bool {
    // `SourceIsFaceDown` is also used by daybound/nightbound lowering to mean
    // "this DFC is currently showing its alternate face."
    game.is_face_down(source) || game.transform_count(source) % 2 == 1
}

fn source_was_cast(
    game: &GameState,
    source: ObjectId,
    triggering_event: Option<&TriggerEvent>,
) -> bool {
    if let Some(event) = triggering_event
        && let Some(etb) = event.downcast::<crate::events::EnterBattlefieldEvent>()
        && etb.object == source
    {
        return etb.from == Zone::Stack;
    }
    if let Some(event) = triggering_event
        && let Some(zc) = event.downcast::<crate::events::ZoneChangeEvent>()
        && zc.to == Zone::Battlefield
        && zc.objects.contains(&source)
    {
        return zc.from == Zone::Stack;
    }
    game.turn_store
        .turn_history
        .spell_cast_order(source)
        .is_some()
}

fn tagged_object_was_cast(game: &GameState, tag: &crate::TagKey, ctx: &ExecutionContext) -> bool {
    let Some(tagged) = ctx.get_tagged_all(tag.as_str()) else {
        return false;
    };
    for snapshot in tagged {
        if let Some(event) = &ctx.triggering_event
            && let Some(etb) = event.downcast::<crate::events::EnterBattlefieldEvent>()
            && etb.from == Zone::Stack
            && etb.object == snapshot.object_id
        {
            return true;
        }
        if let Some(event) = &ctx.triggering_event
            && let Some(zc) = event.downcast::<crate::events::ZoneChangeEvent>()
            && zc.from == Zone::Stack
            && zc.to == Zone::Battlefield
            && zc.objects.contains(&snapshot.object_id)
        {
            return true;
        }
        if game
            .turn_store
            .turn_history
            .spell_cast_order(snapshot.object_id)
            .is_some()
        {
            return true;
        }
    }
    false
}

fn this_spell_was_cast_from_zone(
    game: &GameState,
    source: ObjectId,
    ctx: &ExecutionContext,
    zone: Zone,
) -> bool {
    match &ctx.casting_method {
        crate::alternative_cast::CastingMethod::GrantedFlashback => zone == Zone::Graveyard,
        crate::alternative_cast::CastingMethod::GrantedEscape { .. } => zone == Zone::Graveyard,
        crate::alternative_cast::CastingMethod::PlayFrom {
            zone: from_zone, ..
        } => *from_zone == zone,
        crate::alternative_cast::CastingMethod::SplitOtherHalfPlayFrom {
            zone: from_zone, ..
        } => *from_zone == zone,
        crate::alternative_cast::CastingMethod::Alternative(idx) => game
            .object(source)
            .and_then(|obj| obj.alternative_casts.get(*idx))
            .is_some_and(|method| method.cast_from_zone() == zone),
        crate::alternative_cast::CastingMethod::Normal
        | crate::alternative_cast::CastingMethod::FaceDown
        | crate::alternative_cast::CastingMethod::SplitOtherHalf
        | crate::alternative_cast::CastingMethod::Fuse => false,
    }
}

fn this_spell_was_cast_from_non_hand(
    game: &GameState,
    source: ObjectId,
    ctx: &ExecutionContext,
) -> bool {
    match &ctx.casting_method {
        crate::alternative_cast::CastingMethod::Normal
        | crate::alternative_cast::CastingMethod::FaceDown
        | crate::alternative_cast::CastingMethod::SplitOtherHalf
        | crate::alternative_cast::CastingMethod::Fuse => false,
        crate::alternative_cast::CastingMethod::GrantedFlashback
        | crate::alternative_cast::CastingMethod::GrantedEscape { .. } => true,
        crate::alternative_cast::CastingMethod::PlayFrom { zone, .. }
        | crate::alternative_cast::CastingMethod::SplitOtherHalfPlayFrom { zone, .. } => {
            *zone != Zone::Hand
        }
        crate::alternative_cast::CastingMethod::Alternative(idx) => game
            .object(source)
            .and_then(|obj| obj.alternative_casts.get(*idx))
            .is_some_and(|method| method.cast_from_zone() != Zone::Hand),
    }
}

fn this_spell_escaped(game: &GameState, source: ObjectId, ctx: &ExecutionContext) -> bool {
    if ctx.optional_costs_paid.was_paid_label("Escape") || source_escaped(game, source) {
        return true;
    }

    let Some(spell) = game.object(source) else {
        return matches!(
            ctx.casting_method,
            crate::alternative_cast::CastingMethod::GrantedEscape { .. }
        );
    };

    crate::decision::casting_method_matches_alternative_kind(
        game,
        ctx.controller,
        spell,
        &ctx.casting_method,
        crate::filter::AlternativeCastKind::Escape,
    )
}

fn source_escaped(game: &GameState, source: ObjectId) -> bool {
    game.object(source)
        .is_some_and(|obj| obj.optional_costs_paid.was_paid_label("Escape"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ExecutionContext;
    use crate::events::cause::EventCause;
    use crate::events::{DamageEvent, DamageTarget, RawEvent};
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn add_hand_card(game: &mut GameState, id_raw: u32, name: &str, owner_index: usize) {
        let card = CardBuilder::new(CardId::from_raw(id_raw), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let owner = game.players[owner_index].id;
        game.create_object_from_card(&card, owner, Zone::Hand);
    }

    #[test]
    fn evaluate_player_has_more_cards_in_hand_than_each_other_player_requires_unique_leader() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let condition = Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer {
            player: PlayerFilter::Any,
        };

        add_hand_card(&mut game, 1, "Mountain", 0);
        add_hand_card(&mut game, 2, "Island", 1);
        add_hand_card(&mut game, 3, "Forest", 1);

        let ctx = ExecutionContext::new_default(source, alice);
        assert!(
            evaluate_condition(&game, &condition, &ctx)
                .expect("unique hand-size leader should evaluate"),
            "expected Bob to satisfy the unique-leader condition"
        );

        add_hand_card(&mut game, 4, "Plains", 0);
        assert!(
            !evaluate_condition(&game, &condition, &ctx).expect("ties should evaluate cleanly"),
            "expected tie for most cards in hand to fail the condition"
        );
    }

    #[test]
    fn evaluate_player_has_more_life_than_each_other_player_requires_unique_leader() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let condition = Condition::PlayerHasMoreLifeThanEachOtherPlayer {
            player: PlayerFilter::Any,
        };

        game.players[1].life = 21;
        let ctx = ExecutionContext::new_default(source, alice);
        assert!(
            evaluate_condition(&game, &condition, &ctx)
                .expect("unique life leader should evaluate"),
            "expected Bob to satisfy the unique-leader life condition"
        );

        game.players[0].life = 21;
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("tied life totals should evaluate cleanly"),
            "expected tie for most life to fail the condition"
        );
    }

    #[test]
    fn frodo_ring_bearer_threshold_condition_requires_bearer_and_two_temptations() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let frodo = CardBuilder::new(CardId::from_raw(71_571), "Frodo, Adventurous Hobbit")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 3))
            .build();
        let other_creature = CardBuilder::new(CardId::from_raw(71_572), "Other Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source = game.create_object_from_card(&frodo, alice, Zone::Battlefield);
        let other_source = game.create_object_from_card(&other_creature, alice, Zone::Battlefield);
        let condition = Condition::And(
            Box::new(Condition::SourceIsRingBearer {
                player: PlayerFilter::You,
            }),
            Box::new(Condition::PlayerRingTemptedThisGameOrMore {
                player: PlayerFilter::You,
                count: 2,
            }),
        );
        let ctx = ExecutionContext::new_default(source, alice);

        game.set_ring_bearer(alice, source);
        game.increment_ring_temptations(bob);
        game.increment_ring_temptations(bob);
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("opponent temptations should evaluate cleanly"),
            "Frodo's draw gate must use its controller's Ring temptation count"
        );

        game.increment_ring_temptations(alice);
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("one temptation should evaluate cleanly"),
            "Frodo's draw gate must stay false before the Ring has tempted you twice"
        );

        game.increment_ring_temptations(alice);
        assert!(
            evaluate_condition(&game, &condition, &ctx)
                .expect("two temptations should evaluate cleanly"),
            "Frodo's draw gate should be true once he is your Ring-bearer and you have two temptations"
        );

        game.set_ring_bearer(alice, other_source);
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("different Ring-bearer should evaluate cleanly"),
            "Frodo's draw gate must require the source itself to be your Ring-bearer"
        );

        game.clear_ring_bearer(alice);
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("missing Ring-bearer should evaluate cleanly"),
            "Frodo's draw gate must stay false when the source is no longer your Ring-bearer"
        );
    }

    #[test]
    fn evaluate_player_has_no_opponent_with_more_life_than_allows_ties() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let condition = Condition::PlayerHasNoOpponentWithMoreLifeThan {
            player: PlayerFilter::Specific(alice),
        };

        let ctx = ExecutionContext::new_default(source, alice);
        assert!(
            evaluate_condition(&game, &condition, &ctx).expect("tied life totals should evaluate"),
            "expected tied life totals to satisfy the no-opponent-has-more-life condition"
        );

        game.players[1].life = 21;
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("higher life total should evaluate cleanly"),
            "expected an opposing higher life total to fail the condition"
        );
    }

    #[test]
    fn evaluate_object_put_into_graveyard_from_battlefield_condition_uses_lki_controller() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.new_object_id();
        let land = CardBuilder::new(CardId::from_raw(9), "Test Land")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
        let snapshot = {
            let object = game.object(land_id).expect("land exists");
            crate::snapshot::ObjectSnapshot::from_object(object, &game)
        };
        let zone_change = crate::events::RawEvent::new(
            crate::events::ZoneChangeEvent::with_cause(
                land_id,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
                Some(snapshot.clone()),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&zone_change, Some(snapshot), None);

        let condition = Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(
            crate::target::ObjectFilter::land().controlled_by(PlayerFilter::You),
        );

        assert!(
            evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, alice)
            )
            .expect("land-graveyard condition should evaluate"),
            "expected Alice's historical land to satisfy the condition"
        );
        assert!(
            !evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, bob)
            )
            .expect("land-graveyard condition should evaluate"),
            "expected Bob not to satisfy Alice's historical land condition"
        );
    }

    #[test]
    fn evaluate_object_entered_battlefield_condition_uses_lki_controller() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.new_object_id();
        let artifact = CardBuilder::new(CardId::from_raw(10), "Test Artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_id = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        let snapshot = {
            let object = game.object(artifact_id).expect("artifact exists");
            crate::snapshot::ObjectSnapshot::from_object(object, &game)
        };
        let etb = crate::events::RawEvent::new(
            crate::events::EnterBattlefieldEvent::new(artifact_id, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&etb, Some(snapshot), None);

        let condition = Condition::ObjectEnteredBattlefieldThisTurn(
            crate::target::ObjectFilter::artifact().controlled_by(PlayerFilter::You),
        );

        assert!(
            evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, alice)
            )
            .expect("artifact-entered condition should evaluate"),
            "expected Alice's historical artifact ETB to satisfy the condition"
        );
        assert!(
            !evaluate_condition(
                &game,
                &condition,
                &ExecutionContext::new_default(source, bob)
            )
            .expect("artifact-entered condition should evaluate"),
            "expected Bob not to satisfy Alice's historical artifact condition"
        );
    }

    #[test]
    fn evaluate_external_target_matches_uses_triggering_snapshot_and_source_power() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source_card = CardBuilder::new(CardId::from_raw(11), "Small Watcher")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let larger_card = CardBuilder::new(CardId::from_raw(12), "Departing Giant")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let departing = game.create_object_from_card(&larger_card, alice, Zone::Battlefield);
        let snapshot = {
            let object = game.object(departing).expect("departing creature exists");
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, &game,
            )
        };
        game.move_object_by_effect(departing, Zone::Graveyard);
        let event = TriggerEvent::new_with_provenance(
            crate::events::ZoneChangeEvent::with_cause(
                departing,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
                Some(snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let condition = Condition::TargetMatches(crate::target::ObjectFilter {
            card_types: vec![CardType::Creature],
            power: Some(crate::target::Comparison::GreaterThanExpr(Box::new(
                Value::PowerOf(Box::new(crate::target::ChooseSpec::Source)),
            ))),
            ..crate::target::ObjectFilter::default()
        });
        let ctx = ExternalEvaluationContext {
            controller: alice,
            source,
            defending_player: None,
            attacking_player: None,
            filter_source: None,
            triggering_event: Some(&event),
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };

        assert!(
            evaluate_condition_external(&game, &condition, &ctx),
            "trigger-time target condition should compare the LKI creature to the source's power"
        );
    }

    #[test]
    fn wave_of_rats_condition_true_when_source_dealt_combat_damage_to_player_this_turn() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let rat = CardBuilder::new(CardId::from_raw(13), "Wave of Rats")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 2))
            .build();
        let rat_id = game.create_object_from_card(&rat, alice, Zone::Battlefield);
        let source_snapshot = {
            let object = game.object(rat_id).expect("Wave of Rats exists");
            crate::snapshot::ObjectSnapshot::from_object(object, &game)
        };
        let damage = RawEvent::new(
            DamageEvent::with_cause(
                rat_id,
                DamageTarget::Player(bob),
                4,
                true,
                EventCause::effect(),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&damage, None, Some(source_snapshot));

        let ctx = ExecutionContext::new_default(rat_id, alice);
        assert!(
            evaluate_condition(
                &game,
                &Condition::SourceDealtCombatDamageToPlayerThisTurn,
                &ctx
            )
            .expect("source combat-damage condition should evaluate"),
            "expected Wave of Rats condition to pass after combat damage to a player"
        );
    }

    #[test]
    fn wave_of_rats_condition_false_without_combat_damage_to_player_this_turn() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);
        assert!(
            !evaluate_condition(
                &game,
                &Condition::SourceDealtCombatDamageToPlayerThisTurn,
                &ctx
            )
            .expect("source combat-damage condition should evaluate"),
            "expected Wave of Rats condition to fail without combat damage"
        );
    }

    #[test]
    fn first_combat_phase_condition_requires_started_first_combat() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);
        let condition = Condition::FirstCombatPhaseOfTurn;

        game.turn.phase = crate::game_state::Phase::Combat;
        game.turn_store.combat_phases_started_this_turn = 0;
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("first combat condition should evaluate"),
            "combat phase without a started combat count should not pass"
        );

        game.turn_store.combat_phases_started_this_turn = 1;
        assert!(
            evaluate_condition(&game, &condition, &ctx)
                .expect("first combat condition should evaluate"),
            "first started combat phase should pass"
        );

        game.turn_store.combat_phases_started_this_turn = 2;
        assert!(
            !evaluate_condition(&game, &condition, &ctx)
                .expect("first combat condition should evaluate"),
            "later combat phases should not pass"
        );
    }
}

fn player_has_card_in_hand_matching(
    game: &GameState,
    player: PlayerId,
    filter: &crate::target::ObjectFilter,
    filter_source: Option<ObjectId>,
) -> bool {
    let filter_ctx = game.filter_context_for(player, filter_source);
    game.player(player).is_some_and(|state| {
        state.hand.iter().any(|&card_id| {
            game.object(card_id)
                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
        })
    })
}

fn player_life_compares_to_half_starting(
    game: &GameState,
    player: PlayerId,
    inclusive: bool,
) -> bool {
    game.player(player).is_some_and(|state| {
        let doubled_life = state.life.saturating_mul(2);
        if inclusive {
            doubled_life <= state.starting_life
        } else {
            doubled_life < state.starting_life
        }
    })
}

fn evaluate_value_comparison(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    left: &Value,
    operator: crate::effect::ValueComparisonOperator,
    right: &Value,
) -> bool {
    let mut ctx = ExecutionContext::new_default(source, controller);
    let source_exiled = game
        .get_exiled_with_source_links(source)
        .iter()
        .filter_map(|id| {
            game.object(*id).map(|obj| {
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    obj, game,
                )
            })
        })
        .collect::<Vec<_>>();
    if !source_exiled.is_empty() {
        ctx.set_tagged_objects(crate::tag::SOURCE_EXILED_TAG, source_exiled);
    }
    let Ok(left_value) = resolve_value(game, left, &mut ctx) else {
        return false;
    };
    let Ok(right_value) = resolve_value(game, right, &mut ctx) else {
        return false;
    };
    operator.evaluate(left_value, right_value)
}

fn condition_count_for_player(
    game: &GameState,
    source: ObjectId,
    player_filter: &PlayerFilter,
    candidate: PlayerId,
    filter: &crate::target::ObjectFilter,
) -> usize {
    let opponents: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|p| p.id != candidate)
        .map(|p| p.id)
        .collect();
    let mut filter_ctx = crate::filter::FilterContext::new(candidate)
        .with_source(source)
        .with_opponents(opponents);
    if *player_filter == PlayerFilter::IteratedPlayer {
        filter_ctx = filter_ctx.with_iterated_player(Some(candidate));
    }
    condition_candidate_ids_for_zone(game, filter.zone)
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| condition_object_matches_player_zone(game, obj, candidate, filter.zone))
        .filter(|obj| filter.matches(obj, &filter_ctx, game))
        .count()
}

fn any_opponent_controls_more_than_player(
    game: &GameState,
    source: ObjectId,
    player_filter: &PlayerFilter,
    player_id: PlayerId,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let player_count = condition_count_for_player(game, source, player_filter, player_id, filter);
    game.players
        .iter()
        .filter(|p| p.id != player_id && p.is_in_game())
        .any(|opponent| {
            condition_count_for_player(game, source, player_filter, opponent.id, filter)
                > player_count
        })
}

fn player_has_more_life_than_each_other_player(game: &GameState, player_id: PlayerId) -> bool {
    let Some(life) = game.player(player_id).map(|p| p.life) else {
        return false;
    };
    game.players
        .iter()
        .filter(|candidate| candidate.is_in_game())
        .all(|candidate| candidate.id == player_id || life > candidate.life)
}

fn player_poison_counters_or_more(game: &GameState, player_id: PlayerId, count: u32) -> bool {
    game.player(player_id)
        .map(|player| player.poison_counters >= count)
        .unwrap_or(false)
}

fn player_has_no_opponent_with_more_life_than(game: &GameState, player_id: PlayerId) -> bool {
    let Some(life) = game.player(player_id).map(|p| p.life) else {
        return false;
    };
    game.players
        .iter()
        .filter(|candidate| candidate.is_in_game())
        .all(|candidate| candidate.id == player_id || life >= candidate.life)
}

#[derive(Debug, Clone, Copy)]
struct SharedConditionContext<'a> {
    controller: PlayerId,
    source: ObjectId,
    filter_source: Option<ObjectId>,
    triggering_event: Option<&'a TriggerEvent>,
    trigger_identity: Option<TriggerIdentity>,
}

fn object_matching_was_put_into_graveyard_from_battlefield_this_turn(
    game: &GameState,
    ctx: SharedConditionContext<'_>,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .any(|record| {
            record
                .event
                .downcast::<crate::events::zones::ZoneChangeEvent>()
                .is_some_and(|event| event.from == Zone::Battlefield && event.to == Zone::Graveyard)
                && record
                    .object_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| filter.matches_snapshot(snapshot, &filter_ctx, game))
        })
}

fn creature_card_was_put_into_your_graveyard_this_turn(game: &GameState, player: PlayerId) -> bool {
    let Some(player_state) = game.player(player) else {
        return false;
    };
    player_state.graveyard.iter().any(|card_id| {
        game.object(*card_id).is_some_and(|object| {
            game.object_has_card_type(object.id, crate::types::CardType::Creature)
                && game
                    .turn_store
                    .turn_history
                    .object_was_put_into_graveyard_this_turn(object.stable_id)
        })
    })
}

fn object_matching_entered_battlefield_this_turn(
    game: &GameState,
    ctx: SharedConditionContext<'_>,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .any(|record| {
            let entered = record
                .event
                .downcast::<crate::events::EnterBattlefieldEvent>()
                .is_some()
                || record
                    .event
                    .downcast::<crate::events::zones::ZoneChangeEvent>()
                    .is_some_and(|event| event.is_etb());
            entered
                && record
                    .object_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| filter.matches_snapshot(snapshot, &filter_ctx, game))
        })
}

fn object_matching_entered_battlefield_last_turn(
    game: &GameState,
    ctx: SharedConditionContext<'_>,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
    game.turn_store
        .entered_battlefield_last_turn
        .iter()
        .any(|snapshot| {
            if filter.other && snapshot.object_id == ctx.source {
                return false;
            }
            filter.matches_snapshot(snapshot, &filter_ctx, game)
        })
}

fn condition_filter_context(
    game: &GameState,
    you: PlayerId,
    source: ObjectId,
    player_filter: &PlayerFilter,
    triggering_event: Option<&TriggerEvent>,
) -> crate::filter::FilterContext {
    let opponents: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|p| p.id != you)
        .map(|p| p.id)
        .collect();
    let mut ctx = crate::filter::FilterContext::new(you)
        .with_source(source)
        .with_opponents(opponents);
    if *player_filter == PlayerFilter::IteratedPlayer {
        ctx = ctx.with_iterated_player(Some(you));
    }

    let Some(event) = triggering_event else {
        return ctx;
    };
    let Some(object_id) = event.object_id() else {
        return ctx;
    };
    let Some(snapshot) = event.snapshot().cloned().or_else(|| {
        game.object(object_id)
            .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, game))
    }) else {
        return ctx;
    };

    ctx.target_objects.push(snapshot.clone());
    ctx.tagged_objects
        .entry(crate::tag::TagKey::from("triggering"))
        .or_default()
        .push(snapshot);
    if let Some(entry) = game.stack.iter().find(|entry| entry.object_id == object_id) {
        ctx.target_objects
            .extend(entry.targets.iter().filter_map(|target| {
                match target {
            crate::game_state::Target::Object(target_id) => game.object(*target_id).map(|object| {
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    object, game,
                )
            }),
            crate::game_state::Target::Player(_) => None,
        }
            }));
    }
    ctx
}

fn triggering_object_had_to_attack_this_combat(
    game: &GameState,
    triggering_event: Option<&TriggerEvent>,
) -> bool {
    triggering_event
        .and_then(|event| event.object_id())
        .is_some_and(|object_id| {
            game.combat
                .as_ref()
                .is_some_and(|combat| combat.creature_had_to_attack_this_combat(object_id))
        })
}

fn evaluate_condition_shared_core(
    game: &GameState,
    condition: &Condition,
    ctx: SharedConditionContext<'_>,
) -> Option<bool> {
    match condition {
        Condition::LifeTotalOrLess(threshold) => Some(
            game.player(ctx.controller)
                .map(|p| p.life <= *threshold)
                .unwrap_or(false),
        ),
        Condition::LifeTotalOrGreater(threshold) => Some(
            game.player(ctx.controller)
                .map(|p| p.life >= *threshold)
                .unwrap_or(false),
        ),
        Condition::CardsInHandOrMore(threshold) => Some(
            game.player(ctx.controller)
                .map(|p| p.hand.len() as i32 >= *threshold)
                .unwrap_or(false),
        ),
        Condition::YouHaveCardInHandMatching(filter) => Some(player_has_card_in_hand_matching(
            game,
            ctx.controller,
            filter,
            ctx.filter_source,
        )),
        Condition::YourTurn => Some(game.turn.active_player == ctx.controller),
        Condition::YourFirstTurnsOfTheGameOrFewer(count) => {
            Some(game.turn.active_player == ctx.controller && game.turn.turn_number <= *count)
        }
        Condition::CreatureDiedThisTurn => Some(
            game.turn_store
                .turn_history
                .total_creatures_died_this_turn()
                > 0,
        ),
        Condition::CreatureDiedThisTurnOrMore(count) => Some(
            game.turn_store
                .turn_history
                .total_creatures_died_this_turn()
                >= *count,
        ),
        Condition::CreatureCardPutIntoYourGraveyardThisTurn => Some(
            creature_card_was_put_into_your_graveyard_this_turn(game, ctx.controller),
        ),
        Condition::CastSpellThisTurn => {
            Some(game.turn_store.turn_history.any_spell_was_cast_this_turn())
        }
        Condition::AttackedThisTurn => Some(
            game.turn_store
                .turn_history
                .players_attacked_this_turn
                .contains(&ctx.controller),
        ),
        Condition::OpponentLostLifeThisTurn => {
            let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
            Some(filter_ctx.opponents.iter().any(|opponent| {
                game.turn_store
                    .turn_history
                    .player_lost_life_this_turn(*opponent)
            }))
        }
        Condition::PermanentLeftBattlefieldThisTurn => Some(
            game.turn_store
                .turn_history
                .permanents_left_battlefield_this_turn()
                > 0,
        ),
        Condition::PermanentLeftBattlefieldUnderYourControlThisTurn => Some(
            game.turn_store
                .turn_history
                .permanents_left_battlefield_under_controller(ctx.controller)
                > 0,
        ),
        Condition::ObjectEnteredBattlefieldThisTurn(filter) => Some(
            object_matching_entered_battlefield_this_turn(game, ctx, filter),
        ),
        Condition::ObjectEnteredBattlefieldLastTurn(filter) => Some(
            object_matching_entered_battlefield_last_turn(game, ctx, filter),
        ),
        Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter) => Some(
            object_matching_was_put_into_graveyard_from_battlefield_this_turn(game, ctx, filter),
        ),
        Condition::SourceWasCast => Some(source_was_cast(game, ctx.source, ctx.triggering_event)),
        Condition::TaggedObjectWasCast(_) => None,
        Condition::ThisSpellEscaped => Some(source_escaped(game, ctx.source)),
        Condition::ThisSpellWasCastFromZone(_) => None,
        Condition::ThisSpellWasCastFromNonHand => None,
        Condition::NoSpellsWereCastLastTurn => {
            Some(game.turn_store.spells_cast_last_turn_total == 0)
        }
        Condition::ItIsNight => Some(game.is_night),
        Condition::FirstCombatPhaseOfTurn => Some(
            game.turn.phase == crate::game_state::Phase::Combat
                && game.turn_store.combat_phases_started_this_turn == 1,
        ),
        Condition::SpellsWereCastLastTurnOrMore(count) => {
            Some(game.turn_store.spells_cast_last_turn_total >= *count)
        }
        Condition::YouHaveFullParty => Some(player_has_full_party(game, ctx.controller)),
        Condition::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            let Some(source_obj) = game.object(ctx.source) else {
                return Some(false);
            };
            let spent = if let Some(sym) = symbol {
                source_obj.mana_spent_to_cast.amount(*sym)
            } else {
                source_obj.mana_spent_to_cast.total()
            };
            Some(spent >= *amount)
        }
        Condition::SameColorManaSpentToCastThisSpellAtLeast(amount) => {
            let Some(source_obj) = game.object(ctx.source) else {
                return Some(false);
            };
            let spent = &source_obj.mana_spent_to_cast;
            let most_spent_of_one_color =
                [spent.white, spent.blue, spent.black, spent.red, spent.green]
                    .into_iter()
                    .max()
                    .unwrap_or(0);
            Some(most_spent_of_one_color >= *amount)
        }
        Condition::ColorsOfManaSpentToCastThisSpellOrMore(amount) => {
            let Some(source_obj) = game.object(ctx.source) else {
                return Some(false);
            };
            let spent = &source_obj.mana_spent_to_cast;
            let distinct_colors = [
                spent.white > 0,
                spent.blue > 0,
                spent.black > 0,
                spent.red > 0,
                spent.green > 0,
            ]
            .into_iter()
            .filter(|present| *present)
            .count() as u32;
            Some(distinct_colors >= *amount)
        }
        Condition::SourceHasNoCounter(counter_type) => Some(
            game.object(ctx.source)
                .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) == 0)
                .unwrap_or(false),
        ),
        Condition::SourceHasCounterAtLeast {
            counter_type,
            count,
        } => Some(
            game.object(ctx.source)
                .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) >= *count)
                .unwrap_or(false),
        ),
        Condition::SourceHasCountersAtLeast(count) => Some(
            game.object(ctx.source)
                .map(|obj| obj.counters.values().copied().sum::<u32>() >= *count)
                .unwrap_or(false),
        ),
        Condition::SourcePowerAtLeast(min_power) => Some(
            game.calculated_power(ctx.source)
                .or_else(|| game.object(ctx.source).and_then(|obj| obj.power()))
                .is_some_and(|power| power >= *min_power as i32),
        ),
        Condition::SourceDealtCombatDamageToPlayerThisTurn => {
            Some(game.source_dealt_combat_damage_to_player_this_turn(ctx.source))
        }
        Condition::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype } => {
            let players = matching_condition_players_simple(game, ctx.controller, player);
            Some(
                game.turn_store
                    .turn_history
                    .player_was_dealt_combat_damage_by_creature_subtype_this_turn(
                        &players, *subtype,
                    ),
            )
        }
        Condition::SourceMatches(filter) => {
            let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
            Some(
                game.object(ctx.source)
                    .is_some_and(|obj| filter.matches(obj, &filter_ctx, game)),
            )
        }
        Condition::SourceCameUnderYourControlThisTurn => {
            Some(game.object(ctx.source).is_some_and(|obj| {
                game.turn_store
                    .turn_history
                    .object_came_under_controller_this_turn(obj.stable_id, ctx.controller)
            }))
        }
        Condition::SourceIsInZone(zone) => Some(
            game.object(ctx.source)
                .map(|obj| obj.zone == *zone)
                .unwrap_or(false),
        ),
        Condition::PlayerGraveyardHasCardsAtLeast { player, count } => Some(
            game.player(*player)
                .is_some_and(|p| p.graveyard.len() >= *count),
        ),
        Condition::SourceIsRingBearer { player } => Some(
            matching_condition_players_simple(game, ctx.controller, player)
                .into_iter()
                .any(|player_id| game.current_ring_bearer(player_id) == Some(ctx.source)),
        ),
        Condition::PlayerRingTemptedThisGameOrMore { player, count } => Some(
            matching_condition_players_simple(game, ctx.controller, player)
                .into_iter()
                .any(|player_id| game.ring_temptations(player_id) >= *count),
        ),
        Condition::YouControlCommander => {
            if let Some(player) = game.player(ctx.controller) {
                let commanders = player.get_commanders();
                for &commander_id in commanders {
                    if game.battlefield.contains(&commander_id)
                        && let Some(obj) = game.object(commander_id)
                        && game.controller_of(obj) == ctx.controller
                    {
                        return Some(true);
                    }
                    for &bf_id in &game.battlefield {
                        if let Some(obj) = game.object(bf_id)
                            && game.controller_of(obj) == ctx.controller
                            && obj.stable_id == StableId::from(commander_id)
                        {
                            return Some(true);
                        }
                    }
                }
            }
            Some(false)
        }
        Condition::ThisAbilityResolvedThisTurnExactly(count) => {
            Some(ctx.trigger_identity.is_some_and(|trigger_identity| {
                game.triggered_ability_resolution_count_this_turn(ctx.source, trigger_identity)
                    == *count
            }))
        }
        Condition::Custom(_) => Some(false),
        _ => None,
    }
}

fn assert_condition_variant_coverage(condition: &Condition) {
    match condition {
        Condition::YouControl(..) => {}
        Condition::OpponentControls(..) => {}
        Condition::PlayerControls { .. } => {}
        Condition::PlayerControlsAtLeast { .. } => {}
        Condition::PlayerControlsExactly { .. } => {}
        Condition::PlayerControlsAtLeastWithDifferentPowers { .. } => {}
        Condition::PlayerControlsMost { .. } => {}
        Condition::PlayerControlsMoreThanYou { .. } => {}
        Condition::AnOpponentControlsMoreThanPlayer { .. } => {}
        Condition::PlayerLifeAtMostHalfStartingLifeTotal { .. } => {}
        Condition::PlayerLifeLessThanHalfStartingLifeTotal { .. } => {}
        Condition::LifeTotalOrLess(..) => {}
        Condition::LifeTotalOrGreater(..) => {}
        Condition::CardsInHandOrMore(..) => {}
        Condition::YouHaveCardInHandMatching(..) => {}
        Condition::YourTurn => {}
        Condition::YourFirstTurnsOfTheGameOrFewer(..) => {}
        Condition::CreatureDiedThisTurn => {}
        Condition::CreatureDiedThisTurnOrMore(..) => {}
        Condition::CreatureCardPutIntoYourGraveyardThisTurn => {}
        Condition::CastSpellThisTurn => {}
        Condition::AttackedThisTurn => {}
        Condition::OpponentLostLifeThisTurn => {}
        Condition::PermanentLeftBattlefieldThisTurn => {}
        Condition::PermanentLeftBattlefieldUnderYourControlThisTurn => {}
        Condition::ObjectEnteredBattlefieldThisTurn(..) => {}
        Condition::ObjectEnteredBattlefieldLastTurn(..) => {}
        Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(..) => {}
        Condition::SourceWasCast => {}
        Condition::ThisSpellEscaped => {}
        Condition::ThisSpellWasCastFromZone(..) => {}
        Condition::ThisSpellWasCastFromNonHand => {}
        Condition::NoSpellsWereCastLastTurn => {}
        Condition::ItIsNight => {}
        Condition::FirstCombatPhaseOfTurn => {}
        Condition::SpellsWereCastLastTurnOrMore(..) => {}
        Condition::YouHaveFullParty => {}
        Condition::TargetIsTapped => {}
        Condition::TargetIsAttacking => {}
        Condition::TargetIsBlocked => {}
        Condition::TargetWasKicked => {}
        Condition::ThisSpellWasKicked => {}
        Condition::ThisSpellPaidLabel(..) => {}
        Condition::TargetSpellCastOrderThisTurn(..) => {}
        Condition::TargetSpellControllerIsPoisoned => {}
        Condition::TargetSpellManaSpentToCastAtLeast { .. } => {}
        Condition::YouControlMoreCreaturesThanTargetSpellController => {}
        Condition::TargetHasGreatestPowerAmongCreatures => {}
        Condition::TargetManaValueLteColorsSpentToCastThisSpell => {}
        Condition::SourceIsTapped => {}
        Condition::SourceIsSaddled => {}
        Condition::SourceDevouredCreaturesOrMore(..) => {}
        Condition::SourceIsMonstrous => {}
        Condition::SourceIsFaceDown => {}
        Condition::SourceMatches(..) => {}
        Condition::SourceHasNoCounter(..) => {}
        Condition::SourceHasCounterAtLeast { .. } => {}
        Condition::SourceHasCountersAtLeast(..) => {}
        Condition::SourcePowerAtLeast(..) => {}
        Condition::SourceDealtCombatDamageToPlayerThisTurn => {}
        Condition::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { .. } => {}
        Condition::SourceAttackedOrBlockedThisTurn => {}
        Condition::SourceIsInZone(..) => {}
        Condition::ManaSpentToCastThisSpellAtLeast { .. } => {}
        Condition::SameColorManaSpentToCastThisSpellAtLeast(..) => {}
        Condition::ColorsOfManaSpentToCastThisSpellOrMore(..) => {}
        Condition::YouControlCommander => {}
        Condition::TaggedObjectMatches(..) => {}
        Condition::TaggedObjectIsTopOfLibrary { .. } => {}
        Condition::StableObjectIsTopOfLibrary { .. } => {}
        Condition::TaggedObjectWasCast(..) => {}
        Condition::TaggedObjectIsSoulbondPaired(..) => {}
        Condition::EnchantedPermanentAttackedThisTurn => {}
        Condition::TargetMatches(..) => {}
        Condition::TargetIsSoulbondPaired => {}
        Condition::PlayerTaggedObjectMatches { .. } => {}
        Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn { .. } => {}
        Condition::PlayerOwnsCardNamedInZones { .. } => {}
        Condition::ThisAbilityResolvedThisTurnExactly(..) => {}
        Condition::FirstTimeThisTurn => {}
        Condition::MaxTimesEachTurn(..) => {}
        Condition::DoThisMaxTimesEachTurn(..) => {}
        Condition::TriggeringObjectWasEnchanted => {}
        Condition::TriggeringObjectHadToAttackThisCombat => {}
        Condition::TriggeringObjectHadCounters { .. } => {}
        Condition::ControlCreaturesTotalPowerAtLeast(..) => {}
        Condition::CardInYourGraveyard { .. } => {}
        Condition::ActivationTiming(..) => {}
        Condition::MaxActivationsPerTurn(..) => {}
        Condition::SourceIsEquipped => {}
        Condition::SourceIsEnchanted => {}
        Condition::SecretChoicesMatch => {}
        Condition::VoteOptionGetsMoreVotes(..) => {}
        Condition::VoteOptionGetsMoreVotesOrTied(..) => {}
        Condition::EnchantedPermanentIsCreature => {}
        Condition::EnchantedPermanentIsLand => {}
        Condition::EnchantedPermanentIsEquipment => {}
        Condition::EnchantedPermanentIsVehicle => {}
        Condition::EquippedCreatureTapped => {}
        Condition::EquippedCreatureUntapped => {}
        Condition::EquippedCreatureAttacking => {}
        Condition::SourceChosenOption(..) => {}
        Condition::CountComparison { .. } => {}
        Condition::OwnsCardExiledWithCounter(..) => {}
        Condition::SourceAttackedThisTurn => {}
        Condition::SourceCameUnderYourControlThisTurn => {}
        Condition::SourceIsUntapped => {}
        Condition::SourceIsAttacking => {}
        Condition::SourceIsBlocking => {}
        Condition::SourceIsSoulbondPaired => {}
        Condition::XValueAtLeast(..) => {}
        Condition::Custom(..) => {}
        Condition::Not(..) => {}
        Condition::And(..) => {}
        Condition::Or(..) => {}
        Condition::PlayerCastSpellsThisTurnOrMore { .. } => {}
        Condition::PlayerTappedLandForManaThisTurn { .. } => {}
        Condition::PlayerGainedLifeThisTurnOrMore { .. } => {}
        Condition::PlayerHadLandEnterBattlefieldThisTurn { .. } => {}
        Condition::ValueComparison { .. } => {}
        Condition::PlayerCardsInHandOrMore { .. } => {}
        Condition::PlayerCardsInHandOrFewer { .. } => {}
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { .. } => {}
        Condition::PlayerHasCardTypesInGraveyardOrMore { .. } => {}
        Condition::PlayerHasLessLifeThanYou { .. } => {}
        Condition::PlayerHasMoreLifeThanYou { .. } => {}
        Condition::PlayerHasNoOpponentWithMoreLifeThan { .. } => {}
        Condition::PlayerHasMoreLifeThanEachOtherPlayer { .. } => {}
        Condition::PlayerHasMoreCardsInHandThanYou { .. } => {}
        Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer { .. } => {}
        Condition::PlayerHasPoisonCountersOrMore { .. } => {}
        Condition::PlayerIsMonarch { .. } => {}
        Condition::PlayerHasInitiative { .. } => {}
        Condition::PlayerHasCitysBlessing { .. } => {}
        Condition::SourceIsRingBearer { .. } => {}
        Condition::PlayerRingTemptedThisGameOrMore { .. } => {}
        Condition::PlayerCommittedCrimeThisTurn { .. } => {}
        Condition::PlayerRolledResultThisTurn { .. } => {}
        Condition::PlayerCompletedDungeon { .. } => {}
        Condition::PlayerGraveyardHasCardsAtLeast { .. } => {}
    }
}

/// Condition evaluation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionEvaluationMode {
    /// Cast-time evaluation: no full execution context is available yet.
    CastTime {
        controller: PlayerId,
        source: ObjectId,
    },
    /// Resolution-time evaluation: full execution context is available.
    Resolution,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExternalEvaluationOptions {
    /// If true, treat timing restrictions as satisfied.
    pub ignore_timing: bool,
    /// If true, treat per-turn activation limits as satisfied.
    pub ignore_activation_limits: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExternalEvaluationContext<'a> {
    pub controller: PlayerId,
    pub source: ObjectId,
    /// Player currently being attacked (if evaluation occurs in an attack-defender context).
    pub defending_player: Option<PlayerId>,
    /// Player currently attacking (if different from `controller` in a delegated context).
    pub attacking_player: Option<PlayerId>,
    /// The `FilterContext.source` used when matching ObjectFilters.
    ///
    /// This is intentionally configurable to preserve established semantics:
    /// - Intervening-if checks historically passed `None` so `other` filters do not exclude the source.
    /// - Most other checks should pass `Some(source)`.
    pub filter_source: Option<ObjectId>,
    pub triggering_event: Option<&'a TriggerEvent>,
    pub trigger_identity: Option<TriggerIdentity>,
    pub ability_index: Option<usize>,
    pub options: ExternalEvaluationOptions,
}

/// Evaluate a condition outside of effect resolution (trigger checks, activation gating, statics).
pub fn evaluate_condition_external(
    game: &GameState,
    condition: &Condition,
    ctx: &ExternalEvaluationContext<'_>,
) -> bool {
    assert_condition_variant_coverage(condition);
    use crate::types::{CardType, Subtype};

    if let Condition::Not(inner) = condition {
        return !evaluate_condition_external(game, inner, ctx);
    }
    if let Condition::And(a, b) = condition {
        return evaluate_condition_external(game, a, ctx)
            && evaluate_condition_external(game, b, ctx);
    }
    if let Condition::Or(a, b) = condition {
        return evaluate_condition_external(game, a, ctx)
            || evaluate_condition_external(game, b, ctx);
    }
    if let Some(result) = evaluate_condition_shared_core(
        game,
        condition,
        SharedConditionContext {
            controller: ctx.controller,
            source: ctx.source,
            filter_source: ctx.filter_source,
            triggering_event: ctx.triggering_event,
            trigger_identity: ctx.trigger_identity,
        },
    ) {
        return result;
    }
    if let Condition::ValueComparison {
        left,
        operator,
        right,
    } = condition
    {
        return evaluate_value_comparison(game, ctx.controller, ctx.source, left, *operator, right);
    }

    match condition {
        Condition::XValueAtLeast(_) => false, // X not available in static context
        Condition::ItIsNight => game.is_night,
        Condition::FirstCombatPhaseOfTurn => {
            game.turn.phase == crate::game_state::Phase::Combat
                && game.turn_store.combat_phases_started_this_turn == 1
        }
        Condition::ThisSpellEscaped => source_escaped(game, ctx.source),
        Condition::ThisSpellWasKicked => game
            .object(ctx.source)
            .is_some_and(|obj| obj.optional_costs_paid.was_kicked()),
        Condition::ThisSpellWasCastFromZone(_) => false,
        Condition::ThisSpellWasCastFromNonHand => false,
        Condition::ThisSpellPaidLabel(label) => game
            .object(ctx.source)
            .is_some_and(|obj| obj.optional_costs_paid.was_paid_label(label)),
        Condition::YouHaveFullParty => player_has_full_party(game, ctx.controller),
        Condition::YouControl(filter) => {
            let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
            game.battlefield.iter().any(|&obj_id| {
                game.object(obj_id).is_some_and(|obj| {
                    game.controller_of(obj) == ctx.controller
                        && filter.matches(obj, &filter_ctx, game)
                })
            })
        }
        Condition::OpponentControls(filter) => {
            let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
            let opponents = &filter_ctx.opponents;
            game.battlefield.iter().any(|&obj_id| {
                game.object(obj_id).is_some_and(|obj| {
                    opponents.contains(&game.controller_of(obj))
                        && filter.matches(obj, &filter_ctx, game)
                })
            })
        }
        Condition::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
            let players: Vec<PlayerId> = match player {
                PlayerFilter::You => vec![ctx.controller],
                PlayerFilter::Opponent => filter_ctx.opponents.clone(),
                PlayerFilter::Specific(id) => vec![*id],
                PlayerFilter::Any => game.players.iter().map(|p| p.id).collect(),
                PlayerFilter::NotYou => game
                    .players
                    .iter()
                    .filter_map(|p| (p.id != ctx.controller).then_some(p.id))
                    .collect(),
                _ => Vec::new(),
            };
            let cast_count: u32 = players
                .iter()
                .map(|pid| game.turn_store.turn_history.spells_cast_by_player(*pid))
                .sum();
            cast_count >= *count
        }
        Condition::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype } => {
            let players = matching_condition_players_external(game, ctx, player);
            game.turn_store
                .turn_history
                .player_was_dealt_combat_damage_by_creature_subtype_this_turn(&players, *subtype)
        }
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.turn_store
                .turn_history
                .players_tapped_land_for_mana_this_turn
                .contains(&player_id)
        }
        Condition::PlayerGainedLifeThisTurnOrMore { player, count } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.turn_store
                .turn_history
                .total_life_gained_for_players(&[player_id])
                >= *count
        }
        Condition::CreatureDiedThisTurnOrMore(count) => {
            game.turn_store
                .turn_history
                .total_creatures_died_this_turn()
                >= *count
        }
        Condition::CreatureCardPutIntoYourGraveyardThisTurn => {
            creature_card_was_put_into_your_graveyard_this_turn(game, ctx.controller)
        }
        Condition::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            player_had_land_enter_battlefield_this_turn(game, player_id)
        }
        Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn { player, tag } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let _ = (player_id, tag);
            false
        }
        Condition::PlayerCardsInHandOrMore { player, count } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.player(player_id)
                .map(|p| p.hand.len() as i32 >= *count)
                .unwrap_or(false)
        }
        Condition::PlayerCardsInHandOrFewer { player, count } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.player(player_id)
                .map(|p| p.hand.len() as i32 <= *count)
                .unwrap_or(false)
        }
        Condition::PlayerHasLessLifeThanYou { player } => {
            let you_life = game.player(ctx.controller).map(|p| p.life).unwrap_or(0);
            matching_condition_players_external(game, ctx, player)
                .into_iter()
                .any(|player_id| game.player(player_id).map(|p| p.life).unwrap_or(0) < you_life)
        }
        Condition::PlayerLifeAtMostHalfStartingLifeTotal { player } => {
            matching_condition_players_external(game, ctx, player)
                .into_iter()
                .any(|player_id| player_life_compares_to_half_starting(game, player_id, true))
        }
        Condition::PlayerLifeLessThanHalfStartingLifeTotal { player } => {
            matching_condition_players_external(game, ctx, player)
                .into_iter()
                .any(|player_id| player_life_compares_to_half_starting(game, player_id, false))
        }
        Condition::PlayerHasMoreLifeThanYou { player } => {
            let you_life = game.player(ctx.controller).map(|p| p.life).unwrap_or(0);
            matching_condition_players_external(game, ctx, player)
                .into_iter()
                .any(|player_id| game.player(player_id).map(|p| p.life).unwrap_or(0) > you_life)
        }
        Condition::PlayerHasNoOpponentWithMoreLifeThan { player } => {
            matching_condition_players_external(game, ctx, player)
                .into_iter()
                .any(|player_id| player_has_no_opponent_with_more_life_than(game, player_id))
        }
        Condition::PlayerHasMoreLifeThanEachOtherPlayer { player } => {
            matching_condition_players_external(game, ctx, player)
                .into_iter()
                .any(|player_id| player_has_more_life_than_each_other_player(game, player_id))
        }
        Condition::PlayerHasMoreCardsInHandThanYou { player } => {
            let your_hand = game
                .player(ctx.controller)
                .map(|p| p.hand.len())
                .unwrap_or(0);
            matching_condition_players_external(game, ctx, player)
                .into_iter()
                .any(|player_id| {
                    game.player(player_id).map(|p| p.hand.len()).unwrap_or(0) > your_hand
                })
        }
        Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer { player } => {
            matching_condition_players_external(game, ctx, player)
                .into_iter()
                .any(|player_id| {
                    let hand = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
                    game.players
                        .iter()
                        .filter(|candidate| candidate.is_in_game())
                        .all(|candidate| candidate.id == player_id || hand > candidate.hand.len())
                })
        }
        Condition::PlayerHasPoisonCountersOrMore { player, count } => {
            matching_condition_players_external(game, ctx, player)
                .into_iter()
                .any(|player_id| player_poison_counters_or_more(game, player_id, *count))
        }
        Condition::PlayerIsMonarch { player } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.is_monarch(player_id)
        }
        Condition::PlayerHasInitiative { player } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.has_initiative(player_id)
        }
        Condition::PlayerHasCitysBlessing { player } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.has_citys_blessing(player_id)
        }
        Condition::PlayerCommittedCrimeThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.turn_store
                .turn_history
                .player_committed_crime_this_turn(player_id)
        }
        Condition::PlayerRolledResultThisTurn { player, result } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.turn_store
                .turn_history
                .player_rolled_result_this_turn(player_id, *result)
        }
        Condition::PlayerCompletedDungeon {
            player,
            dungeon_name,
        } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            match dungeon_name {
                Some(name) => game.has_completed_named_dungeon(player_id, name),
                None => game.has_completed_dungeon(player_id),
            }
        }

        Condition::FirstTimeThisTurn => ctx
            .trigger_identity
            .map(|id| game.trigger_fire_count_this_turn(ctx.source, id) == 0)
            .unwrap_or(true),
        Condition::MaxTimesEachTurn(limit) | Condition::DoThisMaxTimesEachTurn(limit) => ctx
            .trigger_identity
            .map(|id| game.trigger_fire_count_this_turn(ctx.source, id) < *limit)
            .unwrap_or(true),
        Condition::TriggeringObjectWasEnchanted => ctx
            .triggering_event
            .and_then(|event| event.snapshot())
            .is_some_and(|snapshot| snapshot.was_enchanted),
        Condition::TriggeringObjectHadToAttackThisCombat => {
            triggering_object_had_to_attack_this_combat(game, ctx.triggering_event)
        }
        Condition::TriggeringObjectHadCounters {
            counter_type,
            min_count,
        } => ctx
            .triggering_event
            .and_then(|event| event.snapshot())
            .is_some_and(|snapshot| {
                snapshot.counters.get(counter_type).copied().unwrap_or(0) >= *min_count
            }),

        Condition::ControlCreaturesTotalPowerAtLeast(required_power) => {
            let total_power = game
                .battlefield
                .iter()
                .copied()
                .filter(|&id| {
                    game.object(id).is_some_and(|obj| {
                        game.controller_of(obj) == ctx.controller && game.current_is_creature(id)
                    })
                })
                .map(|id| game.current_power(id).unwrap_or(0).max(0))
                .sum::<i32>();
            total_power >= *required_power as i32
        }
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count } => {
            use crate::types::Subtype;
            use std::collections::HashSet;

            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };

            let mut seen: HashSet<Subtype> = HashSet::new();
            for obj in game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| game.controller_of(obj) == player_id && obj.is_land())
            {
                for subtype in game.calculated_subtypes(obj.id) {
                    if matches!(
                        subtype,
                        Subtype::Plains
                            | Subtype::Island
                            | Subtype::Swamp
                            | Subtype::Mountain
                            | Subtype::Forest
                    ) {
                        seen.insert(subtype);
                    }
                }
            }
            seen.len() >= *count as usize
        }
        Condition::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            count_distinct_card_types_in_graveyard(game, player_id) >= *count as usize
        }
        Condition::CardInYourGraveyard {
            card_types,
            subtypes,
        } => game.player(ctx.controller).is_some_and(|player_state| {
            player_state.graveyard.iter().any(|&card_id| {
                if game.object(card_id).is_none() {
                    return false;
                }
                let card_type_match = card_types.is_empty()
                    || card_types
                        .iter()
                        .any(|card_type| game.current_has_card_type(card_id, *card_type));
                let subtype_match = subtypes.is_empty()
                    || subtypes
                        .iter()
                        .any(|subtype| game.current_has_subtype(card_id, *subtype));
                card_type_match && subtype_match
            })
        }),
        Condition::PlayerControls { player, filter } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let filter_ctx =
                condition_filter_context(game, player_id, ctx.source, player, ctx.triggering_event);
            condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    condition_object_matches_player_zone(game, obj, player_id, filter.zone)
                })
                .any(|obj| filter.matches(obj, &filter_ctx, game))
        }
        Condition::PlayerControlsAtLeast {
            player,
            filter,
            count,
        } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let filter_ctx =
                condition_filter_context(game, player_id, ctx.source, player, ctx.triggering_event);
            let matches = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    condition_object_matches_player_zone(game, obj, player_id, filter.zone)
                })
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .count();
            matches >= *count as usize
        }
        Condition::PlayerControlsExactly {
            player,
            filter,
            count,
        } => matching_condition_players_external(game, ctx, player)
            .into_iter()
            .any(|player_id| {
                let filter_ctx = condition_filter_context(
                    game,
                    player_id,
                    ctx.source,
                    player,
                    ctx.triggering_event,
                );
                condition_candidate_ids_for_zone(game, filter.zone)
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        condition_object_matches_player_zone(game, obj, player_id, filter.zone)
                    })
                    .filter(|obj| filter.matches(obj, &filter_ctx, game))
                    .count()
                    == *count as usize
            }),
        Condition::PlayerControlsAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let filter_ctx =
                condition_filter_context(game, player_id, ctx.source, player, ctx.triggering_event);
            count_distinct_matching_powers(game, player_id, filter, &filter_ctx) >= *count as usize
        }
        Condition::PlayerControlsMost { player, filter } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let filter_ctx =
                condition_filter_context(game, player_id, ctx.source, player, ctx.triggering_event);
            let your_count = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    condition_object_matches_player_zone(game, obj, player_id, filter.zone)
                })
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .count();
            game.players.iter().filter(|p| p.id != player_id).all(|p| {
                let other_id = p.id;
                let other_ctx = condition_filter_context(
                    game,
                    other_id,
                    ctx.source,
                    player,
                    ctx.triggering_event,
                );
                let other_count = condition_candidate_ids_for_zone(game, filter.zone)
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        condition_object_matches_player_zone(game, obj, other_id, filter.zone)
                    })
                    .filter(|obj| filter.matches(obj, &other_ctx, game))
                    .count();
                your_count >= other_count
            })
        }
        Condition::PlayerControlsMoreThanYou { player, filter } => {
            let count_for = |candidate: PlayerId| {
                let filter_ctx = condition_filter_context(
                    game,
                    candidate,
                    ctx.source,
                    player,
                    ctx.triggering_event,
                );
                condition_candidate_ids_for_zone(game, filter.zone)
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        condition_object_matches_player_zone(game, obj, candidate, filter.zone)
                    })
                    .filter(|obj| filter.matches(obj, &filter_ctx, game))
                    .count()
            };
            matching_condition_players_external(game, ctx, player)
                .into_iter()
                .any(|player_id| count_for(player_id) > count_for(ctx.controller))
        }
        Condition::AnOpponentControlsMoreThanPlayer { player, filter } => {
            matching_condition_players_external(game, ctx, player)
                .into_iter()
                .any(|player_id| {
                    any_opponent_controls_more_than_player(
                        game, ctx.source, player, player_id, filter,
                    )
                })
        }
        Condition::PlayerOwnsCardNamedInZones {
            player,
            name,
            zones,
        } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut filter_ctx = crate::filter::FilterContext::new(player_id)
                .with_source(ctx.source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                filter_ctx = filter_ctx.with_iterated_player(Some(player_id));
            }
            if zones.is_empty() {
                return false;
            }

            let mut filter = crate::target::ObjectFilter::default().named(name.clone());
            for zone in zones {
                filter.zone = Some(*zone);
                let has_matching = condition_candidate_ids_for_zone(game, Some(*zone))
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| obj.owner == player_id)
                    .any(|obj| filter.matches(obj, &filter_ctx, game));
                if !has_matching {
                    return false;
                }
            }
            true
        }
        Condition::ActivationTiming(timing) => {
            if ctx.options.ignore_timing {
                return true;
            }
            match timing {
                crate::ability::ActivationTiming::AnyTime => true,
                crate::ability::ActivationTiming::DuringCombat => {
                    matches!(game.turn.phase, crate::game_state::Phase::Combat)
                }
                crate::ability::ActivationTiming::SorcerySpeed => {
                    game.turn.active_player == ctx.controller
                        && matches!(
                            game.turn.phase,
                            crate::game_state::Phase::FirstMain
                                | crate::game_state::Phase::NextMain
                        )
                        && game.stack_is_empty()
                }
                crate::ability::ActivationTiming::OncePerTurn => {
                    let Some(ability_index) = ctx.ability_index else {
                        return false;
                    };
                    game.ability_activation_count_this_turn(ctx.source, ability_index) == 0
                }
                crate::ability::ActivationTiming::DuringYourTurn => {
                    game.turn.active_player == ctx.controller
                }
                crate::ability::ActivationTiming::DuringOpponentsTurn => {
                    game.turn.active_player != ctx.controller
                }
            }
        }
        Condition::MaxActivationsPerTurn(limit) => {
            if ctx.options.ignore_activation_limits {
                return true;
            }
            let Some(ability_index) = ctx.ability_index else {
                return false;
            };
            game.ability_activation_count_this_turn(ctx.source, ability_index) < *limit
        }

        Condition::SourceIsEquipped => game.object(ctx.source).is_some_and(|source_obj| {
            source_obj.attachments.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.subtypes.contains(&Subtype::Equipment))
            })
        }),
        Condition::SourceIsEnchanted => game.object(ctx.source).is_some_and(|source_obj| {
            source_obj.attachments.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.subtypes.contains(&Subtype::Aura))
            })
        }),
        Condition::EnchantedPermanentIsCreature => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| game.object_has_card_type(attached, CardType::Creature)),
        Condition::EnchantedPermanentIsLand => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| game.object_has_card_type(attached, CardType::Land)),
        Condition::EnchantedPermanentIsEquipment => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| {
                game.calculated_subtypes(attached)
                    .contains(&crate::types::Subtype::Equipment)
            }),
        Condition::EnchantedPermanentIsVehicle => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| {
                game.calculated_subtypes(attached)
                    .contains(&crate::types::Subtype::Vehicle)
            }),
        Condition::EquippedCreatureTapped => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| game.is_tapped(attached)),
        Condition::EquippedCreatureUntapped => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| !game.is_tapped(attached)),
        Condition::EquippedCreatureAttacking => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| {
                game.combat
                    .as_ref()
                    .is_some_and(|combat| crate::combat_state::is_attacking(combat, attached))
            }),
        Condition::SourceChosenOption(expected) => game
            .chosen_named_option(ctx.source)
            .is_some_and(|chosen| chosen.eq_ignore_ascii_case(expected)),
        Condition::SecretChoicesMatch => false,
        Condition::CountComparison {
            count, comparison, ..
        } => comparison.evaluate(crate::static_abilities::resolve_anthem_count_expression(
            count,
            game,
            ctx.source,
            ctx.controller,
        )),
        Condition::OwnsCardExiledWithCounter(counter) => game.exile.iter().any(|&id| {
            game.object(id).is_some_and(|obj| {
                obj.owner == ctx.controller && obj.counters.get(counter).copied().unwrap_or(0) > 0
            })
        }),

        Condition::SourceAttackedThisTurn => game.creature_attacked_this_turn(ctx.source),
        Condition::SourceDealtCombatDamageToPlayerThisTurn => {
            game.source_dealt_combat_damage_to_player_this_turn(ctx.source)
        }
        Condition::SourceCameUnderYourControlThisTurn => {
            game.object(ctx.source).is_some_and(|obj| {
                game.turn_store
                    .turn_history
                    .object_came_under_controller_this_turn(obj.stable_id, ctx.controller)
            })
        }
        Condition::SourceAttackedOrBlockedThisTurn => {
            game.creature_attacked_this_turn(ctx.source)
                || game.creature_blocked_this_turn(ctx.source)
        }
        Condition::SourceIsTapped => game.is_tapped(ctx.source),
        Condition::SourceIsSaddled => game.is_saddled(ctx.source),
        Condition::SourceDevouredCreaturesOrMore(count) => {
            game.devoured_count(ctx.source) >= *count
        }
        Condition::SourceIsMonstrous => game.is_monstrous(ctx.source),
        Condition::SourceIsFaceDown => source_is_face_down_or_alternate_face(game, ctx.source),
        Condition::SourceMatches(filter) => {
            let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
            game.object(ctx.source)
                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
        }
        Condition::TargetMatches(filter) => {
            let filter_ctx = condition_filter_context(
                game,
                ctx.controller,
                ctx.source,
                &PlayerFilter::You,
                ctx.triggering_event,
            );
            let Some(event) = ctx.triggering_event else {
                return false;
            };
            if let Some(snapshot) = event.snapshot() {
                return filter.matches_snapshot(snapshot, &filter_ctx, game);
            }
            event.object_id().is_some_and(|object_id| {
                game.object(object_id)
                    .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
            })
        }
        Condition::SourcePowerAtLeast(min_power) => game
            .calculated_power(ctx.source)
            .or_else(|| game.object(ctx.source).and_then(|obj| obj.power()))
            .is_some_and(|power| power >= *min_power as i32),
        Condition::SourceHasCountersAtLeast(count) => game
            .object(ctx.source)
            .is_some_and(|obj| obj.counters.values().copied().sum::<u32>() >= *count),
        Condition::SourceIsUntapped => !game.is_tapped(ctx.source),
        Condition::SourceIsAttacking => game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_attacking(combat, ctx.source)),
        Condition::SourceIsBlocking => game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_blocking(combat, ctx.source)),
        Condition::SourceIsSoulbondPaired => game.is_soulbond_paired(ctx.source),
        Condition::StableObjectIsTopOfLibrary {
            stable_id,
            player,
            library_top_revision,
        } => crate::grant_registry::stable_card_is_top_of_library_at_revision(
            game,
            *stable_id,
            *player,
            *library_top_revision,
        ),

        // Conditions requiring targets / effect execution context are not evaluable here.
        Condition::TaggedObjectMatches(_, _)
        | Condition::TaggedObjectIsTopOfLibrary { .. }
        | Condition::TaggedObjectWasCast(_)
        | Condition::TaggedObjectIsSoulbondPaired(_)
        | Condition::EnchantedPermanentAttackedThisTurn
        | Condition::TargetIsSoulbondPaired
        | Condition::PlayerTaggedObjectMatches { .. }
        | Condition::TargetIsTapped
        | Condition::TargetIsAttacking
        | Condition::TargetIsBlocked
        | Condition::TargetWasKicked
        | Condition::TargetSpellCastOrderThisTurn(_)
        | Condition::TargetSpellControllerIsPoisoned
        | Condition::TargetSpellManaSpentToCastAtLeast { .. }
        | Condition::YouControlMoreCreaturesThanTargetSpellController
        | Condition::TargetHasGreatestPowerAmongCreatures
        | Condition::TargetManaValueLteColorsSpentToCastThisSpell
        | Condition::VoteOptionGetsMoreVotes(_)
        | Condition::VoteOptionGetsMoreVotesOrTied(_) => false,
        Condition::Custom(_)
        | Condition::LifeTotalOrLess(_)
        | Condition::LifeTotalOrGreater(_)
        | Condition::CardsInHandOrMore(_)
        | Condition::YouHaveCardInHandMatching(_)
        | Condition::YourTurn
        | Condition::YourFirstTurnsOfTheGameOrFewer(_)
        | Condition::CreatureDiedThisTurn
        | Condition::CastSpellThisTurn
        | Condition::AttackedThisTurn
        | Condition::OpponentLostLifeThisTurn
        | Condition::PermanentLeftBattlefieldThisTurn
        | Condition::PermanentLeftBattlefieldUnderYourControlThisTurn
        | Condition::ObjectEnteredBattlefieldThisTurn(_)
        | Condition::ObjectEnteredBattlefieldLastTurn(_)
        | Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(_)
        | Condition::SourceWasCast
        | Condition::NoSpellsWereCastLastTurn
        | Condition::SpellsWereCastLastTurnOrMore(_)
        | Condition::SourceHasNoCounter(_)
        | Condition::SourceHasCounterAtLeast { .. }
        | Condition::SourceIsInZone(_)
        | Condition::ManaSpentToCastThisSpellAtLeast { .. }
        | Condition::SameColorManaSpentToCastThisSpellAtLeast(_)
        | Condition::ColorsOfManaSpentToCastThisSpellOrMore(_)
        | Condition::PlayerGraveyardHasCardsAtLeast { .. }
        | Condition::SourceIsRingBearer { .. }
        | Condition::PlayerRingTemptedThisGameOrMore { .. }
        | Condition::ValueComparison { .. }
        | Condition::YouControlCommander
        | Condition::ThisAbilityResolvedThisTurnExactly(_)
        | Condition::Not(_)
        | Condition::And(_, _)
        | Condition::Or(_, _) => unreachable!("handled before external match"),
    }
}

/// Shared dispatcher for condition evaluation.
pub fn evaluate_condition_with_mode(
    game: &GameState,
    condition: &Condition,
    mode: ConditionEvaluationMode,
    ctx: Option<&ExecutionContext>,
) -> Result<bool, ExecutionError> {
    match mode {
        ConditionEvaluationMode::CastTime { controller, source } => Ok(evaluate_condition_simple(
            game, condition, controller, source,
        )),
        ConditionEvaluationMode::Resolution => {
            let ctx = ctx.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "resolution condition evaluation requires execution context".to_string(),
                )
            })?;
            evaluate_condition(game, condition, ctx)
        }
    }
}

/// Evaluate a condition for cast-time decisions.
pub fn evaluate_condition_cast_time(
    game: &GameState,
    condition: &Condition,
    controller: PlayerId,
    source: ObjectId,
) -> bool {
    evaluate_condition_with_mode(
        game,
        condition,
        ConditionEvaluationMode::CastTime { controller, source },
        None,
    )
    .unwrap_or(false)
}

/// Evaluate a condition during effect resolution.
pub fn evaluate_condition_resolution(
    game: &GameState,
    condition: &Condition,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    evaluate_condition_with_mode(
        game,
        condition,
        ConditionEvaluationMode::Resolution,
        Some(ctx),
    )
}

fn condition_candidate_ids_for_zone(game: &GameState, zone: Option<Zone>) -> Vec<ObjectId> {
    candidate_ids_for_zone(game, zone)
}

fn condition_object_matches_player_zone(
    game: &GameState,
    obj: &crate::object::Object,
    player_id: PlayerId,
    zone: Option<Zone>,
) -> bool {
    match zone {
        Some(Zone::Battlefield) | None => game.controller_of(obj) == player_id,
        _ => obj.owner == player_id,
    }
}

fn count_distinct_card_types_in_graveyard(game: &GameState, player_id: PlayerId) -> usize {
    use std::collections::HashSet;

    let Some(player_state) = game.player(player_id) else {
        return 0;
    };

    let mut seen = HashSet::new();
    for &object_id in &player_state.graveyard {
        for card_type in game.calculated_card_types(object_id) {
            seen.insert(card_type);
        }
    }
    seen.len()
}

fn count_distinct_matching_powers(
    game: &GameState,
    player_id: PlayerId,
    filter: &crate::target::ObjectFilter,
    filter_ctx: &crate::filter::FilterContext,
) -> usize {
    use std::collections::HashSet;

    let mut seen_powers = HashSet::new();
    for obj in condition_candidate_ids_for_zone(game, filter.zone)
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| condition_object_matches_player_zone(game, obj, player_id, filter.zone))
        .filter(|obj| filter.matches(obj, filter_ctx, game))
    {
        if let Some(power) = game.calculated_power(obj.id).or_else(|| obj.power()) {
            seen_powers.insert(power);
        }
    }
    seen_powers.len()
}

fn player_had_land_enter_battlefield_this_turn(game: &GameState, player_id: PlayerId) -> bool {
    game.turn_store
        .turn_history
        .player_had_land_enter_battlefield_this_turn(player_id)
}

fn player_has_full_party(game: &GameState, player_id: PlayerId) -> bool {
    let has_role = |role: crate::types::Subtype| {
        game.battlefield.iter().copied().any(|id| {
            game.current_controller(id) == Some(player_id)
                && game.current_has_card_type(id, crate::types::CardType::Creature)
                && game.current_has_subtype(id, role)
        })
    };

    has_role(crate::types::Subtype::Cleric)
        && has_role(crate::types::Subtype::Rogue)
        && has_role(crate::types::Subtype::Warrior)
        && has_role(crate::types::Subtype::Wizard)
}

/// Evaluate a condition with minimal context (for cast-time evaluation).
///
/// This simplified version is used during spell casting to evaluate conditions
/// like `YouControlCommander` before targets are chosen. It handles common
/// conditions that don't require targets or other context-dependent information.
fn evaluate_condition_simple(
    game: &GameState,
    condition: &Condition,
    controller: PlayerId,
    source: ObjectId,
) -> bool {
    assert_condition_variant_coverage(condition);
    // Build a simple filter context with opponents
    let opponents: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|p| p.id != controller)
        .map(|p| p.id)
        .collect();
    let filter_ctx = crate::filter::FilterContext::new(controller)
        .with_source(source)
        .with_opponents(opponents.clone());

    if let Condition::Not(inner) = condition {
        return !evaluate_condition_simple(game, inner, controller, source);
    }
    if let Condition::And(a, b) = condition {
        return evaluate_condition_simple(game, a, controller, source)
            && evaluate_condition_simple(game, b, controller, source);
    }
    if let Condition::Or(a, b) = condition {
        return evaluate_condition_simple(game, a, controller, source)
            || evaluate_condition_simple(game, b, controller, source);
    }
    if let Some(result) = evaluate_condition_shared_core(
        game,
        condition,
        SharedConditionContext {
            controller,
            source,
            filter_source: Some(source),
            triggering_event: None,
            trigger_identity: None,
        },
    ) {
        return result;
    }
    if let Condition::ValueComparison {
        left,
        operator,
        right,
    } = condition
    {
        return evaluate_value_comparison(game, controller, source, left, *operator, right);
    }

    match condition {
        Condition::ItIsNight => game.is_night,
        Condition::FirstCombatPhaseOfTurn => {
            game.turn.phase == crate::game_state::Phase::Combat
                && game.turn_store.combat_phases_started_this_turn == 1
        }
        Condition::ThisSpellWasKicked => game
            .object(source)
            .is_some_and(|obj| obj.optional_costs_paid.was_kicked()),
        Condition::ThisSpellEscaped => source_escaped(game, source),
        Condition::ThisSpellWasCastFromZone(_) => false,
        Condition::ThisSpellWasCastFromNonHand => false,
        Condition::ThisSpellPaidLabel(label) => game
            .object(source)
            .is_some_and(|obj| obj.optional_costs_paid.was_paid_label(label)),
        Condition::YouHaveFullParty => player_has_full_party(game, controller),
        Condition::YouControl(filter) => game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| game.controller_of(obj) == controller)
            .any(|obj| filter.matches(obj, &filter_ctx, game)),
        Condition::OpponentControls(filter) => game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| opponents.contains(&game.controller_of(obj)))
            .any(|obj| filter.matches(obj, &filter_ctx, game)),
        Condition::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype } => {
            let players = matching_condition_players_simple(game, controller, player);
            game.turn_store
                .turn_history
                .player_was_dealt_combat_damage_by_creature_subtype_this_turn(&players, *subtype)
        }
        Condition::PlayerControls { player, filter } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut ctx = crate::filter::FilterContext::new(player_id)
                .with_source(source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                ctx = ctx.with_iterated_player(Some(player_id));
            }
            condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    condition_object_matches_player_zone(game, obj, player_id, filter.zone)
                })
                .any(|obj| filter.matches(obj, &ctx, game))
        }
        Condition::PlayerOwnsCardNamedInZones {
            player,
            name,
            zones,
        } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut ctx = crate::filter::FilterContext::new(player_id)
                .with_source(source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                ctx = ctx.with_iterated_player(Some(player_id));
            }

            if zones.is_empty() {
                return false;
            }

            let mut filter = crate::target::ObjectFilter::default().named(name.clone());
            for zone in zones {
                filter.zone = Some(*zone);
                let has_matching = condition_candidate_ids_for_zone(game, Some(*zone))
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| obj.owner == player_id)
                    .any(|obj| filter.matches(obj, &ctx, game));
                if !has_matching {
                    return false;
                }
            }
            true
        }
        Condition::PlayerControlsAtLeast {
            player,
            filter,
            count,
        } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut ctx = crate::filter::FilterContext::new(player_id)
                .with_source(source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                ctx = ctx.with_iterated_player(Some(player_id));
            }
            let matches = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    condition_object_matches_player_zone(game, obj, player_id, filter.zone)
                })
                .filter(|obj| filter.matches(obj, &ctx, game))
                .count();
            matches >= *count as usize
        }
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count } => {
            use crate::types::Subtype;
            use std::collections::HashSet;

            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };

            let mut seen: HashSet<Subtype> = HashSet::new();
            for obj in game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| game.controller_of(obj) == player_id && obj.is_land())
            {
                for subtype in game.calculated_subtypes(obj.id) {
                    if matches!(
                        subtype,
                        Subtype::Plains
                            | Subtype::Island
                            | Subtype::Swamp
                            | Subtype::Mountain
                            | Subtype::Forest
                    ) {
                        seen.insert(subtype);
                    }
                }
            }
            seen.len() >= *count as usize
        }
        Condition::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            count_distinct_card_types_in_graveyard(game, player_id) >= *count as usize
        }
        Condition::PlayerControlsExactly {
            player,
            filter,
            count,
        } => matching_condition_players_simple(game, controller, player)
            .into_iter()
            .any(|player_id| {
                condition_count_for_player(game, source, player, player_id, filter)
                    == *count as usize
            }),
        Condition::PlayerControlsAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut ctx = crate::filter::FilterContext::new(player_id)
                .with_source(source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                ctx = ctx.with_iterated_player(Some(player_id));
            }
            count_distinct_matching_powers(game, player_id, filter, &ctx) >= *count as usize
        }
        Condition::PlayerControlsMost { player, filter } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };

            let count_for = |candidate: PlayerId| {
                let opponents: Vec<PlayerId> = game
                    .players
                    .iter()
                    .filter(|p| p.id != candidate)
                    .map(|p| p.id)
                    .collect();
                let mut ctx = crate::filter::FilterContext::new(candidate)
                    .with_source(source)
                    .with_opponents(opponents);
                if *player == PlayerFilter::IteratedPlayer {
                    ctx = ctx.with_iterated_player(Some(candidate));
                }
                condition_candidate_ids_for_zone(game, filter.zone)
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        condition_object_matches_player_zone(game, obj, candidate, filter.zone)
                    })
                    .filter(|obj| filter.matches(obj, &ctx, game))
                    .count()
            };

            let current = count_for(player_id);
            let max_count = game
                .players
                .iter()
                .map(|p| count_for(p.id))
                .max()
                .unwrap_or(0);
            current == max_count
        }
        Condition::PlayerControlsMoreThanYou { player, filter } => {
            let count_for = |candidate: PlayerId| {
                let opponents: Vec<PlayerId> = game
                    .players
                    .iter()
                    .filter(|p| p.id != candidate)
                    .map(|p| p.id)
                    .collect();
                let mut ctx = crate::filter::FilterContext::new(candidate)
                    .with_source(source)
                    .with_opponents(opponents);
                if *player == PlayerFilter::IteratedPlayer {
                    ctx = ctx.with_iterated_player(Some(candidate));
                }
                condition_candidate_ids_for_zone(game, filter.zone)
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        condition_object_matches_player_zone(game, obj, candidate, filter.zone)
                    })
                    .filter(|obj| filter.matches(obj, &ctx, game))
                    .count()
            };

            matching_condition_players_simple(game, controller, player)
                .into_iter()
                .any(|player_id| count_for(player_id) > count_for(controller))
        }
        Condition::AnOpponentControlsMoreThanPlayer { player, filter } => {
            matching_condition_players_simple(game, controller, player)
                .into_iter()
                .any(|player_id| {
                    any_opponent_controls_more_than_player(game, source, player, player_id, filter)
                })
        }
        Condition::PlayerLifeAtMostHalfStartingLifeTotal { player } => {
            matching_condition_players_simple(game, controller, player)
                .into_iter()
                .any(|player_id| player_life_compares_to_half_starting(game, player_id, true))
        }
        Condition::PlayerLifeLessThanHalfStartingLifeTotal { player } => {
            matching_condition_players_simple(game, controller, player)
                .into_iter()
                .any(|player_id| player_life_compares_to_half_starting(game, player_id, false))
        }
        Condition::PlayerHasLessLifeThanYou { player } => {
            let Some(you_life) = game.player(controller).map(|p| p.life) else {
                return false;
            };
            matching_condition_players_simple(game, controller, player)
                .into_iter()
                .filter_map(|player_id| game.player(player_id).map(|p| p.life))
                .any(|other_life| other_life < you_life)
        }
        Condition::PlayerHasNoOpponentWithMoreLifeThan { player } => {
            matching_condition_players_simple(game, controller, player)
                .into_iter()
                .any(|player_id| player_has_no_opponent_with_more_life_than(game, player_id))
        }
        Condition::PlayerHasMoreLifeThanYou { player } => {
            let Some(you_life) = game.player(controller).map(|p| p.life) else {
                return false;
            };
            matching_condition_players_simple(game, controller, player)
                .into_iter()
                .filter_map(|player_id| game.player(player_id).map(|p| p.life))
                .any(|other_life| other_life > you_life)
        }
        Condition::PlayerHasMoreLifeThanEachOtherPlayer { player } => {
            matching_condition_players_simple(game, controller, player)
                .into_iter()
                .any(|player_id| player_has_more_life_than_each_other_player(game, player_id))
        }
        Condition::PlayerIsMonarch { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            game.is_monarch(player_id)
        }
        Condition::PlayerHasInitiative { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            game.has_initiative(player_id)
        }
        Condition::PlayerHasCitysBlessing { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            game.has_citys_blessing(player_id)
        }
        Condition::PlayerCommittedCrimeThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            game.turn_store
                .turn_history
                .player_committed_crime_this_turn(player_id)
        }
        Condition::PlayerRolledResultThisTurn { player, result } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            game.turn_store
                .turn_history
                .player_rolled_result_this_turn(player_id, *result)
        }
        Condition::PlayerCompletedDungeon {
            player,
            dungeon_name,
        } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            match dungeon_name {
                Some(name) => game.has_completed_named_dungeon(player_id, name),
                None => game.has_completed_dungeon(player_id),
            }
        }
        Condition::PlayerCardsInHandOrMore { player, count } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let hand = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
            hand >= *count as usize
        }
        Condition::PlayerCardsInHandOrFewer { player, count } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let hand = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
            hand <= *count as usize
        }
        Condition::PlayerHasMoreCardsInHandThanYou { player } => {
            let your_hand = game.player(controller).map(|p| p.hand.len()).unwrap_or(0);
            matching_condition_players_simple(game, controller, player)
                .into_iter()
                .any(|player_id| {
                    game.player(player_id).map(|p| p.hand.len()).unwrap_or(0) > your_hand
                })
        }
        Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer { player } => {
            matching_condition_players_simple(game, controller, player)
                .into_iter()
                .any(|player_id| {
                    let hand = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
                    game.players
                        .iter()
                        .filter(|candidate| candidate.is_in_game())
                        .all(|candidate| candidate.id == player_id || hand > candidate.hand.len())
                })
        }
        Condition::PlayerHasPoisonCountersOrMore { player, count } => {
            matching_condition_players_simple(game, controller, player)
                .into_iter()
                .any(|player_id| player_poison_counters_or_more(game, player_id, *count))
        }
        Condition::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let filter_ctx = game.filter_context_for(controller, Some(source));
            let players: Vec<PlayerId> = match player {
                PlayerFilter::You => vec![controller],
                PlayerFilter::Opponent => filter_ctx.opponents,
                PlayerFilter::Specific(id) => vec![*id],
                PlayerFilter::Any => game.players.iter().map(|p| p.id).collect(),
                PlayerFilter::NotYou => game
                    .players
                    .iter()
                    .filter_map(|p| (p.id != controller).then_some(p.id))
                    .collect(),
                _ => Vec::new(),
            };
            let cast_count: u32 = players
                .iter()
                .map(|pid| game.turn_store.turn_history.spells_cast_by_player(*pid))
                .sum();
            cast_count >= *count
        }
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            game.turn_store
                .turn_history
                .players_tapped_land_for_mana_this_turn
                .contains(&player_id)
        }
        Condition::PlayerGainedLifeThisTurnOrMore { player, count } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            game.turn_store
                .turn_history
                .total_life_gained_for_players(&[player_id])
                >= *count
        }
        Condition::CreatureDiedThisTurnOrMore(count) => {
            game.turn_store
                .turn_history
                .total_creatures_died_this_turn()
                >= *count
        }
        Condition::CreatureCardPutIntoYourGraveyardThisTurn => {
            creature_card_was_put_into_your_graveyard_this_turn(game, controller)
        }
        Condition::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            player_had_land_enter_battlefield_this_turn(game, player_id)
        }
        Condition::FirstTimeThisTurn
        | Condition::MaxTimesEachTurn(_)
        | Condition::DoThisMaxTimesEachTurn(_) => true,
        Condition::TriggeringObjectWasEnchanted
        | Condition::TriggeringObjectHadToAttackThisCombat
        | Condition::TriggeringObjectHadCounters { .. } => false,
        Condition::ControlCreaturesTotalPowerAtLeast(_)
        | Condition::CardInYourGraveyard { .. }
        | Condition::ActivationTiming(_)
        | Condition::MaxActivationsPerTurn(_)
        | Condition::SourceIsEquipped
        | Condition::SourceIsEnchanted
        | Condition::EnchantedPermanentIsCreature
        | Condition::EnchantedPermanentIsLand
        | Condition::EnchantedPermanentIsEquipment
        | Condition::EnchantedPermanentIsVehicle
        | Condition::EquippedCreatureTapped
        | Condition::EquippedCreatureUntapped
        | Condition::EquippedCreatureAttacking
        | Condition::SourceChosenOption(_)
        | Condition::CountComparison { .. }
        | Condition::OwnsCardExiledWithCounter(_)
        | Condition::SourceAttackedThisTurn
        | Condition::SourceDealtCombatDamageToPlayerThisTurn
        | Condition::SourceCameUnderYourControlThisTurn
        | Condition::SourceAttackedOrBlockedThisTurn
        | Condition::SourceIsUntapped
        | Condition::SourceIsAttacking
        | Condition::SourceIsBlocking
        | Condition::SourceIsSoulbondPaired
        | Condition::SecretChoicesMatch
        | Condition::VoteOptionGetsMoreVotes(_)
        | Condition::VoteOptionGetsMoreVotesOrTied(_)
        | Condition::XValueAtLeast(_) => false,
        Condition::TaggedObjectMatches(_, _)
        | Condition::TaggedObjectIsTopOfLibrary { .. }
        | Condition::TaggedObjectWasCast(_) => false,
        Condition::StableObjectIsTopOfLibrary {
            stable_id,
            player,
            library_top_revision,
        } => crate::grant_registry::stable_card_is_top_of_library_at_revision(
            game,
            *stable_id,
            *player,
            *library_top_revision,
        ),
        Condition::TaggedObjectIsSoulbondPaired(_) => false,
        Condition::EnchantedPermanentAttackedThisTurn => false,
        Condition::TargetMatches(_) => false,
        Condition::TargetIsSoulbondPaired => false,
        Condition::PlayerTaggedObjectMatches { .. } => false,
        Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn { .. } => false,
        // Target-dependent conditions default to false during casting
        Condition::TargetIsTapped
        | Condition::TargetIsAttacking
        | Condition::TargetIsBlocked
        | Condition::TargetWasKicked
        | Condition::TargetSpellCastOrderThisTurn(_)
        | Condition::TargetSpellControllerIsPoisoned
        | Condition::TargetSpellManaSpentToCastAtLeast { .. }
        | Condition::YouControlMoreCreaturesThanTargetSpellController
        | Condition::TargetHasGreatestPowerAmongCreatures
        | Condition::TargetManaValueLteColorsSpentToCastThisSpell
        | Condition::SourceIsTapped
        | Condition::SourceIsSaddled
        | Condition::SourceDevouredCreaturesOrMore(_)
        | Condition::SourceIsMonstrous
        | Condition::SourceIsFaceDown
        | Condition::SourceMatches(_)
        | Condition::SourcePowerAtLeast(_) => false,
        Condition::Custom(_)
        | Condition::LifeTotalOrLess(_)
        | Condition::LifeTotalOrGreater(_)
        | Condition::CardsInHandOrMore(_)
        | Condition::YouHaveCardInHandMatching(_)
        | Condition::YourTurn
        | Condition::YourFirstTurnsOfTheGameOrFewer(_)
        | Condition::CreatureDiedThisTurn
        | Condition::CastSpellThisTurn
        | Condition::AttackedThisTurn
        | Condition::OpponentLostLifeThisTurn
        | Condition::PermanentLeftBattlefieldThisTurn
        | Condition::PermanentLeftBattlefieldUnderYourControlThisTurn
        | Condition::ObjectEnteredBattlefieldThisTurn(_)
        | Condition::ObjectEnteredBattlefieldLastTurn(_)
        | Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(_)
        | Condition::SourceWasCast
        | Condition::NoSpellsWereCastLastTurn
        | Condition::SpellsWereCastLastTurnOrMore(_)
        | Condition::SourceHasNoCounter(_)
        | Condition::SourceHasCounterAtLeast { .. }
        | Condition::SourceHasCountersAtLeast(_)
        | Condition::SourceIsInZone(_)
        | Condition::ManaSpentToCastThisSpellAtLeast { .. }
        | Condition::SameColorManaSpentToCastThisSpellAtLeast(_)
        | Condition::ColorsOfManaSpentToCastThisSpellOrMore(_)
        | Condition::PlayerGraveyardHasCardsAtLeast { .. }
        | Condition::SourceIsRingBearer { .. }
        | Condition::PlayerRingTemptedThisGameOrMore { .. }
        | Condition::ValueComparison { .. }
        | Condition::YouControlCommander
        | Condition::ThisAbilityResolvedThisTurnExactly(_)
        | Condition::Not(_)
        | Condition::And(_, _)
        | Condition::Or(_, _) => {
            unreachable!("handled before cast-time match")
        }
    }
}

fn resolve_condition_player_simple(
    game: &GameState,
    controller: PlayerId,
    player: &PlayerFilter,
) -> Option<PlayerId> {
    match player {
        PlayerFilter::You => Some(controller),
        PlayerFilter::Specific(id) => Some(*id),
        PlayerFilter::Active => Some(game.turn.active_player),
        PlayerFilter::NotYou => game.players.iter().find_map(|p| {
            if p.id != controller && p.is_in_game() {
                Some(p.id)
            } else {
                None
            }
        }),
        PlayerFilter::Opponent => game.players.iter().find_map(|p| {
            if p.id != controller && p.is_in_game() {
                Some(p.id)
            } else {
                None
            }
        }),
        PlayerFilter::MostLifeTied => {
            let max_life = game
                .players
                .iter()
                .filter(|player| player.is_in_game())
                .map(|player| player.life)
                .max()?;
            game.players.iter().find_map(|player| {
                (player.is_in_game() && player.life == max_life).then_some(player.id)
            })
        }
        PlayerFilter::LowestLifeTied => {
            let min_life = game
                .players
                .iter()
                .filter(|player| player.is_in_game())
                .map(|player| player.life)
                .min()?;
            game.players.iter().find_map(|player| {
                (player.is_in_game() && player.life == min_life).then_some(player.id)
            })
        }
        PlayerFilter::MostCardsInHand => {
            let max_hand = game
                .players
                .iter()
                .filter(|player| player.is_in_game())
                .map(|player| player.hand.len())
                .max()?;
            let leaders = game
                .players
                .iter()
                .filter(|player| player.is_in_game() && player.hand.len() == max_hand)
                .map(|player| player.id)
                .collect::<Vec<_>>();
            match leaders.as_slice() {
                [leader] => Some(*leader),
                _ => None,
            }
        }
        PlayerFilter::CardsInHandAtLeastMoreThanYou { .. }
        | PlayerFilter::HasMoreLifeThanYou { .. }
        | PlayerFilter::MaxSpeed { .. } => {
            let filter_ctx = crate::target::FilterContext::new(controller)
                .with_opponents(
                    game.players
                        .iter()
                        .filter(|p| p.id != controller && p.is_in_game())
                        .map(|p| p.id)
                        .collect(),
                )
                .with_active_player(game.turn.active_player);
            game.players.iter().find_map(|candidate| {
                (candidate.is_in_game()
                    && player_filter_matches_game(player, candidate.id, game, &filter_ctx))
                .then_some(candidate.id)
            })
        }
        PlayerFilter::Any
        | PlayerFilter::CastCardTypeThisTurn(_)
        | PlayerFilter::Target(_)
        | PlayerFilter::Teammate
        | PlayerFilter::Attacking
        | PlayerFilter::Defending
        | PlayerFilter::DamagedPlayer
        | PlayerFilter::EffectController
        | PlayerFilter::ChosenPlayer
        | PlayerFilter::TaggedPlayer(_)
        | PlayerFilter::IteratedPlayer
        | PlayerFilter::TargetPlayerOrControllerOfTarget
        | PlayerFilter::Excluding { .. }
        | PlayerFilter::ControllerOf(_)
        | PlayerFilter::OwnerOf(_)
        | PlayerFilter::AliasedOwnerOf(_)
        | PlayerFilter::AliasedControllerOf(_) => None,
    }
}

fn resolve_condition_player_external(
    game: &GameState,
    ctx: &ExternalEvaluationContext<'_>,
    player: &PlayerFilter,
) -> Option<PlayerId> {
    match player {
        PlayerFilter::Defending => ctx.defending_player,
        PlayerFilter::Attacking => Some(ctx.attacking_player.unwrap_or(ctx.controller)),
        _ => resolve_condition_player_simple(game, ctx.controller, player),
    }
}

fn matching_condition_players_simple(
    game: &GameState,
    controller: PlayerId,
    player: &PlayerFilter,
) -> Vec<PlayerId> {
    match player {
        PlayerFilter::Opponent | PlayerFilter::NotYou => game
            .players
            .iter()
            .filter(|p| p.id != controller && p.is_in_game())
            .map(|p| p.id)
            .collect(),
        PlayerFilter::Any => game
            .players
            .iter()
            .filter(|p| p.is_in_game())
            .map(|p| p.id)
            .collect(),
        _ => resolve_condition_player_simple(game, controller, player)
            .into_iter()
            .collect(),
    }
}

fn matching_condition_players_external(
    game: &GameState,
    ctx: &ExternalEvaluationContext<'_>,
    player: &PlayerFilter,
) -> Vec<PlayerId> {
    match player {
        PlayerFilter::Defending => ctx.defending_player.into_iter().collect(),
        PlayerFilter::Attacking => Some(ctx.attacking_player.unwrap_or(ctx.controller))
            .into_iter()
            .collect(),
        _ => matching_condition_players_simple(game, ctx.controller, player),
    }
}

fn matching_condition_players_exec(
    game: &GameState,
    ctx: &ExecutionContext,
    player: &PlayerFilter,
) -> Result<Vec<PlayerId>, ExecutionError> {
    match player {
        PlayerFilter::Opponent | PlayerFilter::NotYou => Ok(game
            .players
            .iter()
            .filter(|p| p.id != ctx.controller && p.is_in_game())
            .map(|p| p.id)
            .collect()),
        PlayerFilter::Any => Ok(game
            .players
            .iter()
            .filter(|p| p.is_in_game())
            .map(|p| p.id)
            .collect()),
        _ => Ok(vec![crate::effects::helpers::resolve_player_filter(
            game, player, ctx,
        )?]),
    }
}

/// Evaluate a condition.
fn evaluate_condition(
    game: &GameState,
    condition: &Condition,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    assert_condition_variant_coverage(condition);

    if let Condition::Not(inner) = condition {
        let inner_result = evaluate_condition(game, inner, ctx)?;
        return Ok(!inner_result);
    }
    if let Condition::And(a, b) = condition {
        let a_result = evaluate_condition(game, a, ctx)?;
        if !a_result {
            return Ok(false);
        }
        return evaluate_condition(game, b, ctx);
    }
    if let Condition::Or(a, b) = condition {
        let a_result = evaluate_condition(game, a, ctx)?;
        if a_result {
            return Ok(true);
        }
        return evaluate_condition(game, b, ctx);
    }
    if let Some(result) = evaluate_condition_shared_core(
        game,
        condition,
        SharedConditionContext {
            controller: ctx.controller,
            source: ctx.source,
            filter_source: Some(ctx.source),
            triggering_event: ctx.triggering_event.as_ref(),
            trigger_identity: ctx.trigger_identity,
        },
    ) {
        return Ok(result);
    }

    match condition {
        Condition::ItIsNight => Ok(game.is_night),
        Condition::FirstCombatPhaseOfTurn => Ok(game.turn.phase
            == crate::game_state::Phase::Combat
            && game.turn_store.combat_phases_started_this_turn == 1),
        Condition::YouControl(filter) => {
            let filter_ctx = ctx.filter_context(game);

            let has_matching = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| game.controller_of(obj) == ctx.controller)
                .any(|obj| filter.matches(obj, &filter_ctx, game));

            Ok(has_matching)
        }
        Condition::OpponentControls(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let opponents = &filter_ctx.opponents;

            let has_matching = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| opponents.contains(&game.controller_of(obj)))
                .any(|obj| filter.matches(obj, &filter_ctx, game));

            Ok(has_matching)
        }
        Condition::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype } => {
            let players = matching_condition_players_exec(game, ctx, player)?;
            Ok(game
                .turn_store
                .turn_history
                .player_was_dealt_combat_damage_by_creature_subtype_this_turn(&players, *subtype))
        }
        Condition::PlayerControls { player, filter } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);
            let has_matching = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    condition_object_matches_player_zone(game, obj, player_id, filter.zone)
                })
                .any(|obj| filter.matches(obj, &filter_ctx, game));
            Ok(has_matching)
        }
        Condition::PlayerOwnsCardNamedInZones {
            player,
            name,
            zones,
        } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);

            if zones.is_empty() {
                return Ok(false);
            }

            let mut filter = crate::target::ObjectFilter::default().named(name.clone());
            for zone in zones {
                filter.zone = Some(*zone);
                let has_matching = condition_candidate_ids_for_zone(game, Some(*zone))
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| obj.owner == player_id)
                    .any(|obj| filter.matches(obj, &filter_ctx, game));
                if !has_matching {
                    return Ok(false);
                }
            }

            Ok(true)
        }
        Condition::PlayerControlsAtLeast {
            player,
            filter,
            count,
        } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);
            let matches = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    condition_object_matches_player_zone(game, obj, player_id, filter.zone)
                })
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .count();
            Ok(matches >= *count as usize)
        }
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count } => {
            use crate::types::Subtype;
            use std::collections::HashSet;

            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut seen: HashSet<Subtype> = HashSet::new();
            for obj in game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| game.controller_of(obj) == player_id && obj.is_land())
            {
                for subtype in game.calculated_subtypes(obj.id) {
                    if matches!(
                        subtype,
                        Subtype::Plains
                            | Subtype::Island
                            | Subtype::Swamp
                            | Subtype::Mountain
                            | Subtype::Forest
                    ) {
                        seen.insert(subtype);
                    }
                }
            }
            Ok(seen.len() >= *count as usize)
        }
        Condition::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(count_distinct_card_types_in_graveyard(game, player_id) >= *count as usize)
        }
        Condition::PlayerControlsExactly {
            player,
            filter,
            count,
        } => Ok(matching_condition_players_exec(game, ctx, player)?
            .into_iter()
            .any(|player_id| {
                condition_count_for_player(game, ctx.source, player, player_id, filter)
                    == *count as usize
            })),
        Condition::PlayerControlsAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);
            let distinct = count_distinct_matching_powers(game, player_id, filter, &filter_ctx);
            Ok(distinct >= *count as usize)
        }
        Condition::PlayerControlsMost { player, filter } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let count_for = |candidate: PlayerId| {
                let mut filter_ctx = ctx.filter_context(game);
                filter_ctx.iterated_player = Some(candidate);
                condition_candidate_ids_for_zone(game, filter.zone)
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        condition_object_matches_player_zone(game, obj, candidate, filter.zone)
                    })
                    .filter(|obj| filter.matches(obj, &filter_ctx, game))
                    .count()
            };
            let current = count_for(player_id);
            let max_count = game
                .players
                .iter()
                .map(|player| count_for(player.id))
                .max()
                .unwrap_or(0);
            Ok(current == max_count)
        }
        Condition::PlayerControlsMoreThanYou { player, filter } => {
            let count_for = |candidate: PlayerId| {
                let mut filter_ctx = ctx.filter_context(game);
                filter_ctx.iterated_player = Some(candidate);
                condition_candidate_ids_for_zone(game, filter.zone)
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        condition_object_matches_player_zone(game, obj, candidate, filter.zone)
                    })
                    .filter(|obj| filter.matches(obj, &filter_ctx, game))
                    .count()
            };
            Ok(matching_condition_players_exec(game, ctx, player)?
                .into_iter()
                .any(|player_id| count_for(player_id) > count_for(ctx.controller)))
        }
        Condition::AnOpponentControlsMoreThanPlayer { player, filter } => {
            Ok(matching_condition_players_exec(game, ctx, player)?
                .into_iter()
                .any(|player_id| {
                    any_opponent_controls_more_than_player(
                        game, ctx.source, player, player_id, filter,
                    )
                }))
        }
        Condition::PlayerLifeAtMostHalfStartingLifeTotal { player } => {
            Ok(matching_condition_players_exec(game, ctx, player)?
                .into_iter()
                .any(|player_id| player_life_compares_to_half_starting(game, player_id, true)))
        }
        Condition::PlayerLifeLessThanHalfStartingLifeTotal { player } => {
            Ok(matching_condition_players_exec(game, ctx, player)?
                .into_iter()
                .any(|player_id| player_life_compares_to_half_starting(game, player_id, false)))
        }
        Condition::PlayerHasLessLifeThanYou { player } => {
            let you_life = game.player(ctx.controller).map(|p| p.life).unwrap_or(0);
            Ok(matching_condition_players_exec(game, ctx, player)?
                .into_iter()
                .any(|player_id| game.player(player_id).map(|p| p.life).unwrap_or(0) < you_life))
        }
        Condition::PlayerHasMoreLifeThanYou { player } => {
            let you_life = game.player(ctx.controller).map(|p| p.life).unwrap_or(0);
            Ok(matching_condition_players_exec(game, ctx, player)?
                .into_iter()
                .any(|player_id| game.player(player_id).map(|p| p.life).unwrap_or(0) > you_life))
        }
        Condition::PlayerHasNoOpponentWithMoreLifeThan { player } => {
            Ok(matching_condition_players_exec(game, ctx, player)?
                .into_iter()
                .any(|player_id| player_has_no_opponent_with_more_life_than(game, player_id)))
        }
        Condition::PlayerHasMoreLifeThanEachOtherPlayer { player } => {
            Ok(matching_condition_players_exec(game, ctx, player)?
                .into_iter()
                .any(|player_id| player_has_more_life_than_each_other_player(game, player_id)))
        }
        Condition::PlayerIsMonarch { player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(game.is_monarch(player_id))
        }
        Condition::PlayerHasInitiative { player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(game.has_initiative(player_id))
        }
        Condition::PlayerHasCitysBlessing { player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(game.has_citys_blessing(player_id))
        }
        Condition::PlayerCommittedCrimeThisTurn { player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(game
                .turn_store
                .turn_history
                .player_committed_crime_this_turn(player_id))
        }
        Condition::PlayerRolledResultThisTurn { player, result } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(game
                .turn_store
                .turn_history
                .player_rolled_result_this_turn(player_id, *result))
        }
        Condition::PlayerCompletedDungeon {
            player,
            dungeon_name,
        } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(match dungeon_name {
                Some(name) => game.has_completed_named_dungeon(player_id, name),
                None => game.has_completed_dungeon(player_id),
            })
        }
        Condition::PlayerCardsInHandOrMore { player, count } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let hand_count = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
            Ok(hand_count >= *count as usize)
        }
        Condition::PlayerCardsInHandOrFewer { player, count } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let hand_count = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
            Ok(hand_count <= *count as usize)
        }
        Condition::PlayerHasMoreCardsInHandThanYou { player } => {
            let your_hand = game
                .player(ctx.controller)
                .map(|p| p.hand.len())
                .unwrap_or(0);
            Ok(matching_condition_players_exec(game, ctx, player)?
                .into_iter()
                .any(|player_id| {
                    game.player(player_id).map(|p| p.hand.len()).unwrap_or(0) > your_hand
                }))
        }
        Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer { player } => {
            Ok(matching_condition_players_exec(game, ctx, player)?
                .into_iter()
                .any(|player_id| {
                    let hand = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
                    game.players
                        .iter()
                        .filter(|candidate| candidate.is_in_game())
                        .all(|candidate| candidate.id == player_id || hand > candidate.hand.len())
                }))
        }
        Condition::PlayerHasPoisonCountersOrMore { player, count } => {
            Ok(matching_condition_players_exec(game, ctx, player)?
                .into_iter()
                .any(|player_id| player_poison_counters_or_more(game, player_id, *count)))
        }
        Condition::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let filter_ctx = ctx.filter_context(game);
            let player_ids: Vec<PlayerId> = match player {
                PlayerFilter::You => vec![ctx.controller],
                PlayerFilter::Opponent => filter_ctx.opponents,
                PlayerFilter::Specific(id) => vec![*id],
                PlayerFilter::Any => game.players.iter().map(|p| p.id).collect(),
                PlayerFilter::NotYou => game
                    .players
                    .iter()
                    .filter_map(|p| (p.id != ctx.controller).then_some(p.id))
                    .collect(),
                _ => Vec::new(),
            };
            let cast_count: u32 = player_ids
                .iter()
                .map(|pid| game.turn_store.turn_history.spells_cast_by_player(*pid))
                .sum();
            Ok(cast_count >= *count)
        }
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(game
                .turn_store
                .turn_history
                .players_tapped_land_for_mana_this_turn
                .contains(&player_id))
        }
        Condition::PlayerGainedLifeThisTurnOrMore { player, count } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(game
                .turn_store
                .turn_history
                .total_life_gained_for_players(&[player_id])
                >= *count)
        }
        Condition::CreatureDiedThisTurnOrMore(count) => Ok(game
            .turn_store
            .turn_history
            .total_creatures_died_this_turn()
            >= *count),
        Condition::CreatureCardPutIntoYourGraveyardThisTurn => Ok(
            creature_card_was_put_into_your_graveyard_this_turn(game, ctx.controller),
        ),
        Condition::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(player_had_land_enter_battlefield_this_turn(game, player_id))
        }
        Condition::TargetIsTapped => {
            // Check if the target is tapped
            if let Some(crate::effects::ResolvedTarget::Object(id)) = ctx.targets.first() {
                return Ok(game.is_tapped(*id));
            }
            Ok(false)
        }
        Condition::TargetWasKicked => {
            for target in &ctx.targets {
                if let crate::effects::ResolvedTarget::Object(id) = target
                    && let Some(obj) = game.object(*id)
                {
                    return Ok(obj.optional_costs_paid.was_kicked());
                }
            }
            Ok(false)
        }
        Condition::ThisSpellWasKicked => Ok(resolve_value(game, &Value::WasKicked, ctx)? != 0),
        Condition::ThisSpellEscaped => Ok(this_spell_escaped(game, ctx.source, ctx)),
        Condition::ThisSpellWasCastFromZone(zone) => {
            Ok(this_spell_was_cast_from_zone(game, ctx.source, ctx, *zone))
        }
        Condition::ThisSpellWasCastFromNonHand => {
            Ok(this_spell_was_cast_from_non_hand(game, ctx.source, ctx))
        }
        Condition::ThisSpellPaidLabel(label) => {
            Ok(resolve_value(game, &Value::WasPaidLabel(label.clone()), ctx)? != 0)
        }
        Condition::YouHaveFullParty => Ok(player_has_full_party(game, ctx.controller)),
        Condition::TargetSpellCastOrderThisTurn(order) => {
            for target in &ctx.targets {
                if let crate::effects::ResolvedTarget::Object(id) = target {
                    let actual = game
                        .turn_store
                        .turn_history
                        .spell_cast_order(*id)
                        .unwrap_or(0);
                    return Ok(actual == *order);
                }
            }
            Ok(false)
        }
        Condition::TargetSpellControllerIsPoisoned => {
            for target in &ctx.targets {
                if let crate::effects::ResolvedTarget::Object(id) = target
                    && let Some(obj) = game.object(*id)
                    && let Some(player) = game.player(game.controller_of(obj))
                {
                    return Ok(player.poison_counters > 0);
                }
            }
            Ok(false)
        }
        Condition::TargetSpellManaSpentToCastAtLeast { amount, symbol } => {
            for target in &ctx.targets {
                if let crate::effects::ResolvedTarget::Object(id) = target
                    && let Some(obj) = game.object(*id)
                {
                    let spent = if let Some(sym) = symbol {
                        obj.mana_spent_to_cast.amount(*sym)
                    } else {
                        obj.mana_spent_to_cast.total()
                    };
                    return Ok(spent >= *amount);
                }
            }
            Ok(false)
        }
        Condition::YouControlMoreCreaturesThanTargetSpellController => {
            let target_controller = ctx.targets.iter().find_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => {
                    game.object(*id).map(|obj| game.controller_of(obj))
                }
                _ => None,
            });
            let Some(target_controller) = target_controller else {
                return Ok(false);
            };

            let you_count = game
                .battlefield
                .iter()
                .filter(|&&id| {
                    game.object(id).is_some_and(|obj| {
                        game.controller_of(obj) == ctx.controller
                            && game.object_has_card_type(id, crate::types::CardType::Creature)
                    })
                })
                .count();
            let target_count = game
                .battlefield
                .iter()
                .filter(|&&id| {
                    game.object(id).is_some_and(|obj| {
                        game.controller_of(obj) == target_controller
                            && game.object_has_card_type(id, crate::types::CardType::Creature)
                    })
                })
                .count();
            Ok(you_count > target_count)
        }
        Condition::TargetHasGreatestPowerAmongCreatures => {
            let target_id = ctx.targets.iter().find_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => Some(*id),
                _ => None,
            });
            let Some(target_id) = target_id else {
                return Ok(false);
            };
            let Some(target_obj) = game.object(target_id) else {
                return Ok(false);
            };
            if !game.object_has_card_type(target_id, crate::types::CardType::Creature) {
                return Ok(false);
            }
            let Some(target_power) = game
                .calculated_power(target_id)
                .or_else(|| target_obj.power())
            else {
                return Ok(false);
            };
            let max_power = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| game.object_has_card_type(obj.id, crate::types::CardType::Creature))
                .filter_map(|obj| game.calculated_power(obj.id).or_else(|| obj.power()))
                .max();
            Ok(max_power.is_some_and(|max| target_power >= max))
        }
        Condition::TargetManaValueLteColorsSpentToCastThisSpell => {
            let target_id = ctx.targets.iter().find_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => Some(*id),
                _ => None,
            });
            let Some(target_id) = target_id else {
                return Ok(false);
            };
            let Some(target_obj) = game.object(target_id) else {
                return Ok(false);
            };
            let Some(source_obj) = game.object(ctx.source) else {
                return Ok(false);
            };
            let target_mana_value = target_obj
                .mana_cost
                .as_ref()
                .map(|cost| cost.mana_value())
                .unwrap_or(0);
            let colors_spent = [
                source_obj.mana_spent_to_cast.white,
                source_obj.mana_spent_to_cast.blue,
                source_obj.mana_spent_to_cast.black,
                source_obj.mana_spent_to_cast.red,
                source_obj.mana_spent_to_cast.green,
            ]
            .into_iter()
            .filter(|amount| *amount > 0)
            .count() as u32;
            Ok(target_mana_value <= colors_spent)
        }
        Condition::SourceIsTapped => Ok(game.is_tapped(ctx.source)),
        Condition::SourceIsSaddled => Ok(game.is_saddled(ctx.source)),
        Condition::SourceDevouredCreaturesOrMore(count) => {
            Ok(game.devoured_count(ctx.source) >= *count)
        }
        Condition::SourceIsMonstrous => Ok(game.is_monstrous(ctx.source)),
        Condition::SourceIsFaceDown => Ok(source_is_face_down_or_alternate_face(game, ctx.source)),
        Condition::SourceMatches(filter) => {
            let filter_ctx = ctx.filter_context(game);
            Ok(game
                .object(ctx.source)
                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game)))
        }
        Condition::SourcePowerAtLeast(min_power) => Ok(game
            .calculated_power(ctx.source)
            .or_else(|| game.object(ctx.source).and_then(|obj| obj.power()))
            .is_some_and(|power| power >= *min_power as i32)),
        Condition::SourceHasCountersAtLeast(count) => Ok(game
            .object(ctx.source)
            .is_some_and(|obj| obj.counters.values().copied().sum::<u32>() >= *count)),
        Condition::SourceAttackedOrBlockedThisTurn => Ok(game
            .creature_attacked_this_turn(ctx.source)
            || game.creature_blocked_this_turn(ctx.source)),
        Condition::TargetIsAttacking => {
            // Check if the target is among declared attackers
            // Note: Combat attackers are tracked in game_loop, not game_state directly.
            // For now, check ctx.attacking_creatures if it exists
            if let Some(crate::effects::ResolvedTarget::Object(id)) = ctx.targets.first() {
                // Simplified: check if it's a creature that's tapped (attackers are usually tapped)
                // Full implementation would need access to combat state from game loop
                if let Some(obj) = game.object(*id) {
                    return Ok(
                        game.object_has_card_type(obj.id, crate::types::CardType::Creature)
                            && game.is_tapped(*id),
                    );
                }
            }
            Ok(false)
        }
        Condition::TargetIsBlocked => {
            if let Some(crate::effects::ResolvedTarget::Object(id)) = ctx.targets.first()
                && let Some(combat) = &game.combat
            {
                return Ok(crate::combat_state::is_blocked(combat, *id));
            }
            Ok(false)
        }
        Condition::TaggedObjectMatches(tag, filter) => {
            let filter_ctx = ctx.filter_context(game);
            if let Some(tagged) = ctx.get_tagged_all(tag.as_str()) {
                return Ok(tagged.iter().any(|snapshot| {
                    let snapshot_matches = filter.matches_snapshot(snapshot, &filter_ctx, game);
                    let current_id = game
                        .object(snapshot.object_id)
                        .map(|object| object.id)
                        .or_else(|| game.find_object_by_stable_id(snapshot.stable_id));
                    if let Some(current_id) = current_id
                        && let Some(object) = game.object(current_id)
                    {
                        return filter.matches(object, &filter_ctx, game) || snapshot_matches;
                    }
                    snapshot_matches
                }));
            }

            // Some compile-time conditional lowering paths synthesize a branch-local tag
            // (for example "countered_0") before runtime tagging exists. In these cases,
            // fall back to evaluating against the first object target.
            let synthetic_tag = tag.as_str().rsplit_once('_').is_some_and(|(head, suffix)| {
                !head.is_empty() && suffix.chars().all(|c| c.is_ascii_digit())
            });
            if !synthetic_tag {
                return Ok(false);
            }

            let Some(crate::effects::ResolvedTarget::Object(id)) = ctx.targets.first() else {
                return Ok(false);
            };
            if let Some(obj) = game.object(*id) {
                return Ok(filter.matches(obj, &filter_ctx, game));
            }
            if let Some(snapshot) = ctx.target_snapshots.get(id) {
                return Ok(filter.matches_snapshot(snapshot, &filter_ctx, game));
            }
            Ok(false)
        }
        Condition::TaggedObjectIsTopOfLibrary { tag, player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let Some(tagged) = ctx.get_tagged_all(tag.as_str()) else {
                return Ok(false);
            };
            Ok(tagged.iter().any(|snapshot| {
                crate::grant_registry::stable_card_is_top_of_library(
                    game,
                    snapshot.stable_id,
                    player_id,
                )
            }))
        }
        Condition::StableObjectIsTopOfLibrary {
            stable_id,
            player,
            library_top_revision,
        } => Ok(
            crate::grant_registry::stable_card_is_top_of_library_at_revision(
                game,
                *stable_id,
                *player,
                *library_top_revision,
            ),
        ),
        Condition::TaggedObjectWasCast(tag) => Ok(tagged_object_was_cast(game, tag, ctx)),
        Condition::TaggedObjectIsSoulbondPaired(tag) => {
            let tagged_id = ctx
                .get_tagged(tag.as_str())
                .map(|snapshot| snapshot.object_id);
            Ok(tagged_id.is_some_and(|id| game.is_soulbond_paired(id)))
        }
        Condition::EnchantedPermanentAttackedThisTurn => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached_to| game.creature_attacked_this_turn(attached_to))),
        Condition::TargetMatches(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let Some(crate::effects::ResolvedTarget::Object(id)) = ctx.targets.first() else {
                return Ok(false);
            };
            if let Some(obj) = game.object(*id) {
                return Ok(filter.matches(obj, &filter_ctx, game));
            }
            if let Some(snapshot) = ctx.target_snapshots.get(id) {
                return Ok(filter.matches_snapshot(snapshot, &filter_ctx, game));
            }
            Ok(false)
        }
        Condition::TargetIsSoulbondPaired => {
            let target_id = ctx.targets.iter().find_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => Some(*id),
                _ => None,
            });
            Ok(target_id.is_some_and(|id| game.is_soulbond_paired(id)))
        }
        Condition::PlayerTaggedObjectMatches {
            player,
            tag,
            filter,
        } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let Some(tagged) = ctx.get_tagged_all(tag.as_str()) else {
                return Ok(false);
            };
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);
            for snapshot in tagged {
                let current_id = game
                    .object(snapshot.object_id)
                    .map(|object| object.id)
                    .or_else(|| game.find_object_by_stable_id(snapshot.stable_id));
                if let Some(current_id) = current_id
                    && let Some(object) = game.object(current_id)
                {
                    if game.controller_of(object) == player_id
                        && filter.matches(object, &filter_ctx, game)
                    {
                        return Ok(true);
                    }
                    continue;
                }
                if snapshot.controller != player_id {
                    continue;
                }
                if filter.matches_snapshot(snapshot, &filter_ctx, game) {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn { player, tag } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let Some(tagged) = ctx.get_tagged_all(tag.as_str()) else {
                return Ok(false);
            };
            Ok(tagged.iter().any(|snapshot| {
                game.turn_store
                    .turn_history
                    .object_entered_battlefield_controller_this_turn(snapshot.stable_id)
                    .is_some_and(|entry_controller| entry_controller == player_id)
            }))
        }
        Condition::FirstTimeThisTurn
        | Condition::MaxTimesEachTurn(_)
        | Condition::DoThisMaxTimesEachTurn(_) => Ok(true),
        Condition::TriggeringObjectWasEnchanted => Ok(ctx
            .triggering_event
            .as_ref()
            .and_then(|event| event.snapshot())
            .is_some_and(|snapshot| snapshot.was_enchanted)),
        Condition::TriggeringObjectHadToAttackThisCombat => Ok(
            triggering_object_had_to_attack_this_combat(game, ctx.triggering_event.as_ref()),
        ),
        Condition::TriggeringObjectHadCounters {
            counter_type,
            min_count,
        } => Ok(ctx
            .triggering_event
            .as_ref()
            .and_then(|event| event.snapshot())
            .is_some_and(|snapshot| {
                snapshot.counters.get(counter_type).copied().unwrap_or(0) >= *min_count
            })),
        Condition::ControlCreaturesTotalPowerAtLeast(required_power) => {
            let total_power = game
                .battlefield
                .iter()
                .copied()
                .filter(|&id| {
                    game.object(id).is_some_and(|obj| {
                        game.controller_of(obj) == ctx.controller && game.current_is_creature(id)
                    })
                })
                .map(|id| game.current_power(id).unwrap_or(0).max(0))
                .sum::<i32>();
            Ok(total_power >= *required_power as i32)
        }
        Condition::CardInYourGraveyard {
            card_types,
            subtypes,
        } => Ok(game.player(ctx.controller).is_some_and(|player_state| {
            player_state.graveyard.iter().any(|&card_id| {
                if game.object(card_id).is_none() {
                    return false;
                }
                let card_type_match = card_types.is_empty()
                    || card_types
                        .iter()
                        .any(|card_type| game.current_has_card_type(card_id, *card_type));
                let subtype_match = subtypes.is_empty()
                    || subtypes
                        .iter()
                        .any(|subtype| game.current_has_subtype(card_id, *subtype));
                card_type_match && subtype_match
            })
        })),
        Condition::ActivationTiming(_) | Condition::MaxActivationsPerTurn(_) => Ok(false),
        Condition::SourceIsEquipped => Ok(game.object(ctx.source).is_some_and(|source_obj| {
            source_obj.attachments.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.subtypes.contains(&crate::types::Subtype::Equipment))
            })
        })),
        Condition::SourceIsEnchanted => Ok(game.object(ctx.source).is_some_and(|source_obj| {
            source_obj.attachments.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.subtypes.contains(&crate::types::Subtype::Aura))
            })
        })),
        Condition::EnchantedPermanentIsCreature => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| {
                game.object_has_card_type(attached, crate::types::CardType::Creature)
            })),
        Condition::EnchantedPermanentIsLand => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| {
                game.object_has_card_type(attached, crate::types::CardType::Land)
            })),
        Condition::EnchantedPermanentIsEquipment => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| {
                game.calculated_subtypes(attached)
                    .contains(&crate::types::Subtype::Equipment)
            })),
        Condition::EnchantedPermanentIsVehicle => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| {
                game.calculated_subtypes(attached)
                    .contains(&crate::types::Subtype::Vehicle)
            })),
        Condition::EquippedCreatureTapped => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| game.is_tapped(attached))),
        Condition::EquippedCreatureUntapped => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| !game.is_tapped(attached))),
        Condition::EquippedCreatureAttacking => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to.and_then(|target| target.object_id()))
            .is_some_and(|attached| {
                game.combat
                    .as_ref()
                    .is_some_and(|combat| crate::combat_state::is_attacking(combat, attached))
            })),
        Condition::SourceChosenOption(expected) => Ok(game
            .chosen_named_option(ctx.source)
            .is_some_and(|chosen| chosen.eq_ignore_ascii_case(expected))),
        Condition::SecretChoicesMatch => Ok(ctx
            .secret_choice_results
            .get(&ctx.source)
            .is_some_and(|result| result.choices_match())),
        Condition::VoteOptionGetsMoreVotes(option) => Ok(ctx
            .vote_results
            .get(&ctx.source)
            .is_some_and(|result| result.option_gets_more_votes(option))),
        Condition::VoteOptionGetsMoreVotesOrTied(option) => Ok(ctx
            .vote_results
            .get(&ctx.source)
            .is_some_and(|result| result.option_gets_more_votes_or_tied(option))),
        Condition::CountComparison {
            count, comparison, ..
        } => Ok(
            comparison.evaluate(crate::static_abilities::resolve_anthem_count_expression(
                count,
                game,
                ctx.source,
                ctx.controller,
            )),
        ),
        Condition::ValueComparison {
            left,
            operator,
            right,
        } => Ok(operator.evaluate(
            resolve_value(game, left, ctx)?,
            resolve_value(game, right, ctx)?,
        )),
        Condition::OwnsCardExiledWithCounter(counter) => Ok(game.exile.iter().any(|&id| {
            game.object(id).is_some_and(|obj| {
                obj.owner == ctx.controller && obj.counters.get(counter).copied().unwrap_or(0) > 0
            })
        })),
        Condition::SourceAttackedThisTurn => Ok(game.creature_attacked_this_turn(ctx.source)),
        Condition::SourceDealtCombatDamageToPlayerThisTurn => {
            Ok(game.source_dealt_combat_damage_to_player_this_turn(ctx.source))
        }
        Condition::SourceCameUnderYourControlThisTurn => {
            Ok(game.object(ctx.source).is_some_and(|obj| {
                game.turn_store
                    .turn_history
                    .object_came_under_controller_this_turn(obj.stable_id, ctx.controller)
            }))
        }
        Condition::SourceIsUntapped => Ok(!game.is_tapped(ctx.source)),
        Condition::SourceIsAttacking => Ok(game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_attacking(combat, ctx.source))),
        Condition::SourceIsBlocking => Ok(game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_blocking(combat, ctx.source))),
        Condition::SourceIsSoulbondPaired => Ok(game.is_soulbond_paired(ctx.source)),
        Condition::XValueAtLeast(min) => Ok(ctx.x_value.unwrap_or(0) >= *min),
        Condition::Custom(_)
        | Condition::LifeTotalOrLess(_)
        | Condition::LifeTotalOrGreater(_)
        | Condition::CardsInHandOrMore(_)
        | Condition::YouHaveCardInHandMatching(_)
        | Condition::YourTurn
        | Condition::YourFirstTurnsOfTheGameOrFewer(_)
        | Condition::CreatureDiedThisTurn
        | Condition::CastSpellThisTurn
        | Condition::AttackedThisTurn
        | Condition::OpponentLostLifeThisTurn
        | Condition::PermanentLeftBattlefieldThisTurn
        | Condition::PermanentLeftBattlefieldUnderYourControlThisTurn
        | Condition::ObjectEnteredBattlefieldThisTurn(_)
        | Condition::ObjectEnteredBattlefieldLastTurn(_)
        | Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(_)
        | Condition::SourceWasCast
        | Condition::NoSpellsWereCastLastTurn
        | Condition::SpellsWereCastLastTurnOrMore(_)
        | Condition::SourceHasNoCounter(_)
        | Condition::SourceHasCounterAtLeast { .. }
        | Condition::SourceIsInZone(_)
        | Condition::ManaSpentToCastThisSpellAtLeast { .. }
        | Condition::SameColorManaSpentToCastThisSpellAtLeast(_)
        | Condition::ColorsOfManaSpentToCastThisSpellOrMore(_)
        | Condition::PlayerGraveyardHasCardsAtLeast { .. }
        | Condition::SourceIsRingBearer { .. }
        | Condition::PlayerRingTemptedThisGameOrMore { .. }
        | Condition::YouControlCommander
        | Condition::ThisAbilityResolvedThisTurnExactly(_)
        | Condition::Not(_)
        | Condition::And(_, _)
        | Condition::Or(_, _) => {
            unreachable!("handled before resolution match")
        }
    }
}

use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, PlayerId, Zone};
fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Lavinia, Azorius Renegade",
    )
    .unwrap()
    .remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}
#[test]
fn strict_snapshot_and_full_quality_gate() {
    let s = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        s.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{:?}",
        s.parse_error
    );
    assert!(!s.parse_lossy && !s.has_unimplemented && s.parse_error.is_none());
    assert!(
        s.similarity_score >= 0.99,
        "{}: {:?}",
        s.similarity_score,
        s.compiled_text
    );
}
#[test]
fn casting_limit_uses_each_opponents_own_lands_and_exempts_creatures() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    for caster in [alice, bob, carol] {
        for card_type in [CardType::Instant, CardType::Creature] {
            for mana_value in [0, 1, 2, 3, 4] {
                let mut game =
                    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                game.create_object_from_definition(&def, alice, Zone::Battlefield);
                game.turn.active_player = caster;
                game.turn.priority_player = Some(caster);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                for (owner, count) in [(bob, 1), (carol, 3)] {
                    for _ in 0..count {
                        let land = CardDefinitionBuilder::new(CardId::new(), "Land probe")
                            .card_types(vec![CardType::Land])
                            .build();
                        game.create_object_from_definition(&land, owner, Zone::Battlefield);
                    }
                }
                let spell = CardDefinitionBuilder::new(CardId::new(), "Spell probe")
                    .card_types(vec![card_type])
                    .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
                        ironsmith::mana::ManaSymbol::Generic(mana_value),
                    ]))
                    .build();
                let spell = game.create_object_from_definition(&spell, caster, Zone::Hand);
                game.player_mut(caster)
                    .unwrap()
                    .mana_pool
                    .add(ironsmith::mana::ManaSymbol::Colorless, 20);
                game.refresh_continuous_state();
                let can_cast = compute_legal_actions(&game, caster).iter().any(
                    |a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell),
                );
                let lands = if caster == bob { 1 } else { 3 };
                let expected =
                    caster == alice || card_type == CardType::Creature || mana_value <= lands;
                assert_eq!(
                    can_cast, expected,
                    "caster={caster:?}, type={card_type:?}, MV={mana_value}"
                );
            }
        }
    }
}

#[test]
fn actual_cast_triggers_only_for_opponents_who_spent_no_mana() {
    use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for caster in [alice, bob] {
        for mana_cost in [0, 1] {
            for (uncounterable, source_leaves) in [(false, false), (true, false), (false, true)] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
                game.turn.active_player = caster;
                game.turn.priority_player = Some(caster);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                let land = CardDefinitionBuilder::new(CardId::new(), "Land probe")
                    .card_types(vec![CardType::Land])
                    .build();
                game.create_object_from_definition(&land, caster, Zone::Battlefield);
                let mut spell = CardDefinitionBuilder::new(CardId::new(), "Cast probe")
                    .card_types(vec![CardType::Instant])
                    .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
                        ironsmith::mana::ManaSymbol::Generic(mana_cost),
                    ]))
                    .with_spell_effect(vec![ironsmith::effect::Effect::gain_life(1)]);
                if uncounterable {
                    spell = spell.with_ability(ironsmith::Ability::static_ability(
                        ironsmith::static_abilities::StaticAbility::uncounterable(),
                    ));
                }
                let spell = spell.build();
                let spell = game.create_object_from_definition(&spell, caster, Zone::Hand);
                game.player_mut(caster)
                    .unwrap()
                    .mana_pool
                    .add(ironsmith::mana::ManaSymbol::Colorless, 1);
                game.refresh_continuous_state();
                let action = compute_legal_actions(&game, caster)
                .into_iter()
                .find(
                    |a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell),
                )
                .unwrap();
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                let mut state =
                    ironsmith::game_loop::PriorityLoopState::new(game.players_in_game());
                let mut dm = SelectFirstDecisionMaker;
                let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
                    &mut game,
                    &mut queue,
                    &mut state,
                    &ironsmith::game_loop::PriorityResponse::PriorityAction(action),
                    &mut dm,
                )
                .unwrap();
                for _ in 0..24 {
                    if !game.stack.is_empty() {
                        break;
                    }
                    let ironsmith::decision::GameProgress::NeedsDecisionCtx(ctx) = progress else {
                        panic!("{progress:?}");
                    };
                    progress = ironsmith::game_loop::apply_decision_context_with_dm(
                        &mut game, &mut queue, &mut state, &ctx, &mut dm,
                    )
                    .unwrap();
                }
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                let triggered = caster == bob && mana_cost == 0;
                assert_eq!(
                    game.stack.len(),
                    if triggered { 2 } else { 1 },
                    "caster={caster:?}, mana={mana_cost}"
                );
                if source_leaves {
                    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
                }
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                if triggered && uncounterable {
                    assert_eq!(
                        game.stack.len(),
                        1,
                        "uncounterable spell survives the counter trigger"
                    );
                    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                }
                assert!(game.stack.is_empty());
                assert_eq!(
                    game.player(caster).unwrap().life,
                    if triggered && !uncounterable { 20 } else { 21 }
                );
            }
        }
    }
}

#[test]
fn casting_limit_updates_with_land_count_and_expires_with_source() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let source = game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
    let land = CardDefinitionBuilder::new(CardId::new(), "Land probe")
        .card_types(vec![CardType::Land])
        .build();
    game.create_object_from_definition(&land, bob, Zone::Battlefield);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Spell probe")
        .card_types(vec![CardType::Instant])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
            ironsmith::mana::ManaSymbol::Generic(2),
        ]))
        .build();
    let spell = game.create_object_from_definition(&spell, bob, Zone::Hand);
    game.player_mut(bob)
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Colorless, 10);
    let available = |game: &mut GameState| {
        game.refresh_continuous_state();
        compute_legal_actions(game, bob)
            .iter()
            .any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
    };
    assert!(!available(&mut game));
    let second_land = game.create_object_from_definition(&land, bob, Zone::Battlefield);
    assert!(available(&mut game));
    game.move_object_by_effect(second_land, Zone::Graveyard)
        .unwrap();
    assert!(!available(&mut game));
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    assert!(available(&mut game));
}

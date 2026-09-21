use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::effect::Effect;
use ironsmith::ids::CardId;
use ironsmith::static_abilities::{CompiledStaticAbility, StaticAbility};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::{Ability, CardType, GameState, PlayerId, Zone};

#[test]
fn target_dependent_flash_requires_an_available_matching_target() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let permission =
        CompiledStaticAbility::flash_if_targets_matching(ObjectFilter::permanent().you_control());
    let definition = CardDefinitionBuilder::new(CardId::new(), "Target-dependent timing fixture")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ironsmith::mana::ManaCost::new())
        .with_ability(
            Ability::static_ability(StaticAbility::from_model(permission)).in_zones(vec![
                Zone::Hand,
                Zone::Graveyard,
                Zone::Stack,
            ]),
        )
        .with_spell_effect(vec![Effect::destroy(ChooseSpec::target_permanent())])
        .build();
    let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let permanent = CardDefinitionBuilder::new(CardId::new(), "Target fixture")
        .card_types(vec![CardType::Artifact])
        .build();
    let opponent_target = game.create_object_from_definition(&permanent, bob, Zone::Battlefield);
    let can_cast = |game: &GameState| {
        compute_legal_actions(game, alice).iter().any(|action| {
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)
        })
    };
    assert!(
        !can_cast(&game),
        "an opponent's permanent cannot enable flash"
    );
    let own_target = game.create_object_from_definition(&permanent, alice, Zone::Battlefield);
    assert!(
        can_cast(&game),
        "a legal controlled target must enable casting to begin"
    );
    game.object_mut(own_target)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::shroud()));
    game.refresh_continuous_state();
    assert!(
        !can_cast(&game),
        "an untargetable controlled permanent cannot enable flash"
    );
    game.object_mut(own_target).unwrap().abilities_mut().pop();
    game.refresh_continuous_state();
    attempt_targeted_cast(&game, spell, opponent_target, false);
    attempt_targeted_cast(&game, spell, own_target, true);
    game.turn.active_player = alice;
    attempt_targeted_cast(&game, spell, opponent_target, true);
    game.turn.active_player = bob;
    game.object_mut(spell)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::flash()));
    attempt_targeted_cast(&game, spell, opponent_target, true);
}

fn attempt_targeted_cast(
    game: &GameState,
    spell: ironsmith::ObjectId,
    target: ironsmith::ObjectId,
    expected: bool,
) -> GameState {
    use ironsmith::decision::{GameProgress, SelectFirstDecisionMaker};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    let alice = PlayerId::from_index(0);
    let original_zone = game.object(spell).unwrap().zone;
    let mut game = game.clone();
    let action = compute_legal_actions(&game, alice).into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)
    ).expect("cast can begin");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = if let ironsmith::decisions::context::DecisionContext::Targets(targets) = &ctx {
            assert_eq!(
                targets.requirements.iter().any(|requirement| requirement
                    .legal_targets
                    .contains(&ironsmith::Target::Object(target))),
                expected,
                "target menu must respect current casting timing"
            );
            ironsmith::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &PriorityResponse::Targets(vec![ironsmith::Target::Object(target)]),
                &mut dm,
            )
        } else {
            ironsmith::game_loop::apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &ctx, &mut dm,
            )
        };
    }
    assert_eq!(!game.stack.is_empty(), expected, "cast result: {result:?}");
    if !expected {
        assert!(result.is_err(), "invalid proposal must be rejected");
        assert_eq!(
            game.object(spell).unwrap().zone,
            original_zone,
            "illegal cast rolls back"
        );
    }
    game
}

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Flash Photography",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_has_supported_structure_and_unrounded_score() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
    println!("Unrounded similarity: {:?}", snapshot.similarity_score);
}

#[test]
fn actual_card_copies_permanents_with_conditional_flash_and_flashback() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let definition = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    for zone in [Zone::Hand, Zone::Graveyard] {
        for sorcery_timing in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = if sorcery_timing { alice } else { bob };
            game.turn.priority_player = Some(alice);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Blue, 6);
            let permanent = CardDefinitionBuilder::new(CardId::new(), "Copy fixture")
                .card_types(vec![CardType::Artifact, CardType::Creature])
                .power_toughness(ironsmith::card::PowerToughness::fixed(3, 4))
                .with_ability(Ability::static_ability(StaticAbility::vigilance()))
                .build();
            let opponent = game.create_object_from_definition(&permanent, bob, Zone::Battlefield);
            let own = game.create_object_from_definition(&permanent, alice, Zone::Battlefield);
            let spell = game.create_object_from_definition(&definition, alice, zone);
            let stable = game.object(spell).unwrap().stable_id;
            attempt_targeted_cast(&game, spell, opponent, sorcery_timing);
            let mut cast = attempt_targeted_cast(&game, spell, own, true);
            assert_eq!(
                cast.player(alice).unwrap().mana_pool.total(),
                if zone == Zone::Hand { 2 } else { 0 }
            );
            assert_eq!(cast.stack[0].targets, vec![ironsmith::Target::Object(own)]);
            let mut lost_target = cast.clone();
            lost_target
                .move_object_by_effect(own, Zone::Graveyard)
                .unwrap();
            ironsmith::game_loop::resolve_stack_entry(&mut lost_target).unwrap();
            assert!(
                lost_target.battlefield.iter().all(|id| !matches!(
                    lost_target.object(*id).unwrap().kind,
                    ironsmith::object::ObjectKind::Token
                )),
                "a spell with its only target gone cannot create a copy"
            );
            // This condition grants timing, not a continuing target restriction.
            cast.set_current_controller(own, bob);
            ironsmith::game_loop::resolve_stack_entry(&mut cast).unwrap();
            let copies: Vec<_> = cast
                .battlefield
                .iter()
                .filter_map(|id| cast.object(*id))
                .filter(|object| {
                    object.name.as_str() == "Copy fixture"
                        && cast.controller_of(object) == alice
                        && object.id != own
                })
                .collect();
            assert_eq!(copies.len(), 1, "one token copy should be created");
            let token = copies[0];
            assert!(matches!(token.kind, ironsmith::object::ObjectKind::Token));
            assert_eq!(cast.calculated_power(token.id), Some(3));
            assert_eq!(cast.calculated_toughness(token.id), Some(4));
            assert!(cast.object_has_static_ability_id(
                token.id,
                ironsmith::static_abilities::StaticAbilityId::Vigilance
            ));
            assert_eq!(
                cast.object(cast.find_object_by_stable_id(stable).unwrap())
                    .unwrap()
                    .zone,
                if zone == Zone::Hand {
                    Zone::Graveyard
                } else {
                    Zone::Exile
                }
            );
        }
    }
}

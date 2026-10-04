//! UNVALIDATED: exact frozen programs and real activation/casting transactions.
use ironsmith::ability::AbilityKind;
use ironsmith::card::CardBuilder;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{ColorsContext, ManaPaymentContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::{
    CardId, CardType, Color, GameProgress, GameState, ObjectId, PlayerId, Target, Zone,
};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/snc_exiled_land_mana_grants.json.fixture"
    ))
    .unwrap()
}
fn definitions(card: &serde_json::Value) -> [CardDefinition; 2] {
    let name = card["name"].as_str().unwrap();
    let text = card["text"].as_str().unwrap();
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [direct, materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}
fn blank_land(game: &mut GameState, owner: PlayerId) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Mana-less land")
            .card_types(vec![CardType::Land])
            .build(),
        owner,
        Zone::Battlefield,
    )
}
fn has_mana(game: &GameState, land: ObjectId) -> bool {
    game.current_abilities(land)
        .unwrap_or_default()
        .iter()
        .any(|ability| ability.is_mana_ability())
}
#[derive(Default)]
struct Choices {
    target: Option<ObjectId>,
    casting_land: Option<ObjectId>,
    saw_cast_payment: bool,
    cancel: bool,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert!(
                context
                    .requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(&Target::Object(target)))
            );
            vec![Target::Object(target)]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_colors(&mut self, _game: &GameState, context: &ColorsContext) -> Vec<Color> {
        vec![context.available_colors.as_ref().unwrap()[0]; context.count as usize]
    }
    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        context: &ManaPaymentContext,
    ) -> ManaPaymentResponse {
        if let Some(land) = self.casting_land {
            let original = game
                .cast_origin_snapshot(context.source)
                .expect("cast proposal retains exact origin");
            assert_eq!(original.zone, Zone::Exile);
            assert!(!game.object_completed_cast_from(original.object_id, Zone::Exile));
            assert!(
                has_mana(game, land),
                "the granted mana ability survives proposal and payment"
            );
            assert!(
                context
                    .plan
                    .mana_ability_steps
                    .iter()
                    .any(|step| step.source == land)
            );
            self.saw_cast_payment = true;
        }
        if self.cancel {
            ManaPaymentResponse::Cancel
        } else {
            ManaPaymentResponse::Confirm {
                plan_id: context.plan.id,
                request_hash: context.plan.request_hash,
            }
        }
    }
}
fn run_action(game: &mut GameState, player: PlayerId, action: LegalAction, choices: &mut Choices) {
    game.turn.priority_player = Some(player);
    assert!(
        compute_legal_actions(game, player)
            .unwrap()
            .contains(&action)
    );
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        choices,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none()
            && state.pending_cast.is_none()
            && state.pending_mana_ability.is_none()
        {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("unfinished transaction: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices)
            .unwrap();
    }
    assert!(
        state.pending_activation.is_none()
            && state.pending_cast.is_none()
            && state.pending_mana_ability.is_none()
    );
}
use ironsmith::triggers::TriggerQueue;
fn exile_activation(
    game: &mut GameState,
    definition: &CardDefinition,
    land: ObjectId,
    resolve: bool,
) -> ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Hand);
    let stable = game.object(source).unwrap().stable_id;
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    let index = definition
        .abilities
        .iter()
        .position(|a| matches!(&a.kind, AbilityKind::Activated(_)))
        .unwrap();
    let mut choices = Choices {
        target: Some(land),
        ..Default::default()
    };
    run_action(
        game,
        A,
        LegalAction::ActivateAbility {
            source,
            ability_index: index,
        },
        &mut choices,
    );
    assert!(
        game.object(source).is_none(),
        "the hand incarnation was actually paid"
    );
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    let paid_source = game
        .stack
        .last()
        .unwrap()
        .tagged_objects
        .get(ironsmith_core::tag::SOURCE_EXILED_SELF_TAG)
        .expect("self-exile exports a precise cost result");
    assert_eq!(paid_source.len(), 1);
    assert_eq!(paid_source[0].object_id, exiled);
    if resolve {
        resolve_stack_entry_with(game, &mut choices).unwrap();
    }
    exiled
}
fn color(symbol: &str) -> ManaSymbol {
    match symbol {
        "W" => ManaSymbol::White,
        "U" => ManaSymbol::Blue,
        "B" => ManaSymbol::Black,
        "R" => ManaSymbol::Red,
        "G" => ManaSymbol::Green,
        _ => panic!("unexpected fixture color"),
    }
}
fn fund_except_land(game: &mut GameState, card: &serde_json::Value) {
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 8);
    for symbol in card["colors"].as_array().unwrap().iter().skip(1) {
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(color(symbol.as_str().unwrap()), 1);
    }
}
fn cast_action(game: &mut GameState, card: ObjectId, player: PlayerId) -> Option<LegalAction> {
    game.turn.priority_player = Some(player);
    compute_legal_actions(game, player).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } if *spell_id == card))
}
#[test]
fn exact_five_grants_pay_their_own_cast_then_expire_at_completed_cast() {
    assert_eq!(fixtures().len(), 5);
    for card in fixtures() {
        for definition in definitions(&card) {
            let mut game = game();
            let land = blank_land(&mut game, A);
            assert!(!has_mana(&game, land));
            let exiled = exile_activation(&mut game, &definition, land, true);
            assert!(has_mana(&game, land));
            assert!(!game.object_completed_cast_from(exiled, Zone::Exile));
            fund_except_land(&mut game, &card);
            let action = cast_action(&mut game, exiled, A)
                .expect("source may be cast from its exact exile incarnation");
            let mut choices = Choices {
                casting_land: Some(land),
                ..Default::default()
            };
            run_action(&mut game, A, action, &mut choices);
            assert!(choices.saw_cast_payment);
            assert!(
                game.is_tapped(land),
                "the planner used the newly granted colored mana"
            );
            assert!(game.object_completed_cast_from(exiled, Zone::Exile));
            assert!(
                !has_mana(&game, land),
                "ending event is completion of casting, before resolution"
            );
            game.next_turn();
            assert!(
                !has_mana(&game, land),
                "cast history survives a turn boundary"
            );
        }
    }
}
#[test]
fn leaving_exile_otherwise_never_expires_land_but_does_not_transfer_permission() {
    for card in fixtures() {
        for definition in definitions(&card) {
            let mut game = game();
            let land = blank_land(&mut game, B);
            let exiled = exile_activation(&mut game, &definition, land, true);
            assert!(
                has_mana(&game, land),
                "target land may belong to an opponent"
            );
            let ability = game
                .current_abilities(land)
                .unwrap()
                .iter()
                .position(|a| a.is_mana_ability())
                .unwrap();
            run_action(
                &mut game,
                B,
                LegalAction::ActivateManaAbility {
                    source: land,
                    ability_index: ability,
                },
                &mut Choices::default(),
            );
            assert_eq!(game.player(B).unwrap().mana_pool.total(), 1);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            let graveyard = game.move_object_by_effect(exiled, Zone::Graveyard).unwrap();
            let new_exile = game.move_object_by_effect(graveyard, Zone::Exile).unwrap();
            game.turn.turn_number += 1;
            assert!(has_mana(&game, land));
            fund_except_land(&mut game, &card);
            for symbol in card["colors"].as_array().unwrap() {
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(color(symbol.as_str().unwrap()), 1);
            }
            assert!(
                cast_action(&mut game, new_exile, A).is_none(),
                "permission cannot follow a later incarnation"
            );
            assert!(!game.object_completed_cast_from(exiled, Zone::Exile));
        }
    }
}
#[test]
fn illegal_land_target_fizzles_both_the_ability_grant_and_cast_permission() {
    for card in fixtures() {
        for definition in definitions(&card) {
            let mut game = game();
            let land = blank_land(&mut game, A);
            let exiled = exile_activation(&mut game, &definition, land, false);
            game.move_object_by_effect(land, Zone::Graveyard).unwrap();
            resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
            for symbol in [
                ManaSymbol::Colorless,
                ManaSymbol::White,
                ManaSymbol::Blue,
                ManaSymbol::Black,
                ManaSymbol::Red,
                ManaSymbol::Green,
            ] {
                game.player_mut(A).unwrap().mana_pool.add(symbol, 8);
            }
            assert!(cast_action(&mut game, exiled, A).is_none());
        }
    }
}
#[test]
fn source_leaving_before_resolution_keeps_exact_duration_but_grants_no_new_incarnation_permission()
{
    for definition in definitions(&fixtures()[0]) {
        let mut game = game();
        let land = blank_land(&mut game, A);
        let exiled = exile_activation(&mut game, &definition, land, false);
        let graveyard = game.move_object_by_effect(exiled, Zone::Graveyard).unwrap();
        let later = game.move_object_by_effect(graveyard, Zone::Exile).unwrap();
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
        assert!(has_mana(&game, land));
        for symbol in [
            ManaSymbol::Colorless,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
        ] {
            game.player_mut(A).unwrap().mana_pool.add(symbol, 8);
        }
        assert!(cast_action(&mut game, later, A).is_none());
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Permission source")
                .card_types(vec![CardType::Enchantment])
                .build(),
            A,
            Zone::Battlefield,
        );
        game.effect_store.grant_registry.grant_to_card(
            later,
            Zone::Exile,
            A,
            ironsmith::grant::Grantable::PlayFrom,
            ironsmith::grant_registry::GrantSource::Effect {
                source_id: source,
                expires_end_of_turn: u32::MAX,
            },
        );
        let action = cast_action(&mut game, later, A).unwrap();
        run_action(&mut game, A, action, &mut Choices::default());
        assert!(
            has_mana(&game, land),
            "casting a later incarnation cannot end the old duration"
        );
    }
}
#[test]
fn cancelling_cast_payment_does_not_consume_the_duration_or_permission() {
    let card = fixtures().remove(0);
    for definition in definitions(&card) {
        let mut game = game();
        let land = blank_land(&mut game, A);
        let exiled = exile_activation(&mut game, &definition, land, true);
        fund_except_land(&mut game, &card);
        let before = game.player(A).unwrap().mana_pool.clone();
        let action = cast_action(&mut game, exiled, A).unwrap();
        run_action(
            &mut game,
            A,
            action,
            &mut Choices {
                cancel: true,
                ..Default::default()
            },
        );
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        assert_eq!(game.player(A).unwrap().mana_pool, before);
        assert!(!game.is_tapped(land));
        assert!(has_mana(&game, land));
        assert!(!game.object_completed_cast_from(exiled, Zone::Exile));
        assert!(cast_action(&mut game, exiled, A).is_some());
    }
}

#[test]
fn snc_duration_payload_round_trips_without_changing_existing_until_shapes() {
    let old: ironsmith_core::Until = serde_json::from_str("\"EndOfTurn\"").unwrap();
    assert_eq!(old, ironsmith_core::Until::EndOfTurn);
    let duration = ironsmith_core::Until::ObjectIsCast {
        object: ironsmith_core::ContinuousDurationObject::Specific(ObjectId::from_raw(321)),
        from_zone: Zone::Exile,
    };
    let wire = serde_json::to_string(&duration).unwrap();
    let decoded: ironsmith_core::Until = serde_json::from_str(&wire).unwrap();
    assert_eq!(decoded, duration);
}

#[test]
fn snc_modified_self_exile_cost_still_grants_mana_without_exile_permission() {
    for card in fixtures() {
        for definition in definitions(&card) {
            for prevented in [false, true] {
                let mut game = game();
                let land = blank_land(&mut game, A);
                let source = game.create_object_from_definition(&definition, A, Zone::Hand);
                let stable = game.object(source).unwrap().stable_id;
                let mut replacement = ironsmith::replacement::ZoneReplacementSpec::new(
                    ironsmith::target::ObjectFilter::specific(source),
                    Zone::Graveyard,
                )
                .from_zone(Zone::Hand)
                .to_zone(Zone::Exile)
                .build(land, A);
                if prevented {
                    replacement.replacement = ironsmith::replacement::ReplacementAction::Prevent;
                }
                game.effect_store
                    .replacement_effects
                    .add_one_shot_effect(replacement);
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 2);
                let ability_index = definition
                    .abilities
                    .iter()
                    .position(|ability| matches!(&ability.kind, AbilityKind::Activated(_)))
                    .unwrap();
                let mut choices = Choices {
                    target: Some(land),
                    ..Default::default()
                };
                run_action(
                    &mut game,
                    A,
                    LegalAction::ActivateAbility {
                        source,
                        ability_index,
                    },
                    &mut choices,
                );
                let result = game.find_object_by_stable_id(stable).unwrap();
                let expected_zone = if prevented {
                    Zone::Hand
                } else {
                    Zone::Graveyard
                };
                assert_eq!(game.object(result).unwrap().zone, expected_zone);
                assert_eq!(
                    game.stack.len(),
                    1,
                    "a modified/prevented legal cost remains paid"
                );
                let retained = &game.stack.last().unwrap().tagged_objects
                    [ironsmith_core::tag::SOURCE_EXILED_SELF_TAG][0];
                assert_eq!(retained.object_id, result);
                assert_eq!(retained.zone, expected_zone);
                resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                assert!(
                    has_mana(&game, land),
                    "replacement must not silently suppress the land grant"
                );
                assert!(
                    !game.effect_store.grant_registry.card_can_play_from_zone(
                        &game,
                        result,
                        expected_zone,
                        A
                    ),
                    "while-exiled permission must not become a hand/graveyard permission"
                );
                game.next_turn();
                assert!(has_mana(&game, land));
            }
        }
    }
}

#[test]
fn snc_unbound_cast_duration_is_an_error_instead_of_silent_no_grant() {
    use ironsmith::effects::EffectExecutor;
    let mut game = game();
    let land = blank_land(&mut game, A);
    let effect = ironsmith::effects::ApplyContinuousEffect::with_spec(
        ironsmith::target::ChooseSpec::SpecificObject(land),
        ironsmith::continuous::Modification::AddAbility(
            ironsmith::static_abilities::StaticAbility::flying(),
        ),
        ironsmith_core::Until::ObjectIsCast {
            object: ironsmith_core::ContinuousDurationObject::Tagged(
                ironsmith_core::tag::SOURCE_EXILED_SELF_TAG.into(),
            ),
            from_zone: Zone::Exile,
        },
    );
    let mut ctx = ironsmith::effects::EffectContext::new_default(land, A);
    assert!(matches!(
        effect.execute(&mut game, &mut ctx),
        Err(ironsmith::effects::ExecutionError::TagNotFound(_))
    ));
}

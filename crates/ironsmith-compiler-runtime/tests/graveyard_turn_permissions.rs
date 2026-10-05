//! UNVALIDATED source-authored graveyard permission scenarios; never run in this stage.
#![allow(dead_code)]
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{LinkedFaceLayout, PowerToughness};
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack, resolve_stack_entry,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::special_actions::{SpecialAction, can_perform_check, perform};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/graveyard_turn_permissions.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|r| r["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    main(&mut game, A);
    for player in [A, B] {
        for color in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            game.player_mut(player).unwrap().mana_pool.add(color, 20);
        }
    }
    game
}
fn main(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}
fn resource(game: &mut GameState, player: PlayerId, text: &str, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition("Permission resource", text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn casts(game: &GameState, player: PlayerId, id: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .filter(|a| matches!(a, LegalAction::CastSpell {spell_id, ..} if *spell_id == id))
        .collect()
}
fn announce(game: &mut GameState, player: PlayerId, action: LegalAction) -> ObjectId {
    let LegalAction::CastSpell { spell_id, .. } = &action else {
        panic!("not a cast");
    };
    let stable = game.object(*spell_id).unwrap().stable_id;
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..40 {
        if !state.has_pending_action() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("pending without a decision");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
            .unwrap();
    }
    assert!(!state.has_pending_action());
    let id = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(id).unwrap().zone, Zone::Stack);
    id
}
fn settle(game: &mut GameState) {
    let mut queue = TriggerQueue::new();
    for _ in 0..20 {
        put_triggers_on_stack(game, &mut queue).unwrap();
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry(game).unwrap();
    }
    panic!("triggers did not settle");
}
fn sourced_cast(game: &GameState, card: ObjectId, provider: ObjectId) -> Option<LegalAction> {
    casts(game, A, card).into_iter().find(|action| matches!(action, LegalAction::CastSpell {
        casting_method: CastingMethod::PlayFrom { source, use_alternative: None, .. }
            | CastingMethod::SplitOtherHalfPlayFrom { source, use_alternative: None, .. }, .. } if *source == provider))
}
fn move_effect(game: &mut GameState, source: ObjectId, object: ObjectId, zone: Zone) {
    let controller = game.object(source).map_or(A, |object| object.owner);
    ironsmith::execute_effect(
        game,
        &ironsmith::effect::Effect::move_to_zone(
            ironsmith::ChooseSpec::SpecificObject(object),
            zone,
            false,
        ),
        &mut ironsmith::effects::EffectContext::new_default(source, controller),
    )
    .unwrap();
}
fn mill(game: &mut GameState, player: PlayerId, amount: u32) {
    let source = resource(game, A, "Type: Artifact", Zone::Battlefield);
    ironsmith::execute_effect(
        game,
        &ironsmith::effect::Effect::mill_player(
            amount as i32,
            ironsmith::PlayerFilter::Specific(player),
        ),
        &mut ironsmith::effects::EffectContext::new_default(source, A),
    )
    .unwrap();
}
fn enter(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Hand);
    let receipt = game
        .move_object_with_etb_processing_with_dm(
            source,
            Zone::Battlefield,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
    assert!(!receipt.pending);
    assert!(receipt.programs.is_empty());
    let source = receipt.original.into_result().unwrap().new_id;
    settle(game);
    source
}
#[test]
fn four_complete_frozen_bodies_keep_strict_artifact_programs() {
    assert_eq!(rows().len(), 4);
    for row in rows() {
        assert_eq!(definitions(row["name"].as_str().unwrap()).len(), 2);
    }
}
#[test]
fn kagha_origin_and_raul_mill_action_use_current_destination_incarnations() {
    for name in ["Kagha, Shadow Archdruid", "Raul, Trouble Shooter"] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let ordinary = resource(
                &mut game,
                A,
                "Mana cost: {0}\nType: Artifact",
                Zone::Library,
            );
            let stable = game.object(ordinary).unwrap().stable_id;
            move_effect(&mut game, source, ordinary, Zone::Graveyard);
            let ordinary = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                sourced_cast(&game, ordinary, source).is_some(),
                name.starts_with("Kagha")
            );
            let milled = resource(
                &mut game,
                A,
                "Mana cost: {0}\nType: Creature\nPower/Toughness: 1/1",
                Zone::Library,
            );
            let stable = game.object(milled).unwrap().stable_id;
            mill(&mut game, A, 1);
            let milled = game.find_object_by_stable_id(stable).unwrap();
            assert!(sourced_cast(&game, milled, source).is_some());
            move_effect(&mut game, source, milled, Zone::Hand);
            let hand = game.find_object_by_stable_id(stable).unwrap();
            move_effect(&mut game, source, hand, Zone::Graveyard);
            let returned = game.find_object_by_stable_id(stable).unwrap();
            assert!(
                sourced_cast(&game, returned, source).is_none(),
                "a later graveyard incarnation has no old permission"
            );
            let foreign = resource(
                &mut game,
                B,
                "Mana cost: {0}\nType: Artifact",
                Zone::Library,
            );
            let stable = game.object(foreign).unwrap().stable_id;
            mill(&mut game, B, 1);
            assert!(
                sourced_cast(
                    &game,
                    game.find_object_by_stable_id(stable).unwrap(),
                    source
                )
                .is_none()
            );
            game.next_turn();
            game.next_turn();
            main(&mut game, A);
            assert!(
                sourced_cast(&game, ordinary, source).is_none(),
                "history expires at the turn boundary"
            );
        }
    }
}
#[test]
fn graveyard_spell_and_land_arms_share_one_completed_use_without_relaxing_timing() {
    for name in [
        "Kagha, Shadow Archdruid",
        "Serra Paragon",
        "The Eighth Doctor",
    ] {
        for definition in definitions(name) {
            for land_first in [false, true] {
                let mut game = game();
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let spell = resource(
                    &mut game,
                    A,
                    "Mana cost: {0}\nType: Artifact",
                    Zone::Library,
                );
                let spell_stable = game.object(spell).unwrap().stable_id;
                let land = resource(&mut game, A, "Type: Legendary Land", Zone::Library);
                let land_stable = game.object(land).unwrap().stable_id;
                mill(&mut game, A, 2);
                let spell = game.find_object_by_stable_id(spell_stable).unwrap();
                let land = game.find_object_by_stable_id(land_stable).unwrap();
                assert!(
                    sourced_cast(&game, spell, source).is_some(),
                    "{name}: missing spell permission"
                );
                assert!(
                    can_perform_check(&SpecialAction::PlayLand { card_id: land }, &game, A).is_ok()
                );
                main(&mut game, B);
                assert!(sourced_cast(&game, spell, source).is_none());
                main(&mut game, A);
                if land_first {
                    perform(
                        SpecialAction::PlayLand { card_id: land },
                        &mut game,
                        A,
                        &mut SelectFirstDecisionMaker,
                    )
                    .unwrap();
                    assert!(sourced_cast(&game, spell, source).is_none());
                } else {
                    let action = sourced_cast(&game, spell, source).unwrap();
                    announce(&mut game, A, action);
                    settle(&mut game);
                    assert!(
                        can_perform_check(&SpecialAction::PlayLand { card_id: land }, &game, A)
                            .is_err()
                    );
                }
                game.phase_out(source);
                game.phase_in(source);
                let other = resource(
                    &mut game,
                    A,
                    "Mana cost: {0}\nType: Artifact",
                    Zone::Library,
                );
                let stable = game.object(other).unwrap().stable_id;
                mill(&mut game, A, 1);
                let other = game.find_object_by_stable_id(stable).unwrap();
                assert!(
                    sourced_cast(&game, other, source).is_none(),
                    "phasing does not reset a used occurrence"
                );
            }
        }
    }
}
#[test]
fn history_qualified_permission_secondary_attack_mill_and_tap_mill_bodies_execute() {
    for definition in definitions("Kagha, Shadow Archdruid") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for _ in 0..2 {
            resource(
                &mut game,
                A,
                "Mana cost: {0}\nType: Artifact",
                Zone::Library,
            );
        }
        game.remove_summoning_sickness(source);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.mark_combat_phase_started();
        let mut combat = ironsmith::CombatState::default();
        let mut queue = TriggerQueue::new();
        ironsmith::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[ironsmith::AttackerDeclaration {
                creature: source,
                target: ironsmith::AttackTarget::Player(B),
            }],
        )
        .unwrap();
        game.combat = Some(combat);
        put_triggers_on_stack(&mut game, &mut queue).unwrap();
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 2);
        assert!(game.current_has_static_ability_id(source, StaticAbilityId::Deathtouch));
        ironsmith::execute_cleanup_step(&mut game);
        assert!(!game.current_has_static_ability_id(source, StaticAbilityId::Deathtouch));
    }
    for definition in definitions("Raul, Trouble Shooter") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        for player in [A, B] {
            resource(
                &mut game,
                player,
                "Mana cost: {0}\nType: Artifact",
                Zone::Library,
            );
        }
        let action = compute_legal_actions(&game, A)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility {source: id, ..} if *id == source))
            .unwrap();
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        settle(&mut game);
        assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        assert_eq!(game.player(B).unwrap().graveyard.len(), 1);
    }
    for definition in definitions("The Eighth Doctor") {
        let mut game = game();
        for _ in 0..3 {
            resource(
                &mut game,
                A,
                "Mana cost: {0}\nType: Artifact",
                Zone::Library,
            );
        }
        enter(&mut game, &definition);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 3);
    }
}
#[test]
fn land_and_spell_recipient_abilities_survive_cleanup_source_departure_and_change_of_control() {
    for name in ["Serra Paragon", "The Eighth Doctor"] {
        for definition in definitions(name) {
            for land in [false, true] {
                let mut game = game();
                let provider =
                    game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let card = resource(
                    &mut game,
                    A,
                    if land {
                        "Type: Legendary Land"
                    } else {
                        "Mana cost: {0}\nType: Artifact"
                    },
                    Zone::Graveyard,
                );
                let stable = game.object(card).unwrap().stable_id;
                if land {
                    perform(
                        SpecialAction::PlayLand { card_id: card },
                        &mut game,
                        A,
                        &mut SelectFirstDecisionMaker,
                    )
                    .unwrap();
                } else {
                    let action = sourced_cast(&game, card, provider).unwrap_or_else(|| {
                        panic!(
                            "{}: missing recipient spell permission",
                            game.object(provider).unwrap().name
                        )
                    });
                    announce(&mut game, A, action);
                    settle(&mut game);
                }
                let permanent = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
                assert_eq!(
                    game.object(permanent)
                        .unwrap()
                        .temporary_static_ability_grants
                        .iter()
                        .filter(|grant| grant.expires_end_of_turn.is_none())
                        .count(),
                    1
                );
                game.phase_out(permanent);
                game.phase_in(permanent);
                move_effect(&mut game, permanent, provider, Zone::Hand);
                ironsmith::execute_cleanup_step(&mut game);
                game.next_turn();
                main(&mut game, B);
                let control = ironsmith::effect::Effect::gain_control_with_duration(
                    ironsmith::ChooseSpec::SpecificObject(permanent),
                    ironsmith::effect::Until::Forever,
                );
                ironsmith::execute_effect(
                    &mut game,
                    &control,
                    &mut ironsmith::effects::EffectContext::new_default(permanent, B),
                )
                .unwrap();
                assert_eq!(game.current_controller(permanent), Some(B));
                move_effect(&mut game, permanent, permanent, Zone::Graveyard);
                let after = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(
                    game.object(after).unwrap().zone,
                    if name == "Serra Paragon" {
                        Zone::Graveyard
                    } else {
                        Zone::Exile
                    }
                );
                settle(&mut game);
                let after = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(after).unwrap().zone, Zone::Exile);
                assert_eq!(
                    game.player(B).unwrap().life,
                    if name == "Serra Paragon" { 22 } else { 20 }
                );
                assert_eq!(game.player(A).unwrap().life, 20);
                // Returning the physical card cannot carry a previous incarnation's ability.
                move_effect(&mut game, after, after, Zone::Battlefield);
                let fresh = game.find_object_by_stable_id(stable).unwrap();
                assert!(
                    game.object(fresh)
                        .unwrap()
                        .temporary_static_ability_grants
                        .is_empty()
                );
                move_effect(&mut game, fresh, fresh, Zone::Hand);
                settle(&mut game);
                assert_eq!(
                    game.object(game.find_object_by_stable_id(stable).unwrap())
                        .unwrap()
                        .zone,
                    Zone::Hand
                );
            }
        }
    }
}
#[test]
fn serra_death_trigger_tracks_the_departing_incarnation_and_still_gains_life_if_it_moves_again() {
    for definition in definitions("Serra Paragon") {
        let mut game = game();
        let provider = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let card = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Artifact",
            Zone::Graveyard,
        );
        let stable = game.object(card).unwrap().stable_id;
        let action = sourced_cast(&game, card, provider).unwrap_or_else(|| {
            panic!(
                "{}: missing recipient spell permission",
                game.object(provider).unwrap().name
            )
        });
        announce(&mut game, A, action);
        settle(&mut game);
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        move_effect(&mut game, provider, permanent, Zone::Graveyard);
        let grave = game.find_object_by_stable_id(stable).unwrap();
        let mut queue = TriggerQueue::new();
        put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert!(!game.stack_is_empty());
        move_effect(&mut game, provider, grave, Zone::Hand);
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 22);
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Hand
        );
    }
}
#[test]
fn persistent_recipient_ability_is_removable_and_never_copied_from_a_spell() {
    for definition in definitions("The Eighth Doctor") {
        let mut game = game();
        let provider = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let card = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Artifact",
            Zone::Graveyard,
        );
        let stable = game.object(card).unwrap().stable_id;
        let action = sourced_cast(&game, card, provider).unwrap_or_else(|| {
            panic!(
                "{}: missing recipient spell permission",
                game.object(provider).unwrap().name
            )
        });
        let stack = announce(&mut game, A, action);
        let copy_id = game.new_object_id();
        let copy = ironsmith::Object::spell_copy_of(game.object(stack).unwrap(), copy_id, A);
        assert!(
            copy.temporary_static_ability_grants.is_empty(),
            "the permission rider is not a copiable value"
        );
        settle(&mut game);
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        let lose =
            ironsmith::effect::Effect::new(ironsmith::effects::ApplyContinuousEffect::with_spec(
                ironsmith::ChooseSpec::SpecificObject(permanent),
                ironsmith::continuous::Modification::RemoveAllAbilities,
                ironsmith::effect::Until::EndOfTurn,
            ));
        ironsmith::execute_effect(
            &mut game,
            &lose,
            &mut ironsmith::effects::EffectContext::new_default(provider, A),
        )
        .unwrap();
        move_effect(&mut game, provider, permanent, Zone::Hand);
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Hand
        );
    }
}
#[test]
fn serra_captured_rider_survives_sacrificing_its_provider_as_a_cast_cost() {
    for definition in definitions("Serra Paragon") {
        let mut game = game();
        let provider = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let card = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Artifact\nAs an additional cost to cast this spell, sacrifice a creature.",
            Zone::Graveyard,
        );
        let stable = game.object(card).unwrap().stable_id;
        let action = sourced_cast(&game, card, provider).unwrap_or_else(|| {
            panic!(
                "{}: missing recipient spell permission",
                game.object(provider).unwrap().name
            )
        });
        announce(&mut game, A, action);
        assert!(game.object(provider).is_none());
        settle(&mut game);
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        move_effect(&mut game, permanent, permanent, Zone::Graveyard);
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 22);
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Exile
        );
    }
}
#[test]
fn serra_mana_value_and_doctor_historic_tests_use_the_chosen_spell_face() {
    for name in ["Serra Paragon", "The Eighth Doctor"] {
        for definition in definitions(name) {
            let mut game = game();
            let provider = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let front_id = CardId::new();
            let back_id = CardId::new();
            let front = CardDefinitionBuilder::new(front_id, "Unqualified front")
                .card_types(vec![CardType::Creature])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(7)]))
                .power_toughness(PowerToughness::fixed(7, 7))
                .other_face(back_id)
                .other_face_name("Qualified back")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            let back = CardDefinitionBuilder::new(back_id, "Qualified back")
                .card_types(vec![CardType::Artifact])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
                .other_face(front_id)
                .other_face_name("Unqualified front")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            let card = game.create_object_from_definition(&front, A, Zone::Graveyard);
            let action = sourced_cast(&game, card, provider).unwrap_or_else(|| {
                panic!(
                    "{}: missing recipient spell permission",
                    game.object(provider).unwrap().name
                )
            });
            assert!(matches!(
                action,
                LegalAction::CastSpell {
                    casting_method: CastingMethod::SplitOtherHalfPlayFrom { .. },
                    ..
                }
            ));
            let stack = announce(&mut game, A, action);
            assert_eq!(game.object(stack).unwrap().name.as_ref(), "Qualified back");
            assert!(
                game.object(stack)
                    .unwrap()
                    .temporary_static_ability_grants
                    .iter()
                    .any(|grant| grant.expires_end_of_turn.is_none())
            );
        }
    }
}
#[test]
fn composite_price_keeps_the_graveyard_origin_rider_and_both_use_identities() {
    for definition in definitions("Serra Paragon") {
        let mut game = game();
        let provider = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let price_rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../fixtures/independent_casting_prices.json.fixture"
        ))
        .unwrap();
        let row = price_rows
            .iter()
            .find(|row| row["name"] == "As Foretold")
            .unwrap();
        let price =
            compile_to_runtime_definition("As Foretold", row["text"].as_str().unwrap(), false)
                .unwrap();
        let price = game.create_object_from_definition(&price, A, Zone::Battlefield);
        let card = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Artifact",
            Zone::Graveyard,
        );
        let stable = game.object(card).unwrap().stable_id;
        let action = casts(&game, A, card).into_iter().find(|action| matches!(action, LegalAction::CastSpell { casting_method: CastingMethod::AlternativePrice {price: selected, origin, ..}, .. }
            if selected.source == price && matches!(origin.as_ref(), CastingMethod::PlayFrom {source, ..} if *source == provider))).unwrap();
        let uses_before = game.turn_store.grant_cast_uses_this_turn.len();
        announce(&mut game, A, action);
        settle(&mut game);
        assert_eq!(
            game.turn_store.grant_cast_uses_this_turn.len(),
            uses_before + 2
        );
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        move_effect(&mut game, provider, permanent, Zone::Graveyard);
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 22);
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Exile
        );
    }
}
#[test]
fn a_paused_land_entry_rolls_back_permission_usage_and_pending_recipient_binding() {
    struct Pause(bool);
    impl ironsmith::DecisionMaker for Pause {
        fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::BooleanContext) -> bool {
            self.0 = true;
            false
        }
        fn awaiting_choice(&self) -> bool {
            self.0
        }
    }
    for definition in definitions("Serra Paragon") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let card = resource(
            &mut game,
            A,
            "Type: Land\nAs this land enters, you may pay 2 life. If you don't, it enters tapped.",
            Zone::Graveyard,
        );
        let stable = game.object(card).unwrap().stable_id;
        let uses = game.turn_store.grant_cast_uses_this_turn.clone();
        let mut pause = Pause(false);
        perform(
            SpecialAction::PlayLand { card_id: card },
            &mut game,
            A,
            &mut pause,
        )
        .unwrap();
        assert!(pause.0);
        assert_eq!(game.object(card).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.turn_store.grant_cast_uses_this_turn, uses);
        assert_eq!(game.player(A).unwrap().lands_played_this_turn, 0);
        assert!(!game.has_library_top_announcement());
        assert_eq!(game.player(A).unwrap().life, 20);
        perform(
            SpecialAction::PlayLand { card_id: card },
            &mut game,
            A,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            game.object(permanent)
                .unwrap()
                .temporary_static_ability_grants
                .iter()
                .filter(|grant| grant.expires_end_of_turn.is_none())
                .count(),
            1
        );
        assert!(!game.has_library_top_announcement());
    }
}
#[test]
fn a_legacy_compiled_grant_without_the_new_vector_retains_its_empty_rider_meaning() {
    // The generic compiled GrantSpec schema is distinct from a retained live
    // Grant and its explicitly complete recipient-registration state.
    type Program = ironsmith_core::GrantSpec<String, String, String, String>;
    let grant = Program::new(
        ironsmith_core::Grantable::PlayFrom,
        ironsmith_core::ObjectFilter::default(),
        ironsmith_core::Zone::Graveyard,
    );
    let mut legacy = serde_json::to_value(&grant).unwrap();
    assert!(
        legacy
            .as_object_mut()
            .unwrap()
            .remove("permanent_this_way_grants")
            .is_some()
    );
    let decoded: Program = serde_json::from_value(legacy).unwrap();
    assert!(decoded.permanent_this_way_grants.is_empty());
    assert_eq!(decoded, grant);
}

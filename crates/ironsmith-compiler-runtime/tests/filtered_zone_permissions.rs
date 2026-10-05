//! UNVALIDATED source-authored scenarios; no build or execution in this stage.
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
        "../../../fixtures/filtered_zone_permissions.json.fixture"
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
#[test]
fn complete_frozen_bodies_round_trip_without_unsupported_fallbacks() {
    for row in rows() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
        }
    }
}
#[test]
fn assemble_uses_current_top_power_not_mana_value_and_spends_only_completed_cast() {
    for definition in definitions("Assemble the Players") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let lower = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 2/2",
            Zone::Library,
        );
        let big = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 3/3",
            Zone::Library,
        );
        assert!(casts(&game, A, big).is_empty());
        assert!(casts(&game, A, lower).is_empty());
        game.move_object_by_effect(big, Zone::Hand).unwrap();
        let action = casts(&game, A, lower).into_iter().next().unwrap();
        let stack = announce(&mut game, A, action);
        assert!(
            game.object(stack)
                .unwrap()
                .cast_grant_usage_identity
                .is_some()
        );
        let later = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 1/1",
            Zone::Library,
        );
        resolve_stack_entry(&mut game).unwrap();
        assert!(casts(&game, A, later).is_empty());
        game.phase_out(host);
        game.phase_in(host);
        assert!(casts(&game, A, later).is_empty());
        let gone = game.move_object_by_effect(host, Zone::Exile).unwrap();
        game.move_object_by_effect(gone, Zone::Battlefield).unwrap();
        assert!(!casts(&game, A, later).is_empty());
    }
}
#[test]
fn library_origins_types_and_live_source_scope_are_authoritative() {
    for (name, cases) in [
        (
            "Crystal Skull, Isu Spyglass",
            vec![
                ("Artifact", true),
                ("Legendary Creature", true),
                ("Creature", false),
            ],
        ),
        (
            "Case of the Locked Hothouse",
            vec![
                ("Enchantment", true),
                ("Creature", true),
                ("Artifact", false),
                ("Sorcery", false),
            ],
        ),
        (
            "Elsha of the Infinite",
            vec![("Artifact", true), ("Sorcery", true), ("Creature", false)],
        ),
    ] {
        for definition in definitions(name) {
            for (kind, permitted) in &cases {
                let mut game = game();
                let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                if name.starts_with("Case") {
                    game.solve_case(host);
                }
                let text = format!("Mana cost: {{0}}\nType: {kind}\nPower/Toughness: 2/2");
                let own = resource(&mut game, A, &text, Zone::Library);
                let other = resource(&mut game, B, &text, Zone::Library);
                assert_eq!(
                    !casts(&game, A, own).is_empty(),
                    *permitted,
                    "{name}: {kind}"
                );
                assert!(casts(&game, A, other).is_empty());
                game.phase_out(host);
                assert!(casts(&game, A, own).is_empty());
                game.phase_in(host);
                game.set_current_controller(host, B).unwrap();
                assert!(casts(&game, A, own).is_empty());
                main(&mut game, B);
                assert_eq!(!casts(&game, B, other).is_empty(), *permitted);
            }
        }
    }
}
#[test]
fn top_land_permission_rejects_deeper_cards_in_direct_special_action_validation() {
    for definition in definitions("Case of the Locked Hothouse") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let deep = resource(&mut game, A, "Type: Land", Zone::Library);
        let top = resource(&mut game, A, "Type: Land", Zone::Library);
        assert!(can_perform_check(&SpecialAction::PlayLand { card_id: top }, &game, A).is_err());
        game.solve_case(host);
        assert!(can_perform_check(&SpecialAction::PlayLand { card_id: deep }, &game, A).is_err());
        assert!(can_perform_check(&SpecialAction::PlayLand { card_id: top }, &game, A).is_ok());
        perform(
            SpecialAction::PlayLand { card_id: top },
            &mut game,
            A,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert!(
            can_perform_check(&SpecialAction::PlayLand { card_id: deep }, &game, A).is_ok(),
            "the unsolved first ability supplies the extra land play"
        );
        perform(
            SpecialAction::PlayLand { card_id: deep },
            &mut game,
            A,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(game.player(A).unwrap().lands_played_this_turn, 2);
    }
}
#[test]
fn elsha_timing_belongs_to_the_selected_library_permission_and_prowess_still_triggers() {
    for definition in definitions("Elsha of the Infinite") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let hand = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.",
            Zone::Hand,
        );
        let top = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.",
            Zone::Library,
        );
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(Step::End);
        assert!(casts(&game, A, hand).is_empty());
        let action = casts(&game, A, top).into_iter().next().unwrap();
        announce(&mut game, A, action);
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 21);
        assert_eq!(
            game.current_power(host),
            Some(4),
            "Prowess observes the actual noncreature cast"
        );
    }
}
fn modal(
    game: &mut GameState,
    owner: PlayerId,
    zone: Zone,
    front_creature: bool,
    back_power: i32,
) -> ObjectId {
    let front_id = CardId::new();
    let back_id = CardId::new();
    let front = CardDefinitionBuilder::new(front_id, "Permission front")
        .mana_cost(ManaCost::new())
        .card_types(vec![if front_creature {
            CardType::Creature
        } else {
            CardType::Artifact
        }])
        .power_toughness(PowerToughness::fixed(2, 2))
        .other_face(back_id)
        .other_face_name("Permission back")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    let back = CardDefinitionBuilder::new(back_id, "Permission back")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(back_power, 3))
        .other_face(front_id)
        .other_face_name("Permission front")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    game.create_object_from_definition(&front, owner, zone)
}
#[test]
fn alternate_spell_face_retains_grant_identity_and_matches_its_own_power() {
    for definition in definitions("Assemble the Players") {
        for (front_creature, back_power, allowed) in [(false, 2, true), (true, 3, false)] {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let candidate = modal(&mut game, A, Zone::Library, front_creature, back_power);
            let alternate = casts(&game, A, candidate).into_iter().find(|a| matches!(a,
            LegalAction::CastSpell {casting_method: CastingMethod::SplitOtherHalfPlayFrom {source, use_alternative: None, ..}, ..} if *source == host));
            assert_eq!(alternate.is_some(), allowed);
            assert_eq!(
                game.object(candidate).unwrap().name.as_ref(),
                "Permission front"
            );
            if let Some(action) = alternate {
                let stack = announce(&mut game, A, action);
                assert_eq!(game.object(stack).unwrap().name.as_ref(), "Permission back");
                assert!(
                    game.object(stack)
                        .unwrap()
                        .cast_grant_usage_identity
                        .is_some()
                );
                resolve_stack_entry(&mut game).unwrap();
                assert!(game.battlefield.iter().any(|id| {
                    game.object(*id)
                        .is_some_and(|o| o.name.as_ref() == "Permission back")
                }));
            }
        }
    }
}
#[test]
fn graveyard_permissions_keep_owner_and_land_subtype_or_spell_subtype_scopes() {
    for name in ["Titania, Nature's Force", "Zask, Skittering Swarmlord"] {
        for definition in definitions(name) {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let forest = resource(&mut game, A, "Type: Land — Forest", Zone::Graveyard);
            let island = resource(&mut game, A, "Type: Land — Island", Zone::Graveyard);
            let foreign = resource(&mut game, B, "Type: Land — Forest", Zone::Graveyard);
            assert!(
                can_perform_check(&SpecialAction::PlayLand { card_id: forest }, &game, A).is_ok()
            );
            assert_eq!(
                can_perform_check(&SpecialAction::PlayLand { card_id: island }, &game, A).is_ok(),
                name.starts_with("Zask")
            );
            assert!(
                can_perform_check(&SpecialAction::PlayLand { card_id: foreign }, &game, A).is_err()
            );
            let insect = resource(
                &mut game,
                A,
                "Mana cost: {0}\nType: Creature — Insect\nPower/Toughness: 2/2",
                Zone::Graveyard,
            );
            let soldier = resource(
                &mut game,
                A,
                "Mana cost: {0}\nType: Creature — Soldier\nPower/Toughness: 2/2",
                Zone::Graveyard,
            );
            assert_eq!(
                !casts(&game, A, insect).is_empty(),
                name.starts_with("Zask")
            );
            assert!(casts(&game, A, soldier).is_empty());
        }
    }
}
#[test]
fn assemble_does_not_cast_land_creatures() {
    for definition in definitions("Assemble the Players") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let arbor = resource(
            &mut game,
            A,
            "Type: Land Creature — Forest Dryad\nPower/Toughness: 1/1",
            Zone::Library,
        );
        assert!(casts(&game, A, arbor).is_empty());
        assert!(can_perform_check(&SpecialAction::PlayLand { card_id: arbor }, &game, A).is_err());
    }
}

#[test]
fn graveyard_forest_permission_enumerates_and_plays_only_the_eligible_land_face() {
    for definition in definitions("Titania, Nature's Force") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let front_id = CardId::new();
        let back_id = CardId::new();
        let front = CardDefinitionBuilder::new(front_id, "Island front")
            .card_types(vec![CardType::Land])
            .subtypes(vec![ironsmith::types::Subtype::Island])
            .other_face(back_id)
            .other_face_name("Forest back")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        let back = CardDefinitionBuilder::new(back_id, "Forest back")
            .card_types(vec![CardType::Land])
            .subtypes(vec![ironsmith::types::Subtype::Forest])
            .other_face(front_id)
            .other_face_name("Island front")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);
        let id = game.create_object_from_definition(&front, A, Zone::Graveyard);
        let actions = compute_legal_actions(&game, A).unwrap();
        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, LegalAction::PlayLand {land_id} if *land_id == id))
        );
        assert!(actions.iter().any(
            |action| matches!(action, LegalAction::PlayLandBackFace {land_id} if *land_id == id)
        ));
        perform(
            SpecialAction::PlayLandBackFace { card_id: id },
            &mut game,
            A,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        settle(&mut game);
        assert!(game.battlefield.iter().any(|id| {
            game.current_has_subtype(*id, ironsmith::types::Subtype::Elemental)
                && game.current_power(*id) == Some(5)
        }));
    }
}
#[test]
fn lunar_whale_crew_and_actual_attack_enable_the_permission_only_for_that_turn() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    for definition in definitions("The Lunar Whale") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(host);
        resource(
            &mut game,
            A,
            "Type: Creature\nPower/Toughness: 2/2",
            Zone::Battlefield,
        );
        let top = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Artifact",
            Zone::Library,
        );
        assert!(casts(&game, A, top).is_empty());
        let action = compute_legal_actions(&game, A)
            .unwrap()
            .into_iter()
            .find(|action| {
                matches!(action,
            LegalAction::ActivateAbility {source, ..} if *source == host)
            })
            .unwrap();
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..20 {
            if !state.has_pending_action() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("pending crew without a decision");
            };
            progress = apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &context, &mut dm,
            )
            .unwrap();
        }
        assert!(!state.has_pending_action());
        resolve_stack_entry(&mut game).unwrap();
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Flying));
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut CombatState::default(),
            &mut queue,
            &[AttackerDeclaration {
                creature: host,
                target: AttackTarget::Player(B),
            }],
        )
        .unwrap();
        main(&mut game, A);
        assert!(!casts(&game, A, top).is_empty());
        game.phase_out(host);
        assert!(casts(&game, A, top).is_empty());
        game.phase_in(host);
        game.next_turn();
        game.next_turn();
        main(&mut game, A);
        assert!(casts(&game, A, top).is_empty());
    }
}

#[test]
fn assemble_casts_printed_morph_disguise_and_megamorph_as_two_power_spells_for_three() {
    for definition in definitions("Assemble the Players") {
        for keyword in ["Morph", "Disguise", "Megamorph"] {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let text =
                format!("Mana cost: {{7}}\nType: Creature\nPower/Toughness: 7/7\n{keyword} {{1}}");
            let lower = resource(&mut game, A, &text, Zone::Library);
            let top = resource(&mut game, A, &text, Zone::Library);
            assert!(casts(&game, A, lower).is_empty());
            let actions = casts(&game, A, top);
            assert!(!actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    casting_method: CastingMethod::PlayFrom { .. },
                    ..
                }
            )));
            let action = actions.into_iter().find(|action| matches!(action,
            LegalAction::CastSpell {casting_method: CastingMethod::FaceDownPlayFrom {source, zone: Zone::Library}, ..} if *source == host)).unwrap();
            let mana = game.player(A).unwrap().mana_pool.total();
            let stack = announce(&mut game, A, action);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - 3);
            assert!(game.is_face_down(stack));
            assert_eq!(game.current_power(stack), Some(2));
            assert!(game.object(stack).unwrap().mana_cost.is_none());
            assert_eq!(
                game.cast_origin_snapshot(stack).unwrap().zone,
                Zone::Library
            );
            assert!(
                game.object(stack)
                    .unwrap()
                    .cast_grant_usage_identity
                    .is_some()
            );
            let stable = game.object(stack).unwrap().stable_id;
            resolve_stack_entry(&mut game).unwrap();
            let permanent = game.find_object_by_stable_id(stable).unwrap();
            assert!(game.is_face_down(permanent));
            assert_eq!(game.current_power(permanent), Some(2));
            assert!(
                casts(&game, A, lower).is_empty(),
                "one shared permission budget includes the face-down route"
            );
        }
    }
}
#[test]
fn a_zone_permission_does_not_invent_morph_or_timing_and_uses_public_face_characteristics() {
    for definition in definitions("Assemble the Players") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let plain = resource(
            &mut game,
            A,
            "Mana cost: {7}\nType: Creature\nPower/Toughness: 7/7",
            Zone::Library,
        );
        assert!(casts(&game, A, plain).is_empty());
        game.move_object_by_effect(plain, Zone::Hand).unwrap();
        let morph = resource(
            &mut game,
            A,
            "Mana cost: {7}\nType: Creature\nPower/Toughness: 7/7\nMorph {1}",
            Zone::Library,
        );
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(Step::End);
        assert!(
            casts(&game, A, morph).is_empty(),
            "the zone permission does not grant flash"
        );
        main(&mut game, A);
        game.phase_out(host);
        assert!(casts(&game, A, morph).is_empty());
    }
    for name in ["Elsha of the Infinite", "Crystal Skull, Isu Spyglass"] {
        for definition in definitions(name) {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let morph = resource(
                &mut game,
                A,
                "Mana cost: {7}\nType: Artifact\nMorph {1}",
                Zone::Library,
            );
            assert!(
                !casts(&game, A, morph).iter().any(|action| matches!(
                    action,
                    LegalAction::CastSpell {
                        casting_method: CastingMethod::FaceDownPlayFrom { .. },
                        ..
                    }
                )),
                "the proposed face-down creature is neither a noncreature nor historic: {name}"
            );
        }
    }
}

#[test]
fn belligerent_attack_fixes_beneficiary_and_grants_changing_top_until_cleanup_after_source_leaves()
{
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    for definition in definitions("The Belligerent") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(host);
        resource(
            &mut game,
            A,
            "Type: Creature\nPower/Toughness: 3/3",
            Zone::Battlefield,
        );
        let lower = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 1/1",
            Zone::Library,
        );
        let top = resource(&mut game, A, "Type: Land", Zone::Library);
        let other = resource(
            &mut game,
            B,
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 1/1",
            Zone::Library,
        );
        assert!(casts(&game, A, lower).is_empty());
        assert!(
            !game
                .effect_store
                .grant_registry
                .grants_private_library_top_view(&game, A)
        );
        let action = compute_legal_actions(&game, A)
            .unwrap()
            .into_iter()
            .find(|action| {
                matches!(action,
            LegalAction::ActivateAbility {source, ..} if *source == host)
            })
            .unwrap();
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..20 {
            if !state.has_pending_action() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("pending crew without prompt");
            };
            progress = apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &context, &mut dm,
            )
            .unwrap();
        }
        assert!(!state.has_pending_action());
        resolve_stack_entry(&mut game).unwrap();
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut CombatState::default(),
            &mut queue,
            &[AttackerDeclaration {
                creature: host,
                target: AttackTarget::Player(B),
            }],
        )
        .unwrap();
        put_triggers_on_stack(&mut game, &mut queue).unwrap();
        game.set_current_controller(host, B).unwrap();
        game.move_object_by_effect(host, Zone::Exile).unwrap();
        settle(&mut game);
        assert!(
            game.battlefield
                .iter()
                .any(|id| game.current_has_subtype(*id, ironsmith::types::Subtype::Treasure))
        );
        assert!(
            game.effect_store
                .grant_registry
                .grants_private_library_top_view(&game, A)
        );
        assert!(
            !game
                .effect_store
                .grant_registry
                .grants_private_library_top_view(&game, B)
        );
        main(&mut game, A);
        assert!(casts(&game, A, lower).is_empty());
        perform(
            SpecialAction::PlayLand { card_id: top },
            &mut game,
            A,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert!(!casts(&game, A, lower).is_empty());
        assert!(casts(&game, B, other).is_empty());
        assert!(
            game.effect_store
                .grant_registry
                .grants_private_library_top_view(&game, A)
        );
        game.next_turn();
        assert!(
            !game
                .effect_store
                .grant_registry
                .grants_private_library_top_view(&game, A)
        );
        assert!(casts(&game, A, lower).is_empty());
    }
}

#[test]
fn multiverse_has_one_free_cast_across_hand_and_current_top_and_independent_paid_permission() {
    for definition in definitions("One with the Multiverse") {
        for first_zone in [Zone::Hand, Zone::Library] {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let text = "Mana cost: {7}\nType: Creature\nPower/Toughness: 2/2";
            let hand = resource(&mut game, A, text, Zone::Hand);
            let lower = resource(&mut game, A, text, Zone::Library);
            let top = resource(&mut game, A, text, Zone::Library);
            let (first, next) = if first_zone == Zone::Hand {
                (hand, top)
            } else {
                (top, hand)
            };
            game.player_mut(A).unwrap().mana_pool.empty();
            let free = |action: &LegalAction| {
                matches!(action, LegalAction::CastSpell {
            casting_method: CastingMethod::PlayFrom {source, use_alternative: Some(_), ..}, ..} if *source == host)
            };
            assert!(casts(&game, A, lower).is_empty());
            assert!(casts(&game, A, first).iter().any(&free));
            assert!(casts(&game, A, next).iter().any(&free));
            let action = casts(&game, A, first).into_iter().find(&free).unwrap();
            let stack = announce(&mut game, A, action);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert!(
                game.object(stack)
                    .unwrap()
                    .cast_grant_usage_identity
                    .is_some()
            );
            resolve_stack_entry(&mut game).unwrap();
            assert!(
                !casts(&game, A, next).iter().any(&free),
                "the two origins share one static ability identity"
            );
            assert!(!casts(&game, A, lower).iter().any(&free));
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 20);
            let current = *game.player(A).unwrap().library.last().unwrap();
            assert!(casts(&game, A, current).iter().any(|action| matches!(action, LegalAction::CastSpell {
            casting_method: CastingMethod::PlayFrom {source, use_alternative: None, ..}, ..} if *source == host)),
            "the independent ordinary-cost top permission remains available");
            game.phase_out(host);
            game.phase_in(host);
            assert!(!casts(&game, A, next).iter().any(&free));
            game.next_turn();
            main(&mut game, B);
            assert!(!casts(&game, A, next).iter().any(&free));
            game.next_turn();
            main(&mut game, A);
            assert!(casts(&game, A, next).iter().any(&free));
        }
    }
}
#[test]
fn multiverse_may_save_the_free_cast_by_paying_normally_and_does_not_cover_command_or_other_owners()
{
    for definition in definitions("One with the Multiverse") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let text = "Mana cost: {1}\nType: Creature\nPower/Toughness: 1/1";
        let hand = resource(&mut game, A, text, Zone::Hand);
        let top = resource(&mut game, A, text, Zone::Library);
        let foreign = resource(&mut game, B, text, Zone::Library);
        let command = resource(&mut game, A, text, Zone::Command);
        assert!(
            game.effect_store
                .grant_registry
                .granted_alternative_casts_for_card(&game, command, Zone::Command, A)
                .is_empty()
        );
        assert!(casts(&game, A, foreign).is_empty());
        let ordinary = casts(&game, A, top).into_iter().find(|action| matches!(action, LegalAction::CastSpell {
            casting_method: CastingMethod::PlayFrom {source, use_alternative: None, ..}, ..} if *source == host)).unwrap();
        announce(&mut game, A, ordinary);
        resolve_stack_entry(&mut game).unwrap();
        assert!(casts(&game, A, hand).iter().any(|action| matches!(action, LegalAction::CastSpell {
            casting_method: CastingMethod::PlayFrom {source, use_alternative: Some(_), ..}, ..} if *source == host)));
    }
}

fn foods(game: &GameState) -> usize {
    game.battlefield
        .iter()
        .filter(|id| game.current_has_subtype(**id, ironsmith::types::Subtype::Food))
        .count()
}
#[test]
fn doctor_queues_a_real_food_trigger_after_exact_land_or_spell_permission_and_shares_one_use() {
    for definition in definitions("The Fourth Doctor") {
        for land_first in [true, false] {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let next = resource(
                &mut game,
                A,
                "Mana cost: {0}\nType: Artifact",
                Zone::Library,
            );
            let top = resource(
                &mut game,
                A,
                if land_first {
                    "Type: Legendary Land"
                } else {
                    "Mana cost: {0}\nType: Artifact"
                },
                Zone::Library,
            );
            if land_first {
                perform(
                    SpecialAction::PlayLand { card_id: top },
                    &mut game,
                    A,
                    &mut SelectFirstDecisionMaker,
                )
                .unwrap();
            } else {
                let action = casts(&game, A, top).into_iter().find(|action| matches!(action, LegalAction::CastSpell {
                casting_method: CastingMethod::PlayFrom {source, use_alternative: None, ..}, ..} if *source == host)).unwrap();
                announce(&mut game, A, action);
            }
            assert_eq!(
                foods(&game),
                0,
                "Food is created only when the reflexive trigger resolves"
            );
            assert!(game.turn_store.grant_cast_uses_this_turn.iter().any(|(player, identity)| *player == A && matches!(identity,
            ironsmith::grant_registry::GrantPermissionIdentity::Static {source, ..} if *source == host)));
            let mut queue = TriggerQueue::new();
            put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert!(!game.stack_is_empty(), "the follow-up uses the stack");
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(foods(&game), 1);
            settle(&mut game);
            assert!(casts(&game, A, next).is_empty());
            // A new incarnation is a new permission, independent of the spent one.
            game.move_object_by_effect(host, Zone::Exile).unwrap();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            assert!(!casts(&game, A, next).is_empty());
        }
    }
}
struct ChooseLandPermission {
    doctor: bool,
    suspend: bool,
    waiting: bool,
}
impl ironsmith::decision::DecisionMaker for ChooseLandPermission {
    fn awaiting_choice(&self) -> bool {
        self.waiting
    }
    fn decide_options(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if context.description == "Choose the permission used to play this land" {
            if self.suspend {
                self.waiting = true;
                return Vec::new();
            }
            return vec![
                context
                    .options
                    .iter()
                    .find(|option| option.description.contains("The Fourth Doctor") == self.doctor)
                    .unwrap()
                    .index,
            ];
        }
        ironsmith::decision::DecisionMaker::decide_options(
            &mut SelectFirstDecisionMaker,
            game,
            context,
        )
    }
}
#[test]
fn land_permission_choice_may_save_doctor_budget_and_pending_choice_commits_nothing() {
    for definition in definitions("The Fourth Doctor") {
        for use_doctor in [false, true] {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let unlimited = CardDefinitionBuilder::new(CardId::new(), "Other top permission")
                .card_types(vec![CardType::Enchantment])
                .with_ability(ironsmith::ability::Ability::static_ability(
                    ironsmith::static_abilities::StaticAbility::grants(
                        ironsmith::grant::GrantSpec::new(
                            ironsmith::grant::Grantable::play_from(),
                            ironsmith::target::ObjectFilter::default()
                                .owned_by(ironsmith::target::PlayerFilter::You),
                            Zone::Library,
                        )
                        .with_top_card_only(),
                    ),
                ))
                .build();
            game.create_object_from_definition(&unlimited, A, Zone::Battlefield);
            let next = resource(
                &mut game,
                A,
                "Mana cost: {0}\nType: Artifact",
                Zone::Library,
            );
            let land = resource(&mut game, A, "Type: Legendary Land", Zone::Library);
            let uses_before = game.turn_store.grant_cast_uses_this_turn.clone();
            let mut paused = ChooseLandPermission {
                doctor: use_doctor,
                suspend: true,
                waiting: false,
            };
            perform(
                SpecialAction::PlayLand { card_id: land },
                &mut game,
                A,
                &mut paused,
            )
            .unwrap();
            assert!(paused.waiting);
            assert_eq!(game.object(land).unwrap().zone, Zone::Library);
            assert_eq!(game.player(A).unwrap().lands_played_this_turn, 0);
            assert_eq!(game.turn_store.grant_cast_uses_this_turn, uses_before);
            assert_eq!(foods(&game), 0);
            let mut choose = ChooseLandPermission {
                doctor: use_doctor,
                suspend: false,
                waiting: false,
            };
            perform(
                SpecialAction::PlayLand { card_id: land },
                &mut game,
                A,
                &mut choose,
            )
            .unwrap();
            settle(&mut game);
            assert_eq!(foods(&game), usize::from(use_doctor));
            assert_eq!(
                casts(&game, A, next)
                    .iter()
                    .any(|action| matches!(action, LegalAction::CastSpell {
            casting_method: CastingMethod::PlayFrom {source, ..}, ..} if *source == host)),
                !use_doctor
            );
        }
    }
}
#[test]
fn spell_permission_trigger_keeps_source_lki_when_its_cost_sacrifices_the_doctor() {
    for definition in definitions("The Fourth Doctor") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let top = resource(
            &mut game,
            A,
            "Mana cost: {0}\nType: Artifact\nAs an additional cost to cast this spell, sacrifice a creature.",
            Zone::Library,
        );
        let action = casts(&game, A, top)
            .into_iter()
            .find(|action| {
                matches!(action, LegalAction::CastSpell {
            casting_method: CastingMethod::PlayFrom {source, ..}, ..} if *source == host)
            })
            .unwrap();
        announce(&mut game, A, action);
        assert!(game.object(host).is_none());
        assert_eq!(foods(&game), 0);
        let mut queue = TriggerQueue::new();
        put_triggers_on_stack(&mut game, &mut queue).unwrap();
        resolve_stack_entry(&mut game).unwrap();
        assert_eq!(foods(&game), 1);
    }
}

#[test]
fn multiverse_can_pay_for_a_linked_spell_face_and_keep_its_separate_free_hand_or_top_use() {
    for definition in definitions("One with the Multiverse") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let hand = resource(&mut game, A, "Mana cost: {7}\nType: Artifact", Zone::Hand);
        let next_top = resource(
            &mut game,
            A,
            "Mana cost: {7}\nType: Artifact",
            Zone::Library,
        );
        let front_id = CardId::new();
        let back_id = CardId::new();
        let front = CardDefinitionBuilder::new(front_id, "Paid front")
            .card_types(vec![CardType::Land])
            .other_face(back_id)
            .other_face_name("Paid back")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        let back = CardDefinitionBuilder::new(back_id, "Paid back")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
            .power_toughness(PowerToughness::fixed(2, 2))
            .other_face(front_id)
            .other_face_name("Paid front")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);
        let candidate = game.create_object_from_definition(&front, A, Zone::Library);
        let action = casts(&game, A, candidate).into_iter().find(|action| matches!(action, LegalAction::CastSpell {
            casting_method: CastingMethod::SplitOtherHalfPlayFrom {source, use_alternative: None, ..}, ..} if *source == host)).unwrap();
        let mana_before = game.player(A).unwrap().mana_pool.total();
        let stack = announce(&mut game, A, action);
        assert_eq!(game.object(stack).unwrap().name.as_ref(), "Paid back");
        assert_eq!(game.player(A).unwrap().mana_pool.total(), mana_before - 2);
        resolve_stack_entry(&mut game).unwrap();
        game.player_mut(A).unwrap().mana_pool.empty();
        for candidate in [hand, next_top] {
            assert!(casts(&game, A, candidate).iter().any(|action| matches!(action, LegalAction::CastSpell {
                casting_method: CastingMethod::PlayFrom {source, use_alternative: Some(_), ..}, ..} if *source == host)));
        }
    }
}

#[test]
fn zask_returns_the_dead_insect_to_its_owners_bottom_but_mills_its_trigger_controller() {
    for definition in definitions("Zask, Skittering Swarmlord") {
        for leaves_grave_before_resolution in [false, true] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for _ in 0..4 {
                resource(&mut game, A, "Type: Artifact", Zone::Library);
            }
            resource(&mut game, B, "Type: Artifact", Zone::Library);
            let insect = resource(
                &mut game,
                B,
                "Type: Creature — Insect\nPower/Toughness: 2/2",
                Zone::Battlefield,
            );
            game.set_current_controller(insect, A).unwrap();
            let stable = game.object(insect).unwrap().stable_id;
            let grave = game.move_object_by_effect(insect, Zone::Graveyard).unwrap();
            let mut queue = TriggerQueue::new();
            put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert!(!game.stack_is_empty());
            if leaves_grave_before_resolution {
                game.move_object_by_effect(grave, Zone::Exile).unwrap();
            }
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(A).unwrap().library.len(), 2);
            assert_eq!(game.player(A).unwrap().graveyard.len(), 2);
            assert!(game.player(B).unwrap().graveyard.is_empty());
            let returned = game.find_object_by_stable_id(stable).unwrap();
            if leaves_grave_before_resolution {
                assert_eq!(game.object(returned).unwrap().zone, Zone::Exile);
                assert_eq!(
                    game.player(B).unwrap().library.len(),
                    1,
                    "the moved incarnation cannot be returned, but the independent mill still occurs"
                );
            } else {
                assert_eq!(game.player(B).unwrap().library.first(), Some(&returned));
                assert_eq!(game.player(B).unwrap().library.len(), 2);
            }
        }
    }
}
struct PermissionTarget(ObjectId);
impl ironsmith::decision::DecisionMaker for PermissionTarget {
    fn decide_targets(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        vec![ironsmith::game_state::Target::Object(self.0)]
    }
}
#[test]
fn zask_hybrid_activation_pays_either_color_and_gives_any_target_insect_a_temporary_bonus() {
    for definition in definitions("Zask, Skittering Swarmlord") {
        for color in [ManaSymbol::Black, ManaSymbol::Green] {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let insect = resource(
                &mut game,
                B,
                "Type: Creature — Insect\nPower/Toughness: 2/2",
                Zone::Battlefield,
            );
            game.player_mut(A).unwrap().mana_pool.empty();
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 1);
            game.player_mut(A).unwrap().mana_pool.add(color, 1);
            let action = compute_legal_actions(&game, A)
                .unwrap()
                .into_iter()
                .find(|action| {
                    matches!(action,
            LegalAction::ActivateAbility {source, ..} if *source == host)
                })
                .unwrap();
            let mut state = PriorityLoopState::new(2);
            let mut queue = TriggerQueue::new();
            let mut dm = PermissionTarget(insect);
            let mut progress = apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &PriorityResponse::PriorityAction(action),
                &mut dm,
            )
            .unwrap();
            for _ in 0..20 {
                if !state.has_pending_action() {
                    break;
                }
                let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
                    panic!("pending activation without prompt");
                };
                progress = apply_decision_context_with_dm(
                    &mut game, &mut queue, &mut state, &context, &mut dm,
                )
                .unwrap();
            }
            assert!(!state.has_pending_action());
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.current_power(insect), Some(3));
            assert_eq!(game.current_toughness(insect), Some(2));
            assert!(game.current_has_static_ability_id(insect, StaticAbilityId::Deathtouch));
            ironsmith::execute_cleanup_step(&mut game);
            game.next_turn();
            assert_eq!(game.current_power(insect), Some(2));
            assert!(!game.current_has_static_ability_id(insect, StaticAbilityId::Deathtouch));
        }
    }
}
struct AcceptElementalMill(bool);
impl ironsmith::decision::DecisionMaker for AcceptElementalMill {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.0 && context.can_accept
    }
}
#[test]
fn titania_elemental_death_mill_is_optional_and_uses_the_dead_creatures_controller() {
    for definition in definitions("Titania, Nature's Force") {
        for owner in [A, B] {
            for accept in [false, true] {
                let mut game = game();
                game.create_object_from_definition(&definition, A, Zone::Battlefield);
                for _ in 0..5 {
                    resource(&mut game, A, "Type: Artifact", Zone::Library);
                }
                let elemental = resource(
                    &mut game,
                    owner,
                    "Type: Creature — Elemental\nPower/Toughness: 2/2",
                    Zone::Battlefield,
                );
                game.move_object_by_effect(elemental, Zone::Graveyard)
                    .unwrap();
                let mut queue = TriggerQueue::new();
                put_triggers_on_stack(&mut game, &mut queue).unwrap();
                if owner == A {
                    assert!(!game.stack_is_empty());
                    ironsmith::game_loop::resolve_stack_entry_with(
                        &mut game,
                        &mut AcceptElementalMill(accept),
                    )
                    .unwrap();
                } else {
                    assert!(
                        game.stack_is_empty(),
                        "another player's Elemental does not satisfy the controller filter"
                    );
                }
                assert_eq!(
                    game.player(A).unwrap().library.len(),
                    if owner == A && accept { 2 } else { 5 }
                );
                assert_eq!(
                    game.player(A).unwrap().graveyard.len(),
                    if owner == A {
                        1 + if accept { 3 } else { 0 }
                    } else {
                        0
                    }
                );
            }
        }
    }
}

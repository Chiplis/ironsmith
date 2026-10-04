//! UNVALIDATED source-authored scenarios; no build or execution in this stage.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::{CardDefinition, CardDefinitionBuilder};
use ironsmith::card::{LinkedFaceLayout, PowerToughness};
use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm, put_triggers_on_stack, resolve_stack_entry};
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
fn rows() -> Vec<serde_json::Value> { serde_json::from_str(include_str!("../../../fixtures/filtered_zone_permissions.json.fixture")).unwrap() }
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|r| r["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) { text.push_str(&format!("Power/Toughness: {p}/{t}\n")); }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text()); artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored); [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    main(&mut game, A);
    for player in [A, B] { for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
        game.player_mut(player).unwrap().mana_pool.add(color, 20);
    } } game
}
fn main(game: &mut GameState, player: PlayerId) { game.turn.active_player = player; game.turn.priority_player = Some(player); game.turn.phase = Phase::FirstMain; game.turn.step = None; }
fn resource(game: &mut GameState, player: PlayerId, text: &str, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition("Permission resource", text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn casts(game: &GameState, player: PlayerId, id: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(game, player).unwrap().into_iter().filter(|a| matches!(a, LegalAction::CastSpell {spell_id, ..} if *spell_id == id)).collect()
}
fn announce(game: &mut GameState, player: PlayerId, action: LegalAction) -> ObjectId {
    let LegalAction::CastSpell {spell_id, ..} = &action else { panic!("not a cast"); };
    let stable = game.object(*spell_id).unwrap().stable_id;
    let mut state = PriorityLoopState::new(game.players.len()); let mut queue = TriggerQueue::new(); let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..40 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending without a decision"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    assert!(!state.has_pending_action());
    let id = game.find_object_by_stable_id(stable).unwrap(); assert_eq!(game.object(id).unwrap().zone, Zone::Stack); id
}
fn settle(game: &mut GameState) {
    let mut queue = TriggerQueue::new();
    for _ in 0..20 { put_triggers_on_stack(game, &mut queue).unwrap(); if game.stack_is_empty() { return; } resolve_stack_entry(game).unwrap(); }
    panic!("triggers did not settle");
}
#[test]
fn seven_complete_frozen_bodies_round_trip_without_unsupported_fallbacks() {
    for row in rows() { for definition in definitions(row["name"].as_str().unwrap()) { assert_eq!(definition.card.name, row["name"].as_str().unwrap()); } }
}
#[test]
fn assemble_uses_current_top_power_not_mana_value_and_spends_only_completed_cast() {
    for definition in definitions("Assemble the Players") {
        let mut game = game(); let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let lower = resource(&mut game, A, "Mana cost: {0}\nType: Creature\nPower/Toughness: 2/2", Zone::Library);
        let big = resource(&mut game, A, "Mana cost: {0}\nType: Creature\nPower/Toughness: 3/3", Zone::Library);
        assert!(casts(&game, A, big).is_empty()); assert!(casts(&game, A, lower).is_empty());
        game.move_object_by_effect(big, Zone::Hand).unwrap();
        let action = casts(&game, A, lower).into_iter().next().unwrap();
        let stack = announce(&mut game, A, action);
        assert!(game.object(stack).unwrap().cast_grant_usage_identity.is_some());
        let later = resource(&mut game, A, "Mana cost: {0}\nType: Creature\nPower/Toughness: 1/1", Zone::Library);
        resolve_stack_entry(&mut game).unwrap(); assert!(casts(&game, A, later).is_empty());
        game.phase_out(host); game.phase_in(host); assert!(casts(&game, A, later).is_empty());
        let gone = game.move_object_by_effect(host, Zone::Exile).unwrap();
        game.move_object_by_effect(gone, Zone::Battlefield).unwrap(); assert!(!casts(&game, A, later).is_empty());
    }
}
#[test]
fn library_origins_types_and_live_source_scope_are_authoritative() {
    for (name, cases) in [
        ("Crystal Skull, Isu Spyglass", vec![("Artifact", true), ("Legendary Creature", true), ("Creature", false)]),
        ("Case of the Locked Hothouse", vec![("Enchantment", true), ("Creature", true), ("Artifact", false), ("Sorcery", false)]),
        ("Elsha of the Infinite", vec![("Artifact", true), ("Sorcery", true), ("Creature", false)]),
    ] { for definition in definitions(name) { for (kind, permitted) in &cases {
        let mut game = game(); let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        if name.starts_with("Case") { game.solve_case(host); }
        let text = format!("Mana cost: {{0}}\nType: {kind}\nPower/Toughness: 2/2");
        let own = resource(&mut game, A, &text, Zone::Library);
        let other = resource(&mut game, B, &text, Zone::Library);
        assert_eq!(!casts(&game, A, own).is_empty(), *permitted, "{name}: {kind}");
        assert!(casts(&game, A, other).is_empty());
        game.phase_out(host); assert!(casts(&game, A, own).is_empty()); game.phase_in(host);
        game.set_current_controller(host, B).unwrap(); assert!(casts(&game, A, own).is_empty());
        main(&mut game, B); assert_eq!(!casts(&game, B, other).is_empty(), *permitted);
    } } }
}
#[test]
fn top_land_permission_rejects_deeper_cards_in_direct_special_action_validation() {
    for definition in definitions("Case of the Locked Hothouse") {
        let mut game = game(); let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let deep = resource(&mut game, A, "Type: Land", Zone::Library);
        let top = resource(&mut game, A, "Type: Land", Zone::Library);
        assert!(can_perform_check(&SpecialAction::PlayLand {card_id: top}, &game, A).is_err());
        game.solve_case(host);
        assert!(can_perform_check(&SpecialAction::PlayLand {card_id: deep}, &game, A).is_err());
        assert!(can_perform_check(&SpecialAction::PlayLand {card_id: top}, &game, A).is_ok());
        perform(SpecialAction::PlayLand {card_id: top}, &mut game, A, &mut SelectFirstDecisionMaker).unwrap();
        assert!(can_perform_check(&SpecialAction::PlayLand {card_id: deep}, &game, A).is_ok(), "the unsolved first ability supplies the extra land play");
        perform(SpecialAction::PlayLand {card_id: deep}, &mut game, A, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(A).unwrap().lands_played_this_turn, 2);
    }
}
#[test]
fn elsha_timing_belongs_to_the_selected_library_permission_and_prowess_still_triggers() {
    for definition in definitions("Elsha of the Infinite") {
        let mut game = game(); let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let hand = resource(&mut game, A, "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.", Zone::Hand);
        let top = resource(&mut game, A, "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.", Zone::Library);
        game.turn.phase = Phase::Ending; game.turn.step = Some(Step::End);
        assert!(casts(&game, A, hand).is_empty());
        let action = casts(&game, A, top).into_iter().next().unwrap(); announce(&mut game, A, action);
        settle(&mut game);
        assert_eq!(game.player(A).unwrap().life, 21);
        assert_eq!(game.current_power(host), Some(4), "Prowess observes the actual noncreature cast");
    }
}
fn modal(game: &mut GameState, owner: PlayerId, zone: Zone, front_creature: bool, back_power: i32) -> ObjectId {
    let front_id = CardId::new(); let back_id = CardId::new();
    let front = CardDefinitionBuilder::new(front_id, "Permission front").mana_cost(ManaCost::new())
        .card_types(vec![if front_creature {CardType::Creature} else {CardType::Artifact}])
        .power_toughness(PowerToughness::fixed(2, 2)).other_face(back_id).other_face_name("Permission back")
        .linked_face_layout(LinkedFaceLayout::TransformLike).build();
    let back = CardDefinitionBuilder::new(back_id, "Permission back").mana_cost(ManaCost::new())
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(back_power, 3))
        .other_face(front_id).other_face_name("Permission front").linked_face_layout(LinkedFaceLayout::TransformLike).build();
    game.register_linked_face_definition(&front); game.register_linked_face_definition(&back);
    game.create_object_from_definition(&front, owner, zone)
}
#[test]
fn alternate_spell_face_retains_grant_identity_and_matches_its_own_power() {
    for definition in definitions("Assemble the Players") { for (front_creature, back_power, allowed) in [(false, 2, true), (true, 3, false)] {
        let mut game = game(); let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let candidate = modal(&mut game, A, Zone::Library, front_creature, back_power);
        let alternate = casts(&game, A, candidate).into_iter().find(|a| matches!(a,
            LegalAction::CastSpell {casting_method: CastingMethod::SplitOtherHalfPlayFrom {source, use_alternative: None, ..}, ..} if *source == host));
        assert_eq!(alternate.is_some(), allowed);
        assert_eq!(game.object(candidate).unwrap().name.as_ref(), "Permission front");
        if let Some(action) = alternate {
            let stack = announce(&mut game, A, action);
            assert_eq!(game.object(stack).unwrap().name.as_ref(), "Permission back");
            assert!(game.object(stack).unwrap().cast_grant_usage_identity.is_some());
            resolve_stack_entry(&mut game).unwrap();
            assert!(game.battlefield.iter().any(|id| game.object(*id).is_some_and(|o| o.name.as_ref() == "Permission back")));
        }
    } }
}
#[test]
fn graveyard_permissions_keep_owner_and_land_subtype_or_spell_subtype_scopes() {
    for name in ["Titania, Nature's Force", "Zask, Skittering Swarmlord"] { for definition in definitions(name) {
        let mut game = game(); game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let forest = resource(&mut game, A, "Type: Land — Forest", Zone::Graveyard);
        let island = resource(&mut game, A, "Type: Land — Island", Zone::Graveyard);
        let foreign = resource(&mut game, B, "Type: Land — Forest", Zone::Graveyard);
        assert!(can_perform_check(&SpecialAction::PlayLand {card_id: forest}, &game, A).is_ok());
        assert_eq!(can_perform_check(&SpecialAction::PlayLand {card_id: island}, &game, A).is_ok(), name.starts_with("Zask"));
        assert!(can_perform_check(&SpecialAction::PlayLand {card_id: foreign}, &game, A).is_err());
        let insect = resource(&mut game, A, "Mana cost: {0}\nType: Creature — Insect\nPower/Toughness: 2/2", Zone::Graveyard);
        let soldier = resource(&mut game, A, "Mana cost: {0}\nType: Creature — Soldier\nPower/Toughness: 2/2", Zone::Graveyard);
        assert_eq!(!casts(&game, A, insect).is_empty(), name.starts_with("Zask")); assert!(casts(&game, A, soldier).is_empty());
    } }
}
#[test]
fn assemble_does_not_cast_land_creatures() {
    for definition in definitions("Assemble the Players") {
        let mut game = game(); game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let arbor = resource(&mut game, A, "Type: Land Creature — Forest Dryad\nPower/Toughness: 1/1", Zone::Library);
        assert!(casts(&game, A, arbor).is_empty());
        assert!(can_perform_check(&SpecialAction::PlayLand {card_id: arbor}, &game, A).is_err());
    }
}

#[test]
fn graveyard_forest_permission_enumerates_and_plays_only_the_eligible_land_face() {
    for definition in definitions("Titania, Nature's Force") {
        let mut game = game(); game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let front_id = CardId::new(); let back_id = CardId::new();
        let front = CardDefinitionBuilder::new(front_id, "Island front").card_types(vec![CardType::Land])
            .subtypes(vec![ironsmith::types::Subtype::Island]).other_face(back_id).other_face_name("Forest back")
            .linked_face_layout(LinkedFaceLayout::TransformLike).build();
        let back = CardDefinitionBuilder::new(back_id, "Forest back").card_types(vec![CardType::Land])
            .subtypes(vec![ironsmith::types::Subtype::Forest]).other_face(front_id).other_face_name("Island front")
            .linked_face_layout(LinkedFaceLayout::TransformLike).build();
        game.register_linked_face_definition(&front); game.register_linked_face_definition(&back);
        let id = game.create_object_from_definition(&front, A, Zone::Graveyard);
        let actions = compute_legal_actions(&game, A).unwrap();
        assert!(!actions.iter().any(|action| matches!(action, LegalAction::PlayLand {land_id} if *land_id == id)));
        assert!(actions.iter().any(|action| matches!(action, LegalAction::PlayLandBackFace {land_id} if *land_id == id)));
        perform(SpecialAction::PlayLandBackFace {card_id: id}, &mut game, A, &mut SelectFirstDecisionMaker).unwrap();
        settle(&mut game);
        assert!(game.battlefield.iter().any(|id| game.current_has_subtype(*id, ironsmith::types::Subtype::Elemental)
            && game.current_power(*id) == Some(5)));
    }
}
#[test]
fn lunar_whale_crew_and_actual_attack_enable_the_permission_only_for_that_turn() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    for definition in definitions("The Lunar Whale") {
        let mut game = game(); let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(host);
        resource(&mut game, A, "Type: Creature\nPower/Toughness: 2/2", Zone::Battlefield);
        let top = resource(&mut game, A, "Mana cost: {0}\nType: Artifact", Zone::Library);
        assert!(casts(&game, A, top).is_empty());
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action,
            LegalAction::ActivateAbility {source, ..} if *source == host)).unwrap();
        let mut state = PriorityLoopState::new(2); let mut queue = TriggerQueue::new(); let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..20 { if !state.has_pending_action() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending crew without a decision"); };
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut dm).unwrap();
        }
        assert!(!state.has_pending_action()); resolve_stack_entry(&mut game).unwrap();
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Flying));
        game.turn.phase = Phase::Combat; game.turn.step = Some(Step::DeclareAttackers);
        ironsmith::game_loop::apply_attacker_declarations(&mut game, &mut CombatState::default(), &mut queue,
            &[AttackerDeclaration {creature: host, target: AttackTarget::Player(B)}]).unwrap();
        main(&mut game, A); assert!(!casts(&game, A, top).is_empty());
        game.phase_out(host); assert!(casts(&game, A, top).is_empty()); game.phase_in(host);
        game.next_turn(); game.next_turn(); main(&mut game, A);
        assert!(casts(&game, A, top).is_empty());
    }
}

#[test]
fn assemble_casts_printed_morph_disguise_and_megamorph_as_two_power_spells_for_three() {
    for definition in definitions("Assemble the Players") { for keyword in ["Morph", "Disguise", "Megamorph"] {
        let mut game = game(); let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let text = format!("Mana cost: {{7}}\nType: Creature\nPower/Toughness: 7/7\n{keyword} {{1}}");
        let lower = resource(&mut game, A, &text, Zone::Library);
        let top = resource(&mut game, A, &text, Zone::Library);
        assert!(casts(&game, A, lower).is_empty());
        let actions = casts(&game, A, top);
        assert!(!actions.iter().any(|action| matches!(action, LegalAction::CastSpell {casting_method: CastingMethod::PlayFrom {..}, ..})));
        let action = actions.into_iter().find(|action| matches!(action,
            LegalAction::CastSpell {casting_method: CastingMethod::FaceDownPlayFrom {source, zone: Zone::Library}, ..} if *source == host)).unwrap();
        let mana = game.player(A).unwrap().mana_pool.total();
        let stack = announce(&mut game, A, action);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - 3);
        assert!(game.is_face_down(stack)); assert_eq!(game.current_power(stack), Some(2));
        assert!(game.object(stack).unwrap().mana_cost.is_none());
        assert_eq!(game.cast_origin_snapshot(stack).unwrap().zone, Zone::Library);
        assert!(game.object(stack).unwrap().cast_grant_usage_identity.is_some());
        let stable = game.object(stack).unwrap().stable_id;
        resolve_stack_entry(&mut game).unwrap();
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        assert!(game.is_face_down(permanent)); assert_eq!(game.current_power(permanent), Some(2));
        assert!(casts(&game, A, lower).is_empty(), "one shared permission budget includes the face-down route");
    } }
}
#[test]
fn a_zone_permission_does_not_invent_morph_or_timing_and_uses_public_face_characteristics() {
    for definition in definitions("Assemble the Players") {
        let mut game = game(); let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let plain = resource(&mut game, A, "Mana cost: {7}\nType: Creature\nPower/Toughness: 7/7", Zone::Library);
        assert!(casts(&game, A, plain).is_empty()); game.move_object_by_effect(plain, Zone::Hand).unwrap();
        let morph = resource(&mut game, A, "Mana cost: {7}\nType: Creature\nPower/Toughness: 7/7\nMorph {1}", Zone::Library);
        game.turn.phase = Phase::Ending; game.turn.step = Some(Step::End);
        assert!(casts(&game, A, morph).is_empty(), "the zone permission does not grant flash");
        main(&mut game, A); game.phase_out(host); assert!(casts(&game, A, morph).is_empty());
    }
    for name in ["Elsha of the Infinite", "Crystal Skull, Isu Spyglass"] { for definition in definitions(name) {
        let mut game = game(); game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let morph = resource(&mut game, A, "Mana cost: {7}\nType: Artifact\nMorph {1}", Zone::Library);
        assert!(!casts(&game, A, morph).iter().any(|action| matches!(action,
            LegalAction::CastSpell {casting_method: CastingMethod::FaceDownPlayFrom {..}, ..})),
            "the proposed face-down creature is neither a noncreature nor historic: {name}");
    } }
}

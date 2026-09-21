use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::ids::CardId;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::types::Subtype;
use ironsmith::{Ability, CardType, GameState, ObjectId, PlayerId, Zone};

fn definition() -> ironsmith::cards::CardDefinition {
    let payloads = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Swift Reconfiguration",
    )
    .unwrap();
    ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap()
}

#[test]
fn transformation_preserves_abilities_and_grants_executable_crew() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let original = CardDefinitionBuilder::new(CardId::new(), "Enchanted fixture")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .power_toughness(ironsmith::card::PowerToughness::fixed(3, 4))
        .with_ability(Ability::static_ability(StaticAbility::vigilance()))
        .build();
    let target = game.create_object_from_definition(&original, bob, Zone::Battlefield);
    let aura = game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(aura, alice);
    execute_effect(
        &mut game,
        &ironsmith::effect::Effect::attach_objects(
            ironsmith::target::ChooseSpec::SpecificObject(aura),
            ironsmith::target::ChooseSpec::SpecificObject(target),
        ),
        &mut ctx,
    )
    .unwrap();
    assert_eq!(
        game.object(aura).unwrap().attached_to,
        Some(ironsmith::object::AttachmentTarget::Object(target))
    );
    let chars = game.calculated_characteristics(target).unwrap();
    assert_eq!(chars.card_types, vec![CardType::Artifact]);
    assert!(chars.subtypes.contains(&Subtype::Vehicle));
    assert!(!chars.subtypes.contains(&Subtype::Elf));
    assert!(game.object_has_static_ability_id(target, StaticAbilityId::Vigilance));
    assert_eq!(game.controller_of_id(target), Some(bob));
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(game.object(aura).is_some(), "enchant Vehicle remains legal");
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    assert!(
        crew_action(&game, bob, target).is_none(),
        "crew requires power 5"
    );
    let helper = CardDefinitionBuilder::new(CardId::new(), "Crew fixture")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(5, 5))
        .build();
    let helper = game.create_object_from_definition(&helper, bob, Zone::Battlefield);
    let action = crew_action(&game, bob, target).expect("granted crew is activatable");
    let mut dm = SelectFirstDecisionMaker;
    let mut state = ironsmith::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &ironsmith::game_loop::PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..16 {
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
    assert_eq!(game.stack.len(), 1);
    assert!(game.is_tapped(helper));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let chars = game.calculated_characteristics(target).unwrap();
    assert!(chars.card_types.contains(&CardType::Creature));
    assert!(chars.card_types.contains(&CardType::Artifact));
    assert_eq!((chars.power, chars.toughness), (Some(3), Some(4)));
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert_eq!(
        game.calculated_characteristics(target).unwrap().card_types,
        vec![CardType::Artifact]
    );
    game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
    let restored = game.calculated_characteristics(target).unwrap();
    assert!(restored.card_types.contains(&CardType::Creature));
    assert!(restored.card_types.contains(&CardType::Enchantment));
    assert!(restored.subtypes.contains(&Subtype::Elf));
    assert!(!restored.subtypes.contains(&Subtype::Vehicle));
}

fn crew_action(game: &GameState, player: PlayerId, target: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, player)
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == target))
}

#[test]
fn flash_and_enchant_creature_or_vehicle_keep_target_restrictions() {
    use ironsmith::object::AuraAttachmentFilterRuntimeExt;
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::White, 1);
    let aura = game.create_object_from_definition(&def, alice, Zone::Hand);
    let filter = def
        .aura_attach_filter
        .as_ref()
        .expect("enchant restriction");
    for (types, subtypes, legal) in [
        (vec![CardType::Creature], vec![Subtype::Elf], true),
        (vec![CardType::Artifact], vec![Subtype::Vehicle], true),
        (vec![CardType::Artifact], vec![], false),
        (vec![CardType::Land], vec![], false),
    ] {
        let fixture = CardDefinitionBuilder::new(CardId::new(), "Target fixture")
            .card_types(types)
            .subtypes(subtypes)
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
            .build();
        let target = game.create_object_from_definition(&fixture, bob, Zone::Battlefield);
        let ctx = game.filter_context_for(alice, Some(aura));
        assert_eq!(
            filter.matches_target(
                ironsmith::object::AttachmentTarget::Object(target),
                &ctx,
                &game
            ),
            legal
        );
    }
    assert!(
        compute_legal_actions(&game, alice).iter().any(
            |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == aura)
        ),
        "flash permits opponent-turn casting"
    );
}

#[test]
fn strict_snapshot_has_supported_structure_and_unrounded_score() {
    let payloads = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Swift Reconfiguration",
    )
    .unwrap();
    assert_eq!(payloads.len(), 1);
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payloads[0]);
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
fn subtype_only_attached_transform_preserves_artifact_card_type() {
    let def = ironsmith_tools::parse_card_definition_with_runtime_builder(
        "Subtype transform fixture",
        "Type: Enchantment — Aura\nEnchant creature\nEnchanted creature is a Citizen with base power and toughness 1/1. It has defender and loses all other abilities.",
        false,
    ).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let original = CardDefinitionBuilder::new(CardId::new(), "Artifact creature fixture")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .power_toughness(ironsmith::card::PowerToughness::fixed(3, 4))
        .with_ability(Ability::static_ability(StaticAbility::vigilance()))
        .build();
    let target = game.create_object_from_definition(&original, alice, Zone::Battlefield);
    let aura = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(aura, alice);
    execute_effect(
        &mut game,
        &ironsmith::effect::Effect::attach_objects(
            ironsmith::target::ChooseSpec::SpecificObject(aura),
            ironsmith::target::ChooseSpec::SpecificObject(target),
        ),
        &mut ctx,
    )
    .unwrap();
    let chars = game.calculated_characteristics(target).unwrap();
    assert!(chars.card_types.contains(&CardType::Artifact));
    assert!(chars.card_types.contains(&CardType::Creature));
    assert!(chars.subtypes.contains(&Subtype::Citizen));
    assert!(!chars.subtypes.contains(&Subtype::Elf));
    assert_eq!((chars.power, chars.toughness), (Some(1), Some(1)));
    assert!(game.object_has_static_ability_id(target, StaticAbilityId::Defender));
    assert!(!game.object_has_static_ability_id(target, StaticAbilityId::Vigilance));
}

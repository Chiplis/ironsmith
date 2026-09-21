use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::ids::CardId;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::types::Subtype;
use ironsmith::{Ability, CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(), "Unable to Scream",
    ).unwrap().remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}
fn setup(face_down: bool) -> (GameState, ObjectId, ObjectId) {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let original = CardDefinitionBuilder::new(CardId::new(), "Enchanted fixture")
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Blue]))
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .power_toughness(ironsmith::card::PowerToughness::fixed(4, 5))
        .with_ability(Ability::static_ability(StaticAbility::vigilance())).build();
    let target = game.create_object_from_definition(&original, bob, Zone::Battlefield);
    if face_down { assert!(game.set_face_down(target)); }
    let aura = game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(aura, alice);
    execute_effect(&mut game, &ironsmith::Effect::attach_objects(
        ironsmith::target::ChooseSpec::SpecificObject(aura),
        ironsmith::target::ChooseSpec::SpecificObject(target),
    ), &mut ctx).unwrap();
    game.refresh_continuous_state();
    (game, target, aura)
}
#[test]
fn strict_snapshot_and_full_quality_gate() {
    let s = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(s.parse_status, ironsmith_tools::ParseStatus::StrictCompiled, "{:?}", s.parse_error);
    assert!(!s.parse_lossy && !s.has_unimplemented && s.parse_error.is_none());
    assert!(s.similarity_score >= 0.99, "{}: {:?}", s.similarity_score, s.compiled_text);
}
#[test]
fn aura_adds_types_removes_abilities_and_sets_base_stats_until_it_leaves() {
    let (mut game, target, aura) = setup(false);
    let chars = game.calculated_characteristics(target).unwrap();
    for kind in [CardType::Enchantment, CardType::Creature, CardType::Artifact] {
        assert!(chars.card_types.contains(&kind));
    }
    assert!(chars.subtypes.contains(&Subtype::Elf));
    assert!(chars.subtypes.contains(&Subtype::Toy));
    assert_eq!((chars.power, chars.toughness), (Some(0), Some(2)));
    assert!(!game.object_has_static_ability_id(target, StaticAbilityId::Vigilance));
    assert_eq!(game.controller_of_id(target), Some(PlayerId::from_index(1)));
    game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
    game.refresh_continuous_state();
    let chars = game.calculated_characteristics(target).unwrap();
    assert!(!chars.card_types.contains(&CardType::Artifact));
    assert!(!chars.subtypes.contains(&Subtype::Toy));
    assert_eq!((chars.power, chars.toughness), (Some(4), Some(5)));
    assert!(game.object_has_static_ability_id(target, StaticAbilityId::Vigilance));
}
#[test]
fn face_up_prohibition_applies_only_while_attached_source_remains() {
    let (mut game, target, aura) = setup(true);
    assert!(!game.can_turn_face_up_permanent(target));
    assert!(!game.set_face_up(target));
    assert!(game.is_face_down(target));
    game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
    game.refresh_continuous_state();
    assert!(game.can_turn_face_up_permanent(target));
    assert!(game.set_face_up(target));
    assert!(!game.is_face_down(target));
}

#[test]
fn generic_filtered_face_up_prohibition_blocks_mutation_and_expires_with_source() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Face-down fixture")
        .card_types(vec![CardType::Creature]).build();
    let yours = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let theirs = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    game.set_face_down(yours);
    game.set_face_down(theirs);
    let rule = CardDefinitionBuilder::new(CardId::new(), "Filtered restriction fixture")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::restriction(
            ironsmith::effect::Restriction::turn_face_up(
                ironsmith::filter::ObjectFilter::creature().you_control(),
            ), "irrelevant display text".into(),
        ))).build();
    let source = game.create_object_from_definition(&rule, alice, Zone::Battlefield);
    game.refresh_continuous_state();
    assert!(!game.can_turn_face_up_permanent(yours));
    assert!(game.can_turn_face_up_permanent(theirs));
    assert!(!game.set_face_up(yours));
    assert!(game.set_face_up(theirs));
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    assert!(game.set_face_up(yours));
}

#[test]
fn manifested_face_up_special_action_is_unavailable_until_aura_leaves() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    let (mut game, target, aura) = setup(true);
    let bob = PlayerId::from_index(1);
    game.set_manifested(target);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(bob).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 1);
    game.refresh_continuous_state();
    let available = |game: &GameState| compute_legal_actions(game, bob).iter().any(|action|
        matches!(action, LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == target));
    assert!(!available(&game));
    game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
    game.refresh_continuous_state();
    assert!(available(&game));
}

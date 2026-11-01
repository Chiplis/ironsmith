//! Revolution reminders expressed only with existing MTG operations.
use ironsmith_compiler_runtime::compile_builder_to_runtime_definition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith::types::{CardType, Subtype, Supertype};
fn definition(name: &str, text: &str, identity: &str) -> ironsmith::cards::CardDefinition {
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), name).with_custom_keyword(identity);
    ironsmith_compiler::word_claims::with_enforcement_options(true, true, || compile_builder_to_runtime_definition(builder, text, false)).unwrap()
}
fn move_by_effect(game: &mut GameState, source: ObjectId, destination: Zone) -> ObjectId {
    let a = PlayerId::from_index(0); let stable = game.object(source).unwrap().stable_id;
    let effect = ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source), destination, false));
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
    let result = ironsmith::effects::execute_effect(game, &effect, &mut ctx).unwrap();
    for event in result.events { game.queue_trigger_event(Default::default(), event); }
    game.find_object_by_stable_id(stable).unwrap()
}
#[test]
fn revolution_master_searches_only_matching_cards_and_can_fail_to_find() {
    struct SearchChoice(bool);
    impl DecisionMaker for SearchChoice {
        fn decide_objects(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            if self.0 { Vec::new() } else { SelectFirstDecisionMaker.decide_objects(game, ctx) }
        }
    }
    for matching in [false, true] { for decline in [false, true] {
        let master = definition("Library mentor", "Type: Creature\nPower/Toughness: 2/2\nWhen this permanent enters, search your library for a card named Frost Breath, reveal it, put it into your hand, then shuffle.", "Master");
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let other = game.create_object_from_definition(&definition("Other lesson", "Type: Sorcery\nDraw a card.", "Lesson"), a, Zone::Library);
        if matching { game.create_object_from_definition(&definition("Frost Breath", "Type: Sorcery\nDraw a card.", "Lesson"), a, Zone::Library); }
        let hand = game.create_object_from_definition(&master, a, Zone::Hand);
        move_by_effect(&mut game, hand, Zone::Battlefield);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SearchChoice(decline)).unwrap();
        assert_eq!(game.player(a).unwrap().hand.len(), usize::from(matching && !decline), "matching={matching} decline={decline}");
        if matching && !decline { assert_eq!(game.object(game.player(a).unwrap().hand[0]).unwrap().name, "Frost Breath"); }
        assert!(game.player(a).unwrap().library.contains(&other));
    } }
}
#[test]
fn revolution_dare_animation_adds_types_and_expires_at_cleanup() {
    let spell = definition("Rebellion", "Type: Sorcery\nTarget artifact becomes a legendary Rebel creature with base power and toughness 5/5 in addition to its other types until end of turn.", "Dare");
    let artifact = definition("Relic", "Type: Artifact", "Relic");
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let a = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&spell, a, Zone::Stack);
    let target = game.create_object_from_definition(&artifact, a, Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, a).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(target)]);
    for effect in spell.spell_effect.as_ref().unwrap().all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    let characteristics = game.current_characteristics(target).unwrap();
    assert!(characteristics.card_types.contains(&CardType::Artifact));
    assert!(characteristics.card_types.contains(&CardType::Creature));
    assert!(characteristics.supertypes.contains(&Supertype::Legendary));
    assert!(characteristics.subtypes.contains(&Subtype::Rebel));
    assert_eq!(game.calculated_power(target), Some(5));
    assert_eq!(game.calculated_toughness(target), Some(5));
    ironsmith::turn::execute_cleanup_step(&mut game);
    let characteristics = game.current_characteristics(target).unwrap();
    assert!(characteristics.card_types.contains(&CardType::Artifact));
    assert!(!characteristics.card_types.contains(&CardType::Creature));
    assert!(!characteristics.supertypes.contains(&Supertype::Legendary));
    assert_eq!(game.calculated_power(target), None);
}
#[test]
fn revolution_reflected_replaces_nonexile_battlefield_departures() {
    for destination in [Zone::Hand, Zone::Graveyard, Zone::Exile] { for replacement in [false, true] {
        let permanent = definition("Reflected mirror", &format!("Type: Artifact\n{}", if replacement { "If this permanent would leave the battlefield, exile it instead." } else { "" }), "Reflected");
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&permanent, a, Zone::Battlefield);
        let moved = move_by_effect(&mut game, source, destination);
        assert_eq!(game.object(moved).unwrap().zone, if replacement { Zone::Exile } else { destination });
    } }
}

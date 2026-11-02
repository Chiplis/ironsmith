//! Source-relative custom reminder programs; parent runs native verification.
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith_compiler_runtime::compile_to_runtime_definition;
const A: PlayerId = PlayerId::from_index(0);
fn definition(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    ironsmith_compiler::word_claims::with_enforcement_options(true, true, || compile_to_runtime_definition(name, text, false)).unwrap_or_else(|error| panic!("{name}: {error}"))
}
fn enter(game: &mut GameState, source: ObjectId) {
    let effect = ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source), Zone::Battlefield, false));
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, A);
    let result = ironsmith::effects::execute_effect(game, &effect, &mut ctx).unwrap();
    for event in result.events { game.queue_trigger_event(Default::default(), event); }
}
#[test]
fn motivate_targets_only_strictly_lower_power_and_payment_is_optional() {
    struct Choice(bool);
    impl DecisionMaker for Choice {
        fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.0 }
    }
    for accept in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()],20);
        let small = definition("Small", "Type: Creature\nPower/Toughness: 2/2");
        let equal = definition("Equal", "Type: Creature\nPower/Toughness: 3/3");
        let large = definition("Large", "Type: Creature\nPower/Toughness: 4/4");
        // Put ineligible objects first to ensure the chooser respects the filter.
        let high = game.create_object_from_definition(&large,A,Zone::Battlefield);
        let same = game.create_object_from_definition(&equal,A,Zone::Battlefield);
        let low = game.create_object_from_definition(&small,A,Zone::Battlefield);
        let source = definition("Motivator", "Type: Creature\nPower/Toughness: 3/2\nWhen this creature enters, you may put 2 +1/+1 counters on target creature with power less than this creature's power.");
        let hand = game.create_object_from_definition(&source,A,Zone::Hand);
        enter(&mut game,hand);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(queue.entries.len(),1);
        ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut Choice(accept)).unwrap();
        assert_eq!(game.calculated_power(low),Some(if accept {4} else {2}));
        assert_eq!(game.calculated_power(same),Some(3));
        assert_eq!(game.calculated_power(high),Some(4));
    }
}
#[test]
fn erratic_reader_keeps_reveal_and_strict_source_mana_value() {
    let card = definition("Erratic reader", "Mana cost: {4}{G}\nType: Creature\nPower/Toughness: 3/3\nWhen this creature dies, reveal the top four cards of your library. Put a creature card with mana value less than this creature from among them onto the battlefield. Put the rest on the bottom of your library in any order.");
    let debug = format!("{card:#?}");
    assert!(debug.contains("LessThanExpr"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
}
#[test]
fn erratic_death_selects_lower_creature_and_preserves_all_other_cards() {
    for eligible in [false,true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()],20);
        let candidate = definition("Candidate", &format!("Mana cost: {{{}}}\nType: Creature\nPower/Toughness: 1/1", if eligible {4} else {5}));
        let same = definition("Equal value", "Mana cost: {5}\nType: Creature\nPower/Toughness: 1/1");
        let noncreature = definition("Noncreature", "Mana cost: {1}\nType: Artifact");
        let ids = [
            game.create_object_from_definition(&candidate,A,Zone::Library),
            game.create_object_from_definition(&same,A,Zone::Library),
            game.create_object_from_definition(&noncreature,A,Zone::Library),
            game.create_object_from_definition(&noncreature,A,Zone::Library),
        ];
        let stable = ids.map(|id| game.object(id).unwrap().stable_id);
        let erratic = definition("Erratic creature", "Mana cost: {5}\nType: Creature\nPower/Toughness: 3/3\nWhen this creature dies, reveal the top four cards of your library. Put a creature card with mana value less than this creature from among them onto the battlefield. Put the rest on the bottom of your library in any order.");
        let source = game.create_object_from_definition(&erratic,A,Zone::Battlefield);
        let effect = ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source), Zone::Graveyard, false));
        let mut ctx = ironsmith::effects::EffectContext::new_default(source,A);
        let result = ironsmith::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();
        for event in result.events {game.queue_trigger_event(Default::default(),event);}
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(queue.entries.len(),1);
        ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
        for (index,identity) in stable.into_iter().enumerate() {
            let id = game.find_object_by_stable_id(identity).expect("every revealed card retained");
            assert_eq!(game.object(id).unwrap().zone,if eligible && index==0 {Zone::Battlefield} else {Zone::Library});
        }
    }
}
#[test]
fn alpha_entry_choice_preserves_counter_and_status_as_independent_facts() {
    let alpha = definition("Alpha creature", "Type: Creature\nPower/Toughness: 3/1\nAs this creature enters, you may put a +1/+1 counter on it. If you do, give this creature the alpha status.\nAs long as this creature has the alpha status, this creature can only attack alone.");
    let mut game = GameState::new(vec!["A".into(), "B".into()],20);
    let hand = game.create_object_from_definition(&alpha,A,Zone::Hand);
    let stable = game.object(hand).unwrap().stable_id;
    enter(&mut game,hand);
    let id=game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.calculated_power(id),Some(4));
    assert!(game.has_custom_status(id,"alpha"),"actual statuses={:?}; compiled definition={alpha:#?}",game.custom_status_names(id));
    game.object_mut(id).unwrap().remove_counters(ironsmith::object::CounterType::PlusOnePlusOne,1);
    assert!(game.has_custom_status(id,"alpha"),"losing the counter must not clear the designation");
    // Entry-time status must already be observable to downstream rules.
    let buddy = game.create_object_from_definition(&definition("Buddy", "Type: Creature\nPower/Toughness: 1/1"), A, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    game.remove_summoning_sickness(buddy);
    game.turn.active_player=A;
    game.turn.phase=ironsmith::game_state::Phase::Combat;
    game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);
    game.refresh_continuous_state().unwrap();
    let declarations=[id,buddy].map(|creature| ironsmith::decision::AttackerDeclaration {creature,target:ironsmith::combat_state::AttackTarget::Player(PlayerId::from_index(1))});
    assert!(ironsmith::game_loop::apply_attacker_declarations(&mut game,&mut ironsmith::combat_state::CombatState::default(),&mut ironsmith::triggers::TriggerQueue::new(),&declarations).is_err());
}

#[test]
fn alpha_designation_is_not_in_copiable_definition_and_does_not_survive_zone_exit() {
    let alpha = definition("Entry designation", "Type: Creature\nPower/Toughness: 3/1\nAs this creature enters, you may put a +1/+1 counter on it. If you do, give this creature the alpha status.\nAs long as this creature has the alpha status, this creature can only attack alone.");
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    let original=game.create_object_from_definition(&alpha,A,Zone::Hand);
    let stable=game.object(original).unwrap().stable_id;
    enter(&mut game,original);
    let entered=game.find_object_by_stable_id(stable).unwrap();
    assert!(game.has_custom_status(entered,"alpha"));
    // Constructing another object from the same copiable definition does not
    // copy the original incarnation's noncopiable designation.
    let copy=game.create_object_from_definition(&alpha,A,Zone::Battlefield);
    assert!(!game.has_custom_status(copy,"alpha"));
    let hand=game.move_object_by_effect(entered,Zone::Hand).unwrap();
    assert!(!game.has_custom_status(hand,"alpha"));
}

#[test]
fn declining_alpha_entry_choice_adds_neither_counter_nor_status() {
    struct Decline;
    impl DecisionMaker for Decline {
        fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{false}
    }
    let alpha=definition("Optional entry designation","Type: Creature\nPower/Toughness: 3/1\nAs this creature enters, you may put a +1/+1 counter on it. If you do, give this creature the alpha status.");
    let mut g=GameState::new(vec!["A".into(),"B".into()],20);
    let source=g.create_object_from_definition(&alpha,A,Zone::Hand);let stable=g.object(source).unwrap().stable_id;
    let result=g.move_object_with_etb_processing_with_dm(source,Zone::Battlefield,&mut Decline).unwrap();
    assert!(!result.pending);
    let entered=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.calculated_power(entered),Some(3));assert!(!g.has_custom_status(entered,"alpha"));
}
#[test]
fn alpha_entry_status_is_visible_to_later_entry_replacement_queries() {
    let alpha=definition("Optional entry designation","Type: Creature\nPower/Toughness: 3/1\nAs this creature enters, you may put a +1/+1 counter on it. If you do, give this creature the alpha status.");
    let observer=definition("Entry designation observer","Type: Enchantment\nCreatures with the alpha status enter with an additional +1/+1 counter on them.");
    let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.create_object_from_definition(&observer,A,Zone::Battlefield);
    let source=g.create_object_from_definition(&alpha,A,Zone::Hand);let stable=g.object(source).unwrap().stable_id;enter(&mut g,source);
    let entered=g.find_object_by_stable_id(stable).unwrap();assert!(g.has_custom_status(entered,"alpha"));assert_eq!(g.calculated_power(entered),Some(5));
}
#[test]
fn paused_later_entry_choice_does_not_leak_designation_and_replay_commits_once() {
    struct PauseColor {pause:bool,pending:bool}
    impl DecisionMaker for PauseColor {
        fn awaiting_choice(&self)->bool{self.pending}
        fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{true}
        fn decide_colors(&mut self,_:&GameState,_:&ironsmith::decisions::context::ColorsContext)->Vec<ironsmith::Color>{self.pending=self.pause;vec![ironsmith::Color::Blue]}
    }
    let alpha=definition("Optional entry designation","Type: Creature\nPower/Toughness: 3/1\nAs this creature enters, you may put a +1/+1 counter on it. If you do, give this creature the alpha status.\nAs this creature enters, choose a color.");
    let mut g=GameState::new(vec!["A".into(),"B".into()],20);let source=g.create_object_from_definition(&alpha,A,Zone::Hand);let stable=g.object(source).unwrap().stable_id;
    let mut dm=PauseColor{pause:true,pending:false};let result=g.move_object_with_etb_processing_with_dm(source,Zone::Battlefield,&mut dm).unwrap();assert!(result.pending);
    assert_eq!(g.object(source).unwrap().zone,Zone::Hand);assert!(!g.has_custom_status(source,"alpha"));assert_eq!(g.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne),0);
    dm.pause=false;dm.pending=false;let resumed=g.move_object_with_etb_processing_with_dm(source,Zone::Battlefield,&mut dm).unwrap();assert!(!resumed.pending);
    let entered=g.find_object_by_stable_id(stable).unwrap();assert!(g.has_custom_status(entered,"alpha"));assert_eq!(g.calculated_power(entered),Some(4));assert_eq!(g.chosen_color(entered),Some(ironsmith::Color::Blue));
}

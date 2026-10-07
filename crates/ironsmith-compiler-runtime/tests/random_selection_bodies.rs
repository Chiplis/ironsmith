//! Source-authored regressions; execution is deferred by the campaign policy.
//! The eight exact frozen bodies remain in the fixture. Five are explicitly
//! held in architecture/card-failure-random-selection-bodies.md.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{SelectObjectsContext, TargetsContext, ViewCardsContext};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Phase, StackEntry};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/random_selection_bodies.json.fixture"
    )).unwrap();
    assert_eq!(rows.len(), 8);
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text += &format!("Power/Toughness: {p}/{t}\n");
    }
    text += row["oracle_text"].as_str().unwrap();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text.clone(), false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name} direct: {error}"));
    assert!(!direct_loss.is_lossy(), "{name} direct: {}", direct_loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    for definition in [&direct, &restored] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
        let rendered = ironsmith_text::canonical_compiled_lines(definition).join("\n");
        assert!(rendered.contains("at random"), "{name}: {rendered}");
    }
    [direct, restored]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 10);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 10);
    game
}
fn creature(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, subtype: &str, mana: u32) -> ObjectId {
    let definition = compile_to_runtime_definition(name, format!(
        "Mana cost: {{{mana}}}\nType: Creature — {subtype}\nPower/Toughness: 2/2"
    ), false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
#[derive(Default)]
struct Choices {
    sacrifice: Option<ObjectId>,
    viewed: Vec<(PlayerId, Vec<ObjectId>)>,
}
impl DecisionMaker for Choices {
    fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        let sacrifice = self.sacrifice.take().expect("random resolution must not request an object choice");
        assert!(context.candidates.iter().any(|candidate| candidate.id == sacrifice && candidate.legal));
        vec![sacrifice]
    }
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        assert!(context.requirements.iter().any(|requirement| requirement.legal_targets.contains(&Target::Player(B))));
        vec![Target::Player(B)]
    }
    fn view_cards(&mut self, _: &GameState, viewer: PlayerId, cards: &[ObjectId], context: &ViewCardsContext) {
        assert!(context.public);
        assert_eq!(context.subject, B);
        assert_eq!(context.zone, Zone::Hand);
        self.viewed.push((viewer, cards.to_vec()));
    }
}
fn settle(game: &mut GameState, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    for _ in 0..32 {
        if game.stack.is_empty() { return; }
        resolve_stack_entry_with(game, dm).unwrap();
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    }
    panic!("stack failed to settle");
}
fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility { source, ability_index }), dm).unwrap();
    for _ in 0..64 {
        if state.pending_activation.is_none() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert!(game.is_tapped(source));
    assert!(game.stack.last().unwrap().targets.is_empty(), "the graveyard choice happens during resolution");
}
#[test]
fn three_complete_bodies_keep_random_semantics_through_artifact_transport() {
    for name in ["Moldgraf Monstrosity", "Tomb Tyrant", "Singe-Mind Ogre"] { definitions(name); }
}
#[test]
fn moldgraf_exiles_itself_then_samples_distinct_owned_creatures_and_does_as_much_as_possible() {
    for definition in definitions("Moldgraf Monstrosity") {
        for count in [0, 1, 2, 4] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let stable = game.object(source).unwrap().stable_id;
            let mut eligible = Vec::new();
            for index in 0..count {
                let id = creature(&mut game, A, Zone::Graveyard, &format!("Eligible {index}"), "Bear", 2);
                eligible.push(game.object(id).unwrap().stable_id);
            }
            let foreign = creature(&mut game, B, Zone::Graveyard, "Foreign", "Bear", 2);
            let noncreature = compile_to_runtime_definition("Noncreature", "Type: Artifact", false).unwrap();
            let noncreature = game.create_object_from_definition(&noncreature, A, Zone::Graveyard);
            game.take_pending_trigger_events();
            let mut dm = Choices::default();
            let mut context = EffectContext::new(source, A, &mut dm);
            let outcome = execute_effect(&mut game, &Effect::destroy(ChooseSpec::SpecificObject(source)), &mut context).unwrap();
            drop(context);
            for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
            settle(&mut game, &mut dm);
            let current_source = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(current_source).unwrap().zone, Zone::Exile);
            let returned = eligible.iter().filter(|stable| {
                game.find_object_by_stable_id(**stable).is_some_and(|id| game.object(id).unwrap().zone == Zone::Battlefield)
            }).count();
            assert_eq!(returned, count.min(2));
            assert_eq!(game.battlefield.len(), returned);
            assert_eq!(game.object(foreign).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.object(noncreature).unwrap().zone, Zone::Graveyard);
        }
    }
}
#[test]
fn tomb_tyrant_keeps_anthem_activation_restrictions_costs_and_resolution_pool() {
    for definition in definitions("Tomb Tyrant") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let sacrifice = creature(&mut game, A, Zone::Battlefield, "Sacrifice", "Zombie", 2);
        let foreign = creature(&mut game, B, Zone::Battlefield, "Foreign", "Zombie", 2);
        assert_eq!(game.current_power(source), Some(4));
        assert_eq!(game.current_power(sacrifice), Some(3));
        assert_eq!(game.current_power(foreign), Some(2));
        let ability_index = definition.abilities.iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
        let available = |game: &GameState| compute_legal_actions(game, A).unwrap().iter().any(|action| matches!(action, LegalAction::ActivateAbility { source: id, ability_index: index } if *id == source && *index == ability_index));
        for index in 0..2 { creature(&mut game, A, Zone::Graveyard, &format!("Grave {index}"), "Zombie", 2); }
        assert!(!available(&game));
        creature(&mut game, A, Zone::Graveyard, "Third", "Zombie", 2);
        assert!(available(&game));
        game.turn.active_player = B;
        assert!(!available(&game));
        game.turn.active_player = A;
        let mana_before = game.player(A).unwrap().mana_pool.total();
        let mut dm = Choices { sacrifice: Some(sacrifice), ..Default::default() };
        activate(&mut game, source, ability_index, &mut dm);
        assert!(game.object(sacrifice).is_none());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), mana_before - 3);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 4);
        let before_random = game.irreversible_random_count();
        settle(&mut game, &mut dm);
        assert_eq!(game.irreversible_random_count(), before_random + 1);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 3);
        let own_creatures: Vec<_> = game.battlefield.iter().copied().filter(|id| game.controller_of(game.object(*id).unwrap()) == A).collect();
        assert_eq!(own_creatures.len(), 2);
        let returned = own_creatures.into_iter().find(|id| *id != source).unwrap();
        assert_eq!(game.current_power(returned), Some(3));
    }
}
#[test]
fn ogre_reveals_exact_random_card_before_life_loss_and_keeps_empty_hand_safe() {
    for definition in definitions("Singe-Mind Ogre") {
        for count in [0, 1, 3] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            let mut candidates = Vec::new();
            for index in 0..count { candidates.push(creature(&mut game, B, Zone::Hand, &format!("Hand {index}"), "Bear", index as u32 + 1)); }
            game.take_pending_trigger_events();
            let mut dm = Choices::default();
            let random_before = game.irreversible_random_count();
            let mut context = EffectContext::new(source, A, &mut dm);
            let entered = execute_effect(&mut game,
                &Effect::move_to_zone(ChooseSpec::SpecificObject(source), Zone::Battlefield, false),
                &mut context).unwrap();
            drop(context);
            for event in entered.events { game.queue_trigger_event(Default::default(), event); }
            settle(&mut game, &mut dm);
            assert_eq!(game.player(B).unwrap().hand.len(), count);
            if count == 0 {
                assert!(dm.viewed.is_empty());
                assert_eq!(game.player(B).unwrap().life, 20);
                assert_eq!(game.irreversible_random_count(), random_before);
            } else {
                let revealed = dm.viewed[0].1[0];
                assert!(candidates.contains(&revealed));
                assert!(dm.viewed.iter().all(|(_, ids)| ids == &[revealed]));
                assert!(dm.viewed.iter().any(|(viewer, _)| *viewer == A));
                assert!(dm.viewed.iter().any(|(viewer, _)| *viewer == B));
                let value = game.object(revealed).unwrap().mana_cost.as_ref().unwrap().mana_value() as i32;
                assert_eq!(game.player(B).unwrap().life, 20 - value);
                assert_eq!(game.irreversible_random_count(), random_before + 1);
            }
        }
    }
}
#[test]
fn transcript_seed_owns_random_return_and_sampling_is_without_replacement() {
    for definition in definitions("Tomb Tyrant") {
        let ability = definition.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(ability) => Some(ability), _ => None }).unwrap();
        let mut results = Vec::new();
        for local_seed in [1, 999] {
            let mut game = game();
            game.set_random_seed(local_seed);
            game.queue_transcript_random_seeds([12345]);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for index in 0..5 { creature(&mut game, A, Zone::Graveyard, &format!("Zombie {index}"), "Zombie", 2); }
            game.push_to_stack(StackEntry::ability(source, A, ability.effects.clone()));
            settle(&mut game, &mut Choices::default());
            let returned = game.battlefield.iter().find(|id| **id != source).unwrap();
            results.push(game.object(*returned).unwrap().name.to_string());
            assert_eq!(game.irreversible_random_count(), 1);
        }
        assert_eq!(results[0], results[1], "verified transcript authority must override local random state");
    }
}
#[test]
fn compiled_random_return_rolls_back_whole_program_when_entry_suspends() {
    struct EntryChoice { pause: bool, pending: bool }
    impl DecisionMaker for EntryChoice {
        fn decide_objects(&mut self, _: &GameState, _: &SelectObjectsContext) -> Vec<ObjectId> {
            panic!("the player does not choose the random graveyard card");
        }
        fn decide_colors(&mut self, _: &GameState, _: &ironsmith::decisions::context::ColorsContext) -> Vec<ironsmith::Color> {
            self.pending = self.pause;
            vec![ironsmith::Color::Blue]
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    for definition in definitions("Tomb Tyrant") {
        let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(ability), _ => None,
        }).unwrap();
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut grave = Vec::new();
        for index in 0..3 {
            let id = creature(&mut game, A, Zone::Graveyard, &format!("Color Zombie {index}"), "Zombie", 2);
            game.object_mut(id).unwrap().abilities_mut().push(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::choose_color_as_enters(None, "As this enters, choose a color.".into()),
            ));
            grave.push(id);
        }
        game.queue_transcript_random_seeds([13579]);
        game.take_pending_trigger_events();
        game.push_to_stack(StackEntry::ability(source, A, ability.effects.clone()));
        let before_seed = game.random_seed();
        let before_count = game.irreversible_random_count();
        let before_ids = game.next_object_id_counter();
        let mut dm = EntryChoice { pause: true, pending: false };
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(dm.pending);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.battlefield, vec![source]);
        assert!(grave.iter().all(|id| game.object(*id).is_some_and(|object| object.zone == Zone::Graveyard)));
        assert_eq!(game.random_seed(), before_seed);
        assert_eq!(game.irreversible_random_count(), before_count);
        assert_eq!(game.next_object_id_counter(), before_ids);
        dm.pause = false;
        dm.pending = false;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.stack.is_empty());
        assert_eq!(game.player(A).unwrap().graveyard.len(), 2);
        assert_eq!(game.battlefield.len(), 2);
        let returned = game.battlefield.iter().find(|id| **id != source).copied().unwrap();
        assert_eq!(game.chosen_color(returned), Some(ironsmith::Color::Blue));
        assert_eq!(game.random_seed(), 13579);
        assert_eq!(game.irreversible_random_count(), before_count + 1);
    }
}

#[path = "random_selection_bodies/kheru_riders.rs"]
mod kheru_riders;
#[path = "random_selection_bodies/sinister_partition.rs"]
mod sinister_partition;
#[path = "random_selection_bodies/ogre_hidden.rs"]
mod ogre_hidden;
#[path = "random_selection_bodies/nebuchadnezzar.rs"]
mod nebuchadnezzar;

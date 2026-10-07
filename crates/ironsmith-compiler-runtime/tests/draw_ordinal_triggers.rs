//! Exact frozen bodies and public draw/cast/equip execution, through both direct
//! compilation and artifact transport. Authored only; no tests have been run.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{AttachmentTarget, CounterType, ObjectKind};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{Effect, GameProgress, GameState, ObjectId, Phase, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::TriggerKind;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn source(name: &str) -> String {
    if name == "Astrologian's Planisphere" {
        let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../fixtures/job_select_materialization.json.fixture")).unwrap();
        let row = rows.iter().find(|row| row["name"] == name).unwrap();
        assert_eq!(row["oracle_id"], "4dcd0583-cee0-46a6-ba43-93bca5eb0428");
        row["text"].as_str().unwrap().to_string()
    } else {
        assert_eq!(name, "Madame Masque");
        let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../fixtures/lossy_metadata.json.fixture")).unwrap();
        let row = rows.iter().find(|row| row["name"] == name).unwrap();
        format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
            row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
            row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
            row["oracle_text"].as_str().unwrap())
    }
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let text = source(name);
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let definitions = [direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()];
    for definition in &definitions {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    definitions
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    for symbol in [ManaSymbol::Blue, ManaSymbol::Black] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 20);
    }
    for owner in [A, B] {
        let card = compile_to_runtime_definition("Draw witness", "Mana cost: {0}\nType: Artifact", false).unwrap();
        for _ in 0..12 { game.create_object_from_definition(&card, owner, Zone::Library); }
    }
    game
}

#[derive(Default)]
struct Choices { target: Option<Target> }
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert_eq!(context.requirements.len(), 1);
            assert!(context.requirements[0].legal_targets.contains(&target));
            vec![target]
        } else { SelectFirstDecisionMaker.decide_targets(game, context) }
    }
}
fn queue(game: &mut GameState, choices: &mut Choices) {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), choices).unwrap();
}
fn settle(game: &mut GameState, choices: &mut Choices) {
    for _ in 0..16 {
        queue(game, choices);
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("bounded ordinal scenario did not settle");
}
fn action(game: &mut GameState, action: LegalAction, choices: &mut Choices) {
    game.turn.priority_player = Some(A);
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut state = PriorityLoopState::new(game.players.len());
    let mut triggers = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut triggers, &mut state,
        &PriorityResponse::PriorityAction(action), choices).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none()
            && state.pending_method_selection.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut triggers, &mut state, &context, choices).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none()
        && state.pending_method_selection.is_none());
    put_triggers_on_stack_with_dm(game, &mut triggers, choices).unwrap();
}
fn cast(game: &mut GameState, definition: &CardDefinition, choices: &mut Choices) {
    let spell_id = game.create_object_from_definition(definition, A, Zone::Hand);
    action(game, LegalAction::CastSpell { spell_id, from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal }, choices);
}
fn battlefield_named(game: &GameState, name: &str) -> ObjectId {
    *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == name).unwrap()
}
fn draw(game: &mut GameState, source: ObjectId, player: PlayerId, count: i32) {
    let outcome = execute_effect(game, &Effect::draw(count),
        &mut EffectContext::new(source, player, &mut SelectFirstDecisionMaker)).unwrap();
    for event in outcome.events { game.queue_trigger_event(event.provenance(), event); }
}
fn contains_ordinal(trigger: &ironsmith_core::trigger_model::Trigger, number: u32) -> bool {
    match &trigger.kind {
        TriggerKind::PlayerDrawsNthCardEachTurn { card_number, .. } => *card_number == number,
        TriggerKind::AnyOf(branches) => branches.iter().any(|branch| contains_ordinal(branch, number)),
        _ => false,
    }
}
fn assert_ordinal(game: &GameState, source: ObjectId, number: u32) {
    let chars = game.try_current_characteristics(source).unwrap().unwrap();
    assert!(chars.abilities.iter().any(|ability| {
        let AbilityKind::Triggered(ability) = &ability.kind else { return false; };
        ability.trigger.compiled_model().is_some_and(|trigger| contains_ordinal(trigger, number))
    }));
}
fn villains(game: &GameState) -> Vec<ObjectId> {
    game.battlefield.iter().copied().filter(|id| {
        game.object(*id).is_some_and(|object| object.kind == ObjectKind::Token)
            && game.current_has_subtype(*id, Subtype::Villain)
    }).collect()
}

#[test]
fn planisphere_third_draw_grant_cast_branch_and_equip_survive_exact_body_transport() {
    for definition in definitions("Astrologian's Planisphere") {
        let mut game = game();
        let mut choices = Choices::default();
        cast(&mut game, &definition, &mut choices);
        settle(&mut game, &mut choices);
        let equipment = battlefield_named(&game, "Astrologian's Planisphere");
        let hero = battlefield_named(&game, "Hero Token");
        assert_eq!(game.object(equipment).unwrap().attached_to, Some(AttachmentTarget::Object(hero)));
        assert!(game.current_has_subtype(hero, Subtype::Wizard));
        assert_ordinal(&game, hero, 3);
        draw(&mut game, equipment, A, 1);
        queue(&mut game, &mut choices);
        assert!(game.stack_is_empty());
        draw(&mut game, equipment, A, 2);
        queue(&mut game, &mut choices);
        assert_eq!(game.stack.len(), 1, "one grant crosses the third-card ordinal");
        assert_eq!(game.counter_count(hero, CounterType::PlusOnePlusOne), 0);
        draw(&mut game, equipment, A, 1);
        queue(&mut game, &mut choices);
        assert_eq!(game.stack.len(), 1, "later draws do not move the captured window");
        settle(&mut game, &mut choices);
        assert_eq!(game.counter_count(hero, CounterType::PlusOnePlusOne), 1);
        draw(&mut game, equipment, B, 3);
        queue(&mut game, &mut choices);
        assert!(game.stack_is_empty());

        let noncreature = compile_to_runtime_definition("Noncreature witness",
            "Mana cost: {0}\nType: Instant\nYou gain 1 life.", false).unwrap();
        cast(&mut game, &noncreature, &mut choices);
        assert_eq!(game.stack.len(), 2, "the other granted trigger still observes casting");
        settle(&mut game, &mut choices);
        assert_eq!(game.counter_count(hero, CounterType::PlusOnePlusOne), 2);
        let spare = compile_to_runtime_definition("New wearer",
            "Mana cost: {0}\nType: Creature — Human\nPower/Toughness: 1/2", false).unwrap();
        let wearer = game.create_object_from_definition(&spare, A, Zone::Battlefield);
        choices.target = Some(Target::Object(wearer));
        let ability_index = definition.abilities.iter()
            .position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
        action(&mut game, LegalAction::ActivateAbility { source: equipment, ability_index }, &mut choices);
        settle(&mut game, &mut choices);
        choices.target = None;
        assert_eq!(game.object(equipment).unwrap().attached_to, Some(AttachmentTarget::Object(wearer)));
        assert!(!game.current_has_subtype(hero, Subtype::Wizard));
        assert!(game.current_has_subtype(wearer, Subtype::Wizard));
        assert_ordinal(&game, wearer, 3);
        game.next_turn();
        draw(&mut game, equipment, A, 3);
        queue(&mut game, &mut choices);
        assert_eq!(game.stack.len(), 1, "a new turn restarts the current wearer's ordinal");
        settle(&mut game, &mut choices);
        assert_eq!(game.counter_count(wearer, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.counter_count(hero, CounterType::PlusOnePlusOne), 2);
    }
}

#[test]
fn madame_connive_counts_as_first_draw_then_second_draw_creates_one_complete_villain() {
    for definition in definitions("Madame Masque") {
        let mut game = game();
        let mut choices = Choices::default();
        cast(&mut game, &definition, &mut choices);
        settle(&mut game, &mut choices);
        let source = battlefield_named(&game, "Madame Masque");
        assert_ordinal(&game, source, 2);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1);
        assert!(game.player(A).unwrap().hand.is_empty(), "connive discards its actual draw");
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        assert!(villains(&game).is_empty());
        draw(&mut game, source, B, 2);
        queue(&mut game, &mut choices);
        assert!(game.stack_is_empty());
        draw(&mut game, source, A, 3);
        queue(&mut game, &mut choices);
        assert_eq!(game.stack.len(), 1, "crossing second once creates one trigger");
        assert!(villains(&game).is_empty(), "token waits for resolution");
        draw(&mut game, source, A, 1);
        queue(&mut game, &mut choices);
        assert_eq!(game.stack.len(), 1);
        settle(&mut game, &mut choices);
        let tokens = villains(&game);
        assert_eq!(tokens.len(), 1);
        let token = tokens[0];
        assert_eq!(game.current_controller(token), Some(A));
        assert_eq!(game.current_power(token), Some(2));
        assert_eq!(game.current_toughness(token), Some(1));
        assert!(game.current_has_static_ability_id(token, StaticAbilityId::Menace));
        assert_eq!(game.object(token).unwrap().colors(), ironsmith::color::ColorSet::BLACK);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1);
        game.next_turn();
        draw(&mut game, source, A, 2);
        queue(&mut game, &mut choices);
        assert_eq!(game.stack.len(), 1);
        settle(&mut game, &mut choices);
        assert_eq!(villains(&game).len(), 2);
    }
}

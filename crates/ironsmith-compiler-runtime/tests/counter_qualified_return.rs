//! Counter-qualified return clauses. All scenarios are authored and unrun.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{compute_legal_actions, DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{apply_decision_context_with_dm, apply_priority_response_with_dm,
    resolve_stack_entry_with, PriorityLoopState, PriorityResponse};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, CounterType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/counter_qualified_return.json.fixture")).unwrap();
    let row = &rows[0];
    let name = row["name"].as_str().unwrap();
    let text = format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, &text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, &text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct.unwrap(), ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 4);
    game
}
fn object(game: &mut GameState, owner: PlayerId, card_type: CardType) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Counter witness")
        .card_types(vec![card_type]).power_toughness(PowerToughness::fixed(2, 3)).build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}
struct NoTargets;
impl DecisionMaker for NoTargets {
    fn decide_targets(&mut self, _: &GameState, _: &TargetsContext) -> Vec<Target> {
        panic!("this return instruction does not target");
    }
}
fn cast(game: &mut GameState, definition: &CardDefinition) {
    let card = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell { spell_id: card, from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut state = PriorityLoopState::new(3);
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut NoTargets).unwrap();
    for _ in 0..40 {
        if !state.has_pending_action() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut NoTargets).unwrap();
    }
    assert!(!state.has_pending_action());
    assert!(game.stack.last().unwrap().targets.is_empty());
    assert_eq!(game.player(A).unwrap().mana_pool.blue, 0, "real mana cost was paid");
}
fn location(game: &GameState, stable: ironsmith::ids::StableId) -> Zone {
    game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone
}

#[test]
fn complete_wave_uses_only_plus_one_counters_and_returns_cards_to_their_owners() {
    for definition in definitions() {
        let mut game = game();
        let plus = object(&mut game, A, CardType::Creature);
        let charge = object(&mut game, B, CardType::Creature);
        let stolen = object(&mut game, C, CardType::Creature);
        let land = object(&mut game, B, CardType::Land);
        game.add_counters(plus, CounterType::PlusOnePlusOne, 1);
        game.add_counters(charge, CounterType::Charge, 1);
        game.set_current_controller(stolen, A);
        assert_eq!(game.current_controller(stolen), Some(A));
        game.object_mut(stolen).unwrap().abilities_mut().push(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::shroud()));
        let charge_stable = game.object(charge).unwrap().stable_id;
        let stolen_stable = game.object(stolen).unwrap().stable_id;
        cast(&mut game, &definition);
        resolve_stack_entry_with(&mut game, &mut NoTargets).unwrap();
        assert_eq!(game.object(plus).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(land).unwrap().zone, Zone::Battlefield);
        assert_eq!(location(&game, charge_stable), Zone::Hand);
        assert_eq!(location(&game, stolen_stable), Zone::Hand);
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        assert_eq!(game.player(C).unwrap().hand.len(), 1);
        assert!(game.player(A).unwrap().hand.is_empty());
    }
}

#[test]
fn the_counter_filter_and_affected_set_are_read_at_resolution() {
    for definition in definitions() {
        let mut game = game();
        let gains_counter = object(&mut game, B, CardType::Creature);
        let loses_counter = object(&mut game, C, CardType::Creature);
        game.add_counters(loses_counter, CounterType::PlusOnePlusOne, 1);
        let loses_stable = game.object(loses_counter).unwrap().stable_id;
        cast(&mut game, &definition);
        game.add_counters(gains_counter, CounterType::PlusOnePlusOne, 1);
        game.remove_counters(loses_counter, CounterType::PlusOnePlusOne, 1, None, None);
        let later = object(&mut game, B, CardType::Creature);
        let later_stable = game.object(later).unwrap().stable_id;
        resolve_stack_entry_with(&mut game, &mut NoTargets).unwrap();
        assert_eq!(game.object(gains_counter).unwrap().zone, Zone::Battlefield);
        assert_eq!(location(&game, loses_stable), Zone::Hand);
        assert_eq!(location(&game, later_stable), Zone::Hand);
    }
}

#[test]
fn empty_wave_is_a_legal_paid_spell() {
    for definition in definitions() {
        let mut game = game();
        cast(&mut game, &definition);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.battlefield.is_empty());
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
    }
}

#[test]
fn unowned_counter_qualification_tails_still_fail_strictly() {
    for oracle in [
        "Return each creature without a +1/+1 counter on it while a puzzle was solved to its owner's hand.",
        "Return each creature without a +1/+1 counter on you to its owner's hand.",
    ] {
        let text = format!("Mana cost: {{2}}{{U}}{{U}}\nType: Sorcery\n{oracle}");
        let (result, loss) = ironsmith_compiler::parse_loss::capture(||
            compile_to_runtime_definition("Counter tail witness", &text, false));
        assert!(result.is_err() || loss.is_lossy(), "unsupported qualification admitted");
    }
}

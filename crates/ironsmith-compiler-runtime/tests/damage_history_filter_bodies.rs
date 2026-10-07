//! Complete frozen dealer-history bodies. Authored only; execution is deferred.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{compute_legal_actions, DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effect::Effect;
use ironsmith::effects::{execute_effect, DealDamageEffect, EffectContext};
use ironsmith::filter::ObjectFilter;
use ironsmith::game_loop::{apply_decision_context_with_dm, apply_priority_response_with_dm,
    put_triggers_on_stack_with_dm, resolve_stack_entry_with, PriorityLoopState, PriorityResponse};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/damage_history_filter_bodies.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, &text, false));
    let (artifact, _) = compiled.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &materialized] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, materialized]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 30);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black,
        ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 20);
    }
    game
}
fn witness(game: &mut GameState, player: PlayerId, creature: bool) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "History witness")
        .card_types(vec![if creature { CardType::Creature } else { CardType::Artifact }])
        .power_toughness(PowerToughness::fixed(2, 20)).build();
    game.create_object_from_card(&card, player, Zone::Battlefield)
}
fn hit(game: &mut GameState, source: ObjectId, target: Target, amount: i32, combat: bool) {
    let target = match target {
        Target::Object(id) => ChooseSpec::SpecificObject(id),
        Target::Player(player) => ChooseSpec::SpecificPlayer(player),
    };
    let mut damage = DealDamageEffect::new(amount, target);
    damage.source_is_combat = combat;
    let controller = game.current_controller(source).unwrap();
    execute_effect(game, &Effect::new(damage),
        &mut EffectContext::new(source, controller, &mut SelectFirstDecisionMaker)).unwrap();
}
#[derive(Default)]
struct Choices { target: Option<Target>, forbidden: Vec<Target>, target_calls: usize }
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.target_calls += 1;
        for requirement in &context.requirements {
            assert!(self.forbidden.iter().all(|target| !requirement.legal_targets.contains(target)));
        }
        if let Some(target) = self.target {
            assert!(context.requirements.iter().all(|r| r.legal_targets.contains(&target)));
            vec![target]
        } else { SelectFirstDecisionMaker.decide_targets(game, context) }
    }
}
fn cast(game: &mut GameState, definition: &CardDefinition, choices: &mut Choices) -> ObjectId {
    let card = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell { spell_id: card, from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut state = PriorityLoopState::new(3);
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), choices).unwrap();
    for _ in 0..60 {
        if !state.has_pending_action() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices).unwrap();
    }
    assert!(!state.has_pending_action());
    game.stack.iter().rev().find(|entry| !entry.is_ability).unwrap().object_id
}
fn resolve_all(game: &mut GameState, choices: &mut Choices) {
    for _ in 0..12 {
        put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), choices).unwrap();
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("unexpected continuing program");
}
fn zone_of(game: &GameState, stable: ironsmith::ids::StableId) -> Zone {
    game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone
}

/// Match through the public target validator: a non-target object spec is
/// exactly the filter evaluated in the context's filter view.
fn filter_matches(game: &ironsmith::GameState, viewer: ironsmith::PlayerId, filter: &ironsmith::target::ObjectFilter, object: ironsmith::ObjectId) -> bool {
    let ctx = ironsmith::effects::EffectContext::new_default(object, viewer);
    ironsmith::effects::validate_target(game, &ironsmith::effects::ResolvedTarget::Object(object),
        &ironsmith::target::ChooseSpec::Object(filter.clone()), &ctx)
}
#[test]
fn arrow_uses_the_dealer_not_the_recipient_for_combat_and_noncombat_damage() {
    for definition in definitions("Avenging Arrow") {
        for combat in [false, true] {
            let mut game = game();
            let dealer = witness(&mut game, B, true);
            let recipient = witness(&mut game, C, true);
            let untouched = witness(&mut game, B, true);
            hit(&mut game, dealer, Target::Object(recipient), 1, combat);
            let stable = game.object(dealer).unwrap().stable_id;
            let mut choices = Choices { target: Some(Target::Object(dealer)),
                forbidden: vec![Target::Object(recipient), Target::Object(untouched)], ..Default::default() };
            cast(&mut game, &definition, &mut choices);
            resolve_all(&mut game, &mut choices);
            assert_eq!(zone_of(&game, stable), Zone::Graveyard);
            assert_eq!(game.object(recipient).unwrap().zone, Zone::Battlefield);
            assert_eq!(choices.target_calls, 1);
        }
    }
}

#[test]
fn reciprocate_binds_you_to_the_spell_controller_and_requires_actual_positive_damage() {
    for definition in definitions("Reciprocate") {
        let mut game = game();
        let qualifying = witness(&mut game, B, true);
        let wrong_player = witness(&mut game, B, true);
        let zero = witness(&mut game, B, true);
        hit(&mut game, qualifying, Target::Player(A), 1, false);
        hit(&mut game, wrong_player, Target::Player(C), 1, true);
        hit(&mut game, zero, Target::Player(A), 0, false);
        let stable = game.object(qualifying).unwrap().stable_id;
        let mut choices = Choices { target: Some(Target::Object(qualifying)),
            forbidden: vec![Target::Object(wrong_player), Target::Object(zero)], ..Default::default() };
        cast(&mut game, &definition, &mut choices);
        resolve_all(&mut game, &mut choices);
        assert_eq!(zone_of(&game, stable), Zone::Exile);
        assert_eq!(game.object(wrong_player).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn restore_returns_every_current_creature_dealer_to_its_owner_without_targets() {
    for definition in definitions("Restore the Peace") {
        let mut game = game();
        let own = witness(&mut game, A, true);
        let opposing = witness(&mut game, B, true);
        let recipient = witness(&mut game, C, true);
        let noncreature = witness(&mut game, B, false);
        hit(&mut game, own, Target::Object(recipient), 1, false);
        hit(&mut game, opposing, Target::Player(A), 1, true);
        hit(&mut game, noncreature, Target::Player(C), 1, false);
        let own_stable = game.object(own).unwrap().stable_id;
        let opposing_stable = game.object(opposing).unwrap().stable_id;
        let mut choices = Choices::default();
        cast(&mut game, &definition, &mut choices);
        resolve_all(&mut game, &mut choices);
        assert_eq!(zone_of(&game, own_stable), Zone::Hand);
        assert_eq!(zone_of(&game, opposing_stable), Zone::Hand);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        assert_eq!(game.object(recipient).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(noncreature).unwrap().zone, Zone::Battlefield);
        assert_eq!(choices.target_calls, 0);
    }
}

#[test]
fn guardian_really_casts_with_flash_and_its_entry_targets_only_opposing_dealers() {
    for definition in definitions("Red Guardian, Super-Soldier") {
        let mut game = game();
        game.turn.active_player = B;
        let opposing = witness(&mut game, B, true);
        let own = witness(&mut game, A, true);
        let untouched = witness(&mut game, B, true);
        hit(&mut game, opposing, Target::Player(C), 1, false);
        hit(&mut game, own, Target::Player(B), 1, false);
        let stable = game.object(opposing).unwrap().stable_id;
        let mut choices = Choices { target: Some(Target::Object(opposing)),
            forbidden: vec![Target::Object(own), Target::Object(untouched)], ..Default::default() };
        let spell = cast(&mut game, &definition, &mut choices);
        let guardian = game.object(spell).unwrap().stable_id;
        resolve_all(&mut game, &mut choices);
        assert_eq!(zone_of(&game, guardian), Zone::Battlefield);
        assert_eq!(zone_of(&game, stable), Zone::Graveyard);
        assert_eq!(choices.target_calls, 1);
    }
}

#[test]
fn damage_history_never_transfers_to_a_new_incarnation_and_clears_each_turn() {
    let mut game = game();
    let old = witness(&mut game, B, true);
    hit(&mut game, old, Target::Player(A), 1, true);
    let saved = game.clone();
    let departed = game.move_object_by_effect(old, Zone::Exile).unwrap();
    let new = game.move_object_by_effect(departed, Zone::Battlefield).unwrap();
    assert_ne!(old, new);
    assert!(!game.source_dealt_damage_this_turn(new));
    assert!(!game.source_dealt_damage_to_player_this_turn(new, A));
    assert!(!game.source_dealt_combat_damage_to_player_this_turn(new));
    let mut dealer_filter = ObjectFilter::creature();
    dealer_filter.dealt_damage_this_turn = true;
    assert!(!filter_matches(&game, A, &dealer_filter, new));
    game = saved;
    assert!(game.source_dealt_damage_this_turn(old));
    assert!(game.source_dealt_damage_to_player_this_turn(old, A));
    assert!(game.source_dealt_combat_damage_to_player_this_turn(old));
    game.turn_store.turn_history.clear_for_new_turn();
    assert!(!game.source_dealt_damage_this_turn(old));
    assert!(!game.source_dealt_damage_to_player_this_turn(old, A));
    assert!(!game.source_dealt_combat_damage_to_player_this_turn(old));
}

#[test]
fn an_announced_damage_dealer_target_that_blinks_fizzles_without_harming_its_return() {
    for name in ["Avenging Arrow", "Reciprocate"] {
        for definition in definitions(name) {
            let mut game = game();
            let old = witness(&mut game, B, true);
            hit(&mut game, old, Target::Player(A), 1, false);
            let mut choices = Choices { target: Some(Target::Object(old)), ..Default::default() };
            cast(&mut game, &definition, &mut choices);
            let departed = game.move_object_by_effect(old, Zone::Exile).unwrap();
            let returned = game.move_object_by_effect(departed, Zone::Battlefield).unwrap();
            resolve_all(&mut game, &mut choices);
            assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        }
    }
}

#[test]
fn unsupported_history_tails_are_never_silently_dropped() {
    for oracle in [
        "Destroy target creature that dealt damage this turn unless its controller solved a puzzle.",
        "Exile target creature that dealt damage to you this turn while a puzzle was solved.",
        "Return each creature that dealt damage this turn while a puzzle was solved to its owner's hand.",
    ] {
        let text = format!("Mana cost: {{1}}\nType: Instant\n{oracle}");
        let (result, loss) = ironsmith_compiler::parse_loss::capture(||
            compile_to_runtime_definition("History tail witness", &text, false));
        assert!(result.is_err() || loss.is_lossy(), "unowned tail accepted: {oracle}");
    }
}

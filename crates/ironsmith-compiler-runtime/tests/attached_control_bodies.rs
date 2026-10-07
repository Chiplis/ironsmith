//! Complete frozen attached control bodies. Authored only; execution is deferred.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{compute_legal_actions, DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effect::Effect;
use ironsmith::effects::{execute_effect, DealDamageEffect, EffectContext};
use ironsmith::filter::{FilterContext, ObjectFilter};
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
        "../../../fixtures/attached_control_bodies.json.fixture")).unwrap();
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
fn object(game: &mut GameState, owner: PlayerId, types: Vec<CardType>) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Controlled witness")
        .card_types(types).power_toughness(PowerToughness::fixed(2, 20)).build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}
fn change(game: &mut GameState, target: ObjectId, modification: ironsmith::continuous::Modification) {
    use ironsmith::effects::EffectExecutor;
    ironsmith::effects::ApplyContinuousEffect::new(ironsmith::continuous::EffectTarget::Specific(target),
        modification, ironsmith::effect::Until::Forever)
        .execute(game, &mut EffectContext::new_default(target, A)).unwrap();
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

fn host_types(name: &str) -> Vec<CardType> {
    match name {
        "Domineer" => vec![CardType::Artifact, CardType::Creature],
        "Steal Enchantment" => vec![CardType::Enchantment],
        _ => vec![CardType::Land],
    }
}

#[test]
fn complete_control_auras_cast_attach_and_follow_live_aura_control_and_attachment() {
    for name in ["Domineer", "Steal Enchantment", "In Bolas's Clutches"] {
        for definition in definitions(name) {
            let mut game = game();
            let first = object(&mut game, B, host_types(name));
            let second = object(&mut game, C, host_types(name));
            let first_stable = game.object(first).unwrap().stable_id;
            let mut choices = Choices { target: Some(Target::Object(first)), ..Default::default() };
            let spell = cast(&mut game, &definition, &mut choices);
            let aura_stable = game.object(spell).unwrap().stable_id;
            resolve_all(&mut game, &mut choices);
            let aura = game.find_object_by_stable_id(aura_stable).unwrap();
            assert_eq!(game.object(aura).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.object(aura).unwrap().attached_to, Some(ironsmith::object::AttachmentTarget::Object(first)));
            assert_eq!(game.current_controller(first), Some(A));
            assert_eq!(game.object(first).unwrap().owner, B);
            if name == "In Bolas's Clutches" {
                assert!(game.current_supertypes(first).unwrap().contains(&ironsmith::types::Supertype::Legendary));
            }
            let saved = game.clone();
            game.set_current_controller(aura, C).unwrap();
            assert_eq!(game.current_controller(first), Some(C));
            assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(second)));
            assert_eq!(game.current_controller(first), Some(B));
            assert_eq!(game.current_controller(second), Some(C));
            game = saved;
            game.phase_out(aura);
            assert_eq!(game.current_controller(first), Some(B));
            game.phase_in(aura);
            assert_eq!(game.current_controller(first), Some(A));
            game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
            assert_eq!(game.current_controller(first), Some(B));
            assert_eq!(zone_of(&game, first_stable), Zone::Battlefield);
            if name == "In Bolas's Clutches" {
                assert!(!game.current_supertypes(first).unwrap().contains(&ironsmith::types::Supertype::Legendary));
            }
            assert_eq!(choices.target_calls, 1);
        }
    }
}

#[test]
fn compound_and_enchantment_enchant_domains_do_not_widen_to_other_permanents() {
    for name in ["Domineer", "Steal Enchantment"] {
        for definition in definitions(name) {
            let mut game = game();
            let valid = object(&mut game, B, host_types(name));
            let artifact = object(&mut game, B, vec![CardType::Artifact]);
            let creature = object(&mut game, B, vec![CardType::Creature]);
            let land = object(&mut game, B, vec![CardType::Land]);
            let mut choices = Choices { target: Some(Target::Object(valid)),
                forbidden: vec![Target::Object(artifact), Target::Object(creature), Target::Object(land)],
                ..Default::default() };
            cast(&mut game, &definition, &mut choices);
            resolve_all(&mut game, &mut choices);
            assert_eq!(game.current_controller(valid), Some(A));
            for id in [artifact, creature, land] { assert_eq!(game.current_controller(id), Some(B)); }
        }
    }
}

#[test]
fn illegal_aura_target_at_resolution_never_transfers_control_to_new_incarnation() {
    for name in ["Domineer", "Steal Enchantment", "In Bolas's Clutches"] {
        for definition in definitions(name) {
            let mut game = game();
            let target = object(&mut game, B, host_types(name));
            let spell = cast(&mut game, &definition, &mut Choices { target: Some(Target::Object(target)), ..Default::default() });
            let aura_stable = game.object(spell).unwrap().stable_id;
            let exile = game.move_object_by_effect(target, Zone::Exile).unwrap();
            let returned = game.move_object_by_effect(exile, Zone::Battlefield).unwrap();
            resolve_all(&mut game, &mut Choices::default());
            assert_ne!(target, returned);
            assert_eq!(game.current_controller(returned), Some(B));
            assert_eq!(zone_of(&game, aura_stable), Zone::Graveyard);
        }
    }
}

#[test]
fn loss_of_enchant_is_completed_by_state_based_aura_cleanup() {
    for name in ["Domineer", "Steal Enchantment", "In Bolas's Clutches"] {
        for definition in definitions(name) {
            let mut game = game();
            let target = object(&mut game, B, host_types(name));
            let spell = cast(&mut game, &definition, &mut Choices { target: Some(Target::Object(target)), ..Default::default() });
            let stable = game.object(spell).unwrap().stable_id;
            resolve_all(&mut game, &mut Choices::default());
            let aura = game.find_object_by_stable_id(stable).unwrap();
            change(&mut game, aura, ironsmith::continuous::Modification::RemoveAllAbilities);
            // Do not read a later-layer ability loss as retroactively removing
            // an earlier-layer effect. Losing enchant makes the Aura illegal;
            // the completed state-based cleanup removes its live source.
            assert!(ironsmith::rules::state_based::apply_state_based_actions_with(
                &mut game, &mut Choices::default()).unwrap());
            assert_eq!(zone_of(&game, stable), Zone::Graveyard);
            assert_eq!(game.current_controller(target), Some(B));
            if name == "In Bolas's Clutches" {
                assert!(!game.current_supertypes(target).unwrap().contains(&ironsmith::types::Supertype::Legendary));
            }
        }
    }
}

#[test]
fn source_supertype_property_keeps_enchantment_legendary_and_strict_control_tails() {
    for definition in definitions("In Bolas's Clutches") {
        assert!(definition.card.supertypes.contains(&ironsmith::types::Supertype::Legendary));
    }
    for text in ["Enchant artifact creature\nYou control enchanted artifact creature until a puzzle is solved.",
        "Enchant enchantment\nYou control enchanted enchantment and draw a card."] {
        assert!(compile_to_runtime_definition("Strict control witness", format!("Type: Enchantment — Aura\n{text}"), false).is_err());
    }
}

//! Complete frozen blocking-history destroy bodies. Authored only; execution is deferred.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{compute_legal_actions, DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effect::Effect;
use ironsmith::effects::{execute_effect, BecomeBlockedEffect, EffectContext};
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration};
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
        "../../../fixtures/combat_history_destroy.json.fixture")).unwrap();
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
fn witness(game: &mut GameState, player: PlayerId, legendary: bool) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "History witness")
        .card_types(vec![CardType::Creature])
        .supertypes(if legendary { vec![ironsmith::types::Supertype::Legendary] } else { vec![] })
        .power_toughness(PowerToughness::fixed(2, 20)).build();
    let id = game.create_object_from_card(&card, player, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
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

fn declare(game: &mut GameState, attacks: &[ObjectId], pairs: &[(ObjectId, ObjectId)]) -> TriggerQueue {
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.mark_combat_phase_started();
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    let declarations = attacks.iter().map(|id| AttackerDeclaration {
        creature: *id, target: AttackTarget::Player(B),
    }).collect::<Vec<_>>();
    ironsmith::game_loop::apply_attacker_declarations(game, &mut combat, &mut queue, &declarations).unwrap();
    game.combat = Some(combat.clone());
    game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
    let blocks = pairs.iter().map(|(blocker, attacker)| BlockerDeclaration {
        blocker: *blocker, blocking: *attacker,
    }).collect::<Vec<_>>();
    ironsmith::game_loop::apply_multiplayer_blocker_declarations(game, &mut combat, &mut queue, &blocks).unwrap();
    game.combat = Some(combat);
    queue
}
fn end_combat(game: &mut GameState, queue: &mut TriggerQueue) {
    game.turn.step = Some(ironsmith::game_state::Step::EndCombat);
    ironsmith::game_loop::generate_and_queue_step_triggers(game, queue);
    put_triggers_on_stack_with_dm(game, queue, &mut SelectFirstDecisionMaker).unwrap();
}

#[test]
fn heat_stroke_resolves_real_end_combat_trigger_over_both_roles_and_effect_blocked_attackers() {
    for definition in definitions("Heat Stroke") {
        let mut game = game();
        let enchantment = cast(&mut game, &definition, &mut Choices::default());
        let enchantment_stable = game.object(enchantment).unwrap().stable_id;
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(zone_of(&game, enchantment_stable), Zone::Battlefield);
        let attacker = witness(&mut game, A, false);
        let blocker = witness(&mut game, B, false);
        let effect_blocked = witness(&mut game, A, false);
        let unblocked = witness(&mut game, A, false);
        let untouched = witness(&mut game, B, false);
        let attacked_stable = game.object(attacker).unwrap().stable_id;
        let blocker_stable = game.object(blocker).unwrap().stable_id;
        let effect_stable = game.object(effect_blocked).unwrap().stable_id;
        let mut queue = declare(&mut game, &[attacker, effect_blocked, unblocked], &[(blocker, attacker)]);
        execute_effect(&mut game, &Effect::new(BecomeBlockedEffect::with_spec(ChooseSpec::SpecificObject(effect_blocked))),
            &mut EffectContext::new(attacker, A, &mut SelectFirstDecisionMaker)).unwrap();
        // Current combat membership is not the historical predicate.
        game.combat = None;
        end_combat(&mut game, &mut queue);
        assert_eq!(game.stack.len(), 1);
        resolve_all(&mut game, &mut Choices::default());
        for stable in [attacked_stable, blocker_stable, effect_stable] { assert_eq!(zone_of(&game, stable), Zone::Graveyard); }
        for id in [unblocked, untouched] { assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield); }
    }
}

#[test]
fn legendary_partner_is_read_at_the_block_event_in_both_directions() {
    for definition in definitions("You Cannot Pass!") {
        for legend_is_attacker in [false, true] {
            for partner_leaves in [false, true] {
                let mut game = game();
                let attacker = witness(&mut game, A, legend_is_attacker);
                let blocker = witness(&mut game, B, !legend_is_attacker);
                let target = if legend_is_attacker { blocker } else { attacker };
                let partner = if legend_is_attacker { attacker } else { blocker };
                let unrelated = witness(&mut game, B, false);
                declare(&mut game, &[attacker], &[(blocker, attacker)]);
                if partner_leaves { game.move_object_by_effect(partner, Zone::Exile).unwrap(); }
                else { game.object_mut(partner).unwrap().supertypes.clear(); }
                let stable = game.object(target).unwrap().stable_id;
                let mut choices = Choices { target: Some(Target::Object(target)), forbidden: vec![Target::Object(unrelated)], ..Default::default() };
                cast(&mut game, &definition, &mut choices);
                resolve_all(&mut game, &mut choices);
                assert_eq!(zone_of(&game, stable), Zone::Graveyard);
                assert_eq!(choices.target_calls, 1);
            }
        }
    }
}

#[test]
fn later_legendary_status_does_not_create_old_eligibility_and_blink_never_inherits_history() {
    for definition in definitions("You Cannot Pass!") {
        let mut game = game();
        let attacker = witness(&mut game, A, false);
        let blocker = witness(&mut game, B, false);
        declare(&mut game, &[attacker], &[(blocker, attacker)]);
        game.object_mut(blocker).unwrap().supertypes.push(ironsmith::types::Supertype::Legendary);
        let card = game.create_object_from_definition(&definition, A, Zone::Hand);
        let action = LegalAction::CastSpell { spell_id: card, from_zone: Zone::Hand, casting_method: CastingMethod::Normal };
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&action));
        let snapshot = game.clone();
        let departed = game.move_object_by_effect(attacker, Zone::Exile).unwrap();
        let returned = game.move_object_by_effect(departed, Zone::Battlefield).unwrap();
        assert_ne!(attacker, returned);
        assert!(!game.creature_was_blocked_this_turn(returned));
        assert!(snapshot.creature_was_blocked_this_turn(attacker));
        game = snapshot;
        game.turn_store.turn_history.clear_for_new_turn();
        assert!(!game.creature_was_blocked_this_turn(attacker));
        assert!(!game.creature_blocked_this_turn(blocker));
    }
}

#[test]
fn an_announced_target_blink_fizzles_and_does_not_destroy_its_new_incarnation() {
    for definition in definitions("You Cannot Pass!") {
        let mut game = game();
        let attacker = witness(&mut game, A, false);
        let blocker = witness(&mut game, B, true);
        declare(&mut game, &[attacker], &[(blocker, attacker)]);
        cast(&mut game, &definition, &mut Choices { target: Some(Target::Object(attacker)), ..Default::default() });
        let exile = game.move_object_by_effect(attacker, Zone::Exile).unwrap();
        let returned = game.move_object_by_effect(exile, Zone::Battlefield).unwrap();
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn historic_partner_filter_rejects_missing_receipts_at_the_checked_legal_query() {
    for definition in definitions("You Cannot Pass!") {
        let mut game = game();
        let attacker = witness(&mut game, A, false);
        let blocker = witness(&mut game, B, true);
        let provenance = game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::CreatureBlocked);
        let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::combat::CreatureBlockedEvent::new(blocker, attacker), provenance);
        game.turn_store.turn_history.record_event(&event, None, None);
        let _card = game.create_object_from_definition(&definition, A, Zone::Hand);
        assert!(compute_legal_actions(&game, A).is_err());
    }
}

#[test]
fn unsupported_history_tail_is_not_dropped() {
    for text in [
        "Destroy target creature that blocked or was blocked by a legendary creature this turn while a puzzle was solved.",
        "At end of combat, destroy each creature that blocked or was blocked this turn while a puzzle was solved.",
    ] {
        assert!(compile_to_runtime_definition("Strict history witness", format!("Type: Instant\n{text}"), false).is_err());
    }
}

#[test]
fn a_complete_matching_occurrence_proves_eligibility_despite_another_unknown_occurrence() {
    for definition in definitions("You Cannot Pass!") {
        let mut game = game();
        let attacker = witness(&mut game, A, true);
        let blocker = witness(&mut game, B, true);
        let provenance = game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::CreatureBlocked);
        let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::combat::CreatureBlockedEvent::new(blocker, attacker), provenance);
        game.turn_store.turn_history.record_event(&event, None, None);
        declare(&mut game, &[attacker], &[(blocker, attacker)]);
        let stable = game.object(attacker).unwrap().stable_id;
        cast(&mut game, &definition, &mut Choices { target: Some(Target::Object(attacker)), ..Default::default() });
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(zone_of(&game, stable), Zone::Graveyard);
    }
}

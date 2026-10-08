//! Restored exact full body and review corrections; all checks are authored, UNRUN.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::effect::{Effect, Until};
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{ColorSet, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions() -> [CardDefinition; 3] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/essence_leak_body.json.fixture")).unwrap(); let row = &rows[0];
    let text = format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Essence Leak", &text, false));
    let (artifact, materialized) = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text()); artifact.validate().unwrap();
    let encoded = artifact.to_json().unwrap(); let json = std::str::from_utf8(&encoded).unwrap();
    assert!(json.contains("source_mana_cost") && json.contains("UnlessPays"), "{json}");
    let decoded = CompiledCardArtifact::from_json(&encoded).unwrap(); assert_eq!(artifact, decoded);
    // compile_to_artifact returns an artifact-materialized definition. Keep
    // its route and the JSON roundtrip separate from true direct lowering.
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition("Essence Leak", &text, false));
    let direct = direct.unwrap();
    assert!(!direct_loss.is_lossy(), "direct route: {}", direct_loss.reasons_text());
    [direct, materialized, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
struct Choices { target: Option<ObjectId>, pay: bool, offers: Vec<(PlayerId, Option<ObjectId>)> }
impl Default for Choices { fn default() -> Self { Self { target: None, pay: true, offers: Vec::new() } } }
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, context: &BooleanContext) -> bool {
        self.offers.push((context.player, context.source)); self.pay && context.can_accept
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if let Some(id) = self.target {
            assert!(context.requirements.iter().all(|r| r.legal_targets.contains(&Target::Object(id)))); vec![Target::Object(id)]
        } else { SelectFirstDecisionMaker.decide_targets(game, context) }
    }
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None; game.turn.active_player = A; game.turn.priority_player = Some(A);
    for player in [A, B] { for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
        game.player_mut(player).unwrap().mana_pool.add(color, 20);
    }} game
}
fn host(game: &mut GameState, cost: Option<&str>) -> ObjectId {
    let metadata = cost.map(|c| format!("Mana cost: {c}\n")).unwrap_or_default();
    let definition = compile_to_runtime_definition("Host", format!("{metadata}Type: Creature — Beast\nPower/Toughness: 3/3"), false).unwrap();
    game.create_object_from_definition(&definition, B, Zone::Battlefield)
}
fn cast(game: &mut GameState, definition: &CardDefinition, player: PlayerId, method: CastingMethod, dm: &mut Choices) -> ObjectId {
    game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None; game.turn.active_player = player; game.turn.priority_player = Some(player);
    let spell = game.create_object_from_definition(definition, player, Zone::Hand); let stable = game.object(spell).unwrap().stable_id;
    let action = LegalAction::CastSpell { spell_id: spell, from_zone: Zone::Hand, casting_method: method };
    assert!(compute_legal_actions(game, player).unwrap().contains(&action));
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap(); resolve_stack_entry_with(game, dm).unwrap(); game.find_object_by_stable_id(stable).unwrap()
}
fn attach(game: &mut GameState, definition: &CardDefinition, recipient: ObjectId) -> ObjectId {
    cast(game, definition, A, CastingMethod::Normal, &mut Choices { target: Some(recipient), ..Default::default() })
}
fn upkeep(game: &mut GameState, player: PlayerId, dm: &mut Choices) -> usize {
    game.turn.phase = ironsmith::Phase::Beginning; game.turn.step = Some(ironsmith::game_state::Step::Upkeep); game.turn.active_player = player;
    game.queue_trigger_event(Default::default(), TriggerEvent::new(ironsmith::events::BeginningOfUpkeepEvent::new(player), Default::default()));
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap(); game.stack.len()
}
fn colors(game: &mut GameState, source: ObjectId, colors: ColorSet) {
    ApplyContinuousEffect::new(EffectTarget::Specific(source), Modification::SetColors(colors), Until::Forever)
        .execute(game, &mut EffectContext::new_default(source, B)).unwrap();
}
fn fund(game: &mut GameState, player: PlayerId, mana: &[(ManaSymbol, u32)]) {
    game.player_mut(player).unwrap().mana_pool.empty();
    for (color, amount) in mana { game.player_mut(player).unwrap().mana_pool.add(*color, *amount); }
}
fn copy_cost(game: &mut GameState, recipient: ObjectId, donor: ObjectId) {
    ApplyContinuousEffect::new_runtime(EffectTarget::Specific(recipient), ironsmith::effects::continuous::RuntimeModification::CopyOf {
        source: ChooseSpec::SpecificObject(donor), preserve_source_abilities: false, name_override: None,
        name_override_surface: None, add_supertypes: Vec::new(), copy_exception_surface: None,
    }, Until::EndOfTurn).execute(game, &mut EffectContext::new_default(recipient, B)).unwrap();
}
#[test]
fn host_pays_colored_hybrid_zero_and_off_stack_x_costs_on_its_own_upkeep() {
    for definition in definitions() { for (cost, funding, spent) in [
        ("{1}{R}", vec![(ManaSymbol::Colorless, 1), (ManaSymbol::Red, 1)], 2),
        ("{R/G}{R/G}", vec![(ManaSymbol::Green, 2)], 2),
        ("{X}{R}", vec![(ManaSymbol::Red, 1)], 1),
        ("{0}", vec![], 0),
    ] {
        let mut game = game(); let recipient = host(&mut game, Some(cost)); colors(&mut game, recipient, ColorSet::RED);
        let aura = attach(&mut game, &definition, recipient); fund(&mut game, B, &funding);
        let owner_mana = game.player(A).unwrap().mana_pool.total(); let mut dm = Choices::default();
        assert_eq!(upkeep(&mut game, A, &mut dm), 0); assert_eq!(upkeep(&mut game, B, &mut dm), 1);
        assert_eq!(game.stack[0].object_id, recipient); assert_ne!(recipient, aura);
        let before = game.player(B).unwrap().mana_pool.total(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.battlefield.contains(&recipient)); assert_eq!(before - game.player(B).unwrap().mana_pool.total(), spent);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), owner_mana);
        assert!(dm.offers.iter().any(|(p, s)| *p == B && *s == Some(recipient)));
    }}
}
#[test]
fn decline_wrong_color_and_absent_mana_cost_sacrifice_without_payment() {
    for definition in definitions() { for (cost, pay, funding) in [
        (Some("{R}"), false, vec![(ManaSymbol::Red, 1)]),
        (Some("{R}"), true, vec![(ManaSymbol::Blue, 8)]),
        (None, true, vec![(ManaSymbol::Red, 8)]),
    ] {
        let mut game = game(); let recipient = host(&mut game, cost); colors(&mut game, recipient, ColorSet::RED);
        attach(&mut game, &definition, recipient); fund(&mut game, B, &funding);
        let before = game.player(B).unwrap().mana_pool.total(); let mut dm = Choices { pay, ..Default::default() };
        assert_eq!(upkeep(&mut game, B, &mut dm), 1); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(!game.battlefield.contains(&recipient)); assert_eq!(game.player(B).unwrap().mana_pool.total(), before);
    }}
}
#[test]
fn live_attachment_color_control_phasing_and_pending_source_are_distinct() {
    for definition in definitions() {
        let mut game = game(); let old = host(&mut game, Some("{R}")); let next = host(&mut game, Some("{G}"));
        let aura = attach(&mut game, &definition, old); let mut dm = Choices::default();
        colors(&mut game, old, ColorSet::BLUE); assert_eq!(upkeep(&mut game, B, &mut dm), 0);
        colors(&mut game, old, ColorSet::GREEN); game.phase_out(aura); assert_eq!(upkeep(&mut game, B, &mut dm), 0);
        game.phase_in(aura); assert_eq!(upkeep(&mut game, B, &mut dm), 1);
        assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(next)));
        fund(&mut game, B, &[(ManaSymbol::Red, 1)]); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.battlefield.contains(&old)); assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        game.set_current_controller(next, A).unwrap(); assert_eq!(upkeep(&mut game, B, &mut dm), 0);
        assert_eq!(upkeep(&mut game, A, &mut dm), 1); dm.pay = false; resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(!game.battlefield.contains(&next)); assert!(game.battlefield.contains(&old));
    }
}
#[test]
fn current_copy_and_exact_departure_or_phasing_lki_preserve_the_cost() {
    for definition in definitions() { for departure in [0, 1, 2] {
        let mut game = game(); let recipient = host(&mut game, Some("{5}{R}")); let donor = host(&mut game, Some("{G}{G}"));
        attach(&mut game, &definition, recipient); let mut dm = Choices::default(); assert_eq!(upkeep(&mut game, B, &mut dm), 1);
        copy_cost(&mut game, recipient, donor);
        if departure == 1 { game.move_object_by_effect(recipient, Zone::Exile).unwrap(); }
        if departure == 2 { game.phase_out(recipient); }
        fund(&mut game, B, &[(ManaSymbol::Green, 2)]); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 0, "case {departure}");
    }}
}
#[test]
fn current_face_down_cost_never_falls_back_to_printed_cost() {
    for definition in definitions() {
        let mut game = game(); let recipient = host(&mut game, Some("{R}")); attach(&mut game, &definition, recipient);
        let mut dm = Choices::default(); assert_eq!(upkeep(&mut game, B, &mut dm), 1);
        assert!(game.set_face_down(recipient)); fund(&mut game, B, &[(ManaSymbol::Red, 1)]);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap(); assert!(!game.battlefield.contains(&recipient)); assert_eq!(game.player(B).unwrap().mana_pool.total(), 1);
    }
}
#[test]
fn actual_prototype_cast_supplies_its_current_cost() {
    for definition in definitions() {
        let mut game = game(); let prototype = compile_to_runtime_definition("Prototype host", "Mana cost: {7}\nType: Artifact Creature — Construct\nPower/Toughness: 7/7\nPrototype {1}{G} — 2/3", false).unwrap();
        let index = prototype.alternative_casts.iter().position(|m| m.name().eq_ignore_ascii_case("prototype")).unwrap();
        let recipient = cast(&mut game, &prototype, B, CastingMethod::Alternative(index), &mut Choices::default());
        attach(&mut game, &definition, recipient); fund(&mut game, B, &[(ManaSymbol::Green, 2)]);
        let mut dm = Choices::default(); assert_eq!(upkeep(&mut game, B, &mut dm), 1); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.battlefield.contains(&recipient)); assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn typed_payment_failure_and_missing_or_mismatched_lki_never_become_decline() {
    for definition in definitions() { for kind in [0, 1, 2] {
        let mut game = game(); let recipient = host(&mut game, Some("{R}")); let unrelated = host(&mut game, Some("{G}"));
        attach(&mut game, &definition, recipient); let mut dm = Choices::default(); assert_eq!(upkeep(&mut game, B, &mut dm), 1);
        let stack_target = game.stack[0].target_id();
        if kind == 0 {
            fund(&mut game, B, &[(ManaSymbol::Red, u32::MAX)]);
            let error = resolve_stack_entry_with(&mut game, &mut dm).unwrap_err();
            assert!(matches!(&error, ironsmith::game_loop::GameLoopError::ExecutionFailed(
                ironsmith::effects::ExecutionError::ContinuousDiscovery(_))), "{error:?}");
            assert_eq!(game.player(B).unwrap().mana_pool.amount(ManaSymbol::Red), u32::MAX);
        } else {
            game.phase_out(recipient); fund(&mut game, B, &[(ManaSymbol::Red, 1), (ManaSymbol::Green, 1)]);
            if kind == 1 {
                game.stack[0].source_snapshot = None;
                let error = resolve_stack_entry_with(&mut game, &mut dm).unwrap_err();
                assert!(matches!(&error, ironsmith::game_loop::GameLoopError::ExecutionFailed(
                    ironsmith::effects::ExecutionError::IncompleteEvidence(message))
                    if message.contains("exact retained source identity")), "{error:?}");
            } else {
                // A copied stack ability intentionally remaps its execution
                // source from its snapshot. Hold this context's source fixed.
                let wrong = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(unrelated).unwrap(), &game);
                let effect = game.stack[0].ability_effects.as_ref().unwrap().all_effects_owned().into_iter().next().unwrap();
                let mut context = EffectContext::new(recipient, B, &mut dm); context.source_snapshot = Some(wrong);
                let error = execute_effect(&mut game, &effect, &mut context).unwrap_err();
                assert!(matches!(&error, ironsmith::effects::ExecutionError::IncompleteEvidence(message)
                    if message.contains("exact retained source identity")), "{error:?}");
            }
            assert_eq!(game.player(B).unwrap().mana_pool.total(), 2);
        }
        assert!(game.battlefield.contains(&recipient));
        assert_eq!(game.stack.len(), 1); assert_eq!(game.stack[0].target_id(), stack_target);
        assert!(dm.offers.is_empty(), "incomplete execution cannot be presented as a payment decision");
        if kind == 0 {
            fund(&mut game, B, &[(ManaSymbol::Red, 1)]);
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(game.stack_is_empty()); assert!(game.battlefield.contains(&recipient));
            assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        }
    }}
}
#[test]
fn copied_trigger_gets_the_fresh_exact_snapshot_when_changed_source_phases_out() {
    for definition in definitions() {
        let mut game = game(); let recipient = host(&mut game, Some("{R}")); let donor = host(&mut game, Some("{G}{G}"));
        attach(&mut game, &definition, recipient); let mut dm = Choices::default(); assert_eq!(upkeep(&mut game, B, &mut dm), 1);
        let trigger = game.stack[0].target_id();
        execute_effect(&mut game, &Effect::copy_spell(ChooseSpec::SpecificObject(trigger)), &mut EffectContext::new(recipient, B, &mut dm)).unwrap();
        assert_eq!(game.stack.len(), 2); assert_ne!(game.stack[0].object_id, game.stack[1].object_id);
        copy_cost(&mut game, recipient, donor); game.phase_out(recipient); fund(&mut game, B, &[(ManaSymbol::Green, 4)]);
        for expected in [2, 0] { resolve_stack_entry_with(&mut game, &mut dm).unwrap(); assert_eq!(game.player(B).unwrap().mana_pool.total(), expected); }
        assert!(game.is_phased_out(recipient));
    }
}

#[test]
fn phase_in_uses_live_cost_again_and_blink_never_substitutes_the_returned_card() {
    for definition in definitions() { for blink in [false, true] {
        let mut game = game();
        let recipient = host(&mut game, Some("{R}"));
        let donor = host(&mut game, Some("{G}{G}"));
        attach(&mut game, &definition, recipient);
        let mut dm = Choices::default();
        assert_eq!(upkeep(&mut game, B, &mut dm), 1);
        let stable = game.object(recipient).unwrap().stable_id;
        let current = if blink {
            copy_cost(&mut game, recipient, donor);
            let exiled = game.move_object_by_effect(recipient, Zone::Exile).unwrap();
            let returned = game.move_object_by_effect(exiled, Zone::Battlefield).unwrap();
            assert_ne!(returned, recipient);
            assert_eq!(game.object(returned).unwrap().stable_id, stable);
            returned
        } else {
            game.phase_out(recipient); game.phase_in(recipient);
            copy_cost(&mut game, recipient, donor);
            recipient
        };
        // The live phased-in source and the departed blink incarnation both
        // require {G}{G}; the blinked return has its printed {R} cost instead.
        fund(&mut game, B, &[(ManaSymbol::Green, 2)]);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        assert!(game.battlefield.contains(&current));
    }}
}

#[test]
fn departed_face_down_absence_is_authoritative_and_decline_cannot_sacrifice_a_return() {
    for definition in definitions() { for absent_cost in [false, true] {
        let mut game = game(); let recipient = host(&mut game, Some("{R}"));
        attach(&mut game, &definition, recipient);
        let mut dm = Choices { pay: absent_cost, ..Default::default() };
        assert_eq!(upkeep(&mut game, B, &mut dm), 1);
        if absent_cost { assert!(game.set_face_down(recipient)); }
        let exiled = game.move_object_by_effect(recipient, Zone::Exile).unwrap();
        let returned = game.move_object_by_effect(exiled, Zone::Battlefield).unwrap();
        assert_ne!(returned, recipient);
        fund(&mut game, B, &[(ManaSymbol::Red, 1)]);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.battlefield.contains(&returned), "the old exact self-sacrifice cannot follow a stable card identity");
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 1,
            "a declined or absent old cost never pays the returned card's printed cost");
    }}
}

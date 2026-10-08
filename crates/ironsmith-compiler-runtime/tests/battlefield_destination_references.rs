//! UNRUN. Exact frozen bodies and native entry/source-reference scenarios.
//! The five additional fixture rows are explicitly partial, not admissions.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, TargetsContext};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
const COMPLETE: [&str; 3] = ["Charmed Griffin", "Endless Whispers", "Trove Warden"];

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/battlefield_destination_references.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text += &format!("Power/Toughness: {p}/{t}\n");
    }
    text += row["oracle_text"].as_str().unwrap();
    definitions_text(name, &text)
}

fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, artifact_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, text, false));
    assert!(!artifact_loss.is_lossy(), "{name}: {}", artifact_loss.reasons_text());
    let (artifact, _) = artifact.unwrap();
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct.unwrap(), ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for symbol in [ironsmith::ManaSymbol::White, ironsmith::ManaSymbol::Blue,
        ironsmith::ManaSymbol::Black, ironsmith::ManaSymbol::Red,
        ironsmith::ManaSymbol::Green, ironsmith::ManaSymbol::Colorless]
    { game.player_mut(A).unwrap().mana_pool.add(symbol, 20); }
    game
}

fn resource(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}

#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    legal_targets: Vec<Target>,
    selections: Vec<(PlayerId, Vec<ObjectId>)>,
    decline: Option<PlayerId>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, context: &BooleanContext) -> bool {
        self.decline != Some(context.player)
    }
    fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal = context.candidates.iter().filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id).collect::<Vec<_>>();
        self.selections.push((context.player, legal.clone()));
        legal.into_iter().take(context.max.unwrap_or(1)).collect()
    }
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.legal_targets.extend(context.requirements.iter().flat_map(|r| r.legal_targets.iter().copied()));
        self.targets.iter().copied().filter(|target|
            context.requirements.iter().any(|r| r.legal_targets.contains(target))).collect()
    }
}

fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) {
    let spell_id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell { spell_id, from_zone: Zone::Hand, casting_method: CastingMethod::Normal };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut state = PriorityLoopState::new(3);
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(!state.has_pending_action());
}

fn settle(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..32 {
        put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
        if game.stack.is_empty() { return; }
        resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("unexpected continuing program");
}

fn apply(game: &mut GameState, source: ObjectId, player: PlayerId, effect: Effect) {
    let outcome = execute_effect(game, &effect,
        &mut EffectContext::new(source, player, &mut SelectFirstDecisionMaker)).unwrap();
    for event in outcome.events { game.queue_trigger_event(event.provenance(), event); }
}

#[test]
fn full_frozen_bodies_keep_secondary_abilities_and_controller_source_text() {
    for name in COMPLETE {
        for definition in definitions(name) {
            let text = ironsmith_text::canonical_compiled_lines(&definition).join("\n").to_lowercase();
            assert!(!text.contains("unsupported"), "{name}: {text}");
            assert!(!text.contains("tagged-object-reference"), "{name}: {text}");
            match name {
                "Charmed Griffin" => {
                    assert!(text.contains("flying") && text.contains("hand") && text.contains("other player"), "{text}");
                    assert!(text.contains("artifact") && text.contains("enchantment"), "{text}");
                }
                "Endless Whispers" => {
                    assert!(text.contains("graveyard") && text.contains("opponent") && text.contains("end step"), "{text}");
                    assert!(text.contains("under their control") || text.contains("under that player's control"), "{text}");
                }
                "Trove Warden" => {
                    assert!(text.contains("vigilance") && text.contains("exiled with") && text.contains("owner"), "{text}");
                    assert!(text.contains("3 or less") && text.contains("graveyard"), "{text}");
                }
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn charmed_griffin_other_players_choose_only_their_own_hand_and_may_decline() {
    for definition in definitions("Charmed Griffin") {
        for decline in [None, Some(C)] {
            let mut game = game();
            let own = resource(&mut game, A, Zone::Hand, "Own artifact", "Type: Artifact");
            let b = resource(&mut game, B, Zone::Hand, "Bob artifact", "Type: Artifact");
            let c = resource(&mut game, C, Zone::Hand, "Cara enchantment", "Type: Enchantment");
            let creature = resource(&mut game, B, Zone::Hand, "Wrong type", "Type: Creature — Bear\nPower/Toughness: 2/2");
            let grave = resource(&mut game, B, Zone::Graveyard, "Wrong zone", "Type: Artifact");
            let bs = game.object(b).unwrap().stable_id;
            let cs = game.object(c).unwrap().stable_id;
            let mut dm = Choices { decline, ..Default::default() };
            cast(&mut game, &definition, &mut dm);
            settle(&mut game, &mut dm);
            let arrived_b = game.find_object_by_stable_id(bs).unwrap();
            let arrived_c = game.find_object_by_stable_id(cs).unwrap();
            assert_eq!(game.object(arrived_b).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.current_controller(arrived_b), Some(B));
            assert_eq!(game.object(arrived_c).unwrap().zone, if decline.is_some() { Zone::Hand } else { Zone::Battlefield });
            assert_eq!(game.object(own).unwrap().zone, Zone::Hand);
            assert_eq!(game.object(creature).unwrap().zone, Zone::Hand);
            assert_eq!(game.object(grave).unwrap().zone, Zone::Graveyard);
            for (player, candidates) in dm.selections {
                assert_ne!(player, A);
                assert!(!candidates.contains(&own) && !candidates.contains(&creature) && !candidates.contains(&grave));
                assert!(candidates.iter().all(|id| *id == if player == B { b } else { c }));
            }
        }
    }
}

#[test]
fn trove_warden_links_each_exile_and_returns_to_each_owner_after_control_changes() {
    for definition in definitions("Trove Warden") {
        for changed_incarnation in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let a = resource(&mut game, A, Zone::Graveyard, "Alice small", "Mana cost: {1}\nType: Artifact");
            let b = resource(&mut game, B, Zone::Graveyard, "Bob small", "Mana cost: {2}\nType: Enchantment");
            let wrong = resource(&mut game, A, Zone::Graveyard, "Too large", "Mana cost: {4}\nType: Artifact");
            let instant = resource(&mut game, A, Zone::Graveyard, "Not permanent", "Mana cost: {1}\nType: Instant\nYou gain 1 life.");
            let stable_a = game.object(a).unwrap().stable_id;
            let stable_b = game.object(b).unwrap().stable_id;
            for (owner, chosen) in [(A, a), (B, b)] {
                game.set_current_controller(source, owner).unwrap();
                let land = resource(&mut game, owner, Zone::Hand, "Landfall resource", "Type: Basic Land — Forest");
                apply(&mut game, source, owner, Effect::new(ironsmith::effects::MoveToZoneEffect::new(
                    ChooseSpec::SpecificObject(land), Zone::Battlefield, false).under_owner_control()));
                let mut dm = Choices { targets: vec![Target::Object(chosen)], ..Default::default() };
                settle(&mut game, &mut dm);
                assert!(dm.legal_targets.contains(&Target::Object(chosen)));
                assert!(!dm.legal_targets.contains(&Target::Object(wrong)));
                assert!(!dm.legal_targets.contains(&Target::Object(instant)));
            }
            if changed_incarnation {
                let exiled = game.find_object_by_stable_id(stable_b).unwrap();
                let hand = game.move_object_by_effect(exiled, Zone::Hand).unwrap();
                game.move_object_by_effect(hand, Zone::Exile).unwrap();
            }
            game.set_current_controller(source, C).unwrap();
            apply(&mut game, source, C, Effect::new(ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(source))));
            settle(&mut game, &mut Choices::default());
            let arrived_a = game.find_object_by_stable_id(stable_a).unwrap();
            let arrived_b = game.find_object_by_stable_id(stable_b).unwrap();
            assert_eq!(game.object(arrived_a).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.current_controller(arrived_a), Some(A));
            assert_eq!(game.object(arrived_b).unwrap().zone, if changed_incarnation { Zone::Exile } else { Zone::Battlefield });
            if !changed_incarnation { assert_eq!(game.current_controller(arrived_b), Some(B)); }
            assert_eq!(game.object(wrong).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.object(instant).unwrap().zone, Zone::Graveyard);
        }
    }
}

#[test]
fn endless_whispers_retains_dying_creatures_controller_opponent_and_exact_graveyard_arrival() {
    for definition in definitions("Endless Whispers") {
        for changed_incarnation in [false, true] {
            let mut game = game();
            let grant = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let victim = resource(&mut game, A, Zone::Battlefield, "Borrowed creature", "Type: Creature — Bear\nPower/Toughness: 2/2");
            let stable = game.object(victim).unwrap().stable_id;
            game.set_current_controller(victim, B).unwrap();
            apply(&mut game, grant, A, Effect::new(ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(victim))));
            let mut dm = Choices { targets: vec![Target::Player(C)], ..Default::default() };
            settle(&mut game, &mut dm);
            assert!(dm.legal_targets.contains(&Target::Player(A)) && dm.legal_targets.contains(&Target::Player(C)));
            assert!(!dm.legal_targets.contains(&Target::Player(B)), "the victim's controller chooses an opponent");
            let grave = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(grave).unwrap().zone, Zone::Graveyard);
            game.move_object_by_effect(grant, Zone::Graveyard).unwrap();
            if changed_incarnation {
                let hand = game.move_object_by_effect(grave, Zone::Hand).unwrap();
                game.move_object_by_effect(hand, Zone::Graveyard).unwrap();
            }
            game.turn.phase = ironsmith::Phase::Ending;
            game.turn.step = Some(ironsmith::game_state::Step::End);
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::generate_and_queue_step_triggers(&mut game, &mut queue);
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            settle(&mut game, &mut dm);
            let final_id = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(final_id).unwrap().zone, if changed_incarnation { Zone::Graveyard } else { Zone::Battlefield });
            assert_eq!(game.object(final_id).unwrap().owner, A);
            if !changed_incarnation { assert_eq!(game.current_controller(final_id), Some(C)); }
        }
    }
}

#[test]
fn relative_put_refuses_unknown_tails_and_unowned_resolution_choices() {
    for tail in [
        "it {R} onto the battlefield under their control",
        "this card {R} onto the battlefield under their control",
        "this card from your graveyard {R} onto the battlefield under their control",
        "those cards {R} onto the battlefield under their control",
        "it {R} onto the battlefield under the control of that card's owner",
        "those cards {R} onto the battlefield under the control of that card's owner",
        "it onto the battlefield under their control {R}",
        "it onto the battlefield under their control instead draw a card",
        "it onto the battlefield under their control and the rest into your graveyard",
        "a creature card from your graveyard onto the battlefield under their control",
        "one of them onto the battlefield under their control",
        "this card from its owner's graveyard {R} onto the battlefield under their control",
        "it onto the battlefield under the control of that card's owner and the rest into your graveyard",
        "it onto the battlefield under its owner's control and the rest into your graveyard",
        "all creature cards from your graveyard onto the battlefield attacking under the control of that card's owner",
        "each creature card from your graveyard onto the battlefield tapped and attacking under its owner's control",
        "all Aura cards from your graveyard onto the battlefield under the control of that card's owner attached to target creature",
        "each Aura card from your graveyard onto the battlefield under your control attached to target creature",
    ] {
        let text = format!("Type: Sorcery\nChoose target opponent. That player puts {tail}.");
        assert!(compile_to_runtime_definition("Invalid destination", &text, false).is_err(), "accepted {tail}");
        assert!(compile_to_artifact("Invalid destination", &text, false).is_err(), "artifact accepted {tail}");
    }
}

#[test]
fn contextual_collection_controllers_are_not_dropped_by_view_or_exile_readers() {
    for controller in ["their", "that player's"] {
        for view in ["Look at", "Reveal", "Exile"] {
            for remainder in ["", " Put the rest on the bottom of your library in any order."] {
                let text = format!("Type: Sorcery\nChoose target opponent. {view} the top three cards of your library. You may put a creature card from among them onto the battlefield under {controller} control.{remainder}");
                assert!(compile_to_runtime_definition("Unsupported collection controller", &text, false).is_err(), "accepted {text}");
                assert!(compile_to_artifact("Unsupported collection controller", &text, false).is_err(), "artifact accepted {text}");
            }
        }
    }
}

#[test]
fn announced_relative_entry_uses_distinct_player_and_card_targets() {
    let text = "Mana cost: {0}\nType: Sorcery\nTarget opponent puts target creature card from your graveyard onto the battlefield under their control. It gains haste.";
    for definition in definitions_text("Announced relative entry", text) {
        let mut game = game();
        let own = resource(&mut game, A, Zone::Graveyard, "Own creature", "Type: Creature — Bear\nPower/Toughness: 2/2");
        let foreign = resource(&mut game, B, Zone::Graveyard, "Foreign creature", "Type: Creature — Bear\nPower/Toughness: 2/2");
        let stable = game.object(own).unwrap().stable_id;
        let mut dm = Choices { targets: vec![Target::Player(C), Target::Object(own)], ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        assert!(dm.legal_targets.contains(&Target::Player(B)) && dm.legal_targets.contains(&Target::Player(C)));
        assert!(dm.legal_targets.contains(&Target::Object(own)));
        assert!(!dm.legal_targets.contains(&Target::Object(foreign)));
        settle(&mut game, &mut dm);
        let arrived = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(arrived).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.current_controller(arrived), Some(C));
        assert_eq!(game.object(arrived).unwrap().owner, A);
        assert!(game.object_has_static_ability_id(arrived, ironsmith::static_abilities::StaticAbilityId::Haste));
        assert_eq!(game.object(foreign).unwrap().zone, Zone::Graveyard);
    }
}

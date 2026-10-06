//! Exact frozen bodies, typed artifact transport, and public gameplay scenarios.
//! Authored only. No compiler or engine execution has been performed.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, NumberContext, TargetsContext};
use ironsmith::effect::{Effect, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, generate_and_queue_step_triggers,
    put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::special_actions::{self, ActionError, SpecialAction};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, Phase, PlayerId, Step, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{StaticAbilityPayload, SuspendTime, TriggerKind};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/dynamic_characteristic_statics.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|row| row["name"] == name).unwrap();
    let mut lines = vec![format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap())];
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().to_string());
    let text = lines.join("\n");
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}
fn witness(game: &mut GameState, owner: PlayerId, zone: Zone, types: &str) -> ObjectId {
    let pt = if types.contains("Creature") { "\nPower/Toughness: 1/2" } else { "" };
    let definition = compile_to_runtime_definition("State witness",
        format!("Mana cost: {{0}}\nType: {types}{pt}"), false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn library(game: &mut GameState, player: PlayerId, count: usize) {
    for _ in 0..count { witness(game, player, Zone::Library, "Artifact"); }
}
fn pt(game: &GameState, object: ObjectId) -> (i32, i32) {
    let chars = game.try_current_characteristics(object).unwrap().unwrap();
    (chars.power.unwrap(), chars.toughness.unwrap())
}
fn base_pt(game: &GameState, object: ObjectId) -> (i32, i32) {
    let chars = game.try_current_characteristics(object).unwrap().unwrap();
    (chars.base_power.unwrap(), chars.base_toughness.unwrap())
}
fn apply(game: &mut GameState, source: ObjectId, controller: PlayerId, effect: Effect) {
    execute_effect(game, &effect,
        &mut EffectContext::new(source, controller, &mut SelectFirstDecisionMaker)).unwrap();
}
fn set_life(game: &mut GameState, source: ObjectId, player: PlayerId, life: i32) {
    apply(game, source, A, Effect::set_life_total_player(life, PlayerFilter::Specific(player)));
}
#[derive(Default)]
struct Choices {
    target: Option<Target>,
    forbidden: Vec<Target>,
    accept: bool,
    x: u32,
    number_bounds: Vec<(u32, u32)>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.accept }
    fn decide_number(&mut self, game: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value {
            self.number_bounds.push((context.min, context.max));
            self.x
        } else { SelectFirstDecisionMaker.decide_number(game, context) }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert_eq!(context.requirements.len(), 1);
            let requirement = &context.requirements[0];
            assert_eq!((requirement.min_targets, requirement.max_targets), (1, Some(1)));
            assert!(requirement.legal_targets.contains(&target), "{context:?}");
            for forbidden in &self.forbidden {
                assert!(!requirement.legal_targets.contains(forbidden), "{context:?}");
            }
            vec![target]
        } else { SelectFirstDecisionMaker.decide_targets(game, context) }
    }
}
fn action(game: &mut GameState, player: PlayerId, action: LegalAction, choices: &mut Choices) {
    game.turn.priority_player = Some(player);
    assert!(compute_legal_actions(game, player).unwrap().contains(&action));
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
fn queue(game: &mut GameState, choices: &mut Choices) {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), choices).unwrap();
}
fn resolve_all(game: &mut GameState, choices: &mut Choices) {
    for _ in 0..32 {
        queue(game, choices);
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("bounded full-body scenario did not settle");
}
fn equip(game: &mut GameState, definition: &CardDefinition, source: ObjectId, target: ObjectId,
    forbidden: ObjectId)
{
    let ability_index = definition.abilities.iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
    let mut choices = Choices { target: Some(Target::Object(target)),
        forbidden: vec![Target::Object(forbidden)], ..Default::default() };
    action(game, A, LegalAction::ActivateAbility { source, ability_index }, &mut choices);
    resolve_all(game, &mut choices);
}

#[test]
fn four_complete_bodies_preserve_static_layers_and_all_other_abilities() {
    assert_eq!(fixtures().len(), 4);
    for row in fixtures() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let static_abilities: Vec<_> = definition.abilities.iter().filter_map(|ability| {
                if let AbilityKind::Static(rule) = &ability.kind { Some((ability, rule)) } else { None }
            }).collect();
            if name == "Aettir and Priwen" || name == "Angry Mob" {
                assert!(static_abilities.iter().all(|(_, rule)| rule.id() != StaticAbilityId::CharacteristicDefiningPT));
                let count = static_abilities.iter().filter(|(_, rule)| rule.id() == StaticAbilityId::SetBasePowerToughnessForFilter).count();
                assert_eq!(count, if name == "Angry Mob" { 2 } else { 1 });
                assert!(static_abilities.iter().filter(|(_, rule)| rule.id() == StaticAbilityId::SetBasePowerToughnessForFilter)
                    .all(|(ability, _)| ability.functional_zones == vec![Zone::Battlefield]));
            } else {
                let (ability, rule) = static_abilities.iter().find(|(_, rule)| rule.id() == StaticAbilityId::CharacteristicDefiningPT).unwrap();
                for zone in [Zone::Hand, Zone::Stack, Zone::Battlefield, Zone::Exile, Zone::Graveyard] {
                    assert!(ability.functional_zones.contains(&zone));
                }
                assert!(matches!(rule.compiled_model().unwrap().payload, StaticAbilityPayload::CharacteristicDefiningPt { .. }));
            }
            if name == "Roiling Horror" {
                assert!(matches!(definition.alternative_casts.as_slice(),
                    [ironsmith::AlternativeCastingMethod::Suspend { time: SuspendTime::X { minimum: 1 }, cost }] if cost.has_x()));
                let body = definition.abilities.iter().find(|ability| {
                    matches!(&ability.kind, AbilityKind::Triggered(body)
                        if matches!(body.trigger.compiled_model().map(|trigger| &trigger.kind),
                            Some(TriggerKind::CounterRemovedFrom(trigger)) if !trigger.last))
                }).unwrap();
                assert_eq!(body.functional_zones, vec![Zone::Exile]);
                let AbilityKind::Triggered(body) = &body.kind else { unreachable!() };
                assert!(body.intervening_if.is_none());
            }
        }
    }
}

#[test]
fn complete_card_dispatch_rejects_unconsumed_static_subjects_values_and_timing() {
    for oracle in [
        "Equipped creature has base power and toughness X/X, where X is your life total and draws a card.\nEquip {5}",
        "Equipped creature has base power and toughness X/X, where X is your life {R} total.\nEquip {5}",
        "Unrecognized equipped creature has base power and toughness X/X, where X is your life total.\nEquip {5}",
        "During turns other than yours nonsense, this creature's power and toughness are each equal to 7.",
        "During turns other than yours, equipped creature's power and toughness are each equal to 7.",
    ] {
        let text = format!("Mana cost: {{1}}\nType: Artifact Creature — Construct\nPower/Toughness: 2/3\n{oracle}");
        let (result, loss) = ironsmith_compiler::parse_loss::capture(||
            compile_to_runtime_definition("Incomplete setting", &text, false));
        assert!(result.is_err() || loss.is_lossy(), "must not silently recover: {oracle}");
    }
}

#[test]
fn aettir_paid_equip_reads_equipment_controller_life_and_tracks_attachment_lifetime() {
    for definition in definitions("Aettir and Priwen") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = witness(&mut game, A, Zone::Battlefield, "Creature — Human");
        let second = witness(&mut game, A, Zone::Battlefield, "Creature — Human");
        let enemy = witness(&mut game, B, Zone::Battlefield, "Creature — Human");
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 10);
        equip(&mut game, &definition, source, first, enemy);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 5);
        assert_eq!(pt(&game, first), (20, 20));
        assert_eq!(pt(&game, second), (1, 2));
        assert_eq!(base_pt(&game, first), (20, 20));
        apply(&mut game, source, A, Effect::pump(3, 4, ChooseSpec::SpecificObject(first), Until::EndOfTurn));
        game.add_counters(first, CounterType::PlusOnePlusOne, 2).unwrap();
        assert_eq!(pt(&game, first), (25, 26));
        set_life(&mut game, source, A, 7);
        assert_eq!(pt(&game, first), (12, 13));
        assert_eq!(base_pt(&game, first), (7, 7));
        game.set_current_controller(first, B).unwrap();
        set_life(&mut game, source, B, 4);
        assert_eq!(pt(&game, first), (12, 13), "host controller is not the life owner");
        equip(&mut game, &definition, source, second, first);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(pt(&game, first), (6, 8));
        assert_eq!(pt(&game, second), (7, 7));
        set_life(&mut game, source, C, 9);
        game.set_current_controller(source, C).unwrap();
        assert_eq!(pt(&game, second), (9, 9));
        game.phase_out(source);
        assert_eq!(pt(&game, second), (1, 2));
        game.phase_in(source);
        assert_eq!(pt(&game, second), (9, 9));
        let stable = game.object(source).unwrap().stable_id;
        let grave = game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
        assert_eq!(pt(&game, second), (1, 2));
        let returned = game.move_object_by_effect(grave, Zone::Battlefield).unwrap();
        assert_eq!(game.find_object_by_stable_id(stable), Some(returned));
        assert_ne!(returned, source);
        assert_eq!(pt(&game, second), (1, 2), "a new equipment incarnation is unattached");
    }
}

#[test]
fn aettir_setting_obeys_later_layer_seven_b_overrides_and_ability_removal() {
    for definition in definitions("Aettir and Priwen") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let host = witness(&mut game, A, Zone::Battlefield, "Creature — Human");
        apply(&mut game, source, A, Effect::attach_objects(ChooseSpec::SpecificObject(source), ChooseSpec::SpecificObject(host)));
        assert_eq!(pt(&game, host), (20, 20));
        apply(&mut game, source, A, Effect::set_base_power_toughness(4, 6,
            ChooseSpec::SpecificObject(host), Until::EndOfTurn));
        game.add_counters(host, CounterType::PlusOnePlusOne, 2).unwrap();
        assert_eq!(pt(&game, host), (6, 8));
        set_life(&mut game, source, A, 13);
        assert_eq!(base_pt(&game, host), (4, 6), "a live earlier setting cannot outrank a later setting");
        game.next_turn();
        assert_eq!(pt(&game, host), (15, 15));
        game.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::new(
            source, A, ironsmith::continuous::EffectTarget::Specific(source),
            ironsmith::continuous::Modification::RemoveAllAbilities).until(Until::EndOfTurn));
        assert_eq!(pt(&game, host), (3, 4));
    }
}

#[test]
fn angry_mob_keeps_both_turn_conditions_opponent_scope_and_battlefield_only_setting() {
    for definition in definitions("Angry Mob") {
        let mut game = game();
        game.set_teams(vec![vec![A, C], vec![B]]).unwrap();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        witness(&mut game, A, Zone::Battlefield, "Basic Land — Swamp");
        witness(&mut game, C, Zone::Battlefield, "Basic Land — Swamp");
        let first = witness(&mut game, B, Zone::Battlefield, "Basic Land — Swamp");
        witness(&mut game, B, Zone::Battlefield, "Land — Swamp");
        witness(&mut game, B, Zone::Graveyard, "Basic Land — Swamp");
        assert_eq!(pt(&game, source), (4, 4));
        assert!(game.current_has_static_ability_id(source, StaticAbilityId::Trample));
        game.phase_out(first);
        assert_eq!(pt(&game, source), (3, 3));
        game.phase_in(first);
        assert_eq!(pt(&game, source), (4, 4));
        game.add_counters(source, CounterType::PlusOnePlusOne, 1).unwrap();
        apply(&mut game, source, A, Effect::pump(2, 3, ChooseSpec::SpecificObject(source), Until::EndOfTurn));
        assert_eq!(pt(&game, source), (7, 8));
        assert_eq!(base_pt(&game, source), (4, 4));
        game.next_turn();
        assert_eq!(game.turn.active_player, B);
        assert_eq!(pt(&game, source), (3, 3));
        game.set_current_controller(source, B).unwrap();
        assert_eq!(pt(&game, source), (5, 5), "your turn follows the current source controller");
        let hand = game.move_object_by_effect(source, Zone::Hand).unwrap();
        assert_eq!(pt(&game, hand), (2, 2), "conditional setting is not an all-zone CDA");
    }
}

#[test]
fn roiling_horror_live_cda_uses_opponent_maximum_in_every_zone_and_checked_signed_values() {
    for definition in definitions("Roiling Horror") {
        for zone in [Zone::Hand, Zone::Library, Zone::Graveyard, Zone::Exile, Zone::Stack, Zone::Battlefield] {
            let mut game = game();
            game.set_teams(vec![vec![A, C], vec![B]]).unwrap();
            game.player_mut(B).unwrap().life = 14;
            game.player_mut(C).unwrap().life = 100;
            let source = game.create_object_from_definition(&definition, A, zone);
            assert_eq!(pt(&game, source), (6, 6));
            game.player_mut(A).unwrap().life = 8;
            assert_eq!(pt(&game, source), (-6, -6));
            game.player_mut(A).unwrap().life = i32::MIN;
            game.player_mut(B).unwrap().life = i32::MIN;
            assert_eq!(pt(&game, source), (0, 0), "the negated operand may exceed i32 before subtraction");
            game.player_mut(A).unwrap().life = i32::MAX;
            assert!(game.try_current_characteristics(source).is_err(), "unrepresentable difference cannot wrap");
            assert!(game.refresh_continuous_state().is_err());
            game.player_mut(B).unwrap().life = i32::MAX - 2;
            assert_eq!(pt(&game, source), (2, 2), "failed publication cannot poison later checked reads");
        }
    }
}

fn remove_time(game: &mut GameState, object: ObjectId, count: u32) {
    let (removed, event) = game.remove_counters(object, CounterType::Time, count, Some(object), Some(B)).unwrap();
    assert_eq!(removed, count);
    game.queue_trigger_event(Default::default(), event);
}

#[test]
fn roiling_paid_suspend_enforces_positive_x_and_last_counter_preserves_both_triggers() {
    for definition in definitions("Roiling Horror") {
        let mut game = game();
        game.player_mut(A).unwrap().life = 24;
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 3);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
        game.player_mut(C).unwrap().life = 10;
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(source).unwrap().stable_id;
        let suspend = SpecialAction::Suspend { card_id: source };
        assert_eq!(special_actions::perform(suspend.clone(), &mut game, A,
            &mut Choices::default()), Err(ActionError::InvalidTarget));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 4);
        assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
        let mut choices = Choices { x: 1, target: Some(Target::Player(B)), accept: true, ..Default::default() };
        special_actions::perform(suspend, &mut game, A, &mut choices).unwrap();
        assert_eq!(choices.number_bounds[0].0, 1);
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        assert_eq!(game.counter_count(exiled, CounterType::Time), 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(game.object(exiled).unwrap().x_value.is_none());
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        let mut triggers = TriggerQueue::new();
        generate_and_queue_step_triggers(&mut game, &mut triggers);
        put_triggers_on_stack_with_dm(&mut game, &mut triggers, &mut choices).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        queue(&mut game, &mut choices);
        assert_eq!(game.stack.len(), 2, "the last removal causes both suspend-cast and the authored body");
        resolve_all(&mut game, &mut choices);
        let entered = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.player(A).unwrap().life, 25);
        assert_eq!(game.player(B).unwrap().life, 19);
        assert_eq!(pt(&game, entered), (6, 6));
        assert!(game.current_has_static_ability_id(entered, StaticAbilityId::Haste));
    }
}

#[test]
fn roiling_counter_body_survives_source_departure_and_fires_once_per_removed_counter() {
    for definition in definitions("Roiling Horror") {
        let mut game = game();
        game.player_mut(A).unwrap().life = 24;
        game.player_mut(B).unwrap().life = 18;
        game.player_mut(C).unwrap().life = 30;
        let source = game.create_object_from_definition(&definition, A, Zone::Exile);
        game.add_counters(source, CounterType::Time, 3).unwrap();
        game.take_pending_trigger_events();
        remove_time(&mut game, source, 2);
        let mut choices = Choices { target: Some(Target::Player(B)), ..Default::default() };
        queue(&mut game, &mut choices);
        assert_eq!(game.stack.len(), 2);
        let returned = game.move_object_by_effect(source, Zone::Battlefield).unwrap();
        game.set_current_controller(returned, C).unwrap();
        resolve_all(&mut game, &mut choices);
        assert_eq!(game.player(A).unwrap().life, 26);
        assert_eq!(game.player(B).unwrap().life, 16);
        assert_eq!(game.player(C).unwrap().life, 30, "pending body retains its trigger controller");
        game.add_counters(returned, CounterType::Time, 2).unwrap();
        game.take_pending_trigger_events();
        remove_time(&mut game, returned, 1);
        queue(&mut game, &mut choices);
        assert!(game.stack_is_empty(), "a battlefield counter removal does not qualify");
    }
}

fn crime_spell(game: &mut GameState, target: PlayerId, choices: &mut Choices) {
    let spell = compile_to_runtime_definition("Crime witness",
        "Mana cost: {0}\nType: Instant\nTarget player loses 1 life.", false).unwrap();
    let spell_id = game.create_object_from_definition(&spell, A, Zone::Hand);
    choices.target = Some(Target::Player(target));
    action(game, A, LegalAction::CastSpell { spell_id, from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal }, choices);
}

#[test]
fn duelist_full_crime_loot_is_optional_capped_and_excludes_teammates() {
    for definition in definitions("Duelist of the Mind") {
        for accept in [false, true] {
            let mut game = game();
            game.set_teams(vec![vec![A, C], vec![B]]).unwrap();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            library(&mut game, A, 6);
            library(&mut game, B, 6);
            let mut choices = Choices { accept, ..Default::default() };
            assert_eq!(pt(&game, source), (0, 3));
            assert!(game.current_has_static_ability_id(source, StaticAbilityId::Flying));
            assert!(game.current_has_static_ability_id(source, StaticAbilityId::Vigilance));
            crime_spell(&mut game, C, &mut choices);
            assert_eq!(game.stack.len(), 1, "targeting your teammate is not a crime");
            resolve_all(&mut game, &mut choices);
            crime_spell(&mut game, B, &mut choices);
            assert_eq!(game.stack.len(), 2);
            resolve_all(&mut game, &mut choices);
            assert!(game.player(A).unwrap().hand.is_empty(), "acceptance draws then discards; decline does neither");
            assert_eq!(pt(&game, source), (i32::from(accept), 3));
            assert_eq!(game.player(A).unwrap().library.len(), if accept { 5 } else { 6 });
            crime_spell(&mut game, B, &mut choices);
            assert_eq!(game.stack.len(), 1, "the cap is consumed even when the optional draw is declined");
            resolve_all(&mut game, &mut choices);
            apply(&mut game, source, B, Effect::draw(3));
            assert_eq!(pt(&game, source), (i32::from(accept), 3), "opponent draws do not enlarge your Duelist");
            game.set_current_controller(source, B).unwrap();
            assert_eq!(pt(&game, source), (3, 3));
            game.next_turn();
            assert_eq!(pt(&game, source), (0, 3));
            game.set_current_controller(source, A).unwrap();
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            choices.accept = true;
            crime_spell(&mut game, B, &mut choices);
            assert_eq!(game.stack.len(), 2, "the trigger cap resets next turn");
            resolve_all(&mut game, &mut choices);
            assert_eq!(pt(&game, source), (1, 3));
            let grave = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            assert_eq!(pt(&game, grave), (1, 3), "the CDA functions outside the battlefield");
        }
    }
}

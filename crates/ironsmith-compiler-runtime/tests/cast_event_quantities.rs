//! Full frozen cards and completed-cast quantity boundaries, authored and unrun.
use std::collections::VecDeque;

use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::LinkedFaceLayout;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{NumberContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::{Effect, Value};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::events::spells::SpellCastEvent;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{Color, GameProgress, GameState, ObjectId, Phase, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{CastEventQuantity, EventValueSpec};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/cast_event_quantities.json.fixture")).unwrap()
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().to_owned());
    definitions_text(name, &lines.join("\n"))
}

fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    let direct = direct.unwrap_or_else(|error| panic!("{name} direct: {error}"));
    assert!(!direct_loss.is_lossy(), "{name} direct: {}", direct_loss.reasons_text());
    let (artifact, artifact_loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, text, false)
    });
    let (artifact, _) = artifact.unwrap_or_else(|error| panic!("{name} artifact: {error}"));
    assert!(!artifact_loss.is_lossy(), "{name} artifact: {}", artifact_loss.reasons_text());
    artifact.validate().unwrap();
    let wire = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, wire);
    [direct, materialize_artifact(&wire).unwrap()]
}

fn resource(name: &str, text: &str) -> CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap()
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for player in [A, B] {
        for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black,
            ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 30);
        }
    }
    game
}

#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    target_groups: VecDeque<Vec<Target>>,
    x: u32,
    prefer_life: bool,
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if self.prefer_life && ctx.description.starts_with("Choose how to pay pip") {
            if let Some(option) = ctx.options.iter().find(|option| {
                option.legal && option.description.to_ascii_lowercase().contains("life")
            }) {
                return vec![option.index];
            }
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }

    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            assert!(self.x <= ctx.max);
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, ctx)
        }
    }

    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        ctx.requirements.iter().flat_map(|requirement| {
            let preferred = self.target_groups.pop_front().unwrap_or_else(|| self.targets.clone());
            let chosen: Vec<_> = preferred.into_iter()
                .filter(|target| requirement.legal_targets.contains(target))
                .take(requirement.max_targets.unwrap_or(usize::MAX))
                .collect();
            if chosen.len() >= requirement.min_targets {
                chosen
            } else {
                SelectFirstDecisionMaker.decide_targets(game, &TargetsContext::new(
                    ctx.player, ctx.source, "cast quantity fallback", vec![requirement.clone()],
                ))
            }
        }).collect()
    }
}

fn cast(
    game: &mut GameState,
    definition: &CardDefinition,
    caster: PlayerId,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    game.turn.active_player = caster;
    game.turn.priority_player = Some(caster);
    let id = game.create_object_from_definition(definition, caster, Zone::Hand);
    let stable = game.object(id).unwrap().stable_id;
    let action = LegalAction::CastSpell { spell_id: id, from_zone: Zone::Hand, casting_method: method };
    assert!(compute_legal_actions(game, caster).unwrap().contains(&action));
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm,
    ).unwrap();
    for _ in 0..80 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("unfinished cast without a decision: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    game.find_object_by_stable_id(stable).unwrap()
}

fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}

fn settle(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..30 {
        if game.stack_is_empty() { return; }
        resolve(game, dm);
    }
    panic!("cast quantity scenario did not settle");
}

fn apply(game: &mut GameState, source: ObjectId, effect: Effect) {
    let mut dm = SelectFirstDecisionMaker;
    execute_effect(game, &effect, &mut EffectContext::new(source, A, &mut dm)).unwrap();
}

fn enter_doom(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let spell = cast(game, definition, A, CastingMethod::Normal, dm);
    let stable = game.object(spell).unwrap().stable_id;
    settle(game, dm);
    let source = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.counter_count(source, CounterType::Doom), 1);
    source
}

fn merfolk(game: &GameState, controller: PlayerId) -> Vec<ObjectId> {
    game.battlefield.iter().copied().filter(|id| {
        game.object(*id).is_some_and(|object| object.kind == ObjectKind::Token)
            && game.current_controller(*id) == Some(controller)
            && game.calculated_subtypes(*id).contains(&Subtype::Merfolk)
    }).collect()
}

fn fill_library(game: &mut GameState, player: PlayerId) {
    let card = resource("Library resource", "Type: Basic Land — Island");
    for _ in 0..12 { game.create_object_from_definition(&card, player, Zone::Library); }
}

fn creature(game: &mut GameState, player: PlayerId) -> ObjectId {
    game.create_object_from_definition(&resource(
        "Target resource", "Type: Creature — Human\nPower/Toughness: 2/5",
    ), player, Zone::Battlefield)
}

fn quantity(quantity: CastEventQuantity) -> Value {
    Value::EventValue(EventValueSpec::CastSpell(quantity))
}

fn captured_event(game: &GameState) -> TriggerEvent {
    game.stack.iter().rev().find_map(|entry| entry.triggering_event.clone()).unwrap()
}

#[test]
fn all_three_complete_frozen_cards_round_trip_with_specific_cast_quantity_owners() {
    assert_eq!(rows().len(), 3);
    for row in rows() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert_eq!(definition.card.name, name);
            let debug = format!("{definition:?}");
            let expected = match name {
                "Imminent Doom" => "CastSpell(ManaValue)",
                "Namor the Sub-Mariner" => "CastSpell(ManaSymbols(Blue))",
                "Voracious Bibliophile" => "CastSpell(DistinctTargets)",
                _ => unreachable!(),
            };
            assert!(debug.contains(expected), "{name}: {debug}");
            assert!(!debug.contains("PendingPriorEffectMetric"), "{name}: {debug}");
            let cast_filter = definition.abilities.iter().find_map(|ability| {
                let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind else { return None; };
                triggered.trigger.downcast_ref::<ironsmith::triggers::SpellCastTrigger>()
                    .and_then(|cast| cast.filter.as_ref())
            }).expect("the full body retains its cast trigger");
            assert!(cast_filter.has_only_completed_cast_characteristics(), "{name}: {cast_filter:?}");
        }
    }
}

#[test]
fn an_unqualified_cast_trigger_does_not_supply_an_arbitrary_event_amount() {
    for text in [
        "Type: Enchantment\nWhenever you cast a spell, draw that many cards.",
        "Type: Enchantment\nWhenever you cast a creature spell, you gain that much life.",
    ] {
        assert!(compile_to_runtime_definition("Missing cast quantity antecedent", text, false).is_err());
    }
}

#[test]
fn doom_enters_with_one_and_keeps_captured_x_mana_value_after_counter_change_and_spell_departure() {
    for definition in definitions("Imminent Doom") {
        let mut game = game();
        let mut dm = Choices { targets: vec![Target::Player(B)], x: 3, ..Default::default() };
        let source = enter_doom(&mut game, &definition, &mut dm);
        apply(&mut game, source, Effect::put_counters(
            CounterType::Doom, 4, ChooseSpec::SpecificObject(source),
        ));
        let spell = cast(&mut game, &resource(
            "Announced X", "Mana cost: {X}{2}\nType: Instant\nYou gain 1 life.",
        ), A, CastingMethod::Normal, &mut dm);
        assert_eq!(game.stack.len(), 2);
        assert_eq!(game.object(spell).unwrap().x_value, Some(3));
        apply(&mut game, source, Effect::put_counters(
            CounterType::Doom, 4, ChooseSpec::SpecificObject(source),
        ));
        apply(&mut game, source, Effect::counter(ChooseSpec::SpecificObject(spell)));
        settle(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, 15, "the original cast had mana value five");
        assert_eq!(game.counter_count(source, CounterType::Doom), 10, "the counter tail still executes");
    }
}

#[test]
fn doom_tests_the_live_count_only_at_cast_time_and_filters_the_caster() {
    for definition in definitions("Imminent Doom") {
        for (caster, cost, expected_triggers) in [(A, "{1}", 1), (A, "{2}", 0), (B, "{1}", 0)] {
            let mut game = game();
            let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
            let source = enter_doom(&mut game, &definition, &mut dm);
            let spell = cast(&mut game, &resource(
                "Cast predicate", &format!("Mana cost: {cost}\nType: Instant\nYou gain 1 life."),
            ), caster, CastingMethod::Normal, &mut dm);
            assert_eq!(game.stack.len(), 1 + expected_triggers);
            apply(&mut game, source, Effect::counter(ChooseSpec::SpecificObject(spell)));
            settle(&mut game, &mut dm);
            assert_eq!(game.player(B).unwrap().life, 20 - expected_triggers as i32);
            assert_eq!(game.counter_count(source, CounterType::Doom), 1 + expected_triggers as u32);
        }
    }
}

#[test]
fn doom_departure_and_blink_do_not_redirect_the_original_counter_tail() {
    for definition in definitions("Imminent Doom") {
        let mut game = game();
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        let source = enter_doom(&mut game, &definition, &mut dm);
        let spell = cast(&mut game, &resource(
            "One mana", "Mana cost: {1}\nType: Instant\nYou gain 1 life.",
        ), A, CastingMethod::Normal, &mut dm);
        apply(&mut game, source, Effect::counter(ChooseSpec::SpecificObject(spell)));
        let departed = game.move_object_by_game_rule(source, Zone::Exile).unwrap();
        let returned = game.move_object_by_game_rule(departed, Zone::Battlefield).unwrap();
        assert_ne!(returned, source);
        let before = game.counter_count(returned, CounterType::Doom);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, 19);
        assert_eq!(game.counter_count(returned, CounterType::Doom), before);
    }
}

#[test]
fn namor_counts_actual_cast_face_hybrid_and_phyrexian_symbols_independently_of_payment() {
    for definition in definitions("Namor the Sub-Mariner") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(game.current_power(source), Some(1));
        assert_eq!(game.current_toughness(source), Some(4));
        assert!(game.current_has_static_ability_id(source, StaticAbilityId::Flying));
        let mut front = resource("Four blue front", "Mana cost: {U}{U}{U}{U}\nType: Sorcery\nYou gain 1 life.");
        let mut back = resource("Two blue back", "Mana cost: {3}{U/R}{U/P}\nType: Sorcery\nYou gain 1 life.");
        front.card.other_face = Some(back.card.id);
        front.card.other_face_name = Some(back.card.name.clone());
        front.card.linked_face_layout = LinkedFaceLayout::Split;
        back.card.other_face = Some(front.card.id);
        back.card.other_face_name = Some(front.card.name.clone());
        back.card.linked_face_layout = LinkedFaceLayout::Split;
        game.register_linked_face_definition(&back);
        game.player_mut(A).unwrap().mana_pool = Default::default();
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red, 1);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
        let mut dm = Choices { prefer_life: true, ..Default::default() };
        let spell = cast(&mut game, &front, A, CastingMethod::SplitOtherHalf, &mut dm);
        assert_eq!(game.object(spell).unwrap().name, "Two blue back");
        assert_eq!(game.object(spell).unwrap().mana_spent_to_cast.blue, 0);
        assert_eq!(game.object(spell).unwrap().mana_spent_to_cast.total(), 4);
        assert_eq!(game.player(A).unwrap().life, 18);
        assert_eq!(game.stack.len(), 2);
        apply(&mut game, source, Effect::counter(ChooseSpec::SpecificObject(spell)));
        settle(&mut game, &mut dm);
        let made = merfolk(&game, A);
        assert_eq!(made.len(), 2, "one per blue-containing printed pip of the selected face");
        for id in &made {
            assert_eq!(game.current_power(*id), Some(1));
            assert_eq!(game.current_toughness(*id), Some(1));
            assert!(game.current_colors(*id).unwrap().contains(Color::Blue));
        }
        assert_eq!(game.current_power(source), Some(3));
        game.move_object_by_game_rule(made[0], Zone::Exile).unwrap();
        assert_eq!(game.current_power(source), Some(2), "Namor's characteristic remains dynamic");
    }
}

#[test]
fn namor_rejects_other_casters_creatures_and_zero_blue_symbols() {
    for definition in definitions("Namor the Sub-Mariner") {
        for (caster, text) in [
            (B, "Mana cost: {U}{U}\nType: Instant\nYou gain 1 life."),
            (A, "Mana cost: {U}{U}\nType: Creature — Human\nPower/Toughness: 1/1"),
            (A, "Mana cost: {2}\nType: Instant\nYou gain 1 life."),
        ] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut dm = Choices::default();
            cast(&mut game, &resource("Nonmatching cast", text), caster, CastingMethod::Normal, &mut dm);
            assert_eq!(game.stack.len(), 1);
            settle(&mut game, &mut dm);
            assert!(merfolk(&game, A).is_empty());
        }
    }
}

#[test]
fn namor_uses_captured_controller_and_symbols_after_source_and_spell_depart() {
    for definition in definitions("Namor the Sub-Mariner") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut dm = Choices::default();
        let spell = cast(&mut game, &resource(
            "Two blue symbols", "Mana cost: {1}{U}{U}\nType: Instant\nYou gain 1 life.",
        ), A, CastingMethod::Normal, &mut dm);
        apply(&mut game, source, Effect::counter(ChooseSpec::SpecificObject(spell)));
        game.set_current_controller(source, B).unwrap();
        game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
        settle(&mut game, &mut dm);
        assert_eq!(merfolk(&game, A).len(), 2);
        assert!(merfolk(&game, B).is_empty());
    }
}

#[test]
fn bibliophile_counts_distinct_targets_across_separate_repeated_target_slots() {
    for definition in definitions("Voracious Bibliophile") {
        for repeated in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            assert!(game.current_has_static_ability_id(source, StaticAbilityId::Flying));
            assert!(game.current_has_static_ability_id(source, StaticAbilityId::Vigilance));
            assert_eq!((game.current_power(source), game.current_toughness(source)), (Some(3), Some(3)));
            fill_library(&mut game, A);
            let first = creature(&mut game, A);
            let second = if repeated { first } else { creature(&mut game, A) };
            let mut dm = Choices {
                target_groups: VecDeque::from(vec![vec![Target::Object(first)], vec![Target::Object(second)]]),
                ..Default::default()
            };
            let spell = cast(&mut game, &resource(
                "Separate target slots",
                "Mana cost: {1}\nType: Instant\nTarget creature gets +1/+1 until end of turn. Target creature gains flying until end of turn.",
            ), A, CastingMethod::Normal, &mut dm);
            let entry = game.stack.iter().find(|entry| entry.object_id == spell).unwrap();
            assert_eq!(entry.targets, vec![Target::Object(first), Target::Object(second)]);
            assert_eq!(game.stack.len(), 2);
            apply(&mut game, source, Effect::counter(ChooseSpec::SpecificObject(spell)));
            game.move_object_by_game_rule(first, Zone::Exile).unwrap();
            if !repeated { game.move_object_by_game_rule(second, Zone::Exile).unwrap(); }
            game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), if repeated { 1 } else { 2 });
        }
    }
}

#[test]
fn bibliophile_counts_player_and_object_targets_and_ignores_later_retargeting() {
    for definition in definitions("Voracious Bibliophile") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        fill_library(&mut game, A);
        let target = creature(&mut game, A);
        let mut dm = Choices {
            target_groups: VecDeque::from(vec![vec![Target::Object(target)], vec![Target::Player(B)]]),
            ..Default::default()
        };
        let spell = cast(&mut game, &resource(
            "Mixed target kinds",
            "Mana cost: {1}\nType: Instant\nTarget creature gets +1/+1 until end of turn. Target player gains 1 life.",
        ), A, CastingMethod::Normal, &mut dm);
        let entry = game.stack.iter_mut().find(|entry| entry.object_id == spell).unwrap();
        assert_eq!(entry.targets, vec![Target::Object(target), Target::Player(B)]);
        // A later current-stack view is deliberately inconsistent with the
        // immutable completed announcement. The observer must retain two.
        entry.targets.clear();
        game.set_current_controller(source, B).unwrap();
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(game.player(B).unwrap().hand.len(), 0);
    }
}

#[test]
fn bibliophile_rejects_nontargeted_casts_other_casters_and_spell_copies() {
    for definition in definitions("Voracious Bibliophile") {
        for (caster, targeted) in [(A, false), (B, true)] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            fill_library(&mut game, A);
            let text = if targeted {
                "Mana cost: {0}\nType: Instant\nTarget player gains 1 life."
            } else { "Mana cost: {0}\nType: Instant\nYou gain 1 life." };
            let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
            cast(&mut game, &resource("Negative target predicate", text), caster, CastingMethod::Normal, &mut dm);
            assert_eq!(game.stack.len(), 1);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), 0);
        }
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        fill_library(&mut game, A);
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        let spell = cast(&mut game, &resource(
            "Copied spell", "Mana cost: {0}\nType: Instant\nTarget player gains 1 life.",
        ), A, CastingMethod::Normal, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        apply(&mut game, source, Effect::new(ironsmith::effects::CopySpellEffect::single(
            ChooseSpec::SpecificObject(spell),
        )));
        put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
        assert_eq!(game.stack.len(), 2, "the two spells have no new cast trigger");
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
    }
}

#[test]
fn typed_cast_quantities_require_their_exact_completed_evidence_and_ignore_generic_overrides() {
    let mut game = game();
    let source = creature(&mut game, A);
    let spell = game.create_object_from_definition(&resource(
        "Evidence spell", "Mana cost: {2}{U}{U}\nType: Instant\nYou gain 1 life.",
    ), A, Zone::Stack);
    let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(game.object(spell).unwrap(), &game);
    let complete = SpellCastEvent::new_with_snapshot(spell, A, Zone::Hand, snapshot.clone())
        .with_targets(vec![Target::Player(A), Target::Player(A), Target::Player(B)]);
    let values = [(CastEventQuantity::ManaValue, 4),
        (CastEventQuantity::ManaSymbols(Color::Blue), 2), (CastEventQuantity::DistinctTargets, 2)];
    for (field, expected) in values {
        let ctx = EffectContext::new_default(source, A)
            .with_triggering_event(TriggerEvent::new_with_provenance(complete.clone(), Default::default()))
            .with_event_value_amount(97);
        assert_eq!(ironsmith::effects::helpers::resolve_value_wide(&game, &quantity(field.clone()), &ctx).unwrap(), expected);
        for event in [
            TriggerEvent::new_with_provenance(SpellCastEvent::new(spell, A, Zone::Hand), Default::default()),
            TriggerEvent::new_with_provenance(ironsmith::events::LifeGainEvent::new(A, 97), Default::default()),
        ] {
            let ctx = EffectContext::new_default(source, A).with_triggering_event(event).with_event_value_amount(97);
            assert!(ironsmith::effects::helpers::resolve_value_wide(&game, &quantity(field.clone()), &ctx).is_err());
        }
    }
    let missing_targets = SpellCastEvent::new_with_snapshot(spell, A, Zone::Hand, snapshot.clone());
    let ctx = EffectContext::new_default(source, A)
        .with_triggering_event(TriggerEvent::new_with_provenance(missing_targets, Default::default()));
    assert!(ironsmith::effects::helpers::resolve_value_wide(&game, &quantity(CastEventQuantity::DistinctTargets), &ctx).is_err());
    let empty_targets = SpellCastEvent::new_with_snapshot(spell, A, Zone::Hand, snapshot.clone()).with_targets(vec![]);
    let ctx = EffectContext::new_default(source, A)
        .with_triggering_event(TriggerEvent::new_with_provenance(empty_targets, Default::default()));
    assert_eq!(ironsmith::effects::helpers::resolve_value_wide(&game, &quantity(CastEventQuantity::DistinctTargets), &ctx).unwrap(), 0);
    let mut wrong_id = snapshot.clone();
    wrong_id.object_id = source;
    let mut wrong_zone = snapshot;
    wrong_zone.zone = Zone::Graveyard;
    for wrong in [wrong_id, wrong_zone] {
        let ctx = EffectContext::new_default(source, A).with_triggering_event(
            TriggerEvent::new_with_provenance(
                SpellCastEvent::new_with_snapshot(spell, A, Zone::Hand, wrong).with_targets(vec![]),
                Default::default(),
            ),
        );
        assert!(ironsmith::effects::helpers::resolve_value_wide(&game, &quantity(CastEventQuantity::ManaValue), &ctx).is_err());
    }
}

#[test]
fn doom_zero_mana_value_is_a_real_zero_and_still_executes_its_counter_tail() {
    for definition in definitions("Imminent Doom") {
        let mut game = game();
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        let source = enter_doom(&mut game, &definition, &mut dm);
        apply(&mut game, source, Effect::remove_counters(
            CounterType::Doom, 1, ChooseSpec::SpecificObject(source),
        ));
        assert_eq!(game.counter_count(source, CounterType::Doom), 0);
        let spell = cast(&mut game, &resource(
            "Zero mana", "Mana cost: {0}\nType: Instant\nYou gain 1 life.",
        ), A, CastingMethod::Normal, &mut dm);
        assert_eq!(game.stack.len(), 2);
        apply(&mut game, source, Effect::counter(ChooseSpec::SpecificObject(spell)));
        settle(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, 20);
        assert_eq!(game.counter_count(source, CounterType::Doom), 1);
    }
}

#[test]
fn face_down_zero_no_cost_zero_and_unannounced_x_have_distinct_evidence() {
    let mut game = game();
    let source = creature(&mut game, A);
    let spell = game.create_object_from_definition(&resource(
        "Public cast characteristics", "Mana cost: {X}{U}{U}\nType: Instant\nYou gain 1 life.",
    ), A, Zone::Stack);
    let mut snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(game.object(spell).unwrap(), &game);
    snapshot.x_value = None;
    let missing_x = SpellCastEvent::new_with_snapshot(spell, A, Zone::Hand, snapshot.clone()).with_targets(vec![]);
    assert!(missing_x.cast_quantity(CastEventQuantity::ManaValue).is_err());
    assert_eq!(missing_x.cast_quantity(CastEventQuantity::ManaSymbols(Color::Blue)).unwrap(), 2);
    snapshot.face_down = true;
    let face_down = SpellCastEvent::new_with_snapshot(spell, A, Zone::Hand, snapshot.clone()).with_targets(vec![]);
    assert_eq!(face_down.cast_quantity(CastEventQuantity::ManaValue).unwrap(), 0);
    assert_eq!(face_down.cast_quantity(CastEventQuantity::ManaSymbols(Color::Blue)).unwrap(), 0);
    snapshot.face_down = false;
    snapshot.mana_cost = None;
    let no_cost = SpellCastEvent::new_with_snapshot(spell, A, Zone::Hand, snapshot.clone()).with_targets(vec![]);
    assert_eq!(no_cost.cast_quantity(CastEventQuantity::ManaValue).unwrap(), 0);
    assert_eq!(no_cost.cast_quantity(CastEventQuantity::ManaSymbols(Color::Blue)).unwrap(), 0);
    // Alternative representations inside one pip cannot turn one printed
    // symbol into multiple counted symbols.
    snapshot.mana_cost = Some(ManaCost::from_pips(vec![
        vec![ManaSymbol::Blue, ManaSymbol::Red, ManaSymbol::Blue],
        vec![ManaSymbol::Generic(2), ManaSymbol::Blue],
    ]));
    let hybrid = SpellCastEvent::new_with_snapshot(spell, A, Zone::Hand, snapshot).with_targets(vec![]);
    let ctx = EffectContext::new_default(source, A).with_triggering_event(
        TriggerEvent::new_with_provenance(hybrid, Default::default()),
    );
    assert_eq!(ironsmith::effects::helpers::resolve_value_wide(
        &game, &quantity(CastEventQuantity::ManaSymbols(Color::Blue)), &ctx,
    ).unwrap(), 2);
}

#[test]
fn missing_completed_cast_evidence_is_a_checked_admission_failure() {
    for name in ["Imminent Doom", "Namor the Sub-Mariner", "Voracious Bibliophile"] {
        for definition in definitions(name) {
            for missing_snapshot in [false, true] {
                let mut game = game();
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                if name == "Imminent Doom" {
                    game.object_mut(source).unwrap().counters.insert(CounterType::Doom, 1);
                }
                let spell = game.create_object_from_definition(&resource(
                    "Live object is not evidence", "Mana cost: {U}\nType: Instant\nTarget player gains 1 life.",
                ), A, Zone::Stack);
                game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, A).with_targets(vec![Target::Player(B)]));
                let event = if missing_snapshot {
                    SpellCastEvent::new(spell, A, Zone::Hand)
                } else {
                    let mut snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(game.object(spell).unwrap(), &game);
                    if name == "Imminent Doom" {
                        snapshot.mana_cost = Some(ManaCost::from_pips(vec![vec![ManaSymbol::X]]));
                        snapshot.x_value = None;
                    } else if name == "Namor the Sub-Mariner" {
                        snapshot.object_id = source;
                    }
                    // Bibliophile has the correct snapshot but no target receipt.
                    SpellCastEvent::new_with_snapshot(spell, A, Zone::Hand, snapshot)
                };
                game.queue_trigger_event(Default::default(), TriggerEvent::new_with_provenance(event, Default::default()));
                let stack_before = game.stack.len();
                let pending_before = game.effect_store.pending_trigger_events.len();
                let mut queue = TriggerQueue::new();
                let error = put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut Choices::default()).unwrap_err();
                assert!(matches!(error, ironsmith::game_loop::GameLoopError::ExecutionFailed(
                    ironsmith::effects::ExecutionError::IncompleteEvidence(_)
                )), "{name}: {error:?}");
                assert_eq!(game.stack.len(), stack_before);
                assert_eq!(game.effect_store.pending_trigger_events.len(), pending_before, "failed admission must retain the event for repair");
            }
        }
    }
}

#[test]
fn cast_mana_value_uses_wide_scalar_math_and_checked_narrowing() {
    let mut game = game();
    let source = creature(&mut game, A);
    let spell = game.create_object_from_definition(&resource(
        "Wide completed cast", "Mana cost: {X}{X}{1}\nType: Instant\nYou gain 1 life.",
    ), A, Zone::Stack);
    let mut snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(game.object(spell).unwrap(), &game);
    snapshot.x_value = Some(u32::MAX);
    let event = SpellCastEvent::new_with_snapshot(spell, A, Zone::Hand, snapshot).with_targets(vec![]);
    let ctx = EffectContext::new_default(source, A).with_triggering_event(
        TriggerEvent::new_with_provenance(event, Default::default()),
    );
    assert_eq!(ironsmith::effects::helpers::resolve_value_wide(
        &game, &quantity(CastEventQuantity::ManaValue), &ctx,
    ).unwrap(), 2 * i64::from(u32::MAX) + 1);
    assert!(ironsmith::effects::helpers::resolve_value(
        &game, &quantity(CastEventQuantity::ManaValue), &ctx,
    ).is_err());
}

#[test]
fn replacement_additions_read_their_local_amount_without_rebinding_original_cast_siblings() {
    for definition in definitions("Voracious Bibliophile") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        fill_library(&mut game, A);
        let target = creature(&mut game, A);
        let mut dm = Choices {
            target_groups: VecDeque::from(vec![vec![Target::Object(target)], vec![Target::Player(B)]]),
            ..Default::default()
        };
        cast(&mut game, &resource(
            "Replacement boundary cast",
            "Mana cost: {1}\nType: Instant\nTarget creature gets +1/+1 until end of turn. Target player gains 1 life.",
        ), A, CastingMethod::Normal, &mut dm);
        let event = captured_event(&game);
        use ironsmith::replacement::{EventModification, ReplacementAction, ReplacementEffect};
        use ironsmith::events::damage::matchers::DamageFromSourceMatcher;
        game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(source, A,
                DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                ReplacementAction::Modify(EventModification::SetTo(7)),
            ).with_priority_override(ironsmith::events::ReplacementPriority::SelfReplacement),
        );
        game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(source, A,
                DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                ReplacementAction::Additionally(vec![Effect::gain_life(Value::EventValue(EventValueSpec::Amount))]),
            ),
        );
        let mut choices = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, A, &mut choices).with_triggering_event(event);
        execute_effect(&mut game, &Effect::deal_damage(
            quantity(CastEventQuantity::DistinctTargets), ChooseSpec::Player(PlayerFilter::Specific(B)),
        ), &mut ctx).unwrap();
        assert_eq!(game.player(B).unwrap().life, 13);
        assert_eq!(game.player(A).unwrap().life, 27, "replacement addition uses the replaced damage amount");
        execute_effect(&mut game, &Effect::gain_life(quantity(CastEventQuantity::DistinctTargets)), &mut ctx).unwrap();
        assert_eq!(game.player(A).unwrap().life, 29, "the next original effect still uses the two cast targets");
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 2, "the native Bibliophile trigger retained the same cast evidence");
    }
}

#[test]
fn compiled_original_siblings_keep_the_cast_quantity_after_a_replaced_draw_result() {
    let text = "Type: Enchantment\nWhenever you cast a spell with one or more targets, draw that many cards. You gain that much life.";
    for definition in definitions_text("Original cast quantity siblings", text) {
        let debug = format!("{definition:?}");
        assert_eq!(debug.matches("CastSpell(DistinctTargets)").count(), 2, "{debug}");
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        fill_library(&mut game, A);
        let target = creature(&mut game, A);
        let mut dm = Choices {
            target_groups: VecDeque::from(vec![vec![Target::Object(target)], vec![Target::Player(B)]]),
            ..Default::default()
        };
        cast(&mut game, &resource(
            "Two-target original sibling cast",
            "Mana cost: {1}\nType: Instant\nTarget creature gets +1/+1 until end of turn. Target player gains 1 life.",
        ), A, CastingMethod::Normal, &mut dm);
        game.effect_store.replacement_effects.add_one_shot_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source, A, ironsmith::events::cards::matchers::WouldDrawCardMatcher::you(),
                ironsmith::replacement::ReplacementAction::Instead(vec![
                    Effect::gain_life(Value::EventValue(EventValueSpec::Amount)),
                ]),
            ),
        );
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 1, "one of the two individual draws was replaced");
        assert_eq!(game.player(A).unwrap().life, 23,
            "replacement-local draw amount one plus the original sibling's captured two targets");
    }
}

#[test]
fn an_effect_driven_free_cast_publishes_the_same_completed_receipt_to_all_three_cards() {
    let dooms = definitions("Imminent Doom");
    let namors = definitions("Namor the Sub-Mariner");
    let bibliophiles = definitions("Voracious Bibliophile");
    for ((doom, namor), bibliophile) in dooms.into_iter().zip(namors).zip(bibliophiles) {
        let mut game = game();
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        let source = enter_doom(&mut game, &doom, &mut dm);
        apply(&mut game, source, Effect::put_counters(
            CounterType::Doom, 4, ChooseSpec::SpecificObject(source),
        ));
        game.create_object_from_definition(&namor, A, Zone::Battlefield);
        game.create_object_from_definition(&bibliophile, A, Zone::Battlefield);
        fill_library(&mut game, A);
        let target = creature(&mut game, A);
        dm.target_groups = VecDeque::from(vec![vec![Target::Object(target)], vec![Target::Player(B)]]);
        let exiled = game.create_object_from_definition(&resource(
            "Effect-driven cast receipt",
            "Mana cost: {3}{U}{U}\nType: Instant\nTarget creature gets +1/+1 until end of turn. Target player gains 1 life.",
        ), A, Zone::Exile);
        let snapshot = ObjectSnapshot::from_object(game.object(exiled).unwrap(), &game);
        let outcome = {
            let mut ctx = EffectContext::new(source, A, &mut dm);
            ctx.tag_object("free-cast", snapshot);
            execute_effect(&mut game, &Effect::cast_tagged(
                "free-cast", PlayerFilter::You, false, false, true, None,
            ), &mut ctx).unwrap()
        };
        let spell = game.stack.iter().find(|entry| !entry.is_ability).unwrap().object_id;
        assert_eq!(game.object(spell).unwrap().mana_spent_to_cast.total(), 0);
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        for event in outcome.events {
            // Captured publication may be reported again, but never admitted twice.
            game.queue_trigger_event(Default::default(), event);
        }
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 4, "one completed free cast and all three observer triggers");
        apply(&mut game, source, Effect::counter(ChooseSpec::SpecificObject(spell)));
        settle(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, 15);
        assert_eq!(game.counter_count(source, CounterType::Doom), 6);
        assert_eq!(merfolk(&game, A).len(), 2);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
    }
}


#[test]
fn missing_cast_evidence_rolls_back_before_negative_result_followups() {
    use ironsmith::effect::{EffectId, EffectPredicate};
    use ironsmith::effects::{ExecutionError, SequenceEffect};
    let mut game = game();
    let source = creature(&mut game, A);
    fill_library(&mut game, A);
    let mut dm = Choices::default();
    let mut ctx = EffectContext::new(source, A, &mut dm);
    let program = Effect::new(SequenceEffect::new(vec![
        Effect::gain_life(1),
        Effect::with_id(7, Effect::draw(quantity(CastEventQuantity::DistinctTargets))),
        Effect::if_then_else(EffectId(7), EffectPredicate::Happened,
            vec![Effect::gain_life(3)], vec![Effect::lose_life(9)]),
    ]));
    let error = execute_effect(&mut game, &program, &mut ctx).unwrap_err();
    assert!(matches!(error, ExecutionError::IncompleteEvidence(_)), "{error:?}");
    assert_eq!(game.player(A).unwrap().life, 20, "neither prefix nor negative followup may commit");
    assert_eq!(game.player(A).unwrap().hand.len(), 0);
    assert_eq!(game.player(A).unwrap().library.len(), 12);
}

#[test]
fn reported_cast_admission_retains_namors_noncreature_frame_after_departure_or_type_change() {
    for definition in definitions("Namor the Sub-Mariner") {
        for leaves in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let exiled = game.create_object_from_definition(&resource(
                "Deferred noncreature cast", "Mana cost: {3}{U}{U}\nType: Instant\nYou gain 1 life.",
            ), A, Zone::Exile);
            let snapshot = ObjectSnapshot::from_object(game.object(exiled).unwrap(), &game);
            let mut dm = Choices::default();
            let outcome = {
                let mut ctx = EffectContext::new(source, A, &mut dm);
                ctx.tag_object("deferred-cast", snapshot);
                execute_effect(&mut game, &Effect::cast_tagged(
                    "deferred-cast", PlayerFilter::You, false, false, true, None,
                ), &mut ctx).unwrap()
            };
            let event = outcome.events.into_iter().find(|event| event.downcast::<SpellCastEvent>().is_some()).unwrap();
            let spell = event.downcast::<SpellCastEvent>().unwrap().spell;
            assert!(event.downcast::<SpellCastEvent>().unwrap().required_completed_snapshot().unwrap()
                .card_types.contains(&ironsmith::CardType::Instant));
            if leaves {
                apply(&mut game, source, Effect::counter(ChooseSpec::SpecificObject(spell)));
            } else {
                game.object_mut(spell).unwrap().card_types = vec![ironsmith::CardType::Creature].into();
            }
            // The ordinary effect-driven reported-event path owns this delayed
            // matching point. No fabricated cast or current-stack reconstruction.
            game.queue_trigger_event(Default::default(), event);
            let mut queue = TriggerQueue::new();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(game.stack.iter().filter(|entry| entry.is_ability).count(), 1);
            resolve(&mut game, &mut dm);
            assert_eq!(merfolk(&game, A).len(), 2);
        }
    }
}

#[test]
fn held_cast_retains_dooms_counter_comparison_at_the_completed_cast() {
    for definition in definitions("Imminent Doom") {
        let mut game = game();
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        let source = enter_doom(&mut game, &definition, &mut dm);
        let exiled = game.create_object_from_definition(&resource(
            "Held doom admission", "Mana cost: {1}\nType: Instant\nYou gain 1 life.",
        ), A, Zone::Exile);
        let snapshot = ObjectSnapshot::from_object(game.object(exiled).unwrap(), &game);
        let outcome = {
            let mut ctx = EffectContext::new(source, A, &mut dm);
            ctx.tag_object("held-doom-cast", snapshot);
            execute_effect(&mut game, &Effect::cast_tagged(
                "held-doom-cast", PlayerFilter::You, false, false, true, None,
            ), &mut ctx).unwrap()
        };
        let event = outcome.events.into_iter().find(|event| event.downcast::<SpellCastEvent>().is_some()).unwrap();
        let spell = event.downcast::<SpellCastEvent>().unwrap().spell;
        assert_eq!(game.counter_count(source, CounterType::Doom), 1);
        apply(&mut game, source, Effect::put_counters(
            CounterType::Doom, 1, ChooseSpec::SpecificObject(source),
        ));
        assert_eq!(game.counter_count(source, CounterType::Doom), 2);
        game.queue_trigger_event(Default::default(), event.clone());
        game.queue_trigger_event(Default::default(), event);
        let mut queue = TriggerQueue::new();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.iter().filter(|entry| entry.is_ability).count(), 1,
            "the source had one doom counter when the mana-value-one cast completed; replayed receipts cannot double it");
        assert_eq!(game.turn_store.turn_history.total_spells_cast_this_turn(), 2);
        apply(&mut game, source, Effect::counter(ChooseSpec::SpecificObject(spell)));
        settle(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, 19);
        assert_eq!(game.counter_count(source, CounterType::Doom), 3);
    }
}


#[test]
fn completed_cast_capture_keeps_intervening_if_admission_separate_from_resolution() {
    let text = "Type: Enchantment\nWhenever you cast a spell with one or more targets, if you control an Island, draw that many cards.";
    for definition in definitions_text("Cast intervening-if probe", text) {
        for island_at_cast in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            fill_library(&mut game, A);
            let island_definition = resource("Condition Island", "Type: Basic Land — Island");
            let island = island_at_cast.then(|| game.create_object_from_definition(&island_definition, A, Zone::Battlefield));
            let exiled = game.create_object_from_definition(&resource(
                "Conditional captured cast", "Mana cost: {1}\nType: Instant\nTarget player gains 1 life.",
            ), A, Zone::Exile);
            let snapshot = ObjectSnapshot::from_object(game.object(exiled).unwrap(), &game);
            let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
            let outcome = {
                let mut ctx = EffectContext::new(source, A, &mut dm);
                ctx.tag_object("condition-cast", snapshot);
                execute_effect(&mut game, &Effect::cast_tagged(
                    "condition-cast", PlayerFilter::You, false, false, true, None,
                ), &mut ctx).unwrap()
            };
            if let Some(island) = island {
                game.move_object_by_game_rule(island, Zone::Exile).unwrap();
            } else {
                game.create_object_from_definition(&island_definition, A, Zone::Battlefield);
            }
            for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
            let mut queue = TriggerQueue::new();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(game.stack.iter().filter(|entry| entry.is_ability).count(), usize::from(island_at_cast));
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), 0,
                "a condition false at admission cannot start a trigger; one false at resolution prevents its body");
        }
    }
}

#[test]
fn namor_power_counts_controlled_noncreature_kindred_merfolk_on_the_battlefield() {
    for definition in definitions("Namor the Sub-Mariner") {
        let power = definition.abilities.iter().find_map(|ability| {
            let ironsmith::ability::AbilityKind::Static(ability) = &ability.kind else { return None; };
            match &ability.compiled_model()?.payload {
                ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt { power, .. } => Some(power),
                _ => None,
            }
        }).expect("the complete Namor body retains its characteristic-defining power");
        let Value::Count(filter) = power.unhinted() else { panic!("unexpected power quantity: {power:?}"); };
        assert_eq!(filter.subtypes, vec![Subtype::Merfolk]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(filter.card_types.is_empty() && filter.all_card_types.is_empty(),
            "a subtype count must not acquire an implicit Creature restriction: {filter:?}");
        assert!(matches!(filter.zone, None | Some(Zone::Battlefield)));

        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(game.current_power(source), Some(1));
        let kindred = resource("Kindred Merfolk count probe", "Type: Kindred Enchantment — Merfolk");
        let controlled = game.create_object_from_definition(&kindred, A, Zone::Battlefield);
        assert!(!game.current_card_types(controlled).unwrap().contains(&ironsmith::CardType::Creature));
        assert_eq!(game.current_power(source), Some(2));
        game.create_object_from_definition(&kindred, B, Zone::Battlefield);
        game.create_object_from_definition(&kindred, A, Zone::Hand);
        game.create_object_from_definition(&kindred, A, Zone::Graveyard);
        assert_eq!(game.current_power(source), Some(2), "only controlled battlefield Merfolk count");
        game.set_current_controller(controlled, B).unwrap();
        assert_eq!(game.current_power(source), Some(1));
        game.set_current_controller(controlled, A).unwrap();
        assert_eq!(game.current_power(source), Some(2));
        game.move_object_by_game_rule(controlled, Zone::Graveyard).unwrap();
        assert_eq!(game.current_power(source), Some(1));
    }
}

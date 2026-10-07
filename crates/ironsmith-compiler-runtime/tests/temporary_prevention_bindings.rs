//! Frozen full bodies and shared bridge contracts. Source-authored, all UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{SelectOptionsContext, TargetsContext};
use ironsmith::effect::{Effect, Until};
use ironsmith::effects::{DealDamageEffect, EffectContext, EffectExecutor, ExecutionError, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::prevention::DamageFilter;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, Color, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::{encode_runtime_definition, encode_runtime_effect,
    materialize_artifact, materialize_definition, materialize_effect};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/temporary_prevention_bindings.json.fixture")).unwrap()
}
fn input(row: &serde_json::Value, oracle: &str) -> String {
    let mut lines = vec![format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap())];
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {p}/{t}"));
    }
    lines.push(oracle.into());
    lines.join("\n")
}
fn definitions(name: &str) -> [CardDefinition; 3] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let text = input(&row, row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let wire = encode_runtime_definition(direct.clone()).unwrap();
    let native = materialize_definition(serde_json::from_slice(&serde_json::to_vec(&wire).unwrap()).unwrap()).unwrap();
    [direct, materialize_artifact(&restored).unwrap(), native]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 40);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Green, ManaSymbol::Colorless] {
        game.player_mut(A).unwrap().mana_pool.add(color, 20);
    }
    game
}
fn object(game: &mut GameState, owner: PlayerId, types: Vec<CardType>, color: ColorSet) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Prevention witness").card_types(types)
        .color_indicator(color).power_toughness(PowerToughness::fixed(2, 30)).loyalty(30).build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}
#[derive(Default)]
struct Choices { target: Option<Target>, forbidden_target: Option<Target>, color: Option<Color>, color_calls: usize, pause: bool, pending: bool }
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert_eq!(context.requirements.len(), 1);
            assert!(context.requirements[0].legal_targets.contains(&target));
            if let Some(forbidden) = self.forbidden_target {
                assert!(!context.requirements[0].legal_targets.contains(&forbidden));
            }
            vec![target]
        } else { SelectFirstDecisionMaker.decide_targets(game, context) }
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description == "Choose a color" {
            self.color_calls += 1;
            assert_eq!(context.player, A);
            if self.pause { self.pending = true; return vec![]; }
            let color = self.color.unwrap_or(Color::Red);
            return vec![Color::ALL.iter().position(|candidate| *candidate == color).unwrap()];
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn announce(game: &mut GameState, action: LegalAction, choices: &mut Choices) {
    game.turn.priority_player = Some(A);
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), choices).unwrap();
    for _ in 0..40 {
        if !state.has_pending_action() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices).unwrap();
    }
    assert!(!state.has_pending_action());
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
}
fn cast(game: &mut GameState, definition: &CardDefinition, choices: &mut Choices) {
    let source = game.create_object_from_definition(definition, A, Zone::Hand);
    announce(game, LegalAction::CastSpell { spell_id: source, from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal }, choices);
}
fn activate(game: &mut GameState, source: ObjectId, ordinal: usize, choices: &mut Choices) {
    let ability_index = game.current_abilities(source).unwrap().iter().enumerate()
        .filter_map(|(index, ability)| matches!(ability.kind, AbilityKind::Activated(_)).then_some(index))
        .nth(ordinal).unwrap();
    announce(game, LegalAction::ActivateAbility { source, ability_index }, choices);
}
fn settle(game: &mut GameState, choices: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("unexpected continuing prevention trigger chain");
}
fn damage(game: &mut GameState, source: ObjectId, target: Target, combat: bool, unpreventable: bool) -> u32 {
    let target = match target { Target::Object(id) => ChooseSpec::SpecificObject(id),
        Target::Player(id) => ChooseSpec::SpecificPlayer(id) };
    let controller = game.current_controller(source).unwrap();
    let outcome = DealDamageEffect::new(3, target).with_combat(combat).with_unpreventable(unpreventable)
        .execute(game, &mut EffectContext::new_default(source, controller)).unwrap();
    outcome.events.iter().filter_map(|event| event.downcast::<ironsmith::events::DamageEvent>()
        .map(|event| event.amount)).sum()
}
fn end_turn(game: &mut GameState) {
    let turn = game.turn.turn_number;
    for _ in 0..40 {
        ironsmith::turn::advance_step(game).unwrap();
        if game.turn.turn_number != turn { return; }
    }
    panic!("turn boundary did not occur");
}

#[test]
fn all_eleven_frozen_whole_bodies_retain_metadata_artifacts_and_rendered_semantics() {
    let rows = rows();
    assert_eq!(rows.len(), 15);
    let complete: Vec<_> = rows.iter().filter(|row| row["proposed_complete"] == true).collect();
    assert_eq!(complete.len(), 11);
    for row in complete {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert_eq!(definition.card.name, name);
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            assert!(!rendered.contains("Unsupported"), "{name}: {rendered}");
            let (reparsed, loss) = ironsmith_compiler::parse_loss::capture(||
                compile_to_runtime_definition(name, input(row, &rendered), false));
            let reparsed = reparsed.unwrap_or_else(|error| panic!("{name}: {rendered}: {error}"));
            assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
            assert_eq!(ironsmith_text::compiled_text_lines(&reparsed).join("\n"), rendered);
        }
    }
}

#[test]
fn chameleon_blur_covers_players_and_creature_sources_without_becoming_combat_only() {
    for definition in definitions("Chameleon Blur") {
        let mut game = game();
        let source = object(&mut game, B, vec![CardType::Creature], ColorSet::RED);
        let artifact = object(&mut game, B, vec![CardType::Artifact], ColorSet::COLORLESS);
        let recipient = object(&mut game, A, vec![CardType::Creature], ColorSet::COLORLESS);
        cast(&mut game, &definition, &mut Choices::default());
        settle(&mut game, &mut Choices::default());
        for player in [A, B] { for combat in [false, true] {
            assert_eq!(damage(&mut game, source, Target::Player(player), combat, false), 0);
        }}
        assert_eq!(damage(&mut game, artifact, Target::Player(A), false, false), 3);
        assert_eq!(damage(&mut game, source, Target::Object(recipient), true, false), 3);
        assert_eq!(damage(&mut game, source, Target::Player(A), true, true), 3);
        end_turn(&mut game);
        assert_eq!(damage(&mut game, source, Target::Player(A), false, false), 3);
    }
}

#[test]
fn ethersworn_flash_trigger_keeps_artifact_and_creature_conjunction_live() {
    for definition in definitions("Ethersworn Shieldmage") {
        let mut game = game();
        game.turn.active_player = B;
        let source = object(&mut game, B, vec![CardType::Creature], ColorSet::RED);
        cast(&mut game, &definition, &mut Choices::default());
        settle(&mut game, &mut Choices::default());
        for owner in [A, B] {
            let both = object(&mut game, owner, vec![CardType::Artifact, CardType::Creature], ColorSet::COLORLESS);
            let only_creature = object(&mut game, owner, vec![CardType::Creature], ColorSet::COLORLESS);
            assert_eq!(damage(&mut game, source, Target::Object(both), false, false), 0);
            // This witness proves the filter isn't an artifact/creature union.
            assert_eq!(damage(&mut game, source, Target::Object(only_creature), false, false), 3);
        }
        assert_eq!(damage(&mut game, source, Target::Player(A), false, false), 3);
    }
}

#[test]
fn focus_keeps_the_pumped_identity_and_rechecks_source_color_at_damage_time() {
    for definition in definitions("Lithomancer's Focus") {
        let mut game = game();
        let target = object(&mut game, A, vec![CardType::Creature], ColorSet::GREEN);
        let other = object(&mut game, A, vec![CardType::Creature], ColorSet::GREEN);
        let source = object(&mut game, B, vec![CardType::Creature], ColorSet::COLORLESS);
        let red = object(&mut game, B, vec![CardType::Creature], ColorSet::RED);
        let mut choices = Choices { target: Some(Target::Object(target)), ..Default::default() };
        cast(&mut game, &definition, &mut choices); settle(&mut game, &mut choices);
        assert_eq!(game.current_power(target), Some(4));
        assert_eq!(damage(&mut game, source, Target::Object(target), false, false), 0);
        assert_eq!(damage(&mut game, source, Target::Object(other), false, false), 3);
        assert_eq!(damage(&mut game, red, Target::Object(target), false, false), 3);
        ironsmith::effects::ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(source),
            ironsmith::continuous::Modification::SetColors(ColorSet::BLUE), Until::EndOfTurn,
        ).execute(&mut game, &mut EffectContext::new_default(source, B)).unwrap();
        assert_eq!(damage(&mut game, source, Target::Object(target), false, false), 3);
        ironsmith::effects::ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(source),
            ironsmith::continuous::Modification::SetColors(ColorSet::COLORLESS), Until::EndOfTurn,
        ).execute(&mut game, &mut EffectContext::new_default(source, B)).unwrap();
        game.set_current_controller(target, B).unwrap();
        assert_eq!(damage(&mut game, source, Target::Object(target), true, false), 0);
        assert_eq!(damage(&mut game, source, Target::Object(target), true, true), 3);
    }
}

#[test]
fn avacyn_chooses_once_on_resolution_and_preserves_each_recipient_kind() {
    for definition in definitions("Avacyn, Guardian Angel") { for recipient_kind in 0..3 {
        let mut game = game();
        let avacyn = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let red = object(&mut game, B, vec![CardType::Creature], ColorSet::RED);
        let blue = object(&mut game, B, vec![CardType::Creature], ColorSet::BLUE);
        let target = match recipient_kind {
            0 => Target::Object(object(&mut game, B, vec![CardType::Creature], ColorSet::GREEN)),
            1 => Target::Player(B),
            _ => Target::Object(object(&mut game, B, vec![CardType::Planeswalker], ColorSet::GREEN)),
        };
        game.set_chosen_color(avacyn, Color::Green);
        let mut choices = Choices { target: Some(target), color: Some(Color::Red),
            forbidden_target: (recipient_kind == 0).then_some(Target::Object(avacyn)), ..Default::default() };
        activate(&mut game, avacyn, if recipient_kind == 0 { 0 } else { 1 }, &mut choices);
        assert_eq!(choices.color_calls, 0, "color is not an announcement target or mode");
        settle(&mut game, &mut choices);
        assert_eq!(choices.color_calls, 1);
        assert_eq!(game.chosen_color(avacyn), Some(Color::Green));
        game.move_object_by_effect(avacyn, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, red, target, false, false), 0);
        assert_eq!(damage(&mut game, blue, target, false, false), 3);
        assert_eq!(damage(&mut game, red, target, true, true), 3);
        end_turn(&mut game);
        assert_eq!(damage(&mut game, red, target, true, false), 3);
    }}
}

#[test]
fn decorated_griffin_spends_only_actual_preventable_combat_damage_from_its_finite_budget() {
    for definition in definitions("Decorated Griffin") {
        let mut game = game();
        let griffin = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = object(&mut game, B, vec![CardType::Creature], ColorSet::RED);
        activate(&mut game, griffin, 0, &mut Choices::default());
        settle(&mut game, &mut Choices::default());
        assert_eq!(damage(&mut game, source, Target::Player(A), false, false), 3);
        assert_eq!(damage(&mut game, source, Target::Player(A), true, true), 3);
        assert_eq!(damage(&mut game, source, Target::Player(B), true, false), 3);
        assert_eq!(damage(&mut game, source, Target::Player(A), true, false), 2);
        let mut recovered = game.clone();
        assert_eq!(damage(&mut recovered, source, Target::Player(A), true, false), 3);
    }
}

#[test]
fn native_prevention_payloads_recover_filters_choices_and_nested_programs() {
    let mut filter = DamageFilter::combat();
    filter.from_source = Some(ObjectFilter::creature());
    let finite = Effect::new(ironsmith::effects::PreventDamageEffect::to_you(4, Until::EndOfTurn)
        .with_filter(filter.clone()).with_follow_up_effects(vec![Effect::gain_life(1)]));
    let mut chosen = ironsmith::effects::PreventAllDamageToTargetEffect::new(
        ChooseSpec::SpecificPlayer(A), Until::EndOfTurn,
    ).with_filter(filter.clone());
    chosen.source_color_of_your_choice = true;
    for original in [finite, Effect::new(chosen)] {
        assert!(original.serialized_model().is_none(), "exercise native encoder");
        let wire = encode_runtime_effect(original.clone()).unwrap();
        let recovered = materialize_effect(serde_json::from_slice(&serde_json::to_vec(&wire).unwrap()).unwrap()).unwrap();
        assert_eq!(ironsmith_text::compile_effect_list(&[original]), ironsmith_text::compile_effect_list(&[recovered.clone()]));
        if let Some(finite) = recovered.downcast_ref::<ironsmith::effects::PreventDamageEffect>() {
            assert_eq!(finite.damage_filter, filter); assert_eq!(finite.follow_up_effects.len(), 1);
        } else {
            let chosen = recovered.downcast_ref::<ironsmith::effects::PreventAllDamageToTargetEffect>().unwrap();
            assert!(chosen.source_color_of_your_choice); assert_eq!(chosen.damage_filter, filter);
        }
    }
}

#[test]
fn native_chosen_color_pending_and_resource_failure_publish_no_partial_shield() {
    let mut native = ironsmith::effects::PreventAllDamageToTargetEffect::new(ChooseSpec::SpecificPlayer(A), Until::EndOfTurn);
    native.source_color_of_your_choice = true;
    for effect in [Effect::new(native.clone()), materialize_effect(encode_runtime_effect(Effect::new(native)).unwrap()).unwrap()] {
        let mut game = game();
        let source = object(&mut game, A, vec![CardType::Creature], ColorSet::WHITE);
        let mut choices = Choices { pause: true, ..Default::default() };
        execute_effect(&mut game, &effect, &mut EffectContext::new(source, A, &mut choices)).unwrap();
        assert!(choices.pending); assert!(game.effect_store.prevention_effects.shields().is_empty());
        choices.pending = false; choices.pause = false;
        execute_effect(&mut game, &effect, &mut EffectContext::new(source, A, &mut choices)).unwrap();
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
        let before = game.effect_store.prevention_effects.next_id();
        game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits {
            max_instructions: 0, ..Default::default()
        });
        let error = execute_effect(&mut game, &effect, &mut EffectContext::new(source, A, &mut choices)).unwrap_err();
        assert!(matches!(error, ExecutionError::ResourceLimitExceeded { .. }));
        assert_eq!(game.effect_store.prevention_effects.next_id(), before);
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
    }
}

#[test]
fn native_chosen_color_invalid_decision_or_missing_target_is_a_typed_error() {
    struct InvalidColor;
    impl DecisionMaker for InvalidColor {
        fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> {
            vec![usize::MAX]
        }
    }
    for spec in [ChooseSpec::SpecificPlayer(A),
        ChooseSpec::Target(Box::new(ChooseSpec::Object(ObjectFilter::creature())))] {
        let mut game = game();
        let source = object(&mut game, A, vec![CardType::Creature], ColorSet::WHITE);
        let mut effect = ironsmith::effects::PreventAllDamageToTargetEffect::new(spec, Until::EndOfTurn);
        effect.source_color_of_your_choice = true;
        let error = effect.execute(&mut game, &mut EffectContext::new(source, A, &mut InvalidColor)).unwrap_err();
        assert!(matches!(error, ExecutionError::InvalidTarget | ExecutionError::UnresolvableValue(_)));
        assert!(game.effect_store.prevention_effects.shields().is_empty());
    }
}

#[test]
fn native_filtered_prevention_respects_the_innermost_iteration_recipient() {
    let original = Effect::new(ironsmith::effects::PreventAllDamageToTargetEffect::new(
        ChooseSpec::Iterated, Until::EndOfTurn,
    ).with_filter(DamageFilter::from_color(Color::Red)));
    for effect in [original.clone(), materialize_effect(encode_runtime_effect(original).unwrap()).unwrap()] {
        for binding in ["object", "nested object", "player"] {
            let mut game = game();
            let source = object(&mut game, A, vec![CardType::Creature], ColorSet::WHITE);
            let recipient = object(&mut game, B, vec![CardType::Creature], ColorSet::GREEN);
            let mut context = EffectContext::new_default(source, A);
            context.iteration.iterated_object = (binding != "player").then_some(recipient);
            context.iteration.iterated_player = (binding != "object").then_some(B);
            execute_effect(&mut game, &effect, &mut context).unwrap();
            let shields = game.effect_store.prevention_effects.shields();
            assert_eq!(shields.len(), 1);
            assert_eq!(shields[0].protected, if binding == "player" {
                ironsmith::prevention::PreventionTarget::Player(B)
            } else { ironsmith::prevention::PreventionTarget::Permanent(recipient) });
        }
    }
}

#[test]
fn native_attacked_recipient_protects_the_player_or_object_without_adding_its_controller() {
    use ironsmith::triggers::{AttackEventTarget, TriggerEvent};
    let native = Effect::new(ironsmith::effects::PreventAllDamageToTargetEffect::new(
        ChooseSpec::AttackedPlayerOrPlaneswalker, Until::EndOfTurn,
    ).with_filter(DamageFilter::combat()));
    for effect in [native.clone(), materialize_effect(encode_runtime_effect(native).unwrap()).unwrap()] {
        for kind in ["player", "planeswalker", "battle", "nothing"] {
            let mut game = game();
            let source = object(&mut game, A, vec![CardType::Creature], ColorSet::WHITE);
            let recipient = object(&mut game, B, vec![if kind == "battle" {
                CardType::Battle
            } else { CardType::Planeswalker }], ColorSet::GREEN);
            let attacked = match kind {
                "player" => AttackEventTarget::Player(B),
                "planeswalker" => AttackEventTarget::Planeswalker(recipient),
                "battle" => AttackEventTarget::Battle(recipient),
                _ => AttackEventTarget::Nothing,
            };
            let event = TriggerEvent::new_with_provenance(
                ironsmith::events::combat::CreatureAttackedEvent::new(source, attacked), Default::default(),
            );
            let result = execute_effect(&mut game, &effect,
                &mut EffectContext::new_default(source, A).with_triggering_event(event));
            if kind == "nothing" {
                assert!(matches!(result, Err(ExecutionError::InvalidTarget)));
                assert!(game.effect_store.prevention_effects.shields().is_empty());
                continue;
            }
            result.unwrap();
            let shields = game.effect_store.prevention_effects.shields();
            assert_eq!(shields.len(), 1);
            assert_eq!(shields[0].protected, if kind == "player" {
                ironsmith::prevention::PreventionTarget::Player(B)
            } else { ironsmith::prevention::PreventionTarget::Permanent(recipient) });
        }
    }
}

#[test]
fn finite_combat_shield_restores_budget_and_replacement_when_a_deferred_quantity_fails() {
    let native = Effect::new(ironsmith::effects::PreventDamageEffect::to_you(4, Until::EndOfTurn)
        .with_filter(DamageFilter::combat())
        .with_follow_up_effects(vec![Effect::gain_life(i32::MAX)]));
    for shield in [native.clone(), materialize_effect(encode_runtime_effect(native).unwrap()).unwrap()] {
        let mut game = game();
        let source = object(&mut game, B, vec![CardType::Creature], ColorSet::RED);
        let creator = object(&mut game, A, vec![CardType::Creature], ColorSet::WHITE);
        execute_effect(&mut game, &shield, &mut EffectContext::new_default(creator, A)).unwrap();
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(source, B,
                ironsmith::events::DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                ironsmith::replacement::ReplacementAction::Double,
            ).with_priority_override(ironsmith::events::ReplacementPriority::SelfReplacement));
        let history = format!("{:?}", game.turn_store.turn_history);
        let error = execute_effect(&mut game,
            &Effect::new(DealDamageEffect::new(3, ChooseSpec::SpecificPlayer(A)).with_combat(true)),
            &mut EffectContext::new_default(source, B)).unwrap_err();
        assert!(matches!(error, ExecutionError::ResourceLimitExceeded { .. }));
        assert_eq!(game.player(A).unwrap().life, 40);
        assert_eq!(format!("{:?}", game.turn_store.turn_history), history);
        assert!(game.effect_store.replacement_effects.get_effect(replacement).is_some());
        assert_eq!(game.effect_store.prevention_effects.shields()[0].amount_remaining, Some(4));
    }
}

fn retained_source_damage(
    game: &mut GameState,
    source: ObjectId,
    target: Target,
    snapshot: &ironsmith::snapshot::ObjectSnapshot,
) -> Result<ironsmith::effect::EffectOutcome, ExecutionError> {
    let target = match target {
        Target::Object(id) => ChooseSpec::SpecificObject(id),
        Target::Player(id) => ChooseSpec::SpecificPlayer(id),
    };
    execute_effect(game, &Effect::new(DealDamageEffect::new(3, target)),
        &mut EffectContext::new_default(source, B).with_source_snapshot(snapshot.clone()))
}

fn dealt_by_retained_source(outcome: ironsmith::effect::EffectOutcome) -> u32 {
    outcome.events.iter().filter_map(|event| event.downcast::<ironsmith::events::DamageEvent>()
        .map(|event| event.amount)).sum()
}

fn prevention_source_quality(game: &mut GameState, source: ObjectId, name: &str, matching: bool) {
    let modification = if name == "Lithomancer's Focus" {
        ironsmith::continuous::Modification::SetColors(
            if matching { ColorSet::COLORLESS } else { ColorSet::BLUE })
    } else {
        ironsmith::continuous::Modification::SetCardTypes(
            if matching { vec![CardType::Artifact, CardType::Creature] }
            else { vec![CardType::Artifact] })
    };
    ironsmith::effects::ApplyContinuousEffect::new(
        ironsmith::continuous::EffectTarget::Specific(source), modification, Until::Forever,
    ).execute(game, &mut EffectContext::new_default(source, B)).unwrap();
}

#[test]
fn focus_and_chameleon_use_current_properties_then_exact_departure_or_phasing_lki() {
    for name in ["Lithomancer's Focus", "Chameleon Blur"] {
        for definition in definitions(name) { for initially_matching in [false, true] {
            let mut game = game();
            let target = if name == "Lithomancer's Focus" {
                Target::Object(object(&mut game, A, vec![CardType::Creature], ColorSet::GREEN))
            } else { Target::Player(A) };
            let source = object(&mut game, B,
                if name == "Chameleon Blur" && !initially_matching { vec![CardType::Artifact] }
                else { vec![CardType::Artifact, CardType::Creature] },
                if name == "Lithomancer's Focus" && !initially_matching { ColorSet::BLUE }
                else { ColorSet::COLORLESS });
            let stale = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(), &game);
            let mut choices = Choices {
                target: (name == "Lithomancer's Focus").then_some(target), ..Default::default()
            };
            cast(&mut game, &definition, &mut choices); settle(&mut game, &mut choices);
            prevention_source_quality(&mut game, source, name, !initially_matching);
            let latest = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(), &game);
            let expected = if initially_matching { 3 } else { 0 };
            // This explicitly supplies the stale ability snapshot: a formerly
            // colorless/creature source cannot override its live blue/artifact state.
            assert_eq!(dealt_by_retained_source(retained_source_damage(
                &mut game, source, target, &stale).unwrap()), expected);
            game.phase_out(source);
            assert!(game.is_phased_out(source));
            assert_eq!(dealt_by_retained_source(retained_source_damage(
                &mut game, source, target, &stale).unwrap()), expected,
                "exact phase-out history takes precedence over an older captured snapshot");
            game.phase_in(source);
            let exiled = game.move_object_by_effect(source, Zone::Exile).unwrap();
            assert!(game.object(source).is_none());
            assert_eq!(dealt_by_retained_source(retained_source_damage(
                &mut game, source, target, &stale).unwrap()), expected,
                "exact departure history is authoritative for the departed source");
            let returned = game.move_object_by_effect(exiled, Zone::Battlefield).unwrap();
            assert_ne!(returned, source);
            prevention_source_quality(&mut game, returned, name, false);
            assert_eq!(dealt_by_retained_source(retained_source_damage(
                &mut game, returned, target, &latest).unwrap()), 3,
                "a fresh incarnation cannot borrow the old source's qualifying snapshot");
            assert_eq!(dealt_by_retained_source(retained_source_damage(
                &mut game, source, target, &stale).unwrap()), expected,
                "the old source's damage still uses its own departure receipt after the blink");
        }}
    }
}

#[test]
fn native_color_and_type_shield_fields_share_the_authoritative_source_frame() {
    let mut filter = DamageFilter::all();
    filter.from_colors = Some(vec![Color::Red]);
    filter.from_card_types = Some(vec![CardType::Creature]);
    let native = Effect::new(ironsmith::effects::PreventDamageEffect::to_you(12, Until::EndOfTurn)
        .with_filter(filter));
    for shield in [native.clone(), materialize_effect(encode_runtime_effect(native).unwrap()).unwrap()] {
        let mut game = game();
        let creator = object(&mut game, A, vec![CardType::Creature], ColorSet::WHITE);
        let source = object(&mut game, B, vec![CardType::Artifact, CardType::Creature], ColorSet::RED);
        let stale = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source).unwrap(), &game);
        execute_effect(&mut game, &shield, &mut EffectContext::new_default(creator, A)).unwrap();
        prevention_source_quality(&mut game, source, "Lithomancer's Focus", false);
        assert_eq!(dealt_by_retained_source(retained_source_damage(
            &mut game, source, Target::Player(A), &stale).unwrap()), 3);
        game.phase_out(source);
        assert_eq!(dealt_by_retained_source(retained_source_damage(
            &mut game, source, Target::Player(A), &stale).unwrap()), 3);
        game.phase_in(source);
        ironsmith::effects::ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(source),
            ironsmith::continuous::Modification::SetColors(ColorSet::RED), Until::Forever,
        ).execute(&mut game, &mut EffectContext::new_default(source, B)).unwrap();
        prevention_source_quality(&mut game, source, "Chameleon Blur", false);
        let exiled = game.move_object_by_effect(source, Zone::Exile).unwrap();
        assert_eq!(dealt_by_retained_source(retained_source_damage(
            &mut game, source, Target::Player(A), &stale).unwrap()), 3,
            "red departure LKI still fails the creature restriction");
        assert_eq!(game.effect_store.prevention_effects.shields()[0].amount_remaining, Some(12));
        let returned = game.move_object_by_effect(exiled, Zone::Battlefield).unwrap();
        let current = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(returned).unwrap(), &game);
        assert_eq!(dealt_by_retained_source(retained_source_damage(
            &mut game, returned, Target::Player(A), &current).unwrap()), 0);
        assert_eq!(game.effect_store.prevention_effects.shields()[0].amount_remaining, Some(9));
        let absent = game.new_object_id();
        assert!(game.turn_store.turn_history.source_last_known_snapshot(absent).is_none());
        let result = ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts(
            &mut game, absent, ironsmith::events::DamageTarget::Player(A), 3, false, false,
            ironsmith::events::cause::EventCause::effect(), Some(&current),
        ).unwrap();
        assert_eq!(result.assignments.iter().map(|assignment| assignment.amount).sum::<u32>(), 3,
            "a different source's qualifying snapshot cannot supply missing source evidence");
        assert_eq!(game.effect_store.prevention_effects.shields()[0].amount_remaining, Some(9));
    }
}

#[test]
fn incomplete_current_source_query_cannot_fall_back_to_a_qualifying_snapshot() {
    for definition in definitions("Lithomancer's Focus") {
        let mut game = game();
        let recipient = object(&mut game, A, vec![CardType::Creature], ColorSet::GREEN);
        let source = object(&mut game, B, vec![CardType::Creature], ColorSet::COLORLESS);
        let stale = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source).unwrap(), &game);
        let mut choices = Choices { target: Some(Target::Object(recipient)), ..Default::default() };
        cast(&mut game, &definition, &mut choices); settle(&mut game, &mut choices);
        let shields = game.effect_store.prevention_effects.shields().to_vec();
        let history = format!("{:?}", game.turn_store.turn_history);
        // The existing checked continuous-query owner rejects this host scalar
        // overflow before any matcher can substitute a stale or default frame.
        game.player_mut(B).unwrap().mana_pool.add(ManaSymbol::Blue, u32::MAX);
        let error = retained_source_damage(&mut game, source, Target::Object(recipient), &stale).unwrap_err();
        assert!(matches!(error, ExecutionError::ContinuousDiscovery(_)));
        assert_eq!(game.effect_store.prevention_effects.shields(), shields.as_slice());
        assert_eq!(format!("{:?}", game.turn_store.turn_history), history);
        assert_eq!(game.damage_on(recipient), 0);
    }
}

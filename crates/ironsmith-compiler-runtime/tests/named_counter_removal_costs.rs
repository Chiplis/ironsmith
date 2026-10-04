//! UNVALIDATED: exact named-source counter costs and modal X announcement.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::object::ObjectKind;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::types::Subtype;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let fixtures: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/named_counter_removal_costs.json.fixture"
    ))
    .unwrap();
    let card = fixtures.iter().find(|card| card["name"] == name).unwrap();
    let text = card["text"].as_str().unwrap();
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    main_phase(&mut game, A);
    game
}
fn main_phase(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}
fn mana(game: &mut GameState, player: PlayerId, symbol: ManaSymbol, count: u32) {
    game.player_mut(player)
        .unwrap()
        .mana_pool
        .add(symbol, count);
}
fn creature(
    game: &mut GameState,
    owner: PlayerId,
    zone: Zone,
    name: &str,
    cost: Option<ManaCost>,
) -> ObjectId {
    let mut builder = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3));
    if let Some(cost) = cost {
        builder = builder.mana_cost(cost);
    }
    game.create_object_from_card(&builder.build(), owner, zone)
}
#[derive(Default)]
struct Choices {
    object: Option<ObjectId>,
    target: Option<Target>,
    x: u32,
    mode: Option<usize>,
    expected_x_bounds: Option<(u32, u32)>,
    saw_x: bool,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.starts_with("Choose mode") {
            if let Some(mode) = self.mode {
                assert!(
                    context
                        .options
                        .iter()
                        .any(|option| option.index == mode && option.legal)
                );
                return vec![mode];
            }
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if let Some(id) = self.object.filter(|id| {
            context
                .candidates
                .iter()
                .any(|candidate| candidate.id == *id && candidate.legal)
        }) {
            return vec![id];
        }
        SelectFirstDecisionMaker.decide_objects(game, context)
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert!(
                context
                    .requirements
                    .iter()
                    .all(|requirement| requirement.legal_targets.contains(&target))
            );
            return vec![target];
        }
        SelectFirstDecisionMaker.decide_targets(game, context)
    }
    fn decide_number(&mut self, _game: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value {
            self.saw_x = true;
            if let Some(bounds) = self.expected_x_bounds {
                assert_eq!((context.min, context.max), bounds);
            }
        }
        assert!(self.x >= context.min && self.x <= context.max);
        self.x
    }
}
fn announce(game: &mut GameState, action: LegalAction, choices: &mut Choices) {
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        choices,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none() && state.pending_cast.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("pending announcement: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices)
            .unwrap();
    }
    assert!(state.pending_activation.is_none() && state.pending_cast.is_none());
    assert_eq!(game.stack.len(), 1);
}
fn activation(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    index: usize,
) -> Option<LegalAction> {
    let index = game
        .current_abilities(source)?
        .iter()
        .enumerate()
        .filter(|(_, ability)| {
            matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))
        })
        .nth(index)?
        .0;
    compute_legal_actions(game, player).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, ability_index } if *id == source && *ability_index == index))
}
fn cast(game: &mut GameState, spell: ObjectId, choices: &mut Choices) {
    let player = game.turn.priority_player.unwrap();
    let action = compute_legal_actions(game, player).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).unwrap();
    announce(game, action, choices);
    resolve_stack_entry_with(game, choices).unwrap();
}

fn counters(game: &GameState, source: ObjectId) -> u32 {
    game.counter_count(source, CounterType::PlusOnePlusOne)
}
fn cast_source(game: &mut GameState, definition: &CardDefinition, taxed: bool) -> ObjectId {
    if taxed {
        let tax = compile_to_runtime_definition(
            "Casting levy",
            "Type: Enchantment\nCreature spells you cast cost {1} more to cast.",
            false,
        )
        .unwrap();
        game.create_object_from_definition(&tax, A, Zone::Battlefield);
    }
    let source = game.create_object_from_definition(definition, A, Zone::Hand);
    let stable = game.object(source).unwrap().stable_id;
    for symbol in [ManaSymbol::Red, ManaSymbol::Green] {
        mana(game, A, symbol, 1);
    }
    if definition.card.name.starts_with("Marath") {
        mana(game, A, ManaSymbol::White, 1);
    } else {
        mana(game, A, ManaSymbol::Colorless, 2);
    }
    if taxed {
        mana(game, A, ManaSymbol::Colorless, 1);
    }
    cast(game, source, &mut Choices::default());
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    game.battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).unwrap().stable_id == stable)
        .unwrap()
}
fn colored_creature(
    game: &mut GameState,
    owner: PlayerId,
    zone: Zone,
    colors: ColorSet,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Color witness")
        .card_types(vec![CardType::Creature])
        .color_indicator(colors)
        .power_toughness(PowerToughness::fixed(3, 5))
        .build();
    game.create_object_from_card(&card, owner, zone)
}
fn destroy(game: &mut GameState, source: ObjectId, target: ObjectId) {
    let mut dm = SelectFirstDecisionMaker;
    let mut context = EffectContext::new(source, A, &mut dm);
    execute_effect(
        game,
        &Effect::destroy(ChooseSpec::SpecificObject(target)),
        &mut context,
    )
    .unwrap();
}
#[test]
fn both_frozen_full_programs_round_trip_without_fallback_or_new_cost_encoding() {
    for name in ["Marath, Will of the Wild", "Ulasht, the Hate Seed"] {
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let activated = definition
                .abilities
                .iter()
                .find_map(|ability| {
                    if let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind {
                        Some(activated)
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(
                activated.activation_x_minimum(),
                if name.starts_with("Marath") { 1 } else { 0 }
            );
            let costs = format!("{:?}", activated.mana_cost);
            assert!(
                costs.contains("RemoveAnyCountersFromSource") || costs.contains("RemoveCounters"),
                "{costs}"
            );
            assert!(
                !costs.contains("RemoveCountersAmong"),
                "named self is never a chosen other permanent: {costs}"
            );
        }
    }
}
#[test]
fn marath_cast_payment_determines_entry_and_all_three_modes_pay_the_announced_x() {
    for definition in definitions("Marath, Will of the Wild") {
        for mode in 0..3 {
            let mut game = game();
            let source = cast_source(&mut game, &definition, true);
            assert_eq!(
                counters(&game, source),
                4,
                "actual four mana spent, not printed mana value three"
            );
            let target = creature(&mut game, B, Zone::Battlefield, "Target", None);
            mana(&mut game, A, ManaSymbol::Colorless, 2);
            let action = activation(&game, A, source, 0).unwrap();
            let mut choices = Choices {
                mode: Some(mode),
                target: (mode != 2).then_some(if mode == 0 {
                    Target::Object(target)
                } else {
                    Target::Player(B)
                }),
                x: 2,
                expected_x_bounds: Some((1, 2)),
                ..Default::default()
            };
            announce(&mut game, action, &mut choices);
            assert!(choices.saw_x);
            assert_eq!(counters(&game, source), 2);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            // Subsequent source state must not redefine X on the stack.
            if mode == 1 {
                destroy(&mut game, target, source);
            } else {
                game.add_counters(source, CounterType::PlusOnePlusOne, 7);
            }
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            match mode {
                0 => assert_eq!(counters(&game, target), 2),
                1 => assert_eq!(game.player(B).unwrap().life, 18),
                _ => {
                    let tokens: Vec<_> = game
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|id| game.object(*id).unwrap().kind == ObjectKind::Token)
                        .collect();
                    assert_eq!(tokens.len(), 1);
                    let token = game.object(tokens[0]).unwrap();
                    assert_eq!(game.current_controller(token.id), Some(A));
                    assert_eq!(token.colors(), ColorSet::GREEN);
                    assert!(token.subtypes.contains(&Subtype::Elemental));
                    assert_eq!(game.current_power(tokens[0]), Some(2));
                    assert_eq!(game.current_toughness(tokens[0]), Some(2));
                }
            }
        }
    }
}
#[test]
fn ulasht_entry_counts_other_controlled_colors_separately_and_each_mode_pays_source_counters() {
    for definition in definitions("Ulasht, the Hate Seed") {
        for mode in 0..2 {
            let mut game = game();
            colored_creature(&mut game, A, Zone::Battlefield, ColorSet::RED);
            colored_creature(&mut game, A, Zone::Battlefield, ColorSet::GREEN);
            let multicolor = colored_creature(
                &mut game,
                A,
                Zone::Battlefield,
                ColorSet::RED.union(ColorSet::GREEN),
            );
            colored_creature(
                &mut game,
                B,
                Zone::Battlefield,
                ColorSet::RED.union(ColorSet::GREEN),
            );
            colored_creature(&mut game, A, Zone::Hand, ColorSet::RED.union(ColorSet::GREEN));
            let source = cast_source(&mut game, &definition, false);
            assert_eq!(
                counters(&game, source),
                4,
                "multicolor counts twice; source, opponent and hand do not count"
            );
            mana(&mut game, A, ManaSymbol::Colorless, 1);
            let action = activation(&game, A, source, 0).unwrap();
            let mut choices = Choices {
                mode: Some(mode),
                target: (mode == 0).then_some(Target::Object(multicolor)),
                ..Default::default()
            };
            announce(&mut game, action, &mut choices);
            assert_eq!(counters(&game, source), 3);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            if mode == 0 {
                destroy(&mut game, multicolor, source);
            }
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            if mode == 0 {
                assert_eq!(game.damage_on(multicolor), 1);
            } else {
                let tokens: Vec<_> = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|id| game.object(*id).unwrap().kind == ObjectKind::Token)
                    .collect();
                assert_eq!(tokens.len(), 1);
                let token = game.object(tokens[0]).unwrap();
                assert_eq!(game.current_controller(token.id), Some(A));
                assert_eq!(token.colors(), ColorSet::GREEN);
                assert!(token.subtypes.contains(&Subtype::Saproling));
                assert_eq!(game.current_power(tokens[0]), Some(1));
                assert_eq!(game.current_toughness(tokens[0]), Some(1));
                // Counter-rich neighbors cannot pay a cost naming this source.
                game.remove_counters(source, CounterType::PlusOnePlusOne, 3, None, None);
                game.add_counters(multicolor, CounterType::PlusOnePlusOne, 9);
                mana(&mut game, A, ManaSymbol::Colorless, 1);
                assert!(activation(&game, A, source, 0).is_none());
                assert_eq!(counters(&game, multicolor), 9);
            }
        }
    }
}
#[test]
fn positive_modal_x_rejects_zero_without_mutation_and_is_bounded_by_source_counters() {
    for definition in definitions("Marath, Will of the Wild") {
        let mut game = game();
        let source = cast_source(&mut game, &definition, false);
        mana(&mut game, A, ManaSymbol::Colorless, 8);
        let action = activation(&game, A, source, 0).unwrap();
        let mut dm = Choices {
            mode: Some(2),
            x: 2,
            expected_x_bounds: Some((1, 3)),
            ..Default::default()
        };
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        loop {
            let GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("expected announcement: {progress:?}");
            };
            if let ironsmith::decisions::context::DecisionContext::Number(number) = &context {
                assert_eq!((number.min, number.max), (1, 3));
                assert!(
                    apply_priority_response_with_dm(
                        &mut game,
                        &mut queue,
                        &mut state,
                        &PriorityResponse::XValue(0),
                        &mut dm
                    )
                    .is_err()
                );
                assert!(state.pending_activation.is_some());
                assert_eq!(counters(&game, source), 3);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 8);
                assert!(game.stack.is_empty());
                // The valid retry uses the same pending announcement.
                progress = apply_decision_context_with_dm(
                    &mut game, &mut queue, &mut state, &context, &mut dm,
                )
                .unwrap();
                break;
            }
            progress = apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &context, &mut dm,
            )
            .unwrap();
        }
        for _ in 0..40 {
            if state.pending_activation.is_none() {
                break;
            }
            let GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("unfinished payment: {progress:?}");
            };
            progress = apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &context, &mut dm,
            )
            .unwrap();
        }
        assert!(state.pending_activation.is_none());
        assert_eq!(counters(&game, source), 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 6);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    }
}
#[test]
fn modal_self_aliases_are_contextual_and_mixed_mode_minima_fail_closed() {
    let text = "Type: Creature\nPower/Toughness: 3/3\n{X}, Remove X +1/+1 counters from Crest Keeper: Choose one —\n• You gain X life. X can't be 0.\n• Draw X cards. X can't be 0.";
    let definition = compile_to_runtime_definition("Crest Keeper", text, false).unwrap();
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(activated) if activated.activation_x_minimum() == 1)));
    assert!(compile_to_runtime_definition("Unrelated", text, false).is_err());
    let mixed = text.replacen("Draw X cards. X can't be 0.", "Draw X cards.", 1);
    assert!(compile_to_runtime_definition("Crest Keeper", &mixed, false).is_err());
}

#[test]
fn paid_targeted_mode_fizzles_when_target_leaves_without_refunding_named_source_cost() {
    for definition in definitions("Marath, Will of the Wild") {
        let mut game = game();
        let source = cast_source(&mut game, &definition, false);
        let target = creature(&mut game, B, Zone::Battlefield, "Departing target", None);
        mana(&mut game, A, ManaSymbol::Colorless, 1);
        let action = activation(&game, A, source, 0).unwrap();
        let mut choices = Choices {
            mode: Some(0),
            x: 1,
            expected_x_bounds: Some((1, 1)),
            target: Some(Target::Object(target)),
            ..Default::default()
        };
        announce(&mut game, action, &mut choices);
        destroy(&mut game, source, target);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(counters(&game, source), 2);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(game.stack.is_empty());
        assert!(!game.battlefield.contains(&target));
    }
}

//! UNVALIDATED source-stage regressions; execution is deferred by the campaign workflow.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{ColorsContext, SelectObjectsContext, SelectOptionsContext};
use ironsmith::effects::{EmitKeywordActionEffect, PutCountersEffect};
use ironsmith::events::other::{KeywordActionEvent, KeywordActionKind};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/compound_keyword_costs.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let card = fixtures()
        .into_iter()
        .find(|card| card["name"] == name)
        .unwrap();
    let text = card["text"].as_str().unwrap();
    let direct = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn keyword_ability(definition: &CardDefinition, action: KeywordActionKind) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return false;
            };
            activated
                .mana_cost
                .costs()
                .iter()
                .filter_map(|cost| cost.effect_ref())
                .any(|effect| {
                    effect
                        .downcast_ref::<EmitKeywordActionEffect>()
                        .is_some_and(|emit| emit.action == action)
                        || effect
                            .downcast_ref::<PutCountersEffect>()
                            .is_some_and(|put| put.completion_action == Some(action))
                })
        })
        .expect("the keyword is a real mandatory cost")
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = PlayerId::from_index(0);
    game.turn.priority_player = Some(PlayerId::from_index(0));
    game
}
fn fixture(game: &mut GameState, player: PlayerId, zone: Zone, food: bool) -> ObjectId {
    let card = if food {
        CardBuilder::new(CardId::new(), "Food resource")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Food])
            .build()
    } else {
        CardBuilder::new(CardId::new(), "Creature resource")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build()
    };
    game.create_object_from_card(&card, player, zone)
}
struct Choices {
    cards: Vec<ObjectId>,
    forage_food: bool,
}
impl DecisionMaker for Choices {
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = self
            .cards
            .iter()
            .copied()
            .filter(|id| {
                ctx.candidates
                    .iter()
                    .any(|candidate| candidate.id == *id && candidate.legal)
            })
            .take(ctx.max.unwrap_or(self.cards.len()))
            .collect::<Vec<_>>();
        assert!(
            selected.len() >= ctx.min,
            "payment choices must be legal and satisfy the exact count"
        );
        selected
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let wanted = if self.forage_food {
            "Sacrifice a Food"
        } else {
            "Exile three cards"
        };
        if let Some(option) = ctx
            .options
            .iter()
            .find(|option| option.legal && option.description.contains(wanted))
        {
            return vec![option.index];
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_colors(&mut self, _game: &GameState, ctx: &ColorsContext) -> Vec<ironsmith::Color> {
        vec![ironsmith::Color::Green; ctx.count as usize]
    }
}
fn action(
    game: &GameState,
    definition: &CardDefinition,
    source: ObjectId,
    kind: KeywordActionKind,
) -> Option<LegalAction> {
    let index = keyword_ability(definition, kind);
    compute_legal_actions(game, PlayerId::from_index(0)).unwrap().into_iter().find(|action| {
        matches!(action, LegalAction::ActivateAbility { source: id, ability_index } | LegalAction::ActivateManaAbility { source: id, ability_index }
            if *id == source && *ability_index == index)
    })
}
fn activate(game: &mut GameState, action: LegalAction, choices: &mut Choices) {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        choices,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none() && state.pending_mana_ability.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("unfinished activation: {progress:?}");
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, choices).unwrap();
    }
    assert!(state.pending_activation.is_none() && state.pending_mana_ability.is_none());
}
fn keyword_events(game: &GameState, kind: KeywordActionKind, player: PlayerId) -> usize {
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .filter_map(|record| record.event.downcast::<KeywordActionEvent>())
        .filter(|event| event.action == kind && event.player == player)
        .count()
}
#[test]
fn seven_complete_compound_keyword_cost_candidates_preserve_full_text_and_typed_artifacts() {
    let cards = fixtures();
    assert_eq!(cards.len(), 8);
    assert_eq!(
        cards
            .iter()
            .filter(|card| card["proposed_coverage"] == "complete")
            .count(),
        7
    );
    for card in cards
        .into_iter()
        .filter(|card| card["proposed_coverage"] == "complete")
    {
        let kind = if card["text"].as_str().unwrap().contains("Forage:") {
            KeywordActionKind::Forage
        } else {
            KeywordActionKind::Blight
        };
        for definition in definitions(card["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            keyword_ability(&definition, kind);
        }
    }
}
#[test]
fn sting_slinger_pays_mana_tap_and_blight_before_damage_and_records_the_actor() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Sting-Slinger") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let victim = fixture(&mut game, alice, Zone::Battlefield, false);
        let foreign = fixture(&mut game, PlayerId::from_index(1), Zone::Battlefield, false);
        assert!(
            action(&game, &definition, source, KeywordActionKind::Blight).is_none(),
            "printed mana remains mandatory"
        );
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 2);
        let next = action(&game, &definition, source, KeywordActionKind::Blight).unwrap();
        activate(
            &mut game,
            next,
            &mut Choices {
                cards: vec![victim],
                forage_food: false,
            },
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert!(game.is_tapped(source));
        assert_eq!(
            game.object(victim)
                .unwrap()
                .counters
                .get(&CounterType::MinusOneMinusOne),
            Some(&1)
        );
        assert!(game.object(foreign).unwrap().counters.is_empty());
        assert_eq!(keyword_events(&game, KeywordActionKind::Blight, alice), 1);
        assert_eq!(game.player(PlayerId::from_index(1)).unwrap().life, 20);
        assert_eq!(game.stack.len(), 1);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        resolve_stack_entry(&mut game).unwrap();
        for index in 1..3 {
            assert_eq!(game.player(PlayerId::from_index(index)).unwrap().life, 18);
        }
    }
}
#[test]
fn evershrikes_gift_requires_own_creature_and_two_counters_before_graveyard_return() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Evershrike's Gift") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        let stable = game.object(source).unwrap().stable_id;
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::White, 2);
        fixture(&mut game, PlayerId::from_index(1), Zone::Battlefield, false);
        assert!(action(&game, &definition, source, KeywordActionKind::Blight).is_none());
        let victim = fixture(&mut game, alice, Zone::Battlefield, false);
        let next = action(&game, &definition, source, KeywordActionKind::Blight).unwrap();
        activate(
            &mut game,
            next,
            &mut Choices {
                cards: vec![victim],
                forage_food: false,
            },
        );
        assert_eq!(
            game.object(victim)
                .unwrap()
                .counters
                .get(&CounterType::MinusOneMinusOne),
            Some(&2)
        );
        assert_eq!(game.object(source).unwrap().zone, Zone::Graveyard);
        resolve_stack_entry(&mut game).unwrap();
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(returned).unwrap().zone, Zone::Hand);
        assert_ne!(source, returned);
    }
}
#[test]
fn thornvault_forager_requires_a_full_own_payment_and_executes_either_forage_branch() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Thornvault Forager") {
        for food in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            let foreign_food = fixture(&mut game, bob, Zone::Battlefield, true);
            for _ in 0..3 {
                fixture(&mut game, bob, Zone::Graveyard, false);
            }
            assert!(action(&game, &definition, source, KeywordActionKind::Forage).is_none());
            let mut cards = vec![];
            if food {
                cards.push(fixture(&mut game, alice, Zone::Battlefield, true));
            } else {
                for _ in 0..2 {
                    cards.push(fixture(&mut game, alice, Zone::Graveyard, false));
                }
                assert!(
                    action(&game, &definition, source, KeywordActionKind::Forage).is_none(),
                    "two cards do not pay forage"
                );
                cards.push(fixture(&mut game, alice, Zone::Graveyard, false));
            }
            let paid = cards
                .iter()
                .map(|id| game.object(*id).unwrap().stable_id)
                .collect::<Vec<_>>();
            let next = action(&game, &definition, source, KeywordActionKind::Forage).unwrap();
            activate(
                &mut game,
                next,
                &mut Choices {
                    cards,
                    forage_food: food,
                },
            );
            assert!(
                game.stack_is_empty(),
                "the printed mana ability does not use the stack"
            );
            assert!(game.is_tapped(source));
            assert_eq!(
                game.player(alice)
                    .unwrap()
                    .mana_pool
                    .amount(ManaSymbol::Green),
                2
            );
            for stable in paid {
                let id = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(
                    game.object(id).unwrap().zone,
                    if food { Zone::Graveyard } else { Zone::Exile }
                );
            }
            assert_eq!(game.object(foreign_food).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.player(bob).unwrap().graveyard.len(), 3);
            assert_eq!(keyword_events(&game, KeywordActionKind::Forage, alice), 1);
            assert_eq!(keyword_events(&game, KeywordActionKind::Forage, bob), 0);
        }
    }
}

#[test]
fn dawnhand_partial_cost_repair_retains_both_blight_counts() {
    // This is expressly a cost-only probe, not complete-card coverage. The
    // fixture retains Dawnhand's full source and records its separate remaining
    // source-exiled permission blocker for the later campaign pass.
    let card = fixtures()
        .into_iter()
        .find(|card| card["name"] == "Dawnhand Dissident")
        .unwrap();
    assert_eq!(card["proposed_coverage"], "partial");
    let cost_probe = card["text"]
        .as_str()
        .unwrap()
        .split("\nDuring your turn,")
        .next()
        .unwrap();
    let definition =
        compile_to_runtime_definition("Dawnhand cost-only probe", cost_probe, false).unwrap();
    let counts = definition
        .abilities
        .iter()
        .filter_map(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return None;
            };
            activated
                .mana_cost
                .costs()
                .iter()
                .filter_map(|cost| cost.effect_ref())
                .filter_map(|effect| effect.downcast_ref::<PutCountersEffect>())
                .find(|put| put.completion_action == Some(KeywordActionKind::Blight))
                .map(|put| put.amount.clone())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        counts,
        vec![
            ironsmith::effect::Value::Fixed(1),
            ironsmith::effect::Value::Fixed(2)
        ]
    );
}

#[test]
fn a_target_opponents_blight_uses_their_creature_and_records_their_action() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Champion of the Weird") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let own = fixture(&mut game, alice, Zone::Battlefield, false);
        let foreign = fixture(&mut game, bob, Zone::Battlefield, false);
        let third = fixture(&mut game, PlayerId::from_index(2), Zone::Battlefield, false);
        let action = action(&game, &definition, source, KeywordActionKind::Blight).unwrap();
        let mut choices = Choices {
            cards: vec![own, foreign],
            forage_food: false,
        };
        activate(&mut game, action, &mut choices);
        assert_eq!(game.player(alice).unwrap().life, 19);
        assert_eq!(
            game.object(own)
                .unwrap()
                .counters
                .get(&CounterType::MinusOneMinusOne),
            Some(&2)
        );
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(
            game.object(foreign)
                .unwrap()
                .counters
                .get(&CounterType::MinusOneMinusOne),
            Some(&2)
        );
        assert_eq!(
            game.object(third)
                .unwrap()
                .counters
                .get(&CounterType::MinusOneMinusOne),
            None
        );
        assert_eq!(keyword_events(&game, KeywordActionKind::Blight, bob), 1);
    }
}

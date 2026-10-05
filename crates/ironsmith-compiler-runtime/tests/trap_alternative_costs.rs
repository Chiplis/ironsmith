//! UNVALIDATED: exact alternative prices, event ownership and full Trap bodies.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{
    AttackerDeclaration, DecisionMaker, LegalAction, SelectFirstDecisionMaker,
    compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DistributeContext, SelectObjectsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_attacker_declarations_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_delayed_triggers};
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/trap_alternative_costs.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows = fixtures();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    main(&mut game, A);
    game
}
fn main(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}
fn mana(game: &mut GameState, player: PlayerId, symbols: &[ManaSymbol]) {
    for symbol in symbols {
        game.player_mut(player).unwrap().mana_pool.add(*symbol, 1);
    }
}
fn printed(game: &mut GameState, name: &str, text: &str, player: PlayerId, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn creature(game: &mut GameState, player: PlayerId, color: ColorSet) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Attack witness")
        .card_types(vec![CardType::Creature])
        .color_indicator(color)
        .power_toughness(PowerToughness::fixed(6, 6))
        .build();
    game.create_object_from_card(&card, player, Zone::Battlefield)
}
fn library(game: &mut GameState, player: PlayerId, count: usize) {
    for n in 0..count {
        printed(
            game,
            &format!("Library {n}"),
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 2/2",
            player,
            Zone::Library,
        );
    }
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    distribution: Vec<(Target, u32)>,
    mana_source: Option<ObjectId>,
    timestamp_last: Option<String>,
}
impl DecisionMaker for Choices {
    fn decide_order(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        if context.description.contains("timestamp")
            && let Some(last) = &self.timestamp_last
        {
            let mut items = context.items.clone();
            items.sort_by_key(|(_, name)| name == last);
            return items.into_iter().map(|(id, _)| id).collect();
        }
        SelectFirstDecisionMaker.decide_order(game, context)
    }
    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::ManaPaymentContext,
    ) -> ironsmith::mana_payment::ManaPaymentResponse {
        if let Some(source) = self.mana_source.take() {
            return ironsmith::mana_payment::ManaPaymentResponse::Activate {
                source,
                ability_index: 0,
            };
        }
        SelectFirstDecisionMaker.decide_mana_payment(game, context)
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if self.targets.is_empty() {
            return SelectFirstDecisionMaker.decide_targets(game, context);
        }
        for target in &self.targets {
            assert!(
                context
                    .requirements
                    .iter()
                    .any(|requirement| requirement.legal_targets.contains(target))
            );
        }
        self.targets.clone()
    }
    fn decide_distribute(
        &mut self,
        _game: &GameState,
        _context: &DistributeContext,
    ) -> Vec<(Target, u32)> {
        self.distribution.clone()
    }
    fn decide_boolean(&mut self, _game: &GameState, _context: &BooleanContext) -> bool {
        true
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(game, context)
    }
}
fn action(
    game: &GameState,
    player: PlayerId,
    spell: ObjectId,
    alternative: bool,
) -> Option<LegalAction> {
    let mut priority = game.clone();
    priority.turn.priority_player = Some(player);
    compute_legal_actions(&priority, player).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == spell && if alternative { matches!(casting_method, CastingMethod::Alternative(_)) } else { matches!(casting_method, CastingMethod::Normal) }))
}
fn announce(game: &mut GameState, action: LegalAction, choices: &mut Choices) {
    let before = game.stack.len();
    let mut state = PriorityLoopState::new(3);
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
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("unfinished announcement: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices)
            .unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    assert_eq!(game.stack.len(), before + 1);
}
fn cast(
    game: &mut GameState,
    player: PlayerId,
    spell: ObjectId,
    alternative: bool,
    choices: &mut Choices,
) {
    game.turn.priority_player = Some(player);
    let action = action(game, player, spell, alternative).unwrap();
    announce(game, action, choices);
    resolve_stack_entry_with(game, choices).unwrap();
}
fn cast_fixture(
    game: &mut GameState,
    player: PlayerId,
    name: &str,
    text: &str,
    targets: Vec<Target>,
) {
    main(game, player);
    let spell = printed(game, name, text, player, Zone::Hand);
    cast(
        game,
        player,
        spell,
        false,
        &mut Choices {
            targets,
            ..Default::default()
        },
    );
}
fn attackers(game: &mut GameState, player: PlayerId, ids: &[ObjectId]) {
    main(game, player);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    for id in ids {
        game.remove_summoning_sickness(*id);
    }
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    let declarations: Vec<_> = ids
        .iter()
        .map(|id| AttackerDeclaration {
            creature: *id,
            target: AttackTarget::Player(if player == A { B } else { A }),
        })
        .collect();
    apply_attacker_declarations_with_dm(
        game,
        &mut combat,
        &mut queue,
        &declarations,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    game.combat = Some(combat);
    game.turn.priority_player = Some(A);
}
#[test]
fn eleven_exact_programs_strict_compile_and_preserve_conditional_alternative_costs() {
    assert_eq!(fixtures().len(), 11);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert_eq!(definition.alternative_casts.len(), 1, "{}", row["name"]);
            assert!(format!("{:?}", definition.alternative_casts).contains("ConditionExpr"));
        }
    }
}
#[test]
fn archive_requires_the_opponent_to_search_their_own_library_and_mills_thirteen_after_free_cast() {
    for definition in definitions("Archive Trap") {
        let mut game = game();
        library(&mut game, B, 20);
        let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
        assert!(action(&game, A, trap, true).is_none());
        // A real failed-to-find search is still a search, but not an ETB/look.
        cast_fixture(
            &mut game,
            B,
            "Search witness",
            "Mana cost: {0}\nType: Instant\nSearch your library for a basic land card, reveal it, put it into your hand, then shuffle.",
            vec![],
        );
        assert!(action(&game, A, trap, true).is_some());
        let before = game.player(B).unwrap().graveyard.len();
        cast(
            &mut game,
            A,
            trap,
            true,
            &mut Choices {
                targets: vec![Target::Player(B)],
                ..Default::default()
            },
        );
        assert_eq!(game.player(B).unwrap().graveyard.len(), before + 13);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn current_attacking_predicates_and_damage_distribution_use_the_announced_combat_state() {
    for name in [
        "Arrow Volley Trap",
        "Lethargy Trap",
        "Pitfall Trap",
        "Slingbow Trap",
    ] {
        for definition in definitions(name) {
            let mut game = game();
            let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
            let count = match name {
                "Arrow Volley Trap" => 4,
                "Lethargy Trap" => 3,
                _ => 1,
            };
            let ids: Vec<_> = (0..count)
                .map(|_| creature(&mut game, B, ColorSet::BLACK))
                .collect();
            if name == "Slingbow Trap" {
                game.object_mut(ids[0]).unwrap().abilities_mut().push(
                    ironsmith::ability::Ability::static_ability(
                        ironsmith::static_abilities::StaticAbility::flying(),
                    ),
                );
            }
            let symbols = match name {
                "Arrow Volley Trap" => vec![ManaSymbol::Colorless, ManaSymbol::White],
                "Lethargy Trap" => vec![ManaSymbol::Blue],
                "Pitfall Trap" => vec![ManaSymbol::White],
                _ => vec![ManaSymbol::Green],
            };
            mana(&mut game, A, &symbols);
            assert!(action(&game, A, trap, true).is_none());
            attackers(&mut game, B, &ids);
            let targets = match name {
                "Lethargy Trap" => vec![],
                "Arrow Volley Trap" => vec![Target::Object(ids[0]), Target::Object(ids[1])],
                _ => vec![Target::Object(ids[0])],
            };
            let distribution = if name == "Arrow Volley Trap" {
                vec![(Target::Object(ids[0]), 3), (Target::Object(ids[1]), 2)]
            } else {
                vec![]
            };
            cast(
                &mut game,
                A,
                trap,
                true,
                &mut Choices {
                    targets,
                    distribution,
                    ..Default::default()
                },
            );
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            match name {
                "Arrow Volley Trap" => {
                    assert_eq!(game.damage_on(ids[0]), 3);
                    assert_eq!(game.damage_on(ids[1]), 2);
                }
                "Lethargy Trap" => {
                    for id in ids {
                        assert_eq!(game.current_power(id), Some(3));
                    }
                }
                _ => assert!(!game.battlefield.contains(&ids[0])),
            }
        }
    }
}
#[test]
fn one_opponents_entry_count_is_not_the_sum_across_opponents_and_survives_departure() {
    for definition in definitions("Whiplash Trap") {
        let mut game = game();
        let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
        mana(&mut game, A, &[ManaSymbol::Blue]);
        cast_fixture(
            &mut game,
            B,
            "Bob entrant",
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 2/2",
            vec![],
        );
        cast_fixture(
            &mut game,
            C,
            "Carol entrant",
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 2/2",
            vec![],
        );
        assert!(
            action(&game, A, trap, true).is_none(),
            "one entry each is not two for any one opponent"
        );
        cast_fixture(
            &mut game,
            B,
            "Bob second entrant",
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 2/2",
            vec![],
        );
        let ids: Vec<_> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| game.current_controller(*id).unwrap() == B)
            .collect();
        assert_eq!(ids.len(), 2);
        cast(
            &mut game,
            A,
            trap,
            true,
            &mut Choices {
                targets: ids.iter().copied().map(Target::Object).collect(),
                ..Default::default()
            },
        );
        assert!(ids.iter().all(|id| !game.battlefield.contains(id)));
        assert_eq!(game.player(B).unwrap().hand.len(), 2);
    }
}

fn search(game: &mut GameState, searcher: PlayerId, owner: PlayerId) {
    let source = game.new_object_id();
    let mut dm = SelectFirstDecisionMaker;
    let mut context = ironsmith::effects::EffectContext::new(source, searcher, &mut dm);
    let outcome = ironsmith::effects::execute_effect(
        game,
        &ironsmith::effect::Effect::new(ironsmith::effects::SearchLibraryEffect::new(
            ironsmith::target::ObjectFilter::land(),
            Zone::Hand,
            ironsmith::target::PlayerFilter::Specific(searcher),
            ironsmith::target::PlayerFilter::Specific(owner),
            false,
        )),
        &mut context,
    )
    .unwrap();
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}
#[test]
fn searcher_library_owner_and_turn_are_separate_conditions() {
    for definition in definitions("Archive Trap") {
        let mut game = game();
        let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
        search(&mut game, A, A);
        assert!(action(&game, A, trap, true).is_none());
        search(&mut game, B, C);
        assert!(action(&game, A, trap, true).is_none());
        search(&mut game, B, B);
        assert!(action(&game, A, trap, true).is_some());
        game.next_turn();
        game.turn.priority_player = Some(A);
        assert!(action(&game, A, trap, true).is_none());
    }
}
#[test]
fn artifact_and_green_entry_history_use_actual_event_characteristics_and_complete_bodies() {
    for name in ["Baloth Cage Trap", "Permafrost Trap"] {
        for definition in definitions(name) {
            let mut game = game();
            let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
            let first = creature(&mut game, B, ColorSet::COLORLESS);
            let second = creature(&mut game, B, ColorSet::COLORLESS);
            let symbols = if name == "Baloth Cage Trap" {
                vec![ManaSymbol::Colorless, ManaSymbol::Green]
            } else {
                vec![ManaSymbol::Blue]
            };
            mana(&mut game, A, &symbols);
            assert!(action(&game, A, trap, true).is_none());
            if name == "Baloth Cage Trap" {
                cast_fixture(
                    &mut game,
                    B,
                    "Artifact entrant",
                    "Mana cost: {0}\nType: Artifact",
                    vec![],
                );
                cast(&mut game, A, trap, true, &mut Choices::default());
                let tokens: Vec<_> = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|id| game.object(*id).unwrap().kind == ObjectKind::Token)
                    .collect();
                assert_eq!(tokens.len(), 1);
                assert_eq!(game.current_power(tokens[0]), Some(4));
                assert_eq!(game.object(tokens[0]).unwrap().colors(), ColorSet::GREEN);
            } else {
                mana(&mut game, B, &[ManaSymbol::Green]);
                cast_fixture(
                    &mut game,
                    B,
                    "Green entrant",
                    "Mana cost: {G}\nType: Creature\nPower/Toughness: 1/1",
                    vec![],
                );
                let entrant = *game
                    .battlefield
                    .iter()
                    .find(|id| game.object(**id).unwrap().name == "Green entrant")
                    .unwrap();
                // A later zone change cannot erase the qualifying earlier entry.
                cast_fixture(
                    &mut game,
                    B,
                    "Own removal",
                    "Mana cost: {0}\nType: Instant\nDestroy target creature.",
                    vec![Target::Object(entrant)],
                );
                cast(
                    &mut game,
                    A,
                    trap,
                    true,
                    &mut Choices {
                        targets: vec![Target::Object(first), Target::Object(second)],
                        ..Default::default()
                    },
                );
                assert!(
                    game.is_tapped(first) && game.is_tapped(second),
                    "first={:?} second={:?} stack={:?}",
                    game.object(first).map(|o| o.zone),
                    game.object(second).map(|o| o.zone),
                    game.stack
                        .iter()
                        .map(|e| (e.object_id, &e.targets))
                        .collect::<Vec<_>>()
                );
                main(&mut game, B);
                game.turn.phase = Phase::Beginning;
                game.turn.step = Some(ironsmith::game_state::Step::Untap);
                ironsmith::turn::execute_untap_step_with(&mut game, &mut SelectFirstDecisionMaker)
                    .unwrap();
                assert!(
                    game.is_tapped(first) && game.is_tapped(second),
                    "first={:?} second={:?} stack={:?}",
                    game.object(first).map(|o| o.zone),
                    game.object(second).map(|o| o.zone),
                    game.stack
                        .iter()
                        .map(|e| (e.object_id, &e.targets))
                        .collect::<Vec<_>>()
                );
            }
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        }
    }
}
#[test]
fn successful_destruction_keeps_frozen_opponent_cause_even_when_the_destination_is_replaced() {
    for definition in definitions("Cobra Trap") {
        for exile_destination in [false, true] {
            let mut game = game();
            let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
            mana(&mut game, A, &[ManaSymbol::Green]);
            let victim = printed(&mut game, "Victim", "Type: Artifact", A, Zone::Battlefield);
            if exile_destination {
                printed(
                    &mut game,
                    "Exile replacement",
                    "Type: Enchantment\nIf a card or token would be put into a graveyard from anywhere, exile it instead.",
                    B,
                    Zone::Battlefield,
                );
            }
            assert!(action(&game, A, trap, true).is_none());
            cast_fixture(
                &mut game,
                B,
                "Opponent destroy",
                "Mana cost: {0}\nType: Instant\nDestroy target artifact.",
                vec![Target::Object(victim)],
            );
            assert!(!game.battlefield.contains(&victim));
            cast(&mut game, A, trap, true, &mut Choices::default());
            let tokens: Vec<_> = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| game.object(*id).unwrap().kind == ObjectKind::Token)
                .collect();
            assert_eq!(tokens.len(), 4);
            for id in tokens {
                assert_eq!(game.current_power(id), Some(1));
                assert_eq!(game.current_controller(id).unwrap(), A);
            }
        }
    }
}
#[test]
fn destruction_condition_does_not_count_own_effects_creatures_sacrifice_or_prevented_destruction() {
    for definition in definitions("Cobra Trap") {
        for case in 0..4 {
            let mut game = game();
            let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
            mana(&mut game, A, &[ManaSymbol::Green]);
            let text = match case {
                1 => "Type: Artifact Creature\nPower/Toughness: 2/2",
                3 => "Type: Artifact\nIndestructible",
                _ => "Type: Artifact",
            };
            let victim = printed(&mut game, "Negative victim", text, A, Zone::Battlefield);
            let actor = if case == 0 || case == 2 { A } else { B };
            if case == 2 {
                cast_fixture(
                    &mut game,
                    A,
                    "Sacrifice witness",
                    "Mana cost: {0}\nType: Instant\nSacrifice an artifact.",
                    vec![],
                );
            } else {
                cast_fixture(
                    &mut game,
                    actor,
                    "Destroy witness",
                    "Mana cost: {0}\nType: Instant\nDestroy target artifact.",
                    vec![Target::Object(victim)],
                );
            }
            assert!(action(&game, A, trap, true).is_none(), "case {case}");
        }
    }
}
#[test]
fn countered_creature_must_be_the_exact_spell_cast_by_you_and_the_counter_must_be_opponents() {
    for definition in definitions("Summoning Trap") {
        for (caster, counter_controller, stolen, expected) in [
            (A, B, false, true),
            (A, A, false, false),
            (B, C, false, false),
            (A, B, true, true),
            (B, C, true, false),
        ] {
            let mut game = game();
            library(&mut game, A, 7);
            let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
            main(&mut game, caster);
            let spell = printed(
                &mut game,
                "Cast creature",
                "Mana cost: {0}\nType: Creature\nPower/Toughness: 3/3",
                caster,
                Zone::Hand,
            );
            let cast_action = action(&game, caster, spell, false).unwrap();
            announce(&mut game, cast_action, &mut Choices::default());
            let stack_spell = game.stack.last().unwrap().object_id;
            if stolen {
                game.set_current_controller(stack_spell, if caster == A { C } else { A })
                    .unwrap();
            }
            let counter = printed(
                &mut game,
                "Counter witness",
                "Mana cost: {0}\nType: Instant\nCounter target spell.",
                counter_controller,
                Zone::Hand,
            );
            cast(
                &mut game,
                counter_controller,
                counter,
                false,
                &mut Choices {
                    targets: vec![Target::Object(stack_spell)],
                    ..Default::default()
                },
            );
            assert!(game.stack.is_empty());
            assert_eq!(action(&game, A, trap, true).is_some(), expected);
            if expected {
                cast(&mut game, A, trap, true, &mut Choices::default());
                assert!(
                    game.battlefield
                        .iter()
                        .any(|id| game.current_controller(*id).unwrap() == A
                            && game.object(*id).unwrap().is_creature())
                );
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            }
        }
    }
}
#[test]
fn nemesis_copies_the_exiled_creatures_copiable_values_and_exiles_only_that_token_next_end_step() {
    for definition in definitions("Nemesis Trap") {
        let mut game = game();
        let victim = creature(&mut game, B, ColorSet::WHITE);
        game.add_counters(victim, CounterType::PlusOnePlusOne, 2);
        let unrelated = printed(
            &mut game,
            "Unrelated",
            "Type: Creature\nPower/Toughness: 1/1",
            A,
            Zone::Battlefield,
        );
        let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
        mana(&mut game, A, &[ManaSymbol::Black, ManaSymbol::Black]);
        attackers(&mut game, B, &[victim]);
        cast(
            &mut game,
            A,
            trap,
            true,
            &mut Choices {
                targets: vec![Target::Object(victim)],
                ..Default::default()
            },
        );
        assert!(!game.battlefield.contains(&victim));
        let copy = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().kind == ObjectKind::Token)
            .unwrap();
        assert_eq!(game.current_controller(copy).unwrap(), A);
        assert_eq!(game.current_power(copy), Some(6));
        assert_eq!(game.counter_count(copy, CounterType::PlusOnePlusOne), 0);
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(Step::End);
        let event = TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfEndStepEvent::new(B),
            Default::default(),
        );
        let mut queue = TriggerQueue::new();
        for entry in check_delayed_triggers(&mut game, &event) {
            queue.add(entry);
        }
        assert_eq!(queue.entries.len(), 1);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut Choices::default()).unwrap();
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
        assert!(!game.battlefield.contains(&copy));
        assert!(game.battlefield.contains(&unrelated));
    }
}

#[test]
fn entry_history_counts_destination_incarnations_and_freezes_entry_control_not_owner() {
    for definition in definitions("Permafrost Trap") {
        let mut game = game();
        let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
        mana(&mut game, A, &[ManaSymbol::Blue]);
        let card = printed(
            &mut game,
            "Borrowed green entrant",
            "Mana cost: {G}\nType: Creature\nPower/Toughness: 2/2",
            A,
            Zone::Graveyard,
        );
        let stable = game.object(card).unwrap().stable_id;
        let reanimate = "Mana cost: {0}\nType: Instant\nPut target creature card from a graveyard onto the battlefield under your control.";
        cast_fixture(
            &mut game,
            B,
            "Entry witness",
            reanimate,
            vec![Target::Object(card)],
        );
        let first = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().stable_id == stable)
            .unwrap();
        assert_eq!(game.object(first).unwrap().owner, A);
        assert_eq!(game.current_controller(first).unwrap(), B);
        let mut filter = ironsmith::target::ObjectFilter::creature();
        filter.colors = Some(ColorSet::GREEN);
        filter.controller = Some(ironsmith::target::PlayerFilter::Specific(B));
        let query = ironsmith::effect::Value::TurnHistoryCount(
            ironsmith::effect::TurnHistoryCount::EnteredBattlefield(filter),
        );
        let count = |game: &GameState| {
            ironsmith::effects::helpers::resolve_value(
                game,
                &query,
                &ironsmith::effects::EffectContext::new_default(trap, A),
            )
            .unwrap()
        };
        assert_eq!(
            count(&game),
            1,
            "zone and ETB notifications describe one entry"
        );
        cast_fixture(
            &mut game,
            A,
            "Removal witness",
            "Mana cost: {0}\nType: Instant\nDestroy target creature.",
            vec![Target::Object(first)],
        );
        assert_eq!(count(&game), 1, "departure cannot erase the entry snapshot");
        let grave = *game
            .player(A)
            .unwrap()
            .graveyard
            .iter()
            .find(|id| game.object(**id).unwrap().stable_id == stable)
            .unwrap();
        cast_fixture(
            &mut game,
            B,
            "Second entry witness",
            reanimate,
            vec![Target::Object(grave)],
        );
        let second = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().stable_id == stable)
            .unwrap();
        assert_ne!(first, second);
        assert_eq!(count(&game), 2, "a later incarnation is a new entry");
        cast(
            &mut game,
            A,
            trap,
            true,
            &mut Choices {
                targets: vec![Target::Object(second)],
                ..Default::default()
            },
        );
        assert!(game.is_tapped(second));
    }
}
#[test]
fn alternative_price_is_optional_and_regular_taxes_apply_after_it_is_chosen() {
    for definition in definitions("Archive Trap") {
        for alternative in [false, true] {
            let mut game = game();
            library(&mut game, B, 14);
            let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
            search(&mut game, B, B);
            printed(
                &mut game,
                "Spell levy",
                "Type: Enchantment\nSpells your opponents cast cost {1} more to cast.",
                B,
                Zone::Battlefield,
            );
            if alternative {
                mana(&mut game, A, &[ManaSymbol::Colorless]);
            } else {
                mana(
                    &mut game,
                    A,
                    &[
                        ManaSymbol::Colorless,
                        ManaSymbol::Colorless,
                        ManaSymbol::Colorless,
                        ManaSymbol::Colorless,
                        ManaSymbol::Blue,
                        ManaSymbol::Blue,
                    ],
                );
            }
            cast(
                &mut game,
                A,
                trap,
                alternative,
                &mut Choices {
                    targets: vec![Target::Player(B)],
                    ..Default::default()
                },
            );
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(B).unwrap().library.len(), 1);
        }
    }
}

#[test]
fn alternative_cost_stays_locked_when_a_mana_ability_changes_the_qualifying_combat_state() {
    for definition in definitions("Arrow Volley Trap") {
        let mut game = game();
        let mana_creature = printed(
            &mut game,
            "Sacrificial mana attacker",
            "Type: Creature\nPower/Toughness: 1/1\nSacrifice this creature: Add {W}.",
            A,
            Zone::Battlefield,
        );
        let other: Vec<_> = (0..3)
            .map(|_| creature(&mut game, A, ColorSet::GREEN))
            .collect();
        let mut attacking = vec![mana_creature];
        attacking.extend_from_slice(&other);
        let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
        mana(&mut game, A, &[ManaSymbol::Colorless]);
        attackers(&mut game, A, &attacking);
        let mut choices = Choices {
            targets: vec![Target::Object(other[0]), Target::Object(other[1])],
            distribution: vec![(Target::Object(other[0]), 3), (Target::Object(other[1]), 2)],
            mana_source: Some(mana_creature),
            ..Default::default()
        };
        cast(&mut game, A, trap, true, &mut choices);
        assert!(!game.battlefield.contains(&mana_creature));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.damage_on(other[0]), 3);
        assert_eq!(game.damage_on(other[1]), 2);
    }
}

#[test]
fn countering_an_uncast_copy_does_not_substitute_the_original_creature_spells_cast_identity() {
    for definition in definitions("Summoning Trap") {
        let mut game = game();
        library(&mut game, A, 7);
        let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
        let spell = printed(
            &mut game,
            "Original creature",
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 2/2",
            A,
            Zone::Hand,
        );
        let cast_action = action(&game, A, spell, false).unwrap();
        announce(&mut game, cast_action, &mut Choices::default());
        let original = game.stack.last().unwrap().object_id;
        cast_fixture(
            &mut game,
            B,
            "Copy witness",
            "Mana cost: {0}\nType: Instant\nCopy target spell.",
            vec![Target::Object(original)],
        );
        assert_eq!(game.stack.len(), 2);
        let copy = game.stack.last().unwrap().object_id;
        assert_ne!(copy, original);
        cast_fixture(
            &mut game,
            C,
            "Copy counter",
            "Mana cost: {0}\nType: Instant\nCounter target spell.",
            vec![Target::Object(copy)],
        );
        assert_eq!(game.stack.len(), 1);
        assert!(action(&game, A, trap, true).is_none());
        cast_fixture(
            &mut game,
            B,
            "Original counter",
            "Mana cost: {0}\nType: Instant\nCounter target spell.",
            vec![Target::Object(original)],
        );
        assert!(action(&game, A, trap, true).is_some());
        cast(&mut game, A, trap, true, &mut Choices::default());
    }
}

#[test]
fn foreign_owned_trap_permission_uses_the_prospective_caster_for_its_condition() {
    for definition in definitions("Archive Trap") {
        for qualifying_search in [false, true] {
            let mut game = game();
            library(&mut game, B, 20);
            let trap = game.create_object_from_definition(&definition, B, Zone::Exile);
            assert_eq!(game.current_controller(trap).unwrap(), B);
            game.effect_store.grant_registry.grant_play_from_to_card(
                trap,
                Zone::Exile,
                A,
                Default::default(),
                ironsmith::grant_registry::GrantSource::Effect {
                    source_id: trap,
                    expires_end_of_turn: game.turn.turn_number,
                },
            );
            let searcher = if qualifying_search { B } else { A };
            search(&mut game, searcher, searcher);
            main(&mut game, A);
            let available = compute_legal_actions(&game, A).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::CastSpell {
                    spell_id, casting_method: CastingMethod::PlayFrom { use_alternative: Some(_), .. }, ..
                } if *spell_id == trap));
            assert_eq!(
                available.is_some(),
                qualifying_search,
                "the owner must not stand in for the actual caster"
            );
            if let Some(action) = available {
                let before = game.player(B).unwrap().library.len();
                let mut choices = Choices {
                    targets: vec![Target::Player(B)],
                    ..Default::default()
                };
                announce(&mut game, action, &mut choices);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
                resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                assert_eq!(game.player(B).unwrap().library.len(), before - 13);
            }
        }
    }
}

fn color_setter(
    game: &mut GameState,
    name: &str,
    color: ColorSet,
    zone: Zone,
    creature: bool,
) -> ObjectId {
    let type_line = if creature { "Creature" } else { "Enchantment" };
    let mut definition = compile_to_runtime_definition(
        name,
        &format!("Type: {type_line}\nPower/Toughness: 2/2"),
        false,
    )
    .unwrap();
    definition
        .abilities
        .push(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::set_colors(
                ironsmith::target::ObjectFilter::creature(),
                color,
            ),
        ));
    let id = game.create_object_from_definition(&definition, B, zone);
    id
}
fn green_entries(game: &GameState, source: ObjectId) -> i32 {
    let mut filter = ironsmith::target::ObjectFilter::creature();
    filter.colors = Some(ColorSet::GREEN);
    filter.controller = Some(ironsmith::target::PlayerFilter::Specific(B));
    ironsmith::effects::helpers::resolve_value(
        game,
        &ironsmith::effect::Value::TurnHistoryCount(
            ironsmith::effect::TurnHistoryCount::EnteredBattlefield(filter),
        ),
        &ironsmith::effects::EffectContext::new_default(source, A),
    )
    .unwrap()
}

#[test]
fn simultaneous_entry_history_waits_for_every_entrant_and_the_chosen_timestamp_order() {
    for definition in definitions("Permafrost Trap") {
        for green_last in [true, false] {
            let mut game = game();
            let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
            mana(&mut game, A, &[ManaSymbol::Blue]);
            printed(
                &mut game,
                "First colorless entrant",
                "Mana cost: {0}\nType: Creature\nPower/Toughness: 2/2",
                B,
                Zone::Graveyard,
            );
            color_setter(
                &mut game,
                "Green frame",
                ColorSet::GREEN,
                Zone::Graveyard,
                true,
            );
            color_setter(
                &mut game,
                "Blue frame",
                ColorSet::BLUE,
                Zone::Graveyard,
                true,
            );
            let restore = printed(
                &mut game,
                "Restore the batch",
                "Mana cost: {0}\nType: Instant\nReturn all creature cards from your graveyard to the battlefield.",
                B,
                Zone::Hand,
            );
            let mut choices = Choices {
                timestamp_last: Some(
                    if green_last {
                        "Green frame"
                    } else {
                        "Blue frame"
                    }
                    .into(),
                ),
                ..Default::default()
            };
            main(&mut game, B);
            cast(&mut game, B, restore, false, &mut choices);
            assert_eq!(green_entries(&game, trap), if green_last { 3 } else { 0 });
            assert_eq!(action(&game, A, trap, true).is_some(), green_last);
            let expected = if green_last {
                ColorSet::GREEN
            } else {
                ColorSet::BLUE
            };
            for &id in &game.battlefield {
                assert_eq!(game.current_colors(id), Some(expected));
            }
            // Every zone event preserves origin LKI, with a separate completed
            // destination frame. Per-object views retain the exact same frame.
            let history = &game.turn_store.turn_history;
            let entries = history
                .event_records
                .iter()
                .chain(history.staged_event_records.iter())
                .filter_map(|record| {
                    record
                        .event
                        .downcast::<ironsmith::events::ZoneChangeEvent>()
                })
                .filter(|zone| zone.to == Zone::Battlefield)
                .collect::<Vec<_>>();
            assert!(!entries.is_empty());
            for entry in entries {
                for &id in entry.destination_objects() {
                    assert_eq!(entry.destination_snapshot(id).unwrap().colors, expected);
                }
                if let Some(parts) = entry.per_object_events(&game) {
                    for part in parts {
                        assert_eq!(part.destination_snapshots.len(), 1);
                        assert_eq!(
                            part.destination_snapshot(part.destination_objects()[0])
                                .unwrap()
                                .colors,
                            expected
                        );
                    }
                }
                assert!(
                    entry
                        .snapshots()
                        .iter()
                        .all(|snapshot| snapshot.zone == Zone::Graveyard)
                );
            }
        }
    }
}

#[test]
fn reported_token_entry_history_keeps_continuous_colors_before_the_next_instruction() {
    for definition in definitions("Permafrost Trap") {
        let mut game = game();
        let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
        mana(&mut game, A, &[ManaSymbol::Blue]);
        let setter = color_setter(
            &mut game,
            "Green observer",
            ColorSet::GREEN,
            Zone::Battlefield,
            false,
        );
        cast_fixture(
            &mut game,
            B,
            "Token entry then remove the color source",
            "Mana cost: {0}\nType: Instant\nCreate two 1/1 colorless Construct artifact creature tokens. Destroy target enchantment.",
            vec![Target::Object(setter)],
        );
        assert!(!game.battlefield.contains(&setter));
        let tokens = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| game.object(*id).unwrap().kind == ObjectKind::Token)
            .collect::<Vec<_>>();
        assert_eq!(tokens.len(), 2);
        assert!(
            tokens
                .iter()
                .all(|id| game.current_colors(*id) == Some(ColorSet::default()))
        );
        assert_eq!(green_entries(&game, trap), 2);
        assert!(action(&game, A, trap, true).is_some());
        cast(
            &mut game,
            A,
            trap,
            true,
            &mut Choices {
                targets: tokens.iter().copied().map(Target::Object).collect(),
                ..Default::default()
            },
        );
        assert!(tokens.iter().all(|id| game.is_tapped(*id)));
    }
}

#[test]
fn each_player_entry_batch_freezes_the_first_players_card_after_the_later_players_setter_enters() {
    for definition in definitions("Permafrost Trap") {
        let mut game = game();
        let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
        mana(&mut game, A, &[ManaSymbol::Blue]);
        let first = printed(
            &mut game,
            "Earlier player's colorless card",
            "Mana cost: {0}\nType: Creature\nPower/Toughness: 2/2",
            B,
            Zone::Graveyard,
        );
        let stable = game.object(first).unwrap().stable_id;
        let setter = CardBuilder::new(CardId::new(), "Later player's green frame")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let setter = game.create_object_from_card(&setter, C, Zone::Graveyard);
        game.object_mut(setter).unwrap().abilities_mut().push(
            ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::set_colors(
                    ironsmith::target::ObjectFilter::creature(),
                    ColorSet::GREEN,
                ),
            ),
        );
        // With Bob active, Carol's original return commits after Bob's.
        cast_fixture(
            &mut game,
            B,
            "Each player's return",
            "Mana cost: {0}\nType: Instant\nEach player returns all creature cards from their graveyard to the battlefield.",
            vec![],
        );
        let first = game
            .battlefield
            .iter()
            .copied()
            .find(|id| game.object(*id).unwrap().stable_id == stable)
            .unwrap();
        assert_eq!(game.current_colors(first), Some(ColorSet::GREEN));
        assert_eq!(
            green_entries(&game, trap),
            1,
            "the first participant must observe the whole original simultaneous batch"
        );
        assert!(action(&game, A, trap, true).is_some());
        let history = &game.turn_store.turn_history;
        assert!(
            history
                .event_records
                .iter()
                .chain(history.staged_event_records.iter())
                .any(|record| record
                    .event
                    .downcast::<ironsmith::events::ZoneChangeEvent>()
                    .is_some_and(|zone| zone
                        .destination_snapshot(first)
                        .is_some_and(|snapshot| snapshot.colors == ColorSet::GREEN)))
        );
    }
}

#[test]
fn each_player_token_entry_and_creation_additions_wait_for_every_original_participant() {
    for definition in definitions("Permafrost Trap") {
        for entry_addition in [false, true] {
            let mut game = game();
            let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
            mana(&mut game, A, &[ManaSymbol::Blue]);
            let setter = color_setter(
                &mut game,
                "Green until the addition",
                ColorSet::GREEN,
                Zone::Battlefield,
                false,
            );
            let additions = vec![
                ironsmith::effect::Effect::gain_life(ironsmith::effect::Value::Count(
                    ironsmith::target::ObjectFilter::creature().token(),
                )),
                ironsmith::effect::Effect::destroy(ironsmith::target::ChooseSpec::SpecificObject(
                    setter,
                )),
            ];
            let replacement = if entry_addition {
                ironsmith::replacement::ReplacementEffect::with_matcher(
                    setter,
                    B,
                    ironsmith::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                        ironsmith::target::ObjectFilter::creature()
                            .token()
                            .you_control(),
                    ),
                    ironsmith::replacement::ReplacementAction::Additionally(additions),
                )
            } else {
                ironsmith::replacement::ReplacementEffect::with_matcher(
                    setter,
                    B,
                    ironsmith::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(
                        ironsmith::target::PlayerFilter::You,
                    ),
                    ironsmith::replacement::ReplacementAction::Additionally(additions),
                )
            };
            game.effect_store
                .replacement_effects
                .add_one_shot_effect(replacement);
            cast_fixture(
                &mut game,
                B,
                "Each player's token",
                "Mana cost: {0}\nType: Instant\nEach player creates a 1/1 colorless Construct artifact creature token.",
                vec![],
            );
            assert_eq!(
                game.player(B).unwrap().life,
                23,
                "the addition must see all three players' original tokens"
            );
            assert!(!game.battlefield.contains(&setter));
            assert_eq!(
                green_entries(&game, trap),
                1,
                "entry facts must freeze before the addition removes the color source"
            );
            assert!(action(&game, A, trap, true).is_some());
            let tokens = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| game.object(*id).unwrap().kind == ObjectKind::Token)
                .collect::<Vec<_>>();
            assert_eq!(tokens.len(), 3);
            assert!(
                tokens
                    .iter()
                    .all(|id| game.current_colors(*id) == Some(ColorSet::default()))
            );
        }
    }
}

#[test]
fn arrow_volley_keeps_announced_shares_and_commits_all_allocations_before_additions() {
    for definition in definitions("Arrow Volley Trap") {
        for illegal_first in [false, true] {
            let mut game = game();
            let ids = (0..4)
                .map(|_| creature(&mut game, B, ColorSet::GREEN))
                .collect::<Vec<_>>();
            attackers(&mut game, B, &ids);
            let trap = game.create_object_from_definition(&definition, A, Zone::Hand);
            mana(&mut game, A, &[ManaSymbol::Colorless, ManaSymbol::White]);
            let mut choices = Choices {
                targets: vec![Target::Object(ids[0]), Target::Object(ids[1])],
                distribution: vec![(Target::Object(ids[0]), 3), (Target::Object(ids[1]), 2)],
                ..Default::default()
            };
            let action = action(&game, A, trap, true).unwrap();
            announce(&mut game, action, &mut choices);
            let spell = game.stack.last().unwrap().object_id;
            if illegal_first {
                let moved = game.move_object_by_effect(ids[0], Zone::Exile).unwrap();
                game.move_object_by_effect(moved, Zone::Battlefield)
                    .unwrap();
            } else {
                game.effect_store.replacement_effects.add_one_shot_effect(
                    ironsmith::replacement::ReplacementEffect::with_matcher(
                        spell,
                        A,
                        ironsmith::events::damage::matchers::DamageToObjectMatcher::new(
                            ironsmith::target::ObjectFilter::specific(ids[0]),
                        ),
                        ironsmith::replacement::ReplacementAction::Additionally(vec![
                            ironsmith::effect::Effect::exile(
                                ironsmith::target::ChooseSpec::SpecificObject(ids[1]),
                            ),
                        ]),
                    ),
                );
            }
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            let receipts = game
                .turn_store
                .turn_history
                .event_records
                .iter()
                .chain(game.turn_store.turn_history.staged_event_records.iter())
                .filter(|record| {
                    record
                        .event
                        .downcast::<ironsmith::events::DamageEvent>()
                        .is_some_and(|event| event.source == spell)
                })
                .collect::<Vec<_>>();
            assert_eq!(receipts.len(), if illegal_first { 1 } else { 2 });
            assert!(receipts.iter().any(|record| {
                record
                    .event
                    .downcast::<ironsmith::events::DamageEvent>()
                    .is_some_and(|event| {
                        event.target == ironsmith::events::DamageTarget::Object(ids[1])
                            && event.amount == 2
                    })
            }));
            if illegal_first {
                assert_eq!(game.damage_on(ids[1]), 2);
            } else {
                assert!(!game.battlefield.contains(&ids[1]));
                assert_eq!(game.damage_on(ids[0]), 3);
                assert!(receipts[0].event.simultaneous_batch().is_some());
                assert_eq!(
                    receipts[0].event.simultaneous_batch(),
                    receipts[1].event.simultaneous_batch()
                );
            }
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        }
    }
}

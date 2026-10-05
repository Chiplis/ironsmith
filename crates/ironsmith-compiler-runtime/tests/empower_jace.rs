//! UNVALIDATED implementation-first regressions for final CR 701.71.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{PartitionContext, SelectObjectsContext, TargetsContext};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, EmpowerJaceEffect, execute_effect};
use ironsmith::events::other::{KeywordActionEvent, KeywordActionKind};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::static_abilities::StaticAbility;
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    Ability, CardId, CardType, ColorSet, GameProgress, GameState, ObjectId, PlayerId, Subtype,
    Target, Zone,
};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/empower_jace.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let card = fixtures()
        .into_iter()
        .find(|card| card["name"] == name)
        .unwrap();
    let text = card["text"].as_str().unwrap();
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
    ] {
        game.player_mut(alice).unwrap().mana_pool.add(color, 20);
    }
    let filler = CardBuilder::new(CardId::new(), "Library fixture")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..8 {
        game.create_object_from_card(&filler, alice, Zone::Library);
    }
    game
}
fn jaces(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == ObjectKind::Token
                    && game.current_controller(*id) == Some(player)
                    && game
                        .current_card_types(*id)
                        .unwrap()
                        .contains(&CardType::Planeswalker)
                    && game.calculated_subtypes(*id).contains(&Subtype::Jace)
            })
        })
        .collect()
}
fn loyalty(game: &GameState, object: ObjectId) -> u32 {
    game.object(object)
        .unwrap()
        .counters
        .get(&CounterType::Loyalty)
        .copied()
        .unwrap_or(0)
}
fn creature(game: &mut GameState, player: PlayerId) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Target creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        player,
        Zone::Battlefield,
    )
}
#[derive(Default)]
struct Choices {
    target: Option<Target>,
    chosen_jace: Option<ObjectId>,
    saw_zero_loyalty_choice: bool,
    surveil_to_graveyard: bool,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert!(
                ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(&target))
            );
            vec![target]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, ctx)
        }
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        if ctx.description.contains("empower") {
            assert_eq!(ctx.min, 1);
            assert_eq!(ctx.max, Some(1));
            self.saw_zero_loyalty_choice |=
                legal.len() == 2 && legal.iter().all(|id| loyalty(game, *id) == 0);
            if let Some(chosen) = self.chosen_jace {
                assert!(legal.contains(&chosen));
                return vec![chosen];
            }
        }
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
    fn decide_partition(&mut self, _game: &GameState, ctx: &PartitionContext) -> Vec<ObjectId> {
        if self.surveil_to_graveyard {
            ctx.cards.iter().map(|(id, _)| *id).collect()
        } else {
            vec![]
        }
    }
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    assert!(
        compute_legal_actions(game, PlayerId::from_index(0))
            .unwrap()
            .contains(&action)
    );
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("unfinished announcement: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    assert_eq!(game.stack.len(), 1);
}
fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) {
    let source =
        game.create_object_from_definition(definition, PlayerId::from_index(0), Zone::Hand);
    announce(
        game,
        LegalAction::CastSpell {
            spell_id: source,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        },
        dm,
    );
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
    assert!(game.stack_is_empty());
}
fn has_empower(effect: &Effect) -> bool {
    if effect.downcast_ref::<EmpowerJaceEffect>().is_some() {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| found |= has_empower(child));
    found
}

#[test]
fn empower_jace_complete_candidates_strict_compile_and_round_trip_with_typed_amounts() {
    let cards = fixtures();
    assert_eq!(cards.len(), 35);
    assert_eq!(
        cards
            .iter()
            .filter(|card| card["proposed_coverage"] == "partial")
            .count(),
        3
    );
    for card in cards
        .into_iter()
        .filter(|card| card["proposed_coverage"] == "complete")
    {
        for definition in definitions(card["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let programs =
                definition
                    .spell_effect
                    .iter()
                    .chain(
                        definition
                            .abilities
                            .iter()
                            .filter_map(|ability| match &ability.kind {
                                AbilityKind::Activated(a) => Some(&a.effects),
                                AbilityKind::Triggered(t) => Some(&t.effects),
                                _ => None,
                            }),
                    );
            assert!(
                programs
                    .flat_map(|program| program.all_effects())
                    .any(has_empower),
                "{}",
                definition.card.name
            );
            assert!(
                ironsmith_text::compiled_text::unprocessed_compiled_lines(&definition)
                    .join("\n")
                    .to_ascii_lowercase()
                    .contains("empower jace"),
                "{}: {}",
                definition.card.name,
                ironsmith_text::compiled_text::unprocessed_compiled_lines(&definition).join("\n")
            );
        }
    }
}

#[test]
fn empower_jace_creates_final_blue_token_with_real_loyalty_costs_and_reuses_it() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Protege's Awakening") {
        let mut game = game();
        let mut dm = Choices::default();
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        let tokens = jaces(&game, alice);
        assert_eq!(tokens.len(), 1);
        let jace = tokens[0];
        assert_eq!(game.object(jace).unwrap().name, "Jace Token");
        assert_eq!(game.current_colors(jace), Some(ColorSet::BLUE));
        assert_eq!(loyalty(&game, jace), 6);
        assert!(
            !game
                .object(jace)
                .unwrap()
                .supertypes
                .contains(&ironsmith::Supertype::Legendary)
        );
        let abilities = game.object(jace).unwrap().abilities.clone();
        assert_eq!(abilities.len(), 2);
        for ability in abilities.iter() {
            assert!(matches!(&ability.kind, AbilityKind::Activated(a) if a.is_loyalty_ability));
        }
        let graveyard_before = game.player(alice).unwrap().graveyard.len();
        dm.surveil_to_graveyard = true;
        announce(
            &mut game,
            LegalAction::ActivateAbility {
                source: jace,
                ability_index: 0,
            },
            &mut dm,
        );
        assert_eq!(
            loyalty(&game, jace),
            5,
            "surveil ability pays one loyalty before resolving"
        );
        resolve(&mut game, &mut dm);
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            graveyard_before + 1
        );
        assert!(
            !compute_legal_actions(&game, alice).unwrap().iter().any(
                |a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == jace)
            ),
            "one loyalty activation per turn"
        );
        game.next_turn();
        game.next_turn();
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        let hand_before = game.player(alice).unwrap().hand.len();
        announce(
            &mut game,
            LegalAction::ActivateAbility {
                source: jace,
                ability_index: 1,
            },
            &mut dm,
        );
        assert_eq!(loyalty(&game, jace), 2, "draw ability pays three loyalty");
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(alice).unwrap().hand.len(), hand_before + 1);
        for symbol in [ManaSymbol::Blue, ManaSymbol::Colorless] {
            game.player_mut(alice).unwrap().mana_pool.add(symbol, 5);
        }
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(
            jaces(&game, alice),
            vec![jace],
            "an existing token forbids optional fresh creation"
        );
        assert_eq!(loyalty(&game, jace), 8);
        let empower_events = game
            .turn_store
            .turn_history
            .event_records
            .iter()
            .chain(game.turn_store.turn_history.staged_event_records.iter())
            .filter_map(|record| record.event.downcast::<KeywordActionEvent>())
            .filter(|event| event.action == KeywordActionKind::EmpowerJace && event.player == alice)
            .count();
        assert_eq!(
            empower_events, 1,
            "one keyword-action event for the later turn's empower"
        );
    }
}

#[test]
fn empower_jace_ignores_nontokens_and_opponents_but_chooses_a_copied_jace_token() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Protege's Awakening") {
        let mut game = game();
        let original = CardDefinitionBuilder::new(CardId::new(), "Alternate Jace")
            .card_types(vec![CardType::Planeswalker])
            .subtypes(vec![Subtype::Jace])
            .color_indicator(ColorSet::RED)
            .loyalty(4)
            .build();
        let nontoken = game.create_object_from_definition(&original, alice, Zone::Battlefield);
        let foreign_token = CardDefinitionBuilder::new(CardId::new(), "Foreign Jace")
            .token()
            .card_types(vec![CardType::Planeswalker])
            .subtypes(vec![Subtype::Jace])
            .loyalty(9)
            .build();
        let foreign = game.create_object_from_definition(&foreign_token, bob, Zone::Battlefield);
        let mut dm = Choices::default();
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        let first = jaces(&game, alice)[0];
        assert_eq!(loyalty(&game, nontoken), 4);
        assert_eq!(loyalty(&game, foreign), 9);
        let mut ctx = EffectContext::new_default(nontoken, alice);
        execute_effect(
            &mut game,
            &Effect::create_token_copy(ChooseSpec::SpecificObject(nontoken)),
            &mut ctx,
        )
        .unwrap();
        let copy = *jaces(&game, alice).iter().find(|id| **id != first).unwrap();
        assert_eq!(game.current_colors(copy), Some(ColorSet::RED));
        assert_eq!(game.object(copy).unwrap().name, "Alternate Jace");
        dm.chosen_jace = Some(copy);
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(jaces(&game, alice).len(), 2);
        assert_eq!(loyalty(&game, copy), 10);
        assert_eq!(loyalty(&game, first), 6);
        assert_eq!(loyalty(&game, nontoken), 4);
        assert_eq!(loyalty(&game, foreign), 9);
        assert_eq!(
            game.current_colors(copy),
            Some(ColorSet::RED),
            "existing copies keep their characteristics"
        );
    }
}

#[test]
fn empower_jace_token_and_counter_doublers_preserve_zero_loyalty_until_the_action_finishes() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Protege's Awakening") {
        let mut game = game();
        let doubler = CardDefinitionBuilder::new(CardId::new(), "Replacement fixture")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(
                StaticAbility::double_token_creation_replacement(
                    PlayerFilter::You,
                    "Double tokens".into(),
                ),
            ))
            .with_ability(Ability::static_ability(
                StaticAbility::double_counters_replacement(
                    ObjectFilter::default().you_control(),
                    Some(CounterType::Loyalty),
                    "Double loyalty".into(),
                ),
            ))
            .build();
        game.create_object_from_definition(&doubler, alice, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let mut dm = Choices::default();
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert!(
            dm.saw_zero_loyalty_choice,
            "both replacement-created tokens survive until the loyalty choice"
        );
        let mut counts = jaces(&game, alice)
            .into_iter()
            .map(|id| loyalty(&game, id))
            .collect::<Vec<_>>();
        counts.sort();
        assert_eq!(
            counts,
            vec![0, 12],
            "only the chosen token gets the doubled counters"
        );
        ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
        let tokens = jaces(&game, alice);
        assert_eq!(tokens.len(), 1);
        assert_eq!(loyalty(&game, tokens[0]), 12);
    }
}

#[test]
fn empower_jace_does_not_happen_when_a_spells_only_target_becomes_illegal() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Academic Ascent") {
        let mut game = game();
        let target = creature(&mut game, alice);
        let mut dm = Choices {
            target: Some(Target::Object(target)),
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        game.move_object_by_effect(target, Zone::Exile).unwrap();
        resolve(&mut game, &mut dm);
        assert!(jaces(&game, alice).is_empty());
    }
}

#[test]
fn empower_jace_where_x_retains_the_exact_prior_exile_result() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Overwrite the Multiverse") {
        let mut game = game();
        for player in [alice, PlayerId::from_index(1), alice] {
            creature(&mut game, player);
        }
        let mut dm = Choices::default();
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.exile.len(), 3);
        let tokens = jaces(&game, alice);
        assert_eq!(tokens.len(), 1);
        assert_eq!(loyalty(&game, tokens[0]), 3);
    }
}

#[test]
fn empower_jace_zero_still_creates_a_token_then_normal_state_based_actions_remove_it() {
    let definition = compile_to_runtime_definition(
        "Zero empower probe",
        "Mana cost: {0}\nType: Sorcery\nEmpower Jace 0.",
        false,
    )
    .unwrap();
    let mut game = game();
    let mut dm = Choices::default();
    cast(&mut game, &definition, &mut dm);
    resolve(&mut game, &mut dm);
    let tokens = jaces(&game, PlayerId::from_index(0));
    assert_eq!(tokens.len(), 1);
    assert_eq!(loyalty(&game, tokens[0]), 0);
    ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
    assert!(jaces(&game, PlayerId::from_index(0)).is_empty());
}

#[test]
fn empower_jace_pending_choice_rolls_back_token_creation_and_replays_once() {
    struct PendingChoice {
        pending: bool,
        suspend: bool,
    }
    impl DecisionMaker for PendingChoice {
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if self.suspend {
                self.pending = true;
                vec![]
            } else {
                SelectFirstDecisionMaker.decide_objects(game, ctx)
            }
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = game();
    let doubler = CardDefinitionBuilder::new(CardId::new(), "Choice checkpoint fixture")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(
            StaticAbility::double_token_creation_replacement(
                PlayerFilter::You,
                "Double tokens".into(),
            ),
        ))
        .build();
    let source = game.create_object_from_definition(&doubler, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let effect = Effect::new(EmpowerJaceEffect::new(3));
    let mut dm = PendingChoice {
        pending: false,
        suspend: true,
    };
    {
        let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
        execute_effect(&mut game, &effect, &mut ctx).unwrap();
    }
    assert!(dm.pending);
    assert!(
        jaces(&game, alice).is_empty(),
        "no partial token creation escapes a suspended action"
    );
    dm.pending = false;
    dm.suspend = false;
    {
        let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
        execute_effect(&mut game, &effect, &mut ctx).unwrap();
    }
    let mut counts = jaces(&game, alice)
        .iter()
        .map(|id| loyalty(&game, *id))
        .collect::<Vec<_>>();
    counts.sort();
    assert_eq!(counts, vec![0, 3]);
}

#[test]
fn sanctum_lurker_zero_loyalty_exception_tracks_live_control_and_source_departure() {
    use ironsmith::rules::state_based::{StateBasedAction, check_state_based_actions};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Sanctum Lurker") {
        let mut game = game();
        let lurker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let zero = CardDefinitionBuilder::new(CardId::new(), "Zero loyalty fixture")
            .card_types(vec![CardType::Planeswalker])
            .subtypes(vec![Subtype::Jace])
            .loyalty(0)
            .build();
        let yours = game.create_object_from_definition(&zero, alice, Zone::Battlefield);
        let theirs = game.create_object_from_definition(&zero, bob, Zone::Battlefield);
        let owns_but_does_not_control =
            game.create_object_from_definition(&zero, alice, Zone::Battlefield);
        game.set_current_controller(owns_but_does_not_control, bob)
            .unwrap();
        let deaths = check_state_based_actions(&game);
        assert!(!deaths.contains(&StateBasedAction::PlaneswalkerDies(yours)));
        assert!(deaths.contains(&StateBasedAction::PlaneswalkerDies(theirs)));
        assert!(deaths.contains(&StateBasedAction::PlaneswalkerDies(
            owns_but_does_not_control
        )));
        assert!(
            game.can_be_destroyed(yours),
            "the exception does not grant indestructible"
        );
        let hybrid = CardDefinitionBuilder::new(CardId::new(), "Zero toughness walker")
            .card_types(vec![CardType::Planeswalker, CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 0))
            .loyalty(0)
            .build();
        let hybrid = game.create_object_from_definition(&hybrid, alice, Zone::Battlefield);
        assert!(check_state_based_actions(&game).contains(&StateBasedAction::ObjectDies(hybrid)));
        game.move_object_by_effect(hybrid, Zone::Graveyard).unwrap();
        // Prime the incremental cache before changing only the rule's source.
        assert_eq!(check_state_based_actions(&game), deaths);
        game.set_current_controller(lurker, bob).unwrap();
        let deaths = check_state_based_actions(&game);
        assert!(deaths.contains(&StateBasedAction::PlaneswalkerDies(yours)));
        assert!(!deaths.contains(&StateBasedAction::PlaneswalkerDies(theirs)));
        game.set_current_controller(lurker, alice).unwrap();
        assert!(
            !check_state_based_actions(&game).contains(&StateBasedAction::PlaneswalkerDies(yours))
        );
        game.phase_out(lurker);
        assert!(
            check_state_based_actions(&game).contains(&StateBasedAction::PlaneswalkerDies(yours))
        );
        game.phase_in(lurker);
        assert!(
            !check_state_based_actions(&game).contains(&StateBasedAction::PlaneswalkerDies(yours))
        );
        let printed_abilities = game.object(lurker).unwrap().abilities.clone();
        game.object_mut(lurker).unwrap().abilities = Vec::new().into();
        assert!(
            check_state_based_actions(&game).contains(&StateBasedAction::PlaneswalkerDies(yours))
        );
        game.object_mut(lurker).unwrap().abilities = printed_abilities;
        assert!(
            !check_state_based_actions(&game).contains(&StateBasedAction::PlaneswalkerDies(yours))
        );
        game.move_object_by_effect(lurker, Zone::Graveyard).unwrap();
        assert!(
            check_state_based_actions(&game).contains(&StateBasedAction::PlaneswalkerDies(yours))
        );
    }
}

#[test]
fn sanctum_lurker_granted_positive_loyalty_ability_can_activate_at_zero() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Sanctum Lurker") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut dm = Choices::default();
        let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
        execute_effect(&mut game, &Effect::new(EmpowerJaceEffect::new(0)), &mut ctx).unwrap();
        let jace = jaces(&game, alice)[0];
        ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
        assert_eq!(loyalty(&game, jace), 0);
        // The granted +2 is the only payable loyalty activation at zero.
        let action = compute_legal_actions(&game, alice).unwrap().into_iter()
            .find(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == jace))
            .expect("the full-card granted +2 loyalty ability must be available");
        announce(&mut game, action, &mut dm);
        assert_eq!(loyalty(&game, jace), 2);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(alice).unwrap().life, 21);
        assert_eq!(game.player(bob).unwrap().life, 19);
        assert!(!compute_legal_actions(&game, alice).unwrap().iter()
            .any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == jace)));
    }
}

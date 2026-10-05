//! Source-authored only; all scenarios remain unrun.
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{DrawCardsEffect, EffectContext, EffectExecutor};
use ironsmith::events::CardsDrawnEvent;
use ironsmith::game_loop::{put_triggers_on_stack, resolve_stack_entry};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
fn a() -> PlayerId {
    PlayerId::from_index(0)
}
fn b() -> PlayerId {
    PlayerId::from_index(1)
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/timed_draw_replacements.json.fixture"
    ))
    .unwrap();
    let card = cards.iter().find(|card| card["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        card["mana_cost"].as_str().unwrap(),
        card["type_line"].as_str().unwrap()
    );
    if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(card["oracle_text"].as_str().unwrap());
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn card(
    game: &mut GameState,
    player: PlayerId,
    name: &str,
    kind: CardType,
    zone: Zone,
) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .mana_cost(ironsmith::mana::ManaCost::new())
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&definition, player, zone)
}
fn fill_library(game: &mut GameState, player: PlayerId, count: usize) {
    for index in 0..count {
        card(
            game,
            player,
            &format!("Library card {index}"),
            CardType::Artifact,
            Zone::Library,
        );
    }
}
fn draw(game: &mut GameState, source: ObjectId, player: PlayerId, count: i32) -> usize {
    let mut dm = SelectFirstDecisionMaker;
    let outcome = DrawCardsEffect::you(count)
        .execute(game, &mut EffectContext::new(source, player, &mut dm))
        .unwrap();
    outcome
        .events
        .iter()
        .filter_map(|event| event.downcast::<CardsDrawnEvent>())
        .map(|event| event.amount() as usize)
        .sum()
}
struct ChooseTarget(Target);
impl DecisionMaker for ChooseTarget {
    fn decide_targets(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        assert_eq!(ctx.requirements.len(), 1);
        assert!(ctx.requirements[0].legal_targets.contains(&self.0));
        vec![self.0]
    }
}
fn announce(game: &mut GameState, source: ObjectId, target: Option<Target>) {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    game.turn.priority_player = Some(a());
    game.player_mut(a())
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Black, 1);
    let action = compute_legal_actions(game, a()).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: found, .. } if *found == source)).unwrap();
    let mut chosen = ChooseTarget(target.unwrap_or(Target::Player(a())));
    let dm = &mut chosen;
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..16 {
        if state.pending_activation.is_none() {
            break;
        }
        if let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress {
            progress =
                apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
        } else {
            break;
        }
    }
    assert!(!game.stack_is_empty());
    if let Some(target) = target {
        assert!(game.stack.last().unwrap().targets.contains(&target));
    }
}
fn activate(game: &mut GameState, host: ObjectId, target: Option<Target>) {
    announce(game, host, target);
    resolve_stack_entry(game).unwrap();
}
fn cast_plagiarize(game: &mut GameState, definition: &CardDefinition, player: PlayerId) {
    use ironsmith::game_state::{StackEntry, TargetAssignment};
    let source = game.create_object_from_definition(definition, a(), Zone::Stack);
    let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        game,
        definition.spell_effect.as_ref().unwrap(),
        a(),
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 1);
    assert!(
        requirements[0]
            .legal_targets
            .contains(&Target::Player(player))
    );
    game.push_to_stack(
        StackEntry::new(source, a())
            .with_targets(vec![Target::Player(player)])
            .with_target_assignments(vec![TargetAssignment {
                spec: requirements[0].spec.clone(),
                range: 0..1,
            }]),
    );
    resolve_stack_entry(game).unwrap();
}
#[test]
fn seven_exact_full_bodies_materialize_without_losing_the_instead_program() {
    for name in [
        "Words of War",
        "Words of Waste",
        "Words of Wilding",
        "Words of Wind",
        "Words of Worship",
        "Plagiarize",
        "Urabrask, Heretic Praetor",
    ] {
        for definition in definitions(name) {
            assert_eq!(definition.card.name, name);
        }
    }
}
#[test]
fn words_worship_replaces_only_one_draw_and_keeps_its_resolving_controller() {
    for definition in definitions("Words of Worship") {
        let mut game = game();
        fill_library(&mut game, a(), 4);
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        activate(&mut game, host, None);
        assert_eq!(game.player(a()).unwrap().life, 20);
        game.set_current_controller(host, b()).unwrap();
        draw(&mut game, host, a(), 3);
        assert_eq!(game.player(a()).unwrap().life, 25);
        assert_eq!(game.player(b()).unwrap().life, 20);
        assert_eq!(game.player(a()).unwrap().hand.len(), 2);
        assert!(game.effect_store.replacement_effects.effects().is_empty());
    }
}
#[test]
fn multiple_words_registrations_consume_independently_and_unused_ones_expire() {
    for definition in definitions("Words of Worship") {
        let mut game = game();
        fill_library(&mut game, a(), 4);
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        activate(&mut game, host, None);
        activate(&mut game, host, None);
        draw(&mut game, host, a(), 2);
        assert_eq!(game.player(a()).unwrap().life, 30);
        assert!(game.player(a()).unwrap().hand.is_empty());
        activate(&mut game, host, None);
        game.effect_store
            .replacement_effects
            .clear_one_shot_effects();
        draw(&mut game, host, a(), 1);
        assert_eq!(game.player(a()).unwrap().life, 30);
        assert_eq!(game.player(a()).unwrap().hand.len(), 1);
    }
}
#[test]
fn words_war_announces_its_target_then_keeps_the_draw_replaced_after_target_departure() {
    for definition in definitions("Words of War") {
        for before_resolution in [false, true] {
            let mut game = game();
            fill_library(&mut game, a(), 2);
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let target = card(
                &mut game,
                b(),
                "Announced victim",
                CardType::Creature,
                Zone::Battlefield,
            );
            announce(&mut game, host, Some(Target::Object(target)));
            if !before_resolution {
                resolve_stack_entry(&mut game).unwrap();
            }
            game.move_object_by_effect(target, Zone::Graveyard).unwrap();
            if before_resolution {
                resolve_stack_entry(&mut game).unwrap();
            }
            draw(&mut game, host, a(), 1);
            assert_eq!(
                game.player(a()).unwrap().hand.len(),
                usize::from(before_resolution),
                "a resolved registration is not retroactively countered"
            );
            assert!(game.effect_store.replacement_effects.effects().is_empty());
        }
    }
}
#[test]
fn words_war_uses_actual_source_departure_or_phase_out_lki() {
    for definition in definitions("Words of War") {
        for phase in [false, true] {
            let mut game = game();
            fill_library(&mut game, a(), 2);
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            activate(&mut game, host, Some(Target::Player(b())));
            game.object_mut(host).unwrap().abilities_mut().push(
                ironsmith::ability::Ability::static_ability(
                    ironsmith::static_abilities::StaticAbility::lifelink(),
                ),
            );
            if phase {
                game.phase_out(host);
            } else {
                game.move_object_by_effect(host, Zone::Graveyard).unwrap();
            }
            draw(&mut game, host, a(), 1);
            assert_eq!(game.player(a()).unwrap().life, 22);
            assert_eq!(game.player(b()).unwrap().life, 18);
            assert!(game.player(a()).unwrap().hand.is_empty());
        }
    }
}
#[test]
fn words_waste_and_wind_apply_real_opponent_discard_and_per_player_return_choices() {
    for definition in definitions("Words of Waste") {
        let mut game = game();
        fill_library(&mut game, a(), 2);
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        card(
            &mut game,
            b(),
            "Discarded card",
            CardType::Artifact,
            Zone::Hand,
        );
        activate(&mut game, host, None);
        draw(&mut game, host, a(), 1);
        assert!(game.player(a()).unwrap().hand.is_empty());
        assert!(game.player(b()).unwrap().hand.is_empty());
        assert_eq!(game.player(b()).unwrap().graveyard.len(), 1);
    }
    for definition in definitions("Words of Wind") {
        let mut game = game();
        fill_library(&mut game, a(), 2);
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let other = card(
            &mut game,
            b(),
            "Returned permanent",
            CardType::Artifact,
            Zone::Battlefield,
        );
        activate(&mut game, host, None);
        draw(&mut game, host, a(), 1);
        assert!(game.object(host).is_none());
        assert!(game.object(other).is_none());
        assert_eq!(game.player(a()).unwrap().hand.len(), 1);
        assert_eq!(game.player(b()).unwrap().hand.len(), 1);
        assert_eq!(game.player(a()).unwrap().library.len(), 2);
    }
}
#[test]
fn words_wilding_creates_the_actual_bear_once_without_drawing() {
    for definition in definitions("Words of Wilding") {
        let mut game = game();
        fill_library(&mut game, a(), 2);
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        activate(&mut game, host, None);
        draw(&mut game, host, a(), 1);
        let tokens: Vec<_> = game
            .objects_in_deterministic_order()
            .into_iter()
            .filter(|object| {
                matches!(object.kind, ironsmith::object::ObjectKind::Token)
                    && object.zone == Zone::Battlefield
            })
            .collect();
        assert_eq!(tokens.len(), 1);
        assert!(game.current_has_subtype(tokens[0].id, ironsmith::Subtype::Bear));
        assert_eq!(game.calculated_power(tokens[0].id), Some(2));
        assert_eq!(game.calculated_toughness(tokens[0].id), Some(2));
        assert!(game.player(a()).unwrap().hand.is_empty());
    }
}
#[test]
fn plagiarize_retains_selected_player_for_all_draws_and_self_targeting_does_not_loop() {
    for definition in definitions("Plagiarize") {
        for player in [a(), b()] {
            let mut game = game();
            fill_library(&mut game, a(), 4);
            fill_library(&mut game, b(), 4);
            cast_plagiarize(&mut game, &definition, player);
            let probe = card(
                &mut game,
                a(),
                "Draw source",
                CardType::Artifact,
                Zone::Battlefield,
            );
            draw(&mut game, probe, player, 2);
            assert_eq!(game.player(a()).unwrap().hand.len(), 2);
            assert!(game.player(b()).unwrap().hand.is_empty());
            game.effect_store
                .replacement_effects
                .clear_until_end_of_turn_effects();
            draw(&mut game, probe, b(), 1);
            assert_eq!(game.player(b()).unwrap().hand.len(), 1);
        }
    }
}
#[test]
fn urabrask_registers_on_opponents_upkeep_and_exiles_only_their_first_draw_with_play_permission() {
    for definition in definitions("Urabrask, Heretic Praetor") {
        let mut game = game();
        fill_library(&mut game, a(), 3);
        fill_library(&mut game, b(), 3);
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        game.turn.active_player = b();
        game.turn.phase = ironsmith::game_state::Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::generate_and_queue_step_triggers(&mut game, &mut queue);
        put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry(&mut game).unwrap();
        assert!(
            game.exile.is_empty(),
            "registration itself must not exile a card or grant a premature permission"
        );
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        draw(&mut game, host, b(), 2);
        assert_eq!(game.exile.len(), 1);
        assert_eq!(game.player(b()).unwrap().hand.len(), 1);
        assert!(game.player(a()).unwrap().hand.is_empty());
        let exiled = game.exile[0];
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(b());
        assert!(
            ironsmith::decision::compute_actions_for_source(&game, b(), Some(exiled))
                .unwrap()
                .iter()
                .any(|action| matches!(action, ironsmith::decision::LegalAction::CastSpell { .. }))
        );
    }
}
#[test]
fn urabrasks_own_upkeep_executes_its_other_body_without_registering_a_draw_replacement() {
    for definition in definitions("Urabrask, Heretic Praetor") {
        let mut game = game();
        fill_library(&mut game, a(), 3);
        game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        game.turn.phase = ironsmith::game_state::Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::generate_and_queue_step_triggers(&mut game, &mut queue);
        put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.exile.len(), 1);
        assert!(game.effect_store.replacement_effects.effects().is_empty());
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(a());
        assert!(
            ironsmith::decision::compute_actions_for_source(&game, a(), Some(game.exile[0]))
                .unwrap()
                .iter()
                .any(|action| matches!(action, ironsmith::decision::LegalAction::CastSpell { .. }))
        );
    }
}
struct PauseReplacement {
    pending: bool,
}
impl DecisionMaker for PauseReplacement {
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
    fn decide_options(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.pending = true;
        vec![
            context
                .options
                .iter()
                .find(|option| option.legal)
                .unwrap()
                .index,
        ]
    }
}
#[test]
fn pending_ordering_choice_does_not_consume_either_registered_program() {
    for (worship, wilding) in definitions("Words of Worship")
        .into_iter()
        .zip(definitions("Words of Wilding"))
    {
        let mut game = game();
        fill_library(&mut game, a(), 2);
        let first = game.create_object_from_definition(&worship, a(), Zone::Battlefield);
        let second = game.create_object_from_definition(&wilding, a(), Zone::Battlefield);
        activate(&mut game, first, None);
        activate(&mut game, second, None);
        let mut pause = PauseReplacement { pending: false };
        DrawCardsEffect::you(1)
            .execute(&mut game, &mut EffectContext::new(first, a(), &mut pause))
            .unwrap();
        assert!(pause.pending);
        assert_eq!(game.effect_store.replacement_effects.effects().len(), 2);
        assert_eq!(game.player(a()).unwrap().life, 20);
        assert!(game.player(a()).unwrap().hand.is_empty());
        draw(&mut game, first, a(), 1);
        assert_eq!(game.effect_store.replacement_effects.effects().len(), 1);
        assert!(game.player(a()).unwrap().hand.is_empty());
    }
}

#[test]
fn words_war_uses_leave_game_lki_when_its_owner_leaves_after_another_player_registered_it() {
    for definition in definitions("Words of War") {
        let c = PlayerId::from_index(2);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        fill_library(&mut game, a(), 2);
        let host = game.create_object_from_definition(&definition, b(), Zone::Battlefield);
        game.set_current_controller(host, a()).unwrap();
        activate(&mut game, host, Some(Target::Player(c)));
        // This ability was absent from the registration-time snapshot.
        game.object_mut(host).unwrap().abilities_mut().push(
            ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::lifelink(),
            ),
        );
        assert!(
            game.leave_game(b())
                .expect("checked designation/departure fixture")
        );
        assert!(game.object(host).is_none());
        assert_eq!(game.effect_store.replacement_effects.effects().len(), 1);
        draw(&mut game, host, a(), 1);
        assert_eq!(game.player(a()).unwrap().life, 22);
        assert_eq!(game.player(c).unwrap().life, 18);
        assert!(game.player(a()).unwrap().hand.is_empty());
    }
}

struct DeclineOrPause {
    pause: bool,
    waiting: bool,
}
impl DecisionMaker for DeclineOrPause {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.waiting = self.pause;
        false
    }
    fn awaiting_choice(&self) -> bool {
        self.waiting
    }
}
#[test]
fn captured_program_preserves_prefix_trigger_receipts_and_rolls_them_back_on_pending_choice() {
    use ironsmith::effects::{
        DestroyEffect, GainLifeEffect, MayEffect, RegisterDrawReplacementEffect,
        ReplacementApplyMode,
    };
    use ironsmith::target::{ChooseSpec, PlayerFilter};
    for pause_first in [false, true] {
        let mut game = game();
        fill_library(&mut game, a(), 2);
        let source = card(
            &mut game,
            a(),
            "Future program source",
            CardType::Artifact,
            Zone::Battlefield,
        );
        let observer = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Life observer",
            "Type: Enchantment\nWhenever you gain life, each opponent loses 1 life.",
            false,
        )
        .unwrap();
        let observer = game.create_object_from_definition(&observer, a(), Zone::Battlefield);
        let program = RegisterDrawReplacementEffect::new(
            PlayerFilter::You,
            vec![
                ironsmith::Effect::new(GainLifeEffect::you(1)),
                ironsmith::Effect::new(MayEffect::new(vec![ironsmith::Effect::new(
                    GainLifeEffect::you(1),
                )])),
                ironsmith::Effect::new(DestroyEffect::with_spec(ChooseSpec::SpecificObject(
                    observer,
                ))),
                ironsmith::Effect::new(GainLifeEffect::you(1)),
            ],
            ReplacementApplyMode::OneShot,
        );
        let mut first = SelectFirstDecisionMaker;
        program
            .execute(&mut game, &mut EffectContext::new(source, a(), &mut first))
            .unwrap();
        if pause_first {
            let mut pause = DeclineOrPause {
                pause: true,
                waiting: false,
            };
            let outcome = DrawCardsEffect::you(1)
                .execute(&mut game, &mut EffectContext::new(source, a(), &mut pause))
                .unwrap();
            assert!(pause.waiting);
            assert!(outcome.events.is_empty());
            assert_eq!(game.player(a()).unwrap().life, 20);
            assert!(game.object(observer).is_some());
            assert_eq!(game.effect_store.replacement_effects.effects().len(), 1);
            put_triggers_on_stack(&mut game, &mut TriggerQueue::new()).unwrap();
            assert!(
                game.stack_is_empty(),
                "pending replay must not publish prefix receipts"
            );
        }
        let mut decline = DeclineOrPause {
            pause: false,
            waiting: false,
        };
        let outcome = DrawCardsEffect::you(1)
            .execute(
                &mut game,
                &mut EffectContext::new(source, a(), &mut decline),
            )
            .unwrap();
        assert!(game.object(observer).is_none());
        assert_eq!(game.player(a()).unwrap().life, 22);
        assert!(game.player(a()).unwrap().hand.is_empty());
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
        put_triggers_on_stack(&mut game, &mut TriggerQueue::new()).unwrap();
        assert_eq!(
            game.stack.len(),
            1,
            "only the gain before observer destruction triggers, exactly once"
        );
        resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(b()).unwrap().life, 19);
        put_triggers_on_stack(&mut game, &mut TriggerQueue::new()).unwrap();
        assert!(game.stack_is_empty());
    }
}

#[derive(Debug, Clone)]
struct CheckFutureDrawEvent;
impl EffectExecutor for CheckFutureDrawEvent {
    fn execute(
        &self,
        _: &mut GameState,
        ctx: &mut EffectContext,
    ) -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
        let draw = ctx
            .triggering_event
            .as_ref()
            .unwrap()
            .downcast::<ironsmith::events::cards::DrawEvent>()
            .unwrap();
        assert_eq!(draw.player, b());
        assert_eq!(draw.count, 1);
        assert_eq!(ctx.iteration.iterated_player, Some(b()));
        Ok(ironsmith::effect::EffectOutcome::count(0))
    }
}
#[test]
fn captured_program_keeps_the_future_event_instead_of_the_registration_event() {
    use ironsmith::effects::{RegisterDrawReplacementEffect, ReplacementApplyMode};
    use ironsmith::target::PlayerFilter;
    let mut game = game();
    fill_library(&mut game, b(), 2);
    let source = card(
        &mut game,
        a(),
        "Future context source",
        CardType::Artifact,
        Zone::Battlefield,
    );
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, a(), &mut dm);
    ctx.triggering_event = Some(ironsmith::triggers::TriggerEvent::new(
        ironsmith::events::cards::DrawEvent::new(a(), 7, false),
        ctx.provenance,
    ));
    RegisterDrawReplacementEffect::new(
        PlayerFilter::Specific(b()),
        vec![ironsmith::Effect::new(CheckFutureDrawEvent)],
        ReplacementApplyMode::OneShot,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    draw(&mut game, source, b(), 1);
    assert!(game.player(b()).unwrap().hand.is_empty());
}

#[test]
fn words_waste_keeps_its_team_scope_and_teammate_choice_handles_empty_sets() {
    use ironsmith::effects::ChoosePlayerEffect;
    use ironsmith::target::PlayerFilter;
    let teammate = PlayerId::from_index(1);
    let bob = PlayerId::from_index(2);
    let charlie = PlayerId::from_index(3);
    for definition in definitions("Words of Waste") {
        for teams in [false, true] {
            let mut game = GameState::new(
                vec![
                    "Alice".into(),
                    "Teammate".into(),
                    "Bob".into(),
                    "Charlie".into(),
                ],
                20,
            );
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.turn.active_player = a();
            if teams {
                game.set_teams(vec![vec![a(), teammate], vec![bob, charlie]])
                    .unwrap();
            }
            fill_library(&mut game, a(), 2);
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            for player in [a(), teammate, bob, charlie] {
                card(
                    &mut game,
                    player,
                    "First discard witness",
                    CardType::Artifact,
                    Zone::Hand,
                );
                card(
                    &mut game,
                    player,
                    "Second discard witness",
                    CardType::Artifact,
                    Zone::Hand,
                );
            }
            activate(&mut game, host, None);
            game.set_current_controller(host, bob).unwrap();
            assert_eq!(draw(&mut game, host, a(), 1), 0);
            assert_eq!(game.player(a()).unwrap().hand.len(), 2);
            assert_eq!(
                game.player(teammate).unwrap().hand.len(),
                if teams { 2 } else { 1 }
            );
            for player in [bob, charlie] {
                assert_eq!(game.player(player).unwrap().hand.len(), 1);
                assert_eq!(game.player(player).unwrap().graveyard.len(), 1);
            }
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = EffectContext::new(host, a(), &mut dm);
            ChoosePlayerEffect::new(PlayerFilter::You, PlayerFilter::Teammate, "team-choice")
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(
                ctx.get_tagged_players("team-choice").unwrap(),
                &if teams { vec![teammate] } else { vec![] }
            );
            assert_eq!(
                game.player(teammate).unwrap().hand.len(),
                if teams { 2 } else { 1 }
            );
            assert_eq!(
                game.player(teammate).unwrap().graveyard.len(),
                usize::from(!teams)
            );
            assert_eq!(game.player(a()).unwrap().hand.len(), 2);
            for player in [bob, charlie] {
                assert_eq!(game.player(player).unwrap().hand.len(), 1);
            }
        }
    }
}

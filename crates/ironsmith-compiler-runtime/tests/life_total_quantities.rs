//! UNVALIDATED exact life quantities, current/starting-life conditions and target references.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext, ViewCardsContext,
};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/life_total_quantities.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|r| r["name"] == name).unwrap();
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {p}/{t}"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().into());
    definitions_text(name, &lines.join("\n"))
}
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
}
fn game_with_starting_life(starting: i32) -> GameState {
    let mut game = GameState::new(
        vec!["Alice".into(), "Bob".into(), "Charlie".into()],
        starting,
    );
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(color, 20);
    }
    game
}
fn vanilla(name: &str, cost: &str, subtype: &str, p: i32, t: i32) -> CardDefinition {
    compile_to_runtime_definition(
        name,
        format!("Mana cost: {cost}\nType: Creature — {subtype}\nPower/Toughness: {p}/{t}"),
        false,
    )
    .unwrap()
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    mode: Option<usize>,
    x: u32,
    prefer_life: bool,
    viewed: Vec<Vec<ObjectId>>,
    untap: bool,
    votes: Option<[&'static str; 3]>,
    optional: Option<bool>,
    kicker_payments: usize,
    decline_targets: bool,
    bounds: Vec<(usize, Option<usize>)>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        if let Some(choice) = self.optional {
            return choice;
        }
        if ctx.description.starts_with("untap ") {
            self.untap
        } else {
            true
        }
    }
    fn view_cards(
        &mut self,
        _game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        _context: &ViewCardsContext,
    ) {
        if viewer == A {
            self.viewed.push(cards.to_vec());
        }
    }

    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.starts_with("Choose optional costs for") {
            return context
                .options
                .iter()
                .filter(|option| option.legal)
                .take(self.kicker_payments)
                .map(|option| option.index)
                .collect();
        }
        if let Some(votes) = self.votes
            && context.options.iter().all(|option| {
                matches!(
                    option.description.to_ascii_lowercase().as_str(),
                    "profit" | "security"
                )
            })
        {
            let index = if context.player == A {
                0
            } else if context.player == B {
                1
            } else {
                2
            };
            return vec![
                context
                    .options
                    .iter()
                    .find(|option| {
                        option.legal && option.description.eq_ignore_ascii_case(votes[index])
                    })
                    .unwrap()
                    .index,
            ];
        }

        if let Some(mode) = self.mode
            && context.description.starts_with("Choose mode for")
        {
            assert!(
                context
                    .options
                    .iter()
                    .any(|option| option.index == mode && option.legal)
            );
            return vec![mode];
        }
        if self.prefer_life && context.description.starts_with("Choose how to pay pip") {
            if let Some(option) = context.options.iter().find(|option| {
                option.legal && option.description.to_ascii_lowercase().contains("life")
            }) {
                return vec![option.index];
            }
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_number(&mut self, game: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value {
            assert!(self.x <= context.max);
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, context)
        }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.bounds.extend(
            context
                .requirements
                .iter()
                .map(|r| (r.min_targets, r.max_targets)),
        );
        if self.decline_targets {
            return Vec::new();
        }
        if !self.targets.is_empty() {
            assert!(self.targets.iter().all(|target| {
                context
                    .requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target))
            }));
            self.targets.clone()
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if !self.objects.is_empty() {
            let selected = self
                .objects
                .iter()
                .copied()
                .filter(|id| {
                    context
                        .candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                })
                .take(context.max.unwrap_or(usize::MAX))
                .collect::<Vec<_>>();
            assert!(
                selected.len() >= context.min,
                "requested cost objects must satisfy the current payment decision"
            );
            selected
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
}
fn queue_outcome(game: &mut GameState, outcome: EffectOutcome, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    // Checked execution already captures some triggers in the original observer frame.
    ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
    for event in outcome.events {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) -> EffectOutcome {
    let mut dm = SelectFirstDecisionMaker;
    let controller = game.current_controller(source).unwrap_or(A);
    execute_effect(
        game,
        &effect,
        &mut EffectContext::new(source, controller, &mut dm),
    )
    .unwrap()
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn resolve_all(game: &mut GameState, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
        if game.stack_is_empty() {
            return;
        }
        resolve(game, dm);
    }
    panic!("unexpected continuing trigger chain");
}
fn cast(
    game: &mut GameState,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    let spell = game
        .stack
        .iter()
        .rev()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}
fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Choices) {
    let action = LegalAction::ActivateAbility {
        source,
        ability_index,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..50 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert_eq!(game.stack.len(), 1);
}
use ironsmith::static_abilities::StaticAbilityId;
fn game() -> GameState {
    game_with_starting_life(20)
}
fn witness(game: &mut GameState) -> ObjectId {
    game.create_object_from_definition(
        &vanilla("Life witness", "{1}", "Human", 1, 1),
        A,
        Zone::Battlefield,
    )
}
fn set_life(game: &mut GameState, source: ObjectId, player: PlayerId, life: i32) {
    apply(
        game,
        source,
        Effect::set_life_total_player(life, PlayerFilter::Specific(player)),
    );
}
fn library(game: &mut GameState, count: usize) {
    for _ in 0..count {
        game.create_object_from_definition(
            &vanilla("Library witness", "{1}", "Human", 1, 1),
            A,
            Zone::Library,
        );
    }
}
fn enter(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let spell = cast(game, definition, CastingMethod::Normal, dm);
    let stable = game.object(spell).unwrap().stable_id;
    resolve_all(game, dm);
    game.find_object_by_stable_id(stable).unwrap()
}
fn pt(game: &GameState, id: ObjectId) -> (i32, i32) {
    (
        game.calculated_power(id).unwrap(),
        game.calculated_toughness(id).unwrap(),
    )
}
fn has(game: &GameState, id: ObjectId, ability: StaticAbilityId) -> bool {
    game.current_has_static_ability_id(id, ability)
}
#[test]
fn ten_full_frozen_sources_round_trip_with_exact_noted_life_lki() {
    let rows = fixtures();
    assert_eq!(rows.len(), 10);
    let complete = rows
        .iter()
        .filter(|r| r["coverage_status"] == "proposed_complete")
        .collect::<Vec<_>>();
    assert_eq!(complete.len(), 10);
    for row in complete {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
        }
    }
}
#[test]
fn anya_counts_opponents_below_their_exact_half_start_and_rechecks_controller_and_indestructibility()
 {
    for definition in definitions("Anya, Merciless Angel") {
        let mut game = game_with_starting_life(41);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(pt(&game, source), (4, 4));
        assert!(!has(&game, source, StaticAbilityId::Indestructible));
        set_life(&mut game, source, B, 20);
        assert_eq!(pt(&game, source), (7, 7));
        assert!(has(&game, source, StaticAbilityId::Indestructible));
        apply(
            &mut game,
            source,
            Effect::destroy(ChooseSpec::SpecificObject(source)),
        );
        assert!(game.object(source).is_some());
        set_life(&mut game, source, C, 20);
        assert_eq!(pt(&game, source), (10, 10));
        set_life(&mut game, source, B, 21);
        assert_eq!(pt(&game, source), (7, 7));
        game.set_current_controller(source, B).unwrap();
        assert_eq!(pt(&game, source), (7, 7));
        set_life(&mut game, source, C, 21);
        assert_eq!(pt(&game, source), (4, 4));
        assert!(!has(&game, source, StaticAbilityId::Indestructible));
        set_life(&mut game, source, A, 20);
        assert_eq!(
            pt(&game, source),
            (7, 7),
            "you is the current source controller, so former controller Alice now counts"
        );
        set_life(&mut game, source, A, 21);
        apply(
            &mut game,
            source,
            Effect::destroy(ChooseSpec::SpecificObject(source)),
        );
        assert!(game.object(source).is_none());
    }
}
#[test]
fn malignus_uses_opponents_maximum_ceiling_current_controller_and_real_unpreventable_damage() {
    for definition in definitions("Malignus") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        set_life(&mut game, source, A, 100);
        set_life(&mut game, source, B, 41);
        set_life(&mut game, source, C, 23);
        assert_eq!(
            pt(&game, source),
            (21, 21),
            "the caster's 100 life is outside the opponent scope"
        );
        apply(
            &mut game,
            source,
            Effect::prevent_damage(100, ChooseSpec::SpecificPlayer(B), Until::EndOfTurn),
        );
        apply(
            &mut game,
            source,
            Effect::new(ironsmith::effects::DealDamageEffect::new(
                ironsmith::effect::Value::PowerOf(Box::new(ChooseSpec::Source)),
                ChooseSpec::SpecificPlayer(B),
            )),
        );
        assert_eq!(game.player(B).unwrap().life, 20);
        assert_eq!(pt(&game, source), (12, 12));
        set_life(&mut game, source, C, 24);
        assert_eq!(pt(&game, source), (12, 12));
        game.set_current_controller(source, B).unwrap();
        assert_eq!(pt(&game, source), (50, 50));
    }
}
#[test]
fn arbiter_captures_one_maximum_before_simultaneous_sets_and_gain_replacements() {
    for definition in definitions("Arbiter of Knollridge") {
        let mut game = game();
        let setup = witness(&mut game);
        set_life(&mut game, setup, B, 30);
        set_life(&mut game, setup, C, 50);
        let doubler = compile_to_runtime_definition(
            "Life gain doubler",
            "Type: Enchantment\nIf you would gain life, you gain twice that much life instead.",
            false,
        )
        .unwrap();
        game.create_object_from_definition(&doubler, A, Zone::Battlefield);
        enter(&mut game, &definition, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().life, 80);
        assert_eq!(game.player(B).unwrap().life, 50);
        assert_eq!(
            game.player(C).unwrap().life,
            50,
            "Alice's replacement-modified 80 must not become a new maximum for later participants"
        );
    }
}
#[test]
fn elenda_dynamic_starting_thresholds_stack_once_and_follow_its_current_controller() {
    for definition in definitions("Elenda, Saint of Dusk") {
        let mut game = game();
        game.player_mut(B).unwrap().starting_life = 41;
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert_eq!(pt(&game, source), (4, 4));
        assert!(!has(&game, source, StaticAbilityId::Menace));
        set_life(&mut game, source, A, 21);
        assert_eq!(pt(&game, source), (5, 5));
        assert!(has(&game, source, StaticAbilityId::Menace));
        set_life(&mut game, source, A, 29);
        assert_eq!(pt(&game, source), (5, 5));
        set_life(&mut game, source, A, 30);
        assert_eq!(pt(&game, source), (10, 10));
        set_life(&mut game, source, B, 41);
        game.set_current_controller(source, B).unwrap();
        assert_eq!(pt(&game, source), (4, 4));
        assert!(!has(&game, source, StaticAbilityId::Menace));
        set_life(&mut game, source, B, 42);
        assert_eq!(pt(&game, source), (5, 5));
        set_life(&mut game, source, B, 51);
        assert_eq!(pt(&game, source), (10, 10));
    }
}
#[test]
fn game_over_uses_any_players_unrounded_half_threshold_for_actual_generic_cost_and_destruction() {
    for definition in definitions("Game Over") {
        let mut game = game_with_starting_life(41);
        let own = witness(&mut game);
        let other = game.create_object_from_definition(
            &vanilla("Opponent witness", "{1}", "Human", 2, 2),
            B,
            Zone::Battlefield,
        );
        set_life(&mut game, own, B, 21);
        game.player_mut(A).unwrap().mana_pool = Default::default();
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 2);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        let probe = game.create_object_from_definition(&definition, A, Zone::Hand);
        let action = LegalAction::CastSpell {
            spell_id: probe,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        };
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&action));
        set_life(&mut game, own, B, 20);
        assert!(compute_legal_actions(&game, A).unwrap().contains(&action));
        game.move_object_by_game_rule(probe, Zone::Exile).unwrap();
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices::default(),
        );
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve_all(&mut game, &mut Choices::default());
        assert!(game.object(own).is_none());
        assert!(game.object(other).is_none());
    }
}
#[test]
fn psychic_transfer_announces_one_player_even_when_its_condition_is_false_then_reevaluates_it() {
    for definition in definitions("Psychic Transfer") {
        for mode in 0..3 {
            let mut game = game();
            let source = witness(&mut game);
            let target = if mode == 2 { A } else { B };
            set_life(&mut game, source, B, if mode == 0 { 50 } else { 23 });
            let mut dm = Choices {
                targets: vec![Target::Player(target)],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            assert_eq!(dm.bounds, vec![(1, Some(1))]);
            if mode == 0 {
                set_life(&mut game, source, B, 23);
            } else if mode == 1 {
                set_life(&mut game, source, B, 50);
            }
            resolve_all(&mut game, &mut Choices::default());
            match mode {
                0 => {
                    assert_eq!(game.player(A).unwrap().life, 23);
                    assert_eq!(game.player(B).unwrap().life, 20);
                }
                1 => {
                    assert_eq!(game.player(A).unwrap().life, 20);
                    assert_eq!(game.player(B).unwrap().life, 50);
                }
                _ => {
                    assert_eq!(game.player(A).unwrap().life, 20);
                    assert_eq!(game.player(B).unwrap().life, 23);
                }
            }
        }
    }
}
#[test]
fn resolute_archangel_keeps_the_intervening_if_and_binds_its_numeric_life_pronoun() {
    for definition in definitions("Resolute Archangel") {
        for respond in [false, true] {
            let mut game = game();
            let setup = witness(&mut game);
            set_life(&mut game, setup, A, 7);
            let spell = cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices::default(),
            );
            let stable = game.object(spell).unwrap().stable_id;
            resolve(&mut game, &mut Choices::default());
            let mut queue = TriggerQueue::new();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut Choices::default()).unwrap();
            assert_eq!(game.stack.len(), 1);
            let source = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(pt(&game, source), (4, 4));
            if respond {
                set_life(&mut game, source, A, 25);
            }
            game.set_current_controller(source, B).unwrap();
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().life, if respond { 25 } else { 20 });
            assert_eq!(game.player(B).unwrap().life, 20);
            assert_eq!(pt(&game, source), (4, 4));
        }
    }
}
#[test]
fn scourge_real_kicker_payment_halves_each_life_and_its_continuous_size_tracks_the_global_maximum()
{
    for definition in definitions("Scourge of the Skyclaves") {
        for kicked in [false, true] {
            let mut game = game_with_starting_life(if kicked { 31 } else { 12 });
            let initial_mana = game.player(A).unwrap().mana_pool.total();
            let source = enter(
                &mut game,
                &definition,
                &mut Choices {
                    kicker_payments: usize::from(kicked),
                    ..Default::default()
                },
            );
            assert_eq!(
                initial_mana - game.player(A).unwrap().mana_pool.total(),
                if kicked { 7 } else { 2 }
            );
            let life = if kicked { 15 } else { 12 };
            for player in [A, B, C] {
                assert_eq!(game.player(player).unwrap().life, life);
            }
            assert_eq!(pt(&game, source), (20 - life, 20 - life));
            set_life(&mut game, source, C, 18);
            assert_eq!(pt(&game, source), (2, 2));
            set_life(&mut game, source, A, 19);
            assert_eq!(
                pt(&game, source),
                (1, 1),
                "all players includes its controller"
            );
        }
    }
}

#[test]
fn cosmos_elixir_checks_current_life_on_resolution_and_keeps_the_trigger_controller() {
    for definition in definitions("Cosmos Elixir") {
        for draw_after_response in [false, true] {
            let mut game = game();
            library(&mut game, 3);
            let source = enter(&mut game, &definition, &mut Choices::default());
            set_life(
                &mut game,
                source,
                A,
                if draw_after_response { 20 } else { 21 },
            );
            game.turn.phase = Phase::Ending;
            game.turn.step = Some(ironsmith::game_state::Step::End);
            let wrong_turn = ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfEndStepEvent::new(B),
                Default::default(),
            );
            assert!(check_triggers(&game, &wrong_turn).is_empty());
            let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfEndStepEvent::new(A),
                Default::default(),
            );
            let mut queue = TriggerQueue::new();
            let entries = check_triggers(&game, &event);
            assert_eq!(
                entries.len(),
                1,
                "the life test belongs to the effect, not an intervening if"
            );
            for entry in entries {
                queue.add(entry);
            }
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut Choices::default()).unwrap();
            let hand = game.player(A).unwrap().hand.len();
            set_life(
                &mut game,
                source,
                A,
                if draw_after_response { 21 } else { 20 },
            );
            game.set_current_controller(source, B).unwrap();
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(
                game.player(A).unwrap().hand.len(),
                hand + usize::from(draw_after_response)
            );
            assert_eq!(
                game.player(A).unwrap().life,
                if draw_after_response { 21 } else { 22 }
            );
            assert_eq!(game.player(B).unwrap().life, 20);
            assert!(game.player(B).unwrap().hand.is_empty());
        }
    }
}

fn sigarda_upkeep(game: &mut GameState) {
    game.turn.active_player = A;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
    let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::BeginningOfUpkeepEvent::new(A),
        Default::default(),
    );
    let entries = check_triggers(game, &event);
    assert_eq!(entries.len(), 1);
    let mut queue = TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, &mut Choices::default()).unwrap();
}

#[test]
fn sigarda_notes_after_both_draw_branches_and_its_white_spell_trigger_gains_real_life() {
    for definition in definitions("Sigarda's Splendor") {
        let mut game = game();
        library(&mut game, 4);
        let source = enter(&mut game, &definition, &mut Choices::default());
        assert_eq!(game.noted_life_total_for_source(source), Some(20));
        set_life(&mut game, source, A, 19);
        sigarda_upkeep(&mut game);
        resolve_all(&mut game, &mut Choices::default());
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(game.noted_life_total_for_source(source), Some(19));
        sigarda_upkeep(&mut game);
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(game.noted_life_total_for_source(source), Some(19));
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(A);
        let white = vanilla("White cast witness", "{W}", "Human", 1, 1);
        cast(
            &mut game,
            &white,
            CastingMethod::Normal,
            &mut Choices::default(),
        );
        assert_eq!(
            game.stack.len(),
            2,
            "casting the white spell creates a separate life-gain trigger"
        );
        resolve(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(
            game.noted_life_total_for_source(source),
            Some(19),
            "gaining life alone does not change the noted total"
        );
        resolve_all(&mut game, &mut Choices::default());
    }
}

#[test]
fn sigarda_copied_pending_trigger_uses_latest_same_incarnation_note_and_cannot_follow_return() {
    for definition in definitions("Sigarda's Splendor") {
        for departed_before_copy_resolves in [false, true] {
            let mut game = game();
            library(&mut game, 4);
            let setup = witness(&mut game);
            let source = enter(&mut game, &definition, &mut Choices::default());
            let stable = game.object(source).unwrap().stable_id;
            sigarda_upkeep(&mut game);
            let original = game.stack.last().unwrap().ability_id.unwrap();
            apply(
                &mut game,
                setup,
                Effect::copy_spell(ChooseSpec::SpecificObject(original)),
            );
            assert_eq!(game.stack.len(), 2);
            let early_graveyard = departed_before_copy_resolves.then(|| {
                game.move_object_by_game_rule(source, Zone::Graveyard)
                    .unwrap()
            });
            set_life(&mut game, setup, A, 18);
            resolve(&mut game, &mut Choices::default());
            assert_eq!(game.noted_life_total_for_source(source), Some(18));
            assert!(game.player(A).unwrap().hand.is_empty());
            let graveyard = early_graveyard.unwrap_or_else(|| {
                game.move_object_by_game_rule(source, Zone::Graveyard)
                    .unwrap()
            });
            if !departed_before_copy_resolves {
                assert_eq!(game.noted_life_total_for_source(source), None);
            }
            assert_eq!(
                game.stack
                    .last()
                    .unwrap()
                    .source_snapshot
                    .as_ref()
                    .unwrap()
                    .noted_life_total,
                Some(if departed_before_copy_resolves {
                    20
                } else {
                    18
                })
            );
            set_life(&mut game, setup, A, 30);
            apply(
                &mut game,
                setup,
                Effect::return_from_graveyard_to_battlefield(
                    ChooseSpec::SpecificObject(graveyard),
                    false,
                ),
            );
            let returned = game.find_object_by_stable_id(stable).unwrap();
            assert_ne!(returned, source);
            assert_eq!(game.noted_life_total_for_source(returned), Some(30));
            set_life(&mut game, setup, A, 19);
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(
                game.player(A).unwrap().hand.len(),
                1,
                "the old pending ability compares with its latest same-incarnation note 18, not its original 20 or returned incarnation's 30"
            );
            assert_eq!(game.noted_life_total_for_source(source), Some(19));
            assert_eq!(
                game.noted_life_total_for_source(returned),
                Some(30),
                "the old ability's re-note cannot mutate the new permanent"
            );
            sigarda_upkeep(&mut game);
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            assert_eq!(game.noted_life_total_for_source(returned), Some(19));
        }
    }
}

#[test]
fn sigarda_owned_by_a_departing_player_preserves_another_players_pending_latest_note() {
    for definition in definitions("Sigarda's Splendor") {
        for prune_departure_history in [false, true] {
            for phased_out in [false, true] {
                let mut game = game();
                library(&mut game, 4);
                let setup = witness(&mut game);
                let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
                game.set_current_controller(source, A).unwrap();
                sigarda_upkeep(&mut game);
                let original = game.stack.last().unwrap().ability_id.unwrap();
                apply(
                    &mut game,
                    setup,
                    Effect::copy_spell(ChooseSpec::SpecificObject(original)),
                );
                if phased_out {
                    game.phase_out(source);
                }
                set_life(&mut game, setup, A, 18);
                resolve(&mut game, &mut Choices::default());
                assert_eq!(game.noted_life_total_for_source(source), Some(18));
                assert!(
                    game.leave_game(B)
                        .expect("checked designation/departure fixture")
                );
                assert!(game.object(source).is_none());
                assert_eq!(game.noted_life_total_for_source(source), None);
                let pending = game.stack.last().unwrap();
                assert_eq!(pending.controller, A);
                assert_eq!(
                    pending.source_snapshot.as_ref().unwrap().noted_life_total,
                    Some(18)
                );
                if prune_departure_history {
                    // Pin the retained owner's fallback independently of the
                    // turn-history receipt, as needed by persisted continuations.
                    game.turn_store.turn_history.clear_for_new_turn();
                }
                set_life(&mut game, setup, A, 19);
                resolve_all(&mut game, &mut Choices::default());
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
                assert_eq!(game.noted_life_total_for_source(source), Some(19));
            }
        }
    }
}

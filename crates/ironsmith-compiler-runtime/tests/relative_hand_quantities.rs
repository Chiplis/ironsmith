//! UNVALIDATED exact-source quantity and live-versus-LKI reference regressions.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    NumberContext, SelectOptionsContext, TargetsContext, ViewCardsContext,
};
use ironsmith::effect::{Effect, EffectOutcome};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/relative_hand_quantities.json.fixture"
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
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
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
    player_targets: Vec<PlayerId>,
    all_modes: bool,
    x: u32,
    prefer_life: bool,
    viewed: Vec<Vec<ObjectId>>,
}
impl DecisionMaker for Choices {
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
        if self.all_modes && context.description.starts_with("Choose mode for") {
            return context
                .options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .collect();
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
        if !self.player_targets.is_empty() {
            context
                .requirements
                .iter()
                .enumerate()
                .map(|(index, requirement)| {
                    let player = self.player_targets[index % self.player_targets.len()];
                    assert!(requirement.legal_targets.contains(&Target::Player(player)));
                    Target::Player(player)
                })
                .collect()
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
}
fn queue_event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) -> usize {
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(game, &event) {
        queue.add(entry);
    }
    let count = queue.entries.len();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    count
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
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, dm).unwrap();
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
    let mut queue = TriggerQueue::new();
    resolve_stack_entry_with(game, dm).unwrap();
    for event in game.take_pending_trigger_events() {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn resolve_all(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..30 {
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
    let mut state = PriorityLoopState::new(game.players.len());
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
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}
fn populate(game: &mut GameState, player: PlayerId, zone: Zone, count: usize) {
    let card = vanilla("Quantity resource", "{1}", "Human", 1, 1);
    for _ in 0..count {
        game.create_object_from_definition(&card, player, zone);
    }
}
fn hand(game: &GameState, player: PlayerId) -> usize {
    game.player(player).unwrap().hand.len()
}
fn permanents(game: &GameState, player: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|id| game.current_controller(**id) == Some(player))
        .count()
}

#[test]
fn nine_quantity_cards_and_joint_kozilek_round_trip_with_executable_quantities() {
    assert_eq!(fixtures().len(), 10);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert!(!format!("{definition:?}").contains("PendingComparison"));
        }
    }
}

#[test]
fn global_half_instructions_sample_each_players_live_resources_and_round_every_choice() {
    for (name, up) in [("Fraying Omnipotence", true), ("Pox Plague", false)] {
        for definition in definitions(name) {
            let mut game = game();
            for (player, count) in [(A, 1), (B, 3), (C, 5)] {
                populate(&mut game, player, Zone::Hand, count);
                populate(&mut game, player, Zone::Battlefield, count);
                populate(&mut game, player, Zone::Library, 4);
            }
            let mut dm = Choices::default();
            let source = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            // All counts are resolution-time quantities, after public changes.
            apply(&mut game, source, Effect::draw(2));
            for (player, loss) in [(A, 1), (B, 3), (C, 5)] {
                apply(
                    &mut game,
                    source,
                    Effect::lose_life_player(loss, PlayerFilter::Specific(player)),
                );
            }
            let moved = *game
                .battlefield
                .iter()
                .find(|id| game.current_controller(**id) == Some(C))
                .unwrap();
            game.set_current_controller(moved, B).unwrap();
            resolve_all(&mut game, &mut dm);
            for (player, cards, life, objects) in [(A, 3, 19, 1), (B, 3, 17, 4), (C, 5, 15, 4)] {
                let remaining = |count: usize| if up { count / 2 } else { (count + 1) / 2 };
                assert_eq!(hand(&game, player), remaining(cards), "{name}: {player:?}");
                assert_eq!(game.player(player).unwrap().life, remaining(life) as i32);
                assert_eq!(permanents(&game, player), remaining(objects));
            }
        }
    }
}

#[test]
fn lord_xander_rounds_down_each_targeted_resource_and_preserves_attack_and_death() {
    for definition in definitions("Lord Xander, the Collector") {
        let mut game = game();
        populate(&mut game, B, Zone::Hand, 5);
        populate(&mut game, C, Zone::Hand, 9);
        populate(&mut game, B, Zone::Library, 7);
        populate(&mut game, B, Zone::Battlefield, 5);
        let mut dm = Choices {
            player_targets: vec![B],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(hand(&game, B), 3);
        assert_eq!(hand(&game, C), 9);
        let source = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().name.as_str() == definition.card.name.as_str())
            .unwrap();
        let attack = TriggerEvent::new_with_provenance(
            ironsmith::events::combat::CreatureAttackedEvent::new(
                source,
                ironsmith::triggers::AttackEventTarget::Player(B),
            ),
            Default::default(),
        );
        assert_eq!(queue_event(&mut game, attack, &mut dm), 1);
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().library.len(), 4);
        let outcome = apply(
            &mut game,
            source,
            Effect::destroy(ChooseSpec::SpecificObject(source)),
        );
        queue_outcome(&mut game, outcome, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(permanents(&game, B), 3);
    }
}

#[test]
fn spree_modes_sample_their_own_target_and_round_up_without_reusing_another_target() {
    for definition in definitions("Rush of Dread") {
        let mut game = game();
        populate(&mut game, B, Zone::Battlefield, 3);
        populate(&mut game, C, Zone::Battlefield, 9);
        populate(&mut game, B, Zone::Hand, 7);
        populate(&mut game, C, Zone::Hand, 5);
        let mut dm = Choices {
            player_targets: vec![B, C, B],
            all_modes: true,
            ..Default::default()
        };
        let source = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        apply(
            &mut game,
            source,
            Effect::lose_life_player(1, PlayerFilter::Specific(B)),
        );
        resolve_all(&mut game, &mut dm);
        assert_eq!(permanents(&game, B), 1);
        assert_eq!(permanents(&game, C), 9);
        assert_eq!(hand(&game, B), 7);
        assert_eq!(hand(&game, C), 2);
        assert_eq!(game.player(B).unwrap().life, 9);
        assert_eq!(game.player(C).unwrap().life, 20);
    }
}

#[test]
fn balance_of_power_uses_the_compared_target_and_only_draws_a_positive_live_difference() {
    for definition in definitions("Balance of Power") {
        for own in [2, 6, 9] {
            let mut game = game();
            populate(&mut game, A, Zone::Hand, own);
            populate(&mut game, B, Zone::Hand, 5);
            populate(&mut game, C, Zone::Hand, 12);
            populate(&mut game, A, Zone::Library, 20);
            let mut dm = Choices {
                player_targets: vec![B],
                ..Default::default()
            };
            let source = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            // Public draw after targets were announced changes the difference.
            apply(&mut game, source, Effect::draw(1));
            resolve_all(&mut game, &mut dm);
            assert_eq!(hand(&game, A), (own + 1).max(5));
            assert_eq!(hand(&game, B), 5);
        }
    }
}

#[test]
fn chosen_opponent_difference_survives_enter_leave_and_evoke_trigger_contexts() {
    for name in ["Sandstone Oracle", "Slithermuse"] {
        for definition in definitions(name) {
            let mut game = game();
            populate(&mut game, A, Zone::Hand, 2);
            populate(&mut game, B, Zone::Hand, 5);
            populate(&mut game, C, Zone::Hand, 11);
            populate(&mut game, A, Zone::Library, 20);
            let method = if name == "Slithermuse" {
                CastingMethod::Alternative(
                    definition
                        .alternative_casts
                        .iter()
                        .position(|a| a.name().eq_ignore_ascii_case("evoke"))
                        .unwrap(),
                )
            } else {
                CastingMethod::Normal
            };
            let mut dm = Choices::default();
            cast(&mut game, &definition, method, &mut dm);
            resolve_all(&mut game, &mut dm);
            assert_eq!(hand(&game, A), 5, "{name}");
            assert_eq!(
                permanents(&game, A),
                usize::from(name == "Sandstone Oracle")
            );
        }
    }
}

#[test]
fn skull_raid_draws_the_shortfall_from_actual_discarded_cards_including_zero() {
    for definition in definitions("Skull Raid") {
        for cards in [0usize, 1, 2, 5] {
            let mut game = game();
            populate(&mut game, A, Zone::Hand, 4);
            populate(&mut game, B, Zone::Hand, cards);
            populate(&mut game, C, Zone::Hand, 9);
            populate(&mut game, A, Zone::Library, 10);
            let mut dm = Choices {
                player_targets: vec![B],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve_all(&mut game, &mut dm);
            assert_eq!(hand(&game, B), cards.saturating_sub(2));
            assert_eq!(hand(&game, A), 4 + 2usize.saturating_sub(cards));
            assert_eq!(hand(&game, C), 9);
        }
    }
}

#[test]
fn heed_the_mists_reads_the_milled_object_not_the_next_card_or_source_cost() {
    for definition in definitions("Heed the Mists") {
        for mana_value in [0, 3, 7] {
            let mut game = game();
            // The library top is the last element. Remaining cards differ.
            populate(&mut game, A, Zone::Library, 12);
            game.create_object_from_definition(
                &vanilla("Milled object", &format!("{{{mana_value}}}"), "Human", 1, 1),
                A,
                Zone::Library,
            );
            let mut dm = Choices::default();
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve_all(&mut game, &mut dm);
            assert_eq!(hand(&game, A), mana_value as usize);
        }
    }
}

#[test]
fn cast_trigger_difference_keeps_seven_as_authored_boundary_and_rechecks_on_resolution() {
    // Joint exact-card closure requires the discard-cost family's 6e29de42.
    for definition in definitions("Kozilek, the Great Distortion") {
        for (initial, response_draw) in [(2usize, 0), (2, 2), (2, 6), (7, 0), (9, 0)] {
            let mut game = game();
            populate(&mut game, A, Zone::Hand, initial);
            populate(&mut game, A, Zone::Library, 20);
            let mut dm = Choices::default();
            let source = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            assert_eq!(game.stack.len(), if initial < 7 { 2 } else { 1 });
            apply(&mut game, source, Effect::draw(response_draw));
            if initial < 7 {
                resolve(&mut game, &mut dm);
            }
            assert_eq!(hand(&game, A), (initial + response_draw as usize).max(7));
            assert_eq!(game.stack.len(), 1);
        }
    }
}

#[test]
fn sandstone_oracle_cannot_choose_a_teammate_as_an_opponent_for_its_real_draw() {
    let teammate = PlayerId::from_index(1);
    let bob = PlayerId::from_index(2);
    let charlie = PlayerId::from_index(3);
    for definition in definitions("Sandstone Oracle") {
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
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Blue, 20);
            if teams {
                game.set_teams(vec![vec![A, teammate], vec![bob, charlie]])
                    .unwrap();
            }
            for (player, count) in [(A, 2), (teammate, 11), (bob, 5), (charlie, 9)] {
                populate(&mut game, player, Zone::Hand, count);
            }
            populate(&mut game, A, Zone::Library, 20);
            cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices::default(),
            );
            resolve(&mut game, &mut Choices::default());
            let mut queue = TriggerQueue::new();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut Choices::default()).unwrap();
            assert_eq!(game.stack.len(), 1);
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(hand(&game, A), if teams { 5 } else { 11 });
            assert_eq!(hand(&game, teammate), 11);
        }
    }
}

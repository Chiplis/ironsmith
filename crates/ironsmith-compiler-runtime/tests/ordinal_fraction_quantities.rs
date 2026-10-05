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
        "../../../fixtures/ordinal_fraction_quantities.json.fixture"
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
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm)
        .unwrap();
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
    let mut state = PriorityLoopState::new(2);
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
fn two_full_ordinal_fraction_cards_round_trip_with_real_resource_executors() {
    assert_eq!(fixtures().len(), 2);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert!(format!("{definition:?}").contains("DividedRoundedDown"));
        }
    }
}

#[test]
fn pox_rounds_up_life_hand_creature_and_land_counts_separately_for_each_player() {
    for definition in definitions("Pox") {
        let mut game = game();
        let land =
            compile_to_runtime_definition("Fraction land", "Type: Basic Land — Forest", false)
                .unwrap();
        let artifact = compile_to_runtime_definition(
            "Excluded permanent",
            "Mana cost: {1}\nType: Artifact",
            false,
        )
        .unwrap();
        for (player, cards, creatures, lands) in [(A, 0, 0, 0), (B, 1, 2, 3), (C, 5, 5, 7)] {
            populate(&mut game, player, Zone::Hand, cards);
            populate(&mut game, player, Zone::Battlefield, creatures);
            for _ in 0..lands {
                game.create_object_from_definition(&land, player, Zone::Battlefield);
            }
            game.create_object_from_definition(&artifact, player, Zone::Battlefield);
        }
        let mut dm = Choices::default();
        let source = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        apply(
            &mut game,
            source,
            Effect::lose_life_player(1, PlayerFilter::Specific(A)),
        );
        apply(
            &mut game,
            source,
            Effect::lose_life_player(3, PlayerFilter::Specific(B)),
        );
        apply(
            &mut game,
            source,
            Effect::lose_life_player(5, PlayerFilter::Specific(C)),
        );
        resolve_all(&mut game, &mut dm);
        let count = |player, kind| {
            game.battlefield
                .iter()
                .filter(|id| {
                    game.current_controller(**id) == Some(player)
                        && game.current_has_card_type(**id, kind)
                })
                .count()
        };
        for (player, cards, creatures, lands, life) in
            [(A, 0, 0, 0, 12), (B, 0, 1, 2, 11), (C, 3, 3, 4, 10)]
        {
            assert_eq!(hand(&game, player), cards);
            assert_eq!(count(player, ironsmith::CardType::Creature), creatures);
            assert_eq!(count(player, ironsmith::CardType::Land), lands);
            assert_eq!(count(player, ironsmith::CardType::Artifact), 1);
            assert_eq!(game.player(player).unwrap().life, life);
        }
    }
}

#[test]
fn ravager_uses_a_third_of_live_life_for_all_players_with_one_upward_rounding() {
    for definition in definitions("Dire Fleet Ravager") {
        let mut game = game();
        let mut dm = Choices::default();
        let source = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        for (player, lost) in [(A, 9), (B, 10), (C, 11)] {
            apply(
                &mut game,
                source,
                Effect::lose_life_player(lost, PlayerFilter::Specific(player)),
            );
        }
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 7); // ceil(11/3) = 4
        assert_eq!(game.player(B).unwrap().life, 6); // ceil(10/3) = 4
        assert_eq!(game.player(C).unwrap().life, 6); // ceil(9/3) = 3
    }
}

#[test]
fn general_unit_fraction_discard_counts_are_bounded_and_round_once_at_selection() {
    for (denominator, word) in [(2usize, "second"), (3, "third"), (4, "fourth")] {
        for up in [false, true] {
            let text = format!(
                "Mana cost: {{0}}\nType: Sorcery\nEach player discards a {word} of the cards in their hand, rounded {}.",
                if up { "up" } else { "down" }
            );
            for definition in definitions_text("Unit fraction probe", &text) {
                for cards in 0usize..=8 {
                    let mut game = game();
                    populate(&mut game, A, Zone::Hand, cards);
                    populate(&mut game, B, Zone::Hand, cards + 1);
                    let mut dm = Choices::default();
                    cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
                    resolve_all(&mut game, &mut dm);
                    for (player, original) in [(A, cards), (B, cards + 1), (C, 0)] {
                        let discarded =
                            (original + if up { denominator - 1 } else { 0 }) / denominator;
                        assert_eq!(
                            hand(&game, player),
                            original - discarded,
                            "d={denominator} up={up} cards={original}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn upward_third_at_maximum_supported_life_does_not_overflow_before_division() {
    for definition in definitions("Dire Fleet Ravager") {
        let mut game = game();
        let mut dm = Choices::default();
        let source = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        apply(
            &mut game,
            source,
            Effect::gain_life_player(i32::MAX - 20, ChooseSpec::SpecificPlayer(A)),
        );
        assert_eq!(game.player(A).unwrap().life, i32::MAX);
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 1_431_655_764);
        assert_eq!(game.player(B).unwrap().life, 13);
    }
}

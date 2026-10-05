//! UNVALIDATED Exact put-into-hand result scenarios; authored, unrun.
#![allow(dead_code)]
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
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
        "../../../fixtures/put_into_hand_results.json.fixture"
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

#[derive(Default)]
struct Choices {
    hit: Option<ObjectId>,
    decline: bool,
    pause: bool,
    waiting: bool,
    proliferated: usize,
}
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool {
        self.waiting
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        if self.pause {
            self.waiting = true;
            return false;
        }
        !self.decline
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.hit
            && ctx.candidates.iter().any(|c| c.id == id && c.legal)
        {
            if self.pause {
                self.waiting = true;
                return Vec::new();
            }
            return if self.decline { Vec::new() } else { vec![id] };
        }
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_proliferate(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::ProliferateContext,
    ) -> ironsmith::decisions::specs::ProliferateResponse {
        self.proliferated += 1;
        ironsmith::decisions::specs::ProliferateResponse {
            permanents: ctx.eligible_permanents.iter().map(|(id, _)| *id).collect(),
            players: ctx.eligible_players.iter().map(|(id, _)| *id).collect(),
        }
    }
}
fn resource(name: &str, types: &str) -> CardDefinition {
    compile_to_runtime_definition(
        name,
        format!(
            "Mana cost: {{1}}\nType: {types}{}",
            if types.contains("Creature") {
                "\nPower/Toughness: 2/2"
            } else {
                ""
            }
        ),
        false,
    )
    .unwrap()
}
fn settle(game: &mut GameState, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for _ in 0..20 {
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, dm).unwrap();
        if dm.waiting {
            return;
        }
    }
    panic!("finite ETB scenario did not settle");
}
#[test]
fn four_complete_bodies_round_trip_and_choose_decline_or_miss() {
    use ironsmith::object::CounterType;
    assert_eq!(fixtures().len(), 4);
    for (name, number, hit_type) in [
        ("Blossom Prancer", 5, "Creature — Bear"),
        ("Contagious Vorrac", 4, "Land"),
        ("Pulsar Squadron Ace", 5, "Artifact — Spacecraft"),
        ("Rosecot Knight", 6, "Enchantment"),
    ] {
        for definition in definitions(name) {
            for mode in 0..3 {
                let mut game = game();
                let witness = game.create_object_from_definition(
                    &resource("Proliferation witness", "Artifact"),
                    A,
                    Zone::Battlefield,
                );
                game.add_counters(witness, CounterType::Charge, 1).unwrap();
                let sentinel = game.create_object_from_definition(
                    &resource("Unviewed bottom sentinel", "Instant"),
                    A,
                    Zone::Library,
                );
                let hit = (mode != 2).then(|| {
                    game.create_object_from_definition(
                        &resource("Exact chosen card", hit_type),
                        A,
                        Zone::Library,
                    )
                });
                for n in 0..(number - usize::from(hit.is_some())) {
                    game.create_object_from_definition(
                        &resource(&format!("Remainder {n}"), "Instant"),
                        A,
                        Zone::Library,
                    );
                }
                let mut dm = Choices {
                    hit,
                    decline: mode == 1,
                    ..Choices::default()
                };
                cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
                settle(&mut game, &mut dm);
                assert!(!dm.waiting);
                let host = game
                    .battlefield
                    .iter()
                    .copied()
                    .find(|id| game.object(*id).unwrap().name == name)
                    .unwrap();
                let succeeded = mode == 0;
                assert_eq!(game.player(A).unwrap().hand.len(), usize::from(succeeded));
                assert_eq!(
                    game.player(A).unwrap().library.len(),
                    number + 1 - usize::from(succeeded)
                );
                assert_eq!(
                    game.player(A).unwrap().library.last(),
                    Some(&sentinel),
                    "the entire viewed remainder must move below the untouched sentinel"
                );
                assert_eq!(
                    game.player(A).unwrap().life,
                    if name == "Blossom Prancer" && !succeeded {
                        24
                    } else {
                        20
                    }
                );
                assert_eq!(
                    dm.proliferated,
                    usize::from(name == "Contagious Vorrac" && !succeeded)
                );
                assert_eq!(
                    game.counter_count(witness, CounterType::Charge),
                    if name == "Contagious Vorrac" && !succeeded {
                        2
                    } else {
                        1
                    }
                );
                assert_eq!(
                    game.counter_count(host, CounterType::PlusOnePlusOne),
                    u32::from(
                        !succeeded && matches!(name, "Pulsar Squadron Ace" | "Rosecot Knight")
                    )
                );
                if name == "Blossom Prancer" {
                    assert!(game.object_has_ability(
                        host,
                        &ironsmith::static_abilities::StaticAbility::reach()
                    ));
                }
                if name == "Rosecot Knight" {
                    assert!(game.object_has_ability(
                        host,
                        &ironsmith::static_abilities::StaticAbility::vigilance()
                    ));
                }
            }
        }
    }
}
fn hand_gate() -> Effect {
    let mut surface = ironsmith::effect::PriorEffectResultSurface::new(
        ironsmith::effect::PriorEffectAction::PutIntoHand,
        ironsmith::target::ObjectFilter::default(),
        ironsmith::effect::PriorEffectResultActor::You,
        ironsmith::effect::PriorEffectResultQuantifier::One,
    );
    surface.negated = true;
    Effect::if_then(
        ironsmith::effect::EffectId(9),
        ironsmith::effect::EffectPredicate::PriorEffectResult(surface),
        vec![Effect::gain_life(4)],
    )
}
#[test]
fn actual_arrival_not_selection_replacement_draw_or_later_current_zone() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for mode in 0..5 {
        let mut game = game();
        let source = game.create_object_from_definition(
            &resource("Result source", "Artifact"),
            A,
            Zone::Battlefield,
        );
        let card = game.create_object_from_definition(
            &resource("Original selected card", "Creature — Bear"),
            A,
            Zone::Library,
        );
        // A separate card is used by an Instead program: it must not stand in
        // for the original selected card's missing move.
        let substitute = game.create_object_from_definition(
            &resource("Independent replacement card", "Artifact"),
            A,
            Zone::Exile,
        );
        let action = match mode {
            0 => None,
            1 => Some(ReplacementAction::ChangeDestination(Zone::Exile)),
            2 => Some(ReplacementAction::Prevent),
            3 => Some(ReplacementAction::Instead(vec![Effect::move_to_zone(
                ChooseSpec::SpecificObject(substitute),
                Zone::Hand,
                false,
            )])),
            _ => Some(ReplacementAction::Additionally(vec![Effect::move_to_zone(
                ChooseSpec::tagged("it"),
                Zone::Exile,
                false,
            )])),
        };
        if let Some(action) = action {
            game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    A,
                    ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                        ironsmith::target::ObjectFilter::specific(card),
                        Some(Zone::Library),
                        Some(Zone::Hand),
                    ),
                    action,
                ),
            );
        }
        let mut dm = Choices::default();
        let mut ctx = EffectContext::new(source, A, &mut dm);
        let result = execute_effect(
            &mut game,
            &Effect::move_to_zone(ChooseSpec::SpecificObject(card), Zone::Hand, false),
            &mut ctx,
        )
        .unwrap();
        ctx.store_outcome(ironsmith::effect::EffectId(9), result);
        execute_effect(&mut game, &hand_gate(), &mut ctx).unwrap();
        assert_eq!(
            game.player(A).unwrap().life,
            if matches!(mode, 1..=3) { 24 } else { 20 },
            "mode {mode}"
        );
        if mode == 4 {
            assert!(
                game.player(A).unwrap().hand.is_empty(),
                "an additional move cannot erase a completed original arrival"
            );
        }
    }
}
#[test]
fn replacement_choice_pause_restores_original_and_does_not_publish_arrival() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    let mut game = game();
    let source =
        game.create_object_from_definition(&resource("Source", "Artifact"), A, Zone::Battlefield);
    let card =
        game.create_object_from_definition(&resource("Selected", "Artifact"), A, Zone::Library);
    game.effect_store
        .replacement_effects
        .add_one_shot_effect(ReplacementEffect::with_matcher(
            source,
            A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                ironsmith::target::ObjectFilter::specific(card),
                Some(Zone::Library),
                Some(Zone::Hand),
            ),
            ReplacementAction::Additionally(vec![Effect::may(vec![Effect::gain_life(1)])]),
        ));
    let mut dm = Choices {
        pause: true,
        ..Choices::default()
    };
    let mut ctx = EffectContext::new(source, A, &mut dm);
    let result = execute_effect(
        &mut game,
        &Effect::move_to_zone(ChooseSpec::SpecificObject(card), Zone::Hand, false),
        &mut ctx,
    )
    .unwrap();
    assert!(ctx.decision_maker.awaiting_choice());
    assert_eq!(game.object(card).unwrap().zone, Zone::Library);
    assert!(!result.execution_facts.iter().any(|fact| matches!(
        fact,
        ironsmith::effect::ExecutionFact::CardsPutIntoHand { .. }
    )));
    assert_eq!(game.player(A).unwrap().life, 20);
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
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}

#[test]
fn result_actor_requires_arrival_in_the_resolving_players_hand() {
    let mut game = game();
    let source = game.create_object_from_definition(
        &resource("Alice source", "Artifact"),
        A,
        Zone::Battlefield,
    );
    let card =
        game.create_object_from_definition(&resource("Bob owned card", "Artifact"), B, Zone::Exile);
    let mut dm = Choices::default();
    let mut ctx = EffectContext::new(source, A, &mut dm);
    let outcome = execute_effect(
        &mut game,
        &Effect::move_to_zone(ChooseSpec::SpecificObject(card), Zone::Hand, false),
        &mut ctx,
    )
    .unwrap();
    ctx.store_outcome(ironsmith::effect::EffectId(9), outcome);
    execute_effect(&mut game, &hand_gate(), &mut ctx).unwrap();
    assert_eq!(game.player(A).unwrap().life, 24);
    assert_eq!(game.player(B).unwrap().hand.len(), 1);
}

#[test]
fn a_spell_copy_bounced_before_state_based_actions_is_not_a_card_arrival() {
    let mut game = game();
    let source =
        game.create_object_from_definition(&resource("Source", "Artifact"), A, Zone::Battlefield);
    let original =
        game.create_object_from_definition(&resource("Copied spell", "Instant"), A, Zone::Stack);
    let copy_id = game.new_object_id();
    let copy = ironsmith::object::Object::spell_copy_of(game.object(original).unwrap(), copy_id, A);
    game.add_object(copy);
    let mut dm = Choices::default();
    let mut ctx = EffectContext::new(source, A, &mut dm);
    let outcome = execute_effect(
        &mut game,
        &Effect::move_to_zone(ChooseSpec::SpecificObject(copy_id), Zone::Hand, false),
        &mut ctx,
    )
    .unwrap();
    assert!(
        !outcome
            .instruction_result()
            .execution_facts
            .iter()
            .any(|fact| matches!(
                fact,
                ironsmith::effect::ExecutionFact::CardsPutIntoHand { .. }
            ))
    );
    ctx.store_outcome(ironsmith::effect::EffectId(9), outcome);
    execute_effect(&mut game, &hand_gate(), &mut ctx).unwrap();
    assert_eq!(game.player(A).unwrap().life, 24);
}

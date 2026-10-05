//! UNVALIDATED exact-source quantity and live-versus-LKI reference regressions.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    NumberContext, SelectOptionsContext, TargetsContext, ViewCardsContext,
};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/zone_history_quantities.json.fixture"
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
    target: Option<ObjectId>,
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
        if let Some(id) = self.target {
            assert!(
                context
                    .requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(&Target::Object(id)))
            );
            vec![Target::Object(id)]
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
fn tokens(game: &GameState, controller: PlayerId, subtype: Subtype) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|o| {
                o.kind == ObjectKind::Token
                    && game.current_controller(*id) == Some(controller)
                    && game.calculated_subtypes(*id).contains(&subtype)
            })
        })
        .collect()
}
fn counter_count(game: &GameState, id: ObjectId) -> u32 {
    game.object(id)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0)
}

fn history(game: &mut GameState, source: ObjectId) {
    // Opponent damage 2+3+3+2 = 10. Opponent life lost 2+3+4+3+2 = 14.
    apply(
        game,
        source,
        Effect::new(
            ironsmith::effects::DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(B))
                .with_combat(true),
        ),
    );
    apply(
        game,
        source,
        Effect::deal_damage(3, ChooseSpec::SpecificPlayer(C)),
    );
    apply(
        game,
        source,
        Effect::lose_life_player(4, PlayerFilter::Specific(B)),
    );
    apply(
        game,
        source,
        Effect::gain_life_player(10, ChooseSpec::SpecificPlayer(B)),
    );
    apply(
        game,
        source,
        Effect::new(ironsmith::effects::PayLifeEffect::new(
            2,
            ChooseSpec::SpecificPlayer(B),
        )),
    );
    apply(
        game,
        source,
        Effect::prevent_damage(2, ChooseSpec::SpecificPlayer(B), Until::EndOfTurn),
    );
    apply(
        game,
        source,
        Effect::deal_damage(5, ChooseSpec::SpecificPlayer(B)),
    );
    apply(
        game,
        source,
        Effect::deal_damage(7, ChooseSpec::SpecificPlayer(A)),
    );
    let infect = compile_to_runtime_definition(
        "Infect history",
        "Type: Creature — Human\nPower/Toughness: 1/1\nInfect",
        false,
    )
    .unwrap();
    let infect = game.create_object_from_definition(&infect, A, Zone::Battlefield);
    apply(
        game,
        infect,
        Effect::deal_damage(2, ChooseSpec::SpecificPlayer(B)),
    );
}
fn resource_spell(name: &str, types: &str) -> CardDefinition {
    compile_to_runtime_definition(
        name,
        format!("Mana cost: {{1}}\nType: {types}\nYou gain 1 life."),
        false,
    )
    .unwrap()
}

#[test]
fn four_full_cards_keep_distinct_history_metrics_and_owned_zone_unions() {
    assert_eq!(fixtures().len(), 4);
    for row in fixtures() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let debug = format!("{definition:?}");
            if name.starts_with("Florian") {
                assert!(debug.contains("LifeLostThisTurn(Opponent)"));
            } else if name == "Notorious Throng" {
                assert!(debug.contains("DamageDealtToPlayersThisTurn(Opponent)"));
            } else {
                assert!(
                    debug.contains("Graveyard")
                        && debug.contains("Exile")
                        && debug.contains("owner: Some(You)")
                );
            }
        }
    }
}

#[test]
fn florian_uses_total_life_lost_not_net_change_damage_or_number_of_opponents() {
    for definition in definitions("Florian, Voldaren Scion") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        history(&mut game, source);
        for index in 0..20 {
            game.create_object_from_definition(
                &resource_spell(&format!("Looked spell {index}"), "Instant"),
                A,
                Zone::Library,
            );
        }
        game.turn.phase = Phase::NextMain;
        let mut dm = Choices::default();
        let event = TriggerEvent::new_with_provenance(
            ironsmith::events::phase::BeginningOfPostcombatMainPhaseEvent::new(A),
            Default::default(),
        );
        assert_eq!(queue_event(&mut game, event, &mut dm), 1);
        resolve_all(&mut game, &mut dm);
        assert!(
            dm.viewed.iter().any(|cards| cards.len() == 14),
            "{:?}",
            dm.viewed
        );
        assert_eq!(game.exile.len(), 1);
        assert_eq!(game.player(A).unwrap().library.len(), 19);
        let exiled = *game.exile.iter().next().unwrap();
        assert!(compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action,
            LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } if *spell_id == exiled)));
        for _ in 0..3 {
            game.next_turn();
        }
        game.turn.phase = Phase::NextMain;
        let event = TriggerEvent::new_with_provenance(
            ironsmith::events::phase::BeginningOfPostcombatMainPhaseEvent::new(A),
            Default::default(),
        );
        dm.viewed.clear();
        assert_eq!(queue_event(&mut game, event, &mut dm), 1);
        resolve_all(&mut game, &mut dm);
        assert!(dm.viewed.iter().all(|cards| cards.is_empty()));
        assert_eq!(game.exile.len(), 1);
    }
}

#[test]
fn notorious_throng_counts_all_actual_opponent_damage_and_preserves_prowl_payment() {
    for definition in definitions("Notorious Throng") {
        for prowl in [false, true] {
            let mut game = game();
            let rogue = game.create_object_from_definition(
                &vanilla("Rogue source", "{1}", "Rogue", 2, 2),
                A,
                Zone::Battlefield,
            );
            history(&mut game, rogue);
            let method = if prowl {
                CastingMethod::Alternative(
                    definition
                        .alternative_casts
                        .iter()
                        .position(|method| method.name().eq_ignore_ascii_case("prowl"))
                        .unwrap(),
                )
            } else {
                CastingMethod::Normal
            };
            let mut dm = Choices::default();
            cast(&mut game, &definition, method, &mut dm);
            resolve_all(&mut game, &mut dm);
            let tokens = tokens(&game, A, Subtype::Faerie);
            assert_eq!(tokens.len(), 10);
            for id in tokens {
                assert!(game.calculated_subtypes(id).contains(&Subtype::Rogue));
                assert!(game.current_has_static_ability_id(
                    id,
                    ironsmith::static_abilities::StaticAbilityId::Flying
                ));
            }
            assert_eq!(
                game.turn_store.extra_turns.as_slice(),
                if prowl { &[A][..] } else { &[][..] }
            );
        }
    }
}

#[test]
fn serpentine_curve_counts_owned_instant_or_sorcery_cards_once_across_both_zones_at_resolution() {
    for definition in definitions("Serpentine Curve") {
        for move_before_resolution in [false, true] {
            let mut game = game();
            let instant = resource_spell("Instant count", "Instant");
            let sorcery = resource_spell("Sorcery count", "Sorcery");
            let both = resource_spell("Both counted once", "Instant Sorcery");
            game.create_object_from_definition(&instant, A, Zone::Exile);
            game.create_object_from_definition(&sorcery, A, Zone::Graveyard);
            let moving = game.create_object_from_definition(&both, A, Zone::Exile);
            game.create_object_from_definition(&instant, B, Zone::Exile);
            game.create_object_from_definition(&sorcery, B, Zone::Graveyard);
            game.create_object_from_definition(&instant, A, Zone::Library);
            game.create_object_from_definition(
                &vanilla("Wrong kind", "{1}", "Human", 1, 1),
                A,
                Zone::Graveyard,
            );
            let mut dm = Choices::default();
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            if move_before_resolution {
                game.move_object_by_game_rule(moving, Zone::Hand).unwrap();
            }
            resolve_all(&mut game, &mut dm);
            let made = tokens(&game, A, Subtype::Fractal);
            assert_eq!(made.len(), 1);
            assert_eq!(
                counter_count(&game, made[0]),
                if move_before_resolution { 3 } else { 4 }
            );
            assert_eq!(
                game.current_power(made[0]),
                Some(if move_before_resolution { 3 } else { 4 })
            );
        }
    }
}

#[test]
fn slime_keeps_name_or_subtype_inside_owned_zone_union_without_counting_a_match_twice() {
    for definition in definitions("Slime Against Humanity") {
        let mut game = game();
        let ooze = vanilla("Ooze count", "{2}", "Ooze", 1, 1);
        let named = resource_spell("Slime Against Humanity", "Sorcery");
        let both = vanilla("Slime Against Humanity", "{2}", "Ooze", 1, 1);
        game.create_object_from_definition(&ooze, A, Zone::Graveyard);
        game.create_object_from_definition(&named, A, Zone::Exile);
        game.create_object_from_definition(&both, A, Zone::Exile);
        game.create_object_from_definition(&named, B, Zone::Graveyard);
        game.create_object_from_definition(&ooze, B, Zone::Exile);
        game.create_object_from_definition(&ooze, A, Zone::Library);
        game.create_object_from_definition(&named, A, Zone::Hand);
        game.create_object_from_definition(
            &vanilla("Wrong name and subtype", "{1}", "Human", 1, 1),
            A,
            Zone::Exile,
        );
        let mut dm = Choices::default();
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve_all(&mut game, &mut dm);
        let made = tokens(&game, A, Subtype::Ooze);
        assert_eq!(made.len(), 1);
        assert_eq!(
            counter_count(&game, made[0]),
            5,
            "two plus three matching card objects"
        );
        assert_eq!(game.current_power(made[0]), Some(5));
        assert!(game.current_has_static_ability_id(
            made[0],
            ironsmith::static_abilities::StaticAbilityId::Trample
        ));
    }
}

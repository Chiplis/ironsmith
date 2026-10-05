//! UNVALIDATED full-card explicit mixed target references; all scenarios are authored, unrun.
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
        "../../../fixtures/declared_any_target_programs.json.fixture"
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
    discard_name: Option<&'static str>,
    objects: Vec<ObjectId>,
    objects_explicit: bool,
    x: u32,
    kicker_payments: usize,
    decline: bool,
    land_choice: Option<&'static str>,
    land_prompts: usize,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description == "Choose a basic land type" {
            self.land_prompts += 1;
            return vec![
                ctx.options
                    .iter()
                    .find(|option| option.description == self.land_choice.unwrap_or("Island"))
                    .unwrap()
                    .index,
            ];
        }
        if ctx.description.starts_with("Choose optional costs") {
            if self.kicker_payments == 0 {
                return vec![];
            }
            let option = ctx.options.iter().find(|option| option.legal).unwrap();
            return vec![option.index; self.kicker_payments];
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        !self.decline
    }
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            assert!(self.x <= ctx.max);
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, ctx)
        }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if !self.targets.is_empty() {
            assert_eq!(
                context.requirements.len(),
                1,
                "the explicit declaration is reused after intervening instructions"
            );
            assert_eq!(context.requirements[0].min_targets, self.targets.len());
            assert_eq!(
                context.requirements[0].max_targets,
                Some(self.targets.len())
            );
            for target in &self.targets {
                assert!(context.requirements[0].legal_targets.contains(target));
            }
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
        if context.description.to_lowercase().contains("discard") {
            if let Some(name) = self.discard_name {
                return vec![
                    context
                        .candidates
                        .iter()
                        .find(|c| game.object(c.id).is_some_and(|o| o.name == name))
                        .unwrap()
                        .id,
                ];
            }
        }
        if self.objects_explicit || !self.objects.is_empty() {
            for id in &self.objects {
                assert!(
                    context
                        .candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                );
            }
            self.objects.clone()
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
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
fn resource(name: &str, types: &str) -> CardDefinition {
    let pt = if types.contains("Creature") {
        "\nPower/Toughness: 1/1"
    } else {
        ""
    };
    compile_to_runtime_definition(name, format!("Mana cost: {{1}}\nType: {types}{pt}"), false)
        .unwrap()
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
fn pt(game: &GameState, id: ObjectId) -> (i32, i32) {
    (
        game.current_power(id).unwrap(),
        game.current_toughness(id).unwrap(),
    )
}
fn pump(game: &mut GameState, source: ObjectId, id: ObjectId, p: i32, t: i32) {
    apply(
        game,
        source,
        Effect::pump(p, t, ChooseSpec::SpecificObject(id), Until::EndOfTurn),
    );
}
fn counter(game: &mut GameState, source: ObjectId, id: ObjectId, count: i32) {
    apply(
        game,
        source,
        Effect::put_counters(
            ironsmith::object::CounterType::PlusOnePlusOne,
            count,
            ChooseSpec::SpecificObject(id),
        ),
    );
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

fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Choices) {
    game.turn.priority_player = Some(A);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| {
            matches!(action, LegalAction::ActivateAbility { source: id, ability_index: index }
            | LegalAction::ActivateManaAbility { source: id, ability_index: index }
            if *id == source && *index == ability_index)
        })
        .expect("the current ability must be legally activatable");
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
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
}
fn activated_at(definition: &CardDefinition, position: usize) -> usize {
    definition
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)
        })
        .nth(position)
        .unwrap()
}
fn current_activated_at(game: &GameState, source: ObjectId, position: usize) -> usize {
    game.calculated_characteristics(source)
        .unwrap()
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)
        })
        .nth(position)
        .unwrap()
}
fn has(
    game: &GameState,
    id: ObjectId,
    ability: ironsmith::static_abilities::StaticAbilityId,
) -> bool {
    game.current_has_static_ability_id(id, ability)
}
fn event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, &event) {
        queue.add(trigger);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn lore(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let outcome = apply(
        game,
        source,
        Effect::put_counters(
            ironsmith::object::CounterType::Lore,
            1,
            ChooseSpec::SpecificObject(source),
        ),
    );
    queue_outcome(game, outcome, dm);
    resolve_all(game, &mut Choices::default());
}

fn library_card(game: &mut GameState, name: &str, cost: &str, land: bool) -> ObjectId {
    let text = if land {
        "Type: Land — Forest".into()
    } else {
        format!("Mana cost: {cost}\nType: Sorcery\nYou gain 1 life.")
    };
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, A, Zone::Library)
}
fn in_zone_named(game: &GameState, zone: Zone, name: &str) -> usize {
    let player = game.player(A).unwrap();
    let ids = match zone {
        Zone::Library => &player.library,
        Zone::Hand => &player.hand,
        Zone::Graveyard => &player.graveyard,
        _ => panic!("unsupported fixture zone"),
    };
    ids.iter()
        .filter(|id| game.object(**id).is_some_and(|o| o.name == name))
        .count()
}
#[test]
fn six_complete_frozen_target_programs_round_trip_without_redeclaring_the_recipient() {
    for row in fixtures()
        .into_iter()
        .filter(|r| r["proposed_complete"] == true)
    {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"]);
        }
    }
}
#[test]
fn blast_keeps_the_announced_player_or_permanent_after_draw_and_discard() {
    for definition in definitions("Blast of Genius") {
        for player_target in [false, true] {
            let mut game = game();
            let recipient = game.create_object_from_definition(
                &vanilla("Recipient", "{2}", "Bear", 2, 8),
                B,
                Zone::Battlefield,
            );
            library_card(&mut game, "Discarded five", "{5}", false);
            library_card(&mut game, "Other one", "{1}", false);
            library_card(&mut game, "Other two", "{2}", false);
            let mut dm = Choices {
                targets: vec![if player_target {
                    Target::Player(B)
                } else {
                    Target::Object(recipient)
                }],
                discard_name: Some("Discarded five"),
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), 2);
            assert_eq!(in_zone_named(&game, Zone::Graveyard, "Discarded five"), 1);
            assert_eq!(
                game.player(B).unwrap().life,
                if player_target { 15 } else { 20 }
            );
            assert_eq!(game.damage_on(recipient), if player_target { 0 } else { 5 });
        }
    }
}
#[test]
fn reveal_until_body_preserves_the_hit_and_all_remainder_destinations() {
    for name in ["Erratic Explosion", "Explosive Revelation"] {
        for definition in definitions(name) {
            for player_target in [false, true] {
                let mut game = game();
                let recipient = game.create_object_from_definition(
                    &vanilla("Recipient", "{2}", "Bear", 2, 8),
                    B,
                    Zone::Battlefield,
                );
                library_card(&mut game, "Hit five", "{5}", false);
                library_card(&mut game, "Revealed land", "", true);
                let mut dm = Choices {
                    targets: vec![if player_target {
                        Target::Player(B)
                    } else {
                        Target::Object(recipient)
                    }],
                    ..Default::default()
                };
                cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
                resolve(&mut game, &mut dm);
                assert_eq!(
                    game.player(B).unwrap().life,
                    if player_target { 15 } else { 20 }
                );
                assert_eq!(game.damage_on(recipient), if player_target { 0 } else { 5 });
                assert_eq!(in_zone_named(&game, Zone::Library, "Revealed land"), 1);
                assert_eq!(
                    in_zone_named(
                        &game,
                        if name == "Explosive Revelation" {
                            Zone::Hand
                        } else {
                            Zone::Library
                        },
                        "Hit five"
                    ),
                    1,
                    "{name}: hit destination"
                );
                assert_eq!(
                    game.player(A).unwrap().library.len(),
                    if name == "Explosive Revelation" { 1 } else { 2 }
                );
            }
        }
    }
}
#[test]
fn heretic_uses_greatest_milled_value_and_the_original_activation_target() {
    for definition in definitions("Heretic's Punishment") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for (name, cost) in [
            ("Milled one", "{1}"),
            ("Milled seven", "{7}"),
            ("Milled four", "{4}"),
        ] {
            library_card(&mut game, name, cost, false);
        }
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        let before = game.player(A).unwrap().mana_pool.total();
        activate(&mut game, source, activated_at(&definition, 0), &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 4);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, 13);
        assert_eq!(game.player(A).unwrap().library.len(), 0);
        for name in ["Milled one", "Milled seven", "Milled four"] {
            assert_eq!(in_zone_named(&game, Zone::Graveyard, name), 1);
        }
    }
}
#[test]
fn riddle_scries_and_reveals_without_moving_or_retargeting_to_the_card() {
    for definition in definitions("Riddle of Lightning") {
        let mut game = game();
        for name in ["Scry four A", "Scry four B", "Scry four C"] {
            library_card(&mut game, name, "{4}", false);
        }
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, 16);
        assert_eq!(game.player(A).unwrap().library.len(), 3);
        assert!(game.player(A).unwrap().hand.is_empty());
    }
}
#[test]
fn all_illegal_target_fizzles_before_intervening_private_information_or_discard() {
    for definition in definitions("Blast of Genius") {
        let mut game = game();
        let recipient = game.create_object_from_definition(
            &vanilla("Departing recipient", "{2}", "Bear", 2, 8),
            B,
            Zone::Battlefield,
        );
        for name in ["Unread A", "Unread B", "Unread C"] {
            library_card(&mut game, name, "{5}", false);
        }
        let mut dm = Choices {
            targets: vec![Target::Object(recipient)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        apply(
            &mut game,
            recipient,
            Effect::exile(ChooseSpec::SpecificObject(recipient)),
        );
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().library.len(), 3);
        assert!(game.player(A).unwrap().hand.is_empty());
    }
}

#[test]
fn comet_counts_actual_kicker_payments_and_reuses_one_mixed_target_group() {
    for definition in definitions("Comet Storm") {
        for kicks in [0usize, 1, 2] {
            for depart in [false, true] {
                let mut game = game();
                let recipient = game.create_object_from_definition(
                    &vanilla("Comet recipient", "{2}", "Bear", 2, 8),
                    B,
                    Zone::Battlefield,
                );
                let mut targets = vec![Target::Object(recipient)];
                if kicks >= 1 {
                    targets.push(Target::Player(B));
                }
                if kicks >= 2 {
                    targets.push(Target::Player(C));
                }
                let mut dm = Choices {
                    targets,
                    x: 3,
                    kicker_payments: kicks,
                    ..Default::default()
                };
                let before = game.player(A).unwrap().mana_pool.total();
                let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
                assert_eq!(
                    game.object(spell).unwrap().optional_costs_paid.kick_count(),
                    kicks as u32
                );
                assert_eq!(
                    game.player(A).unwrap().mana_pool.total(),
                    before - 5 - kicks as u32
                );
                if depart {
                    apply(
                        &mut game,
                        recipient,
                        Effect::exile(ChooseSpec::SpecificObject(recipient)),
                    );
                }
                resolve(&mut game, &mut dm);
                if !depart {
                    assert_eq!(game.damage_on(recipient), 3);
                }
                assert_eq!(
                    game.player(B).unwrap().life,
                    if kicks >= 1 { 17 } else { 20 }
                );
                assert_eq!(
                    game.player(C).unwrap().life,
                    if kicks >= 2 { 17 } else { 20 }
                );
            }
        }
    }
}

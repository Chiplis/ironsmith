//! UNVALIDATED additive damage; scenarios are authored, unrun.
#![allow(dead_code)]
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effect::{Effect, EffectOutcome, Until, Value};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::effects::{RegisterDamageAdditionEffect, ReplacementApplyMode};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ObjectFilter;
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
        "../../../fixtures/additive_damage_replacements.json.fixture"
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
    objects: Vec<ObjectId>,
    objects_explicit: bool,
    x: u32,
    decline: bool,
    land_choice: Option<&'static str>,
    land_prompts: usize,
    modes: Vec<usize>,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose mode") && !self.modes.is_empty() {
            return self.modes.clone();
        }
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
            if self.decline {
                return vec![];
            }
            return ctx
                .options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .collect();
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
            assert_eq!(context.requirements.len(), self.targets.len());
            for (requirement, target) in context.requirements.iter().zip(&self.targets) {
                assert!(requirement.legal_targets.contains(target));
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
    cast_for(game, A, definition, method, dm)
}
fn cast_for(
    game: &mut GameState,
    player: PlayerId,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, player, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(
        compute_legal_actions(game, player)
            .unwrap()
            .contains(&action)
    );
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

#[test]
fn fresh_native_addition_retains_quantity_source_recipient_and_duration_in_its_codec() {
    let effect = RegisterDamageAdditionEffect {
        source_filter: ObjectFilter::default().controlled_by(PlayerFilter::You),
        target_player_filter: Some(PlayerFilter::Any),
        target_object_filter: Some(ObjectFilter::permanent()),
        delta: Value::X,
        noncombat_only: true,
        mode: ReplacementApplyMode::UntilEndOfTurn,
    };
    let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(
        Effect::new(effect.clone()),
    )
    .unwrap();
    let wire = serde_json::from_slice(&serde_json::to_vec(&wire).unwrap()).unwrap();
    let restored =
        ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire).unwrap();
    assert_eq!(
        restored.downcast_ref::<RegisterDamageAdditionEffect>(),
        Some(&effect)
    );
}

fn named(game: &GameState, name: &str) -> ObjectId {
    *game
        .battlefield
        .iter()
        .find(|id| {
            game.object(**id)
                .is_some_and(|object| object.name.as_str() == name)
        })
        .unwrap()
}
fn has(
    game: &GameState,
    id: ObjectId,
    ability: ironsmith::static_abilities::StaticAbilityId,
) -> bool {
    game.current_characteristics(id)
        .unwrap()
        .static_abilities
        .iter()
        .any(|a| a.id() == ability)
}
fn flush(game: &mut GameState, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    resolve_all(game, dm);
}
fn next_chapter(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::add_lore_counter_and_check_chapters(game, source, &mut queue).unwrap();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    resolve_all(game, dm);
}
fn damage(
    game: &mut GameState,
    source: ObjectId,
    target: Target,
    amount: i32,
    combat: bool,
) -> EffectOutcome {
    let choose = match target {
        Target::Object(id) => ChooseSpec::SpecificObject(id),
        Target::Player(id) => ChooseSpec::SpecificPlayer(id),
    };
    apply(
        game,
        source,
        Effect::new(ironsmith::effects::DealDamageEffect::new(amount, choose).with_combat(combat)),
    )
}
#[test]
fn five_complete_bodies_compile_strictly_and_round_trip_without_partial_tail() {
    for name in [
        "Fated Firepower",
        "Hawkeye, Young Avenger",
        "Rankle and Torbran",
        "The Flame of Keld",
        "Aether Revolt",
    ] {
        let _ = definitions(name);
    }
}
#[test]
fn fated_real_flash_cast_captures_entry_x_but_reads_fire_counters_live() {
    for definition in definitions("Fated Firepower") {
        for x in [0, 3] {
            let mut game = game();
            game.turn.active_player = B;
            game.turn.priority_player = Some(A);
            game.turn.phase = Phase::Combat;
            let own = game.create_object_from_definition(
                &vanilla("Own", "{R}", "Human", 2, 2),
                A,
                Zone::Battlefield,
            );
            let enemy = game.create_object_from_definition(
                &vanilla("Enemy", "{R}", "Human", 2, 2),
                B,
                Zone::Battlefield,
            );
            let mut dm = Choices {
                x,
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve_all(&mut game, &mut dm);
            let source = named(&game, "Fated Firepower");
            let fire = ironsmith::object::CounterType::Named("fire".into());
            assert_eq!(
                game.object(source)
                    .unwrap()
                    .counters
                    .get(&fire)
                    .copied()
                    .unwrap_or(0),
                x
            );
            let life = game.player(B).unwrap().life;
            damage(&mut game, own, Target::Player(B), 1, false);
            assert_eq!(game.player(B).unwrap().life, life - 1 - x as i32);
            game.object_mut(source).unwrap().counters.insert(fire, 5);
            let life = game.player(B).unwrap().life;
            damage(&mut game, own, Target::Player(B), 1, true);
            assert_eq!(game.player(B).unwrap().life, life - 6);
            let life = game.player(A).unwrap().life;
            damage(&mut game, enemy, Target::Player(A), 1, false);
            assert_eq!(game.player(A).unwrap().life, life - 1);
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            let life = game.player(B).unwrap().life;
            damage(&mut game, own, Target::Player(B), 1, false);
            assert_eq!(game.player(B).unwrap().life, life - 1);
        }
    }
}
#[test]
fn hawkeye_full_body_uses_its_current_power_only_on_own_noncombat_damage() {
    for definition in definitions("Hawkeye, Young Avenger") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = game.create_object_from_definition(
            &vanilla("Own", "{R}", "Human", 9, 9),
            A,
            Zone::Battlefield,
        );
        assert!(has(
            &game,
            source,
            ironsmith::static_abilities::StaticAbilityId::Reach
        ));
        let life = game.player(B).unwrap().life;
        damage(&mut game, own, Target::Player(B), 1, false);
        assert_eq!(game.player(B).unwrap().life, life - 3);
        pump(&mut game, source, source, 3, 0);
        let life = game.player(B).unwrap().life;
        damage(&mut game, own, Target::Player(B), 1, false);
        assert_eq!(game.player(B).unwrap().life, life - 6);
        let life = game.player(B).unwrap().life;
        damage(&mut game, own, Target::Player(B), 1, true);
        assert_eq!(game.player(B).unwrap().life, life - 1);
    }
}
#[test]
fn flame_three_chapters_keep_hand_draw_and_resolving_red_bonus_after_source_departure() {
    for definition in definitions("The Flame of Keld") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let red = game.create_object_from_definition(
            &vanilla("Red dealer", "{R}", "Human", 2, 2),
            A,
            Zone::Battlefield,
        );
        let blue = game.create_object_from_definition(
            &vanilla("Blue dealer", "{U}", "Human", 2, 2),
            A,
            Zone::Battlefield,
        );
        for _ in 0..3 {
            game.create_object_from_definition(
                &vanilla("Discard", "{1}", "Human", 1, 1),
                A,
                Zone::Hand,
            );
        }
        for _ in 0..4 {
            game.create_object_from_definition(
                &vanilla("Draw", "{1}", "Human", 1, 1),
                A,
                Zone::Library,
            );
        }
        let mut dm = Choices::default();
        next_chapter(&mut game, source, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 0);
        next_chapter(&mut game, source, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        next_chapter(&mut game, source, &mut dm);
        if game
            .object(source)
            .is_some_and(|o| o.zone == Zone::Battlefield)
        {
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        }
        let life = game.player(B).unwrap().life;
        damage(&mut game, red, Target::Player(B), 1, false);
        assert_eq!(game.player(B).unwrap().life, life - 3);
        let life = game.player(B).unwrap().life;
        damage(&mut game, blue, Target::Player(B), 1, false);
        assert_eq!(game.player(B).unwrap().life, life - 1);
        ironsmith::turn::execute_cleanup_step(&mut game);
        let life = game.player(B).unwrap().life;
        damage(&mut game, red, Target::Player(B), 1, false);
        assert_eq!(game.player(B).unwrap().life, life - 1);
    }
}
#[test]
fn rankle_all_modes_retain_treasures_sacrifice_and_player_battle_only_bonus() {
    for definition in definitions("Rankle and Torbran") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for player in [B, C] {
            game.create_object_from_definition(
                &vanilla("Sacrifice", "{1}", "Human", 1, 1),
                player,
                Zone::Battlefield,
            );
        }
        for ability in [
            ironsmith::static_abilities::StaticAbilityId::Flying,
            ironsmith::static_abilities::StaticAbilityId::FirstStrike,
            ironsmith::static_abilities::StaticAbilityId::Haste,
        ] {
            assert!(has(&game, source, ability));
        }
        let outcome = damage(&mut game, source, Target::Player(B), 3, true);
        let mut dm = Choices {
            modes: vec![0, 1, 2],
            ..Default::default()
        };
        queue_outcome(&mut game, outcome, &mut dm);
        resolve_all(&mut game, &mut dm);
        for player in [A, B, C] {
            assert_eq!(
                game.battlefield
                    .iter()
                    .filter(|id| game
                        .object(**id)
                        .is_some_and(|o| game.current_controller(o.id) == Some(player)
                            && o.name.as_str() == "Treasure"))
                    .count(),
                1
            );
        }
        assert!(
            !game
                .object(source)
                .is_some_and(|o| o.zone == Zone::Battlefield)
        );
        let dealer = game.create_object_from_definition(
            &vanilla("Dealer", "{R}", "Human", 2, 5),
            B,
            Zone::Battlefield,
        );
        let creature = game.create_object_from_definition(
            &vanilla("Victim", "{G}", "Human", 2, 10),
            A,
            Zone::Battlefield,
        );
        let life = game.player(A).unwrap().life;
        damage(&mut game, dealer, Target::Player(A), 1, false);
        assert_eq!(game.player(A).unwrap().life, life - 3);
        damage(&mut game, dealer, Target::Object(creature), 1, false);
        assert_eq!(game.damage_on(creature), 1);
        let battle = compile_to_runtime_definition(
            "Battle victim",
            "Mana cost: {2}
Type: Battle — Siege
Defense: 10",
            false,
        )
        .unwrap();
        let battle = game.create_object_from_definition(&battle, A, Zone::Battlefield);
        game.object_mut(battle)
            .unwrap()
            .counters
            .insert(ironsmith::object::CounterType::Defense, 10);
        let defense = game
            .object(battle)
            .unwrap()
            .counters
            .get(&ironsmith::object::CounterType::Defense)
            .copied()
            .unwrap_or(0);
        damage(&mut game, dealer, Target::Object(battle), 1, false);
        assert_eq!(
            game.object(battle)
                .unwrap()
                .counters
                .get(&ironsmith::object::CounterType::Defense)
                .copied()
                .unwrap_or(0),
            defense - 3
        );
    }
}
#[test]
fn aether_energy_trigger_captures_actual_gain_and_revolt_is_live() {
    for definition in definitions("Aether Revolt") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let outcome = apply(
            &mut game,
            source,
            Effect::new(ironsmith::effects::EnergyCountersEffect::you(3)),
        );
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        queue_outcome(&mut game, outcome, &mut dm);
        apply(
            &mut game,
            source,
            Effect::new(ironsmith::effects::PayEnergyEffect::new(
                2,
                ChooseSpec::SpecificPlayer(A),
            )),
        );
        let life = game.player(B).unwrap().life;
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, life - 3);
        let departed = game.create_object_from_definition(
            &vanilla("Departed", "{1}", "Human", 1, 1),
            A,
            Zone::Battlefield,
        );
        game.move_object_by_effect(departed, Zone::Graveyard)
            .unwrap();
        let outcome = apply(
            &mut game,
            source,
            Effect::new(ironsmith::effects::EnergyCountersEffect::you(2)),
        );
        queue_outcome(&mut game, outcome, &mut dm);
        let life = game.player(B).unwrap().life;
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, life - 4);
        let life = game.player(B).unwrap().life;
        damage(&mut game, source, Target::Player(B), 1, true);
        assert_eq!(game.player(B).unwrap().life, life - 1);
    }
}

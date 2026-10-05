//! UNVALIDATED exact-source quantity and live-versus-LKI reference regressions.
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
        "../../../fixtures/damage_multiplier_scopes.json.fixture"
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
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    mode: Option<usize>,
    x: u32,
    prefer_life: bool,
    viewed: Vec<Vec<ObjectId>>,
    player_name: Option<&'static str>,
    replacement_source_first: Option<ObjectId>,
    replacements_chosen_by: Vec<PlayerId>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        true
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
        if context.description == "Choose which replacement effect to apply" {
            self.replacements_chosen_by.push(context.player);
            if let Some(preferred) = self.replacement_source_first {
                if let Some(option) = context
                    .options
                    .iter()
                    .find(|o| o.legal && o.object_id == Some(preferred))
                {
                    return vec![option.index];
                }
            }
        }
        if let Some(name) = self.player_name {
            if let Some(option) = context
                .options
                .iter()
                .find(|o| o.legal && o.description == name)
            {
                return vec![option.index];
            }
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
        if !self.targets.is_empty() {
            for target in &self.targets {
                assert!(
                    context
                        .requirements
                        .iter()
                        .any(|requirement| requirement.legal_targets.contains(target))
                );
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
        if !self.objects.is_empty() {
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
fn activated(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap()
}

fn body(name: &str, text: &str) -> CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap()
}
fn witness(game: &mut GameState, owner: PlayerId) -> ObjectId {
    game.create_object_from_definition(
        &vanilla("Damage witness", "{2}", "Human", 2, 100),
        owner,
        Zone::Battlefield,
    )
}
fn damage(
    game: &mut GameState,
    source: ObjectId,
    target: ChooseSpec,
    combat: bool,
    amount: i32,
) -> EffectOutcome {
    apply(
        game,
        source,
        Effect::new(ironsmith::effects::DealDamageEffect::new(amount, target).with_combat(combat)),
    )
}
fn attach(game: &mut GameState, attachment: ObjectId, target: ObjectId) {
    assert!(game.attach_object_to_target(
        attachment,
        ironsmith::object::AttachmentTarget::Object(target)
    ));
}
fn enter(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let spell = cast(game, definition, CastingMethod::Normal, dm);
    let stable = game.object(spell).unwrap().stable_id;
    resolve(game, dm);
    let id = game
        .battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).unwrap().stable_id == stable)
        .unwrap();
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    id
}
#[test]
fn all_eight_full_metadata_bodies_round_trip_without_loss() {
    for row in fixtures() {
        let defs = definitions(row["name"].as_str().unwrap());
        for definition in defs {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
            let rendered = ironsmith_text::canonical_compiled_lines(&definition)
                .join("\n")
                .to_ascii_lowercase();
            assert!(
                rendered.contains("double"),
                "{}: {rendered}",
                definition.card.name
            );
            if definition.card.name == "The Rollercrusher Ride" {
                assert!(rendered.contains("noncombat damage"), "{rendered}");
                assert!(rendered.contains("graveyard"), "{rendered}");
            }
        }
    }
}

#[test]
fn curse_and_castigator_track_actual_recipients_and_current_static_controller() {
    for definition in definitions("Curse of Bloodletting") {
        let mut game = game();
        let curse = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(
            game.attach_object_to_target(curse, ironsmith::object::AttachmentTarget::Player(B))
        );
        let source = witness(&mut game, C);
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            false,
            3,
        );
        assert_eq!(game.player(B).unwrap().life, 14);
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(A)),
            true,
            3,
        );
        assert_eq!(game.player(A).unwrap().life, 17);
        let permanent = witness(&mut game, B);
        damage(
            &mut game,
            source,
            ChooseSpec::SpecificObject(permanent),
            false,
            3,
        );
        assert_eq!(game.damage_on(permanent), 3);
        assert!(
            game.attach_object_to_target(curse, ironsmith::object::AttachmentTarget::Player(C))
        );
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            false,
            1,
        );
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(C)),
            false,
            1,
        );
        assert_eq!(
            (game.player(B).unwrap().life, game.player(C).unwrap().life),
            (13, 18)
        );
    }
    for definition in definitions("Goldnight Castigator") {
        let mut game = game();
        let castigator = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = witness(&mut game, C);
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(A)),
            false,
            2,
        );
        damage(
            &mut game,
            source,
            ChooseSpec::SpecificObject(castigator),
            true,
            1,
        );
        assert_eq!(
            (game.player(A).unwrap().life, game.damage_on(castigator)),
            (16, 2)
        );
        game.set_current_controller(castigator, B).unwrap();
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(A)),
            false,
            1,
        );
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            false,
            1,
        );
        assert_eq!(
            (game.player(A).unwrap().life, game.player(B).unwrap().life),
            (15, 18)
        );
        game.move_object_by_game_rule(castigator, Zone::Graveyard)
            .unwrap();
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            false,
            1,
        );
        assert_eq!(game.player(B).unwrap().life, 17);
    }
}

#[test]
fn flail_tracks_its_host_and_excludes_that_host_from_incoming_other_sources() {
    for definition in definitions("Inquisitor's Flail") {
        let mut game = game();
        let flail = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let host = witness(&mut game, A);
        let other = witness(&mut game, B);
        let stray = game.create_object_from_definition(
            &body("Other Equipment", "Type: Artifact — Equipment"),
            C,
            Zone::Battlefield,
        );
        attach(&mut game, stray, other);
        damage(
            &mut game,
            other,
            ChooseSpec::Player(PlayerFilter::Specific(A)),
            true,
            1,
        );
        assert_eq!(
            game.player(A).unwrap().life,
            19,
            "unattached Flail must not borrow another Equipment's host"
        );
        // Real equip activation/payment, not a hand-written static marker.
        activate(
            &mut game,
            flail,
            activated(&definition),
            &mut Choices {
                targets: vec![Target::Object(host)],
                ..Default::default()
            },
        );
        resolve(&mut game, &mut Choices::default());
        damage(
            &mut game,
            host,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            true,
            2,
        );
        assert_eq!(game.player(B).unwrap().life, 16);
        damage(&mut game, other, ChooseSpec::SpecificObject(host), true, 2);
        assert_eq!(game.damage_on(host), 4);
        damage(&mut game, host, ChooseSpec::SpecificObject(host), true, 2);
        assert_eq!(
            game.damage_on(host),
            8,
            "self-damage doubles once, not twice"
        );
        damage(&mut game, other, ChooseSpec::SpecificObject(host), false, 2);
        assert_eq!(game.damage_on(host), 10);
        attach(&mut game, flail, other);
        damage(
            &mut game,
            host,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            true,
            1,
        );
        assert_eq!(game.player(B).unwrap().life, 15);
        damage(
            &mut game,
            other,
            ChooseSpec::Player(PlayerFilter::Specific(A)),
            true,
            1,
        );
        assert_eq!(game.player(A).unwrap().life, 17);
    }
}

#[test]
fn blind_fury_is_multi_use_combat_creature_to_creature_only_and_expires() {
    for definition in definitions("Blind Fury") {
        let mut game = game();
        let source = game.create_object_from_definition(
            &body(
                "Trampler",
                "Type: Creature
Power/Toughness: 2/100
Trample",
            ),
            A,
            Zone::Battlefield,
        );
        let target = witness(&mut game, B);
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices::default(),
        );
        resolve(&mut game, &mut Choices::default());
        assert!(!game.current_has_static_ability_id(
            source,
            ironsmith::static_abilities::StaticAbilityId::Trample
        ));
        for _ in 0..2 {
            damage(
                &mut game,
                source,
                ChooseSpec::SpecificObject(target),
                true,
                2,
            );
        }
        assert_eq!(game.damage_on(target), 8);
        damage(
            &mut game,
            source,
            ChooseSpec::SpecificObject(target),
            false,
            2,
        );
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            true,
            2,
        );
        let artifact = game.create_object_from_definition(
            &body("Artifact source", "Type: Artifact"),
            A,
            Zone::Battlefield,
        );
        damage(
            &mut game,
            artifact,
            ChooseSpec::SpecificObject(target),
            true,
            2,
        );
        assert_eq!(game.damage_on(target), 12);
        assert_eq!(game.player(B).unwrap().life, 18);
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(game.current_has_static_ability_id(
            source,
            ironsmith::static_abilities::StaticAbilityId::Trample
        ));
        damage(
            &mut game,
            source,
            ChooseSpec::SpecificObject(target),
            true,
            2,
        );
        assert_eq!(game.damage_on(target), 2);
    }
}

#[test]
fn goblin_goliath_captures_ability_controller_but_queries_each_damage_source_live() {
    for definition in definitions("Goblin Goliath") {
        let mut game = game();
        let mut dm = Choices::default();
        let goliath = enter(&mut game, &definition, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(
            game.battlefield
                .iter()
                .filter(|id| game.object(**id).unwrap().name == "Goblin")
                .count(),
            2
        );
        game.remove_summoning_sickness(goliath);
        activate(&mut game, goliath, activated(&definition), &mut dm);
        game.set_current_controller(goliath, B).unwrap();
        game.move_object_by_game_rule(goliath, Zone::Graveyard)
            .unwrap();
        resolve(&mut game, &mut dm);
        let source = witness(&mut game, A);
        for opponent in [B, C] {
            damage(
                &mut game,
                source,
                ChooseSpec::Player(PlayerFilter::Specific(opponent)),
                false,
                2,
            );
        }
        assert_eq!(
            (game.player(B).unwrap().life, game.player(C).unwrap().life),
            (16, 16)
        );
        let target = witness(&mut game, B);
        damage(
            &mut game,
            source,
            ChooseSpec::SpecificObject(target),
            false,
            2,
        );
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(A)),
            false,
            2,
        );
        assert_eq!(
            (game.damage_on(target), game.player(A).unwrap().life),
            (2, 18)
        );
        game.set_current_controller(source, B).unwrap();
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(C)),
            false,
            2,
        );
        assert_eq!(game.player(C).unwrap().life, 14);
        let pinger_definition = body(
            "Departing Pinger",
            "Type: Creature — Goblin\nPower/Toughness: 1/1\n{T}: This creature deals 2 damage to target player.",
        );
        let pinger = game.create_object_from_definition(&pinger_definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(pinger);
        activate(
            &mut game,
            pinger,
            activated(&pinger_definition),
            &mut Choices {
                targets: vec![Target::Player(C)],
                ..Default::default()
            },
        );
        game.move_object_by_game_rule(pinger, Zone::Graveyard)
            .unwrap();
        resolve(&mut game, &mut dm);
        assert_eq!(
            game.player(C).unwrap().life,
            10,
            "departed damage source uses its exact source LKI controller"
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.set_current_controller(source, A).unwrap();
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(C)),
            false,
            2,
        );
        assert_eq!(game.player(C).unwrap().life, 8);
    }
}

#[test]
fn quest_trigger_builds_counters_then_sacrifice_creates_an_independent_replacement() {
    for definition in definitions("Quest for Pure Flame") {
        let mut game = game();
        let mut dm = Choices::default();
        let quest = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = witness(&mut game, A);
        for _ in 0..4 {
            let outcome = damage(
                &mut game,
                source,
                ChooseSpec::Player(PlayerFilter::Specific(B)),
                false,
                1,
            );
            queue_outcome(&mut game, outcome, &mut dm);
            resolve_all(&mut game, &mut dm);
        }
        assert_eq!(
            game.object(quest)
                .unwrap()
                .counters
                .get(&ironsmith::CounterType::Quest)
                .copied(),
            Some(4)
        );
        activate(&mut game, quest, activated(&definition), &mut dm);
        assert!(game.object(quest).is_none(), "sacrificed during payment");
        resolve(&mut game, &mut dm);
        let victim = witness(&mut game, B);
        damage(
            &mut game,
            source,
            ChooseSpec::SpecificObject(victim),
            false,
            3,
        );
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(A)),
            true,
            2,
        );
        assert_eq!(
            (game.damage_on(victim), game.player(A).unwrap().life),
            (6, 16)
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        damage(
            &mut game,
            source,
            ChooseSpec::SpecificObject(victim),
            false,
            3,
        );
        assert_eq!(game.damage_on(victim), 3);
    }
}

#[test]
fn sawhorn_retains_the_actual_entry_choice_and_tracks_that_players_permanents() {
    for definition in definitions("Sawhorn Nemesis") {
        let mut game = game();
        let mut dm = Choices {
            player_name: Some("Charlie"),
            ..Default::default()
        };
        let nemesis = enter(&mut game, &definition, &mut dm);
        assert_eq!(game.chosen_player(nemesis), Some(C));
        let source = witness(&mut game, B);
        let victim = witness(&mut game, C);
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(C)),
            true,
            2,
        );
        damage(
            &mut game,
            source,
            ChooseSpec::SpecificObject(victim),
            false,
            2,
        );
        assert_eq!(
            (game.player(C).unwrap().life, game.damage_on(victim)),
            (16, 4)
        );
        game.set_current_controller(nemesis, C).unwrap();
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(A)),
            false,
            2,
        );
        assert_eq!(game.player(A).unwrap().life, 18);
        game.set_current_controller(victim, A).unwrap();
        damage(
            &mut game,
            source,
            ChooseSpec::SpecificObject(victim),
            false,
            2,
        );
        assert_eq!(game.damage_on(victim), 6);
    }
}

#[test]
fn rollercrusher_uses_live_delirium_and_announced_x_for_its_entry_targets_and_damage() {
    for definition in definitions("The Rollercrusher Ride") {
        let mut game = game();
        let first = witness(&mut game, B);
        let second = witness(&mut game, C);
        let mut dm = Choices {
            x: 3,
            targets: vec![Target::Object(first), Target::Object(second)],
            ..Default::default()
        };
        let ride = enter(&mut game, &definition, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!((game.damage_on(first), game.damage_on(second)), (3, 3));
        let source = witness(&mut game, A);
        let grave = game.create_object_from_definition(
            &body(
                "Four types",
                "Type: Artifact Enchantment Creature — Human
Power/Toughness: 1/1",
            ),
            A,
            Zone::Graveyard,
        );
        let land = game.create_object_from_definition(
            &body("Graveyard land", "Type: Land"),
            A,
            Zone::Graveyard,
        );
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            false,
            2,
        );
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            true,
            2,
        );
        assert_eq!(game.player(B).unwrap().life, 14);
        game.move_object_by_game_rule(land, Zone::Exile).unwrap();
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            false,
            2,
        );
        assert_eq!(game.player(B).unwrap().life, 12);
        game.set_current_controller(ride, B).unwrap();
        // Alice's three types remain irrelevant to the new controller.
        assert!(game.object(grave).is_some());
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            false,
            2,
        );
        assert_eq!(game.player(B).unwrap().life, 10);
    }
}

#[test]
fn recipient_chooses_order_with_prevention_and_multiple_multipliers_apply_once_each() {
    for definition in definitions("Goldnight Castigator") {
        for prevent_first in [false, true] {
            let mut game = game();
            let castigator = game.create_object_from_definition(&definition, B, Zone::Battlefield);
            let shield_source = witness(&mut game, B);
            let source = witness(&mut game, A);
            apply(
                &mut game,
                shield_source,
                Effect::prevent_damage(
                    2,
                    ChooseSpec::Player(PlayerFilter::Specific(B)),
                    Until::EndOfTurn,
                ),
            );
            let mut dm = Choices {
                replacement_source_first: Some(if prevent_first {
                    shield_source
                } else {
                    castigator
                }),
                ..Default::default()
            };
            execute_effect(
                &mut game,
                &Effect::deal_damage(3, ChooseSpec::Player(PlayerFilter::Specific(B))),
                &mut EffectContext::new(source, A, &mut dm),
            )
            .unwrap();
            assert_eq!(
                game.player(B).unwrap().life,
                if prevent_first { 18 } else { 16 }
            );
            assert!(!dm.replacements_chosen_by.is_empty());
            assert!(dm.replacements_chosen_by.iter().all(|p| *p == B));
        }
        let mut game = game();
        for _ in 0..2 {
            game.create_object_from_definition(&definition, B, Zone::Battlefield);
        }
        let source = witness(&mut game, A);
        damage(
            &mut game,
            source,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            false,
            2,
        );
        assert_eq!(game.player(B).unwrap().life, 12);
    }
}

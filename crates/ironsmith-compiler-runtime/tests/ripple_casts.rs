//! UNVALIDATED Ripple cast-resolution scenarios; authored, unrun.
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
    serde_json::from_str(include_str!("../../../fixtures/ripple_casts.json.fixture")).unwrap()
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
    ripple_casts: usize,
    ripple_label: Option<&'static str>,
    maximum_ripple_casts: Option<usize>,
    stop_ripple: bool,
    pending_ripple: bool,
    waiting: bool,
    invalid_ripple_selection: bool,
}
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool {
        self.waiting
    }

    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description == "Cast a revealed card with ripple, or stop" {
            if self.pending_ripple {
                self.waiting = true;
                return Vec::new();
            }
            if self.invalid_ripple_selection {
                return vec![0, 0];
            }
            if self.stop_ripple
                || self
                    .maximum_ripple_casts
                    .is_some_and(|n| self.ripple_casts >= n)
            {
                return Vec::new();
            }
            self.ripple_casts += 1;
            return vec![
                self.ripple_label
                    .and_then(|name| ctx.options.iter().find(|option| option.description == name))
                    .unwrap_or(&ctx.options[0])
                    .index,
            ];
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

fn attach(game: &mut GameState, aura: ObjectId, host: ObjectId) {
    assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
}

fn flush(game: &mut GameState, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
        if game.stack_is_empty() {
            return;
        }
        resolve(game, dm);
    }
    panic!("control trigger did not settle");
}
fn named(game: &GameState, name: &str) -> ObjectId {
    *game
        .battlefield
        .iter()
        .find(|id| game.object(**id).unwrap().name == name)
        .unwrap()
}
fn enter(
    game: &mut GameState,
    definition: &CardDefinition,
    owner: PlayerId,
    dm: &mut Choices,
) -> ObjectId {
    let old = game.create_object_from_definition(definition, owner, Zone::Hand);
    let receipt = game
        .move_object_with_etb_processing_with_dm(old, Zone::Battlefield, dm)
        .unwrap();
    assert!(!receipt.pending);
    assert!(
        receipt.programs.is_empty(),
        "do not discard entry additions"
    );
    receipt.original.into_result().unwrap().new_id
}

#[test]
fn five_frozen_keywords_retain_full_bodies_and_native_codec() {
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            let ripple=definition.abilities.iter().filter(|ability|matches!(&ability.kind,
            ironsmith::ability::AbilityKind::Triggered(trigger) if trigger.effects.all_effects().iter().any(|effect|effect.downcast_ref::<ironsmith::effects::RippleEffect>().is_some()))).count();
            assert_eq!(ripple, 1, "{}", row["name"]);
        }
    }
    let effect = Effect::new(ironsmith::effects::RippleEffect { amount: 4 });
    let encoded =
        ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(effect).unwrap();
    let encoded = serde_json::from_slice(&serde_json::to_vec(&encoded).unwrap()).unwrap();
    let materialized =
        ironsmith_runtime_catalog::artifact_materializer::materialize_effect(encoded).unwrap();
    assert_eq!(
        materialized
            .downcast_ref::<ironsmith::effects::RippleEffect>()
            .unwrap()
            .amount,
        4
    );
}
#[test]
fn recursive_ripple_creates_real_casts_without_paying_again_and_preserves_other_names() {
    for definition in definitions("Surging Flame") {
        let mut game = game();
        for n in 0..2 {
            game.create_object_from_definition(
                &resource(&format!("Nonmatching {n}"), "Artifact"),
                A,
                Zone::Library,
            );
        }
        for _ in 0..2 {
            game.create_object_from_definition(&definition, A, Zone::Library);
        }
        let mana = game.player(A).unwrap().mana_pool.total();
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        flush(&mut game, &mut dm);
        assert_eq!(dm.ripple_casts, 2);
        assert_eq!(game.player(B).unwrap().life, 14);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - 2);
        assert_eq!(
            game.player(A)
                .unwrap()
                .graveyard
                .iter()
                .filter(|id| game.object(**id).unwrap().name == "Surging Flame")
                .count(),
            3
        );
        assert_eq!(game.player(A).unwrap().library.len(), 2);
        assert!(
            game.player(A).unwrap().library.iter().all(|id| game
                .object(*id)
                .unwrap()
                .name
                .starts_with("Nonmatching"))
        );
    }
}
#[test]
fn declining_reveal_preserves_exact_order_and_each_printed_secondary_body_still_resolves() {
    for name in [
        "Surging Aether",
        "Surging Dementia",
        "Surging Flame",
        "Surging Might",
        "Surging Sentinels",
    ] {
        for definition in definitions(name) {
            let mut game = game();
            for n in 0..5 {
                game.create_object_from_definition(
                    &resource(&format!("Library {n}"), "Artifact"),
                    A,
                    Zone::Library,
                );
            }
            let before = game.player(A).unwrap().library.to_vec();
            let victim = game.create_object_from_definition(
                &vanilla("Victim", "{2}", "Bear", 2, 2),
                B,
                Zone::Battlefield,
            );
            let hand = game.create_object_from_definition(
                &resource("Discard me", "Artifact"),
                B,
                Zone::Hand,
            );
            let target = if matches!(name, "Surging Aether" | "Surging Might") {
                Target::Object(victim)
            } else {
                Target::Player(B)
            };
            let mut dm = Choices {
                targets: if name == "Surging Sentinels" {
                    Vec::new()
                } else {
                    vec![target]
                },
                decline: true,
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            flush(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().library.to_vec(), before);
            assert_eq!(dm.ripple_casts, 0);
            match name {
                "Surging Aether" => assert!(
                    game.player(B)
                        .unwrap()
                        .hand
                        .iter()
                        .any(|id| game.object(*id).unwrap().name == "Victim")
                ),
                "Surging Dementia" => {
                    assert!(!game.player(B).unwrap().hand.contains(&hand));
                    assert_eq!(game.player(B).unwrap().graveyard.len(), 1);
                }
                "Surging Flame" => assert_eq!(game.player(B).unwrap().life, 18),
                "Surging Might" => {
                    assert_eq!(pt(&game, victim), (4, 4));
                    assert_eq!(
                        game.object(named(&game, name)).unwrap().attached_to,
                        Some(ironsmith::object::AttachmentTarget::Object(victim))
                    );
                }
                "Surging Sentinels" => assert!(has(
                    &game,
                    named(&game, name),
                    ironsmith::static_abilities::StaticAbilityId::FirstStrike
                )),
                _ => unreachable!(),
            }
        }
    }
}
#[test]
fn optional_cast_can_stop_while_same_name_cards_remain_and_short_library_reveals_all() {
    for definition in definitions("Surging Sentinels") {
        let mut game = game();
        for _ in 0..3 {
            game.create_object_from_definition(&definition, A, Zone::Library);
        }
        let mut dm = Choices {
            maximum_ripple_casts: Some(1),
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        flush(&mut game, &mut dm);
        assert_eq!(dm.ripple_casts, 1);
        assert_eq!(game.player(A).unwrap().library.len(), 2);
        assert_eq!(
            game.battlefield
                .iter()
                .filter(|id| game.object(**id).unwrap().name == "Surging Sentinels")
                .count(),
            2
        );
    }
}
#[test]
fn pending_or_malformed_cast_choice_restores_reveal_and_all_prior_mutations() {
    for invalid in [false, true] {
        let definition = definitions("Surging Sentinels").into_iter().next().unwrap();
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let card = game.create_object_from_definition(&definition, A, Zone::Library);
        let order = game.player(A).unwrap().library.to_vec();
        let mut dm = Choices {
            pending_ripple: !invalid,
            invalid_ripple_selection: invalid,
            ..Default::default()
        };
        let mut ctx = EffectContext::new(source, A, &mut dm);
        let result = execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::RippleEffect { amount: 4 }),
            &mut ctx,
        );
        assert_eq!(result.is_err(), invalid);
        assert_eq!(game.player(A).unwrap().library.to_vec(), order);
        assert_eq!(game.object(card).unwrap().zone, Zone::Library);
        assert!(game.stack_is_empty());
        assert!(ctx.get_tagged_all("__ripple_revealed").is_none());
    }
}
#[test]
fn source_countered_before_ripple_uses_exact_last_known_name_and_multiple_instances_are_independent()
 {
    for definition in definitions_text(
        "Double Ripple",
        "Mana cost: {1}\nType: Creature — Wizard\nPower/Toughness: 1/1\nRipple 1\nRipple 1",
    ) {
        let mut game = game();
        game.create_object_from_definition(&resource("Remainder", "Artifact"), A, Zone::Library);
        let mut dm = Choices {
            stop_ripple: true,
            ..Default::default()
        };
        let spell = cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        assert_eq!(
            game.stack.iter().filter(|entry| entry.is_ability).count(),
            2
        );
        let outcome = apply(
            &mut game,
            spell,
            Effect::counter(ChooseSpec::SpecificObject(spell)),
        );
        queue_outcome(&mut game, outcome, &mut dm);
        flush(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().library.len(), 1);
        assert!(
            game.player(A)
                .unwrap()
                .graveyard
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Double Ripple")
        );
    }
}

#[test]
fn same_name_is_checked_on_revealed_card_before_choosing_either_split_half() {
    let mut front = definitions_text(
        "First Ripple",
        "Mana cost: {1}\nType: Sorcery\nRipple 1\nYou gain 1 life.",
    )
    .into_iter()
    .next()
    .unwrap();
    let back = definitions_text(
        "Other Half",
        "Mana cost: {7}{B}\nType: Sorcery\nYou gain 3 life.",
    )
    .into_iter()
    .next()
    .unwrap();
    front.card.other_face = Some(back.card.id);
    front.card.other_face_name = Some(back.card.name.clone());
    front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
    let mut game = game();
    game.register_linked_face_definition(&back);
    game.create_object_from_definition(&front, A, Zone::Library);
    let mut dm = Choices {
        ripple_label: Some("Cast Other Half"),
        ..Default::default()
    };
    let mana = game.player(A).unwrap().mana_pool.total();
    cast(&mut game, &front, CastingMethod::Normal, &mut dm);
    flush(&mut game, &mut dm);
    assert_eq!(dm.ripple_casts, 1);
    assert_eq!(game.player(A).unwrap().life, 24);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - 1);
}

#[test]
fn unavailable_first_split_face_does_not_remove_the_other_legal_face() {
    let mut front = definitions_text(
        "Ripple target half",
        "Mana cost: {1}\nType: Sorcery\nDestroy target artifact.",
    )
    .into_iter()
    .next()
    .unwrap();
    let back = definitions_text(
        "Legal other half",
        "Mana cost: {4}\nType: Sorcery\nYou gain 3 life.",
    )
    .into_iter()
    .next()
    .unwrap();
    front.card.other_face = Some(back.card.id);
    front.card.other_face_name = Some(back.card.name.clone());
    front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
    let mut game = game();
    game.register_linked_face_definition(&back);
    let source = game.create_object_from_definition(&front, A, Zone::Stack);
    game.create_object_from_definition(&front, A, Zone::Library);
    let mut dm = Choices::default();
    let outcome = execute_effect(
        &mut game,
        &Effect::new(ironsmith::effects::RippleEffect { amount: 1 }),
        &mut EffectContext::new(source, A, &mut dm),
    )
    .unwrap();
    assert_eq!(
        dm.ripple_casts, 2,
        "unavailable first half must leave the second option"
    );
    assert_eq!(outcome.explicit_objects().unwrap().len(), 1);
    flush(&mut game, &mut dm);
    assert_eq!(game.player(A).unwrap().life, 23);
    assert!(game.player(A).unwrap().library.is_empty());
}

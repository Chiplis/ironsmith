//! UNVALIDATED source-lifetime control; scenarios are authored, unrun.
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
        "../../../fixtures/source_lifetime_control.json.fixture"
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
    targets_explicit: bool,
    rejected_assignment: Vec<Target>,
    rejection_checks: usize,
    objects: Vec<ObjectId>,
    objects_explicit: bool,
    x: u32,
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
        if self.targets_explicit || !self.targets.is_empty() {
            assert!(ironsmith::targeting::validate_flat_target_assignment(
                &context.requirements,
                &self.targets
            ));
            if !self.rejected_assignment.is_empty() {
                assert!(!ironsmith::targeting::validate_flat_target_assignment(
                    &context.requirements,
                    &self.rejected_assignment
                ));
                self.rejection_checks += 1;
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
fn six_frozen_complete_bodies_are_strict_and_round_trip() {
    let rows = fixtures()
        .into_iter()
        .filter(|row| row["proposed_complete"] == true)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 7);
    for row in rows {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"]);
            assert!(
                format!("{definition:?}").contains("ObjectOnBattlefield"),
                "{}: {definition:?}",
                definition.card.name
            );
        }
    }
}
#[test]
fn sower_source_control_changes_do_not_change_the_beneficiary_and_phasing_latches_expiry() {
    for definition in definitions("Sower of Temptation") {
        let mut game = game();
        let victim = game.create_object_from_definition(
            &vanilla("Victim", "{2}", "Bear", 2, 2),
            B,
            Zone::Battlefield,
        );
        let mut dm = Choices {
            targets: vec![Target::Object(victim)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        flush(&mut game, &mut dm);
        let sower = named(&game, "Sower of Temptation");
        assert_eq!(game.current_controller(victim), Some(A));
        assert!(has(
            &game,
            sower,
            ironsmith::static_abilities::StaticAbilityId::Flying
        ));
        let mut first = SelectFirstDecisionMaker;
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::GainControlEffect::permanent(
                ChooseSpec::SpecificObject(sower),
            )),
            &mut EffectContext::new(sower, C, &mut first),
        )
        .unwrap();
        assert_eq!(game.current_controller(sower), Some(C));
        assert_eq!(game.current_controller(victim), Some(A));
        game.phase_out(sower);
        assert_eq!(game.current_controller(victim), Some(B));
        game.phase_in(sower);
        assert_eq!(
            game.current_controller(victim),
            Some(B),
            "expired duration never restarts"
        );
    }
}
#[test]
fn departed_or_blinked_sower_before_resolution_never_starts_the_duration() {
    for definition in definitions("Sower of Temptation") {
        for blink in [false, true] {
            let mut game = game();
            let victim = game.create_object_from_definition(
                &vanilla("Victim", "{2}", "Bear", 2, 2),
                B,
                Zone::Battlefield,
            );
            let mut dm = Choices {
                targets: vec![Target::Object(victim)],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve(&mut game, &mut dm);
            put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
            assert_eq!(game.stack.len(), 1);
            let sower = named(&game, "Sower of Temptation");
            let departed = game.move_object_by_effect(sower, Zone::Exile).unwrap();
            if blink {
                let replacement = game
                    .move_object_by_effect(departed, Zone::Battlefield)
                    .unwrap();
                assert_ne!(replacement, sower);
            }
            resolve(&mut game, &mut dm);
            assert_eq!(game.current_controller(victim), Some(B));
        }
    }
}
#[test]
fn giants_grasp_enchant_restriction_and_lifetime_are_owned_by_the_aura() {
    for definition in definitions("Giant's Grasp") {
        let mut game = game();
        let host = game.create_object_from_definition(
            &vanilla("Giant host", "{3}", "Giant", 3, 3),
            A,
            Zone::Battlefield,
        );
        let victim = game.create_object_from_definition(
            &resource("Enemy relic", "Artifact"),
            B,
            Zone::Battlefield,
        );
        let mut dm = Choices {
            targets: vec![Target::Object(host)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve(&mut game, &mut dm);
        let aura = named(&game, "Giant's Grasp");
        assert_eq!(
            game.object(aura).unwrap().attached_to,
            Some(ironsmith::object::AttachmentTarget::Object(host))
        );
        dm.targets = vec![Target::Object(victim)];
        flush(&mut game, &mut dm);
        assert_eq!(game.current_controller(victim), Some(A));
        game.phase_out(host);
        assert!(game.is_phased_out(aura));
        assert_eq!(game.current_controller(victim), Some(B));
        game.phase_in(host);
        assert_eq!(game.current_controller(victim), Some(B));
    }
}
#[test]
fn charisma_uses_the_damaged_creature_and_original_aura_even_after_reattachment() {
    for definition in definitions("Charisma") {
        let mut game = game();
        let host = game.create_object_from_definition(
            &vanilla("Enchanted dealer", "{2}", "Wizard", 2, 4),
            A,
            Zone::Battlefield,
        );
        let next = game.create_object_from_definition(
            &vanilla("New host", "{2}", "Wizard", 2, 4),
            A,
            Zone::Battlefield,
        );
        let victim = game.create_object_from_definition(
            &vanilla("Damaged victim", "{2}", "Bear", 2, 4),
            B,
            Zone::Battlefield,
        );
        let mut dm = Choices {
            targets: vec![Target::Object(host)],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        flush(&mut game, &mut dm);
        let aura = named(&game, "Charisma");
        let outcome = apply(
            &mut game,
            host,
            Effect::deal_damage(1, ChooseSpec::SpecificObject(victim)),
        );
        dm.targets.clear();
        queue_outcome(&mut game, outcome, &mut dm);
        flush(&mut game, &mut dm);
        assert_eq!(game.current_controller(victim), Some(A));
        attach(&mut game, aura, next);
        assert_eq!(game.current_controller(victim), Some(A));
        game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
        assert_eq!(game.current_controller(victim), Some(B));
    }
}
#[test]
fn cytoplast_graft_entry_and_paid_activation_keep_the_counter_target_requirement() {
    use ironsmith::object::CounterType;
    for definition in definitions("Cytoplast Manipulator") {
        let mut game = game();
        let mut dm = Choices::default();
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        flush(&mut game, &mut dm);
        let source = named(&game, "Cytoplast Manipulator");
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&2)
        );
        let victim = enter(
            &mut game,
            &vanilla("Grafted victim", "{2}", "Bear", 2, 2),
            B,
            &mut dm,
        );
        flush(&mut game, &mut dm);
        assert_eq!(
            game.object(victim)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&1)
        );
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&1)
        );
        game.remove_summoning_sickness(source);
        dm.targets = vec![Target::Object(victim)];
        let index = activated_at(&definition, 0);
        let mana = game.player(A).unwrap().mana_pool.total();
        activate(&mut game, source, index, &mut dm);
        assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - 1);
        resolve(&mut game, &mut dm);
        assert_eq!(game.current_controller(victim), Some(A));
        game.remove_counters(
            victim,
            CounterType::PlusOnePlusOne,
            1,
            Some(source),
            Some(A),
        );
        assert_eq!(
            game.current_controller(victim),
            Some(A),
            "the counter is a target restriction, not the duration"
        );
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(game.current_controller(victim), Some(B));
    }
}
#[test]
fn scarwood_opponent_payment_prevents_control_and_nonpayment_preserves_exact_source_duration() {
    for definition in definitions("Scarwood Bandits") {
        for pay in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            let victim = game.create_object_from_definition(
                &resource("Contested artifact", "Artifact"),
                B,
                Zone::Battlefield,
            );
            if pay {
                game.player_mut(B)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 2);
            }
            let mut dm = Choices {
                targets: vec![Target::Object(victim)],
                ..Default::default()
            };
            let mana = game.player(A).unwrap().mana_pool.total();
            activate(&mut game, source, activated_at(&definition, 0), &mut dm);
            assert!(game.is_tapped(source));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - 3);
            resolve(&mut game, &mut dm);
            assert_eq!(
                game.current_controller(victim),
                Some(if pay { B } else { A })
            );
            if pay {
                assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
            }
            game.phase_out(source);
            assert_eq!(game.current_controller(victim), Some(B));
            game.phase_in(source);
            assert_eq!(game.current_controller(victim), Some(B));
        }
    }
}

#[test]
fn common_grant_and_leading_duration_paths_expire_on_phase_out_but_literal_until_leaves_does_not() {
    for text in [
        "Mana cost: {1}\nType: Artifact\n{T}: Target creature gains flying for as long as this artifact remains on the battlefield.",
        "Mana cost: {1}\nType: Artifact\n{T}: For as long as this artifact remains on the battlefield, target creature gains flying.",
    ] {
        for definition in definitions_text("Duration source", text) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let victim = game.create_object_from_definition(
                &vanilla("Grant recipient", "{2}", "Bear", 2, 2),
                B,
                Zone::Battlefield,
            );
            let mut dm = Choices {
                targets: vec![Target::Object(victim)],
                ..Default::default()
            };
            activate(&mut game, source, 0, &mut dm);
            resolve(&mut game, &mut dm);
            assert!(has(
                &game,
                victim,
                ironsmith::static_abilities::StaticAbilityId::Flying
            ));
            game.phase_out(source);
            assert!(!has(
                &game,
                victim,
                ironsmith::static_abilities::StaticAbilityId::Flying
            ));
            game.phase_in(source);
            assert!(!has(
                &game,
                victim,
                ironsmith::static_abilities::StaticAbilityId::Flying
            ));
            apply(
                &mut game,
                source,
                Effect::new(ironsmith::effects::GainControlEffect::new(
                    ChooseSpec::SpecificObject(victim),
                    Until::ThisLeavesTheBattlefield,
                )),
            );
            assert_eq!(game.current_controller(victim), Some(A));
            game.phase_out(source);
            assert_eq!(
                game.current_controller(victim),
                Some(A),
                "literal until-leaves is not a visible-state predicate"
            );
            game.phase_in(source);
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            assert_eq!(game.current_controller(victim), Some(B));
        }
    }
}

#[test]
fn akroan_all_chapters_keep_source_lifetime_attack_scope_and_zipped_self_damage() {
    fn visit(effect: &Effect, owners: &mut Vec<ironsmith_core::DealDamageBySourcesEffect>) {
        if let Some(damage) = effect.downcast_ref::<ironsmith::effects::DealDamageBySourcesEffect>()
        {
            owners.push(damage.clone());
        }
        if let Some(each) = effect.downcast_ref::<ironsmith::effects::ForEachObject>() {
            assert!(
                !format!("{:?}", each.effects).contains("DealDamage"),
                "self damage must not retain an enclosing serial loop"
            );
        }
        effect.visit_child_effects(&mut |child| visit(child, owners));
    }
    for definition in definitions("The Akroan War") {
        let mut owners = Vec::new();
        for ability in &definition.abilities {
            if let ironsmith::ability::AbilityKind::Triggered(trigger) = &ability.kind {
                for segment in &trigger.effects.segments {
                    for effect in &segment.default_effects {
                        visit(effect, &mut owners);
                    }
                }
            }
        }
        assert_eq!(owners.len(), 1);
        assert_eq!(
            owners[0].recipient_binding,
            ironsmith_core::DamageRecipientSetBinding::EachSource
        );
        assert!(matches!(owners[0].sources.as_slice(), [source]
            if matches!(source.base(), ChooseSpec::All(filter) if filter.tapped)));
        assert!(
            definition
                .canonical_text
                .contains("deals damage to itself equal to its power"),
            "{}",
            definition.canonical_text
        );
        for additions in [false, true] {
            let mut game = game();
            let victim = game.create_object_from_definition(
                &vanilla("Stolen subject", "{2}", "Bear", 1, 50),
                B,
                Zone::Battlefield,
            );
            let first = game.create_object_from_definition(
                &compile_to_runtime_definition(
                    "Lifelink self source",
                    "Type: Creature — Human\nPower/Toughness: 3/50\nLifelink",
                    false,
                )
                .unwrap(),
                A,
                Zone::Battlefield,
            );
            let second = game.create_object_from_definition(&compile_to_runtime_definition(
                "Life-sized self source", "Type: Creature — Human\nPower/Toughness: */*\nThis creature's power and toughness are each equal to your life total.", false).unwrap(), A, Zone::Battlefield);
            let opponent = game.create_object_from_definition(
                &vanilla("Opponent self source", "{2}", "Bear", 5, 50),
                B,
                Zone::Battlefield,
            );
            let untapped = game.create_object_from_definition(
                &vanilla("Untapped excluded", "{2}", "Bear", 17, 50),
                C,
                Zone::Battlefield,
            );
            let mut dm = Choices {
                targets: vec![Target::Object(victim)],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            flush(&mut game, &mut dm);
            let saga = named(&game, "The Akroan War");
            assert_eq!(game.current_controller(victim), Some(A));
            assert_eq!(
                game.counter_count(saga, ironsmith::object::CounterType::Lore),
                1
            );
            lore(&mut game, saga, &mut Choices::default());
            for id in [opponent, untapped] {
                assert!(ironsmith::rules::combat::must_attack_with_game(
                    game.object(id).unwrap(),
                    &game
                ));
            }
            for id in [first, second, victim] {
                assert!(!ironsmith::rules::combat::must_attack_with_game(
                    game.object(id).unwrap(),
                    &game
                ));
            }
            for id in [first, second, opponent] {
                apply(&mut game, saga, Effect::tap(ChooseSpec::SpecificObject(id)));
            }
            if additions {
                game.effect_store.replacement_effects.add_one_shot_effect(
                    ironsmith::replacement::ReplacementEffect::with_matcher(
                        saga,
                        A,
                        ironsmith::events::damage::matchers::DamageToObjectMatcher::new(
                            ironsmith::target::ObjectFilter::specific(first),
                        ),
                        ironsmith::replacement::ReplacementAction::Additionally(vec![
                            Effect::pump(
                                10,
                                10,
                                ChooseSpec::SpecificObject(second),
                                Until::EndOfTurn,
                            ),
                            Effect::tap(ChooseSpec::SpecificObject(untapped)),
                        ]),
                    ),
                );
            }
            lore(&mut game, saga, &mut Choices::default());
            assert_eq!(game.damage_on(first), 3);
            assert_eq!(
                game.damage_on(second),
                20,
                "all powers precede lifelink or replacement additions"
            );
            assert_eq!(game.damage_on(opponent), 5);
            assert_eq!(game.damage_on(victim), 0);
            assert_eq!(
                game.damage_on(untapped),
                0,
                "the tapped source set was captured once"
            );
            assert_eq!(game.player(A).unwrap().life, 23);
            assert_eq!(game.player(B).unwrap().life, 20);
            let damage = game
                .turn_store
                .turn_history
                .event_records
                .iter()
                .chain(game.turn_store.turn_history.staged_event_records.iter())
                .filter(|record| {
                    record
                        .event
                        .downcast::<ironsmith::events::DamageEvent>()
                        .is_some()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                damage.len(),
                3,
                "no Cartesian cross-damage or duplicate receipts"
            );
            let batch = damage[0].event.simultaneous_batch();
            assert!(batch.is_some());
            for record in damage {
                let event = record
                    .event
                    .downcast::<ironsmith::events::DamageEvent>()
                    .unwrap();
                assert_eq!(
                    event.target,
                    ironsmith::events::DamageTarget::Object(event.source)
                );
                assert_eq!(record.event.simultaneous_batch(), batch);
            }
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut queue, &mut dm)
                .unwrap();
            assert!(
                !game.battlefield.contains(&saga),
                "the final chapter releases the Saga"
            );
            assert_eq!(
                game.current_controller(victim),
                Some(B),
                "source-bound chapter I expires"
            );
            assert!(
                ironsmith::rules::combat::must_attack_with_game(
                    game.object(opponent).unwrap(),
                    &game
                ),
                "chapter II has its own duration, independent of Saga departure"
            );
            for expected in [B, C, A] {
                game.next_turn();
                assert_eq!(game.turn.active_player, expected);
                assert_eq!(
                    ironsmith::rules::combat::must_attack_with_game(
                        game.object(opponent).unwrap(),
                        &game
                    ),
                    expected != A
                );
            }
        }
    }
}

#[test]
fn akroan_attack_rule_tracks_new_creatures_and_current_control_and_survives_ability_loss() {
    for definition in definitions("The Akroan War") {
        let mut game = game();
        let victim = game.create_object_from_definition(
            &vanilla("Chapter I target", "{2}", "Bear", 1, 50),
            B,
            Zone::Battlefield,
        );
        let incoming = game.create_object_from_definition(
            &vanilla("Future opponent", "{2}", "Bear", 2, 50),
            A,
            Zone::Battlefield,
        );
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        lore(
            &mut game,
            source,
            &mut Choices {
                targets: vec![Target::Object(victim)],
                ..Default::default()
            },
        );
        lore(&mut game, source, &mut Choices::default());
        let entrant = enter(
            &mut game,
            &vanilla("Later entrant", "{2}", "Bear", 3, 50),
            B,
            &mut Choices::default(),
        );
        assert!(
            ironsmith::rules::combat::must_attack_with_game(game.object(entrant).unwrap(), &game),
            "later entrants satisfy the rule's live filter"
        );
        assert!(!ironsmith::rules::combat::must_attack_with_game(
            game.object(incoming).unwrap(),
            &game
        ));
        let gain = Effect::new(ironsmith::effects::GainControlEffect::permanent(
            ChooseSpec::SpecificObject(incoming),
        ));
        execute_effect(
            &mut game,
            &gain,
            &mut EffectContext::new(source, B, &mut SelectFirstDecisionMaker),
        )
        .unwrap();
        assert!(ironsmith::rules::combat::must_attack_with_game(
            game.object(incoming).unwrap(),
            &game
        ));
        apply(
            &mut game,
            source,
            Effect::new(ironsmith::effects::GainControlEffect::permanent(
                ChooseSpec::SpecificObject(entrant),
            )),
        );
        assert!(
            !ironsmith::rules::combat::must_attack_with_game(game.object(entrant).unwrap(), &game),
            "leaving the opposing-controller set releases the requirement"
        );
        apply(
            &mut game,
            source,
            Effect::new(ironsmith::effects::ApplyContinuousEffect::new(
                ironsmith::continuous::EffectTarget::Specific(incoming),
                ironsmith::continuous::Modification::RemoveAllAbilities,
                Until::Forever,
            )),
        );
        assert!(!has(
            &game,
            incoming,
            ironsmith::static_abilities::StaticAbilityId::MustAttack
        ));
        assert!(
            ironsmith::rules::combat::must_attack_with_game(game.object(incoming).unwrap(), &game),
            "losing abilities cannot erase a resolving combat rule"
        );
        apply(
            &mut game,
            source,
            Effect::exile(ChooseSpec::SpecificObject(source)),
        );
        assert_eq!(game.current_controller(victim), Some(B));
        game.next_turn();
        assert_eq!(game.turn.active_player, B);
        let options = ironsmith::decision::compute_legal_attackers(
            &game,
            &ironsmith::combat_state::CombatState::default(),
        );
        assert!(
            options
                .iter()
                .any(|option| option.creature == incoming && option.must_attack)
        );
        assert!(
            ironsmith::game_loop::apply_attacker_declarations(
                &mut game,
                &mut ironsmith::combat_state::CombatState::default(),
                &mut TriggerQueue::new(),
                &[]
            )
            .is_err(),
            "the declaration optimizer must enforce the actual rule, not just expose a UI hint"
        );
        game.next_turn();
        game.next_turn();
        assert_eq!(game.turn.active_player, A);
        assert!(!ironsmith::rules::combat::must_attack_with_game(
            game.object(incoming).unwrap(),
            &game
        ));
    }
}

fn next_saga_chapter(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::add_lore_counter_and_check_chapters(game, source, &mut queue).unwrap();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    flush(game, dm);
}
#[test]
fn super_hero_civil_war_full_saga_keeps_aggregate_targets_frozen_pump_and_optional_fight() {
    for definition in definitions("The Super Hero Civil War") {
        let mut game = game();
        let own = game.create_object_from_definition(
            &vanilla("Own fighter", "{3}", "Warrior", 2, 8),
            A,
            Zone::Battlefield,
        );
        let four = game.create_object_from_definition(
            &vanilla("Four mana", "{4}", "Bear", 2, 8),
            B,
            Zone::Battlefield,
        );
        let two = game.create_object_from_definition(
            &vanilla("Two mana", "{2}", "Bear", 2, 8),
            C,
            Zone::Battlefield,
        );
        let three = game.create_object_from_definition(
            &vanilla("Over-budget alternative", "{3}", "Bear", 1, 8),
            B,
            Zone::Battlefield,
        );
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut dm = Choices {
            targets: vec![Target::Object(four), Target::Object(two)],
            rejected_assignment: vec![Target::Object(four), Target::Object(three)],
            ..Default::default()
        };
        next_saga_chapter(&mut game, source, &mut dm);
        assert_eq!(dm.rejection_checks, 1);
        assert_eq!(game.current_controller(four), Some(A));
        assert_eq!(game.current_controller(two), Some(A));
        dm.targets.clear();
        dm.rejected_assignment.clear();
        next_saga_chapter(&mut game, source, &mut dm);
        for id in [own, four, two] {
            assert_eq!(pt(&game, id), (3, 9));
            assert!(has(
                &game,
                id,
                ironsmith::static_abilities::StaticAbilityId::Vigilance
            ));
        }
        let later = game.create_object_from_definition(
            &vanilla("Late own entrant", "{1}", "Bear", 1, 3),
            A,
            Zone::Battlefield,
        );
        assert_eq!(pt(&game, later), (1, 3));
        assert!(!has(
            &game,
            later,
            ironsmith::static_abilities::StaticAbilityId::Vigilance
        ));
        dm.targets = vec![Target::Object(own), Target::Object(three)];
        next_saga_chapter(&mut game, source, &mut dm);
        assert_eq!(game.damage_on(own), 1);
        assert_eq!(game.damage_on(three), 3);
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut TriggerQueue::new()).unwrap();
        assert!(game.object(source).is_none());
        assert_eq!(game.current_controller(four), Some(B));
        assert_eq!(game.current_controller(two), Some(C));
        assert_eq!(
            pt(&game, four),
            (3, 9),
            "chapterII's locked pump does not end with its source"
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(pt(&game, four), (2, 8));
    }
}
#[test]
fn super_hero_civil_war_can_take_no_creature_then_decline_its_optional_fight_opponent() {
    for definition in definitions("The Super Hero Civil War") {
        let mut game = game();
        let own = game.create_object_from_definition(
            &vanilla("Only fighter", "{1}", "Bear", 2, 8),
            A,
            Zone::Battlefield,
        );
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut dm = Choices {
            targets_explicit: true,
            ..Default::default()
        };
        next_saga_chapter(&mut game, source, &mut dm);
        dm.targets_explicit = false;
        next_saga_chapter(&mut game, source, &mut dm);
        dm.targets = vec![Target::Object(own)];
        next_saga_chapter(&mut game, source, &mut dm);
        assert_eq!(game.damage_on(own), 0);
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut TriggerQueue::new()).unwrap();
        assert!(game.object(source).is_none());
    }
}

#[test]
fn control_saga_revalidates_the_whole_current_mana_value_group_without_choosing_a_subset() {
    for definition in definitions("The Super Hero Civil War") {
        for case in 0..5 {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let first = game.create_object_from_definition(
                &vanilla("First target", "{4}", "Bear", 2, 8),
                B,
                Zone::Battlefield,
            );
            let second = game.create_object_from_definition(
                &vanilla("Second target", "{2}", "Bear", 2, 8),
                C,
                Zone::Battlefield,
            );
            let mut dm = Choices {
                targets: if case == 0 {
                    vec![Target::Object(first)]
                } else {
                    vec![Target::Object(first), Target::Object(second)]
                },
                ..Default::default()
            };
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::add_lore_counter_and_check_chapters(
                &mut game, source, &mut queue,
            )
            .unwrap();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            if case == 2 {
                game.move_object_by_effect(first, Zone::Exile).unwrap();
            } else {
                let donor = game.create_object_from_definition(
                    &vanilla(
                        "Copied characteristics",
                        if case == 0 { "{7}" } else { "{5}" },
                        "Bear",
                        2,
                        8,
                    ),
                    B,
                    Zone::Battlefield,
                );
                let copy = ironsmith::effects::ApplyContinuousEffect::new_runtime(
                    ironsmith::continuous::EffectTarget::Specific(first),
                    ironsmith::effects::continuous::RuntimeModification::CopyOf {
                        source: ChooseSpec::SpecificObject(donor),
                        preserve_source_abilities: false,
                        name_override: None,
                        name_override_surface: None,
                        add_supertypes: Vec::new(),
                        copy_exception_surface: None,
                    },
                    Until::EndOfTurn,
                );
                apply(&mut game, source, Effect::new(copy));
                assert_eq!(
                    game.current_characteristics(first)
                        .unwrap()
                        .mana_cost
                        .unwrap()
                        .mana_value(),
                    if case == 0 { 7 } else { 5 }
                );
                if case == 3 {
                    game.move_object_by_effect(first, Zone::Exile).unwrap();
                }
                if case == 4 {
                    apply(
                        &mut game,
                        source,
                        Effect::new(ironsmith::effects::ApplyContinuousEffect::new(
                            ironsmith::continuous::EffectTarget::Specific(first),
                            ironsmith::continuous::Modification::AddAbility(
                                ironsmith::static_abilities::StaticAbility::shroud(),
                            ),
                            Until::EndOfTurn,
                        )),
                    );
                }
            }
            resolve(&mut game, &mut dm);
            if case != 2 && case != 3 {
                assert_eq!(game.current_controller(first), Some(B));
            }
            assert_eq!(
                game.current_controller(second),
                Some(if case == 2 { A } else { C })
            );
        }
    }
}

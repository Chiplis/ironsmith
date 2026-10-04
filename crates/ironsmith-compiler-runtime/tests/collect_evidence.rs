//! UNVALIDATED implementation-first coverage for the Collect Evidence family.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, ColorsContext, NumberContext, SelectObjectsContext, TargetsContext,
};
use ironsmith::effect::{Effect, Value};
use ironsmith::effects::{CollectEvidenceEffect, EffectContext, execute_effect};
use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    Ability, CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone,
};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/collect_evidence.json.fixture"
    ))
    .unwrap()
}
fn round_trip(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let f = fixtures().into_iter().find(|f| f["name"] == name).unwrap();
    round_trip(name, f["text"].as_str().unwrap())
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let card = CardBuilder::new(CardId::new(), "Library filler")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..12 {
        game.create_object_from_card(&card, A, Zone::Library);
    }
    game
}
fn evidence(game: &mut GameState, owner: PlayerId, name: &str, value: u8, zone: Zone) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(value)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&card, owner, zone)
}
fn contains_collection(effect: &Effect) -> bool {
    if effect.downcast_ref::<CollectEvidenceEffect>().is_some() {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| found |= contains_collection(child));
    found
}
#[derive(Default)]
struct Choices {
    selected: Vec<ObjectId>,
    excluded: Vec<ObjectId>,
    accept: bool,
    may_prompts: usize,
    x: u32,
    target: Option<Target>,
    suspend: bool,
    pending: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.may_prompts += 1;
        self.accept
    }
    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        assert!(ctx.is_x_value);
        assert!(self.x <= ctx.max);
        self.x
    }
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        for id in &self.excluded {
            assert!(!ctx.candidates.iter().any(|c| c.legal && c.id == *id));
        }
        if self.suspend {
            self.pending = true;
            return vec![];
        }
        for id in &self.selected {
            assert!(ctx.candidates.iter().any(|c| c.legal && c.id == *id));
        }
        self.selected.clone()
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert!(
                ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(&target))
            );
            vec![target]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, ctx)
        }
    }
    fn decide_colors(&mut self, _game: &GameState, ctx: &ColorsContext) -> Vec<ironsmith::Color> {
        vec![ironsmith::Color::Blue; ctx.count as usize]
    }
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}
fn activate(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let index = game.current_abilities(source).unwrap().iter().position(|ability| matches!(&ability.kind,
        AbilityKind::Activated(a) if a.mana_cost.costs().iter().filter_map(|cost| cost.effect_ref()).any(contains_collection))).unwrap();
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|a| matches!(a,
        LegalAction::ActivateAbility { source: s, ability_index: i } | LegalAction::ActivateManaAbility { source: s, ability_index: i } if *s == source && *i == index)).unwrap();
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
    for _ in 0..40 {
        if state.pending_activation.is_none() && state.pending_mana_ability.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("unfinished evidence payment: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none() && state.pending_mana_ability.is_none());
}
fn event_amounts(game: &GameState) -> Vec<u32> {
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .filter_map(|r| r.event.downcast::<KeywordActionEvent>())
        .filter(|e| e.action == KeywordActionKind::CollectEvidence && e.player == A)
        .map(|e| e.amount)
        .collect()
}

#[test]
fn frozen_collect_evidence_programs_have_typed_actions_after_artifact_transport() {
    let cards = fixtures();
    assert_eq!(cards.len(), 14);
    assert_eq!(
        cards.iter().filter(|c| c["mechanic_root"] == true).count(),
        13
    );
    assert_eq!(
        cards
            .iter()
            .filter(|c| c["proposed_coverage"] == "complete")
            .count(),
        11
    );
    for card in cards
        .into_iter()
        .filter(|c| c["proposed_coverage"] == "complete")
    {
        for definition in definitions(card["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert!(
                definition
                    .abilities
                    .iter()
                    .any(|ability| match &ability.kind {
                        AbilityKind::Activated(a) =>
                            a.mana_cost
                                .costs()
                                .iter()
                                .filter_map(|cost| cost.effect_ref())
                                .any(contains_collection)
                                || a.effects.all_effects().into_iter().any(contains_collection),
                        AbilityKind::Triggered(t) =>
                            t.effects.all_effects().into_iter().any(contains_collection),
                        _ => false,
                    }),
                "{} must retain an executable evidence action",
                definition.card.name
            );
        }
    }
}

#[test]
fn cryptex_pays_total_mana_value_from_its_payers_graveyard_and_emits_one_event() {
    for definition in definitions("Cryptex") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let one = evidence(&mut game, A, "One", 1, Zone::Graveyard);
        let four = evidence(&mut game, A, "Four", 4, Zone::Graveyard);
        let foreign = evidence(&mut game, B, "Foreign", 9, Zone::Graveyard);
        let hand = evidence(&mut game, A, "Hand", 9, Zone::Hand);
        let mut dm = Choices {
            selected: vec![one, four],
            excluded: vec![foreign, hand],
            ..Default::default()
        };
        activate(&mut game, source, &mut dm);
        assert!(game.stack_is_empty());
        assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().mana_pool.blue, 1);
        assert_eq!(
            game.counter_count(source, CounterType::Named("unlock".into())),
            1
        );
        assert_eq!(game.player(A).unwrap().graveyard.len(), 0);
        assert_eq!(game.object(foreign).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(hand).unwrap().zone, Zone::Hand);
        assert_eq!(game.exile.len(), 2);
        assert_eq!(
            event_amounts(&game),
            vec![3],
            "threshold, not overpaid total or card count"
        );
    }
}

#[test]
fn zero_collection_is_performed_for_if_you_do_and_triggers_real_observers() {
    for monitor in definitions("Surveillance Monitor") {
        let mut game = game();
        let source = game.create_object_from_definition(&monitor, A, Zone::Battlefield);
        let mut dm = Choices {
            accept: true,
            ..Default::default()
        };
        let program = Effect::may_if_do(
            0,
            Effect::new(CollectEvidenceEffect::new(0)),
            vec![Effect::draw(1)],
        );
        let hand_before = game.player(A).unwrap().hand.len();
        let mut events = Vec::new();
        {
            let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut dm);
            for effect in &program {
                events.extend(execute_effect(&mut game, effect, &mut ctx).unwrap().events);
            }
        }
        assert_eq!(game.player(A).unwrap().hand.len(), hand_before + 1);
        let keyword: Vec<_> = events
            .iter()
            .filter_map(|e| e.downcast::<KeywordActionEvent>())
            .filter(|e| e.action == KeywordActionKind::CollectEvidence)
            .collect();
        assert_eq!(keyword.len(), 1);
        assert_eq!(keyword[0].amount, 0);
        for event in events {
            game.queue_trigger_event(Default::default(), event);
        }
        let mut queue = TriggerQueue::new();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|o| o.subtypes.contains(&ironsmith::Subtype::Thopter))
        }));
        assert!(game.exile.is_empty());
    }
}

#[test]
fn unavailable_optional_evidence_is_not_offered_and_cost_preflight_is_side_effect_free() {
    use ironsmith::costs::{Cost, CostContext, PaymentReason};
    let mut game = game();
    let source = evidence(&mut game, A, "Cast source", 10, Zone::Graveyard);
    let other = evidence(&mut game, A, "Other evidence", 3, Zone::Graveyard);
    let mut dm = Choices {
        accept: true,
        ..Default::default()
    };
    let cost = Cost::try_effect(Effect::new(CollectEvidenceEffect::new(4))).unwrap();
    let mana = game.player(A).unwrap().mana_pool.clone();
    let life = game.player(A).unwrap().life;
    {
        let ctx = CostContext::new(source, A, &mut dm).with_reason(PaymentReason::CastSpell);
        assert!(
            cost.can_pay(&game, &ctx).is_err(),
            "a spell cannot supply its own casting cost from its former graveyard position"
        );
    }
    assert_eq!(game.player(A).unwrap().graveyard, vec![source, other]);
    assert_eq!(game.player(A).unwrap().mana_pool, mana);
    assert_eq!(game.player(A).unwrap().life, life);
    game.move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let mut ctx = EffectContext::new_default(other, A).with_decision_maker(&mut dm);
    let result = execute_effect(
        &mut game,
        &Effect::may_single(Effect::new(CollectEvidenceEffect::new(4))),
        &mut ctx,
    )
    .unwrap();
    assert_eq!(result.status, ironsmith::effect::OutcomeStatus::Declined);
    drop(ctx);
    assert_eq!(dm.may_prompts, 0);
    assert_eq!(game.object(other).unwrap().zone, Zone::Graveyard);
    assert!(game.exile.is_empty());
    let x_cost = Cost::try_effect(Effect::new(CollectEvidenceEffect::new(Value::X))).unwrap();
    let mut cost_ctx = CostContext::new(other, A, &mut dm);
    cost_ctx.x_value = Some(4);
    assert!(
        x_cost.can_pay(&game, &cost_ctx).is_err(),
        "announced X is checked against exact available value"
    );
    cost_ctx.x_value = Some(3);
    assert!(x_cost.can_pay(&game, &cost_ctx).is_ok());
    assert_eq!(game.object(other).unwrap().zone, Zone::Graveyard);
}

#[test]
fn pending_collection_restores_x_tags_and_graveyard_before_replay() {
    let mut game = game();
    let source = evidence(&mut game, A, "Source", 0, Zone::Battlefield);
    let card = evidence(&mut game, A, "Evidence", 7, Zone::Graveyard);
    let effect = Effect::may(vec![
        Effect::gain_life(2),
        Effect::new(CollectEvidenceEffect::new(Value::X)),
    ]);
    let mut dm = Choices {
        selected: vec![card],
        accept: true,
        x: 3,
        suspend: true,
        ..Default::default()
    };
    {
        let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut dm);
        execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(ctx.x_value, None);
        assert!(ctx.tagged_objects.is_empty());
    }
    assert!(dm.pending);
    assert_eq!(
        game.player(A).unwrap().life,
        20,
        "the enclosing optional action rolls back an earlier child"
    );
    assert_eq!(game.object(card).unwrap().zone, Zone::Graveyard);
    assert!(game.exile.is_empty());
    dm.pending = false;
    dm.suspend = false;
    let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut dm);
    let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();
    assert_eq!(ctx.x_value, Some(3));
    assert!(ctx.tagged_objects.is_empty());
    let events: Vec<_> = result
        .events
        .iter()
        .filter_map(|e| e.downcast::<KeywordActionEvent>())
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].amount, 3);
    assert_eq!(game.exile.len(), 1);
    assert_eq!(game.player(A).unwrap().life, 22);
}

#[test]
fn evidence_uses_real_exile_replacement_pipeline() {
    use ironsmith::static_abilities::{CompiledStaticAbility, StaticAbility};
    use ironsmith::target::{ObjectFilter, PlayerFilter};
    let mut game = game();
    let replacement = CardDefinitionBuilder::new(CardId::new(), "Exile replacement")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::from_model(
            CompiledStaticAbility::redirect_zone_change(
                ObjectFilter::default().owned_by(PlayerFilter::You),
                Some(Zone::Graveyard),
                Some(Zone::Exile),
                Zone::Hand,
            ),
        )))
        .build();
    let source = game.create_object_from_definition(&replacement, A, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let card = evidence(&mut game, A, "Replaced evidence", 4, Zone::Graveyard);
    let stable = game.object(card).unwrap().stable_id;
    let mut dm = Choices {
        selected: vec![card],
        ..Default::default()
    };
    let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut dm);
    let outcome = execute_effect(
        &mut game,
        &Effect::new(CollectEvidenceEffect::new(4)),
        &mut ctx,
    )
    .unwrap();
    let current = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(current).unwrap().zone, Zone::Hand);
    assert!(game.exile.is_empty());
    assert_eq!(
        outcome
            .events
            .iter()
            .filter_map(|e| e.downcast::<KeywordActionEvent>())
            .filter(|e| e.action == KeywordActionKind::CollectEvidence)
            .count(),
        1
    );
}

#[test]
fn incinerator_chooses_x_for_collection_and_reflexive_damage_keeps_it_despite_overpayment() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    use ironsmith::game_loop::{
        apply_attacker_declarations, execute_combat_damage_step, queue_combat_damage_triggers,
    };
    use ironsmith::game_state::Step;
    for definition in definitions("Incinerator of the Guilty") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let big = CardBuilder::new(CardId::new(), "Damage recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 8))
            .build();
        let own = game.create_object_from_card(&big, A, Zone::Battlefield);
        let enemy = game.create_object_from_card(&big, B, Zone::Battlefield);
        let card = evidence(&mut game, A, "Seven mana overpayment", 7, Zone::Graveyard);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let mut combat = CombatState::default();
        let mut queue = TriggerQueue::new();
        apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[AttackerDeclaration {
                creature: source,
                target: AttackTarget::Player(B),
            }],
        )
        .unwrap();
        let events = execute_combat_damage_step(&mut game, &combat, false);
        queue_combat_damage_triggers(&mut game, &events, &mut queue);
        assert_eq!(queue.entries.len(), 1);
        let mut dm = Choices {
            selected: vec![card],
            accept: true,
            x: 3,
            ..Default::default()
        };
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(
            game.stack.len(),
            1,
            "collecting evidence creates its own reflexive trigger"
        );
        assert_eq!(game.stack.last().unwrap().x_value, Some(3));
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(
            game.damage_on(enemy),
            3,
            "neither six combat damage nor seven mana value replaces announced X"
        );
        assert_eq!(game.damage_on(own), 0);
        assert_eq!(event_amounts(&game), vec![3]);
    }
}

#[test]
fn public_optional_zero_collection_preserves_if_you_do_after_decline_or_success() {
    use ironsmith::game_state::StackEntry;
    for definition in round_trip(
        "Optional collection probe",
        "Mana cost: {0}\nType: Sorcery\nYou may collect evidence 0. If you do, draw a card.",
    ) {
        for accept in [false, true] {
            let mut game = game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Stack);
            game.push_to_stack(StackEntry::new(spell, A));
            let before = game.player(A).unwrap().hand.len();
            let mut dm = Choices {
                accept,
                ..Default::default()
            };
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(
                game.player(A).unwrap().hand.len(),
                before + usize::from(accept)
            );
            assert_eq!(event_amounts(&game), if accept { vec![0] } else { vec![] });
            assert!(game.exile.is_empty());
        }
    }
}

#[test]
fn existing_additional_and_linked_evidence_controls_keep_executable_event_production() {
    use ironsmith::effects::EmitKeywordActionEffect;
    fn produces_evidence(effect: &Effect) -> bool {
        if contains_collection(effect)
            || effect
                .downcast_ref::<EmitKeywordActionEffect>()
                .is_some_and(|emit| emit.action == KeywordActionKind::CollectEvidence)
        {
            return true;
        }
        let mut found = false;
        effect.visit_child_effects(&mut |child| found |= produces_evidence(child));
        found
    }
    let controls: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/collect_evidence_controls.json.fixture"
    ))
    .unwrap();
    for control in controls {
        let name = control["name"].as_str().unwrap();
        for definition in round_trip(name, control["text"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            if name == "Lamplight Phoenix" {
                assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
                    AbilityKind::Triggered(t) if t.effects.all_effects().into_iter().any(produces_evidence))));
                let lines = ironsmith_text::compiled_text::unprocessed_compiled_lines(&definition)
                    .join("\n");
                assert!(
                    lines.contains("you may exile it and collect evidence 4"),
                    "{lines}"
                );
            } else {
                assert!(
                    format!("{definition:#?}").contains("CollectEvidenceEffect"),
                    "optional evidence casting cost uses the typed action"
                );
            }
        }
    }
}

#[test]
fn unbound_x_can_announce_zero_and_collect_an_empty_graveyard() {
    let mut game = game();
    let source = evidence(&mut game, A, "Empty evidence source", 0, Zone::Battlefield);
    let mut dm = Choices::default();
    let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut dm);
    let result = execute_effect(
        &mut game,
        &Effect::new(CollectEvidenceEffect::new(Value::X)),
        &mut ctx,
    )
    .unwrap();
    assert_eq!(ctx.x_value, Some(0));
    assert_eq!(result.count_or_zero(), 1);
    let events: Vec<_> = result
        .events
        .iter()
        .filter_map(|event| event.downcast::<KeywordActionEvent>())
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].amount, 0);
    assert!(game.exile.is_empty());
}

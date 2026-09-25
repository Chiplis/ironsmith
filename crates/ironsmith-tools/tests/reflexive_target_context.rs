use ironsmith::ability::AbilityKind;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effect::{Effect, EffectOutcome, ExecutionFact};
use ironsmith::effects::{EffectContext, EffectExecutor, ReflexiveTriggerEffect};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn definition(name: &str) -> CardDefinition {
    // Compile the recorded Oracle source afresh; do not depend on generated browser assets.
    let sources: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/reflexive_target_context.json.fixture"
    ))
    .unwrap();
    let block = sources[name].as_str().expect("card source fixture");
    let payload = ironsmith_tools::CardPayload {
        name: name.into(),
        parse_name: None,
        oracle_text: block.into(),
        raw_oracle_text: block.into(),
        metadata_lines: vec![],
        parse_input: block.into(),
        other_face_name: None,
        linked_face_layout: None,
    };
    ironsmith_tools::compile_definition_from_payload(&payload).unwrap()
}
fn find(effect: &Effect) -> Option<ReflexiveTriggerEffect> {
    if let Some(r) = effect.downcast_ref::<ReflexiveTriggerEffect>() {
        return Some(r.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find(child);
        }
    });
    found
}
fn reflexive(def: &CardDefinition) -> ReflexiveTriggerEffect {
    let mut effects = vec![];
    if let Some(p) = &def.spell_effect {
        effects.extend(p.all_effects_owned());
    }
    for ability in &def.abilities {
        if let AbilityKind::Triggered(t) = &ability.kind {
            effects.extend(t.effects.all_effects_owned());
        }
    }
    effects
        .iter()
        .find_map(find)
        .expect("compiled reflexive trigger")
}
fn fixture(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
    mv: u8,
    kind: CardType,
    zone: Zone,
) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![
            ironsmith::mana::ManaSymbol::Generic(mv),
        ]]))
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&def, owner, zone)
}
/// CR 603.12, 603.3: a reflexive trigger is put on the stack the next time a
/// player would receive priority; its targets are chosen then.
fn stack_reflexive(game: &mut GameState, dm: &mut dyn DecisionMaker) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
struct Pick {
    expected: Vec<Target>,
    selected: Vec<Target>,
    maximum: Option<i32>,
    calls: usize,
}
impl DecisionMaker for Pick {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.calls += 1;
        let req = &ctx.requirements[0];
        let mut actual = req.legal_targets.clone();
        actual.sort_by_key(|t| format!("{t:?}"));
        let mut expected = self.expected.clone();
        expected.sort_by_key(|t| format!("{t:?}"));
        assert_eq!(actual, expected, "legal target menu");
        assert_eq!(
            req.aggregate_constraint.as_ref().map(|c| c.maximum),
            self.maximum
        );
        if let Some(c) = &req.aggregate_constraint {
            assert!(c.allows(&self.selected));
            assert!(
                !c.allows(&self.expected),
                "over-budget combined selection must be rejected"
            );
        }
        self.selected.clone()
    }
}
fn damage_event(
    game: &mut GameState,
    source: ObjectId,
    player: PlayerId,
) -> ironsmith::triggers::TriggerEvent {
    ironsmith::triggers::TriggerEvent::new(
        ironsmith::events::DamageEvent::with_cause(
            source,
            ironsmith::events::DamageTarget::Player(player),
            7,
            true,
            ironsmith::events::EventCause::from_effect(source, PlayerId::from_index(0)),
        ),
        game.provenance_graph_mut()
            .alloc_root_event(ironsmith::events::EventKind::Damage),
    )
}
#[test]
fn aggregate_reflexive_limits_keep_roll_or_paid_x_and_damaged_player() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (name, maximum) in [("Ancient Brass Dragon", 16), ("Fire Lord Sozin", 2)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 40);
        let def = definition(name);
        let r = reflexive(&def);
        let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let cost = if maximum == 16 { 7 } else { 2 };
        let a = fixture(
            &mut game,
            bob,
            "First",
            cost,
            CardType::Creature,
            Zone::Graveyard,
        );
        let b = fixture(
            &mut game,
            bob,
            "Second",
            cost,
            CardType::Creature,
            Zone::Graveyard,
        );
        let c = fixture(
            &mut game,
            if maximum == 16 { alice } else { bob },
            "Third",
            cost,
            CardType::Creature,
            Zone::Graveyard,
        );
        let excluded = fixture(
            &mut game,
            alice,
            "Own graveyard",
            1,
            CardType::Creature,
            Zone::Graveyard,
        );
        let mut expected = vec![Target::Object(a), Target::Object(b), Target::Object(c)];
        if maximum == 16 {
            expected.push(Target::Object(excluded));
        }
        let mut pick = Pick {
            expected,
            selected: vec![Target::Object(a)],
            maximum: Some(maximum),
            calls: 0,
        };
        let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut pick);
        ctx.x_value = if maximum == 16 { Some(2) } else { None };
        ctx.triggering_event = Some(damage_event(&mut game, source, bob));
        ctx.store_outcome(
            r.condition,
            EffectOutcome::count(maximum).with_execution_fact(if maximum == 16 {
                ExecutionFact::Accepted
            } else {
                ExecutionFact::ManaPaid { x_value: 2 }
            }),
        );
        r.execute(&mut game, &mut ctx).unwrap();
        drop(ctx);
        stack_reflexive(&mut game, &mut pick);
        assert_eq!(pick.calls, 1);
        assert_eq!(game.stack.len(), 1);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|o| o.name == "First" && game.controller_of(o) == alice)
        }));
        assert!(game.player(bob).unwrap().graveyard.contains(&b));
    }
}
#[test]
fn paid_x_reflexive_targets_are_available_and_survive_revalidation() {
    let alice = PlayerId::from_index(0);
    for (name, kind, x, parent_x) in [
        ("Isareth the Awakener", CardType::Creature, 2, None),
        ("Halo Forager", CardType::Instant, 2, None),
        ("Isareth the Awakener", CardType::Creature, 0, Some(9)),
        ("Halo Forager", CardType::Instant, 0, Some(9)),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let def = definition(name);
        let r = reflexive(&def);
        let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let target = fixture(&mut game, alice, "Matching", x as u8, kind, Zone::Graveyard);
        fixture(
            &mut game,
            alice,
            "Wrong lower",
            if x == 0 { 1 } else { 0 },
            kind,
            Zone::Graveyard,
        );
        fixture(&mut game, alice, "Wrong three", 3, kind, Zone::Graveyard);
        let mut pick = Pick {
            expected: vec![Target::Object(target)],
            selected: vec![Target::Object(target)],
            maximum: None,
            calls: 0,
        };
        let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut pick);
        ctx.x_value = parent_x;
        ctx.store_outcome(
            r.condition,
            EffectOutcome::count(x as i32)
                .with_execution_fact(ExecutionFact::ManaPaid { x_value: x }),
        );
        r.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(
            ctx.x_value, parent_x,
            "the follow-up must not overwrite its parent's X"
        );
        drop(ctx);
        stack_reflexive(&mut game, &mut pick);
        assert_eq!(pick.calls, 1);
        assert_eq!(game.stack[0].x_value, Some(x));
        if name == "Isareth the Awakener" {
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert!(
                game.battlefield
                    .iter()
                    .any(|id| game.object(*id).is_some_and(|o| o.name == "Matching"))
            );
        }
    }
}
#[test]
fn excess_damage_limit_applies_to_both_artifacts_and_enchantments() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for choose_enchantment in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let def = definition("Unleash the Inferno");
        let r = reflexive(&def);
        let source = game.create_object_from_definition(&def, alice, Zone::Stack);
        let artifact = fixture(
            &mut game,
            bob,
            "Legal artifact",
            5,
            CardType::Artifact,
            Zone::Battlefield,
        );
        let enchantment = fixture(
            &mut game,
            bob,
            "Legal enchantment",
            3,
            CardType::Enchantment,
            Zone::Battlefield,
        );
        fixture(
            &mut game,
            bob,
            "Expensive artifact",
            8,
            CardType::Artifact,
            Zone::Battlefield,
        );
        fixture(
            &mut game,
            bob,
            "Expensive enchantment",
            6,
            CardType::Enchantment,
            Zone::Battlefield,
        );
        let chosen = if choose_enchantment {
            enchantment
        } else {
            artifact
        };
        let mut pick = Pick {
            expected: vec![Target::Object(artifact), Target::Object(enchantment)],
            selected: vec![Target::Object(chosen)],
            maximum: None,
            calls: 0,
        };
        let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut pick);
        ctx.store_outcome(
            r.condition,
            EffectOutcome::count(7)
                .with_execution_fact(ExecutionFact::ExcessDamageDealt)
                .with_execution_fact(ExecutionFact::ExcessDamage(5)),
        );
        r.execute(&mut game, &mut ctx).unwrap();
        drop(ctx);
        stack_reflexive(&mut game, &mut pick);
        assert_eq!(pick.calls, 1);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(
            !game.battlefield.contains(&chosen),
            "legal selected target must be destroyed"
        );
    }
}

#[test]
fn missing_aggregate_result_is_an_error_instead_of_an_unlimited_budget() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let def = definition("Ancient Brass Dragon");
    let trigger = reflexive(&def);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let ctx = EffectContext::new_default(source, alice);
    assert!(
        ironsmith::targeting::resolved_target_aggregate_constraint_with_context(
            &game,
            &trigger.choices[0],
            &ctx,
            &[],
        )
        .is_err()
    );
}

#[test]
fn over_budget_reflexive_selection_is_rejected() {
    struct OverBudget(Vec<Target>);
    impl DecisionMaker for OverBudget {
        fn decide_targets(&mut self, _: &GameState, _: &TargetsContext) -> Vec<Target> {
            self.0.clone()
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let def = definition("Ancient Brass Dragon");
    let trigger = reflexive(&def);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let targets = (0..3)
        .map(|_| {
            Target::Object(fixture(
                &mut game,
                alice,
                "Seven",
                7,
                CardType::Creature,
                Zone::Graveyard,
            ))
        })
        .collect();
    let mut pick = OverBudget(targets);
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut pick);
    ctx.store_outcome(
        trigger.condition,
        EffectOutcome::count(16).with_execution_fact(ExecutionFact::Accepted),
    );
    trigger.execute(&mut game, &mut ctx).unwrap();
    drop(ctx);
    stack_reflexive(&mut game, &mut pick);
    assert!(
        game.stack.iter().all(|entry| entry.targets.len() < 3),
        "an over-budget selection is never put on the stack"
    );
}

#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn vanilla_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .build()
}

fn triggered_with_target_choice(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if !triggered.choices.is_empty() => Some(triggered),
            _ => None,
        })
        .expect("card should have a triggered ability with a target choice")
}

#[test]
fn smelted_chargebug_another_attacker_target_excludes_itself() {
    let definition = parse_oracle_card_definition("Smelted Chargebug");
    let triggered = triggered_with_target_choice(&definition);
    let target_spec = triggered.choices[0].clone();
    let ChooseSpec::Target(inner) = target_spec.unhinted() else {
        panic!("Smelted Chargebug should use an explicit target: {target_spec:#?}");
    };
    let ChooseSpec::Object(filter) = inner.unhinted() else {
        panic!("Smelted Chargebug should target an object: {inner:#?}");
    };
    assert!(filter.other, "another must exclude the source: {filter:#?}");
    assert!(
        filter.attacking,
        "the target must be attacking: {filter:#?}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other_attacker = game.create_object_from_definition(
        &vanilla_creature("Other Attacker"),
        alice,
        Zone::Battlefield,
    );
    let nonattacker = game.create_object_from_definition(
        &vanilla_creature("Nonattacker"),
        alice,
        Zone::Battlefield,
    );
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![
            crate::combat_state::AttackerInfo {
                creature: source,
                target: crate::combat_state::AttackTarget::Player(bob),
            },
            crate::combat_state::AttackerInfo {
                creature: other_attacker,
                target: crate::combat_state::AttackTarget::Player(bob),
            },
        ],
        ..crate::combat_state::CombatState::default()
    });

    let legal = crate::game_loop::compute_legal_targets(&game, &target_spec, alice, Some(source));
    assert!(
        legal.contains(&crate::game_state::Target::Object(other_attacker)),
        "another attacking creature should be legal: {legal:#?}"
    );
    assert!(
        !legal.contains(&crate::game_state::Target::Object(source)),
        "Smelted Chargebug must not target itself: {legal:#?}"
    );
    assert!(
        !legal.contains(&crate::game_state::Target::Object(nonattacker)),
        "a nonattacking creature must not be legal: {legal:#?}"
    );
}

#[test]
fn perigee_beckoner_another_controlled_target_excludes_itself() {
    let definition = parse_oracle_card_definition("Perigee Beckoner");
    let triggered = triggered_with_target_choice(&definition);
    let target_spec = triggered.choices[0].clone();
    let ChooseSpec::Target(inner) = target_spec.unhinted() else {
        panic!("Perigee Beckoner should use an explicit target: {target_spec:#?}");
    };
    let ChooseSpec::Object(filter) = inner.unhinted() else {
        panic!("Perigee Beckoner should target an object: {inner:#?}");
    };
    assert!(filter.other, "another must exclude the source: {filter:#?}");
    assert_eq!(filter.controller, Some(PlayerFilter::You));

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let controlled = game.create_object_from_definition(
        &vanilla_creature("Controlled Creature"),
        alice,
        Zone::Battlefield,
    );
    let opposing = game.create_object_from_definition(
        &vanilla_creature("Opposing Creature"),
        bob,
        Zone::Battlefield,
    );

    let legal = crate::game_loop::compute_legal_targets(&game, &target_spec, alice, Some(source));
    assert!(
        legal.contains(&crate::game_state::Target::Object(controlled)),
        "another controlled creature should be legal: {legal:#?}"
    );
    assert!(
        !legal.contains(&crate::game_state::Target::Object(source)),
        "Perigee Beckoner must not target itself: {legal:#?}"
    );
    assert!(
        !legal.contains(&crate::game_state::Target::Object(opposing)),
        "an opposing creature must not be legal: {legal:#?}"
    );
}

struct WickedGuardianDecisions {
    accept: bool,
    chosen: ObjectId,
    legal_candidates: Vec<ObjectId>,
    observed_min: Option<usize>,
    observed_max: Option<Option<usize>>,
}

impl crate::decision::DecisionMaker for WickedGuardianDecisions {
    fn decide_boolean(
        &mut self,
        _game: &crate::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.accept
    }

    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.legal_candidates = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect();
        self.observed_min = Some(ctx.min);
        self.observed_max = Some(ctx.max);
        self.legal_candidates
            .contains(&self.chosen)
            .then_some(vec![self.chosen])
            .unwrap_or_default()
    }
}

fn wicked_guardian_entry_event(
    game: &crate::GameState,
    source: ObjectId,
) -> crate::triggers::TriggerEvent {
    let source_snapshot =
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source).expect("Wicked Guardian should exist"),
            game,
        );
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            source,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(source_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn wicked_guardian_chooses_exactly_one_other_creature_and_draws() {
    let definition = parse_oracle_card_definition("Wicked Guardian");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Wicked Guardian should have an enters trigger");
    let debug = format!("{triggered:#?}");
    assert!(debug.contains("MayEffect"), "{debug}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
    assert!(debug.contains("ExecuteWithSourceEffect"), "{debug}");
    assert!(debug.contains("other: true"), "{debug}");
    assert!(
        !debug.contains("ForEachObject"),
        "Wicked Guardian must choose one creature rather than damage each creature: {debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let chosen = game.create_object_from_definition(
        &vanilla_creature("Chosen Creature"),
        alice,
        Zone::Battlefield,
    );
    let unchosen = game.create_object_from_definition(
        &vanilla_creature("Unchosen Creature"),
        alice,
        Zone::Battlefield,
    );
    let opposing = game.create_object_from_definition(
        &vanilla_creature("Opposing Creature"),
        bob,
        Zone::Battlefield,
    );
    let draw = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Drawn Card").build(),
        alice,
        Zone::Library,
    );
    let draw_stable_id = game.object(draw).expect("draw card should exist").stable_id;

    let event = wicked_guardian_entry_event(&game, source);
    let mut decisions = WickedGuardianDecisions {
        accept: true,
        chosen,
        legal_candidates: Vec::new(),
        observed_min: None,
        observed_max: None,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut decisions)
        .with_triggering_event(event);
    let mut damage_events = Vec::new();
    for effect in triggered.effects.flattened_default_effects() {
        let outcome = crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("accepted Wicked Guardian trigger should resolve");
        damage_events.extend(outcome.events.into_iter().filter_map(|event| {
            event
                .downcast::<crate::events::DamageEvent>()
                .map(|damage| (damage.source, damage.target, damage.amount))
        }));
    }
    drop(ctx);

    assert_eq!(decisions.observed_min, Some(1));
    assert_eq!(decisions.observed_max, Some(Some(1)));
    assert!(decisions.legal_candidates.contains(&chosen));
    assert!(decisions.legal_candidates.contains(&unchosen));
    assert!(!decisions.legal_candidates.contains(&source));
    assert!(!decisions.legal_candidates.contains(&opposing));
    assert_eq!(game.damage_on(chosen), 2);
    assert_eq!(game.damage_on(unchosen), 0);
    assert_eq!(game.damage_on(source), 0);
    assert_eq!(game.damage_on(opposing), 0);
    assert_eq!(damage_events.len(), 1, "{damage_events:#?}");
    assert_eq!(
        damage_events[0].0, source,
        "Wicked Guardian is the damage source"
    );
    assert_eq!(
        damage_events[0].1,
        crate::events::DamageTarget::Object(chosen)
    );
    assert_eq!(damage_events[0].2, 2);
    assert_eq!(
        game.find_object_by_stable_id(draw_stable_id)
            .and_then(|id| game.object(id))
            .map(|object| object.zone),
        Some(Zone::Hand),
        "choosing the damage action should draw a card"
    );
}

#[test]
fn wicked_guardian_decline_deals_no_damage_and_draws_no_card() {
    let definition = parse_oracle_card_definition("Wicked Guardian");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Wicked Guardian should have an enters trigger");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other = game.create_object_from_definition(
        &vanilla_creature("Other Creature"),
        alice,
        Zone::Battlefield,
    );
    let draw = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Undrawn Card").build(),
        alice,
        Zone::Library,
    );
    let event = wicked_guardian_entry_event(&game, source);
    let mut decisions = WickedGuardianDecisions {
        accept: false,
        chosen: other,
        legal_candidates: Vec::new(),
        observed_min: None,
        observed_max: None,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut decisions)
        .with_triggering_event(event);
    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("declined Wicked Guardian trigger should resolve");
    }
    drop(ctx);

    assert_eq!(game.damage_on(source), 0);
    assert_eq!(game.damage_on(other), 0);
    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .library
            .contains(&draw),
        "declining the damage action must not draw"
    );
}

#[test]
fn wicked_guardian_without_another_creature_cannot_do_the_action_or_draw() {
    let definition = parse_oracle_card_definition("Wicked Guardian");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Wicked Guardian should have an enters trigger");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let draw = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Undrawn Card").build(),
        alice,
        Zone::Library,
    );
    let event = wicked_guardian_entry_event(&game, source);
    let mut decisions = WickedGuardianDecisions {
        accept: true,
        chosen: source,
        legal_candidates: Vec::new(),
        observed_min: None,
        observed_max: None,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut decisions)
        .with_triggering_event(event);
    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Wicked Guardian with no legal choice should resolve as not done");
    }
    drop(ctx);

    assert_eq!(game.damage_on(source), 0);
    assert!(decisions.legal_candidates.is_empty());
    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .library
            .contains(&draw),
        "without another creature, the damage action was not done and must not draw"
    );
}

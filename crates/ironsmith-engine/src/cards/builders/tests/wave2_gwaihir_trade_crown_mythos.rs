#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn gwaihir_keeps_fixed_reduction_and_as_long_as_draw_threshold() {
    let definition = parse_oracle_card_definition("Gwaihir the Windlord");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "This spell costs {2} less to cast as long as you've drawn two or more cards this turn.",
            "Flying, vigilance",
            "Other Birds you control have vigilance.",
        ]
    );

    let reduction = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.this_spell_cost_reduction(),
            _ => None,
        })
        .expect("Gwaihir should retain a typed cost reduction");
    assert!(matches!(reduction.reduction.unhinted(), Value::Fixed(2)));
    let crate::static_abilities::ThisSpellCostCondition::AsLongAsConditionExpr {
        condition, ..
    } = &reduction.condition
    else {
        panic!("expected a typed as-long-as condition: {reduction:#?}");
    };
    assert!(matches!(
        condition,
        crate::ConditionExpr::ValueComparison {
            left,
            operator: ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,
            right,
        } if matches!(left.unhinted(), Value::MaxCardsDrawnThisTurn(PlayerFilter::You))
            && matches!(right.unhinted(), Value::Fixed(2))
    ));

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let base_cost = game
        .object(spell)
        .and_then(|object| object.mana_cost.as_ref())
        .expect("Gwaihir should retain its mana cost")
        .clone();
    let effective_before = crate::decision::calculate_effective_mana_cost(
        &game,
        alice,
        game.object(spell).expect("Gwaihir should exist"),
        &base_cost,
    );
    assert_eq!(effective_before.to_oracle(), "{4}{W}{U}");

    let filler = CardDefinitionBuilder::new(CardId::new(), "Draw Threshold Probe")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..2 {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }
    let mut draw_ctx = crate::effects::ExecutionContext::new_default(spell, alice);
    crate::effects::execute_effect(&mut game, &Effect::draw(2), &mut draw_ctx)
        .expect("draw threshold should be recorded");
    let effective_after = crate::decision::calculate_effective_mana_cost(
        &game,
        alice,
        game.object(spell).expect("Gwaihir should exist"),
        &base_cost,
    );
    assert_eq!(effective_after.to_oracle(), "{2}{W}{U}");
}

#[test]
fn trade_route_envoy_keeps_failed_draw_provenance_and_exact_sentence() {
    let definition = parse_oracle_card_definition("Trade Route Envoy");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "When this creature enters, draw a card if you control a creature with a counter on it. If you don't draw a card this way, put a +1/+1 counter on this creature."
        ]
    );
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Trade Route Envoy should have an ETB trigger");
    let [draw, fallback] = triggered.effects.segments.as_slice() else {
        panic!("expected the authored two-sentence program: {triggered:#?}");
    };
    let [draw_root] = draw.default_effects.as_slice() else {
        panic!("expected one conditional draw: {draw:#?}");
    };
    let with_id = draw_root
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("conditional draw should carry result identity");
    let conditional = with_id
        .effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("draw should remain conditionally executable");
    let crate::ConditionExpr::PlayerControls { player, filter } = &conditional.condition else {
        panic!("expected the typed controlled-creature condition: {conditional:#?}");
    };
    let mut expected_filter = ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .with_any_counter();
    expected_filter.set_explicit_card_type_noun(Some(CardType::Creature));
    assert_eq!(player, &PlayerFilter::You);
    assert_eq!(filter, &expected_filter);
    let [draw_effect] = conditional.if_true.as_slice() else {
        panic!("expected one draw branch: {conditional:#?}");
    };
    let draw_effect = draw_effect
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .expect("true branch should draw");
    assert_eq!(draw_effect.player, PlayerFilter::You);
    assert!(matches!(draw_effect.count.unhinted(), Value::Fixed(1)));

    let [fallback_root] = fallback.default_effects.as_slice() else {
        panic!("expected one failed-result branch: {fallback:#?}");
    };
    let fallback = fallback_root
        .downcast_ref::<crate::effects::IfEffect>()
        .expect("second sentence should test the draw result");
    assert_eq!(fallback.condition, with_id.id);
    assert_eq!(
        fallback.predicate,
        crate::effect::EffectPredicate::DidNotHappen
    );
    let [put] = fallback.then.as_slice() else {
        panic!("expected one failed-draw counter effect: {fallback:#?}");
    };
    let put = put
        .downcast_ref::<crate::effects::PutCountersEffect>()
        .expect("failed draw should put a counter");
    assert_eq!(put.counter_type, CounterType::PlusOnePlusOne);
    assert!(matches!(put.amount.unhinted(), Value::Fixed(1)));
    assert!(matches!(put.target.unhinted(), ChooseSpec::Source));
}

fn resolve_trade_route_envoy(with_qualifying_creature: bool) -> (usize, u32) {
    let definition = parse_oracle_card_definition("Trade Route Envoy");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Trade Route Envoy should have an ETB trigger");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Trade Route Draw Probe")
        .card_types(vec![CardType::Land])
        .build();
    game.create_object_from_definition(&filler, alice, Zone::Library);
    if with_qualifying_creature {
        let ally = game.create_object_from_definition(
            &test_permanent("Countered Ally", CardType::Creature),
            alice,
            Zone::Battlefield,
        );
        game.add_counters(ally, CounterType::PlusOnePlusOne, 1)
            .expect("countered ally should remain on the battlefield");
    }
    let hand_before = game.player(alice).expect("Alice should exist").hand.len();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Trade Route Envoy trigger should resolve");
    (
        game.player(alice).expect("Alice should exist").hand.len() - hand_before,
        game.counter_count(source, CounterType::PlusOnePlusOne),
    )
}

#[test]
fn trade_route_envoy_executes_only_the_matching_result_branch() {
    assert_eq!(resolve_trade_route_envoy(true), (1, 0));
    assert_eq!(resolve_trade_route_envoy(false), (0, 1));
}

#[test]
fn crown_of_empires_keeps_same_target_replacement_and_named_artifact_gate() {
    let definition = parse_oracle_card_definition("Crown of Empires");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "{3}, {T}: Tap target creature. Gain control of that creature instead if you control artifacts named Scepter of Empires and Throne of Empires."
        ]
    );
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Crown should have an activated ability");
    let [segment] = activated.effects.segments.as_slice() else {
        panic!("expected one replacement-bearing segment: {activated:#?}");
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("expected one typed self replacement: {segment:#?}");
    };
    let crate::ConditionExpr::And(left, right) = &branch.condition else {
        panic!("both named artifacts should be required: {branch:#?}");
    };
    let named_artifact = |condition: &crate::ConditionExpr| {
        let crate::ConditionExpr::PlayerControls { player, filter } = condition else {
            panic!("expected a controlled named artifact: {condition:#?}");
        };
        assert_eq!(player, &PlayerFilter::You);
        let mut plain = filter.clone();
        let name = plain
            .name
            .take()
            .expect("artifact name should remain typed");
        plain.set_explicit_card_type_noun(None);
        assert_eq!(
            plain,
            ObjectFilter::artifact().controlled_by(PlayerFilter::You)
        );
        name
    };
    assert_eq!(named_artifact(left), "scepter of empires");
    assert_eq!(named_artifact(right), "throne of empires");
    let [replacement] = branch.replacement_effects.as_slice() else {
        panic!("expected one control-change replacement: {branch:#?}");
    };
    let replacement = replacement
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(replacement);
    if let Some(replacement) = replacement.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        assert_eq!(
            replacement.runtime_modifications,
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        );
        let default_target = segment
            .default_effects
            .iter()
            .find_map(|effect| {
                effect
                    .downcast_ref::<crate::effects::TapEffect>()
                    .map(|tap| &tap.target)
            })
            .expect("Crown's default branch should tap the declared target");
        assert_eq!(replacement.target_spec.as_ref(), Some(default_target));
        assert_eq!(replacement.until, crate::Until::Forever);
    } else if let Some(replacement) =
        replacement.downcast_ref::<crate::effects::control::GainControlEffect>()
    {
        let default_target = segment
            .default_effects
            .iter()
            .find_map(|effect| {
                effect
                    .downcast_ref::<crate::effects::TapEffect>()
                    .map(|tap| &tap.target)
            })
            .expect("Crown's default branch should tap the declared target");
        assert_eq!(
            &replacement.target, default_target,
            "the replacement must gain control of the same target"
        );
        assert_eq!(replacement.duration, crate::Until::Forever);
    } else {
        panic!("replacement should be a typed control change: {branch:#?}");
    }
}

fn resolve_crown_of_empires(named_artifacts: &[&str]) -> (bool, PlayerId) {
    let definition = parse_oracle_card_definition("Crown of Empires");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Crown should have an activated ability");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let target = game.create_object_from_definition(
        &test_permanent("Crown Target", CardType::Creature),
        bob,
        Zone::Battlefield,
    );
    for name in named_artifacts {
        game.create_object_from_definition(
            &test_permanent(name, CardType::Artifact),
            alice,
            Zone::Battlefield,
        );
    }
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("Crown activation should resolve");
    (
        game.is_tapped(target),
        game.controller_of_id(target)
            .expect("target should retain a controller"),
    )
}

#[test]
fn crown_of_empires_requires_both_named_artifacts_for_the_replacement() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    assert_eq!(
        resolve_crown_of_empires(&["Scepter of Empires"]),
        (true, bob),
        "one named artifact must leave the tap effect unchanged"
    );
    assert_eq!(
        resolve_crown_of_empires(&["Scepter of Empires", "Throne of Empires"]),
        (false, alice),
        "both named artifacts must replace tapping with control change"
    );
}

fn test_permanent(name: &str, card_type: CardType) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder.power_toughness(PowerToughness::fixed(2, 2)).build()
    } else {
        builder.build()
    }
}

fn resolve_mythos_target(card_type: CardType, paid_green_white: bool) -> Zone {
    let definition = parse_oracle_card_definition("Mythos of Nethroi");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = game.create_object_from_definition(
        &test_permanent("Mythos Target", card_type),
        bob,
        Zone::Battlefield,
    );
    let stable = game.object(target).expect("target exists").stable_id;
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    if paid_green_white {
        game.object_mut(spell)
            .expect("spell exists")
            .mana_spent_to_cast = crate::player::ManaPool {
            green: 1,
            white: 1,
            ..Default::default()
        };
    }
    let mut ctx = crate::effects::ExecutionContext::new_default(spell, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell,
        definition.spell_effect.as_ref().expect("spell program"),
        None,
        &[],
    )
    .expect("Mythos should resolve");
    game.find_object_by_stable_id(stable)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
        .expect("target should retain stable identity")
}

#[test]
fn mythos_of_nethroi_uses_target_characteristic_or_paid_colors() {
    let definition = parse_oracle_card_definition("Mythos of Nethroi");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Destroy target nonland permanent if it's a creature or if {G}{W} was spent to cast this spell."
        ]
    );
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Mythos should have a spell program");
    let [segment] = program.segments.as_slice() else {
        panic!("Mythos should keep one target/action segment: {program:#?}");
    };
    let [target_root, conditional_root] = segment.default_effects.as_slice() else {
        panic!("Mythos should declare one tagged target before its gate: {segment:#?}");
    };
    let target = target_root
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("Mythos target should retain a stable tag");
    let target_only = target
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .expect("Mythos should announce exactly one target");
    assert!(target_only.target.is_target());
    let with_id = conditional_root
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("the conditional destroy should retain result identity");
    let conditional = with_id
        .effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("Mythos should keep one executable OR gate");
    assert!(matches!(
        &conditional.condition,
        crate::ConditionExpr::Or(_, _)
    ));
    let [destroy_root] = conditional.if_true.as_slice() else {
        panic!("the true branch should contain one destroy: {conditional:#?}");
    };
    let destroy = destroy_root
        .downcast_ref::<crate::effects::TaggedEffect>()
        .filter(|destroyed| destroyed.tag == target.tag)
        .and_then(|destroyed| {
            destroyed
                .effect
                .downcast_ref::<crate::effects::DestroyEffect>()
        })
        .expect("the destroy should consume and preserve the announced target tag");
    assert!(matches!(
        destroy.spec.base(),
        ChooseSpec::Tagged(tag) if tag == &target.tag
    ));

    assert_eq!(
        resolve_mythos_target(CardType::Creature, false),
        Zone::Graveyard,
        "the creature arm should work without green-white mana"
    );
    assert_eq!(
        resolve_mythos_target(CardType::Artifact, false),
        Zone::Battlefield,
        "an unpaid noncreature must survive"
    );
    assert_eq!(
        resolve_mythos_target(CardType::Artifact, true),
        Zone::Graveyard,
        "green-white mana should enable the noncreature arm"
    );
}

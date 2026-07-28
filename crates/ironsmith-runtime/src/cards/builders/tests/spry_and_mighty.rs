#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn spry_and_mighty_renders_the_exact_chosen_pair_program() {
    let oracle = "Choose exactly two creatures you control. You draw X cards and the chosen creatures get +X/+X and gain trample until end of turn, where X is the difference between the chosen creatures' powers.";
    let definition = parse_oracle_card_definition("Spry and Mighty");
    let compiled = canonical_compiled_lines(&definition).join(" ");

    assert_eq!(compiled, oracle);

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("min: 2")
            && debug.contains("__chosen_objects__")
            && debug.contains("DrawCardsEffect")
            && debug.contains("GreatestPower")
            && debug.contains("LeastPower")
            && debug.contains("Trample"),
        "the compiled program must retain the exact pair, power gap, draw, pump, and trample: {debug}"
    );
}

#[test]
fn spry_and_mighty_keeps_one_typed_coordinated_reward() {
    fn unwrap_tags(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap_tags(&tagged.effect);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return unwrap_tags(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return unwrap_tags(&with_id.effect);
        }
        effect
    }

    let definition = parse_oracle_card_definition("Spry and Mighty");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Spry and Mighty must retain its resolution program");
    let [choice_segment, reward_segment] = program.segments.as_slice() else {
        panic!("expected one choice sentence and one reward sentence: {program:#?}");
    };
    let [choice_effect] = choice_segment.default_effects.as_slice() else {
        panic!("the first sentence must contain one exact choice: {choice_segment:#?}");
    };
    let choice = choice_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("the first sentence must choose the creature pair");
    assert_eq!((choice.count.min, choice.count.max), (2, Some(2)));
    assert!(
        choice.count.explicit_exactly,
        "the typed choice must preserve the explicitly authored `exactly`"
    );

    let [sequence_effect] = reward_segment.default_effects.as_slice() else {
        panic!("the reward sentence must lower to one sequence: {reward_segment:#?}");
    };
    let sequence = sequence_effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the reward sentence must retain its coordinated boundary");
    assert_eq!(
        sequence.surface,
        ironsmith_core::SequenceSurface::Coordinated
    );
    let [draw_effect, pump_effect, grant_effect] = sequence.effects.as_slice() else {
        panic!("expected coordinated draw, pump, and grant: {sequence:#?}");
    };
    let draw = unwrap_tags(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .expect("the first coordinated arm must draw cards");
    assert!(
        draw.count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::Difference),
        "the draw count must retain the chosen-pair power-gap value: {draw:#?}"
    );

    let tagged_pump = pump_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the pump result must have a durable affected-set tag");
    let pump = unwrap_tags(pump_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("the second coordinated arm must pump the chosen creatures");
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = pump.runtime_modifications.as_slice()
    else {
        panic!("expected one power/toughness modification: {pump:#?}");
    };
    assert_eq!(draw.count.unhinted(), power.unhinted());
    assert_eq!(power.unhinted(), toughness.unhinted());
    let crate::continuous::EffectTarget::Filter(pump_filter) = &pump.target else {
        panic!("the pump must retain its authored chosen-creature filter: {pump:#?}");
    };

    let grant = unwrap_tags(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("the third coordinated arm must grant trample");
    let crate::continuous::EffectTarget::Filter(grant_filter) = &grant.target else {
        panic!("the grant must consume the pump's affected set: {grant:#?}");
    };
    assert_eq!(
        grant_filter.tagged_constraints.as_slice(),
        [crate::filter::TaggedObjectConstraint {
            tag: tagged_pump.tag.clone(),
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        }],
        "the grant must have exactly one affected-set reference"
    );
    let mut pump_base = pump_filter.clone();
    pump_base.tagged_constraints.clear();
    let mut grant_base = grant_filter.clone();
    grant_base.tagged_constraints.clear();
    assert_eq!(
        grant_base, pump_base,
        "the grant must not narrow the successfully pumped set"
    );
    assert!(matches!(
        grant.modification.as_ref(),
        Some(crate::continuous::Modification::AddAbility(ability))
            if ability.id() == StaticAbilityId::Trample
    ));
}

#[test]
fn spry_and_mighty_resolves_power_gap_draw_pump_and_trample() {
    fn test_creature(raw_id: u32, name: &str, power: i32, toughness: i32) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(raw_id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    }

    let definition = parse_oracle_card_definition("Spry and Mighty");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Spry and Mighty must retain its resolution program");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let small = game.create_object_from_definition(
        &test_creature(96_200, "Small Chosen Creature", 2, 2),
        alice,
        Zone::Battlefield,
    );
    let large = game.create_object_from_definition(
        &test_creature(96_201, "Large Chosen Creature", 5, 4),
        alice,
        Zone::Battlefield,
    );
    let opponent = game.create_object_from_definition(
        &test_creature(96_202, "Opponent Creature", 4, 4),
        bob,
        Zone::Battlefield,
    );
    for (raw_id, name) in [
        (96_203, "First Draw"),
        (96_204, "Second Draw"),
        (96_205, "Third Draw"),
    ] {
        let card = CardDefinitionBuilder::new(CardId::from_raw(raw_id), name).build();
        game.create_object_from_definition(&card, alice, Zone::Library);
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut decisions);

    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("the complete chosen-pair program must resolve");

    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        3,
        "the power difference is three, so Alice must draw three cards"
    );
    assert_eq!(
        (
            game.calculated_power(small),
            game.calculated_toughness(small)
        ),
        (Some(5), Some(5))
    );
    assert_eq!(
        (
            game.calculated_power(large),
            game.calculated_toughness(large)
        ),
        (Some(8), Some(7))
    );
    for chosen in [small, large] {
        assert!(
            game.current_has_static_ability_id(chosen, StaticAbilityId::Trample),
            "each chosen creature must gain trample"
        );
    }
    assert_eq!(
        (
            game.calculated_power(opponent),
            game.calculated_toughness(opponent)
        ),
        (Some(4), Some(4)),
        "an unchosen creature must not be pumped"
    );
    assert!(
        !game.current_has_static_ability_id(opponent, StaticAbilityId::Trample),
        "an unchosen creature must not gain trample"
    );
}

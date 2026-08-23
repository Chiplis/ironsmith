use super::*;

#[test]
fn repeated_die_complete_parity_branches_render_as_per_result_in_source_order() {
    let roll_id = crate::effect::EffectId(13);
    let target = Effect::new(crate::effects::TargetOnlyEffect::new(
        ChooseSpec::target_creature(),
    ))
    .tag("targeted_0");
    let repeated_roll = Effect::with_id(
        roll_id.0,
        Effect::new(crate::effects::RepeatEffectsEffect::new(
            Value::X,
            vec![Effect::roll_die(6, PlayerFilter::You)],
        )),
    );
    let even = Effect::if_then(
        roll_id,
        crate::effect::EffectPredicate::Value(crate::effect::Comparison::OneOf(&[2, 4, 6])),
        vec![Effect::gain_life(2)],
    );
    let odd = Effect::if_then(
        roll_id,
        crate::effect::EffectPredicate::Value(crate::effect::Comparison::OneOf(&[1, 3, 5])),
        vec![Effect::gain_life(1)],
    );
    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![target]),
        crate::resolution::ResolutionSegment::from_effects(vec![repeated_roll]),
        crate::resolution::ResolutionSegment::from_effects(vec![even]),
        crate::resolution::ResolutionSegment::from_effects(vec![odd]),
    ]);

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "Choose target creature. Roll X six-sided dice. For each even result, you gain 2 life. For each odd result, you gain 1 life"
    );
}

#[test]
fn repeated_die_partial_value_sets_do_not_claim_complete_parity() {
    let roll_id = crate::effect::EffectId(17);
    let repeated_roll = Effect::with_id(
        roll_id.0,
        Effect::new(crate::effects::RepeatEffectsEffect::new(
            Value::Fixed(3),
            vec![Effect::roll_die(6, PlayerFilter::You)],
        )),
    );
    let incomplete_even = Effect::if_then(
        roll_id,
        crate::effect::EffectPredicate::Value(crate::effect::Comparison::OneOf(&[2, 4])),
        vec![Effect::gain_life(2)],
    );
    let odd = Effect::if_then(
        roll_id,
        crate::effect::EffectPredicate::Value(crate::effect::Comparison::OneOf(&[1, 3, 5])),
        vec![Effect::gain_life(1)],
    );
    let program = crate::resolution::ResolutionProgram::from_effects(vec![
        repeated_roll,
        incomplete_even,
        odd,
    ]);

    assert!(
        describe_repeated_die_parity_result_program(&program).is_none(),
        "an incomplete result set must retain the explicit numeric conditions"
    );
}

fn sequenced_d20_program(
    first: crate::effect::Comparison,
    second: crate::effect::Comparison,
    second_condition: crate::effect::EffectId,
) -> crate::resolution::ResolutionProgram {
    let roll_id = crate::effect::EffectId(23);
    let target = Effect::new(crate::effects::TargetOnlyEffect::explicit(
        ChooseSpec::target_creature(),
    ));
    let setup = Effect::with_id(
        roll_id.0,
        Effect::new(crate::effects::SequenceEffect::comma_then(vec![
            target,
            Effect::roll_die(20, PlayerFilter::You),
        ])),
    );
    let first = Effect::if_then(
        roll_id,
        crate::effect::EffectPredicate::Value(first),
        vec![Effect::gain_life(1)],
    );
    let second = Effect::if_then(
        second_condition,
        crate::effect::EffectPredicate::Value(second),
        vec![Effect::gain_life(2)],
    );
    crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![setup]),
        crate::resolution::ResolutionSegment::from_effects(vec![first]),
        crate::resolution::ResolutionSegment::from_effects(vec![second]),
    ])
}

#[test]
fn sequenced_d20_setup_renders_only_exhaustive_same_result_id_rows() {
    let roll_id = crate::effect::EffectId(23);
    let program = sequenced_d20_program(
        crate::effect::Comparison::BetweenInclusive(1, 9),
        crate::effect::Comparison::BetweenInclusive(10, 20),
        roll_id,
    );
    assert_eq!(
        describe_sequenced_d20_numeric_result_table_program(&program).as_deref(),
        Some(
            "Choose target creature, then roll a d20.\n1—9 | You gain 1 life.\n10—20 | You gain 2 life."
        )
    );
    let one_segment = crate::resolution::ResolutionProgram::from_effects(
        program.flattened_default_effects().to_vec(),
    );
    assert_eq!(
        describe_sequenced_d20_numeric_result_table_program(&one_segment).as_deref(),
        describe_sequenced_d20_numeric_result_table_program(&program).as_deref(),
        "numeric rows may share their setup segment or be lowered into later segments"
    );

    let missing_ten = sequenced_d20_program(
        crate::effect::Comparison::BetweenInclusive(1, 9),
        crate::effect::Comparison::BetweenInclusive(11, 20),
        roll_id,
    );
    assert!(
        describe_sequenced_d20_numeric_result_table_program(&missing_ten).is_none(),
        "a non-exhaustive result table must retain explicit conditions"
    );

    let unrelated_result = sequenced_d20_program(
        crate::effect::Comparison::BetweenInclusive(1, 9),
        crate::effect::Comparison::BetweenInclusive(10, 20),
        crate::effect::EffectId(24),
    );
    assert!(
        describe_sequenced_d20_numeric_result_table_program(&unrelated_result).is_none(),
        "every row must consume the exact setup result ID"
    );
}

#[test]
fn d20_result_conjunction_preserves_explicit_controller_draw_subject() {
    let roll_id = crate::effect::EffectId(29);
    let draw_and_lose = Effect::new(crate::effects::SequenceEffect::result_conjunction(
        vec![
            Effect::draw(Value::Fixed(2)),
            Effect::new(crate::effects::LoseLifeEffect::you(2)),
        ],
        false,
    ));
    let branch = Effect::if_then(
        roll_id,
        crate::effect::EffectPredicate::Value(crate::effect::Comparison::Equal(20)),
        vec![draw_and_lose],
    );
    let low_branch = Effect::if_then(
        roll_id,
        crate::effect::EffectPredicate::Value(crate::effect::Comparison::BetweenInclusive(1, 19)),
        vec![Effect::gain_life(1)],
    );
    let table = vec![
        Effect::with_id(roll_id.0, Effect::roll_die(20, PlayerFilter::You)),
        low_branch.clone(),
        branch,
    ];
    assert_eq!(
        describe_roll_die_with_numeric_result_table(&table).as_deref(),
        Some("Roll a d20.\n1—19 | You gain 1 life.\n20 | You draw two cards and you lose 2 life.")
    );

    let mismatched_life_actor = Effect::new(crate::effects::SequenceEffect::result_conjunction(
        vec![
            Effect::draw(Value::Fixed(2)),
            Effect::new(crate::effects::LoseLifeEffect::with_filter(
                2,
                PlayerFilter::Opponent,
            )),
        ],
        false,
    ));
    let near_miss = vec![
        Effect::with_id(roll_id.0, Effect::roll_die(20, PlayerFilter::You)),
        low_branch,
        Effect::if_then(
            roll_id,
            crate::effect::EffectPredicate::Value(crate::effect::Comparison::Equal(20)),
            vec![mismatched_life_actor],
        ),
    ];
    assert!(
        describe_roll_die_with_numeric_result_table(&near_miss)
            .is_some_and(|rendered| !rendered.contains("You draw")),
        "an unrelated life-loss actor must not claim the controller-draw conjunction surface"
    );
}

#[test]
fn d20_single_card_result_conjunction_keeps_both_controller_subjects() {
    let roll_id = crate::effect::EffectId(31);
    let branch = Effect::if_then(
        roll_id,
        crate::effect::EffectPredicate::Value(crate::effect::Comparison::BetweenInclusive(1, 9)),
        vec![Effect::new(
            crate::effects::SequenceEffect::result_conjunction(
                vec![
                    Effect::draw(Value::Fixed(1)),
                    Effect::new(crate::effects::LoseLifeEffect::you(1)),
                ],
                false,
            ),
        )],
    );
    let table = vec![
        Effect::with_id(roll_id.0, Effect::roll_die(20, PlayerFilter::You)),
        branch,
        Effect::if_then(
            roll_id,
            crate::effect::EffectPredicate::Value(crate::effect::Comparison::BetweenInclusive(
                10, 20,
            )),
            vec![Effect::gain_life(1)],
        ),
    ];
    assert_eq!(
        describe_roll_die_with_numeric_result_table(&table).as_deref(),
        Some("Roll a d20.\n1—9 | You draw a card and you lose 1 life.\n10—20 | You gain 1 life.")
    );
}

#[test]
fn numeric_result_table_renders_only_typed_authored_branch_labels() {
    let roll_id = crate::effect::EffectId(30);
    let labeled = Effect::new(crate::effects::SequenceEffect::result_labeled(
        vec![Effect::new(crate::effects::LoseLifeEffect::you(3))],
        "Trapped!",
    ));
    let table = vec![
        Effect::with_id(roll_id.0, Effect::roll_die(20, PlayerFilter::You)),
        Effect::if_then(
            roll_id,
            crate::effect::EffectPredicate::Value(crate::effect::Comparison::Equal(1)),
            vec![labeled],
        ),
        Effect::if_then(
            roll_id,
            crate::effect::EffectPredicate::Value(crate::effect::Comparison::BetweenInclusive(
                2, 20,
            )),
            vec![Effect::gain_life(1)],
        ),
    ];
    assert_eq!(
        describe_roll_die_with_numeric_result_table(&table).as_deref(),
        Some("Roll a d20.\n1 | Trapped! — You lose 3 life.\n2—20 | You gain 1 life.")
    );

    let mut non_label_surface = crate::effects::SequenceEffect::result_labeled(
        vec![Effect::new(crate::effects::LoseLifeEffect::you(3))],
        "Trapped!",
    );
    non_label_surface.surface = ironsmith_core::SequenceSurface::Coordinated;
    let near_miss = vec![
        Effect::with_id(roll_id.0, Effect::roll_die(20, PlayerFilter::You)),
        Effect::if_then(
            roll_id,
            crate::effect::EffectPredicate::Value(crate::effect::Comparison::Equal(1)),
            vec![Effect::new(non_label_surface)],
        ),
        Effect::if_then(
            roll_id,
            crate::effect::EffectPredicate::Value(crate::effect::Comparison::BetweenInclusive(
                2, 20,
            )),
            vec![Effect::gain_life(1)],
        ),
    ];
    assert!(
        describe_roll_die_with_numeric_result_table(&near_miss)
            .is_some_and(|text| !text.contains("Trapped!")),
        "only the dedicated typed branch-label surface may restore the label"
    );
}

fn roll_then_result_draw_program(
    result_id: crate::effect::EffectId,
    include_result_hint: bool,
) -> crate::resolution::ResolutionProgram {
    let roll_id = crate::effect::EffectId(31);
    let roll = Effect::with_id(roll_id.0, Effect::roll_die(20, PlayerFilter::You));
    let mut count =
        Value::EffectValue(result_id).with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo);
    if include_result_hint {
        count = count.with_surface_hint(ironsmith_core::ValueSurfaceHint::PriorEffectResult);
    }
    crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![roll]),
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::draw(count)]),
    ])
}

#[test]
fn roll_result_draw_requires_exact_id_and_explicit_result_provenance() {
    let roll_id = crate::effect::EffectId(31);
    let program = roll_then_result_draw_program(roll_id, true);
    assert_eq!(
        describe_roll_die_then_draw_equal_result_program(&program).as_deref(),
        Some("Roll a d20. Draw cards equal to the result")
    );
    let mut with_trailing = program.clone();
    with_trailing
        .segments
        .push(crate::resolution::ResolutionSegment::from_effects(vec![
            Effect::gain_life(1),
        ]));
    assert_eq!(
        describe_roll_die_then_draw_equal_result_program(&with_trailing).as_deref(),
        Some("Roll a d20. Draw cards equal to the result. You gain 1 life")
    );

    let wrong_id = roll_then_result_draw_program(crate::effect::EffectId(32), true);
    assert!(
        describe_roll_die_then_draw_equal_result_program(&wrong_id).is_none(),
        "a draw tied to another effect must not claim this roll's result"
    );

    let ambient_amount = roll_then_result_draw_program(roll_id, false);
    assert!(
        describe_roll_die_then_draw_equal_result_program(&ambient_amount).is_none(),
        "an ambient amount without authored result provenance must retain its own surface"
    );
}

#[test]
fn die_result_damage_then_random_source_attachment_keeps_typed_sequence_surface() {
    let roll_id = crate::effect::EffectId(7);
    let enchanted_player = PlayerFilter::TaggedPlayer(TagKey::from("enchanted"));
    let chosen_player_tag = TagKey::from("chosen_player_0");
    let effects = [
        Effect::with_id(
            roll_id.0,
            Effect::roll_die_with_die_text(6, PlayerFilter::You, Some("d6".to_string())),
        ),
        Effect::deal_damage(
            Value::EffectValue(roll_id),
            ChooseSpec::Player(enchanted_player.clone()),
        ),
        Effect::new(
            crate::effects::ChoosePlayerEffect::new(
                PlayerFilter::You,
                PlayerFilter::excluding(PlayerFilter::Opponent, enchanted_player),
                chosen_player_tag.clone(),
            )
            .at_random(),
        ),
        Effect::attach_objects(
            ChooseSpec::Source,
            ChooseSpec::Player(PlayerFilter::TaggedPlayer(chosen_player_tag)),
        ),
    ];

    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![effects[0].clone()]),
        crate::resolution::ResolutionSegment::from_effects(vec![effects[1].clone()]),
        crate::resolution::ResolutionSegment::from_effects(effects[2..].to_vec()),
    ]);

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "Roll a d6. Deal damage to enchanted player equal to the result. Then attach this source to another one of your opponents chosen at random"
    );
}

#[test]
fn die_result_damage_then_direct_random_source_attachment_keeps_typed_sequence_surface() {
    let roll_id = crate::effect::EffectId(11);
    let enchanted_player = PlayerFilter::TaggedPlayer(TagKey::from("enchanted"));
    let effects = [
        Effect::with_id(
            roll_id.0,
            Effect::roll_die_with_die_text(6, PlayerFilter::You, Some("d6".to_string())),
        ),
        Effect::deal_damage(
            Value::EffectValue(roll_id),
            ChooseSpec::Player(enchanted_player.clone()),
        ),
        Effect::attach_objects(
            ChooseSpec::Source,
            ChooseSpec::Player(PlayerFilter::excluding(
                PlayerFilter::Opponent,
                enchanted_player,
            ))
            .with_count(ChoiceCount::exactly(1).at_random()),
        ),
    ];

    let program = crate::resolution::ResolutionProgram::new(vec![
        crate::resolution::ResolutionSegment::from_effects(vec![effects[0].clone()]),
        crate::resolution::ResolutionSegment::from_effects(vec![effects[1].clone()]),
        crate::resolution::ResolutionSegment::from_effects(vec![effects[2].clone()]),
    ]);

    assert_eq!(
        super::super::ast_render::describe_resolution_program(&program),
        "Roll a d6. Deal damage to enchanted player equal to the result. Then attach this source to another one of your opponents chosen at random"
    );
}

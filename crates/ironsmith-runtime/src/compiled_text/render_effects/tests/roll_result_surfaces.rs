use super::*;

#[test]
fn die_result_damage_then_random_source_attachment_keeps_typed_sequence_surface() {
    let roll_id = crate::effect::EffectId(7);
    let enchanted_player = PlayerFilter::TaggedPlayer(TagKey::from("enchanted"));
    let chosen_player_tag = TagKey::from("chosen_player_0");
    let effects = vec![
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
    let effects = vec![
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

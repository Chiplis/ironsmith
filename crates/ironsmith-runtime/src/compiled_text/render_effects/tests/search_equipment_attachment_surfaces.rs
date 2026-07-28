use super::*;
use crate::effect::EffectId;

fn search_put_attach_with_shuffle_condition(condition: EffectId) -> Effect {
    let searched = TagKey::from("searched_multi_zone");
    let attachment_target = TagKey::from("attachment_target_0");
    let choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .with_subtype(Subtype::Equipment)
            .in_zone(Zone::Library),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        searched.clone(),
    )
    .in_zone(Zone::Library)
    .as_search();
    let choose_attachment_target = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::creature().controlled_by(PlayerFilter::You),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        attachment_target.clone(),
    );
    let move_and_attach = crate::effects::ForEachTaggedEffect::new(
        searched.clone(),
        vec![
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(searched.clone()),
                Zone::Battlefield,
                false,
            )),
            Effect::new(choose_attachment_target),
            Effect::new(crate::effects::AttachObjectsEffect::new(
                ChooseSpec::All(ObjectFilter::tagged(searched)),
                ChooseSpec::Tagged(attachment_target),
            )),
        ],
    );
    let moved = Effect::with_id(0, Effect::new(move_and_attach));
    let shuffle = Effect::if_then(
        condition,
        EffectPredicate::SearchedLibrary,
        vec![Effect::new(crate::effects::ShuffleLibraryEffect::new(
            PlayerFilter::You,
        ))],
    );
    Effect::new(crate::effects::SequenceEffect::new(vec![
        Effect::new(choose),
        moved,
        shuffle,
    ]))
}

#[test]
fn a_linked_search_put_attach_and_conditional_shuffle_is_one_instruction() {
    let sequence = search_put_attach_with_shuffle_condition(EffectId(0));
    let expected = "Search your library for an Equipment card, put it onto the battlefield, attach it to a creature you control, then shuffle";

    assert_eq!(describe_effect(&sequence), expected);
    assert_eq!(compile_effect_list(&[sequence.clone()]), expected);
    let program = crate::resolution::ResolutionProgram::from_effects(vec![sequence]);
    assert_eq!(describe_resolution_program(&program), expected);
}

#[test]
fn an_unrelated_shuffle_condition_is_not_folded_into_the_search_instruction() {
    let sequence = search_put_attach_with_shuffle_condition(EffectId(1));
    let compact = "Search your library for an Equipment card, put it onto the battlefield, attach it to a creature you control, then shuffle";

    assert_ne!(describe_effect(&sequence), compact);
    assert_ne!(compile_effect_list(&[sequence.clone()]), compact);
    let program = crate::resolution::ResolutionProgram::from_effects(vec![sequence]);
    assert_ne!(describe_resolution_program(&program), compact);
}

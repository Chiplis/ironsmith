use super::*;

pub(super) fn parse_exile_collection_each_upkeep_return_bundle(
    exile_sentence: &[OwnedLexToken],
    upkeep_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if bundle_grammar::parse_collection_scoped_each_upkeep_return_shape(
        exile_sentence,
        upkeep_sentence,
    )
    .is_none()
    {
        return Ok(None);
    }

    let exile_effects = effect_sentences::parse_effect_sentence_lexed(exile_sentence)?;
    let [exile_effect] = exile_effects.as_slice() else {
        return Ok(None);
    };
    if !matches!(
        exile_effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ExileAll {
                face_down: false,
                ..
            },
            ..
        })
    ) {
        return Ok(None);
    }

    let exiled_tag = crate::tag::CompilerReferenceTag::SourceExiled.key();
    let chosen_tag = crate::tag::CompilerReferenceTag::DelayedOwnedExiledChoice.key();
    let mut owned_exiled_filter = ObjectFilter::default();
    owned_exiled_filter.zone = Some(Zone::Exile);
    owned_exiled_filter.owner = Some(PlayerFilter::Active);
    owned_exiled_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: exiled_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let delayed_effects = vec![
        EffectAst::ChooseObjects {
            filter: owned_exiled_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Active,
            tag: chosen_tag.clone(),
        },
        EffectAst::subject_verb_return_to_battlefield(
            TargetAst::Tagged(chosen_tag, span_from_tokens(upkeep_sentence)),
            false,
            false,
            false,
            ReturnControllerAst::Owner,
            None,
        ),
    ];

    Ok(Some(vec![
        exile_effect.clone(),
        EffectAst::DelayedTriggerForDuration {
            trigger: crate::cards::builders::TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any),
            effects: delayed_effects,
            one_shot: false,
            duration: crate::effect::Until::Forever,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: Some((exiled_tag, Zone::Exile)),
        },
    ]))
}

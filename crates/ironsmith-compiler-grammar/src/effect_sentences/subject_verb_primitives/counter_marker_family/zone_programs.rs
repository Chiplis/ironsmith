use super::*;

pub fn clone_return_effect_with_subtype(base: &EffectAst, subtype: Subtype) -> Option<EffectAst> {
    match base {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::ReturnToHand {
                target,
                random,
                destination_player_surface,
                exiled_with_source_surface,
                set_quantifier_surface,
                set_reference_surface,
            } => {
                let mut cloned_target = target.clone();
                replace_target_subtype(&mut cloned_target, subtype).then_some(
                    EffectAst::subject_verb_return_to_hand(cloned_target, *random)
                        .with_return_destination_player_surface(*destination_player_surface)
                        .with_exiled_with_source_surface(exiled_with_source_surface.clone())
                        .with_return_set_quantifier_surface(*set_quantifier_surface)
                        .with_return_set_reference_surface(set_reference_surface.clone()),
                )
            }
            SubjectVerbActionAst::ReturnAllToHand {
                filter,
                destination_player_surface,
                exiled_with_source_surface,
            } => {
                let mut cloned_filter = filter.clone();
                cloned_filter.subtypes = vec![subtype];
                Some(
                    EffectAst::subject_verb_return_all_to_hand(cloned_filter)
                        .with_return_destination_player_surface(*destination_player_surface)
                        .with_exiled_with_source_surface(exiled_with_source_surface.clone()),
                )
            }
            SubjectVerbActionAst::ReturnToBattlefield {
                target,
                tapped,
                transformed,
                converted,
                controller,
                count_value,
                as_aura,
                top_only,
                ..
            } => {
                let mut cloned_target = target.clone();
                replace_target_subtype(&mut cloned_target, subtype).then(|| {
                    let mut effect = EffectAst::subject_verb_return_to_battlefield(
                        cloned_target,
                        *tapped,
                        *transformed,
                        *converted,
                        *controller,
                        count_value.clone(),
                    )
                    .with_top_only_return_choice(*top_only);
                    if let EffectAst::SubjectVerb(subject_verb) = &mut effect
                        && let SubjectVerbActionAst::ReturnToBattlefield { as_aura: dst, .. } =
                            &mut subject_verb.action
                    {
                        *dst = as_aura.clone();
                    }
                    effect
                })
            }
            SubjectVerbActionAst::ReturnAllToBattlefield {
                filter,
                tapped,
                face_down,
                controller,
                verb_surface,
            } => {
                let mut cloned_filter = filter.clone();
                cloned_filter.subtypes = vec![subtype];
                Some(
                    EffectAst::subject_verb_return_all_to_battlefield(
                        cloned_filter,
                        *tapped,
                        *face_down,
                        *controller,
                    )
                    .with_move_to_zone_verb_surface(*verb_surface),
                )
            }
            _ => None,
        },
        _ => None,
    }
}

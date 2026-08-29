use super::*;

pub(super) fn bind_adjacent_discard_count_draws(effects: &mut [EffectAst]) {
    fn is_discard(effect: &EffectAst) -> bool {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Discard { .. },
                ..
            })
        )
    }

    fn bind_draw(effect: &mut EffectAst) {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { count },
            ..
        }) = effect
        else {
            return;
        };
        let hints = count.surface_hints().to_vec();
        let bound = match count.unhinted() {
            Value::EventValue(crate::effect::EventValueSpec::Amount) => {
                Value::PendingEffectMetric {
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::Count,
                }
            }
            Value::EventValueOffset(crate::effect::EventValueSpec::Amount, offset) => {
                Value::PendingEffectMetricOffset {
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::Count,
                    offset: *offset,
                }
            }
            _ => return,
        };
        *count = bound.with_surface_hints(hints);
    }

    for index in 0..effects.len().saturating_sub(1) {
        if is_discard(&effects[index]) {
            bind_draw(&mut effects[index + 1]);
        }
    }
}

pub(super) fn bind_adjacent_implicit_draw_discard_subjects(
    effects: &mut [EffectAst],
    recognized_shared_actor: bool,
) {
    if !recognized_shared_actor {
        return;
    }
    for index in 0..effects.len().saturating_sub(1) {
        let draw_is_implicit = matches!(
            &effects[index],
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject,
                action: SubjectVerbActionAst::Draw { .. },
            }) if subject.player == PlayerAst::Implicit
        );
        if !draw_is_implicit {
            continue;
        }
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::Discard { .. },
        }) = &mut effects[index + 1]
            && subject.player == PlayerAst::Implicit
        {
            subject.player = PlayerAst::You;
        }
    }
}

pub(super) fn for_each_revealed_this_way_filter(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && (constraint.tag.as_str() == IT_TAG
                || sentence_helper_revealed_tag(constraint.tag.as_str()))
    })
}

pub(super) fn sentence_helper_revealed_tag(tag: &str) -> bool {
    tag.starts_with(SENTENCE_HELPER_REVEALED_TAG_PREFIX)
}

pub(super) fn is_revealed_this_way_scalar_reward(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainLife { .. }
                | SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. },
            ..
        })
    )
}

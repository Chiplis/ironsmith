use super::*;

pub(super) fn parse_proliferate_choose_phase_out_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_proliferate_choose_phase_out_tokens(tokens)?;
    let proliferated_tag = helper_tag_for_tokens(tokens, "proliferated_this_way");
    let chosen_tag = crate::tag::CompilerReferenceTag::It.bind();
    let selection_filter = shape.filter.match_tagged(
        proliferated_tag.clone(),
        TaggedOpbjectRelation::IsTaggedObject,
    );
    let phase_out_filter = ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .match_tagged(chosen_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
    Some(vec![
        // Proliferate already reports the permanents that actually received a
        // counter. Give that generic affected-object outcome a stable tag so
        // the later choice cannot include merely countered permanents that
        // were not selected for this proliferate action.
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_proliferate(Value::Fixed(1))),
            tag: proliferated_tag,
        },
        EffectAst::ChooseObjects {
            filter: selection_filter,
            count: shape.count,
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag,
        },
        EffectAst::subject_verb_phase_out_all(phase_out_filter),
    ])
}

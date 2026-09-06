use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::LibraryActionAst;
use super::*;

pub fn parse_exile_until_match_grant_play_this_turn(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                    mode: crate::cards::builders::LibraryConsultModeAst::Exile,
                    stop_rule,
                    ..
                }),
            ..
        })) if consult_stop_rule_is_single_match(stop_rule)
    ) {
        return Ok(None);
    }

    let Some(clause) = parse_consult_cast_clause(second) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.extend(consult_cast_effects(&clause, parts.match_tag)?);
    Ok(Some(effects))
}

pub fn parse_consult_match_into_hand_exile_others(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            }),
            ..
        }))
    ) {
        return Ok(None);
    }

    let (second_tokens, _gate_on_result) = strip_leading_if_you_do_sentence(second);
    if !effect_grammar::is_consult_hand_then_exile_others_shape(&second_tokens) {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(crate::tag::TagRef::of(parts.match_tag.clone()), None),
        Zone::Hand,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::TagRef::of(parts.all_tag),
        effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                crate::tag::CompilerReferenceTag::It.bind(),
                ObjectFilter::tagged(parts.match_tag),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::subject_verb_exile(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                false,
            )],
        })],
    }));
    Ok(Some(effects))
}

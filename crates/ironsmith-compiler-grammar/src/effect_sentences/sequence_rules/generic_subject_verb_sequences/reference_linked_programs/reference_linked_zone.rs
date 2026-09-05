use super::*;

pub fn parse_consult_match_move_all_to_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };

    if !effect_grammar::is_consult_move_all_to_graveyard_shape(second) {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(parts.all_tag, None),
        Zone::Graveyard,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    Ok(Some(effects))
}

/// Parses the two-sentence pattern:
///   S1: "Reveal cards from the top of your library until you reveal a <filter> card."
///   S2: "Put that card into your hand and all other cards revealed this way into your graveyard."
///
/// This covers cards like Hermit Druid and similar "reveal until, match to hand, rest to graveyard"
/// patterns.
pub fn parse_consult_match_into_hand_others_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some((parts, optional, gate_on_previous_result)) =
        parse_gated_optional_consult_traversal_sentence(first)?
    else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }

    let (second_tokens, gate_on_result) = strip_leading_if_you_do_sentence(second);
    if !effect_grammar::is_consult_hand_others_graveyard_shape(&second_tokens) {
        return Ok(None);
    }

    let followups = vec![
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag.clone(), None),
            Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::ForEachTagged {
            tag: parts.all_tag.clone(),
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    ObjectFilter::tagged(parts.match_tag.clone()),
                ),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    Zone::Graveyard,
                    false,
                    crate::cards::builders::ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        },
    ];
    if !optional && !gate_on_result && !gate_on_previous_result {
        return Ok(Some(vec![
            EffectAst::SourceSentence {
                effects: parts.effects,
                leading_then: false,
                starting_with_controller: false,
            },
            EffectAst::SourceSentence {
                effects: followups,
                leading_then: false,
                starting_with_controller: false,
            },
        ]));
    }

    Ok(Some(wrap_optional_consult_effects(
        parts,
        optional,
        followups,
        gate_on_result,
        gate_on_previous_result,
    )))
}


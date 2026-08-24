use super::*;


/// Parse a conjunction only when every arm independently proves either an
/// executable action or a complete negated restriction, and both kinds are
/// present.
///
/// This proof is shared by the direct sentence dispatcher and the complete
/// effect-body entrypoint. The latter probes tolerant whole-body specialists
/// before dispatching individual sentences; without this earlier bridge, a
/// broad restriction parser can absorb a preceding animation into the
/// restriction's subject filter.
pub(in super::super) fn parse_fully_typed_mixed_restriction_action_chain(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = super::super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut saw_restriction = false;
    let mut saw_affirmative_action = false;
    for (segment_idx, segment) in segments.iter().enumerate() {
        if super::super::super::activation_and_restrictions::find_negation_span(segment).is_some() {
            // This is a shape probe over one arm of a larger coordinated
            // sentence. A restriction parser may commit to the negation and
            // reject the isolated arm even though the intact chain parser
            // can bind its shared target. Treat that diagnostic as a probe
            // miss and leave the complete sentence available below.
            let standalone_restriction =
                matches!(parse_cant_effect_sentence_lexed(segment), Ok(Some(_)));
            let shared_subject_restriction = if !standalone_restriction && segment_idx > 0 {
                let previous = segments[segment_idx - 1];
                if let Some((_, verb_idx)) = super::super::lex_chain_helpers::find_verb_lexed(previous) {
                    let subject = &previous[..verb_idx];
                    if effect_grammar::chain_carry::parse_carryable_subject_tokens(subject)
                        .is_some()
                    {
                        let mut expanded = subject.to_vec();
                        expanded.extend(segment.iter().cloned());
                        matches!(
                            parse_cant_effect_sentence_lexed(&expanded),
                            Ok(Some(_))
                        )
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };
            if standalone_restriction || shared_subject_restriction {
                saw_restriction = true;
            } else {
                return Ok(None);
            }
        } else if super::super::lex_chain_helpers::segment_has_effect_head_lexed(segment) {
            saw_affirmative_action = true;
        } else {
            return Ok(None);
        }
    }

    if saw_restriction && saw_affirmative_action {
        // A bare prohibition after an animation inherits the animation's
        // subject ("this artifact becomes ... and can't be blocked").  The
        // ordinary chain parser cannot discover a verb in that second arm,
        // so its tolerant whole-clause `can't` fallback can absorb the
        // animation words into the restriction's object filter.  At this
        // point the two-arm shape and both typed halves have already been
        // proved; lower the animation and the expanded shared-subject
        // prohibition independently instead of asking the generic chain
        // heuristic to rediscover the boundary.
        if let [affirmative, restriction] = segments.as_slice()
            && let Some((super::super::Verb::Become, verb_word_idx)) =
                super::super::lex_chain_helpers::find_verb_lexed(affirmative)
        {
            let affirmative_words = TokenWordView::new(affirmative);
            let Some(verb_token_idx) =
                affirmative_words.map_word_or_end_to_token_boundary(verb_word_idx)
            else {
                return Ok(None);
            };
            let Some(body_token_idx) =
                affirmative_words.map_word_or_end_to_token_boundary(verb_word_idx + 1)
            else {
                return Ok(None);
            };
            let subject = trim_edge_punctuation(&affirmative[..verb_token_idx]);
            let body = trim_edge_punctuation(&affirmative[body_token_idx..]);
            if !subject.is_empty() && !body.is_empty() {
                let animation = super::super::clause_dispatch::parse_become_clause(&subject, &body)?;
                let mut expanded_restriction = subject;
                expanded_restriction.extend(restriction.iter().cloned());
                if let Some(mut restrictions) =
                    parse_cant_effect_sentence_lexed(&expanded_restriction)?
                {
                    let mut effects = Vec::with_capacity(1 + restrictions.len());
                    effects.push(animation);
                    effects.append(&mut restrictions);
                    return Ok(Some(vec![EffectAst::Coordinated {
                        effects,
                        leading_duration: false,
                        result_conjunction: false,
                    }]));
                }
            }
        }

        // The ordinary chain entrypoint decides whether to split from the
        // number of independently recognized effect heads. A subjectless
        // restriction arm ("and can't be blocked") deliberately has no
        // standalone head, so that heuristic can send this already-proven
        // mixed shape back to the broad `can't` parser and lose the leading
        // animation. Force the segmented carry route now that every arm and
        // the mixed-kind invariant have been proved above, then restore the
        // authored coordination surface.
        let effects = super::super::chain_carry::parse_effect_chain_inner_lexed(tokens)?;
        return Ok(Some(
            super::super::chain_carry::preserve_coordinated_effect_chain_surface(tokens, effects),
        ));
    }
    Ok(None)
}

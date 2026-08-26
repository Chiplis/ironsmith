use super::*;

pub(super) fn first_for_each_object_filter(effects: &[EffectAst]) -> Option<ObjectFilter> {
    for effect in effects {
        if let EffectAst::ForEachObject { filter, .. } = effect {
            return Some(filter.clone());
        }
        let mut found = None;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            if found.is_none() {
                found = first_for_each_object_filter(nested);
            }
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

pub(super) fn mark_matching_for_each_object_leading_then(
    effects: &mut [EffectAst],
    expected: &ObjectFilter,
) -> bool {
    for effect in effects {
        if let EffectAst::ForEachObject { filter, .. } = effect
            && filter == expected
            && !filter.has_for_each_leading_then_surface()
        {
            filter.set_for_each_leading_then_surface(true);
            return true;
        }
        let mut marked = false;
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            if !marked {
                marked = mark_matching_for_each_object_leading_then(nested, expected);
            }
        });
        if marked {
            return true;
        }
    }
    false
}

/// Parse one complete effect body while retaining every source sentence whose
/// boundary is stable under the same joint parse.
///
/// Parsing each sentence in isolation loses the discourse context needed by
/// followups such as "it", "those cards", and "this way". Instead, parse
/// successively longer prefixes and compare them with the corresponding
/// prefix of the whole-body AST. This keeps one shared semantic parse while
/// proving exactly where a later sentence did not rewrite or absorb an
/// earlier effect. Any cross-sentence structural rewrite falls back to the
/// ordinary flat program.
pub(crate) fn parse_effect_sentences_preserving_source_boundaries(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    fn contains_local_rewrite_dependency(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SelfReplacement { .. }
                | EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::RegisterZoneReplacement {
                        duration: crate::cards::builders::ZoneReplacementDurationAst::OneShot,
                        ..
                    },
                    ..
                })
        ) {
            return true;
        }
        let mut found = false;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            found |= nested.iter().any(contains_local_rewrite_dependency);
        });
        found
    }

    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .map(|sentence| sentence.to_vec())
        .collect::<Vec<_>>();
    if sentences.len() >= 2
        && effect_grammar::generic_sequence_shapes::parse_starting_each_player_optional_repeat_shape(
            &sentences[0],
            &sentences[1],
        )
        .is_some()
    {
        // The boundary-preserving fallback below strips the participant-order
        // prefix so ordinary per-sentence parsing can proceed. A repeat
        // sequence needs that prefix while its two authored sentences are
        // still adjacent: the second sentence is the first action's loop
        // terminator, not an independent each-player action.
        return parse_effect_sentences_lexed(tokens);
    }
    let mut parse_sentences = sentences.clone();
    let mut stripped_participant_ordering = false;
    if let Some(first) = parse_sentences.first_mut()
        && let Some((_, remainder)) = crate::grammar::primitives::strip_lexed_prefix_phrases(
            first,
            &[&["starting", "with", "you"]],
        )
    {
        *first = trim_lexed_commas(remainder).to_vec();
        stripped_participant_ordering = true;
    }
    let mut parsed_together = if stripped_participant_ordering {
        parse_effect_sentences_lexed(&join_sentences_with_period(&parse_sentences))?
    } else {
        parse_effect_sentences_lexed(tokens)?
    };
    // Cross-sentence self-replacement construction can rebuild a token-copy
    // action after the sentence-local parser has already discarded its
    // quoted exception. The complete authored token stream is still present
    // at this boundary, so reattach the typed inline rule before comparing
    // prefix parses. A changed joint AST intentionally falls back to the flat
    // program below, preserving the enriched replacement as one unit.
    crate::effect_sentences::attach_inline_token_granted_abilities_to_last_create(
        &mut parsed_together,
        tokens,
    );
    crate::effect_sentences::reconcile_inline_copy_self_replacement_grants(
        &mut parsed_together,
        tokens,
    );
    if sentences.len() < 2 {
        let Some(sentence) = sentences.first() else {
            return Ok(parsed_together);
        };
        let effects = crate::effect_sentences::preserve_coordinated_effect_chain_surface(
            sentence,
            parsed_together,
        );
        if stripped_participant_ordering {
            return Ok(vec![EffectAst::SourceSentence {
                effects,
                leading_then: false,
                starting_with_controller: true,
            }]);
        }
        return Ok(effects);
    }

    // Some authored follow-up sentences modify the preceding action instead
    // of adding a new top-level effect. Treat that exact typed attachment as
    // part of the preceding boundary group while proving source provenance:
    // parsing the preceding sentence alone must differ from the joint AST by
    // construction (the move has not acquired its entry state or grant yet).
    // Keeping the two sentences in one proof group still preserves an earlier
    // leading `Then` boundary and lets the optional procedure own the whole
    // follow-up at runtime.
    let mut boundary_parse_sentences = Vec::<Vec<OwnedLexToken>>::new();
    let mut boundary_surface_sentences = Vec::<Vec<OwnedLexToken>>::new();
    for (parse_sentence, surface_sentence) in parse_sentences
        .iter()
        .cloned()
        .zip(sentences.iter().cloned())
    {
        if effect_grammar::followup_shapes::parse_moved_object_entry_followup_shape(
            &surface_sentence,
        )
        .is_some()
            && let Some(previous) = boundary_parse_sentences.pop()
        {
            boundary_parse_sentences.push(join_sentences_with_period(&[previous, parse_sentence]));
            continue;
        }
        boundary_parse_sentences.push(parse_sentence);
        boundary_surface_sentences.push(surface_sentence);
    }

    let mut groups = Vec::with_capacity(boundary_parse_sentences.len());
    let mut previous_effect_count = 0usize;
    for prefix_len in 1..=boundary_parse_sentences.len() {
        let prefix_tokens = join_sentences_with_period(&boundary_parse_sentences[..prefix_len]);
        let Ok(parsed_prefix) = parse_effect_sentences_lexed(&prefix_tokens) else {
            return Ok(preserve_flat_leading_then_for_each_surface(
                &sentences,
                parsed_together,
            ));
        };
        let prefix_effect_count = parsed_prefix.len();
        if prefix_effect_count <= previous_effect_count
            || prefix_effect_count > parsed_together.len()
            || parsed_prefix.as_slice() != &parsed_together[..prefix_effect_count]
        {
            return Ok(preserve_flat_leading_then_for_each_surface(
                &sentences,
                parsed_together,
            ));
        }

        let sentence_effects = parsed_together[previous_effect_count..prefix_effect_count].to_vec();
        if previous_effect_count > 0
            && parsed_together[..previous_effect_count]
                .iter()
                .rev()
                .any(|effect| crate::effect_sentences::primary_target_from_effect(effect).is_some())
            && sentence_effects
                .iter()
                .any(crate::compile_support::effect_references_it_tag)
            && sentence_effects
                .iter()
                .any(contains_local_rewrite_dependency)
        {
            // The later sentence's demonstrative consumes an explicit target
            // introduced by an earlier sentence and installs a replacement
            // on that producer. That replacement must share one lowering
            // slice even though the later effects append cleanly to the joint
            // AST. Ordinary tagged consumers retain their source boundary;
            // global reference annotation carries those bindings safely.
            return Ok(preserve_flat_leading_then_for_each_surface(
                &sentences,
                parsed_together,
            ));
        }
        let sentence_effects = crate::effect_sentences::preserve_coordinated_effect_chain_surface(
            &boundary_surface_sentences[prefix_len - 1],
            sentence_effects,
        );
        let leading_then = token_word_refs(&boundary_surface_sentences[prefix_len - 1])
            .first()
            .is_some_and(|word| word.eq_ignore_ascii_case("then"));
        let sentence_words = token_word_refs(&boundary_surface_sentences[prefix_len - 1]);
        let starting_with_controller = sentence_words.get(..3).is_some_and(|words| {
            words[0].eq_ignore_ascii_case("starting")
                && words[1].eq_ignore_ascii_case("with")
                && words[2].eq_ignore_ascii_case("you")
        });
        groups.push(EffectAst::SourceSentence {
            effects: sentence_effects,
            leading_then,
            starting_with_controller,
        });
        previous_effect_count = prefix_effect_count;
    }

    if previous_effect_count == parsed_together.len() {
        Ok(groups)
    } else {
        Ok(preserve_flat_leading_then_for_each_surface(
            &sentences,
            parsed_together,
        ))
    }
}

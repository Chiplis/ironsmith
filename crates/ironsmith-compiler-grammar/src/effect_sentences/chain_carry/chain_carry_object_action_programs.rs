use super::*;

pub(super) fn parse_tap_those_then_unattach_equipment_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !chain_grammar::parse_tap_then_unattach_tokens(tokens) {
        return Ok(None);
    }

    let mut tapped_filter = ObjectFilter::creature();
    tapped_filter.zone = Some(Zone::Battlefield);
    tapped_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut equipment_filter = ObjectFilter::permanent();
    equipment_filter.card_types.push(CardType::Artifact);
    equipment_filter.subtypes.push(Subtype::Equipment);
    equipment_filter.zone = Some(Zone::Battlefield);
    equipment_filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::AttachedToTaggedObject,
        });

    Ok(Some(vec![
        EffectAst::subject_verb_tap(TargetAst::Object(tapped_filter, None, None)),
        EffectAst::subject_verb_unattach(TargetAst::WithCount(
            Box::new(TargetAst::Object(equipment_filter, None, None)),
            ChoiceCount::any_number(),
        )),
    ]))
}

pub fn collapse_token_copy_next_end_step_exile_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let facts = chain_grammar::parse_delayed_copy_facts_tokens(tokens);
    let Some(chain_grammar::DelayedCopyTiming::EndStep { player_is_you }) = facts.timing else {
        return;
    };
    if !facts.has_exile || !facts.has_token {
        return;
    }
    let next_end_step_player = if player_is_you {
        PlayerFilter::You
    } else {
        PlayerFilter::Any
    };

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenCopy { .. }
                        | SubjectVerbActionAst::CreateTokenCopyFromSource { .. },
                    ..
                }),
                EffectAst::SubjectVerb(subject_verb),
            ) => match &subject_verb.action {
                SubjectVerbActionAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                }
                | SubjectVerbActionAst::Exile { target, .. } => {
                    target_is_generic_token_filter(target)
                }
                _ => false,
            },
            _ => false,
        };

        if !mark_next_end_step_exile {
            idx += 1;
            continue;
        }

        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenCopy {
                    exile_at_next_end_step,
                    exile_at_next_end_step_reference_surface,
                    next_end_step_player: effect_next_end_step_player,
                    ..
                }
                | SubjectVerbActionAst::CreateTokenCopyFromSource {
                    exile_at_next_end_step,
                    exile_at_next_end_step_reference_surface,
                    next_end_step_player: effect_next_end_step_player,
                    ..
                },
            ..
        }) = &mut effects[idx]
        {
            *exile_at_next_end_step = true;
            *exile_at_next_end_step_reference_surface =
                token_copy_action_reference_surface(tokens, "exile");
            *effect_next_end_step_player = next_end_step_player.clone();
        }
        effects.remove(idx + 1);
    }
}

pub fn expand_segments_with_multi_create_clauses_lexed(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let Some((Verb::Create, _)) = find_verb_lexed(&segment) else {
            expanded.push(segment);
            continue;
        };
        let has_token_rules_tail = chain_grammar::has_token_rules_tail_tokens(&segment);
        if has_token_rules_tail {
            expanded.push(segment);
            continue;
        }
        let token_mentions = chain_grammar::count_token_mentions(&segment);
        if token_mentions < 2 {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon_lexed(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<OwnedLexToken>> = Vec::new();
        for raw_part in comma_parts {
            let mut part = trim_lexed_commas(&raw_part).to_vec();
            while let Some(rest) = chain_grammar::strip_leading_and_tokens(&part) {
                part = rest.to_vec();
            }
            if part.is_empty() {
                continue;
            }
            if let Some(previous) = local_parts.last()
                && is_token_creation_context(previous)
                && starts_with_inline_token_rules_tail(&part)
            {
                if let Some(last) = local_parts.last_mut() {
                    last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                    last.extend(part);
                }
                continue;
            }
            if segment_has_effect_head_lexed(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment_lexed(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }
            if let Some(last) = local_parts.last_mut() {
                last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                last.extend(part);
            } else {
                local_parts.push(part);
            }
        }

        if local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

pub fn collapse_token_copy_next_end_step_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_next_end_step_exile_followup_lexed(effects, tokens);
}

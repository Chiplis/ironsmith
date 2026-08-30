use super::*;

pub(super) fn pre_rule_draw_count_demonstrative_gain_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = LexedClause::new(sentence_tokens).word_refs();
    if !starts_with_demonstrative_object_gain(&words) {
        return Ok(None);
    }
    // "Those tokens gain ..." after a token-producing loop refers to the
    // created tokens, which the token-followup grammar binds. The iterated
    // collection filter would wrongly re-address the loop's source objects.
    if matches!(words.get(1), Some(&"token") | Some(&"tokens"))
        && last_effect_creates_tokens(state.effects)
    {
        return Ok(None);
    }
    let Some(filter) = last_demonstrative_collection_filter(state.effects) else {
        return Ok(None);
    };
    let Some(effect) = build_grant_all_from_demonstrative_gain(filter, sentence_tokens)? else {
        return Ok(None);
    };
    state.effects.push(effect);
    *state.carried_context = None;
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: Some("subject-verb verb=Grant subject=demonstrative recognizer=draw-count-followup"),
    }))
}

pub(super) fn effects_contain_gain_life(effects: &[EffectAst]) -> bool {
    for effect in effects {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GainLife { .. },
                ..
            })
        ) {
            return true;
        }
        let mut nested_gain = false;
        for_each_nested_effects(effect, true, |nested| {
            if !nested_gain {
                nested_gain = effects_contain_gain_life(nested);
            }
        });
        if nested_gain {
            return true;
        }
    }
    false
}

/// Preserve the exact result of a correlated plural sacrifice.
///
/// In a sequence such as "for each player, choose target permanent that
/// player controls. Those players sacrifice those permanents", the second
/// sentence is not a new sacrifice choice. It consumes the preceding target
/// set, partitioned by the iterated sacrificing player. Tag the action's
/// actual affected objects so a later "player who sacrificed a permanent
/// this way" predicate observes the sacrifice result, not merely the earlier
/// target declaration.
pub(super) fn post_rule_correlated_plural_sacrifice_result(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    if !crate::word_primitives::parse_any_sequence_complete(
        &words,
        &[
            &["those", "players", "sacrifice", "those", "permanents"],
            &["those", "players", "sacrifice", "those", "creatures"],
            &["those", "players", "sacrifice", "those", "tokens"],
        ],
    ) || !matches!(state.effects.last(), Some(EffectAst::ForEachPlayer { .. }))
    {
        return Ok(None);
    }

    let [effect] = sentence_effects.as_mut_slice() else {
        return Ok(None);
    };
    let sacrifice = match effect {
        EffectAst::ForEachPlayer { effects } => {
            let [sacrifice] = effects.as_mut_slice() else {
                return Ok(None);
            };
            sacrifice
        }
        EffectAst::SubjectVerb(_) => effect,
        _ => return Ok(None),
    };
    let consumes_prior_result = matches!(
        sacrifice,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SacrificeAll { filter },
            ..
        }) if filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        })
    );
    if !consumes_prior_result {
        return Ok(None);
    }

    let result_tag = crate::util::helper_tag_for_tokens(sentence_tokens, "sacrificed");
    let mut sacrifice = sacrifice.clone();
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst { subject, .. }) = &mut sacrifice {
        subject.player = PlayerAst::That;
    }
    let tagged = EffectAst::TagAffected {
        effect: Box::new(sacrifice),
        tag: result_tag,
    };
    *effect = EffectAst::ForEachPlayer {
        effects: vec![tagged],
    };
    Ok(Some(PostParseFollowupResult::Annotated))
}

/// Connect a typed `for each ... sacrificed this way` iterator to the exact
/// result set of the preceding each-player sacrifice.  Wrapping the complete
/// player loop is important: one shared tag must contain every player's
/// affected objects rather than being overwritten once per player.
pub(super) fn post_rule_typed_sacrificed_result_iterator(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    if !crate::word_primitives::parse_sequence_prefix(&words, &["for", "each"])
        || !crate::word_primitives::sequence_occurs(&words, &["sacrificed", "this", "way"])
    {
        return Ok(None);
    }

    let [EffectAst::ForEachTagged { tag, .. }] = sentence_effects.as_mut_slice() else {
        return Ok(None);
    };
    if tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() {
        return Ok(None);
    }

    let Some(previous) = state.effects.last_mut() else {
        return Ok(None);
    };
    let is_each_player_sacrifice_all = matches!(
        previous,
        EffectAst::ForEachPlayer { effects }
            if matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::SacrificeAll { .. },
                    ..
                })]
            )
    );
    if !is_each_player_sacrifice_all {
        return Ok(None);
    }

    if let Some(previous_sentence) = sentence_idx
        .checked_sub(1)
        .and_then(|index| sentences.get(index))
        && let EffectAst::ForEachPlayer { effects } = previous
        && let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::SacrificeAll { filter },
                ..
            }),
        ] = effects.as_mut_slice()
    {
        super::super::super::zone_handlers::preserve_terminal_nonbasic_land_union(
            previous_sentence.lexed(),
            filter,
        );
    }

    let result_tag = crate::util::helper_tag_for_tokens(sentence_tokens, "sacrificed");
    let previous_effect = previous.clone();
    *previous = EffectAst::TagAffected {
        effect: Box::new(previous_effect),
        tag: result_tag.clone(),
    };
    *tag = result_tag;
    Ok(Some(PostParseFollowupResult::Annotated))
}

pub(super) fn bind_prior_exiled_mana_value(value: &mut Value) {
    match value {
        Value::SurfaceHinted { value, .. } => bind_prior_exiled_mana_value(value),
        Value::ManaValueOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()) =>
        {
            **spec = ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::PriorExiledCard.key());
        }
        _ => {}
    }
}

fn last_effect_creates_tokens(effects: &[EffectAst]) -> bool {
    fn creates_tokens(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CreateTokenWithMods { .. }
                    | SubjectVerbActionAst::CreateTokenCopy { .. }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource { .. }
                    | SubjectVerbActionAst::CreateTokenChoice { .. },
                ..
            })
        ) {
            return true;
        }
        let mut found = false;
        crate::effect_ast_traversal::for_each_nested_effects(effect, true, |nested| {
            found |= nested.iter().any(creates_tokens);
        });
        found
    }
    effects.last().is_some_and(creates_tokens)
}

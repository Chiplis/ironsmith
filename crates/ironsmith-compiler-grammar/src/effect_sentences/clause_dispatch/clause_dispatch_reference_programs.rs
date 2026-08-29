use super::*;

pub(in super::super) fn parse_hexproof_targeting_override_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let (duration, clause_tokens) =
        parse_restriction_duration(tokens)?.unwrap_or((Until::Forever, tokens.to_vec()));
    let Some(spec) = parse_targeting_as_though_no_ability_spec(&clause_tokens)? else {
        return Ok(None);
    };
    Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
        TargetAst::Source(None),
        vec![GrantedAbilityAst::StaticAbility(Box::new(
            crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::targeting_as_though_no_ability(spec),
            ),
        ))],
        duration,
    )))
}

pub fn parse_targeting_as_though_no_ability_spec(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<ironsmith_core::static_ability_model::TargetingAsThoughNoAbilitySpec>,
    CardTextError,
> {
    let tokens = trim_edge_punctuation(tokens);
    let words = tokens
        .iter()
        .filter_map(|token| token.as_word().map(|_| token.parser_text()))
        .collect::<Vec<_>>();
    let Some((_, can)) = crate::word_primitives::find_any_phrase_start(
        &words,
        &[
            &["can", "be", "the", "target"],
            &["can", "be", "the", "targets"],
        ],
    ) else {
        return Ok(None);
    };
    let Some(as_though) = crate::word_primitives::parse_sequence_start(&words, &["as", "though"])
    else {
        return Ok(None);
    };
    if can == 0 || as_though <= can + 4 {
        return Ok(None);
    }
    let ignored_ability =
        if crate::word_primitives::sequence_occurs(&words[as_though..], &["hexproof"]) {
            StaticAbilityId::Hexproof
        } else if crate::word_primitives::sequence_occurs(&words[as_though..], &["shroud"]) {
            StaticAbilityId::Shroud
        } else {
            return Ok(None);
        };
    let permission_words = &words[can + 4..as_though];
    if !crate::word_primitives::parse_any_sequence_prefix(
        permission_words,
        &[
            &["of", "spells", "and", "abilities"],
            &["of", "spells", "or", "abilities"],
        ],
    ) {
        return Ok(None);
    }
    let sources_controlled_by =
        if crate::word_primitives::parse_sequence_suffix(permission_words, &["you", "control"]) {
            PlayerFilter::You
        } else if crate::word_primitives::parse_sequence_suffix(
            permission_words,
            &["controlled", "by", "target", "player"],
        ) {
            PlayerFilter::Target(Box::new(PlayerFilter::Any))
        } else if permission_words.len() == 4 {
            PlayerFilter::Any
        } else {
            return Ok(None);
        };

    let subject_words = &words[..can];
    let (players, object_start) = if crate::word_primitives::parse_sequence_prefix(
        subject_words,
        &["your", "opponents", "and"],
    ) {
        (Some(PlayerFilter::Opponent), 3)
    } else if crate::word_primitives::parse_sequence_complete(subject_words, &["your", "opponents"])
    {
        (Some(PlayerFilter::Opponent), subject_words.len())
    } else {
        (None, 0)
    };
    let object_tokens = &tokens[object_start..can];
    let objects = if object_tokens.is_empty() {
        None
    } else if crate::util::is_source_reference_words(
        &object_tokens
            .iter()
            .filter_map(|token| token.as_word().map(|_| token.parser_text()))
            .collect::<Vec<_>>(),
    ) {
        Some(ObjectFilter::source())
    } else {
        match parse_object_filter(object_tokens, false) {
            Ok(filter) => Some(filter),
            Err(_) if crate::lexer::is_authored_proper_name_phrase(object_tokens) => {
                Some(ObjectFilter::source())
            }
            Err(error) => return Err(error),
        }
    };
    if objects.is_none() && players.is_none() {
        return Ok(None);
    }

    Ok(Some(
        ironsmith_core::static_ability_model::TargetingAsThoughNoAbilitySpec {
            objects,
            players,
            sources_controlled_by,
            ignored_ability,
            display: crate::lexer::render_token_slice(&tokens),
        },
    ))
}

use super::*;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(effect) = parse_until_your_next_turn(tokens) {
        return Ok(Some(effect));
    }
    let Some(shape) = crate::grammar::keyword_static_lines::parse_damage_multiplier_tokens(tokens)
    else {
        return Ok(None);
    };
    if !shape.this_turn || shape.condition_tokens.is_some() {
        return Ok(None);
    }
    // Resolved declarations in this family name classes of sources and
    // recipients, not context-local tags or future attachment choices.
    let source = crate::lexer::parser_token_word_refs(shape.source.filter_tokens);
    // "a source you control" / "any source you control" (Insult, Isengard
    // Unleashed): an articled source noun names every source.
    let articled_source_noun =
        shape.source.source_noun && matches!(source.as_slice(), ["a"] | ["any"]);
    if !shape.source.trailing_filter_tokens.is_empty()
        || !(articled_source_noun
            || matches!(source.as_slice(), [] | ["a", "creature"] | ["creature"]))
        || (!shape.source.source_noun && source.is_empty())
    {
        return Ok(None);
    }
    let recipient = shape
        .damaged_tokens
        .map(crate::lexer::parser_token_word_refs)
        .unwrap_or_default();
    if !matches!(
        recipient.as_slice(),
        ["a", "creature"]
            | ["creature"]
            | ["an", "opponent"]
            | ["a", "player"]
            | ["a", "permanent", "or", "player"]
            | ["a", "permanent", "or", "a", "player"]
            | ["an", "opponent", "or", "a", "permanent", "an", "opponent", "controls"]
    ) && !(recipient.is_empty() && shape.damaged_tokens.is_none())
    {
        return Ok(None);
    }
    let Some(spec) = crate::keyword_static::damage_multiplier_parts_from_shape(shape)? else {
        return Ok(None);
    };
    Ok(Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDamageMultiplier { spec }),
    )))
}

/// "Until your next turn, if a source would deal damage to that player or a
/// permanent that player controls, it deals double that damage instead."
/// (Lightning, Army of One) / "Until your next turn, if that creature would
/// deal combat damage to one of your opponents, it deals triple that damage to
/// that player instead." (Jeska, Thrice Reborn): a resolution-registered
/// multiplier lasting until the controller's next turn (CR 611.2a, 614.1a).
/// "that player" and "that creature" name the ability's antecedents; the
/// registration fixes them as it resolves.
fn parse_until_your_next_turn(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let (_, rest) = crate::grammar::primitives::parse_prefix(
        tokens,
        (
            crate::grammar::primitives::phrase(&["until", "your", "next", "turn"]),
            winnow::combinator::opt(crate::grammar::primitives::comma()),
        ),
    )?;
    let shape = crate::grammar::keyword_static_lines::parse_damage_multiplier_tokens(rest)?;
    if shape.this_turn || shape.condition_tokens.is_some() || !shape.source.trailing_filter_tokens.is_empty() {
        return None;
    }
    let source_words = crate::lexer::parser_token_word_refs(shape.source.filter_tokens);
    let source_filter = match (source_words.as_slice(), shape.source.source_noun) {
        ([] | ["a"] | ["any"], true) => ObjectFilter::default(),
        (["that", "creature"], false) => {
            let mut filter =
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
            filter.card_types = vec![crate::types::CardType::Creature];
            filter
        }
        _ => return None,
    };
    if !matches!(
        shape.source.controller,
        crate::grammar::keyword_static_lines::DamageSourceControllerKind::None
    ) {
        return None;
    }
    let recipient = shape
        .damaged_tokens
        .map(crate::lexer::parser_token_word_refs)
        .unwrap_or_default();
    let (target_player_filter, target_object_filter) = match recipient.as_slice() {
        ["that", "player", "or", "a", "permanent", "that", "player", "controls"] => (
            Some(PlayerFilter::IteratedPlayer),
            Some(ObjectFilter::permanent().controlled_by(PlayerFilter::IteratedPlayer)),
        ),
        ["one", "of", "your", "opponents"] | ["an", "opponent"] => {
            (Some(PlayerFilter::Opponent), None)
        }
        ["a", "player"] => (Some(PlayerFilter::Any), None),
        _ => return None,
    };
    let repeated = shape
        .repeated_target_tokens
        .map(crate::lexer::parser_token_word_refs)
        .unwrap_or_default();
    let repeated_ok = match repeated.as_slice() {
        [] => true,
        ["that", "player"] => target_object_filter.is_none(),
        ["that", "player", "or", "permanent"] | ["that", "permanent", "or", "player"] => {
            target_object_filter.is_some()
        }
        _ => false,
    };
    if !repeated_ok {
        return None;
    }
    Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDamageMultiplier {
            spec: ironsmith_core::RegisterDamageMultiplierEffect {
                source_filter,
                target_player_filter,
                target_object_filter,
                factor: shape.factor,
                combat_only: shape.combat_only,
                noncombat_only: shape.noncombat_only,
                mode: ironsmith_core::ReplacementApplyMode::UntilYourNextTurn,
            },
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolving_multiplier_retains_duration_source_and_recipient_domains() {
        for (text, combat, opponent) in [
            (
                "If a creature would deal combat damage to a creature this turn, it deals double that damage to that creature instead.",
                true,
                false,
            ),
            (
                "If a source you control would deal damage to an opponent this turn, it deals double that damage to that player instead.",
                false,
                true,
            ),
            (
                "If any source you control would deal damage to a permanent or player this turn, it deals double that damage to that permanent or player instead.",
                false,
                false,
            ),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let effect = parse(&tokens).unwrap().unwrap();
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDamageMultiplier {
                        spec,
                    }),
                ..
            }) = effect
            else {
                panic!("not a registration");
            };
            assert_eq!(
                spec.mode,
                ironsmith_core::ReplacementApplyMode::UntilEndOfTurn
            );
            assert_eq!(spec.factor, 2);
            assert_eq!(spec.combat_only, combat);
            if opponent {
                assert_eq!(spec.target_player_filter, Some(PlayerFilter::Opponent));
            }
            if !combat {
                assert_eq!(spec.source_filter, ObjectFilter::default().you_control());
            }
        }
    }
}

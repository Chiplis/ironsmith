use super::*;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
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
    if !shape.source.trailing_filter_tokens.is_empty()
        || !matches!(source.as_slice(), [] | ["a", "creature"] | ["creature"])
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
        // "If a source you control would deal damage this turn, it deals
        // double that damage instead." (Insult): any recipient.
        []
            | ["an", "opponent", "or", "a", "permanent", "an", "opponent", "controls"]
            | ["a", "creature"]
            | ["creature"]
            | ["an", "opponent"]
            | ["a", "player"]
            | ["a", "permanent", "or", "player"]
            | ["a", "permanent", "or", "a", "player"]
    ) {
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

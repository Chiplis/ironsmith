use super::*;

/// The complete plural actor owns its source declarations and one recipient.
/// Preserve separate target groups instead of splitting them at `and` or
/// lowering a per-source serial loop.
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some((start, end)) = find_token_word_sequence_span(
        &tokens,
        &[
            "each", "deal", "damage", "equal", "to", "their", "power", "to",
        ],
    ) else {
        return Ok(None);
    };
    let actors = &tokens[..start];
    if actors.is_empty() || end == tokens.len() {
        return Ok(None);
    }
    let mut sources = Vec::new();
    if crate::lexer::parser_token_word_refs(actors) == ["they"] {
        sources.push(TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(actors),
        ));
    } else {
        let pieces = actors
            .split(|token| token.is_word("and"))
            .collect::<Vec<_>>();
        // A second group has its own explicit cardinality and target noun.
        // Shared color/type conjunctions are deliberately left to their owner.
        if pieces.len() > 2
            || pieces
                .iter()
                .any(|piece| !piece.iter().any(|token| token.is_word("target")))
        {
            return Ok(None);
        }
        for piece in pieces {
            sources.push(parse_target_phrase(piece)?);
        }
    }
    let target = parse_target_phrase(&tokens[end..])?;
    Ok(Some(vec![EffectAst::subject_verb(
        crate::cards::builders::SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Damage(DamageActionAst::DealDamageBySources {
            sources,
            source_binding: ironsmith_core::DamageSourceSetBinding::LiveMembers,
            amount: Value::SourcePower,
            target,
        }),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_groups_keep_their_independent_declaration_cardinality() {
        for (text, groups) in [
            (
                "Up to two target creatures you control each deal damage equal to their power to another target creature.",
                1,
            ),
            (
                "Two target creatures your team controls each deal damage equal to their power to target creature.",
                1,
            ),
            (
                "Target creature you control and up to one other target legendary creature you control each deal damage equal to their power to target creature you don't control.",
                2,
            ),
            (
                "Any number of target enchanted creatures you control and up to one other target creature you control each deal damage equal to their power to target creature you don't control.",
                2,
            ),
            (
                "They each deal damage equal to their power to another target creature, planeswalker, or battle.",
                1,
            ),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
                crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
            });
            let effects = result.unwrap();
            assert!(!loss.is_lossy(), "{text}: {}", loss.reasons_text());
            let [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::Damage(DamageActionAst::DealDamageBySources {
                            sources,
                            amount,
                            ..
                        }),
                    ..
                }),
            ] = effects.as_slice()
            else {
                panic!("{effects:#?}")
            };
            assert_eq!(sources.len(), groups);
            assert_eq!(amount, &Value::SourcePower);
        }
    }
    #[test]
    fn other_amounts_or_serial_actors_are_not_substituted() {
        for text in [
            "Up to two target creatures each deal damage equal to their toughness to target creature.",
            "Target creature deals damage equal to its power to another target creature.",
            "Two target creatures each deal damage equal to their combined power to target creature.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(parse(&tokens).unwrap().is_none());
        }
    }
}

use super::*;

/// A complete shared-amount clause, before `and` can split the recipients
/// into separate instructions. This bounded form declares no new targets.
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some((_, deal_index)) = find_verb(&tokens) else {
        return Ok(None);
    };
    if !tokens[deal_index].is_any_word(&["deal", "deals"]) {
        return Ok(None);
    }
    let source = non_article_token_word_refs(&tokens[..deal_index]);
    if !matches!(
        source.as_slice(),
        ["this"]
            | ["this", "spell"]
            | ["this", "creature"]
            | ["this", "artifact"]
            | ["this", "enchantment"]
            | ["this", "permanent"]
    ) {
        return Ok(None);
    }
    let after = &tokens[deal_index + 1..];
    if after.len() < 3 || !after[0].is_word("damage") || !after[1].is_word("to") {
        return Ok(None);
    }
    let Some((equal_start, equal_end)) = find_token_word_sequence_span(after, &["equal", "to"])
    else {
        return Ok(None);
    };
    let target_tokens = &after[2..equal_start];
    if target_tokens.iter().any(|token| token.is_word("target")) {
        return Ok(None);
    }

    let Some(and) = target_tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    let pieces = [&target_tokens[..and], &target_tokens[and + 1..]];
    let mut recipients = Vec::new();
    let mut object_groups = Vec::new();
    let mut player_groups = Vec::new();
    for piece in pieces {
        let words = crate::lexer::parser_token_word_refs(piece);
        match words.as_slice() {
            ["each", "player"] => player_groups.push(PlayerFilter::Any),
            ["each", "opponent"] => player_groups.push(PlayerFilter::Opponent),
            ["each", ..] => object_groups.push(parse_object_filter(&piece[1..], false)?),
            ["that", "creature"] | ["that", "permanent"] | ["that", "player"] | ["you"] => {
                recipients.push(parse_target_phrase(piece)?)
            }
            _ => return Ok(None),
        }
    }
    let amount_tokens = &after[equal_end..];
    let Some((amount, used)) =
        crate::grammar::shared_util::value_expr::parse_value_expr_tokens(amount_tokens)
    else {
        return Ok(None);
    };
    if used != amount_tokens.len() {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::subject_verb(
        crate::cards::builders::SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Damage(DamageActionAst::DealDamageToRecipients {
            amount,
            recipients,
            object_groups,
            player_groups,
        }),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_shared_suffix_quantity_is_one_typed_recipient_set() {
        for (text, references) in [
            (
                "This spell deals damage to that creature and that player equal to the revealed card's mana value.",
                true,
            ),
            (
                "This spell deals damage to each creature and each player equal to the number of Mountains put into a graveyard this way.",
                false,
            ),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let effects = parse(&tokens).unwrap().unwrap();
            assert_eq!(effects.len(), 1);
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Damage(DamageActionAst::DealDamageToRecipients {
                        amount,
                        recipients,
                        object_groups,
                        player_groups,
                    }),
                ..
            }) = &effects[0]
            else {
                panic!("not a shared packet");
            };
            assert!(matches!(
                amount.unhinted(),
                Value::PendingPriorEffectMetric(_)
            ));
            assert_eq!(recipients.len(), if references { 2 } else { 0 });
            assert_eq!(object_groups.len(), usize::from(!references));
            assert_eq!(player_groups.len(), usize::from(!references));
        }
    }
    #[test]
    fn references_cannot_become_fresh_targets_or_change_the_damage_source() {
        for text in [
            "This spell deals damage to target creature and that player equal to 3.",
            "That creature deals damage to each creature and each player equal to 3.",
            "Each creature deals damage to each creature and each player equal to 3.",
            "This spell deals damage to each creature and each player equal to 3 until end of turn.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(parse(&tokens).unwrap().is_none(), "{text}");
        }
    }
}

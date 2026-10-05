//! A leading explicit mixed target remains the recipient through intervening
//! draw, discard, reveal and mill instructions. Those producers own their card
//! references; they cannot replace the earlier declared recipient.
use crate::cards::builders::*;
use crate::lexer::{OwnedLexToken, TokenKind};

fn target_words<'a>(target: &TargetAst, tokens: &'a [OwnedLexToken]) -> Vec<&'a str> {
    if let TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) = target {
        return target_words(inner, tokens);
    }
    let span = match target {
        TargetAst::Tagged(tag, Some(span))
            if crate::tag::CompilerReferenceTag::It.matches(&tag.key) =>
        {
            span
        }
        TargetAst::Object(filter, Some(span), _)
            if filter.tagged_constraints.iter().any(|constraint| {
                crate::tag::CompilerReferenceTag::It.matches(&constraint.tag)
            }) =>
        {
            span
        }
        _ => return Vec::new(),
    };
    tokens
        .iter()
        .filter(|token| {
            token.span.line == span.line
                && token.span.start >= span.start
                && token.span.end <= span.end
        })
        .filter_map(|token| token.as_word())
        .collect()
}
fn bind_recipient(effect: &mut EffectAst, declared: &TargetAst, tokens: &[OwnedLexToken]) -> usize {
    let mut replaced = 0;
    if let EffectAst::SubjectVerb(subject) = effect {
        if let SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { amount, filter }) =
            &subject.action
            && matches!(
                declared,
                TargetAst::WithCountValue(..) | TargetAst::WithCount(..)
            )
            && filter.tagged_constraints.iter().any(|constraint| {
                crate::tag::CompilerReferenceTag::It.matches(&constraint.tag)
                    && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
            })
        {
            subject.action = SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                amount: amount.clone(),
                target: declared.clone(),
                unpreventable: false,
            });
            replaced += 1;
        }
        let target = match &mut subject.action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { target, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                target,
                ..
            })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage {
                target, ..
            }) => Some(target),
            _ => None,
        };
        if let Some(target) = target {
            let words = target_words(target, tokens);
            let singular = matches!(
                words.as_slice(),
                ["that", "permanent" | "creature", "or", "player"]
            );
            let declared_set = matches!(
                declared,
                TargetAst::WithCountValue(..) | TargetAst::WithCount(..)
            );
            let plural =
                declared_set && matches!(words.as_slice(), ["each", "of", "them"] | ["them"]);
            if singular || plural {
                *target = declared.clone();
                replaced += 1;
            }
        }
    }
    crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
        for child in nested {
            replaced += bind_recipient(child, declared, tokens);
        }
    });
    replaced
}
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.len() < 5
        || !tokens[0].is_word("choose")
        || !tokens[1].is_word("any")
        || !tokens[2].is_word("target")
    {
        return Ok(None);
    }
    let span = crate::util::span_from_tokens(&tokens[1..3]);
    let (declaration, tail) = if tokens[3].kind == TokenKind::Period {
        (
            EffectAst::subject_verb_explicit_target_only(TargetAst::AnyTarget(span)),
            &tokens[4..],
        )
    } else if tokens[3].is_comma() && tokens[4].is_word("then") {
        if tokens.get(5).is_some_and(|token| token.is_word("choose")) {
            let Some(end) = tokens
                .iter()
                .position(|token| token.kind == TokenKind::Period)
            else {
                return Ok(None);
            };
            let Some(mut prelude) =
                super::clause_pattern_helpers::parse_choose_target_prelude_sentence(
                    &tokens[..=end],
                )?
            else {
                return Ok(None);
            };
            if prelude.len() != 1 {
                return Ok(None);
            }
            (prelude.remove(0), &tokens[end + 1..])
        } else {
            (
                EffectAst::subject_verb_explicit_target_only(TargetAst::AnyTarget(span)),
                &tokens[5..],
            )
        }
    } else {
        return Ok(None);
    };
    if tail.is_empty() {
        return Ok(None);
    }
    // Another independent target declaration needs its own discourse binding;
    // this complete-program reading deliberately does not guess between them.
    if tail.iter().any(|token| token.is_word("target")) {
        return Ok(None);
    }
    let EffectAst::SubjectVerb(subject) = &declaration else {
        return Ok(None);
    };
    let SubjectVerbActionAst::TargetOnly {
        target: declared, ..
    } = &subject.action
    else {
        return Ok(None);
    };
    let mut body = match super::bundle_rules::parse_consult_disposition_bundle(tail) {
        Some(effects) => effects,
        None => super::parse_effect_sentences_lexed(tail)?,
    };
    let replacements = body
        .iter_mut()
        .map(|effect| bind_recipient(effect, declared, tail))
        .sum::<usize>();
    if replacements == 0 {
        return Ok(None);
    }
    let mut result = vec![declaration];
    result.append(&mut body);
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_recipient_is_not_the_intervening_discarded_or_revealed_card() {
        for text in [
            "Choose any target. Draw three cards, then discard a card. This spell deals damage equal to the discarded card's mana value to that permanent or player.",
            "Choose any target, then mill three cards. This permanent deals damage to that permanent or player equal to the greatest mana value among the milled cards.",
            "Choose any target. Scry 3, then reveal the top card of your library. This spell deals damage equal to that card's mana value to that permanent or player.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let effects = parse(&tokens).unwrap().unwrap();
            assert!(
                matches!(&effects[0],EffectAst::SubjectVerb(s) if matches!(s.action,SubjectVerbActionAst::TargetOnly{explicit_declaration:true,..}))
            );
            let debug = format!("{effects:?}");
            assert!(debug.matches("AnyTarget").count() >= 2, "{debug}");
        }
        let comet=crate::lexer::lex_line("Choose any target, then choose another target for each time this spell was kicked. This spell deals X damage to each of them.",0).unwrap();
        let result = parse(&comet).unwrap().unwrap();
        let debug = format!("{result:?}");
        assert_eq!(debug.matches("KickCount").count(), 2, "{debug}");
        assert!(
            parse(
                &crate::lexer::lex_line("Choose any target. Unrecognized instruction.", 0).unwrap()
            )
            .is_err()
        );
        assert!(
            parse(
                &crate::lexer::lex_line(
                    "This spell deals 2 damage to that permanent or player.",
                    0
                )
                .unwrap()
            )
            .unwrap()
            .is_none()
        );
    }
}

use crate::cards::builders::{IfResultPredicate, PlayerAst, PredicateAst};
use ironsmith_core::{
    PriorEffectAction, PriorEffectResultActor, PriorEffectResultQuantifier,
    PriorEffectResultSurface,
};
use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalResultSubject {
    If,
    When,
    You,
    They,
    Player,
    Players,
    ThatPlayer,
    FirstPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalResultShape {
    ThisWay {
        subject: ModalResultSubject,
        negated: bool,
    },
    ExactNegated {
        subject: ModalResultSubject,
    },
}

fn normalized_word_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    tokens
        .iter()
        .filter(|token| match token.kind {
            TokenKind::Word => token
                .as_word()
                .is_some_and(|word| leaf::parse_leaf_article_complete(word).is_err()),
            TokenKind::Number | TokenKind::Tilde | TokenKind::Half => true,
            _ => false,
        })
        .cloned()
        .collect()
}

fn parse_subject<'a>(input: &mut LexStream<'a>) -> WResult<ModalResultSubject> {
    alt((
        primitives::phrase(&["that", "player"]).value(ModalResultSubject::ThatPlayer),
        primitives::phrase(&["first", "player"]).value(ModalResultSubject::FirstPlayer),
        primitives::kw("if").value(ModalResultSubject::If),
        primitives::kw("when").value(ModalResultSubject::When),
        primitives::kw("you").value(ModalResultSubject::You),
        primitives::kw("they").value(ModalResultSubject::They),
        primitives::kw("player").value(ModalResultSubject::Player),
        primitives::kw("players").value(ModalResultSubject::Players),
    ))
    .parse_next(input)
}

fn parse_result_verb<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        alt((
            primitives::kw("remove"),
            primitives::kw("removed"),
            primitives::kw("sacrifice"),
            primitives::kw("sacrificed"),
            primitives::kw("discard"),
            primitives::kw("discarded"),
            primitives::kw("exile"),
            primitives::kw("exiled"),
        ))
        .void(),
        alt((primitives::kw("mill"), primitives::kw("milled"))).void(),
    ))
    .void()
    .parse_next(input)
}

fn parse_contracted_negation<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("dont"),
        primitives::kw("don't"),
        primitives::kw("doesnt"),
        primitives::kw("doesn't"),
        primitives::kw("didnt"),
        primitives::kw("didn't"),
        primitives::kw("cant"),
        primitives::kw("can't"),
    ))
    .void()
    .parse_next(input)
}

fn parse_split_negation<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("do"),
        primitives::kw("does"),
        primitives::kw("did"),
        primitives::kw("can"),
    ))
    .void()
    .parse_next(input)
}

fn parse_optional_result_qualifier<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(alt((
        primitives::phrase(&["creature", "card"]).void(),
        primitives::kw("it").void(),
        primitives::kw("them").void(),
        primitives::kw("that").void(),
        primitives::kw("card").void(),
    )))
    .void()
    .parse_next(input)
}

fn parse_this_way_result<'a>(input: &mut LexStream<'a>) -> WResult<ModalResultShape> {
    let subject = parse_subject.parse_next(input)?;
    let negated = alt((
        (parse_contracted_negation, parse_result_verb).value(true),
        (
            parse_split_negation,
            primitives::kw("not"),
            parse_result_verb,
        )
            .value(true),
        parse_result_verb.value(false),
    ))
    .parse_next(input)?;
    parse_optional_result_qualifier.parse_next(input)?;
    primitives::phrase(&["this", "way"]).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(ModalResultShape::ThisWay { subject, negated })
}

fn parse_exact_negated_result<'a>(input: &mut LexStream<'a>) -> WResult<ModalResultShape> {
    let subject = parse_subject.parse_next(input)?;
    alt((
        parse_contracted_negation,
        (parse_split_negation, primitives::kw("not")).void(),
    ))
    .parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(ModalResultShape::ExactNegated { subject })
}

fn parse_modal_result_shape(tokens: &[OwnedLexToken]) -> Option<ModalResultShape> {
    primitives::parse_all(tokens, parse_this_way_result, "modal-this-way-result")
        .ok()
        .or_else(|| {
            primitives::parse_all(tokens, parse_exact_negated_result, "modal-negated-result").ok()
        })
}

fn parse_searched_library_result<'a>(input: &mut LexStream<'a>) -> WResult<ModalResultSubject> {
    let subject = alt((
        primitives::phrase(&["that", "player"]).value(ModalResultSubject::ThatPlayer),
        primitives::phrase(&["first", "player"]).value(ModalResultSubject::FirstPlayer),
        primitives::kw("you").value(ModalResultSubject::You),
        primitives::kw("they").value(ModalResultSubject::They),
        primitives::kw("player").value(ModalResultSubject::Player),
        primitives::kw("players").value(ModalResultSubject::Players),
    ))
    .parse_next(input)?;
    alt((
        primitives::kw("search"),
        primitives::kw("searches"),
        primitives::kw("searched"),
    ))
    .void()
    .parse_next(input)?;
    alt((
        primitives::phrase(&["your", "library"]),
        primitives::phrase(&["their", "library"]),
        primitives::phrase(&["library"]),
    ))
    .void()
    .parse_next(input)?;
    primitives::phrase(&["this", "way"]).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(subject)
}

fn has_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some()
}

fn starts_with_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(phrase)).is_some()
}

fn ends_with_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || (primitives::phrase(phrase), eof).void()).is_some()
}

fn matches_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::parse_all(
        tokens,
        (primitives::phrase(phrase), eof).void(),
        "modal-result-exact-phrase",
    )
    .is_ok()
}

/// Route a typed `... this way` object predicate through the ordinary
/// predicate grammar instead of collapsing it to the legacy `if you do`
/// boolean. The generated result tag supplies identity; the retained filter,
/// action, actor, and cardinality supply exact LKI semantics and rendering.
fn parse_typed_prior_effect_result_surface(
    tokens: &[OwnedLexToken],
) -> Option<PriorEffectResultSurface> {
    let predicate = super::filters::parse_predicate(tokens).ok()?;
    let (actor, mut filter) = match predicate {
        PredicateAst::TaggedMatches(_, filter) => (PriorEffectResultActor::Passive, filter),
        PredicateAst::PlayerTaggedObjectMatches { player, filter, .. } => {
            let actor = match player {
                PlayerAst::You => PriorEffectResultActor::You,
                PlayerAst::That | PlayerAst::Implicit | PlayerAst::ItsController => {
                    PriorEffectResultActor::ThatPlayer
                }
                _ => PriorEffectResultActor::ThatPlayer,
            };
            (actor, filter)
        }
        _ => return None,
    };
    let action = filter.prior_effect_action_surface()?;
    filter.set_prior_effect_action_surface(None);
    // The antecedent result memory provides identity and the action provides
    // the zone transition. Neither an implicit tag nor a parser-default zone
    // belongs in the semantic characteristic filter.
    filter.tagged_constraints.clear();
    filter.zone = None;

    let normalized = normalized_word_tokens(tokens);
    let quantifier = if starts_with_phrase(&normalized, &["one", "or", "more"]) {
        PriorEffectResultQuantifier::OneOrMore
    } else {
        PriorEffectResultQuantifier::One
    };
    Some(PriorEffectResultSurface::new(
        action, filter, actor, quantifier,
    ))
}

fn parse_prior_result_object_filter(
    tokens: &[OwnedLexToken],
) -> Option<crate::target::ObjectFilter> {
    let mut start = 0usize;
    if tokens.get(0).is_some_and(|token| token.is_word("one"))
        && tokens.get(1).is_some_and(|token| token.is_word("or"))
        && tokens.get(2).is_some_and(|token| token.is_word("more"))
    {
        start = 3;
    }
    let mut filter =
        super::filters::parse_object_filter_with_grammar_entrypoint_lexed(&tokens[start..], false)
            .ok()?;
    filter.zone = None;
    filter.tagged_constraints.clear();
    filter.set_prior_effect_action_surface(None);
    Some(filter)
}

/// Recognize prior-result clauses whose verbs are not object-filter
/// predicates in the general condition grammar (active reveal/cast/search,
/// "put into exile", damage prevention, and counter removal).
fn parse_direct_prior_effect_result_surface(
    tokens: &[OwnedLexToken],
) -> Option<PriorEffectResultSurface> {
    let words = normalized_word_tokens(tokens);
    if words.len() < 3 || !ends_with_phrase(&words, &["this", "way"]) {
        return None;
    }
    let normalized_words = words
        .iter()
        .map(OwnedLexToken::parser_text)
        .collect::<Vec<_>>();
    let one_or_more = starts_with_phrase(&words, &["one", "or", "more"]);
    let ordinary_quantifier = if one_or_more {
        PriorEffectResultQuantifier::OneOrMore
    } else {
        PriorEffectResultQuantifier::One
    };

    if normalized_words.as_slice() == ["it", "connives", "this", "way"]
        || normalized_words.as_slice() == ["it", "connive", "this", "way"]
    {
        return Some(PriorEffectResultSurface::new(
            PriorEffectAction::Connived,
            crate::target::ObjectFilter::default(),
            PriorEffectResultActor::It,
            PriorEffectResultQuantifier::ActionOnly,
        ));
    }

    if normalized_words.first() == Some(&"you") {
        let (action, verb_len, action_only) = match normalized_words.get(1).copied()? {
            "cast" => (PriorEffectAction::Cast, 1, false),
            "discard" | "discarded" => (PriorEffectAction::Discarded, 1, false),
            "reveal" | "revealed" => (PriorEffectAction::Revealed, 1, false),
            "tap" | "tapped" => (PriorEffectAction::Tapped, 1, false),
            "search" | "searched" => (PriorEffectAction::Searched, 1, true),
            _ => return None,
        };
        let filter = if action_only {
            crate::target::ObjectFilter::default()
        } else {
            // The result prefix is lexicalized as ordinary words, so the
            // first two raw tokens are the actor and action as well.
            let this_way_idx = tokens.iter().position(|token| token.is_word("this"))?;
            parse_prior_result_object_filter(&tokens[1 + verb_len..this_way_idx])?
        };
        return Some(PriorEffectResultSurface::new(
            action,
            filter,
            PriorEffectResultActor::You,
            if action_only {
                PriorEffectResultQuantifier::ActionOnly
            } else {
                ordinary_quantifier
            },
        ));
    }

    let copula_idx = tokens.iter().position(|token| {
        token
            .as_word()
            .is_some_and(|word| matches!(word, "is" | "are" | "was" | "were"))
    })?;
    let after = tokens[copula_idx + 1..]
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    let action = if after.starts_with(&["put", "into", "exile"]) {
        PriorEffectAction::Exiled
    } else if after.starts_with(&["put", "onto", "the", "battlefield"])
        || after.starts_with(&["put", "onto", "battlefield"])
    {
        PriorEffectAction::PutOntoBattlefield
    } else if after.first() == Some(&"removed") {
        PriorEffectAction::Removed
    } else if after.first() == Some(&"prevented") {
        PriorEffectAction::Prevented
    } else if after.first() == Some(&"countered") {
        PriorEffectAction::Countered
    } else {
        return None;
    };
    let non_object_subject = tokens[..copula_idx].iter().any(|token| {
        token.as_word().is_some_and(|word| {
            matches!(
                word,
                "ability" | "abilities" | "counter" | "counters" | "damage"
            )
        })
    });
    let filter = if non_object_subject {
        crate::target::ObjectFilter::default()
    } else {
        parse_prior_result_object_filter(&tokens[..copula_idx]).unwrap_or_default()
    };
    let action_only = filter == crate::target::ObjectFilter::default();
    Some(PriorEffectResultSurface::new(
        action,
        filter,
        PriorEffectResultActor::Passive,
        if action_only {
            PriorEffectResultQuantifier::ActionOnly
        } else {
            ordinary_quantifier
        },
    ))
}

pub(crate) fn parse_if_result_predicate_tokens(
    tokens: &[OwnedLexToken],
) -> Option<IfResultPredicate> {
    let normalized = normalized_word_tokens(tokens);
    match parse_modal_result_shape(&normalized)? {
        ModalResultShape::ThisWay {
            subject: ModalResultSubject::If | ModalResultSubject::When,
            negated: false,
        }
        | ModalResultShape::ExactNegated {
            subject: ModalResultSubject::If | ModalResultSubject::When,
        } => Some(IfResultPredicate::Did),
        ModalResultShape::ThisWay {
            subject: ModalResultSubject::If | ModalResultSubject::When,
            negated: true,
        } => Some(IfResultPredicate::DidNot),
        _ => None,
    }
}

pub(crate) fn parse_if_result_predicate_lexed_tokens(
    tokens: &[OwnedLexToken],
) -> Option<IfResultPredicate> {
    if let Some(surface) = parse_direct_prior_effect_result_surface(tokens)
        .or_else(|| parse_typed_prior_effect_result_surface(tokens))
    {
        return Some(IfResultPredicate::PriorEffectResult(surface));
    }
    let normalized = normalized_word_tokens(tokens);
    let shape = parse_modal_result_shape(&normalized);
    let word_count = normalized.len();
    let words = normalized
        .iter()
        .map(OwnedLexToken::parser_text)
        .collect::<Vec<_>>();

    if matches_phrase(&normalized, &["you", "do"])
        || matches_phrase(&normalized, &["they", "do"])
        || matches_phrase(&normalized, &["player", "do"])
        || matches_phrase(&normalized, &["player", "does"])
        || matches_phrase(&normalized, &["players", "do"])
        || matches_phrase(&normalized, &["players", "does"])
        || matches_phrase(&normalized, &["that", "player", "do"])
        || matches_phrase(&normalized, &["that", "player", "does"])
        || matches_phrase(&normalized, &["first", "player", "do"])
        || matches_phrase(&normalized, &["first", "player", "does"])
        || matches_phrase(&normalized, &["it", "connive", "this", "way"])
        || matches_phrase(&normalized, &["it", "connives", "this", "way"])
    {
        return Some(IfResultPredicate::Did);
    }
    if (starts_with_phrase(&normalized, &["you", "win"])
        || starts_with_phrase(&normalized, &["you", "won"]))
        && (word_count == 2 || has_phrase(&normalized, &["clash"]))
    {
        return Some(IfResultPredicate::Value(
            crate::effect::Comparison::GreaterThan(0),
        ));
    }
    if word_count == 3
        && (starts_with_phrase(&normalized, &["result", "is"])
            || starts_with_phrase(&normalized, &["result", "was"]))
        && let Ok(value) = leaf::parse_number_i32_complete(normalized[2].parser_text())
    {
        return Some(IfResultPredicate::Value(crate::effect::Comparison::Equal(
            value,
        )));
    }
    if (starts_with_phrase(&normalized, &["you", "win"])
        || starts_with_phrase(&normalized, &["you", "won"]))
        && has_phrase(&normalized, &["flip"])
    {
        return Some(IfResultPredicate::Did);
    }
    if starts_with_phrase(&normalized, &["you", "searched"])
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::Did);
    }
    if (starts_with_phrase(&normalized, &["you", "reveal"])
        || starts_with_phrase(&normalized, &["you", "revealed"])
        || starts_with_phrase(&normalized, &["they", "reveal"])
        || starts_with_phrase(&normalized, &["they", "revealed"])
        || starts_with_phrase(&normalized, &["that", "player", "reveals"])
        || starts_with_phrase(&normalized, &["that", "player", "revealed"]))
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::Did);
    }
    if primitives::parse_all(
        &normalized,
        parse_searched_library_result,
        "searched-library-result",
    )
    .is_ok()
    {
        return Some(IfResultPredicate::SearchedLibrary);
    }
    if matches!(
        shape,
        Some(ModalResultShape::ThisWay {
            subject: ModalResultSubject::You | ModalResultSubject::They,
            negated: false,
        })
    ) {
        return Some(IfResultPredicate::Did);
    }
    if matches_phrase(&normalized, &["no", "one", "do"])
        || matches_phrase(&normalized, &["no", "one", "does"])
    {
        return Some(IfResultPredicate::DidNot);
    }
    if matches_phrase(
        &normalized,
        &["player", "is", "dealt", "damage", "this", "way"],
    ) {
        return Some(IfResultPredicate::DealtDamageToPlayer);
    }
    // A typed object affected by the immediately preceding zone move is a
    // reflexive-result predicate, not a new trigger subject.  This covers
    // surfaces such as "When a creature is put onto the battlefield this
    // way" while retaining the affected object for the follow-up effects.
    let card_type_idx = usize::from(
        words
            .first()
            .is_some_and(|word| matches!(*word, "a" | "an" | "the")),
    );
    let affected_card_type = words.get(card_type_idx).and_then(|word| match *word {
        "artifact" => Some(crate::types::CardType::Artifact),
        "battle" => Some(crate::types::CardType::Battle),
        "creature" => Some(crate::types::CardType::Creature),
        "enchantment" => Some(crate::types::CardType::Enchantment),
        "land" => Some(crate::types::CardType::Land),
        "planeswalker" => Some(crate::types::CardType::Planeswalker),
        _ => None,
    });
    if let Some(card_type) = affected_card_type
        && words
            .get(card_type_idx + 1)
            .is_some_and(|word| matches!(*word, "is" | "was"))
        && words
            .get(card_type_idx + 2)
            .is_some_and(|word| matches!(*word, "put" | "moved" | "returned"))
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::AffectedObjectMatchesCardType {
            card_type,
            negated: false,
        });
    }
    let one_or_more_result = word_count >= 6
        && starts_with_phrase(&normalized, &["one", "or", "more"])
        && ends_with_phrase(&normalized, &["this", "way"])
        && words.get(3).is_some_and(|word| {
            matches!(
                *word,
                "card" | "cards" | "creature" | "creatures" | "permanent" | "permanents"
            )
        })
        && if words
            .get(4)
            .is_some_and(|word| matches!(*word, "is" | "are"))
        {
            words.get(5).is_some_and(|word| {
                matches!(
                    *word,
                    "remove"
                        | "removed"
                        | "sacrifice"
                        | "sacrificed"
                        | "discard"
                        | "discarded"
                        | "exile"
                        | "exiled"
                        | "mill"
                        | "milled"
                )
            })
        } else {
            words.get(4).is_some_and(|word| {
                matches!(
                    *word,
                    "remove"
                        | "removed"
                        | "sacrifice"
                        | "sacrificed"
                        | "discard"
                        | "discarded"
                        | "exile"
                        | "exiled"
                        | "mill"
                        | "milled"
                )
            })
        };
    if one_or_more_result {
        return Some(IfResultPredicate::Did);
    }
    if word_count >= 5
        && (starts_with_phrase(&normalized, &["that", "spell"])
            || starts_with_phrase(&normalized, &["it", "spell"]))
        && has_phrase(&normalized, &["countered"])
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::Did);
    }
    if word_count >= 5
        && (starts_with_phrase(&normalized, &["that", "creature", "dies", "this", "way"])
            || starts_with_phrase(&normalized, &["that", "permanent", "dies", "this", "way"])
            || starts_with_phrase(&normalized, &["that", "card", "dies", "this", "way"])
            || starts_with_phrase(&normalized, &["it", "creature", "dies", "this", "way"])
            || starts_with_phrase(&normalized, &["it", "permanent", "dies", "this", "way"])
            || starts_with_phrase(&normalized, &["it", "card", "dies", "this", "way"]))
    {
        return Some(IfResultPredicate::DiesThisWay);
    }
    if word_count >= 8
        && (starts_with_phrase(
            &normalized,
            &[
                "creature", "dealt", "damage", "this", "way", "would", "die", "this", "turn",
            ],
        ) || starts_with_phrase(
            &normalized,
            &[
                "permanent",
                "dealt",
                "damage",
                "this",
                "way",
                "would",
                "die",
                "this",
                "turn",
            ],
        ) || starts_with_phrase(
            &normalized,
            &[
                "card", "dealt", "damage", "this", "way", "would", "die", "this", "turn",
            ],
        ))
    {
        return Some(IfResultPredicate::DiesThisWay);
    }
    if starts_with_phrase(&normalized, &["excess", "damage", "was", "dealt", "to"])
        && has_phrase(&normalized, &["creature"])
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::ExcessDamageDealt);
    }
    if matches_phrase(
        &normalized,
        &["it", "deals", "excess", "damage", "this", "way"],
    ) {
        return Some(IfResultPredicate::ExcessDamageDealt);
    }
    if word_count == 6
        && (starts_with_phrase(&normalized, &["its", "power", "becomes"])
            || starts_with_phrase(&normalized, &["it", "power", "becomes"]))
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::Did);
    }
    if (starts_with_phrase(&normalized, &["you", "lose"])
        || starts_with_phrase(&normalized, &["you", "lost"]))
        && has_phrase(&normalized, &["flip"])
    {
        return Some(IfResultPredicate::DidNot);
    }
    if matches!(
        shape,
        Some(
            ModalResultShape::ThisWay {
                subject: ModalResultSubject::You
                    | ModalResultSubject::They
                    | ModalResultSubject::Player
                    | ModalResultSubject::Players
                    | ModalResultSubject::ThatPlayer,
                negated: true,
            } | ModalResultShape::ExactNegated {
                subject: ModalResultSubject::You
                    | ModalResultSubject::They
                    | ModalResultSubject::Player
                    | ModalResultSubject::Players
                    | ModalResultSubject::ThatPlayer,
            }
        )
    ) {
        return Some(IfResultPredicate::DidNot);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_modal_result_predicates() {
        for (raw, expected) in [
            ("you do", IfResultPredicate::Did),
            ("you don't discard it this way", IfResultPredicate::DidNot),
            (
                "that creature dies this way",
                IfResultPredicate::DiesThisWay,
            ),
            (
                "result is 3",
                IfResultPredicate::Value(crate::effect::Comparison::Equal(3)),
            ),
            ("that player does", IfResultPredicate::Did),
            ("first player does", IfResultPredicate::Did),
            (
                "they searched their library this way",
                IfResultPredicate::SearchedLibrary,
            ),
            (
                "it connives this way",
                IfResultPredicate::PriorEffectResult(PriorEffectResultSurface::new(
                    PriorEffectAction::Connived,
                    crate::target::ObjectFilter::default(),
                    PriorEffectResultActor::It,
                    PriorEffectResultQuantifier::ActionOnly,
                )),
            ),
            (
                "one or more cards are exiled this way",
                IfResultPredicate::Did,
            ),
            (
                "a player is dealt damage this way",
                IfResultPredicate::DealtDamageToPlayer,
            ),
            ("you lost the flip", IfResultPredicate::DidNot),
            ("that player doesn't", IfResultPredicate::DidNot),
            ("its power becomes 3 this way", IfResultPredicate::Did),
            ("you milled a card this way", IfResultPredicate::Did),
            (
                "you reveal a nonland card this way",
                IfResultPredicate::PriorEffectResult({
                    let mut filter = crate::target::ObjectFilter::default();
                    filter.excluded_card_types = vec![crate::types::CardType::Land];
                    filter.set_explicit_card_noun(true);
                    PriorEffectResultSurface::new(
                        PriorEffectAction::Revealed,
                        filter,
                        PriorEffectResultActor::You,
                        PriorEffectResultQuantifier::One,
                    )
                }),
            ),
        ] {
            let tokens = lex_line(raw, 0).unwrap();
            let actual = parse_if_result_predicate_lexed_tokens(&tokens);
            assert_eq!(actual, Some(expected));
        }
    }
}

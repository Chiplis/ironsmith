use crate::cards::builders::{IfResultPredicate, PlayerAst, PredicateAst, PlayerPredicateAst};
use ironsmith_core::{
    ObjectCharacteristic, PriorEffectAction, PriorEffectResultActor, PriorEffectResultQuantifier,
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
    crate::grammar::primitives::probe_all(tokens, parse_this_way_result, "modal-this-way-result")
        .or_else(|| {
            crate::grammar::primitives::probe_all(
                tokens,
                parse_exact_negated_result,
                "modal-negated-result",
            )
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

fn counted_shared_characteristic(tokens: &[OwnedLexToken]) -> Option<(u32, ObjectCharacteristic)> {
    let normalized = normalized_word_tokens(tokens);
    let count = crate::grammar::primitives::probe_shape(leaf::parse_number_complete(
        normalized.first()?.parser_text(),
    ))?;
    if count < 2 {
        return None;
    }
    let words = normalized
        .iter()
        .map(OwnedLexToken::parser_text)
        .collect::<Vec<_>>();
    let has_relative = |tail: &[&str]| {
        crate::slice_primitives::find_window_by(&words, tail.len() + 2, |window| {
            window[0] == "that"
                && matches!(window[1], "share" | "shares")
                && crate::slice_primitives::starts_with(&window[2..], tail)
        })
        .is_some()
    };
    // `normalized_word_tokens` removes articles, so characteristic tails are
    // compared in that same canonical form.
    let characteristic = if has_relative(&["color"]) {
        ObjectCharacteristic::Color
    } else if has_relative(&["card", "type"]) {
        ObjectCharacteristic::CardType
    } else if has_relative(&["permanent", "type"]) {
        ObjectCharacteristic::PermanentType
    } else if has_relative(&["creature", "type"]) {
        ObjectCharacteristic::Subtype(crate::types::SubtypeFamily::Creature)
    } else if has_relative(&["land", "type"]) {
        ObjectCharacteristic::Subtype(crate::types::SubtypeFamily::Land)
    } else if has_relative(&["mana", "value"]) {
        ObjectCharacteristic::ManaValue
    } else {
        return None;
    };
    Some((count, characteristic))
}

/// Route a typed `... this way` object predicate through the ordinary
/// predicate grammar instead of collapsing it to the legacy `if you do`
/// boolean. The generated result tag supplies identity; the retained filter,
/// action, actor, and cardinality supply exact LKI semantics and rendering.
fn parse_typed_prior_effect_result_surface(
    tokens: &[OwnedLexToken],
) -> Option<PriorEffectResultSurface> {
    let predicate =
        crate::grammar::primitives::probe_shape(super::filters::parse_predicate(tokens))?;
    let (actor, mut filter) = match predicate {
        PredicateAst::TaggedMatches(_, filter) => (PriorEffectResultActor::Passive, filter),
        PredicateAst::Player(PlayerPredicateAst::PlayerTaggedObjectMatches { player, filter, .. }) => {
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
    if let Some(disjunction) = parse_explicit_prior_result_filter_disjunction(tokens) {
        filter = disjunction;
    }
    filter.set_prior_effect_action_surface(None);
    // The antecedent result memory provides the subject identity and the
    // action provides the zone transition. Drop only an implicit identity
    // constraint; comparison constraints remain semantic characteristics.
    // For example, "a card with the chosen name was milled this way" must
    // retain its SameNameAsTagged(__chosen_name__) comparison.
    filter.tagged_constraints.retain(|constraint| {
        !(constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject)
    });
    filter.zone = None;

    let normalized = normalized_word_tokens(tokens);
    let quantifier = if starts_with_phrase(&normalized, &["one", "or", "more"]) {
        PriorEffectResultQuantifier::OneOrMore
    } else {
        PriorEffectResultQuantifier::One
    };
    let mut surface = PriorEffectResultSurface::new(action, filter, actor, quantifier);
    if let Some((count, characteristic)) = counted_shared_characteristic(tokens) {
        surface = surface.with_count_sharing(count, characteristic);
    }
    Some(surface)
}

/// Preserve independently qualified result objects such as
/// "a permanent you controlled or a token was destroyed this way." The
/// ordinary flat object-filter parser correctly recognizes the terminal
/// action but can otherwise intersect the two arms into "a permanent token."
/// Requiring an explicit article on both arms distinguishes this semantic
/// disjunction from compact type lists such as "an artifact or creature."
fn parse_explicit_prior_result_filter_disjunction(
    tokens: &[OwnedLexToken],
) -> Option<crate::target::ObjectFilter> {
    let copula_idx = crate::slice_primitives::select_position(tokens, |token| {
        token
            .as_word()
            .is_some_and(|word| matches!(word, "is" | "are" | "was" | "were"))
    })?;
    let or_idx = crate::slice_primitives::select_last_position(&tokens[..copula_idx], |token| {
        token.is_word("or")
    })?;
    let left = &tokens[..or_idx];
    let right = &tokens[or_idx + 1..copula_idx];
    let has_explicit_article = |branch: &[OwnedLexToken]| {
        branch
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| matches!(word, "a" | "an"))
    };
    if !has_explicit_article(left) || !has_explicit_article(right) {
        return None;
    }

    let parse_branch = |branch: &[OwnedLexToken]| {
        let mut filter = crate::grammar::primitives::probe_shape(
            super::filters::parse_object_filter_with_grammar_entrypoint_lexed(branch, false),
        )?;
        filter.zone = None;
        filter.tagged_constraints.clear();
        filter.set_prior_effect_action_surface(None);
        (filter != crate::target::ObjectFilter::default()).then_some(filter)
    };
    let left = parse_branch(left)?;
    let right = parse_branch(right)?;
    let mut filter = crate::target::ObjectFilter::default();
    filter.any_of = vec![left, right];
    filter.set_explicit_union_branch_articles(true);
    Some(filter)
}

fn parse_prior_result_object_filter(
    tokens: &[OwnedLexToken],
) -> Option<crate::target::ObjectFilter> {
    let mut start = 0usize;
    if tokens.first().is_some_and(|token| token.is_word("one"))
        && tokens.get(1).is_some_and(|token| token.is_word("or"))
        && tokens.get(2).is_some_and(|token| token.is_word("more"))
    {
        start = 3;
    }
    let mut filter = crate::grammar::primitives::probe_shape(
        super::filters::parse_object_filter_with_grammar_entrypoint_lexed(&tokens[start..], false),
    )?;
    filter.zone = None;
    filter.tagged_constraints.clear();
    filter.set_prior_effect_action_surface(None);
    Some(filter)
}

#[cfg(test)]
#[path = "modal_results_inline_tests.rs"]
mod tests;

#[path = "modal_results/modal_results_object_action.rs"]
mod modal_results_object_action_programs;
pub use modal_results_object_action_programs::{
    parse_if_result_predicate_lexed_tokens, parse_if_result_predicate_tokens,
};
#[path = "modal_results/modal_results_core.rs"]
mod modal_results_core_programs;
use modal_results_core_programs::parse_direct_prior_effect_result_surface;

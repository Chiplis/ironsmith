use winnow::Parser;
use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;

use super::{permission_shapes, primitives, structure};
use crate::lexer::{LexStream, OwnedLexToken, TokenKind};
use crate::util::parse_subtype_flexible;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraveyardCastControlCondition {
    Subtype(crate::types::Subtype),
    ColorPair(crate::color::Color, crate::color::Color),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdditionalCombatRewriteKind {
    ConditionalAfterThisPhase,
    AfterThisPhase,
    AlreadyCanonical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AdditionalCombatRewriteShape<'a> {
    pub(crate) before_tokens: &'a [OwnedLexToken],
    pub(crate) after_tokens: &'a [OwnedLexToken],
    pub(crate) kind: AdditionalCombatRewriteKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NonTurnConditionalUntapShape<'a> {
    pub(crate) first_sentence_tokens: &'a [OwnedLexToken],
    pub(crate) untap_sentence_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CountThatNumberLifeTotalRewriteShape<'a> {
    pub(crate) trigger_tokens: &'a [OwnedLexToken],
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) count_value_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_additional_combat_rewrite_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AdditionalCombatRewriteShape<'_>> {
    let (start, kind, after_tokens) =
        primitives::find_prefix(tokens, || additional_combat_rewrite)?;
    Some(AdditionalCombatRewriteShape {
        before_tokens: tokens.get(..start)?,
        after_tokens,
        kind,
    })
}

pub(crate) fn parse_non_turn_conditional_untap_tokens(
    tokens: &[OwnedLexToken],
) -> Option<NonTurnConditionalUntapShape<'_>> {
    let (delimiter, (), after) = primitives::find_prefix(tokens, || non_turn_untap_suffix)?;
    if !after.is_empty() {
        return None;
    }
    let first_sentence_tokens = tokens.get(..delimiter)?;
    let mut first_input = LexStream::new(first_sentence_tokens);
    primitives::phrase(&["creatures", "you", "control", "get"])
        .parse_next(&mut first_input)
        .ok()?;
    Some(NonTurnConditionalUntapShape {
        first_sentence_tokens,
        untap_sentence_tokens: tokens.get(delimiter + 1..)?,
    })
}

pub(crate) fn parse_graveyard_cast_control_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<GraveyardCastControlCondition> {
    let mut input = LexStream::new(tokens);
    let condition = graveyard_cast_control_condition
        .parse_next(&mut input)
        .ok()?;
    input.is_empty().then_some(condition)
}

pub(crate) fn parse_count_that_number_life_total_rewrite_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CountThatNumberLifeTotalRewriteShape<'_>> {
    let (comma, _, effect_tokens) =
        primitives::find_prefix(tokens, || primitives::token_kind(TokenKind::Comma).void())?;
    let trigger_tokens = tokens.get(..comma)?;
    let sentences = structure::split_lexed_sentences(effect_tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    let [count_sentence, life_sentence] = sentences.as_slice() else {
        return None;
    };
    let count_sentence = strip_terminal_period(count_sentence);
    let (_, _, count_value_tokens) =
        primitives::find_prefix(count_sentence, || primitives::kw("count").void())?;
    if !permission_shapes::prefix_tokens(count_value_tokens, &["the", "number", "of"])
        && !permission_shapes::prefix_tokens(count_value_tokens, &["number", "of"])
    {
        return None;
    }

    let life_sentence = strip_terminal_period(life_sentence);
    let (becomes, _, amount_tokens) =
        primitives::find_prefix(life_sentence, || primitives::kw("becomes").void())?;
    let subject_tokens = life_sentence.get(..becomes)?;
    if !permission_shapes::exact_tokens(amount_tokens, &["that", "number"])
        || !permission_shapes::suffix_words(
            &crate::lexer::TokenWordView::new(subject_tokens).word_refs(),
            &["life", "total"],
        )
    {
        return None;
    }

    Some(CountThatNumberLifeTotalRewriteShape {
        trigger_tokens,
        subject_tokens,
        count_value_tokens,
    })
}

fn strip_terminal_period(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        &tokens[..tokens.len().saturating_sub(1)]
    } else {
        tokens
    }
}

fn additional_combat_rewrite(input: &mut LexStream<'_>) -> WResult<AdditionalCombatRewriteKind> {
    alt((
        conditional_after_this_phase.value(AdditionalCombatRewriteKind::ConditionalAfterThisPhase),
        after_this_phase.value(AdditionalCombatRewriteKind::AfterThisPhase),
        canonical_additional_combat.value(AdditionalCombatRewriteKind::AlreadyCanonical),
    ))
    .parse_next(input)
}

fn conditional_after_this_phase(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::phrase(&["if", "it's", "your", "main", "phase"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    after_this_phase.parse_next(input)
}

fn after_this_phase(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::phrase(&[
        "there",
        "is",
        "an",
        "additional",
        "combat",
        "phase",
        "after",
        "this",
        "phase",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["followed", "by", "an", "additional", "main", "phase"]).parse_next(input)
}

fn canonical_additional_combat(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::phrase(&["after", "this", "main", "phase"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "there",
        "is",
        "an",
        "additional",
        "combat",
        "phase",
        "followed",
        "by",
        "an",
        "additional",
        "main",
        "phase",
    ])
    .parse_next(input)
}

fn non_turn_untap_suffix(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::period().parse_next(input)?;
    primitives::phrase(&["if", "it's", "not", "your", "turn"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&["untap", "those", "creatures"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

fn graveyard_cast_control_condition(
    input: &mut LexStream<'_>,
) -> WResult<GraveyardCastControlCondition> {
    primitives::phrase(&[
        "you",
        "may",
        "cast",
        "this",
        "card",
        "from",
        "your",
        "graveyard",
        "as",
        "long",
        "as",
        "you",
        "control",
        "a",
    ])
    .parse_next(input)?;
    alt((graveyard_cast_color_pair, graveyard_cast_subtype)).parse_next(input)
}

fn graveyard_cast_color_pair(input: &mut LexStream<'_>) -> WResult<GraveyardCastControlCondition> {
    let left_word = primitives::word_parser_text.parse_next(input)?;
    primitives::kw("or").parse_next(input)?;
    let right_word = primitives::word_parser_text.parse_next(input)?;
    primitives::kw("permanent").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let left = crate::color::Color::from_name(left_word)
        .ok_or_else(|| primitives::backtrack_err("graveyard cast condition", "color"))?;
    let right = crate::color::Color::from_name(right_word)
        .ok_or_else(|| primitives::backtrack_err("graveyard cast condition", "color"))?;
    Ok(GraveyardCastControlCondition::ColorPair(left, right))
}

fn graveyard_cast_subtype(input: &mut LexStream<'_>) -> WResult<GraveyardCastControlCondition> {
    let subtype_word = primitives::word_parser_text.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let subtype = parse_subtype_flexible(subtype_word)
        .ok_or_else(|| primitives::backtrack_err("graveyard cast condition", "subtype"))?;
    Ok(GraveyardCastControlCondition::Subtype(subtype))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex_line, render_token_slice};

    #[test]
    fn parses_typed_additional_combat_rewrite_variants() {
        let conditional = lex_line(
            "If it's your main phase, there is an additional combat phase after this phase, followed by an additional main phase.",
            0,
        )
        .expect("lex");
        let conditional_shape =
            parse_additional_combat_rewrite_tokens(&conditional).expect("conditional rewrite");
        assert_eq!(
            conditional_shape.kind,
            AdditionalCombatRewriteKind::ConditionalAfterThisPhase
        );
        assert!(conditional_shape.before_tokens.is_empty());
        assert_eq!(render_token_slice(conditional_shape.after_tokens), ".");

        let embedded = lex_line(
            "Untap all creatures that attacked this turn. There is an additional combat phase after this phase, followed by an additional main phase.",
            0,
        )
        .expect("lex");
        let embedded_shape =
            parse_additional_combat_rewrite_tokens(&embedded).expect("embedded rewrite");
        assert_eq!(
            embedded_shape.kind,
            AdditionalCombatRewriteKind::AfterThisPhase
        );
        assert_eq!(
            render_token_slice(embedded_shape.before_tokens),
            "Untap all creatures that attacked this turn."
        );

        let canonical = lex_line(
            "After this main phase, there is an additional combat phase followed by an additional main phase.",
            0,
        )
        .expect("lex");
        assert_eq!(
            parse_additional_combat_rewrite_tokens(&canonical)
                .expect("canonical shape")
                .kind,
            AdditionalCombatRewriteKind::AlreadyCanonical
        );
    }

    #[test]
    fn parses_non_turn_conditional_untap_sentence_pair() {
        let tokens = lex_line(
            "Creatures you control get +1/+1. If it's not your turn, untap those creatures.",
            0,
        )
        .expect("lex");
        let shape =
            parse_non_turn_conditional_untap_tokens(&tokens).expect("conditional untap shape");
        assert_eq!(
            render_token_slice(shape.first_sentence_tokens),
            "Creatures you control get +1/+1"
        );
        assert_eq!(
            render_token_slice(shape.untap_sentence_tokens),
            "If it's not your turn, untap those creatures."
        );
    }

    #[test]
    fn parses_count_that_number_life_total_rewrite_shape() {
        let tokens = lex_line(
            "When this creature enters, count the number of creatures you control. Your life total becomes that number.",
            0,
        )
        .expect("lex");
        let shape = parse_count_that_number_life_total_rewrite_tokens(&tokens)
            .expect("count/life-total rewrite");
        assert_eq!(render_token_slice(shape.subject_tokens), "Your life total");
        assert_eq!(
            render_token_slice(shape.count_value_tokens),
            "the number of creatures you control"
        );
    }

    #[test]
    fn parses_typed_graveyard_cast_control_conditions() {
        let subtype = lex_line(
            "You may cast this card from your graveyard as long as you control a Zombie.",
            0,
        )
        .expect("lex");
        assert_eq!(
            parse_graveyard_cast_control_condition_tokens(&subtype),
            Some(GraveyardCastControlCondition::Subtype(
                crate::types::Subtype::Zombie
            ))
        );

        let colors = lex_line(
            "You may cast this card from your graveyard as long as you control a black or red permanent.",
            0,
        )
        .expect("lex");
        assert_eq!(
            parse_graveyard_cast_control_condition_tokens(&colors),
            Some(GraveyardCastControlCondition::ColorPair(
                crate::color::Color::Black,
                crate::color::Color::Red,
            ))
        );
    }
}

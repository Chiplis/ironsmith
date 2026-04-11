use winnow::combinator::{opt, seq};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::any;

use crate::ConditionExpr;
use crate::object::CounterType;
use crate::target::PlayerFilter;

use super::super::lexer::{LexStream, LexToken, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::primitives;
use crate::cards::builders::compiler::util::parse_counter_type_word;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UntapEachOtherPlayersUntapStepSpec<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatDamageUsingToughnessSubject {
    EachCreature,
    EachCreatureYouControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlyingBlockRestrictionKind {
    FlyingOnly,
    FlyingOrReach,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DoesntUntapDuringUntapStepSpec<'a> {
    Source { tail_tokens: &'a [OwnedLexToken] },
    Attached { subject_tokens: &'a [OwnedLexToken] },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivatedAbilitiesCantBeActivatedSpec<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) non_mana_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TriggerSuppressionSpec<'a> {
    pub(crate) cause_tokens: &'a [OwnedLexToken],
    pub(crate) source_filter_tokens: Option<&'a [OwnedLexToken]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RevealFirstCardYouDrawEachTurnSpec {
    pub(crate) optional: bool,
    pub(crate) your_turns_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExileToCounteredExileInsteadOfGraveyardSpec {
    pub(crate) player: PlayerFilter,
    pub(crate) counter_type: CounterType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AsLongAsConditionPrefixSpec<'a> {
    pub(crate) condition_tokens: &'a [OwnedLexToken],
    pub(crate) remainder_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IfThisSpellCostsSplitSpec<'a> {
    pub(crate) condition_tokens: &'a [OwnedLexToken],
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

fn black_mana_group<'a>(input: &mut LexStream<'a>) -> Result<&'a LexToken, ErrMode<ContextError>> {
    any.verify(|token: &&LexToken| {
        token.kind == TokenKind::ManaGroup && token.parser_text() == "{b}"
    })
    .context(StrContext::Label("black mana group"))
    .context(StrContext::Expected(StrContextValue::Description("{B}")))
    .parse_next(input)
}

fn parse_krrik_black_mana_life_payment_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::phrase(&["for", "each"]),
        black_mana_group,
        primitives::phrase(&["in", "a", "cost"]),
        opt(primitives::comma()),
        primitives::phrase(&[
            "you", "may", "pay", "2", "life", "rather", "than", "pay", "that", "mana",
        ]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_krrik_black_mana_life_payment_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_krrik_black_mana_life_payment_line).is_some()
}

fn parse_each_other_players_untap_step_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::kw("during"),
        primitives::kw("each"),
        primitives::kw("other"),
        winnow::combinator::alt((
            primitives::phrase(&["player's", "untap", "step"]),
            primitives::phrase(&["players", "untap", "step"]),
            primitives::phrase(&["player", "s", "untap", "step"]),
            primitives::phrase(&["player", "untap", "step"]),
        )),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn split_untap_each_other_players_untap_step_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<UntapEachOtherPlayersUntapStepSpec<'_>> {
    let (_, remainder) =
        primitives::parse_prefix(tokens, (primitives::kw("untap"), primitives::kw("all")))?;
    let (subject_tokens, ()) = primitives::split_lexed_once_before_suffix(remainder, 1, || {
        parse_each_other_players_untap_step_suffix
    })?;
    Some(UntapEachOtherPlayersUntapStepSpec { subject_tokens })
}

fn parse_activated_abilities_cant_be_activated_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<bool, ErrMode<ContextError>> {
    winnow::combinator::alt((
        (
            winnow::combinator::alt((primitives::kw("cant"), primitives::kw("can't"))),
            primitives::phrase(&["be", "activated", "unless"]),
            winnow::combinator::alt((primitives::kw("theyre"), primitives::kw("they're"))),
            primitives::phrase(&["mana", "abilities"]),
            primitives::sentence_end(),
        )
            .value(true),
        (
            winnow::combinator::alt((primitives::kw("cant"), primitives::kw("can't"))),
            primitives::phrase(&["be", "activated"]),
            primitives::sentence_end(),
        )
            .value(false),
    ))
    .parse_next(input)
}

pub(crate) fn parse_activated_abilities_cant_be_activated_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedAbilitiesCantBeActivatedSpec<'_>> {
    let (_, remainder) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["activated", "abilities", "of"]),
    )?;
    let (subject_tokens, non_mana_only) =
        primitives::split_lexed_once_before_suffix(remainder, 1, || {
            parse_activated_abilities_cant_be_activated_suffix
        })?;
    let subject_tokens = trim_lexed_commas(subject_tokens);
    (!subject_tokens.is_empty()).then_some(ActivatedAbilitiesCantBeActivatedSpec {
        subject_tokens,
        non_mana_only,
    })
}

fn parse_trigger_suppression_negation_prefix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        winnow::combinator::alt((
            primitives::kw("dont"),
            primitives::kw("don't"),
            primitives::kw("doesnt"),
            primitives::kw("doesn't"),
        )),
        primitives::kw("cause"),
        primitives::kw("abilities"),
    )
        .void()
        .parse_next(input)
}

fn parse_trigger_suppression_plain_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        parse_trigger_suppression_negation_prefix,
        primitives::phrase(&["to", "trigger"]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

fn parse_trigger_suppression_filter_prefix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        parse_trigger_suppression_negation_prefix,
        primitives::kw("of"),
    )
        .void()
        .parse_next(input)
}

fn parse_trigger_suppression_filter_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::phrase(&["to", "trigger"]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn parse_trigger_suppression_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<TriggerSuppressionSpec<'_>> {
    if let Some((cause_tokens, ())) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        parse_trigger_suppression_plain_suffix
    }) {
        let cause_tokens = trim_lexed_commas(cause_tokens);
        if !cause_tokens.is_empty() {
            return Some(TriggerSuppressionSpec {
                cause_tokens,
                source_filter_tokens: None,
            });
        }
    }

    for idx in 1..tokens.len() {
        let cause_tokens = trim_lexed_commas(&tokens[..idx]);
        if cause_tokens.is_empty() {
            continue;
        }

        let Some(((), remainder)) =
            primitives::parse_prefix(&tokens[idx..], parse_trigger_suppression_filter_prefix)
        else {
            continue;
        };

        if let Some((source_filter_tokens, ())) =
            primitives::split_lexed_once_before_suffix(remainder, 1, || {
                parse_trigger_suppression_filter_suffix
            })
        {
            let source_filter_tokens = trim_lexed_commas(source_filter_tokens);
            if !source_filter_tokens.is_empty() {
                return Some(TriggerSuppressionSpec {
                    cause_tokens,
                    source_filter_tokens: Some(source_filter_tokens),
                });
            }
        }
    }

    None
}

fn parse_reveal_first_card_you_draw_each_turn_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::phrase(&["reveal", "the", "first", "card", "you", "draw"]),
        primitives::phrase(&["each", "turn"]),
        opt(primitives::phrase(&["as", "you", "draw", "it"])),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

fn parse_reveal_first_card_you_draw_on_your_turns_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::phrase(&["reveal", "the", "first", "card", "you", "draw"]),
        primitives::phrase(&["on", "each", "of", "your", "turns"]),
        opt(primitives::phrase(&["as", "you", "draw", "it"])),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn parse_reveal_first_card_you_draw_each_turn_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<RevealFirstCardYouDrawEachTurnSpec> {
    for optional in [false, true] {
        let remainder = if optional {
            let (_, remainder) =
                primitives::parse_prefix(tokens, primitives::phrase(&["you", "may"]))?;
            remainder
        } else {
            tokens
        };

        if let Some(((), [])) = primitives::parse_prefix(
            &remainder,
            parse_reveal_first_card_you_draw_each_turn_suffix,
        ) {
            return Some(RevealFirstCardYouDrawEachTurnSpec {
                optional,
                your_turns_only: false,
            });
        }

        if let Some(((), [])) = primitives::parse_prefix(
            &remainder,
            parse_reveal_first_card_you_draw_on_your_turns_suffix,
        ) {
            return Some(RevealFirstCardYouDrawEachTurnSpec {
                optional,
                your_turns_only: true,
            });
        }
    }

    None
}

fn parse_exile_replacement_graveyard_player<'a>(
    input: &mut LexStream<'a>,
) -> Result<PlayerFilter, ErrMode<ContextError>> {
    winnow::combinator::alt((
        primitives::phrase(&["your", "graveyard"]).value(PlayerFilter::You),
        winnow::combinator::alt((
            primitives::phrase(&["an", "opponent's", "graveyard"]),
            primitives::phrase(&["an", "opponents", "graveyard"]),
            primitives::phrase(&["opponent's", "graveyard"]),
            primitives::phrase(&["opponents", "graveyard"]),
        ))
        .value(PlayerFilter::Opponent),
        winnow::combinator::alt((
            primitives::phrase(&["a", "player's", "graveyard"]),
            primitives::phrase(&["a", "players", "graveyard"]),
            primitives::phrase(&["player's", "graveyard"]),
            primitives::phrase(&["players", "graveyard"]),
        ))
        .value(PlayerFilter::Any),
    ))
    .parse_next(input)
}

fn parse_counter_type_token<'a>(
    input: &mut LexStream<'a>,
) -> Result<CounterType, ErrMode<ContextError>> {
    let token: &'a LexToken = any.parse_next(input)?;
    parse_counter_type_word(token.parser_text())
        .ok_or_else(|| primitives::backtrack_err("counter type", "known counter type word"))
}

fn parse_exile_to_countered_exile_instead_of_graveyard_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<ExileToCounteredExileInsteadOfGraveyardSpec, ErrMode<ContextError>> {
    seq!(ExileToCounteredExileInsteadOfGraveyardSpec {
        _: primitives::phrase(&["would", "be", "put", "into"]),
        player: parse_exile_replacement_graveyard_player,
        _: primitives::phrase(&["from", "anywhere"]),
        _: opt(primitives::comma()),
        _: winnow::combinator::alt((
            primitives::phrase(&["exile", "it", "instead", "with"]),
            primitives::phrase(&["instead", "exile", "it", "with"]),
        )),
        _: opt(winnow::combinator::alt((
            primitives::kw("a"),
            primitives::kw("an"),
        ))),
        counter_type: parse_counter_type_token,
        _: primitives::phrase(&["counter", "on", "it"]),
        _: primitives::sentence_end(),
    })
    .parse_next(input)
}

pub(crate) fn parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ExileToCounteredExileInsteadOfGraveyardSpec> {
    let (_, remainder) = primitives::parse_prefix(tokens, primitives::kw("if"))?;
    let (_, spec) = primitives::split_lexed_once_before_suffix(remainder, 1, || {
        parse_exile_to_countered_exile_instead_of_graveyard_suffix
    })?;
    Some(spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::compiler::lexer::TokenWordView;
    use crate::cards::builders::compiler::lexer::lex_line;

    #[test]
    fn untap_each_other_players_untap_step_extracts_subject_tokens() {
        let tokens = lex_line(
            "Untap all creatures during each other player's untap step.",
            0,
        )
        .unwrap();
        let spec = split_untap_each_other_players_untap_step_line_lexed(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(spec.subject_tokens).word_refs(),
            ["creatures"]
        );
    }

    #[test]
    fn activated_abilities_cant_be_activated_extracts_subject_tokens() {
        let tokens = lex_line("Activated abilities of artifacts can't be activated.", 0).unwrap();
        let spec = parse_activated_abilities_cant_be_activated_spec_lexed(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(spec.subject_tokens).word_refs(),
            ["artifacts"]
        );
        assert!(!spec.non_mana_only);
    }

    #[test]
    fn trigger_suppression_with_source_filter_extracts_both_sides() {
        let tokens = lex_line(
            "Creatures don't cause abilities of enchantments to trigger.",
            0,
        )
        .unwrap();
        let spec = parse_trigger_suppression_spec_lexed(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(spec.cause_tokens).word_refs(),
            ["creatures"]
        );
        assert_eq!(
            TokenWordView::new(spec.source_filter_tokens.unwrap()).word_refs(),
            ["enchantments"]
        );
    }

    #[test]
    fn exile_to_countered_exile_skips_condition_prefix() {
        let tokens = lex_line(
            "If a card would be put into your graveyard from anywhere, exile it instead with a charge counter on it.",
            0,
        )
        .unwrap();
        let spec = parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed(&tokens).unwrap();
        assert_eq!(spec.player, PlayerFilter::You);
        assert_eq!(spec.counter_type, CounterType::Charge);
    }
}

fn contains_word_lexed(tokens: &[OwnedLexToken], expected: &str) -> bool {
    tokens.iter().any(|token| token.is_word(expected))
}

fn last_parser_word_text_lexed(tokens: &[OwnedLexToken]) -> Option<&str> {
    tokens.iter().rev().find_map(|token| match token.kind {
        TokenKind::Word | TokenKind::Number | TokenKind::Tilde => Some(token.parser_text()),
        _ => None,
    })
}

fn parser_text_contains_char(text: &str, expected: char) -> bool {
    text.chars().any(|ch| ch == expected)
}

fn contains_any_word_lexed(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    expected
        .iter()
        .copied()
        .any(|word| contains_word_lexed(tokens, word))
}

fn contains_phrase_lexed(tokens: &[OwnedLexToken], expected: &'static [&'static str]) -> bool {
    (0..tokens.len())
        .any(|idx| primitives::parse_prefix(&tokens[idx..], primitives::phrase(expected)).is_some())
}

pub(crate) fn is_draw_replace_exile_top_face_down_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    if primitives::parse_prefix(
        tokens,
        primitives::phrase(&["if", "you", "would", "draw", "a", "card"]),
    )
    .is_none()
    {
        return false;
    }

    contains_word_lexed(tokens, "exile")
        && contains_phrase_lexed(tokens, &["top", "card"])
        && contains_word_lexed(tokens, "library")
        && contains_phrase_lexed(tokens, &["face", "down"])
        && contains_word_lexed(tokens, "instead")
}

pub(crate) fn is_library_of_leng_discard_replacement_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    contains_phrase_lexed(tokens, &["effect", "causes", "you"])
        && contains_word_lexed(tokens, "discard")
        && contains_word_lexed(tokens, "top")
        && contains_word_lexed(tokens, "library")
        && contains_word_lexed(tokens, "instead")
        && contains_word_lexed(tokens, "graveyard")
}

pub(crate) fn is_shuffle_into_library_from_graveyard_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    contains_phrase_lexed(tokens, &["would", "be", "put"])
        && contains_word_lexed(tokens, "graveyard")
        && contains_word_lexed(tokens, "anywhere")
        && contains_word_lexed(tokens, "shuffle")
        && contains_word_lexed(tokens, "library")
        && contains_word_lexed(tokens, "instead")
}

pub(crate) fn is_toph_first_metalbender_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    contains_word_lexed(tokens, "nontoken")
        && contains_any_word_lexed(tokens, &["artifact", "artifacts"])
        && (contains_phrase_lexed(tokens, &["you", "control"])
            || contains_phrase_lexed(tokens, &["you", "controls"]))
        && contains_any_word_lexed(tokens, &["land", "lands"])
        && contains_phrase_lexed(tokens, &["in", "addition", "to", "their"])
}

pub(crate) fn is_discard_or_redirect_replacement_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    contains_any_word_lexed(tokens, &["enter", "enters"])
        && contains_word_lexed(tokens, "battlefield")
        && contains_word_lexed(tokens, "discard")
        && contains_word_lexed(tokens, "land")
        && contains_word_lexed(tokens, "instead")
        && contains_word_lexed(tokens, "graveyard")
}

fn parse_unsigned_integer_token<'a>(
    input: &mut LexStream<'a>,
) -> Result<u32, ErrMode<ContextError>> {
    let token: &'a LexToken = any.parse_next(input)?;
    token
        .parser_text()
        .parse::<u32>()
        .map_err(|_| primitives::backtrack_err("unsigned integer", "unsigned integer token"))
}

pub(crate) fn is_protection_mana_value_marker_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        winnow::combinator::alt((
            (
                primitives::phrase(&["protection", "from"]),
                winnow::combinator::alt((primitives::kw("odd"), primitives::kw("even"))),
                primitives::phrase(&["mana", "values"]),
                primitives::sentence_end(),
            )
                .void(),
            (
                primitives::phrase(&["this", "creature", "has", "protection", "from"]),
                primitives::phrase(&["each", "mana", "value", "of", "the", "chosen", "quality"]),
                primitives::sentence_end(),
            )
                .void(),
        )),
    )
    .is_some()
}

pub(crate) fn is_once_each_turn_play_from_exile_marker_guard_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(
        tokens,
        primitives::phrase(&["once", "each", "turn", "you", "may", "play"]),
    )
    .is_some()
        && contains_word_lexed(tokens, "from")
        && contains_word_lexed(tokens, "exile")
        && contains_word_lexed(tokens, "cast")
        && contains_phrase_lexed(tokens, &["spend", "mana"])
        && contains_phrase_lexed(tokens, &["as", "though", "it", "were"])
        && contains_phrase_lexed(tokens, &["any", "color", "to"])
}

pub(crate) fn is_doctors_companion_marker_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        winnow::combinator::alt((
            primitives::phrase(&["doctors", "companion"]),
            primitives::phrase(&["doctor's", "companion"]),
        )),
    )
    .is_some()
}

pub(crate) fn is_companion_marker_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, primitives::kw("companion")).is_some()
}

pub(crate) fn is_more_than_meets_the_eye_marker_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        primitives::phrase(&["more", "than", "meets", "the", "eye"]),
    )
    .is_some()
}

pub(crate) fn is_mana_group_slash_marker_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .any(|token| token.kind == TokenKind::ManaGroup)
        && last_parser_word_text_lexed(tokens)
            .is_some_and(|word| parser_text_contains_char(word, '/'))
}

pub(crate) fn parse_ward_pay_life_amount_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    primitives::parse_prefix(
        tokens,
        seq!(
            _: primitives::kw("ward"),
            _: primitives::kw("pay"),
            parse_unsigned_integer_token,
            _: primitives::kw("life"),
            _: primitives::sentence_end(),
        ),
    )
    .map(|((amount,), _)| amount)
}

pub(crate) fn is_as_long_as_power_odd_or_even_flash_marker_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"])).is_some()
        && contains_word_lexed(tokens, "power")
        && contains_any_word_lexed(tokens, &["odd", "even"])
        && contains_word_lexed(tokens, "flash")
}

pub(crate) fn is_if_source_you_control_with_mana_value_double_instead_marker_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(
        tokens,
        primitives::phrase(&["if", "source", "you", "control", "with"]),
    )
    .is_some()
        && contains_word_lexed(tokens, "mana")
        && contains_word_lexed(tokens, "value")
        && contains_word_lexed(tokens, "double")
        && last_parser_word_text_lexed(tokens) == Some("instead")
}

pub(crate) fn is_attack_as_haste_unless_entered_this_turn_marker_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&[
                "this", "creature", "can", "attack", "as", "though", "it", "had", "haste",
            ]),
            primitives::phrase(&["unless", "it", "entered", "this", "turn"]),
            primitives::sentence_end(),
        )
            .void(),
    )
    .is_some()
}

pub(crate) fn is_sab_sunen_cant_attack_or_block_unless_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(
        tokens,
        winnow::combinator::alt((
            primitives::phrase(&["sab-sunen", "cant", "attack", "or", "block", "unless"]),
            primitives::phrase(&["sab-sunen", "can't", "attack", "or", "block", "unless"]),
        )),
    )
    .is_some()
}

pub(crate) fn split_as_long_as_condition_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<AsLongAsConditionPrefixSpec<'_>> {
    let (_, remainder) =
        primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"]))?;

    for idx in 1..remainder.len() {
        if let Some((_, after_comma)) =
            primitives::parse_prefix(&remainder[idx..], primitives::comma())
        {
            let condition_tokens = trim_lexed_commas(&remainder[..idx]);
            let remainder_tokens = trim_lexed_commas(after_comma);
            if !condition_tokens.is_empty() && !remainder_tokens.is_empty() {
                return Some(AsLongAsConditionPrefixSpec {
                    condition_tokens,
                    remainder_tokens,
                });
            }
        }
    }

    None
}

pub(crate) fn split_if_this_spell_costs_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<IfThisSpellCostsSplitSpec<'_>> {
    let (_, remainder) = primitives::parse_prefix(tokens, primitives::kw("if"))?;

    for idx in 1..remainder.len() {
        let Some((_, after_comma)) =
            primitives::parse_prefix(&remainder[idx..], primitives::comma())
        else {
            continue;
        };

        let condition_tokens = trim_lexed_commas(&remainder[..idx]);
        let tail_tokens = trim_lexed_commas(after_comma);
        if condition_tokens.is_empty() || tail_tokens.is_empty() {
            continue;
        }
        if primitives::parse_prefix(tail_tokens, primitives::phrase(&["this", "spell", "costs"]))
            .is_some()
        {
            return Some(IfThisSpellCostsSplitSpec {
                condition_tokens,
                tail_tokens,
            });
        }
    }

    None
}

fn parse_players_cant_pay_life_or_sacrifice_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::kw("players"),
        winnow::combinator::alt((primitives::kw("cant"), primitives::kw("can't"))),
        primitives::phrase(&[
            "pay",
            "life",
            "or",
            "sacrifice",
            "nonland",
            "permanents",
            "to",
            "cast",
            "spells",
            "or",
            "activate",
            "abilities",
        ]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_players_cant_pay_life_or_sacrifice_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_players_cant_pay_life_or_sacrifice_line).is_some()
}

fn parse_minimum_spell_total_mana_three_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::phrase(&["as", "long", "as"]),
        winnow::combinator::alt((
            primitives::phrase(&["trinisphere", "is", "untapped"]),
            primitives::phrase(&["this", "is", "untapped"]),
        )),
        opt(primitives::comma()),
        primitives::phrase(&[
            "each", "spell", "that", "would", "cost", "less", "than", "three", "mana", "to",
            "cast", "costs", "three", "mana", "to", "cast",
        ]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_minimum_spell_total_mana_three_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_minimum_spell_total_mana_three_line).is_some()
}

fn parse_permanents_enter_tapped_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::kw("permanents"),
        winnow::combinator::alt((primitives::kw("enter"), primitives::kw("enters"))),
        primitives::kw("tapped"),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_permanents_enter_tapped_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_permanents_enter_tapped_line).is_some()
}

fn parse_creatures_entering_dont_cause_abilities_to_trigger_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::kw("creatures"),
        primitives::kw("entering"),
        winnow::combinator::alt((primitives::kw("dont"), primitives::kw("don't"))),
        primitives::phrase(&["cause", "abilities", "to", "trigger"]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_creatures_entering_dont_cause_abilities_to_trigger_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(
        tokens,
        parse_creatures_entering_dont_cause_abilities_to_trigger_line,
    )
    .is_some()
}

fn parse_assign_combat_damage_using_toughness_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::phrase(&[
            "assigns",
            "combat",
            "damage",
            "equal",
            "to",
            "its",
            "toughness",
            "rather",
            "than",
            "its",
            "power",
        ]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn parse_creatures_assign_combat_damage_using_toughness_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CombatDamageUsingToughnessSubject> {
    if let Some((((), ()), remainder)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["each", "creature", "you", "control"]),
            parse_assign_combat_damage_using_toughness_suffix,
        ),
    ) {
        if remainder.is_empty() {
            return Some(CombatDamageUsingToughnessSubject::EachCreatureYouControl);
        }
    }

    if let Some((((), ()), remainder)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["each", "creature"]),
            parse_assign_combat_damage_using_toughness_suffix,
        ),
    ) {
        if remainder.is_empty() {
            return Some(CombatDamageUsingToughnessSubject::EachCreature);
        }
    }

    None
}

fn parse_players_cant_cycle_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::kw("players"),
        winnow::combinator::alt((primitives::kw("cant"), primitives::kw("can't"))),
        primitives::phrase(&["cycle", "cards"]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_players_cant_cycle_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_players_cant_cycle_line).is_some()
}

fn matches_exact_phrase_line_lexed(
    tokens: &[OwnedLexToken],
    phrase: &'static [&'static str],
) -> bool {
    primitives::parse_prefix(
        tokens,
        (primitives::phrase(phrase), primitives::sentence_end()),
    )
    .is_some()
}

fn matches_any_exact_phrase_line_lexed(
    tokens: &[OwnedLexToken],
    phrases: &'static [&'static [&'static str]],
) -> bool {
    phrases
        .iter()
        .copied()
        .any(|phrase| matches_exact_phrase_line_lexed(tokens, phrase))
}

pub(crate) fn is_players_skip_upkeep_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_phrase_line_lexed(tokens, &["players", "skip", "their", "upkeep", "steps"])
}

pub(crate) fn is_all_permanents_colorless_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_phrase_line_lexed(tokens, &["all", "permanents", "are", "colorless"])
}

pub(crate) fn is_blood_moon_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_phrase_line_lexed(tokens, &["nonbasic", "lands", "are", "mountains"])
}

pub(crate) fn is_remove_snow_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_phrase_line_lexed(tokens, &["all", "lands", "are", "no", "longer", "snow"])
}

pub(crate) fn is_no_maximum_hand_size_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_phrase_line_lexed(tokens, &["you", "have", "no", "maximum", "hand", "size"])
}

pub(crate) fn is_can_be_your_commander_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_phrase_line_lexed(tokens, &["this", "can", "be", "your", "commander"])
}

fn parse_creatures_cant_block_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::kw("creatures"),
        winnow::combinator::alt((primitives::kw("cant"), primitives::kw("can't"))),
        primitives::kw("block"),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_creatures_cant_block_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_creatures_cant_block_line).is_some()
}

pub(crate) fn is_you_have_shroud_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_phrase_line_lexed(tokens, &["you", "have", "shroud"])
}

fn parse_creatures_without_flying_cant_attack_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::phrase(&["creatures", "without", "flying"]),
        winnow::combinator::alt((primitives::kw("cant"), primitives::kw("can't"))),
        primitives::kw("attack"),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_creatures_without_flying_cant_attack_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_creatures_without_flying_cant_attack_line).is_some()
}

fn parse_this_creature_cant_attack_alone_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::phrase(&["this", "creature"]),
        winnow::combinator::alt((primitives::kw("cant"), primitives::kw("can't"))),
        primitives::phrase(&["attack", "alone"]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_this_creature_cant_attack_alone_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_this_creature_cant_attack_alone_line).is_some()
}

fn parse_this_creature_cant_attack_its_owner_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::phrase(&["this", "creature"]),
        winnow::combinator::alt((primitives::kw("cant"), primitives::kw("can't"))),
        primitives::phrase(&["attack", "its", "owner"]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_this_creature_cant_attack_its_owner_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_this_creature_cant_attack_its_owner_line).is_some()
}

fn parse_lands_dont_untap_during_their_controllers_untap_steps_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::kw("lands"),
        winnow::combinator::alt((primitives::kw("dont"), primitives::kw("don't"))),
        primitives::kw("untap"),
        primitives::kw("during"),
        primitives::kw("their"),
        winnow::combinator::alt((
            primitives::kw("controllers"),
            primitives::kw("controller's"),
        )),
        primitives::kw("untap"),
        winnow::combinator::alt((primitives::kw("step"), primitives::kw("steps"))),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_lands_dont_untap_during_their_controllers_untap_steps_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(
        tokens,
        parse_lands_dont_untap_during_their_controllers_untap_steps_line,
    )
    .is_some()
}

fn parse_may_assign_damage_as_unblocked_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::kw("you"),
        opt(primitives::kw("may")),
        primitives::kw("have"),
        primitives::kw("this"),
        opt(primitives::kw("creature")),
        primitives::phrase(&["assign", "its", "combat", "damage", "as", "though", "it"]),
        winnow::combinator::alt((
            primitives::kw("werent"),
            primitives::kw("weren't"),
            primitives::kw("wasnt"),
            primitives::kw("wasn't"),
        )),
        primitives::kw("blocked"),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_may_assign_damage_as_unblocked_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_may_assign_damage_as_unblocked_line).is_some()
}

fn parse_source_doesnt_untap_during_your_untap_step_prefix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        primitives::kw("this"),
        opt(winnow::combinator::alt((
            primitives::kw("land"),
            primitives::kw("artifact"),
            primitives::kw("creature"),
        ))),
        winnow::combinator::alt((
            primitives::kw("doesn't").void(),
            primitives::kw("doesnt").void(),
            (primitives::kw("does"), primitives::kw("not")).void(),
        )),
        primitives::phrase(&["untap", "during", "your", "untap", "step"]),
    )
        .void()
        .parse_next(input)
}

fn parse_attached_doesnt_untap_during_controller_untap_step_line<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        winnow::combinator::alt((
            primitives::phrase(&["enchanted", "creature"]),
            primitives::phrase(&["enchanted", "permanent"]),
            primitives::phrase(&["enchanted", "artifact"]),
            primitives::phrase(&["enchanted", "land"]),
            primitives::phrase(&["equipped", "creature"]),
            primitives::phrase(&["equipped", "permanent"]),
        )),
        winnow::combinator::alt((
            primitives::kw("doesn't").void(),
            primitives::kw("doesnt").void(),
            (primitives::kw("does"), primitives::kw("not")).void(),
        )),
        primitives::kw("untap"),
        primitives::kw("during"),
        primitives::kw("its"),
        winnow::combinator::alt((
            primitives::kw("controller"),
            primitives::kw("controllers"),
            primitives::kw("controller's"),
        )),
        primitives::kw("untap"),
        primitives::kw("step"),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn parse_doesnt_untap_during_untap_step_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<DoesntUntapDuringUntapStepSpec<'_>> {
    if let Some(((), tail_tokens)) = primitives::parse_prefix(
        tokens,
        parse_source_doesnt_untap_during_your_untap_step_prefix,
    ) {
        return Some(DoesntUntapDuringUntapStepSpec::Source { tail_tokens });
    }

    for subject_len in [2usize] {
        if tokens.len() < subject_len {
            continue;
        }
        if let Some(((), [])) = primitives::parse_prefix(
            tokens,
            parse_attached_doesnt_untap_during_controller_untap_step_line,
        ) {
            return Some(DoesntUntapDuringUntapStepSpec::Attached {
                subject_tokens: &tokens[..subject_len],
            });
        }
    }

    None
}

pub(crate) fn parse_flying_block_restriction_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<FlyingBlockRestrictionKind> {
    [
        (
            &[
                "this",
                "can't",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
            ][..],
            FlyingBlockRestrictionKind::FlyingOnly,
        ),
        (
            &[
                "this",
                "creature",
                "can't",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
            ][..],
            FlyingBlockRestrictionKind::FlyingOnly,
        ),
        (
            &[
                "this",
                "cant",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
            ][..],
            FlyingBlockRestrictionKind::FlyingOnly,
        ),
        (
            &[
                "this",
                "creature",
                "cant",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
            ][..],
            FlyingBlockRestrictionKind::FlyingOnly,
        ),
        (
            &[
                "this",
                "can't",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
                "or",
                "reach",
            ][..],
            FlyingBlockRestrictionKind::FlyingOrReach,
        ),
        (
            &[
                "this",
                "creature",
                "can't",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
                "or",
                "reach",
            ][..],
            FlyingBlockRestrictionKind::FlyingOrReach,
        ),
        (
            &[
                "this",
                "cant",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
                "or",
                "reach",
            ][..],
            FlyingBlockRestrictionKind::FlyingOrReach,
        ),
        (
            &[
                "this",
                "creature",
                "cant",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
                "or",
                "reach",
            ][..],
            FlyingBlockRestrictionKind::FlyingOrReach,
        ),
    ]
    .into_iter()
    .find_map(|(phrase, kind)| matches_exact_phrase_line_lexed(tokens, phrase).then_some(kind))
}

pub(crate) fn is_can_block_only_flying_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_exact_phrase_line_lexed(
        tokens,
        &[
            &[
                "this",
                "can",
                "block",
                "only",
                "creatures",
                "with",
                "flying",
            ],
            &[
                "this",
                "creature",
                "can",
                "block",
                "only",
                "creatures",
                "with",
                "flying",
            ],
            &["can", "block", "only", "creatures", "with", "flying"],
            &["this", "can", "block", "only", "creature", "with", "flying"],
            &[
                "this", "creature", "can", "block", "only", "creature", "with", "flying",
            ],
        ],
    )
}

pub(crate) fn is_skulk_rules_text_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_exact_phrase_line_lexed(
        tokens,
        &[
            &[
                "creatures",
                "with",
                "power",
                "less",
                "than",
                "this",
                "creature's",
                "power",
                "can't",
                "block",
                "it",
            ],
            &[
                "creatures",
                "with",
                "power",
                "less",
                "than",
                "this",
                "creature's",
                "power",
                "can't",
                "block",
                "this",
                "creature",
            ],
            &[
                "creatures",
                "with",
                "power",
                "less",
                "than",
                "this",
                "creatures",
                "power",
                "cant",
                "block",
                "it",
            ],
            &[
                "creatures",
                "with",
                "power",
                "less",
                "than",
                "this",
                "creatures",
                "power",
                "cant",
                "block",
                "this",
                "creature",
            ],
        ],
    )
}

pub(crate) fn is_prevent_all_damage_dealt_to_creatures_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    matches_exact_phrase_line_lexed(
        tokens,
        &[
            "prevent",
            "all",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
            "creatures",
        ],
    )
}

pub(crate) fn is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&[
                "if", "damage", "would", "be", "dealt", "to", "another", "creature", "you",
                "control",
            ]),
            opt(primitives::comma()),
            primitives::phrase(&["prevent", "that", "damage"]),
            opt(primitives::period()),
            primitives::phrase(&[
                "put",
                "a",
                "+1/+1",
                "counter",
                "on",
                "that",
                "creature",
                "for",
                "each",
                "1",
                "damage",
                "prevented",
                "this",
                "way",
            ]),
            primitives::sentence_end(),
        ),
    )
    .is_some()
}

pub(crate) fn is_prevent_all_combat_damage_to_source_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_exact_phrase_line_lexed(
        tokens,
        &[
            &[
                "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to", "this",
                "creature",
            ],
            &[
                "prevent",
                "all",
                "combat",
                "damage",
                "that",
                "would",
                "be",
                "dealt",
                "to",
                "this",
                "permanent",
            ],
            &[
                "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to", "it",
            ],
        ],
    )
}

pub(crate) fn is_prevent_all_damage_to_source_by_creatures_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    matches_any_exact_phrase_line_lexed(
        tokens,
        &[
            &[
                "prevent",
                "all",
                "damage",
                "that",
                "would",
                "be",
                "dealt",
                "to",
                "this",
                "creature",
                "by",
                "creatures",
            ],
            &[
                "prevent",
                "all",
                "damage",
                "that",
                "would",
                "be",
                "dealt",
                "to",
                "this",
                "permanent",
                "by",
                "creatures",
            ],
        ],
    )
}

pub(crate) fn is_you_may_look_top_card_any_time_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_exact_phrase_line_lexed(
        tokens,
        &[
            &[
                "you", "may", "look", "at", "the", "top", "card", "of", "your", "library", "any",
                "time",
            ],
            &[
                "you", "may", "look", "at", "top", "card", "of", "your", "library", "any", "time",
            ],
        ],
    )
}

pub(crate) fn is_cast_this_spell_as_though_it_had_flash_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    matches_any_exact_phrase_line_lexed(
        tokens,
        &[
            &[
                "you", "may", "cast", "this", "spell", "as", "though", "it", "had", "flash",
            ],
            &[
                "you", "may", "cast", "this", "as", "though", "it", "had", "flash",
            ],
        ],
    )
}

pub(crate) fn is_play_lands_from_graveyard_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_phrase_line_lexed(
        tokens,
        &["you", "may", "play", "lands", "from", "your", "graveyard"],
    )
}

pub(crate) fn is_this_subject_reference_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_exact_phrase_line_lexed(tokens, &[&["this"], &["this's"], &["thiss"]])
}

pub(crate) fn parse_source_tap_status_condition_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ConditionExpr> {
    [
        (
            &["this", "creature", "is", "tapped"][..],
            ConditionExpr::SourceIsTapped,
        ),
        (
            &["this", "permanent", "is", "tapped"][..],
            ConditionExpr::SourceIsTapped,
        ),
        (&["it", "is", "tapped"][..], ConditionExpr::SourceIsTapped),
        (
            &["this", "creature", "is", "untapped"][..],
            ConditionExpr::SourceIsUntapped,
        ),
        (
            &["this", "permanent", "is", "untapped"][..],
            ConditionExpr::SourceIsUntapped,
        ),
        (
            &["it", "is", "untapped"][..],
            ConditionExpr::SourceIsUntapped,
        ),
    ]
    .into_iter()
    .find_map(|(phrase, condition)| {
        matches_exact_phrase_line_lexed(tokens, phrase).then_some(condition)
    })
}

pub(crate) fn is_enchanted_land_is_chosen_type_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_exact_phrase_line_lexed(
        tokens,
        &[
            &["enchanted", "land", "is", "the", "chosen", "type"],
            &["enchanted", "land", "is", "chosen", "type"],
        ],
    )
}

pub(crate) fn parse_source_is_chosen_type_in_addition_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&'static str> {
    [
        (
            &[
                "this", "creature", "is", "the", "chosen", "type", "in", "addition", "to", "its",
                "other", "types",
            ][..],
            "This creature is the chosen type in addition to its other types.",
        ),
        (
            &[
                "this",
                "permanent",
                "is",
                "the",
                "chosen",
                "type",
                "in",
                "addition",
                "to",
                "its",
                "other",
                "types",
            ][..],
            "This permanent is the chosen type in addition to its other types.",
        ),
        (
            &[
                "it", "is", "the", "chosen", "type", "in", "addition", "to", "its", "other",
                "types",
            ][..],
            "It is the chosen type in addition to its other types.",
        ),
    ]
    .into_iter()
    .find_map(|(phrase, display)| {
        matches_exact_phrase_line_lexed(tokens, phrase).then_some(display)
    })
}

pub(crate) fn is_double_damage_from_sources_you_control_of_chosen_type_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    matches_exact_phrase_line_lexed(
        tokens,
        &[
            "double", "all", "damage", "that", "sources", "you", "control", "of", "the", "chosen",
            "type", "would", "deal",
        ],
    )
}

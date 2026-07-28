use winnow::combinator::{opt, seq};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::any;

use crate::ConditionExpr;
use crate::PowerToughness;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::target::PlayerFilter;

use super::super::lexer::{
    LexStream, LexToken, OwnedLexToken, TokenKind, contains_token_any_word, contains_token_word,
    contains_token_word_sequence, locate_token_kind, locate_token_word, trim_lexed_commas,
};
use super::super::util::trim_edge_punctuation_tokens;
use super::primitives;
use crate::runtime_backend::util::parse_counter_type_word;

mod activation_conditions;
mod flashback;
mod mana_usage;
mod spell_countered_trigger;
mod static_shapes;
mod surface;

pub(crate) use activation_conditions::{
    is_activate_only_restriction_sentence_lexed, is_any_player_may_activate_sentence_lexed,
    is_trigger_only_restriction_sentence_lexed, parse_activate_only_timing_lexed,
    parse_activation_condition_lexed, parse_triggered_times_each_turn_lexed,
};
pub(crate) use flashback::{
    FlashbackCostClause, parse_flashback_cost_clause_tokens,
    parse_flashback_keyword_line_spec_lexed,
};
pub(crate) use mana_usage::{
    is_mana_spend_bonus_sentence_lexed, is_spend_mana_restriction_sentence_lexed,
    parse_mana_spend_bonus_sentence_lexed, parse_mana_usage_restriction_sentence_lexed,
};
pub(crate) use spell_countered_trigger::parse_spell_countered_trigger_spec_lexed;
pub(crate) use static_shapes::{
    is_draw_replacement_double_line_lexed, is_draw_replacement_skip_empty_library_line_lexed,
    is_draw_replacement_win_empty_library_line_lexed, is_land_reveal_enters_static_line_lexed,
    is_opening_hand_begin_game_static_line_lexed,
    is_opponent_effect_discard_this_to_battlefield_replacement_line_lexed,
    is_prevent_all_combat_damage_to_matching_permanents_line_lexed,
    is_prevent_all_noncombat_damage_to_matching_permanents_line_lexed,
    parse_can_block_subtype_as_though_reach_line_lexed,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UntapEachOtherPlayersUntapStepSpec<'a> {
    pub(crate) untap_all: bool,
    pub(crate) subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CombatDamageUsingToughnessSubject {
    ThisCreature,
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
    Source {
        tail_tokens: &'a [OwnedLexToken],
    },
    Attached {
        subject_tokens: &'a [OwnedLexToken],
        tail_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivatedAbilitiesCantBeActivatedSpec<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) non_mana_only: bool,
}

fn ability_token_kind_index(tokens: &[OwnedLexToken], kind: TokenKind) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if tokens[idx].kind == kind {
            return Some(idx);
        }
        idx += 1;
    }
    None
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

const WHENEVER_WORD: &str = "whenever";
fn black_mana_group<'a>(input: &mut LexStream<'a>) -> Result<ManaSymbol, ErrMode<ContextError>> {
    super::leaf::parse_leaf_mana_group_token
        .verify(|group: &Vec<ManaSymbol>| matches!(group.as_slice(), [ManaSymbol::Black]))
        .map(|_| ManaSymbol::Black)
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
    let ((_, untap_all), remainder) = primitives::parse_prefix(
        tokens,
        (primitives::kw("untap"), opt(primitives::kw("all"))),
    )?;
    let (subject_tokens, ()) = primitives::split_lexed_once_before_suffix(remainder, 1, || {
        parse_each_other_players_untap_step_suffix
    })?;
    Some(UntapEachOtherPlayersUntapStepSpec {
        untap_all: untap_all.is_some(),
        subject_tokens,
    })
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

fn parse_dont_word<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    winnow::combinator::alt((primitives::kw("don't"), primitives::kw("dont")))
        .void()
        .parse_next(input)
}

pub(crate) fn split_nested_combat_whenever_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (_, after_intro) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["at", "the", "beginning", "of", "each", "combat"]),
    )?;
    let after_unless = trim_lexed_commas(after_intro);
    let (_, after_pay) =
        primitives::parse_prefix(after_unless, primitives::phrase(&["unless", "you", "pay"]))?;
    let (_, nested_trigger_tokens) = primitives::split_lexed_once_on_comma(after_pay)?;
    nested_trigger_tokens
        .first()
        .is_some_and(|token| token.is_word(WHENEVER_WORD))
        .then_some(nested_trigger_tokens)
}

pub(crate) fn is_activate_only_once_each_turn_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, rest)) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["activate", "only", "once", "each", "turn"]),
    ) else {
        return false;
    };
    primitives::parse_prefix(rest, primitives::end_of_sentence_or_block())
        .is_some_and(|(_, remainder)| remainder.is_empty())
}

pub(crate) fn is_doesnt_untap_during_your_untap_step_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, head_tokens)) = primitives::strip_lexed_suffix_phrases(
        tokens,
        &[&["untap", "during", "your", "untap", "step"]],
    ) else {
        return false;
    };

    let head_tokens = trim_lexed_commas(head_tokens);
    if head_tokens.is_empty() {
        return false;
    }

    primitives::find_prefix(head_tokens, || {
        winnow::combinator::alt((
            primitives::kw("don't").void(),
            primitives::kw("dont").void(),
            primitives::kw("doesn't").void(),
            primitives::kw("doesnt").void(),
            (primitives::kw("do"), primitives::kw("not")).void(),
            (primitives::kw("does"), primitives::kw("not")).void(),
        ))
    })
    .is_some()
}

pub(crate) fn is_ward_or_echo_static_prefix_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        winnow::combinator::alt((primitives::kw("ward"), primitives::kw("echo"))),
    )
    .is_some()
}

pub(crate) fn is_land_reveal_enters_tapped_followup_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        (
            primitives::phrase(&["if", "you"]),
            parse_dont_word,
            winnow::combinator::opt(primitives::comma()),
            winnow::combinator::alt((
                primitives::phrase(&["this", "land", "enters", "tapped"]),
                primitives::phrase(&["it", "enters", "tapped"]),
            )),
        )
            .void()
            .parse_next(input)
    })
    .is_some()
}

pub(crate) fn is_standard_gift_keyword_tokens_lexed(tokens: &[OwnedLexToken]) -> bool {
    let head_tokens = ability_token_kind_index(tokens, TokenKind::LParen)
        .map(|idx| &tokens[..idx])
        .unwrap_or(tokens);
    if !surface::matches_prefix_tokens(head_tokens, &["gift"]) {
        return false;
    }

    const STANDARD_GIFT_KEYWORD_PHRASES: &[&[&str]] = &[
        &["gift", "a", "card"],
        &["gift", "a", "treasure"],
        &["gift", "a", "food"],
        &["gift", "a", "tapped", "fish"],
        &["gift", "an", "extra", "turn"],
        &["gift", "an", "octopus"],
    ];
    surface::matches_any_prefix_tokens(head_tokens, STANDARD_GIFT_KEYWORD_PHRASES)
}

pub(crate) fn additional_cost_tail_tokens_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let comma_idx = locate_token_kind(tokens, TokenKind::Comma);
    let effect_start = if let Some(idx) = comma_idx {
        idx + 1
    } else if let Some(idx) = locate_token_word(tokens, "spell") {
        idx + 1
    } else {
        tokens.len()
    };
    let effect_tokens = tokens.get(effect_start..).unwrap_or_default();
    (!effect_tokens.is_empty()).then_some(effect_tokens)
}

pub(crate) fn is_additional_cost_choice_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        primitives::phrase(&[
            "as",
            "an",
            "additional",
            "cost",
            "to",
            "cast",
            "this",
            "spell",
        ]),
    )
    .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::TokenWordView;
    use crate::runtime_backend::lexer::lex_line;

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
        assert!(spec.untap_all);
    }

    #[test]
    fn untap_each_other_players_untap_step_supports_singular_subjects() {
        let tokens = lex_line(
            "Untap this artifact during each other player's untap step.",
            0,
        )
        .unwrap();
        let spec = split_untap_each_other_players_untap_step_line_lexed(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(spec.subject_tokens).word_refs(),
            ["this", "artifact"]
        );
        assert!(!spec.untap_all);
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

    #[test]
    fn activation_max_speed_condition_uses_player_status_capture() {
        for text in [
            "Activate only if you have max speed.",
            "Activate only if you have maximum speed.",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let condition = parse_activation_condition_lexed(&tokens).unwrap();
            assert_eq!(
                condition,
                ConditionExpr::ValueComparison {
                    left: crate::effect::Value::Speed(PlayerFilter::You),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: crate::effect::Value::Fixed(4),
                },
                "{text}"
            );
        }
    }
}

fn last_parser_word_text_lexed(tokens: &[OwnedLexToken]) -> Option<&str> {
    tokens.iter().rev().find_map(|token| match token.kind {
        TokenKind::Word | TokenKind::Number | TokenKind::Tilde => Some(token.parser_text()),
        _ => None,
    })
}

fn parser_text_contains_char(text: &str, expected: char) -> bool {
    crate::string_primitives::contains_char(text, expected)
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

    contains_token_word(tokens, "exile")
        && contains_token_word_sequence(tokens, &["top", "card"])
        && contains_token_word(tokens, "library")
        && contains_token_word_sequence(tokens, &["face", "down"])
        && contains_token_word(tokens, "instead")
}

pub(crate) fn is_effect_discard_to_library_replacement_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    contains_token_word_sequence(tokens, &["effect", "causes", "you"])
        && contains_token_word(tokens, "discard")
        && contains_token_word(tokens, "top")
        && contains_token_word(tokens, "library")
        && contains_token_word(tokens, "instead")
        && contains_token_word(tokens, "graveyard")
}

pub(crate) fn is_shuffle_into_library_from_graveyard_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    contains_token_word_sequence(tokens, &["would", "be", "put"])
        && contains_token_word(tokens, "graveyard")
        && contains_token_word(tokens, "anywhere")
        && contains_token_word(tokens, "shuffle")
        && contains_token_word(tokens, "library")
        && contains_token_word(tokens, "instead")
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
        && contains_token_word(tokens, "from")
        && contains_token_word(tokens, "exile")
        && contains_token_word(tokens, "cast")
        && contains_token_word_sequence(tokens, &["spend", "mana"])
        && contains_token_word_sequence(tokens, &["as", "though", "it", "were"])
        && contains_token_word_sequence(tokens, &["any", "color", "to"])
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrototypeKeywordSpec {
    pub(crate) cost: ManaCost,
    pub(crate) power_toughness: PowerToughness,
}

pub(crate) fn parse_prototype_keyword_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PrototypeKeywordSpec> {
    let (_, rest) = primitives::parse_prefix(tokens, primitives::kw("prototype"))?;
    let cost = super::leaf::parse_leaf_mana_cost_prefix_tokens(rest)?;
    let tail = trim_edge_punctuation_tokens(rest.get(cost.consumed..)?);
    let [power_toughness] = tail else {
        return None;
    };
    Some(PrototypeKeywordSpec {
        cost: cost.cost,
        power_toughness: super::leaf::parse_leaf_power_toughness_complete(
            power_toughness.parser_text(),
        )
        .ok()?,
    })
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
        && contains_token_word(tokens, "power")
        && contains_token_any_word(tokens, &["odd", "even"])
        && contains_token_word(tokens, "flash")
}

pub(crate) fn is_if_source_you_control_with_mana_value_double_instead_marker_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(
        tokens,
        primitives::phrase(&["if", "source", "you", "control", "with"]),
    )
    .is_some()
        && contains_token_word(tokens, "mana")
        && contains_token_word(tokens, "value")
        && contains_token_word(tokens, "double")
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
    let parsed = super::leaf::parse_leaf_condition_intro_prefix_tokens(tokens)?;
    if parsed.intro != super::leaf::ConditionIntro::AsLongAs {
        return None;
    }
    let (condition_tokens, remainder_tokens) = primitives::split_lexed_once_on_comma(parsed.rest)?;
    let condition_tokens = trim_lexed_commas(condition_tokens);
    let remainder_tokens = trim_lexed_commas(remainder_tokens);
    if condition_tokens.is_empty() || remainder_tokens.is_empty() {
        return None;
    }

    Some(AsLongAsConditionPrefixSpec {
        condition_tokens,
        remainder_tokens,
    })
}

pub(crate) fn split_if_this_spell_costs_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<IfThisSpellCostsSplitSpec<'_>> {
    let parsed = super::leaf::parse_leaf_condition_intro_prefix_tokens(tokens)?;
    if parsed.intro != super::leaf::ConditionIntro::If {
        return None;
    }
    let (condition_tokens, tail_tokens) = primitives::split_lexed_once_on_comma(parsed.rest)?;
    let condition_tokens = trim_lexed_commas(condition_tokens);
    let tail_tokens = trim_lexed_commas(tail_tokens);
    if condition_tokens.is_empty() || tail_tokens.is_empty() {
        return None;
    }
    if primitives::parse_prefix(tail_tokens, primitives::phrase(&["this", "spell", "costs"]))
        .is_none()
    {
        return None;
    }

    Some(IfThisSpellCostsSplitSpec {
        condition_tokens,
        tail_tokens,
    })
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
            primitives::phrase(&["this", "creature"]),
            parse_assign_combat_damage_using_toughness_suffix,
        ),
    ) {
        if remainder.is_empty() {
            return Some(CombatDamageUsingToughnessSubject::ThisCreature);
        }
    }

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

pub(crate) fn is_you_assign_combat_damage_of_creatures_attacking_you_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    const EXPECTED: &[&str] = &[
        "rather",
        "than",
        "the",
        "attacking",
        "player",
        "you",
        "assign",
        "the",
        "combat",
        "damage",
        "of",
        "each",
        "creature",
        "attacking",
        "you",
        "you",
        "can",
        "divide",
        "that",
        "creature's",
        "combat",
        "damage",
        "as",
        "you",
        "choose",
        "among",
        "any",
        "of",
        "the",
        "creatures",
        "blocking",
        "it",
    ];
    words.len() == EXPECTED.len()
        && words
            .iter()
            .zip(EXPECTED.iter())
            .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
}

pub(crate) fn is_lethal_damage_to_creatures_you_control_uses_power_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&[
                "lethal",
                "damage",
                "dealt",
                "to",
                "creatures",
                "you",
                "control",
                "is",
                "determined",
                "by",
                "their",
                "power",
                "rather",
                "than",
                "their",
                "toughness",
            ]),
            primitives::sentence_end(),
        ),
    )
    .is_some_and(|(((), ()), remainder)| remainder.is_empty())
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

pub(crate) fn is_skip_your_draw_step_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_phrase_line_lexed(tokens, &["skip", "your", "draw", "step"])
}

pub(crate) fn is_all_permanents_colorless_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_exact_phrase_line_lexed(tokens, &["all", "permanents", "are", "colorless"])
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
        parse_dependent_doesnt_untap_during_controller_untap_step,
    )
        .void()
        .parse_next(input)
}

fn parse_dependent_doesnt_untap_during_controller_untap_step<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
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
    )
        .void()
        .parse_next(input)
}

pub(crate) fn is_dependent_doesnt_untap_during_controller_untap_step_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_all(
        trim_edge_punctuation_tokens(tokens),
        parse_dependent_doesnt_untap_during_controller_untap_step,
        "dependent doesn't-untap during controller untap step",
    )
    .is_ok()
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
        if let Some(((), tail_tokens)) = primitives::parse_prefix(
            tokens,
            parse_attached_doesnt_untap_during_controller_untap_step_line,
        ) {
            let tail_tokens = trim_edge_punctuation_tokens(tail_tokens);
            return Some(DoesntUntapDuringUntapStepSpec::Attached {
                subject_tokens: &tokens[..subject_len],
                tail_tokens,
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

pub(crate) fn is_during_your_turn_prevent_all_damage_to_source_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["during", "your", "turn"]),
            opt(primitives::comma()),
            primitives::phrase(&[
                "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
            ]),
            primitives::phrase(&["this", "creature"]),
            primitives::sentence_end(),
        ),
    )
    .is_some()
        || primitives::parse_prefix(
            tokens,
            (
                primitives::phrase(&["during", "your", "turn"]),
                opt(primitives::comma()),
                primitives::phrase(&[
                    "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
                ]),
                primitives::phrase(&["this", "permanent"]),
                primitives::sentence_end(),
            ),
        )
        .is_some()
        || primitives::parse_prefix(
            tokens,
            (
                primitives::phrase(&["during", "your", "turn"]),
                opt(primitives::comma()),
                primitives::phrase(&[
                    "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
                ]),
                primitives::phrase(&["it"]),
                primitives::sentence_end(),
            ),
        )
        .is_some()
}

pub(crate) fn is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    matches_exact_phrase_line_lexed(
        tokens,
        &[
            "prevent",
            "all",
            "noncombat",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
            "other",
            "creatures",
            "you",
            "control",
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

pub(crate) fn is_you_may_look_face_down_creatures_you_dont_control_any_time_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    matches_any_exact_phrase_line_lexed(
        tokens,
        &[
            &[
                "you",
                "may",
                "look",
                "at",
                "face-down",
                "creatures",
                "you",
                "dont",
                "control",
                "any",
                "time",
            ],
            &[
                "you",
                "may",
                "look",
                "at",
                "face-down",
                "creatures",
                "you",
                "don't",
                "control",
                "any",
                "time",
            ],
        ],
    )
}

pub(crate) fn is_players_play_top_card_libraries_revealed_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    matches_any_exact_phrase_line_lexed(
        tokens,
        &[&[
            "players",
            "play",
            "with",
            "the",
            "top",
            "card",
            "of",
            "their",
            "libraries",
            "revealed",
        ]],
    )
}

pub(crate) fn is_play_top_card_your_library_revealed_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches_any_exact_phrase_line_lexed(
        tokens,
        &[&[
            "play", "with", "the", "top", "card", "of", "your", "library", "revealed",
        ]],
    )
}

pub(crate) fn is_your_opponents_play_with_hands_revealed_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    matches_any_exact_phrase_line_lexed(
        tokens,
        &[&[
            "your",
            "opponents",
            "play",
            "with",
            "their",
            "hands",
            "revealed",
        ]],
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
    let condition = super::conditions::parse_subject_status_condition(tokens)?;
    if matches!(
        condition.state,
        super::conditions::StatusConditionStateAst::Tapped
            | super::conditions::StatusConditionStateAst::Untapped
    ) {
        condition.condition_expr()
    } else {
        None
    }
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

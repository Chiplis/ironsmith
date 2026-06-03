use winnow::combinator::{opt, seq};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::any;

use crate::ConditionExpr;
use crate::ability::{ActivationTiming, ManaUsageRestriction};
use crate::color::{Color, ColorSet};
use crate::object::CounterType;
use crate::static_abilities::StaticAbilityId;
use crate::target::ObjectFilter;
use crate::target::PlayerFilter;
use crate::zone::Zone;

use super::super::activation_helpers::parse_subtype_flexible;
use super::super::effect_sentences::parse_subtype_word;
use super::super::lexer::{
    LexStream, LexToken, OwnedLexToken, TokenKind, TokenWordView, contains_token_any_word,
    contains_token_word, contains_token_word_sequence, find_token_any_word, find_token_kind,
    find_token_word, token_slice_first_is, token_slice_first_is_any, token_slice_starts_with,
    trim_lexed_commas, word_slice_contains_any_word, word_slice_eq, word_slice_starts_with,
    word_slice_strip_any_prefix,
};
use super::super::token_primitives::{slice_contains, str_strip_suffix};
use super::conditions::{ControlConditionOptions, parse_control_condition};
use super::filters::parse_spell_filter_with_grammar_entrypoint;
use super::primitives;
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};
use crate::runtime_backend::util::{
    parse_card_type, parse_counter_type_from_tokens, parse_counter_type_word,
    parse_less_than_or_equal_quantity_prefix, parse_number, strip_leading_article_word_refs,
};
use crate::runtime_backend::value_helpers::parse_filter_comparison_tokens;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UntapEachOtherPlayersUntapStepSpec<'a> {
    pub(crate) untap_all: bool,
    pub(crate) subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatDamageUsingToughnessSubject {
    ThisCreature,
    EachCreature,
    EachCreatureYouControl,
}

fn parse_number_words(words: &[&str]) -> Option<(u32, usize)> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_number(&tokens)
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

const MANA_CANT_BE_SPENT_TO_CAST_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "mana", "cant", "be", "spent", "to", "cast"],
            &["this", "mana", "can't", "be", "spent", "to", "cast"],
            &["that", "mana", "cant", "be", "spent", "to", "cast"],
            &["that", "mana", "can't", "be", "spent", "to", "cast"],
        ]
);
const SPEND_MANA_ACTIVATE_ABILITY_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "spend",
                "this",
                "mana",
                "only",
                "to",
                "activate",
                "abilities"
            ],
            &[
                "spend", "this", "mana", "only", "to", "activate", "an", "ability",
            ],
        ]
);
const ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["a"], &["an"]]);
const OF_THE_CHOSEN_TYPE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["of", "the", "chosen", "type"]);
const THAT_SPELL_CANT_BE_COUNTERED_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["and", "that", "spell", "can't", "be", "countered"],
            &["and", "that", "spell", "cant", "be", "countered"],
        ]
);
const MANA_USAGE_UNSUPPORTED_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_words
        & [&[
            "activate",
            "activates",
            "activated",
            "activation",
            "ability",
            "abilities",
            "pay",
            "foretell",
            "unlock",
            "turn",
            "cost",
            "costs",
        ]]
);
const PLAIN_SPELL_USAGE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["the"], &["spell"], &["spells"]]);
const YOUR_COMMANDER_USAGE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["your", "commander"],
            &["your", "commander", "spell"],
            &["your", "commander", "spells"]
        ]
);
const MONOCOLORED_OF_CHOSEN_COLOR_USAGE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["monocolored", "spell", "of", "that", "color"],
            &["monocolored", "spells", "of", "that", "color"],
            &["monocolored", "spell", "of", "the", "chosen", "color"],
            &["monocolored", "spells", "of", "the", "chosen", "color"],
        ]
);
const SPELL_FROM_YOUR_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["spell", "from", "your", "graveyard"],
            &["spells", "from", "your", "graveyard"]
        ]
);
const SPELL_FROM_EXILE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["spell", "from", "exile"], &["spells", "from", "exile"]]);
const SPELL_WITH_DEVOID_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["spell", "with", "devoid"], &["spells", "with", "devoid"]]);
const CREATURE_SPELL_WITH_NO_ABILITIES_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["creature", "spell", "with", "no", "abilities"],
            &["creature", "spells", "with", "no", "abilities"]
        ]
);
const SPELL_YOU_DONT_OWN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["spell", "you", "don't", "own"],
            &["spell", "you", "dont", "own"],
            &["spells", "you", "don't", "own"],
            &["spells", "you", "dont", "own"],
        ]
);
const THAT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that"]);
const CAST_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["cast"]);
const ENTER_OR_ENTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enter"], &["enters"]]);
const MANA_SPEND_COUNTER_SUBJECT_NOUN_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["creature"], &["spell"], &["permanent"], &["card"]]);
const ADDITIONAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["additional"]);
const TRIGGER_ONLY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "ability", "triggers", "only"]);
const DO_THIS_ONLY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["do", "this", "only"]);
const TIME_OR_TIMES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["time"], &["times"]]);
const DRAW_REPLACEMENT_DOUBLE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "if", "you", "would", "draw", "a", "card", "draw", "two", "cards", "instead",
        ]
);
const DRAW_REPLACEMENT_SKIP_EMPTY_LIBRARY_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "if", "you", "would", "draw", "a", "card", "while", "your", "library", "has", "no",
            "cards", "in", "it", "skip", "that", "draw", "instead",
        ]
);
const ACTIVATE_ONLY_IF_THERE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["activate", "only", "if", "there", "is"],
            &["activate", "only", "if", "there", "are"]
        ]
);
const YOUR_GRAVEYARD_ZONE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["in", "your", "graveyard"],
            &["in", "graveyard"],
            &["in", "the", "graveyard"],
        ]
);
const ACTIVATE_ONLY_IF_CREATURES_TOTAL_POWER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "activate",
            "only",
            "if",
            "creatures",
            "you",
            "control",
            "have",
            "total",
            "power",
        ]
);
const ACTIVATE_ONLY_IF_RED_SOURCES_DEALT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "activate",
            "only",
            "if",
            "red",
            "sources",
            "you",
            "controlled",
            "dealt",
        ]
);
const NONCOMBAT_DAMAGE_THIS_TURN_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["or", "more", "noncombat", "damage", "this", "turn"],
            &[
                "or",
                "more",
                "noncombat",
                "damage",
                "this",
                "turn",
                "and",
                "only",
                "as",
                "a",
                "sorcery",
            ],
        ]
);
const ACTIVATE_ONLY_IF_YOU_CONTROL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["activate", "only", "if", "you", "control"]);
const CREATURE_WITH_POWER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["a", "creature", "with", "power"],
            &["creature", "with", "power"]
        ]
);
const POWER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["power"]);
const EACH_TURN_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["each", "turn"]);
const WHENEVER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["whenever"]);
const BLACK_MANA_GROUP_TEXT: &str = "{b}";
const CAN_BLOCK_SUBTYPE_AS_THOUGH_REACH_SHAPES: &[(&[&str], &[&str])] = &[
    (
        &["this", "creature", "can", "block"],
        &["as", "though", "it", "had", "reach"],
    ),
    (
        &["this", "can", "block"],
        &["as", "though", "it", "had", "reach"],
    ),
];

fn black_mana_group<'a>(input: &mut LexStream<'a>) -> Result<&'a LexToken, ErrMode<ContextError>> {
    any.verify(|token: &&LexToken| {
        token.kind == TokenKind::ManaGroup && token.parser_text() == BLACK_MANA_GROUP_TEXT
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
        .is_some_and(|token| WHENEVER_WORD_PATTERN.matches_token(token))
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

pub(crate) fn is_land_reveal_enters_static_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        primitives::phrase(&["as", "this", "land", "enters"]),
    )
    .is_some()
        && primitives::contains_phrase(tokens, &["you", "may", "reveal"])
        && primitives::contains_phrase(tokens, &["from", "your", "hand"])
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

pub(crate) fn is_opening_hand_begin_game_static_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        primitives::phrase(&["if", "this", "card", "is", "in", "your", "opening", "hand"]),
    )
    .is_some()
        && primitives::contains_phrase(tokens, &["you", "may", "begin", "the", "game", "with"])
        && primitives::contains_phrase(tokens, &["on", "the", "battlefield"])
}

const ACTIVATE_ONLY_RESTRICTION_PREFIXES: &[&[&str]] =
    &[&["activate", "only"], &["activate", "no", "more", "than"]];
const SPEND_MANA_RESTRICTION_PREFIXES: &[&[&str]] = &[
    &["spend", "only", "mana"],
    &["spend", "this", "mana", "only"],
    &["spend", "that", "mana", "only"],
    &["this", "mana", "cant", "be", "spent", "to", "cast"],
    &["this", "mana", "can't", "be", "spent", "to", "cast"],
    &["that", "mana", "cant", "be", "spent", "to", "cast"],
    &["that", "mana", "can't", "be", "spent", "to", "cast"],
];
const SPEND_MANA_CAST_PREFIXES: &[&[&str]] = &[
    &["spend", "this", "mana", "only", "to", "cast"],
    &["spend", "that", "mana", "only", "to", "cast"],
];
const IF_MANA_SPENT_SPELL_PREFIXES: &[&[&str]] = &[
    &["if", "this", "mana", "is", "spent", "to", "cast"],
    &["if", "that", "mana", "is", "spent", "to", "cast"],
    &["if", "this", "mana", "is", "spent", "on"],
    &["if", "that", "mana", "is", "spent", "on"],
];
const DURING_OPPONENTS_TURN_PREFIXES: &[&[&str]] = &[
    &["activate", "only", "during", "an", "opponents", "turn"],
    &["activate", "only", "during", "opponents", "turn"],
];
const ACTIVATE_ONLY_INSTANT_PREFIXES: &[&[&str]] = &[
    &["activate", "only", "as", "an", "instant"],
    &["activate", "only", "as", "instant"],
];
const ACTIVATE_ONLY_SORCERY_PREFIXES: &[&[&str]] = &[&["activate", "only", "as", "a", "sorcery"]];
const ACTIVATE_ONLY_ONCE_EACH_TURN_PREFIXES: &[&[&str]] =
    &[&["activate", "only", "once", "each", "turn"]];
const ACTIVATE_ONLY_DURING_COMBAT_PREFIXES: &[&[&str]] =
    &[&["activate", "only", "during", "combat"]];
const ACTIVATE_ONLY_DURING_YOUR_TURN_PREFIXES: &[&[&str]] =
    &[&["activate", "only", "during", "your", "turn"]];
const THIS_ABILITY_TRIGGERS_ONLY_PREFIXES: &[&[&str]] = &[
    &["this", "ability", "triggers", "only"],
    &["do", "this", "only"],
];

pub(crate) fn parse_activate_only_timing_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ActivationTiming> {
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_SORCERY_PREFIXES).is_some() {
        return Some(ActivationTiming::SorcerySpeed);
    }
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_ONCE_EACH_TURN_PREFIXES).is_some()
        || primitives::words_find_phrase(tokens, &["once", "each", "turn"]).is_some()
    {
        return Some(ActivationTiming::OncePerTurn);
    }
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_DURING_COMBAT_PREFIXES).is_some()
        || primitives::words_find_phrase(tokens, &["during", "combat"]).is_some()
    {
        return Some(ActivationTiming::DuringCombat);
    }
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_DURING_YOUR_TURN_PREFIXES).is_some()
        || primitives::words_find_phrase(tokens, &["during", "your", "turn"]).is_some()
    {
        return Some(ActivationTiming::DuringYourTurn);
    }
    if primitives::words_match_any_prefix(tokens, DURING_OPPONENTS_TURN_PREFIXES).is_some()
        || primitives::words_find_phrase(tokens, &["during", "an", "opponents", "turn"]).is_some()
        || primitives::words_find_phrase(tokens, &["during", "opponents", "turn"]).is_some()
    {
        return Some(ActivationTiming::DuringOpponentsTurn);
    }
    None
}

pub(crate) fn is_activate_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_RESTRICTION_PREFIXES).is_some()
}

pub(crate) fn is_spend_mana_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::words_match_any_prefix(tokens, SPEND_MANA_RESTRICTION_PREFIXES).is_some()
}

pub(crate) fn parse_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    parse_legacy_mana_usage_restriction_sentence_lexed(tokens)
        .or_else(|| parse_activate_ability_mana_usage_restriction_sentence_lexed(tokens))
        .or_else(|| parse_cant_be_spent_to_cast_sentence_lexed(tokens))
        .or_else(|| parse_filter_mana_usage_restriction_sentence_lexed(tokens))
}

fn parse_cant_be_spent_to_cast_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    let words = TokenWordView::new(tokens).to_word_refs();
    let start_idx = if MANA_CANT_BE_SPENT_TO_CAST_PREFIX_PATTERN.matches_words(&words) {
        7
    } else {
        return None;
    };

    let spec_words = &words[start_idx..];
    if spec_words.is_empty() {
        return None;
    }

    let filter = match spec_words {
        ["nonartifact", "spell"] | ["nonartifact", "spells"] => {
            ObjectFilter::default().with_type(crate::types::CardType::Artifact)
        }
        _ => return None,
    };

    Some(ManaUsageRestriction::CastSpellMatching {
        filter,
        restrict_to_matching_spell: true,
        grant_uncounterable: false,
        enters_with_counters: vec![],
        granted_abilities: vec![],
    })
}

fn parse_activate_ability_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    let words = TokenWordView::new(tokens).to_word_refs();
    if SPEND_MANA_ACTIVATE_ABILITY_PATTERN.matches_words(&words) {
        Some(ManaUsageRestriction::ActivateAbility)
    } else {
        None
    }
}

fn parse_legacy_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    let words = TokenWordView::new(tokens);
    if primitives::words_match_any_prefix(tokens, SPEND_MANA_CAST_PREFIXES).is_none() {
        return None;
    }

    let spell_idx = words.find_any_word(&["spell", "spells"])?;
    let spec_words = (6..spell_idx)
        .filter_map(|idx| words.get(idx))
        .collect::<Vec<_>>();
    if spec_words.is_empty() {
        return None;
    }

    let mut idx = 0usize;
    if ARTICLE_WORD_PATTERN.matches_word_at(&spec_words, 0) {
        idx += 1;
    }

    let card_type = match spec_words.get(idx).copied()? {
        "artifact" => crate::types::CardType::Artifact,
        "battle" => crate::types::CardType::Battle,
        "creature" => crate::types::CardType::Creature,
        "enchantment" => crate::types::CardType::Enchantment,
        "instant" => crate::types::CardType::Instant,
        "land" => crate::types::CardType::Land,
        "planeswalker" => crate::types::CardType::Planeswalker,
        "sorcery" => crate::types::CardType::Sorcery,
        _ => return None,
    };
    idx += 1;

    if idx != spec_words.len() {
        return None;
    }

    let mut tail = ((spell_idx + 1)..words.len())
        .filter_map(|word_idx| words.get(word_idx))
        .collect::<Vec<_>>();
    let subtype_requirement = if OF_THE_CHOSEN_TYPE_PREFIX_PATTERN.matches_words(&tail) {
        tail.drain(0..4);
        Some(crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource)
    } else {
        None
    };

    let grant_uncounterable = THAT_SPELL_CANT_BE_COUNTERED_TAIL_PATTERN.matches_words(&tail);
    if !grant_uncounterable && !tail.is_empty() {
        return None;
    }

    Some(ManaUsageRestriction::CastSpell {
        card_types: vec![card_type],
        subtype_requirement,
        restrict_to_matching_spell: true,
        grant_uncounterable,
        enters_with_counters: vec![],
        granted_abilities: vec![],
    })
}

fn parse_filter_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    let words = TokenWordView::new(tokens);
    if primitives::words_match_any_prefix(tokens, SPEND_MANA_CAST_PREFIXES).is_none() {
        return None;
    }

    let mut word_refs = words.to_word_refs();
    let cast_idx = CAST_WORD_PATTERN.find_word(&word_refs)?;
    if word_refs.len() >= 6 {
        let tail = &word_refs[word_refs.len() - 6..];
        if THAT_SPELL_CANT_BE_COUNTERED_TAIL_PATTERN.matches_words(tail) {
            word_refs.truncate(word_refs.len() - 6);
        }
    }

    let spec_words = word_refs
        .get(cast_idx + 1..)?
        .iter()
        .copied()
        .collect::<Vec<_>>();
    if spec_words.is_empty() {
        return None;
    }
    let special_filter = parse_special_mana_usage_spell_filter_words(&spec_words);
    if special_filter.is_none() && MANA_USAGE_UNSUPPORTED_MARKER_PATTERN.matches_words(&spec_words)
    {
        return None;
    }

    let is_plain_spell = spec_words
        .iter()
        .all(|word| PLAIN_SPELL_USAGE_WORD_PATTERN.matches_word(word));
    let filter = special_filter.or_else(|| {
        let spec_text = spec_words.join(" ");
        let spec_tokens = super::super::lexer::lex_line(&spec_text, 0).ok()?;
        let filter = parse_spell_filter_with_grammar_entrypoint(&spec_tokens);
        (filter != ObjectFilter::default() || is_plain_spell).then_some(filter)
    })?;

    let word_refs = words.to_word_refs();
    let grant_uncounterable = THAT_SPELL_CANT_BE_COUNTERED_TAIL_PATTERN
        .matches_words(&word_refs[word_refs.len().saturating_sub(6)..]);

    Some(ManaUsageRestriction::CastSpellMatching {
        filter,
        restrict_to_matching_spell: true,
        grant_uncounterable,
        enters_with_counters: vec![],
        granted_abilities: vec![],
    })
}

fn parse_special_mana_usage_spell_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let words = strip_leading_article_word_refs(words);
    if words.is_empty() {
        return None;
    }

    if MONOCOLORED_OF_CHOSEN_COLOR_USAGE_PATTERN.matches_words(words) {
        return Some(ObjectFilter::default().monocolored().of_chosen_color());
    }

    if YOUR_COMMANDER_USAGE_PATTERN.matches_words(words) {
        return Some(
            ObjectFilter::default()
                .commander()
                .owned_by(PlayerFilter::You),
        );
    }

    if SPELL_FROM_YOUR_GRAVEYARD_PATTERN.matches_words(words) {
        return Some(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
    }

    if SPELL_FROM_EXILE_PATTERN.matches_words(words) {
        return Some(ObjectFilter::default().in_zone(Zone::Exile));
    }

    if SPELL_WITH_DEVOID_PATTERN.matches_words(words) {
        return Some(ObjectFilter::default().with_static_ability(StaticAbilityId::MakeColorless));
    }

    if CREATURE_SPELL_WITH_NO_ABILITIES_PATTERN.matches_words(words) {
        let mut filter = ObjectFilter::default().with_type(crate::types::CardType::Creature);
        filter.no_abilities = true;
        return Some(filter);
    }

    if SPELL_YOU_DONT_OWN_PATTERN.matches_words(words) {
        return Some(ObjectFilter::default().owned_by(PlayerFilter::NotYou));
    }

    None
}

pub(crate) fn parse_mana_spend_bonus_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    let (prefix, _) = primitives::words_match_any_prefix(tokens, IF_MANA_SPENT_SPELL_PREFIXES)?;

    let words = TokenWordView::new(tokens);
    let spell_idx = words.find_any_word(&["spell", "spells"])?;

    let spec_words = (prefix.len()..spell_idx)
        .filter_map(|idx| words.get(idx))
        .collect::<Vec<_>>();
    if spec_words.is_empty() {
        return None;
    }

    let simple_card_type = parse_simple_mana_spend_bonus_card_type(&spec_words);

    let comma_idx = find_token_kind(tokens, TokenKind::Comma)?;
    let clause_tokens = trim_lexed_commas(&tokens[comma_idx + 1..]);
    if clause_tokens.is_empty() {
        return None;
    }

    let clause_word_view = TokenWordView::new(&clause_tokens);
    let clause_words = clause_word_view.to_word_refs();

    let grant_uncounterable = matches!(
        clause_words.as_slice(),
        ["that", "spell", "can't", "be", "countered"]
            | ["that", "spell", "cant", "be", "countered"]
    );
    let granted_abilities = if matches!(
        clause_words.as_slice(),
        ["it", "gains", "haste"]
            | ["that", "spell", "gains", "haste"]
            | ["that", "creature", "gains", "haste"]
            | ["it", "gains", "haste", "until", "end", "of", "turn"]
            | [
                "that", "spell", "gains", "haste", "until", "end", "of", "turn"
            ]
            | [
                "that", "creature", "gains", "haste", "until", "end", "of", "turn"
            ]
    ) {
        vec![StaticAbilityId::Haste]
    } else {
        Vec::new()
    };

    if grant_uncounterable || !granted_abilities.is_empty() {
        if let Some(card_type) = simple_card_type {
            return Some(ManaUsageRestriction::CastSpell {
                card_types: vec![card_type],
                subtype_requirement: None,
                restrict_to_matching_spell: false,
                grant_uncounterable,
                enters_with_counters: vec![],
                granted_abilities,
            });
        }
        let filter = parse_mana_spend_bonus_spell_filter(&spec_words)?;
        return Some(ManaUsageRestriction::CastSpellMatching {
            filter,
            restrict_to_matching_spell: false,
            grant_uncounterable,
            enters_with_counters: vec![],
            granted_abilities,
        });
    }

    if clause_words.len() < 6 || !THAT_WORD_PATTERN.matches_word_at(&clause_words, 0) {
        return None;
    }

    let card_type = simple_card_type?;
    if !MANA_SPEND_COUNTER_SUBJECT_NOUN_WORD_PATTERN.matches_word_at(&clause_words, 1)
        && parse_card_type(clause_words.get(1).copied()?).is_none()
    {
        return None;
    }

    let enters_idx = ENTER_OR_ENTERS_WORD_PATTERN.find_word(&clause_words)?;
    let with_token_idx = find_token_word(&clause_tokens, "with")?;
    let after_with = &clause_tokens[with_token_idx + 1..];
    if after_with.is_empty() {
        return None;
    }

    let (count, used) = if after_with
        .first()
        .is_some_and(|token| ARTICLE_WORD_PATTERN.matches_token(token))
        && after_with
            .get(1)
            .is_some_and(|token| ADDITIONAL_WORD_PATTERN.matches_token(token))
    {
        (1, 2)
    } else if after_with
        .first()
        .is_some_and(|token| ADDITIONAL_WORD_PATTERN.matches_token(token))
    {
        (1, 1)
    } else if let Some((parsed, number_used)) = parse_number(after_with) {
        let used = if after_with
            .get(number_used)
            .is_some_and(|token| ADDITIONAL_WORD_PATTERN.matches_token(token))
        {
            number_used + 1
        } else {
            number_used
        };
        (parsed, used)
    } else {
        return None;
    };

    let counter_type = parse_counter_type_from_tokens(&after_with[used..])?;
    let counter_idx = find_token_any_word(after_with, &["counter", "counters"])?;
    let tail_tokens = trim_lexed_commas(&after_with[counter_idx + 1..]);
    let mut tail: &[OwnedLexToken] = &tail_tokens;
    if token_slice_first_is(tail, "on") {
        tail = &tail[1..];
    }
    if token_slice_first_is(tail, "it") {
        tail = &tail[1..];
    } else if token_slice_first_is(tail, "that") {
        tail = &tail[1..];
        if tail
            .first()
            .is_some_and(|token| token.as_word().and_then(parse_card_type).is_some())
            || tail.first().is_some_and(|token| {
                matches!(
                    token.as_word(),
                    Some("creature" | "spell" | "permanent" | "card")
                )
            })
        {
            tail = &tail[1..];
        }
    }
    if tail.iter().any(|token| token.as_word().is_some()) {
        return None;
    }

    if enters_idx <= 1 {
        return None;
    }

    Some(ManaUsageRestriction::CastSpell {
        card_types: vec![card_type],
        subtype_requirement: None,
        restrict_to_matching_spell: false,
        grant_uncounterable: false,
        enters_with_counters: vec![(counter_type, count)],
        granted_abilities: vec![],
    })
}

pub(crate) fn is_mana_spend_bonus_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::words_match_any_prefix(tokens, IF_MANA_SPENT_SPELL_PREFIXES).is_some()
}

fn parse_simple_mana_spend_bonus_card_type(spec_words: &[&str]) -> Option<crate::types::CardType> {
    let mut idx = 0usize;
    if ARTICLE_WORD_PATTERN.matches_word_at(spec_words, 0) {
        idx += 1;
    }
    let card_type = parse_card_type(spec_words.get(idx).copied()?)?;
    idx += 1;
    (idx == spec_words.len()).then_some(card_type)
}

fn parse_mana_spend_bonus_spell_filter(spec_words: &[&str]) -> Option<ObjectFilter> {
    let spec_text = spec_words.join(" ");
    let spec_tokens = super::super::lexer::lex_line(&spec_text, 0).ok()?;
    let filter = parse_spell_filter_with_grammar_entrypoint(&spec_tokens);
    (filter != ObjectFilter::default()).then_some(filter)
}

pub(crate) fn is_any_player_may_activate_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::words_match_prefix(
        tokens,
        &["any", "player", "may", "activate", "this", "ability"],
    )
    .is_some()
}

pub(crate) fn is_trigger_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::words_match_any_prefix(tokens, THIS_ABILITY_TRIGGERS_ONLY_PREFIXES).is_some()
}

pub(crate) fn parse_triggered_times_each_turn_from_words(words: &[&str]) -> Option<u32> {
    let (count_idx, prefix_len) = if TRIGGER_ONLY_PREFIX_PATTERN.matches_words(words) {
        (4usize, 4usize)
    } else if DO_THIS_ONLY_PREFIX_PATTERN.matches_words(words) {
        (3usize, 3usize)
    } else {
        return None;
    };

    if words.len() < prefix_len + 3 {
        return None;
    }

    let mut index = count_idx;
    let (count, used) = parse_number_words(&words[index..])?;
    index += used;

    if TIME_OR_TIMES_WORD_PATTERN.matches_word_at(words, index) {
        index += 1;
    }

    if EACH_TURN_TAIL_PATTERN.matches_words(&words[index..]) {
        Some(count)
    } else {
        None
    }
}

pub(crate) fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    let words = TokenWordView::new(tokens);
    parse_triggered_times_each_turn_from_words(&words.to_word_refs())
}

pub(crate) fn parse_activation_condition_lexed(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    let words = TokenWordView::new(tokens);
    if words.len() < 5 {
        return None;
    }

    let word_refs = words.to_word_refs();

    if word_refs.first().copied() == Some("activate") {
        if let Some((count, used)) = parse_less_than_or_equal_quantity_prefix(
            tokens.get(1..).unwrap_or_default(),
            false,
            false,
            "activation frequency condition",
        )
        .ok()
        .flatten()
        {
            let mut index = 1 + used;
            if words.at_is_any(index, &["time", "times"]) {
                index += 1;
            }
            if words.starts_with_at(index, &["each", "turn"]) {
                return Some(ConditionExpr::MaxActivationsPerTurn(count));
            }
        }
    }

    let after_activate_only = (2..words.len())
        .filter_map(|idx| words.get(idx))
        .collect::<Vec<_>>();
    if let Some(count) = parse_activation_count_per_turn(&after_activate_only) {
        return Some(ConditionExpr::MaxActivationsPerTurn(count));
    }
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_INSTANT_PREFIXES).is_some() {
        return Some(ConditionExpr::ActivationTiming(ActivationTiming::AnyTime));
    }
    if ACTIVATE_ONLY_IF_THERE_PREFIX_PATTERN.matches_words(&word_refs) {
        let descriptor_start = 5usize;
        let in_idx = words.find_any_word_from(&["in"], descriptor_start)?;
        let zone_tail = (in_idx..words.len())
            .filter_map(|idx| words.get(idx))
            .collect::<Vec<_>>();
        let points_to_your_graveyard = YOUR_GRAVEYARD_ZONE_TAIL_PATTERN.matches_words(&zone_tail);
        if !points_to_your_graveyard {
            return None;
        }

        let descriptor_words = (descriptor_start..in_idx)
            .filter_map(|idx| words.get(idx))
            .collect::<Vec<_>>();
        if descriptor_words.is_empty() {
            return None;
        }

        let mut card_types = Vec::new();
        let mut subtypes = Vec::new();
        for word in descriptor_words {
            if let Some(card_type) = parse_card_type(word)
                && !slice_contains(&card_types, &card_type)
            {
                card_types.push(card_type);
            }
            if let Some(subtype) = parse_subtype_flexible(word)
                && !slice_contains(&subtypes, &subtype)
            {
                subtypes.push(subtype);
            }
        }

        if card_types.is_empty() && subtypes.is_empty() {
            return None;
        }

        return Some(ConditionExpr::CardInYourGraveyard {
            card_types,
            subtypes,
        });
    }
    if let Some(status_tokens) = tokens.get(3..)
        && let Some(condition) =
            crate::runtime_backend::grammar::conditions::parse_player_status_condition(
                status_tokens,
            )
        && condition.status
            == crate::runtime_backend::grammar::conditions::PlayerStatusAst::MaxSpeed
    {
        return Some(condition.condition_expr());
    }
    if ACTIVATE_ONLY_IF_CREATURES_TOTAL_POWER_PREFIX_PATTERN.matches_words(&word_refs) {
        let comparison_words = (9..words.len())
            .filter_map(|idx| words.get(idx))
            .collect::<Vec<_>>();
        let (comparison, used) =
            parse_filter_comparison_tokens("power", &comparison_words, &word_refs).ok()??;
        let crate::filter::Comparison::GreaterThanOrEqual(threshold) = comparison else {
            return None;
        };
        if used != comparison_words.len() {
            return None;
        }
        return Some(ConditionExpr::ControlCreaturesTotalPowerAtLeast(
            u32::try_from(threshold).ok()?,
        ));
    }
    if ACTIVATE_ONLY_IF_RED_SOURCES_DEALT_PREFIX_PATTERN.matches_words(&word_refs) {
        let (threshold, used) = parse_number_words(word_refs.get(8..).unwrap_or_default())?;
        let tail = word_refs.get(8 + used..).unwrap_or_default();
        if NONCOMBAT_DAMAGE_THIS_TURN_TAIL_PATTERN.matches_words(&tail) {
            return Some(ConditionExpr::ValueComparison {
                left: crate::effect::Value::NoncombatDamageDealtBySourcesControlledThisTurn {
                    player: PlayerFilter::You,
                    colors: Some(ColorSet::from_color(Color::Red)),
                },
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(threshold as i32),
            });
        }
        return None;
    }
    let control_condition_tokens =
        if ACTIVATE_ONLY_IF_YOU_CONTROL_PREFIX_PATTERN.matches_words(&word_refs) {
            tokens.get(3..)?
        } else {
            return None;
        };
    let control_tail = word_refs.get(5..)?.to_vec();
    if CREATURE_WITH_POWER_PREFIX_PATTERN.matches_words(&control_tail) {
        let power_idx = POWER_WORD_PATTERN.find_word(&control_tail)?;
        let comparison_words = &control_tail[power_idx + 1..];
        let (comparison, used) =
            parse_filter_comparison_tokens("power", comparison_words, &control_tail).ok()??;
        if used == comparison_words.len() {
            return Some(ConditionExpr::YouControl(
                ObjectFilter::creature().with_power(comparison),
            ));
        }
        return None;
    }
    if let Some(control_condition) =
        parse_control_condition(control_condition_tokens, ControlConditionOptions::default())
    {
        let count = control_condition.at_least_count()?;
        return Some(ConditionExpr::PlayerHasAtLeast {
            player: control_condition.player_filter?,
            filter: control_condition.filter,
            count,
        });
    }

    let mut subtypes = Vec::new();
    for idx in 0..words.len() {
        let Some(word) = words.get(idx) else {
            continue;
        };
        if let Some(subtype) = parse_subtype_flexible(word)
            && !slice_contains(&subtypes, &subtype)
        {
            subtypes.push(subtype);
        }
    }

    if subtypes.is_empty() {
        return None;
    }

    let mut combined: Option<ConditionExpr> = None;
    for subtype in subtypes {
        let next = ConditionExpr::YouControl(
            ObjectFilter::default()
                .with_type(crate::types::CardType::Land)
                .with_subtype(subtype),
        );
        combined = Some(match combined {
            Some(existing) => ConditionExpr::Or(Box::new(existing), Box::new(next)),
            None => next,
        });
    }

    combined
}

pub(crate) fn parse_activation_count_per_turn(words: &[&str]) -> Option<u32> {
    let (count, used) = parse_number_words(words)?;
    let mut index = used;
    if TIME_OR_TIMES_WORD_PATTERN.matches_word_at(words, index) {
        index += 1;
    }
    if EACH_TURN_TAIL_PATTERN.matches_words(&words[index..]) {
        Some(count)
    } else {
        None
    }
}

pub(crate) fn is_standard_gift_keyword_tokens_lexed(tokens: &[OwnedLexToken]) -> bool {
    let head_tokens = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LParen)
        .map(|idx| &tokens[..idx])
        .unwrap_or(tokens);
    if !token_slice_starts_with(head_tokens, &["gift"]) {
        return false;
    }

    [
        &["gift", "a", "card"][..],
        &["gift", "a", "treasure"],
        &["gift", "a", "food"],
        &["gift", "a", "tapped", "fish"],
        &["gift", "an", "extra", "turn"],
        &["gift", "an", "octopus"],
    ]
    .iter()
    .any(|phrase| primitives::words_match_prefix(head_tokens, phrase).is_some())
}

pub(crate) fn additional_cost_tail_tokens_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let comma_idx = find_token_kind(tokens, TokenKind::Comma);
    let effect_start = if let Some(idx) = comma_idx {
        idx + 1
    } else if let Some(idx) = find_token_word(tokens, "spell") {
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

pub(crate) fn is_draw_replacement_double_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    DRAW_REPLACEMENT_DOUBLE_PATTERN.matches_words(&words)
}

pub(crate) fn is_draw_replacement_skip_empty_library_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    DRAW_REPLACEMENT_SKIP_EMPTY_LIBRARY_PATTERN.matches_words(&words)
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

pub(crate) fn is_opponent_effect_discard_this_to_battlefield_replacement_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    word_slice_eq(
        &words,
        &[
            "if",
            "a",
            "spell",
            "or",
            "ability",
            "an",
            "opponent",
            "controls",
            "causes",
            "you",
            "to",
            "discard",
            "this",
            "card",
            "put",
            "it",
            "onto",
            "the",
            "battlefield",
            "instead",
            "of",
            "putting",
            "it",
            "into",
            "your",
            "graveyard",
        ],
    )
}

pub(crate) fn is_shuffle_into_library_from_graveyard_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    contains_token_word_sequence(tokens, &["would", "be", "put"])
        && contains_token_word(tokens, "graveyard")
        && contains_token_word(tokens, "anywhere")
        && contains_token_word(tokens, "shuffle")
        && contains_token_word(tokens, "library")
        && contains_token_word(tokens, "instead")
}

pub(crate) fn is_discard_or_redirect_replacement_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    contains_token_any_word(tokens, &["enter", "enters"])
        && contains_token_word(tokens, "battlefield")
        && contains_token_word(tokens, "discard")
        && contains_token_word(tokens, "land")
        && contains_token_word(tokens, "instead")
        && contains_token_word(tokens, "graveyard")
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

pub(crate) fn parse_can_block_subtype_as_though_reach_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::types::Subtype> {
    let words = TokenWordView::new(tokens).to_word_refs();
    let subtype_word =
        CAN_BLOCK_SUBTYPE_AS_THOUGH_REACH_SHAPES
            .iter()
            .find_map(|(prefix, suffix)| {
                if words.len() == prefix.len() + 1 + suffix.len()
                    && words.starts_with(prefix)
                    && words[prefix.len() + 1..].starts_with(suffix)
                {
                    Some(words[prefix.len()])
                } else {
                    None
                }
            })?;

    parse_subtype_flexible(subtype_word).filter(|subtype| subtype.is_creature_type())
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

pub(crate) fn is_prevent_all_combat_damage_to_matching_permanents_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = TokenWordView::new(tokens).to_word_refs();
    let prefix = [
        "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to",
    ];
    word_slice_starts_with(&words, &prefix)
        && words.len() > prefix.len()
        && !word_slice_eq(&words[prefix.len()..], &["this", "creature"])
        && !word_slice_eq(&words[prefix.len()..], &["this", "permanent"])
        && !word_slice_eq(&words[prefix.len()..], &["it"])
        && !word_slice_contains_any_word(&words, &["turn"])
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

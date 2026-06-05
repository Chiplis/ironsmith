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
use super::super::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom};
use super::super::lexer::{
    LexStream, LexToken, LexedClause, OwnedLexToken, TokenKind, contains_token_any_word,
    contains_token_word, contains_token_word_sequence, find_token_any_word, find_token_kind,
    find_token_word, token_slice_first_is_any, token_slice_starts_with, trim_lexed_commas,
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
    parse_less_than_or_equal_quantity_prefix, parse_number,
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

fn clause_matches_phrase(clause: LexedClause<'_>, phrase: &[&str]) -> bool {
    LexPattern::new(&[LexPattern::phrase(phrase)]).matches_clause(clause)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LegacyManaUsageCastRestriction<'a> {
    card_type: crate::types::CardType,
    tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FilterManaUsageCastRestriction<'a> {
    spec_tokens: &'a [OwnedLexToken],
    grant_uncounterable: bool,
}

struct CastOrActivateManaUsageRestriction<'a> {
    spell_spec_tokens: &'a [OwnedLexToken],
    ability_source_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManaSpendBonusSentence<'a> {
    spec_tokens: &'a [OwnedLexToken],
    bonus_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManaSpendCounterBonus<'a> {
    counter_tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActivateOnlyIfCardInGraveyardCondition<'a> {
    descriptor_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActivateOnlyIfCreaturesTotalPowerCondition<'a> {
    comparison_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActivateOnlyIfSourcesDealtDamageCondition<'a> {
    source_tokens: &'a [OwnedLexToken],
    threshold_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActivateOnlyIfYouControlCreaturePowerCondition<'a> {
    object_tokens: &'a [OwnedLexToken],
    comparison_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActivateOnlyCountPerTurnCondition<'a> {
    count_tokens: &'a [OwnedLexToken],
}

const ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["a"], &["an"]]);
const MANA_USAGE_UNCOUNTERABLE_TAILS: &[&[&str]] = &[
    &["and", "that", "spell", "can't", "be", "countered"],
    &["and", "that", "spell", "cant", "be", "countered"],
];
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
const MANA_SPEND_COUNTER_SUBJECT_NOUN_WORDS: &[&str] = &["creature", "spell", "permanent", "card"];
const ADDITIONAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["additional"]);
const DRAW_REPLACEMENT_DOUBLE_PHRASE: &[&str] = &[
    "if", "you", "would", "draw", "a", "card", "draw", "two", "cards", "instead",
];
const DRAW_REPLACEMENT_SKIP_EMPTY_LIBRARY_PHRASE: &[&str] = &[
    "if", "you", "would", "draw", "a", "card", "while", "your", "library", "has", "no", "cards",
    "in", "it", "skip", "that", "draw", "instead",
];
const OPPONENT_DISCARD_THIS_TO_BATTLEFIELD_REPLACEMENT_PHRASE: &[&str] = &[
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
];
const WHENEVER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["whenever"]);
const TIMES_EACH_TURN_TAILS: &[&[&str]] = &[
    &["time", "each", "turn"],
    &["times", "each", "turn"],
    &["each", "turn"],
];
const BLACK_MANA_GROUP_TEXT: &str = "{b}";
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
    const LAND_REVEAL_ENTERS_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["as", "this", "land", "enters"]),
        LexPattern::modifier(
            "intro",
            LexCaptureKind::UntilPhrase(&["you", "may", "reveal"]),
        ),
        LexPattern::phrase(&["you", "may", "reveal"]),
        LexPattern::object(
            "revealed_object",
            LexCaptureKind::UntilPhrase(&["from", "your", "hand"]),
        ),
        LexPattern::phrase(&["from", "your", "hand"]),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let Some(matched) = LAND_REVEAL_ENTERS_PATTERN.match_clause(clause) else {
        return false;
    };
    matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .is_some_and(|object| !object.word_refs().is_empty())
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
    const OPENING_HAND_BEGIN_GAME_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["if", "this", "card", "is", "in", "your", "opening", "hand"]),
        LexPattern::modifier(
            "intro",
            LexCaptureKind::UntilPhrase(&["you", "may", "begin", "the", "game", "with"]),
        ),
        LexPattern::phrase(&["you", "may", "begin", "the", "game", "with"]),
        LexPattern::object(
            "starting_object",
            LexCaptureKind::UntilPhrase(&["on", "the", "battlefield"]),
        ),
        LexPattern::phrase(&["on", "the", "battlefield"]),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let Some(matched) = OPENING_HAND_BEGIN_GAME_PATTERN.match_clause(clause) else {
        return false;
    };
    matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .is_some_and(|object| !object.word_refs().is_empty())
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
const ONCE_EACH_TURN_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["once", "each", "turn"]]);
const DURING_COMBAT_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["during", "combat"]]);
const DURING_YOUR_TURN_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["during", "your", "turn"]]);
const DURING_OPPONENTS_TURN_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["during", "an", "opponents", "turn"],
            &["during", "opponents", "turn"],
        ]]
);

pub(crate) fn parse_activate_only_timing_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ActivationTiming> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_SORCERY_PREFIXES).is_some() {
        return Some(ActivationTiming::SorcerySpeed);
    }
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_ONCE_EACH_TURN_PREFIXES).is_some()
        || ONCE_EACH_TURN_MARKER_PATTERN.matches_words(&words)
    {
        return Some(ActivationTiming::OncePerTurn);
    }
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_DURING_COMBAT_PREFIXES).is_some()
        || DURING_COMBAT_MARKER_PATTERN.matches_words(&words)
    {
        return Some(ActivationTiming::DuringCombat);
    }
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_DURING_YOUR_TURN_PREFIXES).is_some()
        || DURING_YOUR_TURN_MARKER_PATTERN.matches_words(&words)
    {
        return Some(ActivationTiming::DuringYourTurn);
    }
    if primitives::words_match_any_prefix(tokens, DURING_OPPONENTS_TURN_PREFIXES).is_some()
        || DURING_OPPONENTS_TURN_MARKER_PATTERN.matches_words(&words)
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
    parse_cast_or_activate_mana_usage_restriction_sentence_lexed(tokens)
        .or_else(|| parse_legacy_mana_usage_restriction_sentence_lexed(tokens))
        .or_else(|| parse_activate_ability_mana_usage_restriction_sentence_lexed(tokens))
        .or_else(|| parse_cant_be_spent_to_cast_sentence_lexed(tokens))
        .or_else(|| parse_filter_mana_usage_restriction_sentence_lexed(tokens))
}

fn parse_cast_or_activate_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    let parsed = parse_cast_or_activate_mana_usage_restriction_tokens(tokens)?;
    let spell_tokens = trim_lexed_commas(parsed.spell_spec_tokens);
    let source_tokens = trim_mana_usage_ability_source_tokens(trim_lexed_commas(
        parsed.ability_source_tokens,
    ));
    if spell_tokens.is_empty() || source_tokens.is_empty() {
        return None;
    }

    let spell_filter = parse_spell_filter_with_grammar_entrypoint(spell_tokens);
    let source_filter = parse_spell_filter_with_grammar_entrypoint(source_tokens);
    if spell_filter == ObjectFilter::default() || spell_filter != source_filter {
        return None;
    }

    Some(ManaUsageRestriction::CastSpellOrActivateAbilityMatching {
        filter: spell_filter,
    })
}

fn parse_cast_or_activate_mana_usage_restriction_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<CastOrActivateManaUsageRestriction<'a>> {
    const CAST_OR_ACTIVATE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(SPEND_MANA_CAST_PREFIXES),
        LexPattern::object(
            "spell_spec",
            LexCaptureKind::UntilPhrase(&["or", "activate", "abilities", "of"]),
        ),
        LexPattern::phrase(&["or", "activate", "abilities", "of"]),
        LexPattern::subject("ability_source", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = CAST_OR_ACTIVATE_PATTERN.match_clause(clause)?;
    let spell_spec = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let ability_source = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    Some(CastOrActivateManaUsageRestriction {
        spell_spec_tokens: spell_spec.tokens(),
        ability_source_tokens: ability_source.tokens(),
    })
}

fn trim_mana_usage_ability_source_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let Some(last) = tokens.last() else {
        return tokens;
    };
    if last.is_word("source") || last.is_word("sources") {
        &tokens[..tokens.len() - 1]
    } else {
        tokens
    }
}

fn parse_cant_be_spent_to_cast_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    let spec_tokens = parse_cant_be_spent_to_cast_spell_spec_tokens(tokens)?;
    let filter = parse_cant_be_spent_to_cast_spell_filter(spec_tokens)?;

    Some(ManaUsageRestriction::CastSpellMatching {
        filter,
        restrict_to_matching_spell: true,
        grant_uncounterable: false,
        enters_with_counters: vec![],
        granted_abilities: vec![],
    })
}

fn parse_cant_be_spent_to_cast_spell_spec_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    const CANT_BE_SPENT_TO_CAST_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(&[
            &["this", "mana", "cant", "be", "spent", "to", "cast"],
            &["this", "mana", "can't", "be", "spent", "to", "cast"],
            &["that", "mana", "cant", "be", "spent", "to", "cast"],
            &["that", "mana", "can't", "be", "spent", "to", "cast"],
        ]),
        LexPattern::object("spell_spec", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = CANT_BE_SPENT_TO_CAST_PATTERN.match_clause(clause)?;
    let spec_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    Some(spec_clause.tokens())
}

fn parse_cant_be_spent_to_cast_spell_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    const NONARTIFACT_SPELL_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("nonartifact"),
        LexPattern::any_word(&["spell", "spells"]),
    ]);

    if NONARTIFACT_SPELL_PATTERN.matches_clause(LexedClause::new(tokens)) {
        Some(ObjectFilter::default().with_type(crate::types::CardType::Artifact))
    } else {
        None
    }
}

fn parse_activate_ability_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    const SPEND_MANA_ACTIVATE_ABILITY_PHRASES: &[&[&str]] = &[
        &[
            "spend",
            "this",
            "mana",
            "only",
            "to",
            "activate",
            "abilities",
        ],
        &[
            "spend", "this", "mana", "only", "to", "activate", "an", "ability",
        ],
    ];
    const SPEND_MANA_ACTIVATE_ABILITY_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::any_phrase(SPEND_MANA_ACTIVATE_ABILITY_PHRASES)]);

    if SPEND_MANA_ACTIVATE_ABILITY_PATTERN.matches_clause(LexedClause::new(tokens)) {
        Some(ManaUsageRestriction::ActivateAbility)
    } else {
        None
    }
}

fn parse_legacy_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    let parsed = parse_legacy_mana_usage_cast_restriction_tokens(tokens)?;
    let (tail_tokens, subtype_requirement) =
        strip_legacy_mana_usage_chosen_type_tail(parsed.tail_tokens);
    let tail_tokens = trim_lexed_commas(tail_tokens);
    let grant_uncounterable = legacy_mana_usage_uncounterable_tail_matches(tail_tokens);
    if !grant_uncounterable && !tail_tokens.is_empty() {
        return None;
    }

    Some(ManaUsageRestriction::CastSpell {
        card_types: vec![parsed.card_type],
        subtype_requirement,
        restrict_to_matching_spell: true,
        grant_uncounterable,
        enters_with_counters: vec![],
        granted_abilities: vec![],
    })
}

fn parse_legacy_mana_usage_cast_restriction_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LegacyManaUsageCastRestriction<'a>> {
    const OPTIONAL_ARTICLE: &[LexPatternAtom<'static>] = &[LexPattern::any_word(&["a", "an"])];
    const LEGACY_MANA_USAGE_CAST_RESTRICTION_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(SPEND_MANA_CAST_PREFIXES),
        LexPattern::optional(OPTIONAL_ARTICLE),
        LexPattern::object("card_type", LexCaptureKind::WordCount(1)),
        LexPattern::any_word(&["spell", "spells"]),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = LEGACY_MANA_USAGE_CAST_RESTRICTION_PATTERN.match_clause(clause)?;
    let card_type_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let card_type = parse_card_type(card_type_clause.first_word()?)?;
    let tail_clause = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;
    Some(LegacyManaUsageCastRestriction {
        card_type,
        tail_tokens: tail_clause.tokens(),
    })
}

fn strip_legacy_mana_usage_chosen_type_tail(
    tail_tokens: &[OwnedLexToken],
) -> (
    &[OwnedLexToken],
    Option<crate::ability::ManaUsageSubtypeRequirement>,
) {
    const OF_THE_CHOSEN_TYPE_PREFIX: &[&str] = &["of", "the", "chosen", "type"];
    let tail_clause = LexedClause::new(trim_lexed_commas(tail_tokens));
    if let Some(rest) = tail_clause.strip_prefix_clause(OF_THE_CHOSEN_TYPE_PREFIX) {
        (
            rest.tokens(),
            Some(crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource),
        )
    } else {
        (tail_tokens, None)
    }
}

fn legacy_mana_usage_uncounterable_tail_matches(tail_tokens: &[OwnedLexToken]) -> bool {
    const UNCOUNTERABLE_TAIL_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::any_phrase(MANA_USAGE_UNCOUNTERABLE_TAILS)]);

    UNCOUNTERABLE_TAIL_PATTERN.matches_clause(LexedClause::new(tail_tokens))
}

fn parse_filter_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    let parsed = parse_filter_mana_usage_cast_restriction_tokens(tokens)?;
    let spec_tokens = trim_lexed_commas(parsed.spec_tokens);
    let spec_words = LexedClause::new(spec_tokens).word_refs();
    if spec_words.is_empty() {
        return None;
    }
    let special_filter = parse_special_mana_usage_spell_filter_tokens(spec_tokens);
    if special_filter.is_none() && MANA_USAGE_UNSUPPORTED_MARKER_PATTERN.matches_words(&spec_words)
    {
        return None;
    }

    let is_plain_spell = spec_words
        .iter()
        .all(|word| PLAIN_SPELL_USAGE_WORD_PATTERN.matches_word(word));
    let filter = special_filter.or_else(|| {
        let filter = parse_spell_filter_with_grammar_entrypoint(spec_tokens);
        (filter != ObjectFilter::default() || is_plain_spell).then_some(filter)
    })?;

    Some(ManaUsageRestriction::CastSpellMatching {
        filter,
        restrict_to_matching_spell: true,
        grant_uncounterable: parsed.grant_uncounterable,
        enters_with_counters: vec![],
        granted_abilities: vec![],
    })
}

fn parse_filter_mana_usage_cast_restriction_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<FilterManaUsageCastRestriction<'a>> {
    const WITH_UNCOUNTERABLE_TAIL_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(SPEND_MANA_CAST_PREFIXES),
        LexPattern::object(
            "spell_spec",
            LexCaptureKind::UntilLastAnyPhrase(MANA_USAGE_UNCOUNTERABLE_TAILS),
        ),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ]);
    const WITHOUT_UNCOUNTERABLE_TAIL_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(SPEND_MANA_CAST_PREFIXES),
        LexPattern::object("spell_spec", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    if let Some(matched) = WITH_UNCOUNTERABLE_TAIL_PATTERN.match_clause(clause) {
        let spec_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        let tail_clause = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;
        if legacy_mana_usage_uncounterable_tail_matches(tail_clause.tokens()) {
            return Some(FilterManaUsageCastRestriction {
                spec_tokens: spec_clause.tokens(),
                grant_uncounterable: true,
            });
        }
    }

    let matched = WITHOUT_UNCOUNTERABLE_TAIL_PATTERN.match_clause(clause)?;
    let spec_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    Some(FilterManaUsageCastRestriction {
        spec_tokens: spec_clause.tokens(),
        grant_uncounterable: false,
    })
}

fn parse_special_mana_usage_spell_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let clause = strip_mana_usage_spell_filter_article_tokens(tokens);
    let clause = LexedClause::new(clause);

    const MONOCOLORED_OF_CHOSEN_COLOR_USAGE_PHRASES: &[&[&str]] = &[
        &["monocolored", "spell", "of", "that", "color"],
        &["monocolored", "spells", "of", "that", "color"],
        &["monocolored", "spell", "of", "the", "chosen", "color"],
        &["monocolored", "spells", "of", "the", "chosen", "color"],
    ];
    if LexPattern::new(&[LexPattern::any_phrase(
        MONOCOLORED_OF_CHOSEN_COLOR_USAGE_PHRASES,
    )])
    .matches_clause(clause)
    {
        return Some(ObjectFilter::default().monocolored().of_chosen_color());
    }

    const YOUR_COMMANDER_USAGE_PHRASES: &[&[&str]] = &[
        &["your", "commander"],
        &["your", "commander", "spell"],
        &["your", "commander", "spells"],
    ];
    if LexPattern::new(&[LexPattern::any_phrase(YOUR_COMMANDER_USAGE_PHRASES)])
        .matches_clause(clause)
    {
        return Some(
            ObjectFilter::default()
                .commander()
                .owned_by(PlayerFilter::You),
        );
    }

    const SPELL_FROM_YOUR_GRAVEYARD_PHRASES: &[&[&str]] = &[
        &["spell", "from", "your", "graveyard"],
        &["spells", "from", "your", "graveyard"],
    ];
    if LexPattern::new(&[LexPattern::any_phrase(SPELL_FROM_YOUR_GRAVEYARD_PHRASES)])
        .matches_clause(clause)
    {
        return Some(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
    }

    const SPELL_FROM_EXILE_PHRASES: &[&[&str]] =
        &[&["spell", "from", "exile"], &["spells", "from", "exile"]];
    if LexPattern::new(&[LexPattern::any_phrase(SPELL_FROM_EXILE_PHRASES)]).matches_clause(clause) {
        return Some(ObjectFilter::default().in_zone(Zone::Exile));
    }

    const SPELL_WITH_DEVOID_PHRASES: &[&[&str]] =
        &[&["spell", "with", "devoid"], &["spells", "with", "devoid"]];
    if LexPattern::new(&[LexPattern::any_phrase(SPELL_WITH_DEVOID_PHRASES)]).matches_clause(clause)
    {
        return Some(ObjectFilter::default().with_static_ability(StaticAbilityId::MakeColorless));
    }

    const CREATURE_SPELL_WITH_NO_ABILITIES_PHRASES: &[&[&str]] = &[
        &["creature", "spell", "with", "no", "abilities"],
        &["creature", "spells", "with", "no", "abilities"],
    ];
    if LexPattern::new(&[LexPattern::any_phrase(
        CREATURE_SPELL_WITH_NO_ABILITIES_PHRASES,
    )])
    .matches_clause(clause)
    {
        let mut filter = ObjectFilter::default().with_type(crate::types::CardType::Creature);
        filter.no_abilities = true;
        return Some(filter);
    }

    const SPELL_YOU_DONT_OWN_PHRASES: &[&[&str]] = &[
        &["spell", "you", "don't", "own"],
        &["spell", "you", "dont", "own"],
        &["spells", "you", "don't", "own"],
        &["spells", "you", "dont", "own"],
    ];
    if LexPattern::new(&[LexPattern::any_phrase(SPELL_YOU_DONT_OWN_PHRASES)]).matches_clause(clause)
    {
        return Some(ObjectFilter::default().owned_by(PlayerFilter::NotYou));
    }

    None
}

fn strip_mana_usage_spell_filter_article_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    const ARTICLE_PREFIX: &[LexPatternAtom<'static>] = &[LexPattern::any_word(&["a", "an"])];
    const ARTICLE_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(ARTICLE_PREFIX);

    ARTICLE_PREFIX_PATTERN
        .match_prefix(LexedClause::new(tokens))
        .and_then(|matched| LexedClause::new(tokens).after_words(matched.word_range.end))
        .map(LexedClause::tokens)
        .unwrap_or(tokens)
}

pub(crate) fn parse_mana_spend_bonus_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    let parsed = parse_mana_spend_bonus_sentence_tokens(tokens)?;
    let spec_tokens = trim_lexed_commas(parsed.spec_tokens);
    let spec_words = LexedClause::new(spec_tokens).word_refs();
    if spec_words.is_empty() {
        return None;
    }

    let simple_card_type = parse_simple_mana_spend_bonus_card_type(&spec_words);

    let clause_tokens = trim_lexed_commas(parsed.bonus_tokens);
    if clause_tokens.is_empty() {
        return None;
    }

    let clause = LexedClause::new(clause_tokens);
    let grant_uncounterable = mana_spend_bonus_grants_uncounterable(clause);
    let granted_abilities = mana_spend_bonus_granted_abilities(clause);

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
        let filter = parse_mana_spend_bonus_spell_filter(spec_tokens)?;
        return Some(ManaUsageRestriction::CastSpellMatching {
            filter,
            restrict_to_matching_spell: false,
            grant_uncounterable,
            enters_with_counters: vec![],
            granted_abilities,
        });
    }

    let card_type = simple_card_type?;
    let counter_bonus = parse_mana_spend_counter_bonus(clause)?;
    let (counter_type, count) = parse_mana_spend_counter_bonus_tail(counter_bonus)?;

    Some(ManaUsageRestriction::CastSpell {
        card_types: vec![card_type],
        subtype_requirement: None,
        restrict_to_matching_spell: false,
        grant_uncounterable: false,
        enters_with_counters: vec![(counter_type, count)],
        granted_abilities: vec![],
    })
}

fn parse_mana_spend_counter_bonus(clause: LexedClause<'_>) -> Option<ManaSpendCounterBonus<'_>> {
    const ENTER_PHRASES: &[&[&str]] = &[&["enter"], &["enters"]];
    const MANA_SPEND_COUNTER_BONUS_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(ENTER_PHRASES)),
        LexPattern::action("enter", LexCaptureKind::OneOf(&["enter", "enters"])),
        LexPattern::word("with"),
        LexPattern::tail("counter_tail", LexCaptureKind::OneOrMoreWords),
    ]);

    let matched = MANA_SPEND_COUNTER_BONUS_PATTERN.match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !mana_spend_counter_bonus_subject_matches(subject_clause) {
        return None;
    }
    let counter_tail = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;
    Some(ManaSpendCounterBonus {
        counter_tail_tokens: counter_tail.tokens(),
    })
}

fn mana_spend_counter_bonus_subject_matches(clause: LexedClause<'_>) -> bool {
    const MANA_SPEND_COUNTER_SUBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("that"),
        LexPattern::subject("noun", LexCaptureKind::WordCount(1)),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ]);

    let Some(matched) = MANA_SPEND_COUNTER_SUBJECT_PATTERN.match_clause(clause) else {
        return false;
    };
    let Some(noun_clause) = matched.capture_clause_by_role(LexCaptureRole::Subject, clause) else {
        return false;
    };
    let Some(noun) = noun_clause.word_refs().first().copied() else {
        return false;
    };
    MANA_SPEND_COUNTER_SUBJECT_NOUN_WORDS.contains(&noun) || parse_card_type(noun).is_some()
}

fn parse_mana_spend_counter_bonus_tail(
    bonus: ManaSpendCounterBonus<'_>,
) -> Option<(CounterType, u32)> {
    let after_with = bonus.counter_tail_tokens;
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
    if !mana_spend_counter_bonus_target_tail_matches(tail_tokens) {
        return None;
    }
    Some((counter_type, count))
}

fn mana_spend_counter_bonus_target_tail_matches(tail_tokens: &[OwnedLexToken]) -> bool {
    const TARGET_TAIL_PHRASES: &[&[&str]] = &[
        &[],
        &["on", "it"],
        &["on", "that", "creature"],
        &["on", "that", "spell"],
        &["on", "that", "permanent"],
        &["on", "that", "card"],
    ];
    let clause = LexedClause::new(tail_tokens);
    TARGET_TAIL_PHRASES
        .iter()
        .any(|phrase| LexPattern::new(&[LexPattern::phrase(phrase)]).matches_clause(clause))
}

fn mana_spend_bonus_grants_uncounterable(clause: LexedClause<'_>) -> bool {
    const UNCOUNTERABLE_BONUS_PHRASES: &[&[&str]] = &[
        &["that", "spell", "can't", "be", "countered"],
        &["that", "spell", "cant", "be", "countered"],
    ];
    LexPattern::new(&[LexPattern::any_phrase(UNCOUNTERABLE_BONUS_PHRASES)]).matches_clause(clause)
}

fn mana_spend_bonus_granted_abilities(clause: LexedClause<'_>) -> Vec<StaticAbilityId> {
    const HASTE_BONUS_PHRASES: &[&[&str]] = &[
        &["it", "gains", "haste"],
        &["that", "spell", "gains", "haste"],
        &["that", "creature", "gains", "haste"],
        &["it", "gains", "haste", "until", "end", "of", "turn"],
        &[
            "that", "spell", "gains", "haste", "until", "end", "of", "turn",
        ],
        &[
            "that", "creature", "gains", "haste", "until", "end", "of", "turn",
        ],
    ];
    if LexPattern::new(&[LexPattern::any_phrase(HASTE_BONUS_PHRASES)]).matches_clause(clause) {
        vec![StaticAbilityId::Haste]
    } else {
        Vec::new()
    }
}

fn parse_mana_spend_bonus_sentence_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<ManaSpendBonusSentence<'a>> {
    const MANA_SPEND_BONUS_HEAD_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(IF_MANA_SPENT_SPELL_PREFIXES),
        LexPattern::object(
            "spell_spec",
            LexCaptureKind::UntilAnyPhrase(&[&["spell"], &["spells"]]),
        ),
        LexPattern::any_word(&["spell", "spells"]),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = MANA_SPEND_BONUS_HEAD_PATTERN.match_prefix(clause)?;
    let spec_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let head_end = clause.token_index_after_words(matched.word_range.end)?;
    let comma_offset =
        find_token_kind(tokens.get(head_end..).unwrap_or_default(), TokenKind::Comma)?;
    let comma_idx = head_end + comma_offset;
    Some(ManaSpendBonusSentence {
        spec_tokens: spec_clause.tokens(),
        bonus_tokens: tokens.get(comma_idx + 1..)?,
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

fn parse_mana_spend_bonus_spell_filter(spec_tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let filter = parse_spell_filter_with_grammar_entrypoint(spec_tokens);
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
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_triggered_times_each_turn_lexed(&tokens)
}

pub(crate) fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    const TRIGGERED_TIMES_EACH_TURN_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(&[
            &["this", "ability", "triggers", "only"],
            &["do", "this", "only"],
        ]),
        LexPattern::amount(
            "count",
            LexCaptureKind::UntilAnyPhrase(TIMES_EACH_TURN_TAILS),
        ),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = TRIGGERED_TIMES_EACH_TURN_PATTERN.match_clause(clause)?;
    let count_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let tail_clause = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;
    parse_count_with_each_turn_tail(count_clause, tail_clause)
}

fn parse_count_with_each_turn_tail(
    count_clause: LexedClause<'_>,
    tail_clause: LexedClause<'_>,
) -> Option<u32> {
    if !matches_each_turn_count_tail(tail_clause) {
        return None;
    }
    let (count, used) = parse_number(count_clause.tokens())?;
    (used == count_clause.word_refs().len()).then_some(count)
}

fn matches_each_turn_count_tail(tail_clause: LexedClause<'_>) -> bool {
    const EACH_TURN_COUNT_TAIL_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::any_phrase(TIMES_EACH_TURN_TAILS)]);

    EACH_TURN_COUNT_TAIL_PATTERN.matches_clause(tail_clause)
}

fn parse_activate_only_if_card_in_graveyard_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivateOnlyIfCardInGraveyardCondition<'_>> {
    const ACTIVATE_ONLY_IF_GRAVEYARD_CARD_TAILS: &[&[&str]] = &[
        &["in", "your", "graveyard"],
        &["in", "graveyard"],
        &["in", "the", "graveyard"],
    ];
    const ACTIVATE_ONLY_IF_GRAVEYARD_CARD_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(&[
            &["activate", "only", "if", "there", "is"],
            &["activate", "only", "if", "there", "are"],
        ]),
        LexPattern::object(
            "descriptor",
            LexCaptureKind::UntilAnyPhrase(ACTIVATE_ONLY_IF_GRAVEYARD_CARD_TAILS),
        ),
        LexPattern::any_phrase(ACTIVATE_ONLY_IF_GRAVEYARD_CARD_TAILS),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = ACTIVATE_ONLY_IF_GRAVEYARD_CARD_PATTERN.match_clause(clause)?;
    let descriptor = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if descriptor.word_refs().is_empty() {
        return None;
    }

    Some(ActivateOnlyIfCardInGraveyardCondition {
        descriptor_tokens: descriptor.tokens(),
    })
}

fn parse_activate_only_if_card_in_graveyard_condition(
    tokens: &[OwnedLexToken],
) -> Option<ConditionExpr> {
    let condition = parse_activate_only_if_card_in_graveyard_condition_tokens(tokens)?;
    let descriptor = LexedClause::new(condition.descriptor_tokens);

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for word in descriptor.word_refs() {
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

    Some(ConditionExpr::CardInYourGraveyard {
        card_types,
        subtypes,
    })
}

fn parse_activate_only_if_creatures_total_power_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivateOnlyIfCreaturesTotalPowerCondition<'_>> {
    const ACTIVATE_ONLY_IF_CREATURES_TOTAL_POWER_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["activate", "only", "if"]),
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["have"])),
        LexPattern::action("verb", LexCaptureKind::WordCount(1)),
        LexPattern::object("measure", LexCaptureKind::WordCount(2)),
        LexPattern::amount("comparison", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = ACTIVATE_ONLY_IF_CREATURES_TOTAL_POWER_PATTERN.match_clause(clause)?;
    let subject = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_creatures_you_control_clause(subject) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !clause_matches_phrase(action, &["have"]) {
        return None;
    }
    let measure = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !is_total_power_clause(measure) {
        return None;
    }
    let comparison = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    Some(ActivateOnlyIfCreaturesTotalPowerCondition {
        comparison_tokens: comparison.tokens(),
    })
}

fn parse_activate_only_if_creatures_total_power_condition(
    tokens: &[OwnedLexToken],
) -> Option<ConditionExpr> {
    let condition = parse_activate_only_if_creatures_total_power_condition_tokens(tokens)?;
    let comparison_words = LexedClause::new(condition.comparison_tokens).word_refs();
    let clause_words = LexedClause::new(tokens).word_refs();
    let (comparison, used) =
        parse_filter_comparison_tokens("power", &comparison_words, &clause_words).ok()??;
    let crate::filter::Comparison::GreaterThanOrEqual(threshold) = comparison else {
        return None;
    };
    if used != comparison_words.len() {
        return None;
    }
    Some(ConditionExpr::ControlCreaturesTotalPowerAtLeast(
        u32::try_from(threshold).ok()?,
    ))
}

fn is_creatures_you_control_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["creatures", "you", "control"])
}

fn is_total_power_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["total", "power"])
}

fn is_red_sources_you_controlled_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["red", "sources", "you", "controlled"])
}

fn is_creature_object_clause(clause: LexedClause<'_>) -> bool {
    const OPTIONAL_ARTICLE: &[LexPatternAtom<'static>] = &[LexPattern::any_word(&["a", "an"])];
    const CREATURE_OBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::optional(OPTIONAL_ARTICLE),
        LexPattern::object("object", LexCaptureKind::OneOf(&["creature"])),
    ]);

    CREATURE_OBJECT_PATTERN.matches_clause(clause)
}

fn is_you_control_condition_clause(clause: LexedClause<'_>) -> bool {
    parse_activate_only_if_you_control_object_clause(clause.tokens()).is_some()
}

fn parse_activate_only_if_sources_dealt_damage_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivateOnlyIfSourcesDealtDamageCondition<'_>> {
    const NONCOMBAT_DAMAGE_THIS_TURN_TAILS: &[&[&str]] = &[
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
    ];
    const ACTIVATE_ONLY_IF_SOURCES_DEALT_DAMAGE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["activate", "only", "if"]),
        LexPattern::subject("sources", LexCaptureKind::UntilPhrase(&["dealt"])),
        LexPattern::action("verb", LexCaptureKind::WordCount(1)),
        LexPattern::amount(
            "threshold",
            LexCaptureKind::UntilAnyPhrase(NONCOMBAT_DAMAGE_THIS_TURN_TAILS),
        ),
        LexPattern::modifier("tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = ACTIVATE_ONLY_IF_SOURCES_DEALT_DAMAGE_PATTERN.match_clause(clause)?;
    let source_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !clause_matches_phrase(action_clause, &["dealt"]) {
        return None;
    }
    let tail_clause = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !LexPattern::new(&[LexPattern::any_phrase(NONCOMBAT_DAMAGE_THIS_TURN_TAILS)])
        .matches_clause(tail_clause)
    {
        return None;
    }
    let threshold_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    Some(ActivateOnlyIfSourcesDealtDamageCondition {
        source_tokens: source_clause.tokens(),
        threshold_tokens: threshold_clause.tokens(),
    })
}

fn parse_activate_only_if_sources_dealt_damage_condition(
    tokens: &[OwnedLexToken],
) -> Option<ConditionExpr> {
    let condition = parse_activate_only_if_sources_dealt_damage_condition_tokens(tokens)?;
    let source_clause = LexedClause::new(condition.source_tokens);
    if !is_red_sources_you_controlled_clause(source_clause) {
        return None;
    }
    let threshold_clause = LexedClause::new(condition.threshold_tokens);
    let (threshold, used) = parse_number(threshold_clause.tokens())?;
    if used != threshold_clause.word_refs().len() {
        return None;
    }
    Some(ConditionExpr::ValueComparison {
        left: crate::effect::Value::NoncombatDamageDealtBySourcesControlledThisTurn {
            player: PlayerFilter::You,
            colors: Some(ColorSet::from_color(Color::Red)),
        },
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: crate::effect::Value::Fixed(threshold as i32),
    })
}

fn parse_activate_only_if_you_control_creature_power_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivateOnlyIfYouControlCreaturePowerCondition<'_>> {
    const ACTIVATE_ONLY_IF_YOU_CONTROL_CREATURE_POWER_PATTERN: LexPattern<'static> =
        LexPattern::new(&[
            LexPattern::phrase(&["activate", "only", "if"]),
            LexPattern::subject("controller", LexCaptureKind::WordCount(1)),
            LexPattern::action("verb", LexCaptureKind::WordCount(1)),
            LexPattern::object("object", LexCaptureKind::UntilPhrase(&["with", "power"])),
            LexPattern::phrase(&["with", "power"]),
            LexPattern::amount("comparison", LexCaptureKind::OneOrMoreWords),
        ]);

    let clause = LexedClause::new(tokens);
    let matched = ACTIVATE_ONLY_IF_YOU_CONTROL_CREATURE_POWER_PATTERN.match_clause(clause)?;
    let controller = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !clause_matches_phrase(controller, &["you"]) || !clause_matches_phrase(action, &["control"])
    {
        return None;
    }
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let comparison = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    Some(ActivateOnlyIfYouControlCreaturePowerCondition {
        object_tokens: object.tokens(),
        comparison_tokens: comparison.tokens(),
    })
}

fn parse_activate_only_if_you_control_creature_power_condition(
    tokens: &[OwnedLexToken],
) -> Option<ConditionExpr> {
    let condition = parse_activate_only_if_you_control_creature_power_condition_tokens(tokens)?;
    let object_clause = LexedClause::new(condition.object_tokens);
    if !is_creature_object_clause(object_clause) {
        return None;
    }

    let comparison_words = LexedClause::new(condition.comparison_tokens).word_refs();
    let clause_words = LexedClause::new(tokens).word_refs();
    let (comparison, used) =
        parse_filter_comparison_tokens("power", &comparison_words, &clause_words).ok()??;
    if used != comparison_words.len() {
        return None;
    }
    Some(ConditionExpr::YouControl(
        ObjectFilter::creature().with_power(comparison),
    ))
}

fn parse_activate_only_count_per_turn_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivateOnlyCountPerTurnCondition<'_>> {
    const EACH_TURN_COUNT_TAILS: &[&[&str]] = &[
        &["each", "turn"],
        &["time", "each", "turn"],
        &["times", "each", "turn"],
    ];
    const ACTIVATE_ONLY_COUNT_PER_TURN_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["activate", "only"]),
        LexPattern::amount(
            "count",
            LexCaptureKind::UntilAnyPhrase(EACH_TURN_COUNT_TAILS),
        ),
        LexPattern::modifier("tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = ACTIVATE_ONLY_COUNT_PER_TURN_PATTERN.match_clause(clause)?;
    let tail = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !LexPattern::new(&[LexPattern::any_phrase(EACH_TURN_COUNT_TAILS)]).matches_clause(tail) {
        return None;
    }
    let count = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    Some(ActivateOnlyCountPerTurnCondition {
        count_tokens: count.tokens(),
    })
}

fn parse_activate_only_count_per_turn_condition(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    let condition = parse_activate_only_count_per_turn_condition_tokens(tokens)?;
    let count_clause = LexedClause::new(condition.count_tokens);
    let (count, used) = parse_number(count_clause.tokens())?;
    if used != count_clause.word_refs().len() {
        return None;
    }
    Some(ConditionExpr::MaxActivationsPerTurn(count))
}

fn parse_activate_only_if_you_control_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    const ACTIVATE_ONLY_IF_YOU_CONTROL_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["activate", "only", "if"]),
        LexPattern::condition("condition", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = ACTIVATE_ONLY_IF_YOU_CONTROL_PATTERN.match_clause(clause)?;
    let condition = matched.capture_clause_by_role(LexCaptureRole::Condition, clause)?;
    if !is_you_control_condition_clause(condition) {
        return None;
    }
    Some(condition.tokens())
}

fn parse_activate_only_if_you_control_object_clause(
    tokens: &[OwnedLexToken],
) -> Option<LexedClause<'_>> {
    const YOU_CONTROL_OBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject("controller", LexCaptureKind::OneOf(&["you"])),
        LexPattern::action("control", LexCaptureKind::OneOf(&["control", "controls"])),
        LexPattern::object("object", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = YOU_CONTROL_OBJECT_PATTERN.match_clause(clause)?;
    matched.capture_clause_by_role(LexCaptureRole::Object, clause)
}

fn activate_only_if_you_control_land_subtype_condition(
    control_condition_tokens: &[OwnedLexToken],
) -> Option<ConditionExpr> {
    let object_clause = parse_activate_only_if_you_control_object_clause(control_condition_tokens)?;
    let mut subtypes = Vec::new();
    for word in object_clause.word_refs() {
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

pub(crate) fn parse_activation_condition_lexed(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    if let Some(condition) = parse_activate_count_each_turn_condition(tokens) {
        return Some(condition);
    }

    if let Some(condition) = parse_activate_only_count_per_turn_condition(tokens) {
        return Some(condition);
    }
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_INSTANT_PREFIXES).is_some() {
        return Some(ConditionExpr::ActivationTiming(ActivationTiming::AnyTime));
    }
    if let Some(condition) = parse_activate_only_if_card_in_graveyard_condition(tokens) {
        return Some(condition);
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
    if let Some(condition) = parse_activate_only_if_creatures_total_power_condition(tokens) {
        return Some(condition);
    }
    if let Some(condition) = parse_activate_only_if_sources_dealt_damage_condition(tokens) {
        return Some(condition);
    }
    if let Some(condition) = parse_activate_only_if_you_control_creature_power_condition(tokens) {
        return Some(condition);
    }
    let control_condition_tokens = parse_activate_only_if_you_control_tail_tokens(tokens)?;
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

    activate_only_if_you_control_land_subtype_condition(control_condition_tokens)
}

fn parse_activate_count_each_turn_condition(tokens: &[OwnedLexToken]) -> Option<ConditionExpr> {
    const ACTIVATE_COUNT_EACH_TURN_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("activate"),
        LexPattern::amount(
            "count",
            LexCaptureKind::UntilAnyPhrase(TIMES_EACH_TURN_TAILS),
        ),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = ACTIVATE_COUNT_EACH_TURN_PATTERN.match_clause(clause)?;
    let count_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let tail_clause = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;
    let count = parse_less_than_or_equal_quantity_prefix(
        count_clause.tokens(),
        false,
        false,
        "activation frequency condition",
    )
    .ok()
    .flatten()
    .and_then(|(count, used)| (used == count_clause.word_refs().len()).then_some(count))?;
    matches_each_turn_count_tail(tail_clause).then_some(ConditionExpr::MaxActivationsPerTurn(count))
}

pub(crate) fn parse_activation_count_per_turn(words: &[&str]) -> Option<u32> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::amount(
            "count",
            LexCaptureKind::UntilAnyPhrase(TIMES_EACH_TURN_TAILS),
        ),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let count_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let tail_clause = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;
    parse_count_with_each_turn_tail(count_clause, tail_clause)
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
    LexPattern::new(&[LexPattern::phrase(DRAW_REPLACEMENT_DOUBLE_PHRASE)])
        .matches_clause(LexedClause::new(tokens))
}

pub(crate) fn is_draw_replacement_skip_empty_library_line_lexed(tokens: &[OwnedLexToken]) -> bool {
    LexPattern::new(&[LexPattern::phrase(
        DRAW_REPLACEMENT_SKIP_EMPTY_LIBRARY_PHRASE,
    )])
    .matches_clause(LexedClause::new(tokens))
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
    LexPattern::new(&[LexPattern::phrase(
        OPPONENT_DISCARD_THIS_TO_BATTLEFIELD_REPLACEMENT_PHRASE,
    )])
    .matches_clause(LexedClause::new(tokens))
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
    const CAN_BLOCK_AS_THOUGH_REACH_TAIL: &[&str] = &["as", "though", "it", "had", "reach"];
    const THIS_CREATURE_CAN_BLOCK_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["this", "creature", "can", "block"]),
        LexPattern::object("subtype", LexCaptureKind::WordCount(1)),
        LexPattern::phrase(CAN_BLOCK_AS_THOUGH_REACH_TAIL),
    ]);
    const THIS_CAN_BLOCK_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["this", "can", "block"]),
        LexPattern::object("subtype", LexCaptureKind::WordCount(1)),
        LexPattern::phrase(CAN_BLOCK_AS_THOUGH_REACH_TAIL),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = THIS_CREATURE_CAN_BLOCK_PATTERN
        .match_clause(clause)
        .or_else(|| THIS_CAN_BLOCK_PATTERN.match_clause(clause))?;
    let subtype_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let subtype_word = subtype_clause.word_refs().first().copied()?;

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

pub(crate) fn is_prevent_all_noncombat_damage_to_matching_permanents_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    const PREVENT_ALL_NONCOMBAT_DAMAGE_TO_PREFIX: &[&str] = &[
        "prevent",
        "all",
        "noncombat",
        "damage",
        "that",
        "would",
        "be",
        "dealt",
        "to",
    ];
    const PREVENT_ALL_NONCOMBAT_DAMAGE_TO_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(PREVENT_ALL_NONCOMBAT_DAMAGE_TO_PREFIX),
        LexPattern::object("object", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let Some(matched) = PREVENT_ALL_NONCOMBAT_DAMAGE_TO_PATTERN.match_clause(clause) else {
        return false;
    };
    let Some(object) = matched.capture_clause_by_role(LexCaptureRole::Object, clause) else {
        return false;
    };
    let object_words = object.word_refs();
    !matches!(
        object_words.as_slice(),
        ["this", "creature"] | ["this", "permanent"] | ["it"]
    ) && !object_words.iter().any(|word| *word == "turn")
}

pub(crate) fn is_prevent_all_combat_damage_to_matching_permanents_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    const PREVENT_ALL_COMBAT_DAMAGE_TO_PREFIX: &[&str] = &[
        "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to",
    ];
    const PREVENT_ALL_COMBAT_DAMAGE_TO_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(PREVENT_ALL_COMBAT_DAMAGE_TO_PREFIX),
        LexPattern::object("object", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let Some(matched) = PREVENT_ALL_COMBAT_DAMAGE_TO_PATTERN.match_clause(clause) else {
        return false;
    };
    let Some(object) = matched.capture_clause_by_role(LexCaptureRole::Object, clause) else {
        return false;
    };
    let object_words = object.word_refs();
    !matches!(
        object_words.as_slice(),
        ["this", "creature"] | ["this", "permanent"] | ["it"]
    ) && !object_words.iter().any(|word| *word == "turn")
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

use winnow::combinator::{opt, seq};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::any;

use crate::ConditionExpr;
use crate::ability::{ActivationTiming, ManaUsageRestriction};
use crate::object::CounterType;
use crate::target::ObjectFilter;
use crate::target::PlayerFilter;
use crate::zone::Zone;

use super::super::activation_helpers::parse_subtype_flexible;
use super::super::effect_sentences::parse_subtype_word;
use super::super::lexer::{
    LexStream, LexToken, OwnedLexToken, TokenKind, TokenWordView, trim_lexed_commas,
};
use super::super::token_primitives::{slice_contains, slice_starts_with, str_strip_suffix};
use super::primitives;
use crate::cards::builders::compiler::util::{
    parse_card_type, parse_counter_type_word, parse_counter_type_from_tokens, parse_number_word_u32,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UntapEachOtherPlayersUntapStepSpec<'a> {
    pub(crate) untap_all: bool,
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
    let ((_, untap_all), remainder) =
        primitives::parse_prefix(tokens, (primitives::kw("untap"), opt(primitives::kw("all"))))?;
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
        .is_some_and(|token| token.is_word("whenever"))
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

pub(crate) fn is_doesnt_untap_during_your_untap_step_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
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
    primitives::parse_prefix(tokens, primitives::phrase(&["as", "this", "land", "enters"]))
        .is_some()
        && primitives::contains_phrase(tokens, &["you", "may", "reveal"])
        && primitives::contains_phrase(tokens, &["from", "your", "hand"])
}

pub(crate) fn is_land_reveal_enters_tapped_followup_line_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
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
    &["spend", "this", "mana", "only"],
    &["spend", "that", "mana", "only"],
];
const SPEND_MANA_CAST_PREFIXES: &[&[&str]] = &[
    &["spend", "this", "mana", "only", "to", "cast"],
    &["spend", "that", "mana", "only", "to", "cast"],
];
const IF_MANA_SPENT_TO_CAST_PREFIXES: &[&[&str]] = &[
    &["if", "this", "mana", "is", "spent", "to", "cast"],
    &["if", "that", "mana", "is", "spent", "to", "cast"],
];
const DURING_OPPONENTS_TURN_PREFIXES: &[&[&str]] = &[
    &["activate", "only", "during", "an", "opponents", "turn"],
    &["activate", "only", "during", "opponents", "turn"],
];
const ACTIVATE_ONLY_INSTANT_PREFIXES: &[&[&str]] = &[
    &["activate", "only", "as", "an", "instant"],
    &["activate", "only", "as", "instant"],
];
const ACTIVATE_ONLY_IF_THERE_PREFIXES: &[&[&str]] = &[
    &["activate", "only", "if", "there", "is"],
    &["activate", "only", "if", "there", "are"],
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

pub(crate) fn parse_activate_only_timing_lexed(tokens: &[OwnedLexToken]) -> Option<ActivationTiming> {
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
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_DURING_YOUR_TURN_PREFIXES)
        .is_some()
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
    let words = TokenWordView::new(tokens);
    if primitives::words_match_any_prefix(tokens, SPEND_MANA_CAST_PREFIXES).is_none() {
        return None;
    }

    let mut spell_idx = None;
    for idx in 0..words.len() {
        if matches!(words.get(idx), Some("spell" | "spells")) {
            spell_idx = Some(idx);
            break;
        }
    }
    let spell_idx = spell_idx?;
    let spec_words = (6..spell_idx)
        .filter_map(|idx| words.get(idx))
        .collect::<Vec<_>>();
    if spec_words.is_empty() {
        return None;
    }

    let mut idx = 0usize;
    if matches!(spec_words.first().copied(), Some("a" | "an")) {
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
    let subtype_requirement = if slice_starts_with(&tail, &["of", "the", "chosen", "type"]) {
        tail.drain(0..4);
        Some(crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource)
    } else {
        None
    };

    let grant_uncounterable = tail == ["and", "that", "spell", "can't", "be", "countered"]
        || tail == ["and", "that", "spell", "cant", "be", "countered"];
    if !grant_uncounterable && !tail.is_empty() {
        return None;
    }

    Some(ManaUsageRestriction::CastSpell {
        card_types: vec![card_type],
        subtype_requirement,
        restrict_to_matching_spell: true,
        grant_uncounterable,
        enters_with_counters: vec![],
    })
}

pub(crate) fn parse_mana_spend_bonus_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaUsageRestriction> {
    if primitives::words_match_any_prefix(tokens, IF_MANA_SPENT_TO_CAST_PREFIXES).is_none() {
        return None;
    }

    let words = TokenWordView::new(tokens);
    let mut spell_idx = None;
    for idx in 0..words.len() {
        if matches!(words.get(idx), Some("spell" | "spells")) {
            spell_idx = Some(idx);
            break;
        }
    }
    let spell_idx = spell_idx?;

    let spec_words = (7..spell_idx)
        .filter_map(|idx| words.get(idx))
        .collect::<Vec<_>>();
    if spec_words.is_empty() {
        return None;
    }

    let mut idx = 0usize;
    if matches!(spec_words.first().copied(), Some("a" | "an")) {
        idx += 1;
    }

    let card_type = parse_card_type(spec_words.get(idx).copied()?)?;
    idx += 1;
    if idx != spec_words.len() {
        return None;
    }

    let comma_idx = tokens.iter().position(OwnedLexToken::is_comma)?;
    let clause_tokens = trim_lexed_commas(&tokens[comma_idx + 1..]);
    if clause_tokens.is_empty() {
        return None;
    }

    let clause_word_view = TokenWordView::new(&clause_tokens);
    let clause_words = clause_word_view.to_word_refs();
    if clause_words.len() < 6 || clause_words.first().copied() != Some("that") {
        return None;
    }
    if !matches!(
        clause_words.get(1).copied(),
        Some("creature" | "spell" | "permanent" | "card")
    ) && parse_card_type(clause_words.get(1).copied()?).is_none()
    {
        return None;
    }

    let enters_idx = clause_words
        .iter()
        .position(|word| matches!(*word, "enter" | "enters"))?;
    let with_token_idx = clause_tokens
        .iter()
        .position(|token| token.is_word("with"))?;
    let after_with = &clause_tokens[with_token_idx + 1..];
    if after_with.is_empty() {
        return None;
    }

    let (count, used) = if after_with
        .first()
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
        && after_with
            .get(1)
            .is_some_and(|token| token.is_word("additional"))
    {
        (1, 2)
    } else if after_with
        .first()
        .is_some_and(|token| token.is_word("additional"))
    {
        (1, 1)
    } else if let Some(word) = after_with.first().and_then(OwnedLexToken::as_word) {
        let parsed = parse_number_word_u32(word)?;
        let used = if after_with
            .get(1)
            .is_some_and(|token| token.is_word("additional"))
        {
            2
        } else {
            1
        };
        (parsed, used)
    } else {
        return None;
    };

    let counter_type = parse_counter_type_from_tokens(&after_with[used..])?;
    let counter_idx = after_with.iter().position(|token| {
        token.is_word("counter") || token.is_word("counters")
    })?;
    let tail_tokens = trim_lexed_commas(&after_with[counter_idx + 1..]);
    let mut tail: &[OwnedLexToken] = &tail_tokens;
    if tail.first().is_some_and(|token| token.is_word("on")) {
        tail = &tail[1..];
    }
    if tail.first().is_some_and(|token| token.is_word("it")) {
        tail = &tail[1..];
    } else if tail.first().is_some_and(|token| token.is_word("that")) {
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
    })
}

pub(crate) fn is_mana_spend_bonus_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::words_match_any_prefix(tokens, IF_MANA_SPENT_TO_CAST_PREFIXES).is_some()
}

pub(crate) fn is_any_player_may_activate_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens);
    words.len() == 6
        && primitives::words_match_prefix(
            tokens,
            &["any", "player", "may", "activate", "this", "ability"],
        )
        .is_some()
}

pub(crate) fn is_trigger_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::words_match_any_prefix(tokens, THIS_ABILITY_TRIGGERS_ONLY_PREFIXES).is_some()
}

pub(crate) fn parse_triggered_times_each_turn_from_words(words: &[&str]) -> Option<u32> {
    let (count_idx, prefix_len) =
        if slice_starts_with(words, &["this", "ability", "triggers", "only"]) {
            (4usize, 4usize)
        } else if slice_starts_with(words, &["do", "this", "only"]) {
            (3usize, 3usize)
        } else {
            return None;
        };

    if words.len() < prefix_len + 3 {
        return None;
    }

    let mut index = count_idx;
    let count = match words.get(index) {
        Some(word) if *word == "once" => Some(1),
        Some(word) if *word == "twice" => Some(2),
        Some(word) => parse_number_word_u32(word),
        None => None,
    }?;
    index += 1;

    if words.get(index) == Some(&"time") || words.get(index) == Some(&"times") {
        index += 1;
    }

    if words.get(index) == Some(&"each") && words.get(index + 1) == Some(&"turn") {
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

    if primitives::words_match_any_prefix(tokens, &[&["activate", "no", "more", "than"]]).is_some()
    {
        let count_word = words.get(4)?;
        let count = match count_word {
            "once" => 1,
            "twice" => 2,
            other => parse_number_word_u32(other)?,
        };
        let mut index = 5usize;
        if matches!(words.get(index), Some("time" | "times")) {
            index += 1;
        }
        if words.get(index) == Some("each") && words.get(index + 1) == Some("turn") {
            return Some(ConditionExpr::MaxActivationsPerTurn(count));
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
    if primitives::words_match_any_prefix(tokens, ACTIVATE_ONLY_IF_THERE_PREFIXES).is_some() {
        let descriptor_start = 5usize;
        let mut in_idx = None;
        for idx in descriptor_start..words.len() {
            if words.get(idx) == Some("in") {
                in_idx = Some(idx);
                break;
            }
        }
        let in_idx = in_idx?;
        let zone_tail = (in_idx..words.len())
            .filter_map(|idx| words.get(idx))
            .collect::<Vec<_>>();
        let points_to_your_graveyard = zone_tail == ["in", "your", "graveyard"]
            || zone_tail == ["in", "graveyard"]
            || zone_tail == ["in", "the", "graveyard"];
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
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| str_strip_suffix(word, "s").and_then(parse_subtype_word))
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
    if primitives::words_match_prefix(
        tokens,
        &[
            "activate",
            "only",
            "if",
            "creatures",
            "you",
            "control",
            "have",
            "total",
            "power",
        ],
    )
    .is_some()
    {
        let threshold_word = words.get(9)?;
        let threshold = parse_number_word_u32(threshold_word)?;
        let tail = (10..words.len())
            .filter_map(|idx| words.get(idx))
            .collect::<Vec<_>>();
        if tail == ["or", "greater"] {
            return Some(ConditionExpr::ControlCreaturesTotalPowerAtLeast(threshold));
        }
        return None;
    }
    if primitives::words_match_prefix(tokens, &["activate", "only", "if", "you", "control"])
        .is_none()
    {
        return None;
    }

    let control_tail = (5..words.len())
        .filter_map(|idx| words.get(idx))
        .collect::<Vec<_>>();
    if slice_starts_with(&control_tail, &["a", "creature", "with", "power"])
        || slice_starts_with(&control_tail, &["creature", "with", "power"])
    {
        let power_idx = control_tail.iter().position(|word| *word == "power")?;
        let threshold = parse_number_word_u32(control_tail.get(power_idx + 1)?)?;
        let tail = &control_tail[power_idx + 2..];
        if tail == ["or", "greater"] {
            return Some(ConditionExpr::YouControl(
                ObjectFilter::creature().with_power(crate::filter::Comparison::GreaterThanOrEqual(
                    threshold as i32,
                )),
            ));
        }
        return None;
    }
    if let Some(count) = control_tail.first().and_then(|word| parse_number_word_u32(word)) {
        let tail = &control_tail[1..];
        if tail == ["or", "more", "artifact"] || tail == ["or", "more", "artifacts"] {
            let mut filter = ObjectFilter::artifact();
            filter.zone = Some(Zone::Battlefield);
            return Some(ConditionExpr::PlayerControlsAtLeast {
                player: PlayerFilter::You,
                filter,
                count,
            });
        }
        if tail == ["or", "more", "land"] || tail == ["or", "more", "lands"] {
            let mut filter = ObjectFilter::default().with_type(crate::types::CardType::Land);
            filter.zone = Some(Zone::Battlefield);
            return Some(ConditionExpr::PlayerControlsAtLeast {
                player: PlayerFilter::You,
                filter,
                count,
            });
        }
    }
    if control_tail == ["an", "artifact"]
        || control_tail == ["a", "artifact"]
        || control_tail == ["artifact"]
        || control_tail == ["artifacts"]
    {
        let mut filter = ObjectFilter::artifact();
        filter.zone = Some(Zone::Battlefield);
        return Some(ConditionExpr::PlayerControlsAtLeast {
            player: PlayerFilter::You,
            filter,
            count: 1,
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
    let count = parse_number_word_u32(words.first()?)?;
    let mut index = 1usize;
    if words
        .get(index)
        .is_some_and(|word| *word == "time" || *word == "times")
    {
        index += 1;
    }
    if words.get(index) == Some(&"each") && words.get(index + 1) == Some(&"turn") {
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
    if TokenWordView::new(head_tokens).word_refs().get(..1) != Some(&["gift"][..]) {
        return false;
    }
    if !primitives::contains_phrase(
        tokens,
        &[
            "you", "may", "promise", "an", "opponent", "a", "gift", "as", "you", "cast", "this",
            "spell",
        ],
    ) || !primitives::contains_phrase(tokens, &["if", "you", "do"])
    {
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
    let comma_idx = tokens
        .iter()
        .enumerate()
        .find_map(|(idx, token)| (token.kind == TokenKind::Comma).then_some(idx));
    let effect_start = if let Some(idx) = comma_idx {
        idx + 1
    } else if let Some(idx) = tokens.iter().position(|token| token.is_word("spell")) {
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

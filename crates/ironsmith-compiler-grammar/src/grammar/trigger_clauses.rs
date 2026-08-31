use std::ops::Range;

use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::events::KeywordActionKind;
use crate::object::CounterType;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

use super::super::lexer::{LexStream, OwnedLexToken};
use super::{filters, leaf, primitives};

#[path = "trigger_clauses/surface_patterns.rs"]
mod surface_patterns;
pub use surface_patterns::*;

#[path = "trigger_clauses/token_helpers.rs"]
mod token_helpers;
use token_helpers::*;

#[path = "trigger_clauses/life_loss.rs"]
mod life_loss;
pub use life_loss::*;

#[cfg(test)]
#[path = "trigger_clauses/tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerClauseTokenSpan {
    pub first: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerClauseAtom {
    Ability,
    Activate,
    And,
    Attack,
    Becomes,
    Block,
    By,
    Cast,
    Copy,
    Counter,
    Create,
    Damage,
    Deal,
    Die,
    Discard,
    Draw,
    Enter,
    For,
    Get,
    Give,
    IsOrAre,
    Leave,
    Mana,
    More,
    One,
    Or,
    Play,
    Put,
    Reveal,
    Roll,
    Sacrifice,
    Search,
    Shuffle,
    Tap,
    Tapped,
    To,
    Transform,
    TriggerIntro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivationCostTapCondition {
    pub required: bool,
    pub condition_word: usize,
    pub condition_token: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterDescriptorSpans {
    pub descriptor: Range<usize>,
    pub with_counter: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterRecipientSpan {
    pub tokens: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerOrSplit {
    pub separator: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoyaltyAbilityTail {
    pub owner: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PossessiveAbilityTail {
    pub owner: Range<usize>,
    pub marker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedAbilityTail {
    pub marker: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityOfObjectTail {
    pub filter: Range<usize>,
    pub non_mana_only: bool,
    pub chosen_type_reference: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayersAttackedClause {
    pub player: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FullyUnlockRoomTrigger {
    pub action: KeywordActionKind,
    pub player: PlayerFilter,
    pub source_filter: ObjectFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntersOriginClause {
    pub zone: Zone,
    pub owner: Option<PlayerFilter>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceTriggerSubjectShape {
    pub filter: ObjectFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct YouOrControlledObjectSubject {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollResultShape {
    HighestNatural,
    Fixed(u32),
    UnspecifiedDie,
    OneOrMoreDice,
}

pub fn parse_not_during_turn_draw_suffix_words(words: &[&str]) -> Option<PlayerFilter> {
    primitives::parse_full_word_slice(words, parse_not_during_turn_draw_suffix_word_slice)
}

pub fn parse_enters_origin_clause_words(words: &[&str]) -> Option<EntersOriginClause> {
    let normalized = words
        .iter()
        .copied()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect::<Vec<_>>();
    primitives::parse_full_word_slice(&normalized, parse_enters_origin_clause_word_slice)
}

pub fn parse_source_trigger_subject_words(words: &[&str]) -> SourceTriggerSubjectShape {
    let mut input: primitives::WordSliceInput<'_> = words;
    let facts = parse_source_trigger_subject_facts
        .parse_next(&mut input)
        .unwrap_or_default();
    let mut filter = ObjectFilter::default();
    if let Some(card_type) = facts.card_type() {
        filter.card_types.push(card_type);
    }
    SourceTriggerSubjectShape { filter }
}

pub fn parse_you_or_controlled_object_subject_words(
    words: &[&str],
) -> Option<YouOrControlledObjectSubject> {
    let normalized = words
        .iter()
        .copied()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect::<Vec<_>>();
    let filter = primitives::parse_full_word_slice(
        &normalized,
        parse_you_or_controlled_object_subject_word_slice,
    )?;
    Some(YouOrControlledObjectSubject {
        player: PlayerFilter::You,
        filter: filter.you_control(),
    })
}

pub fn parse_opponents_each_lose_exact_life_words(words: &[&str]) -> Option<u32> {
    primitives::parse_full_word_slice(words, parse_opponents_each_lose_exact_life_word_slice)
}

pub fn parse_roll_result_words(words: &[&str]) -> Option<RollResultShape> {
    primitives::parse_full_word_slice(words, parse_roll_result_word_slice)
}

fn parse_not_during_turn_draw_suffix_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<PlayerFilter> {
    (
        primitives::word_slice_exact("a"),
        primitives::word_slice_exact("card"),
        primitives::word_slice_exact("if"),
        alt((
            (
                primitives::word_slice_exact("it"),
                primitives::word_slice_exact("isnt"),
            ),
            (
                primitives::word_slice_exact("its"),
                primitives::word_slice_exact("not"),
            ),
        )),
        alt((
            (
                primitives::word_slice_exact("that"),
                primitives::word_slice_exact("players"),
                primitives::word_slice_exact("turn"),
            )
                .value(PlayerFilter::IteratedPlayer),
            (
                primitives::word_slice_exact("their"),
                primitives::word_slice_exact("turn"),
            )
                .value(PlayerFilter::IteratedPlayer),
            (
                primitives::word_slice_exact("your"),
                primitives::word_slice_exact("turn"),
            )
                .value(PlayerFilter::You),
            (
                winnow::combinator::opt(primitives::word_slice_exact("an")),
                primitives::word_slice_exact("opponents"),
                primitives::word_slice_exact("turn"),
            )
                .value(PlayerFilter::Opponent),
        )),
    )
        .map(|(_, _, _, _, player)| player)
        .parse_next(input)
}

fn parse_enters_origin_clause_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<EntersOriginClause> {
    (
        primitives::word_slice_exact("from"),
        alt((
            (
                primitives::word_slice_exact("your"),
                primitives::word_slice_exact("graveyard"),
            )
                .value(EntersOriginClause {
                    zone: Zone::Graveyard,
                    owner: Some(PlayerFilter::You),
                }),
            primitives::word_slice_exact("graveyard").value(EntersOriginClause {
                zone: Zone::Graveyard,
                owner: None,
            }),
            (
                primitives::word_slice_exact("your"),
                primitives::word_slice_exact("hand"),
            )
                .value(EntersOriginClause {
                    zone: Zone::Hand,
                    owner: Some(PlayerFilter::You),
                }),
            primitives::word_slice_exact("hand").value(EntersOriginClause {
                zone: Zone::Hand,
                owner: None,
            }),
            primitives::word_slice_exact("exile").value(EntersOriginClause {
                zone: Zone::Exile,
                owner: None,
            }),
        )),
    )
        .map(|(_, origin)| origin)
        .parse_next(input)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct SourceTriggerSubjectFacts {
    creature: bool,
    land: bool,
    artifact: bool,
    enchantment: bool,
    planeswalker: bool,
    battle: bool,
}

impl SourceTriggerSubjectFacts {
    fn card_type(self) -> Option<CardType> {
        if self.creature {
            Some(CardType::Creature)
        } else if self.land {
            Some(CardType::Land)
        } else if self.artifact {
            Some(CardType::Artifact)
        } else if self.enchantment {
            Some(CardType::Enchantment)
        } else if self.planeswalker {
            Some(CardType::Planeswalker)
        } else if self.battle {
            Some(CardType::Battle)
        } else {
            None
        }
    }
}

fn parse_source_trigger_subject_facts(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<SourceTriggerSubjectFacts> {
    let mut facts = SourceTriggerSubjectFacts::default();
    while !input.is_empty() {
        let word: &str = any.parse_next(input)?;
        match word {
            "creature" => facts.creature = true,
            "land" => facts.land = true,
            "artifact" => facts.artifact = true,
            "enchantment" => facts.enchantment = true,
            "planeswalker" => facts.planeswalker = true,
            "battle" => facts.battle = true,
            _ => {}
        }
    }
    Ok(facts)
}

fn parse_you_or_controlled_object_subject_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<ObjectFilter> {
    (
        primitives::word_slice_exact("you"),
        primitives::word_slice_exact("or"),
    )
        .parse_next(input)?;
    let filter = alt((
        alt((
            primitives::word_slice_exact("permanent"),
            primitives::word_slice_exact("permanents"),
        ))
        .value(ObjectFilter::permanent()),
        alt((
            primitives::word_slice_exact("creature"),
            primitives::word_slice_exact("creatures"),
        ))
        .value(ObjectFilter::creature()),
        alt((
            primitives::word_slice_exact("artifact"),
            primitives::word_slice_exact("artifacts"),
        ))
        .value(ObjectFilter::artifact()),
        alt((
            primitives::word_slice_exact("enchantment"),
            primitives::word_slice_exact("enchantments"),
        ))
        .value(ObjectFilter::enchantment()),
        alt((
            primitives::word_slice_exact("land"),
            primitives::word_slice_exact("lands"),
        ))
        .value(ObjectFilter::land()),
        alt((
            primitives::word_slice_exact("planeswalker"),
            primitives::word_slice_exact("planeswalkers"),
        ))
        .value(ObjectFilter::planeswalker()),
    ))
    .parse_next(input)?;
    (
        primitives::word_slice_exact("you"),
        primitives::word_slice_exact("control"),
    )
        .parse_next(input)?;
    Ok(filter)
}

fn parse_opponents_each_lose_exact_life_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<u32> {
    (
        primitives::word_slice_exact("one"),
        primitives::word_slice_exact("or"),
        primitives::word_slice_exact("more"),
        primitives::word_slice_exact("opponents"),
        primitives::word_slice_exact("each"),
        primitives::word_slice_exact("lose"),
        primitives::word_slice_exact("exactly"),
    )
        .parse_next(input)?;
    let amount = parse_fixed_number_word_slice.parse_next(input)?;
    primitives::word_slice_exact("life").parse_next(input)?;
    Ok(amount)
}

fn parse_roll_result_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<RollResultShape> {
    winnow::combinator::opt(primitives::word_slice_exact("a"))
        .void()
        .parse_next(input)?;
    alt((
        (
            alt((
                primitives::word_slice_exact("die's"),
                primitives::word_slice_exact("dies"),
            )),
            primitives::word_slice_exact("highest"),
            primitives::word_slice_exact("natural"),
            primitives::word_slice_exact("result"),
        )
            .value(RollResultShape::HighestNatural),
        (
            primitives::word_slice_exact("one"),
            primitives::word_slice_exact("or"),
            primitives::word_slice_exact("more"),
            alt((
                primitives::word_slice_exact("die"),
                primitives::word_slice_exact("dice"),
            )),
        )
            .value(RollResultShape::OneOrMoreDice),
        parse_fixed_number_word_slice.map(RollResultShape::Fixed),
        alt((
            primitives::word_slice_exact("die"),
            primitives::word_slice_exact("dice"),
        ))
        .value(RollResultShape::UnspecifiedDie),
    ))
    .parse_next(input)
}

fn parse_fixed_number_word_slice(input: &mut primitives::WordSliceInput<'_>) -> WResult<u32> {
    let parsed = leaf::parse_leaf_number_prefix_words(input)
        .and_then(leaf::LeafNumberPrefix::into_fixed)
        .ok_or_else(|| primitives::backtrack_err("trigger count", "fixed number"))?;
    *input = input
        .get(parsed.1..)
        .ok_or_else(|| primitives::backtrack_err("trigger count", "available words"))?;
    Ok(parsed.0)
}

pub fn parse_trigger_clause_atom_token(
    tokens: &[OwnedLexToken],
    atom: TriggerClauseAtom,
) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_atom_token_lexed(input, atom)
    })
}

pub fn parse_players_attacked_clause(tokens: &[OwnedLexToken]) -> Option<PlayersAttackedClause> {
    let view = primitives::TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    let player_words =
        crate::grammar::primitives::take_leaf(&mut input, parse_players_attacked_words)?;
    let player_end = view.token_start_indices().get(player_words).copied()?;
    Some(PlayersAttackedClause {
        player: 0..player_end,
    })
}

pub fn parse_fully_unlock_room_trigger(tokens: &[OwnedLexToken]) -> Option<FullyUnlockRoomTrigger> {
    let words = primitives::TokenWordView::new(tokens).word_refs();
    primitives::parse_full_word_slice(&words, parse_fully_unlock_room_words)?;
    Some(FullyUnlockRoomTrigger {
        action: KeywordActionKind::UnlockDoor,
        player: PlayerFilter::You,
        source_filter: ObjectFilter::default().with_subtype(Subtype::Room),
    })
}

pub fn parse_trigger_clause_atom_word(words: &[&str], atom: TriggerClauseAtom) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_atom_word_slice(input, atom)
    })
}

pub fn parse_trigger_keyword_action_word(
    words: &[&str],
    action: KeywordActionKind,
) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_keyword_action_word_slice(input, action)
    })
}

pub fn parse_trigger_word_span_tokens(
    tokens: &[OwnedLexToken],
    word_index: usize,
) -> Option<TriggerClauseTokenSpan> {
    let view = primitives::TokenWordView::new(tokens);
    let first = view.token_start_indices().get(word_index).copied()?;
    let end = view.token_index_after_words(word_index + 1)?;
    Some(TriggerClauseTokenSpan { first, end })
}

pub fn parse_activation_cost_tap_condition(
    tokens: &[OwnedLexToken],
) -> Option<ActivationCostTapCondition> {
    let view = primitives::TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    let (condition_word, required) = crate::grammar::primitives::take_leaf(
        &mut input,
        parse_activation_cost_tap_condition_words,
    )?;
    let condition_token = view.token_start_indices().get(condition_word).copied()?;
    Some(ActivationCostTapCondition {
        required,
        condition_word,
        condition_token,
    })
}

pub fn parse_subject_before_suffix_span(
    tokens: &[OwnedLexToken],
    total_word_len: usize,
    suffix_word_len: usize,
) -> TriggerClauseTokenSpan {
    let subject_word_len = total_word_len.saturating_sub(suffix_word_len);
    let end = parse_trigger_word_span_tokens(tokens, subject_word_len)
        .map(|span| span.first)
        .unwrap_or(0);
    TriggerClauseTokenSpan { first: 0, end }
}

pub fn parse_counter_descriptor_spans(
    tokens: &[OwnedLexToken],
    start_word_idx: usize,
    counter_word_idx: usize,
) -> Option<CounterDescriptorSpans> {
    let descriptor_start = parse_trigger_word_span_tokens(tokens, start_word_idx)?.first;
    let descriptor_end = parse_trigger_word_span_tokens(tokens, counter_word_idx)
        .map(|span| span.first)
        .unwrap_or(tokens.len());
    let with_counter_end = descriptor_end.checked_add(1)?.min(tokens.len());
    Some(CounterDescriptorSpans {
        descriptor: descriptor_start..descriptor_end,
        with_counter: descriptor_start..with_counter_end,
    })
}

pub fn parse_trigger_counter_type(tokens: &[OwnedLexToken]) -> Option<CounterType> {
    let view = primitives::TokenWordView::new(tokens);
    let words = view.word_refs();
    let quantifier_words = parse_counter_quantifier_word_count(&words);
    let descriptor_first = if quantifier_words == 0 {
        0
    } else {
        parse_trigger_word_span_tokens(tokens, quantifier_words)?.first
    };
    let descriptor = &tokens[descriptor_first..];
    filters::parse_counter_type_from_tokens(descriptor).or_else(|| {
        parse_energy_descriptor_words(&primitives::TokenWordView::new(descriptor).word_refs())
            .then_some(CounterType::Energy)
    })
}

pub fn parse_counter_recipient_span(
    tokens: &[OwnedLexToken],
    object_word_start: usize,
) -> Option<CounterRecipientSpan> {
    let first = parse_trigger_word_span_tokens(tokens, object_word_start)?.first;
    let mut range = trim_comma_range(tokens, first..tokens.len());
    let candidate = &tokens[range.clone()];
    if let Some(((), rest)) = primitives::parse_prefix(candidate, parse_article_lexed) {
        range.start += candidate.len().saturating_sub(rest.len());
        range = trim_comma_range(tokens, range);
    }
    (range.start < range.end).then_some(CounterRecipientSpan { tokens: range })
}

pub fn parse_transform_destination_span(
    tokens: &[OwnedLexToken],
    transforms_word_idx: usize,
) -> Option<TriggerClauseTokenSpan> {
    parse_trigger_word_span_tokens(tokens, transforms_word_idx.checked_add(2)?)
}

pub fn parse_trigger_or_split(tokens: &[OwnedLexToken]) -> Option<TriggerOrSplit> {
    let mut input = LexStream::new(tokens);
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_trigger_or_split_lexed(input, tokens)
    })
}

pub fn parse_loyalty_ability_tail(tokens: &[OwnedLexToken]) -> Option<LoyaltyAbilityTail> {
    let view = primitives::TokenWordView::new(tokens);
    let words = view.word_refs();
    let ability = parse_trigger_clause_atom_word(&words, TriggerClauseAtom::Ability)?;
    if ability == 0 || !word_slice_has_exact(&words[..ability], "loyalty") {
        return None;
    }
    let of = parse_atom_word_from(&words, ability + 1, "of")?;
    let owner_first = view.token_index_after_words(of + 1)?;
    let owner = trim_comma_range(tokens, owner_first..tokens.len());
    (owner.start < owner.end).then_some(LoyaltyAbilityTail { owner })
}

pub fn parse_named_ability_tail(tokens: &[OwnedLexToken]) -> Option<NamedAbilityTail> {
    let words = primitives::TokenWordView::new(tokens).word_refs();
    let ability = parse_trigger_clause_atom_word(&words, TriggerClauseAtom::Ability)?;
    if ability + 1 != words.len() {
        return None;
    }
    if parse_last_possessive_word(&words[..ability]).is_some() {
        return None;
    }

    let marker_start = usize::from(matches!(words.first().copied(), Some("a" | "an")));
    let marker_words = words.get(marker_start..ability)?;
    if marker_words.is_empty()
        || crate::word_primitives::parse_any_sequence_complete(
            marker_words,
            &[&["activated"], &["loyalty"], &["mana"], &["triggered"]],
        )
    {
        return None;
    }

    Some(NamedAbilityTail {
        marker: marker_words.join(" "),
    })
}

pub fn parse_possessive_ability_tail(tokens: &[OwnedLexToken]) -> Option<PossessiveAbilityTail> {
    let ability_token = parse_trigger_clause_atom_token(tokens, TriggerClauseAtom::Ability)?;
    if ability_token == 0 || ability_token + 1 != tokens.len() {
        return None;
    }
    let view = primitives::TokenWordView::new(tokens);
    let words = view.word_refs();
    let ability_word = parse_trigger_clause_atom_word(&words, TriggerClauseAtom::Ability)?;
    let possessive_word = parse_last_possessive_word(&words[..ability_word])?;
    let owner_end = view.token_index_after_words(possessive_word + 1)?;
    let marker = words
        .get(possessive_word + 1)
        .filter(|_| possessive_word + 1 < ability_word)
        .map(|word| (*word).to_string());
    Some(PossessiveAbilityTail {
        owner: 0..owner_end,
        marker,
    })
}

pub fn parse_ability_of_object_tail(tokens: &[OwnedLexToken]) -> Option<AbilityOfObjectTail> {
    let view = primitives::TokenWordView::new(tokens);
    let words = view.word_refs();
    let ability = parse_trigger_clause_atom_word(&words, TriggerClauseAtom::Ability)?;
    let of = parse_atom_word_from(&words, ability + 1, "of")?;
    let first = view.token_index_after_words(of + 1)?;
    let that = parse_atom_word_from(&words, of + 1, "that");
    let chosen_type_reference = that.is_some_and(|that| {
        that > of + 1
            && words.get(that.saturating_sub(1)).copied() == Some("of")
            && words.get(that + 1).copied() == Some("type")
    });
    let end_word = that.map(|that| {
        if chosen_type_reference {
            that.saturating_sub(1)
        } else {
            that
        }
    });
    let end = end_word
        .and_then(|word| view.token_start_indices().get(word).copied())
        .unwrap_or(tokens.len());
    let filter = trim_comma_range(tokens, first..end);
    if filter.start >= filter.end {
        return None;
    }
    Some(AbilityOfObjectTail {
        filter,
        non_mana_only: parse_trigger_clause_atom_word(&words, TriggerClauseAtom::Mana).is_some(),
        chosen_type_reference,
    })
}

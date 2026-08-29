//! Typed structural grammar for negated restriction clauses.
//!
//! These facts decide routing and clause expansion only. Semantic object and
//! player filters remain owned by their dedicated grammar modules.

use winnow::combinator::{alt, not, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::{activation_restrictions, primitives};
use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MultiSentenceCantDecline<'a> {
    pub first_sentence_tokens: &'a [OwnedLexToken],
    pub remaining_sentence_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectTemporaryCastSubject {
    YourOpponents,
    EachOpponent,
    EachPlayer,
    Players,
    TargetPlayer,
    You,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectTemporaryCastDecline<'a> {
    pub subject: DirectTemporaryCastSubject,
    pub subject_tokens: &'a [OwnedLexToken],
    pub negation_tokens: &'a [OwnedLexToken],
    pub spell_descriptor_tokens: &'a [OwnedLexToken],
    pub duration_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IteratedPlayerLead {
    Each,
    ForEach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IteratedPlayerScope {
    Player,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IteratedPlayerWhoDecline<'a> {
    pub lead: IteratedPlayerLead,
    pub player: IteratedPlayerScope,
    pub prefix_tokens: &'a [OwnedLexToken],
    pub predicate_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeadingIfCantDecline<'a> {
    pub if_tokens: &'a [OwnedLexToken],
    pub clause_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatModifierVerb {
    Get,
    Gets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatModifierConjunctionDecline<'a> {
    pub verb: StatModifierVerb,
    pub subject_tokens: &'a [OwnedLexToken],
    pub modifier_tokens: &'a [OwnedLexToken],
    pub negation_tokens: &'a [OwnedLexToken],
    pub negated_action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CantConjunctionExpansion {
    pub negated_anchor: usize,
    pub segments: Vec<Vec<OwnedLexToken>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCantSubject {
    This,
    ThisCreature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericNegatedCantAction<'a> {
    SourceBlocksAttacker {
        source: SourceCantSubject,
        subject_tokens: &'a [OwnedLexToken],
        negation_tokens: &'a [OwnedLexToken],
        attacker_tokens: &'a [OwnedLexToken],
    },
    SubjectCantTransform {
        subject_tokens: &'a [OwnedLexToken],
        negation_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NegatedUntapRemainder<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub negation_tokens: &'a [OwnedLexToken],
    pub pre_untap_tokens: &'a [OwnedLexToken],
    pub post_untap_tokens: &'a [OwnedLexToken],
}

pub fn parse_multi_sentence_cant_decline_tokens(
    tokens: &[OwnedLexToken],
) -> Option<MultiSentenceCantDecline<'_>> {
    primitives::parse_all(
        tokens,
        parse_multi_sentence_cant_decline_lexed,
        "multi-sentence cant decline",
    )
    .ok()
}

pub fn parse_direct_temporary_cast_decline_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DirectTemporaryCastDecline<'_>> {
    let negation = activation_restrictions::parse_activation_negation_span_tokens(tokens)?;
    let subject_tokens = trim_lexed_commas(tokens.get(..negation.first)?);
    let subject = primitives::parse_all(
        subject_tokens,
        parse_direct_temporary_cast_subject_lexed,
        "temporary cast subject",
    )
    .ok()?;
    let tail = primitives::parse_all(
        tokens.get(negation.end..)?,
        parse_direct_temporary_cast_tail_lexed,
        "temporary cast tail",
    )
    .ok()?;
    Some(DirectTemporaryCastDecline {
        subject,
        subject_tokens,
        negation_tokens: tokens.get(negation.first..negation.end)?,
        spell_descriptor_tokens: tail.spell_descriptor_tokens,
        duration_tokens: tail.duration_tokens,
    })
}

pub fn parse_iterated_player_who_decline_tokens(
    tokens: &[OwnedLexToken],
) -> Option<IteratedPlayerWhoDecline<'_>> {
    primitives::parse_all(
        tokens,
        parse_iterated_player_who_decline_lexed,
        "iterated player-who decline",
    )
    .ok()
}

pub fn parse_leading_if_cant_decline_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeadingIfCantDecline<'_>> {
    primitives::parse_all(
        tokens,
        parse_leading_if_cant_decline_lexed,
        "leading-if cant decline",
    )
    .ok()
}

pub fn parse_stat_modifier_conjunction_decline_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StatModifierConjunctionDecline<'_>> {
    let negation = activation_restrictions::parse_activation_negation_span_tokens(tokens)?;
    let parsed = primitives::parse_all(
        tokens.get(..negation.first)?,
        parse_stat_modifier_prefix_lexed,
        "stat-modifier cant prefix",
    )
    .ok()?;
    Some(StatModifierConjunctionDecline {
        verb: parsed.verb,
        subject_tokens: parsed.subject_tokens,
        modifier_tokens: parsed.modifier_tokens,
        negation_tokens: tokens.get(negation.first..negation.end)?,
        negated_action_tokens: tokens.get(negation.end..)?,
    })
}

pub fn parse_cant_conjunction_expansion_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CantConjunctionExpansion> {
    let segments = primitives::split_lexed_slices_on_and(tokens);
    if segments.len() < 2 {
        return None;
    }

    let mut negated_anchor = None;
    for (index, segment) in segments.iter().enumerate() {
        if activation_restrictions::parse_activation_negation_span_tokens(segment).is_some() {
            negated_anchor = Some(index);
            break;
        }
    }
    let negated_anchor = negated_anchor?;
    // "Creature and enchantment spells you control can't ..." conjoins type
    // adjectives inside one noun phrase; distributing the negated tail over
    // the bare adjective would invent a battlefield-wide restriction. Leave
    // such clauses whole for the subject-filter parse.
    for segment in &segments[..negated_anchor] {
        let trimmed = trim_lexed_commas(segment);
        let pure_type_nouns = !trimmed.is_empty()
            && trimmed.iter().all(|token| {
                token.kind == crate::lexer::TokenKind::Comma
                    || token.as_word().is_some_and(|word| {
                        let singular =
                            crate::string_primitives::strip_suffix_char(word, 's').unwrap_or(word);
                        crate::util::parse_card_type(word).is_some()
                            || crate::util::parse_card_type(singular).is_some()
                            || crate::util::parse_subtype_flexible(word).is_some()
                    })
            });
        if pure_type_nouns {
            return None;
        }
    }
    let anchor_negation =
        activation_restrictions::parse_activation_negation_span_tokens(segments[negated_anchor])?;
    let shared_negated_tail = segments[negated_anchor]
        .get(anchor_negation.first..)?
        .to_vec();
    let shared_subject =
        activation_restrictions::parse_activation_negation_span_tokens(segments[0])
            .and_then(|span| segments[0].get(..span.first))
            .map(trim_lexed_commas)
            .unwrap_or_default();

    let mut expanded_segments = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let negation = activation_restrictions::parse_activation_negation_span_tokens(segment);
        let mut expanded = segment.to_vec();
        match negation {
            None if index < negated_anchor => {
                expanded.extend(shared_negated_tail.iter().cloned());
            }
            // A negation-less trailing segment of bare type/subtype nouns is
            // a continuation of the anchor's own object list ("... except by
            // A, B, and C"); dropping it silently loses that list arm —
            // decline the expansion so the clause parses whole. Verb-phrase
            // tails keep the legacy behavior.
            None => {
                let noun_continuation = !segment.is_empty()
                    && segment.iter().all(|token| {
                        let Some(word) = token.as_word() else {
                            // Punctuation inside or ending the list arm.
                            return true;
                        };
                        let singular =
                            crate::string_primitives::strip_suffix_char(word, 's').unwrap_or(word);
                        crate::util::parse_card_type(word).is_some()
                            || crate::util::parse_card_type(singular).is_some()
                            || crate::util::parse_subtype_flexible(word).is_some()
                    });
                if noun_continuation {
                    return None;
                }
                continue;
            }
            Some(span) if index > 0 && !shared_subject.is_empty() && span.first == 0 => {
                expanded = shared_subject.to_vec();
                expanded.extend(segment.iter().cloned());
            }
            Some(span) if index > 0 && !shared_subject.is_empty() => {
                let inherited_subject = trim_lexed_commas(&segment[..span.first]);
                let inherited_pronoun = matches!(
                    inherited_subject,
                    [token] if token.is_word("it") || token.is_word("they")
                );
                if inherited_pronoun {
                    expanded = shared_subject.to_vec();
                    expanded.extend(segment[span.first..].iter().cloned());
                } else if primitives::parse_prefix(
                    segment,
                    parse_possessive_activated_subject_lexed,
                )
                .is_some()
                {
                    expanded = shared_subject.to_vec();
                    expanded.extend(segment.iter().skip(1).cloned());
                }
            }
            Some(_) => {}
        }
        expanded_segments.push(expanded);
    }

    if expanded_segments.is_empty() {
        return None;
    }
    Some(CantConjunctionExpansion {
        negated_anchor,
        segments: expanded_segments,
    })
}

pub fn parse_generic_negated_cant_action_tokens(
    tokens: &[OwnedLexToken],
) -> Option<GenericNegatedCantAction<'_>> {
    let negation = activation_restrictions::parse_activation_negation_span_tokens(tokens)?;
    let subject_tokens = trim_lexed_commas(tokens.get(..negation.first)?);
    let negation_tokens = tokens.get(negation.first..negation.end)?;
    let tail_tokens = tokens.get(negation.end..)?;

    if let Ok(source) = primitives::parse_all(
        subject_tokens,
        parse_source_cant_subject_lexed,
        "source cant subject",
    ) && let Ok(attacker_tokens) = primitives::parse_all(
        tail_tokens,
        parse_source_blocks_attacker_tail_lexed,
        "source blocks attacker tail",
    ) {
        return Some(GenericNegatedCantAction::SourceBlocksAttacker {
            source,
            subject_tokens,
            negation_tokens,
            attacker_tokens,
        });
    }

    primitives::parse_all(
        tail_tokens,
        parse_transform_tail_lexed,
        "cant transform tail",
    )
    .ok()?;
    if subject_tokens.is_empty() {
        return None;
    }
    Some(GenericNegatedCantAction::SubjectCantTransform {
        subject_tokens,
        negation_tokens,
    })
}

pub fn parse_negated_untap_remainder_tokens(
    tokens: &[OwnedLexToken],
) -> Option<NegatedUntapRemainder<'_>> {
    let negation = activation_restrictions::parse_activation_negation_span_tokens(tokens)?;
    let subject_tokens = trim_lexed_commas(tokens.get(..negation.first)?);
    if subject_tokens.is_empty() {
        return None;
    }
    let parsed = primitives::parse_all(
        tokens.get(negation.end..)?,
        parse_untap_tail_lexed,
        "negated untap remainder",
    )
    .ok()?;
    Some(NegatedUntapRemainder {
        subject_tokens,
        negation_tokens: tokens.get(negation.first..negation.end)?,
        pre_untap_tokens: parsed.pre_untap_tokens,
        post_untap_tokens: parsed.post_untap_tokens,
    })
}

fn parse_multi_sentence_cant_decline_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<MultiSentenceCantDecline<'a>> {
    let first_sentence_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::period().void()))
            .void()
            .take()
            .parse_next(input)?;
    primitives::period().parse_next(input)?;
    let remaining_sentence_tokens = (
        repeat_till::<_, _, (), _, _, _, _>(
            0..,
            any.void(),
            peek(primitives::word_parser_text.void()),
        )
        .void(),
        primitives::word_parser_text.void(),
        repeat::<_, _, (), _, _>(0.., any.void()),
    )
        .void()
        .take()
        .parse_next(input)?;
    Ok(MultiSentenceCantDecline {
        first_sentence_tokens,
        remaining_sentence_tokens,
    })
}

fn parse_direct_temporary_cast_subject_lexed(
    input: &mut LexStream<'_>,
) -> WResult<DirectTemporaryCastSubject> {
    alt((
        primitives::phrase(&["your", "opponents"]).value(DirectTemporaryCastSubject::YourOpponents),
        primitives::phrase(&["each", "opponent"]).value(DirectTemporaryCastSubject::EachOpponent),
        primitives::phrase(&["each", "player"]).value(DirectTemporaryCastSubject::EachPlayer),
        primitives::kw("players").value(DirectTemporaryCastSubject::Players),
        primitives::phrase(&["target", "player"]).value(DirectTemporaryCastSubject::TargetPlayer),
        primitives::kw("you").value(DirectTemporaryCastSubject::You),
    ))
    .parse_next(input)
}

#[derive(Debug, Clone, Copy)]
struct DirectTemporaryCastTail<'a> {
    spell_descriptor_tokens: &'a [OwnedLexToken],
    duration_tokens: &'a [OwnedLexToken],
}

fn parse_direct_temporary_cast_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DirectTemporaryCastTail<'a>> {
    primitives::kw("cast").parse_next(input)?;
    let spell_descriptor_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        parse_temporary_cast_descriptor_token_lexed,
        peek(primitives::phrase(&["this", "turn"]).void()),
    )
    .void()
    .take()
    .parse_next(input)?;
    let duration_tokens = primitives::phrase(&["this", "turn"])
        .void()
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(DirectTemporaryCastTail {
        spell_descriptor_tokens,
        duration_tokens,
    })
}

fn parse_temporary_cast_descriptor_token_lexed(input: &mut LexStream<'_>) -> WResult<()> {
    not(alt((primitives::kw("unless"), primitives::kw("who")))).parse_next(input)?;
    any.void().parse_next(input)
}

fn parse_iterated_player_who_decline_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<IteratedPlayerWhoDecline<'a>> {
    let ((lead, player), prefix_tokens) = (
        parse_iterated_player_lead_lexed,
        parse_iterated_player_scope_lexed,
        primitives::kw("who"),
    )
        .map(|(lead, player, _)| (lead, player))
        .with_taken()
        .parse_next(input)?;
    let predicate_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .void()
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(IteratedPlayerWhoDecline {
        lead,
        player,
        prefix_tokens,
        predicate_tokens,
    })
}

fn parse_iterated_player_lead_lexed(input: &mut LexStream<'_>) -> WResult<IteratedPlayerLead> {
    alt((
        primitives::phrase(&["for", "each"]).value(IteratedPlayerLead::ForEach),
        primitives::kw("each").value(IteratedPlayerLead::Each),
    ))
    .parse_next(input)
}

fn parse_iterated_player_scope_lexed(input: &mut LexStream<'_>) -> WResult<IteratedPlayerScope> {
    alt((
        alt((primitives::kw("opponent"), primitives::kw("opponents")))
            .value(IteratedPlayerScope::Opponent),
        alt((primitives::kw("player"), primitives::kw("players")))
            .value(IteratedPlayerScope::Player),
    ))
    .parse_next(input)
}

fn parse_leading_if_cant_decline_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LeadingIfCantDecline<'a>> {
    let if_tokens = primitives::kw("if").void().take().parse_next(input)?;
    let clause_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .void()
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(LeadingIfCantDecline {
        if_tokens,
        clause_tokens,
    })
}

#[derive(Debug, Clone, Copy)]
struct ParsedStatModifierPrefix<'a> {
    verb: StatModifierVerb,
    subject_tokens: &'a [OwnedLexToken],
    modifier_tokens: &'a [OwnedLexToken],
}

fn parse_stat_modifier_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ParsedStatModifierPrefix<'a>> {
    let subject_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_stat_modifier_verb_lexed))
            .void()
            .take()
            .parse_next(input)?;
    let verb = parse_stat_modifier_verb_lexed.parse_next(input)?;
    let modifier_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((opt(primitives::comma()), primitives::kw("and")).void()),
    )
    .void()
    .take()
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    Ok(ParsedStatModifierPrefix {
        verb,
        subject_tokens,
        modifier_tokens,
    })
}

fn parse_stat_modifier_verb_lexed(input: &mut LexStream<'_>) -> WResult<StatModifierVerb> {
    alt((
        primitives::kw("gets").value(StatModifierVerb::Gets),
        primitives::kw("get").value(StatModifierVerb::Get),
    ))
    .parse_next(input)
}

fn parse_possessive_activated_subject_lexed(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("its"), primitives::kw("their"))).parse_next(input)?;
    primitives::phrase(&["activated", "abilities"])
        .void()
        .parse_next(input)
}

fn parse_source_cant_subject_lexed(input: &mut LexStream<'_>) -> WResult<SourceCantSubject> {
    alt((
        primitives::phrase(&["this", "creature"]).value(SourceCantSubject::ThisCreature),
        primitives::kw("this").value(SourceCantSubject::This),
    ))
    .parse_next(input)
}

fn parse_source_blocks_attacker_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    primitives::kw("block").parse_next(input)?;
    let attacker_tokens = (
        primitives::word_parser_text.void(),
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::sentence_end()))
            .void(),
    )
        .void()
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(attacker_tokens)
}

fn parse_transform_tail_lexed(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("transform").parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

#[derive(Debug, Clone, Copy)]
struct ParsedUntapTail<'a> {
    pre_untap_tokens: &'a [OwnedLexToken],
    post_untap_tokens: &'a [OwnedLexToken],
}

fn parse_untap_tail_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ParsedUntapTail<'a>> {
    let pre_untap_tokens =
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::kw("untap").void()))
            .void()
            .take()
            .parse_next(input)?;
    primitives::kw("untap").parse_next(input)?;
    let post_untap_tokens =
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::sentence_end()))
            .void()
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ParsedUntapTail {
        pre_untap_tokens,
        post_untap_tokens,
    })
}

#[cfg(test)]
#[path = "structure_inline_tests.rs"]
mod tests;

//! Typed structural grammar for negated restriction clauses.
//!
//! These facts decide routing and clause expansion only. Semantic object and
//! player filters remain owned by their dedicated grammar modules.

use winnow::combinator::{alt, not, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::runtime_backend::front_end::grammar::{activation_restrictions, primitives};
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MultiSentenceCantDecline<'a> {
    pub(crate) first_sentence_tokens: &'a [OwnedLexToken],
    pub(crate) remaining_sentence_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectTemporaryCastSubject {
    YourOpponents,
    EachOpponent,
    EachPlayer,
    Players,
    TargetPlayer,
    You,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirectTemporaryCastDecline<'a> {
    pub(crate) subject: DirectTemporaryCastSubject,
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) negation_tokens: &'a [OwnedLexToken],
    pub(crate) spell_descriptor_tokens: &'a [OwnedLexToken],
    pub(crate) duration_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IteratedPlayerLead {
    Each,
    ForEach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IteratedPlayerScope {
    Player,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IteratedPlayerWhoDecline<'a> {
    pub(crate) lead: IteratedPlayerLead,
    pub(crate) player: IteratedPlayerScope,
    pub(crate) prefix_tokens: &'a [OwnedLexToken],
    pub(crate) predicate_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeadingIfCantDecline<'a> {
    pub(crate) if_tokens: &'a [OwnedLexToken],
    pub(crate) clause_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatModifierVerb {
    Get,
    Gets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatModifierConjunctionDecline<'a> {
    pub(crate) verb: StatModifierVerb,
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) modifier_tokens: &'a [OwnedLexToken],
    pub(crate) negation_tokens: &'a [OwnedLexToken],
    pub(crate) negated_action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CantConjunctionExpansion {
    pub(crate) negated_anchor: usize,
    pub(crate) segments: Vec<Vec<OwnedLexToken>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceCantSubject {
    This,
    ThisCreature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenericNegatedCantAction<'a> {
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
pub(crate) struct NegatedUntapRemainder<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) negation_tokens: &'a [OwnedLexToken],
    pub(crate) pre_untap_tokens: &'a [OwnedLexToken],
    pub(crate) post_untap_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_multi_sentence_cant_decline_tokens(
    tokens: &[OwnedLexToken],
) -> Option<MultiSentenceCantDecline<'_>> {
    primitives::parse_all(
        tokens,
        parse_multi_sentence_cant_decline_lexed,
        "multi-sentence cant decline",
    )
    .ok()
}

pub(crate) fn parse_direct_temporary_cast_decline_tokens(
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

pub(crate) fn parse_iterated_player_who_decline_tokens(
    tokens: &[OwnedLexToken],
) -> Option<IteratedPlayerWhoDecline<'_>> {
    primitives::parse_all(
        tokens,
        parse_iterated_player_who_decline_lexed,
        "iterated player-who decline",
    )
    .ok()
}

pub(crate) fn parse_leading_if_cant_decline_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeadingIfCantDecline<'_>> {
    primitives::parse_all(
        tokens,
        parse_leading_if_cant_decline_lexed,
        "leading-if cant decline",
    )
    .ok()
}

pub(crate) fn parse_stat_modifier_conjunction_decline_tokens(
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

pub(crate) fn parse_cant_conjunction_expansion_tokens(
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
                token.kind == crate::runtime_backend::front_end::lexer::TokenKind::Comma
                || token.as_word().is_some_and(|word| {
                    let singular = word.strip_suffix('s').unwrap_or(word);
                    crate::runtime_backend::front_end::shared::util::parse_card_type(word).is_some()
                        || crate::runtime_backend::front_end::shared::util::parse_card_type(
                            singular,
                        )
                        .is_some()
                        || crate::runtime_backend::front_end::shared::util::parse_subtype_flexible(
                            word,
                        )
                        .is_some()
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
                let noun_continuation = !segment.is_empty() && segment.iter().all(|token| {
                    let Some(word) = token.as_word() else {
                        // Punctuation inside or ending the list arm.
                        return true;
                    };
                    let singular = word.strip_suffix('s').unwrap_or(word);
                    crate::runtime_backend::front_end::shared::util::parse_card_type(word).is_some()
                        || crate::runtime_backend::front_end::shared::util::parse_card_type(
                            singular,
                        )
                        .is_some()
                        || crate::runtime_backend::front_end::shared::util::parse_subtype_flexible(
                            word,
                        )
                        .is_some()
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

pub(crate) fn parse_generic_negated_cant_action_tokens(
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

pub(crate) fn parse_negated_untap_remainder_tokens(
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
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{lex_line, parser_token_word_refs};

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).expect("lex cant structure fixture")
    }

    #[test]
    fn captures_multi_sentence_decline_boundaries() {
        let tokens = lex("Damage can't be prevented this turn. This deals 2 damage to any target.");
        let parsed = parse_multi_sentence_cant_decline_tokens(&tokens).unwrap();
        assert_eq!(
            parser_token_word_refs(parsed.first_sentence_tokens),
            ["damage", "cant", "be", "prevented", "this", "turn"]
        );
        assert_eq!(
            parser_token_word_refs(parsed.remaining_sentence_tokens),
            ["this", "deals", "2", "damage", "to", "any", "target"]
        );
        assert!(
            parse_multi_sentence_cant_decline_tokens(&lex("Players can't gain life.")).is_none()
        );
    }

    #[test]
    fn captures_direct_temporary_cast_decline_table() {
        let cases = [
            (
                "Your opponents can't cast spells this turn.",
                DirectTemporaryCastSubject::YourOpponents,
            ),
            (
                "Each opponent cannot cast creature spells this turn.",
                DirectTemporaryCastSubject::EachOpponent,
            ),
            (
                "Each player can't cast more than one spell this turn.",
                DirectTemporaryCastSubject::EachPlayer,
            ),
            (
                "Players can't cast spells this turn.",
                DirectTemporaryCastSubject::Players,
            ),
            (
                "Target player can't cast noncreature spells this turn.",
                DirectTemporaryCastSubject::TargetPlayer,
            ),
            (
                "You can't cast spells from exile this turn.",
                DirectTemporaryCastSubject::You,
            ),
        ];
        for (raw, expected) in cases {
            let tokens = lex(raw);
            let parsed = parse_direct_temporary_cast_decline_tokens(&tokens)
                .unwrap_or_else(|| panic!("fixture did not parse: {raw}"));
            assert_eq!(parsed.subject, expected, "fixture: {raw}");
            assert_eq!(
                parser_token_word_refs(parsed.duration_tokens),
                ["this", "turn"]
            );
            assert!(!parsed.spell_descriptor_tokens.is_empty());
        }
        for raw in [
            "Your opponents can't cast spells.",
            "Your opponents can't cast spells unless they pay {2} this turn.",
            "Each opponent who lost life can't cast spells this turn.",
            "Creatures can't cast spells this turn.",
        ] {
            assert!(
                parse_direct_temporary_cast_decline_tokens(&lex(raw)).is_none(),
                "near miss: {raw}"
            );
        }
    }

    #[test]
    fn captures_iterated_player_and_leading_if_declines() {
        let cases = [
            (
                "Each opponent who lost life can't block.",
                IteratedPlayerLead::Each,
                IteratedPlayerScope::Opponent,
            ),
            (
                "Each players who drew a card can't attack.",
                IteratedPlayerLead::Each,
                IteratedPlayerScope::Player,
            ),
            (
                "For each opponent who does, draw a card.",
                IteratedPlayerLead::ForEach,
                IteratedPlayerScope::Opponent,
            ),
            (
                "For each player who discarded, create a token.",
                IteratedPlayerLead::ForEach,
                IteratedPlayerScope::Player,
            ),
        ];
        for (raw, lead, player) in cases {
            let tokens = lex(raw);
            let parsed = parse_iterated_player_who_decline_tokens(&tokens)
                .unwrap_or_else(|| panic!("fixture did not parse: {raw}"));
            assert_eq!((parsed.lead, parsed.player), (lead, player));
            assert!(!parsed.predicate_tokens.is_empty());
        }
        let tokens = lex("If a creature would attack, it can't block.");
        let parsed = parse_leading_if_cant_decline_tokens(&tokens).unwrap();
        assert_eq!(parser_token_word_refs(parsed.if_tokens), ["if"]);
        assert!(parse_leading_if_cant_decline_tokens(&lex("Players can't gain life.")).is_none());
    }

    #[test]
    fn captures_stat_modifier_conjunction_decline() {
        let tokens = lex("Enchanted creature gets +2/+2 and can't be blocked.");
        let parsed = parse_stat_modifier_conjunction_decline_tokens(&tokens).unwrap();
        assert_eq!(parsed.verb, StatModifierVerb::Gets);
        assert_eq!(
            parser_token_word_refs(parsed.subject_tokens),
            ["enchanted", "creature"]
        );
        assert_eq!(parser_token_word_refs(parsed.modifier_tokens), ["+2/+2"]);
        assert_eq!(parser_token_word_refs(parsed.negation_tokens), ["cant"]);
        for raw in [
            "Enchanted creature gets +2/+2.",
            "Enchanted creature has flying and can't be blocked.",
            "Enchanted creature can't be blocked and gets +2/+2.",
        ] {
            assert!(
                parse_stat_modifier_conjunction_decline_tokens(&lex(raw)).is_none(),
                "near miss: {raw}"
            );
        }
    }

    #[test]
    fn expands_inherited_negation_and_subject_conjunctions() {
        let tokens = lex("Creatures you control and artifacts you control can't be sacrificed.");
        let expanded = parse_cant_conjunction_expansion_tokens(&tokens).unwrap();
        assert_eq!(expanded.negated_anchor, 1);
        assert_eq!(expanded.segments.len(), 2);
        assert_eq!(
            parser_token_word_refs(&expanded.segments[0]),
            ["creatures", "you", "control", "cant", "be", "sacrificed"]
        );

        let tokens = lex("Players can't gain life and can't search libraries.");
        let expanded = parse_cant_conjunction_expansion_tokens(&tokens).unwrap();
        assert_eq!(expanded.negated_anchor, 0);
        assert_eq!(
            parser_token_word_refs(&expanded.segments[1]),
            ["players", "cant", "search", "libraries"]
        );

        let tokens = lex("Players can't gain life and draw cards.");
        let expanded = parse_cant_conjunction_expansion_tokens(&tokens).unwrap();
        assert_eq!(expanded.segments.len(), 1);

        let inherited_subject_cases: &[(&str, &[&str])] = &[
            (
                "Creatures your opponents control can't block, and they can't attack you.",
                &["creatures", "your", "opponents", "control"],
            ),
            (
                "This creature can't block, and it can't attack.",
                &["this", "creature"],
            ),
        ];
        for (text, expected_subject) in inherited_subject_cases {
            let expanded = parse_cant_conjunction_expansion_tokens(&lex(text)).unwrap();
            assert_eq!(expanded.segments.len(), 2, "{text}");
            let second = parser_token_word_refs(&expanded.segments[1]);
            let expected = expected_subject
                .iter()
                .copied()
                .chain(["cant", "attack"])
                .collect::<Vec<_>>();
            assert!(
                second.starts_with(&expected),
                "inherited subject was not expanded for {text}: {second:?}"
            );
        }
    }

    #[test]
    fn captures_generic_block_transform_and_untap_actions() {
        let block = lex("This creature can't block creatures with flying.");
        let GenericNegatedCantAction::SourceBlocksAttacker {
            source,
            attacker_tokens,
            ..
        } = parse_generic_negated_cant_action_tokens(&block).unwrap()
        else {
            panic!("expected source-block action");
        };
        assert_eq!(source, SourceCantSubject::ThisCreature);
        assert_eq!(
            parser_token_word_refs(attacker_tokens),
            ["creatures", "with", "flying"]
        );

        let transform = lex("Non-Human creatures can't transform.");
        let GenericNegatedCantAction::SubjectCantTransform { subject_tokens, .. } =
            parse_generic_negated_cant_action_tokens(&transform).unwrap()
        else {
            panic!("expected transform action");
        };
        assert_eq!(
            parser_token_word_refs(subject_tokens),
            ["non", "human", "creatures"]
        );

        let untap = lex("Target creature can't untap during its controller's next untap step.");
        let parsed = parse_negated_untap_remainder_tokens(&untap).unwrap();
        assert_eq!(
            parser_token_word_refs(parsed.subject_tokens),
            ["target", "creature"]
        );
        assert_eq!(
            parser_token_word_refs(parsed.post_untap_tokens),
            ["during", "its", "controllers", "next", "untap", "step"]
        );

        for raw in [
            "This creature can't block.",
            "This creature can block creatures with flying.",
            "Non-Human creatures can transform.",
            "Target creature can't attack.",
        ] {
            assert!(
                parse_generic_negated_cant_action_tokens(&lex(raw)).is_none(),
                "near miss: {raw}"
            );
        }
    }
}

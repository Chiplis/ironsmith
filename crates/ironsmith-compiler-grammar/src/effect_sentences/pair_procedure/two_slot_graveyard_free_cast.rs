//! "You may cast up to one target instant card and/or up to one target
//! sorcery card from your graveyard each with mana value X or less without
//! paying their mana costs. If a spell cast this way would be put into your
//! graveyard, exile it instead. If X is 10 or more, copy each of those spells
//! twice. You may choose new targets for the copies." (Finale of Promise)
//!
//! Two independent optional target slots read by one instruction: each
//! chosen card may be cast for free, each spell cast this way is exiled
//! instead of going to the graveyard, and only after both casts (so both
//! spells are on the stack) is each of those spells copied.
use super::*;
use crate::cards::builders::{
    ConditionalEffectAst, FutureZoneReplacementCausePolicyAst, PermissionEffectAst, TextSpan,
    ZoneReplacementDurationAst,
};
use crate::lexer::OwnedLexToken;
use crate::target::TaggedOpbjectRelation;

fn word_tokens(sentence: &SentenceInput) -> Vec<OwnedLexToken> {
    sentence
        .lowered()
        .iter()
        .filter(|token| token.as_word().is_some())
        .cloned()
        .collect()
}

fn words_of(tokens: &[OwnedLexToken]) -> Vec<&str> {
    tokens.iter().filter_map(OwnedLexToken::as_word).collect()
}

/// One "up to one target <types> card" slot: the card-type words.
fn slot(words: &[&str]) -> Option<(usize, usize)> {
    let rest = words.strip_prefix(&["up", "to", "one", "target"][..])?;
    let card = rest.iter().position(|word| *word == "card")?;
    (card > 0).then_some((4, 4 + card))
}

struct TwoSlots {
    /// Token ranges of each slot's type words.
    slots: [(usize, usize); 2],
    /// Token range of the shared qualifier after "each" ("with mana value X
    /// or less"), possibly empty.
    shared: (usize, usize),
    /// Token range of the shared origin ("from your graveyard").
    origin: (usize, usize),
}

fn two_slot_cast(words: &[&str]) -> Option<TwoSlots> {
    let rest = words.strip_prefix(&["you", "may", "cast"][..])?;
    let offset = 3;
    let (first_start, first_end) = slot(rest)?;
    let after_first = &rest[first_end + 1..];
    let conjunction = match after_first {
        ["and", "or", ..] => 2,
        ["and/or" | "and", ..] => 1,
        _ => return None,
    };
    let second_rest = &after_first[conjunction..];
    let (second_start, second_end) = slot(second_rest)?;
    let second_base = offset + first_end + 1 + conjunction;
    let after_second = &second_rest[second_end + 1..];
    let tail_base = second_base + second_end + 1;
    let origin_len = match after_second {
        ["from", "your", "graveyard", ..] => 3,
        _ => return None,
    };
    let after_origin = &after_second[origin_len..];
    let without = after_origin
        .windows(5)
        .position(|window| window == ["without", "paying", "their", "mana", "costs"])?;
    if without + 5 != after_origin.len() {
        return None;
    }
    let shared = match &after_origin[..without] {
        [] => (0, 0),
        ["each", qualifier @ ..] if !qualifier.is_empty() => {
            let start = tail_base + origin_len + 1;
            (start, start + qualifier.len())
        }
        _ => return None,
    };
    Some(TwoSlots {
        slots: [
            (offset + first_start, offset + first_end),
            (second_base + second_start, second_base + second_end),
        ],
        shared,
        origin: (tail_base, tail_base + origin_len),
    })
}

fn copy_threshold(words: &[&str]) -> Option<(u32, u32)> {
    let rest = words.strip_prefix(&["if", "x", "is"][..])?;
    let (threshold, rest) = rest.split_first()?;
    let threshold = threshold.parse::<u32>().ok()?;
    let rest = rest.strip_prefix(&["or", "more"][..])?;
    let count = match rest {
        ["copy", "each", "of", "those", "spells", "twice"] => 2,
        ["copy", "each", "of", "those", "spells"] => 1,
        ["copy", "each", "of", "those", "spells", "three", "times"] => 3,
        _ => return None,
    };
    Some((threshold, count))
}

pub(super) fn read(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(rider), Some(copy), Some(retarget)) = (
        sentences.get(index),
        sentences.get(index + 1),
        sentences.get(index + 2),
        sentences.get(index + 3),
    ) else {
        return Ok(None);
    };
    let tokens = word_tokens(first);
    let words = words_of(&tokens);
    let Some(shape) = two_slot_cast(&words) else {
        return Ok(None);
    };
    if !matches!(
        words_of(&word_tokens(rider)).as_slice(),
        [
            "if", "a", "spell", "cast", "this", "way", "would", "be", "put", "into", "your",
            "graveyard", "exile", "it", "instead"
        ]
    ) {
        return Ok(None);
    }
    let Some((threshold, copies)) = copy_threshold(&words_of(&word_tokens(copy))) else {
        return Ok(None);
    };
    if !matches!(
        words_of(&word_tokens(retarget)).as_slice(),
        ["you", "may", "choose", "new", "targets", "for", "the", "copies"]
    ) {
        return Ok(None);
    }

    let mut effects = Vec::new();
    let mut casts = Vec::new();
    let mut cast_tags = Vec::new();
    for (slot_index, (start, end)) in shape.slots.into_iter().enumerate() {
        // "<types> card <shared qualifier> from your graveyard".
        let mut filter_tokens = tokens[start..=end].to_vec();
        filter_tokens.extend_from_slice(&tokens[shape.shared.0..shape.shared.1]);
        filter_tokens.extend_from_slice(&tokens[shape.origin.0..shape.origin.1]);
        let Ok(filter) = crate::object_filters::parse_object_filter(&filter_tokens, false) else {
            return Ok(None);
        };
        if filter.zone != Some(Zone::Graveyard) || filter.card_types.is_empty() {
            return Ok(None);
        }
        let slot_tokens = &tokens[start..=end];
        let (target_prefix, cast_prefix) = if slot_index == 0 {
            ("two_slot_cast_first_target", "two_slot_cast_first_spell")
        } else {
            ("two_slot_cast_second_target", "two_slot_cast_second_spell")
        };
        let chosen_tag = helper_tag_for_tokens(slot_tokens, target_prefix);
        let cast_spell_tag = helper_tag_for_tokens(slot_tokens, cast_prefix);
        effects.push(EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_target_only(TargetAst::WithCount(
                Box::new(TargetAst::Object(filter, Some(TextSpan::synthetic()), None)),
                ChoiceCount::up_to(1),
            ))),
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
        });
        let replacement_filter = ObjectFilter::spell().match_tagged(
            cast_spell_tag.clone(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
        casts.push(EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects: vec![EffectAst::TagAffected {
                effect: Box::new(EffectAst::subject_verb_cast_tagged(
                    crate::tag::TagRef::of(chosen_tag),
                    PlayerAst::You,
                    false,
                    false,
                    true,
                    None,
                )),
                tag: crate::tag::TagRef::of(cast_spell_tag.clone()),
            }],
        }));
        casts.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![
                EffectAst::subject_verb_register_future_zone_replacement(
                    replacement_filter,
                    Some(Zone::Stack),
                    Some(Zone::Graveyard),
                    Zone::Exile,
                    ZoneReplacementDurationAst::OneShot,
                    FutureZoneReplacementCausePolicyAst::Any,
                    false,
                )
                .with_cast_this_way_replacement_surface(),
            ],
        }));
        cast_tags.push(cast_spell_tag);
    }
    // Both targets are chosen as the spell is cast; both casts happen before
    // either spell is copied.
    effects.extend(casts);
    effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::XValueAtLeast(threshold),
        if_true: cast_tags
            .into_iter()
            .map(|tag| {
                EffectAst::subject_verb_copy_spell(
                    TargetAst::Tagged(crate::tag::TagRef::of(tag), None),
                    Value::Fixed(copies as i32),
                    PlayerAst::You,
                    true,
                    false,
                    Vec::new(),
                )
            })
            .collect(),
        if_false: Vec::new(),
    }));
    Ok(Some(effects))
}

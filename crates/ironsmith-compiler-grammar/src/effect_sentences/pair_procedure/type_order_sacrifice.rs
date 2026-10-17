//! "Choose an order for artifacts, creatures, and lands. Each player
//! sacrifices one permanent of their choice of the first type, sacrifices two
//! of their choice of the second type, then sacrifices three of their choice
//! of the third type." (Rite of Ruin): choosing an order of N card types is a
//! choice among the N! orderings, and the second sentence's positional
//! ("first/second/third type") sacrifice counts are instantiated for the
//! chosen ordering. Each sacrifice instruction is performed by every player
//! before the next instruction begins.
use super::*;
use crate::cards::builders::{ForEachEffectAst, ObjectChoiceEffectAst};
use crate::types::CardType;

const PERMANENT_TYPES: [CardType; 6] = [
    CardType::Artifact,
    CardType::Creature,
    CardType::Enchantment,
    CardType::Land,
    CardType::Planeswalker,
    CardType::Battle,
];

fn ordinal_position(word: &str) -> Option<usize> {
    match word {
        "first" => Some(0),
        "second" => Some(1),
        "third" => Some(2),
        "fourth" => Some(3),
        "fifth" => Some(4),
        _ => None,
    }
}

fn permutations(items: &[CardType]) -> Vec<Vec<CardType>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut out = Vec::new();
    for (idx, head) in items.iter().enumerate() {
        let mut rest = items.to_vec();
        rest.remove(idx);
        for mut tail in permutations(&rest) {
            tail.insert(0, *head);
            out.push(tail);
        }
    }
    out
}

/// The ordered types named by "choose an order for <types>".
fn ordered_types(words: &[&str]) -> Option<Vec<CardType>> {
    let rest = words.strip_prefix(&["choose", "an", "order", "for"][..])?;
    let mut types = Vec::new();
    for word in rest {
        if *word == "and" {
            continue;
        }
        let card_type = PERMANENT_TYPES
            .into_iter()
            .find(|card_type| card_type.plural_name() == *word)?;
        if types.contains(&card_type) {
            return None;
        }
        types.push(card_type);
    }
    (types.len() >= 2 && types.len() <= 5).then_some(types)
}

/// "each player sacrifices <n> [permanent(s)] of their choice of the <ordinal>
/// type, [then] sacrifices <n> of their choice of the <ordinal> type, ...":
/// (count, position) per instruction, in authored order.
fn positional_sacrifices(words: &[&str]) -> Option<Vec<(u32, usize)>> {
    let mut rest = words.strip_prefix(&["each", "player"][..])?;
    let mut instructions = Vec::new();
    while !rest.is_empty() {
        if rest.first() == Some(&"then") || rest.first() == Some(&"and") {
            rest = &rest[1..];
        }
        let ["sacrifices", count, tail @ ..] = rest else {
            return None;
        };
        let count = crate::util::parse_number_word_u32(count)?;
        let tail = match tail {
            ["permanent" | "permanents", tail @ ..] => tail,
            _ => tail,
        };
        let ["of", "their", "choice", "of", "the", ordinal, "type", tail @ ..] = tail else {
            return None;
        };
        instructions.push((count, ordinal_position(ordinal)?));
        rest = tail;
    }
    (!instructions.is_empty()).then_some(instructions)
}

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let first_words = crate::lexer::token_word_refs(first.lowered());
    let Some(types) = ordered_types(&first_words) else {
        return Ok(None);
    };
    let second_words = crate::lexer::token_word_refs(second.lowered());
    let Some(instructions) = positional_sacrifices(&second_words) else {
        return Ok(None);
    };
    if instructions
        .iter()
        .any(|(_, position)| *position >= types.len())
    {
        return Err(CardTextError::ParseError(
            "type-order sacrifice names a position beyond the ordered types".to_string(),
        ));
    }
    let modes = permutations(&types)
        .into_iter()
        .map(|order| {
            let description = order
                .iter()
                .map(|card_type| card_type.plural_name())
                .collect::<Vec<_>>()
                .join(", then ");
            let effects = instructions
                .iter()
                .map(|(count, position)| {
                    let filter = ObjectFilter::default()
                        .with_type(order[*position])
                        .in_zone(Zone::Battlefield)
                        .controlled_by(PlayerFilter::IteratedPlayer);
                    EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                        effects: vec![EffectAst::subject_verb_sacrifice(
                            PlayerAst::That,
                            filter,
                            *count,
                            None,
                        )],
                    })
                })
                .collect::<Vec<_>>();
            ChooseOneModeAst {
                description,
                effects,
            }
        })
        .collect::<Vec<_>>();
    Ok(Some(vec![EffectAst::ObjectChoices(
        ObjectChoiceEffectAst::ChooseOneOf {
            chooser: PlayerFilter::You,
            modes,
        },
    )]))
}

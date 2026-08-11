use super::super::super::lexer::{parser_token_word_positions, parser_token_word_refs};
use super::reference_tag_word_facts::{
    parse_last_word_choice_before, parse_phrase_anywhere, parse_phrase_at_head,
    parse_phrase_choice_anywhere, parse_phrase_choice_at_head, parse_phrase_choice_whole,
    parse_phrase_whole, parse_word_choice, parse_word_choice_anywhere,
};
use super::*;
use crate::filter::ObjectFilterUnionConnective;
use crate::target::{ObjectCharacteristic, ObjectCharacteristicRelation};
use crate::types::SubtypeFamily;

const TARGET_OR_TARGETS_WORDS: &[&str] = &["target", "targets"];
const THAT_WORD: &str = "that";

pub(crate) fn compound_filter_subtype_prefix_word_len(words: &[&str]) -> Option<usize> {
    if words.get(..2) == Some(&["time", "lord"]) {
        return Some(2);
    }
    if parse_subtype_flexible(*words.first()?).is_some() {
        return Some(1);
    }
    if let Some(next) = words.get(1) {
        let compound = format!("{}-{next}", words[0]);
        if parse_subtype_flexible(&compound).is_some() {
            return Some(2);
        }
    }
    super::super::leaf::classify_token_definition_subtype(*words.first()?)?;
    words
        .get(1)
        .and_then(|next| parse_subtype_flexible(next))
        .map(|_| 1)
}

fn parse_compound_filter_subtype(words: &[&str], idx: usize) -> Option<Subtype> {
    if words.get(idx..idx + 2) == Some(&["time", "lord"]) {
        return Some(Subtype::TimeLord);
    }
    parse_subtype_flexible(*words.get(idx)?)
        .or_else(|| {
            let compound = format!("{}-{}", words.get(idx)?, words.get(idx + 1)?);
            parse_subtype_flexible(&compound)
        })
        .or_else(|| {
            let subtype = super::super::leaf::classify_token_definition_subtype(*words.get(idx)?)?;
            words
                .get(idx + 1)
                .and_then(|next| parse_subtype_flexible(next))
                .map(|_| subtype)
        })
}
const ONLY_WORD: &str = "only";
const SINGLE_WORD: &str = "single";
const YOU_TARGET_PREFIX: &[&str] = &["you"];
const OPPONENT_TARGET_PREFIXES: &[&[&str]] = &[&["opponent"], &["opponents"]];
const PLAYER_TARGET_PREFIXES: &[&[&str]] = &[&["player"], &["players"]];
const OR_WORD: &str = "or";
const UNTIL_WORD: &str = "until";
const OTHER_OR_ANOTHER_WORDS: &[&str] = &["other", "another"];
const OTHER_THAN_PREFIX: &[&str] = &["other", "than"];
const CHOSEN_OBJECT_EXCLUSION_PHRASES: &[&[&str]] = &[
    &["other", "than", "the", "chosen", "creature"],
    &["other", "than", "chosen", "creature"],
    &["other", "than", "the", "chosen", "permanent"],
    &["other", "than", "chosen", "permanent"],
    &["other", "than", "the", "chosen", "object"],
    &["other", "than", "chosen", "object"],
    &["and", "the", "chosen", "creature"],
    &["and", "chosen", "creature"],
    &["and", "the", "chosen", "permanent"],
    &["and", "chosen", "permanent"],
    &["and", "the", "chosen", "object"],
    &["and", "chosen", "object"],
];
const SELF_REFERENCE_WORDS: &[&str] = &["this", "it", "them"];
const OBJECT_REFERENCE_NOUN_WORDS: &[&str] = &[
    "artifact",
    "artifacts",
    "battle",
    "battles",
    "card",
    "cards",
    "creature",
    "creatures",
    "enchantment",
    "enchantments",
    "land",
    "lands",
    "permanent",
    "permanents",
    "planeswalker",
    "planeswalkers",
    "spell",
    "spells",
    "token",
    "tokens",
];
const EXCLUSION_RELATION_IGNORED_PREFIXES: &[&[&str]] =
    &[&["enchanted"], &["equipped"], &["basic", "land"]];
const REST_REVEALED_OBJECT_PHRASES: &[&[&str]] = &[
    &["rest"],
    &["rest", "of", "revealed", "cards"],
    &["remaining", "revealed", "cards"],
];
const TAGGED_COUNTER_STATE_DISJUNCTION_PHRASES: &[&[&str]] = &[
    &["counter", "on", "it", "or"],
    &["counter", "on", "them", "or"],
];
const SUSPENDED_CARD_DISJUNCTION_PHRASES: &[&[&str]] =
    &[&["or", "suspended", "card"], &["or", "suspended", "cards"]];
const ENTERED_THIS_TURN_UNSUPPORTED_PHRASE: &[&str] = &["entered", "this", "turn"];
const BLOCKED_BY_TAGGED_OBJECT_PHRASES: &[&[&str]] = &[
    &["blocked", "by", "one", "of", "those"],
    &["blocked", "by", "those"],
    &["blocked", "by", "that"],
];
const POWER_OR_TOUGHNESS_PHRASES: &[&[&str]] =
    &[&["power", "or", "toughness"], &["toughness", "or", "power"]];
const TARGET_PLAYER_REFERENCE_PHRASES: &[&[&str]] =
    &[&["target", "player"], &["target", "players"]];
const TARGET_OPPONENT_REFERENCE_PHRASES: &[&[&str]] =
    &[&["target", "opponent"], &["target", "opponents"]];
const WITH_WORD: &str = "with";
const WITHOUT_WORD: &str = "without";
const BASE_POWER_TOUGHNESS_PREFIX: &[&str] = &["base", "power", "and", "toughness"];
const POWER_TOUGHNESS_PREFIX: &[&str] = &["power", "and", "toughness"];
const TOUGHNESS_GREATER_THAN_POWER_PHRASES: &[&[&str]] = &[
    &["toughness", "greater", "than", "its", "power"],
    &["toughness", "greater", "than", "their", "power"],
    &["power", "less", "than", "its", "toughness"],
    &["power", "less", "than", "their", "toughness"],
];
const POWER_GREATER_THAN_TOUGHNESS_PHRASES: &[&[&str]] = &[
    &["power", "greater", "than", "its", "toughness"],
    &["power", "greater", "than", "their", "toughness"],
    &["toughness", "less", "than", "its", "power"],
    &["toughness", "less", "than", "their", "power"],
];
const POWER_TOUGHNESS_NOT_EQUAL_PHRASES: &[&[&str]] = &[
    &["power", "and", "toughness", "aren't", "equal"],
    &["power", "and", "toughness", "arent", "equal"],
    &["power", "and", "toughness", "are", "not", "equal"],
];
const ATTACHMENT_TAGGED_TAIL_PREFIXES: &[&[&str]] = &[
    &["it"],
    &["that", "object"],
    &["that", "creature"],
    &["that", "permanent"],
    &["that", "equipment"],
    &["that", "aura"],
];
const ENCHANTED_PLAYER_ATTACHMENT_PREFIX: &[&str] = &["enchanted", "player"];
const THAT_PLAYER_ATTACHMENT_TAIL: &[&str] = &["that", "player"];

fn token_index_after_word_prefix(tokens: &[OwnedLexToken], word_len: usize) -> Option<usize> {
    if word_len == 0 {
        return Some(0);
    }

    let mut seen_words = 0usize;
    for (token_idx, token) in tokens.iter().enumerate() {
        let token_words = token.parser_word_pieces().len();
        if token_words == 0 {
            continue;
        }
        seen_words += token_words;
        if seen_words == word_len {
            return Some(token_idx + 1);
        }
        if seen_words > word_len {
            return None;
        }
    }
    None
}

/// Return the word spans consumed by characteristic-comparison operands.
/// Object nouns and player relations inside those operands describe the value
/// source, not the candidate object (for example, `the number of Vampires you
/// control` must not add Vampire or `you control` to the outer filter).
fn filter_comparison_rhs_ranges(
    words: &[&str],
) -> Result<Vec<std::ops::Range<usize>>, CardTextError> {
    let mut ranges = Vec::new();
    let mut idx = 0usize;
    while idx < words.len() {
        let (axis, axis_word_count) =
            if parse_phrase_at_head(&words[idx..], MANA_VALUE_PREFIX).is_some() {
                ("mana value", MANA_VALUE_PREFIX.len())
            } else if words[idx] == POWER_WORD {
                // In "total power and toughness N or less", `power` is
                // part of the compound axis, not the start of an ordinary
                // power comparison.  Treating the following `and` as the
                // comparison operand produces a misleading dynamic-value
                // error before the dedicated total-PT pass can handle it.
                if idx > 0
                    && words[idx - 1] == "total"
                    && words.get(idx + 1) == Some(&"and")
                    && words.get(idx + 2) == Some(&"toughness")
                {
                    idx += 1;
                    continue;
                }
                ("power", 1)
            } else if words[idx] == TOUGHNESS_WORD {
                ("toughness", 1)
            } else {
                idx += 1;
                continue;
            };

        // Relational P/T phrases such as "toughness greater than their
        // power" are handled by the dedicated relation pass below.  Do not
        // send their pronoun operand through the generic scalar-comparison
        // parser, which quite reasonably rejects `their` as a dynamic value.
        if (axis == "toughness"
            && parse_phrase_choice_at_head(&words[idx..], TOUGHNESS_GREATER_THAN_POWER_PHRASES)
                .is_some())
            || (axis == "power"
                && parse_phrase_choice_at_head(&words[idx..], POWER_GREATER_THAN_TOUGHNESS_PHRASES)
                    .is_some())
        {
            idx += axis_word_count;
            continue;
        }

        let rhs_start = idx + axis_word_count;
        let Some((_, consumed)) = parse_filter_comparison_tokens(axis, &words[rhs_start..], words)?
        else {
            idx = rhs_start;
            continue;
        };
        let rhs_end = rhs_start.saturating_add(consumed).min(words.len());
        if rhs_start < rhs_end {
            ranges.push(rhs_start..rhs_end);
        }
        idx = rhs_end.max(rhs_start);
    }
    Ok(ranges)
}

fn word_is_in_ranges(word_idx: usize, ranges: &[std::ops::Range<usize>]) -> bool {
    ranges.iter().any(|range| range.contains(&word_idx))
}

/// Split an intrinsic attachment selector into the object being selected and
/// the filter its attachment target must satisfy. This is deliberately done
/// before the ordinary type pass so `Aura attached to a creature` cannot be
/// flattened into the impossible conjunction `Aura creature`.
/// Split "<subject> with a <inner> attached to it" into the subject tokens
/// and the attachment's inner filter tokens.
fn split_with_attached_object_filter(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let trimmed = trim_commas(tokens);
    let n = trimmed.len();
    if n < 5
        || !(trimmed[n - 3].is_word("attached")
            && trimmed[n - 2].is_word("to")
            && trimmed[n - 1].is_word("it"))
    {
        return None;
    }
    let with_idx = trimmed[..n - 3]
        .iter()
        .rposition(|token| token.is_word("with"))?;
    let mut inner = trim_commas(&trimmed[with_idx + 1..n - 3]);
    if inner
        .first()
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        inner.remove(0);
    }
    if inner.is_empty() {
        return None;
    }
    let subject = trim_commas(&trimmed[..with_idx]);
    if subject.is_empty() {
        return None;
    }
    Some((subject.to_vec(), inner.to_vec()))
}

/// Split "<subject> that's enchanted by <inner>" (and the expanded
/// "that is/are enchanted by" forms) into the selected subject and the Aura
/// filter that must match one of its attachments.
fn split_enchanted_by_object_filter(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let trimmed = trim_commas(tokens);
    let enchanted_idx = trimmed
        .iter()
        .position(|token| token.is_word("enchanted"))?;
    if !trimmed
        .get(enchanted_idx + 1)
        .is_some_and(|token| token.is_word("by"))
    {
        return None;
    }

    let subject_end = if enchanted_idx >= 1
        && (trimmed[enchanted_idx - 1].is_word("that's")
            || trimmed[enchanted_idx - 1].is_word("thats"))
    {
        enchanted_idx - 1
    } else if enchanted_idx >= 2
        && trimmed[enchanted_idx - 2].is_word("that")
        && (trimmed[enchanted_idx - 1].is_word("is") || trimmed[enchanted_idx - 1].is_word("are"))
    {
        enchanted_idx - 2
    } else {
        return None;
    };

    let subject = trim_commas(&trimmed[..subject_end]);
    let mut inner = trim_commas(&trimmed[enchanted_idx + 2..]);
    if inner
        .first()
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        inner.remove(0);
    }
    if subject.is_empty() || inner.is_empty() {
        return None;
    }
    Some((subject.to_vec(), inner.to_vec()))
}

fn split_attached_to_object_filter(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let attached_idx = tokens.iter().position(|token| token.is_word("attached"))?;
    let to_idx = attached_idx + 1;
    if !tokens.get(to_idx).is_some_and(|token| token.is_word("to")) {
        return None;
    }

    let tail = trim_commas(&tokens[to_idx + 1..]);
    if tail.is_empty() {
        return None;
    }
    let tail_words = non_article_parser_word_refs(&tail);
    if parse_phrase_choice_at_head(&tail_words, ATTACHMENT_TAGGED_TAIL_PREFIXES).is_some()
        || parse_phrase_at_head(&tail_words, ENCHANTED_PLAYER_ATTACHMENT_PREFIX).is_some()
    {
        return None;
    }

    let mut head_end = attached_idx;
    if head_end > 0
        && tokens[head_end - 1]
            .as_word()
            .is_some_and(|word| matches!(word, "thats" | "that's"))
    {
        head_end -= 1;
    } else if head_end >= 2
        && tokens[head_end - 2].is_word("that")
        && tokens[head_end - 1]
            .as_word()
            .is_some_and(|word| parse_word_choice(word, BE_VERB_WORDS).is_some())
    {
        head_end -= 2;
    }

    let head = trim_commas(&tokens[..head_end]);
    if head.is_empty() {
        return None;
    }
    Some((head, tail))
}

fn has_plural_object_noun_surface(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .any(|word| {
            matches!(
                word,
                "artifacts"
                    | "auras"
                    | "battles"
                    | "cards"
                    | "creatures"
                    | "enchantments"
                    | "lands"
                    | "objects"
                    | "permanents"
                    | "planeswalkers"
                    | "spells"
                    | "tokens"
            )
        })
}

/// Whether the head object noun in a phrase is plural.
///
/// Unlike [`has_plural_object_noun_surface`], this stops at the first object
/// noun so a singular destination such as "a permanent attached to creatures
/// you control" is not widened by a plural noun in its relative clause.
pub(crate) fn has_plural_object_head_surface(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .find_map(|word| {
            if matches!(
                word,
                "artifacts"
                    | "auras"
                    | "battles"
                    | "cards"
                    | "creatures"
                    | "enchantments"
                    | "lands"
                    | "objects"
                    | "permanents"
                    | "planeswalkers"
                    | "spells"
                    | "tokens"
            ) {
                Some(true)
            } else if matches!(
                word,
                "artifact"
                    | "aura"
                    | "battle"
                    | "card"
                    | "creature"
                    | "enchantment"
                    | "land"
                    | "object"
                    | "permanent"
                    | "planeswalker"
                    | "spell"
                    | "token"
            ) {
                Some(false)
            } else {
                None
            }
        })
        == Some(true)
}

fn source_reference_tail_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(usize, crate::target::SourceReferenceSurface)> {
    let words = parser_token_word_refs(tokens);
    for word_len in (1..=words.len()).rev() {
        let prefix = &words[..word_len];
        let Some(surface) = source_reference_surface_for_words(prefix)
            .or_else(|| this_source_surface_for_words(prefix))
            .or_else(|| {
                (word_len == 1 && parse_word_choice(prefix[0], SELF_REFERENCE_WORDS).is_some())
                    .then(|| {
                        crate::target::SourceReferenceSurface::ThisPermanentType(
                            prefix[0].to_string(),
                        )
                    })
            })
        else {
            continue;
        };
        let token_len = token_index_after_word_prefix(tokens, word_len)?;
        return Some((token_len, surface));
    }
    None
}

fn comparison_references_source_power(comparison: &crate::filter::Comparison) -> bool {
    matches!(
        comparison,
        crate::filter::Comparison::GreaterThanExpr(value)
            | crate::filter::Comparison::LessThanExpr(value)
            if matches!(value.as_ref(), crate::effect::Value::SourcePower)
    )
}

fn comparison_references_source_toughness(comparison: &crate::filter::Comparison) -> bool {
    matches!(
        comparison,
        crate::filter::Comparison::GreaterThanExpr(value)
            | crate::filter::Comparison::LessThanExpr(value)
            if matches!(value.as_ref(), crate::effect::Value::SourceToughness)
    )
}

fn clear_redundant_power_toughness_axis_filter(
    filter: &mut ObjectFilter,
    relation: crate::filter::PowerToughnessRelation,
) {
    match relation {
        crate::filter::PowerToughnessRelation::ToughnessGreaterThanPower => {
            if filter
                .power
                .as_ref()
                .is_some_and(comparison_references_source_toughness)
            {
                filter.power = None;
            }
            if filter
                .toughness
                .as_ref()
                .is_some_and(comparison_references_source_power)
            {
                filter.toughness = None;
            }
        }
        crate::filter::PowerToughnessRelation::PowerGreaterThanToughness => {
            if filter
                .power
                .as_ref()
                .is_some_and(comparison_references_source_toughness)
            {
                filter.power = None;
            }
            if filter
                .toughness
                .as_ref()
                .is_some_and(comparison_references_source_power)
            {
                filter.toughness = None;
            }
        }
        crate::filter::PowerToughnessRelation::NotEqual => {}
    }
}
const BASE_WORD: &str = "base";
const POWER_WORD: &str = "power";
const TOUGHNESS_WORD: &str = "toughness";
const AND_WORD: &str = "and";
const AND_OR_WORDS: &[&str] = &["and", "or"];
const BE_VERB_WORDS: &[&str] = &["are", "is", "was", "were"];
const HAS_HAVE_WORDS: &[&str] = &["has", "have"];
const TAGGED_SPELL_REFERENCE_WORDS: &[&str] = &["that", "this", "its", "their"];
const ABILITY_OR_ABILITIES_WORDS: &[&str] = &["ability", "abilities"];
const ACTIVATED_ABILITY_WORDS: &[&str] = &["activated", "ability"];
const TRIGGERED_ABILITY_WORDS: &[&str] = &["triggered", "ability"];
const ACTIVATED_OR_TRIGGERED_ABILITY_PHRASES: &[&[&str]] = &[
    &["activated", "or", "triggered", "ability"],
    &["activated", "or", "triggered", "abilities"],
    &["activated", "and", "triggered", "abilities"],
    &["triggered", "or", "activated", "ability"],
    &["triggered", "or", "activated", "abilities"],
    &["triggered", "and", "activated", "abilities"],
];
const SPELL_AND_ABILITY_PHRASES: &[&[&str]] = &[
    &["spell", "and", "ability"],
    &["spell", "and", "abilities"],
    &["spells", "and", "ability"],
    &["spells", "and", "abilities"],
];
const TEXT_NEGATION_WORDS: &[&str] = &["not", "isnt", "isn't", "arent", "aren't"];
const LEGENDARY_OR_PREFIX: &[&str] = &["legendary", "or"];
const PUT_ON_PREFIX: &[&str] = &["put", "on"];
const PUT_ON_REFERENCE_WORDS: &[&str] = &["it", "them"];
const MANA_VALUE_PREFIX: &[&str] = &["mana", "value"];
const NOT_HISTORIC_PHRASE: &[&str] = &["not", "historic"];
const ATTACKING_WORD: &str = "attacking";
const BLOCKING_WORD: &str = "blocking";
const BLOCKED_WORD: &str = "blocked";
const HISTORIC_WORD: &str = "historic";
const COMMANDER_OR_COMMANDERS_WORDS: &[&str] = &["commander", "commanders"];
const CHOSEN_WORD: &str = "chosen";
const NONCHOSEN_WORD: &str = "nonchosen";
const COLOR_WORD: &str = "color";
const TYPE_WORD: &str = "type";
const PERMANENT_OR_PERMANENTS_WORDS: &[&str] = &["permanent", "permanents"];
const SPELL_OR_SPELLS_WORDS: &[&str] = &["spell", "spells"];
const POWER_GREATER_THAN_BASE_POWER_PHRASE: &[&str] =
    &["power", "greater", "than", "its", "base", "power"];
const NON_WORD: &str = "non";
const ATTACKED_THIS_TURN_PHRASE: &[&str] = &["attacked", "this", "turn"];
const TYPE_LIST_CONJUNCTION_WORDS: &[&str] = &["and", "or", "and/or"];
const STRICT_COMPOUND_COUNT_PREFIXES: &[&[&str]] = &[&["and", "each"], &["and", "every"]];
const STRICT_FOR_EACH_TAIL_PREFIX: &[&str] = &["for", "each"];
const OTHER_THAN_BASIC_LAND_PREFIX: &[&str] = &["other", "than", "basic", "land"];
const CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const AGGREGATE_SCOPE_WORDS: &[&str] = &["greatest", "least", "total"];
const AGGREGATE_SCOPE_MARKER_WORDS: &[&str] = &["among", "of"];
const EXCLUDED_CHOSEN_TYPE_PHRASES: &[&[&str]] = &[
    &["that", "arent", "of", "chosen", "type"],
    &["that", "aren't", "of", "chosen", "type"],
    &["that", "are", "not", "of", "chosen", "type"],
    &["that", "isnt", "of", "chosen", "type"],
    &["that", "isn't", "of", "chosen", "type"],
    &["that", "is", "not", "of", "chosen", "type"],
];
const EXCLUDED_TYPE_CHOSEN_THIS_WAY_PHRASES: &[&[&str]] = &[
    &["that", "arent", "of", "a", "type", "chosen", "this", "way"],
    &["that", "aren't", "of", "a", "type", "chosen", "this", "way"],
    &[
        "that", "are", "not", "of", "a", "type", "chosen", "this", "way",
    ],
    &["that", "isnt", "of", "a", "type", "chosen", "this", "way"],
    &["that", "isn't", "of", "a", "type", "chosen", "this", "way"],
    &[
        "that", "is", "not", "of", "a", "type", "chosen", "this", "way",
    ],
];

fn contains_explicit_card_noun(
    words: &[&str],
    comparison_rhs_ranges: &[std::ops::Range<usize>],
) -> bool {
    words.iter().enumerate().any(|(idx, word)| {
        matches!(*word, "card" | "cards")
            && !word_is_in_ranges(idx, comparison_rhs_ranges)
            // These phrases name a characteristic rather than the object
            // selected by this filter.
            && !words
                .get(idx + 1)
                .is_some_and(|next| matches!(*next, "type" | "types" | "name" | "names"))
    })
}
const NO_SHARED_CREATURE_TYPE_WITH_YOUR_CREATURES_OR_GRAVEYARD_CLAUSES: &[&[&str]] = &[
    &[
        "that",
        "doesn't",
        "share",
        "creature",
        "type",
        "with",
        "creature",
        "you",
        "control",
        "or",
        "creature",
        "card",
        "in",
        "your",
        "graveyard",
    ],
    &[
        "that",
        "doesnt",
        "share",
        "creature",
        "type",
        "with",
        "creature",
        "you",
        "control",
        "or",
        "creature",
        "card",
        "in",
        "your",
        "graveyard",
    ],
    &[
        "that",
        "does",
        "not",
        "share",
        "creature",
        "type",
        "with",
        "creature",
        "you",
        "control",
        "or",
        "creature",
        "card",
        "in",
        "your",
        "graveyard",
    ],
];

fn token_index_for_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if tokens[idx]
            .as_word()
            .is_some_and(|word| parse_word_choice(word, &[expected]).is_some())
        {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn non_article_parser_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str> {
    parser_token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SharedCharacteristicClause {
    start: usize,
    negated: bool,
    characteristics: Vec<ObjectCharacteristic>,
    rhs_start: usize,
}

fn parse_shared_characteristic_axis(
    words: &[&str],
    start: usize,
) -> Option<(Vec<ObjectCharacteristic>, usize)> {
    let mut idx = start;
    if words.get(idx..idx + 3) == Some(["at", "least", "one"].as_slice()) {
        idx += 3;
    }

    let (characteristics, consumed) = if words.get(idx..idx + 4)
        == Some(["color", "or", "mana", "value"].as_slice())
        || words.get(idx..idx + 4) == Some(["colors", "or", "mana", "value"].as_slice())
    {
        (
            vec![ObjectCharacteristic::Color, ObjectCharacteristic::ManaValue],
            4,
        )
    } else if words.get(idx..idx + 2) == Some(["card", "type"].as_slice())
        || words.get(idx..idx + 2) == Some(["card", "types"].as_slice())
    {
        (vec![ObjectCharacteristic::CardType], 2)
    } else if words.get(idx..idx + 2) == Some(["permanent", "type"].as_slice())
        || words.get(idx..idx + 2) == Some(["permanent", "types"].as_slice())
    {
        (vec![ObjectCharacteristic::PermanentType], 2)
    } else if words.get(idx..idx + 2) == Some(["creature", "type"].as_slice())
        || words.get(idx..idx + 2) == Some(["creature", "types"].as_slice())
    {
        (
            vec![ObjectCharacteristic::Subtype(SubtypeFamily::Creature)],
            2,
        )
    } else if words.get(idx..idx + 2) == Some(["land", "type"].as_slice())
        || words.get(idx..idx + 2) == Some(["land", "types"].as_slice())
    {
        (vec![ObjectCharacteristic::Subtype(SubtypeFamily::Land)], 2)
    } else if words.get(idx..idx + 2) == Some(["artifact", "type"].as_slice())
        || words.get(idx..idx + 2) == Some(["artifact", "types"].as_slice())
    {
        (
            vec![ObjectCharacteristic::Subtype(SubtypeFamily::Artifact)],
            2,
        )
    } else if words.get(idx..idx + 2) == Some(["enchantment", "type"].as_slice())
        || words.get(idx..idx + 2) == Some(["enchantment", "types"].as_slice())
    {
        (
            vec![ObjectCharacteristic::Subtype(SubtypeFamily::Enchantment)],
            2,
        )
    } else if words.get(idx..idx + 2) == Some(["spell", "type"].as_slice())
        || words.get(idx..idx + 2) == Some(["spell", "types"].as_slice())
    {
        (vec![ObjectCharacteristic::Subtype(SubtypeFamily::Spell)], 2)
    } else if words.get(idx..idx + 2) == Some(["planeswalker", "type"].as_slice())
        || words.get(idx..idx + 2) == Some(["planeswalker", "types"].as_slice())
    {
        (
            vec![ObjectCharacteristic::Subtype(SubtypeFamily::Planeswalker)],
            2,
        )
    } else if words.get(idx..idx + 2) == Some(["battle", "type"].as_slice())
        || words.get(idx..idx + 2) == Some(["battle", "types"].as_slice())
    {
        (
            vec![ObjectCharacteristic::Subtype(SubtypeFamily::Battle)],
            2,
        )
    } else if words
        .get(idx)
        .is_some_and(|word| matches!(*word, "color" | "colors"))
    {
        (vec![ObjectCharacteristic::Color], 1)
    } else {
        return None;
    };

    Some((characteristics, idx + consumed))
}

fn parse_shared_characteristic_clause_at(
    words: &[&str],
    start: usize,
) -> Option<SharedCharacteristicClause> {
    if !words
        .get(start)
        .is_some_and(|word| matches!(*word, "that" | "which"))
    {
        return None;
    }

    let mut idx = start + 1;
    let negated = match words.get(idx).copied()? {
        "share" | "shares" => {
            idx += 1;
            false
        }
        "doesn't" | "doesnt" | "don't" | "dont" => {
            if words.get(idx + 1) != Some(&"share") {
                return None;
            }
            idx += 2;
            true
        }
        "does" | "do" => {
            if words.get(idx + 1) != Some(&"not") || words.get(idx + 2) != Some(&"share") {
                return None;
            }
            idx += 3;
            true
        }
        _ => return None,
    };

    let (characteristics, with_idx) = parse_shared_characteristic_axis(words, idx)?;
    if words.get(with_idx) != Some(&"with") || with_idx + 1 >= words.len() {
        return None;
    }

    Some(SharedCharacteristicClause {
        start,
        negated,
        characteristics,
        rhs_start: with_idx + 1,
    })
}

fn find_shared_characteristic_clause(words: &[&str]) -> Option<SharedCharacteristicClause> {
    (0..words.len()).find_map(|start| parse_shared_characteristic_clause_at(words, start))
}

fn shared_characteristic_rhs_uses_legacy_reference_path(rhs: &[&str]) -> bool {
    rhs.iter().any(|word| matches!(*word, "it" | "them"))
        || rhs.first() == Some(&"that")
        || rhs
            .iter()
            .any(|word| matches!(*word, "sacrificed" | "tapped" | "additional" | "discarded"))
}

fn token_boundary_for_non_article_word(
    tokens: &[OwnedLexToken],
    non_article_word_idx: usize,
) -> Option<usize> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let full_word_idx = words
        .iter()
        .enumerate()
        .filter(|(_, word)| !is_article(word))
        .nth(non_article_word_idx)
        .map(|(idx, _)| idx)?;
    word_view.token_boundary_for_word_or_end(full_word_idx)
}

fn try_apply_shared_characteristic_relation_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> Result<bool, CardTextError> {
    let Some(clause) = find_shared_characteristic_clause(all_words) else {
        return Ok(false);
    };
    let rhs = &all_words[clause.rhs_start..];
    if shared_characteristic_rhs_uses_legacy_reference_path(rhs) {
        return Ok(false);
    }

    let comparison = crate::runtime_backend::object_filters::parse_object_filter_words(rhs, false)?;
    let relation = if clause.negated {
        ObjectCharacteristicRelation::shares_none(clause.characteristics.clone(), comparison)
    } else {
        ObjectCharacteristicRelation::shares(clause.characteristics.clone(), comparison)
    };

    let segment_words = non_article_parser_word_refs(segment_tokens);
    let segment_clause = find_shared_characteristic_clause(&segment_words).filter(|candidate| {
        candidate.negated == clause.negated && candidate.characteristics == clause.characteristics
    });
    let token_boundary = segment_clause
        .and_then(|candidate| token_boundary_for_non_article_word(segment_tokens, candidate.start));

    filter.characteristic_relations.push(relation);
    all_words.truncate(clause.start);
    if let Some(token_boundary) = token_boundary {
        segment_tokens.truncate(token_boundary);
    }
    Ok(true)
}

fn try_apply_blocked_or_was_blocked_by_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> Result<bool, CardTextError> {
    let words = non_article_parser_word_refs(segment_tokens);
    let Some(blocked_idx) = (0..words.len().saturating_sub(4)).find(|&idx| {
        words.get(idx..idx + 5) == Some(["blocked", "or", "was", "blocked", "by"].as_slice())
    }) else {
        return Ok(false);
    };
    let partner_start = blocked_idx + 5;
    let Some(this_turn_idx) = words
        .windows(2)
        .enumerate()
        .rev()
        .find_map(|(idx, window)| (window == ["this", "turn"]).then_some(idx))
    else {
        return Ok(false);
    };
    if partner_start >= this_turn_idx || this_turn_idx + 2 != words.len() {
        return Ok(false);
    }

    let clause_start = blocked_idx
        .checked_sub(1)
        .filter(|idx| matches!(words[*idx], "that" | "which"))
        .unwrap_or(blocked_idx);
    let Some(partner_token_start) =
        token_boundary_for_non_article_word(segment_tokens, partner_start)
    else {
        return Ok(false);
    };
    let Some(this_turn_token_start) =
        token_boundary_for_non_article_word(segment_tokens, this_turn_idx)
    else {
        return Ok(false);
    };
    let partner_tokens = trim_commas(&segment_tokens[partner_token_start..this_turn_token_start]);
    if partner_tokens.is_empty() {
        return Ok(false);
    }
    let combat_partner = parse_object_filter(&partner_tokens, false)?;
    filter.blocked_or_was_blocked_by_this_turn = Some(Box::new(combat_partner));

    all_words.truncate(clause_start);
    if let Some(clause_token_start) =
        token_boundary_for_non_article_word(segment_tokens, clause_start)
    {
        segment_tokens.truncate(clause_token_start);
    }
    Ok(true)
}

fn has_tap_activated_ability_phrase(words: &[&str]) -> bool {
    const TAP_ACTIVATED_ABILITY_PHRASES: &[&[&str]] = &[
        &[
            "has",
            "activated",
            "ability",
            "with",
            "t",
            "in",
            "its",
            "cost",
        ],
        &[
            "has",
            "activated",
            "ability",
            "with",
            "tap",
            "in",
            "its",
            "cost",
        ],
        &[
            "activated",
            "abilities",
            "with",
            "t",
            "in",
            "their",
            "costs",
        ],
        &[
            "activated",
            "abilities",
            "with",
            "tap",
            "in",
            "their",
            "costs",
        ],
    ];
    parse_phrase_choice_anywhere(words, TAP_ACTIVATED_ABILITY_PHRASES).is_some()
}

fn strip_be_put_on_reference_prefix(all_words: &mut Vec<&str>, segment_tokens: &[OwnedLexToken]) {
    if all_words.len() < 4 || segment_tokens.len() < 4 {
        return;
    }

    let be_words = non_article_parser_word_refs(&segment_tokens[..1]);
    let put_on_words = non_article_parser_word_refs(&segment_tokens[1..4]);
    if !be_words
        .first()
        .is_some_and(|word| parse_word_choice(word, BE_VERB_WORDS).is_some())
        || parse_phrase_at_head(&put_on_words, PUT_ON_PREFIX).is_none()
        || parse_word_choice_anywhere(&put_on_words, PUT_ON_REFERENCE_WORDS).is_none()
    {
        return;
    }

    all_words.drain(0..3);
}

pub(crate) fn parse_object_filter_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let mut filter = parse_object_filter_lexed(tokens, other)?;
    apply_supertype_or_mana_capability_union(&mut filter, tokens);
    super::counter_constraints::preserve_filter_counter_constraint_surface_tokens(
        &mut filter,
        tokens,
    );
    super::simple::preserve_branch_scoped_card_type_union(&mut filter, tokens, other);
    apply_chosen_type_domain(&mut filter, tokens);
    fn remove_explicitly_excluded_positive_subtypes(filter: &mut ObjectFilter) {
        filter
            .subtypes
            .retain(|subtype| !filter.excluded_subtypes.contains(subtype));
        filter
            .all_subtypes
            .retain(|subtype| !filter.excluded_subtypes.contains(subtype));
        for branch in &mut filter.any_of {
            remove_explicitly_excluded_positive_subtypes(branch);
        }
    }

    // Several permissive recovery passes run after the primary atom parser.
    // Preserve the explicit `non-*` meaning as the final invariant even when
    // one of those passes sees the suffix word without its lexical `non`
    // prefix and tentatively restores it as a positive subtype.
    remove_explicitly_excluded_positive_subtypes(&mut filter);

    Ok(filter)
}

/// Preserve a shared object domain around `supertype OR mana capability`.
///
/// The permissive characteristic scan correctly recognizes the supertype in
/// "land that is snow or could produce {C}", but it cannot represent the
/// second arm without this typed capability predicate and would otherwise
/// silently narrow the legal set to snow lands.
pub(crate) fn apply_supertype_or_mana_capability_union(
    filter: &mut ObjectFilter,
    tokens: &[OwnedLexToken],
) {
    if !filter.any_of.is_empty() {
        return;
    }

    let words = parser_token_word_positions(tokens);
    for (word_idx, window) in words.windows(7).enumerate() {
        if window[0].1 != "that"
            || !matches!(window[1].1, "is" | "are")
            || window[3].1 != "or"
            || window[4].1 != "could"
            || window[5].1 != "produce"
            || word_idx + window.len() != words.len()
        {
            continue;
        }
        let Some(supertype) = parse_supertype_word(window[2].1) else {
            continue;
        };
        let Ok(mana_symbol) = parse_mana_symbol(tokens[window[6].0].parser_text()) else {
            continue;
        };
        let Some(supertype_idx) = filter
            .supertypes
            .iter()
            .position(|candidate| *candidate == supertype)
        else {
            continue;
        };

        filter.supertypes.remove(supertype_idx);
        filter.any_of = vec![
            ObjectFilter {
                supertypes: vec![supertype],
                ..ObjectFilter::default()
            },
            ObjectFilter {
                could_produce_mana: vec![mana_symbol],
                ..ObjectFilter::default()
            },
        ];
        return;
    }
}

fn apply_chosen_type_domain(filter: &mut ObjectFilter, tokens: &[OwnedLexToken]) {
    if parse_chosen_type_reference_tokens(tokens).is_none() {
        return;
    }
    let has_land_type = filter
        .card_types
        .iter()
        .chain(filter.all_card_types.iter())
        .any(|card_type| *card_type == CardType::Land);
    let has_nonland_type = filter
        .card_types
        .iter()
        .chain(filter.all_card_types.iter())
        .any(|card_type| *card_type != CardType::Land);
    if has_land_type && !has_nonland_type {
        filter.chosen_land_type = true;
        filter.chosen_creature_type = false;
    }
}

pub(super) fn parse_object_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    if let Some(filter) = super::parse_repeated_selector_domain_union_lexed(tokens, other) {
        return Ok(filter);
    }
    parse_object_filter_inner(tokens, other, true)
}

pub(super) fn parse_object_filter_permissive(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter_inner(tokens, other, false)
}

pub(super) fn parse_object_filter_inner(
    tokens: &[OwnedLexToken],
    other: bool,
    strict: bool,
) -> Result<ObjectFilter, CardTextError> {
    let (tokens, vote_winners_only) = trim_vote_winner_suffix(tokens);
    let trailing_couldnt_attack_exception = tokens.len() >= 6
        && tokens[tokens.len() - 6].is_word("except")
        && tokens[tokens.len() - 5].is_word("for")
        && (tokens[tokens.len() - 4].is_word("creature")
            || tokens[tokens.len() - 4].is_word("creatures"))
        && tokens[tokens.len() - 3].is_word("that")
        && (tokens[tokens.len() - 2].is_word("couldn't")
            || tokens[tokens.len() - 2].is_word("couldnt"))
        && tokens[tokens.len() - 1].is_word("attack");
    let tokens = if trailing_couldnt_attack_exception {
        &tokens[..tokens.len() - 6]
    } else {
        tokens.as_slice()
    };
    // A terminal arity phrase can qualify the stack object without naming a
    // target class: "an instant or sorcery spell with a single target". The
    // relation parser below only sees `that target(s) ...`, so retain this
    // independent grammar fact before parsing the ordinary spell domain.
    let trailing_single_target = tokens.len() >= 4
        && tokens[tokens.len() - 4].is_word("with")
        && tokens[tokens.len() - 3].is_word("a")
        && tokens[tokens.len() - 2].is_word("single")
        && (tokens[tokens.len() - 1].is_word("target")
            || tokens[tokens.len() - 1].is_word("targets"));
    let tokens = if trailing_single_target {
        &tokens[..tokens.len() - 4]
    } else {
        tokens
    };
    let chosen_type_reference = parse_chosen_type_reference_tokens(&tokens);
    let mut filter = ObjectFilter::default();
    filter.could_have_attacked_this_turn = trailing_couldnt_attack_exception;
    if other {
        filter.other = true;
    }

    let mut target_player: Option<PlayerFilter> = None;
    let mut target_object: Option<ObjectFilter> = None;
    let mut targets_only = false;
    let mut target_count = trailing_single_target.then_some(crate::effect::ChoiceCount::exactly(1));
    let mut base_tokens: Vec<OwnedLexToken> = tokens.to_vec();
    let mut targets_idx: Option<usize> = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token
            .as_word()
            .is_some_and(|word| parse_word_choice(word, TARGET_OR_TARGETS_WORDS).is_some())
        {
            if idx > 0
                && tokens[idx - 1]
                    .as_word()
                    .is_some_and(|word| word == THAT_WORD)
            {
                targets_idx = Some(idx);
                break;
            }
        }
    }
    if let Some(targets_idx) = targets_idx {
        let that_idx = targets_idx - 1;
        base_tokens = tokens[..that_idx].to_vec();
        let mut target_tokens = &tokens[targets_idx + 1..];
        let mut relation_target_count = None;
        // A bare article is grammar, not arity: "that targets a permanent you
        // control" constrains what is targeted, never how many targets the
        // spell has. Only an explicit count ("that targets two ...") narrows
        // the relation's target count.
        let leading_article = target_tokens.first().is_some_and(|token| {
            token
                .as_word()
                .is_some_and(|word| word == "a" || word == "an")
        });
        if !leading_article
            && let Some((count, rest)) = primitives::parse_prefix(
                target_tokens,
                crate::runtime_backend::front_end::grammar::leaf::parse_leaf_choice_count_prefix_lexed,
            )
        {
            relation_target_count = Some(count);
            target_tokens = rest;
        }
        let parse_target_fragment = |fragment_tokens: &[OwnedLexToken]| -> Result<
            (
                Option<PlayerFilter>,
                Option<ObjectFilter>,
                bool,
                Option<crate::effect::ChoiceCount>,
            ),
            CardTextError,
        > {
            let mut fragment_tokens = trim_commas(fragment_tokens);
            let mut only = false;
            let mut count = None;
            // The outer scan splits target fragments after the demonstrative
            // "that target(s)" marker, so a fragment never re-introduces a
            // leading "that"; strip one defensively to keep the fragment shape
            // stable if upstream splitting changes.
            if fragment_tokens
                .first()
                .is_some_and(|token| token.as_word().is_some_and(|word| word == THAT_WORD))
            {
                fragment_tokens.drain(..1);
            }
            if fragment_tokens
                .first()
                .is_some_and(|token| token.as_word().is_some_and(|word| word == ONLY_WORD))
            {
                only = true;
                fragment_tokens.drain(..1);
            }
            if fragment_tokens.len() >= 2
                && fragment_tokens[0].is_word("a")
                && fragment_tokens[1]
                    .as_word()
                    .is_some_and(|word| word == SINGLE_WORD)
            {
                count = Some(crate::effect::ChoiceCount::exactly(1));
                fragment_tokens.drain(..2);
            } else if fragment_tokens
                .first()
                .is_some_and(|token| token.as_word().is_some_and(|word| word == SINGLE_WORD))
            {
                count = Some(crate::effect::ChoiceCount::exactly(1));
                fragment_tokens.drain(..1);
            }

            if parse_phrase_at_head(
                &non_article_parser_word_refs(&fragment_tokens),
                YOU_TARGET_PREFIX,
            )
            .is_some()
            {
                return Ok((Some(PlayerFilter::You), None, only, count));
            }
            if parse_phrase_choice_at_head(
                &non_article_parser_word_refs(&fragment_tokens),
                OPPONENT_TARGET_PREFIXES,
            )
            .is_some()
            {
                return Ok((Some(PlayerFilter::Opponent), None, only, count));
            }
            if parse_phrase_choice_at_head(
                &non_article_parser_word_refs(&fragment_tokens),
                PLAYER_TARGET_PREFIXES,
            )
            .is_some()
            {
                return Ok((Some(PlayerFilter::Any), None, only, count));
            }

            let mut target_filter_tokens = fragment_tokens.as_slice();
            if target_filter_tokens.first().is_some_and(|token| {
                token
                    .as_word()
                    .is_some_and(|word| parse_word_choice(word, TARGET_OR_TARGETS_WORDS).is_some())
            }) {
                target_filter_tokens = &target_filter_tokens[1..];
            }
            if target_filter_tokens.is_empty() {
                return Ok((None, None, only, count));
            }
            let source_exclusion_surface =
                target_filter_tokens
                    .windows(2)
                    .enumerate()
                    .find_map(|(index, prefix)| {
                        (prefix[0].is_word("other") && prefix[1].is_word("than"))
                            .then(|| {
                                source_reference_tail_prefix(&target_filter_tokens[index + 2..])
                            })
                            .flatten()
                            .and_then(|(consumed, surface)| {
                                (consumed == target_filter_tokens.len() - index - 2)
                                    .then_some(surface)
                            })
                    });
            let mut target_filter = parse_object_filter_permissive(target_filter_tokens, false)?;
            // The relation parser peels `that targets only a single ...`
            // away from the stack-spell filter before the ordinary source
            // exclusion stage is finalized. Recover the exact proper-name or
            // typed-source tail on the nested target filter: this remains the
            // executable source-identity predicate (`other`), while the
            // authored alias is presentation provenance only.
            if let Some(surface) = source_exclusion_surface {
                target_filter.other = true;
                target_filter.source_surface = Some(surface);
            }
            Ok((None, Some(target_filter), only, count))
        };

        if let Some(or_token_idx) = token_index_for_word(target_tokens, OR_WORD) {
            let left_tokens = trim_commas(&target_tokens[..or_token_idx]);
            let right_tokens = trim_commas(&target_tokens[or_token_idx + 1..]);
            let (left_player, left_object, left_only, left_count) =
                parse_target_fragment(&left_tokens)?;
            let (right_player, right_object, right_only, right_count) =
                parse_target_fragment(&right_tokens)?;
            let is_object_union = left_player.is_none()
                && right_player.is_none()
                && left_object.is_some()
                && right_object.is_some();
            target_player = left_player.or(right_player);
            target_object = if is_object_union {
                // Preserve an object-class union such as "creatures or
                // Vehicles you control" as one target relation. Picking one
                // side here silently broadened/narrowed both trigger matching
                // and event-derived target counts.
                Some(parse_object_filter_permissive(target_tokens, false)?)
            } else {
                left_object.or(right_object)
            };
            targets_only = left_only || right_only;
            target_count = relation_target_count.or(left_count).or(right_count);
            if target_player.is_some() && target_object.is_some() {
                filter.targets_any_of = true;
            }
        } else {
            let (parsed_player, parsed_object, parsed_only, parsed_count) =
                parse_target_fragment(target_tokens)?;
            target_player = parsed_player;
            target_object = parsed_object;
            targets_only = parsed_only;
            target_count = relation_target_count.or(parsed_count);
        }
    }

    // Object filters should not absorb trailing duration clauses such as
    // "... until this enchantment leaves the battlefield".
    if let Some(until_token_idx) = token_index_for_word(&base_tokens, UNTIL_WORD)
        && until_token_idx > 0
    {
        base_tokens.truncate(until_token_idx);
    }

    let not_on_battlefield = strip_not_on_battlefield_phrase(&mut base_tokens);

    // "<subject> with a <inner> attached to it" and "<subject> that's
    // enchanted by <inner>" both select the subject based on an attachment
    // it carries. Intercept before the attached-to-tail split claims the
    // attachment words as the subject's own attachment reference.
    let attachment_split = split_with_attached_object_filter(&base_tokens)
        .map(|(subject, inner)| (subject, inner, false))
        .or_else(|| {
            split_enchanted_by_object_filter(&base_tokens)
                .map(|(subject, inner)| (subject, inner, true))
        });
    if let Some((subject_tokens, inner_tokens, uses_enchanted_by_surface)) = attachment_split {
        let inner_other = inner_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        let mut inner = parse_object_filter_permissive(&inner_tokens, inner_other)?;
        inner.other |= inner_other;
        filter.with_attached_object = Some(Box::new(inner));
        if uses_enchanted_by_surface {
            filter.set_relative_attachment_state_surface(true);
        }
        base_tokens = subject_tokens;
    }

    if let Some((head_tokens, attached_to_tokens)) = split_attached_to_object_filter(&base_tokens) {
        let attached_to_words = non_article_parser_word_refs(&attached_to_tokens);
        if parse_phrase_whole(&attached_to_words, THAT_PLAYER_ATTACHMENT_TAIL).is_some() {
            filter.attached_to_player =
                Some(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)));
        } else {
            let attached_to = if matches!(attached_to_words.as_slice(), ["him"] | ["her"]) {
                ObjectFilter::source_with_surface(crate::target::SourceReferenceSurface::FullName(
                    attached_to_words[0].to_string(),
                ))
            } else {
                let mut attached = parse_object_filter_permissive(&attached_to_tokens, false)?;
                attached.set_plural_object_noun_surface(has_plural_object_noun_surface(
                    &attached_to_tokens,
                ));
                // `this creature` is both a source identity reference and a
                // typed object selector. Preserve the noun on the nested
                // filter so attachment legality does not widen to every
                // source object.
                if attached.source
                    && attached.card_types.is_empty()
                    && attached_to_words.len() == 2
                    && let Some(card_type) = parse_card_type(attached_to_words[1])
                {
                    attached.card_types.push(card_type);
                }
                attached
            };
            filter.attached_to_object = Some(Box::new(attached_to));
        }
        base_tokens = head_tokens;
    }

    // A chosen-object exclusion is an identity relation to the preceding
    // choice. Do not let the generic "other than <type>" pass reinterpret
    // the final noun as an excluded card type (for example, as
    // "noncreature creature").
    let base_words = parser_token_word_refs(&base_tokens);
    if let Some(exclusion) =
        parse_phrase_choice_anywhere(&base_words, CHOSEN_OBJECT_EXCLUSION_PHRASES).filter(
            |exclusion| {
                exclusion.phrase.first() != Some(&"and")
                    || parse_phrase_anywhere(&base_words[..exclusion.span.start], OTHER_THAN_PREFIX)
                        .is_some()
            },
        )
    {
        let start = token_index_after_word_prefix(&base_tokens, exclusion.span.start)
            .unwrap_or(base_tokens.len());
        let end = token_index_after_word_prefix(&base_tokens, exclusion.span.end)
            .unwrap_or(base_tokens.len());
        if start < end {
            let chosen_kind = exclusion.phrase.last().copied().unwrap_or("object");
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: TagKey::from(crate::cards::builders::CHOSEN_OBJECTS_TAG),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
            // For a direct `other than the chosen ...` exclusion this inert
            // surface preserves the chosen noun. In the coordinated
            // `other than <source> and the chosen ...` form, leave the slot
            // available for the independently parsed source identity.
            if exclusion.phrase.first() == Some(&"other") {
                filter.source_surface = Some(crate::target::SourceReferenceSurface::FullName(
                    format!("the chosen {chosen_kind}"),
                ));
            }
            base_tokens.drain(start..end);
        }
    }

    // "other than <source>" marks an exclusion, not an additional type
    // selector. Keep "other" and capture the source surface when available.
    let mut idx = 0usize;
    while idx + 2 < base_tokens.len() {
        if parse_phrase_whole(
            &non_article_parser_word_refs(&base_tokens[idx..idx + 2]),
            OTHER_THAN_PREFIX,
        )
        .is_none()
        {
            idx += 1;
            continue;
        }

        let tail_tokens = &base_tokens[idx + 2..];
        let Some((tail_token_len, surface)) = source_reference_tail_prefix(tail_tokens) else {
            idx += 1;
            continue;
        };

        filter.other = true;
        filter.source_surface.get_or_insert(surface);
        base_tokens.drain(idx..idx + 2 + tail_token_len);
    }

    // "other than Werewolves and Wolves" is an exclusion on the described
    // object class, not the source-relative "other" predicate.
    let mut idx = 0usize;
    while idx + 2 < base_tokens.len() {
        if parse_phrase_whole(
            &non_article_parser_word_refs(&base_tokens[idx..idx + 2]),
            OTHER_THAN_PREFIX,
        )
        .is_none()
        {
            idx += 1;
            continue;
        }

        let mut base_card_types = Vec::new();
        for token in &base_tokens[..idx] {
            for piece in token.parser_word_pieces() {
                if let Some(card_type) = parse_card_type(piece.text.as_str()) {
                    push_unique(&mut base_card_types, card_type);
                }
            }
        }

        let tail_tokens = &base_tokens[idx + 2..];
        if parse_phrase_choice_at_head(
            &non_article_parser_word_refs(tail_tokens),
            EXCLUSION_RELATION_IGNORED_PREFIXES,
        )
        .is_some()
        {
            idx += 1;
            continue;
        }
        let mut excluded_card_types = Vec::new();
        let mut excluded_subtypes = Vec::new();
        let mut excluded_supertypes = Vec::new();
        let mut excluded_colors = ColorSet::new();
        for token in tail_tokens {
            for piece in token.parser_word_pieces() {
                let word = piece.text.as_str();
                if is_article(word) || parse_word_choice(word, AND_OR_WORDS).is_some() {
                    continue;
                }
                if let Some(card_type) = parse_card_type(word) {
                    push_unique(&mut excluded_card_types, card_type);
                }
                if let Some(subtype) = parse_subtype_flexible(word) {
                    push_unique(&mut excluded_subtypes, subtype);
                }
                if let Some(supertype) = parse_supertype_word(word) {
                    push_unique(&mut excluded_supertypes, supertype);
                }
                if let Some(color) = parse_color(word) {
                    excluded_colors = excluded_colors.union(color);
                }
            }
        }

        let has_specific_exclusion = !excluded_subtypes.is_empty()
            || !excluded_supertypes.is_empty()
            || !excluded_colors.is_empty();
        let saw_exclusion = !excluded_card_types.is_empty() || has_specific_exclusion;
        if !saw_exclusion {
            idx += 1;
            continue;
        }

        for card_type in excluded_card_types {
            if has_specific_exclusion && slice_has(&base_card_types, &card_type) {
                continue;
            }
            push_unique(&mut filter.excluded_card_types, card_type);
        }
        for subtype in excluded_subtypes {
            push_unique(&mut filter.excluded_subtypes, subtype);
        }
        for supertype in excluded_supertypes {
            push_unique(&mut filter.excluded_supertypes, supertype);
        }
        filter.excluded_colors = filter.excluded_colors.union(excluded_colors);
        base_tokens.truncate(idx);
        break;
    }

    if let Some(mut disjunction) = parse_attached_reference_or_another_disjunction(&base_tokens)? {
        disjunction.attached_to_object = filter.attached_to_object.take();
        disjunction.attached_to_player = filter.attached_to_player.take();
        if target_player.is_some() || target_object.is_some() {
            disjunction = if targets_only {
                disjunction.targeting_only(target_player.take(), target_object.take())
            } else {
                disjunction.targeting(target_player.take(), target_object.take())
            };
            if let Some(count) = target_count {
                disjunction = disjunction.with_target_count(count);
            } else if targets_only {
                disjunction = disjunction.target_count_exact(1);
            }
        }
        return Ok(disjunction);
    }
    let mut segment_tokens = base_tokens.clone();

    let raw_words_with_articles = parser_token_word_refs(&base_tokens);
    let all_words_with_articles = word_refs_except(&raw_words_with_articles, &["instead"]);

    let map_non_article_index = |non_article_idx: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_idx {
                return Some(idx);
            }
            seen += 1;
        }
        None
    };

    let map_non_article_end = |non_article_end: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_end {
                return Some(idx);
            }
            seen += 1;
        }
        if seen == non_article_end {
            return Some(all_words_with_articles.len());
        }
        None
    };

    let mut all_words = non_article_word_refs(&all_words_with_articles);
    let has_tap_activated_ability = has_tap_activated_ability_phrase(&all_words);
    if parse_phrase_whole(
        &non_article_parser_word_refs(&base_tokens),
        ACTIVATED_ABILITY_WORDS,
    )
    .is_some()
    {
        return Ok(ObjectFilter::activated_ability());
    }
    if parse_phrase_whole(
        &non_article_parser_word_refs(&base_tokens),
        TRIGGERED_ABILITY_WORDS,
    )
    .is_some()
    {
        let mut filter = ObjectFilter::ability();
        filter.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
        return Ok(filter);
    }
    if parse_phrase_choice_whole(
        &non_article_parser_word_refs(&base_tokens),
        ACTIVATED_OR_TRIGGERED_ABILITY_PHRASES,
    )
    .is_some()
    {
        let mut triggered = ObjectFilter::ability();
        triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
        let mut filter = ObjectFilter::default();
        filter.any_of = vec![ObjectFilter::activated_ability(), triggered];
        return Ok(filter);
    }

    // Qualified stack-ability sets (for example, "all other activated and
    // triggered abilities you control") must retain both their stack-object
    // identity and the outer controller/reference qualifiers. The exact-shape
    // branches above intentionally return early, but a qualified shape needs
    // to continue through the ordinary relation parser below.
    let ability_words = non_article_parser_word_refs(&base_tokens);
    if parse_phrase_choice_anywhere(&ability_words, SPELL_AND_ABILITY_PHRASES).is_some() {
        filter.zone = Some(Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::SpellOrAbility);
        filter.has_mana_cost = false;
        filter.set_conjunctive_set_surface(true);
    } else if parse_phrase_choice_anywhere(&ability_words, ACTIVATED_OR_TRIGGERED_ABILITY_PHRASES)
        .is_some()
    {
        let mut triggered = ObjectFilter::ability();
        triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
        filter.zone = Some(Zone::Stack);
        filter.any_of = vec![ObjectFilter::activated_ability(), triggered];
    } else if (parse_phrase_anywhere(&ability_words, &["activated", "ability"]).is_some()
        || parse_phrase_anywhere(&ability_words, &["activated", "abilities"]).is_some())
        && (!has_tap_activated_ability
            || ability_words.starts_with(&["activated", "ability"])
            || ability_words.starts_with(&["activated", "abilities"]))
    {
        filter.zone = Some(Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::ActivatedAbility);
    } else if parse_phrase_anywhere(&ability_words, &["triggered", "ability"]).is_some()
        || parse_phrase_anywhere(&ability_words, &["triggered", "abilities"]).is_some()
    {
        filter.zone = Some(Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
    }
    if parse_phrase_choice_whole(
        &non_article_parser_word_refs(&base_tokens),
        REST_REVEALED_OBJECT_PHRASES,
    )
    .is_some()
    {
        return Ok(ObjectFilter::tagged("rest"));
    }
    if let Some(filter) = parse_permanent_or_suspended_card_disjunction(&base_tokens) {
        return Ok(filter);
    }

    try_apply_distinct_powers_clause(&mut filter, &mut all_words);
    try_apply_distinct_mana_values_clause(&mut filter, &mut all_words);
    try_apply_distinct_creature_types_clause(&mut filter, &mut all_words);
    try_apply_no_shared_creature_type_with_your_creatures_or_graveyard_clause(
        &mut filter,
        &mut all_words,
    );
    try_apply_no_shared_creature_type_with_chosen_creature_clause(&mut filter, &mut all_words);
    try_apply_shared_creature_type_with_source_clause(&mut filter, &mut all_words);

    try_apply_could_be_targeted_by_that_spell_clause(&mut filter, &mut all_words);

    try_apply_blocked_or_was_blocked_by_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    )?;

    // "that were put there from the battlefield this turn" means the card entered
    // a graveyard from the battlefield this turn.
    try_apply_put_there_from_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "put there from their library this turn" is object-specific zone-change
    // history. Consume it before the ordinary zone parser can turn the
    // referenced library into a second current-zone union arm.
    try_apply_put_there_from_their_library_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "legendary or Rat card" (Nashi, Moon's Legacy) is a supertype/subtype disjunction.
    // We parse it by collecting both selectors and then expanding into an `any_of` filter
    // after the normal pass so other shared qualifiers (zone/owner/etc.) are preserved.
    let legendary_or_subtype = parse_phrase_anywhere(&all_words, LEGENDARY_OR_PREFIX)
        .and_then(|fact| all_words.get(fact.span.end).copied())
        .and_then(parse_subtype_word);

    // "in a graveyard that was put there from anywhere this turn" (Reenact the Crime)
    // means the card entered a graveyard this turn.
    try_apply_put_there_from_anywhere_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // A zone-qualified "put there this turn" clause has the same executable
    // graveyard-entry history as the explicit "from anywhere" surface.
    try_apply_put_there_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);

    // "... graveyard from the battlefield this turn" means the card entered a graveyard
    // from the battlefield this turn.
    try_apply_graveyard_from_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // Preserve the source-relative history used by leaves-the-battlefield abilities such as
    // "a creature put onto the battlefield with this enchantment". This is an object-identity
    // relation, not a type clause; consume it before the ordinary noun pass can flatten the
    // source noun into the selected object's card types.
    try_apply_put_onto_battlefield_with_source_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // Token provenance is a source-instance relationship. Consume the source
    // noun before the ordinary type pass can misread "this enchantment" as a
    // requirement that the selected token itself be an enchantment.
    try_apply_created_with_source_clause(&mut filter, &mut all_words, &mut segment_tokens);

    // Preserve negative turn history such as "creatures that didn't attack or
    // enter this turn" before the ordinary word pass can discard the
    // conjunctive predicate.
    try_apply_didnt_enter_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "... entered the battlefield ... this turn" marks a battlefield entry this turn.
    try_apply_entered_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    try_apply_drawn_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);

    try_apply_counters_put_on_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);

    try_apply_ability_activated_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);
    try_apply_not_enchanted_clause(&mut filter, &mut all_words, &mut segment_tokens);

    // Preserve damage history in ordinary object selectors such as "target
    // creature that was dealt damage this turn". This is a runtime legality
    // constraint, not disposable surface text.
    try_apply_was_dealt_damage_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);
    try_apply_dealt_damage_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);

    // A prior player-or-planeswalker target can be referenced through either
    // the chosen player or the chosen planeswalker's controller. Remove that
    // relation before its `or planeswalker` surface is mistaken for a second
    // selected card type.
    try_apply_target_player_or_planeswalker_controller_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    if parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        BLOCKED_BY_TAGGED_OBJECT_PHRASES,
    )
    .is_some()
    {
        filter.blocked = true;
        filter.blocked_by = Some(crate::filter::ObjectRef::Tagged(TagKey::from(IT_TAG)));
    }

    // Avoid treating reference phrases like "... with mana value less than or equal to the number
    // of charge counters on this artifact" as additional type selectors on the filtered object.
    // (Aether Vial: "put a creature card with mana value equal to the number of charge counters
    // on this artifact from your hand onto the battlefield.")
    let _ = try_apply_mana_value_counters_on_source_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    try_apply_attached_exclusion_phrases(&mut filter, &mut all_words);
    let exclude_basic_land_cards =
        strip_other_than_basic_land_cards_clause(&mut all_words, &mut segment_tokens);

    let _ = try_apply_pt_literal_prefix(&mut filter, &mut all_words);

    strip_object_filter_leading_prefixes(&mut all_words);

    let _ = try_apply_required_both_colors_clause(&mut filter, &mut all_words);

    let _ = try_apply_not_all_colors_clause(&mut filter, &mut all_words);

    let _ = try_apply_not_exactly_two_colors_clause(&mut filter, &mut all_words);

    let _ = try_apply_exactly_two_colors_clause(&mut filter, &mut all_words);

    strip_be_put_on_reference_prefix(&mut all_words, &segment_tokens);

    let _ = try_apply_leading_tagged_reference_prefix(&mut filter, &mut all_words);

    let _ = try_apply_target_choice_attribution_reference(&mut filter, &mut all_words);

    let _ = try_apply_entered_since_your_last_turn_ended_clause(&mut filter, &mut all_words);

    strip_object_filter_face_state_words(&mut filter, &mut all_words);

    if parse_phrase_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        ENTERED_THIS_TURN_UNSUPPORTED_PHRASE,
    )
    .is_some()
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported entered-this-turn object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    let has_counter_state_or_clause = parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        TAGGED_COUNTER_STATE_DISJUNCTION_PHRASES,
    )
    .is_some();
    let has_supported_suspended_disjunction = parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        SUSPENDED_CARD_DISJUNCTION_PHRASES,
    )
    .is_some();
    if has_counter_state_or_clause && !has_supported_suspended_disjunction {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-state object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    strip_single_graveyard_phrase(&mut filter, &mut all_words);

    let _ = try_apply_not_named_clause(
        &mut filter,
        &mut all_words,
        &all_words_with_articles,
        &map_non_article_index,
        &map_non_article_end,
        &base_tokens,
    )?;

    let _ = try_apply_named_clause(
        &mut filter,
        &mut all_words,
        &all_words_with_articles,
        &map_non_article_index,
        &map_non_article_end,
    )?;

    // "with the chosen name" — a runtime back-reference to a previously
    // chosen card name, not a literal name.
    if filter.name.is_none() {
        for phrase in [
            ["with", "chosen", "name"].as_slice(),
            ["of", "chosen", "name"].as_slice(),
        ] {
            if let Some(start) = all_words
                .windows(phrase.len())
                .position(|window| window == phrase)
            {
                filter.name = Some("{chosen name}".to_string());
                naming_and_reference::remove_word_range(
                    &mut all_words,
                    start,
                    start + phrase.len(),
                );
                break;
            }
        }
    }

    let _ = try_apply_color_count_phrase(&mut filter, &mut all_words)?;
    let _ = try_apply_sticker_filter_clause(&mut filter, &mut all_words);
    let has_power_or_toughness_clause = parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        POWER_OR_TOUGHNESS_PHRASES,
    )
    .is_some();
    if has_power_or_toughness_clause
        && !all_words
            .iter()
            .any(|word| parse_word_choice(word, SPELL_OR_SPELLS_WORDS).is_some())
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }

    // A sharing clause compares the candidate's characteristics with a
    // separately filtered object set. Parse and remove the entire relation
    // before the ordinary noun/controller passes can leak the comparison
    // object's identity into the candidate filter.
    let _ = try_apply_shared_characteristic_relation_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    )?;

    let explicit_card_rhs_ranges = filter_comparison_rhs_ranges(&all_words)?;
    if contains_explicit_card_noun(&all_words, &explicit_card_rhs_ranges) {
        filter.set_explicit_card_noun(true);
    }

    let reference_stage =
        apply_reference_and_tag_stage(&mut filter, &mut all_words, &mut segment_tokens);
    if reference_stage.early_return {
        return Ok(filter);
    }
    let source_linked_exile_reference = reference_stage.source_linked_exile_reference;

    let references_target_player = parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        TARGET_PLAYER_REFERENCE_PHRASES,
    )
    .is_some();
    let references_target_opponent = parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        TARGET_OPPONENT_REFERENCE_PHRASES,
    )
    .is_some();
    let pronoun_player_filter = if references_target_opponent {
        PlayerFilter::target_opponent()
    } else if references_target_player {
        PlayerFilter::target_player()
    } else {
        PlayerFilter::IteratedPlayer
    };
    let comparison_rhs_ranges = filter_comparison_rhs_ranges(&all_words)?;

    let outer_filter_words = all_words
        .iter()
        .enumerate()
        .map(|(idx, word)| {
            if word_is_in_ranges(idx, &comparison_rhs_ranges) {
                "__comparison_rhs__"
            } else {
                *word
            }
        })
        .collect::<Vec<_>>();
    let has_attack_destination_planeswalker_clause = all_words_with_articles
        .iter()
        .position(|word| matches!(*word, "attacking" | "attacks"))
        .is_some_and(|attacking_idx| {
            all_words_with_articles[..attacking_idx]
                .iter()
                .any(|word| matches!(*word, "creature" | "creatures"))
                && all_words_with_articles[attacking_idx + 1..]
                    .iter()
                    .any(|word| matches!(*word, "planeswalker" | "planeswalkers"))
        });
    if let Some(attacking_filter) =
        attacking_player_filter_from_words(&outer_filter_words, &pronoun_player_filter)
    {
        filter.attacking_player_or_planeswalker_controlled_by = Some(attacking_filter);
        filter.attacking_player_only = !outer_filter_words.contains(&"planeswalker");
    }

    let is_outer_tagged_spell_reference_at = |idx: usize| {
        outer_filter_words
            .get(idx.wrapping_sub(1))
            .is_some_and(|prev| parse_word_choice(prev, TAGGED_SPELL_REFERENCE_WORDS).is_some())
    };
    let contains_unqualified_spell_word =
        outer_filter_words.iter().enumerate().any(|(idx, word)| {
            parse_word_choice(word, SPELL_OR_SPELLS_WORDS).is_some()
                && !is_outer_tagged_spell_reference_at(idx)
        });
    let is_tagged_spell_reference_at = |idx: usize| {
        all_words
            .get(idx.wrapping_sub(1))
            .is_some_and(|prev| parse_word_choice(prev, TAGGED_SPELL_REFERENCE_WORDS).is_some())
    };
    let mentions_ability_word = outer_filter_words
        .iter()
        .any(|word| parse_word_choice(word, ABILITY_OR_ABILITIES_WORDS).is_some());
    if contains_unqualified_spell_word && !mentions_ability_word {
        filter.has_mana_cost = true;
    }
    // Both current and older Oracle surfaces narrow a spell/permanent filter
    // to objects whose printed mana cost includes an {X} symbol.
    let has_x_in_cost_surface =
        parse_phrase_anywhere(&outer_filter_words, &["mana", "cost", "that", "contains"]).is_some()
            || [
                &["with", "x", "in", "its", "mana", "cost"][..],
                &["with", "x", "in", "their", "mana", "cost"][..],
                &["with", "x", "in", "its", "mana", "costs"][..],
                &["with", "x", "in", "their", "mana", "costs"][..],
            ]
            .iter()
            .any(|phrase| parse_phrase_anywhere(&outer_filter_words, phrase).is_some());
    if has_x_in_cost_surface {
        filter.has_x_in_cost = true;
    }

    if !all_words.is_empty() {
        let mut idx = 0usize;
        while idx < all_words.len() {
            if word_is_in_ranges(idx, &comparison_rhs_ranges) {
                idx += 1;
                continue;
            }
            let slice = &all_words[idx..];
            if relation_clause_is_inside_aggregate_scope(&all_words, idx) {
                idx += 1;
                continue;
            }
            if let Some(consumed) =
                try_apply_neither_owned_nor_controlled_clause(&mut filter, slice)
            {
                idx += consumed;
                continue;
            }
            if let Some(consumed) =
                try_apply_joint_owner_controller_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) = try_apply_chosen_player_graveyard_clause(&mut filter, slice) {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) =
                try_apply_negated_you_relation_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) =
                try_apply_player_relation_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) =
                try_apply_passive_player_relation_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            idx += 1;
        }
    }

    let mut with_idx = 0usize;
    while with_idx + 1 < all_words.len() {
        if word_is_in_ranges(with_idx, &comparison_rhs_ranges) {
            with_idx += 1;
            continue;
        }
        if all_words[with_idx] != WITH_WORD {
            with_idx += 1;
            continue;
        }

        if let Some(consumed) = try_apply_with_clause_tail(&mut filter, &all_words[with_idx + 1..])
        {
            with_idx += 1 + consumed;
            continue;
        }

        with_idx += 1;
    }

    let mut has_idx = 0usize;
    while has_idx + 1 < all_words.len() {
        if word_is_in_ranges(has_idx, &comparison_rhs_ranges) {
            has_idx += 1;
            continue;
        }
        if parse_word_choice(all_words[has_idx], HAS_HAVE_WORDS).is_none() {
            has_idx += 1;
            continue;
        }
        if filter.with_counter.is_none()
            && let Some((counter_constraint, consumed)) =
                parse_filter_counter_constraint_words(&all_words[has_idx + 1..])
        {
            filter.with_counter = Some(counter_constraint);
            has_idx += 1 + consumed;
            continue;
        }
        if let Some((constraints, connective, consumed)) =
            parse_filter_keyword_constraint_list_words(&all_words[has_idx + 1..])
        {
            // "that doesn't have <keywords>" — the negation word precedes the
            // has/have word and inverts every list item (has NONE of them).
            let negated = (has_idx > 0
                && matches!(
                    all_words[has_idx - 1],
                    "doesn't" | "doesnt" | "don't" | "dont"
                ))
                || (has_idx > 1
                    && all_words[has_idx - 1] == "not"
                    && matches!(all_words[has_idx - 2], "does" | "do"));
            if negated {
                for constraint in constraints {
                    apply_filter_keyword_constraint(&mut filter, constraint, true);
                }
            } else if constraints.len() > 1
                && filter.any_of.is_empty()
                && !matches!(connective, FilterKeywordListConnective::And)
            {
                // A disjunctive list ("first strike, double strike, and/or
                // haste") matches objects with AT LEAST ONE listed keyword.
                filter.any_of = constraints
                    .into_iter()
                    .map(|constraint| {
                        let mut branch = ObjectFilter::default();
                        apply_filter_keyword_constraint(&mut branch, constraint, false);
                        branch
                    })
                    .collect();
                if matches!(connective, FilterKeywordListConnective::AndOr) {
                    filter.set_union_connective(ObjectFilterUnionConnective::AndOr);
                }
            } else {
                for constraint in constraints {
                    apply_filter_keyword_constraint(&mut filter, constraint, false);
                }
            }
            has_idx += 1 + consumed;
            continue;
        }
        has_idx += 1;
    }

    let mut without_idx = 0usize;
    while without_idx + 1 < all_words.len() {
        if word_is_in_ranges(without_idx, &comparison_rhs_ranges) {
            without_idx += 1;
            continue;
        }
        if all_words[without_idx] != WITHOUT_WORD {
            without_idx += 1;
            continue;
        }

        if let Some(consumed) =
            try_apply_without_clause_tail(&mut filter, &all_words[without_idx + 1..])
        {
            without_idx += 1 + consumed;
            continue;
        }

        without_idx += 1;
    }

    if has_tap_activated_ability {
        filter.has_tap_activated_ability = true;
    }

    let mut referenced_zones = Vec::new();
    for idx in 0..all_words.len() {
        if word_is_in_ranges(idx, &comparison_rhs_ranges) {
            continue;
        }
        if let Some(zone) = parse_zone_word(all_words[idx]) {
            if !slice_has(&referenced_zones, &zone) {
                referenced_zones.push(zone);
            }
            let is_reference_zone_for_spell = if contains_unqualified_spell_word {
                idx > 0
                    && matches!(
                        all_words[idx - 1],
                        "controller"
                            | "controllers"
                            | "owner"
                            | "owners"
                            | "its"
                            | "their"
                            | "that"
                            | "this"
                    )
            } else {
                false
            };
            if is_reference_zone_for_spell {
                continue;
            }
            if filter.zone.is_none() {
                filter.zone = Some(zone);
            }
            if idx > 0 {
                match all_words[idx - 1] {
                    "your" => {
                        filter.owner = Some(PlayerFilter::You);
                    }
                    "opponent" | "opponents" => {
                        filter.owner = Some(PlayerFilter::Opponent);
                    }
                    "their" => {
                        filter.owner = Some(pronoun_player_filter.clone());
                    }
                    _ => {}
                }
            }
            if idx > 1 {
                let owner_pair = (all_words[idx - 2], all_words[idx - 1]);
                match owner_pair {
                    ("defending", "player") | ("defending", "players") => {
                        filter.owner = Some(PlayerFilter::Defending);
                    }
                    ("target", "player") | ("target", "players") => {
                        filter.owner = Some(PlayerFilter::target_player());
                    }
                    ("target", "opponent") | ("target", "opponents") => {
                        filter.owner = Some(PlayerFilter::target_opponent());
                    }
                    ("that", "player") | ("that", "players") => {
                        filter.owner = Some(PlayerFilter::IteratedPlayer);
                    }
                    _ => {}
                }
            }
        }
    }
    if referenced_zones.len() > 1 && filter.any_of.is_empty() {
        filter.zone = None;
        filter.any_of = referenced_zones
            .into_iter()
            .map(|zone| ObjectFilter::default().in_zone(zone))
            .collect();
    }

    let clause_words = all_words.clone();
    for idx in 0..all_words.len() {
        let value_tokens = match all_words.get(idx..) {
            Some(["total", "power", "and", "toughness", rest @ ..])
            | Some(["power", "and", "toughness", "totaling", rest @ ..]) => rest,
            _ => continue,
        };
        let Some((cmp, _consumed)) =
            parse_filter_comparison_tokens("power", value_tokens, &clause_words)?
        else {
            continue;
        };
        filter.total_power_toughness = Some(cmp);
        break;
    }

    for idx in 0..all_words.len() {
        let (is_base_reference, pt_word_idx) = if idx + 4 < all_words.len()
            && parse_phrase_at_head(&all_words[idx..], BASE_POWER_TOUGHNESS_PREFIX).is_some()
        {
            (true, idx + 4)
        } else if idx + 3 < all_words.len()
            && parse_phrase_at_head(&all_words[idx..], POWER_TOUGHNESS_PREFIX).is_some()
            && (idx == 0 || all_words[idx - 1] != BASE_WORD)
        {
            (false, idx + 3)
        } else {
            continue;
        };

        if let Ok((power, toughness)) = parse_pt_modifier(all_words[pt_word_idx]) {
            filter.power = Some(crate::filter::Comparison::Equal(power));
            filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
            filter.power_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
            filter.toughness_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
        }
    }

    let mut idx = 0usize;
    while idx < all_words.len() {
        let axis = if all_words[idx] == POWER_WORD {
            Some("power")
        } else if all_words[idx] == TOUGHNESS_WORD {
            Some("toughness")
        } else if idx + 1 < all_words.len()
            && parse_phrase_at_head(&all_words[idx..], MANA_VALUE_PREFIX).is_some()
        {
            Some("mana value")
        } else {
            None
        };
        let Some(axis) = axis else {
            idx += 1;
            continue;
        };
        let is_base_reference = idx > 0 && all_words[idx - 1] == BASE_WORD;

        let axis_word_count =
            usize::from(parse_phrase_at_head(&all_words[idx..], MANA_VALUE_PREFIX).is_some()) + 1;
        let value_tokens = if idx + axis_word_count < all_words.len() {
            &all_words[idx + axis_word_count..]
        } else {
            &[]
        };
        if axis == POWER_WORD && value_tokens.first().is_some_and(|word| *word == AND_WORD) {
            idx += 1;
            continue;
        }
        if axis == TOUGHNESS_WORD
            && idx >= 3
            && matches!(
                &all_words[idx - 3..idx],
                ["total", "power", "and"] | ["base", "power", "and"] | ["power", "and", "base"]
            )
        {
            idx += 1;
            continue;
        }
        if (axis == TOUGHNESS_WORD
            && parse_phrase_choice_at_head(&all_words[idx..], TOUGHNESS_GREATER_THAN_POWER_PHRASES)
                .is_some())
            || (axis == POWER_WORD
                && parse_phrase_choice_at_head(
                    &all_words[idx..],
                    POWER_GREATER_THAN_TOUGHNESS_PHRASES,
                )
                .is_some())
            || parse_phrase_choice_at_head(&all_words[idx..], POWER_TOUGHNESS_NOT_EQUAL_PHRASES)
                .is_some()
        {
            idx += 1;
            continue;
        }
        let Some((cmp, consumed)) =
            parse_filter_comparison_tokens(axis, value_tokens, &clause_words)?
        else {
            idx += 1;
            continue;
        };

        match axis {
            "power" => {
                filter.power = Some(cmp);
                filter.power_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "toughness" => {
                filter.toughness = Some(cmp);
                filter.toughness_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "mana value" => filter.mana_value = Some(cmp),
            _ => {}
        }
        idx += axis_word_count + consumed;
    }

    apply_parity_filter_phrases(&clause_words, &mut filter);

    if parse_phrase_anywhere(&clause_words, POWER_GREATER_THAN_BASE_POWER_PHRASE).is_some() {
        filter.power_greater_than_base_power = true;
    }
    if parse_phrase_choice_anywhere(&clause_words, TOUGHNESS_GREATER_THAN_POWER_PHRASES).is_some() {
        let relation = crate::filter::PowerToughnessRelation::ToughnessGreaterThanPower;
        filter.power_toughness_relation = Some(relation);
        clear_redundant_power_toughness_axis_filter(&mut filter, relation);
    } else if parse_phrase_choice_anywhere(&clause_words, POWER_GREATER_THAN_TOUGHNESS_PHRASES)
        .is_some()
    {
        let relation = crate::filter::PowerToughnessRelation::PowerGreaterThanToughness;
        filter.power_toughness_relation = Some(relation);
        clear_redundant_power_toughness_axis_filter(&mut filter, relation);
    } else if parse_phrase_choice_anywhere(&clause_words, POWER_TOUGHNESS_NOT_EQUAL_PHRASES)
        .is_some()
    {
        filter.power_toughness_relation = Some(crate::filter::PowerToughnessRelation::NotEqual);
    }

    let mut saw_permanent = false;
    let mut saw_spell = false;
    let mut saw_permanent_type = false;

    let mut saw_subtype = false;
    let mut negated_word_indices = std::collections::HashSet::new();
    let mut negated_historic_indices = std::collections::HashSet::new();
    let mut has_coordinated_negated_characteristic_list = false;
    let is_text_negation_word = |word: &str| parse_word_choice(word, TEXT_NEGATION_WORDS).is_some();
    for idx in 0..all_words.len().saturating_sub(1) {
        if word_is_in_ranges(idx, &comparison_rhs_ranges) {
            continue;
        }
        if all_words[idx] != NON_WORD {
            continue;
        }
        let next = all_words[idx + 1];
        if is_outlaw_word(next) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(card_type) = parse_card_type(next)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(idx + 1);
        }
        if next == ATTACKING_WORD {
            filter.nonattacking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == BLOCKING_WORD {
            filter.nonblocking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == BLOCKED_WORD {
            filter.unblocked = true;
            negated_word_indices.insert(idx + 1);
        }
        if parse_word_choice(next, COMMANDER_OR_COMMANDERS_WORDS).is_some() {
            filter.noncommander = true;
            negated_word_indices.insert(idx + 1);
        }
        if let Some(color) = parse_color(next) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(subtype) = parse_subtype_flexible(next)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(idx + 1);
        }
    }
    for idx in 0..all_words.len() {
        if word_is_in_ranges(idx, &comparison_rhs_ranges) {
            continue;
        }
        if !is_text_negation_word(all_words[idx]) {
            continue;
        }
        let mut target_idx = idx + 1;
        if target_idx >= all_words.len() {
            continue;
        }
        if is_article(all_words[target_idx]) {
            target_idx += 1;
            if target_idx >= all_words.len() {
                continue;
            }
        }

        let negated_word = all_words[target_idx];
        if negated_word == ATTACKING_WORD {
            filter.nonattacking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == BLOCKING_WORD {
            filter.nonblocking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == BLOCKED_WORD {
            filter.unblocked = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == HISTORIC_WORD {
            filter.nonhistoric = true;
            negated_historic_indices.insert(target_idx);
        }
        if parse_word_choice(negated_word, COMMANDER_OR_COMMANDERS_WORDS).is_some() {
            filter.noncommander = true;
            negated_word_indices.insert(target_idx);
        }
        if let Some(card_type) = parse_card_type(negated_word)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(target_idx);
        }
        if let Some(supertype) = parse_supertype_word(negated_word)
            && !slice_has(&filter.excluded_supertypes, &supertype)
        {
            filter.excluded_supertypes.push(supertype);
            negated_word_indices.insert(target_idx);
        }
        if let Some(color) = parse_color(negated_word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(target_idx);
        }
        if let Some(subtype) = parse_subtype_flexible(negated_word)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(target_idx);
        }

        // A single negated copula scopes over the entire coordinated type
        // list: “isn't an Insect, Rat, Spider, or Squirrel” excludes every
        // listed subtype, not just the first one. Token punctuation has
        // already been removed here, so walk through conjunctions/articles
        // until the first word that is not another characteristic.
        let mut coordinated_characteristic_count = usize::from(
            parse_card_type(negated_word).is_some()
                || parse_supertype_word(negated_word).is_some()
                || parse_color(negated_word).is_some()
                || parse_subtype_flexible(negated_word).is_some(),
        );
        let mut list_idx = target_idx + 1;
        while list_idx < all_words.len() {
            let word = all_words[list_idx];
            if matches!(word, "and" | "or" | "and/or") || is_article(word) {
                list_idx += 1;
                continue;
            }
            let mut recognized = false;
            if let Some(card_type) = parse_card_type(word) {
                push_unique(&mut filter.excluded_card_types, card_type);
                recognized = true;
            }
            if let Some(supertype) = parse_supertype_word(word) {
                push_unique(&mut filter.excluded_supertypes, supertype);
                recognized = true;
            }
            if let Some(color) = parse_color(word) {
                filter.excluded_colors = filter.excluded_colors.union(color);
                recognized = true;
            }
            if let Some(subtype) = parse_subtype_flexible(word) {
                push_unique(&mut filter.excluded_subtypes, subtype);
                recognized = true;
            }
            if !recognized {
                break;
            }
            coordinated_characteristic_count += 1;
            negated_word_indices.insert(list_idx);
            list_idx += 1;
        }
        has_coordinated_negated_characteristic_list |= coordinated_characteristic_count > 1;
    }
    for idx in 0..all_words.len().saturating_sub(1) {
        if word_is_in_ranges(idx, &comparison_rhs_ranges) {
            continue;
        }
        if parse_phrase_whole(&all_words[idx..idx + 2], NOT_HISTORIC_PHRASE).is_some() {
            filter.nonhistoric = true;
            negated_historic_indices.insert(idx + 1);
        }
    }

    let excluded_chosen_type_indices: std::collections::HashSet<usize> =
        EXCLUDED_CHOSEN_TYPE_PHRASES
            .iter()
            .filter_map(|phrase| {
                parse_phrase_anywhere(&all_words, phrase)
                    .map(|fact| fact.span.end.saturating_sub(2))
            })
            .collect();
    if EXCLUDED_TYPE_CHOSEN_THIS_WAY_PHRASES
        .iter()
        .any(|phrase| parse_phrase_anywhere(&all_words, phrase).is_some())
    {
        filter.excluded_any_chosen_creature_type = true;
        filter.set_chosen_type_this_way_surface(true);
    }

    if parse_phrase_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        ATTACKED_THIS_TURN_PHRASE,
    )
    .is_some()
    {
        filter.attacked_this_turn = true;
    }

    let blocked_this_turn_word_indices = all_words
        .windows(3)
        .enumerate()
        .filter_map(|(idx, window)| {
            (window == ["blocked", "this", "turn"]
                && !idx
                    .checked_sub(1)
                    .and_then(|previous| all_words.get(previous))
                    .is_some_and(|word| matches!(*word, "was" | "became" | "is")))
            .then_some(idx)
        })
        .collect::<std::collections::HashSet<_>>();
    if !blocked_this_turn_word_indices.is_empty() {
        filter.blocked_this_turn = true;
    }

    for negated_phrase in [
        ["didn't", "attack", "this", "turn"],
        ["didnt", "attack", "this", "turn"],
    ] {
        if parse_phrase_anywhere(
            &non_article_parser_word_refs(&segment_tokens),
            &negated_phrase,
        )
        .is_some()
        {
            filter.didnt_attack_this_turn = true;
            filter.attacked_this_turn = false;
        }
    }

    let basic_land_type_basic_indices = all_words
        .windows(3)
        .enumerate()
        .filter_map(|(idx, window)| {
            (window[0] == "basic" && window[1] == "land" && matches!(window[2], "type" | "types"))
                .then_some(idx)
        })
        .collect::<std::collections::HashSet<_>>();

    for (idx, word) in all_words.iter().enumerate() {
        let idx: usize = idx;
        if word_is_in_ranges(idx, &comparison_rhs_ranges) {
            continue;
        }
        let is_negated_word = set_has(&negated_word_indices, &idx);
        match *word {
            "permanent" | "permanents" => saw_permanent = true,
            "spell" | "spells" => {
                if !is_tagged_spell_reference_at(idx) {
                    saw_spell = true;
                }
            }
            word if word == CHOSEN_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == COLOR_WORD) =>
            {
                filter.chosen_color = true;
            }
            word if word == THAT_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == COLOR_WORD) =>
            {
                // A demonstrative color after a color choice ("creatures of
                // that color") refers to the source program's chosen color.
                filter.chosen_color = true;
            }
            word if word == CHOSEN_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == TYPE_WORD) =>
            {
                if set_has(&excluded_chosen_type_indices, &idx) {
                    filter.excluded_chosen_creature_type = true;
                } else {
                    filter.chosen_creature_type = true;
                }
            }
            word if word == THAT_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == TYPE_WORD) =>
            {
                filter.chosen_creature_type = true;
            }
            word if word == NONCHOSEN_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == TYPE_WORD) =>
            {
                filter.excluded_chosen_creature_type = true;
            }
            "token" | "tokens" => filter.token = true,
            "nontoken" => filter.nontoken = true,
            "foretold" if !is_negated_word => filter.foretold = true,
            "other" => filter.other = true,
            "tapped" => filter.tapped = true,
            "untapped" => filter.untapped = true,
            "attacking" if !is_negated_word => filter.attacking = true,
            "nonattacking" => filter.nonattacking = true,
            // A bare "equipped" adjective not consumed by the attached-to
            // reference paths is the generic has-Equipment state.
            // NOTE(2026-07-25): a copula guard (skip when preceded by
            // is/are/was) was tried for Enkira's "As long as Enkira is
            // equipped, it must be blocked" and REVERTED — with the guard the
            // line HARD-FAILS ("parser does not yet support line family"),
            // meaning the predicate route that used to claim it is gone;
            // find that regression before re-adding the guard.
            "equipped" if !is_negated_word => {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: TagKey::from("equipped"),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            }
            "blocking" if !is_negated_word => filter.blocking = true,
            "nonblocking" => filter.nonblocking = true,
            "blocked" if !is_negated_word && !set_has(&blocked_this_turn_word_indices, &idx) => {
                filter.blocked = true;
            }
            "unblocked" if !is_negated_word => filter.unblocked = true,
            "commander" | "commanders" => {
                let prev = idx.checked_sub(1).and_then(|i| all_words.get(i)).copied();
                let prev2 = idx.checked_sub(2).and_then(|i| all_words.get(i)).copied();
                let negated_by_phrase = prev.is_some_and(is_text_negation_word)
                    || (prev.is_some_and(is_article) && prev2.is_some_and(is_text_negation_word));
                if is_negated_word || negated_by_phrase {
                    filter.noncommander = true;
                } else {
                    filter.is_commander = true;
                    match prev {
                        Some("your") => filter.owner = Some(PlayerFilter::You),
                        Some("opponent") | Some("opponents") => {
                            filter.owner = Some(PlayerFilter::Opponent);
                        }
                        Some("their") => filter.owner = Some(pronoun_player_filter.clone()),
                        _ => {}
                    }
                }
            }
            "noncommander" | "noncommanders" => filter.noncommander = true,
            "basic" if set_has(&basic_land_type_basic_indices, &idx) => {
                filter.has_basic_land_type = true;
            }
            "nonbasic" => {
                if all_words.get(idx + 1).is_some_and(|word| *word == "land")
                    && all_words.get(idx + 2).is_some_and(|word| *word == "type")
                {
                    filter.has_nonbasic_land_type = true;
                    continue;
                }
                filter = filter.without_supertype(Supertype::Basic);
            }
            "colorless" => filter.colorless = true,
            "multicolored" => filter.multicolored = true,
            "monocolored" => filter.monocolored = true,
            "nonhistoric" => filter.nonhistoric = true,
            "historic" if !set_has(&negated_historic_indices, &idx) => filter.historic = true,
            "modified" if !is_negated_word => filter.modified = true,
            "suspected" if !is_negated_word => filter.suspected = true,
            _ => {}
        }

        if is_non_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            continue;
        }

        if set_has(&negated_word_indices, &idx) {
            continue;
        }

        if is_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.subtypes);
            saw_subtype = true;
            continue;
        }

        let mut parsed_explicit_exclusion = false;
        if let Some(card_type) = parse_non_type(word) {
            push_unique(&mut filter.excluded_card_types, card_type);
            parsed_explicit_exclusion = true;
        }

        if let Some(supertype) = parse_non_supertype(word) {
            if !slice_has(&filter.excluded_supertypes, &supertype) {
                filter.excluded_supertypes.push(supertype);
            }
            parsed_explicit_exclusion = true;
        }

        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            parsed_explicit_exclusion = true;
        }
        if let Some(subtype) = parse_non_subtype(word) {
            if !slice_has(&filter.excluded_subtypes, &subtype) {
                filter.excluded_subtypes.push(subtype);
            }
            parsed_explicit_exclusion = true;
        }

        // Flexible positive characteristic parsers deliberately accept some
        // prefixed surfaces. Once this word has been recognized as an
        // explicit `non-*` exclusion, do not feed the same word through those
        // positive parsers as well: `non-Equipment` must not require and
        // exclude Equipment simultaneously.
        if parsed_explicit_exclusion {
            continue;
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }

        if let Some(supertype) = parse_supertype_word(word)
            && !set_has(&basic_land_type_basic_indices, &idx)
            && !slice_has(&filter.supertypes, &supertype)
        {
            filter.supertypes.push(supertype);
        }

        if let Some(card_type) = parse_card_type(word) {
            filter.set_explicit_card_type_noun(Some(card_type));
            push_unique(&mut filter.card_types, card_type);
            if is_permanent_type(card_type) {
                saw_permanent_type = true;
            }
        }

        if let Some(subtype) = parse_compound_filter_subtype(&all_words, idx) {
            push_unique(&mut filter.subtypes, subtype);
            saw_subtype = true;
        }
    }
    if all_words_with_articles
        .windows(2)
        .any(|window| window == ["attacking", "alone"])
    {
        filter.attacking = true;
        filter.attacking_alone = true;
    }
    // In “shares a creature type with each creature tapped this way”, tapped
    // qualifies the cost objects on the right-hand side, not the candidate
    // card being filtered. Preserve an independent leading `tapped` qualifier
    // when one is also present on the candidate itself.
    if let Some(reference_tapped_idx) = all_words
        .windows(3)
        .position(|window| window == ["tapped", "this", "way"])
        && !all_words
            .iter()
            .enumerate()
            .any(|(idx, word)| *word == "tapped" && idx != reference_tapped_idx)
    {
        filter.tapped = false;
    }

    if saw_spell && source_linked_exile_reference {
        // "spell ... exiled with this" describes a stack spell with a relation
        // to source-linked exiled cards, not a spell object in exile.
        filter.zone = Some(Zone::Stack);
    }

    let segments = split_lexed_slices_on_or(&segment_tokens);
    let mut segment_types = Vec::new();
    let mut segment_subtypes = Vec::new();
    let mut segment_marker_counts = Vec::new();
    let mut segment_words_lists: Vec<Vec<String>> = Vec::new();

    for segment in &segments {
        let segment_words: Vec<String> = non_article_parser_word_refs(segment)
            .into_iter()
            .map(ToString::to_string)
            .collect();
        segment_words_lists.push(segment_words.clone());
        let segment_word_refs = segment_words.iter().map(String::as_str).collect::<Vec<_>>();
        let segment_comparison_rhs_ranges = filter_comparison_rhs_ranges(&segment_word_refs)?;
        // Everything after "named" is a card name, already claimed as one by
        // the name clause. Its words are not characteristics: "named Cleric of
        // the Forward Order" must not also constrain the filter to Clerics.
        let name_clause_start = segment_word_refs
            .iter()
            .position(|word| *word == "named")
            .unwrap_or(segment_word_refs.len());
        let mut types = Vec::new();
        let mut subtypes = Vec::new();
        for (word_idx, word) in segment_words.iter().enumerate() {
            if word_idx >= name_clause_start {
                break;
            }
            if word_is_in_ranges(word_idx, &segment_comparison_rhs_ranges) {
                continue;
            }
            // The primary characteristic pass has already recorded explicit
            // `non-*` atoms as exclusions. Suffix recovery must not feed the
            // same atom through the permissive positive parsers and recreate
            // an impossible "has and does not have" filter.
            if parse_non_type(word).is_some()
                || parse_non_supertype(word).is_some()
                || parse_non_color(word).is_some()
                || parse_non_subtype(word).is_some()
            {
                continue;
            }
            // The lexer splits "non-Wall" into ["non", "wall"]; the earlier
            // characteristic pass recorded the exclusion against ITS index
            // space, which this per-segment scan does not share. Skip any
            // atom directly preceded by a negation word so the excluded
            // characteristic is not re-added positively.
            if word_idx > 0
                && (segment_word_refs[word_idx - 1] == NON_WORD
                    || parse_word_choice(segment_word_refs[word_idx - 1], TEXT_NEGATION_WORDS)
                        .is_some())
            {
                continue;
            }
            if let Some(card_type) = parse_card_type(word) {
                push_unique(&mut types, card_type);
            }
            if let Some(subtype) = parse_compound_filter_subtype(&segment_word_refs, word_idx) {
                push_unique(&mut subtypes, subtype);
            }
        }
        segment_marker_counts.push(types.len() + subtypes.len());
        if !types.is_empty() {
            segment_types.push(types);
        }
        if !subtypes.is_empty() {
            segment_subtypes.push(subtypes);
        }
    }

    if segments.len() > 1 {
        let qualifier_in_all_segments = |qualifier: &str| {
            segment_words_lists.iter().all(|segment| {
                let segment_refs = segment.iter().map(String::as_str).collect::<Vec<_>>();
                parse_word_choice_anywhere(&segment_refs, &[qualifier]).is_some()
            })
        };
        let shared_leading_qualifier = |qualifier: &str, opposite: &str| {
            if qualifier_in_all_segments(qualifier) {
                return true;
            }
            if parse_word_choice_anywhere(&all_words, &[opposite]).is_some() {
                return false;
            }
            let Some(first_segment) = segment_words_lists.first() else {
                return false;
            };
            let first_segment_refs = first_segment.iter().map(String::as_str).collect::<Vec<_>>();
            if parse_word_choice_anywhere(&first_segment_refs, &[qualifier]).is_none() {
                return false;
            }
            segment_words_lists.iter().skip(1).all(|segment| {
                let segment_refs = segment.iter().map(String::as_str).collect::<Vec<_>>();
                parse_word_choice_anywhere(&segment_refs, &[opposite]).is_none()
            })
        };

        if filter.tapped && !shared_leading_qualifier("tapped", "untapped") {
            filter.tapped = false;
        }
        if filter.untapped && !shared_leading_qualifier("untapped", "tapped") {
            filter.untapped = false;
        }
    }

    if segments.len() > 1 {
        if !has_coordinated_negated_characteristic_list {
            let type_list_candidate = !segment_marker_counts.is_empty()
                && segment_marker_counts.iter().all(|count| *count == 1);

            if type_list_candidate {
                let mut any_types = Vec::new();
                let mut any_subtypes = Vec::new();
                for types in segment_types {
                    let Some(card_type) = types.first().copied() else {
                        continue;
                    };
                    push_unique(&mut any_types, card_type);
                }
                for subtypes in segment_subtypes {
                    let Some(subtype) = subtypes.first().copied() else {
                        continue;
                    };
                    push_unique(&mut any_subtypes, subtype);
                }
                if !any_types.is_empty() {
                    filter.card_types = any_types;
                }
                if !any_subtypes.is_empty() {
                    filter.subtypes = any_subtypes;
                    filter.all_subtypes.clear();
                }
                if !filter.card_types.is_empty() && !filter.subtypes.is_empty() {
                    filter.type_or_subtype_union = true;
                }
            }
        }
    } else {
        let types = segment_types.into_iter().next().unwrap_or_default();
        let subtypes = segment_subtypes.into_iter().next().unwrap_or_default();
        let normalized_segment_words = non_article_parser_word_refs(&segment_tokens);
        // Only a connector between characteristic atoms makes those atoms an
        // inclusive list. A later suffix connector ("you own and control")
        // must not turn an adjacent compound type or subtype phrase into OR.
        let characteristic_word_indices = normalized_segment_words
            .iter()
            .enumerate()
            .filter_map(|(idx, word)| {
                (parse_card_type(word).is_some()
                    || parse_compound_filter_subtype(&normalized_segment_words, idx).is_some())
                .then_some(idx)
            })
            .collect::<Vec<_>>();
        let has_conjunction = characteristic_word_indices
            .first()
            .zip(characteristic_word_indices.last())
            .is_some_and(|(first, last)| {
                normalized_segment_words[*first..=*last]
                    .iter()
                    .any(|word| TYPE_LIST_CONJUNCTION_WORDS.contains(word))
            });
        let has_and = parse_word_choice_anywhere(&normalized_segment_words, &["and"]).is_some();
        let has_or = parse_word_choice_anywhere(&normalized_segment_words, &["or"]).is_some();
        let has_and_or =
            parse_word_choice_anywhere(&normalized_segment_words, &["and/or"]).is_some();
        if types.len() > 1 {
            if has_conjunction {
                filter.card_types = types;
            } else {
                filter.all_card_types = types;
            }
        } else if types.len() == 1 {
            filter.card_types = types;
        }
        // The fast typed filter parser may already have recognized a compound
        // subtype phrase. Preserve that complete set: the flexible reference
        // scan intentionally recognizes fewer subtype spellings and must not
        // replace it with a partial subset.
        if filter.all_subtypes.is_empty() {
            if subtypes.len() > 1 {
                if has_conjunction {
                    filter.subtypes = subtypes;
                } else {
                    filter.all_subtypes = subtypes;
                    filter.subtypes.clear();
                }
            } else if subtypes.len() == 1 {
                filter.subtypes = subtypes;
            }
        }
        if (has_and_or || (has_and && has_or))
            && !filter.card_types.is_empty()
            && !filter.subtypes.is_empty()
        {
            filter.type_or_subtype_union = true;
        }
    }

    let permanent_type_defaults = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    let and_segments = split_lexed_slices_on_and(&segment_tokens);
    let and_segment_words_lists: Vec<Vec<String>> = and_segments
        .iter()
        .map(|segment| {
            non_article_parser_word_refs(segment)
                .into_iter()
                .map(ToString::to_string)
                .collect()
        })
        .collect();

    let segment_has_standalone_spell = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| parse_word_choice(word, SPELL_OR_SPELLS_WORDS).is_some());
        if !contains_spell {
            return false;
        }

        !segment.iter().any(|word| {
            parse_word_choice(word.as_str(), OBJECT_REFERENCE_NOUN_WORDS).is_some()
                || parse_card_type(word).is_some()
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_nonspell_permanent_head = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| parse_word_choice(word, SPELL_OR_SPELLS_WORDS).is_some());
        if contains_spell {
            return false;
        }

        segment.iter().any(|word| {
            parse_word_choice(word, PERMANENT_OR_PERMANENTS_WORDS).is_some()
                || parse_card_type(word).is_some_and(is_permanent_type)
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_permanent_spell_head = |segment: &[String]| {
        if segment.len() < 2 {
            return false;
        }
        let mut idx = 0usize;
        while idx + 1 < segment.len() {
            let permanent = &segment[idx];
            let spell = &segment[idx + 1];
            if parse_word_choice(permanent, PERMANENT_OR_PERMANENTS_WORDS).is_some()
                && parse_word_choice(spell, SPELL_OR_SPELLS_WORDS).is_some()
            {
                return true;
            }
            idx += 1;
        }
        false
    };
    let has_standalone_spell_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_standalone_spell(segment));
    let has_nonspell_permanent_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_nonspell_permanent_head(segment));
    let has_split_permanent_spell_segments = and_segment_words_lists.len() > 1
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_permanent_spell_head(segment))
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_nonspell_permanent_head(segment));

    if saw_spell && has_standalone_spell_segment && has_nonspell_permanent_segment {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.card_types.clear();
        spell_filter.all_card_types.clear();
        spell_filter.subtypes.clear();
        spell_filter.all_subtypes.clear();
        spell_filter.type_or_subtype_union = false;

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
            && permanent_filter.all_subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent && has_split_permanent_spell_segments {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.has_mana_cost = false;
        if spell_filter.card_types.is_empty()
            && spell_filter.all_card_types.is_empty()
            && spell_filter.subtypes.is_empty()
            && spell_filter.all_subtypes.is_empty()
        {
            spell_filter.card_types = permanent_type_defaults.clone();
        }

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
            && permanent_filter.all_subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent {
        if filter.card_types.is_empty() && filter.all_card_types.is_empty() {
            filter.card_types = permanent_type_defaults.clone();
        }
        filter.zone = Some(Zone::Stack);
    } else {
        if saw_permanent && filter.card_types.is_empty() && filter.all_card_types.is_empty() {
            filter.card_types = permanent_type_defaults.clone();
        }
    }

    if filter.any_of.is_empty() {
        if let Some(zone) = filter.zone {
            if saw_spell && zone != Zone::Stack {
                let is_spell_origin_zone = matches!(
                    zone,
                    Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command
                );
                if !is_spell_origin_zone {
                    return Err(CardTextError::ParseError(
                        "spell targets must be on the stack".to_string(),
                    ));
                }
            }
        } else if saw_spell {
            filter.zone = Some(Zone::Stack);
        } else if saw_permanent || saw_permanent_type || saw_subtype {
            filter.zone = Some(Zone::Battlefield);
        }
    }

    if contains_unqualified_spell_word
        && filter.cast_by.is_some()
        && matches!(
            filter.zone,
            Some(Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command)
        )
    {
        filter.owner = None;
    }

    if target_player.is_some() || target_object.is_some() {
        filter = if targets_only {
            filter.targeting_only(target_player.take(), target_object.take())
        } else {
            filter.targeting(target_player.take(), target_object.take())
        };
        if let Some(count) = target_count {
            filter = filter.with_target_count(count);
        } else if targets_only {
            filter = filter.target_count_exact(1);
        }
    }

    if let Some(or_subtype) = legendary_or_subtype
        && filter.any_of.is_empty()
        && slice_has(&filter.supertypes, &Supertype::Legendary)
        && slice_has(&filter.subtypes, &or_subtype)
    {
        // The zone, owner, target count, and other trailing qualifiers scope
        // the complete disjunction. Keep them on the outer filter rather than
        // cloning them into the two selector arms; reference consumers (for
        // example a subsequent graveyard-card copy) must be able to observe
        // that the selected object itself is in that shared domain.
        let mut disjunction = filter.clone();
        disjunction
            .supertypes
            .retain(|supertype| *supertype != Supertype::Legendary);
        disjunction
            .subtypes
            .retain(|subtype| *subtype != or_subtype);
        let legendary_branch = ObjectFilter {
            supertypes: vec![Supertype::Legendary],
            ..ObjectFilter::default()
        };
        let subtype_branch = ObjectFilter {
            subtypes: vec![or_subtype],
            ..ObjectFilter::default()
        };
        disjunction.any_of = vec![legendary_branch, subtype_branch];
        filter = disjunction;
    }

    let owner_or_controller_player = all_words.iter().enumerate().find_map(|(idx, _)| {
        parse_owner_or_controller_disjunction_player(&all_words[idx..], &pronoun_player_filter)
            .map(|(player_filter, _)| player_filter)
    });
    if let Some(player_filter) = owner_or_controller_player
        && filter.any_of.is_empty()
    {
        let mut base = filter.clone();
        base.any_of.clear();
        base.owner = None;
        base.controller = None;

        let mut owner_branch = base.clone();
        owner_branch.owner = Some(player_filter.clone());

        let mut controller_branch = base;
        controller_branch.controller = Some(player_filter);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![owner_branch, controller_branch];
        filter = disjunction;
    }

    if has_power_or_toughness_clause && saw_spell {
        let mut power_or_toughness_cmp = None;
        for idx in 0..all_words.len() {
            let (_, value_tokens) = match all_words.get(idx..) {
                Some(["power", "or", "toughness", rest @ ..])
                | Some(["toughness", "or", "power", rest @ ..]) => {
                    (crate::filter::PtReference::Effective, rest)
                }
                _ => continue,
            };
            let Some((cmp, _)) =
                parse_filter_comparison_tokens("power", value_tokens, &clause_words)?
            else {
                continue;
            };
            power_or_toughness_cmp = Some(cmp);
            break;
        }
        if let Some(cmp) = power_or_toughness_cmp {
            let mut base = filter.clone();
            base.any_of.clear();
            base.power = None;
            base.toughness = None;

            let mut power_branch = base.clone();
            power_branch.power = Some(cmp.clone());

            let mut toughness_branch = base;
            toughness_branch.toughness = Some(cmp);

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![power_branch, toughness_branch];
            filter = disjunction;
        }
    }

    // In "creature attacking you or a planeswalker you control", the
    // planeswalker and its controller describe the attack destination, not
    // another candidate object. Apply this cleanup after the ordinary
    // characteristic scan so those later passes cannot reintroduce the
    // destination as a candidate. The position check distinguishes this from
    // "creature or planeswalker attacking you", where both nouns genuinely
    // select candidates.
    if has_attack_destination_planeswalker_clause
        && filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
    {
        filter
            .card_types
            .retain(|card_type| *card_type != CardType::Planeswalker);
        filter
            .all_card_types
            .retain(|card_type| *card_type != CardType::Planeswalker);
        filter.controller = None;
    }

    if exclude_basic_land_cards {
        apply_basic_land_exception(&mut filter);
    }

    if chosen_type_reference.is_some() {
        let has_land_type = filter
            .card_types
            .iter()
            .chain(filter.all_card_types.iter())
            .any(|card_type| *card_type == CardType::Land);
        let has_nonland_type = filter
            .card_types
            .iter()
            .chain(filter.all_card_types.iter())
            .any(|card_type| *card_type != CardType::Land);
        if has_land_type && !has_nonland_type {
            filter.chosen_land_type = true;
            filter.chosen_creature_type = false;
        }
    }

    if parse_word_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        TYPE_LIST_CONJUNCTION_WORDS,
    )
    .is_some()
        && !filter.card_types.is_empty()
    {
        filter.all_card_types.clear();
    }

    let has_constraints = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.all_subtypes.is_empty()
        || filter.zone.is_some()
        || filter.controller.is_some()
        || filter.owner.is_some()
        || filter.other
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.foretold
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter.attacking_alone
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || filter.required_colors.is_some()
        || filter.sticker.is_some()
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.color_count.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.has_basic_land_type
        || filter.has_nonbasic_land_type
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.power_toughness_relation.is_some()
        || filter.toughness.is_some()
        || filter.total_power_toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.characteristic_relations.is_empty()
        || !filter.any_of.is_empty();

    if !has_constraints {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase (clause: '{}')",
            all_words.join(" ")
        )));
    }

    let has_object_identity = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.all_subtypes.is_empty()
        || filter.zone.is_some()
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.foretold
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter.attacking_alone
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || filter.required_colors.is_some()
        || filter.sticker.is_some()
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.color_count.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.power_toughness_relation.is_some()
        || filter.toughness.is_some()
        || filter.total_power_toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || !filter.no_shared_creature_types_with.is_empty()
        || !filter.characteristic_relations.is_empty()
        || filter.shares_creature_type_with_source
        || filter.chosen_color
        || filter.chosen_creature_type
        || filter.excluded_chosen_creature_type
        || filter.excluded_any_chosen_creature_type
        || filter.colors.is_some()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.any_of.is_empty();
    if !has_object_identity {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase lacking object selector (clause: '{}')",
            all_words.join(" ")
        )));
    }

    preserve_relative_characteristic_list_surface(&mut filter, tokens);
    preserve_branch_scoped_comparison_union(&mut filter, tokens);
    lift_shared_trailing_mana_value_from_type_union(&mut filter, tokens);

    if vote_winners_only {
        filter = filter.match_tagged(
            TagKey::from(VOTE_WINNERS_TAG),
            TaggedOpbjectRelation::IsTaggedObject,
        );
    }

    if not_on_battlefield && filter.any_of.is_empty() && !matches!(filter.zone, Some(Zone::Stack)) {
        let mut base = filter.clone();
        base.any_of.clear();
        base.zone = None;

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Command,
        ]
        .into_iter()
        .map(|zone| {
            let mut branch = base.clone();
            branch.zone = Some(zone);
            branch
        })
        .collect();
        filter = disjunction;
    }

    // Strict mode: detect structural patterns in the input that indicate
    // unconsumed compound content (e.g. "for each card in your hand AND EACH
    // foretold card you own in exile" where the second clause was silently
    // absorbed into the first filter).
    // This exact coordinated stack domain can be partially rewritten by
    // later noun/reference stages (most visibly back to Spell with a mana
    // cost). Reassert only the grammar-proven final domain after every stage
    // has run so public quantified clauses retain both members.
    let final_words = non_article_parser_word_refs(tokens);
    if parse_phrase_choice_anywhere(&final_words, SPELL_AND_ABILITY_PHRASES).is_some() {
        filter.zone = Some(Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::SpellOrAbility);
        filter.has_mana_cost = false;
        filter.set_conjunctive_set_surface(true);
    }

    if strict {
        let input_words = non_article_parser_word_refs(tokens);
        let all_words = input_words.as_slice();

        // "and each" / "and every" signals a compound count source when
        // the word after "each"/"every" introduces a new filter (type word,
        // zone word, etc.) rather than qualifying the current subject
        // (e.g. "and each other creature" is a subject qualifier, but
        // "and each foretold card you own in exile" is a new clause).
        for (idx, _) in input_words.iter().enumerate() {
            if parse_phrase_choice_at_head(&input_words[idx..], STRICT_COMPOUND_COUNT_PREFIXES)
                .is_none()
            {
                continue;
            }
            // A "other than basic land card(s)" exception is stripped before
            // this point, so it never reaches the compound-clause check; guard
            // for it defensively to keep the strict scan stable.
            if parse_phrase_at_head(&all_words[idx..], OTHER_THAN_BASIC_LAND_PREFIX).is_some() {
                continue;
            }
            // "and each other" is typically a subject qualifier, allow it.
            let after_each = input_words.get(idx + 2).copied();
            if after_each.is_some_and(|w| parse_word_choice(w, OTHER_OR_ANOTHER_WORDS).is_some()) {
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "object filter has unconsumed compound clause '{}' (full input: '{}')",
                input_words[idx..].join(" "),
                input_words.join(" "),
            )));
        }

        // "for each" signals a trailing iteration clause that should have
        // been split out by the caller before passing to the filter parser.
        for (idx, _) in input_words.iter().enumerate() {
            if idx > 0
                && parse_phrase_at_head(&input_words[idx..], STRICT_FOR_EACH_TAIL_PREFIX).is_some()
            {
                return Err(CardTextError::ParseError(format!(
                    "object filter has unconsumed 'for each' clause '{}' (full input: '{}')",
                    input_words[idx..].join(" "),
                    input_words.join(" "),
                )));
            }
        }
    }

    Ok(filter)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelativeCharacteristicSelector {
    CardType(CardType),
    Subtype(Subtype),
    Token,
}

fn positive_relative_characteristic_union(
    words: &[&str],
) -> Option<(
    usize,
    Vec<RelativeCharacteristicSelector>,
    ObjectFilterUnionConnective,
    bool,
)> {
    let (relation_start, characteristic_start) =
        words
            .iter()
            .enumerate()
            .find_map(|(idx, word)| match *word {
                "that's" | "thats" => Some((idx, idx + 1)),
                _ if matches!(
                    words.get(idx..idx + 2),
                    Some(["that", "is"] | ["that", "are"])
                ) =>
                {
                    Some((idx, idx + 2))
                }
                _ => None,
            })?;
    let characteristic_words = words.get(characteristic_start..)?;
    let connective = if characteristic_words.contains(&"and/or") {
        ObjectFilterUnionConnective::AndOr
    } else if characteristic_words.contains(&"or") {
        ObjectFilterUnionConnective::Or
    } else {
        return None;
    };

    let mut selectors = Vec::new();
    let mut selector_occurrences = 0usize;
    let mut selectors_with_articles = 0usize;
    for (idx, word) in characteristic_words.iter().enumerate() {
        if matches!(
            *word,
            "a" | "an" | "the" | "and" | "or" | "and/or" | "card" | "cards"
        ) {
            continue;
        }
        let selector = if matches!(*word, "token" | "tokens") {
            RelativeCharacteristicSelector::Token
        } else if let Some(card_type) = parse_card_type(word) {
            RelativeCharacteristicSelector::CardType(card_type)
        } else if let Some(subtype) = parse_subtype_flexible(word) {
            RelativeCharacteristicSelector::Subtype(subtype)
        } else {
            return None;
        };
        selector_occurrences += 1;
        if idx
            .checked_sub(1)
            .and_then(|previous| characteristic_words.get(previous))
            .is_some_and(|previous| matches!(*previous, "a" | "an"))
        {
            selectors_with_articles += 1;
        }
        if !selectors.contains(&selector) {
            selectors.push(selector);
        }
    }
    (selectors.len() >= 2).then_some((
        relation_start,
        selectors,
        connective,
        selector_occurrences >= 2 && selectors_with_articles == selector_occurrences,
    ))
}

fn preserve_relative_characteristic_list_surface(
    filter: &mut ObjectFilter,
    tokens: &[OwnedLexToken],
) {
    let words = parser_token_word_refs(tokens);
    let has_negative_relative_copula = words
        .iter()
        .any(|word| matches!(*word, "isn't" | "isnt" | "aren't" | "arent"))
        || words
            .windows(2)
            .any(|pair| matches!(pair, ["is", "not"] | ["are", "not"]));
    if has_negative_relative_copula && filter.subtypes.len() + filter.excluded_subtypes.len() >= 2 {
        filter.set_relative_characteristic_list_surface(true);
        return;
    }

    let Some((relation_start, selectors, connective, explicit_branch_articles)) =
        positive_relative_characteristic_union(&words)
    else {
        return;
    };
    if !filter.any_of.is_empty() {
        return;
    }

    let prefix_words = &words[..relation_start];
    let prefix_card_types = prefix_words
        .iter()
        .filter_map(|word| parse_card_type(word))
        .collect::<Vec<_>>();
    let prefix_subtypes = prefix_words
        .iter()
        .filter_map(|word| parse_subtype_flexible(word))
        .collect::<Vec<_>>();
    let prefix_is_token = prefix_words
        .iter()
        .any(|word| matches!(*word, "token" | "tokens"));

    let mut base = filter.clone();
    for selector in &selectors {
        match selector {
            RelativeCharacteristicSelector::CardType(card_type) => {
                base.card_types.retain(|candidate| candidate != card_type);
                base.all_card_types
                    .retain(|candidate| candidate != card_type);
            }
            RelativeCharacteristicSelector::Subtype(subtype) => {
                base.subtypes.retain(|candidate| candidate != subtype);
            }
            RelativeCharacteristicSelector::Token => base.token = false,
        }
    }
    for card_type in prefix_card_types {
        push_unique(&mut base.card_types, card_type);
    }
    for subtype in prefix_subtypes {
        push_unique(&mut base.subtypes, subtype);
    }
    if prefix_is_token {
        base.token = true;
    }
    base.type_or_subtype_union = false;
    base.set_union_connective(connective);
    base.set_explicit_union_branch_articles(explicit_branch_articles);
    base.set_relative_characteristic_list_surface(true);

    if selectors
        .iter()
        .all(|selector| matches!(selector, RelativeCharacteristicSelector::Subtype(_)))
    {
        for selector in selectors {
            let RelativeCharacteristicSelector::Subtype(subtype) = selector else {
                unreachable!("all relative selectors were checked as subtypes");
            };
            push_unique(&mut base.subtypes, subtype);
        }
        *filter = base;
        return;
    }

    base.any_of = selectors
        .into_iter()
        .map(|selector| match selector {
            RelativeCharacteristicSelector::CardType(card_type) => {
                ObjectFilter::default().with_type(card_type)
            }
            RelativeCharacteristicSelector::Subtype(subtype) => {
                ObjectFilter::default().with_subtype(subtype)
            }
            RelativeCharacteristicSelector::Token => ObjectFilter::default().token(),
        })
        .collect();
    *filter = base;
}

/// Lift a trailing mana-value qualifier out of a card-type union when Oracle
/// supplies the object noun only once after the final type.
///
/// `instant or sorcery card with mana value ...` gives both type arms the
/// same qualifier. Parsing the arms independently can otherwise leave the
/// comparison only on the final `sorcery card` arm. By contrast,
/// `land card or creature card with mana value ...` repeats the noun and
/// deliberately keeps the qualifier branch-local.
fn lift_shared_trailing_mana_value_from_type_union(
    filter: &mut ObjectFilter,
    tokens: &[OwnedLexToken],
) {
    if filter.any_of.is_empty() || filter.mana_value.is_some() {
        return;
    }

    let words = parser_token_word_refs(tokens);
    let Some(mana_idx) = words.windows(2).position(|pair| pair == ["mana", "value"]) else {
        return;
    };
    let Some(connector_idx) = words[..mana_idx]
        .iter()
        .rposition(|word| matches!(*word, "or" | "and/or"))
    else {
        return;
    };
    let selector_count = words[..mana_idx]
        .iter()
        .filter_map(|word| parse_card_type(word))
        .collect::<std::collections::HashSet<_>>()
        .len();
    if selector_count < 2 {
        return;
    }
    let is_shared_noun = |word: &&str| {
        matches!(
            *word,
            "card" | "cards" | "spell" | "spells" | "permanent" | "permanents"
        )
    };
    if words[..connector_idx].iter().any(is_shared_noun)
        || words[connector_idx + 1..mana_idx]
            .iter()
            .filter(|word| is_shared_noun(word))
            .count()
            > 1
    {
        return;
    }

    fn collect_mana_value(
        filter: &ObjectFilter,
        shared: &mut Option<crate::filter::Comparison>,
    ) -> bool {
        if let Some(comparison) = &filter.mana_value {
            match shared {
                Some(existing) if existing != comparison => return false,
                Some(_) => {}
                None => *shared = Some(comparison.clone()),
            }
        }
        filter
            .any_of
            .iter()
            .all(|branch| collect_mana_value(branch, shared))
    }

    let mut shared = None;
    if !filter
        .any_of
        .iter()
        .all(|branch| collect_mana_value(branch, &mut shared))
    {
        return;
    }
    let Some(shared) = shared else {
        return;
    };

    fn clear_mana_value(filter: &mut ObjectFilter) {
        filter.mana_value = None;
        for branch in &mut filter.any_of {
            clear_mana_value(branch);
        }
    }
    for branch in &mut filter.any_of {
        clear_mana_value(branch);
    }
    filter.mana_value = Some(shared);

    let shared_zone = filter
        .any_of
        .iter()
        .filter_map(|branch| branch.zone)
        .try_fold(None, |current, zone| match current {
            Some(existing) if existing != zone => None,
            Some(existing) => Some(Some(existing)),
            None => Some(Some(zone)),
        })
        .flatten();
    if filter.zone.is_none() {
        filter.zone = shared_zone;
    }
    let shared_controller = filter
        .any_of
        .iter()
        .filter_map(|branch| branch.controller.clone())
        .try_fold(None, |current, controller| match current {
            Some(existing) if existing != controller => None,
            Some(existing) => Some(Some(existing)),
            None => Some(Some(controller)),
        })
        .flatten();
    if filter.controller.is_none() {
        filter.controller = shared_controller;
    }
    let shared_owner = filter
        .any_of
        .iter()
        .filter_map(|branch| branch.owner.clone())
        .try_fold(None, |current, owner| match current {
            Some(existing) if existing != owner => None,
            Some(existing) => Some(Some(existing)),
            None => Some(Some(owner)),
        })
        .flatten();
    if filter.owner.is_none() {
        filter.owner = shared_owner;
    }
    for branch in &mut filter.any_of {
        if branch.zone == filter.zone {
            branch.zone = None;
        }
        if branch.controller == filter.controller {
            branch.controller = None;
        }
        if branch.owner == filter.owner {
            branch.owner = None;
        }
    }

    let mut card_types = Vec::new();
    for branch in &filter.any_of {
        let [card_type] = branch.card_types.as_slice() else {
            return;
        };
        let mut remainder = branch.clone();
        remainder.card_types.clear();
        remainder.union_surface = Default::default();
        remainder.type_or_subtype_union = false;
        if remainder != ObjectFilter::default() {
            return;
        }
        if !card_types.contains(card_type) {
            card_types.push(*card_type);
        }
    }
    if card_types.len() < 2 {
        return;
    }

    filter.card_types = card_types;
    filter.any_of.clear();
    filter.type_or_subtype_union = true;
    filter.set_explicit_card_noun(true);
    filter.set_terminal_noun_after_type_subtype_union_surface(true);
}

/// Keep a comparison next to the characteristic arm it grammatically
/// qualifies instead of distributing it over the whole inclusive union.
fn preserve_branch_scoped_comparison_union(filter: &mut ObjectFilter, tokens: &[OwnedLexToken]) {
    if !filter.type_or_subtype_union || !filter.any_of.is_empty() || filter.token {
        return;
    }
    let connector_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.is_word("and/or").then_some(idx))
        .collect::<Vec<_>>();
    let [connector_idx] = connector_indices.as_slice() else {
        return;
    };
    let left_tokens = trim_commas(&tokens[..*connector_idx]);
    let right_tokens = trim_commas(&tokens[*connector_idx + 1..]);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return;
    }

    let Ok(left) = parse_object_filter_lexed(&left_tokens, false) else {
        return;
    };
    let Ok(right) = parse_object_filter_lexed(&right_tokens, false) else {
        return;
    };
    if !left.any_of.is_empty()
        || !right.any_of.is_empty()
        || left.card_types.len() + left.subtypes.len() != 1
        || right.card_types.len() + right.subtypes.len() != 1
        || !left.all_card_types.is_empty()
        || !right.all_card_types.is_empty()
        || !left.all_subtypes.is_empty()
        || !right.all_subtypes.is_empty()
    {
        return;
    }

    let power_is_branch_local = filter.power.is_some()
        && ((left.power == filter.power && right.power.is_none())
            || (right.power == filter.power && left.power.is_none()));
    let toughness_is_branch_local = filter.toughness.is_some()
        && ((left.toughness == filter.toughness && right.toughness.is_none())
            || (right.toughness == filter.toughness && left.toughness.is_none()));
    let mana_value_is_branch_local = filter.mana_value.is_some()
        && ((left.mana_value == filter.mana_value && right.mana_value.is_none())
            || (right.mana_value == filter.mana_value && left.mana_value.is_none()));
    if !power_is_branch_local && !toughness_is_branch_local && !mana_value_is_branch_local {
        return;
    }

    let mut outer = filter.clone();
    outer.card_types.clear();
    outer.all_card_types.clear();
    outer.subtypes.clear();
    outer.all_subtypes.clear();
    outer.type_or_subtype_union = false;
    if power_is_branch_local {
        outer.power = None;
    }
    if toughness_is_branch_local {
        outer.toughness = None;
    }
    if mana_value_is_branch_local {
        outer.mana_value = None;
    }

    let mut branches = vec![left, right];
    for branch in &mut branches {
        if branch.zone == outer.zone {
            branch.zone = None;
        }
        if branch.controller == outer.controller {
            branch.controller = None;
        }
        if branch.owner == outer.owner {
            branch.owner = None;
        }
        if outer.other && branch.other {
            branch.other = false;
        }
    }
    outer.any_of = branches;
    *filter = outer;
}

fn relation_clause_is_inside_aggregate_scope(words: &[&str], relation_start: usize) -> bool {
    let Some(with_fact) = parse_last_word_choice_before(words, &[WITH_WORD], relation_start) else {
        return false;
    };
    let with_idx = with_fact.index;
    let prefix = &words[with_idx + 1..relation_start];
    let has_aggregate = prefix
        .iter()
        .any(|word| parse_word_choice(word, AGGREGATE_SCOPE_WORDS).is_some());
    let has_scope_marker =
        parse_word_choice_anywhere(prefix, AGGREGATE_SCOPE_MARKER_WORDS).is_some();
    has_aggregate && has_scope_marker
}

fn strip_other_than_basic_land_cards_clause(
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let mut idx = 0usize;
    while idx + 3 < all_words.len() {
        if parse_phrase_at_head(&all_words[idx..], OTHER_THAN_BASIC_LAND_PREFIX).is_none() {
            idx += 1;
            continue;
        }

        let mut end = idx + 4;
        if all_words
            .get(end)
            .is_some_and(|word| parse_word_choice(word, CARD_OR_CARDS_WORDS).is_some())
        {
            end += 1;
        }
        all_words.drain(idx..end);
        strip_other_than_basic_land_cards_tokens(segment_tokens);
        return true;
    }

    false
}

fn strip_other_than_basic_land_cards_tokens(segment_tokens: &mut Vec<OwnedLexToken>) {
    let mut idx = 0usize;
    while idx + 3 < segment_tokens.len() {
        let word_at = |offset: usize| segment_tokens.get(offset).and_then(OwnedLexToken::as_word);
        if word_at(idx) != Some("other") || word_at(idx + 1) != Some("than") {
            idx += 1;
            continue;
        }

        let mut end = idx + 2;
        if word_at(end).is_some_and(is_article) {
            end += 1;
        }
        if word_at(end) != Some("basic") || word_at(end + 1) != Some("land") {
            idx += 1;
            continue;
        }
        end += 2;
        if word_at(end).is_some_and(|word| parse_word_choice(word, CARD_OR_CARDS_WORDS).is_some()) {
            end += 1;
        }
        segment_tokens.drain(idx..end);
        return;
    }
}

fn apply_basic_land_exception(filter: &mut ObjectFilter) {
    let mut nonland_branch = filter.clone();
    nonland_branch.any_of.clear();
    push_unique(&mut nonland_branch.excluded_card_types, CardType::Land);

    let mut nonbasic_branch = filter.clone();
    nonbasic_branch.any_of.clear();
    push_unique(&mut nonbasic_branch.excluded_supertypes, Supertype::Basic);

    *filter = ObjectFilter {
        any_of: vec![nonland_branch, nonbasic_branch],
        ..Default::default()
    };
}

fn try_apply_could_be_targeted_by_that_spell_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        ["that", "spell", "could", "target"].as_slice(),
        ["this", "spell", "could", "target"].as_slice(),
        ["it", "could", "target"].as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;
        filter.could_be_targeted_by = Some(TargetabilityConstraint::by_stack_object(
            ObjectRef::tagged(TagKey::from(IT_TAG)),
        ));
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PermanentOrSuspendedCardArm {
    Permanent,
    SuspendedCard,
}

fn parse_permanent_or_suspended_card_disjunction(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let or_segments = split_lexed_slices_on_or(tokens);
    let segments = if or_segments.len() == 2 {
        or_segments
    } else {
        // Count expressions commonly coordinate the two disjoint domains
        // additively: "each suspended card ... and each other permanent ...".
        // They still need inclusive `any_of` semantics; flattening them gives
        // the exile arm battlefield/controller constraints from the permanent
        // arm.
        primitives::split_lexed_slices_on_list_conjunction(tokens)
    };
    if segments.len() != 2 {
        return None;
    }

    let (left_kind, left_filter) = parse_permanent_or_suspended_card_arm(segments[0])?;
    let (right_kind, right_filter) = parse_permanent_or_suspended_card_arm(segments[1])?;
    if left_kind == right_kind {
        return None;
    }

    Some(ObjectFilter {
        any_of: vec![left_filter, right_filter],
        ..ObjectFilter::default()
    })
}

fn parse_permanent_or_suspended_card_arm(
    tokens: &[OwnedLexToken],
) -> Option<(PermanentOrSuspendedCardArm, ObjectFilter)> {
    let words = non_article_parser_word_refs(tokens);
    let mut words = words.as_slice();
    if words.first().is_some_and(|word| *word == "each") {
        words = &words[1..];
    }
    let arm_other = words
        .first()
        .is_some_and(|word| matches!(*word, "other" | "another"));
    if arm_other {
        words = &words[1..];
    }
    let words = if words
        .first()
        .is_some_and(|word| parse_word_choice(word, TARGET_OR_TARGETS_WORDS).is_some())
    {
        &words[1..]
    } else {
        words
    };

    let leading_nonland = words.first() == Some(&"nonland");
    let noun_words = if leading_nonland { &words[1..] } else { words };

    match noun_words.first().copied() {
        Some("permanent" | "permanents") => {
            let mut filter = ObjectFilter::permanent();
            if leading_nonland {
                filter.excluded_card_types.push(CardType::Land);
            }
            filter.other = arm_other;
            consume_permanent_or_suspended_card_tail(noun_words, 1, &mut filter, true, true)?;
            Some((PermanentOrSuspendedCardArm::Permanent, filter))
        }
        Some("suspended") if !leading_nonland => {
            let card_word = noun_words.get(1).copied()?;
            if !matches!(card_word, "card" | "cards") {
                return None;
            }
            let mut filter = ObjectFilter::default()
                .in_zone(Zone::Exile)
                .with_alternative_cast(crate::filter::AlternativeCastKind::Suspend)
                .with_counter_type(crate::object::CounterType::Time);
            filter.other = arm_other;
            consume_permanent_or_suspended_card_tail(noun_words, 2, &mut filter, false, true)?;
            Some((PermanentOrSuspendedCardArm::SuspendedCard, filter))
        }
        _ => None,
    }
}

fn consume_permanent_or_suspended_card_tail(
    words: &[&str],
    mut idx: usize,
    filter: &mut ObjectFilter,
    allow_controller: bool,
    allow_owner: bool,
) -> Option<()> {
    while idx < words.len() {
        if allow_controller
            && words.get(idx) == Some(&"you")
            && words.get(idx + 1) == Some(&"control")
        {
            filter.controller = Some(PlayerFilter::You);
            idx += 2;
            continue;
        }
        if allow_owner && words.get(idx) == Some(&"you") && words.get(idx + 1) == Some(&"own") {
            filter.owner = Some(PlayerFilter::You);
            idx += 2;
            continue;
        }
        if words.get(idx) == Some(&"with")
            && words.get(idx + 1) == Some(&"time")
            && words
                .get(idx + 2)
                .is_some_and(|word| matches!(*word, "counter" | "counters"))
        {
            filter.with_counter = Some(crate::filter::CounterConstraint::Typed(
                crate::object::CounterType::Time,
            ));
            idx += 3;
            if words.get(idx) == Some(&"on")
                && words
                    .get(idx + 1)
                    .is_some_and(|word| matches!(*word, "it" | "them"))
            {
                idx += 2;
            }
            continue;
        }
        return None;
    }
    Some(())
}

fn try_apply_distinct_powers_clause(filter: &mut ObjectFilter, all_words: &mut Vec<&str>) -> bool {
    for phrase in [
        ["with", "different", "powers"].as_slice(),
        ["that", "have", "different", "powers"].as_slice(),
        ["that", "has", "different", "powers"].as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;
        filter.distinct_powers = true;
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}

fn try_apply_distinct_mana_values_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        ["with", "different", "mana", "values"].as_slice(),
        ["that", "have", "different", "mana", "values"].as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;
        filter.distinct_mana_values = true;
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}

fn try_apply_distinct_creature_types_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        ["that", "share", "no", "creature", "types"].as_slice(),
        ["that", "shares", "no", "creature", "types"].as_slice(),
        ["with", "no", "creature", "types", "in", "common"].as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;
        filter.distinct_creature_types = true;
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}

fn try_apply_no_shared_creature_type_with_your_creatures_or_graveyard_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in NO_SHARED_CREATURE_TYPE_WITH_YOUR_CREATURES_OR_GRAVEYARD_CLAUSES {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;

        filter
            .no_shared_creature_types_with
            .push(ObjectFilter::creature().you_control());
        filter.no_shared_creature_types_with.push(
            ObjectFilter::default()
                .with_type(CardType::Creature)
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}

fn try_apply_no_shared_creature_type_with_chosen_creature_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        [
            "that", "doesn't", "share", "creature", "type", "with", "chosen", "creature", "they",
            "control",
        ]
        .as_slice(),
        [
            "that", "doesnt", "share", "creature", "type", "with", "chosen", "creature", "they",
            "control",
        ]
        .as_slice(),
        [
            "that", "don't", "share", "creature", "type", "with", "chosen", "creature", "they",
            "control",
        ]
        .as_slice(),
        [
            "that", "dont", "share", "creature", "type", "with", "chosen", "creature", "they",
            "control",
        ]
        .as_slice(),
        [
            "that", "do", "not", "share", "creature", "type", "with", "chosen", "creature", "they",
            "control",
        ]
        .as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        filter
            .no_shared_creature_types_with
            .push(ObjectFilter::tagged(TagKey::from(IT_TAG)));
        all_words.drain(fact.span.start..fact.span.start + phrase.len());
        return true;
    }
    false
}

fn try_apply_shared_creature_type_with_source_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        [
            "that", "share", "creature", "type", "with", "this", "creature",
        ]
        .as_slice(),
        [
            "that", "shares", "creature", "type", "with", "this", "creature",
        ]
        .as_slice(),
        [
            "that",
            "share",
            "creature",
            "type",
            "with",
            "this",
            "permanent",
        ]
        .as_slice(),
        [
            "that",
            "shares",
            "creature",
            "type",
            "with",
            "this",
            "permanent",
        ]
        .as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;

        filter.shares_creature_type_with_source = true;
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}

#[cfg(test)]
mod shared_characteristic_relation_tests {
    use super::*;
    use crate::runtime_backend::lex_line;
    use crate::static_abilities::StaticAbilityId;
    use crate::target::ObjectCharacteristicRelationKind;

    fn parse_filter(text: &str) -> ObjectFilter {
        let tokens = lex_line(text, 0).expect("filter text should lex");
        parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false)
            .expect("filter text should parse")
    }

    #[test]
    fn spell_filter_preserves_authored_convoke_ability_requirement() {
        let filter = parse_filter("a spell that has convoke");

        assert_eq!(filter.static_abilities, [StaticAbilityId::Convoke]);
        assert!(filter.ability_markers.is_empty(), "{filter:#?}");
    }

    #[test]
    fn qualified_spell_and_ability_set_keeps_the_complete_stack_domain() {
        let filter = parse_filter("each spell and ability your opponents control");

        assert_eq!(filter.zone, Some(Zone::Stack));
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::SpellOrAbility)
        );
        assert!(!filter.has_mana_cost, "{filter:#?}");
        assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
    }

    #[test]
    fn suffix_conjunction_does_not_weaken_compound_subtype_identity() {
        let filter = parse_filter("Eldrazi Spawn creature you both own and control");

        assert!(filter.subtypes.is_empty(), "{filter:#?}");
        assert_eq!(filter.all_subtypes, vec![Subtype::Eldrazi, Subtype::Spawn]);
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn foretold_owner_zone_filter_keeps_runtime_state_and_authored_scope() {
        let filter = parse_filter("foretold card you own in exile");

        assert!(filter.foretold);
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Exile));
        assert_eq!(filter.description(), "a foretold card you own in exile");
    }

    #[test]
    fn graveyard_cards_with_different_mana_values_keep_selection_constraint() {
        let filter = parse_filter("cards with different mana values from your graveyard");

        assert!(filter.distinct_mana_values, "{filter:#?}");
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert!(
            filter.description().contains("with different mana values"),
            "{}",
            filter.description()
        );
    }

    #[test]
    fn set_quantifier_before_pt_literal_keeps_the_exact_characteristics() {
        let filter = parse_filter("each 1/1 creature you control");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.power, Some(crate::filter::Comparison::Equal(1)));
        assert_eq!(filter.toughness, Some(crate::filter::Comparison::Equal(1)));
    }

    #[test]
    fn excluded_literal_name_keeps_original_case_apostrophe_and_comma_surface() {
        let filter = parse_filter(
            "target legendary permanent card not named Staff of Eden, Vault's Key from a graveyard",
        );

        assert_eq!(
            filter.excluded_name.as_deref(),
            Some("staff of eden vaults key")
        );
        assert_eq!(
            filter.excluded_name_surface(),
            Some("Staff of Eden, Vault's Key")
        );
    }

    #[test]
    fn creature_type_relation_keeps_comparison_controller_out_of_candidate() {
        let filter =
            parse_filter("creature card that shares a creature type with a creature you control");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.controller, None);
        assert_eq!(filter.characteristic_relations.len(), 1);
        let relation = &filter.characteristic_relations[0];
        assert_eq!(relation.kind, ObjectCharacteristicRelationKind::SharesAny);
        assert_eq!(
            relation.characteristics,
            vec![ObjectCharacteristic::Subtype(SubtypeFamily::Creature)]
        );
        assert_eq!(relation.comparison.card_types, vec![CardType::Creature]);
        assert_eq!(relation.comparison.controller, Some(PlayerFilter::You));
        assert_eq!(relation.comparison.zone, Some(Zone::Battlefield));
        assert_eq!(
            filter.description(),
            "creature card that shares a creature type with a creature you control"
        );
    }

    #[test]
    fn attacked_planeswalker_clause_does_not_expand_or_control_the_attacker() {
        let filter = parse_filter("creature that's attacking you or a planeswalker you control");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.controller, None);
        assert!(filter.attacking);
        assert_eq!(
            filter.attacking_player_or_planeswalker_controlled_by,
            Some(PlayerFilter::You)
        );
        assert!(!filter.attacking_player_only);
    }

    #[test]
    fn attacking_your_opponents_keeps_the_opponent_destination_union() {
        let filter =
            parse_filter("creatures attacking your opponents and/or planeswalkers they control");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.controller, None);
        assert!(filter.attacking);
        assert_eq!(
            filter.attacking_player_or_planeswalker_controlled_by,
            Some(PlayerFilter::Opponent)
        );
        assert!(!filter.attacking_player_only);
        assert_eq!(
            filter.description(),
            "creatures attacking your opponents and/or planeswalkers they control"
        );
    }

    #[test]
    fn attacking_alone_is_an_executable_post_noun_state() {
        let filter = parse_filter("creature that's attacking alone");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.attacking);
        assert!(filter.attacking_alone);
        assert_eq!(filter.description(), "creature that's attacking alone");

        let ordinary = parse_filter("attacking creature");
        assert!(ordinary.attacking);
        assert!(!ordinary.attacking_alone);
    }

    #[test]
    fn attacking_last_chosen_player_keeps_persistent_player_relation() {
        let filter = parse_filter("creature attacking the last chosen player");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.attacking);
        assert_eq!(
            filter.attacking_player_or_planeswalker_controlled_by,
            Some(PlayerFilter::ChosenPlayer)
        );
        assert!(filter.attacking_player_only);
    }

    #[test]
    fn source_and_chosen_object_exclusions_keep_both_identities() {
        let filter = parse_filter("creatures other than this creature and the chosen creature");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.other, "the source exclusion must remain executable");
        assert!(matches!(
            filter.source_surface.as_ref(),
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(noun))
                if noun == "this creature"
        ));
        assert_eq!(filter.tagged_constraints.len(), 1, "{filter:#?}");
        assert_eq!(
            filter.tagged_constraints[0],
            TaggedObjectConstraint {
                tag: TagKey::from(crate::cards::builders::CHOSEN_OBJECTS_TAG),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            }
        );
    }

    #[test]
    fn compound_ambiguous_subtype_phrase_keeps_both_subtypes() {
        let filter = parse_filter("all Sand Warriors");

        assert!(filter.subtypes.is_empty(), "{filter:#?}");
        assert_eq!(filter.all_subtypes, vec![Subtype::Sand, Subtype::Warrior]);
    }

    #[test]
    fn attachment_host_noun_does_not_narrow_the_attachment_filter() {
        let filter = parse_filter("Equipment attached to that creature");

        assert!(filter.card_types.is_empty(), "{filter:#?}");
        assert_eq!(filter.subtypes, vec![Subtype::Equipment], "{filter:#?}");
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && constraint.relation == TaggedOpbjectRelation::AttachedToTaggedObject
        }));
    }

    #[test]
    fn explicit_non_subtype_with_numeric_suffix_is_never_readded_as_positive() {
        for (text, card_type, excluded_subtype) in [
            (
                "non-Equipment artifact you control with mana value 4 or greater",
                CardType::Artifact,
                Subtype::Equipment,
            ),
            (
                "non-Aura enchantment you control with mana value 4 or greater",
                CardType::Enchantment,
                Subtype::Aura,
            ),
        ] {
            let filter = parse_filter(text);

            assert_eq!(filter.card_types, vec![card_type], "{filter:#?}");
            assert_eq!(
                filter.excluded_subtypes,
                vec![excluded_subtype],
                "{filter:#?}"
            );
            assert!(!filter.subtypes.contains(&excluded_subtype), "{filter:#?}");
            assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
            assert!(filter.mana_value.is_some(), "{filter:#?}");
        }
    }

    #[test]
    fn historical_block_relation_keeps_partner_characteristics_nested() {
        let filter =
            parse_filter("creature that blocked or was blocked by a Zombie you control this turn");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.subtypes.is_empty());
        assert!(!filter.blocked);
        assert!(!filter.blocking);
        let partner = filter
            .blocked_or_was_blocked_by_this_turn
            .as_deref()
            .expect("typed historical combat partner");
        assert_eq!(partner.subtypes, vec![Subtype::Zombie]);
        assert_eq!(partner.controller, Some(PlayerFilter::You));
        assert_eq!(partner.zone, Some(Zone::Battlefield));
        assert_eq!(
            filter.description(),
            "creature that blocked or was blocked by a Zombie you control this turn"
        );
    }

    #[test]
    fn active_voice_blocked_this_turn_is_history_not_current_combat_state() {
        let filter = parse_filter("target creature that blocked this turn");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(filter.blocked_this_turn, "{filter:#?}");
        assert!(!filter.blocked, "{filter:#?}");
        assert!(!filter.blocking, "{filter:#?}");
        assert_eq!(filter.description(), "creature that blocked this turn");
    }

    #[test]
    fn color_relation_keeps_legendary_comparison_identity_nested() {
        let filter = parse_filter("card that shares a color with a legendary creature you control");

        assert!(filter.has_explicit_card_noun());
        assert!(filter.supertypes.is_empty());
        assert_eq!(filter.controller, None);
        let relation = &filter.characteristic_relations[0];
        assert_eq!(relation.characteristics, vec![ObjectCharacteristic::Color]);
        assert_eq!(relation.comparison.supertypes, vec![Supertype::Legendary]);
        assert_eq!(relation.comparison.card_types, vec![CardType::Creature]);
        assert_eq!(relation.comparison.controller, Some(PlayerFilter::You));
        assert_eq!(
            filter.description(),
            "card that shares a color with a legendary creature you control"
        );
    }

    #[test]
    fn negated_land_type_relation_keeps_basic_on_candidate_only() {
        let filter =
            parse_filter("basic land card that doesn't share a land type with a land you control");

        assert_eq!(filter.supertypes, vec![Supertype::Basic]);
        assert_eq!(filter.card_types, vec![CardType::Land]);
        assert_eq!(filter.controller, None);
        let relation = &filter.characteristic_relations[0];
        assert_eq!(relation.kind, ObjectCharacteristicRelationKind::SharesNone);
        assert_eq!(
            relation.characteristics,
            vec![ObjectCharacteristic::Subtype(SubtypeFamily::Land)]
        );
        assert!(relation.comparison.supertypes.is_empty());
        assert_eq!(relation.comparison.card_types, vec![CardType::Land]);
        assert_eq!(relation.comparison.controller, Some(PlayerFilter::You));
        assert_eq!(
            filter.description(),
            "basic land card that doesn't share a land type with a land you control"
        );
    }

    #[test]
    fn tagged_comparison_surfaces_remain_nested_in_generic_relations() {
        let equipped = parse_filter("creature that shares a color with equipped creature");
        let equipped_relation = &equipped.characteristic_relations[0];
        assert_eq!(
            equipped_relation.characteristics,
            vec![ObjectCharacteristic::Color]
        );
        assert!(
            equipped_relation
                .comparison
                .tagged_constraints
                .iter()
                .any(|constraint| {
                    constraint.tag.as_str() == "equipped"
                        && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                })
        );
        assert_eq!(
            equipped_relation.comparison_description(),
            "equipped creature"
        );

        let exiled = parse_filter("spell that shares a color or mana value with the exiled card");
        let exiled_relation = &exiled.characteristic_relations[0];
        assert_eq!(
            exiled_relation.characteristics,
            vec![ObjectCharacteristic::Color, ObjectCharacteristic::ManaValue]
        );
        assert!(
            exiled_relation
                .comparison
                .tagged_constraints
                .iter()
                .any(|constraint| {
                    constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                        && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                })
        );
        assert_eq!(exiled_relation.comparison_description(), "the exiled card");
    }

    #[test]
    fn convoked_comparison_keeps_its_tag_and_candidate_identity_separate() {
        let filter = parse_filter(
            "creature that shares a creature type with a creature that convoked this spell",
        );

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.tagged_constraints.len(), 0);
        let comparison = &filter.characteristic_relations[0].comparison;
        assert_eq!(comparison.card_types, vec![CardType::Creature]);
        assert!(comparison.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "convoked_this_spell"
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert_eq!(
            filter.description(),
            "creature that shares a creature type with a creature that convoked this spell"
        );
    }

    #[test]
    fn enchanted_by_relation_keeps_host_and_aura_constraints_separate() {
        let filter =
            parse_filter("creature your opponents control that's enchanted by an Aura you control");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
        assert!(filter.subtypes.is_empty());
        assert!(filter.has_relative_attachment_state_surface());
        let aura = filter
            .with_attached_object
            .as_deref()
            .expect("enchanted-by clause should create a nested attachment filter");
        assert_eq!(aura.subtypes, vec![Subtype::Aura]);
        assert_eq!(aura.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn activated_this_turn_is_a_branch_local_executable_object_history_fact() {
        let filter = parse_filter("planeswalker that was activated this turn or tapped creature");

        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(filter.any_of.iter().any(|branch| {
            branch.card_types == [CardType::Planeswalker] && branch.ability_activated_this_turn
        }));
        assert!(filter.any_of.iter().any(|branch| {
            branch.card_types == [CardType::Creature]
                && branch.tapped
                && !branch.ability_activated_this_turn
        }));
        assert_eq!(
            filter.description(),
            "planeswalker that was activated this turn or tapped creature"
        );
    }

    #[test]
    fn not_enchanted_is_the_negative_aura_attachment_predicate() {
        let filter = parse_filter("creatures that aren't enchanted");

        assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
        let aura = filter
            .without_attached_object
            .as_deref()
            .expect("negative enchanted state should retain a typed attachment filter");
        assert_eq!(aura.card_types, [CardType::Enchantment]);
        assert_eq!(aura.subtypes, [Subtype::Aura]);
        assert_eq!(filter.description(), "creature that isn't enchanted");
    }
}

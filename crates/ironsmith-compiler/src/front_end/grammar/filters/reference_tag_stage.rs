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

pub fn compound_filter_subtype_prefix_word_len(words: &[&str]) -> Option<usize> {
    if words.get(..2) == Some(&["time", "lord"]) {
        return Some(2);
    }
    if parse_subtype_flexible(words.first()?).is_some() {
        return Some(1);
    }
    if let Some(next) = words.get(1) {
        let compound = format!("{}-{next}", words[0]);
        if parse_subtype_flexible(&compound).is_some() {
            return Some(2);
        }
    }
    super::super::leaf::classify_token_definition_subtype(words.first()?)?;
    words
        .get(1)
        .and_then(|next| parse_subtype_flexible(next))
        .map(|_| 1)
}

fn parse_compound_filter_subtype(words: &[&str], idx: usize) -> Option<Subtype> {
    if words.get(idx..idx + 2) == Some(&["time", "lord"]) {
        return Some(Subtype::TimeLord);
    }
    parse_subtype_flexible(words.get(idx)?)
        .or_else(|| {
            let compound = format!("{}-{}", words.get(idx)?, words.get(idx + 1)?);
            parse_subtype_flexible(&compound)
        })
        .or_else(|| {
            let subtype = super::super::leaf::classify_token_definition_subtype(words.get(idx)?)?;
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

pub(super) fn token_index_after_word_prefix(
    tokens: &[OwnedLexToken],
    word_len: usize,
) -> Option<usize> {
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
    let with_idx = crate::slice_primitives::select_last_position(&trimmed[..n - 3], |token| {
        token.is_word("with")
    })?;
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
    let enchanted_idx =
        crate::slice_primitives::select_position(&trimmed, |token| token.is_word("enchanted"))?;
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
    let attached_idx =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("attached"))?;
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
pub fn has_plural_object_head_surface(tokens: &[OwnedLexToken]) -> bool {
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
        let surface = match surface {
            crate::target::SourceReferenceSurface::ThisPermanentType(_) => {
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    crate::lexer::render_token_slice(&tokens[..token_len])
                        .trim()
                        .to_string(),
                )
            }
            surface => surface,
        };
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
    word_view.map_word_or_end_to_token_boundary(full_word_idx)
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

    let comparison = crate::object_filters::parse_object_filter_words(rhs, false)?;
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
    let Some(blocked_idx) = crate::word_primitives::parse_sequence_start(
        &words,
        &["blocked", "or", "was", "blocked", "by"],
    ) else {
        return Ok(false);
    };
    let partner_start = blocked_idx + 5;
    let Some(this_turn_idx) =
        crate::word_primitives::parse_last_sequence_start(&words, &["this", "turn"])
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
    if be_words
        .first()
        .is_none_or(|word| parse_word_choice(word, BE_VERB_WORDS).is_none())
        || parse_phrase_at_head(&put_on_words, PUT_ON_PREFIX).is_none()
        || parse_word_choice_anywhere(&put_on_words, PUT_ON_REFERENCE_WORDS).is_none()
    {
        return;
    }

    all_words.drain(0..3);
}

pub fn parse_object_filter_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let attack_destination_relation = is_attack_destination_relation(tokens);
    let mut filter = if attack_destination_relation {
        parse_object_filter(tokens, other)?
    } else {
        parse_object_filter_lexed(tokens, other)?
    };
    if attack_destination_relation {
        filter.set_plural_object_noun_surface(has_plural_object_head_surface(tokens));
    }
    apply_supertype_or_mana_capability_union(&mut filter, tokens);
    super::counter_constraints::preserve_filter_counter_constraint_surface_tokens(
        &mut filter,
        tokens,
    );
    super::simple::preserve_branch_scoped_card_type_union(&mut filter, tokens, other);
    apply_chosen_type_domain(&mut filter, tokens);
    fn remove_explicitly_excluded_positive_subtypes(filter: &mut ObjectFilter) {
        filter.subtypes.retain(|subtype| {
            !filter
                .excluded_subtypes
                .iter()
                .any(|excluded| excluded == subtype)
        });
        filter.all_subtypes.retain(|subtype| {
            !filter
                .excluded_subtypes
                .iter()
                .any(|excluded| excluded == subtype)
        });
        for branch in &mut filter.any_of {
            remove_explicitly_excluded_positive_subtypes(branch);
        }
    }

    // Several permissive recovery passes run after the primary atom parser.
    // Preserve the explicit `non-*` meaning as the final invariant even when
    // one of those passes sees the suffix word without its lexical `non`
    // prefix and tentatively restores it as a positive subtype.
    remove_explicitly_excluded_positive_subtypes(&mut filter);

    // The public object-filter facade can finish through a simple-filter
    // branch before the reference/tag stage gets a chance to reassert the
    // coordinated stack domain. Preserve the grammar-proven union here as a
    // final entrypoint invariant: a spell-and-ability set contains both
    // stack object kinds, and abilities do not acquire a mana-cost predicate.
    let words = non_article_parser_word_refs(tokens);
    if parse_phrase_choice_anywhere(&words, SPELL_AND_ABILITY_PHRASES).is_some() {
        filter.zone = Some(Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::SpellOrAbility);
        filter.has_mana_cost = false;
        filter.set_conjunctive_set_surface(true);
    }
    if filter.card_types.is_empty()
        && words
            .first()
            .is_some_and(|word| matches!(*word, "creature" | "creatures"))
        && words
            .iter()
            .any(|word| matches!(*word, "attack" | "attacks" | "attacking"))
    {
        filter.card_types.push(CardType::Creature);
    }
    if filter.characteristic_relations.is_empty() {
        let mut relation_words = words.clone();
        let mut relation_tokens = tokens.to_vec();
        let _ = try_apply_shared_characteristic_relation_clause(
            &mut filter,
            &mut relation_words,
            &mut relation_tokens,
        )?;
    }

    Ok(filter)
}

pub fn is_attack_destination_relation(tokens: &[OwnedLexToken]) -> bool {
    let words = non_article_parser_word_refs(tokens);
    let Some(attack_idx) = crate::slice_primitives::select_position(&words, |word| {
        matches!(*word, "attack" | "attacks" | "attacking")
    }) else {
        return false;
    };
    words[..attack_idx]
        .iter()
        .any(|word| matches!(*word, "creature" | "creatures"))
        && words[attack_idx + 1..]
            .iter()
            .any(|word| matches!(*word, "planeswalker" | "planeswalkers"))
        && words[attack_idx + 1..]
            .iter()
            .any(|word| matches!(*word, "control" | "controls"))
}

/// Preserve a shared object domain around `supertype OR mana capability`.
///
/// The permissive characteristic scan correctly recognizes the supertype in
/// "land that is snow or could produce {C}", but it cannot represent the
/// second arm without this typed capability predicate and would otherwise
/// silently narrow the legal set to snow lands.
pub fn apply_supertype_or_mana_capability_union(
    filter: &mut ObjectFilter,
    tokens: &[OwnedLexToken],
) {
    if !filter.any_of.is_empty() {
        return;
    }

    let words = parser_token_word_positions(tokens);
    for word_idx in 0..words.len().saturating_sub(6) {
        let window = &words[word_idx..word_idx + 7];
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
        let Some(supertype_idx) =
            crate::slice_primitives::select_position(&filter.supertypes, |candidate| {
                *candidate == supertype
            })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelativeCharacteristicSelector {
    CardType(CardType),
    Subtype(Subtype),
    Token,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PermanentOrSuspendedCardArm {
    Permanent,
    SuspendedCard,
}

#[cfg(test)]
#[path = "reference_tag_stage_inline_shared_characteristic_relation_tests.rs"]
mod shared_characteristic_relation_tests;

#[path = "reference_tag_stage/reference_tag_stage_reference_programs.rs"]
mod reference_tag_stage_reference_programs;
pub(super) use reference_tag_stage_reference_programs::parse_object_filter_inner;
use reference_tag_stage_reference_programs::{
    try_apply_could_be_targeted_by_that_spell_clause,
    try_apply_shared_creature_type_with_source_clause,
};
#[path = "reference_tag_stage/reference_tag_stage_choice_programs.rs"]
mod reference_tag_stage_choice_programs;
use reference_tag_stage_choice_programs::try_apply_no_shared_creature_type_with_chosen_creature_clause;
#[path = "reference_tag_stage/reference_tag_stage_zone_programs.rs"]
mod reference_tag_stage_zone_programs;
use reference_tag_stage_zone_programs::try_apply_no_shared_creature_type_with_your_creatures_or_graveyard_clause;
#[path = "reference_tag_stage/reference_tag_stage_core_programs.rs"]
mod reference_tag_stage_core_programs;
use reference_tag_stage_core_programs::{
    apply_basic_land_exception, positive_relative_characteristic_union,
    preserve_branch_scoped_comparison_union, preserve_relative_characteristic_list_surface,
    relation_clause_is_inside_aggregate_scope, try_apply_distinct_creature_types_clause,
    try_apply_distinct_powers_clause,
};
#[path = "reference_tag_stage/reference_tag_stage_resource_programs.rs"]
mod reference_tag_stage_resource_programs;
use reference_tag_stage_resource_programs::{
    lift_shared_trailing_mana_value_from_type_union, try_apply_distinct_mana_values_clause,
};
#[path = "reference_tag_stage/reference_tag_stage_library_programs.rs"]
mod reference_tag_stage_library_programs;
use reference_tag_stage_library_programs::{
    consume_permanent_or_suspended_card_tail, parse_permanent_or_suspended_card_arm,
    parse_permanent_or_suspended_card_disjunction, strip_other_than_basic_land_cards_clause,
    strip_other_than_basic_land_cards_tokens,
};

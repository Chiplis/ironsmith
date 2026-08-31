use super::super::super::lexer::{
    LexedClause, OwnedLexToken, TokenKind, TokenWordView, render_token_slice, token_slice_first_is,
    trim_lexed_commas,
};
use super::*;
use crate::cards::{TextSpan, builders::TargetAst};
use crate::grammar::activation_costs::parse_activation_cost_tokens;
use crate::grammar::conditions::{
    parse_control_or_controlled_relation_clauses, parse_control_relation_clauses,
    parse_copula_relation_clauses, parse_existential_object_clause, parse_has_relation_clauses,
    parse_negated_control_relation_clauses, parse_prepositional_copula_relation_clauses,
};
use crate::util::{
    FilterKeywordConstraint, is_article, is_source_reference_words, parse_value,
    strip_leading_article_tokens,
};

#[path = "predicate_phrases/advanced.rs"]
mod advanced;
#[path = "predicate_phrases/capture_shapes.rs"]
mod capture_shapes;
#[path = "predicate_phrases/surface.rs"]
mod surface;

pub use advanced::parse_predicate;

#[cfg(test)]
#[path = "predicate_phrases/tests.rs"]
mod tests;

pub use capture_shapes::{WinnowAtom, WinnowCaptureKind, WinnowCaptureRole, WinnowSequence};

const OUTLAW_SHORTHAND_FILTER_PHRASES: &[&[&str]] = &[
    &["outlaw"],
    &["outlaws"],
    &["outlaw", "creature"],
    &["outlaws", "creatures"],
];
const PERMANENT_WORD: &str = "permanent";
const CREATURE_WORD: &str = "creature";
const COUNTER_WORD_PHRASES: &[&[&str]] = &[&["counter"], &["counters"]];
const PERMANENTS_YOU_CONTROL_SCOPE_PHRASES: &[&[&str]] = &[
    &["permanent", "you", "control"],
    &["permanent", "you", "controls"],
    &["permanents", "you", "control"],
    &["permanents", "you", "controls"],
];
const CARDS_IN_YOUR_GRAVEYARD_SCOPE_PHRASES: &[&[&str]] = &[
    &["card", "in", "your", "graveyard"],
    &["cards", "in", "your", "graveyard"],
];
const SACRIFICED_PERMANENTS_SCOPE_PHRASES: &[&[&str]] = &[
    &["sacrificed"],
    &["sacrificed_0"],
    &["sacrificed", "permanent"],
    &["sacrificed", "permanents"],
    &["sacrificed_0", "permanent"],
    &["sacrificed_0", "permanents"],
];
const PERMANENTS_AND_OR_GRAVEYARD_CONNECTOR_PHRASE: &[&str] = &["and/or"];
const PERMANENTS_AND_OR_SPLIT_CONNECTOR_PHRASE: &[&str] = &["and", "or"];
const THERE_ARE_PREFIX: &[&str] = &["there", "are"];
const AND_YOUR_LIFE_TOTAL_PHRASE: &[&str] = &["and", "your", "life", "total"];
const LIFE_TOTAL_AT_LEAST_STARTING_PHRASE: &[&str] = &[
    "your", "life", "total", "is", "greater", "than", "or", "equal", "to", "your", "starting",
    "life", "total",
];
const LIFE_TOTAL_AT_LEAST_LAST_NOTED_PHRASES: &[&[&str]] = &[
    &[
        "your",
        "life",
        "total",
        "is",
        "greater",
        "than",
        "or",
        "equal",
        "to",
        "last",
        "noted",
        "life",
        "total",
        "for",
        "this",
        "permanent",
    ],
    &[
        "your",
        "life",
        "total",
        "is",
        "greater",
        "than",
        "or",
        "equal",
        "to",
        "last",
        "noted",
        "life",
        "total",
        "for",
        "this",
        "enchantment",
    ],
    &[
        "your", "life", "total", "is", "greater", "than", "or", "equal", "to", "last", "noted",
        "life", "total", "for", "this", "artifact",
    ],
    &[
        "your", "life", "total", "is", "greater", "than", "or", "equal", "to", "last", "noted",
        "life", "total", "for", "this", "creature",
    ],
    &[
        "your", "life", "total", "is", "greater", "than", "or", "equal", "to", "last", "noted",
        "life", "total", "for", "this", "land",
    ],
];
const OR_MORE_PREFIX: &[&str] = &["or", "more"];
const HAS_OR_HAVE_WORDS: &[&str] = &["has", "have"];
const INSTEAD_WORD: &str = "instead";
const OTHER_OR_ANOTHER_WORDS: &[&str] = &["another", "other"];
const OR_WORD: &str = "or";
const CARD_WORD: &str = "card";
const CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const NONLAND_CARD_OBJECT_PHRASES: &[&[&str]] = &[
    &["nonland", "card"],
    &["nonland", "cards"],
    &["non", "land", "card"],
    &["non", "land", "cards"],
];
const BEEN_EXILED_WITH_THIS_SOURCE_PREFIXES: &[&[&str]] = &[
    &["been", "exiled", "with", "this"],
    &["exiled", "with", "this"],
];
const COST_PAID_INSTEAD_TAIL_PHRASES: &[&[&str]] =
    &[&["cost", "was", "paid"], &["cost", "wasnt", "paid"]];
const COST_NOT_PAID_INSTEAD_TAIL_PHRASE: &[&str] = &["cost", "was", "not", "paid"];
const YOU_BOTH_OWN_AND_CONTROL_PHRASE: &[&str] = &["you", "both", "own", "and", "control"];
const EXILE_THEM_PHRASE: &[&str] = &["exile", "them"];
const DEFINITE_ARTICLE_WORD: &str = "the";
const MANA_SPENT_TO_CAST_THIS_SPELL_PHRASES: &[&[&str]] = &[
    &["was", "spent", "to", "cast", "this", "spell"],
    &["were", "spent", "to", "cast", "this", "spell"],
    &["was", "spent", "to", "cast", "it"],
    &["were", "spent", "to", "cast", "it"],
    &["was", "spent", "to", "cast", "that", "spell"],
    &["were", "spent", "to", "cast", "that", "spell"],
];
const YOU_CONTROL_PREFIXES: &[&[&str]] = &[&["you", "control"], &["you", "controls"]];
const THAT_PLAYER_CONTROLS_PREFIXES: &[&[&str]] = &[
    &["that", "player", "control"],
    &["that", "player", "controls"],
    &["that", "players", "control"],
    &["that", "players", "controls"],
    &["they", "control"],
    &["they", "controls"],
];
const WITH_DIFFERENT_POWERS_TAIL_PHRASES: &[&[&str]] = &[
    &["with", "different", "powers"],
    &["with", "different", "power"],
];
const TOUGHNESS_GREATER_THAN_POWER_TAIL_PHRASES: &[&[&str]] = &[
    &[
        "that",
        "each",
        "have",
        "toughness",
        "greater",
        "than",
        "their",
        "power",
    ],
    &[
        "that",
        "each",
        "has",
        "toughness",
        "greater",
        "than",
        "its",
        "power",
    ],
    &["with", "toughness", "greater", "than", "their", "power"],
    &["with", "toughness", "greater", "than", "its", "power"],
    &["with", "power", "less", "than", "their", "toughness"],
    &["with", "power", "less", "than", "its", "toughness"],
];
const POWER_GREATER_THAN_TOUGHNESS_TAIL_PHRASES: &[&[&str]] = &[
    &[
        "that",
        "each",
        "have",
        "power",
        "greater",
        "than",
        "their",
        "toughness",
    ],
    &[
        "that",
        "each",
        "has",
        "power",
        "greater",
        "than",
        "its",
        "toughness",
    ],
    &["with", "power", "greater", "than", "their", "toughness"],
    &["with", "power", "greater", "than", "its", "toughness"],
    &["with", "toughness", "less", "than", "their", "power"],
    &["with", "toughness", "less", "than", "its", "power"],
];
const NOT_TOKEN_PREFIX: &[&str] = &["not", "token"];
const AND_WORD: &str = "and";
const IT_WORD: &str = "it";
const THAT_WORD: &str = "that";
const NO_WORD: &str = "no";
const PREDICATE_REFERENCE_NOUN_WORDS: &[&str] = &[
    "artifact",
    "card",
    "creature",
    "creatures",
    "enchantment",
    "land",
    "object",
    "permanent",
    "source",
    "spell",
    "token",
];
const ENCHANTMENT_WORD: &str = "enchantment";
const PREDICATE_REFERENCE_START_WORDS: &[&str] = &[
    "it", "its", "this", "that", "you", "your", "opponent", "player", "target", "source", "there",
];
const OR_COMPARISON_TAIL_WORDS: &[&str] = &["more", "fewer", "less", "greater", "equal"];
const ITS_WORDS: &[&str] = &["its", "it's"];
const YOUR_WORD: &str = "your";
const THEIR_WORD: &str = "their";
const HAVE_WORD: &str = "have";
const DOESNT_HAVE_PHRASES: &[&[&str]] = &[&["doesnt", "have"], &["doesn't", "have"]];
const DOES_NOT_HAVE_PHRASE: &[&str] = &["does", "not", "have"];
const NEGATED_HAVE_PHRASES: &[&[&str]] = &[
    &["doesnt", "have"],
    &["doesn't", "have"],
    &["does", "not", "have"],
];
const YOU_WORD: &str = "you";
const MORE_WORD: &str = "more";
const THAN_WORD: &str = "than";
const COLORS_SPENT_TO_CAST_SOURCE_TAIL_PHRASES: &[&[&str]] = &[
    &[
        "less", "than", "or", "equal", "to", "number", "of", "colors", "of", "mana", "spent", "to",
        "cast", "this", "spell",
    ],
    &[
        "less", "than", "or", "equal", "to", "number", "of", "color", "of", "mana", "spent", "to",
        "cast", "this", "spell",
    ],
];
const POWER_WORD: &str = "power";
const POWER_OR_TOUGHNESS_WORDS: &[&str] = &["power", "toughness"];
const IS_OR_ARE_WORDS: &[&str] = &["is", "are"];
const BE_VERB_WORDS: &[&str] = &["is", "are", "was", "were"];
const MANA_SYMBOL_WORDS: &[&str] = &["w", "u", "b", "r", "g", "c", "s"];
const SOURCE_FILTER_IGNORED_DESCRIPTOR_WORDS: &[&str] =
    &["attached", "tapped", "untapped", "saddled", "crewed"];
const AURA_WORDS: &[&str] = &["aura", "auras"];
const CONTROL_WORD: &str = "control";
const CONTROL_OR_CONTROLS_WORDS: &[&str] = &["control", "controls"];
const ZONE_WORDS: &[&str] = &["graveyard", "hand", "exile", "library"];
const YOUR_GRAVEYARD_PHRASE: &[&str] = &["your", "graveyard"];
const THAT_PLAYER_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &["that", "player", "graveyard"],
    &["that", "players", "graveyard"],
];
const TARGET_PLAYER_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &["target", "player", "graveyard"],
    &["target", "players", "graveyard"],
];
const TARGET_OPPONENT_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &["target", "opponent", "graveyard"],
    &["target", "opponents", "graveyard"],
];
const OPPONENT_GRAVEYARD_PHRASES: &[&[&str]] =
    &[&["opponent", "graveyard"], &["opponents", "graveyard"]];
const THAT_PLAYER_SUBJECT_PREFIX: &[&str] = &["that", "player"];
const TARGET_PLAYER_SUBJECT_PREFIX: &[&str] = &["target", "player"];
const TARGET_OPPONENT_SUBJECT_PREFIX: &[&str] = &["target", "opponent"];
const EACH_OPPONENT_SUBJECT_PREFIX: &[&str] = &["each", "opponent"];
const A_OR_ANY_PLAYER_SUBJECT_PREFIXES: &[&[&str]] = &[&["a", "player"], &["any", "player"]];
const DEFENDING_PLAYER_SUBJECT_PREFIX: &[&str] = &["defending", "player"];
const ATTACKING_PLAYER_SUBJECT_PREFIX: &[&str] = &["attacking", "player"];
const OPPONENT_SUBJECT_PREFIXES: &[&[&str]] = &[&["opponent"], &["opponents"]];
const PLAYER_SUBJECT_WORD: &str = "player";
const AN_OR_THE_OPPONENT_SUBJECT_PHRASES: &[&[&str]] = &[&["an", "opponent"], &["the", "opponent"]];
const HALF_STARTING_LIFE_TOTAL_PHRASES: &[&[&str]] = &[
    &["half", "your", "starting", "life", "total"],
    &["half", "their", "starting", "life", "total"],
    &["half", "that", "players", "starting", "life", "total"],
    &["half", "target", "players", "starting", "life", "total"],
    &["half", "target", "opponents", "starting", "life", "total"],
    &["half", "opponents", "starting", "life", "total"],
    &["half", "defending", "players", "starting", "life", "total"],
    &["half", "attacking", "players", "starting", "life", "total"],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalCounterPhrase {
    counter_type: Option<ironsmith_core::counter::CounterType>,
    count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HalfStartingLifeThreshold {
    AtMost,
    LessThan,
}

fn token_word_is_any(token: &OwnedLexToken, expected: &[&str]) -> bool {
    token.parser_word_pieces().iter().any(|piece| {
        let mut idx = 0usize;
        while idx < expected.len() {
            if expected[idx] == piece.text.as_str() {
                return true;
            }
            idx += 1;
        }
        false
    })
}

fn token_word_is(token: &OwnedLexToken, expected: &str) -> bool {
    token_word_is_any(token, &[expected])
}

fn word_is_any(word: &str, expected: &[&str]) -> bool {
    let mut idx = 0usize;
    while idx < expected.len() {
        if expected[idx] == word {
            return true;
        }
        idx += 1;
    }
    false
}

fn token_index_for_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if token_word_is(&tokens[idx], expected) {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn token_index_for_word_from(
    tokens: &[OwnedLexToken],
    expected: &str,
    start: usize,
) -> Option<usize> {
    let mut idx = start;
    while idx < tokens.len() {
        if token_word_is(&tokens[idx], expected) {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn non_article_token_word_refs(tokens: &[OwnedLexToken]) -> Vec<&str> {
    tokens
        .iter()
        .flat_map(|token| token.parser_word_pieces().iter())
        .map(|piece| piece.text.as_str())
        .filter(|word| !is_article(word))
        .collect()
}

fn non_article_token_words_starts_with_any(tokens: &[OwnedLexToken], prefixes: &[&[&str]]) -> bool {
    let words = non_article_token_word_refs(tokens);
    prefixes.iter().any(|prefix| {
        words.len() >= prefix.len()
            && words
                .iter()
                .zip(prefix.iter())
                .all(|(word, expected)| word == expected)
    })
}

fn non_article_token_words_eq_phrase(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    let words = non_article_token_word_refs(tokens);
    words.len() == phrase.len()
        && words
            .iter()
            .zip(phrase.iter())
            .all(|(word, expected)| word == expected)
}

fn non_article_token_words_eq_any(tokens: &[OwnedLexToken], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| non_article_token_words_eq_phrase(tokens, phrase))
}

fn is_source_reference_clause(clause: LexedClause<'_>) -> bool {
    let words = clause.word_refs();
    surface::exact_any(clause, &[&["it"], &["its"], &["it's"]]) || is_source_reference_words(&words)
}

fn is_explicit_source_state_subject_clause(clause: LexedClause<'_>) -> bool {
    !surface::exact_any(clause, &[&["it"], &["its"]]) && is_source_reference_clause(clause)
}

fn is_source_card_reference_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["this"], &["this", "card"]])
}

fn parse_source_zone_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_prepositional_copula_relation_clauses(tokens, &["in", "on"])?;
    let source = relation.subject_clause;
    if !is_source_reference_clause(source) {
        return None;
    }

    let zone = relation.tail_clause;
    if surface::exact_any(zone, &[&["the", "battlefield"], &["battlefield"]]) {
        return Some(PredicateAst::SourceIsInZone(Zone::Battlefield));
    }
    if surface::exact(zone, &["your", "graveyard"]) {
        return Some(PredicateAst::SourceIsInZone(Zone::Graveyard));
    }
    if !is_source_card_reference_clause(source) {
        return None;
    }
    if surface::exact(zone, &["your", "hand"]) {
        return Some(PredicateAst::SourceIsInZone(Zone::Hand));
    }
    if surface::exact(zone, &["your", "library"]) {
        return Some(PredicateAst::SourceIsInZone(Zone::Library));
    }
    if surface::exact(zone, &["exile"]) {
        return Some(PredicateAst::SourceIsInZone(Zone::Exile));
    }
    if surface::exact_any(zone, &[&["the", "command", "zone"], &["command", "zone"]]) {
        return Some(PredicateAst::SourceIsInZone(Zone::Command));
    }
    None
}

/// Parse an ordered-graveyard predicate such as
/// `this card is in your graveyard with three or more creature cards above it`.
/// The object filter describes the cards above the source; its zone is supplied
/// by the ordered graveyard traversal at runtime rather than by ordinary
/// battlefield filter defaults.
fn parse_source_graveyard_cards_above_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    const PREFIX: [&str; 7] = ["this", "card", "is", "in", "your", "graveyard", "with"];
    if !crate::word_primitives::parse_sequence_prefix(&words, &PREFIX) {
        return Ok(None);
    }
    if words.len() < PREFIX.len() + 5
        || words.get(8..10) != Some(&["or", "more"])
        || words.get(words.len().saturating_sub(2)..) != Some(&["above", "it"])
    {
        return Err(CardTextError::ParseError(format!(
            "malformed ordered-graveyard source predicate: {}",
            words.join(" ")
        )));
    }
    let count = crate::util::parse_number_word_i32(words[7]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "invalid ordered-graveyard card count: {}",
            words[7]
        ))
    })?;
    let count = u32::try_from(count).map_err(|_| {
        CardTextError::ParseError(format!(
            "ordered-graveyard card count must be positive: {}",
            words[7]
        ))
    })?;
    if count == 0 {
        return Err(CardTextError::ParseError(
            "ordered-graveyard card count must be positive".to_string(),
        ));
    }

    let count_token =
        crate::slice_primitives::select_position(tokens, |token| token.is_word(words[7]))
            .ok_or_else(|| {
                CardTextError::ParseError("missing ordered-graveyard count token".to_string())
            })?;
    let or_token = crate::slice_primitives::select_position(&tokens[count_token + 1..], |token| {
        token.is_word("or")
    })
    .ok_or_else(|| CardTextError::ParseError("missing ordered-graveyard comparison".to_string()))?
        + count_token
        + 1;
    let above_token =
        crate::slice_primitives::select_last_position(tokens, |token| token.is_word("above"))
            .ok_or_else(|| {
                CardTextError::ParseError("missing ordered-graveyard position".to_string())
            })?;
    if !tokens
        .get(or_token + 1)
        .is_some_and(|token| token.is_word("more"))
        || above_token <= or_token + 2
    {
        return Err(CardTextError::ParseError(
            "malformed ordered-graveyard comparison".to_string(),
        ));
    }
    let mut filter = parse_object_filter(&tokens[or_token + 2..above_token], false)?;
    if filter.zone == Some(Zone::Battlefield) {
        filter.zone = None;
    }
    if filter.zone.is_some() || filter.controller.is_some() || filter.owner.is_some() {
        return Err(CardTextError::ParseError(
            "ordered-graveyard filter cannot carry a zone or player scope".to_string(),
        ));
    }
    Ok(Some(PredicateAst::SourceInGraveyardWithCardsAbove {
        filter,
        count,
    }))
}

fn parse_outlaw_shorthand_filter(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    let trimmed_tokens = strip_leading_article_tokens(clause.tokens());
    if !surface::exact_any(
        LexedClause::new(trimmed_tokens),
        OUTLAW_SHORTHAND_FILTER_PHRASES,
    ) {
        return None;
    }

    let mut filter = ObjectFilter::default();
    push_outlaw_subtypes(&mut filter.subtypes);
    filter.card_types.push(CardType::Creature);
    Some(filter)
}

fn parse_attachment_quantity_prefix(
    tokens: &[OwnedLexToken],
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    parse_quantity_comparison_prefix(tokens, false, false, "attachment-count predicate")
}

fn parse_source_attachment_count_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(relation) = parse_copula_relation_clauses(tokens) else {
        return Ok(None);
    };
    let enchanted_by_phrase = &["enchanted", "by"];
    let atoms = [
        WinnowSequence::capture(
            "enchanted_by",
            WinnowCaptureKind::WordCount(enchanted_by_phrase.len()),
        ),
        WinnowSequence::amount("quantity", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(relation.tail_clause) else {
        return Ok(None);
    };
    let source = relation.subject_clause;
    if !is_source_state_subject_clause(source) {
        return Ok(None);
    }
    let enchanted_by = matched
        .capture_clause("enchanted_by", relation.tail_clause)
        .ok_or_else(|| CardTextError::ParseError("missing enchanted-by phrase".to_string()))?;
    if !surface::exact(enchanted_by, &["enchanted", "by"]) {
        return Ok(None);
    }
    let attachment = matched
        .capture_clause_by_role(WinnowCaptureRole::Amount, relation.tail_clause)
        .ok_or_else(|| CardTextError::ParseError("missing attachment count".to_string()))?;
    let (comparison, used) = parse_attachment_quantity_prefix(attachment.tokens())?;
    let filter_tokens = attachment.tokens().get(used..).unwrap_or_default();
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_attachment_count_filter_tokens(filter_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attachment-count predicate tail (predicate: '{}')",
            render_token_slice(tokens)
        ))
    })?;

    Ok(Some(PredicateAst::SourceHasAttachmentsMatching {
        filter,
        comparison,
        display: LexedClause::new(tokens).text(),
    }))
}

fn parse_attachment_count_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    crate::grammar::primitives::probe_shape(parse_object_filter(tokens, false))
        .or_else(|| parse_aura_attachment_filter_clause(LexedClause::new(tokens)))
}

fn parse_aura_attachment_filter_clause(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    const AURA_ATTACHMENT_FILTER_PATTERN: WinnowSequence<'static> =
        WinnowSequence::new(&[WinnowSequence::object(
            "aura",
            WinnowCaptureKind::OneOf(AURA_WORDS),
        )]);

    AURA_ATTACHMENT_FILTER_PATTERN
        .accepts_full(clause)
        .then(|| ObjectFilter::default().with_subtype(Subtype::Aura))
}

fn object_filter_has_identity(filter: &ObjectFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.supertypes.is_empty()
        || filter.colors.is_some()
        || filter.required_colors.is_some()
        || filter.sticker.is_some()
        || filter.token
        || filter.nontoken
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
}

fn parse_source_identity_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let state_phrases: &[&[&str]] = &[
        &["is"],
        &["are"],
        &["isnt"],
        &["isn't"],
        &["arent"],
        &["aren't"],
    ];
    let atoms = [
        WinnowSequence::subject("source", WinnowCaptureKind::UntilAnyPhrase(state_phrases)),
        WinnowSequence::action(
            "state",
            WinnowCaptureKind::OneOf(&["is", "are", "isnt", "isn't", "arent", "aren't"]),
        ),
        WinnowSequence::object("descriptor", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let source = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_source_reference_clause(source) {
        return None;
    }
    if surface::exact(source, &["it"]) {
        return None;
    }
    let action = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    let mut negative = source_identity_copula_is_negative(action);
    let descriptor_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let (descriptor_negative, descriptor_clause) =
        parse_source_identity_descriptor_clause(descriptor_clause)?;
    negative |= descriptor_negative;
    if descriptor_clause.tokens().is_empty() {
        return None;
    }
    if source_identity_descriptor_contains_ignored_state(descriptor_clause) {
        return None;
    }
    let filter = crate::grammar::primitives::probe_shape(parse_object_filter(
        descriptor_clause.tokens(),
        false,
    ))
    .or_else(|| parse_color_only_object_filter_word_refs(descriptor_clause))
    .or_else(|| parse_identity_descriptor_filter_tokens(descriptor_clause.tokens()))?;
    if !object_filter_has_identity(&filter) {
        return None;
    }
    let predicate = PredicateAst::SourceMatches(filter);
    Some(if negative {
        PredicateAst::Not(Box::new(predicate))
    } else {
        predicate
    })
}

fn parse_identity_descriptor_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let [token] = tokens else {
        return None;
    };
    if let Some(card_type) =
        parse_card_type(token.parser_text()).filter(|card_type| is_permanent_type(*card_type))
    {
        return Some(ObjectFilter::default().with_type(card_type));
    }
    parse_subtype_flexible(token.parser_text())
        .map(|subtype| ObjectFilter::default().with_subtype(subtype))
}

fn parse_source_identity_descriptor_clause<'a>(
    descriptor: LexedClause<'a>,
) -> Option<(bool, LexedClause<'a>)> {
    const NEGATED_DESCRIPTOR_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::word("not"),
        WinnowSequence::object("descriptor", WinnowCaptureKind::Rest),
    ]);

    if let Some(matched) = NEGATED_DESCRIPTOR_PATTERN.parse_full(descriptor) {
        let descriptor = matched.capture_clause_by_role(WinnowCaptureRole::Object, descriptor)?;
        return Some((true, descriptor));
    }

    Some((false, descriptor))
}

fn source_identity_copula_is_negative(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["isnt"], &["isn't"], &["arent"], &["aren't"]])
}

fn is_there_are_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["there", "are"])
}

fn source_identity_descriptor_contains_ignored_state(descriptor: LexedClause<'_>) -> bool {
    const IGNORED_SOURCE_DESCRIPTOR_PATTERN: WinnowSequence<'static> =
        WinnowSequence::new(&[WinnowSequence::any_word(
            SOURCE_FILTER_IGNORED_DESCRIPTOR_WORDS,
        )]);

    IGNORED_SOURCE_DESCRIPTOR_PATTERN
        .locate_in(descriptor)
        .is_some()
}

fn parse_filter_keyword_constraint_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(FilterKeywordConstraint, usize)> {
    let words = TokenWordView::new(tokens);
    let constraint_word_refs = words.word_refs();
    let (constraint, consumed_words) =
        parse_filter_keyword_constraint_words(&constraint_word_refs)?;
    let consumed_tokens = words.token_index_after_words(consumed_words)?;
    Some((constraint, consumed_tokens))
}

pub fn parse_source_keyword_condition_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let relation = parse_has_relation_clauses(tokens)?;
    let subject_words = relation.subject_clause.words().word_refs();
    let explicit_this_type = crate::word_primitives::parse_choice_sequence_complete(
        &subject_words,
        &[
            &["this"],
            &[
                "creature",
                "permanent",
                "artifact",
                "enchantment",
                "land",
                "spell",
                "source",
            ],
        ],
    );
    if !is_source_reference_clause(relation.subject_clause) && !explicit_this_type {
        return None;
    }
    let (constraint, consumed) =
        parse_filter_keyword_constraint_tokens(relation.tail_clause.tokens())?;
    if consumed != relation.tail_clause.tokens().len() {
        return None;
    }
    let mut filter = ObjectFilter::default();
    apply_filter_keyword_constraint(&mut filter, constraint, false);
    if crate::word_primitives::parse_sequence_complete(&subject_words, &["it"]) {
        filter.set_trailing_candidate_ability_condition_surface(true);
    }
    Some(filter)
}

fn parse_source_keyword_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_source_keyword_condition_filter(tokens).map(PredicateAst::SourceMatches)
}

fn parse_triggering_object_keyword_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    if !surface::exact_any(
        relation.subject_clause,
        &[&["it"], &["that", "object"], &["that", "spell"]],
    ) {
        return None;
    }
    let (constraint, consumed) =
        parse_filter_keyword_constraint_tokens(relation.tail_clause.tokens())?;
    if consumed != relation.tail_clause.tokens().len() {
        return None;
    }
    let mut filter = ObjectFilter::default();
    apply_filter_keyword_constraint(&mut filter, constraint, false);
    filter.set_trailing_candidate_ability_condition_surface(true);
    Some(PredicateAst::ItMatches(filter))
}

fn parse_you_life_total_at_most_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let have_atoms = [
        WinnowSequence::amount("amount", WinnowCaptureKind::UntilLastPhrase(&["life"])),
        WinnowSequence::object("unit", WinnowCaptureKind::OneOf(&["life"])),
    ];
    if let Some(relation) = parse_has_relation_clauses(tokens)
        && let Some(matched) = WinnowSequence::new(&have_atoms).parse_full(relation.tail_clause)
        && surface::exact(relation.subject_clause, &["you"])
    {
        let amount = matched
            .capture_clause_by_role(WinnowCaptureRole::Amount, relation.tail_clause)
            .ok_or_else(|| {
                CardTextError::ParseError("missing amount in life predicate".to_string())
            })?;
        return life_total_at_most_from_amount_tokens(amount.tokens());
    }

    let total_atoms = [
        WinnowSequence::subject("life_total", WinnowCaptureKind::WordCount(3)),
        WinnowSequence::action("action", WinnowCaptureKind::OneOf(&["is"])),
        WinnowSequence::amount("amount", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&total_atoms).parse_full(clause) else {
        return Ok(None);
    };
    let subject = matched
        .capture_clause_by_role(WinnowCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing subject in life predicate".to_string())
        })?;
    if !surface::exact(subject, &["your", "life", "total"]) {
        return Ok(None);
    }
    let amount = matched
        .capture_clause_by_role(WinnowCaptureRole::Amount, clause)
        .ok_or_else(|| CardTextError::ParseError("missing amount in life predicate".to_string()))?;
    life_total_at_most_from_amount_tokens(amount.tokens())
}

fn life_total_at_most_from_amount_tokens(
    amount_tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(parsed) =
        crate::grammar::shared_util::value_shapes::parse_quantity_comparison_prefix_tokens(
            amount_tokens,
            false,
            false,
        )
    else {
        // Qualitative amounts such as `the most` belong to the life-relation
        // grammar. They are a clean no-match for this numeric leaf.
        return Ok(None);
    };
    let Some(amount) = crate::util::comparison_to_strict_at_most_threshold(&parsed.comparison)
    else {
        return Ok(None);
    };
    if parsed.consumed_tokens != amount_tokens.len() {
        return Ok(None);
    }
    Ok(Some(PredicateAst::ValueComparison {
        left: Value::LifeTotal(PlayerFilter::You),
        operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
        right: Value::Fixed(amount as i32),
    }))
}

fn parse_half_starting_life_total_threshold_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = parse_copula_relation_clauses(tokens)?;
    let player = parse_life_total_subject_clause(relation.subject_clause)?;
    match parse_half_starting_life_total_threshold_clause(relation.tail_clause)? {
        HalfStartingLifeThreshold::AtMost => {
            Some(PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { player })
        }
        HalfStartingLifeThreshold::LessThan => {
            Some(PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { player })
        }
    }
}

fn parse_life_total_subject_clause(clause: LexedClause<'_>) -> Option<PlayerAst> {
    if surface::exact(clause, &["your", "life", "total"]) {
        return Some(PlayerAst::You);
    }
    if surface::exact_any(
        clause,
        &[
            &["their", "life", "total"],
            &["that", "players", "life", "total"],
        ],
    ) {
        return Some(PlayerAst::That);
    }
    if surface::exact(clause, &["target", "players", "life", "total"]) {
        return Some(PlayerAst::Target);
    }
    if surface::exact(clause, &["target", "opponents", "life", "total"]) {
        return Some(PlayerAst::TargetOpponent);
    }
    if surface::exact_any(
        clause,
        &[
            &["opponent", "life", "total"],
            &["opponents", "life", "total"],
        ],
    ) {
        return Some(PlayerAst::Opponent);
    }
    if surface::exact(clause, &["defending", "players", "life", "total"]) {
        return Some(PlayerAst::Defending);
    }
    if surface::exact(clause, &["attacking", "players", "life", "total"]) {
        return Some(PlayerAst::Attacking);
    }
    None
}

fn parse_half_starting_life_total_threshold_clause(
    clause: LexedClause<'_>,
) -> Option<HalfStartingLifeThreshold> {
    const AT_MOST_HALF_STARTING_LIFE_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::phrase(&["less", "than", "or", "equal", "to"]),
        WinnowSequence::any_phrase(HALF_STARTING_LIFE_TOTAL_PHRASES),
    ]);
    const LESS_THAN_HALF_STARTING_LIFE_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::phrase(&["less", "than"]),
        WinnowSequence::any_phrase(HALF_STARTING_LIFE_TOTAL_PHRASES),
    ]);

    if AT_MOST_HALF_STARTING_LIFE_PATTERN.accepts_full(clause) {
        Some(HalfStartingLifeThreshold::AtMost)
    } else if LESS_THAN_HALF_STARTING_LIFE_PATTERN.accepts_full(clause) {
        Some(HalfStartingLifeThreshold::LessThan)
    } else {
        None
    }
}

fn parse_source_power_threshold_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_source_possessive_power_threshold_shape(tokens)
        .or_else(|| parse_source_has_power_threshold_shape(tokens))
}

fn parse_source_possessive_power_threshold_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("source", WinnowCaptureKind::UntilPhrase(&["power"])),
        WinnowSequence::object("stat", WinnowCaptureKind::OneOf(&["power"])),
        WinnowSequence::action("copula", WinnowCaptureKind::OneOf(&["is"])),
        WinnowSequence::amount("amount", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let source_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_explicit_source_state_subject_clause(source_clause) {
        return None;
    }
    let amount_clause = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    source_power_at_least_from_amount_tokens(amount_clause.tokens())
}

fn parse_source_has_power_threshold_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let atoms = [
        WinnowSequence::object("stat", WinnowCaptureKind::OneOf(&["power"])),
        WinnowSequence::amount("amount", WinnowCaptureKind::Rest),
    ];
    let relation = parse_has_relation_clauses(tokens)?;
    if !is_explicit_source_state_subject_clause(relation.subject_clause) {
        return None;
    }
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let amount_clause =
        matched.capture_clause_by_role(WinnowCaptureRole::Amount, relation.tail_clause)?;
    source_power_at_least_from_amount_tokens(amount_clause.tokens())
}

fn source_power_at_least_from_amount_tokens(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let (comparison, used) = predicate_quantity_prefix_tokens(tokens)?;
    if used != tokens.len() {
        return None;
    }
    let count = comparison_to_at_least_threshold(&comparison)?;
    Some(PredicateAst::SourcePowerAtLeast(count))
}

fn parse_source_simple_state_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_source_bare_state_shape(tokens).or_else(|| parse_source_copula_state_shape(tokens))
}

fn parse_source_crewed_by_exactly_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let action_phrase = &["was", "crewed", "by", "exactly"];
    let atoms = [
        WinnowSequence::subject("source", WinnowCaptureKind::UntilPhrase(action_phrase)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
        WinnowSequence::amount("count", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("filter", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let subject = matched
        .capture_clause_by_role(WinnowCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing source in crew-count predicate".to_string())
        })?;
    if !is_source_reference_clause(subject) {
        return Ok(None);
    }
    let action = matched
        .capture_clause_by_role(WinnowCaptureRole::Action, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing action in crew-count predicate".to_string())
        })?;
    if !surface::exact(action, action_phrase) {
        return Ok(None);
    }
    let count_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Amount, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing crew-count predicate quantity".to_string())
        })?;
    let Some((count, used)) = parse_number(count_clause.tokens()) else {
        return Err(CardTextError::ParseError(format!(
            "missing crew-count predicate quantity (predicate: '{}')",
            render_token_slice(tokens)
        )));
    };
    if used != count_clause.tokens().len() {
        return Ok(None);
    }
    let filter_tokens = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing crew-count predicate filter".to_string())
        })?
        .tokens();
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing crew-count predicate filter (predicate: '{}')",
            render_token_slice(tokens)
        )));
    }
    let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported crew-count predicate filter (predicate: '{}')",
            render_token_slice(tokens)
        ))
    })?;
    Ok(Some(PredicateAst::SourceCrewedByExactly { count, filter }))
}

fn parse_source_bare_state_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let state_phrases: &[&[&str]] = &[
        &["enchanted"],
        &["equipped"],
        &["renowned"],
        &["tapped"],
        &["untapped"],
    ];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilAnyPhrase(state_phrases)),
        WinnowSequence::object(
            "state",
            WinnowCaptureKind::OneOf(&["enchanted", "equipped", "tapped", "untapped"]),
        ),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_source_state_subject_clause(subject_clause) {
        return None;
    }
    let state_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    source_state_predicate_from_clause(state_clause, false)
}

fn parse_source_copula_state_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_source_positive_copula_state_shape(tokens)
        .or_else(|| parse_source_negative_copula_state_shape(tokens))
}

fn parse_source_positive_copula_state_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_copula_relation_clauses(tokens)?;
    if !is_source_state_subject_clause(relation.subject_clause) {
        return None;
    }
    source_state_predicate_from_clause(relation.tail_clause, false)
}

fn parse_source_negative_copula_state_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let negated_copula_phrases: &[&[&str]] = &[&["isnt"], &["isn't"], &["is", "not"]];
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilAnyPhrase(negated_copula_phrases),
        ),
        WinnowSequence::action(
            "copula",
            WinnowCaptureKind::OneOfPhrase(negated_copula_phrases),
        ),
        WinnowSequence::object(
            "state",
            WinnowCaptureKind::OneOf(&[
                "enchanted",
                "equipped",
                "renowned",
                "tapped",
                "untapped",
                "saddled",
            ]),
        ),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_source_state_subject_clause(subject_clause) {
        return None;
    }
    let state_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    source_state_predicate_from_clause(state_clause, true)
}

fn is_source_state_subject_clause(clause: LexedClause<'_>) -> bool {
    is_source_reference_clause(clause)
        || this_source_surface_for_words(&clause.word_refs()).is_some()
}

fn source_state_predicate_from_clause(
    clause: LexedClause<'_>,
    negative: bool,
) -> Option<PredicateAst> {
    if surface::exact(clause, &["tapped"]) {
        return if negative {
            Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)))
        } else {
            Some(PredicateAst::SourceIsTapped)
        };
    }
    if surface::exact(clause, &["untapped"]) {
        return if negative {
            Some(PredicateAst::SourceIsTapped)
        } else {
            Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)))
        };
    }
    if surface::exact(clause, &["equipped"]) {
        return if negative {
            Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsEquipped)))
        } else {
            Some(PredicateAst::SourceIsEquipped)
        };
    }
    if surface::exact(clause, &["enchanted"]) {
        return if negative {
            Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsEnchanted)))
        } else {
            Some(PredicateAst::SourceIsEnchanted)
        };
    }
    if surface::exact(clause, &["saddled"]) {
        return if negative {
            Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsSaddled)))
        } else {
            Some(PredicateAst::SourceIsSaddled)
        };
    }
    if surface::exact(clause, &["renowned"]) {
        return if negative {
            Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsRenowned)))
        } else {
            Some(PredicateAst::SourceIsRenowned)
        };
    }
    None
}

fn parse_terminal_counter_phrase(
    tokens: &[OwnedLexToken],
) -> Option<Option<ironsmith_core::counter::CounterType>> {
    parse_terminal_counter_phrase_shape(tokens).map(|parsed| parsed.counter_type)
}

fn parse_terminal_counter_phrase_shape(tokens: &[OwnedLexToken]) -> Option<TerminalCounterPhrase> {
    const TERMINAL_COUNTER_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::amount(
            "count_and_type",
            WinnowCaptureKind::UntilAnyPhrase(COUNTER_WORD_PHRASES),
        ),
        WinnowSequence::any_phrase(COUNTER_WORD_PHRASES),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = TERMINAL_COUNTER_PATTERN.parse_full(clause)?;
    let count_and_type = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let count = parse_number(count_and_type.tokens())
        .map(|(count, _)| count)
        .unwrap_or(1);
    let counter_type = if count_and_type.tokens().is_empty() {
        None
    } else {
        Some(parse_counter_type_from_tokens(tokens)?)
    };
    Some(TerminalCounterPhrase {
        counter_type,
        count,
    })
}

fn parse_source_has_counter_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let atoms = [
        WinnowSequence::object("counter", WinnowCaptureKind::UntilPhrase(&["on"])),
        WinnowSequence::modifier("target", WinnowCaptureKind::Rest),
    ];
    let relation = parse_has_relation_clauses(tokens)?;
    if !is_source_state_subject_clause(relation.subject_clause) {
        return None;
    }
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let target_clause = matched.capture_clause("target", relation.tail_clause)?;
    if !is_exact_counter_on_source_tail_clause(target_clause) {
        return None;
    }
    let counter_clause =
        matched.capture_clause_by_role(WinnowCaptureRole::Object, relation.tail_clause)?;
    if counter_clause
        .token(0)
        .is_some_and(|token| token_word_is(token, NO_WORD))
    {
        let counter_type = parse_terminal_counter_phrase(counter_clause.tokens().get(1..)?)??;
        return Some(PredicateAst::SourceHasNoCounter(counter_type));
    }
    if predicate_quantity_prefix_tokens(counter_clause.tokens()).is_some() {
        return None;
    }
    if predicate_number_or_more_prefix_tokens(counter_clause.tokens()).is_some() {
        return None;
    }
    let counter_type = parse_terminal_counter_phrase(counter_clause.tokens())??;
    Some(PredicateAst::SourceHasCounterAtLeast {
        counter_type,
        count: 1,
        surface: crate::SourceCounterThresholdSurface::SourceHas,
    })
}

fn parse_source_doesnt_have_counter_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilAnyPhrase(NEGATED_HAVE_PHRASES),
        ),
        WinnowSequence::action(
            "action",
            WinnowCaptureKind::OneOfPhrase(NEGATED_HAVE_PHRASES),
        ),
        WinnowSequence::object("counter", WinnowCaptureKind::UntilPhrase(&["on"])),
        WinnowSequence::modifier("target", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_source_reference_clause(subject_clause) {
        return None;
    }
    let target_clause = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    if !is_exact_counter_on_source_tail_clause(target_clause) {
        return None;
    }
    let counter_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let counter_type = parse_terminal_counter_phrase(counter_clause.tokens())??;
    Some(PredicateAst::SourceHasNoCounter(counter_type))
}

fn parse_source_has_counted_counter_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let atoms = [
        WinnowSequence::object("counter", WinnowCaptureKind::UntilPhrase(&["on"])),
        WinnowSequence::modifier("target", WinnowCaptureKind::Rest),
    ];
    let relation = parse_has_relation_clauses(tokens)?;
    if !is_source_state_subject_clause(relation.subject_clause) {
        return None;
    }
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let target_clause = matched.capture_clause("target", relation.tail_clause)?;
    if !is_counter_on_source_pronoun_tail_clause(target_clause) {
        return None;
    }
    let counter_clause =
        matched.capture_clause_by_role(WinnowCaptureRole::Object, relation.tail_clause)?;
    let explicit_one_or_more = primitives::parse_prefix(
        counter_clause.tokens(),
        primitives::phrase(&["one", "or", "more"]),
    )
    .is_some();
    // An indefinite article expresses presence here ("has a counter"), not an
    // exact cardinality of one. Keep explicit quantities such as "exactly one"
    // exact while lowering "a"/"an" to an at-least-one comparison.
    let (comparison, used) =
        crate::grammar::primitives::probe_shape(parse_quantity_comparison_prefix(
            counter_clause.tokens(),
            false,
            true,
            "counter predicate quantity",
        ))?;
    let (operator, count) = comparison_to_value_comparison_operator(comparison)?;
    let counter_tail = counter_clause.tokens().get(used..)?;
    let counter_type = parse_terminal_counter_phrase(counter_tail)??;
    if surface::exact(relation.subject_clause, &["it"]) {
        return Some(PredicateAst::ValueComparison {
            left: Value::CountersOn(
                Box::new(crate::target::ChooseSpec::Tagged(
                    crate::tag::CompilerReferenceTag::It.key(),
                )),
                Some(counter_type),
            ),
            operator,
            right: Value::Fixed(count),
        });
    }
    let source_count = match operator {
        crate::effect::ValueComparisonOperator::GreaterThanOrEqual => Some(count),
        crate::effect::ValueComparisonOperator::Equal if count > 0 => Some(count),
        _ => None,
    };
    if let Some(count) = source_count {
        return Some(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count: crate::util::narrowed_u32(count)?,
            surface: if explicit_one_or_more && count == 1 {
                crate::SourceCounterThresholdSurface::SourceHasOneOrMore
            } else {
                crate::SourceCounterThresholdSurface::SourceHas
            },
        });
    }
    Some(PredicateAst::ValueComparison {
        left: Value::CountersOn(
            Box::new(crate::target::ChooseSpec::Source),
            Some(counter_type),
        ),
        operator,
        right: Value::Fixed(count),
    })
}

/// Parses comparisons between the object that caused a trigger and the source.
///
/// Oracle uses two closely related surfaces for evolve-style gates:
/// `that creature's power is greater than this creature's` and
/// `that creature has greater power or toughness than this creature`.  Both
/// lower through the existing dynamic object-filter comparisons, so the
/// predicate remains executable and can be promoted to an intervening-if gate.
fn parse_triggering_object_source_stat_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let words = LexedClause::new(tokens).word_refs();

    let triggering_stat_filter = |power: bool| {
        let mut filter = ObjectFilter::default();
        let source_value = if power {
            Value::SourcePower
        } else {
            Value::SourceToughness
        };
        let comparison = crate::filter::Comparison::GreaterThanExpr(Box::new(source_value));
        if power {
            filter.power = Some(comparison);
        } else {
            filter.toughness = Some(comparison);
        }
        PredicateAst::ItMatches(filter)
    };

    let single_stat = crate::word_primitives::parse_any_sequence_complete(
        &words,
        &[
            &[
                "that",
                "creatures",
                "power",
                "is",
                "greater",
                "than",
                "this",
                "creatures",
            ],
            &["its", "power", "is", "greater", "than", "this", "creatures"],
        ],
    );
    if single_stat {
        return Some(triggering_stat_filter(true));
    }

    let has_idx = crate::word_primitives::parse_sequence_start(&words, &["has"])?;
    let comparison_tail = &words[has_idx..];
    let explicit_source_reference = surface::exact_words(
        comparison_tail,
        &[
            "has",
            "greater",
            "power",
            "or",
            "toughness",
            "than",
            "this",
            "creature",
        ],
    ) || surface::exact_words(
        comparison_tail,
        &[
            "has",
            "greater",
            "power",
            "or",
            "toughness",
            "than",
            "this",
            "creatures",
        ],
    );
    // A lone proper-name token after `than` is the Oracle source's own name
    // (for example, `... than Hulkling`).  Reserved reference words remain in
    // the ordinary object predicate grammar rather than being reinterpreted.
    let named_source_reference = comparison_tail.len() == 7
        && surface::exact_words(
            &comparison_tail[..6],
            &["has", "greater", "power", "or", "toughness", "than"],
        )
        && !matches!(
            comparison_tail[6],
            "a" | "an" | "another" | "enchanted" | "equipped" | "target" | "that" | "the" | "this"
        );
    if !explicit_source_reference && !named_source_reference {
        return None;
    }
    let subject = &words[..has_idx];
    if !matches!(subject, ["it"] | ["that", "creature"]) {
        return None;
    }

    Some(PredicateAst::Or(
        Box::new(triggering_stat_filter(true)),
        Box::new(triggering_stat_filter(false)),
    ))
}

fn is_counter_on_source_pronoun_tail_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["on", "it"],
            &["on", "him"],
            &["on", "her"],
            &["on", "them"],
            &["on", "this"],
            &["on", "that"],
        ],
    )
}

#[rustfmt::skip]
fn parse_source_verbless_counted_counter_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    for source_len in 1..tokens.len() {
        let source_clause = LexedClause::new(&tokens[..source_len]);
        if !is_source_state_subject_clause(source_clause) {
            continue;
        }
        let mut rest = &tokens[source_len..];
        if rest
            .first()
            .is_some_and(|token| token_word_is_any(token, HAS_OR_HAVE_WORDS))
        {
            rest = &rest[1..];
        }
        let rest_clause = LexedClause::new(rest);
        let atoms = [
            WinnowSequence::object("counter", WinnowCaptureKind::UntilPhrase(&["on"])),
            WinnowSequence::modifier("target", WinnowCaptureKind::Rest),
        ];
        let matched = WinnowSequence::new(&atoms).parse_full(rest_clause)?;
        let counter_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, rest_clause)?;
        let (count, used) = if let Some((comparison, used)) =
            predicate_quantity_prefix_tokens(counter_clause.tokens())
        {
            (comparison_to_at_least_threshold(&comparison)?, used)
        } else {
            predicate_at_least_quantity_prefix_tokens(counter_clause.tokens())?
        };
        let target_clause =
            matched.capture_clause_by_role(WinnowCaptureRole::Modifier, rest_clause)?;
        if !is_counter_on_source_pronoun_tail_clause(target_clause) {
            continue;
        }
        let counter_tokens = counter_clause.tokens().get(used..)?;
        let counter_type = parse_terminal_counter_phrase(counter_tokens)??;
        return Some(PredicateAst::ValueComparison {
            left: Value::CountersOn(
                Box::new(crate::target::ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.key())),
                Some(counter_type),
            ),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }
    None
}

fn parse_there_are_no_counters_on_source_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::amount("quantity", WinnowCaptureKind::OneOf(&["no"])),
        WinnowSequence::object("counter", WinnowCaptureKind::UntilPhrase(&["on"])),
        WinnowSequence::modifier("target", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let existential = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_there_are_clause(existential) {
        return None;
    }
    let target = matched.capture_clause("target", clause)?;
    if !is_exact_counter_on_source_tail_clause(target) {
        return None;
    }
    let counter_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let counter_type = parse_terminal_counter_phrase(counter_clause.tokens())??;
    Some(PredicateAst::SourceHasNoCounter(counter_type))
}

fn parse_triggering_object_had_counter_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(&["had"])),
        WinnowSequence::action("action", WinnowCaptureKind::OneOf(&["had"])),
        WinnowSequence::object("counter", WinnowCaptureKind::UntilPhrase(&["on"])),
        WinnowSequence::modifier("target", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_triggering_object_counter_subject_clause(subject_clause) {
        return None;
    }
    let target_clause = matched.capture_clause("target", clause)?;
    if !is_exact_counter_on_triggering_object_tail_clause(target_clause) {
        return None;
    }
    let counter_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if counter_clause
        .token(0)
        .is_some_and(|token| token_word_is(token, NO_WORD))
    {
        let counter_type = parse_terminal_counter_phrase(counter_clause.tokens().get(1..)?)??;
        return Some(PredicateAst::TriggeringObjectHadNoCounter(counter_type));
    }
    if surface::exact_any(counter_clause, &[&["counter"], &["counters"]]) {
        return Some(PredicateAst::ValueComparison {
            left: Value::CountersOn(
                Box::new(crate::target::ChooseSpec::Tagged(
                    crate::tag::CompilerReferenceTag::Triggering.key(),
                )),
                None,
            ),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(1),
        });
    }
    let counter_type = parse_terminal_counter_phrase(counter_clause.tokens())??;
    Some(PredicateAst::TriggeringObjectHadCounterAtLeast {
        counter_type,
        count: 1,
    })
}

fn is_triggering_object_counter_subject_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["it"],
            &["this", "creature"],
            &["that", "creature"],
            &["this", "permanent"],
            &["that", "permanent"],
        ],
    )
}

fn is_exact_counter_on_triggering_object_tail_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["on", "it"],
            &["on", "them"],
            &["on", "this"],
            &["on", "that"],
            &["on", "itself"],
        ],
    )
}

fn parse_basic_land_types_among_lands_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let land_type_phrases: &[&[&str]] = &[
        &["basic", "land", "type", "among", "land"],
        &["basic", "land", "type", "among", "lands"],
        &["basic", "land", "types", "among", "land"],
        &["basic", "land", "types", "among", "lands"],
    ];
    let atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::amount(
            "count",
            WinnowCaptureKind::UntilAnyPhrase(land_type_phrases),
        ),
        WinnowSequence::object(
            "land_types",
            WinnowCaptureKind::UntilAnyPhrase(&[
                &["you", "control"],
                &["you", "controls"],
                &["that", "player", "control"],
                &["that", "player", "controls"],
                &["that", "players", "controls"],
            ]),
        ),
        WinnowSequence::modifier("controller", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let existential = matched
        .capture_clause_by_role(WinnowCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing existential in basic-land-types predicate".to_string(),
            )
        })?;
    if !is_there_are_clause(existential) {
        return Ok(None);
    }
    let count_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Amount, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing count in basic-land-types predicate".to_string())
        })?;
    let (comparison, used) =
        predicate_quantity_prefix_tokens(count_clause.tokens()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported basic-land-types count (predicate: '{}')",
                render_token_slice(tokens)
            ))
        })?;
    if used != count_clause.tokens().len() {
        return Ok(None);
    }
    let Some(count) = comparison_to_at_least_threshold(&comparison) else {
        return Ok(None);
    };
    let land_types = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in basic-land-types predicate".to_string())
        })?;
    if !surface::exact_any(land_types, land_type_phrases) {
        return Ok(None);
    }
    let controller = matched
        .capture_clause("controller", clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing controller in basic-land-types predicate".to_string(),
            )
        })?;
    let player = parse_basic_land_types_controller_clause(controller).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported basic-land-types predicate tail (predicate: '{}')",
            render_token_slice(tokens)
        ))
    })?;
    Ok(Some(
        PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count },
    ))
}

fn parse_basic_land_types_controller_clause(clause: LexedClause<'_>) -> Option<PlayerAst> {
    if surface::exact_any(clause, &[&["you", "control"], &["you", "controls"]]) {
        return Some(PlayerAst::You);
    }
    if surface::exact_any(
        clause,
        &[
            &["that", "player", "control"],
            &["that", "player", "controls"],
            &["that", "players", "controls"],
        ],
    ) {
        return Some(PlayerAst::That);
    }
    None
}

fn parse_there_are_source_counters_at_least_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::object("counter", WinnowCaptureKind::UntilPhrase(&["on"])),
        WinnowSequence::modifier("target", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let existential = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_there_are_clause(existential) {
        return None;
    }
    let target = matched.capture_clause("target", clause)?;
    let source_surface = counter_on_source_surface(target)?;
    let counter_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let (comparison, used) = predicate_quantity_prefix_tokens(counter_clause.tokens())?;
    let count = comparison_to_at_least_threshold(&comparison)?;
    let counter_tail = counter_clause.tokens().get(used..)?;
    let Some(counter_type) = parse_terminal_counter_phrase(counter_tail)? else {
        return Some(PredicateAst::SourceHasCountersAtLeast(count));
    };
    Some(PredicateAst::SourceHasCounterAtLeast {
        counter_type,
        count,
        surface: crate::SourceCounterThresholdSurface::ThereAreOn(source_surface),
    })
}

fn is_exact_counter_on_source_tail_clause(clause: LexedClause<'_>) -> bool {
    counter_on_source_surface(clause).is_some()
}

fn counter_on_source_surface(
    clause: LexedClause<'_>,
) -> Option<crate::target::SourceReferenceSurface> {
    const COUNTER_ON_SOURCE_TAIL_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::word("on"),
        WinnowSequence::subject("source", WinnowCaptureKind::Rest),
    ]);

    let matched = COUNTER_ON_SOURCE_TAIL_PATTERN.parse_full(clause)?;
    let source = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_source_state_subject_clause(source) {
        return None;
    }
    let words = source.word_refs();
    source_reference_surface_for_words(&words)
        .or_else(|| this_source_surface_for_words(&words))
        .or_else(|| {
            (!words.is_empty())
                .then(|| crate::target::SourceReferenceSurface::ThisPermanentType(words.join(" ")))
        })
}

fn parse_source_exiled_with_counter_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let exiled_with_phrase = &["is", "exiled", "with"];
    let atoms = [
        WinnowSequence::subject("source", WinnowCaptureKind::UntilPhrase(exiled_with_phrase)),
        WinnowSequence::phrase(exiled_with_phrase),
        WinnowSequence::object("counter", WinnowCaptureKind::UntilPhrase(&["on"])),
        WinnowSequence::modifier("target", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let source_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_source_state_subject_clause(source_clause) {
        return None;
    }

    let target_clause = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    if !is_exact_counter_on_source_tail_clause(target_clause) {
        return None;
    }

    let counter_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let counter = parse_terminal_counter_phrase_shape(counter_clause.tokens())?;
    let counter_type = counter.counter_type?;
    Some(PredicateAst::And(
        Box::new(PredicateAst::SourceIsInZone(Zone::Exile)),
        Box::new(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count: counter.count,
            surface: crate::SourceCounterThresholdSurface::SourceHas,
        }),
    ))
}

fn parse_source_is_your_ring_bearer_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_copula_relation_clauses(tokens)?;
    if !is_this_source_clause(relation.subject_clause) {
        return None;
    }
    if !is_your_ring_bearer_clause(relation.tail_clause) {
        return None;
    }
    Some(PredicateAst::SourceIsRingBearer {
        player: PlayerAst::You,
    })
}

fn is_this_source_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["this"], &["this", "creature"]])
}

fn is_your_ring_bearer_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["your", "ring", "bearer"])
}

fn parse_ring_has_tempted_you_this_game_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let article_atoms = [WinnowSequence::word("the")];
    let atoms = [
        WinnowSequence::optional(&article_atoms),
        WinnowSequence::subject("ring", WinnowCaptureKind::OneOf(&["ring"])),
        WinnowSequence::action("tempted", WinnowCaptureKind::WordCount(3)),
        WinnowSequence::amount("count", WinnowCaptureKind::UntilPhrase(&["or", "more"])),
        WinnowSequence::phrase(&["or", "more"]),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let tempted = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !surface::exact(tempted, &["has", "tempted", "you"]) {
        return None;
    }
    let window = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    if !surface::exact_any(
        window,
        &[&["time", "this", "game"], &["times", "this", "game"]],
    ) {
        return None;
    }
    let count_clause = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let (count, used) = parse_number(count_clause.tokens())?;
    if used != count_clause.tokens().len() {
        return None;
    }
    Some(PredicateAst::PlayerRingTemptedThisGameOrMore {
        player: PlayerAst::You,
        count,
    })
}

fn parse_ring_bearer_temptation_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    if let Some(predicate) = parse_source_is_your_ring_bearer_predicate(tokens) {
        return Some(predicate);
    }
    if let Some(predicate) = parse_ring_has_tempted_you_this_game_predicate(tokens) {
        return Some(predicate);
    }

    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::condition("left", WinnowCaptureKind::UntilPhrase(&["and"])),
        WinnowSequence::word("and"),
        WinnowSequence::condition("right", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let left_clause = matched.capture_clause("left", clause)?;
    let right_clause = matched.capture_clause("right", clause)?;
    if left_clause.tokens().is_empty() || right_clause.tokens().is_empty() {
        return None;
    }
    let left = parse_source_is_your_ring_bearer_predicate(left_clause.tokens())?;
    let right = parse_ring_has_tempted_you_this_game_predicate(right_clause.tokens())?;
    Some(PredicateAst::And(Box::new(left), Box::new(right)))
}

fn parse_stack_object_targets_only_source_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "spell",
            WinnowCaptureKind::UntilPhrase(&["targets", "only"]),
        ),
        WinnowSequence::action("targets_only", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::object("target", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let spell = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_stack_object_reference_clause(spell) {
        return None;
    }
    let action = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !surface::exact(action, &["targets", "only"]) {
        return None;
    }

    let target = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let mut target_filter = source_target_filter_from_clause(target)?;
    target_filter.source = true;

    Some(PredicateAst::ItMatches(
        ObjectFilter::spell()
            .targeting_only_object(target_filter)
            .target_count_exact(1),
    ))
}

fn parse_stack_object_targets_object_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("spell", WinnowCaptureKind::UntilPhrase(&["targets"])),
        WinnowSequence::action("targets", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("target", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let spell = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_stack_object_reference_clause(spell) {
        return None;
    }
    let target = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if target
        .tokens()
        .first()
        .is_some_and(|token| token_word_is(token, "only"))
    {
        return None;
    }
    let target = crate::grammar::primitives::probe_shape(
        crate::grammar::shared_util::target_semantics::parse_target_phrase_inner(target.tokens()),
    )?;
    let spell_filter = match target {
        TargetAst::Object(target_filter, None, _) => {
            ObjectFilter::spell().targeting_object(target_filter)
        }
        TargetAst::Player(player_filter, None) => {
            ObjectFilter::spell().targeting_player(player_filter)
        }
        TargetAst::ObjectOrPlayer(object_filter, player_filter, None) => {
            let mut filter =
                ObjectFilter::spell().targeting(Some(player_filter), Some(object_filter));
            filter.targets_any_of = true;
            filter
        }
        _ => return None,
    };
    Some(PredicateAst::ItMatches(spell_filter))
}

fn is_stack_object_reference_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(clause, &[&["that", "spell"], &["spell"], &["it"]])
}

fn source_target_filter_from_clause(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    if surface::exact(clause, &["this", "creature"]) {
        return Some(ObjectFilter::creature());
    }
    if surface::exact(clause, &["this", "artifact"]) {
        return Some(ObjectFilter::artifact());
    }
    if surface::exact(clause, &["this", "enchantment"]) {
        return Some(ObjectFilter::enchantment());
    }
    if surface::exact(clause, &["this", "land"]) {
        return Some(ObjectFilter::land());
    }
    if surface::exact(clause, &["this", "permanent"]) {
        return Some(ObjectFilter::default().in_zone(Zone::Battlefield));
    }
    if surface::exact_any(clause, &[&["this", "source"], &["it"]]) {
        return Some(ObjectFilter::source().in_zone(Zone::Battlefield));
    }
    None
}

fn mana_cost_label_from_words(words: &[&str]) -> Option<String> {
    if words.is_empty() {
        return None;
    }

    let mut label = String::new();
    for word in words {
        if word.chars().all(|ch| ch.is_ascii_digit()) {
            label.push('{');
            label.push_str(word);
            label.push('}');
            continue;
        }
        if parse_mana_symbol(word).is_ok() {
            label.push('{');
            label.push_str(&word.to_ascii_uppercase());
            label.push('}');
            continue;
        }
        return None;
    }

    Some(label)
}

fn strip_source_possessive_label_prefix<'a>(words: &'a [&'a str]) -> &'a [&'a str] {
    if words.len() >= 3
        && surface::prefix_words(words, &["this"])
        && is_this_spell_possessive_word(words[1])
        && surface::exact_words(&words[2..3], &["s"])
    {
        return &words[3..];
    }
    if words.len() >= 2
        && surface::prefix_words(words, &["this"])
        && is_this_spell_possessive_word(words[1])
    {
        return &words[2..];
    }
    words
}

fn named_paid_cost_label_from_word(word: &str) -> Option<String> {
    let mut chars = word.chars();
    let first = chars.next()?;
    Some(format!(
        "{}{}",
        first.to_ascii_uppercase(),
        chars.as_str().to_ascii_lowercase()
    ))
}

fn is_this_spell_possessive_word(word: &str) -> bool {
    matches!(
        word,
        "spell's"
            | "spell"
            | "spells"
            | "card's"
            | "card"
            | "cards"
            | "creature's"
            | "creature"
            | "creatures"
            | "permanent's"
            | "permanent"
            | "permanents"
    )
}

fn is_paid_cost_possessive_word(word: &str) -> bool {
    matches!(word, "its" | "his" | "her" | "their")
}

fn ordinal_number_word(word: &str) -> Option<u32> {
    ironsmith_core::parse_ordinal_word(word).or_else(|| parse_named_number(word))
}

fn predicate_quantity_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(crate::effect::Comparison, usize)> {
    crate::grammar::primitives::probe_shape(parse_quantity_comparison_prefix(
        tokens,
        false,
        false,
        "predicate quantity",
    ))
}

fn predicate_number_or_more_prefix_tokens(tokens: &[OwnedLexToken]) -> Option<(u32, usize)> {
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    let first_word = words.first()?;
    let count = parse_named_number(first_word)?;
    let tail_token_idx = words.token_index_after_words(1)?;
    let tail = tokens.get(tail_token_idx..)?;
    if !surface::exact(LexedClause::new(tail), OR_MORE_PREFIX) {
        return None;
    }
    let consumed = LexedClause::new(tail)
        .token_index_after_words(2)
        .map(|used| tail_token_idx + used)?;
    Some((count, consumed))
}

fn predicate_at_least_quantity_prefix_tokens(tokens: &[OwnedLexToken]) -> Option<(u32, usize)> {
    if let Some((comparison, used)) = predicate_quantity_prefix_tokens(tokens) {
        let count = comparison_to_strict_at_least_threshold(&comparison)?;
        return Some((count, used));
    }

    predicate_number_or_more_prefix_tokens(tokens)
}

fn control_predicate_quantity_tokens(
    tokens: &[OwnedLexToken],
    words: &TokenWordView<'_>,
    prefix_len: usize,
) -> (Option<u32>, Option<u32>, usize) {
    let mut filter_start = prefix_len;
    let mut min_count = None;
    let mut exact_count = None;

    let comparison = words
        .token_span_for_words(prefix_len, words.len())
        .and_then(|range| predicate_quantity_prefix_tokens(&tokens[range]));
    if let Some((comparison, used)) = comparison {
        if let crate::effect::Comparison::Equal(count) = comparison
            && count >= 0
        {
            exact_count = Some(count as u32);
            filter_start = prefix_len + used;
        } else if let Some(threshold) = comparison_to_at_least_threshold(&comparison) {
            min_count = Some(threshold);
            filter_start = prefix_len + used;
        }
    }

    (min_count, exact_count, filter_start)
}

fn parse_player_controls_no_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    parse_player_controls_zero_quantity_predicate(tokens)
        .or_else(|| parse_player_does_not_control_predicate(tokens))
        .transpose()
}

fn parse_player_controls_zero_quantity_predicate(
    tokens: &[OwnedLexToken],
) -> Option<Result<PredicateAst, CardTextError>> {
    let atoms = [
        WinnowSequence::amount("amount", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("object", WinnowCaptureKind::Rest),
    ];
    let relation = parse_control_relation_clauses(tokens, false)?;
    let tail_clause = relation.tail_clause;
    let matched = WinnowSequence::new(&atoms).parse_full(tail_clause)?;
    let (player, controller) = zero_control_subject_clause(relation.subject_clause)?;
    let amount_clause = matched.capture_clause_by_role(WinnowCaptureRole::Amount, tail_clause)?;
    let tagged_neither = zero_control_amount_clause(amount_clause, player)?;
    let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, tail_clause)?;
    if object_clause.tokens().is_empty() {
        return None;
    }
    let result = parse_object_filter(object_clause.tokens(), false).map(|mut filter| {
        filter.controller = Some(controller);
        if tagged_neither {
            filter = filter.match_tagged(
                crate::tag::CompilerReferenceTag::It.key(),
                TaggedOpbjectRelation::IsTaggedObject,
            );
        }
        PredicateAst::PlayerControlsNo { player, filter }
    });
    Some(result)
}

fn zero_control_subject_clause(clause: LexedClause<'_>) -> Option<(PlayerAst, PlayerFilter)> {
    if surface::exact(clause, &["you"]) {
        return Some((PlayerAst::You, PlayerFilter::You));
    }
    if surface::exact_any(clause, &[&["player"], &["a", "player"], &["any", "player"]]) {
        return Some((PlayerAst::Any, PlayerFilter::Any));
    }
    None
}

fn zero_control_amount_clause(clause: LexedClause<'_>, player: PlayerAst) -> Option<bool> {
    if surface::exact(clause, &["no"]) {
        return Some(false);
    }
    (player == PlayerAst::You && surface::exact(clause, &["neither"])).then_some(true)
}

fn parse_player_does_not_control_predicate(
    tokens: &[OwnedLexToken],
) -> Option<Result<PredicateAst, CardTextError>> {
    let relation = parse_negated_control_relation_clauses(tokens)?;
    if !is_you_clause(relation.subject_clause) {
        return None;
    }
    if !is_do_not_clause(relation.negation_clause) {
        return None;
    }
    let object_clause = relation.tail_clause;
    if object_clause.tokens().is_empty() {
        return None;
    }
    let other = object_clause
        .tokens()
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let result = parse_object_filter(object_clause.tokens(), other).map(|mut filter| {
        filter.controller = Some(PlayerFilter::You);
        PredicateAst::PlayerControlsNo {
            player: PlayerAst::You,
            filter,
        }
    });
    Some(result)
}

fn is_you_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["you"])
}

fn is_do_not_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["dont"], &["don't"], &["do", "not"]])
}

fn parse_you_control_or_graveyard_predicate(
    tokens: &[OwnedLexToken],
) -> Option<Result<PredicateAst, CardTextError>> {
    let atoms = [
        WinnowSequence::object("control_object", WinnowCaptureKind::UntilPhrase(&["or"])),
        WinnowSequence::word("or"),
        WinnowSequence::modifier("graveyard_object", WinnowCaptureKind::Rest),
    ];
    let relation = parse_control_relation_clauses(tokens, false)?;
    let tail_clause = relation.tail_clause;
    let matched = WinnowSequence::new(&atoms).parse_full(tail_clause)?;
    if !is_you_clause(relation.subject_clause) {
        return None;
    }

    let control_object = matched.capture_clause("control_object", tail_clause)?;
    if control_object.tokens().is_empty() {
        return None;
    }

    let graveyard_object = matched.capture_clause("graveyard_object", tail_clause)?;
    let graveyard_tokens = graveyard_object_tokens_after_existential(graveyard_object)?;

    let result =
        parse_object_filter(control_object.tokens(), false).and_then(|mut control_filter| {
            parse_object_filter(graveyard_tokens, false).map(|mut graveyard_filter| {
                control_filter.controller = Some(PlayerFilter::You);
                if graveyard_filter.zone.is_none() {
                    graveyard_filter.zone = Some(Zone::Graveyard);
                }
                if graveyard_filter.owner.is_none() {
                    graveyard_filter.owner = Some(PlayerFilter::You);
                }
                PredicateAst::PlayerControlsOrHasCardInGraveyard {
                    player: PlayerAst::You,
                    control_filter,
                    graveyard_filter,
                }
            })
        });
    Some(result)
}

/// Preserve the two independently authored facts in predicates such as
///
/// `you control a Squirrel or returned a Squirrel card to your hand this way`.
///
/// The ordinary `or` recovery inherits a missing subject from the left side.
/// In this shape it must inherit only `you`, not the left-side verb `control`:
/// the right side observes the tagged result of the prior return-to-hand
/// effect rather than asking whether a matching card is currently controlled.
fn parse_you_control_or_returned_to_hand_this_way_predicate(
    tokens: &[OwnedLexToken],
) -> Option<Result<PredicateAst, CardTextError>> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("control", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("controlled", WinnowCaptureKind::UntilPhrase(&["or"])),
        WinnowSequence::word("or"),
        WinnowSequence::action("returned", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object(
            "returned_object",
            WinnowCaptureKind::UntilPhrase(&["to", "your", "hand"]),
        ),
        WinnowSequence::phrase(&["to", "your", "hand"]),
        WinnowSequence::phrase(&["this", "way"]),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject = matched.capture_clause("subject", clause)?;
    let control = matched.capture_clause("control", clause)?;
    let returned = matched.capture_clause("returned", clause)?;
    if !is_you_clause(subject)
        || !surface::exact_any(control, &[&["control"], &["controls"]])
        || !surface::exact(returned, &["returned"])
    {
        return None;
    }

    let controlled = matched.capture_clause("controlled", clause)?;
    let returned_object = matched.capture_clause("returned_object", clause)?;
    if controlled.tokens().is_empty() || returned_object.tokens().is_empty() {
        return None;
    }

    Some(
        parse_object_filter(controlled.tokens(), false).and_then(|mut control_filter| {
            parse_object_filter(returned_object.tokens(), false).map(|mut returned_filter| {
                control_filter.controller = Some(PlayerFilter::You);
                returned_filter.zone = Some(Zone::Hand);
                returned_filter.set_prior_effect_action_surface(Some(
                    ironsmith_core::PriorEffectAction::Returned,
                ));
                PredicateAst::Or(
                    Box::new(PredicateAst::PlayerControls {
                        player: PlayerAst::You,
                        filter: control_filter,
                    }),
                    Box::new(PredicateAst::PlayerTaggedObjectMatches {
                        player: PlayerAst::You,
                        tag: crate::tag::CompilerReferenceTag::It.key(),
                        filter: returned_filter,
                        mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
                    }),
                )
            })
        }),
    )
}

fn graveyard_object_tokens_after_existential<'a>(
    clause: LexedClause<'a>,
) -> Option<&'a [OwnedLexToken]> {
    let object_clause = parse_existential_object_clause(clause.tokens()).unwrap_or(clause);
    let object_clause = object_clause.trimmed();
    (!object_clause.tokens().is_empty() && surface::contains(object_clause, &["your", "graveyard"]))
        .then_some(object_clause.tokens())
}

fn parse_you_control_conjoined_predicate(
    tokens: &[OwnedLexToken],
) -> Option<Result<PredicateAst, CardTextError>> {
    let atoms = [
        WinnowSequence::object("left_object", WinnowCaptureKind::UntilPhrase(&["and"])),
        WinnowSequence::word("and"),
        WinnowSequence::object("right_object", WinnowCaptureKind::Rest),
    ];
    let relation = parse_control_relation_clauses(tokens, false)?;
    let tail_clause = relation.tail_clause;
    let matched = WinnowSequence::new(&atoms).parse_full(tail_clause)?;
    if !is_you_clause(relation.subject_clause) {
        return None;
    }

    let left_object = matched.capture_clause("left_object", tail_clause)?;
    let right_object = matched.capture_clause("right_object", tail_clause)?;
    if left_object.tokens().is_empty() || right_object.tokens().is_empty() {
        return None;
    }

    // A shared plural head scopes `named` over both sides: "you control
    // creatures named Mine Worker and Power Plant Worker" requires two
    // independently named creatures. Parsing the right side as a standalone
    // object phrase instead mistakes words inside the second card name for
    // characteristics (for example, `Plant` as a subtype).
    let tail_words = tail_clause.word_refs();
    let has_shared_named_head = tail_words.get(1) == Some(&"named")
        && tail_words.first().is_some_and(|word| {
            matches!(
                *word,
                "artifacts"
                    | "cards"
                    | "creatures"
                    | "enchantments"
                    | "lands"
                    | "permanents"
                    | "spells"
                    | "tokens"
            )
        });
    if has_shared_named_head {
        let shared_named_result =
            parse_object_filter(left_object.tokens(), false).and_then(|mut left_filter| {
                let right_name = render_token_slice(right_object.tokens())
                    .trim()
                    .to_ascii_lowercase();
                if left_filter.name.is_none() || right_name.is_empty() {
                    return Err(CardTextError::ParseError(
                        "missing card name in conjoined named-control condition".to_string(),
                    ));
                }
                let mut right_filter = left_filter.clone();
                right_filter.name = Some(right_name);
                left_filter.controller = Some(PlayerFilter::You);
                right_filter.controller = Some(PlayerFilter::You);
                Ok(PredicateAst::And(
                    Box::new(PredicateAst::PlayerControls {
                        player: PlayerAst::You,
                        filter: left_filter,
                    }),
                    Box::new(PredicateAst::PlayerControls {
                        player: PlayerAst::You,
                        filter: right_filter,
                    }),
                ))
            });
        return Some(shared_named_result);
    }

    let result = parse_object_filter(left_object.tokens(), false).and_then(|mut left_filter| {
        parse_object_filter(right_object.tokens(), false).map(|mut right_filter| {
            left_filter.controller = Some(PlayerFilter::You);
            right_filter.controller = Some(PlayerFilter::You);
            PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            )
        })
    });
    Some(result)
}

fn comparative_power_toughness_tail(
    clause: LexedClause<'_>,
    filter_start: usize,
    filter_end: usize,
) -> Option<(crate::filter::PowerToughnessRelation, usize)> {
    let words = clause.words().word_refs();
    let find_phrase = |phrases: &[&[&str]]| {
        phrases.iter().find_map(|phrase| {
            let end = filter_end.min(words.len());
            (filter_start..=end.saturating_sub(phrase.len()))
                .find(|start| words.get(*start..(*start + phrase.len())) == Some(*phrase))
        })
    };
    find_phrase(TOUGHNESS_GREATER_THAN_POWER_TAIL_PHRASES)
        .map(|start| {
            (
                crate::filter::PowerToughnessRelation::ToughnessGreaterThanPower,
                start,
            )
        })
        .or_else(|| {
            find_phrase(POWER_GREATER_THAN_TOUGHNESS_TAIL_PHRASES).map(|start| {
                (
                    crate::filter::PowerToughnessRelation::PowerGreaterThanToughness,
                    start,
                )
            })
        })
}

fn parse_player_controls_predicate(
    tokens: &[OwnedLexToken],
    player: PlayerAst,
    controller: Option<PlayerFilter>,
    prefix_len: usize,
    allow_outlaw_shorthand: bool,
    allow_different_powers: bool,
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let has_power_toughness_relation = TOUGHNESS_GREATER_THAN_POWER_TAIL_PHRASES
        .iter()
        .chain(POWER_GREATER_THAN_TOUGHNESS_TAIL_PHRASES)
        .any(|phrase| surface::contains(clause, phrase));
    // Preserve an authored exact cardinality through the local quantity
    // capture below.  The shared possession parser is intentionally tolerant
    // and can normalize singular `exactly one` to an existential control
    // condition, which is not equivalent for intervening-if triggers.
    let has_authored_exact_count = tokens.iter().any(|token| token.is_word("exactly"));
    if !has_power_toughness_relation
        && !has_authored_exact_count
        && let Some(control_condition) = crate::grammar::conditions::parse_control_condition(
            tokens,
            crate::grammar::conditions::ControlConditionOptions {
                allow_that_player: player == PlayerAst::That,
                allow_opponent_players: false,
                allow_defending_player: false,
                bind_filter_controller_to_subject: controller.is_some(),
                allow_different_powers_tail: allow_different_powers,
                default_filter_zone: None,
            },
        )
    {
        return Ok(Some(predicate_from_control_condition(control_condition)));
    }

    let words_view = clause.words();
    let words = words_view.word_refs();
    let (min_count, exact_count, filter_start) =
        control_predicate_quantity_tokens(tokens, &words_view, prefix_len);
    let mut filter_end = words.len();
    if filter_start >= filter_end {
        return Ok(None);
    }

    let mut requires_different_powers = false;
    let mut power_toughness_relation = None;
    if let Some((relation, tail_len)) =
        comparative_power_toughness_tail(clause, filter_start, filter_end)
    {
        power_toughness_relation = Some(relation);
        filter_end = tail_len;
    }
    if allow_different_powers
        && filter_end >= filter_start + 3
        && clause
            .between_word_range(filter_end.saturating_sub(3), filter_end)
            .is_some_and(|tail| surface::exact_any(tail, WITH_DIFFERENT_POWERS_TAIL_PHRASES))
    {
        requires_different_powers = true;
        filter_end = filter_end.saturating_sub(3);
    }

    let Some(filter_range) = words_view.token_span_for_words(filter_start, filter_end) else {
        return Ok(None);
    };
    let filter_tokens = &tokens[filter_range];
    let filter_clause = LexedClause::new(filter_tokens);
    let other = filter_tokens
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let parsed_filter = parse_object_filter(filter_tokens, other).or_else(|_| {
        if allow_outlaw_shorthand {
            parse_outlaw_shorthand_filter(filter_clause)
                .ok_or_else(|| CardTextError::ParseError("unsupported control filter".to_string()))
        } else {
            Err(CardTextError::ParseError(
                "unsupported control filter".to_string(),
            ))
        }
    });
    let Ok(mut filter) = parsed_filter else {
        return Ok(None);
    };
    if let Some(controller) = controller {
        filter.controller = Some(controller);
    }
    if let Some(relation) = power_toughness_relation {
        filter.power_toughness_relation = Some(relation);
    }

    if let Some(count) = exact_count {
        return Ok(Some(PredicateAst::PlayerControlsExactly {
            player,
            filter,
            count,
        }));
    }
    if let Some(count) = min_count
        && count > 1
    {
        if requires_different_powers {
            return Ok(Some(PredicateAst::PlayerHasAtLeastWithDifferentPowers {
                player,
                filter,
                count,
            }));
        }
        return Ok(Some(PredicateAst::PlayerHasAtLeast {
            player,
            filter,
            count,
        }));
    }
    Ok(Some(PredicateAst::PlayerControls { player, filter }))
}

fn predicate_from_control_condition(
    control_condition: crate::grammar::conditions::ControlConditionAst,
) -> PredicateAst {
    if let Some(predicate) = predicate_for_each_global_greatest_power(&control_condition) {
        return predicate;
    }
    if let Some(count) = control_condition.exact_count() {
        return PredicateAst::PlayerControlsExactly {
            player: control_condition.player,
            filter: control_condition.filter,
            count,
        };
    }
    let Some(count) = control_condition.at_least_count() else {
        return PredicateAst::PlayerControls {
            player: control_condition.player,
            filter: control_condition.filter,
        };
    };
    if count > 1 {
        if control_condition.requires_different_powers {
            return PredicateAst::PlayerHasAtLeastWithDifferentPowers {
                player: control_condition.player,
                filter: control_condition.filter,
                count,
            };
        }
        return PredicateAst::PlayerHasAtLeast {
            player: control_condition.player,
            filter: control_condition.filter,
            count,
        };
    }
    PredicateAst::PlayerControls {
        player: control_condition.player,
        filter: control_condition.filter,
    }
}

fn predicate_for_each_global_greatest_power(
    control_condition: &crate::grammar::conditions::ControlConditionAst,
) -> Option<PredicateAst> {
    if control_condition.player != PlayerAst::You
        || !control_condition
            .quantity_words
            .iter()
            .map(String::as_str)
            .eq(["each"])
        || !is_creature_on_battlefield_with_greatest_power(&control_condition.object_words)
    {
        return None;
    }

    let mut global_creatures = control_condition.filter.clone();
    global_creatures.controller = None;
    global_creatures.power = None;
    global_creatures.zone = Some(Zone::Battlefield);

    let mut greatest_creatures = global_creatures.clone();
    greatest_creatures.power = Some(crate::filter::Comparison::EqualExpr(Box::new(
        Value::GreatestPower(global_creatures),
    )));
    let mut greatest_creatures_you_control = greatest_creatures.clone();
    greatest_creatures_you_control.controller = Some(PlayerFilter::You);

    Some(PredicateAst::ValueComparison {
        left: Value::Count(greatest_creatures_you_control),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: Value::Count(greatest_creatures),
    })
}

fn is_creature_on_battlefield_with_greatest_power(words: &[String]) -> bool {
    let mut words = words.iter().map(String::as_str);
    if !matches!(words.next(), Some("creature" | "creatures")) || words.next() != Some("on") {
        return false;
    }
    let mut word = words.next();
    if word == Some("the") {
        word = words.next();
    }
    if word != Some("battlefield") || words.next() != Some("with") {
        return false;
    }
    let mut word = words.next();
    if word == Some("the") {
        word = words.next();
    }
    word == Some("greatest") && words.next() == Some("power") && words.next().is_none()
}

fn parse_this_ability_resolution_count_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    if let Some(counts) = ability_resolution_ordinal_disjunction_counts(clause) {
        let mut predicates = counts
            .into_iter()
            .map(PredicateAst::ThisAbilityResolvedThisTurnExactly);
        let first = predicates.next()?;
        return Some(predicates.fold(first, |left, right| {
            PredicateAst::Or(Box::new(left), Box::new(right))
        }));
    }

    let count = ability_resolution_ordinal_count(clause)?;

    Some(PredicateAst::ThisAbilityResolvedThisTurnExactly(count))
}

fn ability_resolution_ordinal_disjunction_counts(clause: LexedClause<'_>) -> Option<Vec<u32>> {
    const PREFIX: &[&str] = &["this", "is"];
    const SUFFIXES: &[&[&str]] = &[
        &["time", "this", "ability", "has", "resolved", "this", "turn"],
        &["time", "this", "ability", "resolved", "this", "turn"],
    ];

    let words = clause.word_refs();
    if !crate::word_primitives::parse_sequence_prefix(&words, PREFIX) {
        return None;
    }
    let mut start = PREFIX.len();
    if words.get(start) == Some(&"the") {
        start += 1;
    }

    for suffix in SUFFIXES {
        if !crate::word_primitives::parse_sequence_suffix(&words, suffix)
            || words.len() <= start + suffix.len()
        {
            continue;
        }
        let ordinal_words = &words[start..words.len() - suffix.len()];
        let mut counts = Vec::new();
        let mut expect_count = true;
        for word in ordinal_words {
            if expect_count && *word == "the" {
                continue;
            }
            if expect_count {
                counts.push(ordinal_number_word(word)?);
                expect_count = false;
            } else if matches!(*word, "or" | "and" | "and/or") {
                expect_count = true;
            } else {
                return None;
            }
        }
        if counts.len() > 1 && !expect_count {
            return Some(counts);
        }
    }
    None
}

/// Parse intervening-if gates which identify the triggering spell by its
/// ordinal among matching spells cast this turn. Each category becomes an
/// independent event-boundary comparison, so shared Oracle wording such as
/// "the first instant spell, the first sorcery spell, or the first Otter
/// spell ..." retains its inclusive disjunction semantics.
fn split_triggering_spell_ordinal_categories(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    let mut categories = Vec::new();
    let mut category_start = 0usize;
    let mut idx = 0usize;

    while idx < tokens.len() {
        if tokens[idx].kind != TokenKind::Comma && !tokens[idx].is_word("or") {
            idx += 1;
            continue;
        }

        let mut next_category_start = idx + 1;
        while tokens
            .get(next_category_start)
            .is_some_and(|token| token.kind == TokenKind::Comma)
        {
            next_category_start += 1;
        }
        if tokens
            .get(next_category_start)
            .is_some_and(|token| token.is_word("or"))
        {
            next_category_start += 1;
        }

        let mut ordinal_idx = next_category_start;
        if tokens
            .get(ordinal_idx)
            .is_some_and(|token| token.is_word("the"))
        {
            ordinal_idx += 1;
        }
        let starts_repeated_ordinal = tokens
            .get(ordinal_idx)
            .and_then(|token| ordinal_number_word(token.parser_text()))
            .is_some_and(|ordinal| ordinal > 0);
        if !starts_repeated_ordinal {
            idx += 1;
            continue;
        }

        let category = trim_lexed_commas(&tokens[category_start..idx]);
        if !category.is_empty() {
            categories.push(category);
        }
        category_start = next_category_start;
        idx = ordinal_idx;
    }

    let category = trim_lexed_commas(&tokens[category_start..]);
    if !category.is_empty() {
        categories.push(category);
    }
    categories
}

pub fn parse_triggering_spell_ordinal_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    const OPTIONAL_THE: &[WinnowAtom<'static>] = &[WinnowSequence::word("the")];
    const CAST_THIS_TURN_SUFFIXES: &[&[&str]] = &[
        &["you", "cast", "this", "turn"],
        &["youve", "cast", "this", "turn"],
        &["you've", "cast", "this", "turn"],
        &["you", "have", "cast", "this", "turn"],
    ];

    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::any_phrase(&[&["it", "was"], &["it's"], &["its"], &["it", "s"]]),
        WinnowSequence::optional(OPTIONAL_THE),
        WinnowSequence::object(
            "ordinal_categories",
            WinnowCaptureKind::UntilAnyPhrase(CAST_THIS_TURN_SUFFIXES),
        ),
        WinnowSequence::any_phrase(CAST_THIS_TURN_SUFFIXES),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let categories = matched.capture_clause("ordinal_categories", clause)?;

    let mut predicates = Vec::new();
    for category in split_triggering_spell_ordinal_categories(categories.tokens()) {
        let category = LexedClause::new(category).trimmed();
        let category_tokens = strip_leading_article_tokens(category.tokens());
        let (ordinal_token, descriptor_tokens) = category_tokens.split_first()?;
        let ordinal = ordinal_number_word(ordinal_token.parser_text())?;
        if ordinal == 0 || descriptor_tokens.is_empty() {
            return None;
        }
        let left =
            crate::grammar::shared_util::value_semantics::parse_triggering_spell_history_count_value(
                descriptor_tokens,
            )?;
        predicates.push(PredicateAst::ValueComparison {
            left,
            operator: crate::effect::ValueComparisonOperator::Equal,
            right: Value::Fixed(ordinal.saturating_sub(1) as i32),
        });
    }

    let mut predicates = predicates.into_iter();
    let first = predicates.next()?;
    Some(predicates.fold(first, |left, right| {
        PredicateAst::Or(Box::new(left), Box::new(right))
    }))
}

fn ability_resolution_ordinal_count(clause: LexedClause<'_>) -> Option<u32> {
    const OPTIONAL_THE: &[WinnowAtom<'static>] = &[WinnowSequence::word("the")];

    ability_resolution_count_from_pattern(
        clause,
        WinnowSequence::new(&[
            WinnowSequence::phrase(&["this", "is"]),
            WinnowSequence::optional(OPTIONAL_THE),
            WinnowSequence::amount("count", WinnowCaptureKind::WordCount(1)),
            WinnowSequence::phrase(&["time", "this", "ability", "has", "resolved", "this", "turn"]),
        ]),
    )
    .or_else(|| {
        ability_resolution_count_from_pattern(
            clause,
            WinnowSequence::new(&[
                WinnowSequence::phrase(&["this", "is"]),
                WinnowSequence::optional(OPTIONAL_THE),
                WinnowSequence::amount("count", WinnowCaptureKind::WordCount(1)),
                WinnowSequence::phrase(&["time", "this", "ability", "resolved", "this", "turn"]),
            ]),
        )
    })
    .or_else(|| {
        ability_resolution_count_from_pattern(
            clause,
            WinnowSequence::new(&[
                WinnowSequence::phrase(&["this", "ability", "has", "resolved", "for"]),
                WinnowSequence::amount("count", WinnowCaptureKind::WordCount(1)),
                WinnowSequence::phrase(&["time", "this", "turn"]),
            ]),
        )
    })
    .or_else(|| {
        ability_resolution_count_from_pattern(
            clause,
            WinnowSequence::new(&[
                WinnowSequence::phrase(&["this", "ability", "resolved", "for"]),
                WinnowSequence::amount("count", WinnowCaptureKind::WordCount(1)),
                WinnowSequence::phrase(&["time", "this", "turn"]),
            ]),
        )
    })
    .or_else(|| {
        ability_resolution_count_from_pattern(
            clause,
            WinnowSequence::new(&[
                WinnowSequence::any_phrase(&[&["it's"], &["its"], &["it", "s"], &["it"]]),
                WinnowSequence::amount("count", WinnowCaptureKind::WordCount(1)),
                WinnowSequence::phrase(&["time"]),
            ]),
        )
    })
}

fn ability_resolution_count_from_pattern(
    clause: LexedClause<'_>,
    pattern: WinnowSequence<'_>,
) -> Option<u32> {
    let matched = pattern.parse_full(clause)?;
    let count = matched.capture_clause("count", clause)?;
    let count_token = count.token(0)?;
    ordinal_number_word(count_token.parser_text())
}

fn parse_color_only_object_filter_word_refs(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    parse_color_only_object_filter_tokens(clause.tokens())
}

fn parse_color_only_object_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut filter = ObjectFilter::default();
    let mut saw_color = false;
    for token in tokens {
        if token_word_is(token, AND_WORD) || token_word_is(token, OR_WORD) {
            continue;
        }
        if let Some(color) = parse_color(token.parser_text()) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
            saw_color = true;
            continue;
        }
        if let Some(color) = parse_non_color(token.parser_text()) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            saw_color = true;
            continue;
        }
        return None;
    }
    saw_color.then_some(filter)
}

fn parse_color_only_object_filter_clause(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    parse_color_only_object_filter_tokens(clause.tokens())
}

fn strip_clause_suffix<'a>(
    clause: LexedClause<'a>,
    suffix: &'static [&'static str],
) -> Option<LexedClause<'a>> {
    let atoms = [
        WinnowSequence::object("base", WinnowCaptureKind::UntilLastPhrase(suffix)),
        WinnowSequence::phrase(suffix),
    ];
    WinnowSequence::new(&atoms)
        .parse_full(clause)
        .and_then(|matched| matched.capture_clause_by_role(WinnowCaptureRole::Object, clause))
        .map(LexedClause::trimmed)
}

fn parse_this_way_object_filter_clause(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    let (base_clause, needs_chosen_name) =
        if let Some(base_clause) = strip_clause_suffix(clause, &["with", "chosen", "name"]) {
            (base_clause, true)
        } else if let Some(base_clause) =
            strip_clause_suffix(clause, &["with", "the", "chosen", "name"])
        {
            (base_clause, true)
        } else {
            (clause, false)
        };
    let has_card_noun = base_clause
        .tokens()
        .last()
        .is_some_and(|token| token_word_is_any(token, CARD_OR_CARDS_WORDS));
    let candidates = [
        (base_clause, has_card_noun),
        (
            strip_clause_suffix(base_clause, &["card"]).unwrap_or(base_clause),
            true,
        ),
        (
            strip_clause_suffix(base_clause, &["cards"]).unwrap_or(base_clause),
            true,
        ),
    ];
    for (candidate, stripped_card_noun) in candidates {
        if candidate.tokens().is_empty() {
            let mut filter = ObjectFilter::default();
            filter.set_explicit_card_noun(stripped_card_noun);
            if needs_chosen_name {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: crate::tag::CompilerReferenceTag::ChosenName.key(),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                });
            }
            return Some(filter);
        }
        if let Ok(mut filter) = parse_object_filter(candidate.tokens(), false) {
            if stripped_card_noun {
                filter.zone = None;
                filter.set_explicit_card_noun(true);
            }
            if needs_chosen_name {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: crate::tag::CompilerReferenceTag::ChosenName.key(),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                });
            }
            return Some(filter);
        }
        if let Some(mut filter) = parse_color_only_object_filter_clause(candidate) {
            if stripped_card_noun {
                filter.zone = None;
                filter.set_explicit_card_noun(true);
            }
            if needs_chosen_name {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: crate::tag::CompilerReferenceTag::ChosenName.key(),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                });
            }
            return Some(filter);
        }
    }
    None
}

fn parse_passive_this_way_tagged_object_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[
        &["is", "countered"],
        &["are", "countered"],
        &["was", "countered"],
        &["were", "countered"],
        &["is", "destroyed"],
        &["are", "destroyed"],
        &["was", "destroyed"],
        &["were", "destroyed"],
        &["is", "discarded"],
        &["are", "discarded"],
        &["was", "discarded"],
        &["were", "discarded"],
        &["is", "exiled"],
        &["are", "exiled"],
        &["was", "exiled"],
        &["were", "exiled"],
        &["is", "milled"],
        &["are", "milled"],
        &["was", "milled"],
        &["were", "milled"],
        &["is", "returned"],
        &["are", "returned"],
        &["was", "returned"],
        &["were", "returned"],
        &["is", "revealed"],
        &["are", "revealed"],
        &["was", "revealed"],
        &["were", "revealed"],
        &["is", "sacrificed"],
        &["are", "sacrificed"],
        &["was", "sacrificed"],
        &["were", "sacrificed"],
    ];
    let atoms = [
        WinnowSequence::object(
            "object",
            WinnowCaptureKind::UntilLastAnyPhrase(action_phrases),
        ),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::phrase(&["this", "way"]),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let action_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Action, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing action in passive this-way predicate".to_string())
        })?;
    let action_words = action_clause.word_refs();
    let reference_tag = if action_words.get(1) == Some(&"sacrificed") {
        crate::tag::CompilerReferenceTag::ThisWaySacrificed
    } else {
        crate::tag::CompilerReferenceTag::It
    };
    let filter_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in passive this-way predicate".to_string())
        })?;
    if filter_clause.tokens().is_empty() {
        return Ok(None);
    }
    let Some(mut filter) = parse_this_way_object_filter_clause(filter_clause) else {
        return Ok(None);
    };
    filter.set_prior_effect_action_surface(
        crate::grammar::shared_util::value_helper_shapes::parse_prior_effect_action(&action_words)
            .map(|(action, _)| action),
    );
    Ok(Some(PredicateAst::TaggedMatches(
        reference_tag.key(),
        filter,
    )))
}

fn parse_active_this_way_discard_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["discard"], &["discards"], &["discarded"]];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilAnyPhrase(action_phrases)),
        WinnowSequence::action(
            "action",
            WinnowCaptureKind::OneOf(&["discard", "discards", "discarded"]),
        ),
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["this", "way"])),
        WinnowSequence::phrase(&["this", "way"]),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let subject_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing subject in active this-way predicate".to_string())
        })?;
    let Some(player) = active_discard_player_subject_clause(subject_clause) else {
        return Ok(None);
    };
    let filter_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in active this-way predicate".to_string())
        })?;
    if filter_clause.tokens().is_empty() {
        return Ok(None);
    }
    let Some(mut filter) = parse_this_way_object_filter_clause(filter_clause) else {
        return Ok(None);
    };
    filter.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Discarded));
    Ok(Some(PredicateAst::PlayerTaggedObjectMatches {
        player,
        tag: crate::tag::CompilerReferenceTag::It.key(),
        filter,
        mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
    }))
}

fn parse_negative_put_tagged_object_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let destination_phrases: &[&[&str]] = &[
        &["into", "your", "hand"],
        &["onto", "battlefield"],
        &["onto", "the", "battlefield"],
    ];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::modifier("negation", WinnowCaptureKind::UntilPhrase(&["put"])),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object(
            "object",
            WinnowCaptureKind::UntilAnyPhrase(destination_phrases),
        ),
        WinnowSequence::modifier("destination", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let negation_clause = matched.capture_clause("negation", clause)?;
    if !is_do_or_did_not_clause(negation_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !surface::exact(action_clause, &["put"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_tagged_card_reference_clause(object_clause) {
        return None;
    }
    let destination_clause = matched.capture_clause("destination", clause)?;
    let zone = tagged_put_destination_zone(destination_clause)?;
    Some(PredicateAst::Not(Box::new(
        PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.key(),
            filter: ObjectFilter::default().in_zone(zone),
            mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
        },
    )))
}

fn is_do_or_did_not_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["dont"],
            &["don't"],
            &["didnt"],
            &["didn't"],
            &["did", "not"],
        ],
    )
}

fn is_tagged_card_reference_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[&["the", "card"], &["that", "card"], &["card"], &["it"]],
    )
}

fn tagged_put_destination_zone(clause: LexedClause<'_>) -> Option<Zone> {
    if surface::exact(clause, &["into", "your", "hand"]) {
        return Some(Zone::Hand);
    }
    if surface::exact_any(
        clause,
        &[&["onto", "battlefield"], &["onto", "the", "battlefield"]],
    ) {
        return Some(Zone::Battlefield);
    }
    None
}

fn is_battlefield_this_way_destination_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["onto", "battlefield", "this", "way"],
            &["onto", "the", "battlefield", "this", "way"],
        ],
    )
}

fn parse_active_this_way_battlefield_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let destination_phrases: &[&[&str]] =
        &[&["onto", "battlefield"], &["onto", "the", "battlefield"]];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(&["put"])),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object(
            "object",
            WinnowCaptureKind::UntilAnyPhrase(destination_phrases),
        ),
        WinnowSequence::modifier("destination", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let subject_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing subject in this-way battlefield predicate".to_string(),
            )
        })?;
    if !is_you_clause(subject_clause) {
        return Ok(None);
    }
    let destination_clause = matched
        .capture_clause("destination", clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing destination in this-way battlefield predicate".to_string(),
            )
        })?;
    if !is_battlefield_this_way_destination_clause(destination_clause) {
        return Ok(None);
    }
    let filter_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing object in this-way battlefield predicate".to_string(),
            )
        })?;
    let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }
    Ok(Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::You,
        tag: crate::tag::CompilerReferenceTag::It.key(),
        filter,
        mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
    }))
}

fn parse_passive_this_way_battlefield_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["is", "put"])),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::modifier("destination", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let action_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Action, clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing action in passive this-way battlefield predicate".to_string(),
            )
        })?;
    if !surface::exact(action_clause, &["is", "put"]) {
        return Ok(None);
    }
    let destination_clause = matched
        .capture_clause("destination", clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing destination in passive this-way battlefield predicate".to_string(),
            )
        })?;
    if !is_battlefield_this_way_destination_clause(destination_clause) {
        return Ok(None);
    }
    let filter_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing object in passive this-way battlefield predicate".to_string(),
            )
        })?;
    let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }
    Ok(Some(PredicateAst::TaggedMatches(
        crate::tag::CompilerReferenceTag::It.key(),
        filter,
    )))
}

fn active_discard_player_subject_clause(clause: LexedClause<'_>) -> Option<PlayerAst> {
    if surface::exact(clause, &["you"]) {
        return Some(PlayerAst::You);
    }
    if surface::exact_any(clause, &[&["that", "player"], &["that", "players"]]) {
        return Some(PlayerAst::That);
    }
    if surface::exact(clause, &["target", "player"]) {
        return Some(PlayerAst::Target);
    }
    if surface::exact(clause, &["target", "opponent"]) {
        return Some(PlayerAst::TargetOpponent);
    }
    if surface::exact_any(clause, &[&["opponent"], &["opponents"]]) {
        return Some(PlayerAst::Opponent);
    }
    None
}

fn parse_repeated_if_or_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::object("left", WinnowCaptureKind::UntilPhrase(&["or", "if"])),
        WinnowSequence::phrase(&["or", "if"]),
        WinnowSequence::modifier("right", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };

    let left_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing left side in or-if predicate".to_string())
        })?;
    let right_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing right side in or-if predicate".to_string())
        })?;
    if left_clause.tokens().is_empty() || right_clause.tokens().is_empty() {
        return Ok(None);
    }

    let left = match parse_predicate(left_clause.tokens()) {
        Ok(predicate) => predicate,
        Err(_) => return Ok(None),
    };
    let right = parse_predicate(right_clause.tokens())?;
    Ok(Some(PredicateAst::Or(Box::new(left), Box::new(right))))
}

fn parse_repeated_and_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    for split_idx in (1..tokens.len().saturating_sub(1)).rev() {
        if !tokens[split_idx].is_word(AND_WORD) {
            continue;
        }

        let left_tokens = &tokens[..split_idx];
        let right_tokens = &tokens[split_idx + 1..];
        if left_tokens.is_empty() || right_tokens.is_empty() {
            continue;
        }
        if left_tokens.iter().any(|token| token.is_word(AND_WORD))
            || right_tokens.iter().any(|token| token.is_word(AND_WORD))
        {
            continue;
        }
        if !predicate_conjunction_side_looks_standalone(left_tokens)
            || !predicate_conjunction_side_looks_standalone(right_tokens)
        {
            continue;
        }

        let left = match parse_predicate(left_tokens) {
            Ok(predicate) => predicate,
            Err(_) => continue,
        };
        let right = match parse_predicate(right_tokens) {
            Ok(predicate) => predicate,
            Err(_) => continue,
        };
        return Ok(Some(PredicateAst::And(Box::new(left), Box::new(right))));
    }

    Ok(None)
}

fn predicate_conjunction_side_looks_standalone(tokens: &[OwnedLexToken]) -> bool {
    predicate_tokens_start_with_reference(tokens)
        && predicate_tokens_contain_predicate_operator(tokens)
}

fn predicate_tokens_contain_predicate_operator(tokens: &[OwnedLexToken]) -> bool {
    tokens.iter().any(|token| {
        token_word_is_any(
            token,
            &[
                "are", "arent", "aren't", "cast", "control", "controls", "did", "didnt", "didn't",
                "does", "doesnt", "doesn't", "has", "hasnt", "hasn't", "have", "havent", "haven't",
                "is", "isnt", "isn't",
            ],
        )
    })
}

fn predicate_reference_prefix_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    if tokens
        .first()
        .is_some_and(|token| token_word_is(token, IT_WORD))
    {
        return Some(&tokens[..1]);
    }
    if tokens.len() >= 2
        && token_word_is(&tokens[0], THAT_WORD)
        && token_word_is_any(&tokens[1], PREDICATE_REFERENCE_NOUN_WORDS)
    {
        return Some(&tokens[..2]);
    }
    None
}

fn predicate_tokens_start_with_reference(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .first()
        .is_some_and(|token| token_word_is_any(token, PREDICATE_REFERENCE_START_WORDS))
}

fn parse_single_card_type_card_descriptor_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let clause = LexedClause::new(tokens);
    if surface::exact_any(clause, &[&["permanent", "card"], &["permanent", "cards"]]) {
        return Some(ObjectFilter::permanent_card());
    }
    if tokens.len() == 2
        && token_word_is_any(&tokens[1], CARD_OR_CARDS_WORDS)
        && let Some(card_type) = parse_card_type(tokens[0].parser_text())
    {
        return Some(ObjectFilter {
            card_types: vec![card_type],
            ..Default::default()
        });
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DemonstrativeReferenceKind {
    It,
    ThatObject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DemonstrativeMatchTime {
    Current,
    LastKnown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DemonstrativeReferencePrefix {
    word_len: usize,
    kind: DemonstrativeReferenceKind,
    tagged_that_enchantment: bool,
    reference_is_creature: bool,
    antecedent_surface: Option<ironsmith_core::DemonstrativeAntecedentSurface>,
}

fn demonstrative_reference_prefix(clause: LexedClause<'_>) -> Option<DemonstrativeReferencePrefix> {
    if let Some(matched) = WinnowSequence::new(&[WinnowSequence::any_phrase(&[
        &["it"],
        &["its"],
        &["it", "s"],
    ])])
    .parse_prefix(clause)
    {
        return Some(DemonstrativeReferencePrefix {
            word_len: matched.word_range.end,
            kind: DemonstrativeReferenceKind::It,
            tagged_that_enchantment: false,
            reference_is_creature: false,
            antecedent_surface: None,
        });
    }

    let that_reference_atoms = [
        WinnowSequence::word("that"),
        WinnowSequence::object("reference", WinnowCaptureKind::WordCount(1)),
    ];
    let matched = WinnowSequence::new(&that_reference_atoms).parse_prefix(clause)?;
    let reference = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let reference_token = reference.token(0)?;
    if !token_word_is_any(reference_token, PREDICATE_REFERENCE_NOUN_WORDS) {
        return None;
    }
    Some(DemonstrativeReferencePrefix {
        word_len: matched.word_range.end,
        kind: DemonstrativeReferenceKind::ThatObject,
        tagged_that_enchantment: token_word_is(reference_token, ENCHANTMENT_WORD),
        reference_is_creature: token_word_is(reference_token, CREATURE_WORD),
        antecedent_surface: ironsmith_core::DemonstrativeAntecedentSurface::from_noun(
            reference_token.parser_text(),
        ),
    })
}

fn demonstrative_antecedent_surface(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::DemonstrativeAntecedentSurface> {
    demonstrative_reference_prefix(LexedClause::new(tokens))?.antecedent_surface
}

fn clause_word_range_matches_phrase(
    clause: LexedClause<'_>,
    word_start: usize,
    phrase: &[&str],
) -> bool {
    clause
        .between_word_range(word_start, word_start + phrase.len())
        .is_some_and(|words| surface::exact(words, phrase))
}

fn clause_word_range_matches_any_phrase(
    clause: LexedClause<'_>,
    word_start: usize,
    phrases: &[&[&str]],
) -> bool {
    phrases
        .iter()
        .any(|phrase| clause_word_range_matches_phrase(clause, word_start, phrase))
}

fn clause_word_at_is_any(clause: LexedClause<'_>, word_idx: usize, expected: &[&str]) -> bool {
    clause
        .between_word_range(word_idx, word_idx + 1)
        .is_some_and(|word| {
            word.token(0)
                .is_some_and(|token| token_word_is_any(token, expected))
        })
}

#[rustfmt::skip]
fn demonstrative_descriptor_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(
    Vec<OwnedLexToken>,
    bool,
    bool,
    bool,
    DemonstrativeMatchTime,
)> {
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    let reference = demonstrative_reference_prefix(clause)?;
    let reference_end = reference.word_len;
    let tagged_that_enchantment = reference.tagged_that_enchantment;

    let has_card = clause
        .after_words(reference_end)
        .is_some_and(|tail| {
        tail.tokens()
            .iter()
            .any(|token| token_word_is(token, CARD_WORD))
    });
    let mut descriptor_start = reference_end;
    let mut negative = false;
    let mut match_time = DemonstrativeMatchTime::Current;
    if clause_word_range_matches_any_phrase(clause, descriptor_start, DOESNT_HAVE_PHRASES) {
        descriptor_start += 2;
        negative = true;
    } else if clause_word_range_matches_phrase(clause, descriptor_start, DOES_NOT_HAVE_PHRASE) {
        descriptor_start += 3;
        negative = true;
    }
    if clause_word_at_is_any(clause, descriptor_start, IS_OR_ARE_WORDS) {
        descriptor_start += 1;
    // Keep present-tense `has`/`have` in the descriptor. The object-filter
    // grammar consumes that verb as part of counter clauses such as "it has a
    // counter on it"; stripping it here turns the remainder into an
    // unsupported standalone noun phrase.
    } else if clause_word_at_is_any(clause, descriptor_start, &["was", "were", "had"]) {
        descriptor_start += 1;
        match_time = DemonstrativeMatchTime::LastKnown;
    } else if clause_word_range_matches_any_phrase(
        clause,
        descriptor_start,
        &[&["isnt"], &["isn't"], &["arent"], &["aren't"]],
    ) {
        descriptor_start += 1;
        negative = true;
    } else if clause_word_range_matches_any_phrase(
        clause,
        descriptor_start,
        &[
            &["wasnt"],
            &["wasn't"],
            &["werent"],
            &["weren't"],
        ],
    ) {
        descriptor_start += 1;
        negative = true;
        match_time = DemonstrativeMatchTime::LastKnown;
    }

    if clause_word_at_is_any(clause, descriptor_start, &["not"]) {
        descriptor_start += 1;
        negative = true;
    }

    let mut nontoken_prefix = false;
    if clause_word_range_matches_phrase(clause, descriptor_start, NOT_TOKEN_PREFIX) {
        descriptor_start += 2;
        nontoken_prefix = true;
    }

    let range = words.token_span_for_words(descriptor_start, words.len())?;
    let mut descriptor_tokens = strip_leading_article_tokens(tokens.get(range)?).to_vec();
    if nontoken_prefix {
        descriptor_tokens.insert(0, OwnedLexToken::synthetic_word("nontoken"));
    }
    // "entered from <zone>" is a zone-motion clause, not an identity
    // descriptor. The object-filter grammar cannot model the provenance, so
    // absorbing it would silently drop the "entered from" constraint.
    if descriptor_contains_entered_from_motion(&descriptor_tokens) {
        return None;
    }
    (!descriptor_tokens.is_empty()).then_some((
        descriptor_tokens,
        negative,
        has_card,
        tagged_that_enchantment,
        match_time,
    ))
}

fn descriptor_contains_entered_from_motion(tokens: &[OwnedLexToken]) -> bool {
    let mut saw_entered = false;
    for token in tokens {
        if token_word_is(token, "entered") {
            saw_entered = true;
        } else if saw_entered && token_word_is(token, "from") {
            return true;
        }
    }
    false
}

fn demonstrative_match_predicate(
    filter: ObjectFilter,
    match_time: DemonstrativeMatchTime,
) -> PredicateAst {
    match match_time {
        DemonstrativeMatchTime::Current => PredicateAst::ItMatches(filter),
        DemonstrativeMatchTime::LastKnown => PredicateAst::ItMatchedLastKnown(filter),
    }
}

fn demonstrative_reference_kind(tokens: &[OwnedLexToken]) -> Option<DemonstrativeReferenceKind> {
    demonstrative_reference_prefix(LexedClause::new(tokens)).map(|reference| reference.kind)
}

fn parse_demonstrative_or_descriptor_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some((descriptor_tokens, negative, _has_card, tagged_that_enchantment, match_time)) =
        demonstrative_descriptor_filter_tokens(tokens)
    else {
        return Ok(None);
    };
    if tagged_that_enchantment {
        return Ok(None);
    }
    let Some(or_idx) = token_index_for_word(&descriptor_tokens, OR_WORD) else {
        return Ok(None);
    };
    if crate::object_filters::is_comparison_or_delimiter(&descriptor_tokens, or_idx) {
        return Ok(None);
    }
    let left_tokens = &descriptor_tokens[..or_idx];
    let right_tokens = &descriptor_tokens[or_idx + 1..];
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return Ok(None);
    }

    let antecedent_surface = demonstrative_antecedent_surface(tokens);
    let parse_branch = |branch_tokens: &[OwnedLexToken]| -> Result<ObjectFilter, CardTextError> {
        let mut filter =
            if let Some(filter) = parse_single_card_type_card_descriptor_tokens(branch_tokens) {
                filter
            } else {
                parse_object_filter_lexed(branch_tokens, false)?
            };
        if antecedent_surface.is_some() {
            filter.set_demonstrative_antecedent_surface(antecedent_surface);
        }
        Ok(filter)
    };

    let left = parse_branch(left_tokens)?;
    let right = parse_branch(right_tokens)?;
    if left == ObjectFilter::default() || right == ObjectFilter::default() {
        return Ok(None);
    }

    let predicate = PredicateAst::Or(
        Box::new(demonstrative_match_predicate(left, match_time)),
        Box::new(demonstrative_match_predicate(right, match_time)),
    );
    Ok(Some(if negative {
        // A single negated copula scopes over the complete coordinated
        // descriptor: "it isn't a creature or Vehicle" means the object is
        // neither one, not "not a creature OR a Vehicle."
        PredicateAst::Not(Box::new(predicate))
    } else {
        predicate
    }))
}

fn is_it_demonstrative_subject_clause(clause: LexedClause<'_>) -> bool {
    let Some(reference) = demonstrative_reference_prefix(clause) else {
        return false;
    };
    if reference.kind != DemonstrativeReferenceKind::It {
        return false;
    }
    let Some(tail) = clause.after_words(reference.word_len) else {
        return false;
    };
    tail.tokens().is_empty()
        || tail
            .tokens()
            .iter()
            .all(|token| token_word_is_any(token, HAS_OR_HAVE_WORDS))
}

fn parse_demonstrative_toxic_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    let subject = relation.subject_clause;
    let reference = demonstrative_reference_prefix(subject)?;
    if !surface::exact(relation.tail_clause, &["toxic"]) {
        return None;
    }
    let mut filter = ObjectFilter::default().with_ability_marker("toxic");
    let tail_is_creature = subject
        .after_words(reference.word_len)
        .and_then(|tail| tail.token(0))
        .is_some_and(|token| token_word_is(token, CREATURE_WORD));
    if reference.reference_is_creature || tail_is_creature {
        filter.card_types.push(CardType::Creature);
    }
    filter.set_demonstrative_antecedent_surface(reference.antecedent_surface);
    Some(PredicateAst::ItMatches(filter))
}

fn parse_demonstrative_power_or_toughness_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some((descriptor_tokens, _, _, _, mut match_time)) =
        demonstrative_descriptor_filter_tokens(tokens)
    else {
        return Ok(None);
    };
    let antecedent_surface = demonstrative_antecedent_surface(tokens);
    let descriptor_words = LexedClause::new(&descriptor_tokens).word_refs();
    if descriptor_words.len() < 2 || !word_is_any(descriptor_words[0], POWER_OR_TOUGHNESS_WORDS) {
        return Ok(None);
    }
    let axis = descriptor_words[0];
    if descriptor_words
        .get(1)
        .is_some_and(|word| word_is_any(word, &["was", "were"]))
    {
        match_time = DemonstrativeMatchTime::LastKnown;
    }
    let value_tail = if descriptor_words
        .get(1)
        .is_some_and(|word| word_is_any(word, BE_VERB_WORDS))
    {
        &descriptor_words[2..]
    } else {
        &descriptor_words[1..]
    };
    let clause_words = LexedClause::new(tokens).word_refs();
    let Some((cmp, _consumed)) = parse_filter_comparison_tokens(axis, value_tail, &clause_words)?
    else {
        return Ok(None);
    };
    let mut filter = ObjectFilter::default();
    if axis == POWER_WORD {
        filter.power = Some(cmp);
    } else {
        filter.toughness = Some(cmp);
    }
    filter.set_demonstrative_antecedent_surface(antecedent_surface);
    Ok(Some(demonstrative_match_predicate(filter, match_time)))
}

fn parse_demonstrative_mana_value_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let optional_copula = [WinnowSequence::action(
        "copula",
        WinnowCaptureKind::OneOf(&["is", "are", "was", "were"]),
    )];
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilAnyPhrase(&[&["mana", "value"]]),
        ),
        WinnowSequence::phrase(&["mana", "value"]),
        WinnowSequence::optional(&optional_copula),
        WinnowSequence::amount("comparison", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let Some(subject) = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause) else {
        return Ok(None);
    };
    // A disjunction in the subject belongs to the predicate grammar, not to
    // the object-filter prefix of a single mana-value comparison.  Let the
    // outer `or` parser preserve each branch's independent constraints (for
    // example, land OR creature-with-a-bounded-mana-value).
    if subject
        .tokens()
        .iter()
        .any(|token| token_word_is(token, OR_WORD))
    {
        return Ok(None);
    }
    let mut filter = if is_it_demonstrative_subject_clause(subject) {
        ObjectFilter::default()
    } else {
        let Some((mut descriptor_tokens, negative, _has_card, tagged_that_enchantment, _)) =
            demonstrative_descriptor_filter_tokens(subject.tokens())
        else {
            return Ok(None);
        };
        if negative || tagged_that_enchantment {
            return Ok(None);
        }
        while descriptor_tokens
            .last()
            .is_some_and(|token| token_word_is(token, "with"))
        {
            descriptor_tokens.pop();
        }
        let descriptor_tokens = strip_leading_article_tokens(&descriptor_tokens);
        if descriptor_tokens.is_empty() {
            ObjectFilter::default()
        } else if let Some(filter) =
            parse_single_card_type_card_descriptor_tokens(descriptor_tokens)
        {
            filter
        } else {
            parse_object_filter_lexed(descriptor_tokens, false)?
        }
    };
    let Some(comparison) = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause) else {
        return Ok(None);
    };
    let comparison_words = comparison.word_refs();
    if surface::exact_any(comparison, COLORS_SPENT_TO_CAST_SOURCE_TAIL_PHRASES) {
        return Ok(Some(
            PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell,
        ));
    }
    let clause_words = clause.word_refs();
    let Some((cmp, _consumed)) =
        parse_filter_comparison_tokens("mana value", &comparison_words, &clause_words)?
    else {
        return Ok(None);
    };
    filter.mana_value = Some(cmp);
    let match_time = if clause
        .word_refs()
        .iter()
        .any(|word| word_is_any(word, &["was", "were"]))
    {
        DemonstrativeMatchTime::LastKnown
    } else {
        DemonstrativeMatchTime::Current
    };
    Ok(Some(demonstrative_match_predicate(filter, match_time)))
}

fn parse_demonstrative_total_power_toughness_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["total", "power", "and", "toughness"]),
        ),
        WinnowSequence::phrase(&["total", "power", "and", "toughness"]),
        WinnowSequence::amount("comparison", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let Some(subject) = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause) else {
        return Ok(None);
    };
    if !is_it_demonstrative_subject_clause(subject) {
        return Ok(None);
    }
    let Some(comparison) = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause) else {
        return Ok(None);
    };
    let comparison_words = comparison.word_refs();
    let clause_words = clause.word_refs();
    let Some((cmp, _consumed)) =
        parse_filter_comparison_tokens("power", &comparison_words, &clause_words)?
    else {
        return Ok(None);
    };
    Ok(Some(PredicateAst::ItMatches(ObjectFilter {
        total_power_toughness: Some(cmp),
        ..Default::default()
    })))
}

fn parse_demonstrative_keyword_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("reference", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("action", WinnowCaptureKind::OneOf(&["has", "have"])),
        WinnowSequence::object("keyword", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let reference = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    demonstrative_reference_prefix(reference)?;
    let keyword = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let (constraint, consumed) = parse_filter_keyword_constraint_tokens(keyword.tokens())?;
    if consumed != keyword.tokens().len() {
        return None;
    }
    let mut filter = ObjectFilter::default();
    apply_filter_keyword_constraint(&mut filter, constraint, false);
    Some(PredicateAst::ItMatches(filter))
}

fn parse_demonstrative_shares_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let (descriptor_tokens, _, _, _, _) = demonstrative_descriptor_filter_tokens(tokens)?;
    let descriptor = LexedClause::new(&descriptor_tokens);
    if surface::exact_any(
        descriptor,
        &[
            &[
                "shares", "a", "creature", "type", "with", "this", "creature",
            ],
            &["shares", "creature", "type", "with", "this", "creature"],
        ],
    ) {
        let mut filter = ObjectFilter::creature();
        filter.shares_creature_type_with_source = true;
        return Some(PredicateAst::ItMatches(filter));
    }
    if surface::exact_any(
        descriptor,
        &[
            &["shares", "a", "card", "type", "with", "that", "spell"],
            &["shares", "card", "type", "with", "that", "spell"],
        ],
    ) {
        return Some(PredicateAst::ItMatches(
            ObjectFilter::default().shares_card_type_with_tagged("triggering"),
        ));
    }
    if surface::exact_any(
        descriptor,
        &[
            &[
                "shares",
                "a",
                "color",
                "with",
                "the",
                "most",
                "common",
                "color",
                "among",
                "all",
                "permanents",
                "or",
                "a",
                "color",
                "tied",
                "for",
                "most",
                "common",
            ],
            &[
                "shares",
                "color",
                "with",
                "most",
                "common",
                "color",
                "among",
                "all",
                "permanents",
                "or",
                "color",
                "tied",
                "for",
                "most",
                "common",
            ],
        ],
    ) {
        return Some(PredicateAst::ItMatches(
            ObjectFilter::default().shares_most_common_permanent_color(),
        ));
    }
    let words = descriptor.words();
    let shares_color_with_idx = words.find_window_by(4, |window| {
        matches!(
            window,
            ["shares", "a", "color", "with"] | ["shares", "color", "with", _]
        )
    })?;
    let filter_start = if descriptor
        .between_word_range(shares_color_with_idx, shares_color_with_idx + 4)
        .is_some_and(|clause| surface::exact(clause, &["shares", "a", "color", "with"]))
    {
        shares_color_with_idx + 4
    } else {
        shares_color_with_idx + 3
    };
    let filter_tokens = descriptor.after_words(filter_start)?.tokens();
    let mut filter =
        crate::grammar::primitives::probe_shape(parse_object_filter(filter_tokens, false))?;
    let player = match filter.controller.take() {
        Some(PlayerFilter::You) | None => PlayerAst::You,
        Some(PlayerFilter::Opponent) | Some(PlayerFilter::NotYou) => PlayerAst::Opponent,
        Some(PlayerFilter::Any) => PlayerAst::Any,
        _ => return None,
    };
    filter = filter.shares_color_with_tagged(crate::tag::CompilerReferenceTag::It.as_str());
    Some(PredicateAst::PlayerControls { player, filter })
}

fn contains_most_common_color_among_all_permanents_clause(tokens: &[OwnedLexToken]) -> bool {
    const MOST_COMMON_COLOR_AMONG_ALL_PERMANENTS_PATTERN: WinnowSequence<'static> =
        WinnowSequence::new(&[WinnowSequence::phrase(&[
            "most",
            "common",
            "color",
            "among",
            "all",
            "permanents",
        ])]);
    MOST_COMMON_COLOR_AMONG_ALL_PERMANENTS_PATTERN
        .locate_in(LexedClause::new(tokens))
        .is_some()
}

fn parse_shared_suffix_exact_count_or(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    for or_idx in (0..tokens.len()).rev() {
        if !token_word_is(&tokens[or_idx], OR_WORD) {
            continue;
        }
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..];
        if left_tokens.is_empty()
            || right_tokens.len() < 3
            || !right_tokens
                .first()
                .is_some_and(|token| token_word_is(token, "exactly"))
            || !left_tokens
                .iter()
                .any(|token| token_word_is(token, "exactly"))
        {
            continue;
        }
        let Some(have_idx) = crate::slice_primitives::select_last_position(left_tokens, |token| {
            token_word_is(token, "has") || token_word_is(token, "have")
        }) else {
            continue;
        };
        let subject_and_verb = &left_tokens[..=have_idx];

        // The noun phrase is printed once after the second exact count. Try
        // each possible suffix boundary and let the full predicate grammar
        // validate both reconstructed branches.
        for suffix_start in 2..right_tokens.len() {
            let shared_suffix = &right_tokens[suffix_start..];
            if shared_suffix.is_empty() {
                continue;
            }
            let mut complete_left = left_tokens.to_vec();
            complete_left.extend_from_slice(shared_suffix);
            let mut complete_right = subject_and_verb.to_vec();
            complete_right.extend_from_slice(right_tokens);

            let Ok(left) = parse_predicate(&complete_left) else {
                continue;
            };
            let Ok(right) = parse_predicate(&complete_right) else {
                continue;
            };
            return Ok(Some(PredicateAst::Or(Box::new(left), Box::new(right))));
        }
    }

    Ok(None)
}

fn parse_or_predicate(tokens: &[OwnedLexToken]) -> Result<Option<PredicateAst>, CardTextError> {
    if let Some(predicate) = parse_shared_suffix_exact_count_or(tokens)? {
        return Ok(Some(predicate));
    }
    let Some(last_or_idx) = tokens
        .iter()
        .enumerate()
        .rev()
        .find_map(|(idx, token)| token_word_is(token, OR_WORD).then_some(idx))
    else {
        return Ok(None);
    };

    for or_idx in (0..=last_or_idx).rev() {
        if !token_word_is(&tokens[or_idx], OR_WORD) {
            continue;
        }
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..];
        if left_tokens.is_empty()
            || right_tokens.is_empty()
            || right_tokens
                .first()
                .is_some_and(|token| token_word_is_any(token, OR_COMPARISON_TAIL_WORDS))
        {
            continue;
        }

        let Ok(left) = parse_predicate(left_tokens) else {
            continue;
        };

        let right = match parse_predicate(right_tokens) {
            Ok(predicate) => predicate,
            Err(original_err) => {
                let Some(reference_prefix) = predicate_reference_prefix_tokens(left_tokens) else {
                    continue;
                };
                if predicate_tokens_start_with_reference(right_tokens) {
                    continue;
                }
                let mut prefixed_tokens = reference_prefix.to_vec();
                prefixed_tokens.extend_from_slice(right_tokens);
                match parse_predicate(&prefixed_tokens) {
                    Ok(predicate) => predicate,
                    Err(_) => return Err(original_err),
                }
            }
        };
        return Ok(Some(PredicateAst::Or(Box::new(left), Box::new(right))));
    }

    Ok(None)
}

fn parse_attacking_you_own_control_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let mut normalized_tokens: Vec<_> = tokens
        .iter()
        .filter_map(|token| {
            let word = token
                .parser_text()
                .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
            (!word.is_empty()).then(|| OwnedLexToken::synthetic_word(word))
        })
        .collect();
    let normalized_clause = LexedClause::new(&normalized_tokens);
    if let Some(exile_word_idx) = surface::find(normalized_clause, EXILE_THEM_PHRASE)
        && let Some(exile_token_idx) = normalized_clause
            .words()
            .token_span_for_words(exile_word_idx, exile_word_idx + 1)
            .map(|range| range.start)
    {
        normalized_tokens.truncate(exile_token_idx);
    }
    let tokens = normalized_tokens.as_slice();
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::object("left", WinnowCaptureKind::UntilPhrase(&["and"])),
        WinnowSequence::word("and"),
        WinnowSequence::object(
            "right",
            WinnowCaptureKind::UntilPhrase(&[
                "are",
                "attacking",
                "and",
                "you",
                "both",
                "own",
                "and",
                "control",
                "them",
            ]),
        ),
        WinnowSequence::phrase(&[
            "are",
            "attacking",
            "and",
            "you",
            "both",
            "own",
            "and",
            "control",
            "them",
        ]),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let left = matched.capture_clause("left", clause).ok_or_else(|| {
        CardTextError::ParseError("missing left subject in attacking meld predicate".to_string())
    })?;
    let right = matched.capture_clause("right", clause).ok_or_else(|| {
        CardTextError::ParseError("missing right subject in attacking meld predicate".to_string())
    })?;
    if left.tokens().is_empty() || right.tokens().is_empty() {
        return Ok(None);
    }

    let mut left_filter = parse_meld_subject_filter_clause(left).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported attacking meld predicate subject (predicate: '{}')",
            render_token_slice(tokens).trim()
        ))
    })?;
    left_filter.controller = Some(PlayerFilter::You);
    left_filter.owner = Some(PlayerFilter::You);
    left_filter.attacking = true;

    let mut right_filter = parse_meld_subject_filter_clause(right).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported attacking meld predicate tail (predicate: '{}')",
            render_token_slice(tokens).trim()
        ))
    })?;
    right_filter.controller = Some(PlayerFilter::You);
    right_filter.owner = Some(PlayerFilter::You);
    right_filter.attacking = true;

    Ok(Some(PredicateAst::And(
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: left_filter,
        }),
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: right_filter,
        }),
    )))
}

fn parse_you_both_own_and_control_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let atoms = [
        WinnowSequence::object("left", WinnowCaptureKind::UntilPhrase(&["and"])),
        WinnowSequence::word("and"),
        WinnowSequence::object("right", WinnowCaptureKind::Rest),
    ];
    let Some(relation) = parse_control_relation_clauses(tokens, false) else {
        return Ok(None);
    };
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(relation.tail_clause) else {
        return Ok(None);
    };
    if !is_you_both_own_and_clause(relation.subject_clause) {
        return Ok(None);
    }
    let left = matched
        .capture_clause("left", relation.tail_clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing left subject in own/control predicate".to_string())
        })?;
    let right = matched
        .capture_clause("right", relation.tail_clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing right subject in own/control predicate".to_string())
        })?;
    if left.tokens().is_empty() || right.tokens().is_empty() {
        return Ok(None);
    }

    let mut left_filter = parse_meld_subject_filter_clause(left).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported own-and-control predicate subject (predicate: '{}')",
            render_token_slice(tokens).trim()
        ))
    })?;
    left_filter.controller = Some(PlayerFilter::You);
    left_filter.owner = Some(PlayerFilter::You);
    let mut right_filter = parse_meld_subject_filter_clause(right).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported own-and-control predicate tail (predicate: '{}')",
            render_token_slice(tokens).trim()
        ))
    })?;
    right_filter.controller = Some(PlayerFilter::You);
    right_filter.owner = Some(PlayerFilter::You);

    Ok(Some(PredicateAst::And(
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: left_filter,
        }),
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: right_filter,
        }),
    )))
}

fn parse_meld_subject_filter_clause(
    clause: LexedClause<'_>,
) -> Result<ObjectFilter, CardTextError> {
    let clause = clause.trimmed();
    if clause.tokens().is_empty() {
        return Err(CardTextError::ParseError(
            "missing meld predicate subject".to_string(),
        ));
    }
    if is_source_reference_clause(clause) {
        return Ok(ObjectFilter::source());
    }

    parse_object_filter(clause.tokens(), false).or_else(|_| {
        Ok(ObjectFilter::default().named(render_token_slice(clause.tokens()).trim().to_string()))
    })
}

fn is_you_both_own_and_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["you", "both", "own", "and"])
}

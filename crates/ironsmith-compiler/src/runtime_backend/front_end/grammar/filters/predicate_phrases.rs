use super::super::super::leaf::{lower_activation_cost_cst, parse_activation_cost_tokens_rewrite};
use super::super::super::lexer::{
    LexedClause, OwnedLexToken, TokenKind, TokenWordView, render_token_slice, token_slice_first_is,
    token_slice_words_eq,
};
use super::*;
use crate::cards::TextSpan;
use crate::runtime_backend::grammar::conditions::{
    parse_control_or_controlled_relation_clauses, parse_control_relation_clauses,
    parse_copula_relation_clauses, parse_existential_object_clause, parse_has_relation_clauses,
    parse_negated_control_relation_clauses, parse_prepositional_copula_relation_clauses,
};
use crate::runtime_backend::util::{
    FilterKeywordConstraint, is_article, is_source_reference_words, parse_value,
    strip_leading_article_tokens,
};

#[path = "predicate_phrases/capture_shapes.rs"]
mod capture_shapes;
#[path = "predicate_phrases/surface.rs"]
mod surface;

pub(crate) use capture_shapes::{WinnowAtom, WinnowCaptureKind, WinnowCaptureRole, WinnowSequence};

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
const CHOSEN_NAME_TAG: &str = "__chosen_name__";
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
const ARTICLE_WORDS: &[&str] = &["a", "an"];
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
    "it", "its", "this", "that", "you", "your", "opponent", "player", "target", "source",
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
    surface::exact_any(clause, &[&["it"], &["its"]]) || is_source_reference_words(&words)
}

fn is_explicit_source_state_subject_clause(clause: LexedClause<'_>) -> bool {
    !surface::exact_any(clause, &[&["it"], &["its"]]) && is_source_reference_clause(clause)
}

fn is_source_card_reference_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["this"], &["this", "card"]])
}

fn parse_source_zone_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_prepositional_copula_relation_clauses(tokens, &["in"])?;
    let source = relation.subject_clause;
    if !is_source_reference_clause(source) {
        return None;
    }

    let zone = relation.tail_clause;
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
    parse_object_filter(tokens, false)
        .ok()
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
    let filter = parse_object_filter(descriptor_clause.tokens(), false)
        .ok()
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

fn parse_source_keyword_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    if !is_source_reference_clause(relation.subject_clause) {
        return None;
    }
    let (constraint, consumed) =
        parse_filter_keyword_constraint_tokens(relation.tail_clause.tokens())?;
    if consumed != relation.tail_clause.tokens().len() {
        return None;
    }
    let mut filter = ObjectFilter::default();
    apply_filter_keyword_constraint(&mut filter, constraint, false);
    Some(PredicateAst::SourceMatches(filter))
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
    {
        if surface::exact(relation.subject_clause, &["you"]) {
            let amount = matched
                .capture_clause_by_role(WinnowCaptureRole::Amount, relation.tail_clause)
                .ok_or_else(|| {
                    CardTextError::ParseError("missing amount in life predicate".to_string())
                })?;
            return life_total_at_most_from_amount_tokens(amount.tokens());
        }
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
    let Some((amount, used)) = parse_less_than_or_equal_quantity_prefix(
        amount_tokens,
        false,
        false,
        "life-total predicate",
    )?
    else {
        return Ok(None);
    };
    if used != amount_tokens.len() {
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
    let state_phrases: &[&[&str]] = &[&["enchanted"], &["equipped"], &["tapped"], &["untapped"]];
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
            WinnowCaptureKind::OneOf(&["enchanted", "equipped", "tapped", "untapped", "saddled"]),
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
    if !is_source_reference_clause(relation.subject_clause) {
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
    if !is_source_reference_clause(relation.subject_clause) {
        return None;
    }
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let target_clause = matched.capture_clause("target", relation.tail_clause)?;
    if !is_counter_on_source_pronoun_tail_clause(target_clause) {
        return None;
    }
    let counter_clause =
        matched.capture_clause_by_role(WinnowCaptureRole::Object, relation.tail_clause)?;
    let (comparison, used) = predicate_quantity_prefix_tokens(counter_clause.tokens())?;
    let count = comparison_to_at_least_threshold(&comparison)?;
    let counter_tail = counter_clause.tokens().get(used..)?;
    let counter_type = parse_terminal_counter_phrase(counter_tail)??;
    if surface::exact(relation.subject_clause, &["it"]) {
        return Some(PredicateAst::ValueComparison {
            left: Value::CountersOn(
                Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                Some(counter_type),
            ),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }
    Some(PredicateAst::SourceHasCounterAtLeast {
        counter_type,
        count,
    })
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
        let counter_type = parse_terminal_counter_phrase(&counter_tokens)??;
        return Some(PredicateAst::ValueComparison {
            left: Value::CountersOn(
                Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))),
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
    if !is_exact_counter_on_source_tail_clause(target) {
        return None;
    }
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
    })
}

fn is_exact_counter_on_source_tail_clause(clause: LexedClause<'_>) -> bool {
    const COUNTER_ON_SOURCE_TAIL_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::word("on"),
        WinnowSequence::subject("source", WinnowCaptureKind::Rest),
    ]);

    let Some(matched) = COUNTER_ON_SOURCE_TAIL_PATTERN.parse_full(clause) else {
        return false;
    };
    let Some(source) = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause) else {
        return false;
    };
    is_source_state_subject_clause(source)
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
    let target_filter = parse_object_filter(target.tokens(), false).ok()?;
    Some(PredicateAst::ItMatches(
        ObjectFilter::spell().targeting_object(target_filter),
    ))
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
    parse_quantity_comparison_prefix(tokens, false, false, "predicate quantity").ok()
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
            filter =
                filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
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
    let Some(graveyard_tokens) = graveyard_object_tokens_after_existential(graveyard_object) else {
        return None;
    };

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

fn matching_tail_phrase_len(
    clause: LexedClause<'_>,
    filter_start: usize,
    filter_end: usize,
    phrases: &[&[&str]],
) -> Option<usize> {
    phrases.iter().find_map(|phrase| {
        let len = phrase.len();
        (filter_end >= filter_start + len)
            .then(|| clause.between_word_range(filter_end - len, filter_end))
            .flatten()
            .is_some_and(|tail| surface::exact(tail, phrase))
            .then_some(len)
    })
}

fn comparative_power_toughness_tail(
    clause: LexedClause<'_>,
    filter_start: usize,
    filter_end: usize,
) -> Option<(crate::filter::PowerToughnessRelation, usize)> {
    matching_tail_phrase_len(
        clause,
        filter_start,
        filter_end,
        TOUGHNESS_GREATER_THAN_POWER_TAIL_PHRASES,
    )
    .map(|len| {
        (
            crate::filter::PowerToughnessRelation::ToughnessGreaterThanPower,
            len,
        )
    })
    .or_else(|| {
        matching_tail_phrase_len(
            clause,
            filter_start,
            filter_end,
            POWER_GREATER_THAN_TOUGHNESS_TAIL_PHRASES,
        )
        .map(|len| {
            (
                crate::filter::PowerToughnessRelation::PowerGreaterThanToughness,
                len,
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
    if let Some(control_condition) =
        crate::runtime_backend::grammar::conditions::parse_control_condition(
            tokens,
            crate::runtime_backend::grammar::conditions::ControlConditionOptions {
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

    let clause = LexedClause::new(tokens);
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
        filter_end = filter_end.saturating_sub(tail_len);
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
    control_condition: crate::runtime_backend::grammar::conditions::ControlConditionAst,
) -> PredicateAst {
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

fn parse_this_ability_resolution_count_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let count = ability_resolution_ordinal_count(clause)?;

    Some(PredicateAst::ThisAbilityResolvedThisTurnExactly(count))
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
            if needs_chosen_name {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: TagKey::from(CHOSEN_NAME_TAG),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                });
            }
            return Some(filter);
        }
        if let Ok(mut filter) = parse_object_filter(candidate.tokens(), false) {
            if stripped_card_noun {
                filter.zone = None;
            }
            if needs_chosen_name {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: TagKey::from(CHOSEN_NAME_TAG),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                });
            }
            return Some(filter);
        }
        if let Some(mut filter) = parse_color_only_object_filter_clause(candidate) {
            if stripped_card_noun {
                filter.zone = None;
            }
            if needs_chosen_name {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: TagKey::from(CHOSEN_NAME_TAG),
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
    let filter_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in passive this-way predicate".to_string())
        })?;
    if filter_clause.tokens().is_empty() {
        return Ok(None);
    }
    let Some(filter) = parse_this_way_object_filter_clause(filter_clause) else {
        return Ok(None);
    };
    Ok(Some(PredicateAst::TaggedMatches(
        TagKey::from(IT_TAG),
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
    let Some(filter) = parse_this_way_object_filter_clause(filter_clause) else {
        return Ok(None);
    };
    Ok(Some(PredicateAst::PlayerTaggedObjectMatches {
        player,
        tag: TagKey::from(IT_TAG),
        filter,
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
            tag: TagKey::from(IT_TAG),
            filter: ObjectFilter::default().in_zone(zone),
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
        tag: TagKey::from(IT_TAG),
        filter,
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
        TagKey::from(IT_TAG),
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
        if !token_word_is(&tokens[split_idx], AND_WORD) {
            continue;
        }

        let left_tokens = &tokens[..split_idx];
        let right_tokens = &tokens[split_idx + 1..];
        if left_tokens.is_empty() || right_tokens.is_empty() {
            continue;
        }
        if left_tokens
            .iter()
            .any(|token| token_word_is(token, AND_WORD))
            || right_tokens
                .iter()
                .any(|token| token_word_is(token, AND_WORD))
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
struct DemonstrativeReferencePrefix {
    word_len: usize,
    kind: DemonstrativeReferenceKind,
    tagged_that_enchantment: bool,
    reference_is_creature: bool,
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
    })
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
) -> Option<(Vec<OwnedLexToken>, bool, bool, bool)> {
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
    if clause_word_range_matches_any_phrase(clause, descriptor_start, DOESNT_HAVE_PHRASES) {
        descriptor_start += 2;
        negative = true;
    } else if clause_word_range_matches_phrase(clause, descriptor_start, DOES_NOT_HAVE_PHRASE) {
        descriptor_start += 3;
        negative = true;
    }
    if clause_word_at_is_any(clause, descriptor_start, IS_OR_ARE_WORDS) {
        descriptor_start += 1;
    } else if clause_word_range_matches_any_phrase(
        clause,
        descriptor_start,
        &[&["isnt"], &["isn't"], &["arent"], &["aren't"]],
    ) {
        descriptor_start += 1;
        negative = true;
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
    (!descriptor_tokens.is_empty()).then_some((
        descriptor_tokens,
        negative,
        has_card,
        tagged_that_enchantment,
    ))
}

fn demonstrative_reference_kind(tokens: &[OwnedLexToken]) -> Option<DemonstrativeReferenceKind> {
    demonstrative_reference_prefix(LexedClause::new(tokens)).map(|reference| reference.kind)
}

fn parse_demonstrative_or_descriptor_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some((descriptor_tokens, negative, _has_card, tagged_that_enchantment)) =
        demonstrative_descriptor_filter_tokens(tokens)
    else {
        return Ok(None);
    };
    if negative || tagged_that_enchantment {
        return Ok(None);
    }
    let Some(or_idx) = token_index_for_word(&descriptor_tokens, OR_WORD) else {
        return Ok(None);
    };
    let left_tokens = &descriptor_tokens[..or_idx];
    let right_tokens = &descriptor_tokens[or_idx + 1..];
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return Ok(None);
    }

    let parse_branch = |branch_tokens: &[OwnedLexToken]| -> Result<ObjectFilter, CardTextError> {
        if let Some(filter) = parse_single_card_type_card_descriptor_tokens(branch_tokens) {
            return Ok(filter);
        }
        parse_object_filter_lexed(branch_tokens, false)
    };

    let left = parse_branch(left_tokens)?;
    let right = parse_branch(right_tokens)?;
    if left == ObjectFilter::default() || right == ObjectFilter::default() {
        return Ok(None);
    }

    Ok(Some(PredicateAst::Or(
        Box::new(PredicateAst::ItMatches(left)),
        Box::new(PredicateAst::ItMatches(right)),
    )))
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
    Some(PredicateAst::ItMatches(filter))
}

fn parse_demonstrative_power_or_toughness_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some((descriptor_tokens, _, _, _)) = demonstrative_descriptor_filter_tokens(tokens) else {
        return Ok(None);
    };
    let descriptor_words = LexedClause::new(&descriptor_tokens).word_refs();
    if descriptor_words.len() < 2 || !word_is_any(descriptor_words[0], POWER_OR_TOUGHNESS_WORDS) {
        return Ok(None);
    }
    let axis = descriptor_words[0];
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
    Ok(Some(PredicateAst::ItMatches(filter)))
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
    let mut filter = if is_it_demonstrative_subject_clause(subject) {
        ObjectFilter::default()
    } else {
        let Some((mut descriptor_tokens, negative, _has_card, tagged_that_enchantment)) =
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
    Ok(Some(PredicateAst::ItMatches(filter)))
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
    let (descriptor_tokens, _, _, _) = demonstrative_descriptor_filter_tokens(tokens)?;
    let descriptor = LexedClause::new(&descriptor_tokens);
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
    let mut filter = parse_object_filter(filter_tokens, false).ok()?;
    let player = match filter.controller.take() {
        Some(PlayerFilter::You) | None => PlayerAst::You,
        Some(PlayerFilter::Opponent) | Some(PlayerFilter::NotYou) => PlayerAst::Opponent,
        Some(PlayerFilter::Any) => PlayerAst::Any,
        _ => return None,
    };
    filter = filter.shares_color_with_tagged(IT_TAG);
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

fn parse_or_predicate(tokens: &[OwnedLexToken]) -> Result<Option<PredicateAst>, CardTextError> {
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
    left_filter.attacking = true;

    let mut right_filter = parse_meld_subject_filter_clause(right).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported attacking meld predicate tail (predicate: '{}')",
            render_token_slice(tokens).trim()
        ))
    })?;
    right_filter.controller = Some(PlayerFilter::You);
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
    let mut right_filter = parse_meld_subject_filter_clause(right).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported own-and-control predicate tail (predicate: '{}')",
            render_token_slice(tokens).trim()
        ))
    })?;
    right_filter.controller = Some(PlayerFilter::You);

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

fn parse_implicit_subject_and_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::object("left", WinnowCaptureKind::UntilPhrase(&["and"])),
        WinnowSequence::word("and"),
        WinnowSequence::modifier("right", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let left_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing left side in and predicate".to_string())
        })?;
    let right_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing right side in and predicate".to_string())
        })?;
    if left_clause.tokens().is_empty() || right_clause.tokens().is_empty() {
        return Ok(None);
    }
    let Some(right_first) = right_clause.token(0) else {
        return Ok(None);
    };
    let right_starts_with_have = token_word_is(right_first, HAVE_WORD);
    if !right_starts_with_have && !token_word_is(right_first, YOU_WORD) {
        return Ok(None);
    }

    let left = parse_predicate(left_clause.tokens())?;
    let right_tokens = if right_starts_with_have {
        let mut tokens = vec![OwnedLexToken::word(
            YOU_WORD.to_string(),
            TextSpan::synthetic(),
        )];
        tokens.extend_from_slice(right_clause.tokens());
        tokens
    } else {
        right_clause.tokens().to_vec()
    };
    let right = parse_predicate(&right_tokens)?;
    Ok(Some(PredicateAst::And(Box::new(left), Box::new(right))))
}

fn parse_while_conjoined_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::object("left", WinnowCaptureKind::UntilPhrase(&["while"])),
        WinnowSequence::word("while"),
        WinnowSequence::modifier("right", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let left_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing left side in while predicate".to_string())
        })?;
    let right_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing right side in while predicate".to_string())
        })?;
    if left_clause.tokens().is_empty() || right_clause.tokens().is_empty() {
        return Ok(None);
    }

    let left = parse_predicate(left_clause.tokens())?;
    let right = parse_predicate(right_clause.tokens())?;
    if matches!(
        left,
        PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
            | PredicateAst::ColoredManaSpentToCastThisSpellAtLeast(_)
            | PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(_)
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-spent predicate tail (predicate: '{}')",
            render_token_slice(tokens).trim()
        )));
    }
    Ok(Some(PredicateAst::And(Box::new(left), Box::new(right))))
}

fn player_filter_for_turn_value(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Some(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Some(PlayerFilter::MostLifeTied),
        PlayerAst::LowestLifeTied => Some(PlayerFilter::LowestLifeTied),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::NotYou => Some(PlayerFilter::NotYou),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::ThatPlayerOrTargetController => {
            Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
        }
        PlayerAst::ItsController | PlayerAst::ItsOwner => None,
    }
}

fn player_ast_from_status_player_filter(player: PlayerFilter) -> Option<PlayerAst> {
    match player {
        PlayerFilter::You => Some(PlayerAst::You),
        PlayerFilter::Any => Some(PlayerAst::Any),
        PlayerFilter::Defending => Some(PlayerAst::Defending),
        PlayerFilter::Attacking => Some(PlayerAst::Attacking),
        PlayerFilter::Opponent => Some(PlayerAst::Opponent),
        PlayerFilter::IteratedPlayer => Some(PlayerAst::That),
        PlayerFilter::Target(base) if *base == PlayerFilter::Opponent => {
            Some(PlayerAst::TargetOpponent)
        }
        PlayerFilter::Target(_) => Some(PlayerAst::Target),
        _ => None,
    }
}

fn parse_player_status_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let status =
        crate::runtime_backend::grammar::conditions::parse_player_status_condition(tokens)?;
    match status.status {
        crate::runtime_backend::grammar::conditions::PlayerStatusAst::Monarch => {
            Some(PredicateAst::PlayerIsMonarch {
                player: player_ast_from_status_player_filter(status.player)?,
            })
        }
        crate::runtime_backend::grammar::conditions::PlayerStatusAst::Initiative => {
            Some(PredicateAst::PlayerHasInitiative {
                player: player_ast_from_status_player_filter(status.player)?,
            })
        }
        crate::runtime_backend::grammar::conditions::PlayerStatusAst::MaxSpeed => {
            Some(PredicateAst::ValueComparison {
                left: Value::Speed(status.player),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(4),
            })
        }
    }
}

fn parse_world_state_or_timing_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_initiative_choice_predicate_shape(tokens)
        .or_else(|| parse_night_state_predicate_shape(tokens))
        .or_else(|| parse_first_combat_phase_predicate_shape(tokens))
        .or_else(|| parse_cast_this_spell_during_main_phase_shape(tokens))
}

fn parse_empty_battlefield_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let relation = parse_copula_relation_clauses(clause.tokens())?;
    let subject_atoms = [
        WinnowSequence::amount("quantity", WinnowCaptureKind::OneOf(&["no"])),
        WinnowSequence::object(
            "object",
            WinnowCaptureKind::OneOf(&["creature", "creatures"]),
        ),
    ];
    WinnowSequence::new(&subject_atoms).parse_full(relation.subject_clause)?;
    let tail_atoms = [
        WinnowSequence::word("on"),
        WinnowSequence::modifier("zone", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&tail_atoms).parse_full(relation.tail_clause)?;
    let zone = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, relation.tail_clause)?;
    if !is_battlefield_zone_clause(zone) {
        return None;
    }
    Some(PredicateAst::PlayerControlsNo {
        player: PlayerAst::Any,
        filter: ObjectFilter::creature(),
    })
}

fn is_battlefield_zone_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["battlefield"], &["the", "battlefield"]])
}

fn parse_initiative_choice_predicate_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrase = &["has"];
    let atoms = [
        WinnowSequence::subject("first_player", WinnowCaptureKind::OneOf(&["you"])),
        WinnowSequence::word("or"),
        WinnowSequence::subject(
            "second_player",
            WinnowCaptureKind::UntilPhrase(action_phrase),
        ),
        WinnowSequence::action(
            "status_verb",
            WinnowCaptureKind::WordCount(action_phrase.len()),
        ),
        WinnowSequence::object("status", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let second_player = matched.capture_clause("second_player", clause)?;
    if !is_player_youre_attacking_clause(second_player) {
        return None;
    }
    let status = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_initiative_status_clause(status) {
        return None;
    }
    Some(PredicateAst::Or(
        Box::new(PredicateAst::PlayerHasInitiative {
            player: PlayerAst::You,
        }),
        Box::new(PredicateAst::PlayerHasInitiative {
            player: PlayerAst::Defending,
        }),
    ))
}

fn is_player_youre_attacking_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["player", "youre", "attacking"],
            &["a", "player", "youre", "attacking"],
        ],
    )
}

fn is_initiative_status_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["initiative"], &["the", "initiative"]])
}

fn parse_night_state_predicate_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let copula = [WinnowSequence::action(
        "copula",
        WinnowCaptureKind::OneOf(&["is"]),
    )];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::OneOf(&["it", "its"])),
        WinnowSequence::optional(&copula),
        WinnowSequence::object("state", WinnowCaptureKind::OneOf(&["night"])),
    ];
    WinnowSequence::new(&atoms).parse_full(clause)?;
    Some(PredicateAst::ItIsNight)
}

fn parse_first_combat_phase_predicate_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let copula = [WinnowSequence::action(
        "copula",
        WinnowCaptureKind::OneOf(&["is"]),
    )];
    let article = [WinnowSequence::word("the")];
    let tail_article = [WinnowSequence::word("the")];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::OneOf(&["it", "its"])),
        WinnowSequence::optional(&copula),
        WinnowSequence::optional(&article),
        WinnowSequence::object("phase", WinnowCaptureKind::WordCount(3)),
        WinnowSequence::word("of"),
        WinnowSequence::optional(&tail_article),
        WinnowSequence::modifier("turn", WinnowCaptureKind::OneOf(&["turn"])),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let phase = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_first_combat_phase_clause(phase) {
        return None;
    }
    Some(PredicateAst::FirstCombatPhaseOfTurn)
}

fn is_first_combat_phase_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["first", "combat", "phase"])
}

fn parse_cast_this_spell_during_main_phase_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let during_phrase = &["during"];
    let atoms = [
        WinnowSequence::subject("player", WinnowCaptureKind::OneOf(&["you"])),
        WinnowSequence::action("action", WinnowCaptureKind::OneOf(&["cast"])),
        WinnowSequence::object("spell", WinnowCaptureKind::UntilPhrase(during_phrase)),
        WinnowSequence::word("during"),
        WinnowSequence::modifier("phase", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !surface::exact(object, &["this", "spell"]) {
        return None;
    }
    let phase = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    if !is_your_main_phase_clause(phase) {
        return None;
    }
    Some(PredicateAst::ThisSpellPaidLabel(
        "CastDuringYourMainPhase".into(),
    ))
}

fn is_your_main_phase_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["your", "main", "phase"])
}

fn parse_player_achievement_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let achievement =
        crate::runtime_backend::grammar::conditions::parse_player_achievement_condition(tokens)?;
    let player = player_ast_from_status_player_filter(achievement.player)?;
    let predicate = match achievement.achievement {
        crate::runtime_backend::grammar::conditions::PlayerAchievementAst::CitysBlessing => {
            Some(PredicateAst::PlayerHasCitysBlessing { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerAchievementAst::CompletedDungeon {
            dungeon_name,
        } => Some(PredicateAst::PlayerCompletedDungeon {
            player,
            dungeon_name,
        }),
        crate::runtime_backend::grammar::conditions::PlayerAchievementAst::FullParty => {
            if player == PlayerAst::You {
                Some(PredicateAst::YouHaveFullParty)
            } else {
                None
            }
        }
    }?;
    if achievement.negated {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

fn parse_player_cards_in_hand_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    // Detect "... at the beginning of this turn" suffix and/or past-tense "had",
    // both of which select the at-turn-start variants. We rewrite the past-tense
    // verb to present in place (preserving real spans) so the shared captured
    // parser can match, instead of round-tripping through synthetic word tokens.
    let clause = LexedClause::new(tokens);
    let stripped = strip_at_beginning_this_turn_suffix_clause(clause);
    let at_turn_start_suffix = stripped.tokens().len() != clause.tokens().len();
    let base_tokens = stripped.tokens();

    let had_idx = token_index_for_word(base_tokens, "had");
    let at_turn_start = at_turn_start_suffix || had_idx.is_some();

    let mut present_tokens = base_tokens.to_vec();
    if let Some(had_idx) = had_idx {
        present_tokens[had_idx].replace_word("have");
    }

    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_cards_in_hand_condition(
            &present_tokens,
        )?;
    let player = player_ast_from_status_player_filter(condition.player.clone())?;

    if !at_turn_start && player == PlayerAst::You && condition.is_no_cards_in_hand() {
        return Some(PredicateAst::YouHaveNoCardsInHand);
    }

    match condition.comparison {
        crate::effect::Comparison::GreaterThanOrEqual(count) if count >= 0 => {
            Some(cards_in_hand_or_more(player, count as u32, at_turn_start))
        }
        crate::effect::Comparison::GreaterThan(count) if count >= -1 => Some(
            cards_in_hand_or_more(player, (count + 1) as u32, at_turn_start),
        ),
        crate::effect::Comparison::LessThanOrEqual(count) if count >= 0 => {
            Some(cards_in_hand_or_fewer(player, count as u32, at_turn_start))
        }
        crate::effect::Comparison::LessThan(count) if count > 0 => Some(cards_in_hand_or_fewer(
            player,
            (count - 1) as u32,
            at_turn_start,
        )),
        // "you have a card in hand" parses as Equal(1) but means "at least one";
        // map the count-or-more reading so the turn-start variant resolves.
        crate::effect::Comparison::Equal(count) if count >= 0 => {
            Some(cards_in_hand_or_more(player, count as u32, at_turn_start))
        }
        _ => None,
    }
}

fn cards_in_hand_or_more(player: PlayerAst, count: u32, at_turn_start: bool) -> PredicateAst {
    if at_turn_start {
        PredicateAst::PlayerCardsInHandAtTurnStartOrMore { player, count }
    } else {
        PredicateAst::PlayerCardsInHandOrMore { player, count }
    }
}

fn cards_in_hand_or_fewer(player: PlayerAst, count: u32, at_turn_start: bool) -> PredicateAst {
    if at_turn_start {
        PredicateAst::PlayerCardsInHandAtTurnStartOrFewer { player, count }
    } else {
        PredicateAst::PlayerCardsInHandOrFewer { player, count }
    }
}

fn strip_at_beginning_this_turn_suffix_clause(clause: LexedClause<'_>) -> LexedClause<'_> {
    for suffix in [
        ["at", "the", "beginning", "of", "this", "turn"].as_slice(),
        ["at", "beginning", "of", "this", "turn"].as_slice(),
    ] {
        let stripped = clause.without_trailing_phrase(suffix);
        if stripped.tokens().len() != clause.tokens().len() {
            return stripped;
        }
    }
    clause
}

fn parse_player_life_total_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_life_total_condition(tokens)?;
    let (operator, amount) = comparison_to_value_comparison_operator(condition.comparison)?;
    Some(PredicateAst::ValueComparison {
        left: crate::effect::Value::LifeTotal(condition.player),
        operator,
        right: crate::effect::Value::Fixed(amount),
    })
}

fn parse_player_life_relation_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation =
        crate::runtime_backend::grammar::conditions::parse_player_life_relation_condition(tokens)?;
    let player = player_ast_from_status_player_filter(relation.player)?;
    match relation.relation {
        crate::runtime_backend::grammar::conditions::PlayerLifeRelationAst::HasMoreLifeThanYou => {
            Some(PredicateAst::PlayerHasMoreLifeThanYou { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerLifeRelationAst::HasLessLifeThanYou => {
            Some(PredicateAst::PlayerHasLessLifeThanYou { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerLifeRelationAst::HasNoOpponentWithMoreLifeThan => {
            Some(PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer => {
            Some(PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player })
        }
    }
}

fn parse_count_parity_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("count", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::object("scope", WinnowCaptureKind::UntilPhrase(&["is"])),
        WinnowSequence::word("is"),
        WinnowSequence::action("parity", WinnowCaptureKind::OneOf(&["even", "odd"])),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let count_prefix = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !surface::exact_any(
        count_prefix,
        &[
            &["number", "of"],
            &["count", "of"],
            &["the", "number"],
            &["the", "count"],
        ],
    ) {
        return None;
    }
    let parity = matched.capture_clause("parity", clause)?;
    let even = match parity.token(0)?.parser_text() {
        "even" => true,
        "odd" => false,
        _ => return None,
    };
    let captured_scope = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let scope_tokens = if captured_scope.token(0)?.parser_text() == "of" {
        &captured_scope.tokens()[1..]
    } else {
        captured_scope.tokens()
    };
    let scope = LexedClause::new(scope_tokens);
    let count = match scope {
        scope if surface::exact_any(scope, &[&["permanent"], &["permanents"]]) => {
            crate::static_abilities::AnthemCountExpression::MatchingFilter(
                crate::target::ObjectFilter::permanent(),
            )
        }
        _ => return None,
    };
    Some(PredicateAst::CountParity {
        count,
        even,
        display: Some(format!(
            "the number of {} is {}",
            render_token_slice(scope.tokens()),
            if even { "even" } else { "odd" }
        )),
    })
}

fn parse_player_cards_in_hand_relation_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation =
        crate::runtime_backend::grammar::conditions::parse_player_cards_in_hand_relation_condition(
            tokens,
        )?;
    let player = player_ast_from_status_player_filter(relation.player)?;
    match relation.relation {
        crate::runtime_backend::grammar::conditions::PlayerCardsInHandRelationAst::HasMoreCardsInHandThanYou => {
            Some(PredicateAst::PlayerHasMoreCardsInHandThanYou { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerCardsInHandRelationAst::HasMoreCardsInHandThanEachOtherPlayer => {
            Some(PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player })
        }
    }
}

fn parse_player_turn_event_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_turn_event_condition(tokens)?;
    let (operator, count) = comparison_to_value_comparison_operator(condition.comparison)?;
    let left = match condition.event {
        crate::runtime_backend::grammar::conditions::PlayerTurnEventAst::CardsDrawn => {
            Value::MaxCardsDrawnThisTurn(condition.player)
        }
        crate::runtime_backend::grammar::conditions::PlayerTurnEventAst::LandsEnteredBattlefieldUnderControl => {
            if comparison_to_strict_at_least_threshold(&condition.comparison)
                .is_some_and(|count| count <= 1)
                || matches!(
                    condition.comparison,
                    crate::effect::Comparison::Equal(1)
                )
            {
                let player = player_ast_from_status_player_filter(condition.player)?;
                return Some(PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
                    player,
                });
            }
            Value::LandsEnteredBattlefieldThisTurn(condition.player)
        }
    };

    Some(PredicateAst::ValueComparison {
        left,
        operator,
        right: Value::Fixed(count),
    })
}

fn parse_turn_timing_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let subject = [WinnowSequence::subject(
        "subject",
        WinnowCaptureKind::OneOf(&["it", "its"]),
    )];
    let copula = [WinnowSequence::action(
        "copula",
        WinnowCaptureKind::OneOf(&["is", "s"]),
    )];
    let negation = [WinnowSequence::modifier(
        "negation",
        WinnowCaptureKind::OneOf(&["not"]),
    )];
    let atoms = [
        WinnowSequence::optional(&subject),
        WinnowSequence::optional(&copula),
        WinnowSequence::optional(&negation),
        WinnowSequence::object("turn", WinnowCaptureKind::WordCount(2)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    if matched.capture("copula").is_some() && matched.capture("subject").is_none() {
        return None;
    }
    let turn_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_your_turn_clause(turn_clause) {
        return None;
    }
    let predicate = PredicateAst::YourTurn;
    if matched.capture("negation").is_some() {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

fn is_your_turn_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["your", "turn"])
}

fn parse_opponent_controls_tagged_object_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = parse_control_relation_clauses(tokens, false)?;
    if !is_opponent_controller_clause(relation.subject_clause) {
        return None;
    }
    let mut filter = ObjectFilter {
        controller: Some(PlayerFilter::Opponent),
        ..Default::default()
    };
    match controlled_tagged_object_kind(relation.tail_clause)? {
        ControlledTaggedObjectKind::Permanent => {}
        ControlledTaggedObjectKind::Creature => filter.card_types.push(CardType::Creature),
    }
    Some(PredicateAst::ItMatches(filter))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlledTaggedObjectKind {
    Permanent,
    Creature,
}

fn is_opponent_controller_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["opponent"], &["an", "opponent"]])
}

fn controlled_tagged_object_kind(clause: LexedClause<'_>) -> Option<ControlledTaggedObjectKind> {
    if surface::exact_any(clause, &[&["it"], &["that", "permanent"]]) {
        return Some(ControlledTaggedObjectKind::Permanent);
    }
    if surface::exact(clause, &["that", "creature"]) {
        return Some(ControlledTaggedObjectKind::Creature);
    }
    None
}

fn parse_secret_choices_match_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("choices", WinnowCaptureKind::UntilPhrase(&["match"])),
        WinnowSequence::action("action", WinnowCaptureKind::OneOf(&["match"])),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_secret_choices_subject_clause(subject) {
        return None;
    }
    Some(PredicateAst::SecretChoicesMatch)
}

fn is_secret_choices_subject_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["they"], &["those", "choices"]])
}

fn parse_vote_result_predicate(
    tokens: &[OwnedLexToken],
    allow_tied: bool,
) -> Result<Option<PredicateAst>, CardTextError> {
    if let Some(predicate) = parse_vote_option_result_predicate(tokens, allow_tied) {
        return Ok(Some(predicate));
    }
    parse_no_vote_objects_matched_predicate(tokens)
}

fn parse_x_value_comparison_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    if let ["x", "is", tail @ ..] = words.as_slice() {
        let parsed = match tail {
            ["less", "than", "or", "equal", "to", amount] => Some((
                crate::effect::ValueComparisonOperator::LessThanOrEqual,
                parse_named_number(amount)? as i32,
            )),
            ["less", "than", amount] => Some((
                crate::effect::ValueComparisonOperator::LessThan,
                parse_named_number(amount)? as i32,
            )),
            ["greater", "than", "or", "equal", "to", amount] => Some((
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                parse_named_number(amount)? as i32,
            )),
            ["greater", "than", amount] => Some((
                crate::effect::ValueComparisonOperator::GreaterThan,
                parse_named_number(amount)? as i32,
            )),
            ["equal", "to", amount] | ["exactly", amount] => Some((
                crate::effect::ValueComparisonOperator::Equal,
                parse_named_number(amount)? as i32,
            )),
            _ => None,
        };
        if let Some((operator, amount)) = parsed {
            return Some(PredicateAst::ValueComparison {
                left: Value::X,
                operator,
                right: Value::Fixed(amount),
            });
        }
    }

    let relation = parse_copula_relation_clauses(tokens)?;
    if !surface::exact(relation.subject_clause, &["x"]) {
        return None;
    }
    let comparison_clause = relation.tail_clause;
    let (comparison, used) =
        parse_quantity_comparison_prefix(comparison_clause.tokens(), false, false, "x comparison")
            .ok()?;
    if used != comparison_clause.tokens().len() {
        return None;
    }
    let (operator, amount) = comparison_to_value_comparison_operator(comparison)?;
    Some(PredicateAst::ValueComparison {
        left: Value::X,
        operator,
        right: Value::Fixed(amount),
    })
}

fn parse_controlled_creatures_total_power_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    if !surface::exact_any(
        relation.subject_clause,
        &[
            &["creature", "you", "control"],
            &["creature", "you", "controls"],
            &["creatures", "you", "control"],
            &["creatures", "you", "controls"],
        ],
    ) {
        return None;
    }

    let tail_words = relation.tail_clause.word_refs();
    let mut comparison_words: primitives::WordSliceInput<'_> = tail_words.as_slice();
    primitives::word_slice_exact("total")
        .parse_next(&mut comparison_words)
        .ok()?;
    primitives::word_slice_exact("power")
        .parse_next(&mut comparison_words)
        .ok()?;
    let clause_words = LexedClause::new(tokens).word_refs();
    let Some((comparison, used)) =
        parse_filter_comparison_tokens("power", comparison_words, &clause_words).ok()?
    else {
        return None;
    };
    if used != comparison_words.len() {
        return None;
    }
    let (operator, amount) = match comparison {
        crate::filter::Comparison::GreaterThan(amount) => {
            (crate::effect::ValueComparisonOperator::GreaterThan, amount)
        }
        crate::filter::Comparison::GreaterThanOrEqual(amount) => (
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            amount,
        ),
        crate::filter::Comparison::Equal(amount) => {
            (crate::effect::ValueComparisonOperator::Equal, amount)
        }
        crate::filter::Comparison::LessThan(amount) => {
            (crate::effect::ValueComparisonOperator::LessThan, amount)
        }
        crate::filter::Comparison::LessThanOrEqual(amount) => (
            crate::effect::ValueComparisonOperator::LessThanOrEqual,
            amount,
        ),
        crate::filter::Comparison::NotEqual(amount) => {
            (crate::effect::ValueComparisonOperator::NotEqual, amount)
        }
        crate::filter::Comparison::OneOf(_)
        | crate::filter::Comparison::EqualExpr(_)
        | crate::filter::Comparison::NotEqualExpr(_)
        | crate::filter::Comparison::LessThanExpr(_)
        | crate::filter::Comparison::LessThanOrEqualExpr(_)
        | crate::filter::Comparison::GreaterThanExpr(_)
        | crate::filter::Comparison::GreaterThanOrEqualExpr(_) => return None,
    };
    Some(PredicateAst::ValueComparison {
        left: Value::TotalPower(ObjectFilter::creature().you_control()),
        operator,
        right: Value::Fixed(amount),
    })
}

fn parse_value_reference_comparison_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    for comparison_start in 1..tokens.len() {
        let Some((left, left_used)) = parse_value(&tokens[..comparison_start]) else {
            continue;
        };
        if left_used != comparison_start || !is_predicate_reference_value(&left) {
            continue;
        }
        let Some((operator, right_tokens)) =
            crate::runtime_backend::grammar::values::parse_value_comparison_tokens(
                &tokens[comparison_start..],
            )
        else {
            continue;
        };
        let Some((right, right_used)) = parse_value(right_tokens) else {
            continue;
        };
        if right_used != right_tokens.len() {
            continue;
        }
        return Some(PredicateAst::ValueComparison {
            left,
            operator,
            right,
        });
    }
    None
}

fn is_predicate_reference_value(value: &Value) -> bool {
    matches!(
        value,
        Value::PowerOf(_) | Value::ToughnessOf(_) | Value::SourcePower | Value::SourceToughness
    )
}

fn parse_paid_cost_label_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let paid_tail_phrases: &[&[&str]] = &[
        &["cost", "was", "paid"],
        &["cost", "wasnt", "paid"],
        &["cost", "was", "not", "paid"],
    ];
    let atoms = [
        WinnowSequence::object(
            "label",
            WinnowCaptureKind::UntilAnyPhrase(paid_tail_phrases),
        ),
        WinnowSequence::action("paid_tail", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let label_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let mut label_words = label_clause.word_refs();
    if label_words.first().copied() == Some("the") {
        label_words.remove(0);
    }
    let label_words = strip_source_possessive_label_prefix(&label_words);
    let paid_tail = matched.capture_clause("paid_tail", clause)?;
    let negated = paid_cost_tail_is_negated(paid_tail)?;
    let label = if label_words.len() == 3
        && surface::exact_words(&label_words[..1], &["this"])
        && is_this_spell_possessive_word(label_words[1])
    {
        named_paid_cost_label_from_word(label_words[2])?
    } else if label_words.len() == 2 && is_paid_cost_possessive_word(label_words[0]) {
        named_paid_cost_label_from_word(label_words[1])?
    } else if label_words.len() == 1 {
        mana_cost_label_from_words(label_words)
            .or_else(|| named_paid_cost_label_from_word(label_words[0]))?
    } else {
        mana_cost_label_from_words(label_words)?
    };
    let predicate = PredicateAst::ThisSpellPaidLabel(label.into());
    if negated {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

fn paid_cost_tail_is_negated(clause: LexedClause<'_>) -> Option<bool> {
    if surface::prefix(clause, &["cost", "was", "paid"]) {
        return Some(false);
    }
    if surface::prefix_any(
        clause,
        &[&["cost", "wasnt", "paid"], &["cost", "was", "not", "paid"]],
    ) {
        return Some(true);
    }
    None
}

fn parse_vote_option_result_predicate(
    tokens: &[OwnedLexToken],
    allow_tied: bool,
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("option", WinnowCaptureKind::UntilPhrase(&["gets"])),
        WinnowSequence::action("action", WinnowCaptureKind::OneOf(&["gets"])),
        WinnowSequence::object("result", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let option = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if option.tokens().is_empty() {
        return None;
    }
    let result = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let option = render_token_slice(option.tokens());
    if surface::exact(result, &["more", "votes"]) {
        return Some(PredicateAst::VoteOptionGetsMoreVotes { option });
    }
    if allow_tied
        && surface::exact_any(
            result,
            &[
                &["more", "votes", "or", "vote", "is", "tied"],
                &["more", "votes", "or", "the", "vote", "is", "tied"],
            ],
        )
    {
        return Some(PredicateAst::VoteOptionGetsMoreVotesOrTied { option });
    }
    None
}

fn parse_no_vote_objects_matched_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::amount("quantity", WinnowCaptureKind::OneOf(&["no"])),
        WinnowSequence::object("objects", WinnowCaptureKind::UntilPhrase(&["got", "votes"])),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let action = matched
        .capture_clause_by_role(WinnowCaptureRole::Action, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing action in vote result predicate".to_string())
        })?;
    if !surface::exact(action, &["got", "votes"]) {
        return Ok(None);
    }
    let objects = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in vote result predicate".to_string())
        })?;
    let filter = parse_object_filter(objects.tokens(), false)?;
    Ok(Some(PredicateAst::NoVoteObjectsMatched { filter }))
}

fn parse_spell_context_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition =
        crate::runtime_backend::grammar::conditions::parse_spell_context_condition(tokens)?;
    match condition {
        crate::runtime_backend::grammar::conditions::SpellContextConditionAst::ControllerIsPoisoned {
            ..
        } => Some(PredicateAst::TargetSpellControllerIsPoisoned),
        crate::runtime_backend::grammar::conditions::SpellContextConditionAst::NoManaSpentToCast {
            ..
        } => Some(PredicateAst::TargetSpellNoManaSpentToCast),
        crate::runtime_backend::grammar::conditions::SpellContextConditionAst::YouControlMoreCreaturesThanController {
            ..
        } => Some(PredicateAst::YouControlMoreCreaturesThanTargetSpellController),
    }
}

fn parse_player_spell_cast_this_turn_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_spell_cast_this_turn_condition(
            tokens,
        )?;
    match condition {
        crate::runtime_backend::grammar::conditions::PlayerSpellCastThisTurnConditionAst::CountAtLeast {
            player,
            count,
        } => Some(PredicateAst::PlayerCastSpellsThisTurnOrMore {
            player: player_ast_from_status_player_filter(player)?,
            count,
        }),
        crate::runtime_backend::grammar::conditions::PlayerSpellCastThisTurnConditionAst::MatchingFilters {
            player,
            filters,
            negated,
        } => {
            let mut predicates = filters.into_iter().map(|filter| {
                PredicateAst::ValueComparison {
                    left: Value::SpellsCastThisTurnMatching {
                        player: player.clone(),
                        filter,
                        exclude_source: false,
                    },
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                }
            });
            let first = predicates.next()?;
            let predicate = predicates
                .fold(first, |left, right| PredicateAst::And(Box::new(left), Box::new(right)));
            if negated {
                Some(PredicateAst::Not(Box::new(predicate)))
            } else {
                Some(predicate)
            }
        }
    }
}

fn parse_player_life_change_this_turn_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_life_change_this_turn_condition(
            tokens,
        )?;
    match condition.direction {
        crate::runtime_backend::grammar::conditions::PlayerLifeChangeDirectionAst::Gained => {
            let count = comparison_to_strict_at_least_threshold(&condition.comparison)?;
            Some(PredicateAst::PlayerGainedLifeThisTurnOrMore {
                player: player_ast_from_status_player_filter(condition.player)?,
                count,
            })
        }
        crate::runtime_backend::grammar::conditions::PlayerLifeChangeDirectionAst::Lost
            if condition.player == PlayerFilter::Opponent
                && comparison_to_strict_at_least_threshold(&condition.comparison) == Some(1) =>
        {
            Some(PredicateAst::OpponentLostLifeThisTurn)
        }
        crate::runtime_backend::grammar::conditions::PlayerLifeChangeDirectionAst::Lost
            if condition.player == PlayerFilter::Any =>
        {
            let count = comparison_to_strict_at_least_threshold(&condition.comparison)?;
            Some(PredicateAst::AnyPlayerLostLifeThisTurnOrMore { count })
        }
        crate::runtime_backend::grammar::conditions::PlayerLifeChangeDirectionAst::Lost => {
            let (operator, count) = comparison_to_value_comparison_operator(condition.comparison)?;
            Some(PredicateAst::ValueComparison {
                left: Value::LifeLostThisTurn(condition.player),
                operator,
                right: Value::Fixed(count),
            })
        }
    }
}

fn parse_object_death_this_turn_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition =
        crate::runtime_backend::grammar::conditions::parse_object_death_this_turn_condition(
            tokens,
        )?;
    match condition.event {
        crate::runtime_backend::grammar::conditions::ObjectDeathThisTurnEventAst::Died => {
            let count = comparison_to_strict_at_least_threshold(&condition.comparison)?;
            if let Some(damager) = condition.damaged_by {
                return Some(PredicateAst::CreatureDealtDamageBySourceDiedThisTurn {
                    victim: condition.filter,
                    damager,
                    count,
                });
            }
            if let Some(player) = condition.under_controller {
                return Some(PredicateAst::ValueComparison {
                    left: Value::CreaturesDiedThisTurnControlledBy(player),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(count as i32),
                });
            }
            if count <= 1 {
                Some(PredicateAst::CreatureDiedThisTurn)
            } else {
                Some(PredicateAst::CreatureDiedThisTurnOrMore(count))
            }
        }
        crate::runtime_backend::grammar::conditions::ObjectDeathThisTurnEventAst::PutIntoYourGraveyardFromAnywhere => {
            Some(PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn)
        }
    }
}

fn parse_player_would_action_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_would_action_condition(tokens)?;
    let player = player_ast_from_status_player_filter(condition.player)?;
    match condition.action {
        crate::runtime_backend::grammar::conditions::PlayerWouldActionAst::DrawCard => {
            Some(PredicateAst::PlayerWouldDrawCard { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerWouldActionAst::Proliferate => {
            Some(PredicateAst::PlayerWouldProliferate { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerWouldActionAst::BeginExtraTurn => {
            Some(PredicateAst::PlayerWouldBeginExtraTurn { player })
        }
    }
}

fn parse_battlefield_entry_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition =
        crate::runtime_backend::grammar::conditions::parse_battlefield_entry_condition(tokens)?;
    match condition {
        crate::runtime_backend::grammar::conditions::BattlefieldEntryConditionAst::ObjectEntered {
            filter,
            window:
                crate::runtime_backend::grammar::conditions::BattlefieldEntryTurnWindowAst::ThisTurn,
        } => Some(PredicateAst::ObjectEnteredBattlefieldThisTurn(filter)),
        crate::runtime_backend::grammar::conditions::BattlefieldEntryConditionAst::ObjectEntered {
            filter,
            window:
                crate::runtime_backend::grammar::conditions::BattlefieldEntryTurnWindowAst::LastTurn,
        } => Some(PredicateAst::ObjectEnteredBattlefieldLastTurn(filter)),
        crate::runtime_backend::grammar::conditions::BattlefieldEntryConditionAst::LandEnteredUnderYourControlThisTurn {
            player,
        } => Some(PredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player }),
    }
}

fn parse_battlefield_change_this_turn_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition =
        crate::runtime_backend::grammar::conditions::parse_battlefield_change_this_turn_condition(
            tokens,
        )?;
    match condition {
        crate::runtime_backend::grammar::conditions::BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield {
            negated,
        } => {
            let predicate = PredicateAst::PermanentLeftBattlefieldThisTurn;
            if negated {
                Some(PredicateAst::Not(Box::new(predicate)))
            } else {
                Some(predicate)
            }
        }
        crate::runtime_backend::grammar::conditions::BattlefieldChangeThisTurnConditionAst::NonlandPermanentLeftBattlefieldOrSpellWarped => {
            Some(PredicateAst::Or(
                Box::new(PredicateAst::NonlandPermanentLeftBattlefieldThisTurn),
                Box::new(PredicateAst::SpellWasWarpedThisTurn),
            ))
        }
        crate::runtime_backend::grammar::conditions::BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefieldUnderYourControl => {
            Some(PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn)
        }
        crate::runtime_backend::grammar::conditions::BattlefieldChangeThisTurnConditionAst::ObjectPutIntoGraveyardFromBattlefield {
            filter,
        } => Some(PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter)),
    }
}

fn parse_combat_damage_this_turn_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_source_dealt_combat_damage_this_turn_shape(tokens)
        .or_else(|| parse_player_dealt_combat_damage_by_subtype_this_turn_shape(tokens))
}

fn is_player_object_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["player"], &["a", "player"]])
}

fn combat_damage_player_subject_clause(clause: LexedClause<'_>) -> Option<PlayerAst> {
    if surface::exact_any(clause, &[&["a", "player"], &["player"]]) {
        return Some(PlayerAst::Any);
    }
    if surface::exact_any(clause, &[&["an", "opponent"], &["opponent"]]) {
        return Some(PlayerAst::Opponent);
    }
    None
}

fn single_subtype_word_clause(clause: LexedClause<'_>) -> Option<&str> {
    let words = clause.word_refs();
    let words = strip_leading_article_word_refs(&words);
    (words.len() == 1).then_some(words[0])
}

fn is_this_turn_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["this", "turn"])
}

fn is_this_combat_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["this", "combat"])
}

fn is_attacked_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["attacked"])
}

fn is_triggering_attack_subject_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["that", "creature"], &["it"]])
}

fn is_other_creatures_this_combat_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["other", "creature", "this", "combat"],
            &["other", "creatures", "this", "combat"],
            &["others", "creature", "this", "combat"],
            &["others", "creatures", "this", "combat"],
        ],
    )
}

fn is_source_attacked_or_blocked_subject_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["this", "creature"],
            &["this", "permanent"],
            &["this"],
            &["it"],
        ],
    )
}

fn is_attacked_or_blocked_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["attacked", "or", "blocked"])
}

fn is_source_did_not_attack_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact(clause, &["this", "creature"])
}

fn is_entered_under_your_control_tail_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["or", "come", "under", "your", "control"],
            &["or", "came", "under", "your", "control"],
        ],
    )
}

fn parse_source_dealt_combat_damage_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrase = &["dealt", "combat", "damage", "to"];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["this", "turn"])),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !surface::exact(subject_clause, &["it"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_player_object_clause(object_clause) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_this_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::SourceDealtCombatDamageToPlayerThisTurn)
}

fn parse_player_dealt_combat_damage_by_subtype_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrase = &["was", "dealt", "combat", "damage", "by"];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
        WinnowSequence::object("subtype", WinnowCaptureKind::UntilPhrase(&["this", "turn"])),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    let player = combat_damage_player_subject_clause(subject_clause)?;
    let subtype_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let subtype_word = single_subtype_word_clause(subtype_clause)?;
    let subtype = parse_subtype_word(subtype_word)?;
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_this_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype })
}

fn parse_combat_turn_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_you_attacked_this_turn_shape(tokens)
        .or_else(|| parse_triggering_object_had_to_attack_this_combat_shape(tokens))
        .or_else(|| parse_you_attacked_with_n_or_more_creatures_shape(tokens))
        .or_else(|| parse_you_attacked_with_exactly_other_creatures_shape(tokens))
        .or_else(|| parse_source_attacked_or_blocked_this_turn_shape(tokens))
}

fn parse_you_attacked_this_turn_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(&["attacked"])),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_attacked_action_clause(action_clause) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_this_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::YouAttackedThisTurn)
}

fn parse_triggering_object_had_to_attack_this_combat_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["had", "to", "attack"], &["must", "attack"]];
    for action_phrase in action_phrases {
        let atoms = [
            WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
            WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
            WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
        ];
        let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
        if !is_triggering_attack_subject_clause(subject_clause) {
            continue;
        }
        let window_clause = matched.capture_clause("window", clause)?;
        if !is_this_combat_clause(window_clause) {
            continue;
        }
        return Some(PredicateAst::TriggeringObjectHadToAttackThisCombat);
    }
    None
}

/// "you attacked with N or more creatures this turn" (Windbrisk Heights)
fn parse_you_attacked_with_n_or_more_creatures_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let tail_phrases: &[&[&str]] = &[
        &["or", "more", "creatures", "this", "turn"],
        &["or", "more", "creature", "this", "turn"],
    ];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::amount("count", WinnowCaptureKind::UntilAnyPhrase(tail_phrases)),
        WinnowSequence::object("object", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !surface::exact(action_clause, &["attacked", "with"]) {
        return None;
    }
    let count_clause = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let (count, used) = parse_number(count_clause.tokens())?;
    if used != count_clause.tokens().len() {
        return None;
    }
    Some(PredicateAst::YouAttackedWithNOrMoreCreaturesThisTurn(count))
}

fn parse_you_attacked_with_exactly_other_creatures_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let tail_phrases: &[&[&str]] = &[
        &["other", "creature", "this", "combat"],
        &["other", "creatures", "this", "combat"],
        &["others", "creature", "this", "combat"],
        &["others", "creatures", "this", "combat"],
    ];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(3)),
        WinnowSequence::amount("count", WinnowCaptureKind::UntilAnyPhrase(tail_phrases)),
        WinnowSequence::object("object", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !surface::exact(action_clause, &["attacked", "with", "exactly"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_other_creatures_this_combat_clause(object_clause) {
        return None;
    }
    let count_clause = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let (count, used) = parse_number(count_clause.tokens())?;
    if used != count_clause.tokens().len() {
        return None;
    }
    Some(PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(count))
}

fn parse_source_attacked_or_blocked_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["attacked", "or", "blocked"]),
        ),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(3)),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_source_attacked_or_blocked_subject_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_attacked_or_blocked_action_clause(action_clause) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_this_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::SourceAttackedOrBlockedThisTurn)
}

fn parse_source_did_not_attack_or_enter_control_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["didnt", "attack"]),
        ),
        WinnowSequence::modifier("negation", WinnowCaptureKind::OneOf(&["didnt"])),
        WinnowSequence::action("attack", WinnowCaptureKind::OneOf(&["attack"])),
        WinnowSequence::modifier(
            "enter",
            WinnowCaptureKind::UntilAnyPhrase(&[&["this", "turn"]]),
        ),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_source_did_not_attack_subject_clause(subject_clause) {
        return None;
    }
    let enter_clause = matched.capture_clause("enter", clause)?;
    if !is_entered_under_your_control_tail_clause(enter_clause) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_this_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::And(
        Box::new(PredicateAst::Not(Box::new(
            PredicateAst::SourceAttackedThisTurn,
        ))),
        Box::new(PredicateAst::Not(Box::new(
            PredicateAst::SourceCameUnderYourControlThisTurn,
        ))),
    ))
}

fn parse_spell_lifecycle_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_you_cast_source_shape(tokens)
        .or_else(|| parse_tagged_was_cast_shape(tokens))
        .or_else(|| parse_this_spell_was_cast_from_shape(tokens))
        .or_else(|| parse_no_spells_cast_last_turn_shape(tokens))
        .or_else(|| parse_this_spell_paid_named_label_shape(tokens))
        .or_else(|| parse_target_was_kicked_shape(tokens))
}

fn is_cast_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["cast"])
}

fn is_source_spell_object_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(clause, &[&["it"], &["this", "spell"]])
}

fn is_tagged_cast_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(
        clause,
        &[
            &["it"],
            &["that", "creature"],
            &["that", "permanent"],
            &["that", "object"],
        ],
    )
}

fn is_was_cast_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["was", "cast"])
}

fn is_this_spell_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact(clause, &["this", "spell"])
}

fn is_was_cast_from_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["was", "cast", "from"])
}

fn spell_cast_origin_zone_clause(clause: LexedClause<'_>) -> Option<Zone> {
    if surface::exact(clause, &["anywhere", "other", "than", "your", "hand"]) {
        return None;
    }
    let words = clause.word_refs();
    let words = if words
        .first()
        .is_some_and(|word| is_article(word) || *word == DEFINITE_ARTICLE_WORD)
    {
        &words[1..]
    } else {
        words.as_slice()
    };
    (words.len() == 1)
        .then(|| parse_zone_word(words[0]))
        .flatten()
}

fn is_no_amount_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["no"])
}

fn is_spell_object_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["spell"], &["spells"]])
}

fn is_were_cast_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["was", "cast"], &["were", "cast"]])
}

fn is_last_turn_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["last", "turn"])
}

fn is_kicked_source_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(
        clause,
        &[
            &["this", "spell"],
            &["this", "creature"],
            &["this", "permanent"],
            &["it"],
        ],
    )
}

fn is_was_kicked_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["was", "kicked"])
}

fn is_bargained_source_clause(clause: LexedClause<'_>) -> bool {
    is_source_spell_object_clause(clause)
}

fn is_was_bargained_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["was", "bargained"])
}

fn is_that_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["that"]) || surface::exact(clause, &["that", "spell"])
}

fn parse_you_cast_source_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(&["cast"])),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("object", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_cast_action_clause(action_clause) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_source_spell_object_clause(object_clause) {
        return None;
    }
    Some(PredicateAst::SourceWasCast)
}

fn parse_tagged_was_cast_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(&["was", "cast"])),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_tagged_cast_subject_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_was_cast_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::TaggedWasCast(TagKey::from(IT_TAG)))
}

fn parse_this_spell_was_cast_from_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["was", "cast", "from"]),
        ),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(3)),
        WinnowSequence::object("origin", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_this_spell_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_was_cast_from_action_clause(action_clause) {
        return None;
    }
    let origin_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if surface::exact(
        origin_clause,
        &["anywhere", "other", "than", "your", "hand"],
    ) {
        return Some(PredicateAst::ThisSpellWasCastFromNonHand);
    }
    let zone = spell_cast_origin_zone_clause(origin_clause)?;
    Some(PredicateAst::ThisSpellWasCastFromZone(zone))
}

fn parse_no_spells_cast_last_turn_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::amount("amount", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("object", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let amount_clause = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    if !is_no_amount_clause(amount_clause) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_spell_object_clause(object_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_were_cast_action_clause(action_clause) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_last_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::NoSpellsWereCastLastTurn)
}

fn parse_this_spell_paid_named_label_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_this_spell_was_kicked_with_cost_shape(tokens)
        .or_else(|| parse_this_spell_was_kicked_shape(tokens))
        .or_else(|| parse_this_spell_was_bargained_shape(tokens))
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Gift", &["was", "promised"], false)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Gift", &["wasnt", "promised"], true)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Gift", &["wasn't", "promised"], true)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Gift", &["was", "not", "promised"], true)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Tribute", &["was", "paid"], false)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Tribute", &["wasnt", "paid"], true)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Tribute", &["wasn't", "paid"], true)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Tribute", &["was", "not", "paid"], true)
        })
        .or_else(|| parse_behold_spell_label_shape(tokens))
}

fn parse_this_spell_was_kicked_with_cost_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let was_idx = token_index_for_word(tokens, "was")?;
    if !tokens
        .get(was_idx + 1)
        .is_some_and(|token| token.is_word("kicked"))
        || !tokens
            .get(was_idx + 2)
            .is_some_and(|token| token.is_word("with"))
    {
        return None;
    }

    if !is_kicked_source_clause(LexedClause::new(&tokens[..was_idx])) {
        return None;
    }

    let mut cost_start = was_idx + 3;
    if tokens
        .get(cost_start)
        .is_some_and(|token| token.is_word("its") || token.is_word("their"))
    {
        cost_start += 1;
    }
    let kicker_idx = token_index_for_word_from(tokens, "kicker", cost_start)?;
    if kicker_idx + 1 != tokens.len() || cost_start >= kicker_idx {
        return None;
    }

    let parsed_cost = parse_activation_cost_tokens_rewrite(&tokens[cost_start..kicker_idx]).ok()?;
    let lowered_cost = lower_activation_cost_cst(&parsed_cost).ok()?;
    let cost_text = lowered_cost
        .mana_cost()
        .map(|cost| cost.to_oracle())
        .unwrap_or_else(|| lowered_cost.display());
    (!cost_text.is_empty())
        .then(|| PredicateAst::ThisSpellPaidLabel(format!("Kicker {cost_text}").into()))
}

fn parse_this_spell_was_kicked_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["was", "kicked"]),
        ),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_kicked_source_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_was_kicked_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::ThisSpellWasKicked)
}

fn parse_this_spell_was_bargained_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["was", "bargained"]),
        ),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_bargained_source_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_was_bargained_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::ThisSpellPaidLabel("Bargain".into()))
}

fn parse_named_spell_label_action_shape(
    tokens: &[OwnedLexToken],
    label: &str,
    action_phrase: &[&str],
    negated: bool,
) -> Option<PredicateAst> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let mut input: primitives::WordSliceInput<'_> = words.as_slice();
    if input
        .first()
        .is_some_and(|word| matches!(*word, "the" | "a" | "an"))
    {
        input = &input[1..];
    }
    let (actual_label, rest) = input.split_first()?;
    if !actual_label.eq_ignore_ascii_case(label) {
        return None;
    }
    input = rest;
    for expected in action_phrase {
        let (actual, rest) = input.split_first()?;
        if actual != expected {
            return None;
        }
        input = rest;
    }
    if !input.is_empty() {
        return None;
    }
    let predicate = PredicateAst::ThisSpellPaidLabel(label.into());
    if negated {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

fn parse_behold_spell_label_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["was", "beheld"], &["beheld"]];
    let atoms = [
        WinnowSequence::object("subtype", WinnowCaptureKind::UntilAnyPhrase(action_phrases)),
        WinnowSequence::any_phrase(action_phrases),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subtype_clause = matched.capture_clause("subtype", clause)?;
    let subtype_tokens = strip_leading_article_tokens(subtype_clause.tokens());
    let subtype_words = LexedClause::new(subtype_tokens).word_refs();
    if subtype_words.len() != 1 || parse_subtype_word(subtype_words[0]).is_none() {
        return None;
    }
    Some(PredicateAst::ThisSpellPaidLabel("Behold".into()))
}

fn parse_target_was_kicked_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["was", "kicked"]),
        ),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_that_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_was_kicked_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::TargetWasKicked)
}

fn parse_mana_spent_capture_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_no_mana_spent_to_cast_shape(tokens)
        .or_else(|| parse_no_colored_mana_spent_to_cast_shape(tokens))
        .or_else(|| parse_snow_mana_of_any_spell_color_spent_to_cast_shape(tokens))
        .or_else(|| parse_mana_symbol_spent_to_cast_shape(tokens))
        .or_else(|| {
            parse_same_color_mana_spent_to_cast_predicate(tokens)
                .map(|amount| PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(amount))
        })
        .or_else(|| {
            parse_mana_spent_to_cast_predicate(tokens).map(|(amount, symbol)| {
                PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol }
            })
        })
}

fn parse_no_mana_spent_to_cast_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    if !surface::exact_any(
        clause,
        &[
            &["no", "mana", "was", "spent", "to", "cast", "it"],
            &["no", "mana", "were", "spent", "to", "cast", "it"],
            &["no", "mana", "was", "spent", "to", "cast", "this", "spell"],
            &["no", "mana", "were", "spent", "to", "cast", "this", "spell"],
            &["no", "mana", "was", "spent", "to", "cast", "that", "spell"],
            &["no", "mana", "were", "spent", "to", "cast", "that", "spell"],
        ],
    ) {
        return None;
    }
    Some(PredicateAst::Not(Box::new(
        PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: None,
        },
    )))
}

fn parse_no_colored_mana_spent_to_cast_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    if !surface::exact_any(
        clause,
        &[
            &["no", "colored", "mana", "was", "spent", "to", "cast", "it"],
            &["no", "colored", "mana", "were", "spent", "to", "cast", "it"],
            &[
                "no", "colored", "mana", "was", "spent", "to", "cast", "this", "spell",
            ],
            &[
                "no", "colored", "mana", "were", "spent", "to", "cast", "this", "spell",
            ],
            &[
                "no", "colored", "mana", "was", "spent", "to", "cast", "that", "spell",
            ],
            &[
                "no", "colored", "mana", "were", "spent", "to", "cast", "that", "spell",
            ],
        ],
    ) {
        return None;
    }
    Some(PredicateAst::Not(Box::new(
        PredicateAst::ColoredManaSpentToCastThisSpellAtLeast(1),
    )))
}

fn parse_snow_mana_of_any_spell_color_spent_to_cast_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let first = tokens.first()?;
    let symbol = parse_mana_symbol(first.parser_text()).ok()?;
    if symbol != crate::mana::ManaSymbol::Snow {
        return None;
    }

    let clause = LexedClause::new(&tokens[1..]);
    surface::exact_any(
        clause,
        &[
            &[
                "of", "any", "of", "that", "spell", "colors", "was", "spent", "to", "cast", "it",
            ],
            &[
                "of", "any", "of", "that", "spells", "colors", "was", "spent", "to", "cast", "it",
            ],
            &[
                "of", "any", "of", "that", "spell's", "colors", "was", "spent", "to", "cast", "it",
            ],
        ],
    )
    .then_some(PredicateAst::SnowManaOfAnySpellColorSpentToCastThisSpell)
}

fn parse_mana_symbol_spent_to_cast_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::amount(
            "symbols",
            WinnowCaptureKind::UntilAnyPhrase(MANA_SPENT_TO_CAST_THIS_SPELL_PHRASES),
        ),
        WinnowSequence::any_phrase(MANA_SPENT_TO_CAST_THIS_SPELL_PHRASES),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let symbol_clause = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let validation_words = mana_spent_symbol_clause_words(symbol_clause);
    if validation_words.is_empty()
        || !validation_words
            .iter()
            .all(|word| word_is_any(word, MANA_SYMBOL_WORDS))
    {
        return None;
    }
    let mut predicates = symbol_clause
        .tokens()
        .iter()
        .filter_map(|token| parse_mana_symbol(token.parser_text()).ok())
        .map(|symbol| PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(symbol),
        });
    let first = predicates.next()?;
    Some(predicates.fold(first, |left, right| {
        PredicateAst::And(Box::new(left), Box::new(right))
    }))
}

fn parse_attached_tagged_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_this_permanent_attached_to_shape(tokens)
}

fn parse_this_permanent_attached_to_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["attached", "to"], &["is", "attached", "to"]];
    for action_phrase in action_phrases {
        let atoms = [
            WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
            WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
            WinnowSequence::object("attached_to", WinnowCaptureKind::Rest),
        ];
        let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
        if !is_this_or_that_permanent_clause(subject_clause) {
            continue;
        }
        let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
        let mut filter = parse_object_filter(object_clause.tokens(), false).ok()?;
        if filter.card_types.is_empty() {
            filter.card_types.push(CardType::Creature);
        }
        return Some(PredicateAst::TaggedMatches(
            TagKey::from("enchanted"),
            filter,
        ));
    }
    None
}

fn is_this_or_that_permanent_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["this", "permanent"],
            &["that", "permanent"],
            &["this", "equipment"],
            &["that", "equipment"],
        ],
    )
}

fn is_tagged_exiled_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(
        clause,
        &[
            &["any", "of", "those", "cards"],
            &["those", "cards"],
            &["that", "card"],
            &["it"],
        ],
    )
}

fn is_exiled_zone_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["exiled"])
}

fn is_that_permanent_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["that", "permanent"])
}

fn is_tagged_entered_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(
        clause,
        &[&["it"], &["that", "card"], &["that", "permanent"]],
    )
}

fn is_your_control_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["your", "control"])
}

fn is_tagged_creature_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(clause, &[&["it"], &["that", "creature"]])
}

fn is_blocking_state_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["blocking"])
}

fn is_soulbond_partner_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(clause, &[&["creature"], &["another", "creature"]])
}

fn tagged_creature_role_clause(clause: LexedClause<'_>) -> Option<&'static str> {
    if surface::exact(clause, &["equipped", "creature"]) {
        return Some("equipped");
    }
    if surface::exact(clause, &["enchanted", "creature"]) {
        return Some("enchanted");
    }
    None
}

fn parse_sacrificed_permanent_state_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let optional_article = [WinnowSequence::any_word(&["a", "an", "the"])];
    let atoms = [
        WinnowSequence::optional(&optional_article),
        WinnowSequence::word("sacrificed"),
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::word("was"),
        WinnowSequence::modifier("descriptor", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let subject = matched
        .capture_clause_by_role(WinnowCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing subject in sacrificed predicate".to_string())
        })?;
    let Some(subject_token) = subject.token(0) else {
        return Ok(None);
    };
    let subject_card_type = parse_card_type(subject_token.parser_text())
        .filter(|card_type| is_permanent_type(*card_type));
    let subject_is_permanent =
        token_word_is(subject_token, PERMANENT_WORD) || subject_card_type.is_some();
    if !subject_is_permanent {
        return Ok(None);
    }

    let descriptor = matched
        .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing descriptor in sacrificed predicate".to_string())
        })?;
    if descriptor.tokens().is_empty() {
        return Ok(None);
    }
    let mut filter = match parse_object_filter(descriptor.tokens(), false) {
        Ok(filter) => filter,
        Err(err) => parse_color_only_object_filter_clause(descriptor).ok_or(err)?,
    };
    if filter.card_types.is_empty()
        && let Some(card_type) = subject_card_type
    {
        filter.card_types.push(card_type);
    }
    if filter.zone.is_none() && token_word_is(subject_token, PERMANENT_WORD) {
        filter.zone = Some(Zone::Battlefield);
    }
    Ok(Some(PredicateAst::ItMatches(filter)))
}

fn parse_tagged_exiled_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["remain"], &["remains"]];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilAnyPhrase(action_phrases)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("zone", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_tagged_exiled_subject_clause(subject_clause) {
        return None;
    }
    let zone_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_exiled_zone_clause(zone_clause) {
        return None;
    }
    Some(PredicateAst::TaggedMatches(
        TagKey::from(IT_TAG),
        ObjectFilter::default().in_zone(Zone::Exile),
    ))
}

fn parse_tagged_state_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_tagged_controlled_permanent_shape(tokens)
        .or_else(|| parse_tagged_entered_under_your_control_shape(tokens))
        .or_else(|| parse_tagged_wasnt_blocking_shape(tokens))
        .or_else(|| parse_implicit_object_present_state_shape(tokens))
        .or_else(|| parse_implicit_object_bare_state_shape(tokens))
        .or_else(|| parse_tagged_historical_identity_shape(tokens))
        .or_else(|| parse_it_soulbond_paired_shape(tokens))
        .or_else(|| parse_tagged_creature_filter_shape(tokens))
}

fn parse_tagged_controlled_permanent_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_control_or_controlled_relation_clauses(tokens)?;
    if !is_you_clause(relation.subject_clause) {
        return None;
    }
    if !is_that_permanent_clause(relation.tail_clause) {
        return None;
    }
    Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::You,
        tag: TagKey::from(IT_TAG),
        filter: ObjectFilter::default(),
    })
}

fn parse_tagged_entered_under_your_control_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrase = &["entered", "under"];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
        WinnowSequence::object("controller", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_tagged_entered_subject_clause(subject_clause) {
        return None;
    }
    let controller_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_your_control_clause(controller_clause) {
        return None;
    }
    Some(PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn {
        player: PlayerAst::You,
        tag: TagKey::from(IT_TAG),
    })
}

fn parse_tagged_wasnt_blocking_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["wasnt"], &["wasn't"], &["was", "not"]];
    for action_phrase in action_phrases {
        let atoms = [
            WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
            WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
            WinnowSequence::object("state", WinnowCaptureKind::Rest),
        ];
        let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
        if !is_tagged_creature_subject_clause(subject_clause) {
            continue;
        }
        let state_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
        if !is_blocking_state_clause(state_clause) {
            continue;
        }
        return Some(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter {
                nonblocking: true,
                ..Default::default()
            },
        ));
    }
    None
}

fn is_implicit_object_state_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(
        clause,
        &[
            &["it"],
            &["its"],
            &["that", "card"],
            &["that", "creature"],
            &["that", "object"],
            &["that", "permanent"],
            &["that", "spell"],
        ],
    )
}

fn object_filter_has_identity_or_state(filter: &ObjectFilter) -> bool {
    object_filter_has_identity(filter) || object_filter_has_state(filter)
}

fn object_filter_has_state(filter: &ObjectFilter) -> bool {
    filter.tapped
        || filter.untapped
        || filter.attacking
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
}

fn implicit_object_state_predicate_from_filter(
    filter: ObjectFilter,
    negative: bool,
) -> Option<PredicateAst> {
    if !object_filter_has_identity_or_state(&filter) {
        return None;
    }
    let predicate = PredicateAst::ItMatches(filter);
    Some(if negative {
        PredicateAst::Not(Box::new(predicate))
    } else {
        predicate
    })
}

fn parse_implicit_object_present_state_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
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
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilAnyPhrase(state_phrases)),
        WinnowSequence::action(
            "state",
            WinnowCaptureKind::OneOf(&["is", "are", "isnt", "isn't", "arent", "aren't"]),
        ),
        WinnowSequence::object("descriptor", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_implicit_object_state_subject_clause(subject_clause) {
        return None;
    }
    let subject_is_bare_pronoun = surface::exact_any(subject_clause, &[&["it"], &["its"]]);
    let action = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    let mut negative = source_identity_copula_is_negative(action);
    let descriptor_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let (descriptor_negative, descriptor_clause) =
        parse_source_identity_descriptor_clause(descriptor_clause)?;
    negative |= descriptor_negative;
    if descriptor_clause.tokens().is_empty()
        || source_identity_descriptor_contains_ignored_state(descriptor_clause)
    {
        return None;
    }
    let filter = parse_object_filter(descriptor_clause.tokens(), false)
        .ok()
        .or_else(|| parse_color_only_object_filter_word_refs(descriptor_clause))
        .or_else(|| parse_identity_descriptor_filter_tokens(descriptor_clause.tokens()))?;
    if subject_is_bare_pronoun && !object_filter_has_state(&filter) {
        return None;
    }
    implicit_object_state_predicate_from_filter(filter, negative)
}

fn parse_implicit_object_bare_state_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let state_words = &["attacking", "blocking", "tapped", "untapped"];
    let state_phrases: &[&[&str]] = &[&["attacking"], &["blocking"], &["tapped"], &["untapped"]];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilAnyPhrase(state_phrases)),
        WinnowSequence::object("state", WinnowCaptureKind::OneOf(state_words)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_implicit_object_state_subject_clause(subject_clause) {
        return None;
    }
    let state_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let filter = parse_object_filter(state_clause.tokens(), false).ok()?;
    implicit_object_state_predicate_from_filter(filter, false)
}

fn parse_tagged_historical_identity_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["was"], &["were"]];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilAnyPhrase(action_phrases)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("descriptor", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_tagged_identity_subject_clause(subject_clause) {
        return None;
    }
    let descriptor_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let (negative, descriptor_clause) = parse_source_identity_descriptor_clause(descriptor_clause)?;
    if negative
        || descriptor_clause.tokens().is_empty()
        || source_identity_descriptor_contains_ignored_state(descriptor_clause)
    {
        return None;
    }
    let filter = parse_object_filter(descriptor_clause.tokens(), false)
        .ok()
        .or_else(|| parse_color_only_object_filter_word_refs(descriptor_clause))
        .or_else(|| parse_identity_descriptor_filter_tokens(descriptor_clause.tokens()))?;
    if !object_filter_has_identity(&filter) {
        return None;
    }
    Some(PredicateAst::TaggedMatches(TagKey::from(IT_TAG), filter))
}

fn is_tagged_identity_subject_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["it"],
            &["that", "card"],
            &["that", "creature"],
            &["that", "permanent"],
        ],
    )
}

fn parse_it_soulbond_paired_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["paired", "with"], &["is", "paired", "with"]];
    for action_phrase in action_phrases {
        let atoms = [
            WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
            WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
            WinnowSequence::object("partner", WinnowCaptureKind::Rest),
        ];
        let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
        if !surface::exact_any(subject_clause, &[&["it"], &["its"], &["it's"]]) {
            continue;
        }
        let partner_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
        if !is_soulbond_partner_clause(partner_clause) {
            continue;
        }
        return Some(PredicateAst::ItIsSoulbondPaired);
    }
    None
}

fn parse_tagged_creature_filter_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("tagged_subject", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::object("filter", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let tagged_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    let tag = tagged_creature_role_clause(tagged_clause)?;
    let filter_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let mut filter = parse_object_filter(filter_clause.tokens(), false).ok()?;
    if filter.card_types.is_empty() {
        filter.card_types.push(CardType::Creature);
    }
    Some(PredicateAst::TaggedMatches(TagKey::from(tag), filter))
}

fn graveyard_possessive_matches_subject(player: PlayerAst, possessive: LexedClause<'_>) -> bool {
    let Some(token) = possessive.token(0) else {
        return false;
    };
    match player {
        PlayerAst::You | PlayerAst::Implicit => token_word_is(token, YOUR_WORD),
        _ => token_word_is(token, THEIR_WORD),
    }
}

fn comparison_player_subject_clause(clause: LexedClause<'_>) -> Option<PlayerAst> {
    let word_len = clause.word_len();
    if word_len == 2 && surface::exact(clause, THAT_PLAYER_SUBJECT_PREFIX) {
        Some(PlayerAst::That)
    } else if word_len == 2 && surface::exact(clause, TARGET_PLAYER_SUBJECT_PREFIX) {
        Some(PlayerAst::Target)
    } else if word_len == 2 && surface::exact(clause, TARGET_OPPONENT_SUBJECT_PREFIX) {
        Some(PlayerAst::TargetOpponent)
    } else if word_len == 2 && surface::exact(clause, EACH_OPPONENT_SUBJECT_PREFIX) {
        Some(PlayerAst::Opponent)
    } else if word_len == 2 && surface::exact_any(clause, A_OR_ANY_PLAYER_SUBJECT_PREFIXES) {
        Some(PlayerAst::Any)
    } else if word_len == 2 && surface::exact(clause, DEFENDING_PLAYER_SUBJECT_PREFIX) {
        Some(PlayerAst::Defending)
    } else if word_len == 2 && surface::exact(clause, ATTACKING_PLAYER_SUBJECT_PREFIX) {
        Some(PlayerAst::Attacking)
    } else if word_len == 1
        && clause
            .token(0)
            .is_some_and(|token| token_word_is(token, YOU_WORD))
    {
        Some(PlayerAst::You)
    } else if surface::exact_any(clause, AN_OR_THE_OPPONENT_SUBJECT_PHRASES) {
        Some(PlayerAst::Opponent)
    } else if word_len == 1 && surface::exact_any(clause, OPPONENT_SUBJECT_PREFIXES) {
        Some(PlayerAst::Opponent)
    } else if word_len == 1
        && clause
            .token(0)
            .is_some_and(|token| token_word_is(token, PLAYER_SUBJECT_WORD))
    {
        Some(PlayerAst::Any)
    } else {
        None
    }
}

fn parse_player_cards_in_graveyard_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let card_in_phrases: &[&[&str]] = &[&["card", "in"], &["cards", "in"]];
    let atoms = [
        WinnowSequence::amount(
            "quantity",
            WinnowCaptureKind::UntilAnyPhrase(card_in_phrases),
        ),
        WinnowSequence::any_phrase(card_in_phrases),
        WinnowSequence::modifier("possessive", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("zone", WinnowCaptureKind::OneOf(&["graveyard"])),
    ];
    let relation = parse_has_relation_clauses(tokens)?;
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let player = comparison_player_subject_clause(relation.subject_clause)?;
    let quantity =
        matched.capture_clause_by_role(WinnowCaptureRole::Amount, relation.tail_clause)?;
    let (comparison, used) = predicate_quantity_prefix_tokens(quantity.tokens())?;
    if used != quantity.tokens().len() {
        return None;
    }
    let (operator, count) = comparison_to_value_comparison_operator(comparison)?;
    let possessive = matched.capture_clause("possessive", relation.tail_clause)?;
    if !graveyard_possessive_matches_subject(player, possessive) {
        return None;
    }
    let player_filter = player_filter_for_turn_value(player)?;

    Some(PredicateAst::ValueComparison {
        left: Value::CardsInGraveyard(player_filter),
        operator,
        right: Value::Fixed(count),
    })
}

fn parse_quantified_objects_in_graveyard_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = parse_prepositional_copula_relation_clauses(tokens, &["in"])?;
    if !surface::exact(relation.preposition_clause, &["in"])
        || !is_graveyard_location_clause(relation.tail_clause)
    {
        return None;
    }

    let subject_tokens = relation.subject_clause.tokens();
    let (comparison, used) = predicate_quantity_prefix_tokens(subject_tokens)?;
    if used >= subject_tokens.len() {
        return None;
    }

    let descriptor_tokens = &subject_tokens[used..];
    let mut filter = parse_object_filter(descriptor_tokens, false)
        .ok()
        .or_else(|| {
            descriptor_tokens
                .last()
                .filter(|token| token_word_is_any(token, CARD_OR_CARDS_WORDS))
                .and_then(|_| {
                    let trimmed = &descriptor_tokens[..descriptor_tokens.len().saturating_sub(1)];
                    parse_object_filter(trimmed, false).ok()
                })
        })?;
    filter.zone = Some(Zone::Graveyard);
    if surface::exact(relation.tail_clause, &["your", "graveyard"]) {
        filter.owner = Some(PlayerFilter::You);
    }

    let (operator, count) = comparison_to_value_comparison_operator(comparison)?;
    Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator,
        right: Value::Fixed(count),
    })
}

fn parse_player_controls_more_than_you_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let atoms = [
        WinnowSequence::amount("comparison", WinnowCaptureKind::OneOf(&["more"])),
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["than"])),
        WinnowSequence::word("than"),
        WinnowSequence::modifier("comparison_player", WinnowCaptureKind::Rest),
    ];
    let relation = parse_control_relation_clauses(tokens, false)?;
    let subject = relation.subject_clause;
    let player = comparison_player_subject_clause(subject)?;
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let tail = matched.capture_clause("comparison_player", relation.tail_clause)?;
    if !is_you_comparison_tail_clause(tail) {
        return None;
    }
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, relation.tail_clause)?;
    if object.tokens().is_empty() {
        return None;
    }
    let other = object
        .tokens()
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let filter = parse_object_filter(object.tokens(), other).ok()?;
    if filter == ObjectFilter::default() {
        return None;
    }

    Some(PredicateAst::PlayerControlsMoreThanYou { player, filter })
}

fn parse_player_controls_more_than_each_other_player_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let atoms = [
        WinnowSequence::amount("comparison", WinnowCaptureKind::OneOf(&["more"])),
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["than"])),
        WinnowSequence::word("than"),
        WinnowSequence::modifier("comparison_player", WinnowCaptureKind::Rest),
    ];
    let relation = parse_control_relation_clauses(tokens, false)?;
    let subject = relation.subject_clause;
    let player = comparison_player_subject_clause(subject)?;
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let tail = matched.capture_clause("comparison_player", relation.tail_clause)?;
    if !surface::exact_any(
        tail,
        &[&["each", "other", "player"], &["each", "other", "players"]],
    ) {
        return None;
    }
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, relation.tail_clause)?;
    if object.tokens().is_empty() {
        return None;
    }
    let other = object
        .tokens()
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let filter = parse_object_filter(object.tokens(), other).ok()?;
    if filter == ObjectFilter::default() {
        return None;
    }

    Some(PredicateAst::PlayerControlsMoreThanEachOtherPlayer { player, filter })
}

fn parse_opponent_controls_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_control_relation_clauses(tokens, false)?;
    if !is_opponent_controller_clause(relation.subject_clause) {
        return None;
    }
    let object = relation.tail_clause;
    if object_starts_with_more_than_clause(object) {
        return None;
    }
    if object.tokens().is_empty() {
        return None;
    }
    let other = object
        .tokens()
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let mut filter = parse_object_filter(object.tokens(), other).ok()?;
    filter.controller = Some(PlayerFilter::Opponent);
    filter.zone = None;

    Some(PredicateAst::PlayerControls {
        player: PlayerAst::Opponent,
        filter,
    })
}

fn object_starts_with_more_than_clause(clause: LexedClause<'_>) -> bool {
    let Some(first) = clause.token(0) else {
        return false;
    };
    token_word_is(first, MORE_WORD)
        && clause
            .tokens()
            .iter()
            .skip(1)
            .any(|token| token_word_is(token, THAN_WORD))
}

fn is_you_comparison_tail_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["you"], &["you", "do"]])
}

fn parse_keyword_subject_object_filter_tokens(
    object_tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let object_tokens = strip_leading_article_tokens(object_tokens);
    if non_article_token_words_eq_any(object_tokens, NONLAND_CARD_OBJECT_PHRASES) {
        let mut filter = ObjectFilter::default();
        filter.excluded_card_types.push(CardType::Land);
        return Ok(filter);
    }

    let normalized_tokens;
    let object_tokens = if object_tokens
        .last()
        .is_some_and(|token| token.parser_text() == "cards")
    {
        normalized_tokens = {
            let mut tokens = object_tokens.to_vec();
            if let Some(last) = tokens.last_mut() {
                *last = OwnedLexToken::synthetic_word("card");
            }
            tokens
        };
        normalized_tokens.as_slice()
    } else {
        object_tokens
    };
    parse_object_filter(object_tokens, false).or_else(|_| {
        let trimmed = if object_tokens
            .last()
            .is_some_and(|token| token_word_is_any(token, CARD_OR_CARDS_WORDS))
        {
            &object_tokens[..object_tokens.len().saturating_sub(1)]
        } else {
            object_tokens
        };
        parse_object_filter(trimmed, false)
    })
}

fn parse_graveyard_escape_keyword_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    const IN_YOUR_GRAVEYARD_PHRASE: &[&str] = &["in", "your", "graveyard"];
    const GRAVEYARD_SUBJECT_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::object(
            "object",
            WinnowCaptureKind::UntilPhrase(IN_YOUR_GRAVEYARD_PHRASE),
        ),
        WinnowSequence::phrase(IN_YOUR_GRAVEYARD_PHRASE),
    ]);

    let Some(relation) = parse_has_relation_clauses(tokens) else {
        return Ok(None);
    };
    if !surface::exact(relation.tail_clause, &["escape"]) {
        return Ok(None);
    }
    let Some(matched) = GRAVEYARD_SUBJECT_PATTERN.parse_full(relation.subject_clause) else {
        return Ok(None);
    };
    let object = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, relation.subject_clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in escape predicate".to_string())
        })?;
    if object.tokens().is_empty() {
        return Ok(None);
    }

    let mut filter = parse_keyword_subject_object_filter_tokens(object.tokens())?;
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);
    filter.alternative_cast = Some(crate::filter::AlternativeCastKind::Escape);
    Ok(Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter,
    }))
}

fn parse_player_object_keyword_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    if let Some(predicate) = parse_graveyard_escape_keyword_predicate(tokens)? {
        return Ok(Some(predicate));
    }

    let Some(relation) = parse_has_relation_clauses(tokens) else {
        return Ok(None);
    };
    let subject = relation.subject_clause;
    let keyword = relation.tail_clause;
    let Some((constraint, consumed)) = parse_filter_keyword_constraint_tokens(keyword.tokens())
    else {
        return Ok(None);
    };
    if consumed != keyword.tokens().len() {
        return Ok(None);
    }

    let subject_has_control = subject
        .tokens()
        .iter()
        .any(|token| token_word_is(token, CONTROL_WORD));
    let subject_has_zone = subject
        .tokens()
        .iter()
        .any(|token| token_word_is_any(token, ZONE_WORDS));
    let mut filter = if subject_has_control {
        let object_tokens = subject
            .tokens()
            .iter()
            .filter(|token| {
                !token_word_is(token, YOU_WORD)
                    && !token_word_is_any(token, CONTROL_OR_CONTROLS_WORDS)
            })
            .cloned()
            .collect::<Vec<_>>();
        if object_tokens.is_empty() {
            return Ok(None);
        }
        let mut filter = parse_object_filter(&object_tokens, false)?;
        filter.controller = Some(PlayerFilter::You);
        filter
    } else if subject_has_zone {
        if let Ok(mut filter) = parse_object_filter(subject.tokens(), false) {
            if filter.owner.is_none() {
                filter.owner = Some(PlayerFilter::You);
            }
            filter
        } else if let Some(filter) = parse_keyword_subject_object_in_zone_filter(subject.tokens())?
        {
            filter
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    apply_filter_keyword_constraint(&mut filter, constraint, false);
    Ok(Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter,
    }))
}

fn parse_keyword_subject_object_in_zone_filter(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    const OBJECT_IN_ZONE_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["in"])),
        WinnowSequence::word("in"),
        WinnowSequence::modifier("zone", WinnowCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(subject_tokens);
    let Some(matched) = OBJECT_IN_ZONE_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let object = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in keyword-zone predicate".to_string())
        })?;
    let zone = matched
        .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing zone in keyword-zone predicate".to_string())
        })?;
    if object.tokens().is_empty() || zone.tokens().is_empty() {
        return Ok(None);
    }
    let Ok(mut filter) = parse_keyword_subject_object_filter_tokens(object.tokens()) else {
        return Ok(None);
    };
    if is_your_graveyard_clause(zone) {
        filter.zone = Some(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
    } else {
        return Ok(None);
    }
    Ok(Some(filter))
}

fn is_your_graveyard_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["your", "graveyard"])
}

fn is_there_are_or_were_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["there", "are"], &["there", "were"]])
}

fn permanents_you_control_scope(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    if surface::exact_any(clause, PERMANENTS_YOU_CONTROL_SCOPE_PHRASES) {
        return Some(ObjectFilter::permanent().you_control());
    }
    None
}

fn cards_in_your_graveyard_scope(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    if surface::exact_any(clause, CARDS_IN_YOUR_GRAVEYARD_SCOPE_PHRASES) {
        return Some(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
    }
    None
}

fn permanents_and_your_graveyard_scope(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    let word_len = clause.word_len();
    let battlefield_end = (3..=word_len.min(4)).find(|end| {
        clause
            .between_word_range(0, *end)
            .and_then(permanents_you_control_scope)
            .is_some()
    })?;
    let connector_tail = clause.between_word_range(battlefield_end, battlefield_end + 1);
    let split_tail = clause.between_word_range(battlefield_end, battlefield_end + 2);
    let connector_end = if connector_tail
        .is_some_and(|tail| surface::exact(tail, PERMANENTS_AND_OR_GRAVEYARD_CONNECTOR_PHRASE))
    {
        battlefield_end + 1
    } else if split_tail
        .is_some_and(|tail| surface::exact(tail, PERMANENTS_AND_OR_SPLIT_CONNECTOR_PHRASE))
    {
        battlefield_end + 2
    } else {
        return None;
    };
    let battlefield = permanents_you_control_scope(clause.between_word_range(0, battlefield_end)?)?;
    let graveyard =
        cards_in_your_graveyard_scope(clause.between_word_range(connector_end, word_len)?)?;
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![battlefield, graveyard];
    Some(filter)
}

fn parse_colors_among_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::amount(
            "quantity",
            WinnowCaptureKind::UntilAnyPhrase(&[&["color"], &["colors"]]),
        ),
        WinnowSequence::object("unit", WinnowCaptureKind::OneOf(&["color", "colors"])),
        WinnowSequence::word("among"),
        WinnowSequence::modifier("scope", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let existential = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_there_are_or_were_clause(existential) {
        return None;
    }

    let quantity = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let (count, used) = parse_number(quantity.tokens())?;
    if used != quantity.tokens().len() {
        return None;
    }

    let scope = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    let filter = permanents_you_control_scope(scope)?;
    Some(PredicateAst::ValueComparison {
        left: Value::ColorsAmong(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    })
}

fn parse_card_types_among_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let card_type_phrases: &[&[&str]] = &[
        &["card", "type"],
        &["card", "types"],
        &["cards", "type"],
        &["cards", "types"],
    ];
    let atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::amount(
            "quantity",
            WinnowCaptureKind::UntilAnyPhrase(card_type_phrases),
        ),
        WinnowSequence::any_phrase(card_type_phrases),
        WinnowSequence::word("among"),
        WinnowSequence::modifier("scope", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let existential = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_there_are_or_were_clause(existential) {
        return None;
    }

    let quantity = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let (count, used) = predicate_at_least_quantity_prefix_tokens(quantity.tokens())?;
    if used != quantity.tokens().len() {
        return None;
    }

    let scope = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    let filter = if surface::exact_any(scope, SACRIFICED_PERMANENTS_SCOPE_PHRASES) {
        ObjectFilter::tagged("sacrificed_0")
    } else {
        permanents_and_your_graveyard_scope(scope)?
    };

    Some(PredicateAst::ValueComparison {
        left: Value::CardTypesAmong(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    })
}

fn parse_life_total_at_least_starting_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    if non_article_token_words_eq_phrase(tokens, LIFE_TOTAL_AT_LEAST_STARTING_PHRASE) {
        return Some(PredicateAst::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::StartingLifeTotal(PlayerFilter::You),
        });
    }
    None
}

fn parse_life_total_at_least_last_noted_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    if !non_article_token_words_eq_any(tokens, LIFE_TOTAL_AT_LEAST_LAST_NOTED_PHRASES) {
        return None;
    }
    Some(PredicateAst::ValueComparison {
        left: Value::LifeTotal(PlayerFilter::You),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::LastNotedLifeTotal,
    })
}

fn parse_counted_objects_have_counter_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    let counted_object = relation.subject_clause;
    let (comparison, used) = predicate_quantity_prefix_tokens(counted_object.tokens())?;
    let count = comparison_to_strict_at_least_threshold(&comparison)?;
    if used >= counted_object.tokens().len() {
        return None;
    }

    let object_tokens = &counted_object.tokens()[used..];
    if object_tokens.is_empty() {
        return None;
    }
    let counter = relation.tail_clause;
    let (counter_constraint, consumed) = parse_counted_object_counter_constraint_clause(counter)?;
    if consumed != counter.tokens().len() {
        return None;
    }

    let other = object_tokens
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let mut filter = parse_object_filter(object_tokens, other).ok()?;
    filter.with_counter = Some(counter_constraint);
    if filter.zone.is_none()
        && filter.card_types.iter().any(|card_type| {
            matches!(
                card_type,
                CardType::Artifact
                    | CardType::Creature
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
                    | CardType::Battle
            )
        })
    {
        filter.zone = Some(Zone::Battlefield);
    }

    Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    })
}

fn parse_counted_object_counter_constraint_clause(
    clause: LexedClause<'_>,
) -> Option<(crate::filter::CounterConstraint, usize)> {
    if clause.tokens().is_empty() {
        return None;
    }
    let words = TokenWordView::new(clause.tokens());
    let constraint_words = words.word_refs();
    if let Some((counter_constraint, consumed_words)) =
        parse_filter_counter_constraint_words(&constraint_words)
    {
        let consumed_tokens = words.token_index_after_words(consumed_words)?;
        return Some((counter_constraint, consumed_tokens));
    }

    let counter_type = parse_counter_type_from_tokens(clause.tokens())?;
    Some((
        ironsmith_core::CounterConstraint::Typed(counter_type),
        clause.tokens().len(),
    ))
}

#[rustfmt::skip]
fn parse_counted_source_exiled_objects_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    let counted_object = relation.subject_clause;
    let (comparison, used) = predicate_quantity_prefix_tokens(counted_object.tokens())?;
    let (operator, count) = comparison_to_value_comparison_operator(comparison)?;
    if used >= counted_object.tokens().len() {
        return None;
    }

    let tail = relation.tail_clause;
    if !surface::prefix_any(tail, BEEN_EXILED_WITH_THIS_SOURCE_PREFIXES) {
        return None;
    }

    let object_tokens = &counted_object.tokens()[used..];
    let mut filter = if object_tokens
        .iter()
        .all(|token| token_word_is_any(token, CARD_OR_CARDS_WORDS))
    {
        ObjectFilter::default()
    } else {
        parse_object_filter(object_tokens, false).ok()?
    };
    filter.zone = Some(Zone::Exile);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator,
        right: Value::Fixed(count),
    })
}

fn parse_happily_style_conjoined_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let cleaned_tokens: Vec<OwnedLexToken> = tokens
        .iter()
        .filter(|token| token.kind != TokenKind::Comma)
        .cloned()
        .collect();
    let cleaned_clause = LexedClause::new(&cleaned_tokens);
    let words = cleaned_clause.word_refs();
    let second_there_idx = surface::find_words(&words[1..], THERE_ARE_PREFIX).map(|idx| idx + 1)?;
    let life_word_idx =
        surface::find_words(&words[second_there_idx + 1..], AND_YOUR_LIFE_TOTAL_PHRASE)
            .map(|idx| idx + second_there_idx + 1)?;
    let life_idx = cleaned_clause
        .words()
        .token_span_for_words(life_word_idx, life_word_idx + 1)?
        .start;

    let first_clause = cleaned_clause.between_word_range(0, second_there_idx)?;
    let second_clause = cleaned_clause.between_word_range(second_there_idx, life_word_idx)?;

    let first = parse_colors_among_predicate(first_clause.tokens())?;
    let second = parse_card_types_among_predicate(second_clause.tokens())?;
    let third = parse_life_total_at_least_starting_predicate(&cleaned_tokens[life_idx + 1..])?;

    Some(PredicateAst::And(
        Box::new(PredicateAst::And(Box::new(first), Box::new(second))),
        Box::new(third),
    ))
}

fn parse_revealed_or_controlled_subtype_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let suffix_phrase = &["as", "you", "cast", "this", "spell"];
    let atoms = [
        WinnowSequence::subject("revealer", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("reveal_action", WinnowCaptureKind::OneOf(&["revealed"])),
        WinnowSequence::object(
            "revealed_subtype",
            WinnowCaptureKind::UntilPhrase(&["card"]),
        ),
        WinnowSequence::word("card"),
        WinnowSequence::word("or"),
        WinnowSequence::action(
            "control_action",
            WinnowCaptureKind::OneOf(&["control", "controlled", "controls"]),
        ),
        WinnowSequence::object("controlled_subtype", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let revealer = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_you_clause(revealer) {
        return None;
    }

    let revealed_subtype = matched.capture_clause("revealed_subtype", clause)?;
    let controlled_subtype = matched.capture_clause("controlled_subtype", clause)?;
    let revealed_subtype = single_subtype_descriptor_clause(revealed_subtype, &[])?;
    let controlled_subtype = single_subtype_descriptor_clause(controlled_subtype, suffix_phrase)?;
    let revealed_token = revealed_subtype.token(0)?;
    let controlled_token = controlled_subtype.token(0)?;
    if revealed_token.parser_text() != controlled_token.parser_text() {
        return None;
    }
    let subtype = parse_subtype_word(revealed_token.parser_text())?;

    Some(PredicateAst::Or(
        Box::new(PredicateAst::ThisSpellPaidLabel("Behold".into())),
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: ObjectFilter::default().with_subtype(subtype),
        }),
    ))
}

fn single_subtype_descriptor_clause<'a>(
    clause: LexedClause<'a>,
    optional_suffix: &[&str],
) -> Option<LexedClause<'a>> {
    let mut tokens = clause.trimmed().tokens();
    if !optional_suffix.is_empty()
        && let Some(without_suffix) = primitives::strip_lexed_suffix_phrase(tokens, optional_suffix)
    {
        tokens = without_suffix;
    }
    let descriptor = strip_leading_article_tokens(tokens);
    if descriptor.len() != 1 {
        return None;
    }
    parse_subtype_word(descriptor[0].parser_text())?;
    Some(LexedClause::new(descriptor))
}

fn is_card_graveyard_existential_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["there", "is"], &["there", "are"]])
}

fn is_graveyard_location_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["your", "graveyard"],
            &["graveyard"],
            &["the", "graveyard"],
        ],
    )
}

fn parse_subtype_card_descriptor_clause(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    let descriptor_tokens = strip_leading_article_tokens(clause.trimmed().tokens());
    if descriptor_tokens.len() != 2
        || !token_word_is_any(&descriptor_tokens[1], CARD_OR_CARDS_WORDS)
    {
        return None;
    }

    let subtype = descriptor_tokens[0]
        .as_word()
        .and_then(parse_subtype_word)?;
    Some(ObjectFilter::default().with_subtype(subtype))
}

fn parse_card_in_your_graveyard_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::object("descriptor", WinnowCaptureKind::UntilPhrase(&["in"])),
        WinnowSequence::action("preposition", WinnowCaptureKind::OneOf(&["in"])),
        WinnowSequence::modifier("location", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let existential = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_card_graveyard_existential_clause(existential) {
        return None;
    }

    let location = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    if !is_graveyard_location_clause(location) {
        return None;
    }

    let descriptor = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if descriptor.tokens().is_empty() {
        return None;
    }
    let mut filter = parse_object_filter(descriptor.tokens(), false)
        .ok()
        .or_else(|| {
            descriptor
                .tokens()
                .last()
                .and_then(OwnedLexToken::as_word)
                .filter(|word| word_is_any(word, CARD_OR_CARDS_WORDS))
                .and_then(|_| {
                    let trimmed_tokens =
                        &descriptor.tokens()[..descriptor.tokens().len().saturating_sub(1)];
                    parse_object_filter(trimmed_tokens, false).ok()
                })
        })
        .or_else(|| parse_subtype_card_descriptor_clause(descriptor))?;
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);

    Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter,
    })
}

fn parse_object_on_battlefield_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(relation) = parse_prepositional_copula_relation_clauses(tokens, &["on"]) else {
        return Ok(None);
    };
    if !surface::exact(relation.preposition_clause, &["on"])
        || !is_battlefield_zone_clause(relation.tail_clause)
    {
        return Ok(None);
    }

    let object_clause = relation.subject_clause;
    let object_tokens = object_clause.tokens();
    if object_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = parse_object_filter(object_tokens, false)?;
    if filter.name.is_some()
        && let Some(name) = parse_named_object_filter_name_tail(object_tokens)
    {
        filter.name = Some(name);
    }
    filter.zone = Some(Zone::Battlefield);

    Ok(Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThan,
        right: Value::Fixed(0),
    }))
}

fn parse_named_object_filter_name_tail(tokens: &[OwnedLexToken]) -> Option<String> {
    const NAMED_OBJECT_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["named"])),
        WinnowSequence::word("named"),
        WinnowSequence::modifier("name", WinnowCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = NAMED_OBJECT_PATTERN.parse_full(clause)?;
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if object.tokens().is_empty() {
        return None;
    }
    let name = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    let name_words = name.word_refs();
    let name_end = find_name_clause_end(name_words.as_slice(), 0);
    let name = render_token_slice(name.between_words_trimmed(0, name_end).tokens())
        .trim()
        .to_ascii_lowercase();
    (!name.is_empty()).then_some(name)
}

fn graveyard_card_types_subject(clause: LexedClause<'_>) -> Option<PlayerAst> {
    if surface::exact(clause, YOUR_GRAVEYARD_PHRASE) {
        Some(PlayerAst::You)
    } else if surface::exact_any(clause, THAT_PLAYER_GRAVEYARD_PHRASES) {
        Some(PlayerAst::That)
    } else if surface::exact_any(clause, TARGET_PLAYER_GRAVEYARD_PHRASES) {
        Some(PlayerAst::Target)
    } else if surface::exact_any(clause, TARGET_OPPONENT_GRAVEYARD_PHRASES) {
        Some(PlayerAst::TargetOpponent)
    } else if surface::exact_any(clause, OPPONENT_GRAVEYARD_PHRASES) {
        Some(PlayerAst::Opponent)
    } else {
        None
    }
}

fn parse_card_types_in_graveyard_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let card_type_phrases: &[&[&str]] = &[
        &["card", "type", "among", "card", "in"],
        &["card", "type", "among", "cards", "in"],
        &["card", "types", "among", "card", "in"],
        &["card", "types", "among", "cards", "in"],
    ];
    let atoms = [
        WinnowSequence::subject("lead", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::amount(
            "quantity",
            WinnowCaptureKind::UntilAnyPhrase(card_type_phrases),
        ),
        WinnowSequence::any_phrase(card_type_phrases),
        WinnowSequence::modifier("graveyard", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let lead = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    let constrained_player = card_types_graveyard_lead_player_clause(lead)?;
    let quantity = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let (count, used) = predicate_at_least_quantity_prefix_tokens(quantity.tokens())?;
    if used != quantity.tokens().len() {
        return None;
    }
    let graveyard = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    let player = graveyard_card_types_subject(graveyard)?;
    if constrained_player.is_some_and(|expected| expected != player) {
        return None;
    }

    Some(PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count })
}

fn card_types_graveyard_lead_player_clause(clause: LexedClause<'_>) -> Option<Option<PlayerAst>> {
    if is_there_are_clause(clause) {
        return Some(None);
    }
    if surface::exact(clause, &["you", "have"]) {
        return Some(Some(PlayerAst::You));
    }
    None
}

fn parse_there_are_objects_on_battlefield_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::object(
            "counted_object",
            WinnowCaptureKind::UntilLastPhrase(&["on"]),
        ),
        WinnowSequence::action("preposition", WinnowCaptureKind::OneOf(&["on"])),
        WinnowSequence::modifier("location", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let existential = matched
        .capture_clause_by_role(WinnowCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing existential in battlefield count predicate".to_string(),
            )
        })?;
    if !is_there_are_clause(existential) {
        return Ok(None);
    }
    let location = matched
        .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing location in battlefield count predicate".to_string())
        })?;
    if !is_battlefield_zone_clause(location) {
        return Ok(None);
    }

    let counted_object = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in battlefield count predicate".to_string())
        })?;
    let Some((count, used)) = predicate_at_least_quantity_prefix_tokens(counted_object.tokens())
    else {
        return Ok(None);
    };
    let object_tokens = counted_object.tokens().get(used..).unwrap_or_default();
    let other = object_tokens
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let filter_tokens = if other {
        object_tokens.get(1..).unwrap_or_default()
    } else {
        object_tokens
    };
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_object_filter(filter_tokens, other)?;
    filter.zone = Some(Zone::Battlefield);

    Ok(Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    }))
}

fn parse_exploited_triggering_object_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("action", WinnowCaptureKind::OneOf(&["exploited"])),
        WinnowSequence::object("object", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !surface::exact(subject, &["it"]) {
        return None;
    }
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !surface::exact_any(object, &[&["that", "creature"], &["that", "object"]]) {
        return None;
    }
    Some(PredicateAst::And(
        Box::new(PredicateAst::TaggedMatches(
            TagKey::from(crate::tag::EXPLOITED_TAG),
            ObjectFilter::tagged("triggering"),
        )),
        Box::new(PredicateAst::TaggedMatches(
            TagKey::from(crate::tag::EXPLOITER_TAG),
            ObjectFilter::source(),
        )),
    ))
}

fn predicate_diagnostic_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut display_tokens: Vec<OwnedLexToken> = tokens
        .iter()
        .filter(|token| {
            !token
                .as_word()
                .is_some_and(|_| is_article(token.parser_text()))
        })
        .cloned()
        .collect();

    if let Some(first) = display_tokens.first_mut()
        && token_word_is_any(first, ITS_WORDS)
    {
        first.replace_word("it");
    }
    if display_tokens.len() >= 2
        && token_word_is(&display_tokens[0], IT_WORD)
        && display_tokens[1].is_word("s")
    {
        display_tokens.remove(1);
    }

    if let Some(instead_idx) =
        primitives::find_prefix(&display_tokens, || primitives::kw(INSTEAD_WORD))
            .map(|(token_idx, _, _)| token_idx)
        && instead_idx > 0
    {
        let maybe_predicate = &display_tokens[..instead_idx];
        let maybe_clause = LexedClause::new(maybe_predicate);
        let maybe_word_len = maybe_clause.word_len();
        let paid_tail = maybe_word_len >= 3
            && maybe_clause
                .between_word_range(maybe_word_len - 3, maybe_word_len)
                .is_some_and(|tail| surface::exact_any(tail, COST_PAID_INSTEAD_TAIL_PHRASES));
        let unpaid_tail = maybe_word_len >= 4
            && maybe_clause
                .between_word_range(maybe_word_len - 4, maybe_word_len)
                .is_some_and(|tail| surface::exact(tail, COST_NOT_PAID_INSTEAD_TAIL_PHRASE));
        if paid_tail || unpaid_tail {
            display_tokens.truncate(instead_idx);
        }
    }

    let display_clause = LexedClause::new(&display_tokens);
    if surface::contains(display_clause, YOU_BOTH_OWN_AND_CONTROL_PHRASE)
        && let Some(exile_word_idx) = surface::find(display_clause, EXILE_THEM_PHRASE)
        && let Some(exile_token_idx) = display_clause
            .words()
            .token_span_for_words(exile_word_idx, exile_word_idx + 1)
            .map(|range| range.start)
    {
        display_tokens.truncate(exile_token_idx);
    }

    display_tokens
}

fn predicate_diagnostic_text(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(&predicate_diagnostic_tokens(tokens))
}

fn render_unsupported_predicate_message(tokens: &[OwnedLexToken]) -> String {
    format!(
        "unsupported predicate (predicate: '{}')",
        predicate_diagnostic_text(tokens)
    )
}

pub(crate) fn parse_predicate(tokens: &[OwnedLexToken]) -> Result<PredicateAst, CardTextError> {
    let predicate_tokens = if token_slice_first_is(tokens, "if") {
        &tokens[1..]
    } else {
        tokens
    };

    if !predicate_tokens.iter().any(|token| {
        token
            .as_word()
            .is_some_and(|_| !is_article(token.parser_text()))
    }) {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }

    if let Some(predicate) = parse_repeated_if_or_predicate(predicate_tokens)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_repeated_and_predicate(predicate_tokens)? {
        return Ok(predicate);
    }
    {
        let simple_words = non_article_token_word_refs(predicate_tokens);
        if [
            &["this", "creature", "is", "suspected"][..],
            &["this", "permanent", "is", "suspected"][..],
            &["it", "is", "suspected"][..],
            &["its", "suspected"][..],
        ]
        .iter()
        .any(|expected| surface::exact_words(&simple_words, expected))
        {
            return Ok(PredicateAst::SourceSuspected);
        }
    }
    if let Some(predicate) = parse_secret_choices_match_predicate(predicate_tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_vote_result_predicate(predicate_tokens, true)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_passive_this_way_tagged_object_predicate(predicate_tokens)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_active_this_way_discard_predicate(predicate_tokens)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_active_this_way_battlefield_predicate(predicate_tokens)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_passive_this_way_battlefield_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_this_ability_resolution_count_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_stack_object_targets_only_source_predicate(predicate_tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_stack_object_targets_object_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    // Spell-context comparisons are exact typed predicates.  Parse them
    // before broader control/object predicates can accept only the leading
    // "you control ..." portion and discard the relative spell controller.
    if let Some(predicate) = parse_spell_context_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_exploited_triggering_object_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_zone_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_exiled_with_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_happily_style_conjoined_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_colors_among_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_types_among_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_revealed_or_controlled_subtype_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_graveyard_threshold_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_in_your_graveyard_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_quantified_objects_in_graveyard_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_empty_battlefield_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_object_on_battlefield_predicate(tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_life_total_at_least_starting_predicate(predicate_tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_life_total_at_least_last_noted_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_controls_more_than_each_other_player_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_controls_more_than_you_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_status_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_counted_objects_have_counter_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_counted_source_exiled_objects_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_controlled_creatures_total_power_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_half_starting_life_total_threshold_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_you_life_total_at_most_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_object_keyword_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_opponent_controls_tagged_object_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_opponent_controls_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_vote_result_predicate(predicate_tokens, false)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_attacking_you_own_control_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_you_both_own_and_control_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_implicit_subject_and_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_while_conjoined_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_tagged_state_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_simple_state_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_crewed_by_exactly_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_attachment_count_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_identity_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_keyword_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) =
        parse_source_did_not_attack_or_enter_control_this_turn_shape(predicate_tokens)
    {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_there_are_no_counters_on_source_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_doesnt_have_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_has_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_has_counted_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_verbless_counted_counter_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_triggering_object_had_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_there_are_source_counters_at_least_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_power_threshold_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_basic_land_types_among_lands_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_there_are_objects_on_battlefield_predicate(tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_types_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_half_starting_life_total_threshold_predicate(predicate_tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_player_controls_more_than_each_other_player_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_controls_more_than_you_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_life_relation_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_count_parity_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_life_total_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_hand_relation_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_hand_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_turn_event_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_would_action_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_turn_timing_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_life_change_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_object_death_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_battlefield_change_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_battlefield_entry_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_combat_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_spell_lifecycle_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_paid_cost_label_predicate(predicate_tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_mana_spent_capture_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_attached_tagged_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_sacrificed_permanent_state_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_tagged_exiled_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    let demonstrative_reference = demonstrative_reference_kind(predicate_tokens);
    let is_it = demonstrative_reference == Some(DemonstrativeReferenceKind::It);

    if let Some(predicate) = parse_value_reference_comparison_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if is_it {
        if let Some(predicate) = parse_demonstrative_mana_value_predicate(predicate_tokens)? {
            return Ok(predicate);
        }
        if let Some(predicate) =
            parse_demonstrative_total_power_toughness_predicate(predicate_tokens)?
        {
            return Ok(predicate);
        }
        if let Some(predicate) = parse_demonstrative_power_or_toughness_predicate(predicate_tokens)?
        {
            return Ok(predicate);
        }
    }

    if demonstrative_reference.is_some()
        && predicate_tokens
            .iter()
            .any(|token| token_word_is(token, OR_WORD))
        && !contains_most_common_color_among_all_permanents_clause(predicate_tokens)
        && let Some(predicate) = parse_or_predicate(predicate_tokens)?
    {
        return Ok(predicate);
    }

    if demonstrative_reference.is_some() {
        if let Some(predicate) = parse_demonstrative_power_or_toughness_predicate(predicate_tokens)?
        {
            return Ok(predicate);
        }
        if let Some(predicate) = parse_demonstrative_shares_predicate(predicate_tokens) {
            return Ok(predicate);
        }
        if let Some(predicate) = parse_demonstrative_or_descriptor_predicate(predicate_tokens)? {
            return Ok(predicate);
        }
        if let Some(predicate) = parse_demonstrative_toxic_predicate(predicate_tokens) {
            return Ok(predicate);
        }
        if let Some(predicate) = parse_demonstrative_keyword_predicate(predicate_tokens) {
            return Ok(predicate);
        }
        if let Some((descriptor_tokens, negative, has_card, tagged_that_enchantment)) =
            demonstrative_descriptor_filter_tokens(predicate_tokens)
        {
            if let Some(filter) = parse_single_card_type_card_descriptor_tokens(&descriptor_tokens)
            {
                let predicate = if filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Land
                    && filter.subtypes.is_empty()
                    && !filter.nontoken
                    && filter.excluded_card_types.is_empty()
                {
                    PredicateAst::ItIsLandCard
                } else {
                    PredicateAst::ItMatches(filter)
                };
                return Ok(if negative {
                    PredicateAst::Not(Box::new(predicate))
                } else {
                    predicate
                });
            }
            if let Ok(filter) = parse_object_filter_lexed(&descriptor_tokens, false)
                && filter != ObjectFilter::default()
            {
                if has_card
                    && filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Land
                    && filter.subtypes.is_empty()
                    && !filter.nontoken
                    && filter.excluded_card_types.is_empty()
                {
                    let predicate = PredicateAst::ItIsLandCard;
                    return Ok(if negative {
                        PredicateAst::Not(Box::new(predicate))
                    } else {
                        predicate
                    });
                }
                if tagged_that_enchantment {
                    return Ok(PredicateAst::TaggedMatches(
                        TagKey::from("triggering"),
                        filter,
                    ));
                }
                let predicate = PredicateAst::ItMatches(filter);
                return Ok(if negative {
                    PredicateAst::Not(Box::new(predicate))
                } else {
                    predicate
                });
            }
        }
    }

    if let Some(predicate) = parse_player_controls_no_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) =
        parse_you_control_or_graveyard_predicate(predicate_tokens).transpose()?
    {
        return Ok(predicate);
    }

    if non_article_token_words_starts_with_any(predicate_tokens, YOU_CONTROL_PREFIXES) {
        if let Some(predicate) =
            parse_you_control_conjoined_predicate(predicate_tokens).transpose()?
        {
            return Ok(predicate);
        }

        if let Some(predicate) = parse_player_controls_predicate(
            predicate_tokens,
            PlayerAst::You,
            Some(PlayerFilter::You),
            2,
            true,
            true,
        )? {
            return Ok(predicate);
        }
    }

    if non_article_token_words_starts_with_any(predicate_tokens, THAT_PLAYER_CONTROLS_PREFIXES) {
        if let Some(predicate) = parse_player_controls_predicate(
            predicate_tokens,
            PlayerAst::That,
            None,
            3,
            false,
            false,
        )? {
            return Ok(predicate);
        }
    }

    if let Some(predicate) = parse_negative_put_tagged_object_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_achievement_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_ring_bearer_temptation_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_status_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_world_state_or_timing_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_combat_damage_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_spell_cast_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_x_value_comparison_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_or_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    Err(CardTextError::ParseError(
        render_unsupported_predicate_message(predicate_tokens),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CounterType;
    use crate::effect::{ChoiceCount, ValueComparisonOperator};
    use crate::filter::StackObjectKind;
    use crate::runtime_backend::front_end::lexer::lex_line;

    const IF_WORD: &str = "if";

    fn predicate_tokens_after_if(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
        tokens
            .iter()
            .filter(|token| !token_word_is(token, IF_WORD))
            .cloned()
            .collect()
    }

    #[test]
    fn parse_predicate_paid_cost_labels_use_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If this spells surge cost was paid",
                PredicateAst::ThisSpellPaidLabel("Surge".into()),
            ),
            (
                "If this creature's spectacle cost was paid instead discard your hand",
                PredicateAst::ThisSpellPaidLabel("Spectacle".into()),
            ),
            (
                "If {U} cost was paid",
                PredicateAst::ThisSpellPaidLabel("{U}".into()),
            ),
            (
                "If {2}{G} cost wasn't paid",
                PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel("{2}{G}".into()))),
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_opponent_would_begin_extra_turn() -> Result<(), CardTextError> {
        let tokens = lex_line("If an opponent would begin an extra turn", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::PlayerWouldBeginExtraTurn {
                player: PlayerAst::Opponent,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_x_value_comparison_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, operator, amount) in [
            ("If X is 3", ValueComparisonOperator::Equal, 3),
            (
                "If X is less than or equal to two",
                ValueComparisonOperator::LessThanOrEqual,
                2,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::ValueComparison {
                    left: Value::X,
                    operator,
                    right: Value::Fixed(amount),
                },
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_vote_results_use_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If death gets more votes",
                PredicateAst::VoteOptionGetsMoreVotes {
                    option: "death".to_string(),
                },
            ),
            (
                "If torture gets more votes or the vote is tied",
                PredicateAst::VoteOptionGetsMoreVotesOrTied {
                    option: "torture".to_string(),
                },
            ),
            (
                "If no creatures got votes",
                PredicateAst::NoVoteObjectsMatched {
                    filter: ObjectFilter::creature(),
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_secret_choices_match_uses_capture_parser() -> Result<(), CardTextError> {
        for text in ["If they match", "If those choices match"] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, PredicateAst::SecretChoicesMatch, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_source_identity_uses_capture_parser() -> Result<(), CardTextError> {
        let tokens = lex_line("If this enchantment isn't a creature", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert_eq!(
            parsed,
            PredicateAst::Not(Box::new(PredicateAst::SourceMatches(
                ObjectFilter::creature()
            )))
        );

        let tokens = lex_line("If this source is not an artifact", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert_eq!(
            parsed,
            PredicateAst::Not(Box::new(PredicateAst::SourceMatches(
                ObjectFilter::artifact()
            )))
        );

        let tokens = lex_line("If this permanent is red", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        match parsed {
            PredicateAst::SourceMatches(filter) => {
                assert!(filter.colors.is_some(), "{filter:?}");
            }
            other => panic!("expected source identity predicate, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_source_attachment_count_uses_capture_parser() -> Result<(), CardTextError> {
        let tokens = lex_line("If this creature is enchanted by two or more Auras", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        match parsed {
            PredicateAst::SourceHasAttachmentsMatching {
                filter,
                comparison,
                display,
            } => {
                assert_eq!(
                    comparison,
                    crate::effect::Comparison::GreaterThanOrEqual(2),
                    "{display}"
                );
                assert!(filter.subtypes.contains(&Subtype::Aura), "{filter:?}");
                assert_eq!(display, "this creature is enchanted by two or more auras");
            }
            other => panic!("expected source attachment predicate, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_player_object_keywords_use_capture_parser() -> Result<(), CardTextError> {
        let tokens = lex_line("If creatures you control have flying", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        match parsed {
            PredicateAst::PlayerControls { player, filter } => {
                assert_eq!(player, PlayerAst::You);
                assert_eq!(filter.controller, Some(PlayerFilter::You));
                assert!(
                    filter.card_types.contains(&CardType::Creature),
                    "{filter:?}"
                );
                assert!(
                    filter
                        .static_abilities
                        .contains(&crate::static_abilities::StaticAbilityId::Flying),
                    "{filter:?}"
                );
            }
            other => panic!("expected player-controls keyword predicate, got {other:?}"),
        }

        let tokens = lex_line("If nonland cards in your graveyard have escape", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        match parsed {
            PredicateAst::PlayerControls { player, filter } => {
                assert_eq!(player, PlayerAst::You);
                assert_eq!(filter.zone, Some(Zone::Graveyard));
                assert_eq!(filter.owner, Some(PlayerFilter::You));
                assert_eq!(
                    filter.alternative_cast,
                    Some(crate::filter::AlternativeCastKind::Escape),
                    "{filter:?}"
                );
            }
            other => panic!("expected graveyard keyword predicate, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_you_control_that_creature_keeps_tagged_reference()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If you control that creature", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        match parsed {
            PredicateAst::PlayerControls { player, filter } => {
                assert_eq!(player, PlayerAst::You);
                assert_eq!(filter.controller, Some(PlayerFilter::You));
                assert!(
                    filter.card_types.contains(&CardType::Creature),
                    "{filter:?}"
                );
                assert!(
                    filter.tagged_constraints.iter().any(|constraint| {
                        constraint.tag.as_str() == IT_TAG
                            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    }),
                    "{filter:?}"
                );
            }
            other => panic!("expected player-controls tagged predicate, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_opponent_controls_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected_filter) in [
            (
                "If opponent controls artifact",
                ObjectFilter {
                    controller: Some(PlayerFilter::Opponent),
                    card_types: vec![CardType::Artifact],
                    ..Default::default()
                },
            ),
            (
                "If an opponent controls another creature",
                ObjectFilter {
                    controller: Some(PlayerFilter::Opponent),
                    card_types: vec![CardType::Creature],
                    other: true,
                    ..Default::default()
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::PlayerControls {
                    player: PlayerAst::Opponent,
                    filter: expected_filter,
                },
                "{text}"
            );
        }

        let tokens = lex_line("If an opponent controls more creatures than you", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert!(
            matches!(parsed, PredicateAst::PlayerControlsMoreThanYou { .. }),
            "{parsed:?}"
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_opponent_controls_tagged_object_uses_capture_parser()
    -> Result<(), CardTextError> {
        for (text, filter) in [
            (
                "If an opponent controls it",
                ObjectFilter {
                    controller: Some(PlayerFilter::Opponent),
                    ..Default::default()
                },
            ),
            (
                "If opponent controls that creature",
                ObjectFilter {
                    controller: Some(PlayerFilter::Opponent),
                    card_types: vec![CardType::Creature],
                    ..Default::default()
                },
            ),
            (
                "If an opponent controls that permanent",
                ObjectFilter {
                    controller: Some(PlayerFilter::Opponent),
                    ..Default::default()
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, PredicateAst::ItMatches(filter), "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_demonstrative_permanent_card_strips_article() -> Result<(), CardTextError> {
        let tokens = lex_line("If it's a permanent card", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        assert_eq!(
            parsed,
            PredicateAst::ItMatches(ObjectFilter::permanent_card())
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_demonstrative_negated_land_card_keeps_it_reference()
    -> Result<(), CardTextError> {
        for text in ["If it isn't a land card", "If it is not a land card"] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

            assert_eq!(
                parsed,
                PredicateAst::Not(Box::new(PredicateAst::ItIsLandCard)),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_turn_timing_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            ("If it's your turn", PredicateAst::YourTurn),
            ("If your turn", PredicateAst::YourTurn),
            (
                "If it's not your turn",
                PredicateAst::Not(Box::new(PredicateAst::YourTurn)),
            ),
            (
                "If not your turn",
                PredicateAst::Not(Box::new(PredicateAst::YourTurn)),
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_world_state_timing_uses_shared_capture_parser() -> Result<(), CardTextError>
    {
        for (text, expected) in [
            (
                "If you or player you're attacking has initiative",
                PredicateAst::Or(
                    Box::new(PredicateAst::PlayerHasInitiative {
                        player: PlayerAst::You,
                    }),
                    Box::new(PredicateAst::PlayerHasInitiative {
                        player: PlayerAst::Defending,
                    }),
                ),
            ),
            (
                "If you or a player you're attacking has the initiative",
                PredicateAst::Or(
                    Box::new(PredicateAst::PlayerHasInitiative {
                        player: PlayerAst::You,
                    }),
                    Box::new(PredicateAst::PlayerHasInitiative {
                        player: PlayerAst::Defending,
                    }),
                ),
            ),
            ("If it's night", PredicateAst::ItIsNight),
            ("If it is night", PredicateAst::ItIsNight),
            ("If it night", PredicateAst::ItIsNight),
            (
                "If it's the first combat phase of the turn",
                PredicateAst::FirstCombatPhaseOfTurn,
            ),
            (
                "If it first combat phase of turn",
                PredicateAst::FirstCombatPhaseOfTurn,
            ),
            (
                "If you cast this spell during your main phase",
                PredicateAst::ThisSpellPaidLabel("CastDuringYourMainPhase".into()),
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_object_on_battlefield_uses_capture_parser() -> Result<(), CardTextError> {
        for text in [
            "If an artifact is on the battlefield",
            "If creatures are on battlefield",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            match parsed {
                PredicateAst::ValueComparison {
                    left,
                    operator,
                    right,
                } => {
                    assert_eq!(operator, ValueComparisonOperator::GreaterThan, "{text}");
                    assert_eq!(right, Value::Fixed(0), "{text}");
                    match left {
                        Value::Count(filter) => {
                            assert_eq!(filter.zone, Some(Zone::Battlefield), "{text}")
                        }
                        other => panic!("expected count for {text}, got {other:?}"),
                    }
                }
                other => panic!("expected battlefield count predicate for {text}, got {other:?}"),
            }
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_counted_battlefield_objects_uses_capture_parser() -> Result<(), CardTextError>
    {
        for text in [
            "If there are three or more artifacts on the battlefield",
            "If there are two or more other creatures on battlefield",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            match parsed {
                PredicateAst::ValueComparison {
                    left,
                    operator,
                    right,
                } => {
                    assert_eq!(
                        operator,
                        ValueComparisonOperator::GreaterThanOrEqual,
                        "{text}"
                    );
                    match right {
                        Value::Fixed(value) => assert!(value >= 2, "{text}"),
                        other => panic!("expected fixed count for {text}, got {other:?}"),
                    }
                    match left {
                        Value::Count(filter) => {
                            assert_eq!(filter.zone, Some(Zone::Battlefield), "{text}")
                        }
                        other => panic!("expected count for {text}, got {other:?}"),
                    }
                }
                other => panic!("expected battlefield count predicate for {text}, got {other:?}"),
            }
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_empty_battlefield_uses_capture_parser() -> Result<(), CardTextError> {
        for text in [
            "If no creatures are on the battlefield",
            "If no creature is on battlefield",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::PlayerControlsNo {
                    player: PlayerAst::Any,
                    filter: ObjectFilter::creature(),
                },
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_conjoined_control_uses_capture_parser() -> Result<(), CardTextError> {
        let tokens = lex_line("If you control an artifact and a creature", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        let PredicateAst::And(left, right) = parsed else {
            panic!("expected conjoined control predicate");
        };
        assert_eq!(
            *left,
            PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter: ObjectFilter::artifact().controlled_by(PlayerFilter::You),
            }
        );
        assert_eq!(
            *right,
            PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::You),
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_control_or_graveyard_uses_capture_parser() -> Result<(), CardTextError> {
        for text in [
            "If you control a creature or there is a creature card in your graveyard",
            "If you control an artifact or artifact card in your graveyard",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            let PredicateAst::PlayerControlsOrHasCardInGraveyard {
                player,
                control_filter,
                graveyard_filter,
            } = parsed
            else {
                panic!("expected control-or-graveyard predicate for {text}");
            };
            assert_eq!(player, PlayerAst::You, "{text}");
            assert_eq!(control_filter.controller, Some(PlayerFilter::You), "{text}");
            assert_eq!(graveyard_filter.zone, Some(Zone::Graveyard), "{text}");
            assert_eq!(graveyard_filter.owner, Some(PlayerFilter::You), "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_repeated_or_if_uses_capture_parser() -> Result<(), CardTextError> {
        let tokens = lex_line("If you have the initiative or if you're monarch", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        assert_eq!(
            parsed,
            PredicateAst::Or(
                Box::new(PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::PlayerIsMonarch {
                    player: PlayerAst::You,
                }),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_repeated_or_if_supports_value_reference_comparison()
    -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If that creature's power is 2 or less or if you control another Lizard",
            0,
        )?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        let PredicateAst::Or(left, right) = parsed else {
            panic!("expected or predicate");
        };
        assert!(matches!(
            *left,
            PredicateAst::ValueComparison {
                left: Value::PowerOf(_),
                operator: ValueComparisonOperator::LessThanOrEqual,
                right: Value::Fixed(2),
            }
        ));
        let PredicateAst::PlayerControls { player, filter } = *right else {
            panic!("expected player-controls predicate");
        };
        assert_eq!(player, PlayerAst::You);
        assert!(filter.subtypes.contains(&Subtype::Lizard), "{filter:?}");
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_most_common_color_constraint_clause() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If it shares a color with the most common color among all permanents or a color tied for most common",
            0,
        )?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        let PredicateAst::ItMatches(filter) = parsed else {
            panic!("expected it-matches predicate");
        };
        assert!(
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == TaggedOpbjectRelation::SharesMostCommonPermanentColor
            }),
            "expected most-common-color relation, got {filter:?}"
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_source_counter_or_cards_in_hand_uses_capture_parser()
    -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If there are twenty or more counters on it or you have twenty or more cards in hand",
            0,
        )?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        assert_eq!(
            parsed,
            PredicateAst::Or(
                Box::new(PredicateAst::SourceHasCountersAtLeast(20)),
                Box::new(PredicateAst::PlayerCardsInHandOrMore {
                    player: PlayerAst::You,
                    count: 20,
                }),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_player_statuses_use_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If you're monarch",
                PredicateAst::PlayerIsMonarch {
                    player: PlayerAst::You,
                },
            ),
            (
                "If you have the initiative",
                PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::You,
                },
            ),
            (
                "If you have maximum speed",
                PredicateAst::ValueComparison {
                    left: Value::Speed(PlayerFilter::You),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(4),
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_controlled_creatures_total_power_uses_shared_capture_parser()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If creatures you control have total power 8 or greater", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ValueComparison {
                left: Value::TotalPower(ObjectFilter::creature().you_control()),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(8),
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_control_conditions_use_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If you control three or more artifacts",
                PredicateAst::PlayerHasAtLeast {
                    player: PlayerAst::You,
                    filter: ObjectFilter::artifact().controlled_by(PlayerFilter::You),
                    count: 3,
                },
            ),
            (
                "If you control three or more creatures with different powers",
                PredicateAst::PlayerHasAtLeastWithDifferentPowers {
                    player: PlayerAst::You,
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::You),
                    count: 3,
                },
            ),
            (
                "If that player controls exactly two lands",
                PredicateAst::PlayerControlsExactly {
                    player: PlayerAst::That,
                    filter: ObjectFilter::land(),
                    count: 2,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_source_attack_control_gate_uses_capture_parser() -> Result<(), CardTextError>
    {
        for text in [
            "If this creature didn't attack or come under your control this turn",
            "If this creature didn't attack or came under your control this turn",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::And(
                    Box::new(PredicateAst::Not(Box::new(
                        PredicateAst::SourceAttackedThisTurn,
                    ))),
                    Box::new(PredicateAst::Not(Box::new(
                        PredicateAst::SourceCameUnderYourControlThisTurn,
                    ))),
                ),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_source_states_use_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            ("If this tapped", PredicateAst::SourceIsTapped),
            (
                "If this creature is untapped",
                PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)),
            ),
            (
                "If this creature is enchanted",
                PredicateAst::SourceIsEnchanted,
            ),
            (
                "If this creature isn't equipped",
                PredicateAst::Not(Box::new(PredicateAst::SourceIsEquipped)),
            ),
            (
                "If this permanent is saddled",
                PredicateAst::SourceIsSaddled,
            ),
            (
                "If it isn't saddled",
                PredicateAst::Not(Box::new(PredicateAst::SourceIsSaddled)),
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_negative_control_uses_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If you control no artifacts",
                PredicateAst::PlayerControlsNo {
                    player: PlayerAst::You,
                    filter: ObjectFilter::artifact().controlled_by(PlayerFilter::You),
                },
            ),
            (
                "If a player controls no creatures",
                PredicateAst::PlayerControlsNo {
                    player: PlayerAst::Any,
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::Any),
                },
            ),
            (
                "If you do not control another creature",
                PredicateAst::PlayerControlsNo {
                    player: PlayerAst::You,
                    filter: ObjectFilter {
                        other: true,
                        ..ObjectFilter::creature().controlled_by(PlayerFilter::You)
                    },
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_neither_control_keeps_tagged_relation() -> Result<(), CardTextError> {
        let tokens = lex_line("If you control neither creature", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        let mut expected_filter = ObjectFilter::creature().controlled_by(PlayerFilter::You);
        expected_filter = expected_filter
            .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
        assert_eq!(
            parsed,
            PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter: expected_filter,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_player_achievements_use_shared_capture_parser() -> Result<(), CardTextError>
    {
        for (text, expected) in [
            (
                "If you have city's blessing",
                PredicateAst::PlayerHasCitysBlessing {
                    player: PlayerAst::You,
                },
            ),
            (
                "If you've completed a dungeon",
                PredicateAst::PlayerCompletedDungeon {
                    player: PlayerAst::You,
                    dungeon_name: None,
                },
            ),
            (
                "If you have completed Lost Mine of Phandelver",
                PredicateAst::PlayerCompletedDungeon {
                    player: PlayerAst::You,
                    dungeon_name: Some("Lost Mine of Phandelver".to_string()),
                },
            ),
            (
                "If you haven't completed Lost Mine of Phandelver",
                PredicateAst::Not(Box::new(PredicateAst::PlayerCompletedDungeon {
                    player: PlayerAst::You,
                    dungeon_name: Some("Lost Mine of Phandelver".to_string()),
                })),
            ),
            ("If you have a full party", PredicateAst::YouHaveFullParty),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_inherits_it_for_bare_or_descriptor_tail() -> Result<(), CardTextError> {
        let tokens = lex_line("If it's a creature or planeswalker card", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        match parsed {
            PredicateAst::Or(left, right) => {
                assert!(
                    matches!(*left, PredicateAst::ItMatches(ref filter) if filter.card_types == vec![CardType::Creature]),
                    "expected creature left predicate, got {left:?}"
                );
                assert!(
                    matches!(*right, PredicateAst::ItMatches(ref filter) if filter.card_types == vec![CardType::Planeswalker]),
                    "expected planeswalker right predicate, got {right:?}"
                );
            }
            other => panic!("expected inherited-reference or predicate, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_keeps_comma_type_list_disjunctive() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If it's an artifact, creature, enchantment, or land card",
            0,
        )?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        match parsed {
            PredicateAst::Or(left, right) => {
                assert!(
                    matches!(left.as_ref(), PredicateAst::ItMatches(filter)
                        if filter.card_types == vec![
                            CardType::Artifact,
                            CardType::Creature,
                            CardType::Enchantment,
                        ] && filter.all_card_types.is_empty()),
                    "expected disjunctive permanent-type list on left, got {left:?}"
                );
                assert!(
                    matches!(right.as_ref(), PredicateAst::ItMatches(filter)
                        if filter.card_types == vec![CardType::Land]
                            && filter.all_card_types.is_empty()),
                    "expected land-card filter on right, got {right:?}"
                );
            }
            other => panic!("expected inherited-reference type-list predicate, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_you_dont_put_card_into_your_hand() -> Result<(), CardTextError> {
        let tokens = lex_line("If you don't put the card into your hand", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter: ObjectFilter::default().in_zone(Zone::Hand),
            }))
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_negative_put_tagged_object_uses_shared_capture_parser()
    -> Result<(), CardTextError> {
        for (text, zone) in [
            ("If you did not put card into your hand", Zone::Hand),
            (
                "If you didn't put that card onto the battlefield",
                Zone::Battlefield,
            ),
            ("If you don't put it onto battlefield", Zone::Battlefield),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                    filter: ObjectFilter::default().in_zone(zone),
                })),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_combat_damage_this_turn_uses_shared_capture_parser()
    -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "if it dealt combat damage to a player this turn",
                PredicateAst::SourceDealtCombatDamageToPlayerThisTurn,
            ),
            (
                "if a player was dealt combat damage by a Zombie this turn",
                PredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
                    player: PlayerAst::Any,
                    subtype: parse_subtype_word("zombie").expect("known subtype"),
                },
            ),
            (
                "if an opponent was dealt combat damage by a Dragon this turn",
                PredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
                    player: PlayerAst::Opponent,
                    subtype: parse_subtype_word("dragon").expect("known subtype"),
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_you_dont_put_it_into_your_hand() -> Result<(), CardTextError> {
        let tokens = lex_line("If you don't put it into your hand", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter: ObjectFilter::default().in_zone(Zone::Hand),
            }))
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_passive_battlefield_this_way_uses_capture_parser()
    -> Result<(), CardTextError> {
        for (text, filter_text) in [
            (
                "If an Equipment is put onto the battlefield this way",
                "an Equipment",
            ),
            ("If an Aura is put onto the battlefield this way", "an Aura"),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;
            let filter_tokens = lex_line(filter_text, 0)?;
            let mut filter = parse_object_filter(&filter_tokens, false)?;
            filter.zone = Some(Zone::Battlefield);

            assert_eq!(
                parsed,
                PredicateAst::TaggedMatches(TagKey::from(IT_TAG), filter),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_chosen_name_milled_this_way_uses_capture_parser() -> Result<(), CardTextError>
    {
        let tokens = lex_line("If a card with the chosen name was milled this way", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        let mut filter = ObjectFilter::default();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(CHOSEN_NAME_TAG),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
        assert_eq!(
            parsed,
            PredicateAst::TaggedMatches(TagKey::from(IT_TAG), filter)
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_put_filtered_object_onto_battlefield_this_way()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If you put an artifact onto the battlefield this way", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        let filter_tokens = lex_line("an artifact", 0)?;
        let mut filter = parse_object_filter(&filter_tokens, false)?;
        filter.zone = Some(Zone::Battlefield);
        assert_eq!(
            parsed,
            PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_that_player_discards_filtered_card_this_way()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If that player discards an artifact card this way", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;
        let artifact_filter_tokens = lex_line("an artifact card", 0)?;
        let mut artifact_filter = parse_object_filter(&artifact_filter_tokens, false)?;
        artifact_filter.zone = None;

        assert_eq!(
            parsed,
            PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::That,
                tag: TagKey::from(IT_TAG),
                filter: artifact_filter,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_would_draw_card() -> Result<(), CardTextError> {
        let tokens = lex_line("If you would draw a card", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;
        assert_eq!(
            parsed,
            PredicateAst::PlayerWouldDrawCard {
                player: PlayerAst::You
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_player_would_actions_use_shared_capture_parser() -> Result<(), CardTextError>
    {
        for (text, expected) in [
            (
                "If you would draw a card",
                PredicateAst::PlayerWouldDrawCard {
                    player: PlayerAst::You,
                },
            ),
            (
                "If an opponent would draw card",
                PredicateAst::PlayerWouldDrawCard {
                    player: PlayerAst::Opponent,
                },
            ),
            (
                "If opponent would proliferate",
                PredicateAst::PlayerWouldProliferate {
                    player: PlayerAst::Opponent,
                },
            ),
            (
                "If an opponent would begin an extra turn",
                PredicateAst::PlayerWouldBeginExtraTurn {
                    player: PlayerAst::Opponent,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_attacking_own_control_meld_uses_capture_parser() -> Result<(), CardTextError>
    {
        for text in [
            "If this creature and a creature named Midnight Scavengers are attacking and you both own and control them",
            "If this and creature named Phyrexian Dragon Engine are attacking, and you both own and control them, exile them",
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

            let PredicateAst::And(left, right) = parsed else {
                panic!("expected attacking own-control conjoined predicate for {text}");
            };
            for side in [left, right] {
                let PredicateAst::PlayerControls { player, filter } = *side else {
                    panic!("expected controls predicate for {text}");
                };
                assert_eq!(player, PlayerAst::You, "{text}");
                assert_eq!(filter.controller, Some(PlayerFilter::You), "{text}");
                assert!(filter.attacking, "{text}");
            }
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_you_both_own_and_control_uses_capture_parser() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If you both own and control this creature and a creature named Midnight Scavengers",
            0,
        )?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        let PredicateAst::And(left, right) = parsed else {
            panic!("expected own-and-control conjoined predicate");
        };
        let PredicateAst::PlayerControls {
            player: left_player,
            filter: left_filter,
        } = *left
        else {
            panic!("expected left controls predicate");
        };
        let PredicateAst::PlayerControls {
            player: right_player,
            filter: right_filter,
        } = *right
        else {
            panic!("expected right controls predicate");
        };
        assert_eq!(left_player, PlayerAst::You);
        assert_eq!(right_player, PlayerAst::You);
        assert_eq!(left_filter.controller, Some(PlayerFilter::You));
        assert_eq!(right_filter.controller, Some(PlayerFilter::You));
        Ok(())
    }

    #[test]
    fn parse_predicate_implicit_subject_and_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected_right) in [
            (
                "If you're monarch and you have the initiative",
                PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::You,
                },
            ),
            (
                "If you're monarch and have the initiative",
                PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::You,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

            assert_eq!(
                parsed,
                PredicateAst::And(
                    Box::new(PredicateAst::PlayerIsMonarch {
                        player: PlayerAst::You,
                    }),
                    Box::new(expected_right),
                ),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_while_conjoined_uses_capture_parser() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If you would draw a card while you have no cards in hand",
            0,
        )?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::And(
                Box::new(PredicateAst::PlayerWouldDrawCard {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::YouHaveNoCardsInHand),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_cards_in_hand_counts_use_shared_capture_parser() -> Result<(), CardTextError>
    {
        for (text, expected) in [
            (
                "If you have no cards in hand",
                PredicateAst::YouHaveNoCardsInHand,
            ),
            (
                "If you have one or fewer cards in hand",
                PredicateAst::PlayerCardsInHandOrFewer {
                    player: PlayerAst::You,
                    count: 1,
                },
            ),
            (
                "If an opponent has three or more cards in hand",
                PredicateAst::PlayerCardsInHandOrMore {
                    player: PlayerAst::Opponent,
                    count: 3,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_cards_in_hand_relations_use_shared_capture_parser()
    -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If an opponent has more cards in hand than you",
                PredicateAst::PlayerHasMoreCardsInHandThanYou {
                    player: PlayerAst::Opponent,
                },
            ),
            (
                "If a player has more cards in hand than each other player",
                PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer {
                    player: PlayerAst::Any,
                },
            ),
            (
                "If that player has more cards in their hand than you do",
                PredicateAst::PlayerHasMoreCardsInHandThanYou {
                    player: PlayerAst::That,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_turn_event_counts_use_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If you drew two or more cards this turn",
                PredicateAst::ValueComparison {
                    left: Value::MaxCardsDrawnThisTurn(PlayerFilter::You),
                    operator: ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(2),
                },
            ),
            (
                "If an opponent has drawn three cards this turn",
                PredicateAst::ValueComparison {
                    left: Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent),
                    operator: ValueComparisonOperator::Equal,
                    right: Value::Fixed(3),
                },
            ),
            (
                "If that player had two or fewer lands entered battlefield under their control this turn",
                PredicateAst::ValueComparison {
                    left: Value::LandsEnteredBattlefieldThisTurn(PlayerFilter::IteratedPlayer),
                    operator: ValueComparisonOperator::LessThanOrEqual,
                    right: Value::Fixed(2),
                },
            ),
            (
                "If that player had two or more lands enter the battlefield under their control this turn",
                PredicateAst::ValueComparison {
                    left: Value::LandsEnteredBattlefieldThisTurn(PlayerFilter::IteratedPlayer),
                    operator: ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(2),
                },
            ),
            (
                "If that player had another land enter the battlefield under their control this turn",
                PredicateAst::ValueComparison {
                    left: Value::LandsEnteredBattlefieldThisTurn(PlayerFilter::IteratedPlayer),
                    operator: ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(2),
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_spell_context_uses_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If that spells controller poisoned",
                PredicateAst::TargetSpellControllerIsPoisoned,
            ),
            (
                "If no mana was spent to cast that spell",
                PredicateAst::TargetSpellNoManaSpentToCast,
            ),
            (
                "If you control more creatures than its controller",
                PredicateAst::YouControlMoreCreaturesThanTargetSpellController,
            ),
            (
                "If you control more creatures than that spell's controller",
                PredicateAst::YouControlMoreCreaturesThanTargetSpellController,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_tagged_state_uses_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected_filter) in [
            (
                "If that permanent is black",
                ObjectFilter {
                    colors: Some(ColorSet::BLACK),
                    ..Default::default()
                },
            ),
            (
                "If it's blocking",
                ObjectFilter {
                    blocking: true,
                    ..Default::default()
                },
            ),
            (
                "If that creature is attacking",
                ObjectFilter {
                    attacking: true,
                    ..Default::default()
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
            assert_eq!(parsed, PredicateAst::ItMatches(expected_filter), "{text}");
        }

        for (text, expected) in [
            (
                "If those cards remain exiled",
                PredicateAst::TaggedMatches(
                    TagKey::from(IT_TAG),
                    ObjectFilter::default().in_zone(Zone::Exile),
                ),
            ),
            (
                "If it is paired with another creature",
                PredicateAst::ItIsSoulbondPaired,
            ),
            (
                "If it's paired with another creature",
                PredicateAst::ItIsSoulbondPaired,
            ),
            (
                "If it's paired with a creature",
                PredicateAst::ItIsSoulbondPaired,
            ),
            (
                "If you controlled that permanent",
                PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                    filter: ObjectFilter::default(),
                },
            ),
            (
                "If that card entered under your control",
                PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn {
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                },
            ),
            (
                "If that creature was not blocking",
                PredicateAst::TaggedMatches(
                    TagKey::from(IT_TAG),
                    ObjectFilter {
                        nonblocking: true,
                        ..Default::default()
                    },
                ),
            ),
            (
                "If that creature was blue or black",
                PredicateAst::TaggedMatches(
                    TagKey::from(IT_TAG),
                    ObjectFilter {
                        colors: Some(ColorSet::BLUE.union(ColorSet::BLACK)),
                        ..Default::default()
                    },
                ),
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
            assert_eq!(parsed, expected, "{text}");
        }

        let tokens = lex_line("If enchanted creature is a Zombie", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        match parsed {
            PredicateAst::TaggedMatches(tag, filter) => {
                assert_eq!(tag, TagKey::from("enchanted"));
                assert!(
                    !filter.subtypes.is_empty() || !filter.card_types.is_empty(),
                    "{filter:?}"
                );
            }
            other => panic!("expected enchanted tagged predicate, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_attached_tagged_uses_shared_capture_parser() -> Result<(), CardTextError> {
        for text in [
            "If this permanent is attached to a creature",
            "If that permanent attached to an artifact creature",
            "If this permanent attached to an enchantment creature",
            "If that permanent is attached to a land creature",
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
            match parsed {
                PredicateAst::TaggedMatches(tag, filter) => {
                    assert_eq!(tag, TagKey::from("enchanted"), "{text}");
                    assert!(!filter.card_types.is_empty(), "{text}: {filter:?}");
                }
                other => panic!("expected attached tagged predicate for {text}, got {other:?}"),
            }
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_mana_spent_uses_shared_capture_parser() -> Result<(), CardTextError> {
        let tokens = lex_line("If {S} was spent to cast this spell", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert!(
            matches!(
                parsed,
                PredicateAst::ManaSpentToCastThisSpellAtLeast {
                    amount: 1,
                    symbol: Some(_),
                }
            ),
            "{parsed:?}"
        );

        let tokens = lex_line("If {R}{G} was spent to cast this spell", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert!(matches!(parsed, PredicateAst::And(_, _)), "{parsed:?}");

        let tokens = lex_line(
            "If at least three blue mana was spent to cast this spell",
            0,
        )?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert!(
            matches!(
                parsed,
                PredicateAst::ManaSpentToCastThisSpellAtLeast {
                    amount: 3,
                    symbol: Some(_),
                }
            ),
            "{parsed:?}"
        );

        let tokens = lex_line("If at least four mana was spent to cast it", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert!(
            matches!(
                parsed,
                PredicateAst::ManaSpentToCastThisSpellAtLeast {
                    amount: 4,
                    symbol: None,
                }
            ),
            "{parsed:?}"
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_spell_lifecycle_uses_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            ("If you cast this spell", PredicateAst::SourceWasCast),
            (
                "If it was cast",
                PredicateAst::TaggedWasCast(TagKey::from(IT_TAG)),
            ),
            (
                "If this spell was cast from a graveyard",
                PredicateAst::ThisSpellWasCastFromZone(Zone::Graveyard),
            ),
            (
                "If this spell was cast from anywhere other than your hand",
                PredicateAst::ThisSpellWasCastFromNonHand,
            ),
            (
                "If no spells were cast last turn",
                PredicateAst::NoSpellsWereCastLastTurn,
            ),
            ("If this spell was kicked", PredicateAst::ThisSpellWasKicked),
            (
                "If this spell was bargained",
                PredicateAst::ThisSpellPaidLabel("Bargain".into()),
            ),
            (
                "If it was bargained",
                PredicateAst::ThisSpellPaidLabel("Bargain".into()),
            ),
            (
                "If gift was promised",
                PredicateAst::ThisSpellPaidLabel("Gift".into()),
            ),
            (
                "If the gift was promised",
                PredicateAst::ThisSpellPaidLabel("Gift".into()),
            ),
            (
                "If gift was not promised",
                PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel("Gift".into()))),
            ),
            (
                "If tribute was not paid",
                PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel("Tribute".into()))),
            ),
            (
                "If tribute wasn't paid",
                PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel("Tribute".into()))),
            ),
            ("If that was kicked", PredicateAst::TargetWasKicked),
            ("If that spell was kicked", PredicateAst::TargetWasKicked),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_combat_turn_uses_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If you attacked this turn",
                PredicateAst::YouAttackedThisTurn,
            ),
            (
                "If that creature had to attack this combat",
                PredicateAst::TriggeringObjectHadToAttackThisCombat,
            ),
            (
                "If you attacked with exactly two other creatures this combat",
                PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(2),
            ),
            (
                "If this creature attacked or blocked this turn",
                PredicateAst::SourceAttackedOrBlockedThisTurn,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_spell_cast_this_turn_uses_shared_capture_parser() -> Result<(), CardTextError>
    {
        let tokens = lex_line("If you cast another spell this turn", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert_eq!(
            parsed,
            PredicateAst::PlayerCastSpellsThisTurnOrMore {
                player: PlayerAst::You,
                count: 2,
            }
        );

        let tokens = lex_line("If opponent has cast a creature spell this turn", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        let PredicateAst::ValueComparison {
            left:
                Value::SpellsCastThisTurnMatching {
                    player,
                    filter,
                    exclude_source,
                },
            operator: ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(1),
        } = parsed
        else {
            panic!("expected spell-cast matching predicate, got {parsed:?}");
        };
        assert_eq!(player, PlayerFilter::Opponent);
        assert!(!exclude_source);
        assert!(filter.card_types.contains(&CardType::Creature));

        let tokens = lex_line("If you didnt cast a noncreature spell this turn", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert!(
            matches!(&parsed, PredicateAst::Not(inner) if matches!(
                inner.as_ref(),
                PredicateAst::ValueComparison {
                    left: Value::SpellsCastThisTurnMatching { player: PlayerFilter::You, .. },
                    operator: ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                }
            )),
            "expected negated spell-cast matching predicate, got {parsed:?}"
        );

        let tokens = lex_line("If you haven't cast a spell from your hand this turn", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        let PredicateAst::Not(inner) = parsed else {
            panic!("expected negated hand-origin spell-cast predicate, got {parsed:?}");
        };
        let PredicateAst::ValueComparison {
            left:
                Value::SpellsCastThisTurnMatching {
                    player,
                    filter,
                    exclude_source,
                },
            operator: ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(1),
        } = *inner
        else {
            panic!("expected hand-origin spell-cast value comparison, got {inner:?}");
        };
        assert_eq!(player, PlayerFilter::You);
        assert_eq!(filter.zone, Some(Zone::Hand));
        assert!(!exclude_source);

        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_would_proliferate() -> Result<(), CardTextError> {
        let tokens = lex_line("If you would proliferate", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;
        assert_eq!(
            parsed,
            PredicateAst::PlayerWouldProliferate {
                player: PlayerAst::You
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_have_more_life_than_opponent() -> Result<(), CardTextError> {
        let tokens = lex_line("if you have more life than an opponent", 0)?;

        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::PlayerHasLessLifeThanYou {
                player: PlayerAst::Opponent,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_life_relations_use_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "if an opponent has more life than you",
                PredicateAst::PlayerHasMoreLifeThanYou {
                    player: PlayerAst::Opponent,
                },
            ),
            (
                "if you have more life than each opponent",
                PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
                    player: PlayerAst::You,
                },
            ),
            (
                "if no opponent has more life than that player",
                PredicateAst::PlayerHasNoOpponentWithMoreLifeThan {
                    player: PlayerAst::That,
                },
            ),
            (
                "if a player has more life than each other player",
                PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
                    player: PlayerAst::Any,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_life_totals_use_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If you have five or less life",
                PredicateAst::ValueComparison {
                    left: crate::effect::Value::LifeTotal(PlayerFilter::You),
                    operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                    right: crate::effect::Value::Fixed(5),
                },
            ),
            (
                "If your life total is five or less",
                PredicateAst::ValueComparison {
                    left: crate::effect::Value::LifeTotal(PlayerFilter::You),
                    operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                    right: crate::effect::Value::Fixed(5),
                },
            ),
            (
                "If an opponent has ten or more life",
                PredicateAst::ValueComparison {
                    left: crate::effect::Value::LifeTotal(PlayerFilter::Opponent),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: crate::effect::Value::Fixed(10),
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_life_change_this_turn_uses_shared_capture_parser()
    -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If you gained life this turn",
                PredicateAst::PlayerGainedLifeThisTurnOrMore {
                    player: PlayerAst::You,
                    count: 1,
                },
            ),
            (
                "If you gained three or more life this turn",
                PredicateAst::PlayerGainedLifeThisTurnOrMore {
                    player: PlayerAst::You,
                    count: 3,
                },
            ),
            (
                "If you lost two or more life this turn",
                PredicateAst::ValueComparison {
                    left: Value::LifeLostThisTurn(PlayerFilter::You),
                    operator: ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(2),
                },
            ),
            (
                "If one or more opponents lost life this turn",
                PredicateAst::OpponentLostLifeThisTurn,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_ring_bearer_temptation_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If this creature is your Ring-bearer",
                PredicateAst::SourceIsRingBearer {
                    player: PlayerAst::You,
                },
            ),
            (
                "If Ring has tempted you one or more time this game",
                PredicateAst::PlayerRingTemptedThisGameOrMore {
                    player: PlayerAst::You,
                    count: 1,
                },
            ),
            (
                "If this is your Ring-bearer and the Ring has tempted you two or more times this game",
                PredicateAst::And(
                    Box::new(PredicateAst::SourceIsRingBearer {
                        player: PlayerAst::You,
                    }),
                    Box::new(PredicateAst::PlayerRingTemptedThisGameOrMore {
                        player: PlayerAst::You,
                        count: 2,
                    }),
                ),
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_creature_card_put_into_your_graveyard_this_turn()
    -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If a creature card was put into your graveyard from anywhere this turn",
            0,
        )?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_battlefield_change_this_turn_uses_shared_capture_parser()
    -> Result<(), CardTextError> {
        let cases = [
            (
                "If no permanents left battlefield this turn",
                PredicateAst::Not(Box::new(PredicateAst::PermanentLeftBattlefieldThisTurn)),
            ),
            (
                "If a permanent left battlefield this turn",
                PredicateAst::PermanentLeftBattlefieldThisTurn,
            ),
            (
                "If a nonland permanent left the battlefield this turn or a spell was warped this turn",
                PredicateAst::Or(
                    Box::new(PredicateAst::NonlandPermanentLeftBattlefieldThisTurn),
                    Box::new(PredicateAst::SpellWasWarpedThisTurn),
                ),
            ),
            (
                "If creatures left battlefield under your control this turn",
                PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn,
            ),
            (
                "If lands you controlled were put into graveyard from battlefield this turn",
                PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(
                    ObjectFilter::land().controlled_by(PlayerFilter::You),
                ),
            ),
        ];

        for (text, expected) in cases {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_object_death_this_turn_uses_shared_capture_parser()
    -> Result<(), CardTextError> {
        let cases = [
            (
                "If a creature died this turn",
                PredicateAst::CreatureDiedThisTurn,
            ),
            (
                "If seven or more creatures died this turn",
                PredicateAst::CreatureDiedThisTurnOrMore(7),
            ),
            (
                "If a creature died under your control this turn",
                PredicateAst::ValueComparison {
                    left: Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                },
            ),
            (
                "If a creature card was put into your graveyard from anywhere this turn",
                PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn,
            ),
        ];

        for (text, expected) in cases {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_battlefield_entry_uses_shared_capture_parser() -> Result<(), CardTextError> {
        let cases = [
            (
                "If you had another creature entered the battlefield under your control last turn",
                PredicateAst::ObjectEnteredBattlefieldLastTurn(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::You)
                        .other(),
                ),
            ),
            (
                "If artifacts entered battlefield under your control this turn",
                PredicateAst::ObjectEnteredBattlefieldThisTurn(
                    ObjectFilter::artifact().controlled_by(PlayerFilter::You),
                ),
            ),
            (
                "If you had lands entered battlefield under your control this turn",
                PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
                    player: PlayerAst::You,
                },
            ),
        ];

        for (text, expected) in cases {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_card_in_your_graveyard_uses_capture_parser() -> Result<(), CardTextError> {
        let tokens = lex_line("If there is an Elf card in your graveyard", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        let mut expected_filter = ObjectFilter::default()
            .with_subtype(parse_subtype_word("elf").expect("elf subtype"))
            .in_zone(Zone::Graveyard);
        expected_filter.owner = Some(PlayerFilter::You);
        assert_eq!(
            parsed,
            PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter: expected_filter,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_targets_only_source_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected_card_types) in [
            (
                "If that spell targets only this creature",
                vec![CardType::Creature],
            ),
            ("If spell targets only this permanent", vec![]),
            ("If it targets only it", vec![]),
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

            let PredicateAst::ItMatches(filter) = parsed else {
                panic!("expected spell target predicate for {text}");
            };
            assert_eq!(filter.zone, Some(Zone::Stack), "{text}");
            assert_eq!(filter.stack_kind, Some(StackObjectKind::Spell), "{text}");
            assert_eq!(filter.target_count, Some(ChoiceCount::exactly(1)), "{text}");
            let Some(target_filter) = filter.targets_only_object.as_deref() else {
                panic!("expected targets-only object filter for {text}");
            };
            assert!(target_filter.source, "{text}");
            assert_eq!(target_filter.zone, Some(Zone::Battlefield), "{text}");
            assert_eq!(target_filter.card_types, expected_card_types, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_stack_object_targets_object_uses_capture_parser() -> Result<(), CardTextError>
    {
        let tokens = lex_line("If that spell targets a commander you control", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        let PredicateAst::ItMatches(filter) = parsed else {
            panic!("expected spell targeting predicate");
        };
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert_eq!(filter.stack_kind, Some(StackObjectKind::Spell));
        let Some(target_filter) = filter.targets_object.as_deref() else {
            panic!("expected targeted object filter");
        };
        assert!(target_filter.is_commander, "{target_filter:?}");
        assert_eq!(target_filter.controller, Some(PlayerFilter::You));
        Ok(())
    }

    #[test]
    fn parse_predicate_source_zone_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected_zone) in [
            ("If this card is in your hand", Zone::Hand),
            ("If this creature is in your graveyard", Zone::Graveyard),
            ("If this is in exile", Zone::Exile),
            ("If this card is in the command zone", Zone::Command),
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

            assert_eq!(
                parsed,
                PredicateAst::SourceIsInZone(expected_zone),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_behold_or_controlled_subtype_uses_capture_parser()
    -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If you revealed a Dragon card or controlled a Dragon as you cast this spell",
            0,
        )?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Or(
                Box::new(PredicateAst::ThisSpellPaidLabel("Behold".into())),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: ObjectFilter::default()
                        .with_subtype(parse_subtype_word("dragon").expect("dragon subtype")),
                }),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_triggering_object_counters_use_shared_capture_parser()
    -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If it had no stun counters on it",
                PredicateAst::TriggeringObjectHadNoCounter(CounterType::Stun),
            ),
            (
                "If that creature had a +1/+1 counter on it",
                PredicateAst::TriggeringObjectHadCounterAtLeast {
                    counter_type: CounterType::PlusOnePlusOne,
                    count: 1,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_controls_more_than_you_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected_player, expected_filter) in [
            (
                "If an opponent controls more creatures than you",
                PlayerAst::Opponent,
                ObjectFilter::creature(),
            ),
            (
                "If target opponent controls more artifacts than you do",
                PlayerAst::TargetOpponent,
                ObjectFilter::artifact(),
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::PlayerControlsMoreThanYou {
                    player: expected_player,
                    filter: expected_filter,
                },
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_graveyard_card_counts_use_capture_parser() -> Result<(), CardTextError> {
        let tokens = lex_line("If you have seven or more cards in your graveyard", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert_eq!(
            parsed,
            PredicateAst::PlayerHasAtLeast {
                player: PlayerAst::You,
                filter: ObjectFilter {
                    zone: Some(Zone::Graveyard),
                    ..Default::default()
                },
                count: 7,
            }
        );

        let tokens = lex_line("If twenty or more creature cards are in your graveyard", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        let PredicateAst::ValueComparison {
            left: Value::Count(filter),
            operator: ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(20),
        } = parsed
        else {
            panic!("expected quantified graveyard object-count predicate, got {parsed:?}");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert!(filter.card_types.contains(&CardType::Creature));

        for (text, expected_player, expected_operator, expected_count) in [
            (
                "If an opponent has fewer than three cards in their graveyard",
                PlayerFilter::Opponent,
                ValueComparisonOperator::LessThan,
                3,
            ),
            (
                "If target opponent has exactly two card in their graveyard",
                PlayerFilter::target_opponent(),
                ValueComparisonOperator::Equal,
                2,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::ValueComparison {
                    left: Value::CardsInGraveyard(expected_player),
                    operator: expected_operator,
                    right: Value::Fixed(expected_count),
                },
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_colors_among_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected_count) in [
            ("If there are five colors among permanents you control", 5),
            ("If there were one color among permanent you control", 1),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::ValueComparison {
                    left: Value::ColorsAmong(ObjectFilter::permanent().you_control()),
                    operator: ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(expected_count),
                },
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_counted_source_exiled_objects_uses_capture_parser()
    -> Result<(), CardTextError> {
        for (text, expected_count, expected_card_type) in [
            (
                "If three or more cards have been exiled with this artifact",
                3,
                None,
            ),
            (
                "If exactly two creature cards have been exiled with this",
                2,
                Some(CardType::Creature),
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

            let PredicateAst::ValueComparison {
                left: Value::Count(filter),
                right: Value::Fixed(count),
                ..
            } = parsed
            else {
                panic!("expected counted source-exiled predicate for {text}");
            };
            assert_eq!(count, expected_count, "{text}");
            assert_eq!(filter.zone, Some(Zone::Exile), "{text}");
            assert!(
                filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG),
                "{text}"
            );
            if let Some(card_type) = expected_card_type {
                assert!(filter.card_types.contains(&card_type), "{text}");
            }
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_counted_objects_with_counters_uses_capture_parser()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If two or more creatures have +1/+1 counters", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        let PredicateAst::ValueComparison {
            left: Value::Count(filter),
            operator: ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(2),
        } = parsed
        else {
            panic!("expected counted object-with-counter predicate");
        };
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert!(filter.with_counter.is_some());
        Ok(())
    }

    #[test]
    fn parse_predicate_card_types_among_uses_capture_parser() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If there are six or more card types among permanents you control and/or cards in your graveyard",
            0,
        )?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        let PredicateAst::ValueComparison {
            left: Value::CardTypesAmong(filter),
            operator: ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(6),
        } = parsed
        else {
            panic!("expected card-types-among value comparison, got {parsed:#?}");
        };
        assert_eq!(filter.any_of.len(), 2);
        assert!(
            filter
                .any_of
                .contains(&ObjectFilter::permanent().you_control())
        );
        assert!(filter.any_of.iter().any(|filter| {
            filter.zone == Some(Zone::Graveyard) && filter.owner == Some(PlayerFilter::You)
        }));

        let tokens = lex_line(
            "If there are two or more card types among sacrificed permanents",
            0,
        )?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert_eq!(
            parsed,
            PredicateAst::ValueComparison {
                left: Value::CardTypesAmong(ObjectFilter::tagged("sacrificed_0")),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(2),
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_graveyard_card_types_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected_player, expected_count) in [
            (
                "If there are six or more card types among cards in your graveyard",
                PlayerAst::You,
                6,
            ),
            (
                "If you have four or more card types among cards in your graveyard",
                PlayerAst::You,
                4,
            ),
            (
                "If there are three or more card type among card in target player's graveyard",
                PlayerAst::Target,
                3,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::PlayerHasCardTypesInGraveyardOrMore {
                    player: expected_player,
                    count: expected_count,
                },
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_basic_land_types_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If there are two or more basic land types among lands you control",
                PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore {
                    player: PlayerAst::You,
                    count: 2,
                },
            ),
            (
                "If there are three basic land types among lands that player controls",
                PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore {
                    player: PlayerAst::That,
                    count: 3,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_source_counters_use_shared_capture_parser() -> Result<(), CardTextError> {
        let counted_counter_tokens = lex_line("If it three or more +1/+1 counters on it", 0)?;
        assert_eq!(
            parse_source_verbless_counted_counter_predicate(&predicate_tokens_after_if(
                &counted_counter_tokens
            )),
            Some(PredicateAst::ValueComparison {
                left: Value::CountersOn(
                    Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                    Some(CounterType::PlusOnePlusOne),
                ),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(3),
            })
        );

        for (text, expected) in [
            (
                "If this has no stun counters on it",
                PredicateAst::SourceHasNoCounter(CounterType::Stun),
            ),
            (
                "If there are no more scream counters on it",
                PredicateAst::SourceHasNoCounter(CounterType::Named("scream")),
            ),
            (
                "If there are two counters on this creature",
                PredicateAst::SourceHasCountersAtLeast(2),
            ),
            (
                "If there are three stun counters on this",
                PredicateAst::SourceHasCounterAtLeast {
                    counter_type: CounterType::Stun,
                    count: 3,
                },
            ),
            (
                "If this creature has a +1/+1 counter on it",
                PredicateAst::SourceHasCounterAtLeast {
                    counter_type: CounterType::PlusOnePlusOne,
                    count: 1,
                },
            ),
            (
                "If this creature doesn't have a flying counter on it",
                PredicateAst::SourceHasNoCounter(CounterType::Flying),
            ),
            (
                "If this creature has two stun counters on it",
                PredicateAst::SourceHasCounterAtLeast {
                    counter_type: CounterType::Stun,
                    count: 2,
                },
            ),
            (
                "If it has three or more +1/+1 counters on it",
                PredicateAst::ValueComparison {
                    left: Value::CountersOn(
                        Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                        Some(CounterType::PlusOnePlusOne),
                    ),
                    operator: ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(3),
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }

        crate::runtime_backend::front_end::shared::util::with_source_reference_context(
            "Sarulf, Realm Eater",
            || {
                let tokens = lex_line("If Sarulf has one or more +1/+1 counters on it", 0)?;
                let predicate_tokens = predicate_tokens_after_if(&tokens);

                let parsed = parse_predicate(&predicate_tokens)?;

                assert_eq!(
                    parsed,
                    PredicateAst::SourceHasCounterAtLeast {
                        counter_type: CounterType::PlusOnePlusOne,
                        count: 1,
                    }
                );
                Ok::<(), CardTextError>(())
            },
        )?;
        Ok(())
    }

    #[test]
    fn parse_predicate_source_power_uses_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected_count) in [
            ("If this has power 7 or greater", 7),
            ("If this creature's power is 1 or more", 1),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::SourcePowerAtLeast(expected_count),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_source_has_keyword() -> Result<(), CardTextError> {
        for (text, ability) in [
            (
                "If this creature has defender",
                crate::static_abilities::StaticAbilityId::Defender,
            ),
            (
                "If this source has flying",
                crate::static_abilities::StaticAbilityId::Flying,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            let mut expected_filter = ObjectFilter::default();
            expected_filter.static_abilities.push(ability);
            assert_eq!(
                parsed,
                PredicateAst::SourceMatches(expected_filter),
                "{text}"
            );
        }
        Ok(())
    }
}

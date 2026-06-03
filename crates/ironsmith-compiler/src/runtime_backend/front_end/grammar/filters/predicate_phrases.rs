use super::super::super::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern};
use super::super::super::lexer::{LexedClause, OwnedLexToken, render_token_slice};
use super::*;
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};

const OUTLAW_SHORTHAND_FILTER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["outlaw"],
            &["outlaws"],
            &["outlaw", "creature"],
            &["outlaws", "creatures"],
        ]
);
const SACRIFICED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["sacrificed"]);
const PERMANENT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["permanent"]);
const CREATURE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["creature"]);
const COUNTER_WORD_PHRASES: &[&[&str]] = &[&["counter"], &["counters"]];
const PERMANENTS_YOU_CONTROL_SCOPE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["permanent", "you", "control"],
            &["permanent", "you", "controls"],
            &["permanents", "you", "control"],
            &["permanents", "you", "controls"],
        ]
);
const CARDS_IN_YOUR_GRAVEYARD_SCOPE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["card", "in", "your", "graveyard"],
            &["cards", "in", "your", "graveyard"],
        ]
);
const PERMANENTS_AND_OR_GRAVEYARD_CONNECTOR_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["and/or"]);
const PERMANENTS_AND_OR_SPLIT_CONNECTOR_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["and", "or"]);
const THERE_ARE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["there", "are"]);
const AND_YOUR_LIFE_TOTAL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["and", "your", "life", "total"]);
const LIFE_TOTAL_AT_LEAST_STARTING_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "your", "life", "total", "is", "greater", "than", "or", "equal", "to", "your",
            "starting", "life", "total",
        ]
);
const OR_MORE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["or", "more"]);
const HAS_OR_HAVE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["has"], &["have"]]);
const INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const OTHER_OR_ANOTHER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["another"], &["other"]]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const CHOSEN_NAME_TAG: &str = "__chosen_name__";
const CARD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["card"]);
const CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const NONLAND_CARD_OBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["nonland", "card"],
            &["nonland", "cards"],
            &["non", "land", "card"],
            &["non", "land", "cards"],
        ]
);
const BEEN_EXILED_WITH_THIS_SOURCE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["been", "exiled", "with", "this"],
            &["exiled", "with", "this"],
        ]
);
const IT_EXPLOITED_TRIGGERING_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "exploited", "that", "creature"],
            &["it", "exploited", "that", "object"],
        ]
);
const COST_PAID_INSTEAD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["cost", "was", "paid"], &["cost", "wasnt", "paid"]]);
const COST_NOT_PAID_INSTEAD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cost", "was", "not", "paid"]);
const YOU_BOTH_OWN_AND_CONTROL_PHRASE: &[&str] = &["you", "both", "own", "and", "control"];
const YOU_BOTH_OWN_AND_CONTROL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & YOU_BOTH_OWN_AND_CONTROL_PHRASE);
const EXILE_THEM_PHRASE: &[&str] = &["exile", "them"];
const EXILE_THEM_PATTERN: ClauseShape<'static> = clause_shape!(exact & EXILE_THEM_PHRASE);
const ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["a"], &["an"]]);
const DEFINITE_ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const WAS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["was"]);
const MANA_SPENT_TO_CAST_THIS_SPELL_PHRASES: &[&[&str]] = &[
    &["was", "spent", "to", "cast", "this", "spell"],
    &["were", "spent", "to", "cast", "this", "spell"],
];
const YOU_CONTROL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["you", "control"], &["you", "controls"]]);
const THAT_PLAYER_CONTROLS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["that", "player", "control"],
            &["that", "player", "controls"],
            &["that", "players", "control"],
            &["that", "players", "controls"],
        ]
);
const WITH_DIFFERENT_POWERS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["with", "different", "powers"],
            &["with", "different", "power"],
        ]
);
const NOT_TOKEN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["not", "token"]);
const THAT_ENCHANTMENT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "enchantment"]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const IT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["it"]);
const THAT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that"]);
const PREDICATE_REFERENCE_NOUN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["artifact"],
            &["card"],
            &["creature"],
            &["creatures"],
            &["enchantment"],
            &["land"],
            &["object"],
            &["permanent"],
            &["source"],
            &["spell"],
            &["token"],
        ]
);
const OR_COMPARISON_TAIL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["more"], &["fewer"], &["less"], &["greater"], &["equal"]]);
const ITS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["its"], &["it's"]]);
const IT_S_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["it", "s"]);
const YOUR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["your"]);

fn predicate_find_exact_phrase_shape(
    words: &[&str],
    phrase: &[&str],
    shape: &ClauseShape<'static>,
) -> Option<usize> {
    if phrase.is_empty() || words.len() < phrase.len() {
        return None;
    }
    words
        .windows(phrase.len())
        .position(|window| shape.matches_words(window))
}
const THEIR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["their"]);
const HAVE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["have"]);
const YOU_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const MANA_VALUE_HEAD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["mana", "value"]);
const COLORS_SPENT_TO_CAST_SOURCE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "less", "than", "or", "equal", "to", "number", "of", "colors", "of", "mana",
                "spent", "to", "cast", "this", "spell",
            ],
            &[
                "less", "than", "or", "equal", "to", "number", "of", "color", "of", "mana",
                "spent", "to", "cast", "this", "spell",
            ],
        ]
);
const TOTAL_POWER_TOUGHNESS_HEAD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["total", "power", "and", "toughness"]);
const POWER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["power"]);
const POWER_OR_TOUGHNESS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["power"], &["toughness"]]);
const HAS_OR_HAVE_TOXIC_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["has", "toxic"], &["have", "toxic"]]);
const MOST_COMMON_COLOR_AMONG_ALL_PERMANENTS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["most", "common", "color", "among", "all", "permanents"]);
const IS_OR_ARE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["is"], &["are"]]);
const BE_VERB_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["is"], &["are"], &["was"], &["were"]]);
const MANA_SYMBOL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["w"], &["u"], &["b"], &["r"], &["g"], &["c"], &["s"]]);
const SOURCE_FILTER_IGNORED_DESCRIPTOR_WORDS: &[&str] =
    &["attached", "tapped", "untapped", "saddled"];
const AURA_WORDS: &[&str] = &["aura", "auras"];
const CONTROL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["control"]);
const CONTROL_OR_CONTROLS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["control"], &["controls"]]);
const ZONE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["graveyard"], &["hand"], &["exile"], &["library"]]);
const YOUR_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["your", "graveyard"]);
const THAT_PLAYER_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "player", "graveyard"],
            &["that", "players", "graveyard"],
        ]
);
const TARGET_PLAYER_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["target", "player", "graveyard"],
            &["target", "players", "graveyard"],
        ]
);
const TARGET_OPPONENT_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["target", "opponent", "graveyard"],
            &["target", "opponents", "graveyard"],
        ]
);
const OPPONENT_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["opponent", "graveyard"], &["opponents", "graveyard"]]);
const THAT_PLAYER_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "player"]);
const TARGET_PLAYER_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "player"]);
const TARGET_OPPONENT_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "opponent"]);
const EACH_OPPONENT_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["each", "opponent"]);
const A_OR_ANY_PLAYER_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["a", "player"], &["any", "player"]]);
const DEFENDING_PLAYER_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["defending", "player"]);
const ATTACKING_PLAYER_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["attacking", "player"]);
const OPPONENT_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["opponent"], &["opponents"]]);
const PLAYER_WHO_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["player", "who"]);
const PLAYER_SUBJECT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["player"]);
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

fn clause_matches_phrase(clause: LexedClause<'_>, phrase: &[&str]) -> bool {
    LexPattern::new(&[LexPattern::phrase(phrase)]).matches_clause(clause)
}

fn clause_matches_any_phrase(clause: LexedClause<'_>, phrases: &[&[&str]]) -> bool {
    LexPattern::new(&[LexPattern::any_phrase(phrases)]).matches_clause(clause)
}

fn clause_contains_phrase(clause: LexedClause<'_>, phrase: &[&str]) -> bool {
    LexPattern::new(&[LexPattern::phrase(phrase)])
        .find_in_clause(clause)
        .is_some()
}

fn is_source_reference_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(
        clause,
        &[
            &["it"],
            &["its"],
            &["this"],
            &["this", "card"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "object"],
        ],
    )
}

fn is_source_card_reference_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["this"], &["this", "card"]])
}

fn source_zone_from_words(words: &[&str]) -> Option<Zone> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["is", "in"])),
        LexPattern::phrase(&["is", "in"]),
        LexPattern::modifier("zone", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_source_reference_clause(source) {
        return None;
    }

    let zone = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if clause_matches_phrase(zone, &["your", "graveyard"]) {
        return Some(Zone::Graveyard);
    }
    if !is_source_card_reference_clause(source) {
        return None;
    }
    if clause_matches_phrase(zone, &["your", "hand"]) {
        return Some(Zone::Hand);
    }
    if clause_matches_phrase(zone, &["your", "library"]) {
        return Some(Zone::Library);
    }
    if clause_matches_phrase(zone, &["exile"]) {
        return Some(Zone::Exile);
    }
    if clause_matches_any_phrase(zone, &[&["the", "command", "zone"], &["command", "zone"]]) {
        return Some(Zone::Command);
    }
    None
}

fn parse_outlaw_shorthand_filter(words: &[&str]) -> Option<ObjectFilter> {
    let trimmed = strip_leading_article_word_refs(words);
    if !OUTLAW_SHORTHAND_FILTER_PATTERN.matches_words(trimmed) {
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
    words: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let copula_phrases: &[&[&str]] = &[&["is"], &["are"]];
    let enchanted_by_phrase = &["enchanted", "by"];
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilAnyPhrase(copula_phrases)),
        LexPattern::action("copula", LexCaptureKind::OneOf(&["is", "are"])),
        LexPattern::capture(
            "enchanted_by",
            LexCaptureKind::WordCount(enchanted_by_phrase.len()),
        ),
        LexPattern::amount("quantity", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let source = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing source in attachment predicate".to_string())
        })?;
    if !is_source_state_subject_clause(source) {
        return Ok(None);
    }
    let enchanted_by = matched
        .capture_clause("enchanted_by", clause)
        .ok_or_else(|| CardTextError::ParseError("missing enchanted-by phrase".to_string()))?;
    if !clause_matches_phrase(enchanted_by, &["enchanted", "by"]) {
        return Ok(None);
    }
    let attachment = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)
        .ok_or_else(|| CardTextError::ParseError("missing attachment count".to_string()))?;
    let (comparison, used) = parse_attachment_quantity_prefix(attachment.tokens())?;
    let filter_tokens = attachment.tokens().get(used..).unwrap_or_default();
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_attachment_count_filter_tokens(filter_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported attachment-count predicate tail (predicate: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(PredicateAst::SourceHasAttachmentsMatching {
        filter,
        comparison,
        display: words.join(" "),
    }))
}

fn parse_attachment_count_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    parse_object_filter(tokens, false)
        .ok()
        .or_else(|| parse_aura_attachment_filter_clause(LexedClause::new(tokens)))
}

fn parse_aura_attachment_filter_clause(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    const AURA_ATTACHMENT_FILTER_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::object(
            "aura",
            LexCaptureKind::OneOf(AURA_WORDS),
        )]);

    AURA_ATTACHMENT_FILTER_PATTERN
        .matches_clause(clause)
        .then(|| ObjectFilter::default().with_subtype(Subtype::Aura))
}

fn object_filter_has_identity(filter: &ObjectFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.supertypes.is_empty()
        || filter.colors.is_some()
        || filter.token
        || filter.nontoken
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
}

fn parse_source_identity_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let state_phrases: &[&[&str]] = &[
        &["is"],
        &["are"],
        &["isnt"],
        &["isn't"],
        &["arent"],
        &["aren't"],
    ];
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilAnyPhrase(state_phrases)),
        LexPattern::action(
            "state",
            LexCaptureKind::OneOf(&["is", "are", "isnt", "isn't", "arent", "aren't"]),
        ),
        LexPattern::object("descriptor", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_source_reference_clause(source) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    let mut negative = source_identity_copula_is_negative(action);
    let descriptor_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let (descriptor_negative, descriptor_clause) =
        parse_source_identity_descriptor_clause(descriptor_clause)?;
    negative |= descriptor_negative;
    if descriptor_clause.tokens().is_empty() {
        return None;
    }
    if source_identity_descriptor_contains_ignored_state(descriptor_clause) {
        return None;
    }
    let descriptor_words = descriptor_clause.word_refs();
    let filter = parse_object_filter(descriptor_clause.tokens(), false)
        .ok()
        .or_else(|| parse_color_only_object_filter_words(&descriptor_words))?;
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

fn parse_source_identity_descriptor_clause<'a>(
    descriptor: LexedClause<'a>,
) -> Option<(bool, LexedClause<'a>)> {
    const NEGATED_DESCRIPTOR_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("not"),
        LexPattern::object("descriptor", LexCaptureKind::Rest),
    ]);

    if let Some(matched) = NEGATED_DESCRIPTOR_PATTERN.match_clause(descriptor) {
        let descriptor = matched.capture_clause_by_role(LexCaptureRole::Object, descriptor)?;
        return Some((true, descriptor));
    }

    Some((false, descriptor))
}

fn source_identity_copula_is_negative(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["isnt"], &["isn't"], &["arent"], &["aren't"]])
}

fn is_there_are_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["there", "are"])
}

fn source_identity_descriptor_contains_ignored_state(descriptor: LexedClause<'_>) -> bool {
    const IGNORED_SOURCE_DESCRIPTOR_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::any_word(SOURCE_FILTER_IGNORED_DESCRIPTOR_WORDS)]);

    IGNORED_SOURCE_DESCRIPTOR_PATTERN
        .find_in_clause(descriptor)
        .is_some()
}

fn parse_source_keyword_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let action_phrases: &[&[&str]] = &[&["has"], &["have"]];
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["has", "have"])),
        LexPattern::object("keyword", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_source_reference_clause(source) {
        return None;
    }
    let keyword = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let keyword_words = keyword.word_refs();
    let (constraint, consumed) = parse_filter_keyword_constraint_words(&keyword_words)?;
    if consumed != keyword_words.len() {
        return None;
    }
    let mut filter = ObjectFilter::default();
    apply_filter_keyword_constraint(&mut filter, constraint, false);
    Some(PredicateAst::SourceMatches(filter))
}

fn parse_you_life_total_at_most_predicate(
    words: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let have_atoms = [
        LexPattern::subject("player", LexCaptureKind::WordCount(1)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["have"])),
        LexPattern::amount("amount", LexCaptureKind::UntilLastPhrase(&["life"])),
        LexPattern::object("unit", LexCaptureKind::OneOf(&["life"])),
    ];
    if let Some(matched) = LexPattern::new(&have_atoms).match_clause(clause) {
        let player = matched
            .capture_clause_by_role(LexCaptureRole::Subject, clause)
            .ok_or_else(|| {
                CardTextError::ParseError("missing player in life predicate".to_string())
            })?;
        if clause_matches_phrase(player, &["you"]) {
            let amount = matched
                .capture_clause_by_role(LexCaptureRole::Amount, clause)
                .ok_or_else(|| {
                    CardTextError::ParseError("missing amount in life predicate".to_string())
                })?;
            return life_total_at_most_from_amount_tokens(amount.tokens());
        }
    }

    let total_atoms = [
        LexPattern::subject("life_total", LexCaptureKind::WordCount(3)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["is"])),
        LexPattern::amount("amount", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&total_atoms).match_clause(clause) else {
        return Ok(None);
    };
    let subject = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing subject in life predicate".to_string())
        })?;
    if !clause_matches_phrase(subject, &["your", "life", "total"]) {
        return Ok(None);
    }
    let amount = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)
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

fn parse_half_starting_life_total_threshold_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let copula_phrases: &[&[&str]] = &[&["is"], &["are"]];
    let atoms = [
        LexPattern::subject("life_total", LexCaptureKind::UntilAnyPhrase(copula_phrases)),
        LexPattern::action("copula", LexCaptureKind::OneOf(&["is", "are"])),
        LexPattern::condition("threshold", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = parse_life_total_subject_clause(subject)?;
    let threshold = matched.capture_clause_by_role(LexCaptureRole::Condition, clause)?;
    match parse_half_starting_life_total_threshold_clause(threshold)? {
        HalfStartingLifeThreshold::AtMost => {
            Some(PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { player })
        }
        HalfStartingLifeThreshold::LessThan => {
            Some(PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { player })
        }
    }
}

fn parse_life_total_subject_clause(clause: LexedClause<'_>) -> Option<PlayerAst> {
    if clause_matches_phrase(clause, &["your", "life", "total"]) {
        return Some(PlayerAst::You);
    }
    if clause_matches_any_phrase(
        clause,
        &[
            &["their", "life", "total"],
            &["that", "players", "life", "total"],
        ],
    ) {
        return Some(PlayerAst::That);
    }
    if clause_matches_phrase(clause, &["target", "players", "life", "total"]) {
        return Some(PlayerAst::Target);
    }
    if clause_matches_phrase(clause, &["target", "opponents", "life", "total"]) {
        return Some(PlayerAst::TargetOpponent);
    }
    if clause_matches_any_phrase(
        clause,
        &[
            &["opponent", "life", "total"],
            &["opponents", "life", "total"],
        ],
    ) {
        return Some(PlayerAst::Opponent);
    }
    if clause_matches_phrase(clause, &["defending", "players", "life", "total"]) {
        return Some(PlayerAst::Defending);
    }
    if clause_matches_phrase(clause, &["attacking", "players", "life", "total"]) {
        return Some(PlayerAst::Attacking);
    }
    None
}

fn parse_half_starting_life_total_threshold_clause(
    clause: LexedClause<'_>,
) -> Option<HalfStartingLifeThreshold> {
    const AT_MOST_HALF_STARTING_LIFE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["less", "than", "or", "equal", "to"]),
        LexPattern::any_phrase(HALF_STARTING_LIFE_TOTAL_PHRASES),
    ]);
    const LESS_THAN_HALF_STARTING_LIFE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["less", "than"]),
        LexPattern::any_phrase(HALF_STARTING_LIFE_TOTAL_PHRASES),
    ]);

    if AT_MOST_HALF_STARTING_LIFE_PATTERN.matches_clause(clause) {
        Some(HalfStartingLifeThreshold::AtMost)
    } else if LESS_THAN_HALF_STARTING_LIFE_PATTERN.matches_clause(clause) {
        Some(HalfStartingLifeThreshold::LessThan)
    } else {
        None
    }
}

fn parse_source_power_threshold_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_source_possessive_power_threshold_shape(&tokens)
        .or_else(|| parse_source_has_power_threshold_shape(&tokens))
}

fn parse_source_possessive_power_threshold_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["power"])),
        LexPattern::object("stat", LexCaptureKind::OneOf(&["power"])),
        LexPattern::action("copula", LexCaptureKind::OneOf(&["is"])),
        LexPattern::amount("amount", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_source_reference_words(&source_clause.word_refs()) {
        return None;
    }
    let amount_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    source_power_at_least_from_amount_words(&amount_clause.word_refs())
}

fn parse_source_has_power_threshold_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["has"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["has"])),
        LexPattern::object("stat", LexCaptureKind::OneOf(&["power"])),
        LexPattern::amount("amount", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_source_reference_words(&source_clause.word_refs()) {
        return None;
    }
    let amount_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    source_power_at_least_from_amount_words(&amount_clause.word_refs())
}

fn source_power_at_least_from_amount_words(words: &[&str]) -> Option<PredicateAst> {
    let (comparison, used) = predicate_quantity_prefix(words)?;
    if used != words.len() {
        return None;
    }
    let count = comparison_to_at_least_threshold(&comparison)?;
    Some(PredicateAst::SourcePowerAtLeast(count))
}

fn parse_source_simple_state_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_source_bare_state_shape(&tokens).or_else(|| parse_source_copula_state_shape(&tokens))
}

fn parse_source_bare_state_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let state_phrases: &[&[&str]] = &[&["tapped"], &["untapped"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(state_phrases)),
        LexPattern::object("state", LexCaptureKind::OneOf(&["tapped", "untapped"])),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_source_state_subject_clause(subject_clause) {
        return None;
    }
    let state_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    source_state_predicate_from_clause(state_clause, false)
}

fn parse_source_copula_state_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let copula_phrases: &[&[&str]] = &[&["is"], &["isnt"], &["isn't"], &["is", "not"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(copula_phrases)),
        LexPattern::action(
            "copula",
            LexCaptureKind::UntilAnyPhrase(&[&["tapped"], &["untapped"], &["saddled"]]),
        ),
        LexPattern::object(
            "state",
            LexCaptureKind::OneOf(&["tapped", "untapped", "saddled"]),
        ),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_source_state_subject_clause(subject_clause) {
        return None;
    }
    let copula_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    let negative = source_copula_is_negative(copula_clause)?;
    let state_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    source_state_predicate_from_clause(state_clause, negative)
}

fn is_source_state_subject_clause(clause: LexedClause<'_>) -> bool {
    is_source_reference_clause(clause)
}

fn source_copula_is_negative(clause: LexedClause<'_>) -> Option<bool> {
    if clause_matches_phrase(clause, &["is"]) {
        return Some(false);
    }
    if clause_matches_any_phrase(clause, &[&["isnt"], &["isn't"], &["is", "not"]]) {
        return Some(true);
    }
    None
}

fn source_state_predicate_from_clause(
    clause: LexedClause<'_>,
    negative: bool,
) -> Option<PredicateAst> {
    if clause_matches_phrase(clause, &["tapped"]) {
        return if negative {
            Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)))
        } else {
            Some(PredicateAst::SourceIsTapped)
        };
    }
    if clause_matches_phrase(clause, &["untapped"]) {
        return if negative {
            Some(PredicateAst::SourceIsTapped)
        } else {
            Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)))
        };
    }
    if clause_matches_phrase(clause, &["saddled"]) {
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
    const TERMINAL_COUNTER_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::amount(
            "count_and_type",
            LexCaptureKind::UntilAnyPhrase(COUNTER_WORD_PHRASES),
        ),
        LexPattern::any_phrase(COUNTER_WORD_PHRASES),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = TERMINAL_COUNTER_PATTERN.match_clause(clause)?;
    let count_and_type = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let count = parse_number(count_and_type.tokens())
        .map(|(count, _)| count)
        .unwrap_or(1);
    let counter_type = if count_and_type.word_refs().is_empty() {
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
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["has"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["has"])),
        LexPattern::object("counter", LexCaptureKind::UntilPhrase(&["on"])),
        LexPattern::modifier("target", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_source_reference_clause(source_clause) {
        return None;
    }
    let target_clause = matched.capture_clause("target", clause)?;
    if !is_exact_counter_on_source_tail_clause(target_clause) {
        return None;
    }
    let counter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let counter_words = counter_clause.word_refs();
    if matches!(counter_words.as_slice(), ["no", ..]) {
        let counter_type = parse_terminal_counter_phrase(counter_clause.tokens().get(1..)?)??;
        return Some(PredicateAst::SourceHasNoCounter(counter_type));
    }
    if predicate_quantity_prefix(&counter_words).is_some() {
        return None;
    }
    if OR_MORE_PREFIX_PATTERN.matches_words(counter_words.get(1..).unwrap_or_default()) {
        return None;
    }
    let counter_type = parse_terminal_counter_phrase(counter_clause.tokens())??;
    Some(PredicateAst::SourceHasCounterAtLeast {
        counter_type,
        count: 1,
    })
}

fn parse_source_has_counted_counter_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["has"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["has"])),
        LexPattern::object("counter", LexCaptureKind::UntilPhrase(&["on"])),
        LexPattern::modifier("target", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_source_reference_clause(source_clause) {
        return None;
    }
    let target_clause = matched.capture_clause("target", clause)?;
    if !is_counter_on_source_pronoun_tail_clause(target_clause) {
        return None;
    }
    let counter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let counter_words = counter_clause.word_refs();
    let (comparison, used) = predicate_quantity_prefix(&counter_words)?;
    let count = comparison_to_at_least_threshold(&comparison)?;
    let counter_tail = counter_clause.tokens().get(used..)?;
    let counter_type = parse_terminal_counter_phrase(counter_tail)??;
    Some(PredicateAst::SourceHasCounterAtLeast {
        counter_type,
        count,
    })
}

fn is_counter_on_source_pronoun_tail_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(
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

fn parse_there_are_no_counters_on_source_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("existential", LexCaptureKind::WordCount(2)),
        LexPattern::amount("quantity", LexCaptureKind::OneOf(&["no"])),
        LexPattern::object("counter", LexCaptureKind::UntilPhrase(&["on"])),
        LexPattern::modifier("target", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let existential = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_there_are_clause(existential) {
        return None;
    }
    let target = matched.capture_clause("target", clause)?;
    if !is_exact_counter_on_source_tail_clause(target) {
        return None;
    }
    let counter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let counter_type = parse_terminal_counter_phrase(counter_clause.tokens())??;
    Some(PredicateAst::SourceHasNoCounter(counter_type))
}

fn parse_basic_land_types_among_lands_predicate(
    words: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let land_type_phrases: &[&[&str]] = &[
        &["basic", "land", "type", "among", "land"],
        &["basic", "land", "type", "among", "lands"],
        &["basic", "land", "types", "among", "land"],
        &["basic", "land", "types", "among", "lands"],
    ];
    let atoms = [
        LexPattern::subject("existential", LexCaptureKind::WordCount(2)),
        LexPattern::amount("count", LexCaptureKind::UntilAnyPhrase(land_type_phrases)),
        LexPattern::object(
            "land_types",
            LexCaptureKind::UntilAnyPhrase(&[
                &["you", "control"],
                &["you", "controls"],
                &["that", "player", "control"],
                &["that", "player", "controls"],
                &["that", "players", "controls"],
            ]),
        ),
        LexPattern::modifier("controller", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let existential = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing existential in basic-land-types predicate".to_string(),
            )
        })?;
    if !is_there_are_clause(existential) {
        return Ok(None);
    }
    let count_clause = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing count in basic-land-types predicate".to_string())
        })?;
    let count_words = count_clause.word_refs();
    let (comparison, used) = predicate_quantity_prefix(&count_words).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported basic-land-types count (predicate: '{}')",
            words.join(" ")
        ))
    })?;
    if used != count_words.len() {
        return Ok(None);
    }
    let Some(count) = comparison_to_at_least_threshold(&comparison) else {
        return Ok(None);
    };
    let land_types = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in basic-land-types predicate".to_string())
        })?;
    if !clause_matches_any_phrase(land_types, land_type_phrases) {
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
            words.join(" ")
        ))
    })?;
    Ok(Some(
        PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count },
    ))
}

fn parse_basic_land_types_controller_clause(clause: LexedClause<'_>) -> Option<PlayerAst> {
    if clause_matches_any_phrase(clause, &[&["you", "control"], &["you", "controls"]]) {
        return Some(PlayerAst::You);
    }
    if clause_matches_any_phrase(
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
        LexPattern::subject("existential", LexCaptureKind::WordCount(2)),
        LexPattern::object("counter", LexCaptureKind::UntilPhrase(&["on"])),
        LexPattern::modifier("target", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let existential = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_there_are_clause(existential) {
        return None;
    }
    let target = matched.capture_clause("target", clause)?;
    if !is_exact_counter_on_source_tail_clause(target) {
        return None;
    }
    let counter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let counter_words = counter_clause.word_refs();
    let (comparison, used) = predicate_quantity_prefix(&counter_words)?;
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

fn parse_triggering_object_had_counter_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["had"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["had"])),
        LexPattern::object("counter", LexCaptureKind::UntilPhrase(&["on"])),
        LexPattern::modifier("target", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_triggering_object_counter_subject(&subject_clause.word_refs()) {
        return None;
    }
    let target_clause = matched.capture_clause("target", clause)?;
    if !is_exact_counter_on_triggering_object_tail(&target_clause.word_refs()) {
        return None;
    }
    let counter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let counter_words = counter_clause.word_refs();
    if matches!(counter_words.as_slice(), ["no", ..]) {
        let counter_type = parse_terminal_counter_phrase(counter_clause.tokens().get(1..)?)??;
        return Some(PredicateAst::TriggeringObjectHadNoCounter(counter_type));
    }
    let counter_type = parse_terminal_counter_phrase(counter_clause.tokens())??;
    Some(PredicateAst::TriggeringObjectHadCounterAtLeast {
        counter_type,
        count: 1,
    })
}

fn is_triggering_object_counter_subject(words: &[&str]) -> bool {
    matches!(
        words,
        ["it"]
            | ["this", "creature"]
            | ["that", "creature"]
            | ["this", "permanent"]
            | ["that", "permanent"]
    )
}

fn is_exact_counter_on_triggering_object_tail(words: &[&str]) -> bool {
    matches!(
        words,
        ["on", "it"] | ["on", "them"] | ["on", "this"] | ["on", "that"] | ["on", "itself"]
    )
}

fn is_exact_counter_on_source_tail_clause(clause: LexedClause<'_>) -> bool {
    const COUNTER_ON_SOURCE_TAIL_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("on"),
        LexPattern::subject("source", LexCaptureKind::Rest),
    ]);

    let Some(matched) = COUNTER_ON_SOURCE_TAIL_PATTERN.match_clause(clause) else {
        return false;
    };
    let Some(source) = matched.capture_clause_by_role(LexCaptureRole::Subject, clause) else {
        return false;
    };
    is_source_state_subject_clause(source)
}

fn parse_source_exiled_with_counter_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let exiled_with_phrase = &["is", "exiled", "with"];
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(exiled_with_phrase)),
        LexPattern::phrase(exiled_with_phrase),
        LexPattern::object("counter", LexCaptureKind::UntilPhrase(&["on"])),
        LexPattern::modifier("target", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_source_state_subject_clause(source_clause) {
        return None;
    }

    let target_clause = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !is_exact_counter_on_source_tail_clause(target_clause) {
        return None;
    }

    let counter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
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
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["is"])),
        LexPattern::action("copula", LexCaptureKind::OneOf(&["is"])),
        LexPattern::object("role", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_this_source_clause(source) {
        return None;
    }
    let role = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !is_your_ring_bearer_clause(role) {
        return None;
    }
    Some(PredicateAst::SourceIsRingBearer {
        player: PlayerAst::You,
    })
}

fn is_this_source_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["this"], &["this", "creature"]])
}

fn is_your_ring_bearer_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["your", "ring", "bearer"])
}

fn parse_ring_has_tempted_you_this_game_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let article_atoms = [LexPattern::word("the")];
    let atoms = [
        LexPattern::optional(&article_atoms),
        LexPattern::subject("ring", LexCaptureKind::OneOf(&["ring"])),
        LexPattern::action("tempted", LexCaptureKind::WordCount(3)),
        LexPattern::amount("count", LexCaptureKind::UntilPhrase(&["or", "more"])),
        LexPattern::phrase(&["or", "more"]),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let tempted = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !clause_matches_phrase(tempted, &["has", "tempted", "you"]) {
        return None;
    }
    let window = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !clause_matches_any_phrase(
        window,
        &[&["time", "this", "game"], &["times", "this", "game"]],
    ) {
        return None;
    }
    let count_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let (count, used) = parse_number(count_clause.tokens())?;
    if used != count_clause.word_refs().len() {
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
        LexPattern::condition("left", LexCaptureKind::UntilPhrase(&["and"])),
        LexPattern::word("and"),
        LexPattern::condition("right", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let left_clause = matched.capture_clause("left", clause)?;
    let right_clause = matched.capture_clause("right", clause)?;
    if left_clause.word_refs().is_empty() || right_clause.word_refs().is_empty() {
        return None;
    }
    let left = parse_source_is_your_ring_bearer_predicate(left_clause.tokens())?;
    let right = parse_ring_has_tempted_you_this_game_predicate(right_clause.tokens())?;
    Some(PredicateAst::And(Box::new(left), Box::new(right)))
}

fn parse_stack_object_targets_only_source_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("spell", LexCaptureKind::UntilPhrase(&["targets", "only"])),
        LexPattern::action("targets_only", LexCaptureKind::WordCount(2)),
        LexPattern::object("target", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let spell = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_stack_object_reference_clause(spell) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !clause_matches_phrase(action, &["targets", "only"]) {
        return None;
    }

    let target = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let mut target_filter = source_target_filter_from_clause(target)?;
    target_filter.source = true;

    Some(PredicateAst::ItMatches(
        ObjectFilter::spell()
            .targeting_only_object(target_filter)
            .target_count_exact(1),
    ))
}

fn is_stack_object_reference_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["that", "spell"], &["spell"], &["it"]])
}

fn source_target_filter_from_clause(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    if clause_matches_phrase(clause, &["this", "creature"]) {
        return Some(ObjectFilter::creature());
    }
    if clause_matches_phrase(clause, &["this", "artifact"]) {
        return Some(ObjectFilter::artifact());
    }
    if clause_matches_phrase(clause, &["this", "enchantment"]) {
        return Some(ObjectFilter::enchantment());
    }
    if clause_matches_phrase(clause, &["this", "land"]) {
        return Some(ObjectFilter::land());
    }
    if clause_matches_phrase(clause, &["this", "permanent"]) {
        return Some(ObjectFilter::default().in_zone(Zone::Battlefield));
    }
    if clause_matches_any_phrase(clause, &[&["this", "source"], &["it"]]) {
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
            | "spells"
            | "card's"
            | "cards"
            | "creature's"
            | "creatures"
            | "permanent's"
            | "permanents"
    )
}

fn ordinal_number_word(word: &str) -> Option<u32> {
    ironsmith_core::parse_ordinal_word(word).or_else(|| parse_named_number(word))
}

fn predicate_quantity_prefix(words: &[&str]) -> Option<(crate::effect::Comparison, usize)> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_quantity_comparison_prefix(&tokens, false, false, "predicate quantity").ok()
}

fn predicate_number_prefix(words: &[&str]) -> Option<(u32, usize)> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_number(&tokens)
}

fn predicate_at_least_quantity_prefix(words: &[&str]) -> Option<(u32, usize)> {
    if let Some((comparison, used)) = predicate_quantity_prefix(words) {
        let count = comparison_to_strict_at_least_threshold(&comparison)?;
        return Some((count, used));
    }

    let (count, used) = predicate_number_prefix(words)?;
    if words
        .get(used..used + 2)
        .is_some_and(|tail| OR_MORE_PREFIX_PATTERN.matches_words(tail))
    {
        return Some((count, used + 2));
    }

    None
}

fn control_predicate_quantity(
    words: &[&str],
    prefix_len: usize,
) -> (Option<u32>, Option<u32>, usize) {
    let mut filter_start = prefix_len;
    let mut min_count = None;
    let mut exact_count = None;

    if let Some((comparison, used)) =
        predicate_quantity_prefix(words.get(prefix_len..).unwrap_or_default())
    {
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
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    parse_player_controls_zero_quantity_predicate(filtered)
        .or_else(|| parse_player_does_not_control_predicate(filtered))
        .transpose()
}

fn parse_player_controls_zero_quantity_predicate(
    filtered: &[&str],
) -> Option<Result<PredicateAst, CardTextError>> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let action_phrases: &[&[&str]] = &[&["control"], &["controls"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["control", "controls"])),
        LexPattern::amount("amount", LexCaptureKind::WordCount(1)),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let (player, controller) = zero_control_subject_clause(subject_clause)?;
    let amount_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let tagged_neither = zero_control_amount_clause(amount_clause, player)?;
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if object_clause.word_refs().is_empty() {
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
    if clause_matches_phrase(clause, &["you"]) {
        return Some((PlayerAst::You, PlayerFilter::You));
    }
    if clause_matches_phrase(clause, &["player"]) {
        return Some((PlayerAst::Any, PlayerFilter::Any));
    }
    None
}

fn zero_control_amount_clause(clause: LexedClause<'_>, player: PlayerAst) -> Option<bool> {
    if clause_matches_phrase(clause, &["no"]) {
        return Some(false);
    }
    (player == PlayerAst::You && clause_matches_phrase(clause, &["neither"])).then_some(true)
}

fn parse_player_does_not_control_predicate(
    filtered: &[&str],
) -> Option<Result<PredicateAst, CardTextError>> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::WordCount(1)),
        LexPattern::modifier(
            "negation",
            LexCaptureKind::UntilAnyPhrase(&[&["control"], &["controls"]]),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(&["control", "controls"])),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let negation_clause = matched.capture_clause("negation", clause)?;
    if !is_do_not_clause(negation_clause) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if object_clause.word_refs().is_empty() {
        return None;
    }
    let other = object_clause
        .tokens()
        .first()
        .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
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
    clause_matches_phrase(clause, &["you"])
}

fn is_do_not_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["dont"], &["don't"], &["do", "not"]])
}

fn parse_you_control_or_graveyard_predicate(
    filtered: &[&str],
) -> Option<Result<PredicateAst, CardTextError>> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("controller", LexCaptureKind::WordCount(1)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["control", "controls"])),
        LexPattern::object("control_object", LexCaptureKind::UntilPhrase(&["or"])),
        LexPattern::word("or"),
        LexPattern::modifier("graveyard_object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let controller_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_you_clause(controller_clause) {
        return None;
    }

    let control_object = matched.capture_clause("control_object", clause)?;
    if control_object.word_refs().is_empty() {
        return None;
    }

    let graveyard_object = matched.capture_clause("graveyard_object", clause)?;
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
    const THERE_IS_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["there", "is"]),
        LexPattern::object("object", LexCaptureKind::Rest),
    ]);
    const THERE_ARE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["there", "are"]),
        LexPattern::object("object", LexCaptureKind::Rest),
    ]);
    const THERE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("there"),
        LexPattern::object("object", LexCaptureKind::Rest),
    ]);

    let object_clause = THERE_IS_PATTERN
        .match_clause(clause)
        .and_then(|matched| matched.capture_clause_by_role(LexCaptureRole::Object, clause))
        .or_else(|| {
            THERE_ARE_PATTERN
                .match_clause(clause)
                .and_then(|matched| matched.capture_clause_by_role(LexCaptureRole::Object, clause))
        })
        .or_else(|| {
            THERE_PATTERN
                .match_clause(clause)
                .and_then(|matched| matched.capture_clause_by_role(LexCaptureRole::Object, clause))
        })
        .unwrap_or(clause);
    let object_clause = object_clause.trimmed();
    (!object_clause.tokens().is_empty()
        && clause_contains_phrase(object_clause, &["your", "graveyard"]))
    .then_some(object_clause.tokens())
}

fn parse_you_control_conjoined_predicate(
    filtered: &[&str],
) -> Option<Result<PredicateAst, CardTextError>> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("controller", LexCaptureKind::WordCount(1)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["control", "controls"])),
        LexPattern::object("left_object", LexCaptureKind::UntilPhrase(&["and"])),
        LexPattern::word("and"),
        LexPattern::object("right_object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let controller_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_you_clause(controller_clause) {
        return None;
    }

    let left_object = matched.capture_clause("left_object", clause)?;
    let right_object = matched.capture_clause("right_object", clause)?;
    if left_object.word_refs().is_empty() || right_object.word_refs().is_empty() {
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

fn parse_player_controls_predicate(
    words: &[&str],
    player: PlayerAst,
    controller: Option<PlayerFilter>,
    prefix_len: usize,
    allow_outlaw_shorthand: bool,
    allow_different_powers: bool,
) -> Result<Option<PredicateAst>, CardTextError> {
    let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    if let Some(control_condition) =
        crate::runtime_backend::grammar::conditions::parse_control_condition(
            &control_tokens,
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

    let (min_count, exact_count, filter_start) = control_predicate_quantity(words, prefix_len);
    let mut control_words = words[filter_start..].to_vec();
    if control_words.is_empty() {
        return Ok(None);
    }

    let mut requires_different_powers = false;
    if allow_different_powers
        && WITH_DIFFERENT_POWERS_TAIL_PATTERN
            .matches_words(&control_words[control_words.len().saturating_sub(3)..])
    {
        requires_different_powers = true;
        control_words.truncate(control_words.len().saturating_sub(3));
    }

    let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&control_words);
    let other = control_tokens
        .first()
        .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
    let parsed_filter = parse_object_filter(&control_tokens, other).or_else(|_| {
        if allow_outlaw_shorthand {
            parse_outlaw_shorthand_filter(&control_words)
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
    match control_condition.comparison {
        crate::effect::Comparison::Equal(count) if count >= 0 => {
            PredicateAst::PlayerControlsExactly {
                player: control_condition.player,
                filter: control_condition.filter,
                count: count as u32,
            }
        }
        _ => {
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
    }
}

fn parse_this_ability_resolution_count_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let count = match filtered {
        [
            "this",
            "is",
            count,
            "time",
            "this",
            "ability",
            "has",
            "resolved",
            "this",
            "turn",
        ]
        | [
            "this",
            "is",
            count,
            "time",
            "this",
            "ability",
            "resolved",
            "this",
            "turn",
        ]
        | [
            "this",
            "ability",
            "has",
            "resolved",
            "for",
            count,
            "time",
            "this",
            "turn",
        ]
        | [
            "this",
            "ability",
            "resolved",
            "for",
            count,
            "time",
            "this",
            "turn",
        ] => ordinal_number_word(count)?,
        ["it's", count, "time"] | ["its", count, "time"] | ["it", "s", count, "time"] => {
            ordinal_number_word(count)?
        }
        _ => return None,
    };

    Some(PredicateAst::ThisAbilityResolvedThisTurnExactly(count))
}

fn predicate_tokens_from_words(words: &[&str]) -> Vec<OwnedLexToken> {
    crate::runtime_backend::lexer::synthetic_word_tokens(words)
}

fn parse_color_only_object_filter_clause(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    let mut filter = ObjectFilter::default();
    let mut saw_color = false;
    for word in clause.word_refs() {
        if AND_WORD_PATTERN.matches_word(word) || OR_WORD_PATTERN.matches_word(word) {
            continue;
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
            saw_color = true;
            continue;
        }
        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            saw_color = true;
            continue;
        }
        return None;
    }
    saw_color.then_some(filter)
}

fn parse_color_only_object_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let tokens = predicate_tokens_from_words(words);
    parse_color_only_object_filter_clause(LexedClause::new(&tokens))
}

fn strip_clause_suffix<'a>(
    clause: LexedClause<'a>,
    suffix: &'static [&'static str],
) -> Option<LexedClause<'a>> {
    let atoms = [
        LexPattern::object("base", LexCaptureKind::UntilLastPhrase(suffix)),
        LexPattern::phrase(suffix),
    ];
    LexPattern::new(&atoms)
        .match_clause(clause)
        .and_then(|matched| matched.capture_clause_by_role(LexCaptureRole::Object, clause))
        .map(LexedClause::trimmed)
}

fn parse_this_way_object_filter_clause(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    let clause = clause.trimmed();
    let (base_clause, needs_chosen_name) =
        if let Some(base_clause) = strip_clause_suffix(clause, &["with", "chosen", "name"]) {
            (base_clause, true)
        } else {
            (clause, false)
        };
    let has_card_noun = base_clause
        .tokens()
        .last()
        .is_some_and(|token| CARD_OR_CARDS_WORD_PATTERN.matches_word(token.parser_text()));
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
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
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
        LexPattern::object("object", LexCaptureKind::UntilLastAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::WordCount(2)),
        LexPattern::phrase(&["this", "way"]),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let filter_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in passive this-way predicate".to_string())
        })?;
    let filter_words = filter_clause.word_refs();
    if filter_words.is_empty() {
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
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let action_phrases: &[&[&str]] = &[&["discard"], &["discards"], &["discarded"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action(
            "action",
            LexCaptureKind::OneOf(&["discard", "discards", "discarded"]),
        ),
        LexPattern::object("object", LexCaptureKind::UntilPhrase(&["this", "way"])),
        LexPattern::phrase(&["this", "way"]),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let subject_clause = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing subject in active this-way predicate".to_string())
        })?;
    let Some(player) = active_discard_player_subject_clause(subject_clause) else {
        return Ok(None);
    };
    let filter_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in active this-way predicate".to_string())
        })?;
    let filter_words = filter_clause.word_refs();
    if filter_words.is_empty() {
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

fn parse_negative_put_tagged_object_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let destination_phrases: &[&[&str]] = &[
        &["into", "your", "hand"],
        &["onto", "battlefield"],
        &["onto", "the", "battlefield"],
    ];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::WordCount(1)),
        LexPattern::modifier("negation", LexCaptureKind::UntilPhrase(&["put"])),
        LexPattern::action("action", LexCaptureKind::WordCount(1)),
        LexPattern::object(
            "object",
            LexCaptureKind::UntilAnyPhrase(destination_phrases),
        ),
        LexPattern::modifier("destination", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let negation_clause = matched.capture_clause("negation", clause)?;
    if !is_do_or_did_not_clause(negation_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !clause_matches_phrase(action_clause, &["put"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
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
    clause_matches_any_phrase(
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
    clause_matches_any_phrase(
        clause,
        &[&["the", "card"], &["that", "card"], &["card"], &["it"]],
    )
}

fn tagged_put_destination_zone(clause: LexedClause<'_>) -> Option<Zone> {
    if clause_matches_phrase(clause, &["into", "your", "hand"]) {
        return Some(Zone::Hand);
    }
    if clause_matches_any_phrase(
        clause,
        &[&["onto", "battlefield"], &["onto", "the", "battlefield"]],
    ) {
        return Some(Zone::Battlefield);
    }
    None
}

fn is_battlefield_this_way_destination_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(
        clause,
        &[
            &["onto", "battlefield", "this", "way"],
            &["onto", "the", "battlefield", "this", "way"],
        ],
    )
}

fn parse_active_this_way_battlefield_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let destination_phrases: &[&[&str]] =
        &[&["onto", "battlefield"], &["onto", "the", "battlefield"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["put"])),
        LexPattern::action("action", LexCaptureKind::WordCount(1)),
        LexPattern::object(
            "object",
            LexCaptureKind::UntilAnyPhrase(destination_phrases),
        ),
        LexPattern::modifier("destination", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let subject_clause = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)
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
        .capture_clause_by_role(LexCaptureRole::Object, clause)
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
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::object("object", LexCaptureKind::UntilPhrase(&["is", "put"])),
        LexPattern::action("action", LexCaptureKind::WordCount(2)),
        LexPattern::modifier("destination", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let action_clause = matched
        .capture_clause_by_role(LexCaptureRole::Action, clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing action in passive this-way battlefield predicate".to_string(),
            )
        })?;
    if !clause_matches_phrase(action_clause, &["is", "put"]) {
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
        .capture_clause_by_role(LexCaptureRole::Object, clause)
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
    if clause_matches_phrase(clause, &["you"]) {
        return Some(PlayerAst::You);
    }
    if clause_matches_any_phrase(clause, &[&["that", "player"], &["that", "players"]]) {
        return Some(PlayerAst::That);
    }
    if clause_matches_phrase(clause, &["target", "player"]) {
        return Some(PlayerAst::Target);
    }
    if clause_matches_phrase(clause, &["target", "opponent"]) {
        return Some(PlayerAst::TargetOpponent);
    }
    if clause_matches_any_phrase(clause, &[&["opponent"], &["opponents"]]) {
        return Some(PlayerAst::Opponent);
    }
    None
}

fn parse_repeated_if_or_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::object("left", LexCaptureKind::UntilPhrase(&["or", "if"])),
        LexPattern::phrase(&["or", "if"]),
        LexPattern::modifier("right", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };

    let left_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing left side in or-if predicate".to_string())
        })?;
    let right_clause = matched
        .capture_clause_by_role(LexCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing right side in or-if predicate".to_string())
        })?;
    if left_clause.word_refs().is_empty() || right_clause.word_refs().is_empty() {
        return Ok(None);
    }

    let left = match parse_predicate(left_clause.tokens()) {
        Ok(predicate) => predicate,
        Err(_) => return Ok(None),
    };
    let right = parse_predicate(right_clause.tokens())?;
    Ok(Some(PredicateAst::Or(Box::new(left), Box::new(right))))
}

fn predicate_reference_prefix<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    if words
        .first()
        .is_some_and(|word| IT_WORD_PATTERN.matches_word(word))
    {
        return Some(&words[..1]);
    }
    if words.len() >= 2
        && THAT_WORD_PATTERN.matches_word(words[0])
        && PREDICATE_REFERENCE_NOUN_WORD_PATTERN.matches_word(words[1])
    {
        return Some(&words[..2]);
    }
    None
}

fn predicate_words_start_with_reference(words: &[&str]) -> bool {
    matches!(
        words.first().copied(),
        Some(
            "it" | "its"
                | "this"
                | "that"
                | "you"
                | "your"
                | "opponent"
                | "player"
                | "target"
                | "source"
        )
    )
}

fn parse_single_card_type_card_descriptor(words: &[&str]) -> Option<ObjectFilter> {
    if matches!(words, ["permanent", "card"] | ["permanent", "cards"]) {
        return Some(ObjectFilter::permanent_card());
    }
    if words.len() == 2
        && CARD_OR_CARDS_WORD_PATTERN.matches_word(words[1])
        && let Some(card_type) = parse_card_type(words[0])
    {
        return Some(ObjectFilter {
            card_types: vec![card_type],
            ..Default::default()
        });
    }
    None
}

fn parse_or_predicate(filtered: &[&str]) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::object("left", LexCaptureKind::UntilLastPhrase(&["or"])),
        LexPattern::word("or"),
        LexPattern::modifier("right", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let left_clause = matched
        .capture_clause("left", clause)
        .ok_or_else(|| CardTextError::ParseError("missing left or-predicate".to_string()))?;
    let right_clause = matched
        .capture_clause("right", clause)
        .ok_or_else(|| CardTextError::ParseError("missing right or-predicate".to_string()))?;
    let left_words = left_clause.word_refs();
    let right_words = right_clause.word_refs();
    if left_words.is_empty()
        || right_words.is_empty()
        || right_words
            .first()
            .is_some_and(|word| OR_COMPARISON_TAIL_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let left = parse_predicate(left_clause.tokens())?;
    let right = match parse_predicate(right_clause.tokens()) {
        Ok(predicate) => predicate,
        Err(original_err) => {
            let Some(reference_prefix) = predicate_reference_prefix(left_words.as_slice()) else {
                return Err(original_err);
            };
            if predicate_words_start_with_reference(right_words.as_slice()) {
                return Err(original_err);
            }
            let prefixed_words = reference_prefix
                .iter()
                .copied()
                .chain(right_words.iter().copied())
                .collect::<Vec<_>>();
            let prefixed_tokens = predicate_tokens_from_words(&prefixed_words);
            parse_predicate(&prefixed_tokens).map_err(|_| original_err)?
        }
    };
    Ok(Some(PredicateAst::Or(Box::new(left), Box::new(right))))
}

fn parse_attacking_you_own_control_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::object("left", LexCaptureKind::UntilPhrase(&["and"])),
        LexPattern::word("and"),
        LexPattern::object(
            "right",
            LexCaptureKind::UntilPhrase(&[
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
        LexPattern::phrase(&[
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
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
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
            filtered.join(" ")
        ))
    })?;
    left_filter.controller = Some(PlayerFilter::You);
    left_filter.attacking = true;

    let mut right_filter = parse_meld_subject_filter_clause(right).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported attacking meld predicate tail (predicate: '{}')",
            filtered.join(" ")
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
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("owner", LexCaptureKind::WordCount(4)),
        LexPattern::action("control", LexCaptureKind::OneOf(&["control", "controls"])),
        LexPattern::object("left", LexCaptureKind::UntilPhrase(&["and"])),
        LexPattern::word("and"),
        LexPattern::object("right", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let owner = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing owner in own/control predicate".to_string())
        })?;
    if !is_you_both_own_and_clause(owner) {
        return Ok(None);
    }
    let left = matched.capture_clause("left", clause).ok_or_else(|| {
        CardTextError::ParseError("missing left subject in own/control predicate".to_string())
    })?;
    let right = matched.capture_clause("right", clause).ok_or_else(|| {
        CardTextError::ParseError("missing right subject in own/control predicate".to_string())
    })?;
    if left.tokens().is_empty() || right.tokens().is_empty() {
        return Ok(None);
    }

    let mut left_filter = parse_meld_subject_filter_clause(left).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported own-and-control predicate subject (predicate: '{}')",
            filtered.join(" ")
        ))
    })?;
    left_filter.controller = Some(PlayerFilter::You);
    let mut right_filter = parse_meld_subject_filter_clause(right).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported own-and-control predicate tail (predicate: '{}')",
            filtered.join(" ")
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
    clause_matches_phrase(clause, &["you", "both", "own", "and"])
}

fn parse_implicit_subject_and_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::object("left", LexCaptureKind::UntilPhrase(&["and"])),
        LexPattern::word("and"),
        LexPattern::modifier("right", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let left_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing left side in and predicate".to_string())
        })?;
    let right_clause = matched
        .capture_clause_by_role(LexCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing right side in and predicate".to_string())
        })?;
    let right_words = right_clause.word_refs();
    if left_clause.word_refs().is_empty() || right_words.is_empty() {
        return Ok(None);
    }
    let Some(right_first) = right_words.first().copied() else {
        return Ok(None);
    };
    if !HAVE_WORD_PATTERN.matches_word(right_first) && !YOU_WORD_PATTERN.matches_word(right_first) {
        return Ok(None);
    }

    let left = parse_predicate(left_clause.tokens())?;
    let right_tokens = if HAVE_WORD_PATTERN.matches_word(right_first) {
        let mut words = Vec::with_capacity(right_words.len() + 1);
        words.push("you");
        words.extend(right_words.iter().copied());
        crate::runtime_backend::lexer::synthetic_word_tokens(words)
    } else {
        right_clause.tokens().to_vec()
    };
    let right = parse_predicate(&right_tokens)?;
    Ok(Some(PredicateAst::And(Box::new(left), Box::new(right))))
}

fn parse_while_conjoined_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::object("left", LexCaptureKind::UntilPhrase(&["while"])),
        LexPattern::word("while"),
        LexPattern::modifier("right", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let left_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing left side in while predicate".to_string())
        })?;
    let right_clause = matched
        .capture_clause_by_role(LexCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing right side in while predicate".to_string())
        })?;
    if left_clause.word_refs().is_empty() || right_clause.word_refs().is_empty() {
        return Ok(None);
    }

    let left = parse_predicate(left_clause.tokens())?;
    let right = parse_predicate(right_clause.tokens())?;
    if matches!(
        left,
        PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
            | PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(_)
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-spent predicate tail (predicate: '{}')",
            filtered.join(" ")
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

fn parse_player_status_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let status =
        crate::runtime_backend::grammar::conditions::parse_player_status_condition(&tokens)?;
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

fn parse_world_state_or_timing_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_initiative_choice_predicate_shape(&tokens)
        .or_else(|| parse_night_state_predicate_shape(&tokens))
        .or_else(|| parse_first_combat_phase_predicate_shape(&tokens))
        .or_else(|| parse_cast_this_spell_during_main_phase_shape(&tokens))
}

fn parse_empty_battlefield_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::amount("quantity", LexCaptureKind::OneOf(&["no"])),
        LexPattern::object("object", LexCaptureKind::OneOf(&["creature", "creatures"])),
        LexPattern::action("copula", LexCaptureKind::OneOf(&["is", "are"])),
        LexPattern::word("on"),
        LexPattern::modifier("zone", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let zone = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !is_battlefield_zone_clause(zone) {
        return None;
    }
    Some(PredicateAst::PlayerControlsNo {
        player: PlayerAst::Any,
        filter: ObjectFilter::creature(),
    })
}

fn is_battlefield_zone_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["battlefield"], &["the", "battlefield"]])
}

fn parse_initiative_choice_predicate_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrase = &["has"];
    let atoms = [
        LexPattern::subject("first_player", LexCaptureKind::OneOf(&["you"])),
        LexPattern::word("or"),
        LexPattern::subject("second_player", LexCaptureKind::UntilPhrase(action_phrase)),
        LexPattern::action(
            "status_verb",
            LexCaptureKind::WordCount(action_phrase.len()),
        ),
        LexPattern::object("status", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let second_player = matched.capture_clause("second_player", clause)?;
    if !is_player_youre_attacking_clause(second_player) {
        return None;
    }
    let status = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
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
    clause_matches_any_phrase(
        clause,
        &[
            &["player", "youre", "attacking"],
            &["a", "player", "youre", "attacking"],
        ],
    )
}

fn is_initiative_status_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["initiative"], &["the", "initiative"]])
}

fn parse_night_state_predicate_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let copula = [LexPattern::action("copula", LexCaptureKind::OneOf(&["is"]))];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::OneOf(&["it", "its"])),
        LexPattern::optional(&copula),
        LexPattern::object("state", LexCaptureKind::OneOf(&["night"])),
    ];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(PredicateAst::ItIsNight)
}

fn parse_first_combat_phase_predicate_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let copula = [LexPattern::action("copula", LexCaptureKind::OneOf(&["is"]))];
    let article = [LexPattern::word("the")];
    let tail_article = [LexPattern::word("the")];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::OneOf(&["it", "its"])),
        LexPattern::optional(&copula),
        LexPattern::optional(&article),
        LexPattern::object("phase", LexCaptureKind::WordCount(3)),
        LexPattern::word("of"),
        LexPattern::optional(&tail_article),
        LexPattern::modifier("turn", LexCaptureKind::OneOf(&["turn"])),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let phase = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !is_first_combat_phase_clause(phase) {
        return None;
    }
    Some(PredicateAst::FirstCombatPhaseOfTurn)
}

fn is_first_combat_phase_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["first", "combat", "phase"])
}

fn parse_cast_this_spell_during_main_phase_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let during_phrase = &["during"];
    let atoms = [
        LexPattern::subject("player", LexCaptureKind::OneOf(&["you"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["cast"])),
        LexPattern::object("spell", LexCaptureKind::UntilPhrase(during_phrase)),
        LexPattern::word("during"),
        LexPattern::modifier("phase", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !clause_matches_phrase(object, &["this", "spell"]) {
        return None;
    }
    let phase = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !is_your_main_phase_clause(phase) {
        return None;
    }
    Some(PredicateAst::ThisSpellPaidLabel(
        "CastDuringYourMainPhase".to_string(),
    ))
}

fn is_your_main_phase_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["your", "main", "phase"])
}

fn parse_player_achievement_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let achievement =
        crate::runtime_backend::grammar::conditions::parse_player_achievement_condition(&tokens)?;
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

fn parse_player_cards_in_hand_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_cards_in_hand_condition(&tokens)?;
    let player = player_ast_from_status_player_filter(condition.player.clone())?;
    if player == PlayerAst::You && condition.is_no_cards_in_hand() {
        return Some(PredicateAst::YouHaveNoCardsInHand);
    }
    match condition.comparison {
        crate::effect::Comparison::GreaterThanOrEqual(count) if count >= 0 => {
            Some(PredicateAst::PlayerCardsInHandOrMore {
                player,
                count: count as u32,
            })
        }
        crate::effect::Comparison::GreaterThan(count) if count >= -1 => {
            Some(PredicateAst::PlayerCardsInHandOrMore {
                player,
                count: (count + 1) as u32,
            })
        }
        crate::effect::Comparison::LessThanOrEqual(count) if count >= 0 => {
            Some(PredicateAst::PlayerCardsInHandOrFewer {
                player,
                count: count as u32,
            })
        }
        crate::effect::Comparison::LessThan(count) if count > 0 => {
            Some(PredicateAst::PlayerCardsInHandOrFewer {
                player,
                count: (count - 1) as u32,
            })
        }
        _ => None,
    }
}

fn parse_player_life_total_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_life_total_condition(&tokens)?;
    let (operator, amount) = comparison_to_value_comparison_operator(condition.comparison)?;
    Some(PredicateAst::ValueComparison {
        left: crate::effect::Value::LifeTotal(condition.player),
        operator,
        right: crate::effect::Value::Fixed(amount),
    })
}

fn parse_player_life_relation_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let relation =
        crate::runtime_backend::grammar::conditions::parse_player_life_relation_condition(&tokens)?;
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

fn parse_player_cards_in_hand_relation_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let relation =
        crate::runtime_backend::grammar::conditions::parse_player_cards_in_hand_relation_condition(
            &tokens,
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

fn parse_player_turn_event_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_turn_event_condition(&tokens)?;
    let (operator, count) = comparison_to_value_comparison_operator(condition.comparison)?;
    let left = match condition.event {
        crate::runtime_backend::grammar::conditions::PlayerTurnEventAst::CardsDrawn => {
            Value::MaxCardsDrawnThisTurn(condition.player)
        }
        crate::runtime_backend::grammar::conditions::PlayerTurnEventAst::LandsEnteredBattlefieldUnderControl => {
            Value::LandsEnteredBattlefieldThisTurn(condition.player)
        }
    };

    Some(PredicateAst::ValueComparison {
        left,
        operator,
        right: Value::Fixed(count),
    })
}

fn parse_turn_timing_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let subject = [LexPattern::subject(
        "subject",
        LexCaptureKind::OneOf(&["it"]),
    )];
    let copula = [LexPattern::action("copula", LexCaptureKind::OneOf(&["is"]))];
    let negation = [LexPattern::modifier(
        "negation",
        LexCaptureKind::OneOf(&["not"]),
    )];
    let atoms = [
        LexPattern::optional(&subject),
        LexPattern::optional(&copula),
        LexPattern::optional(&negation),
        LexPattern::object("turn", LexCaptureKind::WordCount(2)),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    if matched.capture("copula").is_some() && matched.capture("subject").is_none() {
        return None;
    }
    let turn_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
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
    clause_matches_phrase(clause, &["your", "turn"])
}

fn parse_opponent_controls_tagged_object_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("controller", LexCaptureKind::UntilPhrase(&["controls"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["controls"])),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let controller = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_opponent_controller_clause(controller) {
        return None;
    }
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let mut filter = ObjectFilter {
        controller: Some(PlayerFilter::Opponent),
        ..Default::default()
    };
    match controlled_tagged_object_kind(object)? {
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
    clause_matches_any_phrase(clause, &[&["opponent"], &["an", "opponent"]])
}

fn controlled_tagged_object_kind(clause: LexedClause<'_>) -> Option<ControlledTaggedObjectKind> {
    if clause_matches_any_phrase(clause, &[&["it"], &["that", "permanent"]]) {
        return Some(ControlledTaggedObjectKind::Permanent);
    }
    if clause_matches_phrase(clause, &["that", "creature"]) {
        return Some(ControlledTaggedObjectKind::Creature);
    }
    None
}

fn parse_secret_choices_match_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("choices", LexCaptureKind::UntilPhrase(&["match"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["match"])),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_secret_choices_subject_clause(subject) {
        return None;
    }
    Some(PredicateAst::SecretChoicesMatch)
}

fn is_secret_choices_subject_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["they"], &["those", "choices"]])
}

fn parse_vote_result_predicate(
    words: &[&str],
    allow_tied: bool,
) -> Result<Option<PredicateAst>, CardTextError> {
    if let Some(predicate) = parse_vote_option_result_predicate(words, allow_tied) {
        return Ok(Some(predicate));
    }
    parse_no_vote_objects_matched_predicate(words)
}

fn parse_x_value_comparison_predicate(words: &[&str]) -> Option<PredicateAst> {
    if let ["x", "is", tail @ ..] = words {
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

    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::amount("subject", LexCaptureKind::OneOf(&["x"])),
        LexPattern::action("copula", LexCaptureKind::OneOf(&["is"])),
        LexPattern::modifier("comparison", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let comparison_clause = matched.capture_clause("comparison", clause)?;
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

fn parse_paid_cost_label_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let paid_tail_phrases: &[&[&str]] = &[
        &["cost", "was", "paid"],
        &["cost", "wasnt", "paid"],
        &["cost", "was", "not", "paid"],
    ];
    let atoms = [
        LexPattern::object("label", LexCaptureKind::UntilAnyPhrase(paid_tail_phrases)),
        LexPattern::action("paid_tail", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let label_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let mut label_words = label_clause.word_refs();
    if label_words.first().copied() == Some("the") {
        label_words.remove(0);
    }
    let paid_tail = matched.capture_clause("paid_tail", clause)?;
    let negated = paid_cost_tail_is_negated(paid_tail)?;
    let label = match label_words.as_slice() {
        ["this", possessive, label] if is_this_spell_possessive_word(possessive) => {
            named_paid_cost_label_from_word(label)?
        }
        words => mana_cost_label_from_words(words)?,
    };
    let predicate = PredicateAst::ThisSpellPaidLabel(label);
    if negated {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

fn paid_cost_tail_is_negated(clause: LexedClause<'_>) -> Option<bool> {
    if clause_matches_phrase(clause, &["cost", "was", "paid"]) {
        return Some(false);
    }
    if clause_matches_any_phrase(
        clause,
        &[&["cost", "wasnt", "paid"], &["cost", "was", "not", "paid"]],
    ) {
        return Some(true);
    }
    None
}

fn parse_vote_option_result_predicate(words: &[&str], allow_tied: bool) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("option", LexCaptureKind::UntilPhrase(&["gets"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["gets"])),
        LexPattern::object("result", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let option = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if option.word_refs().is_empty() {
        return None;
    }
    let result = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let option = option.word_refs().join(" ");
    if clause_matches_phrase(result, &["more", "votes"]) {
        return Some(PredicateAst::VoteOptionGetsMoreVotes { option });
    }
    if allow_tied && clause_matches_phrase(result, &["more", "votes", "or", "vote", "is", "tied"]) {
        return Some(PredicateAst::VoteOptionGetsMoreVotesOrTied { option });
    }
    None
}

fn parse_no_vote_objects_matched_predicate(
    words: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::amount("quantity", LexCaptureKind::OneOf(&["no"])),
        LexPattern::object("objects", LexCaptureKind::UntilPhrase(&["got", "votes"])),
        LexPattern::action("action", LexCaptureKind::WordCount(2)),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let action = matched
        .capture_clause_by_role(LexCaptureRole::Action, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing action in vote result predicate".to_string())
        })?;
    if !clause_matches_phrase(action, &["got", "votes"]) {
        return Ok(None);
    }
    let objects = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in vote result predicate".to_string())
        })?;
    let filter = parse_object_filter(objects.tokens(), false)?;
    Ok(Some(PredicateAst::NoVoteObjectsMatched { filter }))
}

fn parse_spell_context_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_spell_context_condition(&tokens)?;
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

fn parse_player_spell_cast_this_turn_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_spell_cast_this_turn_condition(
            &tokens,
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

fn parse_player_life_change_this_turn_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_life_change_this_turn_condition(
            &tokens,
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

fn parse_object_death_this_turn_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_object_death_this_turn_condition(
            &tokens,
        )?;
    match condition.event {
        crate::runtime_backend::grammar::conditions::ObjectDeathThisTurnEventAst::Died => {
            let count = comparison_to_strict_at_least_threshold(&condition.comparison)?;
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

fn parse_player_would_action_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_would_action_condition(&tokens)?;
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

fn parse_battlefield_entry_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_battlefield_entry_condition(&tokens)?;
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

fn parse_battlefield_change_this_turn_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_battlefield_change_this_turn_condition(
            &tokens,
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
        crate::runtime_backend::grammar::conditions::BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefieldUnderYourControl => {
            Some(PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn)
        }
        crate::runtime_backend::grammar::conditions::BattlefieldChangeThisTurnConditionAst::ObjectPutIntoGraveyardFromBattlefield {
            filter,
        } => Some(PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter)),
    }
}

fn parse_combat_damage_this_turn_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_source_dealt_combat_damage_this_turn_shape(&tokens)
        .or_else(|| parse_player_dealt_combat_damage_by_subtype_this_turn_shape(&tokens))
}

fn is_player_object_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["player"], &["a", "player"]])
}

fn combat_damage_player_subject_clause(clause: LexedClause<'_>) -> Option<PlayerAst> {
    if clause_matches_any_phrase(clause, &[&["a", "player"], &["player"]]) {
        return Some(PlayerAst::Any);
    }
    if clause_matches_any_phrase(clause, &[&["an", "opponent"], &["opponent"]]) {
        return Some(PlayerAst::Opponent);
    }
    None
}

fn single_subtype_word_clause(clause: LexedClause<'_>) -> Option<&str> {
    let words = clause.word_refs();
    match words.as_slice() {
        [word] => Some(*word),
        ["a" | "an", word] => Some(*word),
        _ => None,
    }
}

fn is_this_turn_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["this", "turn"])
}

fn is_this_combat_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["this", "combat"])
}

fn is_attacked_action_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["attacked"])
}

fn is_triggering_attack_subject_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["that", "creature"], &["it"]])
}

fn is_other_creatures_this_combat_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(
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
    clause_matches_any_phrase(
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
    clause_matches_phrase(clause, &["attacked", "or", "blocked"])
}

fn is_source_did_not_attack_subject_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["this", "creature"])
}

fn is_entered_under_your_control_tail_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(
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
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(action_phrase)),
        LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
        LexPattern::object("object", LexCaptureKind::UntilPhrase(&["this", "turn"])),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !clause_matches_phrase(subject_clause, &["it"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
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
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(action_phrase)),
        LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
        LexPattern::object("subtype", LexCaptureKind::UntilPhrase(&["this", "turn"])),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = combat_damage_player_subject_clause(subject_clause)?;
    let subtype_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let subtype_word = single_subtype_word_clause(subtype_clause)?;
    let subtype = parse_subtype_word(subtype_word)?;
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_this_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype })
}

fn parse_combat_turn_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_you_attacked_this_turn_shape(&tokens)
        .or_else(|| parse_triggering_object_had_to_attack_this_combat_shape(&tokens))
        .or_else(|| parse_you_attacked_with_exactly_other_creatures_shape(&tokens))
        .or_else(|| parse_source_attacked_or_blocked_this_turn_shape(&tokens))
}

fn parse_you_attacked_this_turn_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["attacked"])),
        LexPattern::action("action", LexCaptureKind::WordCount(1)),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
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
            LexPattern::subject("subject", LexCaptureKind::UntilPhrase(action_phrase)),
            LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
            LexPattern::modifier("window", LexCaptureKind::Rest),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
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
        LexPattern::subject("subject", LexCaptureKind::WordCount(1)),
        LexPattern::action("action", LexCaptureKind::WordCount(3)),
        LexPattern::amount("count", LexCaptureKind::UntilAnyPhrase(tail_phrases)),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !clause_matches_phrase(action_clause, &["attacked", "with", "exactly"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !is_other_creatures_this_combat_clause(object_clause) {
        return None;
    }
    let count_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
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
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilPhrase(&["attacked", "or", "blocked"]),
        ),
        LexPattern::action("action", LexCaptureKind::WordCount(3)),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_source_attacked_or_blocked_subject_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
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
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["didnt", "attack"])),
        LexPattern::modifier("negation", LexCaptureKind::OneOf(&["didnt"])),
        LexPattern::action("attack", LexCaptureKind::OneOf(&["attack"])),
        LexPattern::modifier(
            "enter",
            LexCaptureKind::UntilAnyPhrase(&[&["this", "turn"]]),
        ),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
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

fn parse_spell_lifecycle_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_you_cast_source_shape(&tokens)
        .or_else(|| parse_tagged_was_cast_shape(&tokens))
        .or_else(|| parse_this_spell_was_cast_from_shape(&tokens))
        .or_else(|| parse_no_spells_cast_last_turn_shape(&tokens))
        .or_else(|| parse_this_spell_paid_named_label_shape(&tokens))
        .or_else(|| parse_target_was_kicked_shape(&tokens))
}

fn is_cast_action_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["cast"])
}

fn is_source_spell_object_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["it"], &["this", "spell"]])
}

fn is_tagged_cast_subject_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(
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
    clause_matches_phrase(clause, &["was", "cast"])
}

fn is_this_spell_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["this", "spell"])
}

fn is_was_cast_from_action_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["was", "cast", "from"])
}

fn spell_cast_origin_zone_clause(clause: LexedClause<'_>) -> Option<Zone> {
    if clause_matches_phrase(clause, &["anywhere", "other", "than", "your", "hand"]) {
        return None;
    }
    let words = clause.word_refs();
    match words.as_slice() {
        [zone_word] => parse_zone_word(zone_word),
        [article, zone_word]
            if is_article(article) || DEFINITE_ARTICLE_WORD_PATTERN.matches_word(article) =>
        {
            parse_zone_word(zone_word)
        }
        _ => None,
    }
}

fn is_no_amount_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["no"])
}

fn is_spell_object_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["spell"], &["spells"]])
}

fn is_were_cast_action_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["was", "cast"], &["were", "cast"]])
}

fn is_last_turn_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["last", "turn"])
}

fn is_kicked_source_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(
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
    clause_matches_phrase(clause, &["was", "kicked"])
}

fn is_bargained_source_clause(clause: LexedClause<'_>) -> bool {
    is_source_spell_object_clause(clause)
}

fn is_was_bargained_action_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["was", "bargained"])
}

fn is_named_label_clause(clause: LexedClause<'_>, label: &str) -> bool {
    let label_word = label.to_ascii_lowercase();
    clause_matches_phrase(clause, &[label_word.as_str()])
}

fn is_that_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["that"])
}

fn parse_you_cast_source_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["cast"])),
        LexPattern::action("action", LexCaptureKind::WordCount(1)),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !is_cast_action_clause(action_clause) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !is_source_spell_object_clause(object_clause) {
        return None;
    }
    Some(PredicateAst::SourceWasCast)
}

fn parse_tagged_was_cast_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["was", "cast"])),
        LexPattern::action("action", LexCaptureKind::WordCount(2)),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_tagged_cast_subject_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !is_was_cast_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::TaggedWasCast(TagKey::from(IT_TAG)))
}

fn parse_this_spell_was_cast_from_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilPhrase(&["was", "cast", "from"]),
        ),
        LexPattern::action("action", LexCaptureKind::WordCount(3)),
        LexPattern::object("origin", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_this_spell_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !is_was_cast_from_action_clause(action_clause) {
        return None;
    }
    let origin_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if clause_matches_phrase(
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
        LexPattern::amount("amount", LexCaptureKind::WordCount(1)),
        LexPattern::object("object", LexCaptureKind::WordCount(1)),
        LexPattern::action("action", LexCaptureKind::WordCount(2)),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let amount_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    if !is_no_amount_clause(amount_clause) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !is_spell_object_clause(object_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
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
    parse_this_spell_was_kicked_shape(tokens)
        .or_else(|| parse_this_spell_was_bargained_shape(tokens))
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Gift", &["was", "promised"], false)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Gift", &["wasnt", "promised"], true)
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
            parse_named_spell_label_action_shape(tokens, "Tribute", &["was", "not", "paid"], true)
        })
        .or_else(|| parse_behold_spell_label_shape(tokens))
}

fn parse_this_spell_was_kicked_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["was", "kicked"])),
        LexPattern::action("action", LexCaptureKind::WordCount(2)),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_kicked_source_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !is_was_kicked_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::ThisSpellWasKicked)
}

fn parse_this_spell_was_bargained_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilPhrase(&["was", "bargained"]),
        ),
        LexPattern::action("action", LexCaptureKind::WordCount(2)),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_bargained_source_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !is_was_bargained_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::ThisSpellPaidLabel("Bargain".to_string()))
}

fn parse_named_spell_label_action_shape(
    tokens: &[OwnedLexToken],
    label: &str,
    action_phrase: &[&str],
    negated: bool,
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::object("label", LexCaptureKind::UntilPhrase(action_phrase)),
        LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let label_clause = matched.capture_clause("label", clause)?;
    if !is_named_label_clause(label_clause, label) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !clause_matches_phrase(action_clause, action_phrase) {
        return None;
    }
    let predicate = PredicateAst::ThisSpellPaidLabel(label.to_string());
    if negated {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

fn parse_behold_spell_label_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    let subtype_words = match words.as_slice() {
        [subtype @ .., "was", "beheld"] => subtype,
        [subtype @ .., "beheld"] => subtype,
        _ => return None,
    };
    let subtype_words = if matches!(subtype_words.first(), Some(&word) if ARTICLE_WORD_PATTERN.matches_word(word))
    {
        &subtype_words[1..]
    } else {
        subtype_words
    };
    if subtype_words.len() != 1 || parse_subtype_word(subtype_words[0]).is_none() {
        return None;
    }
    Some(PredicateAst::ThisSpellPaidLabel("Behold".to_string()))
}

fn parse_target_was_kicked_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["was", "kicked"])),
        LexPattern::action("action", LexCaptureKind::WordCount(2)),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_that_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !is_was_kicked_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::TargetWasKicked)
}

fn parse_mana_spent_capture_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_mana_symbol_spent_to_cast_shape(&tokens)
        .or_else(|| {
            parse_same_color_mana_spent_to_cast_predicate(words)
                .map(|amount| PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(amount))
        })
        .or_else(|| {
            parse_mana_spent_to_cast_predicate(words).map(|(amount, symbol)| {
                PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol }
            })
        })
}

fn parse_mana_symbol_spent_to_cast_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::amount(
            "symbols",
            LexCaptureKind::UntilAnyPhrase(MANA_SPENT_TO_CAST_THIS_SPELL_PHRASES),
        ),
        LexPattern::any_phrase(MANA_SPENT_TO_CAST_THIS_SPELL_PHRASES),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let symbol_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let symbol_words = symbol_clause.word_refs();
    if symbol_words.is_empty()
        || !symbol_words
            .iter()
            .all(|word| MANA_SYMBOL_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    let mut predicates = symbol_words
        .iter()
        .filter_map(|word| parse_mana_symbol(word).ok())
        .map(|symbol| PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(symbol),
        });
    let first = predicates.next()?;
    Some(predicates.fold(first, |left, right| {
        PredicateAst::And(Box::new(left), Box::new(right))
    }))
}

fn parse_attached_tagged_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_this_permanent_attached_to_shape(&tokens)
}

fn parse_this_permanent_attached_to_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["attached", "to"], &["is", "attached", "to"]];
    for action_phrase in action_phrases {
        let atoms = [
            LexPattern::subject("subject", LexCaptureKind::UntilPhrase(action_phrase)),
            LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
            LexPattern::object("attached_to", LexCaptureKind::Rest),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        if !is_this_or_that_permanent_clause(subject_clause) {
            continue;
        }
        let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
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
    clause_matches_any_phrase(clause, &[&["this", "permanent"], &["that", "permanent"]])
}

fn is_tagged_exiled_subject_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(
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
    clause_matches_phrase(clause, &["exiled"])
}

fn is_that_permanent_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["that", "permanent"])
}

fn is_tagged_entered_subject_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(
        clause,
        &[&["it"], &["that", "card"], &["that", "permanent"]],
    )
}

fn is_your_control_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["your", "control"])
}

fn is_tagged_creature_subject_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["it"], &["that", "creature"]])
}

fn is_blocking_state_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["blocking"])
}

fn is_soulbond_partner_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["creature"], &["another", "creature"]])
}

fn tagged_creature_role_clause(clause: LexedClause<'_>) -> Option<&'static str> {
    if clause_matches_phrase(clause, &["equipped", "creature"]) {
        return Some("equipped");
    }
    if clause_matches_phrase(clause, &["enchanted", "creature"]) {
        return Some("enchanted");
    }
    None
}

fn parse_tagged_exiled_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let action_phrases: &[&[&str]] = &[&["remain"], &["remains"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::WordCount(1)),
        LexPattern::object("zone", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_tagged_exiled_subject_clause(subject_clause) {
        return None;
    }
    let zone_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !is_exiled_zone_clause(zone_clause) {
        return None;
    }
    Some(PredicateAst::TaggedMatches(
        TagKey::from(IT_TAG),
        ObjectFilter::default().in_zone(Zone::Exile),
    ))
}

fn parse_tagged_state_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_tagged_controlled_permanent_shape(&tokens)
        .or_else(|| parse_tagged_entered_under_your_control_shape(&tokens))
        .or_else(|| parse_tagged_wasnt_blocking_shape(&tokens))
        .or_else(|| parse_it_soulbond_paired_shape(&tokens))
        .or_else(|| parse_tagged_creature_filter_shape(&tokens))
}

fn parse_tagged_controlled_permanent_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["control"], &["controlled"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["control", "controlled"])),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !is_that_permanent_clause(object_clause) {
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
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(action_phrase)),
        LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
        LexPattern::object("controller", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_tagged_entered_subject_clause(subject_clause) {
        return None;
    }
    let controller_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
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
            LexPattern::subject("subject", LexCaptureKind::UntilPhrase(action_phrase)),
            LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
            LexPattern::object("state", LexCaptureKind::Rest),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        if !is_tagged_creature_subject_clause(subject_clause) {
            continue;
        }
        let state_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
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

fn parse_it_soulbond_paired_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["paired", "with"], &["is", "paired", "with"]];
    for action_phrase in action_phrases {
        let atoms = [
            LexPattern::subject("subject", LexCaptureKind::UntilPhrase(action_phrase)),
            LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
            LexPattern::object("partner", LexCaptureKind::Rest),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        if !clause_matches_phrase(subject_clause, &["it"]) {
            continue;
        }
        let partner_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
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
        LexPattern::subject("tagged_subject", LexCaptureKind::WordCount(2)),
        LexPattern::object("filter", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let tagged_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let tag = tagged_creature_role_clause(tagged_clause)?;
    let filter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let mut filter = parse_object_filter(filter_clause.tokens(), false).ok()?;
    if filter.card_types.is_empty() {
        filter.card_types.push(CardType::Creature);
    }
    Some(PredicateAst::TaggedMatches(TagKey::from(tag), filter))
}

fn graveyard_possessive_matches_subject(player: PlayerAst, possessive: &str) -> bool {
    match player {
        PlayerAst::You | PlayerAst::Implicit => YOUR_WORD_PATTERN.matches_word(possessive),
        _ => THEIR_WORD_PATTERN.matches_word(possessive),
    }
}

fn comparison_player_subject(words: &[&str]) -> Option<(PlayerAst, usize)> {
    if THAT_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words) {
        Some((PlayerAst::That, 2))
    } else if TARGET_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words) {
        Some((PlayerAst::Target, 2))
    } else if TARGET_OPPONENT_SUBJECT_PREFIX_PATTERN.matches_words(words) {
        Some((PlayerAst::TargetOpponent, 2))
    } else if EACH_OPPONENT_SUBJECT_PREFIX_PATTERN.matches_words(words) {
        Some((PlayerAst::Opponent, 2))
    } else if A_OR_ANY_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words) {
        Some((PlayerAst::Any, 2))
    } else if DEFENDING_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words) {
        Some((PlayerAst::Defending, 2))
    } else if ATTACKING_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words) {
        Some((PlayerAst::Attacking, 2))
    } else if words
        .first()
        .is_some_and(|word| YOU_WORD_PATTERN.matches_word(word))
    {
        Some((PlayerAst::You, 1))
    } else if matches!(words, ["an", "opponent"] | ["the", "opponent"]) {
        Some((PlayerAst::Opponent, 2))
    } else if OPPONENT_SUBJECT_PREFIX_PATTERN.matches_words(words) {
        Some((PlayerAst::Opponent, 1))
    } else if PLAYER_WHO_SUBJECT_PREFIX_PATTERN.matches_words(words) {
        Some((PlayerAst::That, 1))
    } else if words
        .first()
        .is_some_and(|word| PLAYER_SUBJECT_WORD_PATTERN.matches_word(word))
    {
        Some((PlayerAst::Any, 1))
    } else {
        None
    }
}

fn parse_player_cards_in_graveyard_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["has"], &["have"]];
    let card_in_phrases: &[&[&str]] = &[&["card", "in"], &["cards", "in"]];
    let atoms = [
        LexPattern::subject("player", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["has", "have"])),
        LexPattern::amount("quantity", LexCaptureKind::UntilAnyPhrase(card_in_phrases)),
        LexPattern::any_phrase(card_in_phrases),
        LexPattern::modifier("possessive", LexCaptureKind::WordCount(1)),
        LexPattern::object("zone", LexCaptureKind::OneOf(&["graveyard"])),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let (player, consumed) = comparison_player_subject(&subject.word_refs())?;
    if consumed != subject.word_refs().len() {
        return None;
    }
    let quantity = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let (comparison, used) = predicate_quantity_prefix(&quantity.word_refs())?;
    if used != quantity.word_refs().len() {
        return None;
    }
    let (operator, count) = comparison_to_value_comparison_operator(comparison)?;
    let possessive = matched.capture_clause("possessive", clause)?;
    let possessive_word = possessive.word_refs().first().copied()?;
    if !graveyard_possessive_matches_subject(player, possessive_word) {
        return None;
    }
    let player_filter = player_filter_for_turn_value(player)?;

    Some(PredicateAst::ValueComparison {
        left: Value::CardsInGraveyard(player_filter),
        operator,
        right: Value::Fixed(count),
    })
}

fn parse_player_controls_more_than_you_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let control_phrases: &[&[&str]] = &[&["control"], &["controls"]];
    let atoms = [
        LexPattern::subject("player", LexCaptureKind::UntilAnyPhrase(control_phrases)),
        LexPattern::capture("control", LexCaptureKind::OneOf(&["control", "controls"])),
        LexPattern::amount("comparison", LexCaptureKind::OneOf(&["more"])),
        LexPattern::object("object", LexCaptureKind::UntilPhrase(&["than"])),
        LexPattern::capture("than", LexCaptureKind::OneOf(&["than"])),
        LexPattern::modifier("comparison_player", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let (player, consumed) = comparison_player_subject(&subject.word_refs())?;
    if consumed != subject.word_refs().len() {
        return None;
    }
    let tail = matched.capture_clause("comparison_player", clause)?;
    if !is_you_comparison_tail_clause(tail) {
        return None;
    }
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if object.tokens().is_empty() {
        return None;
    }
    let other = object
        .tokens()
        .first()
        .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
    let filter = parse_object_filter(object.tokens(), other).ok()?;
    if filter == ObjectFilter::default() {
        return None;
    }

    Some(PredicateAst::PlayerControlsMoreThanYou { player, filter })
}

fn parse_opponent_controls_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let control_phrases: &[&[&str]] = &[&["controls"]];
    let atoms = [
        LexPattern::subject(
            "controller",
            LexCaptureKind::UntilAnyPhrase(control_phrases),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(&["controls"])),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let controller = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_opponent_controller_clause(controller) {
        return None;
    }
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let object_words = object.word_refs();
    if object_words.first().is_some_and(|word| *word == "more")
        && word_slice_contains_word(&object_words[1..], "than")
    {
        return None;
    }
    if object.tokens().is_empty() {
        return None;
    }
    let other = object
        .tokens()
        .first()
        .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
    let mut filter = parse_object_filter(object.tokens(), other).ok()?;
    filter.controller = Some(PlayerFilter::Opponent);
    filter.zone = None;

    Some(PredicateAst::PlayerControls {
        player: PlayerAst::Opponent,
        filter,
    })
}

fn is_you_comparison_tail_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["you"], &["you", "do"]])
}

fn parse_keyword_subject_object_filter_words(
    object_words: &[&str],
) -> Result<ObjectFilter, CardTextError> {
    let object = strip_leading_article_word_refs(object_words);
    if NONLAND_CARD_OBJECT_PATTERN.matches_words(object) {
        let mut filter = ObjectFilter::default();
        filter.excluded_card_types.push(CardType::Land);
        return Ok(filter);
    }

    let normalized_object;
    let object = if object.ends_with(&["cards"]) {
        normalized_object = object[..object.len() - 1]
            .iter()
            .copied()
            .chain(std::iter::once("card"))
            .collect::<Vec<_>>();
        normalized_object.as_slice()
    } else {
        object
    };
    let object_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(object);
    parse_object_filter(&object_tokens, false).or_else(|_| {
        let trimmed = object
            .strip_suffix(&["card"])
            .or_else(|| object.strip_suffix(&["cards"]))
            .unwrap_or(object);
        let trimmed_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(trimmed);
        parse_object_filter(&trimmed_tokens, false)
    })
}

fn parse_graveyard_escape_keyword_predicate(
    words: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    const IN_YOUR_GRAVEYARD_PHRASE: &[&str] = &["in", "your", "graveyard"];
    const ESCAPE_KEYWORD_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object(
            "object",
            LexCaptureKind::UntilPhrase(IN_YOUR_GRAVEYARD_PHRASE),
        ),
        LexPattern::phrase(IN_YOUR_GRAVEYARD_PHRASE),
        LexPattern::action("action", LexCaptureKind::OneOf(&["has", "have"])),
        LexPattern::object("keyword", LexCaptureKind::OneOf(&["escape"])),
    ]);

    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let Some(matched) = ESCAPE_KEYWORD_PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let object = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in escape predicate".to_string())
        })?;
    let object_words = object.word_refs();
    if object_words.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_keyword_subject_object_filter_words(object_words.as_slice())?;
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);
    filter.alternative_cast = Some(crate::filter::AlternativeCastKind::Escape);
    Ok(Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter,
    }))
}

fn parse_player_object_keyword_predicate(
    words: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    if let Some(predicate) = parse_graveyard_escape_keyword_predicate(words)? {
        return Ok(Some(predicate));
    }

    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let action_phrases: &[&[&str]] = &[&["has"], &["have"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["has", "have"])),
        LexPattern::object("keyword", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let subject = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing subject in keyword predicate".to_string())
        })?;
    let keyword = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing keyword in keyword predicate".to_string())
        })?;
    let keyword_words = keyword.word_refs();
    let Some((constraint, consumed)) = parse_filter_keyword_constraint_words(&keyword_words) else {
        return Ok(None);
    };
    if consumed != keyword_words.len() {
        return Ok(None);
    }

    let subject_words = subject.word_refs();
    let subject_has_control = subject_words
        .iter()
        .any(|word| CONTROL_WORD_PATTERN.matches_word(word));
    let subject_has_zone = subject_words
        .iter()
        .any(|word| ZONE_WORD_PATTERN.matches_word(word));
    let mut filter = if subject_has_control {
        let object_words = subject_words
            .iter()
            .copied()
            .filter(|word| {
                !YOU_WORD_PATTERN.matches_word(word)
                    && !CONTROL_OR_CONTROLS_WORD_PATTERN.matches_word(word)
            })
            .collect::<Vec<_>>();
        if object_words.is_empty() {
            return Ok(None);
        }
        let object_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&object_words);
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
    const OBJECT_IN_ZONE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object("object", LexCaptureKind::UntilPhrase(&["in"])),
        LexPattern::word("in"),
        LexPattern::modifier("zone", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(subject_tokens);
    let Some(matched) = OBJECT_IN_ZONE_PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let object = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in keyword-zone predicate".to_string())
        })?;
    let zone = matched
        .capture_clause_by_role(LexCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing zone in keyword-zone predicate".to_string())
        })?;
    let object_words = object.word_refs();
    if object_words.is_empty() || zone.word_refs().is_empty() {
        return Ok(None);
    }
    let Ok(mut filter) = parse_keyword_subject_object_filter_words(&object_words) else {
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
    clause_matches_phrase(clause, &["your", "graveyard"])
}

fn is_there_are_or_were_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["there", "are"], &["there", "were"]])
}

fn permanents_you_control_scope(words: &[&str]) -> Option<ObjectFilter> {
    if PERMANENTS_YOU_CONTROL_SCOPE_PATTERN.matches_words(words) {
        return Some(ObjectFilter::permanent().you_control());
    }
    None
}

fn cards_in_your_graveyard_scope(words: &[&str]) -> Option<ObjectFilter> {
    if CARDS_IN_YOUR_GRAVEYARD_SCOPE_PATTERN.matches_words(words) {
        return Some(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
    }
    None
}

fn permanents_and_your_graveyard_scope(words: &[&str]) -> Option<ObjectFilter> {
    let battlefield_end = (3..=words.len().min(4))
        .find(|end| permanents_you_control_scope(&words[..*end]).is_some())?;
    let connector_end = if words
        .get(battlefield_end..battlefield_end + 1)
        .is_some_and(|tail| PERMANENTS_AND_OR_GRAVEYARD_CONNECTOR_PATTERN.matches_words(tail))
    {
        battlefield_end + 1
    } else if words
        .get(battlefield_end..battlefield_end + 2)
        .is_some_and(|tail| PERMANENTS_AND_OR_SPLIT_CONNECTOR_PATTERN.matches_words(tail))
    {
        battlefield_end + 2
    } else {
        return None;
    };
    let battlefield = permanents_you_control_scope(&words[..battlefield_end])?;
    let graveyard_start = connector_end;
    let graveyard = cards_in_your_graveyard_scope(&words[graveyard_start..])?;
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![battlefield, graveyard];
    Some(filter)
}

fn parse_colors_among_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("existential", LexCaptureKind::WordCount(2)),
        LexPattern::amount(
            "quantity",
            LexCaptureKind::UntilAnyPhrase(&[&["color"], &["colors"]]),
        ),
        LexPattern::object("unit", LexCaptureKind::OneOf(&["color", "colors"])),
        LexPattern::word("among"),
        LexPattern::modifier("scope", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let existential = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_there_are_or_were_clause(existential) {
        return None;
    }

    let quantity = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let (count, used) = parse_number(quantity.tokens())?;
    if used != quantity.tokens().len() {
        return None;
    }

    let scope = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    let filter = permanents_you_control_scope(&scope.word_refs())?;
    Some(PredicateAst::ValueComparison {
        left: Value::ColorsAmong(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    })
}

fn parse_card_types_among_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let card_type_phrases: &[&[&str]] = &[
        &["card", "type"],
        &["card", "types"],
        &["cards", "type"],
        &["cards", "types"],
    ];
    let atoms = [
        LexPattern::subject("existential", LexCaptureKind::WordCount(2)),
        LexPattern::amount(
            "quantity",
            LexCaptureKind::UntilAnyPhrase(card_type_phrases),
        ),
        LexPattern::any_phrase(card_type_phrases),
        LexPattern::word("among"),
        LexPattern::modifier("scope", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let existential = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_there_are_or_were_clause(existential) {
        return None;
    }

    let quantity = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let quantity_words = quantity.word_refs();
    let (count, used) = predicate_at_least_quantity_prefix(&quantity_words)?;
    if used != quantity_words.len() {
        return None;
    }

    let scope = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    let scope_words = scope.word_refs();
    let filter = match scope_words.as_slice() {
        ["sacrificed" | "sacrificed_0"]
        | ["sacrificed" | "sacrificed_0", "permanent" | "permanents"] => {
            ObjectFilter::tagged("sacrificed_0")
        }
        _ => permanents_and_your_graveyard_scope(&scope_words)?,
    };

    Some(PredicateAst::ValueComparison {
        left: Value::CardTypesAmong(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    })
}

fn parse_life_total_at_least_starting_predicate(words: &[&str]) -> Option<PredicateAst> {
    if LIFE_TOTAL_AT_LEAST_STARTING_PATTERN.matches_words(words) {
        return Some(PredicateAst::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::StartingLifeTotal(PlayerFilter::You),
        });
    }
    None
}

fn parse_life_total_at_least_last_noted_predicate(words: &[&str]) -> Option<PredicateAst> {
    if !matches!(
        words,
        [
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
            "permanent" | "enchantment" | "artifact" | "creature" | "land",
        ]
    ) {
        return None;
    }
    Some(PredicateAst::ValueComparison {
        left: Value::LifeTotal(PlayerFilter::You),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::LastNotedLifeTotal,
    })
}

fn parse_counted_objects_have_counter_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let have_phrases: &[&[&str]] = &[&["has"], &["have"]];
    let atoms = [
        LexPattern::object(
            "counted_object",
            LexCaptureKind::UntilAnyPhrase(have_phrases),
        ),
        LexPattern::action("have", LexCaptureKind::OneOf(&["has", "have"])),
        LexPattern::modifier("counter", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;

    let counted_object = matched.capture_clause("counted_object", clause)?;
    let counted_words = counted_object.word_refs();
    let (comparison, used) = predicate_quantity_prefix(&counted_words)?;
    let count = comparison_to_strict_at_least_threshold(&comparison)?;
    if used >= counted_words.len() {
        return None;
    }

    let object_tokens = &counted_object.tokens()[used..];
    if object_tokens.is_empty() {
        return None;
    }
    let counter = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    let counter_words = counter.word_refs();
    if counter_words.is_empty() {
        return None;
    }
    let (counter_constraint, consumed) = if let Some(parsed) =
        parse_filter_counter_constraint_words(&counter_words)
    {
        parsed
    } else {
        let counter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&counter_words);
        let counter_type = parse_counter_type_from_tokens(&counter_tokens)?;
        (
            ironsmith_core::CounterConstraint::Typed(counter_type),
            counter_words.len(),
        )
    };
    if consumed != counter_words.len() {
        return None;
    }

    let other = object_tokens
        .first()
        .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
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

fn parse_counted_source_exiled_objects_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let have_phrases: &[&[&str]] = &[&["has"], &["have"]];
    let atoms = [
        LexPattern::object(
            "counted_object",
            LexCaptureKind::UntilAnyPhrase(have_phrases),
        ),
        LexPattern::action("have", LexCaptureKind::OneOf(&["has", "have"])),
        LexPattern::modifier("tail", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;

    let counted_object = matched.capture_clause("counted_object", clause)?;
    let counted_words = counted_object.word_refs();
    let (comparison, used) = predicate_quantity_prefix(&counted_words)?;
    let (operator, count) = comparison_to_value_comparison_operator(comparison)?;
    if used >= counted_words.len() {
        return None;
    }

    let tail = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    let tail_words = tail.word_refs();
    if !BEEN_EXILED_WITH_THIS_SOURCE_PREFIX_PATTERN.matches_words(&tail_words) {
        return None;
    }

    let object_tokens = &counted_object.tokens()[used..];
    let object_words = &counted_words[used..];
    let mut filter = if object_words
        .iter()
        .all(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
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

fn parse_happily_style_conjoined_predicate(words: &[&str]) -> Option<PredicateAst> {
    let cleaned = word_refs_except(words, &[","]);
    let words = cleaned.as_slice();
    let second_there_idx = THERE_ARE_PREFIX_PATTERN
        .find_exact_window_range(&words[1..], 2, 2)
        .map(|idx| idx + 1)?;
    let life_idx = AND_YOUR_LIFE_TOTAL_PATTERN
        .find_exact_window_range(&words[second_there_idx + 1..], 4, 4)
        .map(|idx| idx + second_there_idx + 1)?;

    let first = parse_colors_among_predicate(&words[..second_there_idx])?;
    let second = parse_card_types_among_predicate(&words[second_there_idx..life_idx])?;
    let third = parse_life_total_at_least_starting_predicate(&words[life_idx + 1..])?;

    Some(PredicateAst::And(
        Box::new(PredicateAst::And(Box::new(first), Box::new(second))),
        Box::new(third),
    ))
}

fn parse_revealed_or_controlled_subtype_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let suffix_atoms = [LexPattern::phrase(&["as", "you", "cast", "this", "spell"])];
    let atoms = [
        LexPattern::subject("revealer", LexCaptureKind::WordCount(1)),
        LexPattern::action("reveal_action", LexCaptureKind::OneOf(&["revealed"])),
        LexPattern::object("revealed_subtype", LexCaptureKind::WordCount(1)),
        LexPattern::word("card"),
        LexPattern::word("or"),
        LexPattern::action(
            "control_action",
            LexCaptureKind::OneOf(&["control", "controlled", "controls"]),
        ),
        LexPattern::object("controlled_subtype", LexCaptureKind::WordCount(1)),
        LexPattern::optional(&suffix_atoms),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let revealer = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_you_clause(revealer) {
        return None;
    }

    let revealed_subtype = matched.capture_clause("revealed_subtype", clause)?;
    let controlled_subtype = matched.capture_clause("controlled_subtype", clause)?;
    let revealed_words = revealed_subtype.word_refs();
    let controlled_words = controlled_subtype.word_refs();
    if revealed_words != controlled_words {
        return None;
    }
    let subtype = parse_subtype_word(revealed_words.first().copied()?)?;

    Some(PredicateAst::Or(
        Box::new(PredicateAst::ThisSpellPaidLabel("Behold".to_string())),
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: ObjectFilter::default().with_subtype(subtype),
        }),
    ))
}

fn is_card_graveyard_existential_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["there", "is"], &["there", "are"]])
}

fn is_graveyard_location_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(
        clause,
        &[
            &["your", "graveyard"],
            &["graveyard"],
            &["the", "graveyard"],
        ],
    )
}

fn parse_card_in_your_graveyard_predicate(words: &[&str]) -> Option<PredicateAst> {
    if let [
        "there",
        "is" | "are",
        descriptor @ ..,
        "in",
        "your",
        "graveyard",
    ] = words
    {
        let descriptor = strip_leading_article_word_refs(descriptor);
        let mut filter = if let Ok(filter) = parse_object_filter(
            &crate::runtime_backend::lexer::synthetic_word_tokens(descriptor),
            false,
        ) {
            filter
        } else if let [subtype, "card"] | [subtype, "cards"] = descriptor {
            ObjectFilter::default().with_subtype(parse_subtype_word(subtype)?)
        } else {
            return None;
        };
        filter.zone = Some(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
        return Some(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        });
    }

    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("existential", LexCaptureKind::WordCount(2)),
        LexPattern::object("descriptor", LexCaptureKind::UntilPhrase(&["in"])),
        LexPattern::action("preposition", LexCaptureKind::OneOf(&["in"])),
        LexPattern::modifier("location", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let existential = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !is_card_graveyard_existential_clause(existential) {
        return None;
    }

    let location = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !is_graveyard_location_clause(location) {
        return None;
    }

    let descriptor = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if descriptor.word_refs().is_empty() {
        return None;
    }
    let descriptor_words = descriptor.word_refs();
    let mut filter = parse_object_filter(descriptor.tokens(), false)
        .ok()
        .or_else(|| {
            let trimmed = descriptor_words
                .strip_suffix(&["card"])
                .or_else(|| descriptor_words.strip_suffix(&["cards"]))
                .unwrap_or(descriptor_words.as_slice());
            let trimmed_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(trimmed);
            parse_object_filter(&trimmed_tokens, false).ok()
        })
        .or_else(|| {
            let trimmed = descriptor_words
                .strip_prefix(&["an"])
                .or_else(|| descriptor_words.strip_prefix(&["a"]))
                .unwrap_or(descriptor_words.as_slice());
            match trimmed {
                [subtype, "card"] | [subtype, "cards"] => parse_subtype_word(subtype)
                    .map(|subtype| ObjectFilter::default().with_subtype(subtype)),
                _ => None,
            }
        })?;
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
    let clause = LexedClause::new(tokens);
    let copula_phrases: &[&[&str]] = &[&["is", "on"], &["are", "on"]];
    let atoms = [
        LexPattern::object("object", LexCaptureKind::UntilAnyPhrase(copula_phrases)),
        LexPattern::action("copula", LexCaptureKind::WordCount(2)),
        LexPattern::modifier("location", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let location = matched
        .capture_clause_by_role(LexCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing location in battlefield predicate".to_string())
        })?;
    if !is_battlefield_zone_clause(location) {
        return Ok(None);
    }

    let object_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in battlefield predicate".to_string())
        })?;
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
    const NAMED_OBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object("object", LexCaptureKind::UntilPhrase(&["named"])),
        LexPattern::word("named"),
        LexPattern::modifier("name", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = NAMED_OBJECT_PATTERN.match_clause(clause)?;
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if object.word_refs().is_empty() {
        return None;
    }
    let name = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    let name_words = name.word_refs();
    let name_end = find_name_clause_end(name_words.as_slice(), 0);
    let name = name_words.get(..name_end)?.join(" ");
    (!name.is_empty()).then_some(name)
}

fn graveyard_card_types_subject(words: &[&str]) -> Option<PlayerAst> {
    if YOUR_GRAVEYARD_PATTERN.matches_words(words) {
        Some(PlayerAst::You)
    } else if THAT_PLAYER_GRAVEYARD_PATTERN.matches_words(words) {
        Some(PlayerAst::That)
    } else if TARGET_PLAYER_GRAVEYARD_PATTERN.matches_words(words) {
        Some(PlayerAst::Target)
    } else if TARGET_OPPONENT_GRAVEYARD_PATTERN.matches_words(words) {
        Some(PlayerAst::TargetOpponent)
    } else if OPPONENT_GRAVEYARD_PATTERN.matches_words(words) {
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
        LexPattern::subject("lead", LexCaptureKind::WordCount(2)),
        LexPattern::amount(
            "quantity",
            LexCaptureKind::UntilAnyPhrase(card_type_phrases),
        ),
        LexPattern::any_phrase(card_type_phrases),
        LexPattern::modifier("graveyard", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let lead = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let constrained_player = card_types_graveyard_lead_player_clause(lead)?;
    let quantity = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let (count, used) = predicate_at_least_quantity_prefix(&quantity.word_refs())?;
    if used != quantity.word_refs().len() {
        return None;
    }
    let graveyard = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    let player = graveyard_card_types_subject(&graveyard.word_refs())?;
    if constrained_player.is_some_and(|expected| expected != player) {
        return None;
    }

    Some(PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count })
}

fn card_types_graveyard_lead_player_clause(clause: LexedClause<'_>) -> Option<Option<PlayerAst>> {
    if is_there_are_clause(clause) {
        return Some(None);
    }
    if clause_matches_phrase(clause, &["you", "have"]) {
        return Some(Some(PlayerAst::You));
    }
    None
}

fn parse_there_are_objects_on_battlefield_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("existential", LexCaptureKind::WordCount(2)),
        LexPattern::object("counted_object", LexCaptureKind::UntilLastPhrase(&["on"])),
        LexPattern::action("preposition", LexCaptureKind::OneOf(&["on"])),
        LexPattern::modifier("location", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let existential = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing existential in battlefield count predicate".to_string(),
            )
        })?;
    if !is_there_are_clause(existential) {
        return Ok(None);
    }
    let location = matched
        .capture_clause_by_role(LexCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing location in battlefield count predicate".to_string())
        })?;
    if !is_battlefield_zone_clause(location) {
        return Ok(None);
    }

    let counted_object = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in battlefield count predicate".to_string())
        })?;
    let counted_words = counted_object.word_refs();
    let Some((count, used)) = predicate_at_least_quantity_prefix(&counted_words) else {
        return Ok(None);
    };
    let object_tokens = counted_object.tokens().get(used..).unwrap_or_default();
    let object_words = counted_words.get(used..).unwrap_or_default();
    let other = object_words
        .first()
        .is_some_and(|word| OTHER_OR_ANOTHER_WORD_PATTERN.matches_word(word));
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

pub(crate) fn parse_predicate(tokens: &[OwnedLexToken]) -> Result<PredicateAst, CardTextError> {
    let raw_words_view = GrammarFilterNormalizedWords::new(tokens);
    let raw_words = raw_words_view.to_word_refs();
    let mut filtered = non_article_word_refs(&raw_words);

    if filtered.is_empty() {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }
    if filtered.first().copied() == Some("if") {
        filtered.remove(0);
    }
    if filtered.is_empty() {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }
    if ITS_WORD_PATTERN.matches_word(filtered[0]) {
        filtered[0] = "it";
    }
    if IT_S_PREFIX_PATTERN.matches_words(&filtered) {
        filtered.remove(1);
    }
    if let Some(instead_idx) = INSTEAD_WORD_PATTERN.find_word(&filtered)
        && instead_idx > 0
    {
        let maybe_predicate = &filtered[..instead_idx];
        let paid_tail = maybe_predicate.len() >= 3
            && COST_PAID_INSTEAD_TAIL_PATTERN
                .matches_words(&maybe_predicate[maybe_predicate.len() - 3..]);
        let unpaid_tail = maybe_predicate.len() >= 4
            && COST_NOT_PAID_INSTEAD_TAIL_PATTERN
                .matches_words(&maybe_predicate[maybe_predicate.len() - 4..]);
        if paid_tail || unpaid_tail {
            filtered.truncate(instead_idx);
        }
    }
    if predicate_find_exact_phrase_shape(
        &filtered,
        YOU_BOTH_OWN_AND_CONTROL_PHRASE,
        &YOU_BOTH_OWN_AND_CONTROL_PATTERN,
    )
    .is_some()
        && let Some(exile_idx) =
            predicate_find_exact_phrase_shape(&filtered, EXILE_THEM_PHRASE, &EXILE_THEM_PATTERN)
    {
        filtered.truncate(exile_idx);
    }

    if let Some(predicate) = parse_repeated_if_or_predicate(&filtered)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_secret_choices_match_predicate(&filtered) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_vote_result_predicate(&filtered, true)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_passive_this_way_tagged_object_predicate(&filtered)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_active_this_way_discard_predicate(&filtered)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_active_this_way_battlefield_predicate(&filtered)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_passive_this_way_battlefield_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_this_ability_resolution_count_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_stack_object_targets_only_source_predicate(&filtered) {
        return Ok(predicate);
    }

    if IT_EXPLOITED_TRIGGERING_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::And(
            Box::new(PredicateAst::TaggedMatches(
                TagKey::from(crate::tag::EXPLOITED_TAG),
                ObjectFilter::tagged("triggering"),
            )),
            Box::new(PredicateAst::TaggedMatches(
                TagKey::from(crate::tag::EXPLOITER_TAG),
                ObjectFilter::source(),
            )),
        ));
    }

    if let Some(zone) = source_zone_from_words(&filtered) {
        return Ok(PredicateAst::SourceIsInZone(zone));
    }

    if let Some(predicate) = parse_source_exiled_with_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_happily_style_conjoined_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_colors_among_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_types_among_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_revealed_or_controlled_subtype_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_graveyard_threshold_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_in_your_graveyard_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_empty_battlefield_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_object_on_battlefield_predicate(tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_life_total_at_least_starting_predicate(&filtered) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_life_total_at_least_last_noted_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_controls_more_than_you_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_status_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_counted_objects_have_counter_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_counted_source_exiled_objects_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_half_starting_life_total_threshold_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_you_life_total_at_most_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_object_keyword_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_opponent_controls_tagged_object_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_opponent_controls_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_vote_result_predicate(&filtered, false)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_attacking_you_own_control_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_you_both_own_and_control_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_implicit_subject_and_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_while_conjoined_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_simple_state_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_attachment_count_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_identity_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_keyword_predicate(&filtered) {
        return Ok(predicate);
    }

    let source_state_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&filtered);
    if let Some(predicate) =
        parse_source_did_not_attack_or_enter_control_this_turn_shape(&source_state_tokens)
    {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_there_are_no_counters_on_source_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_has_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_has_counted_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_triggering_object_had_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_there_are_source_counters_at_least_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_power_threshold_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_basic_land_types_among_lands_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_there_are_objects_on_battlefield_predicate(tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_types_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_half_starting_life_total_threshold_predicate(&filtered) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_player_controls_more_than_you_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_life_relation_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_life_total_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_hand_relation_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_hand_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_turn_event_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_would_action_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_turn_timing_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_life_change_this_turn_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_object_death_this_turn_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_battlefield_change_this_turn_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_battlefield_entry_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_combat_turn_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_spell_lifecycle_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_paid_cost_label_predicate(&filtered) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_spell_context_predicate(&filtered) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_mana_spent_capture_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_attached_tagged_predicate(&filtered) {
        return Ok(predicate);
    }

    let sacrificed_idx = if SACRIFICED_WORD_PATTERN.matches_word_at(&filtered, 0) {
        Some(0usize)
    } else if filtered.len() >= 2
        && matches!(filtered[0], "the" | "a" | "an")
        && SACRIFICED_WORD_PATTERN.matches_word_at(&filtered, 1)
    {
        Some(1usize)
    } else {
        None
    };
    if let Some(sacrificed_idx) = sacrificed_idx
        && filtered.len() >= sacrificed_idx + 4
        && WAS_WORD_PATTERN.matches_word_at(&filtered, sacrificed_idx + 2)
    {
        let sacrificed_head = filtered[sacrificed_idx + 1];
        let subject_card_type =
            parse_card_type(sacrificed_head).filter(|card_type| is_permanent_type(*card_type));
        let subject_is_permanent =
            PERMANENT_WORD_PATTERN.matches_word(sacrificed_head) || subject_card_type.is_some();

        if subject_is_permanent {
            let descriptor_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(
                &filtered[sacrificed_idx + 3..],
            );
            let mut filter = match parse_object_filter(&descriptor_tokens, false) {
                Ok(filter) => filter,
                Err(err) => parse_color_only_object_filter_words(&filtered[sacrificed_idx + 3..])
                    .ok_or(err)?,
            };
            if filter.card_types.is_empty() {
                if let Some(card_type) = subject_card_type {
                    filter.card_types.push(card_type);
                }
            }
            if filter.zone.is_none() && PERMANENT_WORD_PATTERN.matches_word(sacrificed_head) {
                filter.zone = Some(Zone::Battlefield);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
    }

    if let Some(predicate) = parse_tagged_exiled_predicate(&filtered) {
        return Ok(predicate);
    }

    if ITS_WORD_PATTERN.matches_word_at(&filtered, 0) {
        filtered[0] = "it";
    }
    if IT_S_PREFIX_PATTERN.matches_words(&filtered) {
        filtered.remove(1);
    }

    let demonstrative_reference_len = if IT_WORD_PATTERN.matches_word_at(&filtered, 0) {
        Some(1usize)
    } else if filtered.len() >= 2
        && THAT_WORD_PATTERN.matches_word_at(&filtered, 0)
        && PREDICATE_REFERENCE_NOUN_WORD_PATTERN.matches_word_at(&filtered, 1)
    {
        Some(2usize)
    } else {
        None
    };

    if let Some(predicate) = parse_tagged_state_predicate(&filtered) {
        return Ok(predicate);
    }

    let is_it = demonstrative_reference_len == Some(1);
    let has_card = demonstrative_reference_len
        .map(|reference_len| {
            filtered[reference_len..]
                .iter()
                .any(|word| CARD_WORD_PATTERN.matches_word(word))
        })
        .unwrap_or(false);

    if is_it {
        if filtered
            .get(1)
            .is_some_and(|word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))
        {
            filtered.remove(1);
        }
        if filtered
            .get(1..3)
            .is_some_and(|words| MANA_VALUE_HEAD_PATTERN.matches_words(words))
        {
            let mana_value_tail = if filtered
                .get(3)
                .is_some_and(|word| BE_VERB_WORD_PATTERN.matches_word(word))
            {
                &filtered[4..]
            } else {
                &filtered[3..]
            };
            let compares_to_colors_spent =
                COLORS_SPENT_TO_CAST_SOURCE_TAIL_PATTERN.matches_words(mana_value_tail);
            if compares_to_colors_spent {
                return Ok(PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell);
            }

            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens("mana value", mana_value_tail, &filtered)?
            {
                return Ok(PredicateAst::ItMatches(ObjectFilter {
                    mana_value: Some(cmp),
                    ..Default::default()
                }));
            }
        }

        if filtered.len() >= 5
            && filtered
                .get(1..5)
                .is_some_and(|words| TOTAL_POWER_TOUGHNESS_HEAD_PATTERN.matches_words(words))
            && let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens("power", &filtered[5..], &filtered)?
        {
            return Ok(PredicateAst::ItMatches(ObjectFilter {
                total_power_toughness: Some(cmp),
                ..Default::default()
            }));
        }

        if filtered.len() >= 3 && POWER_OR_TOUGHNESS_WORD_PATTERN.matches_word(filtered[1]) {
            let axis = filtered[1];
            let value_tail = &filtered[2..];
            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens(axis, value_tail, &filtered)?
            {
                let mut filter = ObjectFilter::default();
                if POWER_WORD_PATTERN.matches_word(axis) {
                    filter.power = Some(cmp);
                } else {
                    filter.toughness = Some(cmp);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if demonstrative_reference_len.is_some()
        && filtered
            .iter()
            .any(|word| OR_WORD_PATTERN.matches_word(word))
        && MOST_COMMON_COLOR_AMONG_ALL_PERMANENTS_PATTERN
            .find_exact_window_range(&filtered, 6, 6)
            .is_none()
        && let Some(predicate) = parse_or_predicate(&filtered)?
    {
        return Ok(predicate);
    }

    if let Some(reference_len) = demonstrative_reference_len {
        let mut descriptor_words = filtered[reference_len..].to_vec();
        if descriptor_words.len() >= 2
            && POWER_OR_TOUGHNESS_WORD_PATTERN.matches_word(descriptor_words[0])
        {
            let axis = descriptor_words[0];
            let value_tail = if descriptor_words
                .get(1)
                .is_some_and(|word| BE_VERB_WORD_PATTERN.matches_word(word))
            {
                &descriptor_words[2..]
            } else {
                &descriptor_words[1..]
            };
            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens(axis, value_tail, &filtered)?
            {
                let mut filter = ObjectFilter::default();
                if POWER_WORD_PATTERN.matches_word(axis) {
                    filter.power = Some(cmp);
                } else {
                    filter.toughness = Some(cmp);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
        if HAS_OR_HAVE_TOXIC_PATTERN.matches_words(&descriptor_words) {
            let mut filter = ObjectFilter::default().with_ability_marker("toxic");
            if CREATURE_WORD_PATTERN.matches_word_at(&filtered, 1) {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
        if descriptor_words
            .first()
            .is_some_and(|word| IS_OR_ARE_WORD_PATTERN.matches_word(word))
        {
            descriptor_words.remove(0);
        }
        if matches!(
            descriptor_words.as_slice(),
            ["shares", "a", "card", "type", "with", "that", "spell"]
                | ["shares", "card", "type", "with", "that", "spell"]
        ) {
            return Ok(PredicateAst::ItMatches(
                ObjectFilter::default().shares_card_type_with_tagged("triggering"),
            ));
        }
        if matches!(
            descriptor_words.as_slice(),
            [
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
                "common"
            ] | [
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
                "common"
            ]
        ) {
            return Ok(PredicateAst::ItMatches(
                ObjectFilter::default().shares_most_common_permanent_color(),
            ));
        }
        if NOT_TOKEN_PREFIX_PATTERN.matches_words(&descriptor_words) {
            descriptor_words.drain(0..2);
            descriptor_words.insert(0, "nontoken");
        }
        if !descriptor_words.is_empty() {
            if let Some(filter) = parse_single_card_type_card_descriptor(&descriptor_words) {
                return Ok(PredicateAst::ItMatches(filter));
            }
            let descriptor_tokens =
                crate::runtime_backend::lexer::synthetic_word_tokens(descriptor_words);
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
                    return Ok(PredicateAst::ItIsLandCard);
                }
                if THAT_ENCHANTMENT_PREFIX_PATTERN.matches_words(&filtered) {
                    return Ok(PredicateAst::TaggedMatches(
                        TagKey::from("triggering"),
                        filter,
                    ));
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if let Some(predicate) = parse_player_controls_no_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_you_control_or_graveyard_predicate(&filtered).transpose()? {
        return Ok(predicate);
    }

    if filtered.len() >= 3 && YOU_CONTROL_PREFIX_PATTERN.matches_words(&filtered) {
        if let Some(predicate) = parse_you_control_conjoined_predicate(&filtered).transpose()? {
            return Ok(predicate);
        }

        if let Some(predicate) = parse_player_controls_predicate(
            &filtered,
            PlayerAst::You,
            Some(PlayerFilter::You),
            2,
            true,
            true,
        )? {
            return Ok(predicate);
        }
    }

    if filtered.len() >= 4 && THAT_PLAYER_CONTROLS_PREFIX_PATTERN.matches_words(&filtered) {
        if let Some(predicate) =
            parse_player_controls_predicate(&filtered, PlayerAst::That, None, 3, false, false)?
        {
            return Ok(predicate);
        }
    }

    if let Some(predicate) = parse_negative_put_tagged_object_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_achievement_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_ring_bearer_temptation_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_status_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_world_state_or_timing_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_combat_damage_this_turn_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_spell_cast_this_turn_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_x_value_comparison_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_or_predicate(&filtered)? {
        return Ok(predicate);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported predicate (predicate: '{}')",
        filtered.join(" ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CounterType;
    use crate::effect::{ChoiceCount, ValueComparisonOperator};
    use crate::filter::StackObjectKind;
    use crate::runtime_backend::front_end::lexer::lex_line;

    const IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);

    fn predicate_tokens_after_if(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
        tokens
            .iter()
            .filter(|token| !IF_WORD_PATTERN.matches_token(token))
            .cloned()
            .collect()
    }

    #[test]
    fn parse_predicate_paid_cost_labels_use_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If this spells surge cost was paid",
                PredicateAst::ThisSpellPaidLabel("Surge".to_string()),
            ),
            (
                "If this creature's spectacle cost was paid instead discard your hand",
                PredicateAst::ThisSpellPaidLabel("Spectacle".to_string()),
            ),
            (
                "If {U} cost was paid",
                PredicateAst::ThisSpellPaidLabel("{U}".to_string()),
            ),
            (
                "If {2}{G} cost wasn't paid",
                PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel(
                    "{2}{G}".to_string(),
                ))),
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
                PredicateAst::ThisSpellPaidLabel("CastDuringYourMainPhase".to_string()),
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
                    dungeon_name: Some("lost mine of phandelver".to_string()),
                },
            ),
            (
                "If you haven't completed Lost Mine of Phandelver",
                PredicateAst::Not(Box::new(PredicateAst::PlayerCompletedDungeon {
                    player: PlayerAst::You,
                    dungeon_name: Some("lost mine of phandelver".to_string()),
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
        let tokens = lex_line(
            "If this creature and a creature named Midnight Scavengers are attacking and you both own and control them",
            0,
        )?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        let PredicateAst::And(left, right) = parsed else {
            panic!("expected attacking own-control conjoined predicate");
        };
        for side in [left, right] {
            let PredicateAst::PlayerControls { player, filter } = *side else {
                panic!("expected controls predicate");
            };
            assert_eq!(player, PlayerAst::You);
            assert_eq!(filter.controller, Some(PlayerFilter::You));
            assert!(filter.attacking);
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
                PredicateAst::ThisSpellPaidLabel("Bargain".to_string()),
            ),
            (
                "If it was bargained",
                PredicateAst::ThisSpellPaidLabel("Bargain".to_string()),
            ),
            (
                "If gift was promised",
                PredicateAst::ThisSpellPaidLabel("Gift".to_string()),
            ),
            (
                "If gift was not promised",
                PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel(
                    "Gift".to_string(),
                ))),
            ),
            (
                "If tribute was not paid",
                PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel(
                    "Tribute".to_string(),
                ))),
            ),
            ("If that was kicked", PredicateAst::TargetWasKicked),
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
                Box::new(PredicateAst::ThisSpellPaidLabel("Behold".to_string())),
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
                "If this creature has two stun counters on it",
                PredicateAst::SourceHasCounterAtLeast {
                    counter_type: CounterType::Stun,
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

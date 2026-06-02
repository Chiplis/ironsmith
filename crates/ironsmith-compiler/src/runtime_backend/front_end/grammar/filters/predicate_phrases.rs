use super::super::super::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern};
use super::super::super::lexer::{LexedClause, OwnedLexToken};
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
const SOURCE_EXILED_WITH_COUNTER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "card", "is", "exiled", "with"],
            &["this", "source", "is", "exiled", "with"],
        ]
);
const COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const COUNTER_ON_SOURCE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["on", "it"], &["on", "this"], &["on", "them"]]);
const THAT_SPELL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["that", "spell"]);
const SPELL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["spell"]);
const IT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["it"]);
const TARGETS_ONLY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["targets", "only"]);
const TARGET_THIS_CREATURE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["this", "creature"]);
const TARGET_THIS_ARTIFACT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["this", "artifact"]);
const TARGET_THIS_ENCHANTMENT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["this", "enchantment"]);
const TARGET_THIS_LAND_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "land"]);
const TARGET_THIS_PERMANENT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["this", "permanent"]);
const TARGET_SOURCE_REFERENCE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this", "source"], &["it"]]);
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
const THERE_ARE_OR_WERE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["there", "are"], &["there", "were"]]);
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
const BEEN_EXILED_WITH_THIS_SOURCE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["been", "exiled", "with", "this"],
            &["exiled", "with", "this"],
        ]
);
const IN_YOUR_GRAVEYARD_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["in", "your", "graveyard"],
            &["in", "graveyard"],
            &["in", "the", "graveyard"],
        ]
);
const IT_EXPLOITED_TRIGGERING_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "exploited", "that", "creature"],
            &["it", "exploited", "that", "object"],
        ]
);
const SOURCE_IN_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "in", "your", "hand"],
            &["this", "card", "is", "in", "your", "hand"],
        ]
);
const SOURCE_IN_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "in", "your", "graveyard"],
            &["this", "card", "is", "in", "your", "graveyard"],
            &["this", "creature", "is", "in", "your", "graveyard"],
            &["this", "permanent", "is", "in", "your", "graveyard"],
            &["this", "object", "is", "in", "your", "graveyard"],
        ]
);
const SOURCE_IN_LIBRARY_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "in", "your", "library"],
            &["this", "card", "is", "in", "your", "library"],
        ]
);
const SOURCE_IN_EXILE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "in", "exile"],
            &["this", "card", "is", "in", "exile"],
        ]
);
const SOURCE_IN_COMMAND_ZONE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "in", "the", "command", "zone"],
            &["this", "card", "is", "in", "the", "command", "zone"],
        ]
);
const COST_PAID_INSTEAD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["cost", "was", "paid"], &["cost", "wasnt", "paid"]]);
const COST_NOT_PAID_INSTEAD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cost", "was", "not", "paid"]);
const MELD_ATTACKING_OWN_CONTROL_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "are",
            "attacking",
            "and",
            "you",
            "both",
            "own",
            "and",
            "control",
            "them",
        ]
);
const ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["a"], &["an"]]);
const DEFINITE_ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const WAS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["was"]);
const MANA_SPENT_TO_CAST_THIS_SPELL_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["was", "spent", "to", "cast", "this", "spell"],
            &["were", "spent", "to", "cast", "this", "spell"],
        ]
);
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
const YOUR_GRAVEYARD_WORDS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["your", "graveyard"]);
const YOU_BOTH_OWN_AND_CONTROL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "both", "own", "and"]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const THIS_WAY_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["this", "way"]);
const PASSIVE_THIS_WAY_COPULA_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["is"], &["are"], &["was"], &["were"]]);
const PASSIVE_THIS_WAY_VERB_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["countered"],
            &["destroyed"],
            &["discarded"],
            &["exiled"],
            &["milled"],
            &["returned"],
            &["revealed"],
            &["sacrificed"],
        ]
);
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
const NOT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["not"]);
const SOURCE_FILTER_IGNORED_DESCRIPTOR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["attached"], &["tapped"], &["untapped"], &["saddled"]]);
const SOURCE_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["its"]]);
const AURA_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["aura"], &["auras"]]);
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
const YOUR_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["your", "life", "total"]);
const THEIR_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["their", "life", "total"]);
const THAT_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "players", "life", "total"]);
const TARGET_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "players", "life", "total"]);
const TARGET_OPPONENTS_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "opponents", "life", "total"]);
const OPPONENT_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["opponents", "life", "total"],
            &["opponent", "life", "total"]
        ]
);
const DEFENDING_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["defending", "players", "life", "total"]);
const ATTACKING_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["attacking", "players", "life", "total"]);
const HALF_STARTING_LIFE_TOTAL_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["half", "your", "starting", "life", "total"],
            &["half", "their", "starting", "life", "total"],
            &["half", "that", "players", "starting", "life", "total"],
            &["half", "target", "players", "starting", "life", "total"],
            &["half", "target", "opponents", "starting", "life", "total"],
            &["half", "opponents", "starting", "life", "total"],
            &["half", "defending", "players", "starting", "life", "total"],
            &["half", "attacking", "players", "starting", "life", "total"],
        ]
);
const LESS_THAN_OR_EQUAL_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["less", "than", "or", "equal", "to"]);
const LESS_THAN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["less", "than"]);

fn source_zone_from_words(words: &[&str]) -> Option<Zone> {
    if SOURCE_IN_HAND_PATTERN.matches_words(words) {
        Some(Zone::Hand)
    } else if SOURCE_IN_GRAVEYARD_PATTERN.matches_words(words) {
        Some(Zone::Graveyard)
    } else if SOURCE_IN_LIBRARY_PATTERN.matches_words(words) {
        Some(Zone::Library)
    } else if SOURCE_IN_EXILE_PATTERN.matches_words(words) {
        Some(Zone::Exile)
    } else if SOURCE_IN_COMMAND_ZONE_PATTERN.matches_words(words) {
        Some(Zone::Command)
    } else {
        None
    }
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
    if !is_source_state_subject_words(&source.word_refs()) {
        return Ok(None);
    }
    let enchanted_by = matched
        .capture_clause("enchanted_by", clause)
        .ok_or_else(|| CardTextError::ParseError("missing enchanted-by phrase".to_string()))?;
    if !matches!(enchanted_by.word_refs().as_slice(), ["enchanted", "by"]) {
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
    let filter = parse_object_filter(filter_tokens, false).or_else(|_| {
        let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
        if AURA_WORD_PATTERN.matches_words(&filter_words) {
            Ok(ObjectFilter::default().with_subtype(Subtype::Aura))
        } else {
            Err(CardTextError::ParseError(format!(
                "unsupported attachment-count predicate tail (predicate: '{}')",
                words.join(" ")
            )))
        }
    })?;

    Ok(Some(PredicateAst::SourceHasAttachmentsMatching {
        filter,
        comparison,
        display: words.join(" "),
    }))
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
    if !is_source_reference_words(&source.word_refs()) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    let mut negative = matches!(
        action.word_refs().as_slice(),
        ["isnt"] | ["isn't"] | ["arent"] | ["aren't"]
    );
    let descriptor = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let descriptor_words = descriptor.word_refs();
    let mut descriptor_tokens = descriptor.tokens();
    if descriptor_words
        .first()
        .is_some_and(|word| NOT_WORD_PATTERN.matches_word(word))
    {
        negative = true;
        descriptor_tokens = descriptor_tokens.get(1..).unwrap_or_default();
    }
    if descriptor_tokens.is_empty() {
        return None;
    }
    let descriptor_words = crate::runtime_backend::token_word_refs(descriptor_tokens);
    if descriptor_words
        .iter()
        .any(|word| SOURCE_FILTER_IGNORED_DESCRIPTOR_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    let filter = parse_object_filter(descriptor_tokens, false).ok()?;
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
    let subject_words = source.word_refs();
    if !(is_source_reference_words(&subject_words)
        || SOURCE_REFERENCE_WORD_PATTERN.matches_words(&subject_words))
    {
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
        if matches!(player.word_refs().as_slice(), ["you"]) {
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
    if !matches!(subject.word_refs().as_slice(), ["your", "life", "total"]) {
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
    if !is_source_state_subject_words(&subject_clause.word_refs()) {
        return None;
    }
    let state_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    source_state_predicate_from_words(&state_clause.word_refs(), false)
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
    if !is_source_state_subject_words(&subject_clause.word_refs()) {
        return None;
    }
    let copula_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    let negative = match copula_clause.word_refs().as_slice() {
        ["is"] => false,
        ["isnt"] | ["isn't"] | ["is", "not"] => true,
        _ => return None,
    };
    let state_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    source_state_predicate_from_words(&state_clause.word_refs(), negative)
}

fn is_source_state_subject_words(words: &[&str]) -> bool {
    is_source_reference_words(words) || SOURCE_REFERENCE_WORD_PATTERN.matches_words(words)
}

fn source_state_predicate_from_words(words: &[&str], negative: bool) -> Option<PredicateAst> {
    match (words, negative) {
        (["tapped"], false) | (["untapped"], true) => Some(PredicateAst::SourceIsTapped),
        (["untapped"], false) | (["tapped"], true) => {
            Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)))
        }
        (["saddled"], false) => Some(PredicateAst::SourceIsSaddled),
        (["saddled"], true) => Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsSaddled))),
        _ => None,
    }
}

fn parse_terminal_counter_phrase(
    tokens: &[OwnedLexToken],
) -> Option<Option<ironsmith_core::counter::CounterType>> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let counter_idx = find_index(&words, |word| {
        COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
    })?;
    if counter_idx + 1 != words.len() {
        return None;
    }
    if counter_idx == 0 {
        return Some(None);
    }
    parse_counter_type_from_tokens(tokens.get(..=counter_idx)?).map(Some)
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
    if !is_source_reference_words(&source_clause.word_refs()) {
        return None;
    }
    let target_clause = matched.capture_clause("target", clause)?;
    if !is_exact_counter_on_source_tail(&target_clause.word_refs()) {
        return None;
    }
    let counter_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let counter_words = counter_clause.word_refs();
    if matches!(counter_words.as_slice(), ["no", ..]) {
        let counter_type = parse_terminal_counter_phrase(counter_clause.tokens().get(1..)?)??;
        return Some(PredicateAst::SourceHasNoCounter(counter_type));
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
    if !is_source_reference_words(&source_clause.word_refs()) {
        return None;
    }
    let target_clause = matched.capture_clause("target", clause)?;
    if !is_counter_on_source_pronoun_tail(&target_clause.word_refs()) {
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

fn is_counter_on_source_pronoun_tail(words: &[&str]) -> bool {
    matches!(
        words,
        ["on", "it"]
            | ["on", "him"]
            | ["on", "her"]
            | ["on", "them"]
            | ["on", "this"]
            | ["on", "that"]
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
    if !matches!(existential.word_refs().as_slice(), ["there", "are"]) {
        return None;
    }
    let target = matched.capture_clause("target", clause)?;
    if !is_exact_counter_on_source_tail(&target.word_refs()) {
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
    if !matches!(existential.word_refs().as_slice(), ["there", "are"]) {
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
    if !land_type_phrases
        .iter()
        .any(|phrase| land_types.word_refs().as_slice() == *phrase)
    {
        return Ok(None);
    }
    let controller = matched
        .capture_clause("controller", clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing controller in basic-land-types predicate".to_string(),
            )
        })?;
    let player = match controller.word_refs().as_slice() {
        ["you", "control"] | ["you", "controls"] => PlayerAst::You,
        ["that", "player", "control"]
        | ["that", "player", "controls"]
        | ["that", "players", "controls"] => PlayerAst::That,
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported basic-land-types predicate tail (predicate: '{}')",
                words.join(" ")
            )));
        }
    };
    Ok(Some(
        PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count },
    ))
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
    if !matches!(existential.word_refs().as_slice(), ["there", "are"]) {
        return None;
    }
    let target = matched.capture_clause("target", clause)?;
    if !is_exact_counter_on_source_tail(&target.word_refs()) {
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

fn is_exact_counter_on_source_tail(words: &[&str]) -> bool {
    matches!(words, ["on", tail @ ..] if is_source_state_subject_words(tail))
}

fn parse_source_exiled_with_counter_predicate(
    raw_words: &[&str],
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let with_idx = if SOURCE_EXILED_WITH_COUNTER_PREFIX_PATTERN.matches_words(raw_words) {
        4
    } else {
        return None;
    };
    let counter_idx = find_index(&raw_words[with_idx + 1..], |word| {
        COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
    })? + with_idx
        + 1;
    if !raw_words
        .get(counter_idx + 1..)
        .is_some_and(|tail| COUNTER_ON_SOURCE_TAIL_PATTERN.matches_words(tail))
    {
        return None;
    }

    let counter_type = parse_counter_type_from_tokens(&tokens[with_idx + 1..=counter_idx])?;
    let count = parse_number(&tokens[with_idx + 1..counter_idx])
        .map(|(count, _)| count)
        .unwrap_or(1);
    Some(PredicateAst::And(
        Box::new(PredicateAst::SourceIsInZone(Zone::Exile)),
        Box::new(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
        }),
    ))
}

fn parse_source_is_your_ring_bearer_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["is"])),
        LexPattern::action("copula", LexCaptureKind::OneOf(&["is"])),
        LexPattern::object("role", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(
        source.word_refs().as_slice(),
        ["this"] | ["this", "creature"]
    ) {
        return None;
    }
    let role = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(role.word_refs().as_slice(), ["your", "ring", "bearer"]) {
        return None;
    }
    Some(PredicateAst::SourceIsRingBearer {
        player: PlayerAst::You,
    })
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
    if !matches!(tempted.word_refs().as_slice(), ["has", "tempted", "you"]) {
        return None;
    }
    let window = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !matches!(
        window.word_refs().as_slice(),
        ["time" | "times", "this", "game"]
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

fn parse_ring_bearer_temptation_predicate(
    words: &[&str],
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    if let Some(predicate) = parse_source_is_your_ring_bearer_predicate(words) {
        return Some(predicate);
    }
    if let Some(predicate) = parse_ring_has_tempted_you_this_game_predicate(tokens) {
        return Some(predicate);
    }

    let and_idx = find_index(words, |word| AND_WORD_PATTERN.matches_word(word))?;
    let left_words = &words[..and_idx];
    let right_words = &words[and_idx + 1..];
    if left_words.is_empty() || right_words.is_empty() {
        return None;
    }
    let left = parse_source_is_your_ring_bearer_predicate(left_words)?;
    let right = parse_ring_has_tempted_you_this_game_predicate(&tokens[and_idx + 1..])?;
    Some(PredicateAst::And(Box::new(left), Box::new(right)))
}

fn parse_stack_object_targets_only_source_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tail = if THAT_SPELL_PREFIX_PATTERN.matches_words(filtered) {
        &filtered[2..]
    } else if SPELL_PREFIX_PATTERN.matches_words(filtered)
        || IT_PREFIX_PATTERN.matches_words(filtered)
    {
        &filtered[1..]
    } else {
        return None;
    };

    if !TARGETS_ONLY_PREFIX_PATTERN.matches_words(tail) {
        return None;
    }

    let target_words = &tail[2..];
    let mut target_filter = if TARGET_THIS_CREATURE_PATTERN.matches_words(target_words) {
        ObjectFilter::creature()
    } else if TARGET_THIS_ARTIFACT_PATTERN.matches_words(target_words) {
        ObjectFilter::artifact()
    } else if TARGET_THIS_ENCHANTMENT_PATTERN.matches_words(target_words) {
        ObjectFilter::enchantment()
    } else if TARGET_THIS_LAND_PATTERN.matches_words(target_words) {
        ObjectFilter::land()
    } else if TARGET_THIS_PERMANENT_PATTERN.matches_words(target_words) {
        ObjectFilter::default().in_zone(Zone::Battlefield)
    } else if TARGET_SOURCE_REFERENCE_PATTERN.matches_words(target_words) {
        ObjectFilter::source()
    } else {
        return None;
    };
    target_filter.source = true;

    Some(PredicateAst::ItMatches(
        ObjectFilter::spell()
            .targeting_only_object(target_filter)
            .target_count_exact(1),
    ))
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
    let (comparison, used) = predicate_quantity_prefix(words)?;
    let count = comparison_to_strict_at_least_threshold(&comparison)?;
    Some((count, used))
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
    let (player, controller) = match subject_clause.word_refs().as_slice() {
        ["you"] => (PlayerAst::You, PlayerFilter::You),
        ["player"] => (PlayerAst::Any, PlayerFilter::Any),
        _ => return None,
    };
    let amount_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let tagged_neither = match amount_clause.word_refs().as_slice() {
        ["no"] => false,
        ["neither"] if player == PlayerAst::You => true,
        _ => return None,
    };
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
    if !matches!(subject_clause.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let negation_clause = matched.capture_clause("negation", clause)?;
    if !matches!(
        negation_clause.word_refs().as_slice(),
        ["dont"] | ["don't"] | ["do", "not"]
    ) {
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
    if !matches!(controller_clause.word_refs().as_slice(), ["you"]) {
        return None;
    }

    let control_object = matched.capture_clause("control_object", clause)?;
    if control_object.word_refs().is_empty() {
        return None;
    }

    let graveyard_object = matched.capture_clause("graveyard_object", clause)?;
    let graveyard_tokens = graveyard_object.tokens();
    let mut graveyard_words = graveyard_object.word_refs();
    let existential_prefix_len = match graveyard_words.as_slice() {
        ["there", "is" | "are", ..] => 2,
        ["there", ..] => 1,
        _ => 0,
    };
    if existential_prefix_len > 0 {
        graveyard_words.drain(0..existential_prefix_len);
    }
    let graveyard_tokens = &graveyard_tokens[existential_prefix_len..];
    if graveyard_tokens.is_empty() || !YOUR_GRAVEYARD_WORDS_PATTERN.matches_words(&graveyard_words)
    {
        return None;
    }

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
    if !matches!(controller_clause.word_refs().as_slice(), ["you"]) {
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
            return Ok(Some(
                PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                    player,
                    filter,
                    count,
                },
            ));
        }
        return Ok(Some(PredicateAst::PlayerControlsAtLeast {
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
                    return PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                        player: control_condition.player,
                        filter: control_condition.filter,
                        count,
                    };
                }
                return PredicateAst::PlayerControlsAtLeast {
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

fn parse_color_only_object_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let mut filter = ObjectFilter::default();
    let mut saw_color = false;
    for word in words {
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

fn parse_this_way_object_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let (words, needs_chosen_name) = if let Some(base_words) =
        crate::runtime_backend::lexer::word_slice_strip_suffix(words, &["with", "chosen", "name"])
    {
        (base_words, true)
    } else {
        (words, false)
    };
    let has_card_noun = words
        .last()
        .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word));
    let candidates = [
        (words, has_card_noun),
        (
            crate::runtime_backend::lexer::word_slice_strip_suffix(words, &["card"])
                .unwrap_or(words),
            true,
        ),
        (
            crate::runtime_backend::lexer::word_slice_strip_suffix(words, &["cards"])
                .unwrap_or(words),
            true,
        ),
    ];
    for (candidate, stripped_card_noun) in candidates {
        if candidate.is_empty() {
            let mut filter = ObjectFilter::default();
            if needs_chosen_name {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: TagKey::from(CHOSEN_NAME_TAG),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                });
            }
            return Some(filter);
        }
        let tokens = predicate_tokens_from_words(candidate);
        if let Ok(mut filter) = parse_object_filter(&tokens, false) {
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
        if let Some(mut filter) = parse_color_only_object_filter_words(candidate) {
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
    if filtered.len() < 5 || !THIS_WAY_SUFFIX_PATTERN.matches_words(filtered) {
        return Ok(None);
    }
    let verb_idx = filtered.len() - 3;
    let copula_idx = verb_idx.saturating_sub(1);
    if copula_idx == 0
        || !PASSIVE_THIS_WAY_COPULA_WORD_PATTERN.matches_word(filtered[copula_idx])
        || !PASSIVE_THIS_WAY_VERB_WORD_PATTERN.matches_word(filtered[verb_idx])
    {
        return Ok(None);
    }

    let filter_words = &filtered[..copula_idx];
    let Some(filter) = parse_this_way_object_filter_words(filter_words) else {
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
    if filtered.len() < 5 || !THIS_WAY_SUFFIX_PATTERN.matches_words(filtered) {
        return Ok(None);
    }

    let Some((player, subject_len)) = active_discard_player_subject(filtered) else {
        return Ok(None);
    };
    let Some(verb) = filtered.get(subject_len) else {
        return Ok(None);
    };
    if !matches!(*verb, "discard" | "discards" | "discarded") {
        return Ok(None);
    }

    let filter_words = &filtered[subject_len + 1..filtered.len() - 2];
    let Some(filter) = parse_this_way_object_filter_words(filter_words) else {
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
    if !matches!(subject_clause.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let negation_clause = matched.capture_clause("negation", clause)?;
    if !matches!(
        negation_clause.word_refs().as_slice(),
        ["dont"] | ["didnt"] | ["did", "not"]
    ) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action_clause.word_refs().as_slice(), ["put"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        object_clause.word_refs().as_slice(),
        ["the", "card"] | ["that", "card"] | ["card"] | ["it"]
    ) {
        return None;
    }
    let destination_clause = matched.capture_clause("destination", clause)?;
    let zone = match destination_clause.word_refs().as_slice() {
        ["into", "your", "hand"] => Zone::Hand,
        ["onto", "battlefield"] | ["onto", "the", "battlefield"] => Zone::Battlefield,
        _ => return None,
    };
    Some(PredicateAst::Not(Box::new(
        PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter: ObjectFilter::default().in_zone(zone),
        },
    )))
}

fn parse_active_this_way_battlefield_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    if filtered.len() < 7 || !THIS_WAY_SUFFIX_PATTERN.matches_words(filtered) {
        return Ok(None);
    }
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
    if !matches!(subject_clause.word_refs().as_slice(), ["you"]) {
        return Ok(None);
    }
    let destination_clause = matched
        .capture_clause("destination", clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing destination in this-way battlefield predicate".to_string(),
            )
        })?;
    if !matches!(
        destination_clause.word_refs().as_slice(),
        ["onto", "battlefield", "this", "way"] | ["onto", "the", "battlefield", "this", "way"]
    ) {
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
    if filtered.len() < 7 || !THIS_WAY_SUFFIX_PATTERN.matches_words(filtered) {
        return Ok(None);
    }
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
    if !matches!(action_clause.word_refs().as_slice(), ["is", "put"]) {
        return Ok(None);
    }
    let destination_clause = matched
        .capture_clause("destination", clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing destination in passive this-way battlefield predicate".to_string(),
            )
        })?;
    if !matches!(
        destination_clause.word_refs().as_slice(),
        ["onto", "battlefield", "this", "way"] | ["onto", "the", "battlefield", "this", "way"]
    ) {
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

fn active_discard_player_subject(words: &[&str]) -> Option<(PlayerAst, usize)> {
    match words {
        ["you", ..] => Some((PlayerAst::You, 1)),
        ["that", "player" | "players", ..] => Some((PlayerAst::That, 2)),
        ["target", "player", ..] => Some((PlayerAst::Target, 2)),
        ["target", "opponent", ..] => Some((PlayerAst::TargetOpponent, 2)),
        ["opponent" | "opponents", ..] => Some((PlayerAst::Opponent, 1)),
        _ => None,
    }
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
    let Some(or_idx) = rfind_index_with(filtered, |idx, word| {
        if !OR_WORD_PATTERN.matches_word(word) || idx == 0 || idx + 1 >= filtered.len() {
            return false;
        }
        if filtered
            .get(idx + 1)
            .is_some_and(|word| OR_COMPARISON_TAIL_WORD_PATTERN.matches_word(word))
        {
            return false;
        }
        true
    }) else {
        return Ok(None);
    };

    let left_words = &filtered[..or_idx];
    let right_words = &filtered[or_idx + 1..];
    let left_tokens = predicate_tokens_from_words(left_words);
    let right_tokens = predicate_tokens_from_words(right_words);
    let left = parse_predicate(&left_tokens)?;
    let right = match parse_predicate(&right_tokens) {
        Ok(predicate) => predicate,
        Err(original_err) => {
            let Some(reference_prefix) = predicate_reference_prefix(left_words) else {
                return Err(original_err);
            };
            if predicate_words_start_with_reference(right_words) {
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
    if !matches!(
        zone.word_refs().as_slice(),
        ["battlefield"] | ["the", "battlefield"]
    ) {
        return None;
    }
    Some(PredicateAst::PlayerControlsNo {
        player: PlayerAst::Any,
        filter: ObjectFilter::creature(),
    })
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
    if !matches!(
        second_player.word_refs().as_slice(),
        ["player", "youre", "attacking"] | ["a", "player", "youre", "attacking"]
    ) {
        return None;
    }
    let status = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        status.word_refs().as_slice(),
        ["initiative"] | ["the", "initiative"]
    ) {
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
    if !matches!(phase.word_refs().as_slice(), ["first", "combat", "phase"]) {
        return None;
    }
    Some(PredicateAst::FirstCombatPhaseOfTurn)
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
    if !matches!(object.word_refs().as_slice(), ["this", "spell"]) {
        return None;
    }
    let phase = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !matches!(phase.word_refs().as_slice(), ["your", "main", "phase"]) {
        return None;
    }
    Some(PredicateAst::ThisSpellPaidLabel(
        "CastDuringYourMainPhase".to_string(),
    ))
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
    if !matches!(turn_clause.word_refs().as_slice(), ["your", "turn"]) {
        return None;
    }
    let predicate = PredicateAst::YourTurn;
    if matched.capture("negation").is_some() {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
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
    if !matches!(
        controller.word_refs().as_slice(),
        ["opponent"] | ["an", "opponent"]
    ) {
        return None;
    }
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let mut filter = ObjectFilter {
        controller: Some(PlayerFilter::Opponent),
        ..Default::default()
    };
    match object.word_refs().as_slice() {
        ["it"] | ["that", "permanent"] => {}
        ["that", "creature"] => filter.card_types.push(CardType::Creature),
        _ => return None,
    }
    Some(PredicateAst::ItMatches(filter))
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
    if !matches!(
        subject.word_refs().as_slice(),
        ["they"] | ["those", "choices"]
    ) {
        return None;
    }
    Some(PredicateAst::SecretChoicesMatch)
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
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::amount("subject", LexCaptureKind::OneOf(&["x"])),
        LexPattern::action("copula", LexCaptureKind::OneOf(&["is"])),
        LexPattern::modifier("comparison", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let comparison_clause = matched.capture_clause("comparison", clause)?;
    let comparison_words = comparison_clause.word_refs();
    let (comparison, used) = predicate_quantity_prefix(&comparison_words)?;
    if used != comparison_words.len() {
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
    let negated = match paid_tail.word_refs().as_slice() {
        ["cost", "was", "paid"] => false,
        ["cost", "wasnt", "paid"] | ["cost", "was", "not", "paid"] => true,
        _ => return None,
    };
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
    match result.word_refs().as_slice() {
        ["more", "votes"] => Some(PredicateAst::VoteOptionGetsMoreVotes {
            option: option.word_refs().join(" "),
        }),
        ["more", "votes", "or", "vote", "is", "tied"] if allow_tied => {
            Some(PredicateAst::VoteOptionGetsMoreVotesOrTied {
                option: option.word_refs().join(" "),
            })
        }
        _ => None,
    }
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
    if !matches!(action.word_refs().as_slice(), ["got", "votes"]) {
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
    if !matches!(subject_clause.word_refs().as_slice(), ["it"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        object_clause.word_refs().as_slice(),
        ["player"] | ["a", "player"]
    ) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !matches!(window_clause.word_refs().as_slice(), ["this", "turn"]) {
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
    let player = match subject_clause.word_refs().as_slice() {
        ["a", "player"] | ["player"] => PlayerAst::Any,
        ["an", "opponent"] | ["opponent"] => PlayerAst::Opponent,
        _ => return None,
    };
    let subtype_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let subtype_words = subtype_clause.word_refs();
    let subtype_word = match subtype_words.as_slice() {
        [word] => *word,
        ["a" | "an", word] => *word,
        _ => return None,
    };
    let subtype = parse_subtype_word(subtype_word)?;
    let window_clause = matched.capture_clause("window", clause)?;
    if !matches!(window_clause.word_refs().as_slice(), ["this", "turn"]) {
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
    if !matches!(subject_clause.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action_clause.word_refs().as_slice(), ["attacked"]) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !matches!(window_clause.word_refs().as_slice(), ["this", "turn"]) {
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
        if !matches!(
            subject_clause.word_refs().as_slice(),
            ["that", "creature"] | ["it"]
        ) {
            continue;
        }
        let window_clause = matched.capture_clause("window", clause)?;
        if !matches!(window_clause.word_refs().as_slice(), ["this", "combat"]) {
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
    if !matches!(subject_clause.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(
        action_clause.word_refs().as_slice(),
        ["attacked", "with", "exactly"]
    ) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        object_clause.word_refs().as_slice(),
        ["other", "creature" | "creatures", "this", "combat"]
            | ["others", "creature" | "creatures", "this", "combat"]
    ) {
        return None;
    }
    let count_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let count_words = count_clause.word_refs();
    let (count, used) = predicate_number_prefix(&count_words)?;
    if used != count_words.len() {
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
    if !matches!(
        subject_clause.word_refs().as_slice(),
        ["this", "creature"] | ["this", "permanent"] | ["this"] | ["it"]
    ) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(
        action_clause.word_refs().as_slice(),
        ["attacked", "or", "blocked"]
    ) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !matches!(window_clause.word_refs().as_slice(), ["this", "turn"]) {
        return None;
    }
    Some(PredicateAst::SourceAttackedOrBlockedThisTurn)
}

fn parse_source_did_not_attack_or_enter_control_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let enter_phrases: &[&[&str]] = &[
        &["or", "come", "under", "your", "control"],
        &["or", "came", "under", "your", "control"],
    ];
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
    if !matches!(subject_clause.word_refs().as_slice(), ["this", "creature"]) {
        return None;
    }
    let enter_clause = matched.capture_clause("enter", clause)?;
    if !enter_phrases
        .iter()
        .any(|phrase| enter_clause.word_refs().as_slice() == *phrase)
    {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !matches!(window_clause.word_refs().as_slice(), ["this", "turn"]) {
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

fn parse_you_cast_source_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["cast"])),
        LexPattern::action("action", LexCaptureKind::WordCount(1)),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(subject_clause.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action_clause.word_refs().as_slice(), ["cast"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        object_clause.word_refs().as_slice(),
        ["it"] | ["this", "spell"]
    ) {
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
    if !matches!(
        subject_clause.word_refs().as_slice(),
        ["it"] | ["that", "creature"] | ["that", "permanent"] | ["that", "object"]
    ) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action_clause.word_refs().as_slice(), ["was", "cast"]) {
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
    if !matches!(subject_clause.word_refs().as_slice(), ["this", "spell"]) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(
        action_clause.word_refs().as_slice(),
        ["was", "cast", "from"]
    ) {
        return None;
    }
    let origin_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let origin_words = origin_clause.word_refs();
    if matches!(
        origin_words.as_slice(),
        ["anywhere", "other", "than", "your", "hand"]
    ) {
        return Some(PredicateAst::ThisSpellWasCastFromNonHand);
    }
    let zone_words = origin_words.as_slice();
    let zone = if zone_words.len() == 1 {
        parse_zone_word(zone_words[0])
    } else if zone_words.len() == 2
        && (is_article(zone_words[0]) || DEFINITE_ARTICLE_WORD_PATTERN.matches_word(zone_words[0]))
    {
        parse_zone_word(zone_words[1])
    } else {
        None
    }?;
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
    if !matches!(amount_clause.word_refs().as_slice(), ["no"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(object_clause.word_refs().as_slice(), ["spell" | "spells"]) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(
        action_clause.word_refs().as_slice(),
        ["was" | "were", "cast"]
    ) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !matches!(window_clause.word_refs().as_slice(), ["last", "turn"]) {
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
    if !matches!(
        subject_clause.word_refs().as_slice(),
        ["this", "spell"] | ["this", "creature"] | ["this", "permanent"] | ["it"]
    ) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action_clause.word_refs().as_slice(), ["was", "kicked"]) {
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
    if !matches!(
        subject_clause.word_refs().as_slice(),
        ["this", "spell"] | ["it"]
    ) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action_clause.word_refs().as_slice(), ["was", "bargained"]) {
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
    let label_word = label.to_ascii_lowercase();
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::object("label", LexCaptureKind::UntilPhrase(action_phrase)),
        LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let label_clause = matched.capture_clause("label", clause)?;
    if !matches!(label_clause.word_refs().as_slice(), [word] if *word == label_word) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if action_clause.word_refs().as_slice() != action_phrase {
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
    let atoms = [
        LexPattern::object("beheld", LexCaptureKind::UntilPhrase(&["beheld"])),
        LexPattern::action("action", LexCaptureKind::WordCount(1)),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let beheld_clause = matched.capture_clause("beheld", clause)?;
    let beheld_words = beheld_clause.word_refs();
    let subtype_words = if matches!(beheld_words.first(), Some(&word) if ARTICLE_WORD_PATTERN.matches_word(word))
    {
        &beheld_words[1..]
    } else {
        beheld_words.as_slice()
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
    if !matches!(subject_clause.word_refs().as_slice(), ["that"]) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action_clause.word_refs().as_slice(), ["was", "kicked"]) {
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
    let spent_phrases: &[&[&str]] = &[
        &["was", "spent", "to", "cast", "this", "spell"],
        &["were", "spent", "to", "cast", "this", "spell"],
    ];
    let atoms = [
        LexPattern::amount("symbols", LexCaptureKind::UntilAnyPhrase(spent_phrases)),
        LexPattern::action("spent", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let spent_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !MANA_SPENT_TO_CAST_THIS_SPELL_TAIL_PATTERN.matches_words(&spent_clause.word_refs()) {
        return None;
    }
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
        if !matches!(
            subject_clause.word_refs().as_slice(),
            ["this", "permanent"] | ["that", "permanent"]
        ) {
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
    if !matches!(
        subject_clause.word_refs().as_slice(),
        ["any", "of", "those", "cards"] | ["those", "cards"] | ["that", "card"] | ["it"]
    ) {
        return None;
    }
    let zone_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(zone_clause.word_refs().as_slice(), ["exiled"]) {
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
    if !matches!(subject_clause.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(object_clause.word_refs().as_slice(), ["that", "permanent"]) {
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
    if !matches!(
        subject_clause.word_refs().as_slice(),
        ["it"] | ["that", "card"] | ["that", "permanent"]
    ) {
        return None;
    }
    let controller_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        controller_clause.word_refs().as_slice(),
        ["your", "control"]
    ) {
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
        if !matches!(
            subject_clause.word_refs().as_slice(),
            ["it"] | ["that", "creature"]
        ) {
            continue;
        }
        let state_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        if !matches!(state_clause.word_refs().as_slice(), ["blocking"]) {
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
        if !matches!(subject_clause.word_refs().as_slice(), ["it"]) {
            continue;
        }
        let partner_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        if !matches!(
            partner_clause.word_refs().as_slice(),
            ["creature"] | ["another", "creature"]
        ) {
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
    let tag = match tagged_clause.word_refs().as_slice() {
        ["equipped", "creature"] => "equipped",
        ["enchanted", "creature"] => "enchanted",
        _ => return None,
    };
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
    if !matches!(tail.word_refs().as_slice(), ["you"] | ["you", "do"]) {
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
    if !matches!(
        controller.word_refs().as_slice(),
        ["opponent"] | ["an", "opponent"]
    ) {
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

    Some(PredicateAst::PlayerControls {
        player: PlayerAst::Opponent,
        filter,
    })
}

fn parse_player_object_keyword_predicate(
    words: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
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
        let mut filter = parse_object_filter(subject.tokens(), false)?;
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        filter
    } else {
        return Ok(None);
    };

    apply_filter_keyword_constraint(&mut filter, constraint, false);
    Ok(Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter,
    }))
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
    let graveyard_start = if words.len() == 8
        && words
            .get(3..4)
            .is_some_and(|tail| PERMANENTS_AND_OR_GRAVEYARD_CONNECTOR_PATTERN.matches_words(tail))
    {
        4
    } else if words.len() == 9
        && words
            .get(3..5)
            .is_some_and(|tail| PERMANENTS_AND_OR_SPLIT_CONNECTOR_PATTERN.matches_words(tail))
    {
        5
    } else {
        return None;
    };
    let battlefield = permanents_you_control_scope(&words[..3])?;
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
    if !THERE_ARE_OR_WERE_PREFIX_PATTERN.matches_words(&existential.word_refs()) {
        return None;
    }

    let quantity = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let quantity_words = quantity.word_refs();
    let (count, used) = predicate_number_prefix(&quantity_words)?;
    if used != quantity_words.len() {
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
    if !THERE_ARE_OR_WERE_PREFIX_PATTERN.matches_words(&existential.word_refs()) {
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
    let (counter_constraint, consumed) = parse_filter_counter_constraint_words(&counter_words)?;
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
    if !matches!(revealer.word_refs().as_slice(), ["you"]) {
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

fn parse_card_in_your_graveyard_predicate(words: &[&str]) -> Option<PredicateAst> {
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
    if !matches!(existential.word_refs().as_slice(), ["there", "is"]) {
        return None;
    }

    let location = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !IN_YOUR_GRAVEYARD_TAIL_PATTERN.matches_words(&location.word_refs()) {
        return None;
    }

    let descriptor = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if descriptor.word_refs().is_empty() {
        return None;
    }
    let mut filter = parse_object_filter(descriptor.tokens(), false).ok()?;
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
    if !matches!(
        location.word_refs().as_slice(),
        ["battlefield"] | ["the", "battlefield"]
    ) {
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
        && let Some(named_idx) = object_tokens
            .iter()
            .position(|token| token.is_word("named"))
    {
        let object_words = object_tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>();
        let name_end = find_name_clause_end(&object_words, named_idx + 1);
        let name = object_tokens[named_idx + 1..name_end]
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>()
            .join(" ");
        if !name.is_empty() {
            filter.name = Some(name);
        }
    }
    filter.zone = Some(Zone::Battlefield);

    Ok(Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThan,
        right: Value::Fixed(0),
    }))
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
    let constrained_player = match lead.word_refs().as_slice() {
        ["there", "are"] => None,
        ["you", "have"] => Some(PlayerAst::You),
        _ => return None,
    };
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
    if !matches!(existential.word_refs().as_slice(), ["there", "are"]) {
        return Ok(None);
    }
    let location = matched
        .capture_clause_by_role(LexCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing location in battlefield count predicate".to_string())
        })?;
    if !matches!(
        location.word_refs().as_slice(),
        ["battlefield"] | ["the", "battlefield"]
    ) {
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

    if let Some(predicate) = parse_source_exiled_with_counter_predicate(&raw_words, tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_happily_style_conjoined_predicate(&filtered) {
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

    if let Some(predicate) = parse_object_on_battlefield_predicate(tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_colors_among_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_types_among_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_life_total_at_least_starting_predicate(&filtered) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_life_total_at_least_last_noted_predicate(&filtered) {
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

    if let Some(attacking_idx) = (0..filtered.len())
        .find(|idx| MELD_ATTACKING_OWN_CONTROL_TAIL_PATTERN.matches_words(&filtered[*idx..]))
        && let Some(and_idx) = find_meld_subject_split(&filtered[..attacking_idx])
    {
        let left_words = &filtered[..and_idx];
        let right_words = &filtered[and_idx + 1..attacking_idx];
        if !left_words.is_empty() && !right_words.is_empty() {
            let mut left_filter = parse_meld_subject_filter(left_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate subject (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            left_filter.controller = Some(PlayerFilter::You);
            left_filter.attacking = true;

            let mut right_filter = parse_meld_subject_filter(right_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate tail (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            right_filter.controller = Some(PlayerFilter::You);
            right_filter.attacking = true;

            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if filtered.len() >= 8
        && YOU_BOTH_OWN_AND_CONTROL_PREFIX_PATTERN.matches_words(&filtered)
        && filtered
            .get(4)
            .is_some_and(|word| CONTROL_OR_CONTROLS_WORD_PATTERN.matches_word(word))
        && let Some(and_idx) = find_meld_subject_split(&filtered[5..])
    {
        let and_idx = 5 + and_idx;
        if and_idx > 5 && and_idx + 1 < filtered.len() {
            let mut left_filter =
                parse_meld_subject_filter(&filtered[5..and_idx]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate subject (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            left_filter.controller = Some(PlayerFilter::You);
            let mut right_filter =
                parse_meld_subject_filter(&filtered[and_idx + 1..]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate tail (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            right_filter.controller = Some(PlayerFilter::You);
            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if let Some(and_idx) = find_index(&filtered, |word| AND_WORD_PATTERN.matches_word(word))
        && and_idx > 0
        && and_idx + 1 < filtered.len()
    {
        let right_first = filtered.get(and_idx + 1).copied();
        if right_first.is_some_and(|word| {
            HAVE_WORD_PATTERN.matches_word(word) || YOU_WORD_PATTERN.matches_word(word)
        }) {
            let left_words = &filtered[..and_idx];
            let mut right_words = filtered[and_idx + 1..].to_vec();
            if right_words
                .first()
                .is_some_and(|word| HAVE_WORD_PATTERN.matches_word(word))
            {
                right_words.insert(0, "you");
            }
            let left_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(left_words);
            let right_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(right_words);
            let left = parse_predicate(&left_tokens)?;
            let right = parse_predicate(&right_tokens)?;
            return Ok(PredicateAst::And(Box::new(left), Box::new(right)));
        }
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

    let parse_life_total_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        if YOUR_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::You, 3))
        } else if THEIR_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::That, 3))
        } else if THAT_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::That, 4))
        } else if TARGET_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Target, 4))
        } else if TARGET_OPPONENTS_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::TargetOpponent, 4))
        } else if OPPONENT_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Opponent, 3))
        } else if DEFENDING_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Defending, 4))
        } else if ATTACKING_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Attacking, 4))
        } else {
            None
        }
    };
    if let Some((player, subject_len)) = parse_life_total_subject(&filtered)
        && filtered
            .get(subject_len)
            .is_some_and(|word| IS_OR_ARE_WORD_PATTERN.matches_word(word))
    {
        let tail = &filtered[subject_len + 1..];
        if LESS_THAN_OR_EQUAL_TO_PREFIX_PATTERN.matches_words(tail)
            && HALF_STARTING_LIFE_TOTAL_TAIL_PATTERN.matches_words(&tail[5..])
        {
            return Ok(PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { player });
        }
        if LESS_THAN_PREFIX_PATTERN.matches_words(tail)
            && HALF_STARTING_LIFE_TOTAL_TAIL_PATTERN.matches_words(&tail[2..])
        {
            return Ok(PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { player });
        }
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

    if let Some(predicate) = parse_empty_battlefield_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_achievement_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_ring_bearer_temptation_predicate(&filtered, tokens) {
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
    use crate::effect::ValueComparisonOperator;
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
                PredicateAst::PlayerControlsAtLeast {
                    player: PlayerAst::You,
                    filter: ObjectFilter::artifact().controlled_by(PlayerFilter::You),
                    count: 3,
                },
            ),
            (
                "If you control three or more creatures with different powers",
                PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
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
        for (text, expected_player, expected_operator, expected_count) in [
            (
                "If you have seven or more cards in your graveyard",
                PlayerFilter::You,
                ValueComparisonOperator::GreaterThanOrEqual,
                7,
            ),
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
            panic!("expected card-types-among value comparison");
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

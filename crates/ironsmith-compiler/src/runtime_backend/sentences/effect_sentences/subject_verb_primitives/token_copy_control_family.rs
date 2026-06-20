use super::*;
use crate::runtime_backend::effect_sentences::parse_artifact_enchantment_or_token_filter;
use crate::runtime_backend::front_end::lex_patterns::{
    LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom, LexPatternMatch,
};

const ALL_OR_EACH_WORDS: &[&str] = &["all", "each"];
const RETURN_WORD: &str = "return";
const CREATE_WORD: &str = "create";
const RETURN_WORDS: &[&str] = &["return"];
const CHOOSE_WORDS: &[&str] = &["choose"];
const SACRIFICE_WORDS: &[&str] = &["sacrifice"];
const TARGET_HEAD_WORDS: &[&str] = &["target"];
const WHERE_X_IS_WORDS: &[&str] = &["where", "x", "is"];
const PUTS_ALL_PERMANENT_CARDS_PREFIX: &[&str] = &["puts", "all", "permanent", "cards"];
const REVEALED_THIS_WAY_PHRASE: &[&str] = &["revealed", "this", "way"];
const ONTO_THE_BATTLEFIELD_PHRASE: &[&str] = &["onto", "the", "battlefield"];
const EACH_PLAYER_REVEALS_TOP_PREFIX: &[&str] = &[
    "each", "player", "reveals", "a", "number", "of", "cards", "from", "the", "top", "of", "their",
    "library", "equal", "to",
];
const EACH_PLAYER_PUTS_REST_GRAVEYARD: &[&str] =
    &["puts", "the", "rest", "into", "their", "graveyard"];
const TOKEN_COPY_EXILE_LEADING_WORDS: &[&str] = &["exile"];
const TOKEN_COPY_YOU_EXILE_LEADING_WORDS: &[&str] = &["you", "exile"];
const YOU_EXILE_PREFIX: &[&str] = &["you", "exile"];
const EXILE_HEAD_PREFIX: &[&str] = &["exile"];
const TOKEN_COPY_AND_OR_WORDS: &[&str] = &["and", "or"];

fn where_x_is_prefixed_clause(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> SubjectVerbPrimitiveOwnedClause {
    let mut tokens: Vec<OwnedLexToken> = WHERE_X_IS_WORDS
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect();
    tokens.extend_from_slice(clause.tokens());
    SubjectVerbPrimitiveOwnedClause::new(tokens)
}
const TOKEN_COPY_DEAL_OR_DEALS_WORDS: &[&str] = &["deal", "deals"];
const SHUFFLE_OR_SHUFFLES_WORDS: &[&str] = &["shuffle", "shuffles"];
const GRAVEYARD_OR_GRAVEYARDS_WORDS: &[&str] = &["graveyard", "graveyards"];
const LIBRARY_OR_LIBRARIES_WORDS: &[&str] = &["library", "libraries"];
const IT_OR_THEM_PHRASES: &[&[&str]] = &[&["it"], &["them"]];
const TOKEN_COPY_EXACT_TARGET_REFERENCE_PHRASES: &[&[&str]] = &[&["you"], &["it"]];
const TOKEN_COPY_THAT_TARGET_REFERENCE_PHRASES: &[&[&str]] = &[
    &["that", "creature"],
    &["that", "permanent"],
    &["that", "land"],
    &["that", "artifact"],
    &["that", "enchantment"],
];
const TOKEN_COPY_TIMING_TAIL_WORDS: &[&str] =
    &["at", "beginning", "end", "combat", "turn", "step", "until"];
const TOKEN_COPY_TARGET_HEAD_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "target_head",
    LexCaptureKind::OneOf(TARGET_HEAD_WORDS),
)]);
const TOKEN_COPY_EXACT_TARGET_REFERENCE_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "reference",
        LexCaptureKind::OneOfPhrase(TOKEN_COPY_EXACT_TARGET_REFERENCE_PHRASES),
    )]);
const TOKEN_COPY_THAT_TARGET_REFERENCE_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "reference",
        LexCaptureKind::OneOfPhrase(TOKEN_COPY_THAT_TARGET_REFERENCE_PHRASES),
    )]);
const TOKEN_COPY_TIMING_TAIL_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "timing",
        LexCaptureKind::OneOf(TOKEN_COPY_TIMING_TAIL_WORDS),
    )]);
const EACH_PLAYER_PUTS_REVEALED_PERMANENTS_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(PUTS_ALL_PERMANENT_CARDS_PREFIX),
    LexPattern::role_capture(
        "revealed_filter",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(REVEALED_THIS_WAY_PHRASE),
    ),
    LexPattern::phrase(REVEALED_THIS_WAY_PHRASE),
    LexPattern::role_capture(
        "destination_prefix",
        LexCaptureRole::Modifier,
        LexCaptureKind::UntilPhrase(ONTO_THE_BATTLEFIELD_PHRASE),
    ),
    LexPattern::phrase(ONTO_THE_BATTLEFIELD_PHRASE),
]);
const TOKEN_COPY_THAT_PLAYER_TAIL_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::subject(
        "player",
        LexCaptureKind::OneOfPhrase(THAT_PLAYER_TAIL_PHRASES),
    )]);
const THAT_PLAYER_TAIL_PHRASES: &[&[&str]] = &[&["that", "player"]];
const TOKEN_COPY_TO_PHRASE: &[&str] = &["to"];
const TOKEN_COPY_INTO_PHRASE: &[&str] = &["into"];
const TOKEN_COPY_ARTICLE_WORDS: &[&str] = &["a", "an", "the"];
const TOKEN_COPY_EXILE_ZONE_WORDS: &[&str] = &["exile"];
const TOKEN_COPY_EXILE_ZONE_PHRASES: &[&[&str]] = &[&["exile"]];
const TOKEN_COPY_BATTLEFIELD_ZONE_WORDS: &[&str] = &["battlefield"];
const TOKEN_COPY_BATTLEFIELD_PREPOSITION_WORDS: &[&str] = &["into", "onto", "to"];
const RETURN_OR_RETURNS_WORDS: &[&str] = &["return", "returns"];
const PUT_OR_PUTS_WORDS: &[&str] = &["put", "puts"];
const TOKEN_COPY_ON_TOP_PHRASES: &[&[&str]] = &[
    &["on", "top"],
    &["on", "the", "top"],
    &["third", "from", "top"],
    &["third", "from", "the", "top"],
];
const TOKEN_COPY_HAND_LOCATION_PHRASES: &[&[&str]] = &[&["hand"], &["hands"]];
const TOKEN_COPY_HAND_LOCATION_WORDS: &[&str] = &["hand", "hands"];
const TOKEN_COPY_LIBRARY_LOCATION_PHRASES: &[&[&str]] = &[&["library"], &["libraries"]];
const TOKEN_COPY_GRAVEYARD_LOCATION_PHRASES: &[&[&str]] = &[&["graveyard"], &["graveyards"]];
fn token_copy_destroy_attached_supported_target(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    TOKEN_COPY_TARGET_HEAD_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("target_head"))
        .is_some_and(|range| range.start == 0)
        || clause
            .match_pattern(TOKEN_COPY_EXACT_TARGET_REFERENCE_PATTERN)
            .and_then(|matched| matched.capture_word_range("reference"))
            .is_some()
        || TOKEN_COPY_THAT_TARGET_REFERENCE_PATTERN
            .find_in_word_refs(&words)
            .and_then(|matched| matched.capture_word_range("reference"))
            .is_some_and(|range| range.start == 0)
}

fn token_copy_destroy_attached_has_timing_tail(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    TOKEN_COPY_TIMING_TAIL_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("timing"))
        .is_some()
}

fn token_copy_each_player_reveal_count_clause(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<SubjectVerbPrimitiveClause<'_>> {
    let reveal_words = clause.word_refs();
    if !word_slice_starts_with(&reveal_words, EACH_PLAYER_REVEALS_TOP_PREFIX) {
        return None;
    }
    let count_clause = clause.after_words(EACH_PLAYER_REVEALS_TOP_PREFIX.len())?;
    if count_clause.is_empty() {
        return None;
    }
    Some(count_clause.trimmed())
}

fn token_copy_each_player_puts_revealed_permanents_matches(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> bool {
    clause
        .match_pattern(EACH_PLAYER_PUTS_REVEALED_PERMANENTS_PATTERN)
        .and_then(|matched| matched.capture_word_range("destination_prefix"))
        .is_some()
}

fn token_copy_exile_head_leading_words(head_words: &[&str]) -> &'static [&'static str] {
    if word_slice_starts_with(head_words, YOU_EXILE_PREFIX) {
        TOKEN_COPY_YOU_EXILE_LEADING_WORDS
    } else {
        TOKEN_COPY_EXILE_LEADING_WORDS
    }
}

fn token_copy_action_starts_clause(words: &[&str], pattern: LexPattern<'static>) -> bool {
    pattern
        .find_in_word_refs(words)
        .and_then(|matched| matched.capture_word_range("action"))
        .is_some_and(|range| range.start == 0)
}

const RETURN_THIS_OWNER_HAND_TAIL_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action("action", LexCaptureKind::OneOf(RETURN_OR_RETURNS_WORDS)),
    LexPattern::word("this"),
    LexPattern::object("source", LexCaptureKind::UntilPhrase(TOKEN_COPY_TO_PHRASE)),
    LexPattern::word("to"),
    LexPattern::modifier(
        "owner",
        LexCaptureKind::UntilAnyPhrase(TOKEN_COPY_HAND_LOCATION_PHRASES),
    ),
    LexPattern::object(
        "zone",
        LexCaptureKind::OneOf(TOKEN_COPY_HAND_LOCATION_WORDS),
    ),
]);
const PUT_THIS_OWNER_TOP_LIBRARY_TAIL_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action("action", LexCaptureKind::OneOf(PUT_OR_PUTS_WORDS)),
    LexPattern::word("this"),
    LexPattern::object(
        "source",
        LexCaptureKind::UntilAnyPhrase(TOKEN_COPY_ON_TOP_PHRASES),
    ),
    LexPattern::any_phrase(TOKEN_COPY_ON_TOP_PHRASES),
    LexPattern::optional(&[LexPattern::word("of")]),
    LexPattern::modifier(
        "owner",
        LexCaptureKind::UntilAnyPhrase(TOKEN_COPY_LIBRARY_LOCATION_PHRASES),
    ),
    LexPattern::object("zone", LexCaptureKind::OneOf(LIBRARY_OR_LIBRARIES_WORDS)),
]);
const SHUFFLE_GRAVEYARD_INTO_LIBRARY_TAIL_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action("action", LexCaptureKind::OneOf(SHUFFLE_OR_SHUFFLES_WORDS)),
    LexPattern::modifier(
        "source_owner",
        LexCaptureKind::UntilAnyPhrase(TOKEN_COPY_GRAVEYARD_LOCATION_PHRASES),
    ),
    LexPattern::object(
        "source_zone",
        LexCaptureKind::OneOf(GRAVEYARD_OR_GRAVEYARDS_WORDS),
    ),
    LexPattern::modifier(
        "before_destination",
        LexCaptureKind::UntilPhrase(TOKEN_COPY_INTO_PHRASE),
    ),
    LexPattern::word("into"),
    LexPattern::modifier(
        "destination_owner",
        LexCaptureKind::UntilAnyPhrase(TOKEN_COPY_LIBRARY_LOCATION_PHRASES),
    ),
    LexPattern::object(
        "destination_zone",
        LexCaptureKind::OneOf(LIBRARY_OR_LIBRARIES_WORDS),
    ),
]);
const TOKEN_COPY_FROM_EXILE_SOURCE_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("from"),
    LexPattern::modifier(
        "owner",
        LexCaptureKind::UntilAnyPhrase(TOKEN_COPY_EXILE_ZONE_PHRASES),
    ),
    LexPattern::object("zone", LexCaptureKind::OneOf(TOKEN_COPY_EXILE_ZONE_WORDS)),
]);
const TOKEN_COPY_BATTLEFIELD_DESTINATION_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::modifier(
        "preposition",
        LexCaptureKind::OneOf(TOKEN_COPY_BATTLEFIELD_PREPOSITION_WORDS),
    ),
    LexPattern::optional(&[LexPattern::any_word(TOKEN_COPY_ARTICLE_WORDS)]),
    LexPattern::object(
        "zone",
        LexCaptureKind::OneOf(TOKEN_COPY_BATTLEFIELD_ZONE_WORDS),
    ),
]);
const CHOOSE_CARD_NAME_TAIL_PREFIXES: &[&[&str]] = &[
    &["choose", "any", "card", "name"],
    &["choose", "a", "card", "name"],
];
const CHOOSE_CARD_NAME_TAIL_PREFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "choice",
        LexCaptureKind::OneOfPhrase(CHOOSE_CARD_NAME_TAIL_PREFIXES),
    )]);
const LOOK_WORDS: &[&str] = &["look"];
const DRAW_WORDS: &[&str] = &["draw"];
const TAP_OR_UNTAP_WORDS: &[&str] = &["tap", "untap"];
const LOOK_ACTION_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "action",
    LexCaptureKind::OneOf(LOOK_WORDS),
)]);
const DRAW_ACTION_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "action",
    LexCaptureKind::OneOf(DRAW_WORDS),
)]);
const TAP_OR_UNTAP_ACTION_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "action",
    LexCaptureKind::OneOf(TAP_OR_UNTAP_WORDS),
)]);
const RETURN_ACTION_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "action",
    LexCaptureKind::OneOf(RETURN_WORDS),
)]);
const CHOOSE_ACTION_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "action",
    LexCaptureKind::OneOf(CHOOSE_WORDS),
)]);
const SACRIFICE_ACTION_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "action",
    LexCaptureKind::OneOf(SACRIFICE_WORDS),
)]);

fn token_copy_tail_starts_with_that_player(words: &[&str]) -> bool {
    TOKEN_COPY_THAT_PLAYER_TAIL_PATTERN
        .find_in_word_refs(words)
        .and_then(|matched| matched.capture_word_range("player"))
        .is_some_and(|range| range.start == 0)
}

fn token_copy_tail_returns_this_to_owner_hand(words: &[&str]) -> bool {
    RETURN_THIS_OWNER_HAND_TAIL_PATTERN
        .match_word_refs(words)
        .is_some()
}

fn token_copy_tail_puts_this_on_top_of_owner_library(words: &[&str]) -> bool {
    PUT_THIS_OWNER_TOP_LIBRARY_TAIL_PATTERN
        .match_word_refs(words)
        .is_some()
}

fn token_copy_tail_puts_counted_from_exile_onto_battlefield(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> bool {
    let Some((_count, used)) = parse_choice_count_token_prefix_consumed(clause.tokens()) else {
        return false;
    };
    let tail_tokens = &clause.tokens()[used..];
    TOKEN_COPY_FROM_EXILE_SOURCE_PATTERN
        .find_in_clause(LexedClause::new(tail_tokens))
        .is_some()
        && TOKEN_COPY_BATTLEFIELD_DESTINATION_PATTERN
            .find_in_clause(LexedClause::new(tail_tokens))
            .is_some()
}

fn token_copy_tail_starts_with_choose_card_name(words: &[&str]) -> bool {
    CHOOSE_CARD_NAME_TAIL_PREFIX_PATTERN
        .find_in_word_refs(words)
        .and_then(|matched| matched.capture_word_range("choice"))
        .is_some_and(|range| range.start == 0)
}

fn tokens_with_leading_words(words: &[&str], tail: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut tokens = Vec::with_capacity(words.len() + tail.len());
    tokens.extend(
        words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic())),
    );
    tokens.extend_from_slice(tail);
    tokens
}

fn sacrifice_choice_filter(mut filter: ObjectFilter) -> ObjectFilter {
    if filter.controller.is_none() {
        filter.controller = Some(PlayerFilter::You);
    }
    filter
}

pub(crate) const DESTROY_ALL_ATTACHED_TO_TARGET_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("destroy"),
    LexPattern::any_word(ALL_OR_EACH_WORDS),
    LexPattern::role_capture(
        "filter",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["attached", "to"]),
    ),
    LexPattern::phrase(&["attached", "to"]),
    LexPattern::role_capture(
        "target",
        LexCaptureRole::Tail,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const RETURN_THEN_CREATE_HEAD_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::word("return"),
    LexPattern::capture("return_tail", LexCaptureKind::UntilPhrase(&["then"])),
];
const EXILE_THEN_MAY_PUT_HEAD_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::word("exile"),
    LexPattern::capture("exile_tail", LexCaptureKind::UntilPhrase(&["then"])),
];
const YOU_EXILE_THEN_MAY_PUT_HEAD_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["you", "exile"]),
    LexPattern::capture("exile_tail", LexCaptureKind::UntilPhrase(&["then"])),
];
const EXILE_THEN_MAY_PUT_HEAD_SEQUENCES: &[&[LexPatternAtom<'static>]] = &[
    EXILE_THEN_MAY_PUT_HEAD_SEQUENCE,
    YOU_EXILE_THEN_MAY_PUT_HEAD_SEQUENCE,
];
pub(crate) const RETURN_THEN_CREATE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_sequence(&[RETURN_THEN_CREATE_HEAD_SEQUENCE]),
    LexPattern::word("then"),
    LexPattern::word("create"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
pub(crate) const EXILE_THEN_MAY_PUT_FROM_EXILE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_sequence(EXILE_THEN_MAY_PUT_HEAD_SEQUENCES),
    LexPattern::word("then"),
    LexPattern::phrase(&["you", "may", "put"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
pub(crate) const THEN_CHAIN_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "head",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilPhrase(&["then"]),
    ),
    LexPattern::word("then"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
const OPTIONAL_OF_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[LexPattern::word("of")];
const SACRIFICE_ANY_NUMBER_THEN_TAIL_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["then"]),
    ),
    LexPattern::word("then"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
const SACRIFICE_ANY_NUMBER_OBJECT_SEQUENCE: &[LexPatternAtom<'static>] =
    &[LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::OneOrMoreWords,
    )];
const SACRIFICE_ANY_NUMBER_SEQUENCES: &[&[LexPatternAtom<'static>]] = &[
    SACRIFICE_ANY_NUMBER_THEN_TAIL_SEQUENCE,
    SACRIFICE_ANY_NUMBER_OBJECT_SEQUENCE,
];
pub(crate) const SACRIFICE_ANY_NUMBER_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("sacrifice"),
    LexPattern::phrase(&["any", "number"]),
    LexPattern::optional(OPTIONAL_OF_PATTERN_ATOMS),
    LexPattern::any_sequence(SACRIFICE_ANY_NUMBER_SEQUENCES),
];
pub(crate) const SACRIFICE_ONE_OR_MORE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("sacrifice"),
    LexPattern::capture("minimum", LexCaptureKind::WordCount(3)),
    LexPattern::optional(OPTIONAL_OF_PATTERN_ATOMS),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::OneOrMoreWords,
    ),
];
pub(crate) const CHOOSE_THEN_DO_SAME_FOR_FILTER_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "head",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilPhrase(&["then", "do", "the", "same", "for"]),
    ),
    LexPattern::phrase(&["then", "do", "the", "same", "for"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
pub(crate) const CHOOSE_THEN_CHOOSE_OBJECTS_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "head",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilPhrase(&["then"]),
    ),
    LexPattern::word("then"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
pub(crate) const RETURN_THEN_DO_SAME_FOR_SUBTYPES_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("return"),
    LexPattern::role_capture(
        "head",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilPhrase(&["do", "the", "same", "for"]),
    ),
    LexPattern::phrase(&["do", "the", "same", "for"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
pub(crate) const EACH_PLAYER_REVEALS_TOP_PUT_PERMANENTS_PATTERN_ATOMS: &[LexPatternAtom<
    'static,
>] = &[
    LexPattern::phrase(&["each", "player", "reveals"]),
    LexPattern::role_capture(
        "reveal_count",
        LexCaptureRole::Amount,
        LexCaptureKind::UntilPhrase(&["puts", "all", "permanent", "cards"]),
    ),
    LexPattern::phrase(&["puts", "all", "permanent", "cards"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
pub(crate) const EXILE_SOURCE_WITH_COUNTERS_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("exile"),
    LexPattern::role_capture(
        "source",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["with"]),
    ),
    LexPattern::word("with"),
    LexPattern::role_capture("counter", LexCaptureRole::Modifier, LexCaptureKind::Rest),
];
pub(crate) const DESTROY_THEN_LAND_GRAVEYARD_DAMAGE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("destroy"),
    LexPattern::role_capture(
        "destroy_clause",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["then"]),
    ),
    LexPattern::word("then"),
    LexPattern::role_capture("damage_clause", LexCaptureRole::Tail, LexCaptureKind::Rest),
];

pub(crate) fn parse_sentence_each_player_return_with_additional_counter(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_return_with_additional_counter_sentence(clause)
}

pub(crate) fn parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = clause.trimmed_comma_segments();
    if segments.len() != 3 {
        return Ok(None);
    }

    let reveal_clause = segments[0];
    let Some(count_clause) = token_copy_each_player_reveal_count_clause(reveal_clause) else {
        return Ok(None);
    };
    let synthetic_where_clause = where_x_is_prefixed_clause(count_clause);
    let Some(count) = parse_value_binding_clause(synthetic_where_clause.tokens()) else {
        return Ok(None);
    };

    let put_clause = segments[1];
    if !token_copy_each_player_puts_revealed_permanents_matches(put_clause) {
        return Ok(None);
    }

    let rest_clause = segments[2].without_leading_connectors_clause();
    if !word_slice_eq(&rest_clause.word_refs(), EACH_PLAYER_PUTS_REST_GRAVEYARD) {
        return Ok(None);
    }

    let revealed_tag_key = helper_tag_for_tokens(clause.tokens(), "revealed");
    let iterated_target = TargetAst::Tagged(TagKey::from(IT_TAG), clause.span());

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::That,
                count,
                revealed_tag_key.clone(),
            ),
            EffectAst::subject_verb_reveal_tagged(revealed_tag_key.clone()),
            EffectAst::ForEachTagged {
                tag: revealed_tag_key,
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::ItMatches(ObjectFilter::permanent_card()),
                    if_true: vec![EffectAst::subject_verb_move_to_zone(
                        iterated_target.clone(),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Owner,
                        false,
                        None,
                    )],
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        iterated_target,
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                }],
            },
        ],
    }]))
}

pub(crate) fn parse_return_then_do_same_for_subtypes_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(RETURN_THEN_DO_SAME_FOR_SUBTYPES_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_return_then_do_same_for_subtypes_sentence_matched(clause, &matched)
}

pub(crate) fn parse_return_then_do_same_for_subtypes_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = clause.word_refs();
    if !token_copy_action_starts_clause(&clause_words, RETURN_ACTION_PATTERN) {
        return Ok(None);
    }
    let Some(head_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Subject) else {
        return Ok(None);
    };
    let Some(tail_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail) else {
        return Ok(None);
    };
    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let tail_words = tail_clause.word_refs();
    let subtype_words = tail_words.as_slice();
    if subtype_words.is_empty() {
        return Ok(None);
    }

    let mut extra_subtypes = Vec::new();
    for word in subtype_words {
        if TOKEN_COPY_AND_OR_WORDS.contains(word) {
            continue;
        }
        let Some(subtype) = parse_pluralized_subtype_word(word) else {
            return Ok(None);
        };
        extra_subtypes.push(subtype);
    }
    if extra_subtypes.is_empty() {
        return Ok(None);
    }

    let head_tail_tokens = trim_commas(head_clause.tokens());
    let return_tokens = tokens_with_leading_words(&[RETURN_WORD], &head_tail_tokens);
    let mut effects = parse_effect_chain(&return_tokens)?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let base_effect = effects[0].clone();
    for subtype in extra_subtypes {
        let Some(cloned) = clone_return_effect_with_subtype(&base_effect, subtype) else {
            return Ok(None);
        };
        effects.push(cloned);
    }

    Ok(Some(effects))
}

fn split_choose_same_followup_filters(filter: &ObjectFilter) -> Vec<ObjectFilter> {
    match filter.mana_value.clone() {
        Some(crate::filter::Comparison::OneOf(values)) if !values.is_empty() => values
            .into_iter()
            .map(|value| {
                let mut cloned = filter.clone();
                cloned.mana_value = Some(crate::filter::Comparison::Equal(value));
                cloned
            })
            .collect(),
        _ => vec![filter.clone()],
    }
}

pub(crate) fn parse_choose_then_do_same_for_filter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(CHOOSE_THEN_DO_SAME_FOR_FILTER_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_choose_then_do_same_for_filter_sentence_matched(clause, &matched)
}

pub(crate) fn parse_choose_then_do_same_for_filter_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = clause.word_refs();
    if !token_copy_action_starts_clause(&clause_words, CHOOSE_ACTION_PATTERN) {
        return Ok(None);
    }
    let Some(head_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Subject) else {
        return Ok(None);
    };
    let Some(followup_filter_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail)
    else {
        return Ok(None);
    };

    if head_clause.is_empty() || followup_filter_clause.is_empty() {
        return Ok(None);
    }

    let Some((player, base_filter, count)) = parse_you_choose_objects_clause(head_clause.tokens())?
        .or_else(|| {
            parse_target_player_choose_objects_clause(head_clause.tokens())
                .ok()
                .flatten()
        })
    else {
        return Ok(None);
    };
    let tag = TagKey::from(IT_TAG);

    let followup_filter = parse_object_filter(followup_filter_clause.tokens(), false)?;
    if followup_filter.controller.is_some() || followup_filter.owner.is_some() {
        return Ok(None);
    }

    let merged_filter = merge_filters(&base_filter, &followup_filter);
    let followup_filters = split_choose_same_followup_filters(&merged_filter);
    if followup_filters.is_empty() {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::ChooseObjects {
        filter: base_filter.clone(),
        count: count.clone(),
        count_value: None,
        player: player.clone(),
        tag: tag.clone(),
    }];
    for filter in followup_filters {
        effects.push(EffectAst::ChooseObjects {
            filter,
            count: count.clone(),
            count_value: None,
            player: player.clone(),
            tag: tag.clone(),
        });
    }

    Ok(Some(effects))
}

fn parse_choose_objects_clause_for_chain(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    if let Some(parsed) = clause.parse_value_with_lexed(parse_you_choose_objects_clause)? {
        return Ok(Some(parsed));
    }
    clause.parse_value_with_lexed(parse_target_player_choose_objects_clause)
}

fn choose_clause_trails_from_it(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    matches!(
        words.as_slice(),
        [.., "from", "it"] | [.., "from", "them"] | [.., "in", "it"] | [.., "in", "them"]
    )
}

fn preserve_choose_clause_it_reference(
    clause: SubjectVerbPrimitiveClause<'_>,
    filter: &mut ObjectFilter,
) {
    if !choose_clause_trails_from_it(clause) {
        return;
    }
    if filter.zone.is_none() || filter.zone == Some(Zone::Battlefield) {
        filter.zone = Some(Zone::Hand);
    }
    filter.controller = None;
    filter.owner = None;
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
}

pub(crate) fn parse_choose_then_choose_objects_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(CHOOSE_THEN_CHOOSE_OBJECTS_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_choose_then_choose_objects_sentence_matched(clause, &matched)
}

pub(crate) fn parse_choose_then_choose_objects_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(head_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Subject) else {
        return Ok(None);
    };
    let Some(tail_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail) else {
        return Ok(None);
    };

    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let Some((first_player, mut first_filter, first_count)) =
        parse_choose_objects_clause_for_chain(head_clause)?
    else {
        return Ok(None);
    };
    let Some((mut second_player, mut second_filter, second_count)) =
        parse_choose_objects_clause_for_chain(tail_clause)?
    else {
        return Ok(None);
    };

    preserve_choose_clause_it_reference(head_clause, &mut first_filter);
    preserve_choose_clause_it_reference(tail_clause, &mut second_filter);

    if second_player == PlayerAst::Implicit {
        second_player = first_player.clone();
    }

    let tag = TagKey::from(IT_TAG);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: first_filter,
            count: first_count,
            count_value: None,
            player: first_player,
            tag: tag.clone(),
        },
        EffectAst::ChooseObjects {
            filter: second_filter,
            count: second_count,
            count_value: None,
            player: second_player,
            tag,
        },
    ]))
}

pub(crate) fn parse_sentence_return_then_do_same_for_subtypes(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_then_do_same_for_subtypes_sentence(clause)
}

pub(crate) fn parse_sentence_choose_then_choose_objects(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_choose_then_choose_objects_sentence(clause)
}

pub(crate) fn parse_sentence_choose_then_do_same_for_filter(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_choose_then_do_same_for_filter_sentence(clause)
}

pub(crate) fn parse_sacrifice_any_number_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(SACRIFICE_ANY_NUMBER_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sacrifice_any_number_sentence_matched(clause, &matched)
}

pub(crate) fn parse_sacrifice_any_number_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(filter_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Object) else {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            clause.text()
        )));
    };

    let filter_clause = filter_clause.trimmed();
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            clause.text()
        )));
    }

    let parsed_filter =
        if let Some(filter) = parse_artifact_enchantment_or_token_filter(filter_clause.tokens()) {
            filter
        } else {
            parse_object_filter(filter_clause.tokens(), false)?
        };
    let filter = sacrifice_choice_filter(parsed_filter);
    let tag = TagKey::from(IT_TAG);

    let mut effects = vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::any_number(),
            count_value: None,
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_sacrifice_all(PlayerAst::Implicit, ObjectFilter::tagged(tag)),
    ];
    if let Some(tail_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail)
        && !tail_clause.is_empty()
    {
        let mut tail_effects = parse_effect_chain(tail_clause.tokens())?;
        effects.append(&mut tail_effects);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_sacrifice_any_number(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_any_number_sentence(clause)
}

pub(crate) fn parse_sacrifice_one_or_more_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(SACRIFICE_ONE_OR_MORE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sacrifice_one_or_more_sentence_matched(clause, &matched)
}

pub(crate) fn parse_sacrifice_one_or_more_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = clause.word_refs();
    if !token_copy_action_starts_clause(&clause_words, SACRIFICE_ACTION_PATTERN) {
        return Ok(None);
    }

    let Some(minimum_clause) = clause.pattern_capture(matched, "minimum") else {
        return Ok(None);
    };
    let Ok(Some((minimum, used))) =
        crate::runtime_backend::util::parse_greater_than_or_equal_quantity_prefix(
            minimum_clause.tokens(),
            false,
            false,
            "sacrifice count",
        )
    else {
        return Ok(None);
    };
    if used != minimum_clause.len() {
        return Ok(None);
    }
    let Some(filter_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Object) else {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            clause.text()
        )));
    };
    let filter_clause = filter_clause.trimmed();
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            clause.text()
        )));
    }
    let parsed_filter =
        if let Some(filter) = parse_artifact_enchantment_or_token_filter(filter_clause.tokens()) {
            filter
        } else {
            parse_object_filter(filter_clause.tokens(), false)?
        };
    let filter = sacrifice_choice_filter(parsed_filter);
    let tag = TagKey::from(IT_TAG);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::at_least(minimum as usize),
            count_value: None,
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_sacrifice_all(PlayerAst::Implicit, ObjectFilter::tagged(tag)),
    ]))
}

pub(crate) fn parse_sentence_sacrifice_one_or_more(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_one_or_more_sentence(clause)
}

pub(crate) fn parse_sentence_keyword_then_chain(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(THEN_CHAIN_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_keyword_then_chain_matched(clause, &matched)
}

pub(crate) fn parse_sentence_keyword_then_chain_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(head_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Subject) else {
        return Ok(None);
    };
    let Some(tail_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail) else {
        return Ok(None);
    };

    let Some(head_effect) = parse_keyword_mechanic_clause(head_clause.tokens())? else {
        return Ok(None);
    };

    if tail_clause.is_empty() {
        return Ok(Some(vec![head_effect]));
    }

    let mut effects = vec![head_effect];
    if let Some(mut counter_effects) = parse_sentence_put_counter_sequence(tail_clause)? {
        effects.append(&mut counter_effects);
        return Ok(Some(effects));
    }

    let mut tail_effects = parse_effect_chain(tail_clause.tokens())?;
    effects.append(&mut tail_effects);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_chain_then_keyword(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(THEN_CHAIN_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_chain_then_keyword_matched(clause, &matched)
}

pub(crate) fn parse_sentence_chain_then_keyword_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(head_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Subject) else {
        return Ok(None);
    };
    let Some(tail_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail) else {
        return Ok(None);
    };
    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let Some(keyword_effect) = parse_keyword_mechanic_clause(tail_clause.tokens())? else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(head_clause.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    head_effects.push(keyword_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_return_then_create(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(RETURN_THEN_CREATE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_return_then_create_matched(clause, &matched)
}

pub(crate) fn parse_sentence_return_then_create_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(return_tail_clause) = clause.pattern_capture(matched, "return_tail") else {
        return Ok(None);
    };
    let Some(create_tail_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail)
    else {
        return Ok(None);
    };
    if return_tail_clause.is_empty() || create_tail_clause.is_empty() {
        return Ok(None);
    }

    let return_tokens =
        tokens_with_leading_words(&[RETURN_WORD], &trim_commas(return_tail_clause.tokens()));
    let create_tokens =
        tokens_with_leading_words(&[CREATE_WORD], &trim_commas(create_tail_clause.tokens()));
    let mut head_effects = parse_effect_chain(&return_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(&create_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_exile_then_may_put_from_exile(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(EXILE_THEN_MAY_PUT_FROM_EXILE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_exile_then_may_put_from_exile_matched(clause, &matched)
}

pub(crate) fn parse_sentence_exile_then_may_put_from_exile_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(exile_tail_clause) = clause.pattern_capture(matched, "exile_tail") else {
        return Ok(None);
    };
    let Some(put_tail_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail) else {
        return Ok(None);
    };
    if exile_tail_clause.is_empty() || put_tail_clause.is_empty() {
        return Ok(None);
    }

    if !token_copy_tail_puts_counted_from_exile_onto_battlefield(put_tail_clause) {
        return Ok(None);
    }

    let clause_words = clause.word_refs();
    let exile_leading_words = token_copy_exile_head_leading_words(&clause_words);
    let exile_tokens = tokens_with_leading_words(
        exile_leading_words,
        &trim_commas(exile_tail_clause.tokens()),
    );
    let put_tokens = tokens_with_leading_words(
        &["you", "may", "put"],
        &trim_commas(put_tail_clause.tokens()),
    );

    let mut head_effects = parse_effect_chain(&exile_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    let mut tail_effects = parse_effect_chain(&put_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_exile_then_shuffle_graveyard_into_library_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = clause.split_once_on_then_trimmed();
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    if head_slice.is_empty() || tail_slice.is_empty() {
        return Ok(None);
    }

    let head_words = head_slice.word_refs();
    if !word_slice_starts_with(&head_words, YOU_EXILE_PREFIX)
        && !word_slice_starts_with(&head_words, EXILE_HEAD_PREFIX)
    {
        return Ok(None);
    }

    let tail_words = tail_slice.word_refs();
    if SHUFFLE_GRAVEYARD_INTO_LIBRARY_TAIL_PATTERN
        .match_word_refs(&tail_words)
        .is_none()
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_slice.tokens())?;
    if !head_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Exile { .. },
                ..
            }) | EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileAll { .. },
                ..
            }) | EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileUntilSourceLeaves { .. },
                ..
            })
        )
    }) {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(tail_slice.tokens())?;
    if !tail_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ShuffleGraveyardIntoLibrary,
                ..
            })
        )
    }) {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_exile_source_with_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "exile <source> with <counter descriptor> on it/them"
    let pattern = LexPattern::new(EXILE_SOURCE_WITH_COUNTERS_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    let Some(source_name_clause) = clause
        .pattern_capture_role(&matched, LexCaptureRole::Object)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    let Some(counter_clause) = clause
        .pattern_capture_role(&matched, LexCaptureRole::Modifier)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };

    if source_name_clause.is_empty() {
        return Ok(None);
    }
    let source_name_words = source_name_clause.word_refs();
    if !is_likely_named_or_source_reference_words(&source_name_words) {
        return Ok(None);
    }
    let Some(on_idx) = counter_clause.rfind_token_word("on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause.len() {
        return Ok(None);
    }

    let on_target_clause = counter_clause.from(on_idx + 1);
    if !word_slice_eq_any(&on_target_clause.word_refs(), IT_OR_THEM_PHRASES) {
        return Ok(None);
    }

    let descriptor_clause = counter_clause.before(on_idx).trimmed();
    if descriptor_clause.is_empty() {
        return Ok(None);
    }
    let (count, counter_type) = parse_counter_descriptor(descriptor_clause.tokens())?;

    let source_target = TargetAst::Source(clause.span());
    Ok(Some(vec![
        EffectAst::subject_verb_exile(source_target.clone(), false),
        EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            source_target,
            None,
            false,
        ),
    ]))
}

pub(crate) fn parse_sentence_exile_source_with_counters(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_source_with_counters_sentence(clause)
}

pub(crate) fn parse_sentence_comma_then_chain_special(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        crate::runtime_backend::util::possessive_normalized_word_refs(words)
    }

    let Some((head_clause, tail_clause)) = clause.split_comma_then_trimmed() else {
        return Ok(None);
    };
    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let head_word_storage = head_clause.word_refs();
    let tail_word_storage = tail_clause.word_refs();
    let head_words = normalize_words(&head_word_storage);
    let tail_words = normalize_words(&tail_word_storage);
    let is_that_player_tail = token_copy_tail_starts_with_that_player(&tail_words);
    let is_return_source_tail = token_copy_tail_returns_this_to_owner_hand(&tail_words);
    let is_put_source_on_top_of_library_tail =
        token_copy_tail_puts_this_on_top_of_owner_library(&tail_words);
    let is_choose_card_name_tail = token_copy_tail_starts_with_choose_card_name(&tail_words)
        && token_copy_action_starts_clause(&head_words, LOOK_ACTION_PATTERN);
    if !is_that_player_tail
        && !is_return_source_tail
        && !is_put_source_on_top_of_library_tail
        && !is_choose_card_name_tail
    {
        return Ok(None);
    }
    if is_return_source_tail
        && !token_copy_action_starts_clause(&head_words, TAP_OR_UNTAP_ACTION_PATTERN)
    {
        return Ok(None);
    }
    if is_put_source_on_top_of_library_tail
        && !token_copy_action_starts_clause(&head_words, DRAW_ACTION_PATTERN)
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_clause.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(tail_clause.tokens())?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    if is_that_player_tail && head_is_single_return_to_hand(&head_effects) {
        bind_that_player_tail_to_returned_owner(&mut tail_effects);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

fn head_is_single_return_to_hand(effects: &[EffectAst]) -> bool {
    let [effect] = effects else {
        return false;
    };

    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnToHand { .. },
            ..
        })
    )
}

fn bind_that_player_tail_to_returned_owner(effects: &mut [EffectAst]) {
    for effect in effects {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && subject_verb.subject.player == PlayerAst::That
        {
            subject_verb.subject.player = PlayerAst::ItsOwner;
        }
    }
}

pub(crate) fn parse_destroy_then_land_controller_graveyard_count_damage_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((head_clause, tail_clause)) = clause.split_comma_then_trimmed() else {
        return Ok(None);
    };
    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    let tail_words = tail_clause.word_refs();
    let suffix = [
        "damage",
        "to",
        "that",
        "lands",
        "controller",
        "equal",
        "to",
        "the",
        "number",
        "of",
        "land",
        "cards",
        "in",
        "that",
        "players",
        "graveyard",
    ];
    let Some(suffix_start) = tail_clause.find_phrase_start(&suffix) else {
        return Ok(None);
    };
    if suffix_start == 0 || !TOKEN_COPY_DEAL_OR_DEALS_WORDS.contains(&tail_words[suffix_start - 1])
    {
        return Ok(None);
    }
    if suffix_start + suffix.len() != tail_words.len() {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_clause.tokens())?;
    if !head_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Destroy { .. },
                ..
            })
        )
    }) {
        return Ok(None);
    }

    let mut count_filter = ObjectFilter::default();
    count_filter.zone = Some(Zone::Graveyard);
    let tagged_ref = crate::target::ObjectRef::tagged(IT_TAG);
    count_filter.owner = Some(PlayerFilter::ControllerOf(tagged_ref.clone()));
    count_filter.card_types.push(CardType::Land);
    head_effects.push(EffectAst::subject_verb_damage(
        Value::Count(count_filter),
        TargetAst::Player(PlayerFilter::ControllerOf(tagged_ref), tail_clause.span()),
    ));
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_destroy_all_attached_to_target(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "destroy all/each <filter> attached to <target>"
    let pattern = LexPattern::new(DESTROY_ALL_ATTACHED_TO_TARGET_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_destroy_all_attached_to_target_matched(clause, &matched)
}

pub(crate) fn parse_sentence_destroy_all_attached_to_target_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(filter_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Object)
        .map(|clause| clause.without_trailing_words_clause(&["that", "were", "was", "is", "are"]))
    else {
        return Ok(None);
    };
    let Some(target_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Tail)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    let has_timing_tail = token_copy_destroy_attached_has_timing_tail(target_clause);
    let supported_target = token_copy_destroy_attached_supported_target(target_clause);
    if filter_clause.is_empty() || target_clause.is_empty() || !supported_target || has_timing_tail
    {
        return Ok(None);
    }

    let filter = parse_object_filter(filter_clause.tokens(), false)?;
    let target = parse_target_phrase(target_clause.tokens())?;
    Ok(Some(vec![EffectAst::subject_verb_destroy_all_attached_to(
        filter, target,
    )]))
}

pub(crate) fn parse_sentence_destroy_then_land_controller_graveyard_count_damage(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_then_land_controller_graveyard_count_damage_sentence(clause)
}

pub(crate) fn find_creature_type_choice_phrase(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize)> {
    const CREATURE_TYPE_CHOICE_PHRASES: &[&[&str]] = &[
        &["of", "the", "creature", "type", "of", "your", "choice"],
        &["of", "creature", "type", "of", "your", "choice"],
    ];
    clause.find_any_phrase_span(CREATURE_TYPE_CHOICE_PHRASES)
}

pub(super) fn find_type_choice_phrase(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize)> {
    const TYPE_CHOICE_PHRASES: &[&[&str]] = &[
        &["of", "the", "chosen", "type"],
        &["of", "chosen", "type"],
        &["of", "that", "type"],
        &["that", "type"],
    ];
    find_creature_type_choice_phrase(clause)
        .or_else(|| clause.find_any_phrase_span(TYPE_CHOICE_PHRASES))
}

pub(crate) fn find_color_choice_phrase(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize)> {
    const COLOR_CHOICE_PHRASES: &[&[&str]] = &[
        &["of", "the", "color", "of", "your", "choice"],
        &["of", "the", "color", "of", "their", "choice"],
        &["of", "color", "of", "your", "choice"],
        &["of", "color", "of", "their", "choice"],
    ];
    clause.find_any_phrase_span(COLOR_CHOICE_PHRASES)
}

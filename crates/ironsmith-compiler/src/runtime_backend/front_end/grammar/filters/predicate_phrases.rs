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
const THERE_IS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["there", "is"]);
const OR_IF_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or", "if"]);
const AND_YOUR_LIFE_TOTAL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["and", "your", "life", "total"]);
const COLOR_OR_COLORS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["color"], &["colors"]]);
const AMONG_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["among"]);
const TYPE_OR_TYPES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["type"], &["types"]]);
const SACRIFICED_OR_SACRIFICED_TAG_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["sacrificed"], &["sacrificed_0"]]);
const PERMANENT_OR_PERMANENTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["permanent"], &["permanents"]]);
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
const IN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["in"]);
const INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const GRAVEYARD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["graveyard"]);
const MORE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["more"]);
const OTHER_OR_ANOTHER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["another"], &["other"]]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const CHOSEN_NAME_TAG: &str = "__chosen_name__";
const PUT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["put"]);
const YOU_REVEALED_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "revealed"]);
const BEHOLD_CAST_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["as", "you", "cast", "this", "spell"]);
const CONTROL_OR_CONTROLLED_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["control"], &["controlled"]]);
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
const COST_WAS_PAID_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cost", "was", "paid"]);
const COST_WASNT_PAID_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cost", "wasnt", "paid"]);
const COST_WAS_NOT_PAID_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cost", "was", "not", "paid"]);
const ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["a"], &["an"]]);
const DEFINITE_ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const WAS_OR_WERE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["was"], &["were"]]);
const WAS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["was"]);
const BEHELD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["beheld"]);
const THIS_POSSESSIVE_PAID_LABEL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this"]; suffix & ["cost", "was", "paid"]);
const THIS_POSSESSIVE_PAID_SUBJECT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["spell's"],
            &["spells"],
            &["card's"],
            &["cards"],
            &["creature's"],
            &["creatures"],
            &["permanent's"],
            &["permanents"],
        ]
);
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
const EQUIPPED_CREATURE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["equipped", "creature"]);
const ENCHANTED_CREATURE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["enchanted", "creature"]);
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
const WHILE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["while"]);
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
const THERE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["there"]);
const ONTO_BATTLEFIELD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["onto", "battlefield"], &["onto", "the", "battlefield"]]);
const MOST_COMMON_COLOR_AMONG_ALL_PERMANENTS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["most", "common", "color", "among", "all", "permanents"]);
const IS_OR_ARE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["is"], &["are"]]);
const BE_VERB_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["is"], &["are"], &["was"], &["were"]]);
const MANA_SYMBOL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["w"], &["u"], &["b"], &["r"], &["g"], &["c"], &["s"]]);
const SOURCE_FILTER_STATE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["is"],
            &["are"],
            &["isnt"],
            &["isn't"],
            &["arent"],
            &["aren't"],
        ]
);
const NEGATED_STATE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["isnt"], &["isn't"], &["arent"], &["aren't"]]);
const NOT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["not"]);
const SOURCE_FILTER_IGNORED_DESCRIPTOR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["attached"], &["tapped"], &["untapped"], &["saddled"]]);
const SOURCE_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["its"]]);
const ENCHANTED_BY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["enchanted", "by"]);
const AURA_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["aura"], &["auras"]]);
const CONTROL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["control"]);
const CONTROL_OR_CONTROLS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["control"], &["controls"]]);
const ZONE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["graveyard"], &["hand"], &["exile"], &["library"]]);
const OPPONENT_CONTROLS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["opponent", "controls"]);
const AN_OPPONENT_CONTROLS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["an", "opponent", "controls"]);
const ON_BATTLEFIELD_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["on", "the", "battlefield"], &["on", "battlefield"]]);
const ON_THE_BATTLEFIELD_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["on", "the", "battlefield"]);
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
const THAN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["than"]);
const THAN_YOU_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["than", "you"], &["than", "you", "do"]]);

fn source_zone_from_words(words: &[&str]) -> Option<Zone> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["is", "in"])),
        LexPattern::action("location", LexCaptureKind::WordCount(2)),
        LexPattern::modifier("zone", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject_words = subject.word_refs();
    if !is_source_reference_words(&subject_words) {
        return None;
    }
    let location = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(location.word_refs().as_slice(), ["is", "in"]) {
        return None;
    }
    let zone = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    source_zone_tail(&zone.word_refs())
}

fn source_zone_tail(words: &[&str]) -> Option<Zone> {
    match words {
        ["your", "hand"] => Some(Zone::Hand),
        ["your", "graveyard"] => Some(Zone::Graveyard),
        ["your", "library"] => Some(Zone::Library),
        ["exile"] => Some(Zone::Exile),
        ["the", "command", "zone"] => Some(Zone::Command),
        _ => None,
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
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["is"])),
        LexPattern::action("copula", LexCaptureKind::WordCount(1)),
        LexPattern::object("role", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject_words = subject.word_refs();
    if !is_source_reference_words(&subject_words) {
        return None;
    }
    let copula = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(copula.word_refs().as_slice(), ["is"]) {
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
    _words: &[&str],
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let window_phrases: &[&[&str]] = &[&["times", "this", "game"], &["time", "this", "game"]];
    let atoms = [
        LexPattern::subject("ring", LexCaptureKind::UntilPhrase(&["has", "tempted"])),
        LexPattern::action("action", LexCaptureKind::WordCount(3)),
        LexPattern::amount("count", LexCaptureKind::UntilAnyPhrase(window_phrases)),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let ring = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(ring.word_refs().as_slice(), ["ring"] | ["the", "ring"]) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["has", "tempted", "you"]) {
        return None;
    }
    let count_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let count_words = count_clause.word_refs();
    let (count, used) = parse_number(count_clause.tokens())?;
    if used + 2 != count_words.len() || !OR_MORE_PREFIX_PATTERN.matches_words(&count_words[used..])
    {
        return None;
    }
    let window = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !matches!(
        window.word_refs().as_slice(),
        ["times", "this", "game"] | ["time", "this", "game"]
    ) {
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
    if let Some(predicate) = parse_ring_has_tempted_you_this_game_predicate(words, tokens) {
        return Some(predicate);
    }

    let and_idx = find_index(words, |word| AND_WORD_PATTERN.matches_word(word))?;
    let left_words = &words[..and_idx];
    let right_words = &words[and_idx + 1..];
    if left_words.is_empty() || right_words.is_empty() {
        return None;
    }
    let left = parse_source_is_your_ring_bearer_predicate(left_words)?;
    let right =
        parse_ring_has_tempted_you_this_game_predicate(right_words, &tokens[and_idx + 1..])?;
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
    if !matches!(
        spell.word_refs().as_slice(),
        ["that", "spell"] | ["spell"] | ["it"]
    ) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["targets", "only"]) {
        return None;
    };

    let target = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let mut target_filter = match target.word_refs().as_slice() {
        ["this", "creature"] => ObjectFilter::creature(),
        ["this", "artifact"] => ObjectFilter::artifact(),
        ["this", "enchantment"] => ObjectFilter::enchantment(),
        ["this", "land"] => ObjectFilter::land(),
        ["this", "permanent"] => ObjectFilter::default().in_zone(Zone::Battlefield),
        ["this", "source"] | ["it"] => ObjectFilter::source(),
        _ => return None,
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

fn parse_negative_player_controls_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    if let Some(predicate) = parse_player_controls_no_shape(&tokens) {
        return predicate.map(Some);
    }
    if let Some(predicate) = parse_you_dont_control_shape(&tokens) {
        return predicate.map(Some);
    }
    Ok(None)
}

fn parse_player_controls_no_shape(
    tokens: &[OwnedLexToken],
) -> Option<Result<PredicateAst, CardTextError>> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject(
            "player",
            LexCaptureKind::UntilAnyPhrase(&[&["control"], &["controls"]]),
        ),
        LexPattern::action("control", LexCaptureKind::WordCount(1)),
        LexPattern::amount("negator", LexCaptureKind::WordCount(1)),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let player_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = match player_clause.word_refs().as_slice() {
        ["you"] => PlayerAst::You,
        ["player"] => PlayerAst::Any,
        _ => return None,
    };
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["control"] | ["controls"]) {
        return None;
    }
    let negator = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let tagged_relation = match negator.word_refs().as_slice() {
        ["no"] => false,
        ["neither"] if player == PlayerAst::You => true,
        _ => return None,
    };
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let object_words = object.word_refs();
    if object_words.is_empty() {
        return None;
    }
    let object_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&object_words);
    Some(
        parse_object_filter(&object_tokens, false).map(|mut filter| {
            filter.controller = Some(match player {
                PlayerAst::You => PlayerFilter::You,
                PlayerAst::Any => PlayerFilter::Any,
                _ => PlayerFilter::Any,
            });
            if tagged_relation {
                filter = filter
                    .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
            }
            PredicateAst::PlayerControlsNo { player, filter }
        }),
    )
}

fn parse_you_dont_control_shape(
    tokens: &[OwnedLexToken],
) -> Option<Result<PredicateAst, CardTextError>> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[
        &["dont", "control"],
        &["dont", "controls"],
        &["don't", "control"],
        &["don't", "controls"],
        &["do", "not", "control"],
        &["do", "not", "controls"],
    ];
    for action_phrase in action_phrases {
        let atoms = [
            LexPattern::subject("player", LexCaptureKind::UntilPhrase(action_phrase)),
            LexPattern::action("control", LexCaptureKind::WordCount(action_phrase.len())),
            LexPattern::object("object", LexCaptureKind::Rest),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let Some(player_clause) = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)
        else {
            continue;
        };
        if !matches!(player_clause.word_refs().as_slice(), ["you"]) {
            continue;
        }
        let Some(action) = matched.capture_clause_by_role(LexCaptureRole::Action, clause) else {
            continue;
        };
        if action.word_refs().as_slice() != *action_phrase {
            continue;
        }
        let Some(object) = matched.capture_clause_by_role(LexCaptureRole::Object, clause) else {
            continue;
        };
        let object_words = object.word_refs();
        if object_words.is_empty() {
            continue;
        }
        let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&object_words);
        let other = control_tokens
            .first()
            .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
        return Some(
            parse_object_filter(&control_tokens, other).map(|mut filter| {
                filter.controller = Some(PlayerFilter::You);
                PredicateAst::PlayerControlsNo {
                    player: PlayerAst::You,
                    filter,
                }
            }),
        );
    }
    None
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
    let Some(or_idx) = OR_IF_PATTERN.find_exact_window_range(filtered, 2, 2) else {
        return Ok(None);
    };
    if or_idx == 0 || or_idx + 2 >= filtered.len() {
        return Ok(None);
    }

    let left_tokens = predicate_tokens_from_words(&filtered[..or_idx]);
    let right_tokens = predicate_tokens_from_words(&filtered[or_idx + 2..]);
    let left = match parse_predicate(&left_tokens) {
        Ok(predicate) => predicate,
        Err(_) => return Ok(None),
    };
    let right = parse_predicate(&right_tokens)?;
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

fn parse_combat_turn_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_you_attacked_this_turn_shape(&tokens)
        .or_else(|| parse_triggering_object_had_to_attack_this_combat_shape(&tokens))
        .or_else(|| parse_you_attacked_with_exactly_other_creatures_shape(&tokens))
        .or_else(|| parse_source_attacked_or_blocked_this_turn_shape(&tokens))
        .or_else(|| parse_source_didnt_attack_or_enter_control_this_turn_shape(&tokens))
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

fn parse_source_didnt_attack_or_enter_control_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["didnt", "attack", "or"], &["did", "not", "attack", "or"]];
    for action_phrase in action_phrases {
        let atoms = [
            LexPattern::subject("subject", LexCaptureKind::UntilPhrase(action_phrase)),
            LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
            LexPattern::object(
                "control_event",
                LexCaptureKind::UntilPhrase(&["this", "turn"]),
            ),
            LexPattern::modifier("window", LexCaptureKind::Rest),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        if !matches!(subject_clause.word_refs().as_slice(), ["this", "creature"]) {
            continue;
        }
        let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
        if action_clause.word_refs().as_slice() != *action_phrase {
            continue;
        }
        let control_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        if !matches!(
            control_clause.word_refs().as_slice(),
            ["come", "under", "your", "control"] | ["came", "under", "your", "control"]
        ) {
            continue;
        }
        let window_clause = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
        if !matches!(window_clause.word_refs().as_slice(), ["this", "turn"]) {
            continue;
        }
        return Some(PredicateAst::And(
            Box::new(PredicateAst::Not(Box::new(
                PredicateAst::SourceAttackedThisTurn,
            ))),
            Box::new(PredicateAst::Not(Box::new(
                PredicateAst::SourceCameUnderYourControlThisTurn,
            ))),
        ));
    }
    None
}

fn parse_source_status_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    parse_source_status_with_copula_shape(&tokens)
        .or_else(|| parse_compact_source_status_shape(&tokens))
}

fn parse_source_status_with_copula_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let copula_phrases: &[&[&str]] = &[
        &["is"],
        &["are"],
        &["isnt"],
        &["isn't"],
        &["arent"],
        &["aren't"],
    ];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(copula_phrases)),
        LexPattern::action("copula", LexCaptureKind::WordCount(1)),
        LexPattern::object("state", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject_words = subject_clause.word_refs();
    if !is_source_reference_words(&subject_words) {
        return None;
    }
    let copula_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    let negative = matches!(
        copula_clause.word_refs().as_slice(),
        ["isnt"] | ["isn't"] | ["arent"] | ["aren't"]
    );
    let state_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    source_status_predicate_from_state(&state_clause.word_refs(), negative)
}

fn parse_compact_source_status_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let state_phrases: &[&[&str]] = &[&["tapped"], &["untapped"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(state_phrases)),
        LexPattern::object("state", LexCaptureKind::WordCount(1)),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject_words = subject_clause.word_refs();
    if !is_source_reference_words(&subject_words) {
        return None;
    }
    let state_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    source_status_predicate_from_state(&state_clause.word_refs(), false)
}

fn source_status_predicate_from_state(words: &[&str], negative: bool) -> Option<PredicateAst> {
    match (negative, words) {
        (false, ["tapped"]) => Some(PredicateAst::SourceIsTapped),
        (false, ["untapped"]) | (true, ["tapped"]) => {
            Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)))
        }
        (false, ["saddled"]) => Some(PredicateAst::SourceIsSaddled),
        (true, ["saddled"]) => Some(PredicateAst::Not(Box::new(PredicateAst::SourceIsSaddled))),
        _ => None,
    }
}

fn parse_source_cast_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    parse_active_source_cast_shape(&tokens)
        .or_else(|| parse_tagged_passive_cast_shape(&tokens))
        .or_else(|| parse_source_cast_from_zone_shape(&tokens))
}

fn parse_cast_payment_state_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    parse_no_spells_cast_last_turn_shape(&tokens)
        .or_else(|| parse_source_spell_state_shape(&tokens))
        .or_else(|| parse_target_spell_state_shape(&tokens))
        .or_else(|| parse_cast_this_spell_during_your_main_phase_shape(&tokens))
        .or_else(|| parse_named_payment_state_shape(&tokens))
}

fn parse_cast_this_spell_during_your_main_phase_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("player", LexCaptureKind::WordCount(1)),
        LexPattern::action("cast", LexCaptureKind::WordCount(1)),
        LexPattern::object("spell", LexCaptureKind::UntilPhrase(&["during"])),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let player = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(player.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["cast"]) {
        return None;
    }
    let spell = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(spell.word_refs().as_slice(), ["this", "spell"]) {
        return None;
    }
    let window = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !matches!(
        window.word_refs().as_slice(),
        ["during", "your", "main", "phase"]
    ) {
        return None;
    }
    Some(PredicateAst::ThisSpellPaidLabel(
        "CastDuringYourMainPhase".to_string(),
    ))
}

fn parse_no_spells_cast_last_turn_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::word("no"),
        LexPattern::object("spell", LexCaptureKind::WordCount(1)),
        LexPattern::action("action", LexCaptureKind::WordCount(2)),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let spell = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(spell.word_refs().as_slice(), ["spell"] | ["spells"]) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(
        action.word_refs().as_slice(),
        ["was", "cast"] | ["were", "cast"]
    ) {
        return None;
    }
    let window = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !matches!(window.word_refs().as_slice(), ["last", "turn"]) {
        return None;
    }
    Some(PredicateAst::NoSpellsWereCastLastTurn)
}

fn parse_source_spell_state_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("spell", LexCaptureKind::UntilPhrase(&["was"])),
        LexPattern::action("copula", LexCaptureKind::WordCount(1)),
        LexPattern::object("state", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let spell = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(
        spell.word_refs().as_slice(),
        ["this", "spell"] | ["this", "creature"] | ["this", "permanent"] | ["it"]
    ) {
        return None;
    }
    let copula = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(copula.word_refs().as_slice(), ["was"]) {
        return None;
    }
    let state = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    match state.word_refs().as_slice() {
        ["kicked"] => Some(PredicateAst::ThisSpellWasKicked),
        ["bargained"] => Some(PredicateAst::ThisSpellPaidLabel("Bargain".to_string())),
        _ => None,
    }
}

fn parse_target_spell_state_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("spell", LexCaptureKind::UntilPhrase(&["was"])),
        LexPattern::action("copula", LexCaptureKind::WordCount(1)),
        LexPattern::object("state", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let spell = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(spell.word_refs().as_slice(), ["that"]) {
        return None;
    }
    let copula = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(copula.word_refs().as_slice(), ["was"]) {
        return None;
    }
    let state = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(state.word_refs().as_slice(), ["kicked"]) {
        return None;
    }
    Some(PredicateAst::TargetWasKicked)
}

fn parse_global_state_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    parse_no_creatures_on_battlefield_shape(&tokens)
        .or_else(|| parse_you_or_defending_initiative_shape(&tokens))
        .or_else(|| parse_it_is_night_shape(&tokens))
        .or_else(|| parse_first_combat_phase_shape(&tokens))
}

fn parse_no_creatures_on_battlefield_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::word("no"),
        LexPattern::object("object", LexCaptureKind::WordCount(1)),
        LexPattern::action("location", LexCaptureKind::WordCount(2)),
        LexPattern::modifier("zone", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(object.word_refs().as_slice(), ["creature"] | ["creatures"]) {
        return None;
    }
    let location = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(
        location.word_refs().as_slice(),
        ["are", "on"] | ["is", "on"]
    ) {
        return None;
    }
    let zone = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !matches!(zone.word_refs().as_slice(), ["battlefield"]) {
        return None;
    }
    Some(PredicateAst::PlayerControlsNo {
        player: PlayerAst::Any,
        filter: ObjectFilter::creature(),
    })
}

fn parse_you_or_defending_initiative_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("left", LexCaptureKind::WordCount(1)),
        LexPattern::word("or"),
        LexPattern::subject("right", LexCaptureKind::UntilPhrase(&["has"])),
        LexPattern::action("action", LexCaptureKind::WordCount(1)),
        LexPattern::object("status", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let left = matched.capture_clause("left", clause)?;
    if !matches!(left.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let right = matched.capture_clause("right", clause)?;
    if !matches!(
        right.word_refs().as_slice(),
        ["player", "youre", "attacking"] | ["a", "player", "youre", "attacking"]
    ) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["has"]) {
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

fn parse_it_is_night_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let copula_atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["is"])),
        LexPattern::action("copula", LexCaptureKind::WordCount(1)),
        LexPattern::object("state", LexCaptureKind::Rest),
    ];
    if let Some(matched) = LexPattern::new(&copula_atoms).match_clause(clause) {
        let subject = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        let copula = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
        let state = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        if matches!(subject.word_refs().as_slice(), ["it"])
            && matches!(copula.word_refs().as_slice(), ["is"])
            && matches!(state.word_refs().as_slice(), ["night"])
        {
            return Some(PredicateAst::ItIsNight);
        }
    }

    let compact_atoms = [
        LexPattern::subject("subject", LexCaptureKind::WordCount(1)),
        LexPattern::object("state", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&compact_atoms).match_clause(clause)?;
    let subject = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let state = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if matches!(subject.word_refs().as_slice(), ["it"])
        && matches!(state.word_refs().as_slice(), ["night"])
    {
        Some(PredicateAst::ItIsNight)
    } else {
        None
    }
}

fn parse_first_combat_phase_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let copula_atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["is"])),
        LexPattern::action("copula", LexCaptureKind::WordCount(1)),
        LexPattern::object("state", LexCaptureKind::Rest),
    ];
    if let Some(matched) = LexPattern::new(&copula_atoms).match_clause(clause) {
        let subject = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        let copula = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
        let state = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        if matches!(subject.word_refs().as_slice(), ["it"])
            && matches!(copula.word_refs().as_slice(), ["is"])
            && first_combat_phase_tail(&state.word_refs())
        {
            return Some(PredicateAst::FirstCombatPhaseOfTurn);
        }
    }

    let compact_atoms = [
        LexPattern::subject("subject", LexCaptureKind::WordCount(1)),
        LexPattern::object("state", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&compact_atoms).match_clause(clause)?;
    let subject = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let state = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if matches!(subject.word_refs().as_slice(), ["it"])
        && first_combat_phase_tail(&state.word_refs())
    {
        Some(PredicateAst::FirstCombatPhaseOfTurn)
    } else {
        None
    }
}

fn first_combat_phase_tail(words: &[&str]) -> bool {
    matches!(
        words,
        ["the", "first", "combat", "phase", "of", "the", "turn"]
            | ["first", "combat", "phase", "of", "turn"]
            | ["the", "first", "combat", "phase", "of", "turn"]
    )
}

fn parse_source_dealt_combat_damage_to_player_this_turn(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["dealt"])),
        LexPattern::action("action", LexCaptureKind::WordCount(4)),
        LexPattern::object("recipient", LexCaptureKind::UntilPhrase(&["this", "turn"])),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(source.word_refs().as_slice(), ["it"]) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(
        action.word_refs().as_slice(),
        ["dealt", "combat", "damage", "to"]
    ) {
        return None;
    }
    let recipient = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        recipient.word_refs().as_slice(),
        ["player"] | ["a", "player"]
    ) {
        return None;
    }
    let window = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !matches!(window.word_refs().as_slice(), ["this", "turn"]) {
        return None;
    }
    Some(PredicateAst::SourceDealtCombatDamageToPlayerThisTurn)
}

fn parse_player_was_dealt_combat_damage_by_subtype_this_turn(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("player", LexCaptureKind::UntilPhrase(&["was", "dealt"])),
        LexPattern::action("damage", LexCaptureKind::WordCount(5)),
        LexPattern::object("source", LexCaptureKind::UntilPhrase(&["this", "turn"])),
        LexPattern::modifier("window", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let Some(player_clause) = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)
    else {
        return Ok(None);
    };
    let player = match player_clause.word_refs().as_slice() {
        ["player"] | ["a", "player"] => PlayerAst::Any,
        ["opponent"] | ["an", "opponent"] => PlayerAst::Opponent,
        _ => return Ok(None),
    };
    let Some(action) = matched.capture_clause_by_role(LexCaptureRole::Action, clause) else {
        return Ok(None);
    };
    if !matches!(
        action.word_refs().as_slice(),
        ["was", "dealt", "combat", "damage", "by"]
    ) {
        return Ok(None);
    }
    let Some(source) = matched.capture_clause_by_role(LexCaptureRole::Object, clause) else {
        return Ok(None);
    };
    let subtype_words = source.word_refs();
    if subtype_words.len() != 1 {
        return Ok(None);
    }
    let subtype = parse_subtype_word(subtype_words[0]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported combat-damage source subtype predicate: {}",
            filtered.join(" ")
        ))
    })?;
    let Some(window) = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause) else {
        return Ok(None);
    };
    if !matches!(window.word_refs().as_slice(), ["this", "turn"]) {
        return Ok(None);
    }
    Ok(Some(
        PredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype },
    ))
}

fn parse_tagged_object_lifecycle_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    parse_you_controlled_tagged_permanent_shape(&tokens)
        .or_else(|| parse_tagged_entered_under_your_control_shape(&tokens))
        .or_else(|| parse_you_didnt_put_tagged_object_in_zone_shape(&tokens))
        .or_else(|| parse_tagged_wasnt_blocking_shape(&tokens))
}

fn parse_tagged_battlefield_this_way_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    if let Some(predicate) = parse_you_put_tagged_battlefield_this_way_shape(&tokens)? {
        return Ok(Some(predicate));
    }
    parse_tagged_is_put_battlefield_this_way_shape(&tokens)
}

fn parse_you_put_tagged_battlefield_this_way_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let destination_phrases: &[&[&str]] = &[
        &["onto", "battlefield", "this", "way"],
        &["onto", "the", "battlefield", "this", "way"],
    ];
    let atoms = [
        LexPattern::subject("player", LexCaptureKind::WordCount(1)),
        LexPattern::action("put", LexCaptureKind::WordCount(1)),
        LexPattern::object(
            "object",
            LexCaptureKind::UntilAnyPhrase(destination_phrases),
        ),
        LexPattern::modifier("destination", LexCaptureKind::Rest),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let Some(player) = matched.capture_clause_by_role(LexCaptureRole::Subject, clause) else {
        return Ok(None);
    };
    if !matches!(player.word_refs().as_slice(), ["you"]) {
        return Ok(None);
    }
    let Some(action) = matched.capture_clause_by_role(LexCaptureRole::Action, clause) else {
        return Ok(None);
    };
    if !matches!(action.word_refs().as_slice(), ["put"]) {
        return Ok(None);
    }
    let Some(destination) = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause) else {
        return Ok(None);
    };
    if !matches!(
        destination.word_refs().as_slice(),
        ["onto", "battlefield", "this", "way"] | ["onto", "the", "battlefield", "this", "way"]
    ) {
        return Ok(None);
    }
    let Some(object) = matched.capture_clause_by_role(LexCaptureRole::Object, clause) else {
        return Ok(None);
    };
    let object_words = object.word_refs();
    if object_words.is_empty() {
        return Ok(None);
    }
    let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&object_words);
    let filter = parse_object_filter(&filter_tokens, false)?;
    Ok(Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::You,
        tag: TagKey::from(IT_TAG),
        filter,
    }))
}

fn parse_tagged_is_put_battlefield_this_way_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["is", "put", "onto"], &["is", "put", "onto", "the"]];
    for action_phrase in action_phrases {
        let atoms = [
            LexPattern::object("object", LexCaptureKind::UntilPhrase(action_phrase)),
            LexPattern::action("put", LexCaptureKind::WordCount(action_phrase.len())),
            LexPattern::modifier("destination", LexCaptureKind::Rest),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let Some(action) = matched.capture_clause_by_role(LexCaptureRole::Action, clause) else {
            continue;
        };
        if action.word_refs().as_slice() != *action_phrase {
            continue;
        }
        let Some(destination) = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)
        else {
            continue;
        };
        if !matches!(
            destination.word_refs().as_slice(),
            ["battlefield", "this", "way"]
        ) {
            continue;
        }
        let Some(object) = matched.capture_clause_by_role(LexCaptureRole::Object, clause) else {
            continue;
        };
        let object_words = object.word_refs();
        if object_words.is_empty() {
            continue;
        }
        let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&object_words);
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(Some(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            filter,
        )));
    }
    Ok(None)
}

fn parse_you_controlled_tagged_permanent_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("player", LexCaptureKind::WordCount(1)),
        LexPattern::action("control", LexCaptureKind::WordCount(1)),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let player = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(player.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["control"] | ["controlled"]) {
        return None;
    }
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(object.word_refs().as_slice(), ["that", "permanent"]) {
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
    let atoms = [
        LexPattern::subject("object", LexCaptureKind::UntilPhrase(&["entered"])),
        LexPattern::action("entered", LexCaptureKind::WordCount(4)),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let object = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(
        object.word_refs().as_slice(),
        ["it"] | ["that", "card"] | ["that", "permanent"]
    ) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(
        action.word_refs().as_slice(),
        ["entered", "under", "your", "control"]
    ) {
        return None;
    }
    Some(PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn {
        player: PlayerAst::You,
        tag: TagKey::from(IT_TAG),
    })
}

fn parse_you_didnt_put_tagged_object_in_zone_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    parse_you_didnt_put_tagged_object_in_zone_with_action(tokens, &["dont", "put"])
        .or_else(|| {
            parse_you_didnt_put_tagged_object_in_zone_with_action(tokens, &["didnt", "put"])
        })
        .or_else(|| {
            parse_you_didnt_put_tagged_object_in_zone_with_action(tokens, &["did", "not", "put"])
        })
}

fn parse_you_didnt_put_tagged_object_in_zone_with_action(
    tokens: &[OwnedLexToken],
    action_phrase: &[&str],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let destination_phrases: &[&[&str]] = &[&["into", "your", "hand"], &["onto", "battlefield"]];
    let atoms = [
        LexPattern::subject("player", LexCaptureKind::UntilPhrase(action_phrase)),
        LexPattern::action("put", LexCaptureKind::WordCount(action_phrase.len())),
        LexPattern::object(
            "object",
            LexCaptureKind::UntilAnyPhrase(destination_phrases),
        ),
        LexPattern::modifier("destination", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let player = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(player.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if action.word_refs().as_slice() != action_phrase {
        return None;
    }
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        object.word_refs().as_slice(),
        ["it"] | ["card"] | ["the", "card"] | ["that", "card"]
    ) {
        return None;
    }
    let destination = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    let zone = match destination.word_refs().as_slice() {
        ["into", "your", "hand"] => Zone::Hand,
        ["onto", "battlefield"] => Zone::Battlefield,
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

fn parse_tagged_wasnt_blocking_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["wasnt", "blocking"], &["was", "not", "blocking"]];
    for action_phrase in action_phrases {
        let atoms = [
            LexPattern::subject("object", LexCaptureKind::UntilPhrase(action_phrase)),
            LexPattern::action("blocking", LexCaptureKind::WordCount(action_phrase.len())),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let object = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        if !matches!(object.word_refs().as_slice(), ["it"] | ["that", "creature"]) {
            continue;
        }
        let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
        if action.word_refs().as_slice() != *action_phrase {
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

fn parse_opponent_controls_referenced_object_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("player", LexCaptureKind::UntilPhrase(&["controls"])),
        LexPattern::action("control", LexCaptureKind::WordCount(1)),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let player = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(
        player.word_refs().as_slice(),
        ["opponent"] | ["an", "opponent"]
    ) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["controls"]) {
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

fn parse_source_exploited_triggering_object_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["exploited"])),
        LexPattern::action("exploit", LexCaptureKind::WordCount(1)),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(source.word_refs().as_slice(), ["it"]) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["exploited"]) {
        return None;
    }
    let object = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        object.word_refs().as_slice(),
        ["that", "creature"] | ["that", "object"]
    ) {
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

fn parse_vote_option_gets_more_votes_or_tied_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::object("option", LexCaptureKind::UntilPhrase(&["gets"])),
        LexPattern::action("gets", LexCaptureKind::WordCount(1)),
        LexPattern::modifier("result", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let option = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let option_words = option.word_refs();
    if option_words.is_empty() {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["gets"]) {
        return None;
    }
    let result = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !matches!(
        result.word_refs().as_slice(),
        ["more", "votes", "or", "vote", "is", "tied"]
    ) {
        return None;
    }
    Some(PredicateAst::VoteOptionGetsMoreVotesOrTied {
        option: option_words.join(" "),
    })
}

fn parse_vote_option_gets_more_votes_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::object("option", LexCaptureKind::UntilPhrase(&["gets"])),
        LexPattern::action("gets", LexCaptureKind::WordCount(1)),
        LexPattern::modifier("result", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let option = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let option_words = option.word_refs();
    if option_words.is_empty() {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["gets"]) {
        return None;
    }
    let result = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !matches!(result.word_refs().as_slice(), ["more", "votes"]) {
        return None;
    }
    Some(PredicateAst::VoteOptionGetsMoreVotes {
        option: option_words.join(" "),
    })
}

fn parse_no_vote_objects_matched_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let atoms = [
        LexPattern::word("no"),
        LexPattern::object("object", LexCaptureKind::UntilPhrase(&["got", "votes"])),
        LexPattern::action("got_votes", LexCaptureKind::WordCount(2)),
    ];
    let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
        return Ok(None);
    };
    let Some(object) = matched.capture_clause_by_role(LexCaptureRole::Object, clause) else {
        return Ok(None);
    };
    let object_words = object.word_refs();
    if object_words.is_empty() {
        return Ok(None);
    }
    let Some(action) = matched.capture_clause_by_role(LexCaptureRole::Action, clause) else {
        return Ok(None);
    };
    if !matches!(action.word_refs().as_slice(), ["got", "votes"]) {
        return Ok(None);
    }
    let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&object_words);
    let filter = parse_object_filter(&filter_tokens, false)?;
    Ok(Some(PredicateAst::NoVoteObjectsMatched { filter }))
}

fn parse_triggering_object_counter_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    parse_triggering_object_counter_shape(&tokens, &["had", "no"], true)
        .or_else(|| parse_triggering_object_counter_shape(&tokens, &["had"], false))
}

fn parse_source_has_counter_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let tail_phrases: &[&[&str]] = &[
        &["on", "it"],
        &["on", "him"],
        &["on", "her"],
        &["on", "them"],
        &["on", "this"],
        &["on", "that"],
    ];
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["has"])),
        LexPattern::action("has", LexCaptureKind::WordCount(1)),
        LexPattern::amount("counter", LexCaptureKind::UntilAnyPhrase(tail_phrases)),
        LexPattern::modifier("tail", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(
        source.word_refs().as_slice(),
        ["this"]
            | ["this", "creature"]
            | ["this", "permanent"]
            | ["this", "artifact"]
            | ["this", "enchantment"]
            | ["this", "land"]
            | ["this", "planeswalker"]
            | ["this", "battle"]
    ) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["has"]) {
        return None;
    }
    let tail = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !tail.word_refs().starts_with(&["on"]) {
        return None;
    }
    let counter_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let counter_words = counter_clause.word_refs();
    source_counter_predicate_from_words(&counter_words)
}

fn source_counter_predicate_from_words(words: &[&str]) -> Option<PredicateAst> {
    if words.len() >= 3
        && matches!(words.first(), Some(&"no"))
        && let Some(counter_type) = parse_counter_type_word(words[1])
        && COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(words[2])
        && words.len() == 3
    {
        return Some(PredicateAst::SourceHasNoCounter(counter_type));
    }

    if let Some((comparison, used)) = predicate_quantity_prefix(words)
        && let Some(count) = comparison_to_at_least_threshold(&comparison)
        && let Some(counter_idx) = find_index(&words[used..], |word| {
            COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
        })
        && counter_idx > 0
    {
        let counter_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&words[used..=used + counter_idx]);
        let counter_type = parse_counter_type_from_tokens(&counter_tokens)?;
        return Some(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
        });
    }

    if !OR_MORE_PREFIX_PATTERN.matches_words(words.get(1..).unwrap_or_default())
        && let Some(counter_idx) = find_index(words, |word| {
            COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
        })
        && counter_idx > 0
    {
        let counter_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&words[..=counter_idx]);
        let counter_type = parse_counter_type_from_tokens(&counter_tokens)?;
        return Some(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count: 1,
        });
    }

    None
}

fn parse_there_are_source_counter_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let counter_words: &[&[&str]] = &[&["counter"], &["counters"]];
    let atoms = [
        LexPattern::word("there"),
        LexPattern::action("are", LexCaptureKind::WordCount(1)),
        LexPattern::amount("quantity", LexCaptureKind::UntilAnyPhrase(counter_words)),
        LexPattern::object("counter_word", LexCaptureKind::WordCount(1)),
        LexPattern::modifier("tail", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["are"]) {
        return None;
    }
    let counter_word = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        counter_word.word_refs().as_slice(),
        ["counter"] | ["counters"]
    ) {
        return None;
    }
    let tail = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !source_counter_tail_matches(tail.word_refs().as_slice()) {
        return None;
    }
    let quantity = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let quantity_words = quantity.word_refs();
    let (comparison, used) = predicate_quantity_prefix(&quantity_words)?;
    let count = comparison_to_at_least_threshold(&comparison)?;
    let rest = &tokens[2 + used..tokens.len().saturating_sub(tail.word_refs().len())];
    let rest_words = crate::runtime_backend::token_word_refs(rest);
    if rest_words
        .first()
        .is_some_and(|word| COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word))
    {
        return Some(PredicateAst::SourceHasCountersAtLeast(count));
    }
    if let Some(counter_idx) = find_index(rest_words.as_slice(), |word| {
        COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
    }) && counter_idx > 0
    {
        let counter_type = parse_counter_type_from_tokens(&rest[..=counter_idx])?;
        return Some(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
        });
    }
    None
}

fn parse_there_are_no_source_counter_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let counter_words: &[&[&str]] = &[&["counter"], &["counters"]];
    let atoms = [
        LexPattern::word("there"),
        LexPattern::action("are", LexCaptureKind::WordCount(1)),
        LexPattern::amount("counter", LexCaptureKind::UntilAnyPhrase(counter_words)),
        LexPattern::object("counter_word", LexCaptureKind::WordCount(1)),
        LexPattern::modifier("tail", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["are"]) {
        return None;
    }
    let counter_word = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        counter_word.word_refs().as_slice(),
        ["counter"] | ["counters"]
    ) {
        return None;
    }
    let tail = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !source_counter_tail_matches(tail.word_refs().as_slice())
        && !matches!(tail.word_refs().as_slice(), ["on", "them"])
    {
        return None;
    }
    let counter = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let counter_words = counter.word_refs();
    if !matches!(counter_words.first(), Some(&"no")) {
        return None;
    }
    let counter_prefix_len = 2 + counter_words.len();
    let counter_type = parse_counter_type_from_tokens(&tokens[..=counter_prefix_len])?;
    Some(PredicateAst::SourceHasNoCounter(counter_type))
}

fn parse_source_power_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    parse_source_power_is_shape(&tokens).or_else(|| parse_source_has_power_shape(&tokens))
}

fn parse_source_power_is_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["power", "is"])),
        LexPattern::action("power", LexCaptureKind::WordCount(2)),
        LexPattern::amount("amount", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(
        source.word_refs().as_slice(),
        ["this", "creature"]
            | ["this", "creatures"]
            | ["this", "permanent"]
            | ["this", "permanents"]
    ) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["power", "is"]) {
        return None;
    }
    let amount = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    source_power_predicate_from_amount(amount.word_refs().as_slice())
}

fn parse_source_has_power_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("source", LexCaptureKind::WordCount(1)),
        LexPattern::action("has_power", LexCaptureKind::WordCount(2)),
        LexPattern::amount("amount", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let source = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(source.word_refs().as_slice(), ["this"]) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["has", "power"]) {
        return None;
    }
    let amount = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    source_power_predicate_from_amount(amount.word_refs().as_slice())
}

fn source_power_predicate_from_amount(words: &[&str]) -> Option<PredicateAst> {
    let (comparison, used) = predicate_quantity_prefix(words)?;
    if used != words.len() {
        return None;
    }
    let count = comparison_to_at_least_threshold(&comparison)?;
    Some(PredicateAst::SourcePowerAtLeast(count))
}

fn parse_basic_land_types_among_lands_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let basic_land_phrases: &[&[&str]] = &[
        &["basic", "land", "type", "among", "land"],
        &["basic", "land", "type", "among", "lands"],
        &["basic", "land", "types", "among", "land"],
        &["basic", "land", "types", "among", "lands"],
    ];
    let atoms = [
        LexPattern::word("there"),
        LexPattern::action("are", LexCaptureKind::WordCount(1)),
        LexPattern::amount("count", LexCaptureKind::UntilAnyPhrase(basic_land_phrases)),
        LexPattern::object("domain", LexCaptureKind::WordCount(5)),
        LexPattern::subject("player", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["are"]) {
        return None;
    }
    let domain = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !basic_land_phrases
        .iter()
        .any(|phrase| domain.word_refs().as_slice() == *phrase)
    {
        return None;
    }
    let count_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let (comparison, used) = predicate_quantity_prefix(&count_clause.word_refs())?;
    if used != count_clause.word_refs().len() {
        return None;
    }
    let count = comparison_to_at_least_threshold(&comparison)?;
    let player_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = match player_clause.word_refs().as_slice() {
        ["you", "control"] | ["you", "controls"] => PlayerAst::You,
        ["that", "player", "control"]
        | ["that", "player", "controls"]
        | ["that", "players", "controls"] => PlayerAst::That,
        _ => return None,
    };
    Some(PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count })
}

fn parse_graveyard_card_types_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filtered);
    let clause = LexedClause::new(&tokens);
    let card_type_phrases: &[&[&str]] = &[
        &["card", "type", "among", "card", "in"],
        &["card", "type", "among", "cards", "in"],
        &["card", "types", "among", "card", "in"],
        &["card", "types", "among", "cards", "in"],
    ];
    let atoms = [
        LexPattern::subject("intro", LexCaptureKind::WordCount(2)),
        LexPattern::amount("count", LexCaptureKind::UntilAnyPhrase(card_type_phrases)),
        LexPattern::object("domain", LexCaptureKind::WordCount(5)),
        LexPattern::modifier("graveyard", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let intro = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let constrain_to_you = match intro.word_refs().as_slice() {
        ["there", "are"] => false,
        ["you", "have"] => true,
        _ => return None,
    };

    let domain = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !card_type_phrases
        .iter()
        .any(|phrase| domain.word_refs().as_slice() == *phrase)
    {
        return None;
    }

    let count_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let (comparison, used) = predicate_quantity_prefix(&count_clause.word_refs())?;
    if used != count_clause.word_refs().len() {
        return None;
    }
    let count = comparison_to_at_least_threshold(&comparison)?;

    let graveyard = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    let player = match graveyard.word_refs().as_slice() {
        ["your", "graveyard"] => PlayerAst::You,
        ["that", "player", "graveyard"] | ["that", "players", "graveyard"] => PlayerAst::That,
        ["target", "player", "graveyard"] | ["target", "players", "graveyard"] => PlayerAst::Target,
        ["target", "opponent", "graveyard"] | ["target", "opponents", "graveyard"] => {
            PlayerAst::TargetOpponent
        }
        ["opponent", "graveyard"] | ["opponents", "graveyard"] => PlayerAst::Opponent,
        _ => return None,
    };
    if constrain_to_you && player != PlayerAst::You {
        return None;
    }

    Some(PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count })
}

fn source_counter_tail_matches(words: &[&str]) -> bool {
    matches!(
        words,
        ["on", "it"]
            | ["on", "this"]
            | ["on", "this", "artifact"]
            | ["on", "this", "creature"]
            | ["on", "this", "enchantment"]
            | ["on", "this", "land"]
            | ["on", "this", "permanent"]
    )
}

fn parse_triggering_object_counter_shape(
    tokens: &[OwnedLexToken],
    action_phrase: &[&str],
    negative: bool,
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let counter_words: &[&[&str]] = &[&["counter"], &["counters"]];
    let atoms = [
        LexPattern::subject("object", LexCaptureKind::UntilPhrase(action_phrase)),
        LexPattern::action("had", LexCaptureKind::WordCount(action_phrase.len())),
        LexPattern::amount(
            "counter_type",
            LexCaptureKind::UntilAnyPhrase(counter_words),
        ),
        LexPattern::object("counter_word", LexCaptureKind::WordCount(1)),
        LexPattern::modifier("tail", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let object = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(
        object.word_refs().as_slice(),
        ["it"] | ["this" | "that", "creature"] | ["this" | "that", "permanent"]
    ) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if action.word_refs().as_slice() != action_phrase {
        return None;
    }
    let counter_word = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        counter_word.word_refs().as_slice(),
        ["counter"] | ["counters"]
    ) {
        return None;
    }
    let tail = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !matches!(
        tail.word_refs().as_slice(),
        ["on", "it"] | ["on", "them"] | ["on", "this"] | ["on", "that"] | ["on", "itself"]
    ) {
        return None;
    }
    let counter_type = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let counter_words = counter_type.word_refs();
    if counter_words.is_empty() {
        return None;
    }
    let counter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&counter_words);
    let counter_type = if negative && counter_words.len() == 1 {
        parse_counter_type_word(counter_words[0])
    } else {
        parse_counter_type_from_tokens(&counter_tokens)
    }?;
    if negative {
        Some(PredicateAst::TriggeringObjectHadNoCounter(counter_type))
    } else {
        Some(PredicateAst::TriggeringObjectHadCounterAtLeast {
            counter_type,
            count: 1,
        })
    }
}

fn parse_named_payment_state_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["was"], &["wasnt"], &["was", "not"]];
    let atoms = [
        LexPattern::subject("label", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action(
            "state",
            LexCaptureKind::UntilAnyPhrase(&[&["promised"], &["paid"]]),
        ),
        LexPattern::object("result", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let label_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let (label, expected_result) = match label_clause.word_refs().as_slice() {
        ["gift"] => ("Gift", "promised"),
        ["tribute"] => ("Tribute", "paid"),
        _ => return None,
    };
    let state = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    let negative = match state.word_refs().as_slice() {
        ["was"] => false,
        ["wasnt"] | ["was", "not"] => true,
        _ => return None,
    };
    let result = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if result.word_refs().as_slice() != [expected_result] {
        return None;
    }
    let predicate = PredicateAst::ThisSpellPaidLabel(label.to_string());
    if negative {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

fn parse_active_source_cast_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("caster", LexCaptureKind::UntilPhrase(&["cast"])),
        LexPattern::action("action", LexCaptureKind::WordCount(1)),
        LexPattern::object("spell", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let caster = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(caster.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["cast"]) {
        return None;
    }
    let spell = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(spell.word_refs().as_slice(), ["it"] | ["this", "spell"]) {
        return None;
    }
    Some(PredicateAst::SourceWasCast)
}

fn parse_tagged_passive_cast_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("spell", LexCaptureKind::UntilPhrase(&["was", "cast"])),
        LexPattern::action("action", LexCaptureKind::WordCount(2)),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let spell = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(
        spell.word_refs().as_slice(),
        ["it"] | ["that", "creature"] | ["that", "permanent"] | ["that", "object"]
    ) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["was", "cast"]) {
        return None;
    }
    Some(PredicateAst::TaggedWasCast(TagKey::from(IT_TAG)))
}

fn parse_source_cast_from_zone_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject(
            "spell",
            LexCaptureKind::UntilPhrase(&["was", "cast", "from"]),
        ),
        LexPattern::action("action", LexCaptureKind::WordCount(3)),
        LexPattern::modifier("origin", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let spell = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(spell.word_refs().as_slice(), ["this", "spell"]) {
        return None;
    }
    let action = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(action.word_refs().as_slice(), ["was", "cast", "from"]) {
        return None;
    }
    let origin = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    let zone_words = origin.word_refs();
    if zone_words == ["anywhere", "other", "than", "your", "hand"] {
        return Some(PredicateAst::ThisSpellWasCastFromNonHand);
    }
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

fn graveyard_possessive_matches_subject(player: PlayerAst, possessive: &str) -> bool {
    match player {
        PlayerAst::You | PlayerAst::Implicit => YOUR_WORD_PATTERN.matches_word(possessive),
        _ => THEIR_WORD_PATTERN.matches_word(possessive),
    }
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
    if words.len() >= 7
        && THERE_ARE_OR_WERE_PREFIX_PATTERN.matches_words(words)
        && let Some((count, used)) = predicate_number_prefix(&words[2..])
        && words
            .get(2 + used)
            .is_some_and(|word| COLOR_OR_COLORS_WORD_PATTERN.matches_word(word))
        && words
            .get(3 + used)
            .is_some_and(|word| AMONG_WORD_PATTERN.matches_word(word))
        && let Some(filter) = permanents_you_control_scope(&words[4 + used..])
    {
        return Some(PredicateAst::ValueComparison {
            left: Value::ColorsAmong(filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }
    None
}

fn parse_card_types_among_predicate(words: &[&str]) -> Option<PredicateAst> {
    if words.len() >= 9
        && THERE_ARE_OR_WERE_PREFIX_PATTERN.matches_words(words)
        && let Some((count, rest_start)) = predicate_at_least_quantity_prefix(&words[2..])
        && words
            .get(2 + rest_start)
            .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
        && words
            .get(3 + rest_start)
            .is_some_and(|word| TYPE_OR_TYPES_WORD_PATTERN.matches_word(word))
        && words
            .get(4 + rest_start)
            .is_some_and(|word| AMONG_WORD_PATTERN.matches_word(word))
        && words
            .get(5 + rest_start)
            .is_some_and(|word| SACRIFICED_OR_SACRIFICED_TAG_WORD_PATTERN.matches_word(word))
        && (words
            .get(6 + rest_start)
            .is_some_and(|word| PERMANENT_OR_PERMANENTS_WORD_PATTERN.matches_word(word))
            || words.len() == 6 + rest_start)
    {
        return Some(PredicateAst::ValueComparison {
            left: Value::CardTypesAmong(ObjectFilter::tagged("sacrificed_0")),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }

    if words.len() >= 13
        && THERE_ARE_OR_WERE_PREFIX_PATTERN.matches_words(words)
        && let Some((count, rest_start)) = predicate_at_least_quantity_prefix(&words[2..])
        && words
            .get(2 + rest_start)
            .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
        && words
            .get(3 + rest_start)
            .is_some_and(|word| TYPE_OR_TYPES_WORD_PATTERN.matches_word(word))
        && words
            .get(4 + rest_start)
            .is_some_and(|word| AMONG_WORD_PATTERN.matches_word(word))
        && let Some(filter) = permanents_and_your_graveyard_scope(&words[5 + rest_start..])
    {
        return Some(PredicateAst::ValueComparison {
            left: Value::CardTypesAmong(filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }
    None
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
    if words.len() < 7 {
        return None;
    }
    let (comparison, used) = predicate_quantity_prefix(words)?;
    let count = comparison_to_strict_at_least_threshold(&comparison)?;
    let have_idx = find_index(words, |word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))?;
    if have_idx <= used {
        return None;
    }
    let object_words = &words[used..have_idx];
    let counter_words = &words[have_idx + 1..];
    if object_words.is_empty() || counter_words.is_empty() {
        return None;
    }
    let (counter_constraint, consumed) = parse_filter_counter_constraint_words(counter_words)?;
    if consumed != counter_words.len() {
        return None;
    }

    let object_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(object_words);
    let other = object_tokens
        .first()
        .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
    let mut filter = parse_object_filter(&object_tokens, other).ok()?;
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
    if words.len() < 7 {
        return None;
    }
    let (comparison, used) = predicate_quantity_prefix(words)?;
    let (operator, count) = comparison_to_value_comparison_operator(comparison)?;
    let have_idx = find_index(words, |word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))?;
    if have_idx <= used {
        return None;
    }
    let object_words = &words[used..have_idx];
    let tail_words = &words[have_idx + 1..];
    if object_words.is_empty()
        || !BEEN_EXILED_WITH_THIS_SOURCE_PREFIX_PATTERN.matches_words(tail_words)
    {
        return None;
    }

    let object_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(object_words);
    let mut filter = if object_words
        .iter()
        .all(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
    {
        ObjectFilter::default()
    } else {
        parse_object_filter(&object_tokens, false).ok()?
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
    let suffix_len = usize::from(BEHOLD_CAST_SUFFIX_PATTERN.matches_words(words)) * 5;
    let core_words = if suffix_len > 0 {
        &words[..words.len().saturating_sub(suffix_len)]
    } else {
        words
    };

    if core_words.len() != 7
        || !core_words
            .get(0..2)
            .is_some_and(|prefix| YOU_REVEALED_PREFIX_PATTERN.matches_words(prefix))
        || parse_subtype_word(core_words[2]).is_none()
        || !CARD_WORD_PATTERN.matches_word(core_words[3])
        || !OR_WORD_PATTERN.matches_word(core_words[4])
        || !CONTROL_OR_CONTROLLED_WORD_PATTERN.matches_word(core_words[5])
        || parse_subtype_word(core_words[6]).is_none()
        || core_words[2] != core_words[6]
    {
        return None;
    }

    Some(PredicateAst::Or(
        Box::new(PredicateAst::ThisSpellPaidLabel("Behold".to_string())),
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: ObjectFilter::default().with_subtype(parse_subtype_word(core_words[2])?),
        }),
    ))
}

fn parse_card_in_your_graveyard_predicate(words: &[&str]) -> Option<PredicateAst> {
    if words.len() < 6 || !THERE_IS_PREFIX_PATTERN.matches_words(words) {
        return None;
    }

    let in_idx = IN_WORD_PATTERN.find_word(&words[2..]).map(|idx| idx + 2)?;
    if in_idx <= 2 {
        return None;
    }
    if !IN_YOUR_GRAVEYARD_TAIL_PATTERN.matches_words(&words[in_idx..]) {
        return None;
    }

    let descriptor_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&words[2..in_idx]);
    let mut filter = parse_object_filter(&descriptor_tokens, false).ok()?;
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
    let words = crate::runtime_backend::token_word_refs(tokens);
    let suffix_len = if word_slice_ends_with(&words, &["is", "on", "the", "battlefield"])
        || word_slice_ends_with(&words, &["are", "on", "the", "battlefield"])
    {
        4
    } else if word_slice_ends_with(&words, &["is", "on", "battlefield"])
        || word_slice_ends_with(&words, &["are", "on", "battlefield"])
    {
        3
    } else {
        return Ok(None);
    };
    let object_token_end = tokens.len().saturating_sub(suffix_len);
    if object_token_end == 0 {
        return Ok(None);
    }

    let object_tokens = &tokens[..object_token_end];
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
    if matches!(
        filtered.as_slice(),
        ["they", "match"] | ["those", "choices", "match"]
    ) {
        return Ok(PredicateAst::SecretChoicesMatch);
    }
    if let Some(predicate) = parse_vote_option_gets_more_votes_or_tied_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_passive_this_way_tagged_object_predicate(&filtered)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_active_this_way_discard_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_this_ability_resolution_count_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_stack_object_targets_only_source_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_exploited_triggering_object_predicate(&filtered) {
        return Ok(predicate);
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

    if filtered.len() >= 4 && filtered.get(0..2) == Some(&["you", "have"]) {
        let tail_words = &filtered[2..];
        if tail_words.last().copied() == Some("life") {
            let quantity_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(
                &tail_words[..tail_words.len() - 1],
            );
            if let Some((amount, used)) = parse_less_than_or_equal_quantity_prefix(
                &quantity_tokens,
                false,
                false,
                "life-total predicate",
            )
            .ok()
            .flatten()
                && used == tail_words.len() - 1
            {
                return Ok(PredicateAst::ValueComparison {
                    left: Value::LifeTotal(PlayerFilter::You),
                    operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                    right: Value::Fixed(amount as i32),
                });
            }
        }
    }
    if filtered.len() >= 6 && filtered.get(0..4) == Some(&["your", "life", "total", "is"]) {
        let quantity_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[4..]);
        if let Some((amount, used)) = parse_less_than_or_equal_quantity_prefix(
            &quantity_tokens,
            false,
            false,
            "life-total predicate",
        )
        .ok()
        .flatten()
            && used == filtered.len() - 4
        {
            return Ok(PredicateAst::ValueComparison {
                left: Value::LifeTotal(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                right: Value::Fixed(amount as i32),
            });
        }
    }

    if let Some(has_idx) = find_index(&filtered, |word| {
        HAS_OR_HAVE_WORD_PATTERN.matches_word(word)
    }) && has_idx > 0
        && has_idx + 1 < filtered.len()
        && filtered[..has_idx]
            .iter()
            .any(|word| CONTROL_WORD_PATTERN.matches_word(word))
        && let Some((constraint, consumed)) =
            parse_filter_keyword_constraint_words(&filtered[has_idx + 1..])
        && has_idx + 1 + consumed == filtered.len()
    {
        let mut subject_words = filtered[..has_idx].to_vec();
        subject_words.retain(|word| {
            !YOU_WORD_PATTERN.matches_word(word)
                && !CONTROL_OR_CONTROLS_WORD_PATTERN.matches_word(word)
        });
        let subject_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(subject_words);
        let mut filter = parse_object_filter(&subject_tokens, false)?;
        apply_filter_keyword_constraint(&mut filter, constraint, false);
        filter.controller = Some(PlayerFilter::You);
        return Ok(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        });
    }

    if let Some(has_idx) = find_index(&filtered, |word| {
        HAS_OR_HAVE_WORD_PATTERN.matches_word(word)
    }) && has_idx > 0
        && has_idx + 1 < filtered.len()
        && filtered[..has_idx]
            .iter()
            .any(|word| ZONE_WORD_PATTERN.matches_word(word))
        && let Some((constraint, consumed)) =
            parse_filter_keyword_constraint_words(&filtered[has_idx + 1..])
        && has_idx + 1 + consumed == filtered.len()
    {
        let subject_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[..has_idx]);
        let mut filter = parse_object_filter(&subject_tokens, false)?;
        apply_filter_keyword_constraint(&mut filter, constraint, false);
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        return Ok(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        });
    }

    if let Some(predicate) = parse_opponent_controls_referenced_object_predicate(&filtered) {
        return Ok(predicate);
    }

    if filtered.len() >= 3
        && OPPONENT_CONTROLS_PREFIX_PATTERN.matches_words(&filtered)
        && !(filtered[2] == "more" && word_slice_contains_word(&filtered[3..], "than"))
    {
        let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[2..]);
        let other = control_tokens
            .first()
            .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::Opponent);
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::Opponent,
                filter,
            });
        }
    }

    if raw_words.len() >= 4
        && AN_OPPONENT_CONTROLS_PREFIX_PATTERN.matches_words(&raw_words)
        && !(raw_words[3] == "more" && word_slice_contains_word(&raw_words[4..], "than"))
    {
        let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&raw_words[3..]);
        let other = control_tokens
            .first()
            .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::Opponent);
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::Opponent,
                filter,
            });
        }
    }

    if let Some(predicate) = parse_vote_option_gets_more_votes_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_no_vote_objects_matched_predicate(&filtered)? {
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

    if let Some(while_idx) = find_index(&filtered, |word| WHILE_WORD_PATTERN.matches_word(word))
        && while_idx > 0
        && while_idx + 1 < filtered.len()
    {
        let left_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[..while_idx]);
        let right_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[while_idx + 1..]);
        let left = parse_predicate(&left_tokens)?;
        let right = parse_predicate(&right_tokens)?;
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
        return Ok(PredicateAst::And(Box::new(left), Box::new(right)));
    }

    if let Some(predicate) = parse_source_status_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(is_idx) = find_index(&filtered, |word| IS_OR_ARE_WORD_PATTERN.matches_word(word)) {
        let subject_words = &filtered[..is_idx];
        let is_source_subject = is_source_reference_words(subject_words)
            || SOURCE_REFERENCE_WORD_PATTERN.matches_words(subject_words);
        if is_source_subject && ENCHANTED_BY_PREFIX_PATTERN.matches_words(&filtered[is_idx + 1..]) {
            let attachment_tokens =
                crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[is_idx + 3..]);
            let (comparison, used) = parse_attachment_quantity_prefix(&attachment_tokens)?;
            let filter_tokens = &attachment_tokens[used..];
            if !filter_tokens.is_empty() {
                let filter = parse_object_filter(filter_tokens, false).or_else(|_| {
                    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
                    if AURA_WORD_PATTERN.matches_words(&filter_words) {
                        Ok(ObjectFilter::default().with_subtype(Subtype::Aura))
                    } else {
                        Err(CardTextError::ParseError(format!(
                            "unsupported attachment-count predicate tail (predicate: '{}')",
                            filtered.join(" ")
                        )))
                    }
                })?;
                return Ok(PredicateAst::SourceHasAttachmentsMatching {
                    filter,
                    comparison,
                    display: filtered.join(" "),
                });
            }
        }
    }

    let source_filter_predicate = {
        let predicate_idx = find_index(&filtered, |word| {
            SOURCE_FILTER_STATE_WORD_PATTERN.matches_word(word)
        });
        predicate_idx.and_then(|idx| {
            let subject_words = &filtered[..idx];
            let is_source_subject = is_source_reference_words(subject_words);
            if !is_source_subject {
                return None;
            }

            let mut negative = NEGATED_STATE_WORD_PATTERN.matches_word(filtered[idx]);
            let mut tail_start = idx + 1;
            if filtered
                .get(tail_start)
                .is_some_and(|word| NOT_WORD_PATTERN.matches_word(word))
            {
                negative = true;
                tail_start += 1;
            }
            let descriptor_words = &filtered[tail_start..];
            if descriptor_words.is_empty()
                || descriptor_words
                    .iter()
                    .any(|word| SOURCE_FILTER_IGNORED_DESCRIPTOR_WORD_PATTERN.matches_word(word))
            {
                return None;
            }

            let descriptor_tokens =
                crate::runtime_backend::lexer::synthetic_word_tokens(descriptor_words);
            let Ok(filter) = parse_object_filter(&descriptor_tokens, false) else {
                return None;
            };
            let has_identity = !filter.card_types.is_empty()
                || !filter.all_card_types.is_empty()
                || !filter.subtypes.is_empty()
                || !filter.supertypes.is_empty()
                || filter.colors.is_some()
                || filter.token
                || filter.nontoken
                || !filter.excluded_card_types.is_empty()
                || !filter.excluded_subtypes.is_empty();
            has_identity.then_some((filter, negative))
        })
    };
    if let Some((filter, negative)) = source_filter_predicate {
        let predicate = PredicateAst::SourceMatches(filter);
        return Ok(if negative {
            PredicateAst::Not(Box::new(predicate))
        } else {
            predicate
        });
    }

    if let Some(has_idx) = find_index(&filtered, |word| {
        HAS_OR_HAVE_WORD_PATTERN.matches_word(word)
    }) && has_idx > 0
        && has_idx + 1 < filtered.len()
    {
        let subject_words = &filtered[..has_idx];
        let is_source_subject = is_source_reference_words(subject_words)
            || SOURCE_REFERENCE_WORD_PATTERN.matches_words(subject_words);
        if is_source_subject
            && let Some((constraint, consumed)) =
                parse_filter_keyword_constraint_words(&filtered[has_idx + 1..])
            && has_idx + 1 + consumed == filtered.len()
        {
            let mut filter = ObjectFilter::default();
            apply_filter_keyword_constraint(&mut filter, constraint, false);
            return Ok(PredicateAst::SourceMatches(filter));
        }
    }

    if let Some(predicate) = parse_there_are_no_source_counter_predicate(&raw_words) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_has_counter_predicate(&raw_words) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_triggering_object_counter_predicate(&raw_words) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_there_are_source_counter_predicate(&raw_words) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_power_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_basic_land_types_among_lands_predicate(&filtered) {
        return Ok(predicate);
    }

    if filtered.len() >= 7
        && THERE_ARE_PREFIX_PATTERN.matches_words(&filtered)
        && let Some((count, idx)) = predicate_at_least_quantity_prefix(&filtered[2..])
            .map(|(count, used)| (count, 2 + used))
    {
        let battlefield_suffix_len =
            if ON_BATTLEFIELD_SUFFIX_PATTERN.matches_words(&filtered[idx..]) {
                if ON_THE_BATTLEFIELD_SUFFIX_PATTERN.matches_words(&filtered) {
                    Some(3usize)
                } else {
                    Some(2usize)
                }
            } else {
                None
            };
        if let Some(battlefield_suffix_len) = battlefield_suffix_len {
            let raw_filter_words = &filtered[idx..filtered.len() - battlefield_suffix_len];
            let other = raw_filter_words
                .first()
                .is_some_and(|word| OTHER_OR_ANOTHER_WORD_PATTERN.matches_word(word));
            let filter_words = if other {
                &raw_filter_words[1..]
            } else {
                raw_filter_words
            };
            if !filter_words.is_empty() {
                let filter_tokens =
                    crate::runtime_backend::lexer::synthetic_word_tokens(filter_words);
                if let Ok(mut filter) = parse_object_filter(&filter_tokens, other) {
                    filter.zone = Some(Zone::Battlefield);

                    return Ok(PredicateAst::ValueComparison {
                        left: Value::Count(filter),
                        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(count as i32),
                    });
                }
            }
        }
    }

    if let Some(predicate) = parse_graveyard_card_types_predicate(&filtered) {
        return Ok(predicate);
    }

    let parse_comparison_player_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
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
    };
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
    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered
            .get(subject_len)
            .is_some_and(|word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))
        && let Some((comparison, used)) = predicate_quantity_prefix(&filtered[subject_len + 1..])
        && let Some((operator, count)) = comparison_to_value_comparison_operator(comparison)
        && filtered
            .get(subject_len + 1 + used)
            .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
        && filtered
            .get(subject_len + 2 + used)
            .is_some_and(|word| IN_WORD_PATTERN.matches_word(word))
        && let Some(possessive) = filtered.get(subject_len + 3 + used).copied()
        && graveyard_possessive_matches_subject(player, possessive)
        && filtered
            .get(subject_len + 4 + used)
            .is_some_and(|word| GRAVEYARD_WORD_PATTERN.matches_word(word))
        && filtered.len() == subject_len + 5 + used
        && let Some(player_filter) = player_filter_for_turn_value(player)
    {
        return Ok(PredicateAst::ValueComparison {
            left: Value::CardsInGraveyard(player_filter),
            operator,
            right: Value::Fixed(count),
        });
    }
    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered
            .get(subject_len)
            .is_some_and(|word| CONTROL_OR_CONTROLS_WORD_PATTERN.matches_word(word))
        && filtered
            .get(subject_len + 1)
            .is_some_and(|word| MORE_WORD_PATTERN.matches_word(word))
        && let Some(than_offset) = find_index(&filtered[subject_len + 2..], |word| {
            THAN_WORD_PATTERN.matches_word(word)
        })
    {
        let than_idx = subject_len + 2 + than_offset;
        let tail = &filtered[than_idx..];
        if THAN_YOU_TAIL_PATTERN.matches_words(tail) {
            let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(
                &filtered[subject_len + 2..than_idx],
            );
            if !filter_tokens.is_empty() {
                let other = filter_tokens
                    .first()
                    .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
                if let Ok(filter) = parse_object_filter(&filter_tokens, other)
                    && filter != ObjectFilter::default()
                {
                    return Ok(PredicateAst::PlayerControlsMoreThanYou { player, filter });
                }
            }
        }
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

    if matches!(
        filtered.as_slice(),
        ["it", "your", "turn"] | ["its", "your", "turn"] | ["your", "turn"]
    ) {
        return Ok(PredicateAst::YourTurn);
    }

    if matches!(
        filtered.as_slice(),
        ["it", "not", "your", "turn"]
            | ["its", "not", "your", "turn"]
            | ["it", "is", "not", "your", "turn"]
            | ["its", "is", "not", "your", "turn"]
            | ["not", "your", "turn"]
    ) {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::YourTurn)));
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

    if let Some(predicate) = parse_source_cast_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_cast_payment_state_predicate(&filtered) {
        return Ok(predicate);
    }
    if filtered.len() == 4
        && ARTICLE_WORD_PATTERN.matches_word(filtered[0])
        && parse_subtype_word(filtered[1]).is_some()
        && WAS_OR_WERE_WORD_PATTERN.matches_word(filtered[2])
        && BEHELD_WORD_PATTERN.matches_word(filtered[3])
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if filtered.len() == 3
        && parse_subtype_word(filtered[0]).is_some()
        && WAS_OR_WERE_WORD_PATTERN.matches_word(filtered[1])
        && BEHELD_WORD_PATTERN.matches_word(filtered[2])
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if filtered.len() >= 4
        && COST_WAS_PAID_TAIL_PATTERN.matches_words(&filtered[filtered.len() - 3..])
    {
        let start = usize::from(DEFINITE_ARTICLE_WORD_PATTERN.matches_word_at(&filtered, 0));
        if let Some(label) = mana_cost_label_from_words(&filtered[start..filtered.len() - 3]) {
            return Ok(PredicateAst::ThisSpellPaidLabel(label));
        }
    }
    if filtered.len() >= 4
        && COST_WASNT_PAID_TAIL_PATTERN.matches_words(&filtered[filtered.len() - 3..])
    {
        let start = usize::from(DEFINITE_ARTICLE_WORD_PATTERN.matches_word_at(&filtered, 0));
        if let Some(label) = mana_cost_label_from_words(&filtered[start..filtered.len() - 3]) {
            return Ok(PredicateAst::Not(Box::new(
                PredicateAst::ThisSpellPaidLabel(label),
            )));
        }
    }
    if filtered.len() >= 5
        && COST_WAS_NOT_PAID_TAIL_PATTERN.matches_words(&filtered[filtered.len() - 4..])
    {
        let start = usize::from(DEFINITE_ARTICLE_WORD_PATTERN.matches_word_at(&filtered, 0));
        if let Some(label) = mana_cost_label_from_words(&filtered[start..filtered.len() - 4]) {
            return Ok(PredicateAst::Not(Box::new(
                PredicateAst::ThisSpellPaidLabel(label),
            )));
        }
    }
    if filtered.len() == 6
        && THIS_POSSESSIVE_PAID_LABEL_PATTERN.matches_words(&filtered)
        && THIS_POSSESSIVE_PAID_SUBJECT_WORD_PATTERN.matches_word(filtered[1])
    {
        let mut chars = filtered[2].chars();
        let Some(first) = chars.next() else {
            return Err(CardTextError::ParseError(
                "missing paid-cost label in predicate".to_string(),
            ));
        };
        let label = format!(
            "{}{}",
            first.to_ascii_uppercase(),
            chars.as_str().to_ascii_lowercase()
        );
        return Ok(PredicateAst::ThisSpellPaidLabel(label));
    }
    if let Some(predicate) = parse_spell_context_predicate(&filtered) {
        return Ok(predicate);
    }
    if filtered.len() == 7
        && MANA_SYMBOL_WORD_PATTERN.matches_word(filtered[0])
        && MANA_SPENT_TO_CAST_THIS_SPELL_TAIL_PATTERN.matches_words(&filtered[1..])
        && let Ok(symbol) = parse_mana_symbol(filtered[0])
    {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(symbol),
        });
    }
    if filtered.len() >= 8
        && MANA_SPENT_TO_CAST_THIS_SPELL_TAIL_PATTERN.matches_words(&filtered[filtered.len() - 6..])
        && filtered[..filtered.len() - 6]
            .iter()
            .all(|word| MANA_SYMBOL_WORD_PATTERN.matches_word(word))
    {
        let mut predicates = filtered[..filtered.len() - 6]
            .iter()
            .filter_map(|word| parse_mana_symbol(word).ok())
            .map(|symbol| PredicateAst::ManaSpentToCastThisSpellAtLeast {
                amount: 1,
                symbol: Some(symbol),
            });
        if let Some(first) = predicates.next() {
            return Ok(predicates.fold(first, |left, right| {
                PredicateAst::And(Box::new(left), Box::new(right))
            }));
        }
    }

    if let Some(amount) = parse_same_color_mana_spent_to_cast_predicate(&filtered) {
        return Ok(PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(
            amount,
        ));
    }

    if let Some((amount, symbol)) = parse_mana_spent_to_cast_predicate(&filtered) {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol });
    }

    if filtered.len() >= 5
        && matches!(
            filtered.as_slice(),
            ["this", "permanent", "attached", "to", ..]
                | ["that", "permanent", "attached", "to", ..]
                | ["this", "permanent", "is", "attached", "to", ..]
                | ["that", "permanent", "is", "attached", "to", ..]
        )
    {
        let attached_start = if IS_OR_ARE_WORD_PATTERN.matches_word_at(&filtered, 2) {
            5
        } else {
            4
        };
        let attached_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[attached_start..]);
        let mut filter = parse_object_filter(&attached_tokens, false)?;
        if filter.card_types.is_empty() {
            filter.card_types.push(CardType::Creature);
        }
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from("enchanted"),
            filter,
        ));
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

    if matches!(
        filtered.as_slice(),
        ["any", "of", "those", "cards", "remain", "exiled"]
            | ["those", "cards", "remain", "exiled"]
            | ["that", "card", "remains", "exiled"]
            | ["it", "remains", "exiled"]
    ) {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter::default().in_zone(Zone::Exile),
        ));
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

    let is_it_soulbond_paired = matches!(
        filtered.as_slice(),
        ["it", "paired", "with", "creature"]
            | ["it", "paired", "with", "another", "creature"]
            | ["it", "s", "paired", "with", "creature"]
            | ["it", "s", "paired", "with", "another", "creature"]
    );
    if is_it_soulbond_paired {
        return Ok(PredicateAst::ItIsSoulbondPaired);
    }

    if filtered.len() >= 2 {
        let tag = if EQUIPPED_CREATURE_PREFIX_PATTERN.matches_words(&filtered) {
            Some("equipped")
        } else if ENCHANTED_CREATURE_PREFIX_PATTERN.matches_words(&filtered) {
            Some("enchanted")
        } else {
            None
        };
        if let Some(tag) = tag {
            let remainder = filtered[2..].to_vec();
            let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(remainder);
            let mut filter = parse_object_filter(&tokens, false)?;
            if filter.card_types.is_empty() {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::TaggedMatches(TagKey::from(tag), filter));
        }
    }

    let onto_battlefield_idx = ONTO_BATTLEFIELD_PATTERN.find_exact_window_range(&filtered, 2, 3);
    if filtered.len() >= 7
        && YOU_WORD_PATTERN.matches_word_at(&filtered, 0)
        && PUT_WORD_PATTERN.matches_word_at(&filtered, 1)
        && THIS_WAY_SUFFIX_PATTERN.matches_words(&filtered)
        && let Some(onto_idx) = onto_battlefield_idx
    {
        let filter_words = &filtered[2..onto_idx];
        let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filter_words);
        let mut filter = parse_object_filter(&filter_tokens, false)?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        });
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

    if let Some(predicate) = parse_negative_player_controls_predicate(&filtered)? {
        return Ok(predicate);
    }

    if filtered.len() >= 7
        && YOU_CONTROL_PREFIX_PATTERN.matches_words(&filtered)
        && let Some(or_idx) = find_index(&filtered, |word| OR_WORD_PATTERN.matches_word(word))
        && or_idx > 2
    {
        let left_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[2..or_idx]);
        let mut right_words = filtered[or_idx + 1..].to_vec();
        if right_words
            .first()
            .is_some_and(|word| THERE_WORD_PATTERN.matches_word(word))
        {
            right_words = right_words[1..].to_vec();
        }
        if YOUR_GRAVEYARD_WORDS_PATTERN.matches_words(&right_words) {
            let right_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(right_words);
            if let (Ok(mut control_filter), Ok(mut graveyard_filter)) = (
                parse_object_filter(&left_tokens, false),
                parse_object_filter(&right_tokens, false),
            ) {
                control_filter.controller = Some(PlayerFilter::You);
                if graveyard_filter.zone.is_none() {
                    graveyard_filter.zone = Some(Zone::Graveyard);
                }
                if graveyard_filter.owner.is_none() {
                    graveyard_filter.owner = Some(PlayerFilter::You);
                }
                return Ok(PredicateAst::PlayerControlsOrHasCardInGraveyard {
                    player: PlayerAst::You,
                    control_filter,
                    graveyard_filter,
                });
            }
        }
    }

    if filtered.len() >= 3 && YOU_CONTROL_PREFIX_PATTERN.matches_words(&filtered) {
        if let Some(and_idx) =
            find_index(&filtered[2..], |word| AND_WORD_PATTERN.matches_word(word))
        {
            let and_idx = 2 + and_idx;
            if and_idx > 2 && and_idx + 1 < filtered.len() {
                let left_tokens =
                    crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[2..and_idx]);
                let right_tokens =
                    crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[and_idx + 1..]);
                if let (Ok(mut left_filter), Ok(mut right_filter)) = (
                    parse_object_filter(&left_tokens, false),
                    parse_object_filter(&right_tokens, false),
                ) {
                    left_filter.controller = Some(PlayerFilter::You);
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

    if let Some(predicate) = parse_tagged_object_lifecycle_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_tagged_battlefield_this_way_predicate(&filtered)? {
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

    if let Some(predicate) = parse_global_state_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_dealt_combat_damage_to_player_this_turn(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_was_dealt_combat_damage_by_subtype_this_turn(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_spell_cast_this_turn_predicate(&filtered) {
        return Ok(predicate);
    }

    if filtered.len() >= 4
        && filtered.first() == Some(&"x")
        && filtered.get(1) == Some(&"is")
        && let Some((comparison, used)) = predicate_quantity_prefix(&filtered[2..])
        && used + 2 == filtered.len()
        && let Some((operator, amount)) = comparison_to_value_comparison_operator(comparison)
    {
        return Ok(PredicateAst::ValueComparison {
            left: Value::X,
            operator,
            right: Value::Fixed(amount),
        });
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
    fn parse_predicate_accepts_unapostrophed_spell_paid_label() -> Result<(), CardTextError> {
        let tokens = lex_line("If this spells surge cost was paid", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ThisSpellPaidLabel("Surge".to_string())
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_accepts_paid_label_with_trailing_instead_effect_tail()
    -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If this creature's spectacle cost was paid instead discard your hand",
            0,
        )?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ThisSpellPaidLabel("Spectacle".to_string())
        );
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
    fn parse_predicate_supports_you_or_player_youre_attacking_has_initiative()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If you or a player you're attacking has the initiative", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Or(
                Box::new(PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::Defending,
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
    fn parse_predicate_global_states_use_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If no creatures are on the battlefield",
                PredicateAst::PlayerControlsNo {
                    player: PlayerAst::Any,
                    filter: ObjectFilter::creature(),
                },
            ),
            ("If it's night", PredicateAst::ItIsNight),
            (
                "If it is the first combat phase of the turn",
                PredicateAst::FirstCombatPhaseOfTurn,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

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
    fn parse_predicate_supports_its_night() -> Result<(), CardTextError> {
        let tokens = lex_line("If it's night", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, PredicateAst::ItIsNight);
        Ok(())
    }

    #[test]
    fn parse_predicate_accepts_first_combat_phase_of_turn() -> Result<(), CardTextError> {
        let tokens = lex_line("If it's the first combat phase of the turn", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, PredicateAst::FirstCombatPhaseOfTurn);
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
        for text in [
            "If you don't put the card into your hand",
            "If you didnt put card into your hand",
            "If you did not put it into your hand",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                    filter: ObjectFilter::default().in_zone(Zone::Hand),
                })),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_tagged_lifecycle_uses_capture_parser() -> Result<(), CardTextError> {
        for text in [
            "if you controlled that permanent",
            "if you control that permanent",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                    filter: ObjectFilter::default(),
                },
                "{text}"
            );
        }

        for text in [
            "if it entered under your control",
            "if that card entered under your control",
            "if that permanent entered under your control",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn {
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                },
                "{text}"
            );
        }

        for text in [
            "if you dont put that card onto battlefield",
            "if you didnt put it onto battlefield",
            "if you did not put the card onto battlefield",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                    filter: ObjectFilter::default().in_zone(Zone::Battlefield),
                })),
                "{text}"
            );
        }

        for text in [
            "if it wasnt blocking",
            "if it was not blocking",
            "if that creature wasnt blocking",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::TaggedMatches(
                    TagKey::from(IT_TAG),
                    ObjectFilter {
                        nonblocking: true,
                        ..Default::default()
                    }
                ),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_opponent_controls_reference_uses_capture_parser() -> Result<(), CardTextError>
    {
        for text in [
            "if an opponent controls it",
            "if opponent controls it",
            "if an opponent controls that permanent",
            "if opponent controls that permanent",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::ItMatches(ObjectFilter {
                    controller: Some(PlayerFilter::Opponent),
                    ..Default::default()
                }),
                "{text}"
            );
        }

        for text in [
            "if an opponent controls that creature",
            "if opponent controls that creature",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::ItMatches(ObjectFilter {
                    controller: Some(PlayerFilter::Opponent),
                    card_types: vec![CardType::Creature],
                    ..Default::default()
                }),
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_source_exploited_triggering_object_uses_capture_parser()
    -> Result<(), CardTextError> {
        let expected = PredicateAst::And(
            Box::new(PredicateAst::TaggedMatches(
                TagKey::from(crate::tag::EXPLOITED_TAG),
                ObjectFilter::tagged("triggering"),
            )),
            Box::new(PredicateAst::TaggedMatches(
                TagKey::from(crate::tag::EXPLOITER_TAG),
                ObjectFilter::source(),
            )),
        );
        for text in [
            "if it exploited that creature",
            "if it exploited that object",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_vote_results_use_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "if return gets more votes",
                PredicateAst::VoteOptionGetsMoreVotes {
                    option: "return".to_string(),
                },
            ),
            (
                "if embark gets more votes or vote is tied",
                PredicateAst::VoteOptionGetsMoreVotesOrTied {
                    option: "embark".to_string(),
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }

        let tokens = lex_line("if no creatures got votes", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);
        let parsed = parse_predicate(&predicate_tokens)?;
        assert_eq!(
            parsed,
            PredicateAst::NoVoteObjectsMatched {
                filter: ObjectFilter::creature(),
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_negative_control_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "if you control no creatures",
                PredicateAst::PlayerControlsNo {
                    player: PlayerAst::You,
                    filter: ObjectFilter {
                        controller: Some(PlayerFilter::You),
                        ..ObjectFilter::creature()
                    },
                },
            ),
            (
                "if player controls no artifacts",
                PredicateAst::PlayerControlsNo {
                    player: PlayerAst::Any,
                    filter: ObjectFilter {
                        controller: Some(PlayerFilter::Any),
                        card_types: vec![CardType::Artifact],
                        ..Default::default()
                    },
                },
            ),
            (
                "if you don't control another creature",
                PredicateAst::PlayerControlsNo {
                    player: PlayerAst::You,
                    filter: ObjectFilter {
                        controller: Some(PlayerFilter::You),
                        ..ObjectFilter::creature()
                    },
                },
            ),
            (
                "if you do not control artifacts",
                PredicateAst::PlayerControlsNo {
                    player: PlayerAst::You,
                    filter: ObjectFilter {
                        controller: Some(PlayerFilter::You),
                        card_types: vec![CardType::Artifact],
                        ..Default::default()
                    },
                },
            ),
            (
                "if you control neither creature",
                PredicateAst::PlayerControlsNo {
                    player: PlayerAst::You,
                    filter: ObjectFilter {
                        controller: Some(PlayerFilter::You),
                        ..ObjectFilter::creature().match_tagged(
                            TagKey::from(IT_TAG),
                            TaggedOpbjectRelation::IsTaggedObject,
                        )
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
    fn parse_predicate_triggering_object_counters_use_capture_parser() -> Result<(), CardTextError>
    {
        for (text, expected) in [
            (
                "if it had no stun counter on it",
                PredicateAst::TriggeringObjectHadNoCounter(CounterType::Stun),
            ),
            (
                "if that creature had no time counter on itself",
                PredicateAst::TriggeringObjectHadNoCounter(CounterType::Time),
            ),
            (
                "if this permanent had a stun counter on that",
                PredicateAst::TriggeringObjectHadCounterAtLeast {
                    counter_type: CounterType::Stun,
                    count: 1,
                },
            ),
            (
                "if it had time counters on them",
                PredicateAst::TriggeringObjectHadCounterAtLeast {
                    counter_type: CounterType::Time,
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
    fn parse_predicate_source_counters_use_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "if this has no stun counter on it",
                PredicateAst::SourceHasNoCounter(CounterType::Stun),
            ),
            (
                "if this creature has a time counter on it",
                PredicateAst::SourceHasCounterAtLeast {
                    counter_type: CounterType::Time,
                    count: 1,
                },
            ),
            (
                "if this permanent has three or more stun counters on them",
                PredicateAst::SourceHasCounterAtLeast {
                    counter_type: CounterType::Stun,
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
    fn parse_predicate_there_are_source_counters_uses_capture_parser() -> Result<(), CardTextError>
    {
        for (text, expected) in [
            (
                "if there are three or more counters on it",
                PredicateAst::SourceHasCountersAtLeast(3),
            ),
            (
                "if there are two or more stun counters on this creature",
                PredicateAst::SourceHasCounterAtLeast {
                    counter_type: CounterType::Stun,
                    count: 2,
                },
            ),
            (
                "if there are five time counters on this permanent",
                PredicateAst::SourceHasCounterAtLeast {
                    counter_type: CounterType::Time,
                    count: 5,
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
    fn parse_predicate_there_are_no_source_counters_uses_capture_parser()
    -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "if there are no more scream counters on it",
                PredicateAst::SourceHasNoCounter(CounterType::Named("scream")),
            ),
            (
                "if there are no time counters on them",
                PredicateAst::SourceHasNoCounter(CounterType::Time),
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
    fn parse_predicate_basic_land_types_among_lands_uses_capture_parser()
    -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "if there are three or more basic land types among lands you control",
                PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore {
                    player: PlayerAst::You,
                    count: 3,
                },
            ),
            (
                "if there are five basic land type among land that player controls",
                PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore {
                    player: PlayerAst::That,
                    count: 5,
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
    fn parse_predicate_graveyard_card_types_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "if there are four or more card types among cards in your graveyard",
                PredicateAst::PlayerHasCardTypesInGraveyardOrMore {
                    player: PlayerAst::You,
                    count: 4,
                },
            ),
            (
                "if you have four or more card types among cards in your graveyard",
                PredicateAst::PlayerHasCardTypesInGraveyardOrMore {
                    player: PlayerAst::You,
                    count: 4,
                },
            ),
            (
                "if there are three card type among card in target opponent graveyard",
                PredicateAst::PlayerHasCardTypesInGraveyardOrMore {
                    player: PlayerAst::TargetOpponent,
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
    fn parse_predicate_source_combat_damage_to_player_uses_capture_parser()
    -> Result<(), CardTextError> {
        for text in [
            "if it dealt combat damage to a player this turn",
            "if it dealt combat damage to player this turn",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::SourceDealtCombatDamageToPlayerThisTurn,
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_player_combat_damage_by_subtype_uses_capture_parser()
    -> Result<(), CardTextError> {
        for (text, player) in [
            (
                "if a player was dealt combat damage by Zombie this turn",
                PlayerAst::Any,
            ),
            (
                "if player was dealt combat damage by Zombie this turn",
                PlayerAst::Any,
            ),
            (
                "if an opponent was dealt combat damage by Zombie this turn",
                PlayerAst::Opponent,
            ),
            (
                "if opponent was dealt combat damage by Zombie this turn",
                PlayerAst::Opponent,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
                    player,
                    subtype: crate::types::Subtype::Zombie,
                },
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_source_didnt_attack_or_enter_control_uses_capture_parser()
    -> Result<(), CardTextError> {
        let expected = PredicateAst::And(
            Box::new(PredicateAst::Not(Box::new(
                PredicateAst::SourceAttackedThisTurn,
            ))),
            Box::new(PredicateAst::Not(Box::new(
                PredicateAst::SourceCameUnderYourControlThisTurn,
            ))),
        );
        for text in [
            "if this creature didnt attack or come under your control this turn",
            "if this creature didnt attack or came under your control this turn",
            "if this creature did not attack or come under your control this turn",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_cast_this_spell_during_your_main_phase()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If you cast this spell during your main phase", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ThisSpellPaidLabel("CastDuringYourMainPhase".to_string())
        );
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
    fn parse_predicate_you_put_object_onto_battlefield_this_way_uses_capture_parser()
    -> Result<(), CardTextError> {
        for text in [
            "If you put an artifact onto the battlefield this way",
            "If you put an artifact onto battlefield this way",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;
            let artifact_filter_tokens = lex_line("an artifact", 0)?;
            let artifact_filter = parse_object_filter(&artifact_filter_tokens, false)?;

            assert_eq!(
                parsed,
                PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                    filter: artifact_filter,
                },
                "{text}"
            );
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_equipment_is_put_onto_the_battlefield_this_way()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If an Equipment is put onto the battlefield this way", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;
        let equipment_filter_tokens = lex_line("an Equipment", 0)?;
        let equipment_filter = parse_object_filter(&equipment_filter_tokens, false)?;

        assert_eq!(
            parsed,
            PredicateAst::TaggedMatches(TagKey::from(IT_TAG), equipment_filter)
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_aura_is_put_onto_the_battlefield_this_way()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If an Aura is put onto the battlefield this way", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;
        let aura_filter_tokens = lex_line("an Aura", 0)?;
        let aura_filter = parse_object_filter(&aura_filter_tokens, false)?;

        assert_eq!(
            parsed,
            PredicateAst::TaggedMatches(TagKey::from(IT_TAG), aura_filter)
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
    fn parse_predicate_supports_would_draw_while_no_cards_in_hand() -> Result<(), CardTextError> {
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
    fn parse_predicate_ring_state_uses_capture_parser() -> Result<(), CardTextError> {
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
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_source_status_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            ("If this tapped", PredicateAst::SourceIsTapped),
            (
                "If this creature is untapped",
                PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)),
            ),
            ("If it is saddled", PredicateAst::SourceIsSaddled),
            (
                "If this permanent isnt saddled",
                PredicateAst::Not(Box::new(PredicateAst::SourceIsSaddled)),
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_source_cast_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            ("If you cast it", PredicateAst::SourceWasCast),
            (
                "If that creature was cast",
                PredicateAst::TaggedWasCast(TagKey::from(IT_TAG)),
            ),
            (
                "If this spell was cast from the graveyard",
                PredicateAst::ThisSpellWasCastFromZone(Zone::Graveyard),
            ),
            (
                "If this spell was cast from anywhere other than your hand",
                PredicateAst::ThisSpellWasCastFromNonHand,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_cast_payment_state_uses_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If no spells were cast last turn",
                PredicateAst::NoSpellsWereCastLastTurn,
            ),
            ("If this spell was kicked", PredicateAst::ThisSpellWasKicked),
            ("If it was kicked", PredicateAst::ThisSpellWasKicked),
            ("If that was kicked", PredicateAst::TargetWasKicked),
            (
                "If it was bargained",
                PredicateAst::ThisSpellPaidLabel("Bargain".to_string()),
            ),
            (
                "If gift wasnt promised",
                PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel(
                    "Gift".to_string(),
                ))),
            ),
            (
                "If tribute was paid",
                PredicateAst::ThisSpellPaidLabel("Tribute".to_string()),
            ),
            (
                "If tribute was not paid",
                PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel(
                    "Tribute".to_string(),
                ))),
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

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
    fn parse_predicate_supports_ring_bearer_temptation_gate() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If this is your Ring-bearer and the Ring has tempted you two or more times this game",
            0,
        )?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::And(
                Box::new(PredicateAst::SourceIsRingBearer {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::PlayerRingTemptedThisGameOrMore {
                    player: PlayerAst::You,
                    count: 2,
                })
            )
        );
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
    fn parse_predicate_supports_card_in_your_graveyard_existence() -> Result<(), CardTextError> {
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
    fn parse_predicate_supports_behold_or_controlled_subtype_as_cast() -> Result<(), CardTextError>
    {
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
    fn parse_predicate_supports_this_has_power_or_greater() -> Result<(), CardTextError> {
        for text in [
            "If this has power 7 or greater",
            "If this creature power is 7 or greater",
            "If this permanents power is 7 or greater",
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, PredicateAst::SourcePowerAtLeast(7), "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_source_has_keyword() -> Result<(), CardTextError> {
        let tokens = lex_line("If this creature has defender", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        let mut expected_filter = ObjectFilter::default();
        expected_filter
            .static_abilities
            .push(crate::static_abilities::StaticAbilityId::Defender);
        assert_eq!(parsed, PredicateAst::SourceMatches(expected_filter));
        Ok(())
    }
}

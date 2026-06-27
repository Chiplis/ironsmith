use super::*;
use crate::runtime_backend::front_end::lex_patterns::{
    LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom, LexPatternMatch,
};

const ALL_CREATURE_TYPES_SEQUENCE: &[LexPatternAtom<'static>] = &[LexPattern::phrase(&[
    "all", "creature", "types", "until", "end", "of", "turn",
])];
const EVERY_CREATURE_TYPE_SEQUENCE: &[LexPatternAtom<'static>] = &[LexPattern::phrase(&[
    "every", "creature", "type", "until", "end", "of", "turn",
])];
const CREATURE_TYPE_TAIL_SEQUENCES: &[&[LexPatternAtom<'static>]] =
    &[ALL_CREATURE_TYPES_SEQUENCE, EVERY_CREATURE_TYPE_SEQUENCE];
pub(crate) const GAINS_OR_LOSES_ALL_CREATURE_TYPES_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(&[&["gain"], &["gains"], &["lose"], &["loses"]]),
    ),
    LexPattern::role_capture(
        "verb",
        LexCaptureRole::Action,
        LexCaptureKind::OneOf(&["gain", "gains", "lose", "loses"]),
    ),
    LexPattern::any_sequence(CREATURE_TYPE_TAIL_SEQUENCES),
];
const REPEAT_IF_WIN_SEQUENCE: &[LexPatternAtom<'static>] = &[LexPattern::phrase(&[
    "if", "you", "win", "repeat", "this", "process",
])];
pub(crate) const LOSE_DRAW_CLASH_REPEAT_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["you", "lose"]),
    LexPattern::role_capture("life", LexCaptureRole::Amount, LexCaptureKind::WordCount(1)),
    LexPattern::phrase(&["life", "and", "draw"]),
    LexPattern::capture("draw", LexCaptureKind::WordCount(1)),
    LexPattern::any_word(&["card", "cards"]),
    LexPattern::phrase(&["then", "clash", "with", "an", "opponent"]),
    LexPattern::optional(REPEAT_IF_WIN_SEQUENCE),
];
pub(crate) const DELAYED_NEXT_STEP_UNLESS_PAYS_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "effect",
        LexCaptureRole::Action,
        LexCaptureKind::UntilPhrase(&["unless"]),
    ),
    LexPattern::word("unless"),
    LexPattern::role_capture("payment", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
pub(crate) const SEARCH_DELAYED_NEXT_UPKEEP_LOSE_GAME_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("search"),
    LexPattern::role_capture(
        "search_effect",
        LexCaptureRole::Action,
        LexCaptureKind::UntilPhrase(&["at", "the", "beginning"]),
    ),
    LexPattern::phrase(&["at", "the", "beginning"]),
    LexPattern::role_capture(
        "upkeep_and_loss",
        LexCaptureRole::Tail,
        LexCaptureKind::Rest,
    ),
];
const DELAYED_LOSE_GAME_UNLESS_PAID_WORDS: &[&[&str]] = &[
    &["if", "you", "dont", "you", "lose", "the", "game"],
    &["if", "you", "do", "not", "you", "lose", "the", "game"],
    &["if", "you", "don't", "you", "lose", "the", "game"],
];
const DELAYED_PLAYER_PREFIX_YOU: &[&str] = &["you"];
const DELAYED_PLAYER_YOU_PHRASES: &[&[&str]] = &[DELAYED_PLAYER_PREFIX_YOU];
const DELAYED_PLAYER_TARGET_OPPONENT_PHRASES: &[&[&str]] =
    &[&["target", "opponent"], &["target", "opponents"]];
const DELAYED_PLAYER_TARGET_PHRASES: &[&[&str]] = &[&["target", "player"], &["target", "players"]];
const DELAYED_PLAYER_ANY_PHRASES: &[&[&str]] = &[&["any", "player"], &["any", "players"]];
const DELAYED_PLAYER_THEY_PHRASES: &[&[&str]] = &[&["they"]];
const DELAYED_PLAYER_DEFENDING_PHRASES: &[&[&str]] =
    &[&["defending", "player"], &["defending", "players"]];
const DELAYED_PLAYER_THAT_PHRASES: &[&[&str]] = &[&["that", "player"], &["that", "players"]];
const DELAYED_PLAYER_ITS_CONTROLLER_PHRASES: &[&[&str]] =
    &[&["its", "controller"], &["their", "controller"]];
const DELAYED_PLAYER_ITS_OWNER_PHRASES: &[&[&str]] = &[&["its", "owner"], &["their", "owner"]];
const DELAYED_PAY_OR_PAYS_WORDS: &[&str] = &["pay", "pays"];
const DELAYED_DRAW_OR_DRAWS_WORDS: &[&str] = &["draw", "draws"];
const DELAYED_DISCARD_OR_DISCARDS_WORDS: &[&str] = &["discard", "discards"];
const DELAYED_SACRIFICE_OR_SACRIFICES_WORDS: &[&str] = &["sacrifice", "sacrifices"];
const DELAYED_REFERRED_OBJECT_NOUN_WORDS: &[&str] = &[
    "ability",
    "abilitys",
    "card",
    "cards",
    "creature",
    "creatures",
    "object",
    "objects",
    "permanent",
    "permanents",
    "planeswalker",
    "planeswalkers",
    "source",
    "sources",
    "spell",
    "spells",
];
const DELAYED_REFERRED_PERMANENT_NOUN_WORDS: &[&str] = &[
    "card",
    "cards",
    "creature",
    "creatures",
    "object",
    "objects",
    "permanent",
    "permanents",
    "planeswalker",
    "planeswalkers",
    "source",
    "sources",
    "spell",
    "spells",
];
const DELAYED_CONTROLLER_OR_CONTROLLERS_WORDS: &[&str] = &["controller", "controllers"];
const DELAYED_OWNER_OR_OWNERS_WORDS: &[&str] = &["owner", "owners"];
const DELAYED_MECHANIC_CHOOSE_ONE_OF_THEM_WORDS: &[&str] = &["you", "choose", "one", "of", "them"];
const DELAYED_VENTURE_DUNGEON_WORDS: &[&str] = &["venture", "into", "the", "dungeon"];
const DELAYED_STILL_LAND_WORDS: &[&[&str]] = &[
    &["its", "still", "a", "land"],
    &["it", "still", "a", "land"],
];
const DELAYED_CAST_OR_PLAY_WORDS: &[&str] = &[
    "may", "cast", "casts", "casting", "play", "plays", "playing", "played",
];
const DELAYED_REMAINS_WORDS: &[&str] = &["remains"];
const DELAYED_TAPPED_WORDS: &[&str] = &["tapped"];
const DELAYED_MANA_COST_WORDS: &[&str] = &["cost", "costs"];
const DELAYED_STILL_WORDS: &[&str] = &["still"];
const DELAYED_NEGATED_BE_PREFIXES: &[&[&str]] = &[&["is", "not"], &["are", "not"]];
const DELAYED_CONTRACTION_NEGATED_BE_WORDS: &[&str] = &["isnt", "isn't", "arent", "aren't"];
const DELAYED_BE_WORDS: &[&str] = &["is", "are", "s", "’s"];
const DELAYED_UNTIL_END_OF_TURN_TAIL: &[&str] = &["until", "end", "of", "turn"];
const DELAYED_NOT_ARTICLE_PREFIXES: &[&[&str]] = &[&["not", "a"], &["not", "an"]];
const DELAYED_NOT_PREFIX: &[&str] = &["not"];
const DELAYED_ARTICLE_WORDS: &[&str] = &["a", "an", "the"];
const DELAYED_GET_WORDS: &[&str] = &["get", "gets"];
const DELAYED_ADDITION_OTHER_TYPES_TAILS: &[&[&str]] = &[
    &["in", "addition", "to", "its", "other", "types"],
    &["in", "addition", "to", "their", "other", "types"],
    &["in", "addition", "to", "its", "other", "type"],
    &["in", "addition", "to", "their", "other", "type"],
];
const DELAYED_IT_OR_THAT_CREATURE_PHRASES: &[&[&str]] = &[&["it"], &["that", "creature"]];
const DELAYED_TAGGED_CREATURE_REFERENCE_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "reference",
        LexCaptureKind::OneOfPhrase(DELAYED_IT_OR_THAT_CREATURE_PHRASES),
    )]);
const DELAYED_THAT_PLAYER_OR_OBJECT_CONTROLLER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["that", "player", "or", "that"]),
    LexPattern::object(
        "object",
        LexCaptureKind::OneOf(DELAYED_REFERRED_OBJECT_NOUN_WORDS),
    ),
    LexPattern::subject(
        "controller",
        LexCaptureKind::OneOf(DELAYED_CONTROLLER_OR_CONTROLLERS_WORDS),
    ),
]);
const DELAYED_THAT_OBJECT_CONTROLLER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("that"),
    LexPattern::object(
        "object",
        LexCaptureKind::OneOf(DELAYED_REFERRED_OBJECT_NOUN_WORDS),
    ),
    LexPattern::subject(
        "controller",
        LexCaptureKind::OneOf(DELAYED_CONTROLLER_OR_CONTROLLERS_WORDS),
    ),
]);
const DELAYED_THAT_OBJECT_OWNER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("that"),
    LexPattern::object(
        "object",
        LexCaptureKind::OneOf(DELAYED_REFERRED_OBJECT_NOUN_WORDS),
    ),
    LexPattern::subject(
        "owner",
        LexCaptureKind::OneOf(DELAYED_OWNER_OR_OWNERS_WORDS),
    ),
]);
const DELAYED_THAT_PERMANENT_CONTROLLER_OR_PLAYER_PATTERN: LexPattern<'static> =
    LexPattern::new(&[
        LexPattern::word("that"),
        LexPattern::object(
            "object",
            LexCaptureKind::OneOf(DELAYED_REFERRED_PERMANENT_NOUN_WORDS),
        ),
        LexPattern::subject(
            "controller",
            LexCaptureKind::OneOf(DELAYED_CONTROLLER_OR_CONTROLLERS_WORDS),
        ),
        LexPattern::phrase(&["or", "that", "player"]),
    ]);
const DELAYED_CAST_OR_PLAY_ACTION_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "action",
        LexCaptureKind::OneOf(DELAYED_CAST_OR_PLAY_WORDS),
    )]);
const DELAYED_MECHANIC_CHOOSE_ONE_OF_THEM_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(
        DELAYED_MECHANIC_CHOOSE_ONE_OF_THEM_WORDS,
    )]);
const DELAYED_VENTURE_DUNGEON_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(DELAYED_VENTURE_DUNGEON_WORDS)]);
const DELAYED_STILL_LAND_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::modifier(
    "still_land",
    LexCaptureKind::OneOfPhrase(DELAYED_STILL_LAND_WORDS),
)]);
const DELAYED_REMAINS_MARKER_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::condition(
        "remains",
        LexCaptureKind::OneOf(DELAYED_REMAINS_WORDS),
    )]);
const DELAYED_TAPPED_MARKER_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "tapped",
        LexCaptureKind::OneOf(DELAYED_TAPPED_WORDS),
    )]);
const DELAYED_PAY_ACTION_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "action",
    LexCaptureKind::OneOf(DELAYED_PAY_OR_PAYS_WORDS),
)]);
const DELAYED_DRAW_ACTION_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "action",
    LexCaptureKind::OneOf(DELAYED_DRAW_OR_DRAWS_WORDS),
)]);
const DELAYED_DISCARD_ACTION_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "action",
    LexCaptureKind::OneOf(DELAYED_DISCARD_OR_DISCARDS_WORDS),
)]);
const DELAYED_SACRIFICE_ACTION_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "action",
        LexCaptureKind::OneOf(DELAYED_SACRIFICE_OR_SACRIFICES_WORDS),
    )]);
const DELAYED_MANA_COST_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("mana"),
    LexPattern::object("cost", LexCaptureKind::OneOf(DELAYED_MANA_COST_WORDS)),
]);
const DELAYED_STILL_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::modifier(
    "still",
    LexCaptureKind::OneOf(DELAYED_STILL_WORDS),
)]);
const DELAYED_BE_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::condition(
    "be",
    LexCaptureKind::OneOf(DELAYED_BE_WORDS),
)]);
const DELAYED_NOT_ARTICLE_PREFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::condition(
        "not_article",
        LexCaptureKind::OneOfPhrase(DELAYED_NOT_ARTICLE_PREFIXES),
    )]);
const DELAYED_NOT_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::condition(
    "not",
    LexCaptureKind::OneOf(DELAYED_NOT_PREFIX),
)]);
const DELAYED_UNTIL_END_OF_TURN_SUFFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "duration",
        LexCaptureKind::OneOfPhrase(&[DELAYED_UNTIL_END_OF_TURN_TAIL]),
    )]);
const DELAYED_ADDITION_OTHER_TYPES_SUFFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "addition",
        LexCaptureKind::OneOfPhrase(DELAYED_ADDITION_OTHER_TYPES_TAILS),
    )]);

struct DelayedPlayerPrefixEntry {
    phrases: &'static [&'static [&'static str]],
    player: PlayerAst,
}

struct DelayedDynamicPlayerPrefixEntry {
    pattern: LexPattern<'static>,
    capture: &'static str,
    player: PlayerAst,
}

const DELAYED_STATIC_PLAYER_PREFIXES: &[DelayedPlayerPrefixEntry] = &[
    DelayedPlayerPrefixEntry {
        phrases: DELAYED_PLAYER_YOU_PHRASES,
        player: PlayerAst::You,
    },
    DelayedPlayerPrefixEntry {
        phrases: DELAYED_PLAYER_TARGET_OPPONENT_PHRASES,
        player: PlayerAst::TargetOpponent,
    },
    DelayedPlayerPrefixEntry {
        phrases: DELAYED_PLAYER_TARGET_PHRASES,
        player: PlayerAst::Target,
    },
    DelayedPlayerPrefixEntry {
        phrases: DELAYED_PLAYER_ANY_PHRASES,
        player: PlayerAst::Any,
    },
    DelayedPlayerPrefixEntry {
        phrases: DELAYED_PLAYER_THEY_PHRASES,
        player: PlayerAst::That,
    },
    DelayedPlayerPrefixEntry {
        phrases: DELAYED_PLAYER_DEFENDING_PHRASES,
        player: PlayerAst::Defending,
    },
    DelayedPlayerPrefixEntry {
        phrases: DELAYED_PLAYER_THAT_PHRASES,
        player: PlayerAst::That,
    },
    DelayedPlayerPrefixEntry {
        phrases: DELAYED_PLAYER_ITS_CONTROLLER_PHRASES,
        player: PlayerAst::ItsController,
    },
    DelayedPlayerPrefixEntry {
        phrases: DELAYED_PLAYER_ITS_OWNER_PHRASES,
        player: PlayerAst::ItsOwner,
    },
];

const DELAYED_DYNAMIC_PLAYER_PREFIXES: &[DelayedDynamicPlayerPrefixEntry] = &[
    DelayedDynamicPlayerPrefixEntry {
        pattern: DELAYED_THAT_PLAYER_OR_OBJECT_CONTROLLER_PATTERN,
        capture: "controller",
        player: PlayerAst::ThatPlayerOrTargetController,
    },
    DelayedDynamicPlayerPrefixEntry {
        pattern: DELAYED_THAT_OBJECT_CONTROLLER_PATTERN,
        capture: "controller",
        player: PlayerAst::ItsController,
    },
    DelayedDynamicPlayerPrefixEntry {
        pattern: DELAYED_THAT_OBJECT_OWNER_PATTERN,
        capture: "owner",
        player: PlayerAst::ItsOwner,
    },
    DelayedDynamicPlayerPrefixEntry {
        pattern: DELAYED_THAT_PERMANENT_CONTROLLER_OR_PLAYER_PATTERN,
        capture: "controller",
        player: PlayerAst::ThatPlayerOrTargetController,
    },
];

fn parse_delayed_static_player_prefix(words: &[&str]) -> Option<(PlayerAst, usize)> {
    DELAYED_STATIC_PLAYER_PREFIXES.iter().find_map(|entry| {
        let atoms = [LexPattern::subject(
            "player",
            LexCaptureKind::OneOfPhrase(entry.phrases),
        )];
        LexPattern::new(&atoms)
            .match_prefix_word_refs(words)
            .and_then(|matched| matched.capture_word_range("player"))
            .map(|range| (entry.player, range.end))
    })
}

fn parse_delayed_static_player_exact(words: &[&str]) -> Option<(PlayerAst, usize)> {
    let prefix = words;
    if word_slice_eq(prefix, DELAYED_PLAYER_PREFIX_YOU) {
        return Some((PlayerAst::You, DELAYED_PLAYER_PREFIX_YOU.len()));
    }
    DELAYED_STATIC_PLAYER_PREFIXES.iter().find_map(|entry| {
        let atoms = [LexPattern::subject(
            "player",
            LexCaptureKind::OneOfPhrase(entry.phrases),
        )];
        LexPattern::new(&atoms)
            .match_word_refs(words)
            .and_then(|matched| matched.capture_word_range("player"))
            .map(|range| (entry.player, range.end))
    })
}

fn parse_delayed_dynamic_player_prefix(words: &[&str]) -> Option<(PlayerAst, usize)> {
    DELAYED_DYNAMIC_PLAYER_PREFIXES.iter().find_map(|entry| {
        entry
            .pattern
            .match_prefix_word_refs(words)
            .filter(|matched| matched.capture_word_range(entry.capture).is_some())
            .map(|matched| (entry.player, matched.word_range.end))
    })
}

fn parse_delayed_player_prefix(words: &[&str]) -> Option<(PlayerAst, usize)> {
    parse_delayed_static_player_prefix(words).or_else(|| parse_delayed_dynamic_player_prefix(words))
}

fn parse_delayed_player_before_pay(words: &[&str]) -> Option<(PlayerAst, usize)> {
    parse_delayed_static_player_exact(words).or_else(|| parse_delayed_dynamic_player_prefix(words))
}

fn delayed_tagged_creature_reference_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    clause
        .match_pattern(DELAYED_TAGGED_CREATURE_REFERENCE_PATTERN)
        .and_then(|matched| matched.capture_word_range("reference"))
        .is_some()
}

fn delayed_lose_game_unless_paid_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let lose_words = clause.word_refs();
    word_slice_eq_any(&lose_words, DELAYED_LOSE_GAME_UNLESS_PAID_WORDS)
}

fn delayed_clause_mentions_cast_or_play_action(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    DELAYED_CAST_OR_PLAY_ACTION_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("action"))
        .is_some()
}

fn delayed_clause_exactly_matches(pattern: LexPattern<'static>, words: &[&str]) -> bool {
    pattern.match_word_refs(words).is_some()
}

fn delayed_clause_starts_with_mechanic_marker<'p>(
    clause: SubjectVerbPrimitiveClause<'_>,
    marker_prefixes: &'p [&'p [&'p str]],
) -> bool {
    let atoms = [LexPattern::modifier(
        "marker",
        LexCaptureKind::OneOfPhrase(marker_prefixes),
    )];
    let words = clause.word_refs();
    LexPattern::new(&atoms)
        .match_prefix_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("marker"))
        .is_some()
}

fn delayed_clause_mentions_remains_tapped(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    DELAYED_REMAINS_MARKER_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("remains"))
        .is_some()
        && DELAYED_TAPPED_MARKER_PATTERN
            .find_in_word_refs(&words)
            .and_then(|matched| matched.capture_word_range("tapped"))
            .is_some()
}

fn delayed_clause_starts_with_action(
    clause: SubjectVerbPrimitiveClause<'_>,
    pattern: LexPattern<'static>,
) -> bool {
    let words = clause.word_refs();
    pattern
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("action"))
        .is_some_and(|range| range.start == 0)
}

fn delayed_clause_mentions_mana_cost(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    DELAYED_MANA_COST_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("cost"))
        .is_some()
}

fn delayed_word_prefix_len(
    words: &[&str],
    pattern: LexPattern<'static>,
    capture: &str,
) -> Option<usize> {
    pattern
        .match_prefix_word_refs(words)
        .and_then(|matched| matched.capture_word_range(capture))
        .filter(|range| range.start == 0)
        .map(|range| range.end - range.start)
}

fn delayed_word_suffix_len(
    words: &[&str],
    pattern: LexPattern<'static>,
    capture: &str,
) -> Option<usize> {
    (0..words.len()).find_map(|start| {
        let suffix = &words[start..];
        pattern
            .match_word_refs(suffix)
            .and_then(|matched| matched.capture_word_range(capture))
            .map(|range| range.end - range.start)
    })
}

const DELAYED_IF_YOU_WIN_REPEAT_PREFIX: &[&str] = &["if", "you", "win"];
const DELAYED_LOSE_DRAW_CLASH_PREFIX: &[&str] = &["you", "lose"];
const DELAYED_CREATURE_TYPES_EOT_TAILS: &[&[&str]] = &[
    &["all", "creature", "types", "until", "end", "of", "turn"],
    &["every", "creature", "type", "until", "end", "of", "turn"],
];
const NEXT_END_STEP_PREFIXES: &[&[&str]] = &[
    &["at", "the", "beginning", "of", "the", "next", "end", "step"],
    &["at", "the", "beginning", "of", "next", "end", "step"],
];

pub(super) fn wrap_delayed_next_step_unless_pays(
    step: DelayedNextStepKind,
    player: PlayerAst,
    effects: Vec<EffectAst>,
) -> EffectAst {
    match step {
        DelayedNextStepKind::Upkeep => EffectAst::DelayedUntilNextUpkeep { player, effects },
        DelayedNextStepKind::DrawStep => EffectAst::DelayedUntilNextDrawStep { player, effects },
    }
}

pub(crate) fn find_unquoted_token_word(
    clause: SubjectVerbPrimitiveClause<'_>,
    word: &str,
) -> Option<usize> {
    clause.find_unquoted_token_word(word)
}

fn bind_unless_player_context(effect: &mut EffectAst, player: PlayerAst) {
    match effect {
        EffectAst::UnlessPays {
            player: unless_player,
            effects,
            ..
        } => {
            if matches!(*unless_player, PlayerAst::Implicit) {
                *unless_player = player;
            }
            for nested in effects {
                bind_unless_player_context(nested, player);
            }
        }
        EffectAst::UnlessAction {
            player: unless_player,
            effects,
            alternative,
        } => {
            if matches!(*unless_player, PlayerAst::Implicit) {
                *unless_player = player;
            }
            for nested in effects {
                bind_unless_player_context(nested, player);
            }
            for nested in alternative {
                bind_unless_player_context(nested, player);
            }
        }
        _ => bind_implicit_player_context(effect, player),
    }
}

fn rewrite_value_source_to_it_tag(value: &mut Value) {
    match value {
        Value::SurfaceHinted { value, .. } => rewrite_value_source_to_it_tag(value),
        Value::Add(left, right) | Value::Min(left, right) => {
            rewrite_value_source_to_it_tag(left);
            rewrite_value_source_to_it_tag(right);
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner) => rewrite_value_source_to_it_tag(inner),
        Value::PowerOf(spec) | Value::ToughnessOf(spec) | Value::ManaValueOf(spec)
            if matches!(spec.as_ref(), crate::target::ChooseSpec::Source) =>
        {
            *spec = Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG)));
        }
        _ => {}
    }
}

fn rewrite_cost_source_values_to_it_tag(cost: &mut crate::cost::TotalCost) {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(_) => {
            let mut components = cost.costs().to_vec();
            for component in &mut components {
                match component {
                    crate::costs::Cost::DynamicMana(dynamic) => {
                        if let Some(value) = dynamic.x_value.as_mut() {
                            rewrite_value_source_to_it_tag(value);
                        }
                        if let Some(value) = dynamic.additional_generic.as_mut() {
                            rewrite_value_source_to_it_tag(value);
                        }
                        if let Some(value) = dynamic.multiplier.as_mut() {
                            rewrite_value_source_to_it_tag(value);
                        }
                    }
                    crate::costs::Cost::Energy(value)
                    | crate::costs::Cost::Mill(value)
                    | crate::costs::Cost::Life(value) => rewrite_value_source_to_it_tag(value),
                    _ => {}
                }
            }
            *cost = crate::cost::TotalCost::from_costs(components);
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            let mut branches = branches.to_vec();
            for branch in &mut branches {
                rewrite_cost_source_values_to_it_tag(branch);
            }
            *cost = crate::cost::TotalCost::one_of(branches);
        }
    }
}

pub(crate) fn rewrite_unless_cost_source_values_to_it_tag(effect: &mut EffectAst) {
    if let EffectAst::UnlessPays { cost, .. } = effect {
        rewrite_cost_source_values_to_it_tag(cost);
    }
}

pub(crate) fn parse_sentence_delayed_next_step_unless_pays(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = clause.trimmed_period_segments();
    if segments.is_empty() {
        return Ok(None);
    }

    let (leading_segments, final_segment) = segments.split_at(segments.len() - 1);
    if let Some((_, after_timing)) =
        final_segment[0].strip_any_prefix_clause(NEXT_END_STEP_PREFIXES)
    {
        let timing_clause = after_timing.trimmed();
        if timing_clause.is_empty() {
            return Ok(None);
        }
        let Some(unless_idx) = timing_clause.find_token_word("unless") else {
            return Ok(None);
        };
        let delayed_effect_clause = timing_clause.before(unless_idx).trimmed();
        if delayed_effect_clause.is_empty() {
            return Ok(None);
        }
        let delayed_effects = parse_effect_chain(delayed_effect_clause.tokens())?;
        if delayed_effects.is_empty() {
            return Ok(None);
        }
        let delayed_words = delayed_effect_clause.word_refs();
        let delayed_refs_it = matches!(
            delayed_words.as_slice(),
            ["sacrifice", "it"] | ["sacrifice", "that", "card"] | ["sacrifice", "that", "token"]
        );
        let Some(mut unless_effect) = try_build_unless(delayed_effects, timing_clause, unless_idx)?
        else {
            return Ok(None);
        };
        if delayed_refs_it {
            rewrite_unless_cost_source_values_to_it_tag(&mut unless_effect);
        }

        let mut effects = Vec::new();
        for segment in leading_segments {
            let parsed = parse_effect_chain(segment.tokens())?;
            if parsed.is_empty() {
                return Ok(None);
            }
            effects.extend(parsed);
        }
        effects.push(EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![unless_effect],
        });
        return Ok(Some(effects));
    }
    let Some((timing_start_word, _timing_end_word, step, player)) =
        delayed_next_step_marker(final_segment[0])
    else {
        return Ok(None);
    };

    let Some(delayed_effect_clause) = final_segment[0]
        .before_word(timing_start_word)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if delayed_effect_clause.is_empty() {
        return Ok(None);
    }

    let delayed_effects = parse_effect_chain(delayed_effect_clause.tokens())?;
    if delayed_effects.is_empty() {
        return Ok(None);
    }

    let Some(timing_clause) = final_segment[0]
        .from_word(timing_start_word)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    let Some(unless_idx) = timing_clause.find_token_word("unless") else {
        return Ok(None);
    };
    let Some(unless_effect) = try_build_unless(delayed_effects, timing_clause, unless_idx)? else {
        return Ok(None);
    };

    let mut effects = Vec::new();
    for segment in leading_segments {
        let parsed = parse_effect_chain(segment.tokens())?;
        if parsed.is_empty() {
            return Ok(None);
        }
        effects.extend(parsed);
    }
    effects.push(wrap_delayed_next_step_unless_pays(
        step,
        player,
        vec![unless_effect],
    ));
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_delayed_next_upkeep_unless_pays_lose_game(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = clause.trimmed_period_segments();
    if segments.len() != 2 && segments.len() != 3 {
        return Ok(None);
    }

    let (mut effects, upkeep_clause, lose_clause) = if segments.len() == 3 {
        let first_effects = parse_effect_chain(segments[0].tokens())?;
        if first_effects.is_empty() {
            return Ok(None);
        }
        (first_effects, segments[1], segments[2])
    } else {
        (Vec::new(), segments[0], segments[1])
    };
    let pay_idx = if upkeep_clause
        .strip_prefix(&[
            "at",
            "the",
            "beginning",
            "of",
            "your",
            "next",
            "upkeep",
            "pay",
        ])
        .is_some()
    {
        7usize
    } else if upkeep_clause
        .strip_prefix(&[
            "at",
            "the",
            "beginning",
            "of",
            "the",
            "next",
            "upkeep",
            "pay",
        ])
        .is_some()
    {
        8usize
    } else {
        return Ok(None);
    };

    let Some(mana_clause) = upkeep_clause.after_words(pay_idx + 1) else {
        return Ok(None);
    };
    if mana_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing mana payment in delayed next-upkeep clause (clause: '{}')",
            upkeep_clause.text()
        )));
    }

    let mana = {
        use super::super::super::grammar::primitives as grammar;
        use super::super::super::lexer::LexStream;
        use winnow::prelude::*;

        let mut stream = LexStream::new(mana_clause.tokens());
        grammar::collect_mana_symbols
            .parse_next(&mut stream)
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "missing mana payment in delayed next-upkeep clause (clause: '{}')",
                    upkeep_clause.text()
                ))
            })?
    };

    if !delayed_lose_game_unless_paid_matches(lose_clause) {
        return Ok(None);
    }

    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::subject_verb_lose_game(PlayerAst::You)],
            player: PlayerAst::You,
            cost: crate::cost::TotalCost::mana(crate::mana::ManaCost::from_symbols(mana)),
        }],
    });
    Ok(Some(effects))
}

fn normalize_unless_payment_clause_tokens(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<SubjectVerbPrimitiveOwnedClause> {
    let payment_clause = clause
        .split_once_on_word_trimmed("before")
        .map(|(payment_clause, _)| payment_clause.trimmed())
        .unwrap_or_else(|| clause.trimmed());
    let mut payment_clause =
        SubjectVerbPrimitiveOwnedClause::from_comma_trimmed_clause(payment_clause);
    let first = payment_clause.first_word()?;
    let normalized_first = match first {
        "pay" | "pays" => "pay",
        "sacrifice" | "sacrifices" => "sacrifice",
        _ => return None,
    };

    if first != normalized_first {
        payment_clause.replace_leading_word(normalized_first);
    }

    Some(payment_clause)
}

fn parse_unless_payment_clause_as_cost(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<crate::cost::TotalCost>, CardTextError> {
    let Some(payment_tokens) = normalize_unless_payment_clause_tokens(clause) else {
        return Ok(None);
    };
    crate::runtime_backend::families::activation_and_restrictions::parse_payment_clause_as_total_cost(
        payment_tokens.tokens(),
    )
}

fn parse_unless_sacrifice_clause_as_cost(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<crate::cost::TotalCost>, CardTextError> {
    let words = clause.word_refs();
    if !matches!(words.first().copied(), Some("sacrifice" | "sacrifices")) {
        return Ok(None);
    }
    let effect = super::super::zone_handlers::parse_sacrifice(clause.tokens(), None, None)?;
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Sacrifice {
                filter,
                count: 1,
                ..
            },
        ..
    }) = effect
    else {
        return Ok(None);
    };
    Ok(Some(crate::cost::TotalCost::from_cost(
        crate::costs::Cost::sacrifice(filter),
    )))
}

fn parse_unless_sacrifice_or_pay_cost(
    after_clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<(PlayerAst, crate::cost::TotalCost)>, CardTextError> {
    let after_words = after_clause.words().to_word_refs();
    let Some((player, action_word_start)) = parse_delayed_player_prefix(&after_words) else {
        return Ok(None);
    };
    let Some(action_clause) = after_clause.after_words(action_word_start) else {
        return Ok(None);
    };
    let action_clause = action_clause.trimmed();
    let Some(or_idx) =
        crate::runtime_backend::families::activation_and_restrictions::find_payment_alternative_or(
            action_clause.tokens(),
        )
    else {
        return Ok(None);
    };
    let left_clause = SubjectVerbPrimitiveClause::new(&action_clause.tokens()[..or_idx]).trimmed();
    let right_clause =
        SubjectVerbPrimitiveClause::new(&action_clause.tokens()[or_idx + 1..]).trimmed();
    if !delayed_clause_starts_with_action(left_clause, DELAYED_SACRIFICE_ACTION_PATTERN)
        || !delayed_clause_starts_with_action(right_clause, DELAYED_PAY_ACTION_PATTERN)
    {
        return Ok(None);
    }
    let Some(sacrifice_cost) = parse_unless_sacrifice_clause_as_cost(left_clause)? else {
        return Ok(None);
    };
    let Some(payment_cost) = parse_unless_payment_clause_as_cost(right_clause)? else {
        return Ok(None);
    };
    Ok(Some((
        player,
        crate::cost::TotalCost::one_of(vec![sacrifice_cost, payment_cost]),
    )))
}

/// Try to build an UnlessPays or UnlessAction AST from the tokens after "unless".
/// Returns the unless wrapper containing the given `effects` as the main effects.
pub(crate) fn try_build_unless(
    effects: Vec<EffectAst>,
    clause: SubjectVerbPrimitiveClause<'_>,
    unless_idx: usize,
) -> Result<Option<EffectAst>, CardTextError> {
    let after_clause = clause.from(unless_idx + 1).trimmed();
    let after_words = after_clause.words().to_word_refs();
    let pay_word_idx = after_clause.find_word_any(&["pay", "pays"]);

    if let Some((player, cost)) = parse_unless_sacrifice_or_pay_cost(after_clause)? {
        return Ok(Some(EffectAst::UnlessPays {
            effects,
            player,
            cost,
        }));
    }

    // Determine the player from the "unless" clause
    let Some((player, action_word_start)) = (if let Some(pay_idx) = pay_word_idx {
        parse_delayed_player_before_pay(&after_words[..pay_idx])
            .map(|(player, _)| (player, pay_idx))
    } else {
        parse_delayed_player_prefix(&after_words)
    }) else {
        return Ok(None);
    };

    let action_clause = if let Some(pay_idx) = pay_word_idx {
        after_clause.from_word(pay_idx)
    } else {
        after_clause.after_words(action_word_start)
    }
    .unwrap_or_else(|| after_clause.from(0))
    .trimmed();
    let action_word_storage = action_clause.words();
    let action_words = action_word_storage.to_word_refs();

    if delayed_clause_starts_with_action(action_clause, DELAYED_PAY_ACTION_PATTERN) {
        if delayed_clause_mentions_mana_cost(action_clause) {
            return Err(CardTextError::ParseError(format!(
                "unsupported unless-payment mana-cost clause (clause: '{}')",
                clause.text()
            )));
        }
    } else if delayed_clause_starts_with_action(action_clause, DELAYED_DRAW_ACTION_PATTERN) {
        return Err(CardTextError::ParseError(format!(
            "unsupported non-cost unless action (clause: '{}')",
            clause.text()
        )));
    }

    if matches!(
        action_words.first().copied(),
        Some("sacrifice" | "sacrifices")
    ) && let Some(cost) = parse_unless_payment_clause_as_cost(action_clause)?
    {
        return Ok(Some(EffectAst::UnlessPays {
            effects,
            player,
            cost,
        }));
    }

    if matches!(
        action_words.first().copied(),
        Some("sacrifice" | "sacrifices")
    ) && let Ok(mut alternative) = super::super::zone_handlers::parse_sacrifice(
        action_clause.tokens(),
        Some(SubjectAst::Player(player)),
        None,
    )
    .map(|effect| vec![effect])
    {
        for effect in &mut alternative {
            bind_unless_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    if let Some(cost) = parse_unless_payment_clause_as_cost(action_clause)? {
        return Ok(Some(EffectAst::UnlessPays {
            effects,
            player,
            cost,
        }));
    }

    // Prefer the action-only slice for explicit-player clauses like
    // "unless that player discards ... or sacrifices ...". Parsing the full
    // clause first can flatten the trailing "or" branch into the first action.
    if let Ok(mut alternative) = parse_effect_chain(action_clause.tokens()) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    // Fall back to the full clause when the action-only parse needs the
    // explicit player prefix to succeed.
    if let Ok(mut alternative) = parse_effect_chain(after_clause.tokens()) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_sentence_lexed(after_clause.tokens()) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_sentence_lexed(action_clause.tokens()) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) =
        parse_effect_clause(action_clause.tokens()).map(|effect| vec![effect])
    {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_unless_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if delayed_clause_starts_with_action(action_clause, DELAYED_DISCARD_ACTION_PATTERN)
        && let Ok(mut alternative) =
            super::super::zone_handlers::parse_discard(action_clause.tokens(), None)
                .map(|effect| vec![effect])
    {
        for effect in &mut alternative {
            bind_unless_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn try_build_unless_prefers_action_only_parse_for_explicit_player_or_choice() {
        let tokens = lex_line(
            "Target opponent loses 5 life unless that player discards two cards or sacrifices a creature or planeswalker of their choice.",
            0,
        )
        .expect("rewrite lexer should classify explicit-player unless choice");
        let clause = SubjectVerbPrimitiveClause::new(&tokens);
        let unless_idx = clause.find_token_word("unless").expect("unless token");
        let effects = parse_effect_chain(&tokens[..unless_idx])
            .expect("lead effect should parse before unless clause");

        let unless_effect = try_build_unless(effects, clause, unless_idx)
            .expect("unless choice should parse")
            .expect("unless choice should lower");
        let debug = format!("{unless_effect:?}");

        assert!(
            debug.contains("Discard"),
            "expected explicit-player unless choice to keep the discard branch, got {debug}"
        );
        assert!(
            debug.contains("Sacrifice"),
            "expected explicit-player unless choice to keep the sacrifice branch, got {debug}"
        );
        assert!(
            debug.contains("TargetOpponent"),
            "expected explicit-player unless choice to bind the target opponent context, got {debug}"
        );
    }

    #[test]
    fn try_build_unless_parses_sacrifice_or_pay_as_one_payment_choice() {
        let tokens = lex_line(
            "Draw a card unless target opponent sacrifices a creature of their choice or pays 3 life.",
            0,
        )
        .expect("unless sacrifice-or-pay text should lex");
        let clause = SubjectVerbPrimitiveClause::new(&tokens);
        let unless_idx = clause.find_token_word("unless").expect("unless token");
        let effects = parse_effect_chain(&tokens[..unless_idx])
            .expect("lead effect should parse before unless clause");

        let unless_effect = try_build_unless(effects, clause, unless_idx)
            .expect("unless sacrifice-or-pay should parse")
            .expect("unless sacrifice-or-pay should lower");
        let debug = format!("{unless_effect:?}");

        assert!(debug.contains("UnlessPays"), "{debug}");
        assert!(debug.contains("TargetOpponent"), "{debug}");
        assert!(debug.contains("OneOf"), "{debug}");
        assert!(debug.contains("Sacrifice"), "{debug}");
        assert!(debug.contains("Creature"), "{debug}");
        assert!(debug.contains("Life"), "{debug}");
    }
}

pub(crate) fn parse_sentence_fallback_mechanic_marker(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if delayed_clause_mentions_cast_or_play_action(clause)
        && clause
            .parse_value_with_lexed(parse_cast_or_play_tagged_clause)?
            .is_some()
    {
        return Ok(None);
    }

    let clause_words = clause.word_refs();
    if delayed_clause_exactly_matches(DELAYED_MECHANIC_CHOOSE_ONE_OF_THEM_PATTERN, &clause_words) {
        return Ok(None);
    }
    if delayed_clause_exactly_matches(DELAYED_VENTURE_DUNGEON_PATTERN, &clause_words) {
        return Ok(Some(vec![EffectAst::subject_verb_venture_into_dungeon(
            crate::cards::builders::PlayerAst::You,
            false,
        )]));
    }

    let is_match = delayed_clause_exactly_matches(DELAYED_STILL_LAND_PATTERN, &clause_words)
        || delayed_clause_starts_with_mechanic_marker(clause, &MECHANIC_MARKER_PREFIXES[..3])
        || clause
            .strip_prefix(&[
                "chooses",
                "any",
                "number",
                "of",
                "creatures",
                "they",
                "control",
            ])
            .is_some()
        || clause
            .strip_prefix(&[
                "each",
                "player",
                "chooses",
                "any",
                "number",
                "of",
                "creatures",
                "they",
                "control",
            ])
            .is_some()
        || clause
            .strip_prefix(&["an", "opponent", "chooses", "one", "of", "those", "piles"])
            .is_some()
        || clause
            .strip_prefix(&["put", "that", "pile", "into", "your", "hand"])
            .is_some()
        || clause
            .strip_prefix(&["cast", "that", "card", "for", "as", "long", "as"])
            .is_some()
        || clause
            .strip_prefix(&[
                "until", "end", "of", "turn", "this", "creature", "loses", "prevent", "all",
                "damage",
            ])
            .is_some()
        || clause
            .strip_prefix(&[
                "until",
                "end",
                "of",
                "turn",
                "target",
                "creature",
                "loses",
                "all",
                "abilities",
                "and",
                "has",
                "base",
                "power",
                "and",
                "toughness",
            ])
            .is_some()
        || clause
            .strip_prefix(&["for", "each", "1", "damage", "prevented", "this", "way"])
            .is_some()
        || clause
            .strip_prefix(&[
                "for", "each", "card", "less", "than", "two", "a", "player", "draws", "this", "way",
            ])
            .is_some()
        || clause
            .strip_prefix(&["this", "deals", "4", "damage", "if", "there", "are"])
            .is_some()
        || clause
            .strip_prefix(&[
                "this", "deals", "4", "damage", "instead", "if", "there", "are",
            ])
            .is_some()
        || clause
            .strip_prefix(&[
                "that", "spell", "deals", "damage", "to", "each", "opponent", "equal", "to",
            ])
            .is_some()
        || clause
            .strip_prefix(&[
                "the", "next", "spell", "you", "cast", "this", "turn", "costs",
            ])
            .is_some()
        || clause
            .strip_prefix(&[
                "that",
                "creature",
                "attacks",
                "during",
                "its",
                "controllers",
                "next",
                "combat",
                "phase",
                "if",
                "able",
            ])
            .is_some()
        || clause
            .strip_prefix(&[
                "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to", "target",
                "creature", "you", "control", "by", "a", "source", "of", "your", "choice", "is",
                "dealt", "to", "another", "target", "creature", "instead",
            ])
            .is_some()
        || (delayed_clause_starts_with_mechanic_marker(clause, &MECHANIC_MARKER_PREFIXES[3..])
            && delayed_clause_mentions_remains_tapped(clause));
    if !is_match {
        return Ok(None);
    }
    Err(CardTextError::ParseError(format!(
        "unsupported mechanic marker clause (clause: '{}')",
        clause.text()
    )))
}

pub(crate) fn parse_sentence_implicit_become_clause(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((target, rest_clause)) = clause.strip_prefix_value_clause(&[
        (&["this", "permanent"], TargetAst::Source(None)),
        (&["this", "creature"], TargetAst::Source(None)),
        (&["this", "land"], TargetAst::Source(None)),
        (&["this"], TargetAst::Source(None)),
        (
            &["each", "of", "them"],
            TargetAst::Tagged(TagKey::from(IT_TAG), None),
        ),
        (&["they're"], TargetAst::Tagged(TagKey::from(IT_TAG), None)),
        (&["they’re"], TargetAst::Tagged(TagKey::from(IT_TAG), None)),
        (&["theyre"], TargetAst::Tagged(TagKey::from(IT_TAG), None)),
        (
            &["they", "are"],
            TargetAst::Tagged(TagKey::from(IT_TAG), None),
        ),
        (&["they"], TargetAst::Tagged(TagKey::from(IT_TAG), None)),
        (&["its"], TargetAst::Tagged(TagKey::from(IT_TAG), None)),
        (&["it"], TargetAst::Tagged(TagKey::from(IT_TAG), None)),
    ]) else {
        return Ok(None);
    };
    let rest_clause = rest_clause.trimmed();
    let (mut duration, duration_remainder_clause) =
        if let Some((duration, remainder)) = parse_restriction_duration(rest_clause.tokens())? {
            (duration, SubjectVerbPrimitiveOwnedClause::new(remainder))
        } else {
            (
                Until::Forever,
                SubjectVerbPrimitiveOwnedClause::from_clause(rest_clause),
            )
        };
    let mut rest_words = duration_remainder_clause.as_clause().trimmed_word_refs();
    if let Some(prefix_len) =
        delayed_word_prefix_len(&rest_words, DELAYED_STILL_PREFIX_PATTERN, "still")
    {
        rest_words.drain(..prefix_len);
    }
    if rest_words.is_empty() {
        return Ok(None);
    }

    let negated = if word_slice_starts_with_any(&rest_words, DELAYED_NEGATED_BE_PREFIXES) {
        let prefix_len = DELAYED_NEGATED_BE_PREFIXES
            .iter()
            .find(|prefix| word_slice_starts_with(&rest_words, prefix))
            .map(|prefix| prefix.len())
            .unwrap_or(0);
        rest_words.drain(..prefix_len);
        true
    } else if rest_words
        .first()
        .is_some_and(|word| DELAYED_CONTRACTION_NEGATED_BE_WORDS.contains(word))
    {
        rest_words.drain(..1);
        true
    } else {
        if let Some(prefix_len) =
            delayed_word_prefix_len(&rest_words, DELAYED_BE_PREFIX_PATTERN, "be")
        {
            rest_words.drain(..prefix_len);
        }
        false
    };
    if let Some(suffix_len) = delayed_word_suffix_len(
        &rest_words,
        DELAYED_UNTIL_END_OF_TURN_SUFFIX_PATTERN,
        "duration",
    ) {
        duration = Until::EndOfTurn;
        let new_len = rest_words.len().saturating_sub(suffix_len);
        rest_words.truncate(new_len);
    }
    if rest_words.is_empty() {
        return Ok(None);
    }

    let negative_type_words = if negated {
        if rest_words
            .first()
            .copied()
            .is_some_and(|word| DELAYED_ARTICLE_WORDS.contains(&word))
        {
            Some(&rest_words[1..])
        } else {
            Some(&rest_words[..])
        }
    } else if let Some(prefix_len) = delayed_word_prefix_len(
        &rest_words,
        DELAYED_NOT_ARTICLE_PREFIX_PATTERN,
        "not_article",
    ) && rest_words.len() > prefix_len
    {
        Some(&rest_words[prefix_len..])
    } else if let Some(prefix_len) =
        delayed_word_prefix_len(&rest_words, DELAYED_NOT_PREFIX_PATTERN, "not")
        && rest_words.len() > prefix_len
    {
        Some(&rest_words[prefix_len..])
    } else {
        None
    };
    if let Some(type_words) = negative_type_words {
        let mut card_types = Vec::new();
        let mut all_card_types = true;
        for word in type_words {
            if let Some(card_type) = parse_card_type(word) {
                if !iter_contains(card_types.iter(), &card_type) {
                    card_types.push(card_type);
                }
            } else {
                all_card_types = false;
                break;
            }
        }
        if all_card_types && !card_types.is_empty() {
            return Ok(Some(vec![EffectAst::subject_verb_remove_card_types(
                target, card_types, duration,
            )]));
        }
    }

    let addition_tail_len = delayed_word_suffix_len(
        &rest_words,
        DELAYED_ADDITION_OTHER_TYPES_SUFFIX_PATTERN,
        "addition",
    );

    let body_words = if rest_words
        .first()
        .is_some_and(|word| DELAYED_ARTICLE_WORDS.contains(word))
    {
        &rest_words[1..]
    } else {
        &rest_words[..]
    };
    if body_words.is_empty() {
        return Ok(None);
    }

    if let Ok((power, toughness)) = parse_pt_modifier_values(body_words[0])
        && body_words.len() > 1
    {
        let mut card_types = Vec::new();
        let mut subtypes = Vec::new();
        let mut parsed_all_descriptor_words = true;
        let mut saw_subtype = false;
        for word in &body_words[1..] {
            if matches!(*word, "and" | "or") {
                continue;
            }
            if let Some(card_type) = parse_card_type(word) {
                if !iter_contains(card_types.iter(), &card_type) {
                    card_types.push(card_type);
                }
            } else if let Some(subtype) = parse_pluralized_subtype_word(word) {
                if !iter_contains(subtypes.iter(), &subtype) {
                    subtypes.push(subtype);
                }
                saw_subtype = true;
            } else {
                parsed_all_descriptor_words = false;
                break;
            }
        }
        if parsed_all_descriptor_words && (!card_types.is_empty() || saw_subtype) {
            if saw_subtype && !iter_contains(card_types.iter(), &CardType::Creature) {
                card_types.insert(0, CardType::Creature);
            }
            return Ok(Some(vec![EffectAst::subject_verb_become_base_pt_creature(
                power,
                toughness,
                target,
                card_types,
                subtypes,
                Vec::new(),
                None,
                Vec::new(),
                Vec::new(),
                duration,
            )]));
        }
    }

    if let Ok((power, toughness)) = parse_pt_modifier_values(body_words[0])
        && let Some(tail_len) = addition_tail_len
        && body_words.len() > 1 + tail_len
    {
        let subtype_words = &body_words[1..body_words.len().saturating_sub(tail_len)];
        let mut subtypes = Vec::new();
        for word in subtype_words {
            let Some(subtype) = parse_pluralized_subtype_word(word) else {
                return Ok(None);
            };
            if !iter_contains(subtypes.iter(), &subtype) {
                subtypes.push(subtype);
            }
        }
        if subtypes.is_empty() {
            return Ok(None);
        }
        return Ok(Some(vec![
            EffectAst::subject_verb_set_base_power_toughness(
                power,
                toughness,
                target.clone(),
                duration.clone(),
            ),
            EffectAst::subject_verb_add_subtypes(target, subtypes, duration),
        ]));
    }

    let type_words = if let Some(tail_len) = addition_tail_len {
        &body_words[..body_words.len().saturating_sub(tail_len)]
    } else {
        body_words
    };
    if type_words.is_empty() {
        return Ok(None);
    }

    let mut card_types = Vec::new();
    let mut all_card_types = true;
    for word in type_words {
        if let Some(card_type) = parse_card_type(word) {
            if !iter_contains(card_types.iter(), &card_type) {
                card_types.push(card_type);
            }
        } else {
            all_card_types = false;
            break;
        }
    }
    if all_card_types && !card_types.is_empty() {
        return Ok(Some(vec![EffectAst::subject_verb_add_card_types(
            target, card_types, duration,
        )]));
    }

    let mut subtypes = Vec::new();
    for word in type_words {
        let Some(subtype) = parse_pluralized_subtype_word(word) else {
            return Ok(None);
        };
        if !iter_contains(subtypes.iter(), &subtype) {
            subtypes.push(subtype);
        }
    }
    if subtypes.is_empty() {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::subject_verb_add_subtypes(
        target, subtypes, duration,
    )]))
}

pub(crate) fn parse_sentence_gains_or_loses_all_creature_types(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = clause.word_refs();
    let ends_with_creature_types_eot_tail =
        DELAYED_CREATURE_TYPES_EOT_TAILS.iter().any(|expected| {
            let tail = &words[words.len().saturating_sub(expected.len())..];
            word_slice_eq_any(tail, DELAYED_CREATURE_TYPES_EOT_TAILS)
        });
    if !ends_with_creature_types_eot_tail {
        return Ok(None);
    }
    let pattern = LexPattern::new(GAINS_OR_LOSES_ALL_CREATURE_TYPES_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_gains_or_loses_all_creature_types_matched(clause, &matched)
}

pub(crate) fn parse_sentence_gains_or_loses_all_creature_types_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(subject_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Subject) else {
        return Ok(None);
    };
    let Some(verb_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Action) else {
        return Ok(None);
    };
    let verb_words = verb_clause.word_refs();
    let is_gain = matches!(verb_words.as_slice(), ["gain"] | ["gains"]);
    let subject_words = subject_clause.word_refs();

    if !is_gain
        && let Some(get_word_idx) = subject_words
            .iter()
            .position(|word| DELAYED_GET_WORDS.contains(word))
    {
        let Some(modifier_word) = subject_words.get(get_word_idx + 1).copied() else {
            return Ok(None);
        };
        let Ok((power, toughness)) = parse_pt_modifier_values(modifier_word) else {
            return Ok(None);
        };
        let Some(target_clause) = subject_clause
            .before_word(get_word_idx)
            .map(SubjectVerbPrimitiveClause::trimmed)
        else {
            return Ok(None);
        };
        if target_clause.is_empty() {
            return Ok(None);
        }
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(vec![
            EffectAst::subject_verb_pump(power, toughness, target.clone(), Until::EndOfTurn, None),
            EffectAst::subject_verb_remove_all_subtypes_of_family(
                target,
                crate::types::SubtypeFamily::Creature,
                Until::EndOfTurn,
            ),
        ]));
    }

    let target = if delayed_tagged_creature_reference_matches(subject_clause) {
        TargetAst::Tagged(TagKey::from(IT_TAG), None)
    } else {
        parse_target_phrase(subject_clause.trimmed().tokens())?
    };
    let effect = if is_gain {
        EffectAst::subject_verb_add_all_subtypes_of_family(
            target,
            crate::types::SubtypeFamily::Creature,
            Until::EndOfTurn,
        )
    } else {
        EffectAst::subject_verb_remove_all_subtypes_of_family(
            target,
            crate::types::SubtypeFamily::Creature,
            Until::EndOfTurn,
        )
    };
    Ok(Some(vec![effect]))
}

fn fixed_count_word(word: &str) -> Option<i32> {
    ironsmith_core::parse_cardinal_word(word).and_then(|value| value.try_into().ok())
}

pub(crate) fn parse_sentence_lose_draw_clash_repeat_process(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let body_words_vec = clause.word_refs();
    let body_words = body_words_vec.as_slice();
    if !word_slice_starts_with(body_words, DELAYED_LOSE_DRAW_CLASH_PREFIX) {
        return Ok(None);
    }
    let pattern = LexPattern::new(LOSE_DRAW_CLASH_REPEAT_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_lose_draw_clash_repeat_process_matched(clause, &matched)
}

pub(crate) fn parse_sentence_lose_draw_clash_repeat_process_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(life_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Amount) else {
        return Ok(None);
    };
    let Some(draw_clause) = clause.pattern_capture(matched, "draw") else {
        return Ok(None);
    };
    let life_words = life_clause.word_refs();
    let draw_words = draw_clause.word_refs();
    let Some(life_count) = life_words.first().and_then(|word| fixed_count_word(word)) else {
        return Ok(None);
    };
    let Some(draw_count) = draw_words.first().and_then(|word| fixed_count_word(word)) else {
        return Ok(None);
    };

    let effects = vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LoseLife {
                amount: Value::Fixed(life_count),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: Value::Fixed(draw_count),
            },
        ),
        EffectAst::subject_verb_clash(ClashOpponentAst::Opponent),
    ];
    let words = clause.word_refs();
    if word_slice_find_phrase_start(&words, DELAYED_IF_YOU_WIN_REPEAT_PREFIX).is_none() {
        return Ok(Some(effects));
    }

    Ok(Some(vec![EffectAst::RepeatProcess {
        effects,
        continue_effect_index: 2,
        continue_predicate: IfResultPredicate::Value(crate::effect::Comparison::GreaterThan(0)),
    }]))
}

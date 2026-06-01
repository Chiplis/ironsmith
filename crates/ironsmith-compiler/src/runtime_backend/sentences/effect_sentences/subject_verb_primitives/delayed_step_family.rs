use super::*;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
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
const DELAYED_LOSE_GAME_UNLESS_PAID_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["if", "you", "dont", "you", "lose", "the", "game"],
            &["if", "you", "do", "not", "you", "lose", "the", "game"],
            &["if", "you", "don't", "you", "lose", "the", "game"],
        ]
);
const DELAYED_PLAYER_PREFIX_YOU: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const DELAYED_PLAYER_PREFIX_TARGET_OPPONENT: ClauseShape<'static> =
    clause_shape!(exact & ["target", "opponent"]);
const DELAYED_PLAYER_PREFIX_TARGET_PLAYER: ClauseShape<'static> =
    clause_shape!(exact & ["target", "player"]);
const DELAYED_PLAYER_PREFIX_ANY_PLAYER: ClauseShape<'static> =
    clause_shape!(exact & ["any", "player"]);
const DELAYED_PLAYER_PREFIX_THEY: ClauseShape<'static> = clause_shape!(exact & ["they"]);
const DELAYED_PLAYER_PREFIX_DEFENDING_PLAYER: ClauseShape<'static> =
    clause_shape!(exact & ["defending", "player"]);
const DELAYED_PLAYER_PREFIX_THAT_PLAYER: ClauseShape<'static> =
    clause_shape!(exact & ["that", "player"]);
const DELAYED_PLAYER_PREFIX_ITS_CONTROLLER: ClauseShape<'static> =
    clause_shape!(exact_any & [&["its", "controller"], &["their", "controller"]]);
const DELAYED_PLAYER_PREFIX_ITS_OWNER: ClauseShape<'static> =
    clause_shape!(exact_any & [&["its", "owner"], &["their", "owner"]]);
const DELAYED_PAY_OR_PAYS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["pay"], &["pays"]]);
const DELAYED_MANA_COST_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["mana", "cost"]]);
const DELAYED_DRAW_OR_DRAWS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["draw"], &["draws"]]);
const DELAYED_DISCARD_OR_DISCARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["discard"], &["discards"]]);
const DELAYED_THAT_PLAYER_OR_THAT_PREFIX: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "player", "or", "that"]);
const DELAYED_THAT_PREFIX: ClauseShape<'static> = clause_shape!(prefix & ["that"]);
const DELAYED_OR_THAT_PLAYER_TAIL: ClauseShape<'static> =
    clause_shape!(suffix & ["or", "that", "player"]);
const DELAYED_REFERRED_OBJECT_NOUN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["ability"],
            &["abilitys"],
            &["card"],
            &["cards"],
            &["creature"],
            &["creatures"],
            &["object"],
            &["objects"],
            &["permanent"],
            &["permanents"],
            &["planeswalker"],
            &["planeswalkers"],
            &["source"],
            &["sources"],
            &["spell"],
            &["spells"],
        ]
);
const DELAYED_REFERRED_PERMANENT_NOUN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["card"],
            &["cards"],
            &["creature"],
            &["creatures"],
            &["object"],
            &["objects"],
            &["permanent"],
            &["permanents"],
            &["planeswalker"],
            &["planeswalkers"],
            &["source"],
            &["sources"],
            &["spell"],
            &["spells"],
        ]
);
const DELAYED_CONTROLLER_OR_CONTROLLERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["controller"], &["controllers"]]);
const DELAYED_OWNER_OR_OWNERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["owner"], &["owners"]]);
const DELAYED_MECHANIC_CHOOSE_ONE_OF_THEM_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "choose", "one", "of", "them"]);
const DELAYED_VENTURE_DUNGEON_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["venture", "into", "the", "dungeon"]);
const DELAYED_STILL_LAND_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["its", "still", "a", "land"],
            &["it", "still", "a", "land"]
        ]
);
const DELAYED_STILL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["still"]);
const DELAYED_NEGATED_BE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["is", "not"], &["are", "not"]]);
const DELAYED_CONTRACTION_NEGATED_BE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["isnt"], &["isn't"], &["arent"], &["aren't"]]);
const DELAYED_BE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["is"], &["are"], &["s"], &["’s"]]);
const DELAYED_UNTIL_END_OF_TURN_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["until", "end", "of", "turn"]);
const DELAYED_NOT_ARTICLE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["not", "a"], &["not", "an"]]);
const DELAYED_NOT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["not"]);
const DELAYED_ARTICLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["the"]]);
const DELAYED_GAIN_LOSE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["gain"], &["gains"], &["lose"], &["loses"]]);
const DELAYED_GAIN_OR_GAINS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["gain"], &["gains"]]);
const DELAYED_GET_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["get"], &["gets"]]);
const DELAYED_ADDITION_OTHER_TYPES_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["in", "addition", "to", "its", "other", "types"],
            &["in", "addition", "to", "their", "other", "types"],
            &["in", "addition", "to", "its", "other", "type"],
            &["in", "addition", "to", "their", "other", "type"],
        ]
);
const DELAYED_CREATURE_TYPES_EOT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["all", "creature", "types", "until", "end", "of", "turn"],
            &["every", "creature", "type", "until", "end", "of", "turn"],
        ]
);
const DELAYED_IT_OR_THAT_CREATURE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["that", "creature"]]);
const DELAYED_LOSE_DRAW_CLASH_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you", "lose"]; contains_phrases & [&["and", "draw"], &["then", "clash", "with", "an", "opponent"]]);
const DELAYED_IF_YOU_WIN_REPEAT_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["if", "you", "win"]);
const DELAYED_REPEAT_THIS_PROCESS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["repeat", "this", "process"]);
const DELAYED_LIFE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["life"]);
const DELAYED_CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);

fn delayed_find_phrase_start(words: &[&str], shape: ClauseShape<'static>) -> Option<usize> {
    (0..words.len()).find(|idx| shape.matches_words(&words[*idx..]))
}

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

pub(crate) fn parse_sentence_delayed_next_step_unless_pays(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = clause.trimmed_period_segments();
    if segments.is_empty() {
        return Ok(None);
    }

    let (leading_segments, final_segment) = segments.split_at(segments.len() - 1);
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

    let lose_words = lose_clause.word_refs();
    let valid_lose_clause = DELAYED_LOSE_GAME_UNLESS_PAID_PATTERN.matches_words(&lose_words);
    if !valid_lose_clause {
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

    let match_player_prefix = |prefix: &[&str]| -> Option<(PlayerAst, usize)> {
        if DELAYED_PLAYER_PREFIX_YOU.matches_words(prefix) {
            Some((PlayerAst::You, 1))
        } else if DELAYED_PLAYER_PREFIX_TARGET_OPPONENT.matches_words(prefix) {
            Some((PlayerAst::TargetOpponent, 2))
        } else if DELAYED_PLAYER_PREFIX_TARGET_PLAYER.matches_words(prefix) {
            Some((PlayerAst::Target, 2))
        } else if DELAYED_PLAYER_PREFIX_ANY_PLAYER.matches_words(prefix) {
            Some((PlayerAst::Any, 2))
        } else if DELAYED_PLAYER_PREFIX_THEY.matches_words(prefix) {
            Some((PlayerAst::That, 1))
        } else if DELAYED_PLAYER_PREFIX_DEFENDING_PLAYER.matches_words(prefix) {
            Some((PlayerAst::Defending, 2))
        } else if DELAYED_PLAYER_PREFIX_THAT_PLAYER.matches_words(prefix) {
            Some((PlayerAst::That, 2))
        } else if DELAYED_PLAYER_PREFIX_ITS_CONTROLLER.matches_words(prefix) {
            Some((PlayerAst::ItsController, 2))
        } else if DELAYED_PLAYER_PREFIX_ITS_OWNER.matches_words(prefix) {
            Some((PlayerAst::ItsOwner, 2))
        } else if prefix.len() >= 6
            && DELAYED_THAT_PLAYER_OR_THAT_PREFIX.matches_words(prefix)
            && DELAYED_REFERRED_OBJECT_NOUN_WORD_PATTERN.matches_word(prefix[4])
            && DELAYED_CONTROLLER_OR_CONTROLLERS_WORD_PATTERN.matches_word(prefix[5])
        {
            Some((PlayerAst::ThatPlayerOrTargetController, 6))
        } else if prefix.len() >= 3
            && DELAYED_THAT_PREFIX.matches_words(prefix)
            && DELAYED_REFERRED_OBJECT_NOUN_WORD_PATTERN.matches_word(prefix[1])
            && DELAYED_CONTROLLER_OR_CONTROLLERS_WORD_PATTERN.matches_word(prefix[2])
        {
            Some((PlayerAst::ItsController, 3))
        } else if prefix.len() >= 3
            && DELAYED_THAT_PREFIX.matches_words(prefix)
            && DELAYED_REFERRED_OBJECT_NOUN_WORD_PATTERN.matches_word(prefix[1])
            && DELAYED_OWNER_OR_OWNERS_WORD_PATTERN.matches_word(prefix[2])
        {
            Some((PlayerAst::ItsOwner, 3))
        } else if prefix.len() >= 6
            && DELAYED_THAT_PREFIX.matches_words(prefix)
            && DELAYED_REFERRED_PERMANENT_NOUN_WORD_PATTERN.matches_word(prefix[1])
            && DELAYED_CONTROLLER_OR_CONTROLLERS_WORD_PATTERN.matches_word(prefix[2])
            && DELAYED_OR_THAT_PLAYER_TAIL.matches_words(&prefix[3..6])
        {
            Some((PlayerAst::ThatPlayerOrTargetController, 6))
        } else {
            None
        }
    };

    let match_player_clause_prefix = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        let max_prefix_len = words.len().min(6);
        for prefix_len in 1..=max_prefix_len {
            if let Some((player, consumed)) = match_player_prefix(&words[..prefix_len]) {
                return Some((player, consumed));
            }
        }
        None
    };

    // Determine the player from the "unless" clause
    let Some((player, action_word_start)) = (if let Some(pay_idx) = pay_word_idx {
        match_player_prefix(&after_words[..pay_idx]).map(|(player, _)| (player, pay_idx))
    } else {
        match_player_clause_prefix(&after_words)
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

    if DELAYED_PAY_OR_PAYS_WORD_PATTERN.matches_first_word(&action_words) {
        if DELAYED_MANA_COST_MARKER_PATTERN.matches_words(&action_clause.word_refs()) {
            return Err(CardTextError::ParseError(format!(
                "unsupported unless-payment mana-cost clause (clause: '{}')",
                clause.text()
            )));
        }
    } else if DELAYED_DRAW_OR_DRAWS_WORD_PATTERN.matches_first_word(&action_words) {
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

    if DELAYED_DISCARD_OR_DISCARDS_WORD_PATTERN.matches_first_word(&action_words)
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
}

pub(crate) fn parse_sentence_fallback_mechanic_marker(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if clause.contains_any_word(&[
        "may", "cast", "casts", "casting", "play", "plays", "playing", "played",
    ]) && clause
        .parse_value_with_lexed(parse_cast_or_play_tagged_clause)?
        .is_some()
    {
        return Ok(None);
    }

    let clause_words = clause.word_refs();
    if DELAYED_MECHANIC_CHOOSE_ONE_OF_THEM_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }
    if DELAYED_VENTURE_DUNGEON_PATTERN.matches_words(&clause_words) {
        return Ok(Some(vec![EffectAst::subject_verb_venture_into_dungeon(
            crate::cards::builders::PlayerAst::You,
            false,
        )]));
    }

    let is_match = DELAYED_STILL_LAND_PATTERN.matches_words(&clause_words)
        || clause
            .strip_any_prefix(&MECHANIC_MARKER_PREFIXES[..3])
            .is_some()
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
        || (clause
            .strip_any_prefix(&MECHANIC_MARKER_PREFIXES[3..])
            .is_some()
            && clause.contains_word("remains")
            && clause.contains_word("tapped"));
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
    if DELAYED_STILL_WORD_PATTERN.matches_first_word(&rest_words) {
        rest_words.remove(0);
    }
    if rest_words.is_empty() {
        return Ok(None);
    }

    let negated = if DELAYED_NEGATED_BE_PREFIX_PATTERN.matches_words(&rest_words) {
        rest_words.drain(..2);
        true
    } else if DELAYED_CONTRACTION_NEGATED_BE_WORD_PATTERN.matches_first_word(&rest_words) {
        rest_words.remove(0);
        true
    } else {
        if DELAYED_BE_WORD_PATTERN.matches_first_word(&rest_words) {
            rest_words.remove(0);
        }
        false
    };
    if DELAYED_UNTIL_END_OF_TURN_TAIL_PATTERN.matches_words(&rest_words) {
        duration = Until::EndOfTurn;
        let new_len = rest_words.len().saturating_sub(4);
        rest_words.truncate(new_len);
    }
    if rest_words.is_empty() {
        return Ok(None);
    }

    let negative_type_words = if negated {
        if rest_words
            .first()
            .copied()
            .is_some_and(|word| DELAYED_ARTICLE_WORD_PATTERN.matches_word(word))
        {
            Some(&rest_words[1..])
        } else {
            Some(&rest_words[..])
        }
    } else if DELAYED_NOT_ARTICLE_PREFIX_PATTERN.matches_words(&rest_words) && rest_words.len() > 2
    {
        Some(&rest_words[2..])
    } else if DELAYED_NOT_PREFIX_PATTERN.matches_words(&rest_words) && rest_words.len() > 1 {
        Some(&rest_words[1..])
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

    let addition_tail_len = if DELAYED_ADDITION_OTHER_TYPES_TAIL_PATTERN.matches_words(&rest_words)
    {
        Some(6usize)
    } else {
        None
    };

    let body_words = if rest_words
        .first()
        .is_some_and(|word| DELAYED_ARTICLE_WORD_PATTERN.matches_word(word))
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
    let pattern = LexPattern::new(GAINS_OR_LOSES_ALL_CREATURE_TYPES_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_gains_or_loses_all_creature_types_matched(clause, &matched)
}

pub(crate) fn parse_sentence_gains_or_loses_all_creature_types_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = clause.word_refs();
    let Some(verb_idx) = words
        .iter()
        .position(|word| DELAYED_GAIN_LOSE_WORD_PATTERN.matches_word(word))
    else {
        return Ok(None);
    };
    let is_gain = DELAYED_GAIN_OR_GAINS_WORD_PATTERN.matches_word(words[verb_idx]);
    let tail = &words[verb_idx + 1..];
    if !DELAYED_CREATURE_TYPES_EOT_TAIL_PATTERN.matches_words(tail) {
        return Ok(None);
    }

    if !is_gain
        && let Some(get_word_idx) = words[..verb_idx]
            .iter()
            .position(|word| DELAYED_GET_WORD_PATTERN.matches_word(word))
    {
        let Some(modifier_word) = words.get(get_word_idx + 1).copied() else {
            return Ok(None);
        };
        let Ok((power, toughness)) = parse_pt_modifier_values(modifier_word) else {
            return Ok(None);
        };
        let Some(target_clause) = clause
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

    let target = if DELAYED_IT_OR_THAT_CREATURE_PATTERN.matches_words(&words[..verb_idx]) {
        TargetAst::Tagged(TagKey::from(IT_TAG), None)
    } else {
        let Some(target_clause) = clause
            .before_word(verb_idx)
            .map(SubjectVerbPrimitiveClause::trimmed)
        else {
            return Ok(None);
        };
        parse_target_phrase(target_clause.tokens())?
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
    let pattern = LexPattern::new(LOSE_DRAW_CLASH_REPEAT_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_lose_draw_clash_repeat_process_matched(clause, &matched)
}

pub(crate) fn parse_sentence_lose_draw_clash_repeat_process_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = clause.word_refs();
    let if_idx = delayed_find_phrase_start(&words, DELAYED_IF_YOU_WIN_REPEAT_PATTERN);
    let body_words = if let Some(if_idx) = if_idx {
        if !words
            .get(if_idx + 3..)
            .is_some_and(|tail| DELAYED_REPEAT_THIS_PROCESS_PATTERN.matches_words(tail))
        {
            return Ok(None);
        }
        &words[..if_idx]
    } else {
        &words[..]
    };
    if body_words.len() != 13
        || !DELAYED_LOSE_DRAW_CLASH_PREFIX_PATTERN.matches_words(body_words)
        || !DELAYED_LIFE_WORD_PATTERN.matches_word_at(body_words, 3)
        || !DELAYED_CARD_OR_CARDS_WORD_PATTERN.matches_word_at(body_words, 7)
    {
        return Ok(None);
    }
    let Some(life_count) = fixed_count_word(body_words[2]) else {
        return Ok(None);
    };
    let Some(draw_count) = fixed_count_word(body_words[6]) else {
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
    if if_idx.is_none() {
        return Ok(Some(effects));
    }

    Ok(Some(vec![EffectAst::RepeatProcess {
        effects,
        continue_effect_index: 2,
        continue_predicate: IfResultPredicate::Value(crate::effect::Comparison::GreaterThan(0)),
    }]))
}

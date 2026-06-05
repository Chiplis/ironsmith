const REPLACE_EXCEPT_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["except"]);
const REPLACE_EXILE_UNTIL_LEAVES_BATTLEFIELD_PATTERN: ClauseShape<'static> =
    ClauseShape::new()
        .contains_words(&["until"])
        .suffix(&["leaves", "the", "battlefield"]);
const REPLACE_EXILE_ALL_CARDS_FROM_ZONES_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action("verb", LexCaptureKind::OneOf(&["exile"])),
    LexPattern::phrase(&["all", "cards", "from"]),
    LexPattern::tail("zones", LexCaptureKind::Rest),
]);
const REPLACE_ZONE_LIST_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::capture("first_zone", LexCaptureKind::UntilPhrase(&["and"])),
    LexPattern::word("and"),
    LexPattern::tail("second_zone", LexCaptureKind::OneOrMoreWords),
]);
const REPLACE_HAND_ZONE_PHRASES: &[&[&str]] = &[&["hand"], &["hands"]];
const REPLACE_GRAVEYARD_ZONE_PHRASES: &[&[&str]] = &[&["graveyard"], &["graveyards"]];
const REPLACE_HAND_ZONE_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::capture("owner", LexCaptureKind::UntilAnyPhrase(REPLACE_HAND_ZONE_PHRASES)),
    LexPattern::any_phrase(REPLACE_HAND_ZONE_PHRASES),
]);
const REPLACE_GRAVEYARD_ZONE_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::capture(
        "owner",
        LexCaptureKind::UntilAnyPhrase(REPLACE_GRAVEYARD_ZONE_PHRASES),
    ),
    LexPattern::any_phrase(REPLACE_GRAVEYARD_ZONE_PHRASES),
]);
const REPLACE_VOTED_WITH_YOU_SCRY_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::subject(
        "voters",
        LexCaptureKind::OneOfPhrase(&[&[
            "you", "and", "each", "opponent", "who", "voted", "for", "a", "choice", "you",
            "voted", "for",
        ]]),
    ),
    LexPattern::capture("may", LexCaptureKind::OneOf(&["may"])),
    LexPattern::action("scry", LexCaptureKind::OneOf(&["scry"])),
    LexPattern::amount("count", LexCaptureKind::Rest),
]);
const REPLACE_TOKEN_END_COMBAT_OBJECT_PHRASES: &[&[&str]] = &[
    &["that", "token"],
    &["that", "tokens"],
    &["the", "token"],
    &["the", "tokens"],
    &["those", "token"],
    &["those", "tokens"],
    &["it"],
];
const REPLACE_TOKEN_END_COMBAT_TIMING_PHRASES: &[&[&str]] =
    &[&["end", "of", "combat"], &["the", "end", "of", "combat"]];
const REPLACE_TARGET_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["target"]);
const REPLACE_LOOK_HAND_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["look", "at"]),
    LexPattern::object("player", LexCaptureKind::UntilPhrase(&["hand"])),
    LexPattern::word("hand"),
    LexPattern::tail("followup", LexCaptureKind::Rest),
]);
const REPLACE_LOOK_HAND_TARGET_PLAYER_PHRASES: &[&[&str]] =
    &[&["target", "players"], &["target", "player"]];
const REPLACE_LOOK_HAND_TARGET_OPPONENT_PHRASES: &[&[&str]] =
    &[&["target", "opponent"], &["target", "opponents"]];
const REPLACE_LOOK_HAND_OPPONENT_PHRASES: &[&[&str]] = &[&["an", "opponents"], &["opponents"]];
const REPLACE_LOOK_HAND_ITERATED_PLAYER_PHRASES: &[&[&str]] = &[&["that", "players"]];
const REPLACE_LOOK_HAND_TARGET_PLAYER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::object(
        "player",
        LexCaptureKind::OneOfPhrase(REPLACE_LOOK_HAND_TARGET_PLAYER_PHRASES),
    ),
]);
const REPLACE_LOOK_HAND_TARGET_OPPONENT_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::object(
        "player",
        LexCaptureKind::OneOfPhrase(REPLACE_LOOK_HAND_TARGET_OPPONENT_PHRASES),
    ),
]);
const REPLACE_LOOK_HAND_OPPONENT_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::object(
        "player",
        LexCaptureKind::OneOfPhrase(REPLACE_LOOK_HAND_OPPONENT_PHRASES),
    ),
]);
const REPLACE_LOOK_HAND_ITERATED_PLAYER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::object(
        "player",
        LexCaptureKind::OneOfPhrase(REPLACE_LOOK_HAND_ITERATED_PLAYER_PHRASES),
    ),
]);
const REPLACE_LOOK_HAND_CHOOSE_NAME_OBJECT_PHRASES: &[&[&str]] = &[&["any", "card", "name"]];
const REPLACE_LOOK_HAND_CHOOSE_NAME_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("then"),
    LexPattern::action("choose", LexCaptureKind::OneOf(&["choose"])),
    LexPattern::object(
        "name",
        LexCaptureKind::OneOfPhrase(REPLACE_LOOK_HAND_CHOOSE_NAME_OBJECT_PHRASES),
    ),
]);
const REPLACE_LOOK_TOP_COUNT_CARD_OF_SEQUENCE: &[LexPatternAtom<'static>] =
    &[LexPattern::phrase(&["card", "of"])];
const REPLACE_LOOK_TOP_COUNT_CARDS_OF_SEQUENCE: &[LexPatternAtom<'static>] =
    &[LexPattern::phrase(&["cards", "of"])];
const REPLACE_LOOK_TOP_COUNT_OF_SEQUENCE: &[LexPatternAtom<'static>] = &[LexPattern::word("of")];
const REPLACE_LOOK_TOP_COUNT_TAIL_SEQUENCES: &[&[LexPatternAtom<'static>]] = &[
    REPLACE_LOOK_TOP_COUNT_CARDS_OF_SEQUENCE,
    REPLACE_LOOK_TOP_COUNT_CARD_OF_SEQUENCE,
    REPLACE_LOOK_TOP_COUNT_OF_SEQUENCE,
];
const REPLACE_OPTIONAL_THE_ATOMS: &[LexPatternAtom<'static>] = &[LexPattern::word("the")];
const REPLACE_LOOK_TOP_THEN_EXILE_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["look", "at"]),
    LexPattern::optional(REPLACE_OPTIONAL_THE_ATOMS),
    LexPattern::word("top"),
    LexPattern::amount(
        "count",
        LexCaptureKind::UntilAnyPhrase(&[&["card", "of"], &["cards", "of"], &["of"]]),
    ),
    LexPattern::any_sequence(REPLACE_LOOK_TOP_COUNT_TAIL_SEQUENCES),
    LexPattern::object("owner", LexCaptureKind::UntilPhrase(&["library"])),
    LexPattern::word("library"),
    LexPattern::tail("followup", LexCaptureKind::Rest),
]);
const REPLACE_LOOK_TOP_EXILE_LOOKED_SEQUENCE: &[LexPatternAtom<'static>] =
    &[LexPattern::phrase(&["exile", "one", "of", "them"])];
const REPLACE_LOOK_TOP_EXILE_THOSE_SEQUENCE: &[LexPatternAtom<'static>] =
    &[LexPattern::phrase(&["exile", "one", "of", "those"])];
const REPLACE_LOOK_TOP_EXILE_THOSE_CARDS_SEQUENCE: &[LexPatternAtom<'static>] =
    &[LexPattern::phrase(&["exile", "one", "of", "those", "cards"])];
const REPLACE_LOOK_TOP_EXILE_SEQUENCES: &[&[LexPatternAtom<'static>]] = &[
    REPLACE_LOOK_TOP_EXILE_THOSE_CARDS_SEQUENCE,
    REPLACE_LOOK_TOP_EXILE_THOSE_SEQUENCE,
    REPLACE_LOOK_TOP_EXILE_LOOKED_SEQUENCE,
];
const REPLACE_LOOK_TOP_OPTIONAL_FOLLOWUP_LEAD_ATOMS: &[LexPatternAtom<'static>] =
    &[LexPattern::any_word(&["then", "and"])];
const REPLACE_LOOK_TOP_EXILE_FOLLOWUP_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::optional(REPLACE_LOOK_TOP_OPTIONAL_FOLLOWUP_LEAD_ATOMS),
    LexPattern::any_sequence(REPLACE_LOOK_TOP_EXILE_SEQUENCES),
    LexPattern::tail("tail", LexCaptureKind::Rest),
]);
const REPLACE_EXILE_RETURN_OPTIONAL_YOU_MAY_ATOMS: &[LexPatternAtom<'static>] =
    &[LexPattern::phrase(&["you", "may"])];
const REPLACE_EXILE_RETURN_OPTIONAL_YOU_ATOMS: &[LexPatternAtom<'static>] =
    &[LexPattern::word("you")];
const REPLACE_EXILE_THEN_RETURN_SAME_OBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::optional(REPLACE_EXILE_RETURN_OPTIONAL_YOU_MAY_ATOMS),
    LexPattern::optional(REPLACE_EXILE_RETURN_OPTIONAL_YOU_ATOMS),
    LexPattern::action("exile_clause", LexCaptureKind::UntilPhrase(&["then"])),
    LexPattern::word("then"),
    LexPattern::tail("return_clause", LexCaptureKind::OneOrMoreWords),
]);
const REPLACE_RETURN_WITH_COUNTER_ON_OBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action(
        "return_clause",
        LexCaptureKind::UntilLastPhrase(&["with"]),
    ),
    LexPattern::word("with"),
    LexPattern::modifier("counter", LexCaptureKind::UntilLastPhrase(&["on"])),
    LexPattern::word("on"),
    LexPattern::object("recipient", LexCaptureKind::OneOf(&["it", "them"])),
    LexPattern::tail("tail", LexCaptureKind::Rest),
]);
const REPLACE_EXILE_TOKEN_END_COMBAT_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action("verb", LexCaptureKind::OneOf(&["exile"])),
    LexPattern::object(
        "object",
        LexCaptureKind::OneOfPhrase(REPLACE_TOKEN_END_COMBAT_OBJECT_PHRASES),
    ),
    LexPattern::word("at"),
    LexPattern::modifier(
        "timing",
        LexCaptureKind::OneOfPhrase(REPLACE_TOKEN_END_COMBAT_TIMING_PHRASES),
    ),
]);
const REPLACE_SACRIFICE_TOKEN_END_COMBAT_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action("verb", LexCaptureKind::OneOf(&["sacrifice"])),
    LexPattern::object(
        "object",
        LexCaptureKind::OneOfPhrase(REPLACE_TOKEN_END_COMBAT_OBJECT_PHRASES),
    ),
    LexPattern::word("at"),
    LexPattern::modifier(
        "timing",
        LexCaptureKind::OneOfPhrase(REPLACE_TOKEN_END_COMBAT_TIMING_PHRASES),
    ),
]);
const REPLACE_TAKE_EXTRA_TURN_YOU_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action("take", LexCaptureKind::OneOf(&["take"])),
    LexPattern::phrase(&["an", "extra", "turn"]),
    LexPattern::modifier(
        "anchor",
        LexCaptureKind::OneOfPhrase(&[&["after", "this", "one"]]),
    ),
]);
const REPLACE_TAKE_EXTRA_TURN_CHOSEN_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::subject(
        "player",
        LexCaptureKind::OneOfPhrase(&[&["the", "chosen", "player"]]),
    ),
    LexPattern::action("take", LexCaptureKind::OneOf(&["takes"])),
    LexPattern::phrase(&["an", "extra", "turn"]),
    LexPattern::modifier(
        "anchor",
        LexCaptureKind::OneOfPhrase(&[&["after", "this", "one"]]),
    ),
]);
const REPLACE_TAKE_EXTRA_TURN_THAT_AFTER_REFERENCED_PATTERN: LexPattern<'static> =
    LexPattern::new(&[
        LexPattern::modifier("anchor", LexCaptureKind::OneOfPhrase(&[&["after", "that", "turn"]])),
        LexPattern::subject("player", LexCaptureKind::OneOfPhrase(&[&["that", "player"]])),
        LexPattern::action("take", LexCaptureKind::OneOf(&["takes"])),
        LexPattern::phrase(&["an", "extra", "turn"]),
    ]);
const REPLACE_FOR_EACH_COUNTER_REMOVED_THIS_WAY_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["for", "each", "counter", "removed", "this", "way"]),
    LexPattern::subject(
        "subject",
        LexCaptureKind::UntilAnyPhrase(&[&["get"], &["gets"]]),
    ),
    LexPattern::action("action", LexCaptureKind::OneOf(&["get", "gets"])),
    LexPattern::modifier("modifier", LexCaptureKind::WordCount(1)),
    LexPattern::tail("tail", LexCaptureKind::Rest),
]);
const REPLACE_DESTROY_ALL_SPLIT_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action("verb", LexCaptureKind::OneOf(&["destroy"])),
    LexPattern::word("all"),
    LexPattern::object("objects", LexCaptureKind::Rest),
]);
const REPLACE_EXILE_ALL_SPLIT_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action("verb", LexCaptureKind::OneOf(&["exile"])),
    LexPattern::word("all"),
    LexPattern::object("objects", LexCaptureKind::Rest),
]);
const REPLACE_DESTROY_EXILE_ALL_OBJECT_LIST_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::object("first_objects", LexCaptureKind::UntilPhrase(&["and"])),
    LexPattern::word("and"),
    LexPattern::tail("remaining_objects", LexCaptureKind::OneOrMoreWords),
]);
const REPLACE_EXILE_UP_TO_ONE_EACH_TARGET_TYPE_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("exile"),
    LexPattern::object("target_clauses", LexCaptureKind::Rest),
]);
const REPLACE_MONSTROSITY_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("monstrosity"),
    LexPattern::amount("amount", LexCaptureKind::Rest),
]);
const REPLACE_ADDITIONAL_PHASE_EXISTENCE_PHRASES: &[&[&str]] = &[
    &["there", "is"],
    &["theres"],
    &["there", "are"],
    &["therere"],
];
const REPLACE_ADDITIONAL_PHASE_TYPE_PHRASES: &[&[&str]] =
    &[&["combat", "phase"], &["combat", "phases"]];
const REPLACE_ADDITIONAL_PHASE_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::condition(
        "intro",
        LexCaptureKind::UntilAnyPhrase(REPLACE_ADDITIONAL_PHASE_EXISTENCE_PHRASES),
    ),
    LexPattern::any_phrase(REPLACE_ADDITIONAL_PHASE_EXISTENCE_PHRASES),
    LexPattern::amount("count", LexCaptureKind::OneOf(&["an", "two"])),
    LexPattern::word("additional"),
    LexPattern::object(
        "phase",
        LexCaptureKind::OneOfPhrase(REPLACE_ADDITIONAL_PHASE_TYPE_PHRASES),
    ),
    LexPattern::tail("tail", LexCaptureKind::Rest),
]);
const REPLACE_EMPTY_PATTERN: LexPattern<'static> = LexPattern::new(&[]);
const REPLACE_ADDITIONAL_PHASE_AFTER_THIS_PHASE_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["after", "this", "phase"])]);
const REPLACE_ADDITIONAL_PHASE_AFTER_THIS_COMBAT_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["after", "this", "combat", "phase"])]);
const REPLACE_ADDITIONAL_PHASE_AFTER_THIS_MAIN_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["after", "this", "main", "phase"])]);
const REPLACE_ADDITIONAL_PHASE_IF_MAIN_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::any_phrase(&[
        &["if", "it", "your", "main", "phase"],
        &["if", "its", "your", "main", "phase"],
    ]),
]);
const REPLACE_ADDITIONAL_PHASE_FOLLOWED_BY_MAIN_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["followed", "by", "an", "additional", "main", "phase"]),
]);
const REPLACE_ADDITIONAL_PHASE_AFTER_THIS_FOLLOWED_BY_MAIN_PATTERN: LexPattern<'static> =
    LexPattern::new(&[
        LexPattern::phrase(&["after", "this", "phase"]),
        LexPattern::phrase(&["followed", "by", "an", "additional", "main", "phase"]),
    ]);
const REPLACE_ADDITIONAL_PHASE_ONE_COUNT_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::amount("count", LexCaptureKind::OneOf(&["an"]))]);
const REPLACE_ADDITIONAL_PHASE_TWO_COUNT_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::amount("count", LexCaptureKind::OneOf(&["two"]))]);
const REPLACE_ADDITIONAL_PHASE_COMBAT_SINGULAR_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "phase",
        LexCaptureKind::OneOfPhrase(&[&["combat", "phase"]]),
    )]);
const REPLACE_ADDITIONAL_PHASE_COMBAT_PLURAL_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "phase",
        LexCaptureKind::OneOfPhrase(&[&["combat", "phases"]]),
    )]);

fn replace_up_to_one_target_prefix_len(tokens: &[OwnedLexToken]) -> Option<usize> {
    let (count, used) = parse_choice_count_before_target_prefix(tokens)?;
    (count == ChoiceCount::up_to(1)).then_some(used)
}

fn replace_up_to_one_target_segments<'a>(target_clauses: LexedClause<'a>) -> Vec<LexedClause<'a>> {
    target_clauses
        .trimmed_and_comma_segments()
        .into_iter()
        .flat_map(|segment| split_lexed_slices_on_or(segment.tokens()))
        .map(LexedClause::new)
        .map(LexedClause::trimmed)
        .filter(|segment| !segment.is_empty())
        .collect()
}

pub(crate) fn parse_monstrosity_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = REPLACE_MONSTROSITY_PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(amount_clause) = matched.capture_clause_by_role(LexCaptureRole::Amount, clause) else {
        return Ok(None);
    };

    let (amount, _) = parse_value(amount_clause.trimmed().tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing monstrosity amount (clause: '{}')",
            render_token_slice(clause.tokens()).trim()
        ))
    })?;

    Ok(Some(EffectAst::subject_verb_monstrosity(amount)))
}

pub(crate) fn parse_for_each_counter_removed_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = REPLACE_FOR_EACH_COUNTER_REMOVED_THIS_WAY_PATTERN.match_clause(clause)
    else {
        return Ok(None);
    };

    let Some(subject_clause) = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)
    else {
        return Ok(None);
    };
    let Some(modifier_clause) = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)
    else {
        return Ok(None);
    };

    let subject = parse_subject(subject_clause.trimmed().tokens());
    let target = match subject {
        SubjectAst::This => TargetAst::Source(None),
        _ => return Ok(None),
    };

    let modifier_token = modifier_clause
        .trimmed()
        .tokens()
        .first()
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier (clause: '{}')",
                render_token_slice(clause.tokens()).trim()
            ))
        })?;
    let (power, toughness) = parse_pt_modifier(modifier_token)?;

    Ok(Some(EffectAst::subject_verb_pump_by_last_effect(
        power,
        toughness,
        target,
        Until::EndOfTurn,
    )))
}

pub(crate) fn is_exile_that_token_at_end_of_combat(tokens: &[OwnedLexToken]) -> bool {
    has_token_at_end_of_combat_shape(tokens, "exile")
}

pub(crate) fn is_exile_that_token_at_end_of_combat_lexed(tokens: &[OwnedLexToken]) -> bool {
    has_token_at_end_of_combat_shape(tokens, "exile")
}

pub(crate) fn is_sacrifice_that_token_at_end_of_combat(tokens: &[OwnedLexToken]) -> bool {
    has_token_at_end_of_combat_shape(tokens, "sacrifice")
}

pub(crate) fn is_sacrifice_that_token_at_end_of_combat_lexed(tokens: &[OwnedLexToken]) -> bool {
    has_token_at_end_of_combat_shape(tokens, "sacrifice")
}

fn has_token_at_end_of_combat_shape(tokens: &[OwnedLexToken], expected_verb: &str) -> bool {
    let clause = LexedClause::new(tokens).trimmed();
    match expected_verb {
        "exile" => REPLACE_EXILE_TOKEN_END_COMBAT_PATTERN
            .match_clause(clause)
            .is_some(),
        "sacrifice" => REPLACE_SACRIFICE_TOKEN_END_COMBAT_PATTERN
            .match_clause(clause)
            .is_some(),
        _ => false,
    }
}

pub(crate) fn parse_take_extra_turn_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    if REPLACE_TAKE_EXTRA_TURN_YOU_PATTERN
        .match_clause(clause)
        .is_some()
    {
        return Ok(Some(EffectAst::subject_verb_extra_turn_after_turn(
            PlayerAst::You,
            ExtraTurnAnchorAst::CurrentTurn,
        )));
    }
    if REPLACE_TAKE_EXTRA_TURN_CHOSEN_PATTERN
        .match_clause(clause)
        .is_some()
    {
        return Ok(Some(EffectAst::subject_verb_extra_turn_after_turn(
            PlayerAst::Chosen,
            ExtraTurnAnchorAst::CurrentTurn,
        )));
    }
    if REPLACE_TAKE_EXTRA_TURN_THAT_AFTER_REFERENCED_PATTERN
        .match_clause(clause)
        .is_some()
    {
        return Ok(Some(EffectAst::subject_verb_extra_turn_after_turn(
            PlayerAst::That,
            ExtraTurnAnchorAst::ReferencedTurn,
        )));
    }
    Ok(None)
}

pub(crate) fn parse_additional_phase_sentence(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    use crate::effects::AdditionalPhase;

    #[derive(Clone, Copy)]
    enum AdditionalPhaseIntro {
        Empty,
        AfterThisPhase,
        AfterThisCombatPhase,
        AfterThisMainPhase,
        IfYourMainPhase,
    }

    #[derive(Clone, Copy)]
    enum AdditionalPhaseTail {
        Empty,
        AfterThisPhase,
        FollowedByMain,
        AfterThisPhaseFollowedByMain,
    }

    let clause = LexedClause::new(tokens).trimmed();
    let matched = REPLACE_ADDITIONAL_PHASE_PATTERN.match_clause(clause)?;
    let intro_clause = matched.capture_clause_by_role(LexCaptureRole::Condition, clause)?;
    let count_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let phase_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let tail_clause = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;

    let intro_clause = intro_clause.trimmed();
    let intro = if REPLACE_EMPTY_PATTERN.matches_clause(intro_clause) {
        AdditionalPhaseIntro::Empty
    } else if REPLACE_ADDITIONAL_PHASE_AFTER_THIS_PHASE_PATTERN.matches_clause(intro_clause) {
        AdditionalPhaseIntro::AfterThisPhase
    } else if REPLACE_ADDITIONAL_PHASE_AFTER_THIS_COMBAT_PATTERN.matches_clause(intro_clause) {
        AdditionalPhaseIntro::AfterThisCombatPhase
    } else if REPLACE_ADDITIONAL_PHASE_AFTER_THIS_MAIN_PATTERN.matches_clause(intro_clause) {
        AdditionalPhaseIntro::AfterThisMainPhase
    } else if REPLACE_ADDITIONAL_PHASE_IF_MAIN_PATTERN.matches_clause(intro_clause) {
        AdditionalPhaseIntro::IfYourMainPhase
    } else {
        return None;
    };
    let tail_clause = tail_clause.trimmed();
    let tail = if REPLACE_EMPTY_PATTERN.matches_clause(tail_clause) {
        AdditionalPhaseTail::Empty
    } else if REPLACE_ADDITIONAL_PHASE_AFTER_THIS_PHASE_PATTERN.matches_clause(tail_clause) {
        AdditionalPhaseTail::AfterThisPhase
    } else if REPLACE_ADDITIONAL_PHASE_FOLLOWED_BY_MAIN_PATTERN.matches_clause(tail_clause) {
        AdditionalPhaseTail::FollowedByMain
    } else if REPLACE_ADDITIONAL_PHASE_AFTER_THIS_FOLLOWED_BY_MAIN_PATTERN
        .matches_clause(tail_clause)
    {
        AdditionalPhaseTail::AfterThisPhaseFollowedByMain
    } else {
        return None;
    };
    let count_clause = count_clause.trimmed();
    let phase_clause = phase_clause.trimmed();
    let count = if REPLACE_ADDITIONAL_PHASE_ONE_COUNT_PATTERN.matches_clause(count_clause)
        && REPLACE_ADDITIONAL_PHASE_COMBAT_SINGULAR_PATTERN.matches_clause(phase_clause)
    {
        1
    } else if REPLACE_ADDITIONAL_PHASE_TWO_COUNT_PATTERN.matches_clause(count_clause)
        && REPLACE_ADDITIONAL_PHASE_COMBAT_PLURAL_PATTERN.matches_clause(phase_clause)
    {
        2
    } else {
        return None;
    };

    let followed_by_main = match (intro, tail) {
        (
            AdditionalPhaseIntro::AfterThisPhase | AdditionalPhaseIntro::AfterThisMainPhase,
            AdditionalPhaseTail::FollowedByMain,
        )
        | (
            AdditionalPhaseIntro::Empty | AdditionalPhaseIntro::IfYourMainPhase,
            AdditionalPhaseTail::AfterThisPhaseFollowedByMain,
        ) => true,
        (
            AdditionalPhaseIntro::AfterThisPhase
            | AdditionalPhaseIntro::AfterThisCombatPhase
            | AdditionalPhaseIntro::AfterThisMainPhase,
            AdditionalPhaseTail::Empty,
        )
        | (AdditionalPhaseIntro::Empty, AdditionalPhaseTail::AfterThisPhase) => false,
        _ => return None,
    };

    let phases = if followed_by_main {
        if count != 1 {
            return None;
        }
        vec![AdditionalPhase::Combat, AdditionalPhase::Main]
    } else if count == 2 {
        vec![AdditionalPhase::Combat, AdditionalPhase::Combat]
    } else {
        vec![AdditionalPhase::Combat]
    };
    Some(EffectAst::subject_verb_additional_phases(phases))
}

pub(crate) fn parse_destroy_or_exile_all_split_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let (verb, matched) =
        if let Some(matched) = REPLACE_DESTROY_ALL_SPLIT_PATTERN.match_clause(clause) {
            (Verb::Destroy, matched)
        } else if let Some(matched) = REPLACE_EXILE_ALL_SPLIT_PATTERN.match_clause(clause) {
            (Verb::Exile, matched)
        } else {
            return Ok(None);
        };
    let Some(objects_clause) = matched.capture_clause_by_role(LexCaptureRole::Object, clause)
    else {
        return Ok(None);
    };

    if REPLACE_DESTROY_EXILE_ALL_OBJECT_LIST_PATTERN
        .match_clause(objects_clause.trimmed())
        .is_none()
        || destroy_exile_split_has_exception(clause)
    {
        return Ok(None);
    }
    if destroy_exile_split_is_temporary_exile_until_leaves_battlefield(clause, verb) {
        return Ok(None);
    }
    if destroy_exile_split_is_multi_zone_card_exile(clause) {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for segment_clause in objects_clause.trimmed_and_comma_segments() {
        let mut segment = segment_clause.tokens().to_vec();
        if segment.is_empty() {
            continue;
        }
        if token_slice_first_is(&segment, "all") {
            segment.remove(0);
        }
        if segment.is_empty() {
            continue;
        }
        let filter = parse_object_filter(&segment, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in split all clause (clause: '{}')",
                render_token_slice(clause.tokens()).trim()
            ))
        })?;
        let effect = match verb {
            Verb::Destroy => EffectAst::subject_verb_destroy_all(filter),
            Verb::Exile => EffectAst::subject_verb_exile_all(filter, false),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported split all clause verb".to_string(),
                ));
            }
        };
        effects.push(effect);
    }

    if effects.len() >= 2 {
        return Ok(Some(effects));
    }
    Ok(None)
}

fn destroy_exile_split_has_exception(clause: LexedClause<'_>) -> bool {
    REPLACE_EXCEPT_MARKER_PATTERN.matches(clause)
}

fn destroy_exile_split_is_temporary_exile_until_leaves_battlefield(
    clause: LexedClause<'_>,
    verb: Verb,
) -> bool {
    matches!(verb, Verb::Exile) && REPLACE_EXILE_UNTIL_LEAVES_BATTLEFIELD_PATTERN.matches(clause)
}

fn destroy_exile_split_is_multi_zone_card_exile(clause: LexedClause<'_>) -> bool {
    let Some(matched) = REPLACE_EXILE_ALL_CARDS_FROM_ZONES_PATTERN.match_clause(clause) else {
        return false;
    };
    let Some(zones_clause) = matched.capture_clause_by_role(LexCaptureRole::Tail, clause) else {
        return false;
    };
    let zones_clause = zones_clause.trimmed();
    let Some(zone_match) = REPLACE_ZONE_LIST_PATTERN.match_clause(zones_clause) else {
        return false;
    };
    let Some(first_zone) = zone_match.capture_clause("first_zone", zones_clause) else {
        return false;
    };
    let Some(second_zone) = zone_match.capture_clause_by_role(LexCaptureRole::Tail, zones_clause)
    else {
        return false;
    };
    let first_zone = first_zone.trimmed();
    let second_zone = second_zone.trimmed();
    let first_is_hand = REPLACE_HAND_ZONE_PATTERN.matches_clause(first_zone);
    let first_is_graveyard = REPLACE_GRAVEYARD_ZONE_PATTERN.matches_clause(first_zone);
    let second_is_hand = REPLACE_HAND_ZONE_PATTERN.matches_clause(second_zone);
    let second_is_graveyard = REPLACE_GRAVEYARD_ZONE_PATTERN.matches_clause(second_zone);
    (first_is_hand && second_is_graveyard) || (first_is_graveyard && second_is_hand)
}

pub(crate) fn parse_exile_then_return_same_object_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn target_references_it_tag(target: &TargetAst) -> bool {
        match target {
            TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
            TargetAst::Object(filter, _, _) => filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
            }),
            _ => false,
        }
    }

    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = REPLACE_EXILE_THEN_RETURN_SAME_OBJECT_PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(exile_clause) = matched.capture_clause("exile_clause", clause) else {
        return Ok(None);
    };
    let Some(return_clause) = matched.capture_clause("return_clause", clause) else {
        return Ok(None);
    };
    if !exile_clause.trimmed().first_is_word("exile")
        || !return_clause.trimmed().first_is_word("return")
    {
        return Ok(None);
    }

    let first_clause_tokens = trim_commas(exile_clause.trimmed().tokens());
    let second_clause_tokens = trim_commas(return_clause.trimmed().tokens());
    if first_clause_tokens.is_empty() || second_clause_tokens.is_empty() {
        return Ok(None);
    }

    let (first_clause, delayed_until_end_of_combat) = if let Some(before) =
        grammar::strip_lexed_suffix_phrase(
            &first_clause_tokens,
            &["at", "the", "end", "of", "combat"],
        )
    {
        (before, true)
    } else if let Some(before) =
        grammar::strip_lexed_suffix_phrase(&first_clause_tokens, &["at", "end", "of", "combat"])
    {
        (before, true)
    } else {
        (first_clause_tokens.as_slice(), false)
    };

    let mut first_effects = parse_effect_chain_inner(first_clause)?;
    if !first_effects
        .iter()
        .any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Exile { .. },
                    ..
                })
            )
        })
    {
        return Ok(None);
    }

    // Preserve return follow-up clauses (for example "with a +1/+1 counter on it")
    // while still rewriting the "it" return target to the tagged exiled object.
    let mut second_effects =
        if let Some(effects) =
            parse_sentence_return_with_counters_on_it(super::SubjectVerbPrimitiveClause::new(
                &second_clause_tokens,
            ))?
        {
            effects
        } else {
            parse_effect_chain_inner(&second_clause_tokens)?
        };
    let has_counter_followup = second_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutCounters { .. },
                ..
            })
        )
    });
    if !has_counter_followup {
        let return_clause = LexedClause::new(&second_clause_tokens).trimmed();
        if let Some(matched) =
            REPLACE_RETURN_WITH_COUNTER_ON_OBJECT_PATTERN.match_clause(return_clause)
            && let Some(counter_clause) =
                matched.capture_clause_by_role(LexCaptureRole::Modifier, return_clause)
        {
            let counter_tokens = trim_commas(counter_clause.trimmed().tokens());
            if counter_tokens.is_empty() {
                return Ok(None);
            }
            let (count, counter_type) =
                super::zone_counter_helpers::parse_counter_descriptor(&counter_tokens)?;
            second_effects.push(EffectAst::subject_verb_put_counters(
                counter_type,
                Value::Fixed(count as i32),
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                None,
                false,
            ));
        }
    }
    let mut rewrote_return = false;
    for effect in &mut second_effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ReturnToBattlefield { target, .. },
                ..
            }) if target_references_it_tag(target) => {
                *target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
                rewrote_return = true;
            }
            EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
                SubjectVerbActionAst::ReturnToHand { target, .. }
                    if target_references_it_tag(target) =>
                {
                    *target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
                    rewrote_return = true;
                }
                _ => {}
            },
            _ => {}
        }
    }
    if !rewrote_return {
        return Ok(None);
    }

    if delayed_until_end_of_combat {
        let mut delayed_effects = first_effects;
        delayed_effects.extend(second_effects);
        return Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
            effects: delayed_effects,
        }]));
    }

    first_effects.extend(second_effects);
    Ok(Some(first_effects))
}

pub(crate) fn parse_exile_up_to_one_each_target_type_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = REPLACE_EXILE_UP_TO_ONE_EACH_TARGET_TYPE_PATTERN.match_clause(clause)
    else {
        return Ok(None);
    };
    let Some(target_clauses) = matched.capture_clause_by_role(LexCaptureRole::Object, clause)
    else {
        return Ok(None);
    };
    let target_tokens = target_clauses.trimmed().tokens();
    if target_tokens.len() < 5 || replace_up_to_one_target_prefix_len(target_tokens).is_none() {
        return Ok(None);
    }

    // This primitive is for repeated clauses like:
    // "Exile up to one target artifact, up to one target creature, ..."
    // Not for a single disjunctive target like:
    // "Exile up to one target artifact, creature, or enchantment ..."
    let target_positions: Vec<usize> = target_tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| {
            REPLACE_TARGET_WORD_PATTERN.matches_token(token).then_some(idx)
        })
        .collect();
    if target_positions.len() < 2 {
        return Ok(None);
    }
    for pos in target_positions.iter().skip(1) {
        if *pos < 3
            || replace_up_to_one_target_prefix_len(&target_tokens[*pos - 3..=*pos]).is_none()
        {
            return Ok(None);
        }
    }

    let mut filters = Vec::new();
    for segment_clause in replace_up_to_one_target_segments(target_clauses.trimmed()) {
        let mut slice = segment_clause.tokens();
        if let Some(used) = replace_up_to_one_target_prefix_len(slice) {
            slice = &slice[used..];
        }
        if token_slice_first_is(slice, "target") {
            slice = &slice[1..];
        }
        if slice.is_empty() {
            continue;
        }

        let mut filter = parse_object_filter(slice, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in 'exile up to one each target type' clause (clause: '{}')",
                render_token_slice(clause.tokens()).trim()
            ))
        })?;
        if filter.controller.is_none() {
            // Keep this unrestricted to avoid implicit "you control" defaulting in ChooseObjects compilation.
            filter.controller = Some(PlayerFilter::Any);
        }
        filters.push(filter);
    }

    if filters.len() < 2 {
        return Ok(None);
    }

    let tag = helper_tag_for_tokens(tokens, "exiled");
    let mut effects: Vec<EffectAst> = filters
        .into_iter()
        .map(|filter| EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
        })
        .collect();
    effects.push(EffectAst::subject_verb_exile(TargetAst::Tagged(tag, None), false));

    Ok(Some(effects))
}

pub(crate) fn parse_look_at_hand_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = REPLACE_LOOK_HAND_PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(player_clause) = matched.capture_clause_by_role(LexCaptureRole::Object, clause)
    else {
        return Ok(None);
    };
    let Some(followup_clause) = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };
    let player_clause = player_clause.trimmed();
    let target = if REPLACE_LOOK_HAND_TARGET_PLAYER_PATTERN.matches_clause(player_clause) {
        TargetAst::Player(PlayerFilter::target_player(), Some(TextSpan::synthetic()))
    } else if REPLACE_LOOK_HAND_TARGET_OPPONENT_PATTERN.matches_clause(player_clause) {
        TargetAst::Player(PlayerFilter::target_opponent(), Some(TextSpan::synthetic()))
    } else if REPLACE_LOOK_HAND_OPPONENT_PATTERN.matches_clause(player_clause) {
        TargetAst::Player(PlayerFilter::Opponent, None)
    } else if REPLACE_LOOK_HAND_ITERATED_PLAYER_PATTERN.matches_clause(player_clause) {
        TargetAst::Player(PlayerFilter::IteratedPlayer, None)
    } else {
        return Ok(None);
    };
    let followup_clause = followup_clause.trimmed();
    let mut effects = vec![EffectAst::subject_verb_look_at_hand(target)];
    if REPLACE_LOOK_HAND_CHOOSE_NAME_PATTERN
        .match_clause(followup_clause)
        .is_some()
    {
        effects.push(EffectAst::subject_verb_choose_card_name(
            PlayerAst::You,
            None,
            TagKey::from(IT_TAG),
        ));
    } else if !followup_clause.tokens().is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}

pub(crate) fn parse_look_at_top_then_exile_one_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = REPLACE_LOOK_TOP_THEN_EXILE_PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(count_clause) = matched.capture_clause_by_role(LexCaptureRole::Amount, clause) else {
        return Ok(None);
    };
    let Some((count, used_count)) = parse_number(count_clause.trimmed().tokens()) else {
        return Ok(None);
    };
    if used_count != count_clause.trimmed().tokens().len() {
        return Ok(None);
    }
    let Some(owner_clause) = matched.capture_clause_by_role(LexCaptureRole::Object, clause) else {
        return Ok(None);
    };
    let player = match parse_subject(owner_clause.trimmed().tokens()) {
        SubjectAst::Player(player) => player,
        _ => return Ok(None),
    };

    let Some(followup_clause) = matched.capture_clause_by_role(LexCaptureRole::Tail, clause) else {
        return Ok(None);
    };
    if !REPLACE_LOOK_TOP_EXILE_FOLLOWUP_PATTERN.matches_prefix(followup_clause.trimmed()) {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(tokens, "looked");
    let chosen_tag = helper_tag_for_tokens(tokens, "chosen");
    let mut looked_filter = ObjectFilter::tagged(looked_tag.clone());
    looked_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, Value::Fixed(count as i32), looked_tag),
        EffectAst::ChooseObjects {
            filter: looked_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag.clone(),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(chosen_tag, None), false),
    ]))
}

pub(crate) fn parse_gain_life_equal_to_age_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Legacy fallback previously returned a hardcoded 0-life effect for age-counter clauses.
    // Let generic life parsing handle these so counter-scaled amounts compile correctly.
    let _ = tokens;
    Ok(None)
}

pub(crate) fn parse_you_and_each_opponent_voted_with_you_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = REPLACE_VOTED_WITH_YOU_SCRY_PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(count_clause) = matched.capture_clause_by_role(LexCaptureRole::Amount, clause) else {
        return Ok(None);
    };

    let Some((count, _)) = parse_value(count_clause.trimmed().tokens()) else {
        return Err(CardTextError::ParseError(format!(
            "missing scry count in vote-with-you clause (clause: '{}')",
            render_token_slice(clause.tokens()).trim()
        )));
    };

    let you_effect = EffectAst::May {
        effects: vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::Chooser,
            PlayerAst::You,
            SubjectVerbActionAst::Scry {
                count: count.clone(),
            },
        )],
    };

    let opponent_effect = EffectAst::ForEachTaggedPlayer {
        tag: TagKey::from("voted_with_you"),
        effects: vec![EffectAst::May {
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::Chooser,
                PlayerAst::Implicit,
                SubjectVerbActionAst::Scry { count },
            )],
        }],
    };

    Ok(Some(vec![you_effect, opponent_effect]))
}

#[cfg(test)]
mod replacement_and_prevention_shape_tests {
    use super::*;

    #[test]
    fn look_top_then_exile_one_uses_captured_count_owner_and_followup() {
        let tokens = crate::runtime_backend::lex_line(
            "Look at the top three cards of your library, then exile one of those cards.",
            0,
        )
        .expect("look-top exile-one text should lex");

        let effects = parse_look_at_top_then_exile_one_sentence(&tokens)
            .expect("look-top exile-one parser should not error")
            .expect("look-top exile-one parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("LookAtTopCards"), "{debug}");
        assert!(debug.contains("Fixed") && debug.contains("3"), "{debug}");
        assert!(debug.contains("ChooseObjects"), "{debug}");
        assert!(debug.contains("Exile"), "{debug}");
    }

    #[test]
    fn exile_then_return_same_object_uses_captured_clauses_and_counter_followup() {
        let tokens = crate::runtime_backend::lex_line(
            "You may exile target artifact or creature, then return it to the battlefield under its owner's control with a +1/+1 counter on it.",
            0,
        )
        .expect("exile-return text should lex");

        let effects = parse_exile_then_return_same_object_sentence(&tokens)
            .expect("exile-return parser should not error")
            .expect("exile-return parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("Exile"), "{debug}");
        assert!(debug.contains("ReturnToBattlefield"), "{debug}");
        assert!(debug.contains("PutCounters"), "{debug}");
        assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    }

    #[test]
    fn token_end_of_combat_recognizer_uses_captured_verb_object_and_timing() {
        let exile_tokens =
            crate::runtime_backend::lex_line("Exile those tokens at end of combat.", 0)
                .expect("exile token end-combat text should lex");
        assert!(is_exile_that_token_at_end_of_combat(&exile_tokens));
        assert!(!is_sacrifice_that_token_at_end_of_combat(
            &exile_tokens
        ));

        let sacrifice_tokens =
            crate::runtime_backend::lex_line("Sacrifice it at the end of combat.", 0)
                .expect("sacrifice token end-combat text should lex");
        assert!(is_sacrifice_that_token_at_end_of_combat_lexed(
            &sacrifice_tokens
        ));
        assert!(!is_exile_that_token_at_end_of_combat_lexed(
            &sacrifice_tokens
        ));
    }

    #[test]
    fn extra_turn_parser_uses_captured_subject_action_and_anchor() {
        let you_tokens = crate::runtime_backend::lex_line("Take an extra turn after this one.", 0)
            .expect("you extra-turn text should lex");
        let chosen_tokens = crate::runtime_backend::lex_line(
            "The chosen player takes an extra turn after this one.",
            0,
        )
        .expect("chosen-player extra-turn text should lex");
        let that_tokens = crate::runtime_backend::lex_line(
            "After that turn that player takes an extra turn.",
            0,
        )
        .expect("that-player referenced-turn text should lex");

        let you_effect = parse_take_extra_turn_sentence(&you_tokens)
            .expect("you extra-turn parser should not error")
            .expect("you extra-turn parser should match");
        let chosen_effect = parse_take_extra_turn_sentence(&chosen_tokens)
            .expect("chosen extra-turn parser should not error")
            .expect("chosen extra-turn parser should match");
        let that_effect = parse_take_extra_turn_sentence(&that_tokens)
            .expect("that extra-turn parser should not error")
            .expect("that extra-turn parser should match");
        let debug = format!("{you_effect:#?}\n{chosen_effect:#?}\n{that_effect:#?}");

        assert!(debug.contains("ExtraTurnAfterTurn"), "{debug}");
        assert!(debug.contains("You"), "{debug}");
        assert!(debug.contains("Chosen"), "{debug}");
        assert!(debug.contains("That"), "{debug}");
        assert!(debug.contains("CurrentTurn"), "{debug}");
        assert!(debug.contains("ReferencedTurn"), "{debug}");
    }

    #[test]
    fn additional_phase_parser_uses_captured_count_and_tail() {
        let one_combat_tokens = crate::runtime_backend::lex_line(
            "After this phase, there is an additional combat phase.",
            0,
        )
        .expect("single additional combat text should lex");
        let two_combat_tokens = crate::runtime_backend::lex_line(
            "After this main phase, there are two additional combat phases.",
            0,
        )
        .expect("two additional combats text should lex");
        let combat_main_tokens = crate::runtime_backend::lex_line(
            "After this main phase, there is an additional combat phase followed by an additional main phase.",
            0,
        )
        .expect("combat-then-main text should lex");

        let one_combat = parse_additional_phase_sentence(&one_combat_tokens)
            .expect("single additional combat parser should match");
        let two_combat = parse_additional_phase_sentence(&two_combat_tokens)
            .expect("two additional combats parser should match");
        let combat_main = parse_additional_phase_sentence(&combat_main_tokens)
            .expect("combat-then-main parser should match");
        let debug = format!("{one_combat:#?}\n{two_combat:#?}\n{combat_main:#?}");

        assert!(debug.contains("AdditionalPhases"), "{debug}");
        assert!(debug.contains("Combat"), "{debug}");
        assert!(debug.contains("Main"), "{debug}");
        assert_eq!(debug.matches("Combat").count(), 4, "{debug}");
        assert_eq!(debug.matches("Main").count(), 1, "{debug}");
    }

    #[test]
    fn look_at_hand_parser_uses_captured_player_and_followup() {
        let target_player_tokens =
            crate::runtime_backend::lex_line("Look at target player's hand.", 0)
                .expect("target-player hand text should lex");
        let opponent_choose_tokens =
            crate::runtime_backend::lex_line("Look at an opponent's hand, then choose any card name.", 0)
                .expect("opponent choose-name hand text should lex");
        let iterated_tokens =
            crate::runtime_backend::lex_line("Look at that player's hand.", 0)
                .expect("iterated-player hand text should lex");

        let target_player = parse_look_at_hand_sentence(&target_player_tokens)
            .expect("target-player hand parser should not error")
            .expect("target-player hand parser should match");
        let opponent_choose = parse_look_at_hand_sentence(&opponent_choose_tokens)
            .expect("opponent choose-name hand parser should not error")
            .expect("opponent choose-name hand parser should match");
        let iterated = parse_look_at_hand_sentence(&iterated_tokens)
            .expect("iterated-player hand parser should not error")
            .expect("iterated-player hand parser should match");
        let debug = format!("{target_player:#?}\n{opponent_choose:#?}\n{iterated:#?}");

        assert!(debug.contains("LookAtHand"), "{debug}");
        assert!(debug.contains("Target") && debug.contains("Any"), "{debug}");
        assert!(debug.contains("Opponent"), "{debug}");
        assert!(debug.contains("ChooseCardName"), "{debug}");
        assert!(debug.contains("IteratedPlayer"), "{debug}");
    }

    #[test]
    fn voted_with_you_scry_parser_uses_captured_count() {
        let tokens = crate::runtime_backend::lex_line(
            "You and each opponent who voted for a choice you voted for may scry 2.",
            0,
        )
        .expect("voted-with-you scry text should lex");

        let effects = parse_you_and_each_opponent_voted_with_you_sentence(&tokens)
            .expect("voted-with-you scry parser should not error")
            .expect("voted-with-you scry parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("May"), "{debug}");
        assert!(debug.contains("Scry"), "{debug}");
        assert!(debug.contains("Fixed") && debug.contains("2"), "{debug}");
        assert!(debug.contains("ForEachTaggedPlayer"), "{debug}");
        assert!(debug.contains("voted_with_you"), "{debug}");
    }

    #[test]
    fn for_each_counter_removed_uses_captured_subject_action_and_modifier() {
        let tokens = crate::runtime_backend::lex_line(
            "For each counter removed this way, this creature gets +1/+0 until end of turn.",
            0,
        )
        .expect("counter-removed pump text should lex");

        let effect = parse_for_each_counter_removed_sentence(&tokens)
            .expect("counter-removed parser should not error")
            .expect("counter-removed parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("PumpByLastEffect"), "{debug}");
        assert!(debug.contains("power: 1"), "{debug}");
        assert!(debug.contains("toughness: 0"), "{debug}");
        assert!(debug.contains("Source"), "{debug}");
    }

    #[test]
    fn destroy_all_split_uses_captured_verb_and_object_tail() {
        let tokens =
            crate::runtime_backend::lex_line("Destroy all artifacts and enchantments.", 0)
                .expect("destroy-all split text should lex");

        let effects = parse_destroy_or_exile_all_split_sentence(&tokens)
            .expect("destroy-all split parser should not error")
            .expect("destroy-all split parser should match");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 2, "{debug}");
        assert!(debug.contains("DestroyAll"), "{debug}");
        assert!(debug.contains("Artifact"), "{debug}");
        assert!(debug.contains("Enchantment"), "{debug}");
    }

    #[test]
    fn destroy_all_split_exclusions_use_clause_and_zone_captures() {
        let except_tokens =
            crate::runtime_backend::lex_line("Destroy all creatures except Elves and Goblins.", 0)
                .expect("except split text should lex");
        let temporary_exile_tokens = crate::runtime_backend::lex_line(
            "Exile all creatures and planeswalkers until this enchantment leaves the battlefield.",
            0,
        )
        .expect("temporary exile split text should lex");
        let multi_zone_tokens = crate::runtime_backend::lex_line(
            "Exile all cards from target player's graveyard and hand.",
            0,
        )
        .expect("multi-zone exile text should lex");

        assert!(
            parse_destroy_or_exile_all_split_sentence(&except_tokens)
                .expect("except split parser should not error")
                .is_none()
        );
        assert!(
            parse_destroy_or_exile_all_split_sentence(&temporary_exile_tokens)
                .expect("temporary exile parser should not error")
                .is_none()
        );
        assert!(
            parse_destroy_or_exile_all_split_sentence(&multi_zone_tokens)
                .expect("multi-zone exile parser should not error")
                .is_none()
        );
    }

    #[test]
    fn monstrosity_uses_captured_amount() {
        let tokens =
            crate::runtime_backend::lex_line("Monstrosity 3.", 0)
                .expect("monstrosity text should lex");

        let effect = parse_monstrosity_sentence(&tokens)
            .expect("monstrosity parser should not error")
            .expect("monstrosity parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("Monstrosity"), "{debug}");
        assert!(debug.contains("Fixed") && debug.contains("3"), "{debug}");
    }

    #[test]
    fn exile_up_to_one_each_target_type_uses_captured_target_clauses() {
        let tokens = crate::runtime_backend::lex_line(
            "Exile up to one target artifact, up to one target creature, and up to one target enchantment.",
            0,
        )
        .expect("exile repeated target type text should lex");

        let effects = parse_exile_up_to_one_each_target_type_sentence(&tokens)
            .expect("exile repeated target type parser should not error")
            .expect("exile repeated target type parser should match");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 4, "{debug}");
        assert_eq!(debug.matches("ChooseObjects").count(), 3, "{debug}");
        assert!(debug.contains("Artifact"), "{debug}");
        assert!(debug.contains("Creature"), "{debug}");
        assert!(debug.contains("Enchantment"), "{debug}");
        assert!(debug.contains("Exile"), "{debug}");
    }
}

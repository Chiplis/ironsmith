const DELAYED_ATTACKS_UNBLOCKED_PHRASES: &[&[&str]] = &[
    &["attacks", "and", "isn't", "blocked"],
    &["attacks", "and", "isnt", "blocked"],
];
const DELAYED_TARGET_ATTACK_UNBLOCKED_TRIGGER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("target"),
    LexPattern::object(
        "subject",
        LexCaptureKind::UntilAnyPhrase(DELAYED_ATTACKS_UNBLOCKED_PHRASES),
    ),
    LexPattern::any_phrase(DELAYED_ATTACKS_UNBLOCKED_PHRASES),
]);
const COPY_NEXT_THIS_TURN_DELAYED_TRIGGER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::modifier(
        "duration",
        LexCaptureKind::OneOfPhrase(&[&["this", "turn"]]),
    ),
    LexPattern::action("intro", LexCaptureKind::OneOf(&["when", "whenever"])),
    LexPattern::condition("trigger", LexCaptureKind::UntilToken(TokenKind::Comma)),
    LexPattern::tail("effect", LexCaptureKind::Rest),
]);
const DELAYED_TRIGGER_THIS_TURN_SUFFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action("intro", LexCaptureKind::OneOf(&["when", "whenever"])),
    LexPattern::condition(
        "trigger",
        LexCaptureKind::UntilLastPhraseBeforeToken(&["this", "turn"], TokenKind::Comma),
    ),
    LexPattern::modifier(
        "duration",
        LexCaptureKind::OneOfPhrase(&[&["this", "turn"]]),
    ),
    LexPattern::token(TokenKind::Comma),
    LexPattern::tail("effect", LexCaptureKind::Rest),
]);
const DELAYED_NEXT_TRIGGER_MARKER_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::capture(
        "next",
        LexCaptureKind::OneOf(&["next"]),
    )]);
const DELAYED_TAGGED_DEALT_DAMAGE_OPTIONAL_COMBAT_ATOMS: &[LexPatternAtom<'static>] =
    &[LexPattern::capture(
        "combat",
        LexCaptureKind::OneOf(&["combat"]),
    )];
const DELAYED_TAGGED_DEALT_DAMAGE_TRIGGER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("that"),
    LexPattern::object("kind", LexCaptureKind::OneOf(&["creature", "permanent"])),
    LexPattern::phrase(&["is", "dealt"]),
    LexPattern::optional(DELAYED_TAGGED_DEALT_DAMAGE_OPTIONAL_COMBAT_ATOMS),
    LexPattern::word("damage"),
]);
const DELAYED_THAT_DEALS_COMBAT_DAMAGE_TO_PLAYER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("that"),
    LexPattern::object("kind", LexCaptureKind::OneOf(&["creature", "permanent"])),
    LexPattern::phrase(&["deals", "combat", "damage", "to", "a", "player"]),
]);
const DELAYED_TAGGED_DAMAGE_CREATURE_KIND_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "kind",
        LexCaptureKind::OneOf(&["creature"]),
    )]);
const DELAYED_TAGGED_DAMAGE_PERMANENT_KIND_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "kind",
        LexCaptureKind::OneOf(&["permanent"]),
    )]);
const DELAYED_DIES_INTRO_WORDS: &[&str] = &["when", "whenever", "if"];
const DELAYED_DIES_THIS_TURN_PHRASE: &[&str] = &["dies", "this", "turn"];
const DELAYED_DIES_THIS_WAY_PHRASES: &[&[&str]] = &[
    &["dealt", "damage", "this", "way", "dies", "this", "turn"],
    &[
        "dealt", "damage", "this", "way", "would", "die", "this", "turn",
    ],
];
const DELAYED_THAT_DIES_THIS_TURN_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::any_word(DELAYED_DIES_INTRO_WORDS),
    LexPattern::word("that"),
    LexPattern::capture(
        "that_reference",
        LexCaptureKind::UntilPhrase(DELAYED_DIES_THIS_TURN_PHRASE),
    ),
    LexPattern::phrase(DELAYED_DIES_THIS_TURN_PHRASE),
    LexPattern::token(TokenKind::Comma),
    LexPattern::tail("effect", LexCaptureKind::Rest),
]);
const DELAYED_DIES_THIS_WAY_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::any_word(DELAYED_DIES_INTRO_WORDS),
    LexPattern::object(
        "subject",
        LexCaptureKind::UntilAnyPhrase(DELAYED_DIES_THIS_WAY_PHRASES),
    ),
    LexPattern::any_phrase(DELAYED_DIES_THIS_WAY_PHRASES),
    LexPattern::token(TokenKind::Comma),
    LexPattern::tail("effect", LexCaptureKind::Rest),
]);
const DELAYED_END_STEP_OPTIONAL_THE_ATOMS: &[LexPatternAtom<'static>] = &[LexPattern::word("the")];
const DELAYED_END_STEP_OWNER_WORDS: &[&str] = &["your"];
const DELAYED_END_STEP_YOUR_OWNER_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::capture(
        "owner",
        LexCaptureKind::OneOf(DELAYED_END_STEP_OWNER_WORDS),
    )]);
const DELAYED_END_STEP_THAT_PLAYER_OWNER_PHRASES: &[&[&str]] = &[
    &["that", "player"],
    &["that", "players"],
    &["that", "player's"],
    &["that", "players'"],
];
const DELAYED_END_STEP_THAT_PLAYER_OWNER_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::capture(
        "owner",
        LexCaptureKind::OneOfPhrase(DELAYED_END_STEP_THAT_PLAYER_OWNER_PHRASES),
    )]);
const DELAYED_END_STEP_TARGET_PLAYER_OWNER_PHRASES: &[&[&str]] = &[
    &["target", "player"],
    &["target", "players"],
    &["target", "player's"],
    &["target", "players'"],
];
const DELAYED_END_STEP_TARGET_PLAYER_OWNER_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::capture(
        "owner",
        LexCaptureKind::OneOfPhrase(DELAYED_END_STEP_TARGET_PLAYER_OWNER_PHRASES),
    )]);
const DELAYED_END_STEP_OPTIONAL_STEP_OWNER_ATOMS: &[LexPatternAtom<'static>] =
    &[LexPattern::capture(
        "step_owner",
        LexCaptureKind::OneOf(DELAYED_END_STEP_OWNER_WORDS),
    )];
const DELAYED_NEXT_END_STEP_SEQUENCE: &[LexPatternAtom<'static>] =
    &[LexPattern::phrase(&["next", "end", "step"])];
const DELAYED_END_STEP_SEQUENCE: &[LexPatternAtom<'static>] =
    &[LexPattern::phrase(&["end", "step"])];
const DELAYED_END_STEP_SEQUENCES: &[&[LexPatternAtom<'static>]] =
    &[DELAYED_NEXT_END_STEP_SEQUENCE, DELAYED_END_STEP_SEQUENCE];
const DELAYED_END_STEP_TURN_OWNER_TAIL_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("of"),
    LexPattern::capture("turn_owner", LexCaptureKind::UntilPhrase(&["next", "turn"])),
    LexPattern::phrase(&["next", "turn"]),
];
const DELAYED_END_STEP_HEADER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("at"),
    LexPattern::optional(DELAYED_END_STEP_OPTIONAL_THE_ATOMS),
    LexPattern::word("beginning"),
    LexPattern::word("of"),
    LexPattern::optional(DELAYED_END_STEP_OPTIONAL_THE_ATOMS),
    LexPattern::optional(DELAYED_END_STEP_OPTIONAL_STEP_OWNER_ATOMS),
    LexPattern::any_sequence(DELAYED_END_STEP_SEQUENCES),
    LexPattern::optional(DELAYED_END_STEP_TURN_OWNER_TAIL_ATOMS),
    LexPattern::token(TokenKind::Comma),
    LexPattern::tail("effect", LexCaptureKind::Rest),
]);

const DELAYED_NEXT_COMBAT_OPTIONAL_PHASE_ATOMS: &[LexPatternAtom<'static>] =
    &[LexPattern::word("phase")];
const DELAYED_NEXT_COMBAT_HEADER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["at", "the", "beginning", "of", "the", "next", "combat"]),
    LexPattern::optional(DELAYED_NEXT_COMBAT_OPTIONAL_PHASE_ATOMS),
    LexPattern::phrase(&["this", "turn"]),
    LexPattern::token(TokenKind::Comma),
    LexPattern::tail("effect", LexCaptureKind::Rest),
]);

/// "At the beginning of the next combat [phase] this turn, <effects>" — a
/// one-shot delayed trigger scheduled for the next beginning of combat,
/// expiring at end of turn.
pub(crate) fn parse_delayed_next_combat_phase_this_turn_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    if clause.is_empty() {
        return Ok(None);
    }
    let Some(matched) = DELAYED_NEXT_COMBAT_HEADER_PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(effect_clause) = matched.capture_clause_by_role(LexCaptureRole::Tail, clause) else {
        return Ok(None);
    };
    let remainder = effect_clause.trimmed().tokens();
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed next-combat-phase effect clause (clause: '{}')",
            crate::runtime_backend::lexer::render_token_slice(tokens).trim()
        )));
    }
    let delayed_effects = parse_effect_chain(remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed next-combat-phase effect clause (clause: '{}')",
            crate::runtime_backend::lexer::render_token_slice(tokens).trim()
        )));
    }
    Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
        trigger: TriggerSpec::BeginningOfCombat(PlayerFilter::Any),
        effects: delayed_effects,
        one_shot: true,
    }]))
}

fn delayed_end_step_player_from_owner(
    owner_clause: Option<LexedClause<'_>>,
) -> Option<PlayerFilter> {
    let Some(owner_clause) = owner_clause.map(LexedClause::trimmed) else {
        return Some(PlayerFilter::Any);
    };
    if owner_clause.is_empty() {
        return Some(PlayerFilter::Any);
    }
    if DELAYED_END_STEP_YOUR_OWNER_PATTERN.matches_clause(owner_clause) {
        return Some(PlayerFilter::You);
    }
    if DELAYED_END_STEP_THAT_PLAYER_OWNER_PATTERN.matches_clause(owner_clause) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if DELAYED_END_STEP_TARGET_PLAYER_OWNER_PATTERN.matches_clause(owner_clause) {
        return Some(PlayerFilter::Target(Box::new(PlayerFilter::Any)));
    }
    None
}

fn delayed_dies_this_way_filter(
    matched: &crate::runtime_backend::lex_patterns::LexPatternMatch<'_>,
    clause: LexedClause<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(subject_clause) = matched.capture_clause_by_role(LexCaptureRole::Object, clause)
    else {
        return Ok(None);
    };
    let clause_display = crate::runtime_backend::lexer::render_token_slice(clause.tokens());
    let mut subject_tokens = trim_edge_punctuation(subject_clause.tokens());
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object filter in delayed dies-this-way clause (clause: '{}')",
            clause_display.trim()
        )));
    }
    let stripped_subject = strip_leading_articles(&subject_tokens);
    if !stripped_subject.is_empty() {
        subject_tokens = stripped_subject;
    }
    parse_object_filter(&subject_tokens, false)
        .map(Some)
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported object filter in delayed dies-this-way clause (clause: '{}')",
                clause_display.trim()
            ))
        })
}

pub(crate) fn parse_delayed_until_next_end_step_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    if clause.is_empty() {
        return Ok(None);
    }

    let Some(matched) = DELAYED_END_STEP_HEADER_PATTERN.match_clause(clause) else {
        return Ok(None);
    };

    let mut player =
        delayed_end_step_player_from_owner(matched.capture_clause("step_owner", clause))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported delayed end-step owner (clause: '{}')",
                    crate::runtime_backend::lexer::render_token_slice(tokens).trim()
                ))
            })?;
    let start_next_turn = matched.capture("turn_owner").is_some();
    if let Some(turn_owner) = matched.capture_clause("turn_owner", clause) {
        player = delayed_end_step_player_from_owner(Some(turn_owner)).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported delayed end-step turn owner (clause: '{}')",
                crate::runtime_backend::lexer::render_token_slice(tokens).trim()
            ))
        })?;
    }

    let Some(effect_clause) = matched.capture_clause_by_role(LexCaptureRole::Tail, clause) else {
        return Ok(None);
    };
    let remainder = effect_clause.trimmed().tokens();
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(
            "missing delayed end-step effect clause".to_string(),
        ));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed end-step effect clause (clause: '{}')",
            crate::runtime_backend::lexer::render_token_slice(tokens).trim()
        )));
    }

    if start_next_turn {
        let player_ast = match player {
            PlayerFilter::You => PlayerAst::You,
            PlayerFilter::IteratedPlayer => PlayerAst::That,
            PlayerFilter::Target(_) => PlayerAst::Target,
            PlayerFilter::Opponent => PlayerAst::Opponent,
            _ => PlayerAst::Any,
        };
        Ok(Some(vec![EffectAst::DelayedUntilEndStepOfExtraTurn {
            player: player_ast,
            effects: delayed_effects,
        }]))
    } else {
        Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
            player,
            effects: delayed_effects,
        }]))
    }
}

fn retarget_source_copy_spell_to_delayed_triggering_object(effects: &mut [EffectAst]) {
    fn visit(effect: &mut EffectAst) {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::CopySpell { target, .. } = &mut subject_verb.action
            && matches!(target, TargetAst::Source(_))
        {
            *target = TargetAst::Tagged(TagKey::from("triggering"), None);
        }

        crate::runtime_backend::effect_ast_traversal::for_each_nested_effects_mut(
            effect,
            true,
            |nested| retarget_source_copy_spell_to_delayed_triggering_object(nested),
        );
    }

    for effect in effects {
        visit(effect);
    }
}

fn delayed_attack_unblocked_filter_from_trigger(
    trigger_tokens: &[OwnedLexToken],
    full_sentence_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let trigger_clause = LexedClause::new(trigger_tokens).trimmed();
    let Some(matched) =
        DELAYED_TARGET_ATTACK_UNBLOCKED_TRIGGER_PATTERN.match_clause(trigger_clause)
    else {
        return Ok(None);
    };
    let Some(subject_clause) =
        matched.capture_clause_by_role(LexCaptureRole::Object, trigger_clause)
    else {
        return Ok(None);
    };
    let subject_tokens = subject_clause.trimmed().tokens();
    let full_sentence_display =
        crate::runtime_backend::lexer::render_token_slice(full_sentence_tokens);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target subject for delayed attack trigger (clause: '{}')",
            full_sentence_display.trim()
        )));
    }

    parse_object_filter(subject_tokens, false)
        .map(Some)
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported delayed attack target filter (clause: '{}')",
                full_sentence_display.trim()
            ))
        })
}

fn delayed_tagged_dealt_damage_trigger_from_core(
    trigger_core_tokens: &[OwnedLexToken],
) -> Option<TriggerSpec> {
    let trigger_clause = LexedClause::new(trigger_core_tokens).trimmed();
    let matched = DELAYED_TAGGED_DEALT_DAMAGE_TRIGGER_PATTERN.match_clause(trigger_clause)?;
    let kind_clause = matched.capture_clause_by_role(LexCaptureRole::Object, trigger_clause)?;
    let kind_clause = kind_clause.trimmed();
    let mut filter = if DELAYED_TAGGED_DAMAGE_CREATURE_KIND_PATTERN.matches_clause(kind_clause) {
        ObjectFilter::creature()
    } else if DELAYED_TAGGED_DAMAGE_PERMANENT_KIND_PATTERN.matches_clause(kind_clause) {
        ObjectFilter::permanent()
    } else {
        return None;
    };
    filter = filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);

    if matched.capture("combat").is_some() {
        Some(TriggerSpec::IsDealtCombatDamage(filter))
    } else {
        Some(TriggerSpec::IsDealtDamage(filter))
    }
}

fn delayed_that_deals_combat_damage_to_player_trigger_from_core(
    trigger_core_tokens: &[OwnedLexToken],
) -> Option<TriggerSpec> {
    let trigger_clause = LexedClause::new(trigger_core_tokens).trimmed();
    let matched =
        DELAYED_THAT_DEALS_COMBAT_DAMAGE_TO_PLAYER_PATTERN.match_clause(trigger_clause)?;
    let kind_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, trigger_clause)?
        .trimmed();
    let mut filter = if DELAYED_TAGGED_DAMAGE_CREATURE_KIND_PATTERN.matches_clause(kind_clause) {
        ObjectFilter::creature()
    } else if DELAYED_TAGGED_DAMAGE_PERMANENT_KIND_PATTERN.matches_clause(kind_clause) {
        ObjectFilter::permanent()
    } else {
        return None;
    };
    filter = filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
    Some(TriggerSpec::DealsCombatDamageToPlayer {
        source: filter,
        player: PlayerFilter::Any,
    })
}

fn next_cast_instant_sorcery_or_loyalty_trigger_from_core(
    trigger_core_tokens: &[OwnedLexToken],
) -> Option<TriggerSpec> {
    let words = trigger_core_tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    let matches_split_instant_sorcery = words.as_slice()
        == [
            "you".to_string(),
            "next".to_string(),
            "cast".to_string(),
            "an".to_string(),
            "instant".to_string(),
            "spell".to_string(),
            "cast".to_string(),
            "a".to_string(),
            "sorcery".to_string(),
            "spell".to_string(),
            "or".to_string(),
            "activate".to_string(),
            "a".to_string(),
            "loyalty".to_string(),
            "ability".to_string(),
        ];
    let matches_joined_instant_sorcery = words.as_slice()
        == [
            "you".to_string(),
            "next".to_string(),
            "cast".to_string(),
            "an".to_string(),
            "instant".to_string(),
            "or".to_string(),
            "sorcery".to_string(),
            "spell".to_string(),
            "or".to_string(),
            "activate".to_string(),
            "a".to_string(),
            "loyalty".to_string(),
            "ability".to_string(),
        ];
    if !matches_split_instant_sorcery && !matches_joined_instant_sorcery {
        return None;
    }

    let spell_cast = TriggerSpec::SpellCast {
        filter: Some(ObjectFilter::instant_or_sorcery()),
        caster: PlayerFilter::You,
        during_turn: None,
        min_spells_this_turn: None,
        exact_spells_this_turn: None,
        from_not_hand: false,
    };
    let loyalty_activated = TriggerSpec::AbilityActivated {
        activator: PlayerFilter::You,
        filter: ObjectFilter::default(),
        non_mana_only: false,
        loyalty_only: true,
        activation_cost_has_tap: None,
    };
    Some(TriggerSpec::Either(
        Box::new(spell_cast),
        Box::new(loyalty_activated),
    ))
}

fn delayed_trigger_is_one_shot(trigger_clause: LexedClause<'_>) -> bool {
    DELAYED_NEXT_TRIGGER_MARKER_PATTERN
        .find_in_clause(trigger_clause.trimmed())
        .is_some()
}

fn delayed_trigger_provides_triggering_stack_object(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::SpellCast { .. } | TriggerSpec::AbilityActivated { .. } => true,
        TriggerSpec::Either(left, right) => {
            delayed_trigger_provides_triggering_stack_object(left)
                || delayed_trigger_provides_triggering_stack_object(right)
        }
        _ => false,
    }
}

fn parse_copy_that_spell_or_ability_twice_tail(
    effect_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let words = effect_tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    let copy_twice_prefix = [
        "copy".to_string(),
        "that".to_string(),
        "spell".to_string(),
        "or".to_string(),
        "ability".to_string(),
        "twice".to_string(),
    ];
    if !words.starts_with(&copy_twice_prefix) {
        return None;
    }
    let may_choose_new_targets = words[copy_twice_prefix.len()..]
        == [
            "you".to_string(),
            "may".to_string(),
            "choose".to_string(),
            "new".to_string(),
            "targets".to_string(),
            "for".to_string(),
            "the".to_string(),
            "copies".to_string(),
        ];
    if words.len() != copy_twice_prefix.len() && !may_choose_new_targets {
        return None;
    }

    Some(vec![EffectAst::subject_verb_copy_spell(
        TargetAst::Tagged(TagKey::from("triggering"), None),
        Value::Fixed(2),
        PlayerAst::Implicit,
        may_choose_new_targets,
        Vec::new(),
    )])
}

fn parse_next_cast_spell_or_loyalty_delayed_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let tokens = clause.tokens();
    let Some(first_word) = tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .map(str::to_ascii_lowercase)
    else {
        return Ok(None);
    };
    if !matches!(first_word.as_str(), "when" | "whenever") {
        return Ok(None);
    }
    let Some(this_turn_idx) = find_token_word_sequence(tokens, &["this", "turn"]) else {
        return Ok(None);
    };
    let trigger_tokens = tokens.get(1..this_turn_idx).unwrap_or_default();
    let Some(trigger) = next_cast_instant_sorcery_or_loyalty_trigger_from_core(trigger_tokens)
    else {
        return Ok(None);
    };
    let Some(comma_idx) = tokens
        .iter()
        .enumerate()
        .skip(this_turn_idx + 2)
        .find_map(|(idx, token)| (token.kind == TokenKind::Comma).then_some(idx))
    else {
        return Ok(None);
    };
    let effect_tokens = tokens.get(comma_idx + 1..).unwrap_or_default();
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed spell-or-loyalty effect clause (clause: '{}')",
            crate::runtime_backend::lexer::render_token_slice(tokens).trim()
        )));
    }

    let mut delayed_effects =
        if let Some(effects) = parse_copy_that_spell_or_ability_twice_tail(effect_tokens) {
            effects
        } else {
            parse_effect_chain(effect_tokens)?
        };
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed spell-or-loyalty effect clause (clause: '{}')",
            crate::runtime_backend::lexer::render_token_slice(tokens).trim()
        )));
    }
    retarget_source_copy_spell_to_delayed_triggering_object(&mut delayed_effects);
    Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
        trigger,
        effects: delayed_effects,
        one_shot: true,
    }]))
}

pub(crate) fn parse_sentence_delayed_trigger_this_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let clause_display = crate::runtime_backend::lexer::render_token_slice(clause.tokens());
    if DELAYED_THAT_DIES_THIS_TURN_PATTERN
        .match_clause(clause)
        .is_some()
        || DELAYED_DIES_THIS_WAY_PATTERN.match_clause(clause).is_some()
    {
        return parse_delayed_when_that_dies_this_turn_sentence(tokens);
    }

    if let Some(effects) = parse_next_cast_spell_or_loyalty_delayed_sentence(tokens)? {
        return Ok(Some(effects));
    }

    if let Some(matched) = COPY_NEXT_THIS_TURN_DELAYED_TRIGGER_PATTERN.match_clause(clause) {
        let Some(trigger_clause) =
            matched.capture_clause_by_role(LexCaptureRole::Condition, clause)
        else {
            return Ok(None);
        };
        let Some(effect_clause) = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)
        else {
            return Ok(None);
        };

        let trigger_tokens = trigger_clause.trimmed().tokens();
        if trigger_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing delayed trigger clause after 'this turn' (clause: '{}')",
                clause_display.trim()
            )));
        }

        let mut delayed_effects = parse_effect_chain(effect_clause.trimmed().tokens())?;
        if delayed_effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing delayed trigger effect clause (clause: '{}')",
                clause_display.trim()
            )));
        }

        if let Some(filter) = delayed_attack_unblocked_filter_from_trigger(trigger_tokens, tokens)?
        {
            let mut trigger_filter = filter.clone();
            trigger_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            return Ok(Some(vec![
                EffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                },
                EffectAst::DelayedTriggerThisTurn {
                    trigger: TriggerSpec::AttacksAndIsntBlocked(trigger_filter),
                    effects: delayed_effects,
                    one_shot: true,
                },
            ]));
        }

        if let Some(trigger) =
            delayed_that_deals_combat_damage_to_player_trigger_from_core(trigger_tokens)
        {
            return Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
                trigger,
                effects: delayed_effects,
                one_shot: false,
            }]));
        }

        let trigger = next_cast_instant_sorcery_or_loyalty_trigger_from_core(trigger_tokens)
            .map(Ok)
            .unwrap_or_else(|| parse_trigger_clause_lexed(&trigger_tokens))?;
        let one_shot = delayed_trigger_is_one_shot(trigger_clause);
        if delayed_trigger_provides_triggering_stack_object(&trigger) {
            retarget_source_copy_spell_to_delayed_triggering_object(&mut delayed_effects);
        }
        return Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
            trigger,
            effects: delayed_effects,
            one_shot,
        }]));
    }

    let Some(matched) = DELAYED_TRIGGER_THIS_TURN_SUFFIX_PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(trigger_clause) = matched.capture_clause_by_role(LexCaptureRole::Condition, clause)
    else {
        return Ok(None);
    };
    let Some(effect_clause) = matched.capture_clause_by_role(LexCaptureRole::Tail, clause) else {
        return Ok(None);
    };

    let trigger_core_tokens = trigger_clause.trimmed().tokens();
    if trigger_core_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger clause before 'this turn' (clause: '{}')",
            clause_display.trim()
        )));
    }
    let trigger = if let Some(trigger) =
        next_cast_instant_sorcery_or_loyalty_trigger_from_core(trigger_core_tokens)
    {
        trigger
    } else if let Some(trigger) =
        delayed_that_deals_combat_damage_to_player_trigger_from_core(trigger_core_tokens)
    {
        trigger
    } else if let Some(trigger) = delayed_tagged_dealt_damage_trigger_from_core(trigger_core_tokens)
    {
        trigger
    } else {
        parse_trigger_clause_lexed(trigger_core_tokens)?
    };
    let remainder = effect_clause.trimmed().tokens();
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger effect clause (clause: '{}')",
            clause_display.trim()
        )));
    }

    let mut delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger effect clause (clause: '{}')",
            clause_display.trim()
        )));
    }
    if delayed_trigger_provides_triggering_stack_object(&trigger) {
        retarget_source_copy_spell_to_delayed_triggering_object(&mut delayed_effects);
    }

    let one_shot = delayed_trigger_is_one_shot(trigger_clause);
    Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
        trigger,
        effects: delayed_effects,
        one_shot,
    }]))
}

pub(crate) fn parse_delayed_when_that_dies_this_turn_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let clause_display = crate::runtime_backend::lexer::render_token_slice(clause.tokens());
    let (delayed_filter, effect_clause) =
        if let Some(matched) = DELAYED_THAT_DIES_THIS_TURN_PATTERN.match_clause(clause) {
            let Some(effect_clause) = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)
            else {
                return Ok(None);
            };
            (None, effect_clause)
        } else if let Some(matched) = DELAYED_DIES_THIS_WAY_PATTERN.match_clause(clause) {
            let delayed_filter = delayed_dies_this_way_filter(&matched, clause)?;
            let Some(effect_clause) = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)
            else {
                return Ok(None);
            };
            (delayed_filter, effect_clause)
        } else {
            return Ok(None);
        };

    let remainder = effect_clause.trimmed().tokens();
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed dies-this-turn effect clause (clause: '{}')",
            clause_display.trim()
        )));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed dies-this-turn effect clause (clause: '{}')",
            clause_display.trim()
        )));
    }

    Ok(Some(vec![EffectAst::DelayedWhenLastObjectDiesThisTurn {
        filter: delayed_filter,
        effects: delayed_effects,
    }]))
}

pub(crate) fn find_from_among(tokens: &[OwnedLexToken]) -> Option<usize> {
    crate::runtime_backend::lexer::find_token_word_sequence(tokens, &["from", "among"])
}

pub(crate) fn find_list_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        if is_article(word) {
            if tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .and_then(parse_card_type)
                .is_some()
            {
                return Some(idx);
            }
        } else if parse_card_type(word).is_some() {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn split_choose_list(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    for segment in split_lexed_slices_on_and(tokens) {
        for sub in split_lexed_slices_on_comma(segment) {
            let trimmed = trim_commas(sub);
            if !trimmed.is_empty() {
                segments.push(trimmed);
            }
        }
    }
    segments
}

pub(crate) fn merge_filters(base: &ObjectFilter, specific: &ObjectFilter) -> ObjectFilter {
    let mut merged = base.clone();

    if !specific.card_types.is_empty() {
        merged.card_types = specific.card_types.clone();
    }
    if !specific.all_card_types.is_empty() {
        merged.all_card_types = specific.all_card_types.clone();
    }
    if !specific.subtypes.is_empty() {
        merged.subtypes.extend(specific.subtypes.clone());
    }
    if !specific.excluded_card_types.is_empty() {
        merged
            .excluded_card_types
            .extend(specific.excluded_card_types.clone());
    }
    if !specific.excluded_colors.is_empty() {
        merged.excluded_colors = merged.excluded_colors.union(specific.excluded_colors);
    }
    if let Some(colors) = specific.colors {
        merged.colors = Some(
            merged
                .colors
                .map_or(colors, |existing| existing.union(colors)),
        );
    }
    merged.chosen_color |= specific.chosen_color;
    if merged.zone.is_none() {
        merged.zone = specific.zone;
    }
    if merged.controller.is_none() {
        merged.controller = specific.controller.clone();
    }
    if merged
        .attacking_player_or_planeswalker_controlled_by
        .is_none()
    {
        merged.attacking_player_or_planeswalker_controlled_by = specific
            .attacking_player_or_planeswalker_controlled_by
            .clone();
    }
    if merged.owner.is_none() {
        merged.owner = specific.owner.clone();
    }
    merged.other |= specific.other;
    merged.token |= specific.token;
    merged.nontoken |= specific.nontoken;
    merged.tapped |= specific.tapped;
    merged.untapped |= specific.untapped;
    merged.attacking |= specific.attacking;
    merged.nonattacking |= specific.nonattacking;
    merged.blocking |= specific.blocking;
    merged.nonblocking |= specific.nonblocking;
    merged.blocked |= specific.blocked;
    merged.unblocked |= specific.unblocked;
    merged.is_commander |= specific.is_commander;
    merged.noncommander |= specific.noncommander;
    merged.colorless |= specific.colorless;
    merged.multicolored |= specific.multicolored;
    merged.monocolored |= specific.monocolored;

    if let Some(mv) = &specific.mana_value {
        merged.mana_value = Some(mv.clone());
    }
    if let Some(power) = &specific.power {
        merged.power = Some(power.clone());
        merged.power_reference = specific.power_reference;
    }
    if let Some(toughness) = &specific.toughness {
        merged.toughness = Some(toughness.clone());
        merged.toughness_reference = specific.toughness_reference;
    }
    if specific.has_mana_cost {
        merged.has_mana_cost = true;
    }
    if specific.no_x_in_cost {
        merged.no_x_in_cost = true;
    }
    if merged.with_counter.is_none() {
        merged.with_counter = specific.with_counter;
    }
    if merged.without_counter.is_none() {
        merged.without_counter = specific.without_counter;
    }
    if merged.alternative_cast.is_none() {
        merged.alternative_cast = specific.alternative_cast;
    }
    for ability_id in &specific.static_abilities {
        if !iter_contains(merged.static_abilities.iter(), ability_id) {
            merged.static_abilities.push(*ability_id);
        }
    }
    for ability_id in &specific.excluded_static_abilities {
        if !iter_contains(merged.excluded_static_abilities.iter(), ability_id) {
            merged.excluded_static_abilities.push(*ability_id);
        }
    }
    for marker in &specific.ability_markers {
        if !merged
            .ability_markers
            .iter()
            .any(|value| value.eq_ignore_ascii_case(marker))
        {
            merged.ability_markers.push(marker.clone());
        }
    }
    for marker in &specific.excluded_ability_markers {
        if !merged
            .excluded_ability_markers
            .iter()
            .any(|value| value.eq_ignore_ascii_case(marker))
        {
            merged.excluded_ability_markers.push(marker.clone());
        }
    }

    merged
}

#[cfg(test)]
mod copy_and_next_spell_shape_tests {
    use super::*;

    #[test]
    fn delayed_end_step_header_uses_captured_step_owner() {
        let tokens = crate::runtime_backend::lex_line(
            "At the beginning of your next end step, draw a card.",
            0,
        )
        .expect("delayed end-step text should lex");

        let effects = parse_delayed_until_next_end_step_sentence(&tokens)
            .expect("delayed end-step parser should not error")
            .expect("delayed end-step parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedUntilNextEndStep"), "{debug}");
        assert!(debug.contains("player: You"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn delayed_end_step_header_uses_captured_turn_owner() {
        let tokens = crate::runtime_backend::lex_line(
            "At the beginning of the end step of that player's next turn, draw a card.",
            0,
        )
        .expect("extra-turn delayed end-step text should lex");

        let effects = parse_delayed_until_next_end_step_sentence(&tokens)
            .expect("extra-turn delayed end-step parser should not error")
            .expect("extra-turn delayed end-step parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedUntilEndStepOfExtraTurn"), "{debug}");
        assert!(debug.contains("player: That"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn delayed_dies_this_way_uses_captured_filter() {
        let tokens = crate::runtime_backend::lex_line(
            "If a creature dealt damage this way would die this turn, exile it instead.",
            0,
        )
        .expect("dies-this-way delayed text should lex");

        let effects = parse_delayed_when_that_dies_this_turn_sentence(&tokens)
            .expect("dies-this-way parser should not error")
            .expect("dies-this-way parser should match");
        let debug = format!("{effects:#?}");

        assert!(
            debug.contains("DelayedWhenLastObjectDiesThisTurn"),
            "{debug}"
        );
        assert!(debug.contains("filter: Some"), "{debug}");
        assert!(debug.contains("card_types"), "{debug}");
        assert!(debug.contains("Exile"), "{debug}");
    }

    #[test]
    fn delayed_that_dies_this_turn_uses_captured_effect_tail() {
        let tokens =
            crate::runtime_backend::lex_line("When that creature dies this turn, draw a card.", 0)
                .expect("that-dies delayed text should lex");

        let effects = parse_delayed_when_that_dies_this_turn_sentence(&tokens)
            .expect("that-dies parser should not error")
            .expect("that-dies parser should match");
        let debug = format!("{effects:#?}");

        assert!(
            debug.contains("DelayedWhenLastObjectDiesThisTurn"),
            "{debug}"
        );
        assert!(debug.contains("filter: None"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn this_turn_delayed_trigger_uses_captured_duration_tail() {
        let tokens = crate::runtime_backend::lex_line(
            "This turn, whenever you draw a card, draw a card.",
            0,
        )
        .expect("this-turn delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("this-turn delayed trigger parser should not error")
            .expect("this-turn delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("YouDrawCard"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn suffix_this_turn_delayed_trigger_uses_captured_trigger_and_effect() {
        let tokens =
            crate::runtime_backend::lex_line("Whenever you draw a card this turn, draw a card.", 0)
                .expect("suffix-this-turn delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("suffix-this-turn delayed trigger parser should not error")
            .expect("suffix-this-turn delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("YouDrawCard"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn suffix_this_turn_delayed_trigger_supports_spell_or_loyalty_union() {
        let tokens = crate::runtime_backend::lex_line(
            "When you next cast an instant spell, cast a sorcery spell, or activate a loyalty ability this turn, copy that spell or ability twice. You may choose new targets for the copies.",
            0,
        )
        .expect("next spell-or-loyalty delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("spell-or-loyalty delayed trigger parser should not error")
            .expect("spell-or-loyalty delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("Either"), "{debug}");
        assert!(debug.contains("SpellCast"), "{debug}");
        assert!(debug.contains("AbilityActivated"), "{debug}");
        assert!(debug.contains("loyalty_only: true"), "{debug}");
        assert!(debug.contains("CopySpell"), "{debug}");
        let [
            EffectAst::DelayedTriggerThisTurn {
                effects: delayed_effects,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one delayed trigger effect, got {effects:#?}");
        };
        let [EffectAst::SubjectVerb(subject_verb)] = delayed_effects.as_slice() else {
            panic!("expected one delayed copy effect, got {delayed_effects:#?}");
        };
        let SubjectVerbActionAst::CopySpell {
            count,
            may_choose_new_targets,
            ..
        } = &subject_verb.action
        else {
            panic!("expected delayed copy spell action, got {subject_verb:#?}");
        };
        assert_eq!(*count, Value::Fixed(2));
        assert!(*may_choose_new_targets);
    }

    #[test]
    fn leading_this_turn_target_attack_unblocked_uses_captured_subject() {
        let tokens = crate::runtime_backend::lex_line(
            "This turn, when target creature you control attacks and isn't blocked, draw a card.",
            0,
        )
        .expect("targeted attack-unblocked delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("targeted attack-unblocked delayed trigger parser should not error")
            .expect("targeted attack-unblocked delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("ChooseObjects"), "{debug}");
        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("AttacksAndIsntBlocked"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn suffix_this_turn_tagged_dealt_damage_uses_captured_kind() {
        let tokens = crate::runtime_backend::lex_line(
            "Whenever that creature is dealt damage this turn, draw a card.",
            0,
        )
        .expect("tagged dealt-damage delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("tagged dealt-damage delayed trigger parser should not error")
            .expect("tagged dealt-damage delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("IsDealtDamage"), "{debug}");
        assert!(debug.contains("TaggedObjectConstraint"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn suffix_this_turn_tagged_combat_damage_uses_captured_marker() {
        let tokens = crate::runtime_backend::lex_line(
            "Whenever that permanent is dealt combat damage this turn, draw a card.",
            0,
        )
        .expect("tagged combat-damage delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("tagged combat-damage delayed trigger parser should not error")
            .expect("tagged combat-damage delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("IsDealtCombatDamage"), "{debug}");
        assert!(debug.contains("TaggedObjectConstraint"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }
}

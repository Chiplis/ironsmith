use super::super::grammar::effects::delayed_sentence_shapes as delayed_shapes;

/// "At the beginning of the next combat [phase] this turn, <effects>" — a
/// one-shot delayed trigger scheduled for the next beginning of combat,
/// expiring at end of turn.
pub(crate) fn parse_delayed_next_combat_phase_this_turn_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = delayed_shapes::parse_delayed_next_combat_shape(tokens) else {
        return Ok(None);
    };
    let remainder = shape.effect_tokens;
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed next-combat-phase effect clause (clause: '{}')",
            crate::lexer::render_token_slice(tokens).trim()
        )));
    }
    let delayed_effects = parse_effect_chain(remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed next-combat-phase effect clause (clause: '{}')",
            crate::lexer::render_token_slice(tokens).trim()
        )));
    }
    Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
        trigger: TriggerSpec::BeginningOfCombat(PlayerFilter::Any),
        effects: delayed_effects,
        one_shot: true,
        until_end_of_combat: false,
        attach_to_previous_ability: false,
    }]))
}

fn delayed_dies_this_way_filter(
    subject_tokens: &[OwnedLexToken],
    full_sentence_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let clause_display = crate::lexer::render_token_slice(full_sentence_tokens);
    let mut subject_tokens = trim_edge_punctuation(subject_tokens);
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
    let Some(shape) = delayed_shapes::parse_delayed_end_step_shape(tokens) else {
        return Ok(None);
    };
    let player = shape.player;
    let start_next_turn = shape.start_next_turn;
    let remainder = shape.effect_tokens;
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(
            "missing delayed end-step effect clause".to_string(),
        ));
    }

    let delayed_effects = super::parse_effect_sentences_lexed(remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed end-step effect clause (clause: '{}')",
            crate::lexer::render_token_slice(tokens).trim()
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

        crate::model::visit::for_each_nested_effects_mut(
            effect,
            true,
            retarget_source_copy_spell_to_delayed_triggering_object,
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
    let Some(subject_tokens) =
        delayed_shapes::parse_delayed_attack_unblocked_subject(trigger_tokens)
    else {
        return Ok(None);
    };
    let full_sentence_display =
        crate::lexer::render_token_slice(full_sentence_tokens);
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
    let shape = delayed_shapes::parse_delayed_tagged_damage_shape(trigger_core_tokens)?;
    let mut filter = match shape.kind {
        delayed_shapes::DelayedObjectKind::Creature => ObjectFilter::creature(),
        delayed_shapes::DelayedObjectKind::Permanent => ObjectFilter::permanent(),
    };
    filter = filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);

    if shape.combat {
        Some(TriggerSpec::IsDealtCombatDamage(filter))
    } else {
        Some(TriggerSpec::IsDealtDamage(filter))
    }
}

fn delayed_that_deals_combat_damage_to_player_trigger_from_core(
    trigger_core_tokens: &[OwnedLexToken],
) -> Option<TriggerSpec> {
    let kind = delayed_shapes::parse_delayed_deals_combat_damage_kind(trigger_core_tokens)?;
    let mut filter = match kind {
        delayed_shapes::DelayedObjectKind::Creature => ObjectFilter::creature(),
        delayed_shapes::DelayedObjectKind::Permanent => ObjectFilter::permanent(),
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
    if !delayed_shapes::is_next_cast_spell_or_loyalty_shape(trigger_core_tokens) {
        return None;
    }

    let spell_cast = TriggerSpec::SpellCast {
        filter: Some(ObjectFilter::instant_or_sorcery()),
        mana_source_filter: None,
        caster: PlayerFilter::You,
        timing: None,
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
    let tokens = trigger_clause.trimmed().tokens();
    delayed_shapes::delayed_trigger_has_next_marker(tokens)
        || delayed_shapes::delayed_trigger_has_first_time_marker(tokens)
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
    let shape = delayed_shapes::parse_copy_twice_shape(effect_tokens)?;

    Some(vec![EffectAst::subject_verb_copy_spell(
        TargetAst::Tagged(TagKey::from("triggering"), None),
        Value::Fixed(2),
        PlayerAst::Implicit,
        shape.may_choose_new_targets,
        false,
        Vec::new(),
    )])
}

fn parse_next_cast_spell_or_loyalty_delayed_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = delayed_shapes::parse_delayed_this_turn_shape(tokens) else {
        return Ok(None);
    };
    let Some(trigger) =
        next_cast_instant_sorcery_or_loyalty_trigger_from_core(shape.trigger_tokens)
    else {
        return Ok(None);
    };
    let effect_tokens = shape.effect_tokens;
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed spell-or-loyalty effect clause (clause: '{}')",
            crate::lexer::render_token_slice(tokens).trim()
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
            crate::lexer::render_token_slice(tokens).trim()
        )));
    }
    retarget_source_copy_spell_to_delayed_triggering_object(&mut delayed_effects);
    Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
        trigger,
        effects: delayed_effects,
        one_shot: true,
        until_end_of_combat: false,
        attach_to_previous_ability: shape.references_previous_creature,
    }]))
}

/// Preserve the correlated choice loop in
/// "When you next cast an instant or sorcery spell that targets only a
/// single opponent or a single permanent an opponent controls this turn,
/// for each other opponent, choose that player or a permanent they control,
/// copy that spell, and the copy targets the chosen player or permanent."
///
/// The broad delayed-trigger route sees each `or` independently and can turn
/// the trigger into unrelated spell-filter arms.  Worse, its ordinary copy
/// parser treats the entire quantified tail as copy-target surface and drops
/// the per-opponent choice.  Claim only the complete authored grammar here so
/// the target relation, loop exclusion, choice, copy, and fixed retarget stay
/// in one executable delayed program.
fn parse_next_cast_single_opponent_or_permanent_copy_loop(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let words = tokens
        .iter()
        .filter_map(|token| token.as_word().map(|_| token.parser_text()))
        .collect::<Vec<_>>();
    if words
        != [
            "when",
            "you",
            "next",
            "cast",
            "an",
            "instant",
            "or",
            "sorcery",
            "spell",
            "that",
            "targets",
            "only",
            "a",
            "single",
            "opponent",
            "or",
            "a",
            "single",
            "permanent",
            "an",
            "opponent",
            "controls",
            "this",
            "turn",
            "for",
            "each",
            "other",
            "opponent",
            "choose",
            "that",
            "player",
            "or",
            "a",
            "permanent",
            "they",
            "control",
            "copy",
            "that",
            "spell",
            "and",
            "the",
            "copy",
            "targets",
            "the",
            "chosen",
            "player",
            "or",
            "permanent",
        ]
    {
        return None;
    }

    let mut spell_filter = ObjectFilter::instant_or_sorcery()
        .targeting_only(
            Some(PlayerFilter::Opponent),
            Some(ObjectFilter::permanent().controlled_by(PlayerFilter::Opponent)),
        )
        .target_count_exact(1);
    spell_filter.has_mana_cost = true;

    let choice = TargetAst::ObjectOrPlayer(
        ObjectFilter::permanent().controlled_by(PlayerFilter::IteratedPlayer),
        PlayerFilter::IteratedPlayer,
        None,
    );
    let chosen_destination = TargetAst::ObjectOrPlayer(
        ObjectFilter::permanent()
            .controlled_by(PlayerFilter::IteratedPlayer)
            .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject),
        PlayerFilter::IteratedPlayer,
        None,
    );
    let copy = EffectAst::subject_verb_copy_spell(
        TargetAst::Tagged(TagKey::from("triggering"), None),
        Value::Fixed(1),
        PlayerAst::You,
        false,
        false,
        Vec::new(),
    );
    let retarget = EffectAst::subject_verb_retarget_stack_object(
        PlayerAst::You,
        TargetAst::Tagged(TagKey::from("__copied_stack_object__"), None),
        crate::cards::builders::RetargetModeAst::OneToFixed {
            target: chosen_destination,
        },
        false,
    );

    Some(vec![EffectAst::DelayedTriggerThisTurn {
        trigger: TriggerSpec::SpellCast {
            filter: Some(spell_filter),
            mana_source_filter: None,
            caster: PlayerFilter::You,
            timing: None,
            during_turn: None,
            min_spells_this_turn: None,
            exact_spells_this_turn: None,
            from_not_hand: false,
        },
        effects: vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::excluding(
                PlayerFilter::Opponent,
                PlayerFilter::TargetPlayerOrControllerOfTarget,
            ),
            effects: vec![EffectAst::subject_verb_target_only(choice), copy, retarget],
        }],
        one_shot: true,
        until_end_of_combat: false,
        attach_to_previous_ability: false,
    }])
}

pub(crate) fn parse_sentence_delayed_trigger_this_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let clause_display = crate::lexer::render_token_slice(clause.tokens());
    if let Some(effects) = parse_next_cast_single_opponent_or_permanent_copy_loop(tokens) {
        return Ok(Some(effects));
    }
    if delayed_shapes::parse_delayed_dies_shape(tokens).is_some() {
        return parse_delayed_when_that_dies_this_turn_sentence(tokens);
    }

    if let Some(effects) = parse_next_cast_spell_or_loyalty_delayed_sentence(tokens)? {
        return Ok(Some(effects));
    }

    let Some(shape) = delayed_shapes::parse_delayed_this_turn_shape(tokens) else {
        return Ok(None);
    };
    let trigger_tokens = shape.trigger_tokens;
    let trigger_clause = LexedClause::new(trigger_tokens).trimmed();

    if shape.placement == delayed_shapes::DelayedThisTurnPlacement::LeadingDuration {
        let mut delayed_effects = parse_effect_chain(shape.effect_tokens)?;
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
                    until_end_of_combat: false,
                    attach_to_previous_ability: shape.references_previous_creature,
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
                until_end_of_combat: false,
                attach_to_previous_ability: shape.references_previous_creature,
            }]));
        }

        let trigger = next_cast_instant_sorcery_or_loyalty_trigger_from_core(trigger_tokens)
            .map(Ok)
            .unwrap_or_else(|| parse_trigger_clause_lexed(trigger_tokens))?;
        let one_shot = delayed_trigger_is_one_shot(trigger_clause);
        if delayed_trigger_provides_triggering_stack_object(&trigger) {
            retarget_source_copy_spell_to_delayed_triggering_object(&mut delayed_effects);
        }
        return Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
            trigger,
            effects: delayed_effects,
            one_shot,
            until_end_of_combat: false,
            attach_to_previous_ability: shape.references_previous_creature,
        }]));
    }

    let trigger_core_tokens = trigger_tokens;
    if trigger_core_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger clause before 'this turn' (clause: '{}')",
            clause_display.trim()
        )));
    }

    let delayed_target_shape =
        delayed_shapes::parse_delayed_target_dies_subject(trigger_core_tokens)
            .map(|subject| (subject, false))
            .or_else(|| {
                delayed_shapes::parse_delayed_target_put_into_your_graveyard_subject(
                    trigger_core_tokens,
                )
                .map(|subject| (subject, true))
            });
    if let Some((subject_tokens, put_into_your_graveyard)) = delayed_target_shape {
        let filter = parse_object_filter(subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported delayed target dies filter (clause: '{}')",
                clause_display.trim()
            ))
        })?;
        let tag = helper_tag_for_tokens(tokens, "targeted");
        let mut watched_filter = filter
            .clone()
            .match_tagged(tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
        if put_into_your_graveyard {
            watched_filter.owner = Some(PlayerFilter::You);
        }
        let delayed_effects = parse_effect_chain(shape.effect_tokens)?;
        if delayed_effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing delayed target-dies effect clause (clause: '{}')",
                clause_display.trim()
            )));
        }
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                // `target` identifies the chosen object, not its controller.
                // An implicit chooser still resolves to the spell's controller
                // without adding a "you control" restriction to the filter.
                player: PlayerAst::Implicit,
                tag,
            },
            EffectAst::DelayedTriggerThisTurn {
                trigger: if put_into_your_graveyard {
                    TriggerSpec::PutIntoGraveyard(watched_filter)
                } else {
                    TriggerSpec::Dies(watched_filter)
                },
                effects: delayed_effects,
                one_shot: true,
                until_end_of_combat: false,
                attach_to_previous_ability: false,
            },
        ]));
    }
    if let Some(history_shape) =
        delayed_shapes::parse_delayed_dies_after_damage_by_previous_creature_shape(
            trigger_core_tokens,
        )
    {
        let mut victim = parse_object_filter(history_shape.victim_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported delayed damage-history victim filter (clause: '{}')",
                clause_display.trim()
            ))
        })?;
        victim.dealt_damage_by_source_this_turn =
            Some(ironsmith_core::DamagedBySource::ThisCreature);
        let delayed_effects = parse_effect_chain(shape.effect_tokens)?;
        if delayed_effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing delayed damage-history death effect clause (clause: '{}')",
                clause_display.trim()
            )));
        }
        return Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
            trigger: TriggerSpec::Dies(victim),
            effects: delayed_effects,
            one_shot: delayed_trigger_is_one_shot(trigger_clause),
            until_end_of_combat: false,
            attach_to_previous_ability: true,
        }]));
    }
    if delayed_shapes::is_delayed_prior_object_put_into_a_graveyard(trigger_core_tokens) {
        let delayed_effects = parse_effect_chain(shape.effect_tokens)?;
        if delayed_effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing delayed prior-object graveyard effect clause (clause: '{}')",
                clause_display.trim()
            )));
        }
        return Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
            trigger: TriggerSpec::PutIntoGraveyard(ObjectFilter::tagged(TagKey::from(IT_TAG))),
            effects: delayed_effects,
            one_shot: true,
            until_end_of_combat: false,
            attach_to_previous_ability: true,
        }]));
    }
    if let Some(combat_shape) =
        delayed_shapes::parse_delayed_target_deals_combat_damage_shape(trigger_core_tokens)
        && let Ok(filter) = parse_object_filter(combat_shape.subject_tokens, false)
        && let Ok(recipient) = parse_object_filter(combat_shape.recipient_tokens, false)
    {
        let tag = helper_tag_for_tokens(tokens, "targeted");
        let watched_filter = filter
            .clone()
            .match_tagged(tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
        let delayed_effects = parse_effect_chain(shape.effect_tokens)?;
        if delayed_effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing delayed target combat-damage effect clause (clause: '{}')",
                clause_display.trim()
            )));
        }
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                // `target` identifies the chosen object, not its controller.
                player: PlayerAst::Implicit,
                tag,
            },
            EffectAst::DelayedTriggerThisTurn {
                trigger: TriggerSpec::DealsCombatDamageTo {
                    source: watched_filter,
                    target: recipient,
                },
                effects: delayed_effects,
                one_shot: false,
                until_end_of_combat: false,
                attach_to_previous_ability: false,
            },
        ]));
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
    let remainder = shape.effect_tokens;
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger effect clause (clause: '{}')",
            clause_display.trim()
        )));
    }

    let mut delayed_effects = parse_effect_chain(remainder)?;
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
        until_end_of_combat: false,
        attach_to_previous_ability: shape.references_previous_creature,
    }]))
}

pub(crate) fn parse_delayed_when_that_dies_this_turn_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let clause_display = crate::lexer::render_token_slice(clause.tokens());
    let Some(shape) = delayed_shapes::parse_delayed_dies_shape(tokens) else {
        return Ok(None);
    };
    let (delayed_filter, remainder) = match shape {
        delayed_shapes::DelayedDiesShape::ThatReference { effect_tokens } => (None, effect_tokens),
        delayed_shapes::DelayedDiesShape::DefinitePriorTarget {
            subject_tokens,
            effect_tokens,
        } => {
            let mut filter =
                delayed_dies_this_way_filter(subject_tokens, tokens)?.ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing definite delayed-death object filter (clause: '{}')",
                        clause_display.trim()
                    ))
                })?;
            let noun = crate::lexer::token_word_refs(subject_tokens)
                .get(1)
                .and_then(|noun| ironsmith_core::DemonstrativeAntecedentSurface::from_noun(noun));
            filter.set_demonstrative_antecedent_surface(noun);
            (Some(filter), effect_tokens)
        }
        delayed_shapes::DelayedDiesShape::ThisWay {
            subject_tokens,
            effect_tokens,
        } => (
            delayed_dies_this_way_filter(subject_tokens, tokens)?,
            effect_tokens,
        ),
    };
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed dies-this-turn effect clause (clause: '{}')",
            clause_display.trim()
        )));
    }

    let remainder_words = crate::lexer::token_word_refs(remainder);
    let delayed_effects = if matches!(
        remainder_words.as_slice(),
        ["exile", "its", "controllers", "graveyard"]
            | ["exile", "its", "controller's", "graveyard"]
            | ["exile", "its", "controller", "s", "graveyard"]
    ) {
        let mut graveyard = ObjectFilter::default();
        graveyard.zone = Some(Zone::Graveyard);
        graveyard.owner = Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::Tagged(TagKey::from(crate::cards::builders::IT_TAG)),
        ));
        vec![EffectAst::subject_verb_exile_all(graveyard, false)]
    } else {
        parse_effect_chain(remainder)?
    };
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

pub(crate) fn parse_delayed_when_that_leaves_battlefield_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = delayed_shapes::parse_delayed_tagged_leaves_shape(tokens) else {
        return Ok(None);
    };
    let filter = match shape.kind {
        delayed_shapes::DelayedLeavesObjectKind::Creature => ObjectFilter::creature(),
        delayed_shapes::DelayedLeavesObjectKind::Permanent => ObjectFilter::permanent(),
        delayed_shapes::DelayedLeavesObjectKind::Token => ObjectFilter::default().token(),
    };
    let delayed_effects = parse_effect_chain(shape.effect_tokens)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed leaves-the-battlefield effect clause (clause: '{}')",
            crate::lexer::render_token_slice(tokens).trim()
        )));
    }
    Ok(Some(vec![
        EffectAst::DelayedWhenLastObjectLeavesBattlefield {
            filter,
            effects: delayed_effects,
        },
    ]))
}

pub(crate) fn find_from_among(tokens: &[OwnedLexToken]) -> Option<usize> {
    crate::lexer::find_token_word_sequence(tokens, &["from", "among"])
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
    merged.suspected |= specific.suspected;
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
        let tokens = crate::lexer::lex_line(
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
        let tokens = crate::lexer::lex_line(
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
    fn delayed_end_step_body_uses_typed_consult_bundle_dispatch() {
        let tokens = crate::lexer::lex_line(
            "At the beginning of the next end step, reveal cards from the top of your library until you reveal that many creature cards, put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into your library.",
            0,
        )
        .expect("delayed consult text should lex");

        let effects = parse_delayed_until_next_end_step_sentence(&tokens)
            .expect("delayed consult parser should not error")
            .expect("delayed consult parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedUntilNextEndStep"), "{debug}");
        assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
        assert!(debug.contains("ShuffleLibrary"), "{debug}");
    }

    #[test]
    fn delayed_dies_this_way_uses_captured_filter() {
        let tokens = crate::lexer::lex_line(
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
            crate::lexer::lex_line("When that creature dies this turn, draw a card.", 0)
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
    fn delayed_death_exiles_the_referenced_objects_controllers_whole_graveyard() {
        let tokens = crate::lexer::lex_line(
            "When that creature dies this turn, exile its controller's graveyard.",
            0,
        )
        .expect("whole-graveyard delayed text should lex");

        let effects = parse_delayed_when_that_dies_this_turn_sentence(&tokens)
            .expect("delayed death parser should not error")
            .expect("delayed death parser should match");
        let [
            EffectAst::DelayedWhenLastObjectDiesThisTurn {
                effects: delayed, ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one delayed watcher: {effects:#?}");
        };
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileAll { filter, .. },
                ..
            }),
        ] = delayed.as_slice()
        else {
            panic!("expected exhaustive graveyard exile: {delayed:#?}");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(matches!(
            &filter.owner,
            Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag)))
                if tag.as_str() == crate::cards::builders::IT_TAG
        ));
    }

    #[test]
    fn definite_filtered_death_subject_watches_the_prior_object_once() {
        let tokens = crate::lexer::lex_line(
            "When the permanent you don't control dies this turn, you gain 2 life.",
            0,
        )
        .expect("definite delayed-death text should lex");

        let effects = parse_delayed_when_that_dies_this_turn_sentence(&tokens)
            .expect("definite delayed-death parser should not error")
            .expect("definite delayed-death parser should match");
        let [
            EffectAst::DelayedWhenLastObjectDiesThisTurn {
                filter: Some(filter),
                effects,
            },
        ] = effects.as_slice()
        else {
            panic!("expected a prior-object delayed death watcher: {effects:#?}");
        };

        assert_eq!(
            filter.demonstrative_antecedent_surface(),
            Some(ironsmith_core::DemonstrativeAntecedentSurface::Permanent)
        );
        assert_eq!(filter.controller, Some(PlayerFilter::NotYou));
        assert!(matches!(effects.as_slice(), [EffectAst::SubjectVerb(_)]));
    }

    #[test]
    fn prior_target_damage_then_definite_death_lowers_to_tagged_this_dies() {
        let definition = crate::CardDefinitionBuilder::new(
            crate::CardId::new(),
            "Prior Target Death Watch",
        )
        .card_types(vec![crate::CardType::Sorcery])
        .parse_text(
            "Target creature you control deals damage equal to its power to target creature or planeswalker you don't control. When the permanent you don't control dies this turn, you gain 2 life.",
        )
        .expect("a definite delayed-death subject should compile against the prior target");
        let debug = format!("{:#?}", definition.spell_effect);

        assert!(debug.contains("ThisDies"), "{debug}");
        assert!(debug.contains("one_shot: true"), "{debug}");
        assert!(debug.contains("target_tag: Some"), "{debug}");
        assert!(
            debug.contains("demonstrative_antecedent: Some(\n") && debug.contains("Permanent"),
            "{debug}"
        );
        assert!(!debug.contains("ThisLeavesBattlefield"), "{debug}");
    }

    #[test]
    fn indefinite_damage_history_subject_keeps_this_way_collection_shape() {
        let tokens = crate::lexer::lex_line(
            "Whenever a creature dealt damage this way dies this turn, you gain 2 life.",
            0,
        )
        .expect("damage-history delayed-death text should lex");

        let effects = parse_delayed_when_that_dies_this_turn_sentence(&tokens)
            .expect("damage-history delayed-death parser should not error")
            .expect("damage-history delayed-death parser should match");
        let [
            EffectAst::DelayedWhenLastObjectDiesThisTurn {
                filter: Some(filter),
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected a this-way delayed death watcher: {effects:#?}");
        };

        assert_eq!(filter.demonstrative_antecedent_surface(), None);
    }

    #[test]
    fn delayed_that_creature_leaves_uses_captured_effect_tail() {
        let tokens = crate::lexer::lex_line(
            "When that creature leaves the battlefield, return this card from exile to the battlefield under its owner's control.",
            0,
        )
        .expect("that-leaves delayed text should lex");

        let effects = parse_delayed_when_that_leaves_battlefield_sentence(&tokens)
            .expect("that-leaves parser should not error")
            .expect("that-leaves parser should match");
        let debug = format!("{effects:#?}");

        assert!(
            debug.contains("DelayedWhenLastObjectLeavesBattlefield"),
            "{debug}"
        );
        assert!(debug.contains("Creature"), "{debug}");
        assert!(debug.contains("Return"), "{debug}");
    }

    #[test]
    fn this_turn_delayed_trigger_uses_captured_duration_tail() {
        let tokens = crate::lexer::lex_line(
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
            crate::lexer::lex_line("Whenever you draw a card this turn, draw a card.", 0)
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
    fn next_single_opponent_or_permanent_copy_keeps_each_other_opponent_choice() {
        let tokens = crate::lexer::lex_line(
            "When you next cast an instant or sorcery spell that targets only a single opponent or a single permanent an opponent controls this turn, for each other opponent, choose that player or a permanent they control, copy that spell, and the copy targets the chosen player or permanent.",
            0,
        )
        .expect("correlated next-spell copy text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("correlated next-spell copy parser should not error")
            .expect("correlated next-spell copy parser should match");
        let [
            EffectAst::DelayedTriggerThisTurn {
                trigger:
                    TriggerSpec::SpellCast {
                        filter: Some(filter),
                        caster: PlayerFilter::You,
                        ..
                    },
                effects: delayed,
                one_shot: true,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one exact delayed spell watcher: {effects:#?}");
        };
        assert_eq!(filter.card_types, [CardType::Instant, CardType::Sorcery]);
        assert_eq!(filter.target_count, Some(ChoiceCount::exactly(1)));
        assert_eq!(filter.targets_only_player, Some(PlayerFilter::Opponent));
        assert!(filter.targets_only_any_of);
        assert_eq!(
            filter
                .targets_only_object
                .as_deref()
                .and_then(|target| target.controller.as_ref()),
            Some(&PlayerFilter::Opponent)
        );
        let [
            EffectAst::ForEachPlayersFiltered {
                filter: player_filter,
                effects: per_opponent,
            },
        ] = delayed.as_slice()
        else {
            panic!("expected one per-other-opponent loop: {delayed:#?}");
        };
        assert_eq!(
            player_filter,
            &PlayerFilter::excluding(
                PlayerFilter::Opponent,
                PlayerFilter::TargetPlayerOrControllerOfTarget,
            )
        );
        let debug = format!("{per_opponent:#?}");
        assert!(debug.contains("ObjectOrPlayer"), "{debug}");
        assert!(debug.contains("CopySpell"), "{debug}");
        assert!(debug.contains("RetargetStackObject"), "{debug}");
    }

    #[test]
    fn next_copy_loop_does_not_claim_a_broader_target_or_missing_choice() {
        for text in [
            "When you next cast an instant or sorcery spell that targets an opponent or a permanent an opponent controls this turn, for each other opponent, choose that player or a permanent they control, copy that spell, and the copy targets the chosen player or permanent.",
            "When you next cast an instant or sorcery spell that targets only a single opponent or a single permanent an opponent controls this turn, copy that spell.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).expect("near miss should lex");
            assert!(
                parse_next_cast_single_opponent_or_permanent_copy_loop(&tokens).is_none(),
                "near miss must not use the exact correlated route: {text}"
            );
        }
    }

    #[test]
    fn delayed_death_after_damage_by_previous_creature_keeps_both_identities() {
        let tokens = crate::lexer::lex_line(
            "Whenever a creature dealt damage by that creature dies this turn, its controller loses 2 life.",
            0,
        )
        .expect("damage-history death watcher should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("damage-history death watcher should not error")
            .expect("damage-history death watcher should match");
        let [
            EffectAst::DelayedTriggerThisTurn {
                trigger: TriggerSpec::Dies(victim),
                effects: delayed_effects,
                one_shot,
                attach_to_previous_ability,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one delayed death watcher: {effects:#?}");
        };

        assert_eq!(victim.card_types, [CardType::Creature]);
        assert_eq!(
            victim.dealt_damage_by_source_this_turn,
            Some(ironsmith_core::DamagedBySource::ThisCreature)
        );
        assert!(!*one_shot, "`Whenever` must remain repeatable this turn");
        assert!(
            *attach_to_previous_ability,
            "the damager must be the preceding creature target"
        );
        assert!(format!("{delayed_effects:#?}").contains("LoseLife"));
    }

    #[test]
    fn suffix_this_turn_first_matching_cast_is_one_shot() {
        let tokens = crate::lexer::lex_line(
            "When you cast a spell with the chosen name for the first time this turn, draw two cards.",
            0,
        )
        .expect("first-matching-cast delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("first-matching-cast delayed trigger parser should not error")
            .expect("first-matching-cast delayed trigger parser should match");

        let [
            EffectAst::DelayedTriggerThisTurn {
                trigger:
                    TriggerSpec::SpellCast {
                        filter: Some(filter),
                        caster: PlayerFilter::You,
                        ..
                    },
                effects: delayed_effects,
                one_shot,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one named-spell delayed trigger, got {effects:#?}");
        };

        assert_eq!(filter.name.as_deref(), Some("{chosen name}"));
        assert!(
            *one_shot,
            "the first matching cast must consume the trigger"
        );
        assert!(format!("{delayed_effects:#?}").contains("Draw"));
    }

    #[test]
    fn suffix_this_turn_delayed_trigger_supports_spell_or_loyalty_union() {
        let tokens = crate::lexer::lex_line(
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
        let tokens = crate::lexer::lex_line(
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
        let tokens = crate::lexer::lex_line(
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
        let tokens = crate::lexer::lex_line(
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

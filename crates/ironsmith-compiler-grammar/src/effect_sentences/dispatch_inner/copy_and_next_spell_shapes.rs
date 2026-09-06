use crate::cards::builders::DelayedEffectAst;
use crate::cards::builders::StackActionAst;
use super::super::grammar::effects::delayed_sentence_shapes as delayed_shapes;

/// "At the beginning of the next combat [phase] this turn, <effects>" — a
/// one-shot delayed trigger scheduled for the next beginning of combat,
/// expiring at end of turn.
pub fn parse_delayed_next_combat_phase_this_turn_sentence(
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
    Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
        trigger: TriggerSpec::BeginningOfCombat(PlayerFilter::Any),
        effects: delayed_effects,
        one_shot: true,
        until_end_of_combat: false,
        attach_to_previous_ability: false,
    })]))
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

pub fn parse_delayed_until_next_end_step_sentence(
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
        Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndStepOfExtraTurn {
            player: player_ast,
            effects: delayed_effects,
        })]))
    } else {
        Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep {
            player,
            effects: delayed_effects,
        })]))
    }
}

fn retarget_source_copy_spell_to_delayed_triggering_object(effects: &mut [EffectAst]) {
    fn visit(effect: &mut EffectAst) {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::Stack(StackActionAst::CopySpell { target, .. }) = &mut subject_verb.action
            && matches!(target, TargetAst::Source(_))
        {
            *target = TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind(), None);
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
    filter = filter.match_tagged(crate::tag::CompilerReferenceTag::It.bind(), TaggedOpbjectRelation::IsTaggedObject);

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
    filter = filter.match_tagged(crate::tag::CompilerReferenceTag::It.bind(), TaggedOpbjectRelation::IsTaggedObject);
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
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind(), None),
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
    Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
        trigger,
        effects: delayed_effects,
        one_shot: true,
        until_end_of_combat: false,
        attach_to_previous_ability: shape.references_previous_creature,
    })]))
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
    if !crate::word_primitives::parse_sequence_complete(
        &words,
        &[
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
        ],
    )
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
            .match_tagged(crate::tag::CompilerReferenceTag::It.bind(), TaggedOpbjectRelation::IsTaggedObject),
        PlayerFilter::IteratedPlayer,
        None,
    );
    let copy = EffectAst::subject_verb_copy_spell(
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind(), None),
        Value::Fixed(1),
        PlayerAst::You,
        false,
        false,
        Vec::new(),
    );
    let retarget = EffectAst::subject_verb_retarget_stack_object(
        PlayerAst::You,
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::CopiedStackObject.bind(), None),
        crate::cards::builders::RetargetModeAst::OneToFixed {
            target: chosen_destination,
        },
        false,
    );

    Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
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
        effects: vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::excluding(
                PlayerFilter::Opponent,
                PlayerFilter::TargetPlayerOrControllerOfTarget,
            ),
            effects: vec![EffectAst::subject_verb_target_only(choice), copy, retarget],
        })],
        one_shot: true,
        until_end_of_combat: false,
        attach_to_previous_ability: false,
    })])
}

pub fn parse_sentence_delayed_trigger_this_turn(
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
                    tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            return Ok(Some(vec![
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: crate::tag::CompilerReferenceTag::It.bind(),
                }),
                EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
                    trigger: TriggerSpec::AttacksAndIsntBlocked(trigger_filter),
                    effects: delayed_effects,
                    one_shot: true,
                    until_end_of_combat: false,
                    attach_to_previous_ability: shape.references_previous_creature,
                }),
            ]));
        }

        if let Some(trigger) =
            delayed_that_deals_combat_damage_to_player_trigger_from_core(trigger_tokens)
        {
            return Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
                trigger,
                effects: delayed_effects,
                one_shot: false,
                until_end_of_combat: false,
                attach_to_previous_ability: shape.references_previous_creature,
            })]));
        }

        let trigger = next_cast_instant_sorcery_or_loyalty_trigger_from_core(trigger_tokens)
            .map(Ok)
            .unwrap_or_else(|| parse_trigger_clause_lexed(trigger_tokens))?;
        let one_shot = delayed_trigger_is_one_shot(trigger_clause);
        if delayed_trigger_provides_triggering_stack_object(&trigger) {
            retarget_source_copy_spell_to_delayed_triggering_object(&mut delayed_effects);
        }
        return Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
            trigger,
            effects: delayed_effects,
            one_shot,
            until_end_of_combat: false,
            attach_to_previous_ability: shape.references_previous_creature,
        })]));
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
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                // `target` identifies the chosen object, not its controller.
                // An implicit chooser still resolves to the spell's controller
                // without adding a "you control" restriction to the filter.
                player: PlayerAst::Implicit,
                tag: crate::tag::TagRef::of(tag),
            }),
            EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
                trigger: if put_into_your_graveyard {
                    TriggerSpec::PutIntoGraveyard(watched_filter)
                } else {
                    TriggerSpec::Dies(watched_filter)
                },
                effects: delayed_effects,
                one_shot: true,
                until_end_of_combat: false,
                attach_to_previous_ability: false,
            }),
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
        return Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
            trigger: TriggerSpec::Dies(victim),
            effects: delayed_effects,
            one_shot: delayed_trigger_is_one_shot(trigger_clause),
            until_end_of_combat: false,
            attach_to_previous_ability: true,
        })]));
    }
    if delayed_shapes::is_delayed_prior_object_put_into_a_graveyard(trigger_core_tokens) {
        let delayed_effects = parse_effect_chain(shape.effect_tokens)?;
        if delayed_effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing delayed prior-object graveyard effect clause (clause: '{}')",
                clause_display.trim()
            )));
        }
        return Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
            trigger: TriggerSpec::PutIntoGraveyard(ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind())),
            effects: delayed_effects,
            one_shot: true,
            until_end_of_combat: false,
            attach_to_previous_ability: true,
        })]));
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
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                // `target` identifies the chosen object, not its controller.
                player: PlayerAst::Implicit,
                tag: crate::tag::TagRef::of(tag),
            }),
            EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
                trigger: TriggerSpec::DealsCombatDamageTo {
                    source: watched_filter,
                    target: recipient,
                },
                effects: delayed_effects,
                one_shot: false,
                until_end_of_combat: false,
                attach_to_previous_ability: false,
            }),
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
    Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
        trigger,
        effects: delayed_effects,
        one_shot,
        until_end_of_combat: false,
        attach_to_previous_ability: shape.references_previous_creature,
    })]))
}

pub fn parse_delayed_when_that_dies_this_turn_sentence(
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
            let subject_words = crate::lexer::token_word_refs(subject_tokens);
            let noun = subject_words
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
    let delayed_effects = if crate::word_primitives::parse_any_sequence_complete(
        &remainder_words,
        &[
            &["exile", "its", "controllers", "graveyard"],
            &["exile", "its", "controller's", "graveyard"],
            &["exile", "its", "controller", "s", "graveyard"],
        ],
    ) {
        let mut graveyard = ObjectFilter::default();
        graveyard.zone = Some(Zone::Graveyard);
        graveyard.owner = Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::Tagged((crate::tag::CompilerReferenceTag::It.bind()).into()),
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

    Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectDiesThisTurn {
        filter: delayed_filter,
        effects: delayed_effects,
    })]))
}

pub fn parse_delayed_when_that_leaves_battlefield_sentence(
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
        EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectLeavesBattlefield {
            filter,
            effects: delayed_effects,
        }),
    ]))
}

pub fn find_from_among(tokens: &[OwnedLexToken]) -> Option<usize> {
    crate::lexer::find_token_word_sequence(tokens, &["from", "among"])
}

pub fn find_list_start(tokens: &[OwnedLexToken]) -> Option<usize> {
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

pub fn split_choose_list(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
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

pub fn merge_filters(base: &ObjectFilter, specific: &ObjectFilter) -> ObjectFilter {
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
#[path = "copy_and_next_spell_shapes_inline_copy_and_next_spell_shape_tests.rs"]
mod copy_and_next_spell_shape_tests;

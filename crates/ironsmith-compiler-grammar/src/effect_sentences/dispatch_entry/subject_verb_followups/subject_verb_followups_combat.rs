use super::*;
use crate::cards::builders::ForEachEffectAst;

/// Bind the omitted recipient before the independent conditional/clause
/// readers run. Copy the complete original action and change only its amount;
/// the existing self-replacement lowering owns the shared target declaration,
/// condition timing and mutually exclusive execution of the two arms.
pub(super) fn pre_rule_damage_amount_replacement(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_damage_amount_replacement(sentence_tokens) else {
        return Ok(None);
    };
    let Some(outer_previous) = state.effects.last() else {
        return Ok(None);
    };
    // "When you do, this creature deals damage ... . If ..., this creature
    // deals twice that much damage instead." (Surtland Flinger): the
    // replacement modifies the damage inside the reflexive trigger's body.
    let in_reflexive_body = matches!(
        outer_previous,
        EffectAst::Conditionals(ConditionalEffectAst::WhenResult { effects, .. })
            if effects.last().is_some_and(|last| primary_damage_source_from_effect(last).is_some())
    );
    let previous = match outer_previous {
        EffectAst::Conditionals(ConditionalEffectAst::WhenResult { effects, .. })
            if in_reflexive_body =>
        {
            effects.last().expect("checked non-empty reflexive body")
        }
        other => other,
    };
    let Some(source) = primary_damage_source_from_effect(previous) else {
        return Ok(None);
    };
    // "it deals twice that much damage to itself instead" repeats a
    // self-damage instruction's recipient, never a declared target or player.
    // A declared target counts only when it is the instruction's own source
    // ("Target creature deals damage to itself ..."; Cut Propulsion).
    if shape.self_recipient
        && primary_damage_target_from_effect(previous).is_none_or(|target| {
            matches!(target, TargetAst::Player(..))
                || (matches!(target, TargetAst::Object(_, Some(_), _)) && target != source)
        })
    {
        return Ok(None);
    }
    // A definite description ("the creature you control") names the
    // preceding instruction's explicitly targeted damage source; it never
    // names a fresh object.
    let definite_previous_source =
        shape.definite_source && matches!(&source, TargetAst::Object(_, Some(_), _));
    if !shape.repeats_source
        && !definite_previous_source
        && !matches!(&source, TargetAst::Source(_))
        && !matches!(&source, TargetAst::Object(filter, None, _) if filter.source)
    {
        return Ok(None);
    }
    // "twice that much": the amount the replaced instruction would deal,
    // doubled. Both arms resolve at the same time, so the default arm's own
    // amount expression is exactly "that much" (CR 614.1a).
    // "deals double that damage instead" (Goblin Charbelcher) is the same
    // doubled amount.
    let doubles_prior_amount = crate::grammar::primitives::probe_all(
        shape.amount_tokens,
        crate::grammar::primitives::phrase(&["twice", "that", "much"]),
        "twice that much damage",
    )
    .is_some()
        || crate::grammar::primitives::probe_all(
            shape.amount_tokens,
            crate::grammar::primitives::phrase(&["double", "that"]),
            "double that damage",
        )
        .is_some();
    let amount = if doubles_prior_amount {
        None
    } else {
        let Some((amount, used)) = crate::util::parse_value(shape.amount_tokens) else {
            return Ok(None);
        };
        if used != shape.amount_tokens.len() {
            return Ok(None);
        }
        // Other event quantities need their own exact event/LKI owner. This
        // family admits only a literal or the spell's X plus a literal.
        fn scalar_amount(value: &Value) -> bool {
            match value.unhinted() {
                Value::Fixed(_) | Value::X => true,
                // The optional reveal/choose additional cost freezes this
                // object's identity on the spell; the replacement reads its
                // characteristic with the existing tagged-object/LKI path.
                Value::PowerOf(spec) | Value::ToughnessOf(spec)
                    if matches!(spec.as_ref(), crate::target::ChooseSpec::Tagged(tag)
                        if tag.as_str() == crate::tag::CompilerReferenceTag::BeheldCost0.as_str()) => true,
                Value::Add(left, right) => scalar_amount(left) && scalar_amount(right),
                _ => false,
            }
        }
        if !scalar_amount(&amount) {
            return Ok(None);
        }
        Some(amount)
    };
    fn replace_amount(effect: &mut EffectAst, replacement: Option<&Value>) -> bool {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Damage(
                    DamageActionAst::DealDamage { amount, .. }
                    | DamageActionAst::DealDamageEqualToPower { amount, .. },
                ),
                ..
            }) => {
                *amount = match replacement {
                    Some(replacement) => replacement.clone(),
                    None => Value::Scaled(Box::new(amount.clone()), 2),
                };
                true
            }
            EffectAst::SourceSentence { effects, .. } | EffectAst::Sequence { effects }
                if effects.len() == 1 => replace_amount(&mut effects[0], replacement),
            // "deals damage equal to the number of nonland cards revealed
            // this way" (Goblin Charbelcher) can carry snapshot/tag steps
            // before the damage instruction; the instruction is the last step.
            EffectAst::SourceSentence { effects, .. } | EffectAst::Sequence { effects }
                if effects.len() > 1
                    && effects[..effects.len() - 1].iter().all(|effect| {
                        !matches!(
                            effect,
                            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                                action: SubjectVerbActionAst::Damage(_),
                                ..
                            })
                        )
                    }) =>
            {
                let last = effects.len() - 1;
                replace_amount(&mut effects[last], replacement)
            }
            _ => false,
        }
    }
    let mut replacement = previous.clone();
    if !replace_amount(&mut replacement, amount.as_ref()) {
        return Ok(None);
    }
    let Some(predicate) =
        consult_match_quality_predicate(&state.effects[..], shape.predicate_tokens)
            .or_else(|| parse_trailing_if_predicate_lexed(shape.predicate_tokens))
    else {
        return Ok(None);
    };
    fn has_unbound_cast_time_control(predicate: &PredicateAst) -> bool {
        match predicate {
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControls {
                filter, ..
            }) => filter.has_as_you_cast_this_turn_surface(),
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
                has_unbound_cast_time_control(left) || has_unbound_cast_time_control(right)
            }
            PredicateAst::Not(inner) => has_unbound_cast_time_control(inner),
            _ => false,
        }
    }
    // CR 601.2: "if you controlled a modified creature as you cast this
    // spell" reads the cast-time evidence the stack entry froze under
    // `CastModifiedCreatures` (the same snapshot "for each modified creature
    // you controlled as you cast this spell" counts), never current control.
    fn bind_cast_time_control(predicate: PredicateAst) -> PredicateAst {
        match predicate {
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter,
            }) if filter.has_as_you_cast_this_turn_surface()
                && filter.modified
                && filter.card_types == [crate::types::CardType::Creature] =>
            {
                PredicateAst::ValueComparison {
                    left: Value::Count(crate::target::ObjectFilter::tagged(
                        crate::tag::CompilerReferenceTag::CastModifiedCreatures.bind(),
                    )),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                }
            }
            PredicateAst::And(left, right) => PredicateAst::And(
                Box::new(bind_cast_time_control(*left)),
                Box::new(bind_cast_time_control(*right)),
            ),
            PredicateAst::Or(left, right) => PredicateAst::Or(
                Box::new(bind_cast_time_control(*left)),
                Box::new(bind_cast_time_control(*right)),
            ),
            PredicateAst::Not(inner) => {
                PredicateAst::Not(Box::new(bind_cast_time_control(*inner)))
            }
            other => other,
        }
    }
    let predicate = bind_cast_time_control(predicate);
    if has_unbound_cast_time_control(&predicate) {
        return Err(CardTextError::ParseError(
            "damage amount replacement requires retained cast-time control evidence".into(),
        ));
    }
    let predicate = bind_self_replacement_condition_to_previous_target(
        predicate,
        shape.predicate_tokens,
        primary_damage_target_from_effect(previous).as_ref(),
    );
    let slot = if in_reflexive_body {
        match state.effects.last_mut() {
            Some(EffectAst::Conditionals(ConditionalEffectAst::WhenResult { effects, .. })) => {
                effects
            }
            _ => unreachable!("checked reflexive body"),
        }
    } else {
        &mut *state.effects
    };
    let previous = slot.pop().expect("the bound damage instruction exists");
    slot.push(EffectAst::SelfReplacement {
        predicate,
        if_true: vec![replacement],
        if_false: vec![previous],
        attach_to_previous_ability: false,
    });
    *state.carried_context = None;
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: Some("subject-verb verb=Deal subject=source recognizer=damage-amount-replacement"),
    }))
}

/// The match tag of the latest reveal traversal ("reveal cards ... until you
/// reveal a land card") among the instructions parsed so far.
pub(super) fn last_reveal_consult_match_tag(effects: &[EffectAst]) -> Option<crate::tag::TagRef> {
    for effect in effects.iter().rev() {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Library(crate::cards::builders::LibraryActionAst::ConsultTopOfLibrary {
                    mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                    match_tag,
                    ..
                }),
            ..
        }) = effect
        {
            return Some(match_tag.clone());
        }
        let mut found = None;
        for_each_nested_effects(effect, false, |nested| {
            if found.is_none() {
                found = last_reveal_consult_match_tag(nested);
            }
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

/// "If the revealed land card was a Mountain, ..." (Goblin Charbelcher):
/// after a reveal traversal, the definite "revealed <kind> card" is the card
/// that stopped the traversal, never the ordinary `it` antecedent a later
/// instruction made current (such as a damage recipient). Read the quality
/// as "it is <quality>" and test it against the traversal's match tag.
pub(super) fn consult_match_quality_predicate(
    effects: &[EffectAst],
    predicate_tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let tokens = crate::lexer::trim_lexed_commas(predicate_tokens);
    let verb = tokens
        .iter()
        .position(|token| token.is_word("was") || token.is_word("is"))?;
    if verb < 4
        || !tokens[0].is_word("if")
        || !tokens[1].is_word("the")
        || !tokens[2].is_word("revealed")
        || !tokens[verb - 1].is_word("card")
        || verb + 1 >= tokens.len()
        || !tokens[3..verb].iter().all(|token| token.as_word().is_some())
    {
        return None;
    }
    let match_tag = last_reveal_consult_match_tag(effects)?;
    let mut rewritten = vec![
        tokens[0].clone(),
        OwnedLexToken::word("it", tokens[1].span),
        OwnedLexToken::word("is", tokens[verb].span),
    ];
    rewritten.extend_from_slice(&tokens[verb + 1..]);
    let filter = match parse_trailing_if_predicate_lexed(&rewritten)? {
        PredicateAst::ItMatches(filter) => filter,
        PredicateAst::TaggedMatches(tag, filter)
            if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() =>
        {
            filter
        }
        _ => return None,
    };
    Some(PredicateAst::TaggedMatches(match_tag, filter))
}

pub(super) fn primary_damage_source_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { .. }) => {
                Some(TargetAst::Source(None))
            }
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                source,
                ..
            })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage {
                source, ..
            }) => Some(source.clone()),
            _ => None,
        },
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_damage_source_from_effect);
                }
            });
            found
        }
    }
}

pub(super) fn replace_anaphoric_damage_source_in_effects(
    effects: &mut [EffectAst],
    source: &TargetAst,
) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                    source: effect_source,
                    ..
                })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage {
                    source: effect_source,
                    ..
                }) if target_references_it(effect_source) => {
                    *effect_source = source.clone();
                }
                _ => {}
            },
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                replace_anaphoric_damage_source_in_effects(nested, source);
            }),
        }
    }
}

pub(super) fn sole_damage_payload(effects: &[EffectAst]) -> Option<(Value, bool)> {
    let [effect] = effects else {
        return None;
    };
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                    amount,
                    unpreventable,
                    ..
                })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                    amount,
                    unpreventable,
                    ..
                }),
            ..
        }) => Some((amount.clone(), *unpreventable)),
        EffectAst::Sequence { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::Coordinated { effects, .. } => sole_damage_payload(effects),
        _ => None,
    }
}

/// Collapse an authored singular damage-source anaphor before reference
/// resolution can interpret it as the most recent object result. In a
/// self-replacement, both "It" and "that creature" repeat the source and
/// target of the default damage event; neither refers to an object used to pay
/// an earlier cost.
pub(super) fn normalize_anaphoric_damage_self_replacement(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
    source: &TargetAst,
    target: &TargetAst,
) -> bool {
    if !effect_grammar::followup_shapes::is_anaphoric_damage_self_replacement(tokens) {
        return false;
    }
    let Some((amount, unpreventable)) = sole_damage_payload(effects) else {
        return false;
    };
    *effects = vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
            source: source.clone(),
            amount,
            target: target.clone(),
            unpreventable,
        }),
    )];
    true
}

/// "That player or that planeswalker's controller may choose a creature they
/// control and have this spell deal that damage to it instead." (Flaming
/// Gambit): the damaged player (or the damaged planeswalker's controller)
/// picks the recipient as the spell resolves, before the damage is dealt
/// once — to the chosen object, or to the original target if they choose
/// nothing. The prior damage instruction becomes the declined arm.
pub(super) fn pre_rule_damage_recipient_choice_instead(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    const CHOOSERS: &[&[&str]] = &[
        &["that", "player", "or", "that", "planeswalker's", "controller"],
        &["that", "player", "or", "that", "planeswalkers", "controller"],
        &["that", "player", "or", "that", "planeswalker", "s", "controller"],
    ];
    const TAIL: &[&str] = &["deal", "that", "damage", "to", "it", "instead"];
    let tokens = crate::util::trim_edge_punctuation_tokens(sentence_tokens);
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    let Some(chooser_len) = CHOOSERS
        .iter()
        .find(|phrase| words.starts_with(phrase))
        .map(|phrase| phrase.len())
    else {
        return Ok(None);
    };
    if words.get(chooser_len..chooser_len + 2) != Some(&["may", "choose"][..])
        || !words.ends_with(TAIL)
    {
        return Ok(None);
    }
    let filter_start = chooser_len + 2;
    let Some(have) = words[filter_start..]
        .windows(2)
        .position(|pair| pair == ["and", "have"])
        .map(|offset| filter_start + offset)
    else {
        return Ok(None);
    };
    // "have <source> deal": the source must be named between them.
    if have + 2 >= words.len() - TAIL.len() || have == filter_start {
        return Ok(None);
    }
    let Some(previous) = state.effects.last() else {
        return Ok(None);
    };
    if !matches!(
        primary_damage_target_from_effect(previous),
        Some(TargetAst::PlayerOrPlaneswalker(..))
    ) || !matches!(primary_damage_source_from_effect(previous), Some(TargetAst::Source(_)))
    {
        return Ok(None);
    }
    fn retarget_sole_damage(effect: &mut EffectAst, recipient: &TargetAst) -> bool {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { target, .. }),
                ..
            }) => {
                *target = recipient.clone();
                true
            }
            EffectAst::SourceSentence { effects, .. } | EffectAst::Sequence { effects }
                if effects.len() == 1 =>
            {
                retarget_sole_damage(&mut effects[0], recipient)
            }
            _ => false,
        }
    }
    let Some(filter_range) = view.token_span_for_words(filter_start, have) else {
        return Ok(None);
    };
    let Ok(filter) = crate::object_filters::parse_object_filter(&tokens[filter_range], false)
    else {
        return Ok(None);
    };
    // "they control": the choosing player — the damaged player, or the
    // damaged planeswalker's controller.
    let filter = filter
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::TargetPlayerOrControllerOfTarget);
    let tag = crate::util::helper_tag_for_tokens(tokens, "damage_recipient_choice");
    let mut redirected = previous.clone();
    if !retarget_sole_damage(&mut redirected, &TargetAst::Tagged(tag.clone(), None)) {
        return Ok(None);
    }
    let original = state.effects.pop().expect("the bound damage instruction exists");
    // Announce the original recipient before asking that player (or the
    // planeswalker controller) whether to redirect. Both damage branches
    // reuse this declaration; neither branch creates an additional target.
    let recipient = primary_damage_target_from_effect(&original)
        .expect("checked original damage recipient");
    state.effects.push(EffectAst::subject_verb_explicit_target_only(recipient));
    state.effects.push(EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
        player: PlayerAst::ThatPlayerOrTargetController,
        effects: vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::ThatPlayerOrTargetController,
            tag: tag.clone(),
        })],
    }));
    state.effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::TaggedMatches(tag, ObjectFilter::default()),
        if_true: vec![redirected],
        if_false: vec![original],
    }));
    *state.carried_context = None;
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: Some("subject-verb verb=Deal subject=source recognizer=damage-recipient-choice-instead"),
    }))
}

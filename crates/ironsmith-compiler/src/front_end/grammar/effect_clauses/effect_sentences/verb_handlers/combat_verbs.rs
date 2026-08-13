use super::super::grammar::effects::combat_shapes as combat_grammar;

fn attach_tagged_filter(
    shape: combat_grammar::CombatAttachTaggedObjectShape,
) -> Option<ObjectFilter> {
    let mut filter = ObjectFilter::default();
    match shape {
        combat_grammar::CombatAttachTaggedObjectShape::Plain => return None,
        combat_grammar::CombatAttachTaggedObjectShape::Equipment => {
            filter.card_types.push(CardType::Artifact);
            filter.subtypes.push(Subtype::Equipment);
        }
        combat_grammar::CombatAttachTaggedObjectShape::Aura => {
            filter.card_types.push(CardType::Enchantment);
            filter.subtypes.push(Subtype::Aura);
        }
        combat_grammar::CombatAttachTaggedObjectShape::Artifact => {
            filter.card_types.push(CardType::Artifact);
        }
        combat_grammar::CombatAttachTaggedObjectShape::Enchantment => {
            filter.card_types.push(CardType::Enchantment);
        }
    }
    filter.zone = Some(Zone::Battlefield);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    Some(filter)
}

fn parse_combat_player_damage_target(
    tokens: &[OwnedLexToken],
    allow_prefix: bool,
) -> Option<combat_grammar::CombatPlayerDamageTargetShape> {
    combat_grammar::parse_combat_player_damage_target_shape_lexed(tokens, allow_prefix)
}

fn combat_player_damage_target_effect(
    amount: Value,
    target: combat_grammar::CombatPlayerDamageTargetShape,
) -> EffectAst {
    match target {
        combat_grammar::CombatPlayerDamageTargetShape::EachPlayer => EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_damage(
                amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        },
        combat_grammar::CombatPlayerDamageTargetShape::EachOtherPlayer => {
            EffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::NotYou,
                effects: vec![EffectAst::subject_verb_damage(
                    amount,
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            }
        }
        combat_grammar::CombatPlayerDamageTargetShape::EachOpponent => EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        },
        combat_grammar::CombatPlayerDamageTargetShape::EachOtherOpponent => {
            damage_each_other_opponent(amount)
        }
    }
}

fn combat_player_damage_target_filter(
    target: combat_grammar::CombatPlayerDamageTargetShape,
) -> PlayerFilter {
    match target {
        combat_grammar::CombatPlayerDamageTargetShape::EachPlayer => PlayerFilter::Any,
        combat_grammar::CombatPlayerDamageTargetShape::EachOtherPlayer => PlayerFilter::NotYou,
        combat_grammar::CombatPlayerDamageTargetShape::EachOpponent => PlayerFilter::Opponent,
        combat_grammar::CombatPlayerDamageTargetShape::EachOtherOpponent => {
            PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::DamagedPlayer)
        }
    }
}

fn combat_simple_damage_target_ast(
    shape: combat_grammar::CombatSimpleDamageTargetShape,
    tokens: &[OwnedLexToken],
) -> TargetAst {
    match shape {
        combat_grammar::CombatSimpleDamageTargetShape::DefaultAny => {
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
        }
        combat_grammar::CombatSimpleDamageTargetShape::CreatureController => TargetAst::Player(
            PlayerFilter::ControllerOf(crate::target::ObjectRef::tagged(IT_TAG)),
            span_from_tokens(tokens),
        ),
        combat_grammar::CombatSimpleDamageTargetShape::IteratedPlayer => {
            TargetAst::Player(PlayerFilter::IteratedPlayer, span_from_tokens(tokens))
        }
    }
}

fn damage_each_other_opponent(amount: Value) -> EffectAst {
    EffectAst::ForEachPlayersFiltered {
        filter: PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::DamagedPlayer),
        effects: vec![EffectAst::subject_verb_damage(
            amount,
            TargetAst::Player(PlayerFilter::IteratedPlayer, None),
        )],
    }
}

fn damage_to_embedded_target_controller(
    amount: Value,
    target_tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let anchor =
        match combat_grammar::parse_combat_embedded_target_controller_shape_lexed(target_tokens)? {
            combat_grammar::CombatEmbeddedTargetControllerShape::Spell => {
                TargetAst::Spell(span_from_tokens(target_tokens))
            }
        };
    let recipient = TargetAst::Player(
        PlayerFilter::ControllerOf(crate::target::ObjectRef::tagged(IT_TAG)),
        None,
    );
    Some(EffectAst::Sequence {
        effects: vec![
            EffectAst::subject_verb_target_only(anchor),
            EffectAst::subject_verb_damage(amount, recipient),
        ],
    })
}

pub(crate) fn parse_attach_object_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let object_span = span_from_tokens(tokens);
    let shape = combat_grammar::parse_combat_attach_object_shape_lexed(tokens)
        .ok_or_else(|| CardTextError::ParseError("missing object to attach".to_string()))?;
    match shape {
        combat_grammar::CombatAttachObjectShape::Source
        | combat_grammar::CombatAttachObjectShape::NameLikeSource => {
            Ok(TargetAst::Source(object_span))
        }
        combat_grammar::CombatAttachObjectShape::Tagged(shape) => {
            if let Some(tagged_filter) = attach_tagged_filter(shape) {
                Ok(TargetAst::Object(tagged_filter, None, None))
            } else {
                Ok(TargetAst::Tagged(TagKey::from(IT_TAG), object_span))
            }
        }
        combat_grammar::CombatAttachObjectShape::All { object_tokens } => {
            let mut filter = parse_object_filter(object_tokens, false)?;
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }
            Ok(TargetAst::Object(filter, None, None))
        }
        combat_grammar::CombatAttachObjectShape::Counted {
            count,
            object_tokens,
            starts_with_target,
        } => {
            let target = if starts_with_target {
                parse_target_phrase(object_tokens)?
            } else {
                let mut filter = parse_object_filter(object_tokens, false)?;
                if filter.zone.is_none() {
                    filter.zone = Some(Zone::Battlefield);
                }
                TargetAst::Object(filter, None, None)
            };
            Ok(TargetAst::WithCount(Box::new(target), count))
        }
        combat_grammar::CombatAttachObjectShape::Target { target_tokens }
        | combat_grammar::CombatAttachObjectShape::GeneralTarget { target_tokens } => {
            parse_target_phrase(target_tokens)
        }
    }
}
pub(crate) fn parse_attach(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let clause = crate::token_word_refs(tokens).join(" ");
    let shape =
        combat_grammar::parse_combat_attach_clause_shape_lexed(tokens).map_err(|error| {
            let message = match error {
                combat_grammar::CombatAttachClauseError::MissingDestination => {
                    format!("attach clause missing destination (clause: '{clause}')")
                }
                combat_grammar::CombatAttachClauseError::MissingObjectOrDestination => {
                    if tokens.is_empty() {
                        "attach clause missing object and destination".to_string()
                    } else {
                        format!("attach clause missing object or destination (clause: '{clause}')")
                    }
                }
            };
            CardTextError::ParseError(message)
        })?;

    match shape {
        combat_grammar::CombatAttachClauseShape::DestinationFirstTagged {
            tagged_tokens,
            object_tokens,
        } => {
            let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tagged_tokens));
            let object = parse_attach_object_phrase(object_tokens)?;
            Ok(EffectAst::subject_verb_attach(object, target))
        }
        combat_grammar::CombatAttachClauseShape::Standard {
            object_tokens,
            target_tokens,
            triggering_object_to_token,
            target_is_tagged,
        } => {
            if triggering_object_to_token {
                return Ok(EffectAst::subject_verb_attach(
                    TargetAst::Tagged(TagKey::from("triggering"), span_from_tokens(object_tokens)),
                    TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(target_tokens)),
                ));
            }
            if let Some(host_tokens) =
                grammar::match_word_prefix(object_tokens, &["all", "auras", "enchanting"])
            {
                let destination_words = crate::token_word_refs(target_tokens);
                if matches!(
                    destination_words.as_slice(),
                    ["another", "permanent", "with", "same", "controller"]
                        | ["another", "permanent", "with", "the", "same", "controller"]
                ) {
                    let host = parse_target_phrase(host_tokens)?;
                    let mut aura_filter = ObjectFilter::permanent().in_zone(Zone::Battlefield);
                    aura_filter.subtypes.push(Subtype::Aura);
                    aura_filter.tagged_constraints.push(TaggedObjectConstraint {
                        tag: TagKey::from(IT_TAG),
                        relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                    });

                    let mut destination = ObjectFilter::permanent().in_zone(Zone::Battlefield);
                    for relation in [
                        TaggedOpbjectRelation::SameControllerAsTagged,
                        TaggedOpbjectRelation::IsNotTaggedObject,
                    ] {
                        destination.tagged_constraints.push(TaggedObjectConstraint {
                            tag: TagKey::from(IT_TAG),
                            relation,
                        });
                    }

                    return Ok(EffectAst::Sequence {
                        effects: vec![
                            EffectAst::subject_verb_target_only(host),
                            EffectAst::subject_verb_attach(
                                TargetAst::Object(aura_filter, None, None),
                                TargetAst::Object(destination, None, None),
                            ),
                        ],
                    });
                }
            }
            let object = parse_attach_object_phrase(object_tokens)?;
            let mut target = if target_is_tagged {
                TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(target_tokens))
            } else {
                parse_target_phrase(target_tokens)?
            };
            if crate::grammar::filters::reference_tag_stage::has_plural_object_head_surface(
                target_tokens,
            )
                && let Some(filter) =
                crate::sentences::effect_sentences::zone_counter_helpers::target_object_filter_mut(
                    &mut target,
                )
            {
                filter.set_plural_object_noun_surface(true);
            }
            Ok(EffectAst::subject_verb_attach(object, target))
        }
    }
}
fn parse_attached_object_reference(tokens: &[OwnedLexToken]) -> Option<TargetAst> {
    let shape = combat_grammar::parse_attached_object_reference_tokens(tokens)?;
    let tag = match shape.tag {
        combat_grammar::AttachedObjectReferenceTag::Enchanted => "enchanted",
        combat_grammar::AttachedObjectReferenceTag::Equipped => "equipped",
    };
    let mut filter = match shape.kind {
        combat_grammar::AttachedObjectReferenceKind::Equipment => {
            let mut filter = ObjectFilter::permanent();
            filter.card_types.push(CardType::Artifact);
            filter.subtypes.push(Subtype::Equipment);
            filter
        }
        combat_grammar::AttachedObjectReferenceKind::Artifact => {
            let mut filter = ObjectFilter::permanent();
            filter.card_types.push(CardType::Artifact);
            filter
        }
        combat_grammar::AttachedObjectReferenceKind::Creature => ObjectFilter::creature(),
        combat_grammar::AttachedObjectReferenceKind::Enchantment => {
            let mut filter = ObjectFilter::permanent();
            filter.card_types.push(CardType::Enchantment);
            filter
        }
        combat_grammar::AttachedObjectReferenceKind::Land => ObjectFilter::land(),
        combat_grammar::AttachedObjectReferenceKind::Permanent => ObjectFilter::permanent(),
    }
    .match_tagged(TagKey::from(tag), TaggedOpbjectRelation::IsTaggedObject);
    filter.zone = Some(Zone::Battlefield);
    Some(TargetAst::Object(filter, None, None))
}

pub(crate) fn parse_unattach(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "unattach clause missing object".to_string(),
        ));
    }

    let object_tokens = trim_commas(tokens);
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "unattach clause missing object".to_string(),
        ));
    }

    if let Some(target_tokens) =
        grammar::match_word_prefix(&object_tokens, &["all", "equipment", "from"])
    {
        let target = parse_target_phrase(target_tokens)?;
        let mut equipment_filter = ObjectFilter::permanent();
        equipment_filter.card_types.push(CardType::Artifact);
        equipment_filter.subtypes.push(Subtype::Equipment);
        equipment_filter.zone = Some(Zone::Battlefield);
        equipment_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::AttachedToTaggedObject,
            });

        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::subject_verb_unattach(TargetAst::WithCount(
                    Box::new(TargetAst::Object(equipment_filter, None, None)),
                    ChoiceCount::any_number(),
                )),
            ],
        });
    }

    let object = parse_attached_object_reference(&object_tokens)
        .map(Ok)
        .unwrap_or_else(|| parse_target_phrase(&object_tokens))?;
    Ok(EffectAst::subject_verb_unattach(object))
}

pub(crate) fn damage_clause_has_terminal_unpreventable_rider(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::token_word_refs(tokens);
    const RIDERS: &[&[&str]] = &[
        &["and", "the", "damage", "cant", "be", "prevented"],
        &["and", "the", "damage", "can't", "be", "prevented"],
        &["and", "that", "damage", "cant", "be", "prevented"],
        &["and", "that", "damage", "can't", "be", "prevented"],
    ];
    RIDERS.iter().any(|rider| {
        words
            .get(words.len().saturating_sub(rider.len())..)
            .is_some_and(|tail| tail == *rider)
    })
}

pub(crate) fn mark_damage_ast_unpreventable(effect: &mut EffectAst) {
    if let EffectAst::SubjectVerb(subject_verb) = effect {
        match &mut subject_verb.action {
            SubjectVerbActionAst::DealDamage { unpreventable, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { unpreventable, .. } => {
                *unpreventable = true;
            }
            _ => {}
        }
    }
    crate::model::visit::for_each_nested_effects_mut(
        effect,
        true,
        |nested| {
            for nested_effect in nested {
                mark_damage_ast_unpreventable(nested_effect);
            }
        },
    );
}

pub(crate) fn parse_deal_damage(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let has_unpreventable_rider = damage_clause_has_terminal_unpreventable_rider(tokens);
    let mut effect = parse_deal_damage_inner(tokens)?;
    if has_unpreventable_rider {
        mark_damage_ast_unpreventable(&mut effect);
    }
    Ok(effect)
}

fn parse_damage_each_filter(
    filter_tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let mut filter = parse_object_filter(filter_tokens, false)?;
    let words = crate::token_word_refs(filter_tokens);
    if words.first() == Some(&"those") || words.starts_with(&["of", "those"]) {
        filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those));
    }
    Ok(filter)
}

fn parse_deal_damage_inner(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let shape = combat_grammar::parse_combat_damage_head_shape_lexed(tokens);
    let tokens = shape.body_tokens;
    let clause_words = crate::token_word_refs(tokens);
    if shape.direct_hand_size_each_opponent {
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                Value::CardsInHand(PlayerFilter::IteratedPlayer),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        });
    }
    if shape.divided {
        if let Some((value, used)) = parse_value(tokens) {
            return parse_divided_damage_with_amount(tokens, value, used);
        }
        if let Some(effect) = parse_divided_damage_equal_to_amount(tokens)? {
            return Ok(effect);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported divided-damage distribution clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) = parse_deal_damage_equal_to_clause(tokens)? {
        return Ok(effect);
    }
    if let Some(effect) = parse_deal_damage_to_target_equal_to_clause(tokens)? {
        return Ok(effect);
    }
    if let Some(prefix_len) = shape.event_amount_prefix_len {
        return parse_deal_damage_with_amount(
            tokens,
            Value::EventValue(EventValueSpec::Amount),
            prefix_len,
        );
    }

    if let Some((value, used)) = parse_value(tokens) {
        return parse_deal_damage_with_amount(tokens, value, used);
    }

    if shape.fallback_hand_size_each_opponent {
        let value = Value::CardsInHand(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                value,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        });
    }

    Err(CardTextError::ParseError(format!(
        "missing damage amount (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_divided_damage_equal_to_amount(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = combat_grammar::parse_combat_divided_equal_shape_lexed(tokens) else {
        return Ok(None);
    };
    let words = crate::token_word_refs(tokens);
    let Some((amount, used)) = parse_value(shape.amount_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage amount (clause: '{}')",
            words.join(" ")
        )));
    };
    if used != shape.amount_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported divided-damage amount (clause: '{}')",
            words.join(" ")
        )));
    }
    let target = parse_divided_damage_target(shape.target_tokens)?;
    let chooser = if crate::token_word_refs(shape.target_tokens)
        .windows(4)
        .any(|window| window == ["as", "its", "controller", "chooses"])
    {
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Target)
    } else {
        PlayerFilter::You
    };
    Ok(Some(
        EffectAst::subject_verb_distributed_damage_with_source(
            amount,
            target,
            TargetAst::Source(None),
            chooser,
        ),
    ))
}

fn preserve_equal_to_surface(value: Value) -> Value {
    if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo) {
        value
    } else {
        value.with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo)
    }
}

/// Preserve an authored optional single target on the equal-to-damage
/// family. Some broader subject/verb routes have already normalized a bare
/// object union by the time the target reaches this handler; the terminal
/// words remain the authoritative proof that choosing no target is legal.
fn preserve_optional_single_damage_target(
    target: TargetAst,
    target_tokens: &[OwnedLexToken],
) -> TargetAst {
    let words = crate::token_word_refs(target_tokens);
    if !words.starts_with(&["up", "to", "one", "target"]) {
        return target;
    }

    match target {
        TargetAst::WithCount(inner, count) if count == ChoiceCount::up_to(1) => {
            TargetAst::WithCount(inner, count)
        }
        TargetAst::WithCount(inner, count) if count.is_single() => {
            TargetAst::WithCount(inner, ChoiceCount::up_to(1))
        }
        target @ TargetAst::WithCountValue(..) => target,
        other => TargetAst::WithCount(Box::new(other), ChoiceCount::up_to(1)),
    }
}

pub(crate) fn parse_deal_damage_to_target_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = combat_grammar::parse_combat_damage_to_target_equal_shape_lexed(tokens)
    else {
        return Ok(None);
    };
    let clause_words = crate::token_word_refs(tokens);
    // A relative-controller count must preempt the tolerant generic value
    // parser, which otherwise absorbs the antecedent noun into the counted
    // object filter (for example, Land + Creature).
    let amount = parse_equal_to_aggregate_filter_value(tokens)
        .or(parse_equal_to_number_of_filter_value(tokens))
        .or(parse_add_mana_equal_amount_value(tokens))
        .or(parse_devotion_value_from_add_clause(tokens)?)
        .or_else(|| {
            shape
                .amount_is_event_result
                .then_some(Value::EventValue(EventValueSpec::Amount))
        })
        .or(parse_dynamic_cost_modifier_value(tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let amount = preserve_equal_to_surface(amount);
    if let Some(effect) = damage_to_embedded_target_controller(amount.clone(), shape.target_tokens)
    {
        return Ok(Some(effect));
    }
    if let Some(target) = parse_combat_player_damage_target(shape.target_tokens, false) {
        return Ok(Some(combat_player_damage_target_effect(
            amount.clone(),
            target,
        )));
    }
    if shape.target_is_each_or_all {
        if shape.target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter = parse_damage_each_filter(&shape.target_tokens[1..])?;
        return Ok(Some(EffectAst::subject_verb_damage_each(amount, filter)));
    }
    let target = preserve_optional_single_damage_target(
        parse_target_phrase(shape.target_tokens)?,
        shape.target_tokens,
    );
    Ok(Some(EffectAst::subject_verb_damage(amount, target)))
}
pub(crate) fn parse_deal_damage_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = combat_grammar::parse_combat_damage_equal_shape_lexed(tokens) else {
        return Ok(None);
    };
    let clause_words = crate::token_word_refs(tokens);
    let authored_difference = crate::token_word_refs(shape.amount_tokens)
        .windows(2)
        .any(|window| window == ["difference", "between"])
        .then(|| parse_add_mana_equal_amount_value(shape.amount_tokens))
        .flatten();
    // Count expressions with a relative controller tail need the typed count
    // parser before the permissive value-expression fallback. Otherwise
    // `nonbasic lands that creature's controller controls` is accepted as a
    // single Land+Creature filter and loses the same-target controller link.
    let aggregate_amount = parse_equal_to_aggregate_filter_value(shape.amount_tokens);
    let relative_count = parse_equal_to_number_of_filter_value(shape.amount_tokens);
    let complete_value = aggregate_amount
        .or(authored_difference)
        .or(relative_count)
        .or_else(|| {
            parse_value(shape.amount_tokens)
                .and_then(|(value, used)| (used == shape.amount_tokens.len()).then_some(value))
                .map(preserve_equal_to_surface)
        });
    let amount = complete_value
        .or(parse_add_mana_equal_amount_value(shape.amount_tokens))
        .or(parse_devotion_value_from_add_clause(shape.amount_tokens)?)
        .or(parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
            shape.amount_tokens,
        ))
        .or(parse_equal_to_number_of_opponents_you_have_value(
            shape.amount_tokens,
        ))
        .or(parse_equal_to_number_of_counters_on_reference_value(
            shape.amount_tokens,
        ))
        .or(parse_dynamic_cost_modifier_value(shape.amount_tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let amount = preserve_equal_to_surface(amount);
    if let Some(effect) = damage_to_embedded_target_controller(amount.clone(), shape.target_tokens)
    {
        return Ok(Some(effect));
    }
    if let Some(target) = parse_combat_player_damage_target(shape.target_tokens, true) {
        return Ok(Some(combat_player_damage_target_effect(
            amount.clone(),
            target,
        )));
    }
    if shape.target_is_each_or_all {
        if shape.target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter = parse_damage_each_filter(&shape.target_tokens[1..])?;
        return Ok(Some(EffectAst::subject_verb_damage_each(amount, filter)));
    }
    let target = preserve_optional_single_damage_target(
        parse_target_phrase(shape.target_tokens)?,
        shape.target_tokens,
    );
    Ok(Some(EffectAst::subject_verb_damage(amount, target)))
}
fn parse_divided_damage_target(
    target_tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let clause = crate::token_word_refs(target_tokens).join(" ");
    let shape = combat_grammar::parse_combat_divided_target_shape_lexed(target_tokens).map_err(
        |error| {
            let message = match error {
                combat_grammar::CombatDividedTargetError::MissingTargetsAfterAmong => {
                    format!("missing divided-damage targets after 'among' (clause: '{clause}')")
                }
                combat_grammar::CombatDividedTargetError::MissingTargetPhrase => {
                    format!("missing divided-damage target phrase (clause: '{clause}')")
                }
                combat_grammar::CombatDividedTargetError::UnsupportedTargetCount => {
                    format!("unsupported divided-damage target count (clause: '{clause}')")
                }
                combat_grammar::CombatDividedTargetError::MissingTargetCount => {
                    format!("missing divided-damage target count (clause: '{clause}')")
                }
            };
            CardTextError::ParseError(message)
        },
    )?;
    let base_target = if shape.any_target {
        TargetAst::AnyTarget(span_from_tokens(shape.target_tokens))
    } else if shape
        .target_tokens
        .first()
        .is_some_and(|token| token.as_word() == Some("them"))
    {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(shape.target_tokens))
    } else if shape
        .target_tokens
        .first()
        .is_some_and(|token| token.as_word() == Some("those"))
    {
        let mut filter = parse_object_filter(&shape.target_tokens[1..], false)?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        TargetAst::Object(filter, None, span_from_tokens(shape.target_tokens))
    } else {
        parse_target_phrase(shape.target_tokens)?
    };
    Ok(TargetAst::WithCount(Box::new(base_target), shape.count))
}
fn parse_divided_damage_with_amount(
    tokens: &[OwnedLexToken],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let shape =
        combat_grammar::parse_combat_divided_amount_shape_lexed(tokens, used).map_err(|_| {
            CardTextError::ParseError(format!(
                "missing damage keyword in divided-damage clause (clause: '{}')",
                crate::token_word_refs(tokens).join(" ")
            ))
        })?;
    match shape {
        combat_grammar::CombatDividedAmountShape::EvenlyEach { filter_tokens } => {
            let filter = parse_damage_each_filter(filter_tokens)?;
            Ok(EffectAst::subject_verb_damage_each(amount, filter))
        }
        combat_grammar::CombatDividedAmountShape::Distributed {
            target_tokens,
            evenly_rounded_down,
        } => {
            let target = parse_divided_damage_target(target_tokens)?;
            if evenly_rounded_down {
                Ok(EffectAst::subject_verb_evenly_distributed_damage(
                    amount, target,
                ))
            } else {
                Ok(EffectAst::subject_verb_distributed_damage(amount, target))
            }
        }
    }
}
pub(crate) fn parse_deal_damage_with_amount(
    tokens: &[OwnedLexToken],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let clause = crate::token_word_refs(tokens).join(" ");
    let shape =
        combat_grammar::parse_combat_damage_target_shape_lexed(tokens, used).map_err(|error| {
            let message = match error {
                combat_grammar::CombatDamageTargetShapeError::MissingDamageKeyword => {
                    "missing damage keyword".to_string()
                }
                combat_grammar::CombatDamageTargetShapeError::UnsupportedTrailingIfClause
                | combat_grammar::CombatDamageTargetShapeError::UnsupportedEmbeddedIfClause => {
                    format!("unsupported trailing if clause in damage effect (clause: '{clause}')")
                }
                combat_grammar::CombatDamageTargetShapeError::MissingEachFilter => {
                    "missing damage target filter after 'each'".to_string()
                }
            };
            CardTextError::ParseError(message)
        })?;

    match shape {
        combat_grammar::CombatDamageTargetShape::InsteadIf {
            target_tokens,
            predicate_tokens,
            instead_tail_tokens,
        } => {
            let predicate = if let Some(predicate) =
                parse_instead_if_control_predicate(predicate_tokens)?
            {
                predicate
            } else {
                parse_trailing_instead_if_predicate_lexed(instead_tail_tokens).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported trailing instead-if clause in damage effect (clause: '{clause}')"
                    ))
                })?
            };
            let target = if target_tokens.is_empty() {
                TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
            } else {
                parse_target_phrase(target_tokens)?
            };
            Ok(EffectAst::TrailingIf {
                predicate,
                effects: vec![EffectAst::subject_verb_damage(amount, target)],
            })
        }
        combat_grammar::CombatDamageTargetShape::TrailingIf {
            target_tokens,
            predicate,
        } => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(EffectAst::Conditional {
                predicate,
                if_true: vec![EffectAst::subject_verb_damage(amount, target)],
                if_false: Vec::new(),
            })
        }
        combat_grammar::CombatDamageTargetShape::TrailingUnless {
            target_tokens,
            predicate,
        } => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(EffectAst::TrailingUnless {
                predicate,
                effects: vec![EffectAst::subject_verb_damage(amount, target)],
            })
        }
        combat_grammar::CombatDamageTargetShape::OmittedTargetIf { predicate } => {
            Ok(EffectAst::TrailingIf {
                predicate,
                effects: vec![EffectAst::subject_verb_damage(
                    amount,
                    TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None),
                )],
            })
        }
        combat_grammar::CombatDamageTargetShape::Simple {
            shape,
            target_tokens,
        } => {
            let amount = if shape == combat_grammar::CombatSimpleDamageTargetShape::IteratedPlayer
                && crate::token_word_refs(target_tokens) == ["them"]
            {
                amount.with_surface_hint(ironsmith_core::ValueSurfaceHint::DamageRecipientPronoun)
            } else {
                amount
            };
            Ok(EffectAst::subject_verb_damage(
                amount,
                combat_simple_damage_target_ast(shape, target_tokens),
            ))
        }
        combat_grammar::CombatDamageTargetShape::EachOfCount { count, span_tokens } => {
            let target = TargetAst::WithCount(
                Box::new(TargetAst::AnyTarget(span_from_tokens(span_tokens))),
                count,
            );
            Ok(EffectAst::subject_verb_damage(amount, target))
        }
        combat_grammar::CombatDamageTargetShape::EachOfTarget { target_tokens } => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(EffectAst::subject_verb_damage(amount, target))
        }
        combat_grammar::CombatDamageTargetShape::PlayerGroup(target) => {
            Ok(combat_player_damage_target_effect(amount, target))
        }
        combat_grammar::CombatDamageTargetShape::MaxSpeedPlayers { has_max_speed } => {
            let filter = if has_max_speed {
                PlayerFilter::with_max_speed(PlayerFilter::Any)
            } else {
                PlayerFilter::without_max_speed(PlayerFilter::Any)
            };
            Ok(EffectAst::ForEachPlayersFiltered {
                filter,
                effects: vec![EffectAst::subject_verb_damage(
                    amount,
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            })
        }
        combat_grammar::CombatDamageTargetShape::OpponentWho { predicate_tokens } => {
            let predicate = parse_who_did_this_way_predicate(predicate_tokens)?;
            Ok(EffectAst::ForEachOpponentDid {
                effects: vec![EffectAst::subject_verb_damage(
                    amount,
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
                predicate,
                result_predicate: IfResultPredicate::Did,
            })
        }
        combat_grammar::CombatDamageTargetShape::PlayerWho { predicate_tokens } => {
            let predicate = parse_who_did_this_way_predicate(predicate_tokens)?;
            Ok(EffectAst::ForEachPlayerDid {
                effects: vec![EffectAst::subject_verb_damage(
                    amount,
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
                predicate,
                result_predicate: IfResultPredicate::Did,
            })
        }
        combat_grammar::CombatDamageTargetShape::PlayerAndObjects {
            player_filter,
            player_span,
            filter_tokens,
        } => {
            let mut filter = parse_object_filter(filter_tokens, false)?;
            if filter.controller.is_none() {
                filter.controller = Some(player_filter.clone());
            }
            Ok(EffectAst::Sequence {
                effects: vec![
                    EffectAst::subject_verb_damage(
                        amount.clone(),
                        TargetAst::Player(player_filter, player_span),
                    ),
                    EffectAst::subject_verb_damage_each(amount, filter),
                ],
            })
        }
        combat_grammar::CombatDamageTargetShape::EachObjectsAndPlayer { filter_tokens } => {
            let mut filter = parse_object_filter(filter_tokens, false)?;
            if filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::IteratedPlayer);
            }
            Ok(EffectAst::ForEachPlayer {
                effects: vec![
                    EffectAst::subject_verb_damage(
                        amount.clone(),
                        TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                    ),
                    EffectAst::subject_verb_damage_each(amount, filter),
                ],
            })
        }
        combat_grammar::CombatDamageTargetShape::OpponentAndControlledCreaturePlaneswalker => {
            let mut filter = ObjectFilter::default();
            filter.card_types = vec![CardType::Creature, CardType::Planeswalker];
            filter.controller = Some(PlayerFilter::IteratedPlayer);
            Ok(EffectAst::ForEachOpponent {
                effects: vec![
                    EffectAst::subject_verb_damage(
                        amount.clone(),
                        TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                    ),
                    EffectAst::subject_verb_damage_each(amount, filter),
                ],
            })
        }
        combat_grammar::CombatDamageTargetShape::HistoricalDamageRecipients {
            players,
            filter_tokens,
        } => {
            let player_filter = PlayerFilter::was_dealt_damage_by_source_this_game(
                combat_player_damage_target_filter(players),
            );
            let mut object_filter = parse_object_filter(filter_tokens, false)?;
            object_filter.was_dealt_damage_by_source_this_game = true;
            Ok(EffectAst::Sequence {
                effects: vec![
                    EffectAst::ForEachPlayersFiltered {
                        filter: player_filter,
                        effects: vec![EffectAst::subject_verb_damage(
                            amount.clone(),
                            TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                        )],
                    },
                    EffectAst::subject_verb_damage_each(amount, object_filter),
                ],
            })
        }
        combat_grammar::CombatDamageTargetShape::EachFilter { filter_tokens } => {
            let filter = parse_damage_each_filter(filter_tokens)?;
            Ok(EffectAst::subject_verb_damage_each(amount, filter))
        }
        combat_grammar::CombatDamageTargetShape::DelayedEndOfCombat { target_tokens } => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(EffectAst::DelayedUntilEndOfCombat {
                effects: vec![EffectAst::subject_verb_damage(amount, target)],
            })
        }
        combat_grammar::CombatDamageTargetShape::General { target_tokens } => {
            let target = parse_target_phrase(target_tokens)?;
            Ok(EffectAst::subject_verb_damage(amount, target))
        }
    }
}
pub(crate) fn parse_instead_if_control_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(shape) = combat_grammar::parse_combat_control_predicate_shape_lexed(tokens) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter(shape.filter_tokens, shape.other)?;
    if let Some(relation) = shape.power_toughness_relation {
        filter.power_toughness_relation = Some(relation);
    }
    if let Some(count) = shape.min_count {
        if shape.requires_different_powers {
            return Ok(Some(PredicateAst::PlayerHasAtLeastWithDifferentPowers {
                player: PlayerAst::You,
                filter,
                count,
            }));
        }
        Ok(Some(PredicateAst::PlayerHasAtLeast {
            player: PlayerAst::You,
            filter,
            count,
        }))
    } else {
        Ok(Some(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        }))
    }
}

#[cfg(test)]
mod equal_to_damage_surface_tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn historical_mixed_damage_recipients_keep_both_typed_filters() {
        let tokens = lex_line(
            "1 damage to each opponent and planeswalker it has dealt damage to this game",
            0,
        )
        .expect("historical damage clause should lex");
        let effect = parse_deal_damage(&tokens).expect("historical damage clause should parse");
        let debug = format!("{effect:#?}");
        assert!(
            debug.contains("WasDealtDamageBySourceThisGame"),
            "player history filter missing: {debug}"
        );
        assert!(
            debug.contains("was_dealt_damage_by_source_this_game: true"),
            "object history filter missing: {debug}"
        );
        assert!(
            debug.contains("Planeswalker"),
            "object domain missing: {debug}"
        );
        assert_eq!(debug.matches("DealDamage").count(), 2, "{debug}");
    }

    #[test]
    fn damage_to_each_of_those_preserves_the_demonstrative_set_surface() {
        let tokens = lex_line("X damage to each of those creatures", 0)
            .expect("demonstrative damage clause should lex");
        let effect = parse_deal_damage(&tokens).expect("demonstrative damage clause should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamageEach { filter, .. },
            ..
        }) = effect
        else {
            panic!("expected typed damage fanout: {effect:#?}");
        };

        assert_eq!(
            filter.set_quantifier_surface(),
            Some(ironsmith_core::SetQuantifierSurface::Those)
        );
    }

    #[test]
    fn fixed_plus_count_damage_keeps_equal_to_surface() {
        let tokens = lex_line(
            "damage equal to 2 plus the number of Lesson cards in your graveyard to target creature",
            0,
        )
        .expect("equal-to damage should lex");
        let effect = parse_deal_damage_equal_to_clause(&tokens)
            .expect("equal-to damage should parse")
            .expect("equal-to damage should match");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamage { amount, .. },
            ..
        }) = effect
        else {
            panic!("expected typed damage effect");
        };

        assert!(amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo));
        assert!(matches!(amount.unhinted(), Value::Add(_, _)));
    }

    #[test]
    fn equal_to_damage_keeps_authored_optional_single_target() {
        for (target_words, expected_count) in [
            (
                "up to one target creature or planeswalker",
                ChoiceCount::up_to(1),
            ),
            ("target creature or planeswalker", ChoiceCount::exactly(1)),
        ] {
            let tokens = lex_line(
                &format!("damage equal to that card's mana value to {target_words}"),
                0,
            )
            .expect("linked damage clause should lex");
            let effect = parse_deal_damage_equal_to_clause(&tokens)
                .expect("linked damage clause should parse")
                .expect("linked damage clause should match");
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::DealDamage { amount, target, .. },
                ..
            }) = effect
            else {
                panic!("expected typed damage effect");
            };
            let actual_count = match target {
                TargetAst::WithCount(_, count) => count,
                _ => ChoiceCount::exactly(1),
            };

            assert_eq!(actual_count, expected_count, "{target_words}");
            assert!(amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo));
            assert!(matches!(amount.unhinted(), Value::ManaValueOf(_)));
        }
    }

    #[test]
    fn relative_controller_count_preempts_the_permissive_filter_value() {
        let tokens = lex_line(
            "damage to target creature equal to the number of nonbasic lands that creature's controller controls",
            0,
        )
        .expect("relative-controller damage should lex");
        let effect = parse_deal_damage(&tokens).expect("relative-controller damage should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::DealDamage {
                    amount,
                    target: TargetAst::Object(target, Some(_), _),
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected typed targeted damage: {effect:#?}");
        };
        let Value::Count(counted) = amount.unhinted() else {
            panic!("expected typed land count: {amount:#?}");
        };

        assert_eq!(target.card_types, vec![crate::CardType::Creature]);
        assert_eq!(counted.card_types, vec![crate::CardType::Land]);
        assert!(!counted.card_types.contains(&crate::CardType::Creature));
        assert!(
            counted
                .excluded_supertypes
                .contains(&crate::Supertype::Basic)
        );
        assert_eq!(
            counted.controller,
            Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target))
        );
    }

    #[test]
    fn authored_damage_recipient_pronoun_is_preserved_on_the_amount() {
        for (text, expects_hint) in [
            ("2 damage to them", true),
            ("2 damage to that player", false),
        ] {
            let tokens = lex_line(text, 0).expect("damage clause should lex");
            let effect = parse_deal_damage(&tokens).expect("damage clause should parse");
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::DealDamage {
                        amount,
                        target: TargetAst::Player(PlayerFilter::IteratedPlayer, _),
                        ..
                    },
                ..
            }) = effect
            else {
                panic!("expected iterated-player damage for {text}: {effect:#?}");
            };

            assert_eq!(
                amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::DamageRecipientPronoun),
                expects_hint,
                "{text}"
            );
        }
    }

    #[test]
    fn target_spell_controller_damage_materializes_the_spell_target_first() {
        for text in [
            "damage to target spell's controller equal to that spell's mana value",
            "damage equal to that spell's mana value to target spell's controller",
        ] {
            let tokens = lex_line(text, 0).expect("stack-target damage should lex");
            let effect = if text.starts_with("damage to") {
                parse_deal_damage_to_target_equal_to_clause(&tokens)
            } else {
                parse_deal_damage_equal_to_clause(&tokens)
            }
            .expect("stack-target damage should parse")
            .expect("stack-target damage should match");
            let EffectAst::Sequence { effects } = effect else {
                panic!("expected target prelude plus damage for {text}: {effect:#?}");
            };
            let [target, damage] = effects.as_slice() else {
                panic!("expected exactly two typed effects for {text}: {effects:#?}");
            };
            assert!(matches!(
                target,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::TargetOnly {
                        target: TargetAst::Spell(Some(_)),
                        explicit_declaration: false,
                    },
                    ..
                })
            ));
            assert!(matches!(
                damage,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::DealDamage {
                            amount,
                            target:
                                TargetAst::Player(
                                    PlayerFilter::ControllerOf(
                                        crate::target::ObjectRef::Tagged(tag)
                                    ),
                                    None
                                ),
                            ..
                        },
                    ..
                }) if tag.as_str() == IT_TAG
                    && matches!(
                        amount.unhinted(),
                        Value::ManaValueOf(spec)
                            if matches!(
                                spec.unhinted(),
                                ChooseSpec::Tagged(value_tag) if value_tag.as_str() == IT_TAG
                            )
                    )
            ));
        }
    }
}

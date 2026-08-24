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

pub fn parse_attach_object_phrase(
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
pub fn parse_attach(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let clause = crate::lexer::token_word_refs(tokens).join(" ");
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
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.key(), span_from_tokens(object_tokens)),
                    TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(target_tokens)),
                ));
            }
            if let Some(host_tokens) =
                grammar::match_word_prefix(object_tokens, &["all", "auras", "enchanting"])
            {
                let destination_words = crate::lexer::token_word_refs(target_tokens);
                if crate::word_primitives::parse_any_sequence_complete(
                    &destination_words,
                    &[
                        &["another", "permanent", "with", "same", "controller"],
                        &[
                            "another",
                            "permanent",
                            "with",
                            "the",
                            "same",
                            "controller",
                        ],
                    ],
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
                crate::effect_sentences::zone_counter_helpers::target_object_filter_mut(
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

pub fn parse_unattach(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
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

pub fn damage_clause_has_terminal_unpreventable_rider(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::token_word_refs(tokens);
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

fn strip_terminal_unpreventable_damage_rider(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    for (index, token) in tokens.iter().enumerate() {
        if !token.is_word("and") {
            continue;
        }
        let words = crate::lexer::token_word_refs(&tokens[index..]);
        if crate::word_primitives::parse_choice_sequence_complete(
            &words,
            &[
                &["and"],
                &["the", "that"],
                &["damage"],
                &["cant", "can't"],
                &["be"],
                &["prevented"],
            ],
        ) {
            return crate::util::trim_edge_punctuation_tokens(&tokens[..index]);
        }
    }
    tokens
}

pub fn mark_damage_ast_unpreventable(effect: &mut EffectAst) {
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

pub fn parse_deal_damage(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let has_unpreventable_rider = damage_clause_has_terminal_unpreventable_rider(tokens);
    let parse_tokens = if has_unpreventable_rider {
        strip_terminal_unpreventable_damage_rider(tokens)
    } else {
        tokens
    };
    let mut effect = parse_deal_damage_inner(parse_tokens)?;
    if has_unpreventable_rider {
        mark_damage_ast_unpreventable(&mut effect);
    }
    Ok(effect)
}

fn parse_damage_each_filter(
    filter_tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let mut filter = parse_object_filter(filter_tokens, false)?;
    let words = crate::lexer::token_word_refs(filter_tokens);
    if words.first() == Some(&"those")
        || crate::word_primitives::parse_sequence_prefix(&words, &["of", "those"])
    {
        filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those));
    }
    Ok(filter)
}

fn parse_deal_damage_inner(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let shape = combat_grammar::parse_combat_damage_head_shape_lexed(tokens);
    let tokens = shape.body_tokens;
    let clause_words = crate::lexer::token_word_refs(tokens);
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
    let words = crate::lexer::token_word_refs(tokens);
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
    let chooser = if crate::word_primitives::sequence_occurs(
        &crate::lexer::token_word_refs(shape.target_tokens),
        &["as", "its", "controller", "chooses"],
    )
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
    let words = crate::lexer::token_word_refs(target_tokens);
    if !crate::word_primitives::parse_sequence_prefix(&words, &["up", "to", "one", "target"])
    {
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

pub fn parse_deal_damage_to_target_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = combat_grammar::parse_combat_damage_to_target_equal_shape_lexed(tokens)
    else {
        return Ok(None);
    };
    let clause_words = crate::lexer::token_word_refs(tokens);
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

#[cfg(test)]
#[path = "combat_verbs_inline_equal_to_damage_surface_tests.rs"]
mod equal_to_damage_surface_tests;

#[path = "combat_verbs/combat_verbs_object_action_programs.rs"]
mod combat_verbs_object_action_programs;
pub use combat_verbs_object_action_programs::{parse_instead_if_control_predicate};
#[path = "combat_verbs/combat_verbs_combat_programs.rs"]
mod combat_verbs_combat_programs;
pub use combat_verbs_combat_programs::{parse_deal_damage_equal_to_clause, parse_deal_damage_with_amount};
use combat_verbs_combat_programs::{parse_divided_damage_target, parse_divided_damage_with_amount};

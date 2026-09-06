use crate::cards::builders::DamageActionAst;
use super::super::grammar::effects::combat_shapes as combat_grammar;
use crate::recognition::{ParseOutcome, RuleId};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

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
        tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
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
        combat_grammar::CombatPlayerDamageTargetShape::EachPlayer => EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_damage(
                amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        }),
        combat_grammar::CombatPlayerDamageTargetShape::EachOtherPlayer => {
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::NotYou,
                effects: vec![EffectAst::subject_verb_damage(
                    amount,
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            })
        }
        combat_grammar::CombatPlayerDamageTargetShape::EachOpponent => EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        }),
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
            PlayerFilter::ControllerOf(crate::target::ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
            )),
            span_from_tokens(tokens),
        ),
        combat_grammar::CombatSimpleDamageTargetShape::IteratedPlayer => {
            TargetAst::Player(PlayerFilter::IteratedPlayer, span_from_tokens(tokens))
        }
    }
}

fn damage_each_other_opponent(amount: Value) -> EffectAst {
    EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
        filter: PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::DamagedPlayer),
        effects: vec![EffectAst::subject_verb_damage(
            amount,
            TargetAst::Player(PlayerFilter::IteratedPlayer, None),
        )],
    })
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
        PlayerFilter::ControllerOf(crate::target::ObjectRef::tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
        )),
        None,
    );
    Some(EffectAst::Sequence {
        effects: vec![
            EffectAst::subject_verb_target_only(anchor),
            EffectAst::subject_verb_damage(amount, recipient),
        ],
    })
}

pub fn parse_attach_object_phrase(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
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
                Ok(TargetAst::Tagged(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    object_span,
                ))
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
            let target = TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                span_from_tokens(tagged_tokens),
            );
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
                    TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::Triggering.bind(),
                        span_from_tokens(object_tokens),
                    ),
                    TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        span_from_tokens(target_tokens),
                    ),
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
                        &["another", "permanent", "with", "the", "same", "controller"],
                    ],
                ) {
                    let host = parse_target_phrase(host_tokens)?;
                    let mut aura_filter = ObjectFilter::permanent().in_zone(Zone::Battlefield);
                    aura_filter.subtypes.push(Subtype::Aura);
                    aura_filter.tagged_constraints.push(TaggedObjectConstraint {
                        tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
                        relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                    });

                    let mut destination = ObjectFilter::permanent().in_zone(Zone::Battlefield);
                    for relation in [
                        TaggedOpbjectRelation::SameControllerAsTagged,
                        TaggedOpbjectRelation::IsNotTaggedObject,
                    ] {
                        destination.tagged_constraints.push(TaggedObjectConstraint {
                            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
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
                TargetAst::Tagged(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    span_from_tokens(target_tokens),
                )
            } else {
                parse_target_phrase(target_tokens)?
            };
            if crate::grammar::filters::reference_tag_stage::has_plural_object_head_surface(
                target_tokens,
            ) && let Some(filter) =
                crate::effect_sentences::zone_counter_helpers::target_object_filter_mut(&mut target)
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
        combat_grammar::AttachedObjectReferenceTag::Enchanted => {
            crate::tag::CompilerReferenceTag::Enchanted.bind()
        }
        combat_grammar::AttachedObjectReferenceTag::Equipped => {
            crate::tag::CompilerReferenceTag::Equipped.bind()
        }
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
    .match_tagged(tag, TaggedOpbjectRelation::IsTaggedObject);
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
                tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
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
    crate::grammar::effects::clause_dispatch_shapes::split_terminal_unpreventable_damage_rider(
        tokens,
    )
    .is_some()
}

fn strip_terminal_unpreventable_damage_rider(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    crate::grammar::effects::clause_dispatch_shapes::split_terminal_unpreventable_damage_rider(
        tokens,
    )
    .unwrap_or(tokens)
}

pub fn mark_damage_ast_unpreventable(effect: &mut EffectAst) {
    if let EffectAst::SubjectVerb(subject_verb) = effect {
        match &mut subject_verb.action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { unpreventable, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { unpreventable, .. }) => {
                *unpreventable = true;
            }
            _ => {}
        }
    }
    crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
        for nested_effect in nested {
            mark_damage_ast_unpreventable(nested_effect);
        }
    });
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

pub fn is_historical_player_object_damage_recipient_clause(tokens: &[OwnedLexToken]) -> bool {
    let shape = combat_grammar::parse_combat_damage_head_shape_lexed(tokens);
    let Some((_, amount_used)) = parse_value(shape.body_tokens) else {
        return false;
    };
    matches!(
        combat_grammar::parse_combat_damage_target_shape_lexed(shape.body_tokens, amount_used),
        Ok(combat_grammar::CombatDamageTargetShape::HistoricalDamageRecipients { .. })
    )
}

fn parse_damage_each_filter(
    filter_tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    if let Some(shape) = combat_grammar::parse_combat_except_filter_shape_lexed(filter_tokens) {
        let mut included = parse_object_filter(shape.included_filter_tokens, false)?;
        let excluded = parse_object_filter(shape.excluded_filter_tokens, false)?;
        if excluded.controller == Some(PlayerFilter::You)
            && excluded.static_abilities.len() == 1
            && excluded.excluded_static_abilities.is_empty()
        {
            let mut excluded_basis = excluded.clone();
            excluded_basis.controller = None;
            excluded_basis.static_abilities.clear();
            excluded_basis.union_surface = included.union_surface.clone();
            if excluded_basis == included {
                included.any_of = vec![
                    ObjectFilter::default().controlled_by(PlayerFilter::NotYou),
                    ObjectFilter::default().without_static_ability(excluded.static_abilities[0]),
                ];
                return Ok(included);
            }
        }
    }
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
        return Ok(EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                Value::CardsInHand(PlayerFilter::IteratedPlayer),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        }));
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
        return Ok(EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                value,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        }));
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
    ) {
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

/// `equal to <n> plus the number of <spells cast ...> this turn` sums a fixed
/// base with a typed turn-history count.
fn parse_fixed_plus_turn_history_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let equal_idx = (0..tokens.len().saturating_sub(3)).find(|&idx| {
        tokens[idx].is_word("equal") && tokens[idx + 1].is_word("to")
    })?;
    let base_token = tokens.get(equal_idx + 2)?;
    let base_word = base_token.parser_word_pieces().first()?.text.as_str();
    let base = crate::util::parse_number_word_u32(base_word)?;
    if !tokens.get(equal_idx + 3)?.is_word("plus") {
        return None;
    }
    let tail = crate::lexer::trim_lexed_commas(tokens.get(equal_idx + 4..)?);
    let history =
        crate::grammar::shared_util::value_semantics::parse_turn_history_count_value(tail)?;
    Some(Value::Add(
        Box::new(Value::Fixed(base as i32)),
        Box::new(history),
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
    if !crate::word_primitives::parse_sequence_prefix(&words, &["up", "to", "one", "target"]) {
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
    let amount_tokens = shape.amount_clause_tokens;
    let clause_words = crate::lexer::token_word_refs(tokens);
    let mut candidates = Vec::new();
    let span = span_from_tokens(tokens);
    let mut add_candidate = |id: &'static str, value: Option<Value>| {
        let Some(value) = value else { return };
        // Independently written grammars can prove the same typed amount.
        // Semantic identity ignores presentation-only surface hints: equal
        // unhinted values are one semantic candidate (the first authored
        // surface wins); unequal values remain an explicit ambiguity
        // diagnosed by the registry resolver.
        if candidates
            .iter()
            .any(|candidate: &RegistryCandidate<Value>| {
                candidate.value.unhinted() == value.unhinted()
            })
        {
            return;
        }
        candidates.push(RegistryCandidate::new(
            RegistryRuleMetadata::distinct(RuleId::new(id), HeadDiscriminator::grammar(id)),
            value,
            span,
        ));
    };
    add_candidate(
        "damage-amount-relative-aggregate",
        parse_equal_to_aggregate_filter_value(amount_tokens),
    );
    let object_count = parse_equal_to_number_of_filter_value(amount_tokens);
    add_candidate("damage-amount-object-count", object_count.clone());
    add_candidate(
        "damage-amount-devotion",
        parse_devotion_value_from_add_clause(amount_tokens)?,
    );
    add_candidate(
        "damage-amount-event-result",
        shape
            .amount_is_event_result
            .then_some(Value::EventValue(EventValueSpec::Amount)),
    );
    let fixed_plus_history = parse_fixed_plus_turn_history_value(amount_tokens);
    // The plain `equal to the number of <filter>` shape and the summed
    // `equal to <n> plus <history>` shape each own their complete amount
    // phrase; the dynamic cost-modifier grammar recovers fragments of the
    // same words (a bare history count, a re-derived filter count bound to
    // a nearby reference) and therefore covers only what those shapes
    // cannot prove.
    if fixed_plus_history.is_none() && object_count.is_none() {
        add_candidate(
            "damage-amount-dynamic-cost-modifier",
            parse_dynamic_cost_modifier_value(amount_tokens)?,
        );
    }
    add_candidate("damage-amount-fixed-plus-turn-history", fixed_plus_history);

    let specific_amount = match resolve_registry_candidates(
        RuleId::new("damage-equal-amount-registry"),
        candidates,
        Vec::new(),
    ) {
        ParseOutcome::Match(matched) => Some(matched.value.value),
        ParseOutcome::NoMatch => None,
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    };
    // The generic equal-to value grammar is an explicit fallback phase. It
    // cannot compete with a relationship-, aggregate-, event-, devotion-, or
    // cost-specific amount proven by the registry above.
    let amount = specific_amount
        .or_else(|| parse_add_mana_equal_amount_value(amount_tokens))
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

#[path = "combat_verbs/combat_verbs_object_action.rs"]
mod combat_verbs_object_action_programs;
pub use combat_verbs_object_action_programs::parse_instead_if_control_predicate;
#[path = "combat_verbs/combat_verbs_combat.rs"]
mod combat_verbs_combat_programs;
pub use combat_verbs_combat_programs::{
    parse_deal_damage_equal_to_clause, parse_deal_damage_with_amount,
};
use combat_verbs_combat_programs::{parse_divided_damage_target, parse_divided_damage_with_amount};

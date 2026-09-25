use super::*;
use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::ObjectChoiceEffectAst;
use crate::cards::builders::TurnEventPredicateAst;
use winnow::Parser;

pub fn parse_sacrifice(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    target: Option<TargetAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let opponent_chooses_object =
        crate::grammar::choices::parse_possessive_object_choice_tokens(tokens).is_some_and(
            |shape| shape.actor == crate::grammar::choices::PossessiveObjectChoiceActor::Opponent,
        );
    let clause_shape = sacrifice_discard_grammar::parse_sacrifice_clause_shape(tokens);
    let choice_body =
        crate::grammar::choices::parse_possessive_object_choice_tokens(clause_shape.body_tokens);
    let tokens = crate::util::trim_edge_punctuation_tokens(
        choice_body
            .as_ref()
            .map(|choice| choice.object_tokens.as_slice())
            .unwrap_or(clause_shape.body_tokens),
    );
    let normalized_words = crate::lexer::token_word_refs(tokens);
    let unless_escaped = matches!(
        clause_shape.unless_kind,
        sacrifice_discard_grammar::SacrificeUnlessKind::Escaped
    );
    if !matches!(
        clause_shape.unless_kind,
        sacrifice_discard_grammar::SacrificeUnlessKind::None
            | sacrifice_discard_grammar::SacrificeUnlessKind::Escaped
    ) {
        let sacrifice_tokens = trim_commas(clause_shape.body_tokens);
        let base = parse_sacrifice(&sacrifice_tokens, subject, target.clone())?;
        match clause_shape.unless_kind {
            sacrifice_discard_grammar::SacrificeUnlessKind::ManaSpent(symbol) => {
                return Ok(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate: PredicateAst::ManaSpentToCastThisSpellAtLeast {
                        amount: 1,
                        symbol: Some(symbol),
                    },
                    if_true: Vec::new(),
                    if_false: vec![base],
                }));
            }
            sacrifice_discard_grammar::SacrificeUnlessKind::OpponentDamagedThisTurn => {
                return Ok(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate: PredicateAst::TurnEvents(
                        TurnEventPredicateAst::OpponentWasDealtDamageThisTurn,
                    ),
                    if_true: Vec::new(),
                    if_false: vec![base],
                }));
            }
            sacrifice_discard_grammar::SacrificeUnlessKind::General => {
                let Some(unless_token_offset) = clause_shape.unless_token_offset else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported sacrifice-unless clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                };
                if let Some(mut unless_effect) = try_build_unless(
                    vec![base],
                    SubjectVerbPrimitiveClause::new(clause_shape.full_body_tokens),
                    unless_token_offset,
                )? {
                    if clause_shape.sacrifice_references_it {
                        rewrite_unless_cost_source_values_to_it_tag(&mut unless_effect);
                    }
                    return Ok(unless_effect);
                }
            }
            _ => unreachable!(),
        }
        if matches!(
            clause_shape.unless_kind,
            sacrifice_discard_grammar::SacrificeUnlessKind::General
        ) {
            return Err(CardTextError::ParseError(format!(
                "unsupported sacrifice-unless clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }
    if clause_shape.has_graveyard_history {
        return Err(CardTextError::ParseError(format!(
            "unsupported graveyard-history sacrifice clause (clause: '{}')",
            normalized_words.join(" ")
        )));
    }

    // Preserve an omitted actor until surrounding optional/player clauses
    // have bound it. Lowering supplies You when no outer actor exists.
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    // "Sacrifice a creature, an artifact, and a land": each listed object is
    // its own choice of a different permanent, and all of them are
    // sacrificed at the same time (CR 701.21a).
    if let Some(effect) = parse_sacrifice_object_list(tokens, player, opponent_chooses_object)? {
        return Ok(wrap_unless_escaped(effect, unless_escaped));
    }

    // A definite singular choice reference identifies the previously chosen
    // object; it does not ask the player to make a new sacrifice choice.
    if let Some(chosen) = crate::grammar::targets::parse_chosen_object_target(tokens) {
        let noun = crate::lexer::token_word_refs(chosen.filter_tokens);
        if matches!(noun.as_slice(), ["creature" | "artifact" | "enchantment" | "land" | "planeswalker" | "battle" | "permanent" | "object"])
        {
            let target = parse_target_phrase(tokens)?;
            return Ok(wrap_unless_escaped(
                EffectAst::subject_verb_sacrifice(player, ObjectFilter::default(), 1, Some(target)),
                unless_escaped,
            ));
        }
    }

    if let Some(fraction) =
        sacrifice_discard_grammar::parse_sacrifice_fraction_rounded_shape(tokens)
    {
        let mut filter = parse_object_filter_lexed(fraction.filter_tokens, false)?;
        filter.zone = Some(Zone::Battlefield);
        let basis = Value::Count(filter.clone());
        let count_value = if fraction.denominator == 2 {
            let basis = if fraction.rounded_up {
                Value::Add(Box::new(basis), Box::new(Value::Fixed(1)))
            } else {
                basis
            };
            Value::HalfRoundedDown(Box::new(basis))
        } else {
            let denominator = i32::try_from(fraction.denominator).map_err(|_| {
                CardTextError::ParseError(format!(
                    "sacrifice fraction denominator is too large (clause: '{}')",
                    normalized_words.join(" ")
                ))
            })?;
            let basis = if fraction.rounded_up {
                Value::Add(
                    Box::new(basis),
                    Box::new(Value::Fixed(denominator.saturating_sub(1))),
                )
            } else {
                basis
            };
            Value::DividedRoundedDown(Box::new(basis), denominator)
        };
        let tag = crate::util::helper_tag_for_tokens(tokens, "sacrificed");
        return Ok(wrap_unless_escaped(
            EffectAst::Sequence {
                effects: vec![
                    EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                        filter,
                        count: crate::effect::ChoiceCount::dynamic_x(),
                        count_value: Some(count_value),
                        player,
                        tag: crate::tag::TagRef::of(tag.clone()),
                    }),
                    EffectAst::subject_verb_sacrifice_all(
                        PlayerAst::That,
                        ObjectFilter::tagged(tag),
                    ),
                ],
            },
            unless_escaped,
        ));
    }

    if let Some((choice_count, used)) =
        crate::util::parse_choice_count_token_prefix_consumed(tokens)
        && !choice_count.is_single()
    {
        let choice_body = &tokens[used..];
        let comma_then =
            crate::grammar::primitives::split_lexed_once_on_separator(choice_body, || {
                (
                    crate::grammar::primitives::comma(),
                    crate::grammar::primitives::kw("then"),
                )
                    .void()
            });
        let (filter_tokens, followup_tokens) =
            comma_then.map_or((choice_body, None), |(filter_tokens, followup_tokens)| {
                (
                    crate::util::trim_edge_punctuation_tokens(filter_tokens),
                    Some(crate::util::trim_edge_punctuation_tokens(followup_tokens)),
                )
            });
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing sacrifice object after choice count (clause: '{}')",
                normalized_words.join(" ")
            )));
        }
        let filter = parse_object_filter_lexed(filter_tokens, false)?;
        let tag = crate::util::helper_tag_for_tokens(tokens, "sacrificed");
        // Fixed counts use the sacrifice effect's own controlled-permanent
        // selection; this keeps the chooser and sacrificing actor identical.
        let mut effects = if !choice_count.dynamic_x
            && !choice_count.random
            && choice_count.max == Some(choice_count.min)
            && !opponent_chooses_object
        {
            let count = u32::try_from(choice_count.min).map_err(|_| {
                CardTextError::ParseError("sacrifice count exceeds the supported range".into())
            })?;
            vec![EffectAst::subject_verb_sacrifice(
                player, filter, count, None,
            )]
        } else {
            vec![
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter,
                    count: choice_count,
                    count_value: None,
                    player,
                    tag: crate::tag::TagRef::of(tag.clone()),
                }),
                // The sacrifice is made by whoever just chose the objects.
                // `That` re-reads the last bound player, and an implicit
                // actor never binds one, so it would otherwise inherit a
                // player bound by the surrounding trigger.
                EffectAst::subject_verb_sacrifice_all(
                    if matches!(player, PlayerAst::Implicit) {
                        PlayerAst::Implicit
                    } else {
                        PlayerAst::That
                    },
                    ObjectFilter::tagged(tag),
                ),
            ]
        };
        if let Some(followup_tokens) = followup_tokens {
            if followup_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing effect after sacrifice comma-then (clause: '{}')",
                    normalized_words.join(" ")
                )));
            }
            let mut followup = crate::effect_sentences::parse_effect_chain(followup_tokens)?;
            if followup.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported effect after sacrifice comma-then (clause: '{}')",
                    normalized_words.join(" ")
                )));
            }
            effects.append(&mut followup);
        }
        return Ok(wrap_unless_escaped(
            EffectAst::Sequence { effects },
            unless_escaped,
        ));
    }

    if let Some(quantity) = sacrifice_discard_grammar::parse_sacrifice_quantity_shape(tokens) {
        match quantity {
            sacrifice_discard_grammar::SacrificeQuantityShape::ThatMany { filter_tokens } => {
                if filter_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing sacrifice object after that many (clause: '{}')",
                        normalized_words.join(" ")
                    )));
                }
                let filter = parse_object_filter_lexed(filter_tokens, false)?;
                let tag = crate::util::helper_tag_for_tokens(tokens, "sacrificed");
                return Ok(wrap_unless_escaped(
                    EffectAst::Sequence {
                        effects: vec![
                            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                                filter,
                                count: crate::effect::ChoiceCount::dynamic_x(),
                                count_value: Some(Value::EventValue(EventValueSpec::Amount)),
                                player,
                                tag: crate::tag::TagRef::of(tag.clone()),
                            }),
                            EffectAst::subject_verb_sacrifice_all(
                                PlayerAst::That,
                                ObjectFilter::tagged(tag),
                            ),
                        ],
                    },
                    unless_escaped,
                ));
            }
            sacrifice_discard_grammar::SacrificeQuantityShape::AllOrEach {
                filter_tokens,
                other,
                each_surface,
            } => {
                let mut filter = parse_object_filter_lexed(filter_tokens, other)?;
                preserve_branch_scoped_card_type_union(&mut filter, filter_tokens, other);
                preserve_terminal_nonbasic_land_union(filter_tokens, &mut filter);
                if other {
                    filter.other = true;
                }
                if each_surface {
                    filter.set_set_quantifier_surface(Some(
                        ironsmith_core::SetQuantifierSurface::Each,
                    ));
                }
                return Ok(wrap_unless_escaped(
                    EffectAst::subject_verb_sacrifice_all(player, filter),
                    unless_escaped,
                ));
            }
            sacrifice_discard_grammar::SacrificeQuantityShape::AllExcept {
                filter_tokens,
                keep_count,
                other,
            } => {
                let mut filter = parse_object_filter_lexed(filter_tokens, other)?;
                preserve_branch_scoped_card_type_union(&mut filter, filter_tokens, other);
                preserve_terminal_nonbasic_land_union(filter_tokens, &mut filter);
                filter.zone = Some(Zone::Battlefield);
                if other {
                    filter.other = true;
                }
                let keep_count = i32::try_from(keep_count).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "sacrifice exception count is too large (clause: '{}')",
                        normalized_words.join(" ")
                    ))
                })?;
                let count_value = Value::Add(
                    Box::new(Value::Count(filter.clone())),
                    Box::new(Value::Fixed(-keep_count)),
                );
                let tag = crate::util::helper_tag_for_tokens(tokens, "sacrificed");
                return Ok(wrap_unless_escaped(
                    EffectAst::Sequence {
                        effects: vec![
                            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                                filter,
                                count: crate::effect::ChoiceCount::dynamic_x(),
                                count_value: Some(count_value),
                                player,
                                tag: crate::tag::TagRef::of(tag.clone()),
                            }),
                            EffectAst::subject_verb_sacrifice_all(
                                PlayerAst::That,
                                ObjectFilter::tagged(tag),
                            ),
                        ],
                    },
                    unless_escaped,
                ));
            }
        }
    }

    let count_shape = sacrifice_discard_grammar::parse_sacrifice_count_shape(tokens);
    let count = count_shape.count;
    let other = count_shape.other;

    // Split off a trailing "for each ..." suffix before parsing the filter.
    let remaining_tokens = count_shape.filter_tokens;
    let mut greatest_mana_value_reference_filter = None;
    let mut greatest_power_reference_filter = None;
    let object_clause_tokens = if let Some(aggregate) =
        sacrifice_discard_grammar::parse_sacrifice_aggregate_shape(remaining_tokens)
    {
        if aggregate.among_tokens.is_empty() {
            let axis = match aggregate.kind {
                sacrifice_discard_grammar::SacrificeAggregateKind::GreatestManaValue => {
                    "mana value"
                }
                sacrifice_discard_grammar::SacrificeAggregateKind::GreatestPower => "power",
            };
            return Err(CardTextError::ParseError(format!(
                "missing object set after greatest {axis} among (clause: '{}')",
                normalized_words.join(" ")
            )));
        }
        let among_filter = parse_object_filter_lexed(aggregate.among_tokens, false)?;
        match aggregate.kind {
            sacrifice_discard_grammar::SacrificeAggregateKind::GreatestManaValue => {
                greatest_mana_value_reference_filter = Some(among_filter);
            }
            sacrifice_discard_grammar::SacrificeAggregateKind::GreatestPower => {
                greatest_power_reference_filter = Some(among_filter);
            }
        }
        aggregate.object_tokens
    } else {
        remaining_tokens
    };

    if object_clause_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in clause (clause: '{}')",
            normalized_words.join(" ")
        )));
    }
    let for_each_idx =
        grammar::find_prefix(object_clause_tokens, || grammar::phrase(&["for", "each"]))
            .map(|(idx, _, _)| idx);

    let (object_tokens, for_each_filter) = if let Some(fe_idx) = for_each_idx {
        let fe_count_tokens = &object_clause_tokens[fe_idx..];
        let fe_value = parse_get_for_each_count_value(fe_count_tokens)?;
        (&object_clause_tokens[..fe_idx], fe_value)
    } else {
        (object_clause_tokens, None)
    };

    let object_shape = sacrifice_discard_grammar::parse_sacrifice_object_shape(object_tokens);
    let one_of_referenced_set = matches!(
        object_shape.tagged_reference,
        Some(sacrifice_discard_grammar::SacrificeTaggedReferenceKind::OneOfTaggedSet)
    );
    let filter_tokens = object_shape.filter_tokens;
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object after chooser suffix (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }
    let all_of_referenced_set = matches!(
        object_shape.tagged_reference,
        Some(sacrifice_discard_grammar::SacrificeTaggedReferenceKind::AllOfTaggedSet)
    );
    let mut filter = if all_of_referenced_set {
        // Preserve the authored plural noun and demonstrative surface while
        // keeping the filter tied to the exact preceding result set.
        parse_object_filter_lexed(filter_tokens, other)?
    } else if let Some(tagged_reference) = object_shape.tagged_reference {
        let mut tagged_filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
        tagged_filter.zone = Some(Zone::Battlefield);
        if tagged_reference == sacrifice_discard_grammar::SacrificeTaggedReferenceKind::Token {
            tagged_filter.token = true;
        }
        tagged_filter
    } else if let Some(filter) = parse_artifact_enchantment_or_token_filter(filter_tokens) {
        filter
    } else {
        parse_object_filter_lexed(filter_tokens, other)?
    };
    if other {
        filter.other = true;
    }
    if let Some(among_filter) = greatest_mana_value_reference_filter {
        filter.mana_value = Some(crate::filter::Comparison::EqualExpr(Box::new(
            Value::GreatestManaValue(among_filter),
        )));
    }
    if let Some(among_filter) = greatest_power_reference_filter {
        filter.power = Some(crate::filter::Comparison::EqualExpr(Box::new(
            Value::GreatestPower(among_filter),
        )));
    }
    if filter.source && count != 1 {
        return Err(CardTextError::ParseError(format!(
            "source sacrifice only supports count 1 (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }
    let excludes_attached_object =
        sacrifice_discard_grammar::parse_sacrifice_attached_exclusion(tokens);
    if excludes_attached_object
        && filter.controller.is_none()
        && let Some(controller) = controller_filter_for_token_player(player)
    {
        filter.controller = Some(controller);
    }

    if opponent_chooses_object {
        let sacrificing_player = if player == PlayerAst::Implicit {
            PlayerAst::You
        } else {
            player
        };
        if filter.controller.is_none()
            && let Some(controller) = controller_filter_for_token_player(sacrificing_player)
        {
            filter.controller = Some(controller);
        }
        let tag = crate::util::helper_tag_for_tokens(tokens, "sacrificed");
        return Ok(wrap_unless_escaped(
            EffectAst::Sequence {
                effects: vec![
                    EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                        filter,
                        count: crate::effect::ChoiceCount::exactly(count as usize),
                        count_value: None,
                        player: PlayerAst::Opponent,
                        tag: crate::tag::TagRef::of(tag.clone()),
                    }),
                    EffectAst::subject_verb_sacrifice_all(
                        sacrificing_player,
                        ObjectFilter::tagged(tag),
                    ),
                ],
            },
            unless_escaped,
        ));
    }

    // A caller-supplied antecedent target ("its controller sacrifices IT")
    // only applies when the object phrase is a co-referent pronoun. For a
    // real filter ("its controller sacrifices a land of their choice") the
    // target would silently replace the filter at lowering.
    let target = if object_shape.tagged_reference.is_some() {
        target
    } else {
        None
    };
    let sacrifice = if all_of_referenced_set {
        EffectAst::subject_verb_sacrifice_all(player, filter)
    } else {
        let sacrifice = EffectAst::subject_verb_sacrifice(player, filter, count, target);
        if one_of_referenced_set {
            sacrifice.with_sacrifice_one_of_referenced_set()
        } else {
            sacrifice
        }
    };

    // Wrap in ForEachObject when the clause has a "for each <filter>" suffix,
    // e.g. "sacrifices a land for each card in your hand".
    let effect = match for_each_filter {
        Some(Value::Count(fe_filter)) => EffectAst::ForEach(ForEachEffectAst::ForEachObject {
            filter: fe_filter,
            effects: vec![sacrifice],
        }),
        Some(count) => EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
            count: count.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
            effects: vec![sacrifice],
        }),
        None => sacrifice,
    };
    Ok(wrap_unless_escaped(effect, unless_escaped))
}

/// Split "a creature, an artifact, and a land" into its indefinite object
/// phrases. Only an `and` list of two or more `a`/`an` phrases qualifies; an
/// `or` list is one type union ("a creature, artifact, or land").
pub fn sacrifice_object_list_members(tokens: &[OwnedLexToken]) -> Option<Vec<&[OwnedLexToken]>> {
    let mut members = Vec::new();
    let mut start = 0usize;
    let mut saw_and = false;
    for (index, token) in tokens.iter().enumerate() {
        let is_and = token.is_word("and");
        if token.is_word("or") {
            return None;
        }
        if token.is_comma() || is_and {
            let member = crate::util::trim_edge_punctuation_tokens(&tokens[start..index]);
            if !member.is_empty() {
                members.push(member);
            }
            saw_and |= is_and;
            start = index + 1;
        }
    }
    let last = crate::util::trim_edge_punctuation_tokens(&tokens[start..]);
    if !last.is_empty() {
        members.push(last);
    }
    if !saw_and || members.len() < 2 {
        return None;
    }
    members
        .iter()
        .all(|member| {
            member.len() >= 2
                && member
                    .first()
                    .is_some_and(|token| token.is_word("a") || token.is_word("an"))
        })
        .then_some(members)
}

fn parse_sacrifice_object_list(
    tokens: &[OwnedLexToken],
    player: PlayerAst,
    opponent_chooses_object: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    if opponent_chooses_object {
        return Ok(None);
    }
    let Some(members) = sacrifice_object_list_members(tokens) else {
        return Ok(None);
    };
    let mut filters = Vec::with_capacity(members.len());
    for member in &members {
        let Ok(mut filter) = parse_object_filter_lexed(&member[1..], false) else {
            return Ok(None);
        };
        if filter.source || !filter.tagged_constraints.is_empty() {
            return Ok(None);
        }
        filter.zone = Some(Zone::Battlefield);
        if filter.controller.is_none() {
            filter.controller = controller_filter_for_token_player(player);
        }
        filters.push(filter);
    }
    let tag = crate::util::helper_tag_for_tokens(tokens, "sacrificed");
    let mut effects = filters
        .into_iter()
        .map(|filter| {
            // Each later choice excludes the permanents already chosen, so an
            // artifact creature can't answer for both "a creature" and "an
            // artifact".
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: filter.not_tagged(tag.clone()),
                count: crate::effect::ChoiceCount::exactly(1),
                count_value: None,
                player,
                tag: crate::tag::TagRef::of(tag.clone()),
            })
        })
        .collect::<Vec<_>>();
    effects.push(EffectAst::subject_verb_sacrifice_all(
        if matches!(player, PlayerAst::Implicit) {
            PlayerAst::Implicit
        } else {
            PlayerAst::That
        },
        ObjectFilter::tagged(tag),
    ));
    Ok(Some(EffectAst::Sequence { effects }))
}

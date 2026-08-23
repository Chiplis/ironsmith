use super::*;
use crate::effect_sentences::parse_artifact_enchantment_or_token_filter;
use crate::effect_sentences::subject_verb_primitives::{
    SubjectVerbPrimitiveClause, rewrite_unless_cost_source_values_to_it_tag, try_build_unless,
};
use crate::grammar::effects::sacrifice_discard_shapes as sacrifice_discard_grammar;
use crate::grammar::filters::preserve_branch_scoped_card_type_union;

fn trim_trailing_discard_alternative_action(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let discard_tokens = sacrifice_discard_grammar::parse_discard_alternative_shape(tokens)
        .map(|shape| shape.discard_tokens)
        .unwrap_or(tokens);
    trim_commas(discard_tokens)
}

fn parse_trailing_discard_unless_predicate(
    trailing_tokens: &[OwnedLexToken],
    player: PlayerAst,
    count: Value,
    any_number: bool,
    discard_filter: Option<ObjectFilter>,
) -> Result<Option<EffectAst>, CardTextError> {
    let predicate_tokens =
        match sacrifice_discard_grammar::parse_discard_unless_shape(trailing_tokens) {
            sacrifice_discard_grammar::DiscardUnlessShape::None => return Ok(None),
            sacrifice_discard_grammar::DiscardUnlessShape::MissingPredicate => {
                return Err(CardTextError::ParseError(
                    "missing predicate after trailing discard unless".to_string(),
                ));
            }
            sacrifice_discard_grammar::DiscardUnlessShape::Predicate(predicate_tokens) => {
                predicate_tokens
            }
        };
    let predicate =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens)?;
    let discard =
        EffectAst::subject_verb_discard(player, count, false, any_number, discard_filter, None);

    Ok(Some(EffectAst::Conditional {
        predicate: PredicateAst::Not(Box::new(predicate)),
        if_true: vec![discard],
        if_false: Vec::new(),
    }))
}

fn wrap_unless_escaped(effect: EffectAst, unless_escaped: bool) -> EffectAst {
    if unless_escaped {
        EffectAst::Conditional {
            predicate: PredicateAst::ThisSpellEscaped,
            if_true: Vec::new(),
            if_false: vec![effect],
        }
    } else {
        effect
    }
}

fn triggering_same_mana_value_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter
        .tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: crate::TagKey::from("triggering"),
            relation: crate::target::TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    filter
}

/// Keep an adjective attached to its terminal noun in a serialized union.
///
/// The broad card-type parser can flatten `enchantments and nonbasic lands`
/// into one branch, which incorrectly applies `nonbasic` to both types.  When
/// the source tokens explicitly anchor the adjective immediately before
/// `land(s)`, split that mixed branch and retain the Basic exclusion only on
/// the land arm.
pub fn preserve_terminal_nonbasic_land_union(tokens: &[OwnedLexToken], filter: &mut ObjectFilter) {
    let adjective_is_land_local = tokens.windows(2).any(|window| {
        window[0].is_word("nonbasic") && (window[1].is_word("land") || window[1].is_word("lands"))
    });
    if !adjective_is_land_local {
        return;
    }

    // Some whole-sentence subject/verb routes flatten the type list before
    // the later correlated-result pass sees it. The source adjacency still
    // proves `nonbasic` belongs only to the land arm, so recover the same
    // inclusive union from the flattened selector.
    if filter.any_of.is_empty()
        && filter.card_types.len() > 1
        && filter.card_types.contains(&crate::types::CardType::Land)
        && filter
            .excluded_supertypes
            .contains(&crate::types::Supertype::Basic)
    {
        let card_types = std::mem::take(&mut filter.card_types);
        filter
            .excluded_supertypes
            .retain(|supertype| *supertype != crate::types::Supertype::Basic);
        filter.any_of = card_types
            .into_iter()
            .map(|card_type| {
                let mut branch = ObjectFilter::default().with_type(card_type);
                if card_type == crate::types::CardType::Land {
                    branch
                        .excluded_supertypes
                        .push(crate::types::Supertype::Basic);
                }
                branch
            })
            .collect();
        return;
    }

    if filter.any_of.is_empty() {
        return;
    }

    let mut branches = Vec::new();
    for branch in std::mem::take(&mut filter.any_of) {
        if branch.card_types.len() > 1
            && branch.card_types.contains(&crate::types::CardType::Land)
            && branch
                .excluded_supertypes
                .contains(&crate::types::Supertype::Basic)
        {
            for card_type in &branch.card_types {
                let mut split = branch.clone();
                split.card_types = vec![*card_type];
                if *card_type != crate::types::CardType::Land {
                    split
                        .excluded_supertypes
                        .retain(|supertype| *supertype != crate::types::Supertype::Basic);
                }
                branches.push(split);
            }
        } else {
            branches.push(branch);
        }
    }
    filter.any_of = branches;
}

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
    let tokens = clause_shape.body_tokens;
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
                return Ok(EffectAst::Conditional {
                    predicate: PredicateAst::ManaSpentToCastThisSpellAtLeast {
                        amount: 1,
                        symbol: Some(symbol),
                    },
                    if_true: Vec::new(),
                    if_false: vec![base],
                });
            }
            sacrifice_discard_grammar::SacrificeUnlessKind::OpponentDamagedThisTurn => {
                return Ok(EffectAst::Conditional {
                    predicate: PredicateAst::OpponentWasDealtDamageThisTurn,
                    if_true: Vec::new(),
                    if_false: vec![base],
                });
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

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

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
                    EffectAst::ChooseObjects {
                        filter,
                        count: crate::effect::ChoiceCount::dynamic_x(),
                        count_value: Some(count_value),
                        player,
                        tag: tag.clone(),
                    },
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
        let filter_tokens = &tokens[used..];
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing sacrifice object after choice count (clause: '{}')",
                normalized_words.join(" ")
            )));
        }
        let filter = parse_object_filter_lexed(filter_tokens, false)?;
        let tag = crate::util::helper_tag_for_tokens(tokens, "sacrificed");
        return Ok(wrap_unless_escaped(
            EffectAst::Sequence {
                effects: vec![
                    EffectAst::ChooseObjects {
                        filter,
                        count: choice_count,
                        count_value: None,
                        player,
                        tag: tag.clone(),
                    },
                    EffectAst::subject_verb_sacrifice_all(
                        PlayerAst::That,
                        ObjectFilter::tagged(tag),
                    ),
                ],
            },
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
                            EffectAst::ChooseObjects {
                                filter,
                                count: crate::effect::ChoiceCount::dynamic_x(),
                                count_value: Some(Value::EventValue(EventValueSpec::Amount)),
                                player,
                                tag: tag.clone(),
                            },
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
                            EffectAst::ChooseObjects {
                                filter,
                                count: crate::effect::ChoiceCount::dynamic_x(),
                                count_value: Some(count_value),
                                player,
                                tag: tag.clone(),
                            },
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
        let mut tagged_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
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
                    EffectAst::ChooseObjects {
                        filter,
                        count: crate::effect::ChoiceCount::exactly(count as usize),
                        count_value: None,
                        player: PlayerAst::Opponent,
                        tag: tag.clone(),
                    },
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
        Some(Value::Count(fe_filter)) => EffectAst::ForEachObject {
            filter: fe_filter,
            effects: vec![sacrifice],
        },
        Some(count) => EffectAst::RepeatEffects {
            count: count.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
            effects: vec![sacrifice],
        },
        None => sacrifice,
    };
    Ok(wrap_unless_escaped(effect, unless_escaped))
}

pub fn parse_discard(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::lexer::token_word_refs(tokens);
    let clause_shape = sacrifice_discard_grammar::parse_discard_clause_shape(tokens).map_err(
        |error| match error {
            sacrifice_discard_grammar::DiscardShapeError::MissingCount => {
                CardTextError::ParseError(format!(
                    "missing discard count (clause: '{}')",
                    clause_words.join(" ")
                ))
            }
            sacrifice_discard_grammar::DiscardShapeError::MissingCardKeyword => {
                CardTextError::ParseError("missing card keyword".to_string())
            }
        },
    )?;
    let cards_shape = match clause_shape {
        sacrifice_discard_grammar::DiscardClauseShape::Hand => {
            return Ok(EffectAst::subject_verb_discard_hand(player));
        }
        sacrifice_discard_grammar::DiscardClauseShape::AllCardsInHand => {
            let owner = if clause_words
                .windows(2)
                .any(|words| words == ["your", "hand"])
            {
                PlayerFilter::You
            } else if clause_words
                .windows(2)
                .any(|words| words == ["their", "hand"])
                || clause_words
                    .windows(3)
                    .any(|words| words == ["that", "players", "hand"])
            {
                PlayerFilter::IteratedPlayer
            } else {
                discard_subject_owner_filter(subject).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing full-hand discard owner (clause: '{}')",
                        clause_words.join(" ")
                    ))
                })?
            };
            return Ok(EffectAst::subject_verb_discard(
                player,
                Value::CardsInHand(owner)
                    .with_surface_hint(ironsmith_core::ValueSurfaceHint::AllCardsInHand),
                false,
                false,
                None,
                None,
            ));
        }
        sacrifice_discard_grammar::DiscardClauseShape::TaggedOne => {
            let mut tagged_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
            tagged_filter.zone = Some(Zone::Hand);
            return Ok(EffectAst::subject_verb_discard(
                player,
                Value::Fixed(1),
                false,
                false,
                Some(tagged_filter),
                None,
            ));
        }
        sacrifice_discard_grammar::DiscardClauseShape::TaggedAll => {
            let mut tagged_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
            tagged_filter.zone = Some(Zone::Hand);
            return Ok(EffectAst::subject_verb_discard(
                player,
                Value::Count(tagged_filter.clone()),
                false,
                false,
                Some(tagged_filter),
                None,
            ));
        }
        sacrifice_discard_grammar::DiscardClauseShape::EqualCount {
            count,
            trailing_tokens,
        } => {
            let trailing_tokens = trim_commas(trailing_tokens);
            let trailing_shape =
                sacrifice_discard_grammar::parse_discard_trailing_shape(&trailing_tokens);
            let random = trailing_shape == sacrifice_discard_grammar::DiscardTrailingShape::Random;
            if trailing_shape != sacrifice_discard_grammar::DiscardTrailingShape::Empty && !random {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing discard clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(EffectAst::subject_verb_discard(
                player, count, random, false, None, None,
            ));
        }
        sacrifice_discard_grammar::DiscardClauseShape::Cards(cards) => cards,
    };
    let uses_all_count = cards_shape.uses_all_count;
    let mut count = cards_shape.count;
    let any_number = cards_shape.any_number;
    let qualifier_tokens = trim_commas(cards_shape.qualifier_tokens);
    let qualifier_shape =
        sacrifice_discard_grammar::parse_discard_qualifier_shape(&qualifier_tokens);
    let mut discard_filter = None;
    if qualifier_shape != sacrifice_discard_grammar::DiscardQualifierShape::EmptyOrThe {
        let mut filter = if let Ok(filter) = parse_object_filter(&qualifier_tokens, false) {
            filter
        } else {
            match qualifier_shape {
                sacrifice_discard_grammar::DiscardQualifierShape::ChosenColor => {
                    let mut filter = ObjectFilter::default();
                    filter.chosen_color = true;
                    filter
                }
                sacrifice_discard_grammar::DiscardQualifierShape::Colors(colors) => {
                    let mut filter = ObjectFilter::default();
                    filter.colors = Some(colors);
                    filter
                }
                _ => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported discard card qualifier (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
            }
        };
        filter.zone = Some(Zone::Hand);
        if uses_all_count
            && let Some(owner) = discard_subject_owner_filter(subject)
            && filter.owner.is_none()
        {
            filter.owner = Some(owner);
        }
        discard_filter = Some(filter);
    }

    let trailing_tokens_storage =
        trim_trailing_discard_alternative_action(cards_shape.trailing_tokens);
    let trailing_tokens = trailing_tokens_storage.as_slice();
    if let Some(dynamic_count) = parse_get_for_each_count_value(trailing_tokens)? {
        count = dynamic_count.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
        return Ok(EffectAst::subject_verb_discard(
            player,
            count,
            false,
            any_number,
            discard_filter,
            None,
        ));
    }
    if let Some(effect) = parse_trailing_discard_unless_predicate(
        trailing_tokens,
        player,
        count.clone(),
        any_number,
        discard_filter.clone(),
    )? {
        return Ok(effect);
    }
    let trailing_shape = sacrifice_discard_grammar::parse_discard_trailing_shape(trailing_tokens);
    let random = trailing_shape == sacrifice_discard_grammar::DiscardTrailingShape::Random;
    if trailing_shape != sacrifice_discard_grammar::DiscardTrailingShape::Empty && !random {
        let additional_cost_colors =
            sacrifice_discard_grammar::parse_additional_cost_object_colors_surface(trailing_tokens);
        let trailing_filter = if let Some(surface) = additional_cost_colors {
            let mut filter = ObjectFilter::default().match_tagged(
                TagKey::from(ADDITIONAL_COST_OBJECT_TAG),
                TaggedOpbjectRelation::SharesColorWithTagged,
            );
            filter.set_additional_cost_object_surface(Some(surface));
            Some(filter)
        } else if let Ok(filter) = parse_object_filter(trailing_tokens, false) {
            Some(filter)
        } else {
            match trailing_shape {
                sacrifice_discard_grammar::DiscardTrailingShape::ChosenName => {
                    let mut filter = ObjectFilter::default();
                    filter.name = Some("{chosen name}".to_string());
                    Some(filter)
                }
                sacrifice_discard_grammar::DiscardTrailingShape::ChosenColor => {
                    let mut filter = ObjectFilter::default();
                    filter.chosen_color = true;
                    Some(filter)
                }
                sacrifice_discard_grammar::DiscardTrailingShape::SameManaValueAsTriggering => {
                    Some(triggering_same_mana_value_filter())
                }
                sacrifice_discard_grammar::DiscardTrailingShape::Colors(colors) => {
                    let mut filter = ObjectFilter::default();
                    filter.colors = Some(colors);
                    Some(filter)
                }
                _ => None,
            }
        };

        if let Some(mut filter) = trailing_filter {
            filter.zone = Some(Zone::Hand);
            if uses_all_count
                && let Some(owner) = discard_subject_owner_filter(subject)
                && filter.owner.is_none()
            {
                filter.owner = Some(owner);
            }
            discard_filter = Some(filter);
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing discard clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if uses_all_count {
        count = if let Some(filter) = discard_filter.as_ref() {
            Value::Count(filter.clone())
        } else if let Some(owner) = discard_subject_owner_filter(subject) {
            Value::CardsInHand(owner)
        } else {
            return Err(CardTextError::ParseError(format!(
                "missing discard count (clause: '{}')",
                clause_words.join(" ")
            )));
        };
    }

    Ok(EffectAst::subject_verb_discard(
        player,
        count,
        random,
        any_number,
        discard_filter,
        None,
    ))
}

pub fn discard_subject_owner_filter(subject: Option<SubjectAst>) -> Option<PlayerFilter> {
    match subject {
        Some(SubjectAst::Player(PlayerAst::Target)) => Some(PlayerFilter::target_player()),
        Some(SubjectAst::Player(PlayerAst::TargetOpponent)) => {
            Some(PlayerFilter::target_opponent())
        }
        Some(SubjectAst::Player(PlayerAst::That)) => Some(PlayerFilter::IteratedPlayer),
        Some(SubjectAst::Player(PlayerAst::You)) => Some(PlayerFilter::You),
        _ => None,
    }
}

#[cfg(test)]
mod selected_sacrifice_tests {
    use super::*;
    use crate::lexer::lex_line;
    use crate::model::ast::{SubjectVerbActionAst, SubjectVerbEffectAst};

    #[test]
    fn opponent_choice_delegates_selection_without_changing_the_sacrificing_player() {
        let tokens = lex_line("Sacrifice a land of an opponent's choice.", 0)
            .expect("sacrifice clause should lex");
        let parsed =
            parse_sacrifice(&tokens, None, None).expect("opponent-chosen sacrifice should parse");
        let debug = format!("{parsed:#?}");

        assert!(debug.contains("ChooseObjects"), "{debug}");
        assert!(debug.contains("player: Opponent"), "{debug}");
        assert!(debug.contains("player: You"), "{debug}");
    }

    #[test]
    fn chooser_sacrifices_only_the_selected_set() {
        let tokens = lex_line("Sacrifices that many permanents of their choice.", 0)
            .expect("sacrifice clause should lex");
        let parsed = parse_sacrifice(
            &tokens,
            Some(SubjectAst::Player(PlayerAst::ItsController)),
            None,
        )
        .expect("sacrifice choice should parse");
        let debug = format!("{parsed:#?}");

        assert!(debug.contains("ChooseObjects"), "{debug}");
        assert!(debug.contains("player: ItsController"), "{debug}");
        assert!(debug.contains("player: That"), "{debug}");
        assert!(debug.contains("IsTaggedObject"), "{debug}");
    }

    #[test]
    fn one_of_them_is_a_choice_from_the_referenced_set() {
        let tokens =
            lex_line("Sacrifice one of them.", 0).expect("tagged-set sacrifice clause should lex");
        let parsed =
            parse_sacrifice(&tokens, None, None).expect("tagged-set sacrifice choice should parse");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Sacrifice {
                    filter,
                    count,
                    target,
                    one_of_referenced_set,
                },
            ..
        }) = parsed
        else {
            panic!("expected subject-verb sacrifice AST");
        };

        assert_eq!(count, 1);
        assert!(target.is_none());
        assert!(one_of_referenced_set);
        assert_eq!(filter.tagged_constraints.len(), 1);
        assert_eq!(filter.tagged_constraints[0].tag.as_str(), IT_TAG);
    }

    #[test]
    fn those_permanents_sacrifices_the_complete_referenced_set() {
        let tokens = lex_line("Sacrifice those permanents.", 0)
            .expect("plural tagged-set sacrifice clause should lex");
        let parsed = parse_sacrifice(&tokens, Some(SubjectAst::Player(PlayerAst::That)), None)
            .expect("plural tagged-set sacrifice should parse");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SacrificeAll { filter },
            ..
        }) = parsed
        else {
            panic!("expected all-of-result-set sacrifice AST");
        };
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(
            filter.card_types.len() > 1,
            "the permanent noun should survive alongside the result tag: {filter:#?}"
        );
    }

    #[test]
    fn sacrifice_all_keeps_a_terminal_nonbasic_qualifier_on_only_the_land_arm() {
        let tokens = lex_line(
            "all artifacts, enchantments, and nonbasic lands they control.",
            0,
        )
        .expect("sacrifice-all clause should lex");
        let parsed = parse_sacrifice(&tokens, Some(SubjectAst::Player(PlayerAst::That)), None)
            .expect("sacrifice-all clause should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SacrificeAll { filter },
            ..
        }) = parsed
        else {
            panic!("expected sacrifice-all subject-verb effect");
        };

        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
        assert!(filter.any_of.iter().any(|branch| {
            branch.card_types == [crate::types::CardType::Artifact]
                && branch.excluded_supertypes.is_empty()
        }));
        assert!(filter.any_of.iter().any(|branch| {
            branch.card_types == [crate::types::CardType::Enchantment]
                && branch.excluded_supertypes.is_empty()
        }));
        assert!(filter.any_of.iter().any(|branch| {
            branch.card_types == [crate::types::CardType::Land]
                && branch.excluded_supertypes == [crate::types::Supertype::Basic]
        }));
    }

    #[test]
    fn unit_fraction_rounded_up_sacrifice_chooses_the_exact_dynamic_set() {
        let tokens = lex_line(
            "Sacrifices a tenth of the creatures they control of their choice, rounded up.",
            0,
        )
        .expect("fractional sacrifice clause should lex");
        let parsed = parse_sacrifice(&tokens, Some(SubjectAst::Player(PlayerAst::That)), None)
            .expect("fractional sacrifice clause should parse");
        let EffectAst::Sequence { effects } = parsed else {
            panic!("fractional sacrifice must lower to a chosen-set sequence");
        };
        let [choose, sacrifice] = effects.as_slice() else {
            panic!("fractional sacrifice must have one choice and one consumer");
        };
        let EffectAst::ChooseObjects {
            filter,
            count,
            count_value: Some(count_value),
            player,
            tag,
        } = choose
        else {
            panic!("fractional sacrifice must choose a dynamic object set");
        };
        assert!(count.is_dynamic_x());
        assert_eq!(*player, PlayerAst::That);
        assert_eq!(filter.zone, Some(Zone::Battlefield));

        let Value::DividedRoundedDown(numerator, 10) = count_value else {
            panic!("a tenth rounded up must use exact ceil-division: {count_value:#?}");
        };
        let Value::Add(left, right) = numerator.as_ref() else {
            panic!("ceil-division must add denominator minus one: {numerator:#?}");
        };
        assert!(
            matches!(
                (left.as_ref(), right.as_ref()),
                (Value::Count(count_filter), Value::Fixed(9))
                    | (Value::Fixed(9), Value::Count(count_filter))
                    if count_filter == filter
            ),
            "ceil-division must count exactly the selectable set: {count_value:#?}"
        );

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::SacrificeAll { filter: sacrificed },
        }) = sacrifice
        else {
            panic!("chosen set must feed the typed sacrifice consumer");
        };
        assert_eq!(subject.player, PlayerAst::That);
        assert_eq!(sacrificed, &ObjectFilter::tagged(tag.clone()));
    }

    #[test]
    fn sacrifice_all_except_kept_count_chooses_count_minus_keep_set() {
        let tokens = lex_line("Sacrifices all lands they control except for three.", 0)
            .expect("all-except sacrifice clause should lex");
        let parsed = parse_sacrifice(&tokens, Some(SubjectAst::Player(PlayerAst::That)), None)
            .expect("all-except sacrifice clause should parse");
        let EffectAst::Sequence { effects } = parsed else {
            panic!("all-except sacrifice must lower to a chosen-set sequence");
        };
        let [
            EffectAst::ChooseObjects {
                filter,
                count,
                count_value: Some(Value::Add(left, right)),
                player: PlayerAst::That,
                tag,
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject,
                action: SubjectVerbActionAst::SacrificeAll { filter: sacrificed },
            }),
        ] = effects.as_slice()
        else {
            panic!("expected exact dynamic choice and sacrifice: {effects:#?}");
        };
        assert!(count.is_dynamic_x());
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.card_types, [crate::types::CardType::Land]);
        assert!(matches!(
            (left.as_ref(), right.as_ref()),
            (Value::Count(count_filter), Value::Fixed(-3))
                | (Value::Fixed(-3), Value::Count(count_filter))
                if count_filter == filter
        ));
        assert_eq!(subject.player, PlayerAst::That);
        assert_eq!(sacrificed, &ObjectFilter::tagged(tag.clone()));
    }
}

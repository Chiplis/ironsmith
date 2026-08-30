use super::*;

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
            let owner = if crate::word_primitives::sequence_occurs(&clause_words, &["your", "hand"])
            {
                PlayerFilter::You
            } else if crate::word_primitives::sequence_occurs(&clause_words, &["their", "hand"])
                || crate::word_primitives::sequence_occurs(
                    &clause_words,
                    &["that", "players", "hand"],
                )
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
            let mut tagged_filter =
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key());
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
            let mut tagged_filter =
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key());
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
                crate::tag::CompilerReferenceTag::AdditionalCostObject.key(),
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

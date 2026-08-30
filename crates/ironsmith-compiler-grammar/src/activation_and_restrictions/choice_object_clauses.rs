use super::*;
use crate::grammar::choices::{
    ChoiceBattlefieldController, ChoiceBecomeKind, ChoiceBecomeSyntaxError, ChoiceClauseActor,
    ChoiceObjectClauseSyntaxError, ChoiceObjectCountSource, ChoicePlayerClauseSyntaxError,
    ChoiceTypePhraseSyntaxError, ChosenCantBlockSyntaxError, TargetPlayerChoiceActor,
    TypedChoiceBecomeSubject, TypedChoiceObjectClauseKind,
    parse_choice_basic_land_type_phrase_words, parse_choice_battlefield_move_shape,
    parse_choice_card_type_phrase_words as parse_typed_choice_card_type_phrase_words,
    parse_choice_card_type_reveal_shape_words,
    parse_choice_color_phrase_words as parse_typed_choice_color_phrase_words,
    parse_choice_creature_type_phrase_words as parse_typed_choice_creature_type_phrase_words,
    parse_choice_land_type_phrase_words as parse_typed_choice_land_type_phrase_words,
    parse_choice_library_move_shape, parse_choice_player_clause_tokens,
    parse_choice_player_phrase_words as parse_typed_choice_player_phrase_words,
    parse_that_type_tokens, parse_typed_choice_become_shape,
    parse_typed_choice_object_clause_tokens, parse_typed_chosen_cant_block_tokens,
    parse_typed_target_player_choice_tokens,
};

pub fn parse_target_player_choose_objects_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    Ok(
        parse_target_player_choose_objects_clause_with_count_value(tokens)?
            .map(|(chooser, filter, count, _count_value)| (chooser, filter, count)),
    )
}

pub fn parse_target_player_choose_objects_clause_with_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount, Option<Value>)>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let parsed = match parse_typed_target_player_choice_tokens(tokens) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => return Ok(None),
        Err(ChoiceObjectClauseSyntaxError::MissingObject) => {
            return Err(CardTextError::ParseError(format!(
                "missing chosen object after target-player choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        Err(ChoiceObjectClauseSyntaxError::MissingFilter) => {
            return Err(CardTextError::ParseError(format!(
                "missing chosen object filter after count in target-player choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        Err(ChoiceObjectClauseSyntaxError::UnsupportedFilter) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported chosen object filter in target-player choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };

    let mut chooser = match parsed.actor {
        TargetPlayerChoiceActor::TargetPlayer => PlayerAst::Target,
        TargetPlayerChoiceActor::TargetOpponent => PlayerAst::TargetOpponent,
        TargetPlayerChoiceActor::Opponent => PlayerAst::Opponent,
        TargetPlayerChoiceActor::ThatPlayer | TargetPlayerChoiceActor::Voter => PlayerAst::That,
    };
    let mut choose_filter = parsed.filter;
    if chooser == PlayerAst::That
        && choose_filter.controller.is_none()
        && choose_filter.owner.is_none()
        && choose_filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
    {
        chooser = PlayerAst::ItsController;
    }
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    // Choosing an unrestricted battlefield object does not imply that the
    // chooser controls it. Preserve an inferred controller only for a tagged
    // antecedent whose actor is explicitly derived from that object; ordinary
    // text must say `they control` when that restriction is intended.
    if choose_filter.controller.is_none()
        && choose_filter.owner.is_none()
        && choose_filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
    {
        choose_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::Opponent => PlayerFilter::Opponent,
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            PlayerAst::ItsController => PlayerFilter::ControllerOf(
                crate::filter::ObjectRef::tagged(crate::tag::CompilerReferenceTag::It.key()),
            ),
            _ => PlayerFilter::target_player(),
        });
    }

    let count_value = choice_count_value(parsed.count_source, &clause_words)?;

    Ok(Some((chooser, choose_filter, parsed.count, count_value)))
}

fn choice_count_value(
    count_source: Option<ChoiceObjectCountSource>,
    clause_words: &[&str],
) -> Result<Option<Value>, CardTextError> {
    match count_source {
        Some(ChoiceObjectCountSource::CardsDiscardedThisWay) => Ok(Some(Value::Count(
            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key()),
        ))),
        Some(ChoiceObjectCountSource::ThatMany) => Ok(Some(Value::EventValue(
            crate::effect::EventValueSpec::Amount,
        ))),
        Some(ChoiceObjectCountSource::ForEach(count_words)) => {
            let count_word_refs = count_words.iter().map(String::as_str).collect::<Vec<_>>();
            let Some((value, consumed)) =
                crate::util::parse_for_each_count_value_words(&count_word_refs)
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported for-each object-choice count (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            if consumed != count_word_refs.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing words in object-choice count (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            Ok(Some(value.with_surface_hint(
                ironsmith_core::ValueSurfaceHint::ForEach,
            )))
        }
        None => Ok(None),
    }
}

pub fn parse_you_choose_objects_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    Ok(parse_you_choose_objects_clause_with_count_value(tokens)?
        .map(|(chooser, filter, count, _count_value)| (chooser, filter, count)))
}

pub fn parse_you_choose_objects_clause_with_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount, Option<Value>)>, CardTextError> {
    let trimmed_tokens = trim_edge_punctuation(tokens);
    let tokens = trimmed_tokens.as_slice();
    let clause_words = crate::lexer::parser_token_word_refs(tokens);
    let parsed = match parse_typed_choice_object_clause_tokens(tokens) {
        Ok(Some(TypedChoiceObjectClauseKind::Object(parsed))) => parsed,
        Ok(Some(TypedChoiceObjectClauseKind::CardName)) | Ok(None) => return Ok(None),
        Err(ChoiceObjectClauseSyntaxError::MissingObject) => {
            return Err(CardTextError::ParseError(format!(
                "missing chosen object after choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        Err(ChoiceObjectClauseSyntaxError::MissingFilter) => {
            return Err(CardTextError::ParseError(format!(
                "missing chosen object filter in choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        Err(ChoiceObjectClauseSyntaxError::UnsupportedFilter) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported chosen object filter in choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };
    let references_it = parsed.references.references_it;
    let count_value = choice_count_value(parsed.count_source, &clause_words)?;
    let mut choose_filter = parsed.filter;
    let chooser = match parsed.actor {
        // Preserve the implicit actor until the enclosing sentence shape is
        // known. At top level lowering resolves it to you; inside
        // `Each player/opponent chooses ...`, the participant loop binds it
        // to the iterated player.
        ChoiceClauseActor::Implicit => PlayerAst::Implicit,
        ChoiceClauseActor::You => PlayerAst::You,
        ChoiceClauseActor::Opponent => PlayerAst::Opponent,
    };

    if references_it
        && !choose_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        })
    {
        choose_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::It.key(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
    }

    if !references_it
        && chooser == PlayerAst::You
        && choose_filter.controller.is_none()
        && choose_filter.owner.is_none()
        && choose_filter.could_be_targeted_by.is_none()
    {
        choose_filter.controller = Some(PlayerFilter::You);
    }

    Ok(Some((chooser, choose_filter, parsed.count, count_value)))
}

pub fn parse_you_choose_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, PlayerFilter, bool, usize)>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let trimmed_tokens = trim_edge_punctuation(tokens);
    let parsed = match parse_choice_player_clause_tokens(&trimmed_tokens) {
        Ok(Some(parsed)) => parsed,
        Ok(None) => return Ok(None),
        Err(ChoicePlayerClauseSyntaxError::UnsupportedFilter) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported chosen player filter in choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };

    Ok(Some((
        PlayerAst::You,
        parsed.filter,
        parsed.random,
        parsed.exclude_previous_choices,
    )))
}

pub fn parse_target_player_chooses_then_other_cant_block(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((chooser, mut choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(first)?
    else {
        return Ok(None);
    };
    if choose_filter.card_types.is_empty() {
        choose_filter.card_types.push(CardType::Creature);
    }

    let second_words = crate::lexer::token_word_refs(second);
    let shape = match parse_typed_chosen_cant_block_tokens(second) {
        Ok(Some(shape)) => shape,
        Ok(None) => return Ok(None),
        Err(ChosenCantBlockSyntaxError::MissingSubject) => {
            return Err(CardTextError::ParseError(format!(
                "missing subject in cant-block clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
        Err(ChosenCantBlockSyntaxError::MissingObjectFilter) => {
            return Err(CardTextError::ParseError(format!(
                "missing object phrase in cant-block clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
        Err(ChosenCantBlockSyntaxError::UnsupportedObjectFilter) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported cant-block subject filter (clause: '{}')",
                second_words.join(" ")
            )));
        }
    };

    let mut restriction_filter = shape.filter;
    if restriction_filter.card_types.is_empty() {
        restriction_filter.card_types.push(CardType::Creature);
    }
    if restriction_filter.controller.is_none() {
        restriction_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            _ => PlayerFilter::target_player(),
        });
    }
    if shape.exclude_tagged_choice
        && !restriction_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                    && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            })
    {
        restriction_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::It.key(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: crate::tag::CompilerReferenceTag::It.key(),
        },
        EffectAst::subject_verb_cant(
            crate::effect::Restriction::block(restriction_filter),
            Until::EndOfTurn,
            None,
        ),
    ]))
}

#[cfg(test)]
#[path = "choice_object_clauses_inline_tests.rs"]
mod tests;

pub fn parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_words = crate::lexer::token_word_refs(first);
    let second_words = crate::lexer::token_word_refs(second);
    let Some(shape) = parse_choice_card_type_reveal_shape_words(&first_words, &second_words) else {
        return Ok(None);
    };

    Ok(Some(vec![
        compose_reveal_top_choose_card_type_put_to_hand_rest_bottom(first, shape.count),
    ]))
}

/// Composes the "choose a card type, reveal the top N, put all of that type into
/// your hand and the rest on the bottom" effect as a player modal
/// (`EffectAst::ChooseOneOf`) over the nine card types, mirroring the runtime
/// `Effect::choose_one` the retired `RevealTopChooseCardTypePutToHandRestBottom`
/// recipe lowered to. Each mode looks at the top N cards, reveals them, and for
/// each looked card moves it to hand if it matches the mode's card type, else to
/// the bottom of the library.
fn compose_reveal_top_choose_card_type_put_to_hand_rest_bottom(
    first: &[OwnedLexToken],
    count: u32,
) -> EffectAst {
    let card_type_modes = [
        ("Artifact", CardType::Artifact),
        ("Battle", CardType::Battle),
        ("Creature", CardType::Creature),
        ("Enchantment", CardType::Enchantment),
        ("Instant", CardType::Instant),
        ("Kindred", CardType::Kindred),
        ("Land", CardType::Land),
        ("Planeswalker", CardType::Planeswalker),
        ("Sorcery", CardType::Sorcery),
    ];

    let modes = card_type_modes
        .into_iter()
        .map(|(label, card_type)| {
            let looked_tag =
                crate::util::helper_tag_for_tokens(first, &format!("revealed_{label}"));
            let mut card_type_filter = ObjectFilter::default();
            card_type_filter.card_types.push(card_type);

            let effects = vec![
                EffectAst::subject_verb_look_at_top_cards(
                    PlayerAst::You,
                    Value::Fixed(count as i32),
                    looked_tag.clone(),
                ),
                EffectAst::subject_verb_reveal_tagged(looked_tag.clone()),
                EffectAst::ForEachTagged {
                    tag: looked_tag,
                    effects: vec![EffectAst::Conditional {
                        predicate: PredicateAst::TaggedMatches(
                            crate::tag::CompilerReferenceTag::It.key(),
                            card_type_filter,
                        ),
                        if_true: vec![EffectAst::subject_verb_move_to_zone(
                            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
                            Zone::Hand,
                            false,
                            ReturnControllerAst::Preserve,
                            false,
                            None,
                        )],
                        if_false: vec![EffectAst::subject_verb_move_to_zone(
                            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
                            Zone::Library,
                            false,
                            ReturnControllerAst::Preserve,
                            false,
                            None,
                        )],
                    }],
                },
            ];

            crate::cards::builders::ChooseOneModeAst {
                description: label.to_string(),
                effects,
            }
        })
        .collect();

    EffectAst::ChooseOneOf { modes }
}

pub fn parse_choose_creature_type_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Vec<Subtype>)>, CardTextError> {
    match parse_typed_choice_creature_type_phrase_words(words) {
        Ok(Some(parsed)) => Ok(Some((parsed.consumed, parsed.excluded_subtypes))),
        Ok(None) => Ok(None),
        Err(ChoiceTypePhraseSyntaxError::MissingCreatureSubtypeExclusion) => {
            Err(CardTextError::ParseError(format!(
                "missing creature subtype exclusion in creature-type choice clause (clause: '{}')",
                words.join(" ")
            )))
        }
        Err(ChoiceTypePhraseSyntaxError::UnsupportedCreatureSubtypeExclusion) => {
            Err(CardTextError::ParseError(format!(
                "unsupported creature subtype exclusion in creature-type choice clause (clause: '{}')",
                words.join(" ")
            )))
        }
        Err(
            ChoiceTypePhraseSyntaxError::MissingColorExclusion
            | ChoiceTypePhraseSyntaxError::UnsupportedColorExclusion,
        ) => Ok(None),
    }
}

pub fn parse_choose_color_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Option<ColorSet>)>, CardTextError> {
    match parse_typed_choice_color_phrase_words(words) {
        Ok(Some(parsed)) => Ok(Some((parsed.consumed, parsed.excluded))),
        Ok(None) => Ok(None),
        Err(ChoiceTypePhraseSyntaxError::MissingColorExclusion) => {
            Err(CardTextError::ParseError(format!(
                "missing color exclusion in choose-color clause (clause: '{}')",
                words.join(" ")
            )))
        }
        Err(ChoiceTypePhraseSyntaxError::UnsupportedColorExclusion) => {
            Err(CardTextError::ParseError(format!(
                "unsupported color exclusion in choose-color clause (clause: '{}')",
                words.join(" ")
            )))
        }
        Err(
            ChoiceTypePhraseSyntaxError::MissingCreatureSubtypeExclusion
            | ChoiceTypePhraseSyntaxError::UnsupportedCreatureSubtypeExclusion,
        ) => Ok(None),
    }
}

pub fn parse_choose_card_type_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Vec<CardType>)>, CardTextError> {
    Ok(parse_typed_choice_card_type_phrase_words(words)
        .map(|parsed| (parsed.consumed, parsed.options)))
}

pub fn parse_choose_player_phrase_words(words: &[&str]) -> Option<usize> {
    parse_typed_choice_player_phrase_words(words).map(|parsed| parsed.consumed)
}

pub fn parse_choose_basic_land_type_phrase_words(words: &[&str]) -> Option<usize> {
    parse_choice_basic_land_type_phrase_words(words).map(|parsed| parsed.consumed)
}

pub fn parse_choose_land_type_phrase_words(words: &[&str]) -> Option<usize> {
    parse_typed_choice_land_type_phrase_words(words).map(|parsed| parsed.consumed)
}

pub fn parse_choose_creature_type_then_become_type(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_words = crate::lexer::token_word_refs(first);
    let second_words = crate::lexer::token_word_refs(second);
    let shape = match parse_typed_choice_become_shape(first, second) {
        Ok(Some(shape)) => shape,
        Ok(None) => return Ok(None),
        Err(ChoiceBecomeSyntaxError::MissingCreatureSubtypeExclusion) => {
            return Err(CardTextError::ParseError(format!(
                "missing creature subtype exclusion in creature-type choice clause (clause: '{}')",
                first_words.join(" ")
            )));
        }
        Err(ChoiceBecomeSyntaxError::UnsupportedCreatureSubtypeExclusion) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported creature subtype exclusion in creature-type choice clause (clause: '{}')",
                first_words.join(" ")
            )));
        }
        Err(ChoiceBecomeSyntaxError::UnsupportedCreatureTypeClause) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported creature-type choice clause (clause: '{}')",
                first_words.join(" ")
            )));
        }
        Err(ChoiceBecomeSyntaxError::UnsupportedBasicLandTypeClause) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported basic-land-type choice clause (clause: '{}')",
                first_words.join(" ")
            )));
        }
        Err(ChoiceBecomeSyntaxError::MissingSubject) => {
            return Err(CardTextError::ParseError(format!(
                "missing target in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
        Err(ChoiceBecomeSyntaxError::MissingObjectFilter) => {
            return Err(CardTextError::ParseError(format!(
                "missing object filter in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
        Err(ChoiceBecomeSyntaxError::UnsupportedObjectFilter) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported object filter in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
    };

    let (duration, become_tokens) =
        if let Some((duration, remainder)) = parse_restriction_duration(shape.tail_tokens)? {
            (duration, remainder)
        } else {
            (Until::Forever, shape.tail_tokens.to_vec())
        };
    if !parse_that_type_tokens(&become_tokens) {
        return Ok(None);
    }

    let target = match shape.subject {
        TypedChoiceBecomeSubject::AllObjects(filter) => TargetAst::Object(filter, None, None),
        TypedChoiceBecomeSubject::Target(subject_tokens) => parse_target_phrase(subject_tokens)?,
    };

    let effect = match shape.kind {
        ChoiceBecomeKind::CreatureType { excluded_subtypes } => {
            EffectAst::subject_verb_become_creature_type_choice(target, duration, excluded_subtypes)
        }
        ChoiceBecomeKind::BasicLandType => {
            EffectAst::subject_verb_become_basic_land_type_choice(target, duration)
        }
    };

    Ok(Some(vec![effect]))
}

pub fn parse_sentence_target_player_chooses_then_puts_on_top_of_library(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = parse_choice_library_move_shape(tokens) else {
        return Ok(None);
    };

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(shape.first_clause)?
    else {
        return Ok(None);
    };

    let target = if shape.moved_is_tagged_choice {
        TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.key(),
            span_from_tokens(shape.second_clause),
        )
    } else {
        parse_target_phrase(shape.moved_tokens)?
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: crate::tag::CompilerReferenceTag::It.key(),
        },
        EffectAst::subject_verb_move_to_zone(
            target,
            Zone::Library,
            true,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ]))
}

pub fn parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = parse_choice_battlefield_move_shape(tokens) else {
        return Ok(None);
    };

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(shape.first_clause)?
    else {
        return Ok(None);
    };
    let battlefield_controller = match shape.controller {
        ChoiceBattlefieldController::Preserve => ReturnControllerAst::Preserve,
        ChoiceBattlefieldController::You => ReturnControllerAst::You,
        ChoiceBattlefieldController::Owner => ReturnControllerAst::Owner,
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: crate::tag::CompilerReferenceTag::It.key(),
        },
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.key(),
                span_from_tokens(shape.second_clause),
            ),
            Zone::Battlefield,
            false,
            battlefield_controller,
            shape.tapped,
            None,
        ),
    ]))
}

#[cfg(test)]
#[path = "choice_object_clauses_inline_result_choice_tests_2.rs"]
mod result_choice_tests;

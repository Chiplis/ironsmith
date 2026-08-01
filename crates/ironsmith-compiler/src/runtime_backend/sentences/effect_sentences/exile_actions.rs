use super::*;
use crate::CardType;
use crate::runtime_backend::front_end::grammar::effects as effect_grammar;
use crate::runtime_backend::front_end::grammar::effects::control_copy_attach_shapes as cca_shapes;
use crate::runtime_backend::lexer::LexedClause;

pub(crate) fn parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    if let Some(shape) =
        effect_grammar::parse_each_player_exile_counted_hand_permanent_shape(tokens)
    {
        let effects = exile_iterated_hand_cards_and_permanents(
            tokens,
            crate::effect::ChoiceCount::dynamic_x(),
            Some(Value::X),
        );
        return Some(match shape.group {
            effect_grammar::EachPlayerExileGroup::Player => EffectAst::ForEachPlayer { effects },
            effect_grammar::EachPlayerExileGroup::Opponent => {
                EffectAst::ForEachOpponent { effects }
            }
        });
    }

    let shape = effect_grammar::parse_each_opponent_exile_choice_shape(tokens)?;
    parse_exile_card_from_their_hand_or_permanent_they_control(
        &shape.choice,
        Some(SubjectAst::Player(PlayerAst::Opponent)),
    )
}

fn exile_iterated_hand_cards_and_permanents(
    tokens: &[OwnedLexToken],
    count: crate::effect::ChoiceCount,
    count_value: Option<Value>,
) -> Vec<EffectAst> {
    let mut hand_card = ObjectFilter::default().in_zone(Zone::Hand);
    hand_card.owner = Some(PlayerFilter::IteratedPlayer);
    let mut permanent = ObjectFilter::permanent_card().in_zone(Zone::Battlefield);
    permanent.controller = Some(PlayerFilter::IteratedPlayer);

    let mut filter = ObjectFilter::default();
    filter.any_of = vec![hand_card, permanent];
    let tag = helper_tag_for_tokens(tokens, "exiled");
    vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value,
            player: PlayerAst::That,
            tag: tag.clone(),
            zones: vec![Zone::Hand, Zone::Battlefield],
            search_mode: None,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(tag, None), false),
    ]
}

fn parse_exile_card_from_their_hand_or_permanent_they_control(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    if !effect_grammar::is_exile_hand_or_permanent_choice_shape(tokens) {
        return None;
    }

    let wrap_for_each_opponent = matches!(subject, Some(SubjectAst::Player(PlayerAst::Opponent)));
    let chooser = match subject {
        Some(SubjectAst::Player(PlayerAst::That | PlayerAst::Opponent)) => PlayerAst::That,
        Some(SubjectAst::This) => PlayerAst::That,
        _ => return None,
    };

    let mut effects = exile_iterated_hand_cards_and_permanents(
        tokens,
        crate::effect::ChoiceCount::exactly(1),
        None,
    );
    if let Some(EffectAst::ChooseObjectsAcrossZones { player, .. }) = effects.first_mut() {
        *player = chooser;
    }

    Some(if wrap_for_each_opponent {
        EffectAst::ForEachOpponent { effects }
    } else {
        EffectAst::Sequence { effects }
    })
}

pub(crate) use effect_grammar::ParsedExileOwnerPrefix as ParsedOwnerPrefix;

fn with_exile_actor(mut effect: EffectAst, subject: Option<SubjectAst>) -> EffectAst {
    if let Some(SubjectAst::Player(player)) = subject
        && let EffectAst::SubjectVerb(subject_verb) = &mut effect
    {
        subject_verb.subject.player = player;
    }
    effect
}

fn strip_source_top_only_prefix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    use winnow::Parser as _;

    crate::runtime_backend::grammar::primitives::parse_prefix(
        tokens,
        crate::runtime_backend::grammar::primitives::phrase(&["the", "top"]).void(),
    )
    .map(|(_, rest)| (rest, true))
    .unwrap_or((tokens, false))
}

/// Parse an authored pair such as "a Human you control and an artifact you
/// control" as two independent selections. Repeated indefinite articles are
/// the semantic boundary: without them, `and` can still be joining
/// characteristics inside one object filter.
fn parse_independent_exile_pair(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.iter().filter(|word| **word == "and").count() != 1 {
        return Ok(None);
    }
    let Some((first_tokens, second_tokens)) =
        crate::runtime_backend::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            crate::runtime_backend::grammar::primitives::kw("and").void()
        })
    else {
        return Ok(None);
    };
    let starts_with_indefinite_article = |branch: &[OwnedLexToken]| {
        crate::runtime_backend::token_word_refs(branch)
            .first()
            .is_some_and(|word| matches!(*word, "a" | "an"))
    };
    if !starts_with_indefinite_article(first_tokens)
        || !starts_with_indefinite_article(second_tokens)
    {
        return Ok(None);
    }

    let mut first = parse_target_phrase(first_tokens)?;
    let mut second = parse_target_phrase(second_tokens)?;
    let is_non_target_object = |target: &TargetAst| match target {
        TargetAst::Object(_, explicit_target_span, _) => explicit_target_span.is_none(),
        TargetAst::WithCount(inner, count) if count.is_single() => {
            matches!(
                inner.as_ref(),
                TargetAst::Object(_, explicit_target_span, _)
                    if explicit_target_span.is_none()
            )
        }
        _ => false,
    };
    if !is_non_target_object(&first) || !is_non_target_object(&second) {
        return Ok(None);
    }

    apply_exile_subject_hand_owner_context(&mut first, subject.clone());
    apply_exile_subject_hand_owner_context(&mut second, subject);
    let exile = |target| {
        if until_source_leaves {
            EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
        } else {
            EffectAst::subject_verb_exile(target, face_down)
        }
    };
    Ok(Some(EffectAst::Coordinated {
        effects: vec![exile(first), exile(second)],
        leading_duration: false,
        result_conjunction: false,
    }))
}

pub(crate) fn parse_exile(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    if let Some((target_tokens, leave_watcher_tokens)) = split_until_target_leaves_tail(tokens) {
        let (target_tokens, face_down) = split_exile_face_down_suffix(target_tokens);
        let mut target = parse_target_phrase(target_tokens)?;
        apply_exile_subject_hand_owner_context(&mut target, subject);
        let leave_watcher = parse_target_phrase(leave_watcher_tokens)?;
        return Ok(EffectAst::subject_verb_exile_until_target_leaves(
            target,
            leave_watcher,
            face_down,
        ));
    }

    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let (tokens, face_down) = split_exile_face_down_suffix(tokens);
    let tokens = split_exile_graveyard_replacement_suffix(tokens);
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::contains_word(tokens, "unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported exile-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_face_down_manifest_tail = (grammar::contains_word(tokens, "face-down")
        || grammar::contains_word(tokens, "facedown")
        || grammar::contains_word(tokens, "manifest")
        || grammar::contains_word(tokens, "pile"))
        && grammar::contains_word(tokens, "then");
    if has_face_down_manifest_tail {
        return Err(CardTextError::ParseError(format!(
            "unsupported face-down/manifest exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) = parse_same_name_exile_hand_and_graveyard_clause(
        tokens,
        subject,
        until_source_leaves,
        face_down,
    )? {
        return Ok(effect);
    }
    if !face_down
        && !until_source_leaves
        && let Some(effect) =
            parse_exile_card_from_their_hand_or_permanent_they_control(tokens, subject)
    {
        return Ok(effect);
    }
    if let Some(shape) = effect_grammar::parse_exile_one_per_card_type_from_graveyard_shape(tokens)
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = controller_filter_for_token_player(shape.owner);
        filter.one_per_card_type = true;
        let target = TargetAst::WithCount(
            Box::new(TargetAst::Object(filter, None, None)),
            crate::effect::ChoiceCount::any_number(),
        );
        return Ok(if until_source_leaves {
            EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
        } else {
            EffectAst::subject_verb_exile(target, face_down)
        });
    }
    if let Some(effect) =
        parse_battlefield_graveyard_exile_all_pair(tokens, subject, until_source_leaves, face_down)?
    {
        return Ok(effect);
    }
    if let Some(filter_tokens) = effect_grammar::strip_exile_all_or_each_shape(tokens) {
        if let Some(effect) = parse_except_then_additional_exile_all_filter(
            filter_tokens,
            subject.clone(),
            until_source_leaves,
            face_down,
        )? {
            return Ok(effect);
        }
        let mut filter = parse_object_filter_lexed(filter_tokens, false)?;
        apply_exile_subject_owner_context(&mut filter, subject);
        return Ok(if until_source_leaves {
            EffectAst::subject_verb_exile_all_until_source_leaves(
                TargetAst::Object(filter, None, None),
                face_down,
            )
        } else {
            EffectAst::subject_verb_exile_all(filter, face_down)
        });
    }
    if let Some(filter) = parse_target_player_graveyard_filter(tokens) {
        return Ok(if until_source_leaves {
            EffectAst::subject_verb_exile_until_source_leaves(
                TargetAst::Object(filter, None, None),
                face_down,
            )
        } else {
            EffectAst::subject_verb_exile_all(filter, face_down)
        });
    }
    if let Some(effect) =
        parse_mixed_target_and_all_exile_list(tokens, subject, until_source_leaves, face_down)?
    {
        return Ok(effect);
    }
    if !until_source_leaves
        && let Some(effect) = parse_exile_bottom_library_clause(tokens, subject, face_down)
    {
        return Ok(effect);
    }
    if !face_down
        && !until_source_leaves
        && let Some(effect) = parse_exile_dynamic_count_from_top_library_clause(tokens, subject)
    {
        return Ok(effect);
    }
    if !face_down
        && !until_source_leaves
        && let Some(effect) = parse_exile_top_library_clause(tokens, subject)
    {
        return Ok(effect);
    }

    if grammar::contains_word(tokens, "dealt")
        && grammar::contains_word(tokens, "damage")
        && grammar::contains_word(tokens, "turn")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_until_total_mana_value = grammar::contains_word(tokens, "until")
        && grammar::contains_word(tokens, "exiled")
        && grammar::contains_word(tokens, "total")
        && grammar::contains_word(tokens, "mana")
        && grammar::contains_word(tokens, "value");
    if has_until_total_mana_value {
        return Err(CardTextError::ParseError(format!(
            "unsupported iterative exile-total clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) = parse_attached_object_exile_bundle(tokens, face_down)? {
        return Ok(effect);
    }
    let has_same_name_token_bundle = grammar::contains_word(tokens, "and")
        && grammar::contains_word(tokens, "tokens")
        && grammar::contains_word(tokens, "same")
        && grammar::contains_word(tokens, "name");
    if has_same_name_token_bundle {
        return Err(CardTextError::ParseError(format!(
            "unsupported same-name token exile bundle (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) =
        parse_independent_exile_pair(tokens, subject.clone(), until_source_leaves, face_down)?
    {
        return Ok(effect);
    }

    if let Some((before_and, after_and)) =
        crate::runtime_backend::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            crate::runtime_backend::grammar::primitives::kw("and").void()
        })
        && !before_and.is_empty()
    {
        let starts_multi_target = effect_grammar::starts_exile_multi_target_shape(after_and);
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target exile clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if let Some(spec) = split_trailing_if_clause_lexed(tokens) {
        let (target_tokens, source_top_only) = strip_source_top_only_prefix(spec.leading_tokens);
        if source_top_only && until_source_leaves {
            return Err(CardTextError::ParseError(
                "top-of-zone exile-until-source-leaves is not supported".to_string(),
            ));
        }
        let mut target = parse_target_phrase(target_tokens)?;
        apply_exile_subject_hand_owner_context(&mut target, subject);
        let plural_surface = cca_shapes::is_plural_tagged_object_reference(target_tokens);
        return Ok(EffectAst::TrailingIf {
            predicate: spec.predicate,
            effects: vec![with_exile_actor(
                if until_source_leaves {
                    EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
                } else {
                    EffectAst::subject_verb_exile(target, face_down)
                        .with_source_top_only(source_top_only)
                }
                .with_move_to_zone_plural_surface_if(plural_surface),
                subject,
            )],
        });
    } else if grammar::contains_word(tokens, "if") {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (target_tokens, source_top_only) = strip_source_top_only_prefix(tokens);
    if source_top_only && until_source_leaves {
        return Err(CardTextError::ParseError(
            "top-of-zone exile-until-source-leaves is not supported".to_string(),
        ));
    }
    let mut target = parse_target_phrase(target_tokens)?;
    apply_exile_subject_hand_owner_context(&mut target, subject);
    let plural_surface = cca_shapes::is_plural_tagged_object_reference(target_tokens);
    Ok(with_exile_actor(
        if until_source_leaves {
            EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
        } else {
            EffectAst::subject_verb_exile(target, face_down).with_source_top_only(source_top_only)
        }
        .with_move_to_zone_plural_surface_if(plural_surface),
        subject,
    ))
}

fn parse_attached_object_exile_bundle(
    tokens: &[OwnedLexToken],
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = effect_grammar::parse_attached_object_exile_shape(tokens) else {
        return Ok(None);
    };
    let target = parse_target_phrase(&shape.target)?;
    let mut attachment_filter = parse_object_filter_lexed(&shape.attachment_filter, false)?;
    let antecedent_surface = crate::runtime_backend::token_word_refs(&shape.target)
        .into_iter()
        .rev()
        .find_map(ironsmith_core::DemonstrativeAntecedentSurface::from_noun);
    attachment_filter.set_demonstrative_antecedent_surface(antecedent_surface);
    Ok(Some(EffectAst::subject_verb_exile_all_attached_to(
        attachment_filter,
        target,
        face_down,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Subtype;
    use crate::runtime_backend::ast::{SubjectVerbActionAst, SubjectVerbEffectAst};
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn attached_exile_bundle_retains_the_authored_antecedent_noun() {
        let tokens = lex_line("enchanted creature and all Auras attached to it", 0)
            .expect("attached exile bundle should lex");
        let effect = parse_attached_object_exile_bundle(&tokens, false)
            .expect("attached exile bundle should parse")
            .expect("attached exile bundle should be recognized");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ExileAllAttachedTo { filter, .. },
            ..
        }) = effect
        else {
            panic!("expected a typed attached-object exile bundle");
        };
        assert_eq!(
            filter.demonstrative_antecedent_surface(),
            Some(ironsmith_core::DemonstrativeAntecedentSurface::Creature)
        );
    }

    #[test]
    fn plural_demonstrative_exile_retains_the_referenced_collection() {
        let tokens = lex_line("those Auras", 0).expect("plural demonstrative should lex");
        let effect = parse_exile(&tokens, None).expect("plural demonstrative should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Exile {
                    target: TargetAst::Object(filter, ..),
                    target_plural_surface,
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected a typed object-filter exile");
        };
        assert!(filter.subtypes.contains(&Subtype::Aura));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(filter.has_plural_object_noun_surface());
        assert!(target_plural_surface);
    }

    #[test]
    fn exile_until_distinct_target_leaves_keeps_both_targets_in_order() {
        let tokens = lex_line(
            "target creature or enchantment you don't control until target enchantment you control leaves the battlefield",
            0,
        )
        .expect("clause should lex");
        let effect = parse_exile(&tokens, None).expect("clause should parse");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ExileUntilSourceLeaves {
                    target: TargetAst::Object(exiled_filter, Some(_), _),
                    leave_watcher: Some(TargetAst::Object(watcher_filter, Some(_), _)),
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected typed exile target and distinct leave watcher");
        };

        assert!(exiled_filter.card_types.contains(&CardType::Creature));
        assert!(exiled_filter.card_types.contains(&CardType::Enchantment));
        assert_eq!(watcher_filter.card_types, vec![CardType::Enchantment]);
        assert_eq!(watcher_filter.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn exile_one_per_card_type_keeps_owner_and_selection_constraint() {
        let tokens = lex_line(
            "up to one card of each card type from defending player's graveyard",
            0,
        )
        .expect("clause should lex");
        let effect = parse_exile(&tokens, None).expect("clause should parse");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Exile {
                    target: TargetAst::WithCount(target, count),
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected counted exile");
        };
        let TargetAst::Object(filter, None, None) = target.as_ref() else {
            panic!("expected graveyard object filter");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::Defending));
        assert!(filter.one_per_card_type);
        assert_eq!(count, crate::effect::ChoiceCount::any_number());
    }
}

pub(crate) fn parse_same_name_exile_hand_and_graveyard_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = effect_grammar::parse_same_name_hand_graveyard_exile_shape(tokens) else {
        return Ok(None);
    };
    let owner_from_subject = match subject {
        Some(SubjectAst::Player(player)) => controller_filter_for_token_player(player),
        _ => None,
    };
    let owner = match shape.owner {
        effect_grammar::SameNameExileOwnerShape::TargetPlayer => {
            Some(PlayerFilter::target_player())
        }
        effect_grammar::SameNameExileOwnerShape::TargetOpponent => {
            Some(PlayerFilter::target_opponent())
        }
        effect_grammar::SameNameExileOwnerShape::ThatPlayer => Some(PlayerFilter::IteratedPlayer),
        effect_grammar::SameNameExileOwnerShape::You => Some(PlayerFilter::You),
        effect_grammar::SameNameExileOwnerShape::TheirOrHisHer => {
            owner_from_subject.or(Some(PlayerFilter::IteratedPlayer))
        }
        effect_grammar::SameNameExileOwnerShape::FromSubject => owner_from_subject,
    };
    let Some(owner) = owner else {
        return Ok(None);
    };

    let mut filter = ObjectFilter::default();
    filter.owner = Some(owner);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    filter.any_of = [Zone::Hand, Zone::Graveyard]
        .into_iter()
        .map(|zone| ObjectFilter::default().in_zone(zone))
        .collect();

    Ok(Some(if until_source_leaves {
        EffectAst::subject_verb_exile_until_source_leaves(
            TargetAst::Object(filter, None, None),
            face_down,
        )
    } else {
        EffectAst::subject_verb_exile_all(filter, face_down)
    }))
}

fn strip_exile_list_segment_leading_conjunction(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let trimmed = trim_commas(tokens);
    if trimmed
        .first()
        .is_some_and(|token| token.as_word().is_some_and(|word| word == "and"))
    {
        trim_commas(&trimmed[1..])
    } else {
        trimmed
    }
}

fn split_exile_all_list_tail_segment(tokens: Vec<OwnedLexToken>) -> Vec<Vec<OwnedLexToken>> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;
    while idx + 1 < tokens.len() {
        let is_and = tokens[idx].as_word().is_some_and(|word| word == "and");
        let next_starts_all_clause =
            effect_grammar::strip_exile_all_or_each_shape(&tokens[idx + 1..]).is_some();
        if is_and && next_starts_all_clause {
            let part = trim_commas(&tokens[start..idx]);
            if !part.is_empty() {
                parts.push(part);
            }
            start = idx + 1;
            idx += 1;
        }
        idx += 1;
    }
    let part = trim_commas(&tokens[start..]);
    if !part.is_empty() {
        parts.push(part);
    }
    parts
}

fn parse_except_then_additional_exile_all_filter(
    filter_tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::runtime_backend::grammar::primitives as grammar;
    use crate::target::ObjectCharacteristicRelationKind;
    use winnow::Parser as _;

    let segments = grammar::split_lexed_slices_on_comma(filter_tokens);
    if segments.len() < 2 {
        return Ok(None);
    }
    let first = trim_commas(segments[0]);
    let Some((base_tokens, exception_tokens)) =
        grammar::split_lexed_once_on_separator(&first, || grammar::kw("except").void())
    else {
        return Ok(None);
    };
    let base_tokens = trim_commas(base_tokens);
    let mut exception_tokens = trim_commas(exception_tokens);
    if let Some(stripped) = grammar::strip_lexed_prefix_phrase(&exception_tokens, &["for"]) {
        exception_tokens = trim_commas(stripped);
    }
    if base_tokens.is_empty() || exception_tokens.is_empty() {
        return Ok(None);
    }

    let mut additional_segments = Vec::new();
    for segment in segments.iter().skip(1) {
        let segment = strip_exile_list_segment_leading_conjunction(segment);
        for segment in split_exile_all_list_tail_segment(segment) {
            let Some(tokens) = effect_grammar::strip_exile_all_or_each_shape(&segment) else {
                return Ok(None);
            };
            let tokens = trim_commas(tokens);
            if tokens.is_empty() {
                return Ok(None);
            }
            additional_segments.push(tokens);
        }
    }
    if additional_segments.is_empty() {
        return Ok(None);
    }

    let mut base = parse_object_filter_lexed(&base_tokens, false)?;
    let mut exception = parse_object_filter_lexed(&exception_tokens, false)?;
    if exception.characteristic_relations.is_empty() {
        return Ok(None);
    }
    let mut inverse_relations = std::mem::take(&mut exception.characteristic_relations);
    if exception != ObjectFilter::default() {
        return Ok(None);
    }
    for relation in &mut inverse_relations {
        relation.kind = match relation.kind {
            ObjectCharacteristicRelationKind::SharesAny => {
                ObjectCharacteristicRelationKind::SharesNone
            }
            ObjectCharacteristicRelationKind::SharesNone => {
                ObjectCharacteristicRelationKind::SharesAny
            }
        };
    }
    base.characteristic_relations.extend(inverse_relations);
    apply_exile_subject_owner_context(&mut base, subject.clone());

    let mut branches = vec![base];
    for segment in additional_segments {
        let mut filter = parse_object_filter_lexed(&segment, false)?;
        apply_exile_subject_owner_context(&mut filter, subject.clone());
        branches.push(filter);
    }
    let mut union = ObjectFilter::default();
    union.any_of = branches;

    Ok(Some(if until_source_leaves {
        EffectAst::subject_verb_exile_all_until_source_leaves(
            TargetAst::Object(union, None, None),
            face_down,
        )
    } else {
        EffectAst::subject_verb_exile_all(union, face_down)
    }))
}

fn parse_battlefield_graveyard_exile_all_pair(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let segments = split_exile_all_list_tail_segment(tokens.to_vec());
    let [first, second] = segments.as_slice() else {
        return Ok(None);
    };
    let mut filters = Vec::with_capacity(2);
    for segment in [first, second] {
        let Some(filter_tokens) = effect_grammar::strip_exile_all_or_each_shape(segment) else {
            return Ok(None);
        };
        let mut filter = parse_object_filter_lexed(filter_tokens, false)?;
        apply_exile_subject_owner_context(&mut filter, subject.clone());
        filters.push(filter);
    }

    let is_creature_planeswalker_limit = |filter: &ObjectFilter| {
        filter.card_types.len() == 2
            && filter.card_types.contains(&CardType::Creature)
            && filter.card_types.contains(&CardType::Planeswalker)
            && filter.mana_value.is_some()
    };
    if !filters.iter().all(is_creature_planeswalker_limit)
        || !matches!(
            (filters[0].zone, filters[1].zone),
            (Some(Zone::Battlefield), Some(Zone::Graveyard))
                | (Some(Zone::Graveyard), Some(Zone::Battlefield))
        )
        || filters[0].mana_value != filters[1].mana_value
    {
        return Ok(None);
    }

    let effects = filters
        .into_iter()
        .map(|filter| {
            if until_source_leaves {
                EffectAst::subject_verb_exile_all_until_source_leaves(
                    TargetAst::Object(filter, None, None),
                    face_down,
                )
            } else {
                EffectAst::subject_verb_exile_all(filter, face_down)
            }
        })
        .collect();
    Ok(Some(EffectAst::Sequence { effects }))
}

fn parse_mixed_target_and_all_exile_list(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    until_source_leaves: bool,
    face_down: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let segments = crate::runtime_backend::grammar::primitives::split_lexed_slices_on_comma(tokens);
    if segments.len() < 2 {
        return Ok(None);
    }
    let first_segment = trim_commas(segments[0]);
    if first_segment.is_empty() {
        return Ok(None);
    }
    let mut all_segments = Vec::new();
    for segment in segments.iter().skip(1) {
        let segment = strip_exile_list_segment_leading_conjunction(segment);
        for segment in split_exile_all_list_tail_segment(segment) {
            if segment.is_empty() {
                return Ok(None);
            }
            if effect_grammar::strip_exile_all_or_each_shape(&segment).is_none() {
                return Ok(None);
            }
            all_segments.push(segment);
        }
    }

    let mut effects = Vec::new();
    let mut target = parse_target_phrase(&first_segment)?;
    apply_exile_subject_hand_owner_context(&mut target, subject);
    effects.push(if until_source_leaves {
        EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
    } else {
        EffectAst::subject_verb_exile(target, face_down)
    });

    for segment in all_segments {
        let Some(filter_tokens) = effect_grammar::strip_exile_all_or_each_shape(&segment) else {
            return Ok(None);
        };
        let filter_tokens = trim_commas(filter_tokens);
        if filter_tokens.is_empty() {
            return Ok(None);
        }
        let mut filter = parse_object_filter_lexed(&filter_tokens, false)?;
        apply_exile_subject_owner_context(&mut filter, subject);
        effects.push(if until_source_leaves {
            EffectAst::subject_verb_exile_all_until_source_leaves(
                TargetAst::Object(filter, None, None),
                face_down,
            )
        } else {
            EffectAst::subject_verb_exile_all(filter, face_down)
        });
    }

    Ok(Some(EffectAst::Sequence { effects }))
}

pub(crate) fn split_exile_face_down_suffix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    let shape = effect_grammar::parse_exile_face_down_suffix_shape(tokens);
    (shape.core, shape.face_down)
}

pub(crate) fn split_exile_graveyard_replacement_suffix(
    tokens: &[OwnedLexToken],
) -> &[OwnedLexToken] {
    use crate::runtime_backend::grammar::primitives as grammar;

    let Some((main_slice, tail_slice)) = grammar::split_lexed_once_on_separator(tokens, || {
        use winnow::Parser as _;
        grammar::kw("instead").void()
    }) else {
        return tokens;
    };
    if main_slice.is_empty() {
        return tokens;
    }

    let is_graveyard_replacement =
        grammar::strip_lexed_prefix_phrase(tail_slice, &["of", "putting"]).is_some()
            && (grammar::contains_word(tail_slice, "graveyard")
                || grammar::contains_word(tail_slice, "graveyards"));
    if is_graveyard_replacement {
        main_slice
    } else {
        tokens
    }
}

pub(crate) fn parse_graveyard_owner_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ParsedOwnerPrefix> {
    effect_grammar::parse_exile_graveyard_owner_shape(tokens)
}

fn parse_exile_dynamic_count_from_top_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let shape = effect_grammar::parse_exile_dynamic_top_library_shape(tokens, default_player)?;
    let effect_grammar::ExileLibraryPlayerShape::Player(player) = shape.player else {
        return None;
    };
    let tag_tokens = trim_commas(tokens);
    let surface = (default_player != PlayerAst::Implicit && default_player == player)
        .then_some(ironsmith_core::ExileTopLibrarySurface::LibraryOwnerAsActor);

    Some(
        EffectAst::subject_verb_exile_top_of_library_with_optional_surface(
            player,
            shape.count,
            vec![helper_tag_for_tokens(&tag_tokens, "exiled")],
            Vec::new(),
            surface,
        ),
    )
}

pub(crate) fn parse_exile_top_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let shape = effect_grammar::parse_exile_top_library_shape(tokens, default_player)?;
    let tag_tokens = trim_commas(tokens);
    match shape.player {
        effect_grammar::ExileLibraryPlayerShape::EachOpponent => Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::That,
                shape.count,
                Vec::new(),
                vec![helper_tag_for_tokens(&tag_tokens, "exiled")],
            )],
        }),
        effect_grammar::ExileLibraryPlayerShape::Player(player) => {
            let surface = (default_player != PlayerAst::Implicit && default_player == player)
                .then_some(ironsmith_core::ExileTopLibrarySurface::LibraryOwnerAsActor);
            Some(
                EffectAst::subject_verb_exile_top_of_library_with_optional_surface(
                    player,
                    shape.count,
                    vec![helper_tag_for_tokens(&tag_tokens, "exiled")],
                    Vec::new(),
                    surface,
                ),
            )
        }
    }
}

fn parse_exile_bottom_library_clause(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    face_down: bool,
) -> Option<EffectAst> {
    let default_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let shape = effect_grammar::parse_exile_bottom_library_shape(tokens, default_player)?;
    let tag_tokens = trim_commas(tokens);
    let tag = helper_tag_for_tokens(&tag_tokens, "exiled");
    let mut filter = ObjectFilter::default();
    filter.zone = Some(Zone::Library);

    let choose_and_exile = |player: PlayerAst, tag: TagKey| {
        vec![
            EffectAst::ChooseObjectsBottomOfLibrary {
                filter: filter.clone(),
                count: crate::effect::ChoiceCount::exactly(1),
                count_value: None,
                player,
                tag: tag.clone(),
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(tag, None), face_down),
        ]
    };

    match shape.player {
        effect_grammar::ExileLibraryPlayerShape::EachOpponent => Some(EffectAst::ForEachOpponent {
            effects: choose_and_exile(PlayerAst::That, tag),
        }),
        effect_grammar::ExileLibraryPlayerShape::Player(player) => Some(EffectAst::Sequence {
            effects: choose_and_exile(player, tag),
        }),
    }
}

pub(crate) fn parse_target_player_graveyard_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let tokens = trim_commas(tokens);
    let owner = parse_graveyard_owner_prefix_lexed(&tokens)?;
    if owner.consumed_words != LexedClause::new(&tokens).word_refs().len() {
        return None;
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    filter.owner = match owner.player {
        PlayerAst::You => Some(PlayerFilter::You),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent))),
        PlayerAst::ItsController => Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::tagged("triggering"),
        )),
        PlayerAst::ItsOwner => Some(PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(
            "triggering",
        ))),
        _ => None,
    };
    filter.owner.as_ref()?;
    Some(filter)
}

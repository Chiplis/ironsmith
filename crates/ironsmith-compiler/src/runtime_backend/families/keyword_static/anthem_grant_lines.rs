use crate::ability::{PresentationKeyword, PresentationLabel};
use crate::host::{EffectAst, TriggerSpec};

type AnthemNormalizedWords<'a> = crate::runtime_backend::grammar::primitives::TokenWordView<'a>;

fn leading_set_quantifier_surface(
    subject_tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::SetQuantifierSurface> {
    match subject_tokens.first().and_then(OwnedLexToken::as_word) {
        Some("each") => Some(ironsmith_core::SetQuantifierSurface::Each),
        Some("all") => Some(ironsmith_core::SetQuantifierSurface::All),
        _ => None,
    }
}

fn with_leading_set_quantifier_surface(
    ability: StaticAbilityAst,
    subject_tokens: &[OwnedLexToken],
) -> StaticAbilityAst {
    if !matches!(
        &ability,
        StaticAbilityAst::GrantStaticAbility { .. }
            | StaticAbilityAst::GrantKeywordAction { .. }
            | StaticAbilityAst::GrantObjectAbility { .. }
    ) {
        return ability;
    }
    let Some(surface) = leading_set_quantifier_surface(subject_tokens) else {
        return ability;
    };
    StaticAbilityAst::WithSetQuantifierSurface {
        ability: Box::new(ability),
        surface,
    }
}

fn first_spell_each_turn_subject(filter_tokens: &[OwnedLexToken]) -> Option<AnthemSubjectAst> {
    anthem_grant_grammar::parse_first_spell_each_turn_subject_tokens(filter_tokens).map(|_| {
        AnthemSubjectAst::Filter(
            ObjectFilter::spell()
                .cast_by(PlayerFilter::You)
                .first_spell_cast_each_turn(),
        )
    })
}
fn first_spell_each_turn_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<AnthemSubjectAst>, CardTextError> {
    let Some(parsed) = anthem_grant_grammar::parse_first_spell_each_turn_clause(tokens) else {
        return Ok(None);
    };

    let mut filter = parse_object_filter_lexed(parsed.filter_tokens, false)?;
    if !filter.has_mana_cost
        && filter.stack_kind != Some(crate::filter::StackObjectKind::Spell)
        && filter.zone != Some(Zone::Stack)
    {
        return Ok(None);
    }
    filter.cast_by = Some(PlayerFilter::You);
    filter.first_spell_cast_each_turn = true;
    Ok(Some(AnthemSubjectAst::Filter(filter)))
}

fn parse_cant_be_blocked_as_long_as_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::CantBeBlockedAsLongAsClause<'_>> {
    anthem_grant_grammar::parse_cant_be_blocked_as_long_as_clause(tokens)
}

fn parse_cant_be_blocked_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::CantBeBlockedClause<'_>> {
    anthem_grant_grammar::parse_cant_be_blocked_clause(tokens)
}

fn parse_keywords_and_cant_be_blocked_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::KeywordsAndCantBeBlockedClause<'_>> {
    anthem_grant_grammar::parse_keywords_and_cant_be_blocked_clause(tokens)
}

fn parse_keywords_and_cant_be_blocked_by_more_than_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::KeywordsAndCantBeBlockedByMoreThanClause<'_>> {
    anthem_grant_grammar::parse_keywords_and_cant_be_blocked_by_more_than_clause(tokens)
}

fn parse_cant_be_blocked_and_has_keywords_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::CantBeBlockedAndHasKeywordsClause<'_>> {
    anthem_grant_grammar::parse_cant_be_blocked_and_has_keywords_clause(tokens)
}

fn parse_landwalk_block_override_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::LandwalkBlockOverrideClause<'_>> {
    anthem_grant_grammar::parse_landwalk_block_override_clause(tokens)
}

fn parse_granted_escape_cost_tail_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::GrantedEscapeCostTail<'_>> {
    anthem_grant_grammar::parse_granted_escape_cost_tail_clause(tokens)
}

fn parse_granted_miracle_cost_reduction_tail_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::GrantedMiracleCostReductionTail<'_>> {
    anthem_grant_grammar::parse_granted_miracle_cost_reduction_tail_clause(tokens)
}

fn parse_cant_be_blocked_by_more_than_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::CantBeBlockedByMoreThanClause<'_>> {
    anthem_grant_grammar::parse_cant_be_blocked_by_more_than_clause(tokens)
}

fn parse_can_block_additional_creature_clause(
    tokens: &[OwnedLexToken],
) -> Option<anthem_grant_grammar::CanBlockAdditionalCreatureClause<'_>> {
    anthem_grant_grammar::parse_can_block_additional_creature_clause(tokens)
}

fn triggered_grant_effects_and_condition(
    trigger: &TriggerSpec,
    effects: &[EffectAst],
) -> Result<(Vec<EffectAst>, Option<crate::ConditionExpr>), CardTextError> {
    if let [
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        },
    ] = effects
        && if_false.is_empty()
    {
        let mut imports = ReferenceImports::default();
        imports.last_player_filter =
            crate::runtime_backend::compile_support::inferred_trigger_player_filter(trigger);
        let reference_env = crate::runtime_backend::reference_model::ReferenceEnv::from_imports(
            &imports, false, false, false, None,
        );
        let condition =
            crate::runtime_backend::compile_support::compile_condition_from_predicate_ast_with_env(
                predicate,
                &reference_env,
                None,
            )?;
        return Ok((if_true.clone(), Some(condition)));
    }

    Ok((effects.to_vec(), None))
}

pub(crate) fn parse_subject_cant_be_blocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_clause(tokens) else {
        return Ok(None);
    };
    let subject_tokens = parsed.subject_tokens;
    let subject_facts = anthem_grant_grammar::parse_cant_be_blocked_subject_facts(subject_tokens);
    if subject_facts.has_conjunction_or_comma {
        return Ok(None);
    }
    if subject_facts.starts_with_source_pronoun {
        return Ok(None);
    }
    if subject_facts.has_rejected_clause_word {
        return Ok(None);
    }
    if subject_facts.mentions_power_or_toughness {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness cant-be-blocked subject (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let subject = first_spell_each_turn_subject(&subject_tokens)
        .map(Ok)
        .unwrap_or_else(|| parse_anthem_subject(&subject_tokens))?;
    let ability = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::KeywordAction(KeywordAction::Unblockable),
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter,
            action: KeywordAction::Unblockable,
            condition: None,
        },
    };
    Ok(Some(with_leading_set_quantifier_surface(
        ability,
        &subject_tokens,
    )))
}

pub(crate) fn parse_subject_has_keywords_and_cant_be_blocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let Some(head) = anthem_grant_grammar::parse_has_keyword_unblockable_head(tokens) else {
        return Ok(None);
    };
    let has_idx = head.has_token;

    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    let Some(parsed_tail) = parse_keywords_and_cant_be_blocked_clause(&ability_tokens) else {
        return Ok(None);
    };
    let keyword_tokens = parsed_tail.keyword_tokens;
    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    let keyword_actions = actions
        .into_iter()
        .filter(|action| action.lowers_to_static_ability())
        .collect::<Vec<_>>();
    if keyword_actions.is_empty() {
        return Ok(None);
    }

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, has_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let mut granted = Vec::new();
    for action in keyword_actions
        .into_iter()
        .chain(std::iter::once(KeywordAction::Unblockable))
    {
        granted.push(match &subject {
            AnthemSubjectAst::Source => match &condition {
                Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                    action,
                    condition: condition.clone(),
                },
                None => StaticAbilityAst::KeywordAction(action),
            },
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
                filter: filter.clone(),
                action,
                condition: condition.clone(),
            },
        });
    }

    Ok(Some(granted))
}

pub(crate) fn parse_subject_has_keywords_and_cant_be_blocked_by_more_than_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(parsed) = parse_keywords_and_cant_be_blocked_by_more_than_clause(tokens) else {
        return Ok(None);
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(actions) = parse_ability_line(parsed.keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    let keyword_actions = actions
        .into_iter()
        .filter(|action| action.lowers_to_static_ability())
        .collect::<Vec<_>>();
    if keyword_actions.is_empty() {
        return Ok(None);
    }
    let Some((minimum_blockers, used)) = parse_greater_than_or_equal_quantity_prefix(
        parsed.blocker_threshold_tokens,
        false,
        false,
        "cant-be-blocked blocker threshold",
    )?
    else {
        return Ok(None);
    };
    if minimum_blockers == 0 || used != parsed.blocker_threshold_tokens.len() {
        return Ok(None);
    }
    let maximum_blockers = usize::try_from(minimum_blockers - 1).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported cant-be-blocked blocker threshold (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let subject = match parse_anthem_subject(parsed.subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let mut granted = keyword_actions
        .into_iter()
        .map(|action| match &subject {
            AnthemSubjectAst::Source => StaticAbilityAst::KeywordAction(action),
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
                filter: filter.clone(),
                action,
                condition: None,
            },
        })
        .collect::<Vec<_>>();
    let restriction = StaticAbility::cant_be_blocked_by_more_than(maximum_blockers);
    granted.push(match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::Static(restriction),
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(restriction)),
            condition: None,
        },
    });
    Ok(Some(granted))
}

pub(crate) fn parse_subject_cant_be_blocked_and_has_keywords_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_and_has_keywords_clause(tokens) else {
        return Ok(None);
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(actions) = parse_ability_line(parsed.keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    let keyword_actions = actions
        .into_iter()
        .filter(|action| action.lowers_to_static_ability())
        .collect::<Vec<_>>();
    if keyword_actions.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(parsed.subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let mut granted = Vec::new();
    for action in std::iter::once(KeywordAction::Unblockable).chain(keyword_actions) {
        granted.push(match &subject {
            AnthemSubjectAst::Source => StaticAbilityAst::KeywordAction(action),
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
                filter: filter.clone(),
                action,
                condition: None,
            },
        });
    }
    Ok(Some(granted))
}

pub(crate) fn parse_landwalk_as_though_block_override_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_landwalk_block_override_clause(tokens) else {
        return Ok(None);
    };
    if !is_landwalk_ability_word(parsed.ability_word) {
        return Ok(None);
    }

    let AnthemSubjectAst::Filter(filter) = parse_anthem_subject(parsed.subject_tokens)? else {
        return Ok(None);
    };

    let removed = StaticAbility::keyword_marker(parsed.ability_word);
    Ok(Some(StaticAbilityAst::Static(
        StaticAbility::remove_ability(filter, removed),
    )))
}

fn is_landwalk_ability_word(word: &str) -> bool {
    matches!(
        parse_single_word_keyword_action(word),
        Some(KeywordAction::Landwalk(_))
    )
}

pub(crate) fn parse_subject_cant_be_blocked_as_long_as_condition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_as_long_as_clause(tokens) else {
        return Ok(None);
    };
    let subject_tokens = parsed.subject_tokens;
    let condition = parse_static_condition_clause(parsed.condition_tokens)?;

    let subject = first_spell_each_turn_subject(&subject_tokens)
        .map(Ok)
        .unwrap_or_else(|| parse_anthem_subject(&subject_tokens))?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalKeywordAction {
            action: KeywordAction::Unblockable,
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter,
            action: KeywordAction::Unblockable,
            condition: Some(condition),
        },
    };
    Ok(Some(with_leading_set_quantifier_surface(
        granted,
        &subject_tokens,
    )))
}

pub(crate) fn parse_subject_cant_be_blocked_while_defending_player_controls_most_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_as_long_as_clause(tokens) else {
        return Ok(None);
    };
    if !anthem_grant_grammar::parse_defending_player_controls_most_creatures_or_tied_condition(
        parsed.condition_tokens,
    ) {
        return Ok(None);
    }

    let subject = first_spell_each_turn_subject(parsed.subject_tokens)
        .map(Ok)
        .unwrap_or_else(|| parse_anthem_subject(parsed.subject_tokens))?;
    let unblockable =
        StaticAbility::cant_be_blocked_while_defending_player_controls_most_creatures();
    let ability = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::Static(unblockable),
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(unblockable)),
            condition: None,
        },
    };
    Ok(Some(ability))
}

fn simple_card_types_from_control_filter(mut filter: ObjectFilter) -> Option<Vec<CardType>> {
    let mut card_types = if filter.all_card_types.is_empty() {
        Vec::new()
    } else {
        std::mem::take(&mut filter.all_card_types)
    };

    if !filter.card_types.is_empty() {
        if card_types.is_empty() {
            card_types = std::mem::take(&mut filter.card_types);
        } else if filter.card_types.len() == card_types.len()
            && filter
                .card_types
                .iter()
                .all(|card_type| card_types.contains(card_type))
        {
            filter.card_types.clear();
        } else {
            return None;
        }
    }

    if card_types.is_empty()
        || !card_types.iter().all(|card_type| {
            matches!(
                card_type,
                CardType::Artifact
                    | CardType::Battle
                    | CardType::Creature
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
            )
        })
    {
        return None;
    }

    filter.zone = None;
    (filter == ObjectFilter::default()).then_some(card_types)
}

fn defending_player_controlled_card_types_from_condition_tokens(
    condition_tokens: &[OwnedLexToken],
) -> Option<Vec<CardType>> {
    let condition = crate::runtime_backend::grammar::conditions::parse_control_condition(
        condition_tokens,
        crate::runtime_backend::grammar::conditions::ControlConditionOptions {
            allow_that_player: false,
            allow_opponent_players: false,
            allow_defending_player: true,
            bind_filter_controller_to_subject: false,
            allow_different_powers_tail: false,
            default_filter_zone: Some(Zone::Battlefield),
        },
    )?;

    if condition.player_filter != Some(PlayerFilter::Defending)
        || condition.requires_different_powers
        || condition.at_least_count()? > 1
    {
        return None;
    }

    simple_card_types_from_control_filter(condition.filter)
}

pub(crate) fn parse_subject_cant_be_blocked_as_long_as_defending_player_controls_card_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_as_long_as_clause(tokens) else {
        return Ok(None);
    };
    let Some(card_types) =
        defending_player_controlled_card_types_from_condition_tokens(parsed.condition_tokens)
    else {
        return Ok(None);
    };

    let subject = first_spell_each_turn_subject(parsed.subject_tokens)
        .map(Ok)
        .unwrap_or_else(|| parse_anthem_subject(parsed.subject_tokens))?;
    let unblockable = if card_types.len() == 1 {
        StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_type(card_types[0])
    } else {
        StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_types(card_types)
    };
    let ability = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::Static(unblockable),
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(unblockable)),
            condition: None,
        },
    };
    Ok(Some(ability))
}

/// Split a leading "During your turn," timing prefix off a static line
/// ("During your turn, this creature is a Bear with base power and toughness
/// 4/2."). Returns the remainder after the comma.
fn split_during_your_turn_static_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (_, rest) = crate::runtime_backend::grammar::primitives::parse_prefix(
        tokens,
        crate::runtime_backend::grammar::primitives::phrase(&["during", "your", "turn"]),
    )?;
    let remainder = trim_lexed_commas(rest);
    (remainder.len() < rest.len() && !remainder.is_empty()).then_some(remainder)
}

fn parse_filtered_object_animation_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let mut timing_condition = None;
    let (condition_tokens, animation_tokens) =
        if let Some(prefix) = split_as_long_as_condition_prefix_lexed(tokens) {
            (Some(prefix.condition_tokens), prefix.remainder_tokens)
        } else if let Some(remainder) = split_during_your_turn_static_prefix_lexed(tokens) {
            timing_condition = Some(crate::ConditionExpr::ActivationTiming(
                crate::ability::ActivationTiming::DuringYourTurn,
            ));
            (None, remainder)
        } else {
            (None, tokens)
        };
    let Some(shape) =
        crate::runtime_backend::grammar::effects::become_shapes::parse_filtered_object_animation_tokens(
            animation_tokens,
        )
    else {
        return Ok(None);
    };

    let attached_subject_filter =
        condition_tokens.and_then(infer_attached_subject_filter_from_condition_tokens);
    let mut subject = if shape.dependent_subject {
        let Some(filter) = attached_subject_filter else {
            return Ok(None);
        };
        AnthemSubjectAst::Filter(filter)
    } else {
        match parse_anthem_subject(shape.subject_tokens) {
            Ok(subject) => subject,
            Err(_) => return Ok(None),
        }
    };
    let condition = condition_tokens
        .map(parse_static_condition_clause)
        .transpose()?
        .or(timing_condition)
        .map(|condition| bind_attachment_condition_to_subject(condition, &subject));

    if let AnthemSubjectAst::Filter(filter) = &mut subject {
        filter.set_set_quantifier_surface(leading_set_quantifier_surface(shape.subject_tokens));
    }
    let filter = anthem_subject_filter(&subject);
    let abilities = filtered_object_animation_abilities(filter, shape);
    Ok(Some(
        abilities
            .into_iter()
            .map(|ability| conditional_static_ability(ability, condition.clone()))
            .collect(),
    ))
}

fn filtered_object_animation_abilities(
    filter: ObjectFilter,
    shape: crate::runtime_backend::grammar::effects::become_shapes::FilteredObjectAnimationShape<
        '_,
    >,
) -> Vec<StaticAbility> {
    let mut abilities = Vec::new();
    if shape.removes_all_abilities {
        abilities.push(StaticAbility::remove_all_abilities(filter.clone()));
    }
    if !shape.descriptor.card_types.is_empty() {
        abilities.push(StaticAbility::set_card_types(
            filter.clone(),
            shape.descriptor.card_types,
        ));
    }
    if !shape.descriptor.subtypes.is_empty() {
        abilities.push(StaticAbility::set_creature_subtypes(
            filter.clone(),
            shape.descriptor.subtypes,
        ));
    }
    if let Some(colors) = shape.descriptor.colors {
        abilities.push(StaticAbility::set_colors(filter.clone(), colors));
    }
    abilities.push(StaticAbility::set_base_power_toughness_value(
        filter,
        shape.power,
        shape.toughness,
    ));

    abilities
}

/// One half of a union subject in a granted-keyword line ("you and this
/// creature have hexproof", "Dion and other Knights you control have flying").
#[derive(Debug, Clone)]
enum UnionGrantSubject {
    /// The player half of "you and <object-subject>".
    PlayerYou,
    /// The source half of "<source-reference> and <object filter>".
    Source,
    /// An ordinary object-filter half.
    Filter(ObjectFilter),
}

/// Split a granted-keyword subject union at a top-level "and".
///
/// Only unions whose LEFT half is the player ("you") or a source reference
/// ("this creature", the card's own name, "it") are recognized: those shapes
/// can never be a single object filter, so without this split the lossy
/// suffix-filter recovery silently drops the left half. Filter-and-filter
/// unions ("artifacts and creatures you control") stay with the ordinary
/// domain-union filter grammar.
fn split_union_grant_subjects(
    subject_tokens: &[OwnedLexToken],
) -> Option<(UnionGrantSubject, UnionGrantSubject)> {
    // A serial union can put the player before a comma and keep two or more
    // object-filter arms in the remainder:
    //
    //   "You, planeswalkers you control, and other creatures you control"
    //
    // Treat the whole object remainder as one (possibly compound) filter.
    // The ordinary anthem-subject parser already preserves its branch-local
    // qualifiers; recognizing the leading player here prevents the generic
    // suffix recovery from silently discarding that member.
    if subject_tokens.first().and_then(OwnedLexToken::as_word) == Some("you")
        && subject_tokens.get(1).is_some_and(OwnedLexToken::is_comma)
    {
        let right = trim_commas(&subject_tokens[2..]);
        if right.iter().any(|token| token.as_word() == Some("and")) {
            let (right_subject, losses) =
                crate::parse_loss::capture(|| parse_anthem_subject(&right));
            if let Ok(right_subject) = right_subject
                && !losses.is_lossy()
            {
                let right_subject = match right_subject {
                    AnthemSubjectAst::Source => UnionGrantSubject::Source,
                    AnthemSubjectAst::Filter(filter) => UnionGrantSubject::Filter(filter),
                };
                return Some((UnionGrantSubject::PlayerYou, right_subject));
            }
        }
    }

    for (idx, token) in subject_tokens.iter().enumerate() {
        if token.as_word() != Some("and") {
            continue;
        }
        let left = trim_commas(&subject_tokens[..idx]);
        let right = trim_commas(&subject_tokens[idx + 1..]);
        if left.is_empty() || right.is_empty() {
            continue;
        }
        let left_words = crate::runtime_backend::token_word_refs(&left);
        let left_subject = if left_words == ["you"] {
            UnionGrantSubject::PlayerYou
        } else if anthem_grant_grammar::is_source_it_subject(&left)
            || is_source_reference_words(&left_words)
        {
            UnionGrantSubject::Source
        } else {
            continue;
        };
        // The right half must parse as a clean anthem subject on its own; a
        // lossy suffix recovery here would silently drop part of the union.
        let (right_subject, losses) = crate::parse_loss::capture(|| parse_anthem_subject(&right));
        let Ok(right_subject) = right_subject else {
            continue;
        };
        if losses.is_lossy() {
            continue;
        }
        let right_subject = match right_subject {
            AnthemSubjectAst::Source => UnionGrantSubject::Source,
            AnthemSubjectAst::Filter(filter) => UnionGrantSubject::Filter(filter),
        };
        return Some((left_subject, right_subject));
    }
    None
}

fn player_you_hexproof_static() -> StaticAbility {
    StaticAbility::restriction(
        crate::effect::Restriction::be_targeted_player_from(
            PlayerFilter::You,
            ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
        ),
        "You have hexproof".to_string(),
    )
}

/// Compile "you and <object-subject> have <keywords>" / "<source-reference>
/// and <object filter> have <keywords>" as one ability per subject half, each
/// carrying the line's condition.
fn parse_union_subject_keyword_grants(
    subject_tokens: &[OwnedLexToken],
    keyword_tokens: &[OwnedLexToken],
    condition: Option<crate::ConditionExpr>,
    clause_text: &str,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some((left_subject, right_subject)) = split_union_grant_subjects(subject_tokens) else {
        return Ok(None);
    };
    let Some(actions) = parse_ability_line(keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, clause_text)?;
    if actions.is_empty() {
        return Ok(None);
    }

    let supports = |subject: &UnionGrantSubject, action: &KeywordAction| match subject {
        // The player half only models keywords with a player-level
        // restriction form ("You have hexproof").
        UnionGrantSubject::PlayerYou => matches!(action, KeywordAction::Hexproof),
        UnionGrantSubject::Source | UnionGrantSubject::Filter(_) => {
            action.lowers_to_static_ability()
        }
    };
    if !actions
        .iter()
        .all(|action| supports(&left_subject, action) && supports(&right_subject, action))
    {
        return Ok(None);
    }

    let mut compiled = Vec::new();
    for subject in [&left_subject, &right_subject] {
        for action in &actions {
            match subject {
                UnionGrantSubject::PlayerYou => compiled.push(conditional_static_ability(
                    player_you_hexproof_static(),
                    condition.clone(),
                )),
                UnionGrantSubject::Source => compiled.push(match &condition {
                    Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                        action: action.clone(),
                        condition: condition.clone(),
                    },
                    None => StaticAbilityAst::KeywordAction(action.clone()),
                }),
                UnionGrantSubject::Filter(filter) => {
                    compiled.push(StaticAbilityAst::GrantKeywordAction {
                        filter: filter.clone(),
                        action: action.clone(),
                        condition: condition.clone(),
                    })
                }
            }
        }
    }
    Ok(Some(compiled))
}

pub(crate) fn parse_granted_keyword_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    fn extract_grant_spec_from_subject(
        subject_tokens: &[OwnedLexToken],
        grantable: crate::grant::Grantable,
    ) -> Result<Option<crate::grant::GrantSpec>, CardTextError> {
        let subject = parse_anthem_subject(subject_tokens)?;
        let AnthemSubjectAst::Filter(mut filter) = subject else {
            return Ok(None);
        };
        let zone = filter.zone.unwrap_or(Zone::Battlefield);
        filter.zone = None;
        Ok(Some(crate::grant::GrantSpec::new(grantable, filter, zone)))
    }

    fn parse_granted_escape_cost_tail(
        trailing_tokens: &[OwnedLexToken],
    ) -> Result<Option<u32>, CardTextError> {
        let trailing_word_refs = crate::runtime_backend::token_word_refs(trailing_tokens);
        let Some(parsed) = parse_granted_escape_cost_tail_clause(trailing_tokens) else {
            return Ok(None);
        };

        let Some((count, used)) = parse_number(parsed.exile_count_tokens) else {
            return Err(CardTextError::ParseError(format!(
                "escape cost clause missing exile count (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        };
        if used != parsed.exile_count_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported escape cost clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        }
        Ok(Some(count as u32))
    }

    fn parse_granted_miracle_cost_reduction_tail(
        trailing_tokens: &[OwnedLexToken],
    ) -> Result<Option<u32>, CardTextError> {
        let trailing_word_refs = crate::runtime_backend::token_word_refs(trailing_tokens);
        let Some(parsed) = parse_granted_miracle_cost_reduction_tail_clause(trailing_tokens) else {
            return Ok(None);
        };

        let Some((cost, used)) =
            crate::runtime_backend::front_end::shared::util::leading_mana_cost_from_tokens(
                parsed.reduction_cost_tokens,
            )
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported miracle cost reduction clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        };
        if used != parsed.reduction_cost_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported miracle cost reduction clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        }
        let generic = cost.generic_mana_total();
        if generic == 0 || cost.mana_value() != generic {
            return Err(CardTextError::ParseError(format!(
                "unsupported miracle cost reduction clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        }
        Ok(Some(generic))
    }

    fn parse_granted_alternative_cast_static(
        subject_tokens: &[OwnedLexToken],
        keyword_tokens: &[OwnedLexToken],
        trailing_tokens: &[OwnedLexToken],
        condition: Option<crate::ConditionExpr>,
    ) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
        let spec = match anthem_grant_grammar::parse_granted_alternative_cast_keyword_tokens(
            keyword_tokens,
        ) {
            Some(anthem_grant_grammar::GrantedAlternativeCastKeyword::Flashback) => {
                if !anthem_grant_grammar::parse_granted_flashback_cost_equals_mana(trailing_tokens)
                {
                    return Ok(None);
                }
                extract_grant_spec_from_subject(
                    subject_tokens,
                    crate::grant::Grantable::flashback_from_cards_mana_cost(),
                )?
            }
            Some(anthem_grant_grammar::GrantedAlternativeCastKeyword::Blitz) => {
                if !is_granted_blitz_cost_tail(trailing_tokens) {
                    return Ok(None);
                }
                return granted_blitz_abilities_from_subject(subject_tokens, condition);
            }
            Some(anthem_grant_grammar::GrantedAlternativeCastKeyword::Emerge) => {
                if !is_granted_emerge_cost_tail(trailing_tokens) {
                    return Ok(None);
                }
                return granted_emerge_abilities_from_subject(subject_tokens, condition);
            }
            Some(anthem_grant_grammar::GrantedAlternativeCastKeyword::Miracle) => {
                let Some(reduction) = parse_granted_miracle_cost_reduction_tail(trailing_tokens)?
                else {
                    return Ok(None);
                };
                extract_grant_spec_from_subject(
                    subject_tokens,
                    crate::grant::Grantable::miracle_from_cards_mana_cost_reduced_by(reduction),
                )?
            }
            Some(anthem_grant_grammar::GrantedAlternativeCastKeyword::Escape) => {
                let Some(exile_count) = parse_granted_escape_cost_tail(trailing_tokens)? else {
                    return Ok(None);
                };
                extract_grant_spec_from_subject(
                    subject_tokens,
                    crate::grant::Grantable::escape(exile_count),
                )?
            }
            None => None,
        };

        let Some(spec) = spec else {
            return Ok(None);
        };

        let mut ability = StaticAbilityAst::Static(StaticAbility::grants(spec));
        if let Some(condition) = condition {
            ability = StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            };
        }
        Ok(Some(vec![ability]))
    }

    if let Some(abilities) = parse_filtered_object_animation_static_line(tokens)? {
        return Ok(Some(abilities));
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(verb_facts) = anthem_grant_grammar::parse_granted_keyword_verb_facts(tokens) else {
        return Ok(None);
    };
    let have_token_idx = verb_facts.have_token;
    if verb_facts.prefix_has_get {
        return Ok(None);
    }

    if verb_facts.starts_with_as_long_as {
        if !verb_facts.tail_has_have && verb_facts.tail_has_get_or_be {
            return Ok(None);
        }
    }

    let (prefix_condition, subject_start) =
        match parse_anthem_prefix_condition(tokens, have_token_idx) {
            Ok(parsed) => parsed,
            Err(_) => return Ok(None),
        };
    let subject_tokens = trim_commas(&tokens[subject_start..have_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    if anthem_grant_grammar::granted_keyword_subject_is_rejected(&subject_tokens) {
        return Ok(None);
    }

    let tail_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let mut tail_tokens = tail_tokens;
    let mut trailing_clause_tokens: Vec<OwnedLexToken> = Vec::new();
    let tail_sentences =
        crate::runtime_backend::grammar::primitives::split_lexed_slices_on_period(&tail_tokens);
    if tail_sentences.len() > 1 {
        let leading = trim_edge_punctuation(tail_sentences[0]);
        let trailing = tail_sentences[1..]
            .iter()
            .flat_map(|sentence| trim_edge_punctuation(sentence))
            .collect::<Vec<_>>();
        trailing_clause_tokens = trailing;
        tail_tokens = leading;
    }

    let mut keyword_tokens = tail_tokens.clone();
    let mut suffix_condition = None;
    if let Some(split) = anthem_grant_grammar::split_trailing_as_long_as_clause(&tail_tokens) {
        keyword_tokens = split.keyword_tokens.to_vec();
        suffix_condition = Some(parse_static_condition_clause(split.condition_tokens)?);
    }
    if keyword_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut grants_must_attack = false;
    if let Some(split) = anthem_grant_grammar::split_must_attack_keyword_tail(&keyword_tokens) {
        keyword_tokens = split.keyword_tokens.to_vec();
        grants_must_attack = true;
    }
    if keyword_tokens.is_empty() {
        return Ok(None);
    }

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple static conditions are not supported in granted-keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        (Some(cond), None) | (None, Some(cond)) => Some(cond),
        (None, None) => None,
    };

    let keyword_kind = anthem_grant_grammar::classify_granted_keyword_tokens(&keyword_tokens);
    if keyword_kind == anthem_grant_grammar::GrantedKeywordTokenKind::Blitz
        && (trailing_clause_tokens.is_empty()
            || is_granted_blitz_cost_tail(&trailing_clause_tokens))
    {
        return granted_blitz_abilities_from_subject(&subject_tokens, condition);
    }
    if keyword_kind == anthem_grant_grammar::GrantedKeywordTokenKind::Emerge
        && (trailing_clause_tokens.is_empty()
            || is_granted_emerge_cost_tail(&trailing_clause_tokens))
    {
        return granted_emerge_abilities_from_subject(&subject_tokens, condition);
    }

    if !trailing_clause_tokens.is_empty() {
        if let Some(compiled) = parse_granted_alternative_cast_static(
            &subject_tokens,
            &keyword_tokens,
            &trailing_clause_tokens,
            condition.clone(),
        )? {
            return Ok(Some(compiled));
        }

        let ignore_keyword_reminder =
            keyword_kind == anthem_grant_grammar::GrantedKeywordTokenKind::IgnoredReminder;
        if !ignore_keyword_reminder {
            if parse_ability_line(&keyword_tokens).is_some() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing granted-keyword clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(None);
        }
    }

    if let Some(compiled) = parse_union_subject_keyword_grants(
        &subject_tokens,
        &keyword_tokens,
        condition.clone(),
        &clause_words.join(" "),
    )? {
        return Ok(Some(compiled));
    }

    if let Some(compiled) = parse_color_filtered_keyword_grants(
        &subject_tokens,
        &keyword_tokens,
        condition.clone(),
        &clause_words.join(" "),
    )? {
        return Ok(Some(
            compiled
                .into_iter()
                .map(|ability| with_leading_set_quantifier_surface(ability, &subject_tokens))
                .collect(),
        ));
    }

    if keyword_kind == anthem_grant_grammar::GrantedKeywordTokenKind::Exploit {
        let subject = parse_anthem_subject(&subject_tokens)?;
        return Ok(Some(vec![grant_exploit_for_anthem_subject(
            &subject, condition,
        )]));
    }

    let attached_subject_filter =
        infer_attached_subject_filter_from_condition_expr(condition.as_ref());
    let subject = first_spell_each_turn_subject(&subject_tokens)
        .map(Ok)
        .unwrap_or_else(|| {
            parse_anthem_subject_with_attached_fallback(
                &subject_tokens,
                attached_subject_filter.as_ref(),
            )
        })?;
    let condition =
        condition.map(|condition| bind_attachment_condition_to_subject(condition, &subject));

    let keyword_words = crate::runtime_backend::token_word_refs(&keyword_tokens);
    if keyword_words == ["bands", "with", "other", "legendary", "creatures"] {
        let ability = StaticAbilityAst::Static(StaticAbility::bands_with_other(
            ObjectFilter::creature().with_supertype(Supertype::Legendary),
            "bands with other legendary creatures",
        ));
        let compiled = match &subject {
            AnthemSubjectAst::Source => match condition {
                Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(ability),
                    condition,
                },
                None => ability,
            },
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
                filter: filter.clone(),
                ability: Box::new(ability),
                condition,
            },
        };
        return Ok(Some(vec![with_leading_set_quantifier_surface(
            compiled,
            &subject_tokens,
        )]));
    }

    if let Some((actions, subtypes)) = parse_keyword_and_subtype_addition_tail(&keyword_tokens)? {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        let mut compiled = Vec::new();
        for action in actions {
            if !action.lowers_to_static_ability() {
                return Ok(None);
            }
            match &subject {
                AnthemSubjectAst::Source => match &condition {
                    Some(condition) => compiled.push(StaticAbilityAst::ConditionalKeywordAction {
                        action,
                        condition: condition.clone(),
                    }),
                    None => compiled.push(StaticAbilityAst::KeywordAction(action)),
                },
                AnthemSubjectAst::Filter(filter) => {
                    compiled.push(StaticAbilityAst::GrantKeywordAction {
                        filter: filter.clone(),
                        action,
                        condition: condition.clone(),
                    });
                }
            }
        }

        match &subject {
            AnthemSubjectAst::Source => {
                compiled.push(conditional_static_ability(
                    StaticAbility::add_subtypes(ObjectFilter::source(), subtypes),
                    condition,
                ));
            }
            AnthemSubjectAst::Filter(filter) => {
                compiled.push(conditional_static_ability(
                    StaticAbility::add_subtypes(filter.clone(), subtypes),
                    condition,
                ));
            }
        }
        return Ok(Some(
            compiled
                .into_iter()
                .map(|ability| with_leading_set_quantifier_surface(ability, &subject_tokens))
                .collect(),
        ));
    }

    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    if actions.is_empty() {
        return Ok(None);
    }

    let grants_conspire = actions
        .iter()
        .filter(|action| matches!(action, KeywordAction::Conspire))
        .count();
    if grants_conspire > 0 {
        let mut compiled = Vec::new();
        for _ in 0..grants_conspire {
            match &subject {
                AnthemSubjectAst::Source => {
                    let ability =
                        StaticAbilityAst::Static(StaticAbility::keyword_marker("Conspire"));
                    if let Some(condition) = &condition {
                        compiled.push(StaticAbilityAst::ConditionalStaticAbility {
                            ability: Box::new(ability),
                            condition: condition.clone(),
                        });
                    } else {
                        compiled.push(ability);
                    }
                }
                AnthemSubjectAst::Filter(filter) => {
                    compiled.push(StaticAbilityAst::GrantStaticAbility {
                        filter: filter.clone(),
                        ability: Box::new(StaticAbilityAst::Static(StaticAbility::keyword_marker(
                            "Conspire",
                        ))),
                        condition: condition.clone(),
                    });
                }
            }
        }
        return Ok(Some(
            compiled
                .into_iter()
                .map(|ability| with_leading_set_quantifier_surface(ability, &subject_tokens))
                .collect(),
        ));
    }

    let mut mapped = Vec::new();
    let mut object_ability_grants = Vec::new();
    for action in actions {
        if action.lowers_to_static_ability() {
            mapped.push(action);
        } else if let Some(granted) = granted_object_ability_for_keyword_action(&action) {
            object_ability_grants.push(granted);
        } else {
            return Ok(None);
        }
    }
    if mapped.is_empty() && object_ability_grants.is_empty() && !grants_must_attack {
        return Ok(None);
    }

    let mut compiled = Vec::new();
    if grants_must_attack {
        match &subject {
            AnthemSubjectAst::Source => {
                if let Some(condition) = &condition {
                    compiled.push(StaticAbilityAst::ConditionalStaticAbility {
                        ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
                        condition: condition.clone(),
                    });
                } else {
                    compiled.push(StaticAbilityAst::Static(StaticAbility::must_attack()));
                }
            }
            AnthemSubjectAst::Filter(filter) => {
                compiled.push(StaticAbilityAst::GrantStaticAbility {
                    filter: filter.clone(),
                    ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
                    condition: condition.clone(),
                })
            }
        }
    }
    for action in mapped {
        let ast = match &subject {
            AnthemSubjectAst::Source => match &condition {
                Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                    action,
                    condition: condition.clone(),
                },
                None => StaticAbilityAst::KeywordAction(action),
            },
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
                filter: filter.clone(),
                action,
                condition: condition.clone(),
            },
        };
        compiled.push(ast);
    }
    let grant_clause = ParsedAnthemClause {
        subject,
        power: AnthemValue::Fixed(0),
        toughness: AnthemValue::Fixed(0),
        condition,
        set_quantifier_surface: leading_set_quantifier_surface(&subject_tokens),
        count_uses_where_x: false,
    };
    for (ability, display) in object_ability_grants {
        compiled.push(grant_object_ability_for_anthem_subject(
            &grant_clause,
            ability,
            display,
        ));
    }
    Ok(Some(
        compiled
            .into_iter()
            .map(|ability| with_leading_set_quantifier_surface(ability, &subject_tokens))
            .collect(),
    ))
}

pub(crate) fn parse_all_creatures_lose_flying_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if anthem_grant_grammar::parse_all_creatures_lose_flying(tokens) {
        return Ok(Some(StaticAbilityAst::RemoveKeywordAction {
            filter: ObjectFilter::creature(),
            action: KeywordAction::Flying,
            mode: ironsmith_core::AbilityLossMode::Lose,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_subject_loses_keywords_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(parsed) = anthem_grant_grammar::parse_subject_loses_keywords_clause(tokens) else {
        return Ok(None);
    };
    let subject_tokens = parsed.subject_tokens;
    let filter = match parse_object_filter(&subject_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    let Some(loss_actions) = parse_ability_line(parsed.loss_tokens) else {
        return Ok(None);
    };

    let actions = loss_actions;
    if let Some(gain_tokens) = parsed.additional_gain_tokens {
        let Some(gain_actions) = parse_ability_line(gain_tokens) else {
            return Ok(None);
        };
        if actions.len() != gain_actions.len()
            || actions.iter().any(|action| !gain_actions.contains(action))
        {
            return Ok(None);
        }
    }

    let clause_text = crate::runtime_backend::token_word_refs(tokens).join(" ");
    reject_unimplemented_keyword_actions(&actions, &clause_text)?;

    let mut result = Vec::new();
    for action in actions {
        if !action.lowers_to_static_ability() {
            return Ok(None);
        }
        if result.iter().any(|existing| {
            matches!(
                existing,
                StaticAbilityAst::RemoveKeywordAction {
                    filter: existing_filter,
                    action: existing_action,
                    ..
                } if existing_filter == &filter && existing_action == &action
            )
        }) {
            continue;
        }
        result.push(StaticAbilityAst::RemoveKeywordAction {
            filter: filter.clone(),
            action,
            mode: parsed.loss_mode,
        });
    }

    if result.is_empty() {
        return Ok(None);
    }
    Ok(Some(result))
}

pub(crate) fn parse_each_creature_cant_be_blocked_by_more_than_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_by_more_than_clause(tokens) else {
        return Ok(None);
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(subject) = anthem_grant_grammar::parse_each_creature_subject(parsed.subject_tokens)
    else {
        return Ok(None);
    };
    let Some((minimum_blockers, used)) = parse_greater_than_or_equal_quantity_prefix(
        parsed.blocker_threshold_tokens,
        false,
        false,
        "cant-be-blocked blocker threshold",
    )?
    else {
        return Ok(None);
    };
    if minimum_blockers == 0 || used != parsed.blocker_threshold_tokens.len() {
        return Ok(None);
    }
    let amount = minimum_blockers - 1;
    let filter_tokens = subject.filter_tokens;
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported cant-be-blocked-by-more-than subject (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let granted = StaticAbility::cant_be_blocked_by_more_than(amount as usize);
    let ability = StaticAbilityAst::GrantStaticAbility {
        filter,
        ability: Box::new(StaticAbilityAst::Static(granted)),
        condition: None,
    };
    Ok(Some(StaticAbilityAst::WithSetQuantifierSurface {
        ability: Box::new(ability),
        surface: ironsmith_core::SetQuantifierSurface::Each,
    }))
}

pub(crate) fn parse_each_creature_can_block_additional_creature_each_combat_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    // High Ground: "Each creature can block an additional creature each combat."
    let Some(parsed) = parse_can_block_additional_creature_clause(tokens) else {
        return Ok(None);
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(subject) = anthem_grant_grammar::parse_each_creature_subject(parsed.subject_tokens)
    else {
        return Ok(None);
    };

    let Some(additional) =
        anthem_grant_grammar::parse_additional_creature_count(parsed.additional_count_tokens)
    else {
        return Ok(None);
    };

    let filter_tokens = subject.filter_tokens;
    let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported can-block-additional subject (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let granted = StaticAbility::can_block_additional_creature_each_combat(additional);
    let ability = StaticAbilityAst::GrantStaticAbility {
        filter,
        ability: Box::new(StaticAbilityAst::Static(granted)),
        condition: None,
    };
    Ok(Some(StaticAbilityAst::WithSetQuantifierSurface {
        ability: Box::new(ability),
        surface: ironsmith_core::SetQuantifierSurface::Each,
    }))
}

pub(crate) fn parse_lose_all_abilities_and_transform_base_pt_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    fn title_case_words(words: &[&str]) -> String {
        words
            .iter()
            .map(|word| {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    let mut out = String::new();
                    out.extend(first.to_uppercase());
                    out.push_str(chars.as_str());
                    out
                } else {
                    String::new()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    let Some(shape) = anthem_grant_grammar::parse_lose_all_transform_shape(tokens) else {
        return Ok(None);
    };
    let Some(pt_word) = words.get(shape.power_toughness_word) else {
        return Ok(None);
    };
    let (power, toughness) = parse_pt_modifier(pt_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let subject_token_end = word_view
        .token_index_after_words(shape.subject_word_end)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "invalid subject span in lose-all-abilities transform clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    let subject_tokens = trim_commas(tokens.get(..subject_token_end).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "invalid subject token span in lose-all-abilities transform clause (clause: '{}')",
            words.join(" ")
        ))
    })?);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities transform clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let descriptor_span = words.get(shape.descriptor_words.clone()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "invalid descriptor span in lose-all-abilities transform clause (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let mut descriptor_words = non_article_word_refs_except(descriptor_span, &["and"]);
    if descriptor_words.is_empty() {
        return Ok(None);
    }
    if descriptor_words.first().copied() == Some("all") {
        descriptor_words.remove(0);
    }
    if descriptor_words.is_empty() {
        return Ok(None);
    }

    let mut set_colors = ColorSet::new();
    let mut set_card_types: Vec<CardType> = Vec::new();
    let mut creature_subtypes: Vec<Subtype> = Vec::new();

    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            set_colors = set_colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !set_card_types.iter().any(|existing| *existing == card_type) {
                set_card_types.push(card_type);
            }
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            if !creature_subtypes
                .iter()
                .any(|existing| *existing == subtype)
            {
                creature_subtypes.push(subtype);
            }
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported transform descriptor '{}' (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if !creature_subtypes.is_empty()
        && !set_card_types
            .iter()
            .any(|existing| *existing == CardType::Creature)
    {
        set_card_types.push(CardType::Creature);
    }

    let set_name = shape
        .name_words
        .as_ref()
        .map(|range| {
            words
                .get(range.clone())
                .map(title_case_words)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "invalid name span in lose-all-abilities transform clause (clause: '{}')",
                        words.join(" ")
                    ))
                })
        })
        .transpose()?;

    let mut abilities = vec![if shape.except_mana_abilities {
        StaticAbility::remove_all_abilities_except_mana(filter.clone())
    } else {
        StaticAbility::remove_all_abilities(filter.clone())
    }];

    if !set_card_types.is_empty() {
        abilities.push(StaticAbility::set_card_types(
            filter.clone(),
            set_card_types,
        ));
    }
    if !creature_subtypes.is_empty() {
        abilities.push(StaticAbility::set_creature_subtypes(
            filter.clone(),
            creature_subtypes,
        ));
    }
    if !set_colors.is_empty() {
        abilities.push(StaticAbility::set_colors(filter.clone(), set_colors));
    }
    if let Some(name) = set_name {
        abilities.push(StaticAbility::set_name(filter.clone(), name));
    }
    abilities.push(StaticAbility::set_base_power_toughness(
        filter, power, toughness,
    ));

    Ok(Some(abilities))
}

pub(crate) fn parse_lose_all_abilities_and_doesnt_untap_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clauses = split_lexed_slices_on_and(tokens);
    let [remove_clause, untap_clause] = clauses.as_slice() else {
        return Ok(None);
    };
    let remove_clause = trim_edge_punctuation_tokens(remove_clause);
    if !is_dependent_doesnt_untap_during_controller_untap_step_line_lexed(untap_clause) {
        return Ok(None);
    }

    let Some(shape) = anthem_grant_grammar::parse_lose_all_abilities_shape(remove_clause) else {
        return Ok(None);
    };
    if shape.becomes || shape.except_mana_abilities || shape.base_power_toughness_word.is_some() {
        return Ok(None);
    }

    let word_view = TokenWordView::new(remove_clause);
    let subject_token_end = word_view
        .token_index_after_words(shape.subject_word_end)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "invalid subject span in lose-all-abilities and untap clause (clause: '{}')",
                render_token_slice(tokens)
            ))
        })?;
    let subject_tokens =
        trim_edge_punctuation_tokens(remove_clause.get(..subject_token_end).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "invalid subject token span in lose-all-abilities and untap clause (clause: '{}')",
                render_token_slice(tokens)
            ))
        })?);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities and untap clause (clause: '{}')",
            render_token_slice(tokens)
        ))
    })?;
    let subject = render_token_slice(subject_tokens);

    Ok(Some(vec![
        StaticAbility::remove_all_abilities(filter.clone()),
        StaticAbility::restriction(
            crate::effect::Restriction::untap(filter),
            format!("{subject} doesn't untap during its controller's untap step"),
        ),
    ]))
}

pub(crate) fn parse_lose_all_abilities_and_base_pt_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    if let Some(animation) =
        crate::runtime_backend::grammar::effects::become_shapes::parse_filtered_object_animation_tokens(
            tokens,
        )
        && animation.removes_all_abilities
        && !animation.dependent_subject
    {
        let mut filter = parse_object_filter(animation.subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported subject in lose-all-abilities animation clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        filter.set_set_quantifier_surface(leading_set_quantifier_surface(animation.subject_tokens));
        return Ok(Some(filtered_object_animation_abilities(filter, animation)));
    }

    let Some(shape) = anthem_grant_grammar::parse_lose_all_abilities_shape(tokens) else {
        return Ok(None);
    };
    if shape.becomes {
        let Some(becomes_word) = words.iter().position(|word| *word == "becomes") else {
            return Ok(None);
        };
        let Some(power_toughness) =
            crate::runtime_backend::grammar::effects::become_shapes::parse_become_base_pt_words(
                words.get(becomes_word + 1..).unwrap_or_default(),
            )
        else {
            return Ok(None);
        };
        let Some(descriptor) = crate::runtime_backend::grammar::effects::become_shapes::parse_become_creature_descriptor_words(
            power_toughness.descriptor_words,
        ) else {
            return Ok(None);
        };

        let subject_token_end = word_view
            .token_index_after_words(shape.subject_word_end)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "invalid subject span in lose-all-abilities becomes clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        let subject_tokens = trim_commas(tokens.get(..subject_token_end).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "invalid subject token span in lose-all-abilities becomes clause (clause: '{}')",
                words.join(" ")
            ))
        })?);
        let mut filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported subject in lose-all-abilities becomes clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        filter.set_set_quantifier_surface(leading_set_quantifier_surface(&subject_tokens));

        let mut abilities = vec![if shape.except_mana_abilities {
            StaticAbility::remove_all_abilities_except_mana(filter.clone())
        } else {
            StaticAbility::remove_all_abilities(filter.clone())
        }];
        if !descriptor.card_types.is_empty() {
            abilities.push(StaticAbility::set_card_types(
                filter.clone(),
                descriptor.card_types,
            ));
        }
        if !descriptor.subtypes.is_empty() {
            abilities.push(StaticAbility::set_creature_subtypes(
                filter.clone(),
                descriptor.subtypes,
            ));
        }
        if let Some(colors) = descriptor.colors {
            abilities.push(StaticAbility::set_colors(filter.clone(), colors));
        }
        abilities.push(match (power_toughness.power, power_toughness.toughness) {
            (Value::Fixed(power), Value::Fixed(toughness)) => {
                StaticAbility::set_base_power_toughness(filter, power, toughness)
            }
            (power, toughness) => {
                StaticAbility::set_base_power_toughness_value(filter, power, toughness)
            }
        });
        return Ok(Some(abilities));
    }

    let subject_token_end = word_view
        .token_index_after_words(shape.subject_word_end)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "invalid subject span in lose-all-abilities clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    let subject_tokens = tokens.get(..subject_token_end).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "invalid subject token span in lose-all-abilities clause (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let filter = parse_object_filter(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let mut abilities = vec![if shape.except_mana_abilities {
        StaticAbility::remove_all_abilities_except_mana(filter.clone())
    } else {
        StaticAbility::remove_all_abilities(filter.clone())
    }];

    if let Some(modifier_word) = shape.base_power_toughness_word {
        if let Some(modifier_token) = words.get(modifier_word)
            && let Ok((power, toughness)) = parse_pt_modifier(modifier_token)
        {
            abilities.push(StaticAbility::set_base_power_toughness(
                filter, power, toughness,
            ));
        }
    }

    Ok(Some(abilities))
}

pub(crate) fn parse_all_have_indestructible_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(parsed) = anthem_grant_grammar::parse_indestructible_grant_clause(tokens) else {
        return Ok(None);
    };
    let Some(actions) = parse_ability_line(parsed.ability_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &words.join(" "))?;
    if actions.len() != 1
        || !actions
            .first()
            .is_some_and(|action| matches!(action, KeywordAction::Indestructible))
    {
        return Ok(None);
    }

    let filter = parse_object_filter(parsed.subject_tokens, false)?;
    Ok(Some(StaticAbilityAst::GrantKeywordAction {
        filter,
        action: KeywordAction::Indestructible,
        condition: None,
    }))
}

#[derive(Debug, Clone)]
pub(crate) enum AnthemSubjectAst {
    Source,
    Filter(ObjectFilter),
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedAnthemClause {
    pub(crate) subject: AnthemSubjectAst,
    pub(crate) power: AnthemValue,
    pub(crate) toughness: AnthemValue,
    pub(crate) condition: Option<crate::ConditionExpr>,
    pub(crate) set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    /// Whether the scaling count was written as "where X is …" (vs "for each …")
    /// in the original oracle text. Surface hint preserved for rendering.
    pub(crate) count_uses_where_x: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnimationSubtypeMode {
    Add,
    ReplaceCreatureTypes,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedGrantedTailAst {
    pub(crate) granted_static: Vec<StaticAbilityAst>,
    pub(crate) granted_keyword_actions: Vec<KeywordAction>,
    pub(crate) granted_object_abilities: Vec<(ParsedAbility, String)>,
    pub(crate) removes_all_other_abilities: bool,
    pub(crate) type_color_additions: Vec<TypeColorAdditionClause>,
}

#[derive(Debug, Clone)]
pub(crate) struct StaticAnimationBundleAst {
    pub(crate) subject: AnthemSubjectAst,
    pub(crate) condition: Option<crate::ConditionExpr>,
    pub(crate) ensure_creature_type: bool,
    pub(crate) subtypes: Vec<Subtype>,
    pub(crate) subtype_mode: AnimationSubtypeMode,
    pub(crate) base_power_toughness: Option<(Value, Value)>,
    pub(crate) granted_tail: ParsedGrantedTailAst,
}

fn is_granted_blitz_cost_tail(trailing_tokens: &[OwnedLexToken]) -> bool {
    anthem_grant_grammar::parse_granted_blitz_cost_equals_mana(trailing_tokens)
}

fn is_granted_emerge_cost_tail(trailing_tokens: &[OwnedLexToken]) -> bool {
    anthem_grant_grammar::parse_granted_emerge_cost_equals_mana(trailing_tokens)
}

fn normalize_granted_alternative_spell_filter(
    mut filter: ObjectFilter,
) -> (ObjectFilter, Vec<Zone>) {
    let parsed_zone = filter.zone.unwrap_or(Zone::Battlefield);
    if parsed_zone == Zone::Stack || filter.stack_kind.is_some() {
        filter.zone = None;
        filter.stack_kind = None;
        filter.cast_by = None;
        return (filter, vec![Zone::Hand, Zone::Exile, Zone::Graveyard]);
    }

    filter.zone = None;
    (filter, vec![parsed_zone])
}

fn granted_blitz_abilities_from_subject(
    subject_tokens: &[OwnedLexToken],
    condition: Option<crate::ConditionExpr>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let subject = parse_anthem_subject(&subject_tokens)?;
    let AnthemSubjectAst::Filter(filter) = subject else {
        return Ok(None);
    };
    let (filter, zones) = normalize_granted_alternative_spell_filter(filter);
    let mut abilities = Vec::new();
    for zone in zones {
        let spec = crate::grant::GrantSpec::new(
            crate::grant::Grantable::blitz_from_cards_mana_cost(),
            filter.clone(),
            zone,
        );
        let mut ability = StaticAbilityAst::Static(StaticAbility::grants(spec));
        if let Some(condition) = condition.clone() {
            ability = StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            };
        }
        abilities.push(ability);
    }
    Ok(Some(abilities))
}

fn granted_emerge_abilities_from_subject(
    subject_tokens: &[OwnedLexToken],
    condition: Option<crate::ConditionExpr>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let subject = parse_anthem_subject(subject_tokens)?;
    let AnthemSubjectAst::Filter(filter) = subject else {
        return Ok(None);
    };
    let (filter, zones) = if anthem_grant_grammar::emerge_subject_is_spell_cast(subject_tokens) {
        let mut filter = filter;
        filter.zone = None;
        filter.stack_kind = None;
        filter.cast_by = None;
        (filter, vec![Zone::Hand])
    } else {
        normalize_granted_alternative_spell_filter(filter)
    };
    let mut abilities = Vec::new();
    for zone in zones {
        if zone != Zone::Hand {
            continue;
        }
        let spec = crate::grant::GrantSpec::new(
            crate::grant::Grantable::emerge_from_cards_mana_cost(),
            filter.clone(),
            zone,
        );
        let mut ability = StaticAbilityAst::Static(StaticAbility::grants(spec));
        if let Some(condition) = condition.clone() {
            ability = StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            };
        }
        abilities.push(ability);
    }
    Ok((!abilities.is_empty()).then_some(abilities))
}

fn parse_keyword_and_subtype_addition_tail(
    keyword_tokens: &[OwnedLexToken],
) -> Result<Option<(Vec<KeywordAction>, Vec<Subtype>)>, CardTextError> {
    let Some(split) = anthem_grant_grammar::split_keyword_and_type_addition(keyword_tokens) else {
        return Ok(None);
    };
    let Some(actions) = parse_ability_line(split.keyword_tokens) else {
        return Ok(None);
    };
    if actions.is_empty() {
        return Ok(None);
    }

    let Some(additions) = parse_type_color_addition_clause(split.addition_tokens)? else {
        return Ok(None);
    };
    if !additions.added_colors.is_empty()
        || !additions.set_colors.is_empty()
        || !additions.card_types.is_empty()
        || additions.subtypes.is_empty()
    {
        return Ok(None);
    }

    Ok(Some((actions, additions.subtypes)))
}

fn conditional_static_ability(
    ability: StaticAbility,
    condition: Option<crate::ConditionExpr>,
) -> StaticAbilityAst {
    let ast = StaticAbilityAst::Static(ability);
    match condition {
        Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(ast),
            condition,
        },
        None => ast,
    }
}

pub(crate) fn parse_source_counter_threshold_keyword_and_subtype_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(head) = anthem_grant_grammar::parse_source_counter_threshold_head(tokens) else {
        return Ok(None);
    };
    let have_token_idx = head.have_token;
    let Ok((Some(condition), subject_start)) =
        parse_anthem_prefix_condition(tokens, have_token_idx)
    else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(&tokens[subject_start..have_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = parse_anthem_subject(&subject_tokens)?;

    let tail_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
    let Some((actions, subtypes)) = parse_keyword_and_subtype_addition_tail(&tail_tokens)? else {
        return Ok(None);
    };

    let clause_text = crate::runtime_backend::token_word_refs(tokens).join(" ");
    reject_unimplemented_keyword_actions(&actions, &clause_text)?;

    let mut compiled = Vec::new();
    for action in actions {
        if !action.lowers_to_static_ability() {
            return Ok(None);
        }
        match &subject {
            AnthemSubjectAst::Source => compiled.push(StaticAbilityAst::ConditionalKeywordAction {
                action,
                condition: condition.clone(),
            }),
            AnthemSubjectAst::Filter(filter) => {
                compiled.push(StaticAbilityAst::GrantKeywordAction {
                    filter: filter.clone(),
                    action,
                    condition: Some(condition.clone()),
                })
            }
        }
    }

    let subtype_ability = match &subject {
        AnthemSubjectAst::Source => StaticAbility::add_subtypes(ObjectFilter::source(), subtypes),
        AnthemSubjectAst::Filter(filter) => StaticAbility::add_subtypes(filter.clone(), subtypes),
    };
    compiled.push(StaticAbilityAst::ConditionalStaticAbility {
        ability: Box::new(StaticAbilityAst::Static(subtype_ability)),
        condition,
    });

    Ok((!compiled.is_empty()).then_some(compiled))
}

pub(crate) fn find_source_reference_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut token_indices = Vec::new();
    let mut token_words = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if let Some(word) = token.as_word() {
            token_indices.push(idx);
            token_words.push(word);
        }
    }

    for word_start in 0..token_words.len() {
        if is_source_reference_words(&token_words[word_start..]) {
            return token_indices.get(word_start).copied();
        }
    }
    None
}

pub(crate) fn parse_best_object_filter_suffix(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut best: Option<(usize, usize, ObjectFilter)> = None;
    for start in 0..tokens.len() {
        if tokens[start].as_word().is_none() {
            continue;
        }
        let mut other = false;
        let mut candidate = &tokens[start..];
        match anthem_grant_grammar::classify_suffix_filter_head(candidate) {
            anthem_grant_grammar::SuffixFilterHead::Other => {
                other = true;
                candidate = &candidate[1..];
            }
            anthem_grant_grammar::SuffixFilterHead::Pronoun => continue,
            anthem_grant_grammar::SuffixFilterHead::Normal => {}
        }
        if candidate.is_empty() {
            continue;
        }
        let Ok(filter) = parse_object_filter(candidate, other) else {
            continue;
        };
        let score = anthem_grant_grammar::object_filter_specificity_score(&filter);
        if best
            .as_ref()
            .is_none_or(|(best_score, _, _)| score > *best_score)
        {
            best = Some((score, start, filter));
        }
    }
    best.map(|(_, start, filter)| {
        if start > 0 {
            crate::parse_loss::record(
                "suffix_object_filter_recovery",
                format!(
                    "parsed '{}' as suffix of '{}'",
                    crate::runtime_backend::token_word_refs(&tokens[start..]).join(" "),
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ),
            );
        }
        filter
    })
}

pub(crate) fn parse_anthem_subject(
    tokens: &[OwnedLexToken],
) -> Result<AnthemSubjectAst, CardTextError> {
    let subject_words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    if let Some(subject) = first_spell_each_turn_subject_tokens(tokens)? {
        return Ok(subject);
    }
    if let Some(subject) = first_spell_each_turn_subject(tokens) {
        return Ok(subject);
    }
    if anthem_grant_grammar::is_source_it_subject(tokens) {
        return Ok(AnthemSubjectAst::Source);
    }
    if is_source_reference_words(&subject_words) {
        return Ok(AnthemSubjectAst::Source);
    }
    let generic_filter = parse_object_filter(tokens, false).ok();
    if let Some(filter) = generic_filter.as_ref()
        && (filter.in_combat_with_source
            || filter.attached_to_object.is_some()
            || filter.attached_to_player.is_some()
            || !filter.characteristic_relations.is_empty())
    {
        return Ok(AnthemSubjectAst::Filter(filter.clone()));
    }
    match anthem_grant_grammar::parse_exact_anthem_subject_grammar(tokens) {
        Some(anthem_grant_grammar::AnthemSubjectGrammarMatch::Filter(filter)) => {
            return Ok(AnthemSubjectAst::Filter(filter));
        }
        Some(anthem_grant_grammar::AnthemSubjectGrammarMatch::RejectFragment) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported anthem subject (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        None => {}
    }
    if let Some(filter) = parse_commander_creatures_you_own_subject(tokens) {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    if let Some(filter) = parse_enchanted_player_controls_subject(tokens)? {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    // A complete typed parse is strictly more faithful than suffix recovery.
    // Keep the specialized subject grammars above for their authored union and
    // quantifier surfaces, then use the complete generic filter before trying
    // lossy recovery from a trailing fragment.
    if let Some(filter) = generic_filter {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    if let Some(filter) = parse_best_object_filter_suffix(tokens) {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    // A named permanent can be the grammatical source of a static ability
    // even when no card definition has installed a source-reference alias for
    // the name yet (for example, "Balan has double strike ...").
    if tokens.iter().all(|token| token.as_word().is_some())
        && !subject_words.iter().any(|word| {
            matches!(
                *word,
                "a" | "an"
                    | "another"
                    | "each"
                    | "all"
                    | "any"
                    | "target"
                    | "you"
                    | "your"
                    | "opponent"
                    | "opponents"
                    | "this"
                    | "that"
                    | "it"
            )
        })
    {
        return Ok(AnthemSubjectAst::Source);
    }
    if find_source_reference_start(tokens).is_some() {
        return Ok(AnthemSubjectAst::Source);
    }
    Err(CardTextError::ParseError(format!(
        "unsupported anthem subject (clause: '{}')",
        crate::runtime_backend::token_word_refs(tokens).join(" ")
    )))
}

fn parse_commander_creatures_you_own_subject(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    anthem_grant_grammar::parse_commander_creature_subject_tokens(tokens).map(|_| {
        ObjectFilter::creature()
            .commander()
            .owned_by(PlayerFilter::You)
    })
}

fn parse_enchanted_player_controls_subject(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(prefix_tokens) = anthem_grant_grammar::parse_enchanted_player_controls_prefix(tokens)
    else {
        return Ok(None);
    };
    let mut filter = parse_object_filter(prefix_tokens, false)?;
    filter.controller = Some(PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted")));
    Ok(Some(filter))
}

fn infer_attached_subject_filter_from_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let condition_tokens = trim_edge_punctuation(tokens);
    let subject_tokens = anthem_grant_grammar::parse_attached_condition_subject(&condition_tokens)?;
    parse_object_filter(subject_tokens, false).ok()
}

fn parse_anthem_subject_with_attached_fallback(
    tokens: &[OwnedLexToken],
    attached_subject_filter: Option<&ObjectFilter>,
) -> Result<AnthemSubjectAst, CardTextError> {
    if anthem_grant_grammar::is_source_it_subject(tokens) {
        return Ok(match attached_subject_filter {
            Some(filter) => AnthemSubjectAst::Filter(filter.clone()),
            None => AnthemSubjectAst::Source,
        });
    }
    parse_anthem_subject(tokens)
}

fn infer_attached_subject_filter_from_condition_expr(
    condition: Option<&crate::ConditionExpr>,
) -> Option<ObjectFilter> {
    match condition {
        Some(crate::ConditionExpr::EnchantedPermanentIsCreature)
        | Some(crate::ConditionExpr::EnchantedPermanentIsLand)
        | Some(crate::ConditionExpr::EnchantedPermanentIsEquipment)
        | Some(crate::ConditionExpr::EnchantedPermanentIsVehicle) => {
            Some(ObjectFilter::tagged("enchanted"))
        }
        Some(crate::ConditionExpr::AttachmentCount {
            host: ironsmith_core::AttachmentConditionHost::Matching(filter),
            ..
        }) if attachment_condition_host_has_tag(filter, &["enchanted", "equipped"]) => {
            Some(filter.clone())
        }
        _ => None,
    }
}

fn attachment_condition_host_has_tag(filter: &ObjectFilter, tags: &[&str]) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && tags.contains(&constraint.tag.as_str())
    })
}

fn bind_attachment_condition_to_subject(
    condition: crate::ConditionExpr,
    subject: &AnthemSubjectAst,
) -> crate::ConditionExpr {
    match condition {
        crate::ConditionExpr::AttachmentCount {
            attachment,
            host: ironsmith_core::AttachmentConditionHost::Matching(filter),
            comparison,
            display,
        } => {
            let refers_to_it = attachment_condition_host_has_tag(&filter, &["__it__"]);
            let refers_to_attached_object =
                attachment_condition_host_has_tag(&filter, &["enchanted", "equipped"]);
            let host = if refers_to_it && matches!(subject, AnthemSubjectAst::Source) {
                ironsmith_core::AttachmentConditionHost::Source
            } else if (refers_to_it || refers_to_attached_object)
                && attached_object_anthem_subject_filter(subject).is_some()
            {
                ironsmith_core::AttachmentConditionHost::SourceAttachedObject
            } else {
                ironsmith_core::AttachmentConditionHost::Matching(filter)
            };
            crate::ConditionExpr::AttachmentCount {
                attachment,
                host,
                comparison,
                display,
            }
        }
        crate::ConditionExpr::And(left, right) => crate::ConditionExpr::And(
            Box::new(bind_attachment_condition_to_subject(*left, subject)),
            Box::new(bind_attachment_condition_to_subject(*right, subject)),
        ),
        crate::ConditionExpr::Or(left, right) => crate::ConditionExpr::Or(
            Box::new(bind_attachment_condition_to_subject(*left, subject)),
            Box::new(bind_attachment_condition_to_subject(*right, subject)),
        ),
        crate::ConditionExpr::Not(inner) => crate::ConditionExpr::Not(Box::new(
            bind_attachment_condition_to_subject(*inner, subject),
        )),
        other => other,
    }
}

pub(crate) fn parse_static_quantity_prefix(
    tokens: &[OwnedLexToken],
    allow_default_one: bool,
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    parse_quantity_comparison_prefix(tokens, allow_default_one, true, "static condition")
}

pub(crate) fn parse_permanent_card_count_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let facts = anthem_grant_grammar::parse_permanent_card_count_facts(tokens)?;

    let mut filter = ObjectFilter::default();
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];

    filter.zone = Some(facts.zone);
    filter.owner = match facts.owner {
        Some(anthem_grant_grammar::PermanentCardOwner::You) => Some(PlayerFilter::You),
        Some(anthem_grant_grammar::PermanentCardOwner::Opponent) => Some(PlayerFilter::Opponent),
        None => None,
    };
    Some(filter)
}

fn parse_negated_subject_descriptor_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let positions = crate::runtime_backend::lexer::parser_token_word_positions(tokens);
    let mut copula = None;
    for (word_idx, (token_idx, word)) in positions.iter().enumerate() {
        let replacement = match *word {
            "isnt" | "isn't" => Some("is"),
            "arent" | "aren't" => Some("are"),
            "is" | "are"
                if positions
                    .get(word_idx + 1)
                    .is_some_and(|(_, next)| *next == "not") =>
            {
                Some(*word)
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            let remove_not = positions
                .get(word_idx + 1)
                .filter(|(_, next)| *next == "not")
                .map(|(token_idx, _)| *token_idx);
            copula = Some((*token_idx, replacement, remove_not));
            break;
        }
    }
    let (copula_token, replacement, remove_not) = copula?;

    let mut positive = tokens.to_vec();
    if !positive.get_mut(copula_token)?.replace_word(replacement) {
        return None;
    }
    if let Some(not_token) = remove_not {
        positive.remove(not_token);
    }
    let condition =
        crate::runtime_backend::grammar::conditions::parse_subject_descriptor_condition(&positive)?;
    let positive_display = crate::runtime_backend::token_word_refs(&positive).join(" ");
    Some(crate::ConditionExpr::Not(Box::new(
        condition.condition_expr(positive_display),
    )))
}

pub(crate) fn parse_static_condition_clause(
    tokens: &[OwnedLexToken],
) -> Result<crate::ConditionExpr, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let clause_word_storage = AnthemNormalizedWords::new(&tokens);
    let clause_words = clause_word_storage.word_refs();
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing condition clause after 'as long as'".to_string(),
        ));
    }
    let display = clause_words.join(" ");

    if let Some(condition) = parse_negated_subject_descriptor_condition(&tokens) {
        return Ok(condition);
    }

    if let Some(condition) = parse_cards_in_hand_static_condition(&tokens) {
        return Ok(condition);
    }
    if let Some(condition) = parse_life_total_static_condition(&tokens) {
        return Ok(condition);
    }
    if let Some(filter) =
        crate::runtime_backend::grammar::filters::parse_source_keyword_condition_filter_lexed(
            &tokens,
        )
    {
        return Ok(crate::ConditionExpr::SourceMatches(filter));
    }

    if let Some(kind) = anthem_grant_grammar::parse_fixed_static_condition_kind(&tokens) {
        use anthem_grant_grammar::FixedStaticConditionKind;
        return match kind {
            FixedStaticConditionKind::SourceIsEquipped => {
                Ok(crate::ConditionExpr::SourceIsEquipped)
            }
            FixedStaticConditionKind::SourceSpellWasKicked => {
                Ok(crate::ConditionExpr::ThisSpellWasKicked)
            }
            FixedStaticConditionKind::OpponentLostLifeThisTurn => {
                Ok(crate::ConditionExpr::OpponentLostLifeThisTurn)
            }
            FixedStaticConditionKind::YouDidNotCastSpellThisTurn => Ok(crate::ConditionExpr::Not(
                Box::new(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                    player: PlayerFilter::You,
                    count: 1,
                }),
            )),
            FixedStaticConditionKind::YouCastSpellThisTurn => {
                Ok(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                    player: PlayerFilter::You,
                    count: 1,
                })
            }
            FixedStaticConditionKind::NoCardsInYourLibrary => {
                Ok(crate::ConditionExpr::CountComparison {
                    count: AnthemCountExpression::MatchingFilter(
                        ObjectFilter::default()
                            .in_zone(Zone::Library)
                            .owned_by(PlayerFilter::You),
                    ),
                    comparison: crate::effect::Comparison::Equal(0),
                    display: Some("there are no cards in your library".to_string()),
                })
            }
            FixedStaticConditionKind::SourceIsOnBattlefield => {
                Ok(crate::ConditionExpr::SourceIsInZone(Zone::Battlefield))
            }
            FixedStaticConditionKind::SourceDevouredCreature => {
                Ok(crate::ConditionExpr::SourceDevouredCreaturesOrMore(1))
            }
            FixedStaticConditionKind::SourceIsSoulbondPaired => {
                Ok(crate::ConditionExpr::SourceIsSoulbondPaired)
            }
            FixedStaticConditionKind::SourceAttackedThisTurn => {
                Ok(crate::ConditionExpr::SourceAttackedThisTurn)
            }
            FixedStaticConditionKind::YouAttackedThisTurn => {
                Ok(crate::ConditionExpr::AttackedThisTurn)
            }
            FixedStaticConditionKind::SourceEnteredThisTurn => {
                let mut filter = ObjectFilter::source();
                filter.entered_battlefield_this_turn = true;
                Ok(crate::ConditionExpr::CountComparison {
                    count: AnthemCountExpression::MatchingFilter(filter),
                    comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                    display: Some(display.clone()),
                })
            }
            FixedStaticConditionKind::YourTurn => Ok(crate::ConditionExpr::YourTurn),
            FixedStaticConditionKind::SourcePowerEven => Err(CardTextError::ParseError(
                "unsupported source power parity condition (clause: 'this power is even')"
                    .to_string(),
            )),
            FixedStaticConditionKind::SourcePowerOdd => Err(CardTextError::ParseError(
                "unsupported source power parity condition (clause: 'this power is odd')"
                    .to_string(),
            )),
            FixedStaticConditionKind::NotYourTurn => Ok(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::YourTurn,
            ))),
            FixedStaticConditionKind::YourLifeAtMostHalfStarting => Ok(
                crate::ConditionExpr::PlayerLifeAtMostHalfStartingLifeTotal {
                    player: PlayerFilter::You,
                },
            ),
            FixedStaticConditionKind::YouCommittedCrimeThisTurn => {
                Ok(crate::ConditionExpr::PlayerCommittedCrimeThisTurn {
                    player: PlayerFilter::You,
                })
            }
        };
    }

    if let Some(life) = anthem_grant_grammar::parse_life_total_or_less_condition(&tokens) {
        return Ok(crate::ConditionExpr::LifeTotalOrLess(life as i32));
    }

    if let Some(counter) =
        crate::runtime_backend::grammar::conditions::parse_player_counter_condition(&tokens)
    {
        let Some((operator, value)) =
            crate::runtime_backend::util::comparison_to_value_comparison_operator(
                counter.comparison,
            )
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported player-counter comparison (clause: '{display}')"
            )));
        };
        return Ok(crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::PlayerCounters(counter.player, counter.counter_type),
            operator,
            right: crate::effect::Value::Fixed(value),
        });
    }

    if let Some(attachment) =
        crate::runtime_backend::grammar::conditions::parse_object_attached_to_object_condition(
            &tokens,
        )
    {
        return Ok(crate::ConditionExpr::AttachmentCount {
            attachment: attachment.attachment_filter,
            host: ironsmith_core::AttachmentConditionHost::Matching(attachment.attached_to_filter),
            comparison: attachment.comparison,
            display: attachment.display,
        });
    }

    if let Some(condition) = parse_devotion_static_condition(&tokens)? {
        return Ok(condition);
    }

    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_subject_status_condition(&tokens)
            .and_then(|condition| condition.condition_expr())
    {
        return Ok(condition);
    }
    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_subject_descriptor_condition(&tokens)
    {
        return Ok(condition.condition_expr(display.clone()));
    }
    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_player_status_condition(&tokens)
    {
        return Ok(condition.condition_expr());
    }
    if let Some(count) = anthem_grant_grammar::parse_x_value_at_least_condition(&tokens) {
        return Ok(crate::ConditionExpr::XValueAtLeast(count));
    }
    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_player_achievement_condition(&tokens)
    {
        return Ok(condition.condition_expr());
    }
    if let Some(condition) = parse_cards_drawn_this_turn_static_condition(&tokens) {
        return Ok(condition);
    }
    if let Some(shape) = anthem_grant_grammar::parse_blocking_source_condition(&tokens) {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::BlockingSource,
            comparison: shape.comparison,
            display: Some(display.clone()),
        });
    }
    if let Some(condition) = parse_dice_rolled_this_turn_static_condition(&tokens) {
        return Ok(condition);
    }

    if anthem_grant_grammar::parse_source_in_graveyard_condition(&tokens) {
        let mut filter = ObjectFilter::source();
        filter.zone = Some(Zone::Graveyard);
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
            display: Some(display.clone()),
        });
    }

    if let Some(conjoined) = parse_conjoined_static_condition_clause(&tokens) {
        return Ok(conjoined);
    }

    match anthem_grant_grammar::parse_existential_condition_shape(&tokens) {
        Err(()) => {
            return Err(CardTextError::ParseError(
                "missing quantity in static condition".to_string(),
            ));
        }
        Ok(Some(shape)) => {
            use anthem_grant_grammar::ExistentialConditionTail;
            match shape.tail {
                ExistentialConditionTail::CardTypesInYourGraveyard { threshold } => {
                    return Ok(crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore {
                        player: PlayerFilter::You,
                        count: threshold,
                    });
                }
                ExistentialConditionTail::CardsInYourGraveyard => {
                    let Some((operator, value)) =
                        crate::runtime_backend::util::comparison_to_value_comparison_operator(
                            shape.comparison,
                        )
                    else {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported graveyard card-count condition (clause: '{display}')"
                        )));
                    };
                    return Ok(crate::ConditionExpr::ValueComparison {
                        left: crate::effect::Value::CardsInGraveyard(PlayerFilter::You),
                        operator,
                        right: crate::effect::Value::Fixed(value),
                    });
                }
                ExistentialConditionTail::DistinctCounterTypesAmong { filter_tokens } => {
                    let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported distinct-counter-kind filter in static condition (clause: '{display}')"
                        ))
                    })?;
                    return Ok(crate::ConditionExpr::CountComparison {
                        count: AnthemCountExpression::DistinctCounterTypesAmong(filter),
                        comparison: shape.comparison,
                        display: Some(display.clone()),
                    });
                }
                ExistentialConditionTail::CountersAmong {
                    filter_tokens,
                    counter_type,
                } => {
                    let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported counter-among filter in static condition (clause: '{display}')"
                        ))
                    })?;
                    return Ok(crate::ConditionExpr::CountComparison {
                        count: AnthemCountExpression::CountersAmong(filter, counter_type),
                        comparison: shape.comparison,
                        display: Some(display.clone()),
                    });
                }
                ExistentialConditionTail::SourceInGraveyard => {
                    let mut filter = ObjectFilter::source();
                    filter.zone = Some(Zone::Graveyard);
                    return Ok(crate::ConditionExpr::CountComparison {
                        count: AnthemCountExpression::MatchingFilter(filter),
                        comparison: shape.comparison,
                        display: Some(display.clone()),
                    });
                }
                ExistentialConditionTail::Generic { filter_tokens } => {
                    if filter_tokens.is_empty() {
                        return Err(CardTextError::ParseError(format!(
                            "missing object phrase in static condition (clause: '{display}')"
                        )));
                    }
                    let filter = parse_permanent_card_count_filter(filter_tokens)
                        .or_else(|| parse_object_filter(filter_tokens, false).ok())
                        .ok_or_else(|| {
                            CardTextError::ParseError(format!(
                                "unsupported counted object phrase in static condition (clause: '{display}')"
                            ))
                        })?;
                    return Ok(crate::ConditionExpr::CountComparison {
                        count: AnthemCountExpression::MatchingFilter(filter),
                        comparison: shape.comparison,
                        display: Some(display.clone()),
                    });
                }
            }
        }
        Ok(None) => {}
    }

    let count_condition_tokens =
        crate::runtime_backend::grammar::leaf::parse_leaf_static_condition_intro_prefix_tokens(
            &tokens,
        )
        .map(|parsed| parsed.rest)
        .unwrap_or(&tokens);

    if let Some(control_condition) =
        crate::runtime_backend::grammar::conditions::parse_control_condition(
            count_condition_tokens,
            crate::runtime_backend::grammar::conditions::ControlConditionOptions {
                allow_that_player: false,
                allow_opponent_players: true,
                allow_defending_player: false,
                bind_filter_controller_to_subject: true,
                allow_different_powers_tail: false,
                default_filter_zone: None,
            },
        )
    {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(control_condition.filter),
            comparison: control_condition.comparison,
            display: Some(display.clone()),
        });
    }

    if let Some(ownership_condition) =
        crate::runtime_backend::grammar::conditions::parse_ownership_condition(
            count_condition_tokens,
            crate::runtime_backend::grammar::conditions::OwnershipConditionOptions {
                allow_opponent_players: true,
                bind_filter_owner_to_subject: true,
                default_filter_zone: None,
            },
        )
    {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(ownership_condition.filter),
            comparison: ownership_condition.comparison,
            display: Some(display.clone()),
        });
    }

    if let Some(shape) = anthem_grant_grammar::parse_entered_count_condition(&tokens)
        && let Ok(filter) = parse_object_filter(shape.filter_tokens, shape.begins_with_other)
        && (filter.entered_battlefield_this_turn || filter.entered_battlefield_controller.is_some())
    {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison: shape.comparison,
            display: Some(display.clone()),
        });
    }

    match anthem_grant_grammar::parse_source_counter_condition(&tokens) {
        Ok(Some(shape)) => {
            let mut filter = ObjectFilter::source();
            filter.with_counter = Some(match shape.counter_type {
                Some(counter_type) => crate::filter::CounterConstraint::Typed(counter_type),
                None => crate::filter::CounterConstraint::Any,
            });
            return Ok(crate::ConditionExpr::CountComparison {
                count: AnthemCountExpression::MatchingFilter(filter),
                comparison: shape.comparison,
                display: Some(display.clone()),
            });
        }
        Err(anthem_grant_grammar::SourceCounterConditionError::MissingQuantity) => {
            return Err(CardTextError::ParseError(
                "missing quantity in static condition".to_string(),
            ));
        }
        Err(anthem_grant_grammar::SourceCounterConditionError::MissingCounterPhrase) => {
            return Err(CardTextError::ParseError(format!(
                "missing counter phrase in static condition (clause: '{display}')"
            )));
        }
        Err(anthem_grant_grammar::SourceCounterConditionError::UnsupportedTail) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported source-counter condition tail (clause: '{display}')"
            )));
        }
        Ok(None) => {}
    }

    Err(CardTextError::ParseError(format!(
        "unsupported static condition clause (clause: '{display}')"
    )))
}

fn parse_devotion_static_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::ConditionExpr>, CardTextError> {
    use anthem_grant_grammar::{DevotionConditionError, DevotionPlayerKind};

    let shape = match anthem_grant_grammar::parse_devotion_condition_shape(tokens) {
        Ok(shape) => shape,
        Err(error) => {
            let display = crate::runtime_backend::token_word_refs(tokens).join(" ");
            let message = match error {
                DevotionConditionError::UnsupportedPlayer => {
                    format!("unsupported devotion player in static condition (clause: '{display}')")
                }
                DevotionConditionError::UnsupportedColor(word) => format!(
                    "unsupported devotion color '{word}' in static condition (clause: '{display}')"
                ),
                DevotionConditionError::MissingColor => {
                    format!("missing devotion color in static condition (clause: '{display}')")
                }
                DevotionConditionError::UnsupportedComparison => format!(
                    "unsupported devotion comparison in static condition (clause: '{display}')"
                ),
                DevotionConditionError::MissingValue => format!(
                    "missing devotion comparison value in static condition (clause: '{display}')"
                ),
                DevotionConditionError::UnsupportedValue(word) => format!(
                    "unsupported devotion comparison value '{word}' in static condition (clause: '{display}')"
                ),
            };
            return Err(CardTextError::ParseError(message));
        }
    };
    let Some(shape) = shape else {
        return Ok(None);
    };

    let player = match shape.player {
        DevotionPlayerKind::You => PlayerFilter::You,
        DevotionPlayerKind::IteratedPlayer => PlayerFilter::IteratedPlayer,
        DevotionPlayerKind::Opponent => PlayerFilter::Opponent,
    };
    let mut values = shape
        .colors
        .into_iter()
        .map(|color| crate::effect::Value::Devotion {
            player: player.clone(),
            color,
        });
    let Some(mut left) = values.next() else {
        return Ok(None);
    };
    for value in values {
        left = crate::effect::Value::Add(Box::new(left), Box::new(value));
    }

    Ok(Some(crate::ConditionExpr::ValueComparison {
        left,
        operator: shape.operator,
        right: crate::effect::Value::Fixed(shape.amount as i32),
    }))
}

fn parse_conjoined_static_condition_clause(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    for split in anthem_grant_grammar::parse_conjoined_condition_splits(tokens) {
        let Ok(left) = parse_static_condition_clause(split.left_tokens) else {
            continue;
        };
        let right = parse_conjoined_static_condition_clause(split.right_tokens)
            .or_else(|| parse_static_condition_clause(split.right_tokens).ok());
        if let Some(right) = right {
            return Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)));
        }
    }
    None
}

fn parse_cards_drawn_this_turn_static_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let threshold =
        crate::runtime_backend::grammar::anthem_grants::parse_cards_drawn_this_turn_threshold(
            tokens,
        )?;
    let player = anthem_turn_threshold_player_filter(threshold.player);

    Some(crate::ConditionExpr::ValueComparison {
        left: crate::effect::Value::MaxCardsDrawnThisTurn(player),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: crate::effect::Value::Fixed(threshold.count as i32),
    })
}

fn parse_dice_rolled_this_turn_static_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let threshold =
        crate::runtime_backend::grammar::anthem_grants::parse_dice_rolled_this_turn_threshold(
            tokens,
        )?;
    let player = anthem_turn_threshold_player_filter(threshold.player);

    Some(crate::ConditionExpr::ValueComparison {
        left: crate::effect::Value::MaxDiceRolledThisTurn(player),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: crate::effect::Value::Fixed(threshold.count as i32),
    })
}

fn anthem_turn_threshold_player_filter(
    player: crate::runtime_backend::grammar::anthem_grants::TurnThresholdPlayer,
) -> PlayerFilter {
    use crate::runtime_backend::grammar::anthem_grants::TurnThresholdPlayer;
    match player {
        TurnThresholdPlayer::You => PlayerFilter::You,
        TurnThresholdPlayer::Opponent => PlayerFilter::Opponent,
        TurnThresholdPlayer::Any => PlayerFilter::Any,
    }
}

fn parse_cards_in_hand_static_condition(tokens: &[OwnedLexToken]) -> Option<crate::ConditionExpr> {
    crate::runtime_backend::grammar::conditions::parse_player_cards_in_hand_condition(tokens)?
        .condition_expr()
}

fn parse_life_total_static_condition(tokens: &[OwnedLexToken]) -> Option<crate::ConditionExpr> {
    crate::runtime_backend::grammar::conditions::parse_player_life_total_condition(tokens)?
        .condition_expr()
}

fn explicit_source_counter_surface(
    source_words: &[&str],
) -> Option<crate::target::SourceReferenceSurface> {
    crate::runtime_backend::front_end::shared::util::source_reference_surface_for_words(
        source_words,
    )
    .or_else(|| {
        (source_words.len() > 1)
            .then(|| {
                crate::runtime_backend::front_end::shared::util::this_source_surface_for_words(
                    source_words,
                )
            })
            .flatten()
    })
}

fn source_counter_count_expression(
    counter_type: crate::CounterType,
    source_words: &[&str],
) -> AnthemCountExpression {
    if let Some(surface) = explicit_source_counter_surface(source_words) {
        AnthemCountExpression::CountersOnSourceWithSurface {
            counter_type,
            surface,
        }
    } else {
        AnthemCountExpression::CountersOnSource(counter_type)
    }
}

fn anthem_count_expression_from_value(value: Value) -> Option<AnthemCountExpression> {
    match value.into_unhinted() {
        Value::Count(filter) => Some(AnthemCountExpression::MatchingFilter(filter)),
        Value::CountersOnSource(counter_type) => {
            Some(AnthemCountExpression::CountersOnSource(counter_type))
        }
        Value::CountersOn(spec, Some(counter_type))
            if matches!(spec.unhinted(), crate::target::ChooseSpec::Source) =>
        {
            spec.source_reference_surface()
                .cloned()
                .map(
                    |surface| AnthemCountExpression::CountersOnSourceWithSurface {
                        counter_type,
                        surface,
                    },
                )
                .or(Some(AnthemCountExpression::CountersOnSource(counter_type)))
        }
        Value::CountersOn(spec, Some(counter_type)) => match spec.unhinted() {
            crate::target::ChooseSpec::All(filter) => Some(AnthemCountExpression::CountersAmong(
                filter.clone(),
                counter_type,
            )),
            _ => None,
        },
        _ => None,
    }
}

fn anthem_for_each_prefers_specialized_parser(tokens: &[OwnedLexToken]) -> bool {
    let Some(rest) = anthem_grant_grammar::parse_for_each_rest(tokens) else {
        return false;
    };
    // Stickers and attached permanents also use source-relative wording such
    // as "on this Aura"/"attached to it", but they are not counter values.
    // Keep their typed grammar ahead of the generic object/count fallback.
    parse_sticker_count_expression(rest).is_some()
        || anthem_grant_grammar::parse_for_each_special_shape(rest).is_some()
        || parse_commander_cast_count_player(rest).is_some()
}

fn parse_sticker_count_expression(tokens: &[OwnedLexToken]) -> Option<AnthemCountExpression> {
    let shape = anthem_grant_grammar::parse_sticker_count_shape(tokens)?;
    let action = match shape.kind {
        anthem_grant_grammar::StickerCountKind::Any => crate::events::KeywordActionKind::Sticker,
        anthem_grant_grammar::StickerCountKind::PowerToughness => {
            crate::events::KeywordActionKind::PowerToughnessSticker
        }
        anthem_grant_grammar::StickerCountKind::Name => {
            crate::events::KeywordActionKind::NameSticker
        }
        anthem_grant_grammar::StickerCountKind::Art => crate::events::KeywordActionKind::ArtSticker,
        anthem_grant_grammar::StickerCountKind::Ability => {
            crate::events::KeywordActionKind::AbilitySticker
        }
    };
    let source_words = crate::runtime_backend::token_word_refs(shape.source_tokens);
    let surface = (!source_words.is_empty())
        .then(|| explicit_source_counter_surface(&source_words))
        .flatten();
    Some(AnthemCountExpression::StickersOnSource {
        action,
        surface,
        max_name_letters: shape.max_name_letters,
    })
}

pub(crate) fn parse_anthem_for_each_expression(
    tokens: &[OwnedLexToken],
) -> Result<AnthemCountExpression, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(rest) = anthem_grant_grammar::parse_for_each_rest(&tokens) else {
        return Err(CardTextError::ParseError(
            "missing 'for each' in anthem scaling clause".to_string(),
        ));
    };
    if rest.is_empty() {
        return Err(CardTextError::ParseError(
            "missing object phrase after 'for each'".to_string(),
        ));
    }

    if let Some(shape) = anthem_grant_grammar::parse_for_each_special_shape(rest) {
        match shape {
            anthem_grant_grammar::ForEachSpecialShape::AffectedAttackedThisTurn => {
                return Ok(AnthemCountExpression::AffectedAttackedThisTurn);
            }
            anthem_grant_grammar::ForEachSpecialShape::ColorsOfAffected => {
                return Ok(AnthemCountExpression::ColorsOfAffected);
            }
            anthem_grant_grammar::ForEachSpecialShape::CreatureTypesOfAffected => {
                return Ok(AnthemCountExpression::CreatureTypesAmong(
                    ObjectFilter::source(),
                ));
            }
            anthem_grant_grammar::ForEachSpecialShape::BlockingSource => {
                return Ok(AnthemCountExpression::BlockingSource);
            }
            anthem_grant_grammar::ForEachSpecialShape::AttachedToSource { filter_tokens } => {
                let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported attached-object filter in anthem scaling clause (clause: '{}')",
                        crate::runtime_backend::token_word_refs(&tokens).join(" ")
                    ))
                })?;
                return Ok(AnthemCountExpression::AttachedToSource(filter));
            }
            anthem_grant_grammar::ForEachSpecialShape::UnspentGreenManaYouHave => {
                return Ok(AnthemCountExpression::UnspentMana {
                    player: PlayerFilter::You,
                    symbol: crate::mana::ManaSymbol::Green,
                });
            }
        }
    }

    if let Some(aggregate_value) = parse_aggregate_scope_value_lexed(rest) {
        match aggregate_value {
            Value::BasicLandTypesAmong(filter) => {
                return Ok(AnthemCountExpression::BasicLandTypesAmong(filter));
            }
            Value::CreatureTypesAmong(filter) => {
                return Ok(AnthemCountExpression::CreatureTypesAmong(filter));
            }
            _ => {}
        }
    }

    if let Some(player) = parse_commander_cast_count_player(rest) {
        return Ok(AnthemCountExpression::CommanderCastCount(player));
    }

    if let Some(sticker_count) = parse_sticker_count_expression(rest) {
        return Ok(sticker_count);
    }

    if let Some(filter) = parse_compound_anthem_count_filter(rest) {
        return Ok(AnthemCountExpression::MatchingFilter(filter));
    }

    if let Some(counter_clause) =
        crate::runtime_backend::grammar::anthem_grants::parse_source_counter_count_clause(rest)
        && let Some(counter_type) = parse_counter_type_word(counter_clause.counter_type_word)
    {
        let source_words = crate::runtime_backend::token_word_refs(counter_clause.source_tokens);
        if counter_clause.starts_with_source_pronoun
            || explicit_source_counter_surface(&source_words).is_some()
        {
            return Ok(source_counter_count_expression(counter_type, &source_words));
        }
    }

    let filter = parse_object_filter(rest, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported 'for each' filter in anthem clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(&tokens).join(" ")
        ))
    })?;
    Ok(AnthemCountExpression::MatchingFilter(filter))
}

fn parse_compound_anthem_count_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let segments = anthem_grant_grammar::parse_compound_count_segments(tokens)?;
    let mut branches = Vec::new();
    for segment in segments {
        let segment = trim_commas(anthem_grant_grammar::strip_each_or_every(segment));
        if segment.is_empty() {
            return None;
        }
        branches.push(parse_object_filter(&segment, false).ok()?);
    }

    if branches.len() < 2 {
        return None;
    }

    // Repeated "each" clauses are additive authored count domains. Hoist a
    // genuinely shared player scope so every branch keeps that constraint,
    // while the branch-local zones and characteristics remain independently
    // executable.
    let shared_owner = branches.first()?.owner.clone().filter(|owner| {
        branches
            .iter()
            .all(|branch| branch.owner.as_ref() == Some(owner))
    });
    if shared_owner.is_some() {
        for branch in &mut branches {
            branch.owner = None;
        }
    }
    let shared_controller = branches.first()?.controller.clone().filter(|controller| {
        branches
            .iter()
            .all(|branch| branch.controller.as_ref() == Some(controller))
    });
    if shared_controller.is_some() {
        for branch in &mut branches {
            branch.controller = None;
        }
    }

    let mut combined = ObjectFilter::default();
    combined.any_of = branches;
    combined.owner = shared_owner;
    combined.controller = shared_controller;
    combined.set_conjunctive_set_surface(true);
    Some(combined)
}

pub(crate) fn parse_anthem_prefix_condition(
    tokens: &[OwnedLexToken],
    get_idx: usize,
) -> Result<(Option<crate::ConditionExpr>, usize), CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_prefix_condition_shape(tokens, get_idx) else {
        return Ok((None, 0));
    };
    if shape.kind == anthem_grant_grammar::AnthemPrefixConditionKind::DuringTurnsOtherThanYours {
        let subject_start = shape
            .comma_subject_start
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(shape.prefix_end);
        return Ok((
            Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::YourTurn,
            ))),
            subject_start,
        ));
    }
    if shape.kind == anthem_grant_grammar::AnthemPrefixConditionKind::DuringYourTurn {
        let subject_start = shape
            .comma_subject_start
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(shape.prefix_end);
        return Ok((
            Some(crate::ConditionExpr::ActivationTiming(
                crate::ability::ActivationTiming::DuringYourTurn,
            )),
            subject_start,
        ));
    }

    if shape.kind == anthem_grant_grammar::AnthemPrefixConditionKind::AsLongAs {
        let subject_start = shape
            .comma_subject_start
            .or_else(|| infer_as_long_as_subject_start(tokens, get_idx))
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing subject boundary in leading static condition clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
        if subject_start <= shape.prefix_end {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[shape.prefix_end..subject_start]);
        let condition = parse_static_condition_clause(&condition_tokens)?;
        return Ok((Some(condition), subject_start));
    }
    Ok((None, 0))
}

fn infer_as_long_as_subject_start(tokens: &[OwnedLexToken], action_idx: usize) -> Option<usize> {
    if action_idx <= 3 {
        return None;
    }

    for subject_start in anthem_grant_grammar::parse_word_token_candidates(tokens, 4, action_idx) {
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        if condition_tokens.is_empty() {
            continue;
        }
        if parse_static_condition_clause(&condition_tokens).is_err() {
            continue;
        }

        let subject_tokens = trim_commas(&tokens[subject_start..action_idx]);
        if subject_tokens.is_empty() {
            continue;
        }
        if parse_anthem_subject(&subject_tokens).is_ok() {
            return Some(subject_start);
        }
    }

    None
}

fn bind_unique_count_condition_anthem_subject(
    subject: &mut AnthemSubjectAst,
    condition: Option<&crate::ConditionExpr>,
) {
    let AnthemSubjectAst::Filter(subject_filter) = subject else {
        return;
    };
    let Some(crate::ConditionExpr::CountComparison {
        count: AnthemCountExpression::MatchingFilter(antecedent),
        comparison,
        ..
    }) = condition
    else {
        return;
    };
    if *comparison != crate::effect::Comparison::Equal(1) {
        return;
    }

    crate::runtime_backend::condition_antecedent::bind_condition_filter_antecedent(
        subject_filter,
        antecedent,
    );
}

pub(crate) fn parse_anthem_clause(
    tokens: &[OwnedLexToken],
    get_idx: usize,
    tail_end: usize,
) -> Result<ParsedAnthemClause, CardTextError> {
    let (prefix_condition, subject_start) = parse_anthem_prefix_condition(tokens, get_idx)?;
    let prefix_shape = anthem_grant_grammar::parse_prefix_condition_shape(tokens, get_idx);
    let prefix_attached_subject = if let Some(shape) = prefix_shape
        && shape.kind == anthem_grant_grammar::AnthemPrefixConditionKind::AsLongAs
        && subject_start > shape.prefix_end
    {
        infer_attached_subject_filter_from_condition_tokens(
            &tokens[shape.prefix_end..subject_start],
        )
    } else {
        None
    };
    let subject_tokens = trim_commas(&tokens[subject_start..get_idx]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing anthem subject (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }
    let set_quantifier_surface = match subject_tokens.first().and_then(OwnedLexToken::as_word) {
        Some("all") => Some(ironsmith_core::SetQuantifierSurface::All),
        Some("each") => Some(ironsmith_core::SetQuantifierSurface::Each),
        _ => None,
    };

    let modifier_shape = anthem_grant_grammar::parse_modifier_shape(tokens, get_idx, tail_end)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in anthem clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            ))
        })?;
    let modifier_token = modifier_shape.modifier_word;
    let tail_tokens = trim_edge_punctuation(modifier_shape.tail_tokens);
    let (tail_tokens, maximum_modifier) =
        anthem_grant_grammar::split_trailing_modifier_maximum(&tail_tokens);
    let trailing_condition = anthem_grant_grammar::split_trailing_as_long_as_clause(tail_tokens);
    let anthem_tail_tokens = trailing_condition
        .map(|split| split.keyword_tokens)
        .unwrap_or(tail_tokens);
    let mut explicit_values = None;
    let (raw_power, raw_toughness) = match parse_pt_modifier_values(modifier_token) {
        Ok(values) => values,
        Err(_) => {
            if let Some(values) = parse_dynamic_xy_anthem_values(modifier_token, anthem_tail_tokens)
            {
                explicit_values = Some(values);
                (Value::Fixed(0), Value::Fixed(0))
            } else {
                return Err(CardTextError::ParseError(format!(
                    "invalid power/toughness modifier in anthem clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
        }
    };
    let mut scale: Option<AnthemCountExpression> = None;
    let mut value_scale: Option<Value> = None;
    let mut count_uses_where_x = false;
    let mut suffix_condition: Option<crate::ConditionExpr> = None;
    let mut suffix_attached_subject: Option<ObjectFilter> = None;
    if let Some(split) = trailing_condition {
        suffix_attached_subject =
            infer_attached_subject_filter_from_condition_tokens(split.condition_tokens);
        suffix_condition = Some(parse_static_condition_clause(split.condition_tokens)?);
    }
    if explicit_values.is_none() && !anthem_tail_tokens.is_empty() {
        match anthem_grant_grammar::parse_tail_shape(anthem_tail_tokens) {
            Some(anthem_grant_grammar::AnthemTailShape::ForEach(tail)) => {
                if anthem_for_each_prefers_specialized_parser(tail) {
                    scale = Some(parse_anthem_for_each_expression(tail)?);
                } else {
                    let words = crate::runtime_backend::token_word_refs(tail);
                    if let Some((value, used)) = parse_for_each_count_value_words(&words)
                        && used == words.len()
                    {
                        if matches!(value.unhinted(), Value::PartySize(_))
                            || matches!(value.unhinted(), Value::CountersOn(_, None))
                        {
                            value_scale = Some(
                                value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
                            );
                        } else if let Some(count_expression) =
                            anthem_count_expression_from_value(value)
                        {
                            scale = Some(count_expression);
                        } else {
                            scale = Some(parse_anthem_for_each_expression(tail)?);
                        }
                    } else {
                        scale = Some(parse_anthem_for_each_expression(tail)?);
                    }
                }
            }
            Some(anthem_grant_grammar::AnthemTailShape::WhereX(tail)) => {
                count_uses_where_x = true;
                let x_value = parse_value_binding_clause(tail).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported where-x anthem clause (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    ))
                })?;
                if matches!(x_value.unhinted(), Value::PartySize(_)) {
                    value_scale = Some(x_value);
                } else {
                    scale = Some(match x_value {
                        Value::Count(filter) => AnthemCountExpression::MatchingFilter(filter),
                        Value::GreatestManaValue(filter) => {
                            AnthemCountExpression::GreatestManaValueAmong(filter)
                        }
                        value if anthem_count_expression_from_value(value.clone()).is_some() => {
                            anthem_count_expression_from_value(value)
                                .expect("checked anthem count expression")
                        }
                        Value::BasicLandTypesAmong(filter) => {
                            AnthemCountExpression::BasicLandTypesAmong(filter)
                        }
                        Value::CreatureTypesAmong(filter) => {
                            AnthemCountExpression::CreatureTypesAmong(filter)
                        }
                        Value::Speed(player) => AnthemCountExpression::PlayerSpeed(player),
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported where-x anthem value (clause: '{}')",
                                crate::runtime_backend::token_word_refs(tokens).join(" ")
                            )));
                        }
                    });
                }
            }
            Some(anthem_grant_grammar::AnthemTailShape::AsLongAs { condition_tokens }) => {
                suffix_attached_subject =
                    infer_attached_subject_filter_from_condition_tokens(condition_tokens);
                suffix_condition = Some(parse_static_condition_clause(condition_tokens)?);
            }
            None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing anthem clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
        }
    }

    let tail_words = crate::runtime_backend::token_word_refs(anthem_tail_tokens);
    let counts_cards_in_affected_controller_hand = tail_words.windows(3).any(|words| {
        matches!(
            words,
            ["its", "controller", "hand"]
                | ["its", "controllers", "hand"]
                | ["its", "controller's", "hand"]
        )
    });
    if counts_cards_in_affected_controller_hand
        && let Some(AnthemCountExpression::MatchingFilter(filter)) = scale.as_mut()
    {
        filter.zone = Some(Zone::Hand);
        filter.owner = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
    }

    let attached_subject_filter = prefix_attached_subject
        .as_ref()
        .or(suffix_attached_subject.as_ref());
    let mut subject =
        parse_anthem_subject_with_attached_fallback(&subject_tokens, attached_subject_filter)?;

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_prefix), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        (Some(condition), None) | (None, Some(condition)) => Some(condition),
        (None, None) => None,
    }
    .map(|condition| bind_attachment_condition_to_subject(condition, &subject));
    bind_unique_count_condition_anthem_subject(&mut subject, condition.as_ref());

    let has_dynamic_component = matches!(raw_power, Value::X | Value::XTimes(_))
        || matches!(raw_toughness, Value::X | Value::XTimes(_));
    let scale_fixed_components =
        (scale.is_some() || value_scale.is_some()) && !has_dynamic_component;
    let resolve_anthem_value = |component: Value,
                                scale_expr: Option<&AnthemCountExpression>,
                                value_scale: Option<&Value>,
                                scale_fixed_components: bool|
     -> Result<AnthemValue, CardTextError> {
        let dynamic_scaled = |multiplier: i32| match (multiplier, value_scale) {
            (0, _) => Some(AnthemValue::Fixed(0)),
            (_, None) => None,
            (1, Some(value)) => Some(AnthemValue::Dynamic(value.clone())),
            (multiplier, Some(value)) => {
                let scaled = Value::Scaled(Box::new(value.clone()), multiplier);
                let scaled = if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach) {
                    scaled.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach)
                } else {
                    scaled
                };
                Some(AnthemValue::Dynamic(scaled))
            }
        };
        match component {
            Value::Fixed(value) => Ok(if scale_fixed_components {
                dynamic_scaled(value).unwrap_or_else(|| {
                    scale_expr
                        .map(|scale_expr| AnthemValue::scaled(value, scale_expr.clone()))
                        .unwrap_or(AnthemValue::Fixed(value))
                })
            } else {
                AnthemValue::Fixed(value)
            }),
            Value::X => {
                if let Some(value) = dynamic_scaled(1) {
                    Ok(value)
                } else if let Some(scale_expr) = scale_expr {
                    Ok(AnthemValue::scaled(1, scale_expr.clone()))
                } else {
                    Err(CardTextError::ParseError(format!(
                        "unsupported X power/toughness modifier without count expression (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )))
                }
            }
            Value::XTimes(multiplier) => {
                if let Some(value) = dynamic_scaled(multiplier) {
                    Ok(value)
                } else if let Some(scale_expr) = scale_expr {
                    Ok(AnthemValue::scaled(multiplier, scale_expr.clone()))
                } else {
                    Err(CardTextError::ParseError(format!(
                        "unsupported X power/toughness modifier without count expression (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )))
                }
            }
            _ => Err(CardTextError::ParseError(format!(
                "invalid power/toughness modifier in anthem clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            ))),
        }
    };

    let (mut power, mut toughness) = if let Some((power, toughness)) = explicit_values {
        (power, toughness)
    } else {
        (
            resolve_anthem_value(
                raw_power,
                scale.as_ref(),
                value_scale.as_ref(),
                scale_fixed_components,
            )?,
            resolve_anthem_value(
                raw_toughness,
                scale.as_ref(),
                value_scale.as_ref(),
                scale_fixed_components,
            )?,
        )
    };

    // When the anthem affects multiple creatures (subject is a filter rather
    // than "this creature"), any "... on/attached to it" count expression
    // refers to the affected creature, not the anthem source. Promote those
    // source-relative expressions so the runtime evaluates the count
    // per-creature.
    if matches!(subject, AnthemSubjectAst::Filter(_)) {
        promote_attached_to_affected(&mut power);
        promote_attached_to_affected(&mut toughness);
        promote_counters_on_affected(&mut power);
        promote_counters_on_affected(&mut toughness);
    }
    if let Some(maximum) = maximum_modifier {
        power = apply_anthem_modifier_maximum(power, maximum)?;
        toughness = apply_anthem_modifier_maximum(toughness, maximum)?;
    }

    parser_trace_stack("parse_static:anthem-clause:matched", tokens);
    Ok(ParsedAnthemClause {
        subject,
        power,
        toughness,
        condition,
        set_quantifier_surface,
        count_uses_where_x,
    })
}

fn apply_anthem_modifier_maximum(
    value: AnthemValue,
    maximum: i32,
) -> Result<AnthemValue, CardTextError> {
    match value {
        AnthemValue::Fixed(0) => Ok(AnthemValue::Fixed(0)),
        AnthemValue::PerCount { multiplier, count } => {
            Ok(AnthemValue::scaled_capped(multiplier, count, maximum))
        }
        other => Err(CardTextError::ParseError(format!(
            "unsupported maximum on anthem value {other:?}"
        ))),
    }
}

fn parse_dynamic_xy_anthem_values(
    modifier_token: &str,
    tail_tokens: &[OwnedLexToken],
) -> Option<(AnthemValue, AnthemValue)> {
    let (power_raw, toughness_raw) = split_pt_modifier_components(modifier_token).ok()?;
    let power_var = pt_modifier_variable(power_raw)?;
    let toughness_var = pt_modifier_variable(toughness_raw)?;
    let bindings = parse_where_x_y_bindings(tail_tokens)?;
    let value_for = |var| match var {
        'x' => Some(AnthemValue::Dynamic(bindings.0.clone())),
        'y' => Some(AnthemValue::Dynamic(bindings.1.clone())),
        _ => None,
    };
    Some((value_for(power_var)?, value_for(toughness_var)?))
}

fn pt_modifier_variable(raw: &str) -> Option<char> {
    let value = strip_leading_plus_char(raw).trim();
    match value {
        "x" | "X" => Some('x'),
        "y" | "Y" => Some('y'),
        _ => None,
    }
}

fn parse_where_x_y_bindings(tokens: &[OwnedLexToken]) -> Option<(Value, Value)> {
    let shape = anthem_grant_grammar::parse_where_x_y_bindings_shape(tokens)?;
    let x_words = crate::runtime_backend::lexer::token_word_refs(shape.x_tokens);
    let y_words = crate::runtime_backend::lexer::token_word_refs(shape.y_tokens);
    let (x_value, x_used) = parse_value_expr_words(&x_words)?;
    let (y_value, y_used) = parse_value_expr_words(&y_words)?;
    (x_used == x_words.len() && y_used == y_words.len()).then_some((x_value, y_value))
}

/// When an anthem targets a filter of creatures (not just the source),
/// "attached to it" refers to the affected creature, not the source.
/// Promote `AttachedToSource` -> `AttachedToAffected` in the anthem value.
fn promote_attached_to_affected(value: &mut AnthemValue) {
    if let AnthemValue::PerCount {
        count: count @ AnthemCountExpression::AttachedToSource(_),
        ..
    } = value
    {
        // Extract the inner filter and replace with AttachedToAffected.
        let AnthemCountExpression::AttachedToSource(filter) = std::mem::replace(
            count,
            AnthemCountExpression::AttachedToAffected(ObjectFilter::default()),
        ) else {
            unreachable!()
        };
        *count = AnthemCountExpression::AttachedToAffected(filter);
    }
}

fn promote_counters_on_affected(value: &mut AnthemValue) {
    if let AnthemValue::PerCount {
        count: count @ AnthemCountExpression::CountersOnSource(_),
        ..
    } = value
    {
        let AnthemCountExpression::CountersOnSource(counter_type) = std::mem::replace(
            count,
            AnthemCountExpression::CountersOnAffected(crate::CounterType::Charge),
        ) else {
            unreachable!()
        };
        *count = AnthemCountExpression::CountersOnAffected(counter_type);
    }
}

pub(crate) fn build_anthem_static_ability(clause: &ParsedAnthemClause) -> StaticAbility {
    StaticAbility::new(build_anthem(clause))
}

fn build_anthem(clause: &ParsedAnthemClause) -> Anthem {
    let mut anthem = match &clause.subject {
        AnthemSubjectAst::Source => Anthem::for_source(0, 0),
        AnthemSubjectAst::Filter(filter) => Anthem::new(filter.clone(), 0, 0),
    }
    .with_values(clause.power.clone(), clause.toughness.clone())
    .with_set_quantifier_surface(clause.set_quantifier_surface)
    .with_count_uses_where_x(clause.count_uses_where_x);

    if let Some(condition) = &clause.condition {
        anthem = anthem.with_condition(condition.clone());
    }

    anthem
}

#[derive(Debug, Clone)]
pub(crate) struct TypeColorAdditionClause {
    pub(crate) added_colors: ColorSet,
    pub(crate) set_colors: ColorSet,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
}

pub(crate) fn parse_type_color_addition_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<TypeColorAdditionClause>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_type_color_addition_shape(tokens) else {
        return Ok(None);
    };
    let words = crate::runtime_backend::token_word_refs(tokens);
    // An unscoped copular tail ("and is black") replaces colors. Scoped
    // tails ("in addition to its other colors/types") retain the additive
    // behavior handled below. Reject non-color unscoped descriptors here so
    // the dedicated type-changing parsers can keep their layer semantics.
    if shape.scopes.is_empty() {
        let descriptor_word_storage =
            crate::runtime_backend::token_word_refs(shape.descriptor_tokens);
        let descriptor_words = non_article_word_refs_except(&descriptor_word_storage, &["and"]);
        if descriptor_words.is_empty() {
            return Ok(None);
        }
        let mut set_colors = ColorSet::new();
        for descriptor in descriptor_words {
            let Some(color) = parse_color(descriptor) else {
                return Ok(None);
            };
            set_colors = set_colors.union(color);
        }
        return Ok(Some(TypeColorAdditionClause {
            added_colors: ColorSet::new(),
            set_colors,
            card_types: Vec::new(),
            subtypes: Vec::new(),
        }));
    }

    let mut allow_colors = false;
    let mut allow_types = false;
    for scope in shape.scopes {
        match scope {
            anthem_grant_grammar::TypeColorScope::Colors => allow_colors = true,
            anthem_grant_grammar::TypeColorScope::Types { qualifier_tokens } => {
                let qualifiers = crate::runtime_backend::token_word_refs(qualifier_tokens);
                if qualifiers
                    .iter()
                    .all(|word| is_type_scope_qualifier_word(word))
                {
                    allow_types = true;
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported in-addition scope in type/color clause (clause: '{}')",
                        words.join(" ")
                    )));
                }
            }
            anthem_grant_grammar::TypeColorScope::Unsupported { .. } => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported in-addition scope in type/color clause (clause: '{}')",
                    words.join(" ")
                )));
            }
        }
    }
    if !allow_colors && !allow_types {
        return Ok(None);
    }

    let descriptor_word_storage = crate::runtime_backend::token_word_refs(shape.descriptor_tokens);
    let descriptor_words = non_article_word_refs_except(&descriptor_word_storage, &["and"]);
    if descriptor_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing type/color descriptors in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut added_colors = ColorSet::new();
    let mut set_colors = ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            if allow_colors {
                added_colors = added_colors.union(color);
            } else if allow_types {
                // "is black Zombie in addition to its other creature types"
                // sets color while only preserving existing types.
                set_colors = set_colors.union(color);
            } else {
                return Err(CardTextError::ParseError(format!(
                    "color descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                    descriptor,
                    words.join(" ")
                )));
            }
            continue;
        }

        if let Some(card_type) = parse_card_type(descriptor) {
            if allow_types {
                if !card_types.iter().any(|existing| *existing == card_type) {
                    card_types.push(card_type);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "card type descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            if allow_types {
                if !subtypes.iter().any(|existing| *existing == subtype) {
                    subtypes.push(subtype);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "subtype descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in type/color addition clause (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if added_colors.is_empty()
        && set_colors.is_empty()
        && card_types.is_empty()
        && subtypes.is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "missing type/color additions in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(Some(TypeColorAdditionClause {
        added_colors,
        set_colors,
        card_types,
        subtypes,
    }))
}

pub(crate) fn is_type_scope_qualifier_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || matches!(
            word,
            "card" | "creature" | "permanent" | "basic" | "legendary" | "snow" | "nonbasic"
        )
}

pub(crate) fn parse_soulbond_shared_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_soulbond_shared_shape(tokens) else {
        return Ok(None);
    };
    let subject_words = crate::runtime_backend::token_word_refs(shape.subject_tokens);
    let source_like_subject = is_source_reference_words(&subject_words)
        || shape.subject_is_source_pronoun
        || !shape.subject_has_rejected_word;
    if !source_like_subject {
        return Ok(None);
    }
    if let anthem_grant_grammar::SoulbondSharedEffect::PowerToughness { modifier_word } =
        shape.effect
    {
        let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
            CardTextError::ParseError(format!(
                "invalid power/toughness modifier in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        return Ok(Some(vec![
            StaticAbility::soulbond_shared_power_toughness(power, toughness).into(),
        ]));
    }
    if let anthem_grant_grammar::SoulbondSharedEffect::Ability {
        ability_tokens,
        mills_each_opponent_by_toughness,
    } = shape.effect
    {
        if mills_each_opponent_by_toughness {
            let display = display_text_for_tokens(ability_tokens, false);
            let ability = parsed_triggered_ability(
                TriggerSpec::ThisAttacks,
                vec![EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    crate::cards::builders::PlayerAst::Opponent,
                    crate::cards::builders::SubjectVerbActionAst::Mill {
                        count: Value::ToughnessOf(Box::new(ChooseSpec::Source)),
                    },
                )],
                vec![Zone::Battlefield],
                Some(display.clone()),
                None,
                None,
                ReferenceImports::default(),
            );
            return Ok(Some(vec![StaticAbilityAst::SoulbondSharedObjectAbility {
                ability,
            }]));
        }

        if let Some(actions) = parse_ability_line(ability_tokens) {
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            let abilities: Vec<StaticAbility> = actions
                .into_iter()
                .filter_map(|action| keyword_action_to_static_ability(action))
                .collect();
            if abilities.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported shared ability in soulbond clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let shared = abilities
                .into_iter()
                .map(StaticAbility::soulbond_shared_ability)
                .map(StaticAbilityAst::from)
                .collect();
            return Ok(Some(shared));
        }

        if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, .. }) =
            parse_granted_activated_or_triggered_ability_for_gain(ability_tokens, &clause_words)?
        {
            return Ok(Some(vec![StaticAbilityAst::SoulbondSharedObjectAbility {
                ability,
            }]));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported shared ability in soulbond clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    Ok(None)
}

pub(crate) fn parse_anthem_and_type_color_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_anthem_and_addition_shape(tokens) else {
        return Ok(None);
    };
    if shape.temporary {
        return Ok(None);
    }
    let Some(additions) = parse_type_color_addition_clause(shape.addition_tokens)? else {
        return Ok(None);
    };

    let clause = parse_anthem_clause(tokens, shape.get_token, shape.and_token)?;
    let AnthemSubjectAst::Filter(filter) = &clause.subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported source-only type/color addition clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let mut result = vec![build_anthem_static_ability(&clause)];
    if !additions.set_colors.is_empty() {
        result.push(StaticAbility::set_colors(
            filter.clone(),
            additions.set_colors,
        ));
    }
    if !additions.added_colors.is_empty() {
        result.push(StaticAbility::add_colors(
            filter.clone(),
            additions.added_colors,
        ));
    }
    if !additions.card_types.is_empty() {
        result.push(StaticAbility::add_card_types(
            filter.clone(),
            additions.card_types,
        ));
    }
    if !additions.subtypes.is_empty() {
        result.push(StaticAbility::add_subtypes(
            filter.clone(),
            additions.subtypes,
        ));
    }
    Ok(Some(result))
}

pub(crate) fn parse_carried_subject_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_carried_subject_type_addition(tokens) else {
        return Ok(None);
    };
    let Some(mut result) = parse_static_ability_ast_line_lexed_single(shape.first_sentence_tokens)?
    else {
        return Ok(None);
    };
    let Some(additions) = parse_type_color_addition_clause(shape.addition_tokens)? else {
        return Ok(None);
    };
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let clause = ParsedAnthemClause {
        subject,
        power: AnthemValue::Fixed(0),
        toughness: AnthemValue::Fixed(0),
        condition: None,
        set_quantifier_surface: None,
        count_uses_where_x: false,
    };
    push_type_color_additions_for_anthem_subject(&mut result, &clause, additions);
    Ok(Some(result))
}

fn fixed_anthem_clause(
    subject: AnthemSubjectAst,
    power: i32,
    toughness: i32,
    condition: Option<crate::ConditionExpr>,
) -> ParsedAnthemClause {
    ParsedAnthemClause {
        subject,
        power: AnthemValue::Fixed(power),
        toughness: AnthemValue::Fixed(toughness),
        condition,
        set_quantifier_surface: None,
        count_uses_where_x: false,
    }
}

#[cfg(test)]
mod dynamic_anthem_tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn first_spell_cast_from_zone_grant_keeps_typed_turn_and_origin_filter() {
        let tokens = lex_line(
            "The first spell you cast from exile each turn has cascade.",
            0,
        )
        .expect("first-from-exile grant should lex");
        let abilities = parse_granted_keyword_static_line(&tokens)
            .expect("first-from-exile grant should parse")
            .expect("granted-keyword parser should match");
        let [
            StaticAbilityAst::GrantKeywordAction {
                filter,
                action: KeywordAction::Cascade,
                condition: None,
            },
        ] = abilities.as_slice()
        else {
            panic!("expected one typed cascade grant: {abilities:#?}");
        };

        assert_eq!(filter.zone, Some(Zone::Exile));
        assert_eq!(filter.cast_by, Some(PlayerFilter::You));
        assert!(filter.has_mana_cost);
        assert!(filter.first_spell_cast_each_turn);
    }

    #[test]
    fn serial_player_and_object_union_keeps_every_hexproof_subject() {
        let tokens = lex_line(
            "You, planeswalkers you control, and other creatures you control have hexproof.",
            0,
        )
        .expect("serial hexproof union should lex");
        let abilities = parse_granted_keyword_static_line(&tokens)
            .expect("serial hexproof union should parse")
            .expect("granted-keyword parser should match");

        let [
            StaticAbilityAst::Static(player_hexproof),
            StaticAbilityAst::GrantKeywordAction {
                filter,
                action: KeywordAction::Hexproof,
                condition: None,
            },
        ] = abilities.as_slice()
        else {
            panic!("expected player restriction plus one compound object grant: {abilities:#?}");
        };

        assert!(
            format!("{player_hexproof:?}").contains("BeTargetedPlayerFrom(You"),
            "the player member must compile as a typed targeting restriction: {player_hexproof:#?}"
        );
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.any_of.len(), 2);
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| branch.card_types == [CardType::Planeswalker])
        );
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| branch.card_types == [CardType::Creature] && branch.other)
        );
    }

    #[test]
    fn ability_loss_and_untap_restriction_share_the_original_typed_subject() {
        let tokens = lex_line(
            "Enchanted permanent loses all abilities and doesn't untap during its controller's untap step.",
            0,
        )
        .expect("compound attached restriction fixture should lex");
        let abilities = parse_lose_all_abilities_and_doesnt_untap_line(&tokens)
            .expect("compound attached restriction should parse")
            .expect("compound parser should match");
        let [remove, untap] = abilities.as_slice() else {
            panic!("expected exactly two typed static abilities: {abilities:#?}");
        };

        let ironsmith_core::StaticAbilityPayload::RemoveAllAbilities(remove_filter) =
            &remove.payload
        else {
            panic!("expected typed ability removal: {remove:#?}");
        };
        let ironsmith_core::StaticAbilityPayload::RuleRestriction {
            restriction: crate::effect::Restriction::Untap(untap_filter),
            additional_restrictions,
            ..
        } = &untap.payload
        else {
            panic!("expected typed untap restriction: {untap:#?}");
        };
        assert!(additional_restrictions.is_empty());
        assert_eq!(remove_filter, untap_filter);
        assert!(
            remove_filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == "enchanted"),
            "{remove_filter:#?}"
        );
    }

    fn parse_clause(text: &str) -> ParsedAnthemClause {
        let tokens = lex_line(text, 0).expect("anthem fixture should lex");
        let get_idx = tokens
            .iter()
            .position(|token| token.is_word("gets") || token.is_word("get"))
            .expect("get/gets token");
        parse_anthem_clause(&tokens, get_idx, tokens.len()).expect("anthem fixture should parse")
    }

    #[test]
    fn compound_subtype_anthem_subject_requires_every_subtype() {
        let subject_tokens =
            lex_line("Eldrazi Spawn creatures you control", 0).expect("subject should lex");
        let AnthemSubjectAst::Filter(direct_filter) =
            parse_anthem_subject(&subject_tokens).expect("subject should parse")
        else {
            panic!("compound subtype subject must be filtered");
        };
        assert!(direct_filter.subtypes.is_empty(), "{direct_filter:#?}");
        assert_eq!(
            direct_filter.all_subtypes,
            vec![Subtype::Eldrazi, Subtype::Spawn]
        );

        let clause = parse_clause("Eldrazi Spawn creatures you control get +2/+1.");
        let AnthemSubjectAst::Filter(filter) = clause.subject else {
            panic!("compound subtype anthem must have a filtered subject");
        };
        assert!(filter.subtypes.is_empty(), "{filter:#?}");
        assert_eq!(
            filter.all_subtypes,
            vec![Subtype::Eldrazi, Subtype::Spawn]
        );
        assert_eq!(
            filter.description(),
            "an Eldrazi Spawn creature you control"
        );
    }

    #[test]
    fn additive_anthem_count_domains_keep_each_zone_and_shared_owner() {
        let clause = parse_clause(
            "This creature gets +1/+1 for each card in your hand and each foretold card you own in exile.",
        );

        for value in [&clause.power, &clause.toughness] {
            let AnthemValue::PerCount {
                multiplier,
                count: AnthemCountExpression::MatchingFilter(filter),
            } = value
            else {
                panic!("expected a typed additive count filter: {clause:#?}");
            };
            assert_eq!(*multiplier, 1);
            assert_eq!(filter.owner, Some(PlayerFilter::You));
            assert!(filter.has_conjunctive_set_surface());
            assert_eq!(filter.any_of.len(), 2);
            assert!(filter.any_of.iter().any(|branch| {
                branch.zone == Some(Zone::Hand)
                    && branch.owner.is_none()
                    && !branch.foretold
            }));
            assert!(filter.any_of.iter().any(|branch| {
                branch.zone == Some(Zone::Exile)
                    && branch.owner.is_none()
                    && branch.foretold
            }));
        }
    }

    #[test]
    fn affected_creature_type_scaling_keeps_per_object_identity_and_maximum() {
        let clause = parse_clause(
            "Each non-Human creature you control gets +1/+1 for each of its creature types, to a maximum of 10.",
        );
        for value in [&clause.power, &clause.toughness] {
            let AnthemValue::CappedPerCount {
                multiplier,
                count: AnthemCountExpression::CreatureTypesAmong(filter),
                maximum,
            } = value
            else {
                panic!("expected capped per-creature-type scaling: {clause:#?}");
            };
            assert_eq!(*multiplier, 1);
            assert_eq!(*maximum, 10);
            assert!(filter.source, "{filter:#?}");
        }
    }

    #[test]
    fn nested_this_token_anthem_subject_is_the_ability_source() {
        let tokens = lex_line(
            "This token gets +1/+1 for each card named Sound the Call in each graveyard.",
            0,
        )
        .expect("nested token anthem should lex");
        let get_idx = tokens
            .iter()
            .position(|token| token.is_word("gets"))
            .expect("gets token");
        let (clause, loss) =
            crate::parse_loss::capture(|| parse_anthem_clause(&tokens, get_idx, tokens.len()));
        let clause = clause.expect("nested token anthem should parse");

        assert!(matches!(clause.subject, AnthemSubjectAst::Source));
        assert!(
            !loss.is_lossy(),
            "source recognition must not require suffix recovery: {}",
            loss.reasons_text()
        );
    }

    #[test]
    fn party_scaled_anthem_keeps_typed_party_value() {
        let tokens = lex_line(
            "Equipped creature gets +1/+0 for each creature in your party.",
            0,
        )
        .expect("party anthem fixture should lex");
        let get_idx = tokens
            .iter()
            .position(|token| token.is_word("gets"))
            .expect("gets token");
        let clause =
            parse_anthem_clause(&tokens, get_idx, tokens.len()).expect("party anthem should parse");

        let AnthemValue::Dynamic(power) = &clause.power else {
            panic!("expected dynamic PartySize power, got {:?}", clause.power);
        };
        assert!(power.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
        assert_eq!(power.unhinted(), &Value::PartySize(PlayerFilter::You));
        assert_eq!(clause.toughness, AnthemValue::Fixed(0));
    }

    #[test]
    fn commander_cast_count_anthem_precedes_generic_commander_filter() {
        let clause = parse_clause(
            "Creatures you control get +1/+1 for each time you've cast your commander from the command zone this game.",
        );

        assert!(
            matches!(
                clause.power,
                AnthemValue::PerCount {
                    multiplier: 1,
                    count: AnthemCountExpression::CommanderCastCount(PlayerFilter::You),
                }
            ),
            "{clause:#?}"
        );
        assert!(
            matches!(
                clause.toughness,
                AnthemValue::PerCount {
                    multiplier: 1,
                    count: AnthemCountExpression::CommanderCastCount(PlayerFilter::You),
                }
            ),
            "{clause:#?}"
        );
    }

    #[test]
    fn hinted_source_counter_value_converts_to_anthem_count() {
        let source = crate::target::ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this Equipment".to_string(),
                ),
            ),
        );
        let value = Value::CountersOn(Box::new(source), Some(crate::CounterType::Named("rev")))
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo);

        assert!(matches!(
            anthem_count_expression_from_value(value),
            Some(AnthemCountExpression::CountersOnSourceWithSurface {
                counter_type: crate::CounterType::Named("rev"),
                surface: crate::target::SourceReferenceSurface::ThisPermanentType(surface),
            }) if surface == "this Equipment"
        ));
    }

    #[test]
    fn sticker_anthem_precedes_generic_object_counting() {
        let tokens = lex_line(
            "Enchanted creature gets +0/+2 for each name sticker on this Aura with eight or more letters.",
            0,
        )
        .expect("sticker anthem fixture should lex");
        let get_idx = tokens
            .iter()
            .position(|token| token.is_word("gets"))
            .expect("gets token");
        let clause = parse_anthem_clause(&tokens, get_idx, tokens.len())
            .expect("sticker anthem should parse");

        assert!(matches!(
            clause.toughness,
            AnthemValue::PerCount {
                multiplier: 2,
                count: AnthemCountExpression::StickersOnSource {
                    action: crate::events::KeywordActionKind::NameSticker,
                    ..
                },
            }
        ));
    }

    #[test]
    fn affected_controller_hand_count_stays_bound_to_affected_creature() {
        let tokens = lex_line(
            "Enchanted creature gets +1/+1 for each card in its controller's hand.",
            0,
        )
        .expect("controller-hand anthem fixture should lex");
        let get_idx = tokens
            .iter()
            .position(|token| token.is_word("gets"))
            .expect("gets token");
        let clause = parse_anthem_clause(&tokens, get_idx, tokens.len())
            .expect("controller-hand anthem should parse");

        let AnthemValue::PerCount {
            count: AnthemCountExpression::MatchingFilter(filter),
            ..
        } = clause.power
        else {
            panic!("expected a matching-filter anthem count");
        };
        assert_eq!(filter.zone, Some(Zone::Hand));
        assert_eq!(
            filter.owner,
            Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target))
        );
    }

    #[test]
    fn for_each_anthem_keeps_trailing_condition_out_of_count_filter() {
        let clause = parse_clause(
            "This creature gets +1/+1 for each black permanent your opponents control as long as there are seven or more cards in your graveyard.",
        );

        assert!(matches!(clause.subject, AnthemSubjectAst::Source));
        assert_eq!(clause.power, clause.toughness);
        let AnthemValue::PerCount {
            multiplier: 1,
            count: AnthemCountExpression::MatchingFilter(filter),
        } = &clause.power
        else {
            panic!(
                "expected a matching-filter anthem count, got {:?}",
                clause.power
            );
        };
        assert!(matches!(filter.zone, None | Some(Zone::Battlefield)));
        assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
        assert_eq!(filter.owner, None);
        assert!(
            filter
                .colors
                .is_some_and(|colors| colors.contains(Color::Black))
        );
        assert!(matches!(
            clause.condition,
            Some(crate::ConditionExpr::ValueComparison {
                left: Value::CardsInGraveyard(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(7),
            })
        ));

        let plain = parse_clause(
            "This creature gets +1/+1 for each black permanent your opponents control.",
        );
        assert!(plain.condition.is_none());
        assert!(matches!(plain.power, AnthemValue::PerCount { .. }));

        let conditioned = parse_clause(
            "This creature gets +1/+1 as long as there are seven or more cards in your graveyard.",
        );
        assert_eq!(conditioned.power, AnthemValue::Fixed(1));
        assert_eq!(conditioned.toughness, AnthemValue::Fixed(1));
        assert!(conditioned.condition.is_some());
    }

    #[test]
    fn source_keyword_grant_keeps_source_keyword_trailing_condition() {
        let tokens = lex_line(
            "This creature has indestructible as long as it has defender.",
            0,
        )
        .expect("conditional keyword fixture should lex");
        let abilities = parse_granted_keyword_static_line(&tokens)
            .expect("conditional keyword fixture should parse")
            .expect("conditional keyword parser should match");

        let [
            StaticAbilityAst::ConditionalKeywordAction {
                action: KeywordAction::Indestructible,
                condition: crate::ConditionExpr::SourceMatches(filter),
            },
        ] = abilities.as_slice()
        else {
            panic!("unexpected conditional keyword AST: {abilities:#?}");
        };
        assert_eq!(
            filter.static_abilities,
            vec![crate::static_abilities::StaticAbilityId::Defender]
        );
    }
}

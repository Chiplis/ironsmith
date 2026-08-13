use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, IfResultPredicate, OwnedLexToken, PlayerAst,
    PredicateAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey,
    TargetAst,
};
use crate::diagnostics::TextSpan;
use crate::effect::{Until, Value};
use crate::target::{ObjectFilter, ObjectRef, PlayerFilter};
use ironsmith_core::ValueSurfaceHint;

use super::super::effect_ast_traversal::{for_each_nested_effects, for_each_nested_effects_mut};
use super::super::grammar::effects::for_each_shapes::{
    self, ForEachParticipantScope, ManaClauseShape, ModifierTailAction, OpponentSpecialShape,
    RelativeControlClauseShape, WhoClauseShape,
};
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::LexedClause;
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    comparison_to_value_comparison_operator, parse_for_each_count_value_words, parse_target_phrase,
    replace_unbound_x_with_value, value_contains_unbound_x,
};
use super::chain_carry::bind_implicit_player_context;
use super::chain_carry::{parse_effect_chain, parse_effect_chain_inner, remove_first_word};
use super::conditionals::parse_for_each_doesnt_control_lose_game;
use super::dispatch_entry::replace_unbound_x_in_effects_anywhere;

fn prepend_that_player_life_total_subject(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    if !for_each_shapes::starts_life_total_becomes(tokens) {
        return tokens.to_vec();
    }
    prepend_that_player_subject(tokens)
}

fn prepend_that_player_subject(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut rewritten = Vec::with_capacity(tokens.len() + 2);
    rewritten.push(OwnedLexToken::word(
        "that".to_string(),
        TextSpan::synthetic(),
    ));
    rewritten.push(OwnedLexToken::word(
        "players".to_string(),
        TextSpan::synthetic(),
    ));
    rewritten.extend_from_slice(tokens);
    rewritten
}

pub(crate) fn parse_for_each_object_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_object_subject_shape(subject_tokens) else {
        return Ok(None);
    };
    Ok(Some(parse_for_each_object_filter(shape.filter_tokens)?))
}

pub(crate) fn parse_for_each_object_filter(
    filter_tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let mut filter = parse_object_filter(filter_tokens, false)?;
    let words = crate::runtime_backend::token_word_refs(filter_tokens);
    // Quantified subjects use the older family-level object-filter parser,
    // so they do not pass through the grammar filter finalizer that normally
    // restores this exact coordinated Stack domain. Reassert only the
    // grammar-proven terminal noun phrase here; ordinary spell filters must
    // retain their mana-cost predicate and Spell-only domain.
    if words
        .windows(3)
        .any(|window| matches!(window, ["spell" | "spells", "and", "ability" | "abilities"]))
    {
        filter.zone = Some(crate::zone::Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::SpellOrAbility);
        filter.has_mana_cost = false;
        filter.set_conjunctive_set_surface(true);
    }
    let owner_index = words.windows(2).position(|window| window == ["you", "own"]);
    let zone_index = words.iter().position(|word| {
        matches!(
            *word,
            "battlefield" | "graveyard" | "hand" | "library" | "exile" | "command"
        )
    });
    if owner_index
        .zip(zone_index)
        .is_some_and(|(owner, zone)| owner < zone)
    {
        filter.set_owner_before_zone_surface(true);
    }
    let counter_index = words
        .iter()
        .rposition(|word| matches!(*word, "counter" | "counters"));
    if zone_index
        .zip(counter_index)
        .is_some_and(|(zone, counter)| zone < counter)
    {
        filter.set_counter_requirement_after_zone_surface(true);
    }
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("those"))
    {
        filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those));
    }
    Ok(filter)
}

pub(crate) fn parse_for_each_targeted_object_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_target_subject_shape(subject_tokens) else {
        return Ok(None);
    };
    let target = match parse_target_phrase(shape.target_tokens) {
        Ok(target) => target,
        Err(_) => return Ok(None),
    };
    let TargetAst::WithCount(inner, count) = target else {
        return Ok(None);
    };
    let TargetAst::Object(filter, _, _) = *inner else {
        return Ok(None);
    };
    Ok(Some((filter, count)))
}

pub(crate) fn is_target_player_dealt_damage_by_this_turn_subject(words: &[&str]) -> bool {
    for_each_shapes::is_target_player_damage_subject_words(words)
}

pub(crate) fn is_mana_replacement_clause_words(words: &[&str]) -> bool {
    for_each_shapes::parse_mana_clause_shape_words(words) == Some(ManaClauseShape::Replacement)
}

pub(crate) fn is_mana_trigger_additional_clause_words(words: &[&str]) -> bool {
    for_each_shapes::parse_mana_clause_shape_words(words)
        == Some(ManaClauseShape::AdditionalTrigger)
}

pub(crate) fn parse_has_base_power_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_base_power_clause_shape(tokens)? else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;
    Ok(Some(EffectAst::subject_verb_set_base_power(
        shape.power,
        target,
        shape.duration,
    )))
}

pub(crate) fn parse_has_base_power_toughness_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // Copular animation clauses such as "Each of them is a 1/1 Spirit in
    // addition to its other types" carry type/subtype semantics beyond the
    // P/T assignment. Leave those for the complete animation parser instead
    // of consuming only their leading P/T fragment here.
    if super::super::grammar::effects::clause_dispatch_shapes::parse_copular_animation_shape(tokens)
        .is_some()
    {
        return Ok(None);
    }
    let Some(shape) = for_each_shapes::parse_base_power_toughness_clause_shape(tokens)? else {
        return Ok(None);
    };
    // "<subject> lose all abilities and have base power and toughness N/N"
    // — the target phrase must not eat the remove-abilities half.
    let mut target_tokens = shape.target_tokens;
    let mut lose_all_abilities = false;
    {
        let mut end = target_tokens.len();
        if end > 0 && target_tokens[end - 1].is_word("and") {
            end -= 1;
        }
        if end >= 3
            && (target_tokens[end - 3].is_word("lose") || target_tokens[end - 3].is_word("loses"))
            && target_tokens[end - 2].is_word("all")
            && target_tokens[end - 1].is_word("abilities")
        {
            lose_all_abilities = true;
            target_tokens = &target_tokens[..end - 3];
        }
    }
    let target = parse_target_phrase(target_tokens)?;
    if lose_all_abilities && shape.where_x_tokens.is_none() {
        let set_pt = EffectAst::subject_verb_set_base_power_toughness(
            shape.power.clone(),
            shape.toughness.clone(),
            target.clone(),
            shape.duration.clone(),
        );
        let remove = EffectAst::subject_verb_remove_abilities_from_target(
            target,
            Vec::new(),
            shape.duration,
        );
        return Ok(Some(EffectAst::Sequence {
            effects: vec![remove, set_pt],
        }));
    }
    let (mut power, mut toughness) = (shape.power, shape.toughness);
    if let Some(where_x_tokens) = shape.where_x_tokens {
        let clause = LexedClause::new(tokens).text();
        if !value_contains_unbound_x(&power) && !value_contains_unbound_x(&toughness) {
            return Err(CardTextError::ParseError(format!(
                "where-X base power/toughness clause missing X value (clause: '{}')",
                clause
            )));
        }
        let x_value = parse_value_binding_clause(where_x_tokens)
            .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported where-X base power/toughness clause (clause: '{}')",
                    clause
                ))
            })?;
        power = replace_unbound_x_with_value(power, &x_value, &clause)?;
        toughness = replace_unbound_x_with_value(toughness, &x_value, &clause)?;
    }
    Ok(Some(EffectAst::subject_verb_set_base_power_toughness(
        power,
        toughness,
        target,
        shape.duration,
    )))
}

pub(crate) fn parse_get_for_each_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_target_subject_shape(tokens) else {
        return Ok(None);
    };
    if let Some(value) = crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_count_value(tokens)
    {
        return Ok(Some(value.with_surface_hint(ValueSurfaceHint::ForEach)));
    }
    let words = LexedClause::new(tokens).word_refs();
    let Some((value, _)) = parse_for_each_count_value_words(&words) else {
        return Err(CardTextError::ParseError(
            "missing filter after 'for each' in gets clause".to_string(),
        ));
    };
    // The word fallback intentionally handles the broad dynamic-value
    // vocabulary, but it loses punctuation. When the result is an ordinary
    // object count, reparse the exact token slice so serial subtype-list
    // provenance (commas plus terminal "or") survives into rendering.
    let exact_surface_filter = |original: ObjectFilter| {
        let Ok(reparsed) = parse_object_filter(shape.target_tokens, false) else {
            return original;
        };
        // Exact token reparsing is only a surface enrichment. Preserve a
        // specialized semantic count (for example, the cast-time snapshot tag
        // for "modified creatures you controlled as you cast this spell")
        // whenever the ordinary object-filter parse denotes a different set.
        if reparsed == original {
            reparsed
        } else {
            original
        }
    };
    let value = match value {
        Value::Count(original) => Value::Count(exact_surface_filter(original)),
        Value::CountScaled(original, multiplier) => {
            Value::CountScaled(exact_surface_filter(original), multiplier)
        }
        other => other,
    };
    Ok(Some(value.with_surface_hint(ValueSurfaceHint::ForEach)))
}

pub(crate) fn parse_get_modifier_values_with_tail(
    modifier_tokens: &[OwnedLexToken],
    power: Value,
    toughness: Value,
) -> Result<(Value, Value, Until, Option<crate::ConditionExpr>), CardTextError> {
    let clause = LexedClause::new(modifier_tokens).text();
    let mut out_power = power;
    let mut out_toughness = toughness;
    let shape = for_each_shapes::parse_modifier_tail_shape(modifier_tokens);

    if let ModifierTailAction::DynamicForEach(count_tokens) = shape.action {
        let Some(count) = parse_get_for_each_count_value(count_tokens)? else {
            return Err(CardTextError::ParseError(
                "missing filter after 'for each' in gets clause".to_string(),
            ));
        };
        let scale_modifier = |modifier: Value| -> Result<Value, CardTextError> {
            match modifier {
                Value::Fixed(0) => Ok(Value::Fixed(0)),
                Value::Fixed(1) => Ok(count.clone()),
                Value::Fixed(multiplier) => Ok(Value::Scaled(Box::new(count.clone()), multiplier)),
                other if value_contains_unbound_x(&other) => {
                    replace_unbound_x_with_value(other, &count, &clause)
                }
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported dynamic gets-for-each clause (clause: '{}')",
                    clause
                ))),
            }
        };
        out_power = scale_modifier(out_power)?;
        out_toughness = scale_modifier(out_toughness)?;
        return Ok((out_power, out_toughness, shape.duration, shape.condition));
    }

    let ModifierTailAction::WhereX(binding_tokens) = shape.action else {
        return match shape.action {
            ModifierTailAction::Complete => {
                Ok((out_power, out_toughness, shape.duration, shape.condition))
            }
            ModifierTailAction::Unsupported => Err(CardTextError::ParseError(format!(
                "unsupported trailing gets clause (clause: '{}')",
                clause
            ))),
            ModifierTailAction::DynamicForEach(_) | ModifierTailAction::WhereX(_) => unreachable!(),
        };
    };
    if !value_contains_unbound_x(&out_power) && !value_contains_unbound_x(&out_toughness) {
        return Err(CardTextError::ParseError(format!(
            "where-X gets clause missing X modifier (clause: '{}')",
            clause
        )));
    }
    let x_value = parse_value_binding_clause(binding_tokens)
        .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-X gets clause (clause: '{}')",
                clause
            ))
        })?;
    out_power = replace_unbound_x_with_value(out_power, &x_value, &clause)?;
    out_toughness = replace_unbound_x_with_value(out_toughness, &x_value, &clause)?;
    Ok((out_power, out_toughness, shape.duration, shape.condition))
}

pub(crate) fn force_implicit_token_controller_you(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenWithMods { player, .. }
                    | SubjectVerbActionAst::CreateTokenCopy { player, .. }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. },
                ..
            }) => {
                if matches!(*player, PlayerAst::Implicit) {
                    *player = PlayerAst::You;
                }
            }
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                force_implicit_token_controller_you(nested);
            }),
        }
    }
}

fn bind_implicit_choose_chooser(effects: &mut [EffectAst], chooser: PlayerAst) {
    for effect in effects {
        match effect {
            EffectAst::ChooseObjects { player, .. }
            | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. }
            | EffectAst::ChooseObjectsBottomOfLibrary { player, .. }
            | EffectAst::ChooseObjectsTopOfLibrary { player, .. }
            | EffectAst::ChooseObjectsAcrossZones { player, .. }
            | EffectAst::ChooseTaggedObjectsInZone { player, .. }
                if matches!(*player, PlayerAst::Implicit) =>
            {
                *player = chooser;
            }
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                bind_implicit_choose_chooser(nested, chooser);
            }),
        }
    }
}

/// Give a participant's standalone choice a stable aggregate tag.
///
/// An implicit `__it__` choice is normally iteration-local: the runtime
/// replaces it for the next participant so an immediate nested consumer sees
/// only that participant's choice. When the participant clause consists only
/// of the choice, however, a later sentence can refer to the union ("chosen
/// this way"). Use a source-anchored explicit tag so those selections
/// accumulate across the participant loop without colliding with another
/// standalone participant choice in the same ability.
fn stabilize_standalone_participant_choice_tag(
    effects: &mut [EffectAst],
    source_tokens: &[OwnedLexToken],
) {
    let [effect] = effects else {
        return;
    };
    let tag = match effect {
        EffectAst::ChooseObjects { tag, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { tag, .. }
        | EffectAst::ChooseObjectsBottomOfLibrary { tag, .. }
        | EffectAst::ChooseObjectsTopOfLibrary { tag, .. }
        | EffectAst::ChooseObjectsAcrossZones { tag, .. }
        | EffectAst::ChooseTaggedObjectsInZone { tag, .. } => tag,
        _ => return,
    };
    if tag.as_str() != IT_TAG {
        return;
    }
    let anchor = source_tokens
        .first()
        .map(|token| token.span)
        .unwrap_or_else(TextSpan::synthetic);
    *tag = TagKey::from(format!(
        "participant_choice_l{}_s{}",
        anchor.line, anchor.start
    ));
}

fn tagged_predicate(filter_tokens: Option<&[OwnedLexToken]>) -> Option<PredicateAst> {
    let filter = parse_object_filter(filter_tokens?, false).ok()?;
    Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::That,
        tag: TagKey::from(IT_TAG),
        filter,
        mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
    })
}

fn parse_maybe_effects(
    tokens: &[OwnedLexToken],
    parse_inner: bool,
    scope_may_to_that_player: bool,
) -> Result<Vec<EffectAst>, CardTextError> {
    if !for_each_shapes::contains_may(tokens) {
        return if parse_inner {
            parse_effect_chain_inner(tokens)
        } else {
            parse_effect_chain(tokens)
        };
    }
    let stripped = remove_first_word(tokens);
    let scoped;
    let may_tokens = if scope_may_to_that_player {
        scoped = prepend_that_player_subject(&stripped);
        scoped.as_slice()
    } else {
        stripped.as_slice()
    };
    let effects = parse_effect_chain_inner(may_tokens)?;
    Ok(vec![EffectAst::May { effects }])
}

fn find_word_phrase(tokens: &[OwnedLexToken], phrase: &[&str]) -> Option<usize> {
    tokens.windows(phrase.len()).position(|window| {
        window
            .iter()
            .zip(phrase)
            .all(|(token, word)| token.is_word(word))
    })
}

/// Normalize Oracle's prose upper bound
/// "a number of <objects> less than or equal to the difference" to the typed
/// dynamic-choice surface understood by the ordinary search parser. The
/// caller subsequently binds X to the actual count difference, so runtime
/// semantics do not depend on the authored shorthand.
fn rewrite_difference_bounded_search(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    const COUNT_PREFIX: &[&str] = &["a", "number", "of"];
    const DIFFERENCE_SUFFIX: &[&str] = &["less", "than", "or", "equal", "to", "the", "difference"];

    let prefix = find_word_phrase(tokens, COUNT_PREFIX)?;
    let selector_start = prefix + COUNT_PREFIX.len();
    let suffix_relative = find_word_phrase(tokens.get(selector_start..)?, DIFFERENCE_SUFFIX)?;
    let suffix = selector_start + suffix_relative;
    if suffix == selector_start {
        return None;
    }

    let mut rewritten =
        Vec::with_capacity(tokens.len() + 3 - COUNT_PREFIX.len() - DIFFERENCE_SUFFIX.len());
    rewritten.extend_from_slice(&tokens[..prefix]);
    for word in ["up", "to", "x"] {
        rewritten.push(OwnedLexToken::word(word.to_string(), TextSpan::synthetic()));
    }
    rewritten.extend_from_slice(&tokens[selector_start..suffix]);
    rewritten.extend_from_slice(&tokens[suffix + DIFFERENCE_SUFFIX.len()..]);
    Some(rewritten)
}

/// Recover an authored where-X binding from the body of a participant-scoped
/// clause. These clauses are lowered before the ordinary sentence-level
/// where-X pass, so a binding that follows a complete search procedure (for
/// example, after its battlefield destination) would otherwise leave the
/// search's dynamic choice count unbound.
fn parse_participant_body_where_x_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let where_idx = find_word_phrase(tokens, &["where", "x", "is"])?;
    parse_value_binding_clause(&tokens[where_idx..])
        .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
}

fn parse_participant_choice_complement_effects(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut full_clause = Vec::with_capacity(tokens.len() + 2);
    full_clause.push(OwnedLexToken::word(
        "each".to_string(),
        TextSpan::synthetic(),
    ));
    full_clause.push(OwnedLexToken::word(
        "player".to_string(),
        TextSpan::synthetic(),
    ));
    full_clause.extend_from_slice(tokens);

    let Some(effect) = super::parse_choice_complement_subject_verb(&full_clause)? else {
        return Ok(None);
    };
    let EffectAst::ForEachPlayer { effects } = effect else {
        return Ok(None);
    };
    Ok(Some(effects))
}

fn opponent_filter(scope: ForEachParticipantScope) -> Option<PlayerFilter> {
    match scope {
        ForEachParticipantScope::Opponent => Some(PlayerFilter::Opponent),
        ForEachParticipantScope::OpponentExceptDefending => Some(PlayerFilter::excluding(
            PlayerFilter::Opponent,
            PlayerFilter::Defending,
        )),
        ForEachParticipantScope::Player
        | ForEachParticipantScope::PlayerExceptYou
        | ForEachParticipantScope::PlayerExceptTarget
        | ForEachParticipantScope::PlayerExceptItsController
        | ForEachParticipantScope::PlayerOnYourTeam => None,
    }
}

fn player_filter(scope: ForEachParticipantScope) -> Option<PlayerFilter> {
    match scope {
        ForEachParticipantScope::Player => Some(PlayerFilter::Any),
        ForEachParticipantScope::PlayerExceptYou => Some(PlayerFilter::NotYou),
        ForEachParticipantScope::PlayerExceptTarget => Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::target_player(),
        )),
        ForEachParticipantScope::PlayerExceptItsController => Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::ControllerOf(ObjectRef::tagged(TagKey::from(IT_TAG))),
        )),
        ForEachParticipantScope::PlayerOnYourTeam => Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::Opponent,
        )),
        ForEachParticipantScope::Opponent | ForEachParticipantScope::OpponentExceptDefending => {
            None
        }
    }
}

/// In `each other player may copy that spell`, "other" is relative to the
/// player who controls the referenced spell, not necessarily the ability's
/// controller. Keep ordinary `each other player` clauses controller-relative,
/// but anchor this typed stack-copy shape to the triggering stack object.
fn reanchor_other_player_copy_filter(filter: PlayerFilter, effects: &[EffectAst]) -> PlayerFilter {
    if filter != PlayerFilter::NotYou || !effects.iter().any(effect_copies_triggering_stack_object)
    {
        return filter;
    }
    PlayerFilter::excluding(
        PlayerFilter::Any,
        PlayerFilter::AliasedControllerOf(ObjectRef::tagged("triggering")),
    )
}

fn effect_copies_triggering_stack_object(effect: &EffectAst) -> bool {
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell {
                target: TargetAst::Tagged(tag, _),
                ..
            },
            ..
        }) if tag.as_str() == "triggering"
    ) {
        return true;
    }
    let mut found = false;
    for_each_nested_effects(effect, true, |nested| {
        if !found {
            found = nested.iter().any(effect_copies_triggering_stack_object);
        }
    });
    found
}

fn wrap_opponents(filter: &PlayerFilter, effects: Vec<EffectAst>) -> EffectAst {
    if *filter == PlayerFilter::Opponent {
        EffectAst::ForEachOpponent { effects }
    } else {
        EffectAst::ForEachPlayersFiltered {
            filter: filter.clone(),
            effects,
        }
    }
}

fn wrap_players(filter: &PlayerFilter, effects: Vec<EffectAst>) -> EffectAst {
    if *filter == PlayerFilter::Any {
        EffectAst::ForEachPlayer { effects }
    } else {
        EffectAst::ForEachPlayersFiltered {
            filter: filter.clone(),
            effects,
        }
    }
}

fn parse_relative_control_conditional(
    relative: RelativeControlClauseShape<'_>,
    participant_is_actor: bool,
    clause_text: &str,
) -> Result<EffectAst, CardTextError> {
    let mut filter = parse_object_filter(relative.filter_tokens, false)?;
    let mut branch_effects;
    let participant_where_x = parse_participant_body_where_x_value(relative.effect_tokens);
    let participant_choice_effects =
        parse_participant_choice_complement_effects(relative.effect_tokens)?;
    let predicate = if let Some(most_filter_tokens) = relative.fewer_than_most_filter_tokens {
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        let mut most_filter = parse_object_filter(most_filter_tokens, false)?;
        most_filter.controller = Some(PlayerFilter::Any);
        let difference = Value::Add(
            Box::new(Value::GreatestCount(most_filter.clone())),
            Box::new(Value::Scaled(Box::new(Value::Count(filter.clone())), -1)),
        )
        .with_surface_hint(ValueSurfaceHint::Difference);

        let rewritten = rewrite_difference_bounded_search(relative.effect_tokens);
        branch_effects = if let Some(effects) = participant_choice_effects.clone() {
            effects
        } else {
            parse_maybe_effects(
                rewritten.as_deref().unwrap_or(relative.effect_tokens),
                true,
                false,
            )?
        };
        if rewritten.is_some() {
            replace_unbound_x_in_effects_anywhere(&mut branch_effects, &difference, clause_text)?;
        }
        PredicateAst::ValueComparison {
            left: Value::Count(filter),
            operator: crate::effect::ValueComparisonOperator::LessThan,
            right: Value::GreatestCount(most_filter),
        }
    } else if relative.fewer_than_you {
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        let mut your_filter = filter.clone();
        your_filter.controller = Some(PlayerFilter::You);
        branch_effects = if let Some(effects) = participant_choice_effects.clone() {
            effects
        } else {
            parse_maybe_effects(relative.effect_tokens, true, false)?
        };
        PredicateAst::ValueComparison {
            left: Value::Count(filter),
            operator: crate::effect::ValueComparisonOperator::LessThan,
            right: Value::Count(your_filter),
        }
    } else if let Some(comparison) = relative.count_comparison {
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        branch_effects = if let Some(effects) = participant_choice_effects.clone() {
            effects
        } else {
            parse_maybe_effects(relative.effect_tokens, true, false)?
        };
        let (operator, count) =
            comparison_to_value_comparison_operator(comparison).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported for-each control count comparison (clause: '{clause_text}')"
                ))
            })?;
        PredicateAst::ValueComparison {
            left: Value::Count(filter),
            operator,
            right: Value::Fixed(count),
        }
    } else if relative.controls_most {
        branch_effects = if let Some(effects) = participant_choice_effects.clone() {
            effects
        } else {
            parse_maybe_effects(relative.effect_tokens, true, false)?
        };
        PredicateAst::PlayerControlsMost {
            player: PlayerAst::That,
            filter,
        }
    } else {
        branch_effects = if let Some(effects) = participant_choice_effects {
            effects
        } else {
            parse_maybe_effects(relative.effect_tokens, true, false)?
        };
        PredicateAst::PlayerControls {
            player: PlayerAst::That,
            filter,
        }
    };
    if let Some(where_x) = participant_where_x {
        replace_unbound_x_in_effects_anywhere(&mut branch_effects, &where_x, clause_text)?;
    }
    if participant_is_actor {
        for effect in &mut branch_effects {
            bind_implicit_player_context(effect, PlayerAst::That);
        }
    }
    Ok(EffectAst::Conditional {
        predicate,
        if_true: branch_effects,
        if_false: Vec::new(),
    })
}

pub(crate) fn parse_for_each_opponent_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // Voter-relative opponent sets are already represented by an event-
    // populated player tag. Recognize that typed set before the ordinary
    // quantified-opponent path wraps it in a second loop, which would apply
    // the tagged-player action once for every opponent.
    if let Some(mut effects) = super::dispatch_inner::parse_vote_affinity_subject_verb(tokens)? {
        if effects.len() == 1 {
            return Ok(effects.pop());
        }
        return Err(CardTextError::ParseError(
            "voter-relative opponent clause produced multiple outer effects".to_string(),
        ));
    }

    let Some(outer) = for_each_shapes::parse_participant_clause_shape(tokens) else {
        return Ok(None);
    };
    let Some(iteration_filter) = opponent_filter(outer.scope) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    let slot_chooser = if outer.participant_is_actor {
        PlayerAst::That
    } else {
        PlayerAst::You
    };
    if let Some(effects) =
        super::parse_for_each_type_slot_choice_clause(outer.inner_tokens, slot_chooser)?
    {
        return Ok(Some(wrap_opponents(&iteration_filter, effects)));
    }
    if iteration_filter == PlayerFilter::Opponent
        && let Some(effect) = parse_for_each_doesnt_control_lose_game(tokens, true)?
    {
        return Ok(Some(effect));
    }

    if let Some(relative) = for_each_shapes::parse_relative_control_clause_shape(outer.inner_tokens)
    {
        let conditional =
            parse_relative_control_conditional(relative, outer.participant_is_actor, &clause_text)?;
        return Ok(Some(wrap_opponents(&iteration_filter, vec![conditional])));
    }

    if let Some(effect) =
        parse_combat_damage_history_participant(outer.inner_tokens, iteration_filter.clone())?
    {
        return Ok(Some(effect));
    }

    if let Some(special) = for_each_shapes::parse_opponent_special_shape(outer.inner_tokens)? {
        match special {
            OpponentSpecialShape::IgnoreScryOrSurveil => return Ok(None),
            OpponentSpecialShape::ChooseReturnUnlessDraw { target_tokens } => {
                let target = parse_target_phrase(target_tokens)?;
                let return_target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
                return Ok(Some(wrap_opponents(
                    &iteration_filter,
                    vec![
                        EffectAst::subject_verb_target_only(target),
                        EffectAst::UnlessAction {
                            effects: vec![EffectAst::subject_verb_return_to_hand(
                                return_target,
                                false,
                            )],
                            alternative: vec![EffectAst::subject_verb(
                                SubjectVerbRoleAst::AffectedPlayer,
                                PlayerAst::You,
                                SubjectVerbActionAst::Draw {
                                    count: Value::Fixed(1),
                                },
                            )],
                            player: PlayerAst::ItsController,
                        },
                    ],
                )));
            }
            OpponentSpecialShape::LessLifeThanYou { effect_tokens } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each opponent who has less life than you' (clause: '{}')",
                        clause_text
                    )));
                }
                let mut branch_effects = parse_maybe_effects(effect_tokens, false, false)?;
                force_implicit_token_controller_you(&mut branch_effects);
                return Ok(Some(wrap_opponents(
                    &iteration_filter,
                    vec![EffectAst::Conditional {
                        predicate: PredicateAst::PlayerHasLessLifeThanYou {
                            player: PlayerAst::That,
                        },
                        if_true: branch_effects,
                        if_false: Vec::new(),
                    }],
                )));
            }
            OpponentSpecialShape::PoisonCounters {
                count,
                effect_tokens,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each opponent who has ... poison counters' (clause: '{}')",
                        clause_text
                    )));
                }
                let mut branch_effects = parse_effect_chain(effect_tokens)?;
                force_implicit_token_controller_you(&mut branch_effects);
                return Ok(Some(wrap_opponents(
                    &iteration_filter,
                    vec![EffectAst::Conditional {
                        predicate: PredicateAst::PlayerHasPoisonCountersOrMore {
                            player: PlayerAst::That,
                            count,
                        },
                        if_true: branch_effects,
                        if_false: Vec::new(),
                    }],
                )));
            }
        }
    }

    if let Some(who) = for_each_shapes::parse_who_clause_shape(outer.inner_tokens) {
        match who {
            WhoClauseShape::TappedLandForMana { effect_tokens } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each player who tapped a land for mana this turn' (clause: '{}')",
                        clause_text
                    )));
                }
                let branch_effects = parse_maybe_effects(effect_tokens, true, false)?;
                return Ok(Some(EffectAst::ForEachPlayer {
                    effects: vec![EffectAst::Conditional {
                        predicate: PredicateAst::PlayerTappedLandForManaThisTurn {
                            player: PlayerAst::That,
                        },
                        if_true: branch_effects,
                        if_false: Vec::new(),
                    }],
                }));
            }
            WhoClauseShape::Negated {
                effect_tokens,
                tagged_filter_tokens,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect in for each opponent who doesn't clause (clause: '{}')",
                        clause_text
                    )));
                }
                return Ok(Some(EffectAst::ForEachOpponentDoesNot {
                    effects: parse_effect_chain_inner(effect_tokens)?,
                    predicate: tagged_predicate(tagged_filter_tokens),
                }));
            }
            WhoClauseShape::DidThisWay {
                effect_tokens,
                tagged_filter_tokens,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each opponent who ... this way' (clause: '{}')",
                        clause_text
                    )));
                }
                return Ok(Some(EffectAst::ForEachOpponentDid {
                    effects: parse_effect_chain_inner(effect_tokens)?,
                    predicate: tagged_predicate(tagged_filter_tokens),
                    result_predicate: IfResultPredicate::Did,
                }));
            }
            WhoClauseShape::DidAction {
                effect_tokens,
                implicit_player_is_you,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each opponent who does' (clause: '{}')",
                        clause_text
                    )));
                }
                let mut effects = parse_effect_chain_inner(effect_tokens)?;
                let player = if implicit_player_is_you {
                    PlayerAst::You
                } else {
                    PlayerAst::That
                };
                for effect in &mut effects {
                    bind_implicit_player_context(effect, player.clone());
                }
                return Ok(Some(EffectAst::ForEachOpponentDid {
                    effects,
                    predicate: None,
                    result_predicate: IfResultPredicate::AcceptedChoice,
                }));
            }
        }
    }

    let participant_may = outer.participant_is_actor
        && outer
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_word("may"));
    let participant_chooses = for_each_shapes::starts_choose(outer.inner_tokens);
    let normalized = if outer.participant_is_actor && !participant_may && !participant_chooses {
        prepend_that_player_subject(outer.inner_tokens)
    } else {
        prepend_that_player_life_total_subject(outer.inner_tokens)
    };
    let mut effects = parse_maybe_effects(&normalized, false, outer.participant_is_actor)?;
    if !outer.participant_is_actor {
        // The quantified participant is the iteration key, not the actor, in
        // imperative clauses such as "For each opponent, create a token."
        // Resolve the otherwise implicit token controller to the effect
        // controller before lowering enters iterated-player context.
        force_implicit_token_controller_you(&mut effects);
    }
    if participant_chooses {
        bind_implicit_choose_chooser(
            &mut effects,
            if outer.participant_is_actor {
                PlayerAst::That
            } else {
                PlayerAst::You
            },
        );
        stabilize_standalone_participant_choice_tag(&mut effects, outer.inner_tokens);
    }
    Ok(Some(wrap_opponents(&iteration_filter, effects)))
}

pub(crate) fn parse_for_each_target_players_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_target_players_shape(tokens) else {
        return Ok(None);
    };
    if shape.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after target-player each clause (clause: '{}')",
            LexedClause::new(tokens).text()
        )));
    }
    // `target player <action> ... for each <counted set>` contains the same
    // lexical markers as `N target players <qualifier> each <action>`. The
    // shape parser intentionally keeps the qualifier open-ended, so require
    // that its proposed target slice is actually a target phrase before
    // claiming the clause. Otherwise the ordinary action family (for example,
    // discard) must receive the complete `for each` count suffix.
    let Ok(target) = parse_target_phrase(shape.target_tokens) else {
        return Ok(None);
    };
    let filter = match target {
        TargetAst::Player(filter, _) => filter,
        TargetAst::WithCount(inner, _) => match *inner {
            TargetAst::Player(filter, _) => filter,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "expected player target in target-player each clause (clause: '{}')",
                    LexedClause::new(tokens).text()
                )));
            }
        },
        _ => {
            return Err(CardTextError::ParseError(format!(
                "expected player target in target-player each clause (clause: '{}')",
                LexedClause::new(tokens).text()
            )));
        }
    };
    // The participant after `each` is the actor of the trailing instruction.
    // Supplying that subject before parsing also lets possessive dynamic
    // values such as "half their library" bind to the iterated player rather
    // than falling back to the spell's controller.
    let effects = if for_each_shapes::contains_may(shape.effect_tokens) {
        parse_maybe_effects(shape.effect_tokens, true, true)?
    } else {
        let normalized = prepend_that_player_subject(shape.effect_tokens);
        parse_maybe_effects(&normalized, true, false)?
    };
    Ok(Some(EffectAst::ForEachTargetPlayers {
        count: shape.count,
        filter,
        effects,
    }))
}

pub(crate) fn parse_who_did_this_way_predicate(
    inner_tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    Ok(tagged_predicate(
        for_each_shapes::parse_who_tagged_filter_shape(inner_tokens),
    ))
}

fn parse_combat_damage_history_participant(
    inner_tokens: &[OwnedLexToken],
    iteration_filter: PlayerFilter,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(history) =
        for_each_shapes::parse_combat_damage_history_player_clause_shape(inner_tokens)
    else {
        return Ok(None);
    };
    let sources = parse_object_filter(history.source_tokens, false)?;
    let normalized = prepend_that_player_subject(history.effect_tokens);
    let effects = parse_maybe_effects(&normalized, false, true)?;
    Ok(Some(EffectAst::ForEachPlayersFiltered {
        filter: PlayerFilter::was_dealt_combat_damage_by_sources_this_game(
            iteration_filter,
            sources,
        ),
        effects,
    }))
}

pub(crate) fn parse_for_each_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(outer) = for_each_shapes::parse_participant_clause_shape(tokens) else {
        return Ok(None);
    };
    let Some(iteration_filter) = player_filter(outer.scope) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    let slot_chooser = if outer.participant_is_actor {
        PlayerAst::That
    } else {
        PlayerAst::You
    };
    if let Some(effects) =
        super::parse_for_each_type_slot_choice_clause(outer.inner_tokens, slot_chooser)?
    {
        return Ok(Some(wrap_players(&iteration_filter, effects)));
    }
    if iteration_filter == PlayerFilter::Any
        && let Some(effect) = parse_for_each_doesnt_control_lose_game(tokens, false)?
    {
        return Ok(Some(effect));
    }

    if let Some(relative) = for_each_shapes::parse_relative_control_clause_shape(outer.inner_tokens)
    {
        let conditional =
            parse_relative_control_conditional(relative, outer.participant_is_actor, &clause_text)?;
        return Ok(Some(wrap_players(&iteration_filter, vec![conditional])));
    }

    if iteration_filter == PlayerFilter::Any
        && let Some(source_attacked) =
            for_each_shapes::parse_source_attacked_player_clause_shape(outer.inner_tokens)
    {
        let normalized = prepend_that_player_subject(source_attacked.effect_tokens);
        let effects = parse_maybe_effects(&normalized, false, true)?;
        return Ok(Some(EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::AttackedBySourceThisTurn,
            effects,
        }));
    }

    if let Some(effect) =
        parse_combat_damage_history_participant(outer.inner_tokens, iteration_filter.clone())?
    {
        return Ok(Some(effect));
    }

    if let Some(who) = for_each_shapes::parse_who_clause_shape(outer.inner_tokens) {
        match who {
            WhoClauseShape::TappedLandForMana { effect_tokens } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each player who tapped a land for mana this turn' (clause: '{}')",
                        clause_text
                    )));
                }
                let branch_effects = parse_maybe_effects(effect_tokens, true, false)?;
                return Ok(Some(wrap_players(
                    &iteration_filter,
                    vec![EffectAst::Conditional {
                        predicate: PredicateAst::PlayerTappedLandForManaThisTurn {
                            player: PlayerAst::That,
                        },
                        if_true: branch_effects,
                        if_false: Vec::new(),
                    }],
                )));
            }
            WhoClauseShape::Negated {
                effect_tokens,
                tagged_filter_tokens,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect in for each player who doesn't clause (clause: '{}')",
                        clause_text
                    )));
                }
                return Ok(Some(EffectAst::ForEachPlayerDoesNot {
                    effects: parse_effect_chain_inner(effect_tokens)?,
                    predicate: tagged_predicate(tagged_filter_tokens),
                }));
            }
            WhoClauseShape::DidThisWay {
                effect_tokens,
                tagged_filter_tokens,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each player who ... this way' (clause: '{}')",
                        clause_text
                    )));
                }
                return Ok(Some(EffectAst::ForEachPlayerDid {
                    effects: parse_effect_chain_inner(effect_tokens)?,
                    predicate: tagged_predicate(tagged_filter_tokens),
                    result_predicate: IfResultPredicate::Did,
                }));
            }
            WhoClauseShape::DidAction {
                effect_tokens,
                implicit_player_is_you,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each player who does' (clause: '{}')",
                        clause_text
                    )));
                }
                let mut effects = parse_effect_chain_inner(effect_tokens)?;
                let player = if implicit_player_is_you {
                    PlayerAst::You
                } else {
                    PlayerAst::That
                };
                for effect in &mut effects {
                    bind_implicit_player_context(effect, player.clone());
                }
                return Ok(Some(EffectAst::ForEachPlayerDid {
                    effects,
                    predicate: None,
                    result_predicate: IfResultPredicate::AcceptedChoice,
                }));
            }
        }
    }

    let participant_may = outer.participant_is_actor
        && outer
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_word("may"));
    let participant_chooses = for_each_shapes::starts_choose(outer.inner_tokens);
    let normalized = if outer.participant_is_actor && !participant_may && !participant_chooses {
        prepend_that_player_subject(outer.inner_tokens)
    } else {
        prepend_that_player_life_total_subject(outer.inner_tokens)
    };
    let mut effects = parse_maybe_effects(&normalized, false, outer.participant_is_actor)?;
    if !outer.participant_is_actor {
        force_implicit_token_controller_you(&mut effects);
    }
    if participant_chooses {
        bind_implicit_choose_chooser(
            &mut effects,
            if outer.participant_is_actor {
                PlayerAst::That
            } else {
                PlayerAst::You
            },
        );
        stabilize_standalone_participant_choice_tag(&mut effects, outer.inner_tokens);
    }
    let iteration_filter = reanchor_other_player_copy_filter(iteration_filter, &effects);
    Ok(Some(wrap_players(&iteration_filter, effects)))
}

#[cfg(test)]
mod dynamic_modifier_surface_tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn resolving_gets_for_each_count_keeps_authored_surface() {
        let tokens = lex_line("for each permanent card in your graveyard", 0)
            .expect("for-each count should lex");
        let count = parse_get_for_each_count_value(&tokens)
            .expect("for-each count should parse")
            .expect("for-each count should match");

        assert!(count.has_surface_hint(ValueSurfaceHint::ForEach));
        assert!(matches!(count.unhinted(), Value::Count(_)));
    }

    #[test]
    fn resolving_gets_for_each_count_keeps_serial_relative_subtype_surface() {
        let tokens = lex_line(
            "for each creature you control that's an Insect, Rat, Spider, or Squirrel",
            0,
        )
        .expect("serial for-each count should lex");
        let count = parse_get_for_each_count_value(&tokens)
            .expect("serial for-each count should parse")
            .expect("serial for-each count should match");
        let Value::Count(filter) = count.unhinted() else {
            panic!("expected an object count, got {count:#?}");
        };

        assert!(filter.has_serial_or_list_surface(), "{filter:#?}");
        assert_eq!(
            filter.description(),
            "a creature you control that's an Insect, Rat, Spider, or Squirrel"
        );
    }

    #[test]
    fn exact_surface_reparse_preserves_specialized_cast_time_count_tag() {
        let tokens = lex_line(
            "for each modified creature you controlled as you cast this spell",
            0,
        )
        .expect("cast-time for-each count should lex");
        let count = parse_get_for_each_count_value(&tokens)
            .expect("cast-time for-each count should parse")
            .expect("cast-time for-each count should match");
        let Value::Count(filter) = count.unhinted() else {
            panic!("expected an object count, got {count:#?}");
        };

        assert!(matches!(
            filter.tagged_constraints.as_slice(),
            [constraint]
                if constraint.tag.as_str() == ironsmith_core::CAST_MODIFIED_CREATURES_TAG
                    && constraint.relation
                        == crate::target::TaggedOpbjectRelation::IsTaggedObject
        ));
        assert!(filter.card_types.is_empty(), "{filter:#?}");
    }

    #[test]
    fn targeted_discard_for_each_suffix_does_not_claim_target_player_iteration() {
        for text in [
            "Target player discards a card for each Swamp you control.",
            "Target player discards a card for each charge counter on this artifact.",
            "Target player discards a card for each Swamp returned this way.",
        ] {
            let tokens = lex_line(text, 0).expect("targeted discard clause should lex");
            let parsed = parse_for_each_target_players_clause(&tokens)
                .expect("ambiguous target-player shape should yield cleanly");

            assert!(
                parsed.is_none(),
                "discard count suffix must not become a target-player iterator: {parsed:#?}"
            );
        }
    }

    #[test]
    fn targeted_life_gain_for_each_suffix_does_not_claim_target_player_iteration() {
        let tokens = lex_line(
            "Target player gains 2 life for each creature on the battlefield.",
            0,
        )
        .expect("Congregate life-gain clause should lex");
        let parsed = parse_for_each_target_players_clause(&tokens)
            .expect("ambiguous target-player life-gain shape should yield cleanly");

        assert!(
            parsed.is_none(),
            "life-gain count suffix must not become a target-player iterator: {parsed:#?}"
        );
    }

    #[test]
    fn targeted_life_loss_for_each_suffix_does_not_claim_target_player_iteration() {
        let tokens = lex_line(
            "Target player loses 2 life plus 2 life for each Spirit sacrificed this way.",
            0,
        )
        .expect("Devouring Greed life-loss clause should lex");
        let parsed = parse_for_each_target_players_clause(&tokens)
            .expect("ambiguous target-player life-loss shape should yield cleanly");

        assert!(
            parsed.is_none(),
            "life-loss count suffix must not become a target-player iterator: {parsed:#?}"
        );
    }

    #[test]
    fn candlekeep_shape_binds_where_x_to_an_owned_multi_zone_count() {
        let tokens = lex_line(
            "Until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards you own in exile and in your graveyard that are instant cards, are sorcery cards, and/or have an Adventure.",
            0,
        )
        .expect("Candlekeep-style base-P/T clause should lex");
        let effect = parse_has_base_power_toughness_clause(&tokens)
            .expect("Candlekeep-style base-P/T clause should parse")
            .expect("Candlekeep-style base-P/T clause should be recognized");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SetBasePowerToughness {
                    power,
                    toughness,
                    target,
                    duration,
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected typed base-P/T effect, got {effect:#?}");
        };

        assert_eq!(duration, Until::EndOfTurn);
        assert_eq!(power, toughness);
        assert!(power.has_surface_hint(ValueSurfaceHint::WhereXIs));
        let Value::Count(filter) = power.unhinted() else {
            panic!("expected a counted object-filter basis, got {power:#?}");
        };
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(
            filter.card_types,
            vec![
                crate::types::CardType::Instant,
                crate::types::CardType::Sorcery
            ]
        );
        assert_eq!(filter.subtypes, vec![crate::types::Subtype::Adventure]);
        assert!(filter.type_or_subtype_union);
        assert_eq!(filter.any_of.len(), 2);
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| branch.zone == Some(crate::zone::Zone::Exile))
        );
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| { branch.zone == Some(crate::zone::Zone::Graveyard) })
        );
        assert!(matches!(target, TargetAst::Object(_, _, _)));
    }

    #[test]
    fn jolrael_shape_binds_where_x_to_cards_in_hand() {
        let tokens = lex_line(
            "Until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards in your hand.",
            0,
        )
        .expect("Jolrael-style base-P/T clause should lex");
        let effect = parse_has_base_power_toughness_clause(&tokens)
            .expect("Jolrael-style base-P/T clause should parse")
            .expect("Jolrael-style base-P/T clause should be recognized");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SetBasePowerToughness {
                    power,
                    toughness,
                    target,
                    duration,
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected typed base-P/T effect, got {effect:#?}");
        };

        assert_eq!(duration, Until::EndOfTurn);
        assert_eq!(power, toughness);
        assert!(power.has_surface_hint(ValueSurfaceHint::WhereXIs));
        assert!(matches!(
            power.unhinted(),
            Value::CardsInHand(PlayerFilter::You)
        ));
        let TargetAst::Object(filter, _, _) = target else {
            panic!("expected a creature-filter target, got {target:#?}");
        };
        assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
    }
}

#[cfg(test)]
mod participant_choice_ownership_tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn parsed_debug(text: &str) -> String {
        let tokens = lex_line(text, 0).expect("participant choice should lex");
        let effect =
            if text.starts_with("Each opponent") || text.starts_with("For each opponent") {
                parse_for_each_opponent_clause(&tokens)
            } else {
                parse_for_each_player_clause(&tokens)
            }
            .expect("participant choice should parse")
            .expect("participant choice shape should match");
        format!("{effect:#?}")
    }

    #[test]
    fn participant_subject_owns_choice_but_for_each_imperative_does_not() {
        let each_opponent =
            parsed_debug("Each opponent chooses a creature they control and sacrifices it.");
        assert!(each_opponent.contains("player: That"), "{each_opponent}");
        assert!(!each_opponent.contains("player: You"), "{each_opponent}");

        let each_player = parsed_debug("Each player chooses a creature they control.");
        assert!(each_player.contains("player: That"), "{each_player}");

        let controller = parsed_debug("For each opponent, choose a creature they control.");
        assert!(controller.contains("player: You"), "{controller}");
    }

    #[test]
    fn source_attacked_player_subject_keeps_runtime_filter() {
        let tokens = lex_line(
            "Each player this creature attacked this turn loses the game.",
            0,
        )
        .expect("source-relative player clause should lex");
        let effect = parse_for_each_player_clause(&tokens)
            .expect("source-relative player clause should parse")
            .expect("source-relative player clause should match");
        let EffectAst::ForEachPlayersFiltered { filter, effects } = effect else {
            panic!("expected filtered player iteration, got {effect:#?}");
        };
        assert_eq!(filter, PlayerFilter::AttackedBySourceThisTurn);
        assert!(format!("{effects:#?}").contains("LoseGame"), "{effects:#?}");
    }

    #[test]
    fn named_creature_combat_damage_history_keeps_filtered_participant() {
        let tokens = lex_line(
            "Each opponent dealt combat damage this game by a creature named Gollum, Obsessed Stalker loses life equal to the amount of life you gained this turn.",
            0,
        )
        .expect("combat-history participant clause should lex");
        let effect = parse_for_each_opponent_clause(&tokens)
            .expect("combat-history participant clause should parse")
            .expect("combat-history participant clause should match");
        let EffectAst::ForEachPlayersFiltered { filter, effects } = effect else {
            panic!("expected filtered player iteration, got {effect:#?}");
        };
        assert!(
            matches!(
                filter,
                PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. }
            ),
            "{filter:#?}"
        );
        let PlayerFilter::WasDealtCombatDamageBySourcesThisGame { sources, .. } = &filter else {
            unreachable!("typed history variant was already asserted")
        };
        assert_eq!(sources.name.as_deref(), Some("gollum obsessed stalker"));
        assert!(format!("{effects:#?}").contains("LoseLife"), "{effects:#?}");
    }

    #[test]
    fn other_players_copying_triggering_spell_exclude_its_controller() {
        let tokens = lex_line(
            "Each other player may copy that spell and may choose new targets for the copy they control.",
            0,
        )
        .expect("triggering-spell fanout should lex");
        let effect = parse_for_each_player_clause(&tokens)
            .expect("triggering-spell fanout should parse")
            .expect("triggering-spell fanout should match");
        let EffectAst::ForEachPlayersFiltered { filter, effects } = effect else {
            panic!("expected filtered player iteration, got {effect:#?}");
        };
        assert_eq!(
            filter,
            PlayerFilter::excluding(
                PlayerFilter::Any,
                PlayerFilter::AliasedControllerOf(ObjectRef::tagged("triggering")),
            )
        );
        assert!(matches!(effects.as_slice(), [EffectAst::May { .. }]));
    }

    #[test]
    fn standalone_participant_choices_use_an_aggregate_tag_but_nested_choices_remain_local() {
        let standalone = parsed_debug("Each player chooses a creature they control.");
        assert!(
            standalone.contains("participant_choice_l0_s"),
            "{standalone}"
        );
        assert!(!standalone.contains("\"__it__\""), "{standalone}");

        let nested =
            parsed_debug("Each opponent chooses a creature they control and sacrifices it.");
        assert!(
            !nested.contains("participant_choice_l0_s"),
            "an immediate per-participant consumer must not share choices across iterations: \
            {nested}"
        );
    }

    #[test]
    fn for_each_object_filter_preserves_typed_those_set_surface() {
        let those_tokens = lex_line("those permanents", 0).expect("those filter should lex");
        let those = parse_for_each_object_filter(&those_tokens).expect("those filter should parse");
        assert_eq!(
            those.set_quantifier_surface(),
            Some(ironsmith_core::SetQuantifierSurface::Those)
        );

        let ordinary_tokens =
            lex_line("permanent destroyed this way", 0).expect("ordinary filter should lex");
        let ordinary =
            parse_for_each_object_filter(&ordinary_tokens).expect("ordinary filter should parse");
        assert_eq!(ordinary.set_quantifier_surface(), None);
    }

    #[test]
    fn for_each_object_filter_preserves_owned_exile_counter_scope() {
        let tokens = lex_line(
            "creature card you own in exile with a memory counter on it",
            0,
        )
        .expect("owned exile filter should lex");
        let filter =
            parse_for_each_object_filter(&tokens).expect("owned exile filter should parse");

        assert_eq!(filter.zone, Some(crate::zone::Zone::Exile));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
        assert_eq!(
            filter.with_counter,
            Some(crate::filter::CounterConstraint::Typed(
                crate::object::CounterType::Named("memory")
            ))
        );
        assert!(filter.has_owner_before_zone_surface());
        assert!(filter.has_counter_requirement_after_zone_surface());
        assert_eq!(
            filter.description(),
            "a creature card you own in exile with a memory counter on it"
        );
    }

    #[test]
    fn for_each_object_filter_restores_only_the_exact_coordinated_stack_domain() {
        let coordinated = lex_line("spell and ability your opponents control", 0)
            .expect("coordinated Stack filter should lex");
        let coordinated = parse_for_each_object_filter(&coordinated)
            .expect("coordinated Stack filter should parse");
        assert_eq!(coordinated.zone, Some(crate::zone::Zone::Stack));
        assert_eq!(
            coordinated.stack_kind,
            Some(crate::filter::StackObjectKind::SpellOrAbility)
        );
        assert!(!coordinated.has_mana_cost);
        assert!(coordinated.has_conjunctive_set_surface());

        let ordinary =
            lex_line("spell your opponents control", 0).expect("ordinary spell filter should lex");
        let ordinary =
            parse_for_each_object_filter(&ordinary).expect("ordinary spell filter should parse");
        assert_eq!(
            ordinary.stack_kind,
            Some(crate::filter::StackObjectKind::Spell)
        );
        assert!(ordinary.has_mana_cost);
        assert!(!ordinary.has_conjunctive_set_surface());
    }

    #[test]
    fn relative_participant_count_compares_the_same_set_for_them_and_you() {
        let effect =
            parsed_debug("Each opponent who controls fewer creatures than you draws a card.");
        let compact: String = effect.chars().filter(|ch| !ch.is_whitespace()).collect();

        assert!(effect.contains("ValueComparison"), "{effect}");
        assert!(effect.contains("operator: LessThan"), "{effect}");
        assert!(
            compact.contains("controller:Some(IteratedPlayer,)"),
            "{effect}"
        );
        assert!(compact.contains("controller:Some(You,)"), "{effect}");
        assert!(!effect.contains("PlayerControls {"), "{effect}");
    }
}

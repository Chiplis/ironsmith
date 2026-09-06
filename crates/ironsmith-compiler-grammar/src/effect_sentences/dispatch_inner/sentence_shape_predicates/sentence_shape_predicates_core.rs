use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::ObjectChoiceEffectAst;
use crate::cards::builders::ControlActionAst;
use crate::cards::builders::TokenActionAst;
use crate::cards::builders::StackActionAst;
use crate::cards::builders::StatChangeActionAst;
use crate::cards::builders::DamageActionAst;
use crate::cards::builders::PermanentStateActionAst;
use crate::cards::builders::ZoneMoveActionAst;
use crate::cards::builders::KeywordActionAst;
use crate::cards::builders::CharacteristicActionAst;
use crate::cards::builders::ExchangeActionAst;
use crate::cards::builders::LibraryActionAst;
use crate::cards::builders::GrantActionAst;
use crate::cards::builders::DamagePreventionActionAst;
use crate::cards::builders::CounterActionAst;
use super::*;

#[inline(never)]
fn parse_complete_conditional_gain_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((predicate_clause, consequence_tokens)) =
        crate::grammar::primitives::split_lexed_once_on_comma(tokens)
    else {
        return Ok(None);
    };
    if !predicate_clause
        .first()
        .is_some_and(|token| token.is_word("if"))
    {
        return Ok(None);
    }
    let Some(effects) =
        super::super::gain_ability::parse_gain_ability_sentence(consequence_tokens)?
    else {
        return Ok(None);
    };
    let predicate_tokens = trim_commas(&predicate_clause[1..]);
    let Ok(predicate) =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&predicate_tokens)
    else {
        return Ok(None);
    };
    Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate,
        if_true: effects,
        if_false: Vec::new(),
    })]))
}

/// The sentence rule. Memoized per card: every distinct span is parsed once,
/// and every recognizer that asks about it sees that one parse.
#[inline(never)]
#[track_caller]
pub fn parse_effect_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    crate::sentence_memo::memoized(
        crate::sentence_memo::Rule::Sentence,
        tokens,
        std::panic::Location::caller(),
        || parse_effect_sentence_lexed_uncached(tokens),
    )
}

#[inline(never)]
fn parse_effect_sentence_lexed_uncached(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = super::super::parse_complete_create_statement(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = crate::effect_sentences::dispatch_entry::parse_complete_compound_gain_statement(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_complete_conditional_gain_ability(tokens)? {
        return Ok(effects);
    }
    dispatch_effect_sentence_lexed_remaining(tokens)
}

#[path = "sentence_shape_predicates_core/remaining_readings.rs"]
mod remaining_readings;

#[inline(never)]
fn dispatch_effect_sentence_lexed_remaining(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let input = remaining_readings::RemainingSentence {
        tokens,
        read_by_cache: Default::default(),
    };
    match remaining_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    let stripped_tokens = strip_embedded_token_rules_text(tokens);
    let has_embedded_token_rules = stripped_tokens.len() != tokens.len();
    let parse_tokens = if has_embedded_token_rules {
        stripped_tokens.as_slice()
    } else {
        tokens
    };
    let mut effects = {
        if let Some(effects) = parse_prefix_then_look_at_top_exile_one(parse_tokens)? {
            Ok(effects)
        } else if let Some(effects) = parse_bounded_x_mana_payment_sentence(parse_tokens) {
            Ok(effects)
        } else {
            parse_effect_sentence_lexed_inner(parse_tokens)
        }
    }?;
    super::super::fanout_family::bind_removed_counter_damage_fanout(&mut effects);
    if has_embedded_token_rules {
        super::super::creation_handlers::attach_inline_token_granted_abilities_to_last_create(
            &mut effects,
            tokens,
        );
    }
    if let Some(surface) = parse_set_quantifier_surface(parse_tokens) {
        set_first_continuous_set_quantifier(&mut effects, surface);
    }
    if let Some(surface) = parse_return_set_reference_surface(parse_tokens) {
        set_first_return_set_reference_surface(&mut effects, &surface);
    }
    Ok(crate::effect_sentences::preserve_coordinated_effect_chain_surface(parse_tokens, effects))
}

pub fn parse_effect_sentence_lexed_with_context(
    context: crate::parse_context::ParseContextView<'_>,
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let authored_surface = crate::util::authored_named_source_reference_surface(context, tokens);
    let leading_normalized =
        crate::util::normalize_leading_named_source_reference_tokens_with_context(context, tokens);
    let normalized = if leading_normalized == tokens {
        super::super::normalize_source_references_with_context(context, tokens)?
    } else {
        leading_normalized
    };
    let mut effects = if authored_surface.is_some() {
        match super::super::clause_primitives::parse_anaphoric_object_deals_damage_clause(
            &normalized,
        )? {
            Some(effect) => vec![effect],
            None => parse_effect_sentence_lexed(&normalized)?,
        }
    } else {
        parse_effect_sentence_lexed(&normalized)?
    };
    if let Some(surface) = authored_surface {
        restore_authored_damage_source_surface(&mut effects, &surface);
    }
    Ok(effects)
}

pub(super) fn has_unrecognized_leading_effect_label(tokens: &[OwnedLexToken]) -> bool {
    if crate::grammar::structure::split_leading_result_prefix_lexed(tokens).is_some() {
        return false;
    }
    effect_grammar::labeled_dispatch::parse_leading_effect_label_tokens(tokens).is_some_and(
        |shape| shape.kind == effect_grammar::labeled_dispatch::LeadingEffectLabelKind::Unknown,
    )
}

pub(super) fn parse_effect_sentence_lexed_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_sentence_lexed_inner_unstacked(tokens)
}

use crate::recognition::ParseOutcome;
#[path = "sentence_shape_predicates_core/readings.rs"]
mod readings;

pub(super) fn parse_effect_sentence_lexed_inner_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let input = readings::Sentence::new(tokens);
    match readings::read_sentence(&input) {
        ParseOutcome::Match(matched) => Ok(matched.value.value),
        ParseOutcome::NoMatch => {
            readings::diagnose(&input)?;
            // The sentence dispatcher has exhausted its specialized routes here.
            // Delegate to the lower-level chain parser; calling this dispatcher again
            // with the same tokens recurses forever for ordinary subject/verb clauses.
            // The sentence dispatcher has exhausted its specialized routes here.
            // Delegate to the lower-level chain parser; calling this dispatcher again
            // with the same tokens recurses forever for ordinary subject/verb clauses.
            let mut effects = super::super::parse_effect_chain_inner_lexed(tokens)?;
            apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
            normalize_search_followup_shuffles(&mut effects);
            Ok(effects)
        }
        ParseOutcome::Error(diagnostic) => Err(diagnostic.into_card_text_error()),
    }
}

fn search_followup_shuffle_player(effect: &EffectAst) -> Option<PlayerAst> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary { player, .. }),
            ..
        }) => Some(*player),
        _ => None,
    }
}

fn normalize_search_followup_shuffles(effects: &mut [EffectAst]) {
    for idx in 0..effects.len() {
        let is_default_shuffle = matches!(
            effects.get(idx),
            Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject,
                action: SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
            }))
                if matches!(subject.player, PlayerAst::You | PlayerAst::Implicit)
        );
        if !is_default_shuffle {
            continue;
        }
        let Some(search_player) = effects[..idx]
            .iter()
            .rev()
            .find_map(search_followup_shuffle_player)
        else {
            continue;
        };
        if !matches!(search_player, PlayerAst::You | PlayerAst::Implicit)
            && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject,
                action: SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
            }) = &mut effects[idx]
        {
            subject.player = search_player;
        }
    }
}

// A duration-scoped trigger may itself contain a damage action. Preserve
// the grammar-proven outer `Until ..., whenever ...` scope before the
// broad damage recognizers examine the whole sentence as a direct action.
// The trigger parser recursively dispatches only the smaller payload, so
// this route cannot claim an ordinary leading-duration continuous effect.

pub(super) fn parse_effect_sentence_with_where_x_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let view = crate::rule_engine::LexClauseView::from_tokens(tokens);
    match crate::effect_sentences::subject_verb_special_recognizers::parse_cross_zone_where_x_fanout_rule_lexed(&view) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }

    fn replace_search_filter_x(effect: &mut EffectAst, replacement: &Value) {
        let (filter, count, count_value) = match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                        filter,
                        count,
                        count_value,
                        ..
                    }),
                ..
            }) => (filter, count, count_value),
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter,
                count,
                count_value,
                ..
            })
            | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
                filter,
                count,
                count_value,
                ..
            }) => (filter, count, count_value),
            _ => return,
        };

        if count.dynamic_x && count_value.is_none() {
            *count_value = Some(replacement.clone());
        }
        if let Some(mana_value) = filter.mana_value.as_mut() {
            use crate::filter::Comparison;

            match mana_value {
                Comparison::EqualExpr(value)
                | Comparison::NotEqualExpr(value)
                | Comparison::LessThanExpr(value)
                | Comparison::LessThanOrEqualExpr(value)
                | Comparison::GreaterThanExpr(value)
                | Comparison::GreaterThanOrEqualExpr(value)
                    if matches!(value.as_ref(), Value::X) =>
                {
                    **value = replacement.clone();
                }
                _ => {}
            }
        }
    }

    fn bind_dynamic_target_count(target: &mut TargetAst, replacement: &Value) {
        fn bind_comparison_x(
            comparison: &mut Option<crate::filter::Comparison>,
            replacement: &Value,
        ) {
            let Some(
                crate::filter::Comparison::EqualExpr(value)
                | crate::filter::Comparison::NotEqualExpr(value)
                | crate::filter::Comparison::LessThanExpr(value)
                | crate::filter::Comparison::LessThanOrEqualExpr(value)
                | crate::filter::Comparison::GreaterThanExpr(value)
                | crate::filter::Comparison::GreaterThanOrEqualExpr(value),
            ) = comparison
            else {
                return;
            };
            if matches!(value.as_ref(), Value::X) {
                **value = replacement.clone();
            }
        }

        fn bind_filter_x(filter: &mut crate::target::ObjectFilter, replacement: &Value) {
            bind_comparison_x(&mut filter.power, replacement);
            bind_comparison_x(&mut filter.toughness, replacement);
            bind_comparison_x(&mut filter.mana_value, replacement);
            if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
                bind_filter_x(attached_to, replacement);
            }
            for branch in &mut filter.any_of {
                bind_filter_x(branch, replacement);
            }
        }

        match target {
            TargetAst::Object(filter, _, _) => bind_filter_x(filter, replacement),
            TargetAst::WithCount(inner, count) => {
                bind_dynamic_target_count(inner, replacement);
                if count.is_dynamic_x() {
                    let old = std::mem::replace(target, TargetAst::Source(None));
                    if let TargetAst::WithCount(inner, count) = old {
                        *target = TargetAst::WithCountValue(inner, count, replacement.clone());
                    }
                }
            }
            TargetAst::WithCountValue(inner, _, value) => {
                bind_dynamic_target_count(inner, replacement);
                if matches!(value, Value::X) {
                    *value = replacement.clone();
                }
            }
            _ => {}
        }
    }

    fn bind_dynamic_target_counts(effect: &mut EffectAst, replacement: &Value) {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return;
        };
        match action {
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Explore { target })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Endure { target, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Connive { target, .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeTextBoxes { target })
            | SubjectVerbActionAst::Control(ControlActionAst::Attach { target, .. })
            | SubjectVerbActionAst::Control(ControlActionAst::Unattach { object: target })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MayMoveToZone { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { target, .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice { target })
            | SubjectVerbActionAst::TargetOnly { target, .. }
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddCardTypes { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCardTypes { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveCardTypes { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddColors { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddAllSubtypesOfFamily { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAllSubtypesOfFamily { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandType { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetColors { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandTypeChoice { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCreatureTypeChoice { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantToTarget { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget { target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource { target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                source: target,
            })
            | SubjectVerbActionAst::Stack(StackActionAst::RetargetStackObject { target, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { target, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { target, .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { target })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap { target }) => {
                bind_dynamic_target_count(target, replacement)
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { target, .. }) => {
                bind_dynamic_target_count(target, replacement)
            }
            SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
                target,
                target_count,
                ..
            }) => {
                bind_dynamic_target_count(target, replacement);
                if let Some(count) = target_count
                    .as_ref()
                    .copied()
                    .filter(|count| count.is_dynamic_x())
                    && !matches!(target, TargetAst::WithCountValue(_, _, _))
                {
                    let inner = std::mem::replace(target, TargetAst::Source(None));
                    *target =
                        TargetAst::WithCountValue(Box::new(inner), count, replacement.clone());
                }
            }
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget {
                protected_target,
                destination_target,
                ..
            }) => {
                if let Some(target) = protected_target {
                    bind_dynamic_target_count(target, replacement);
                }
                if let Some(target) = destination_target {
                    bind_dynamic_target_count(target, replacement);
                }
            }
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight {
                creature1,
                creature2,
                ..
            })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                source: creature1,
                target: creature2,
                ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy {
                target: creature1,
                source: creature2,
                ..
            }) => {
                bind_dynamic_target_count(creature1, replacement);
                bind_dynamic_target_count(creature2, replacement);
            }
            SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { source, .. }) => {
                bind_dynamic_target_count(source, replacement);
            }
            SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                attached_to: Some(target),
                ..
            }) => bind_dynamic_target_count(target, replacement),
            _ => {}
        }
    }

    let clause_display = render_token_slice(tokens).trim().to_string();
    let Some(where_shape) = sentence_shapes::parse_where_x_sentence_tokens(tokens) else {
        return parse_effect_sentence_inner_lexed(tokens);
    };
    let aggregate_where =
        crate::keyword_static::parse_where_x_is_aggregate_filter_value(where_shape.where_tokens);
    let turn_history_where = aggregate_where
        .is_none()
        .then(|| {
            crate::grammar::shared_util::value_semantics::parse_turn_history_value_binding(
                where_shape.where_tokens,
            )
        })
        .flatten();
    let has_semicolon_tail =
        where_shape.has_trailing_segment() && tokens.iter().any(OwnedLexToken::is_semicolon);
    let full_where_is_count_value = !where_shape.comma_tail_has_effect_clause
        && !has_semicolon_tail
        && (turn_history_where.is_some()
            || crate::keyword_static::parse_where_x_is_sum_of_number_of_filter_values(
                where_shape.where_tokens,
            )
            .is_some()
            || crate::keyword_static::parse_where_x_is_number_of_filter_value(
                where_shape.where_tokens,
            )
            .is_some());
    let layout = where_shape.layout(full_where_is_count_value);
    let primary_where_tokens = layout.primary_where_tokens;
    let trailing_after_where = layout.trailing_after_where;
    let stripped = trim_edge_punctuation(where_shape.stripped_tokens);

    if let Some(effects) = parse_target_deals_power_damage_to_other_and_self_where_x(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) =
        parse_tap_then_damage_for_number_tapped_this_way(&stripped, primary_where_tokens)?
    {
        return Ok(effects);
    }

    let mut prelude_effects = Vec::new();
    // Only the action before the where-X binding determines what a possessive
    // reference denotes. A later effect clause is dispatched independently
    // and cannot turn "target creature ... where X is its power" back into a
    // source-relative value.
    let typed_where_references_target = where_shape.stripped_references_target
        && !sentence_shapes::starts_with_source_deals_x_tokens(&stripped);
    // Prefer the complete number-of family before the generic typed value
    // shape. The latter can correctly find the trailing object scope while
    // still losing the aggregate being measured, as in "the number of
    // abilities from among ... found among creatures you control."
    // A player-comparison value ends in an object noun ("more lands than
    // you"), but its cardinality is the number of qualifying players. Parse
    // that participant domain before the generic number-of-filter family can
    // collapse it to a battlefield-object count.
    let participant_comparison_where = turn_history_where
        .is_none()
        .then(|| {
            crate::grammar::values::parse_players_who_control_more_than_you_value_lexed(
                primary_where_tokens,
            )
        })
        .flatten();
    // The complete value-expression parser treats a bare possessive such as
    // `its power` as source-relative. When the action before the where-X
    // clause introduced a target, the typed reference shape owns that
    // pronoun (`target creature ... where X is its power`). Explicit `this
    // creature's` remains source-relative through the shape's own reference
    // classification.
    let targeted_reference_where = typed_where_references_target
        .then(|| sentence_shapes::parse_where_x_value_shape_tokens(primary_where_tokens, true))
        .flatten()
        .filter(|shape| {
            matches!(
                shape,
                sentence_shapes::WhereXValueShape::ReferenceMetric { .. }
            )
        })
        .and_then(lower_where_x_shape);
    let exact_where_value = (turn_history_where.is_none()
        && participant_comparison_where.is_none()
        && targeted_reference_where.is_none())
    .then(|| {
        super::super::dispatch_entry::parse_exact_where_x_value_expression(primary_where_tokens)
    })
    .flatten();
    let complete_number_where = (turn_history_where.is_none()
        && participant_comparison_where.is_none()
        && exact_where_value.is_none())
    .then(|| crate::keyword_static::parse_where_x_is_number_of_filter_value(primary_where_tokens))
    .flatten();
    let typed_where_value = if targeted_reference_where.is_some() {
        targeted_reference_where
    } else if turn_history_where.is_none()
        && participant_comparison_where.is_none()
        && exact_where_value.is_none()
        && complete_number_where.is_none()
    {
        sentence_shapes::parse_where_x_value_shape_tokens(
            primary_where_tokens,
            typed_where_references_target,
        )
        .and_then(lower_where_x_shape)
    } else {
        None
    };
    let where_value = if let Some(value) = aggregate_where {
        value
    } else if let Some(value) = turn_history_where {
        value
    } else if let Some(value) = participant_comparison_where {
        value
    } else if let Some(value) = exact_where_value {
        value
    } else if let Some(value) = complete_number_where {
        value
    } else if let Some((prelude, value)) = typed_where_value {
        if let Some(prelude) = prelude {
            prelude_effects.push(prelude);
        }
        value
    } else {
        let activation_time_trimmed =
            sentence_shapes::parse_before_activation_time_tokens(primary_where_tokens)
                .map(trim_edge_punctuation);
        let specific_where_value =
            super::super::dispatch_entry::parse_exact_where_x_value_expression(
                primary_where_tokens,
            )
            .or_else(|| {
                crate::grammar::values::parse_players_who_control_more_than_you_value_lexed(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_sum_of_number_of_filter_values(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_number_of_different_powers_filter_value(
                    primary_where_tokens,
                )
            });
        let number_of_filter_value = specific_where_value
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_colored_mana_symbols_value(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_number_of_filter_value(primary_where_tokens)
            })
            .or_else(|| {
                activation_time_trimmed
                    .as_deref()
                    .and_then(crate::keyword_static::parse_where_x_is_number_of_filter_value)
            });
        if let Some(value) = number_of_filter_value {
            value
        } else if let Some(trimmed) = activation_time_trimmed.as_deref() {
            parse_value_binding_clause_lexed(trimmed).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported where-x clause (clause: '{}')",
                    &clause_display
                ))
            })?
        } else {
            parse_value_binding_clause_lexed(primary_where_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported where-x clause (clause: '{}')",
                    &clause_display
                ))
            })?
        }
    };
    let where_value = crate::effect_sentences::dispatch_entry::with_where_x_surface_hints(
        where_value,
        primary_where_tokens,
    );

    let search_like = where_shape.stripped_starts_search;
    let granted_entry_static = if crate::word_primitives::any_sequence_occurs(
        &crate::lexer::parser_token_word_refs(&stripped),
        &[&["enters", "with"], &["enter", "with"]],
    ) {
        let words = crate::lexer::parser_token_word_refs(&stripped);
        let explicit_source_subject = words.first() == Some(&"this")
            && words.get(2) == Some(&"enters")
            && words.get(1).is_some_and(|subject| {
                matches!(
                    *subject,
                    "artifact"
                        | "battle"
                        | "card"
                        | "creature"
                        | "enchantment"
                        | "land"
                        | "permanent"
                        | "planeswalker"
                        | "source"
                )
            });
        let entry_abilities = if explicit_source_subject {
            crate::keyword_static::parse_enters_with_counters_line(&stripped)?
        } else {
            parse_enters_with_additional_counter_for_filter_line(&stripped)?
                .map(|ability| vec![ability])
        };
        entry_abilities
            .filter(|abilities| !abilities.is_empty())
            .map(|abilities| {
                EffectAst::subject_verb_grant_abilities_to_target(
                    if explicit_source_subject {
                        TargetAst::Source(None)
                    } else {
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None)
                    },
                    abilities
                        .into_iter()
                        .map(|ability| {
                            GrantedAbilityAst::StaticAbility(Box::new(
                                crate::cards::builders::StaticAbilityAst::Static(ability),
                            ))
                        })
                        .collect(),
                    Until::Forever,
                )
            })
    } else {
        None
    };
    let mut effects = if search_like && !trailing_after_where.is_empty() {
        let mut recombined = stripped.clone();
        recombined.extend(trailing_after_where.clone());
        parse_effect_sentence_lexed(&recombined)?
    } else if let Some(grant) = granted_entry_static {
        let mut parsed = vec![grant];
        if !trailing_after_where.is_empty() {
            let mut trailing_effects = parse_effect_sentence_lexed(&trailing_after_where)?;
            parsed.append(&mut trailing_effects);
        }
        parsed
    } else {
        // The terminal where-X owner strips its binding before inner
        // dispatch. Preserve explicit player-subject boundaries at that
        // point too; otherwise `each opponent ... and you ...` is accepted
        // as one participant body and the controller action is repeated for
        // every opponent. The shared where value is rebound across both
        // resulting effects below.
        let mut parsed = if let Some(clauses) =
            super::super::player_subject_sequences::split_explicit_player_subject_clauses(&stripped)
        {
            let mut split_effects = Vec::new();
            for clause in clauses {
                split_effects.extend(parse_effect_sentence_inner_lexed(clause)?);
            }
            split_effects
        } else {
            parse_effect_sentence_inner_lexed(&stripped)?
        };
        if parsed.is_empty() && !stripped.is_empty() {
            parsed.push(super::super::parse_effect_clause_lexed(&stripped)?);
        }
        if !trailing_after_where.is_empty() {
            let mut trailing_effects = parse_effect_sentence_lexed(&trailing_after_where)?;
            parsed.append(&mut trailing_effects);
        }
        parsed
    };
    rebind_plural_create_followup_damage_source(&mut effects);
    replace_unbound_x_in_effects_anywhere(&mut effects, &where_value, &clause_display)?;
    for effect in &mut effects {
        replace_search_filter_x(effect, &where_value);
        bind_dynamic_target_counts(effect, &where_value);
    }
    if !prelude_effects.is_empty() {
        prelude_effects.append(&mut effects);
        return Ok(prelude_effects);
    }
    Ok(effects)
}

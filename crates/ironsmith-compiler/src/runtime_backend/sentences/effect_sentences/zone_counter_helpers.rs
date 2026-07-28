use crate::cards::TextSpan;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, OwnedLexToken, PlayerAst, PredicateAst,
    SubjectAst, SubjectVerbActionAst, SubjectVerbEffectAst, TargetAst,
};
use crate::effect::EventValueSpec;
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;
use crate::{ChooseSpec, CounterType, TagKey, Value};
use ironsmith_core::{EffectMetric, EffectMetricSource, ValueSurfaceHint};

use super::super::activation_and_restrictions::parse_devotion_value_from_add_clause;
use super::super::grammar::structure::split_trailing_if_clause_lexed;
use super::super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_dynamic_cost_modifier_value,
};
use super::super::lexer::TokenWordView;
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    parse_counter_type_from_tokens, parse_target_phrase, parse_value,
    record_source_reference_surface, span_from_tokens,
};
use crate::runtime_backend::front_end::grammar::effects::zone_counter_shapes as shapes;
use crate::runtime_backend::grammar::shared_util::value_semantics::{
    parse_equal_to_aggregate_filter_value, parse_equal_to_number_of_filter_value,
};
type ZoneCounterCompatWords<'a> = TokenWordView<'a>;

fn this_way_object_count_value() -> Value {
    Value::PendingEffectMetric {
        source: EffectMetricSource::AffectedObjects,
        metric: EffectMetric::Count,
    }
}

fn render_clause_words(tokens: &[OwnedLexToken]) -> String {
    ZoneCounterCompatWords::new(tokens).to_word_refs().join(" ")
}

fn parse_create_for_each_dynamic_count(tokens: &[OwnedLexToken]) -> Option<Value> {
    // Counter clauses have a few historical/count surfaces whose qualifiers
    // are broader than an object filter (for example, "spells you've cast
    // this turn other than the first"). Recognize those typed values before
    // the generic object-count fallback, which would otherwise retain only
    // "other spell" and silently lose the turn-history semantics.
    let affected_this_way_fallback = match shapes::parse_dynamic_counter_count_shape(tokens) {
        Some(dynamic) => match dynamic {
            shapes::DynamicCounterCountShape::LifeLostThisWay { group_size } => {
                let life_lost = Value::EventValue(EventValueSpec::LifeAmount);
                return Some(if group_size <= 1 {
                    life_lost
                } else {
                    Value::DividedRoundedDown(Box::new(life_lost), group_size)
                });
            }
            shapes::DynamicCounterCountShape::CreaturesDiedThisTurn => {
                return Some(Value::CreaturesDiedThisTurn);
            }
            shapes::DynamicCounterCountShape::SpellsCastThisTurn {
                player,
                other_than_first,
            } => {
                let count = Value::SpellsCastThisTurn(player);
                return Some(if other_than_first {
                    Value::Add(Box::new(count), Box::new(Value::Fixed(-1)))
                } else {
                    count
                });
            }
            shapes::DynamicCounterCountShape::ColorsOfManaSpentToCastThisSpell => {
                return Some(Value::ColorsOfManaSpentToCastThisSpell);
            }
            shapes::DynamicCounterCountShape::BasicLandTypesAmongLandsYouControl => {
                return Some(Value::BasicLandTypesAmong(
                    ObjectFilter::land().you_control(),
                ));
            }
            // This is only a compatibility fallback. Give the typed
            // prior-action grammar first refusal so authored facts such as
            // "permanent destroyed this way" retain both their action and
            // object filter instead of collapsing to "objects affected".
            shapes::DynamicCounterCountShape::ObjectsAffectedThisWay => true,
        },
        None => false,
    };

    // Reuse the generic for-each value grammar so a prior-action reference
    // retains its object restriction (for example, creature cards among all
    // cards exiled this way). Reference resolution replaces IT_TAG with the
    // concrete snapshot tag emitted by the prior action.
    let mut for_each_words = vec!["for", "each"];
    for_each_words.extend(crate::runtime_backend::token_word_refs(tokens));
    if let Some((value, used)) =
        crate::runtime_backend::front_end::grammar::shared_util::count_shapes::parse_for_each_count_value_words(
            &for_each_words,
        )
        && used == for_each_words.len()
    {
        return Some(value);
    }
    affected_this_way_fallback.then(this_way_object_count_value)
}

pub(crate) fn describe_counter_type_for_mode(counter_type: CounterType) -> String {
    counter_type.description().into_owned()
}

pub(crate) fn describe_counter_phrase_for_mode(count: u32, counter_type: CounterType) -> String {
    let counter_name = describe_counter_type_for_mode(counter_type);
    if count == 1 {
        format!("a {counter_name} counter")
    } else {
        format!("{count} {counter_name} counters")
    }
}

pub(crate) fn sentence_case_mode_text(text: &str) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_uppercase());
    out.extend(chars);
    out
}

pub(crate) fn parse_counter_descriptor(
    tokens: &[OwnedLexToken],
) -> Result<(u32, CounterType), CardTextError> {
    let descriptor = shapes::parse_counter_descriptor_shape(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter descriptor (clause: '{}')",
            render_clause_words(tokens)
        ))
    })?;
    Ok((descriptor.count, descriptor.counter_type))
}

fn parse_put_counter_count_value(
    tokens: &[OwnedLexToken],
) -> Result<(Value, usize), CardTextError> {
    let clause = render_clause_words(tokens);
    match shapes::parse_counter_count_prefix_shape(tokens) {
        shapes::CounterCountPrefixShape::UpTo { inner_tokens } => {
            let (value, used) = parse_put_counter_count_value(inner_tokens)?;
            let prefix = tokens.len().saturating_sub(inner_tokens.len());
            return Ok((
                value.with_surface_hint(ValueSurfaceHint::UpTo),
                used + prefix,
            ));
        }
        shapes::CounterCountPrefixShape::EventAmount { consumed } => {
            return Ok((Value::EventValue(EventValueSpec::Amount), consumed));
        }
        shapes::CounterCountPrefixShape::Another => return Ok((Value::Fixed(1), 1)),
        shapes::CounterCountPrefixShape::Referential(reference) => {
            let source = shapes::source_spec_for_reference(reference.source);
            return Ok((
                Value::CountersOn(Box::new(source), reference.counter_type),
                reference.consumed,
            ));
        }
        shapes::CounterCountPrefixShape::NumberOf {
            value_tokens,
            equal_to_difference,
            equal_to_after_target,
        } => {
            if equal_to_difference {
                return Ok((Value::Fixed(0), 3));
            }
            let preserve_surface = |value: Value| {
                let value = if value.has_surface_hint(ValueSurfaceHint::EqualTo) {
                    value
                } else {
                    value.with_surface_hint(ValueSurfaceHint::EqualTo)
                };
                if equal_to_after_target {
                    value.with_surface_hint(ValueSurfaceHint::EqualToAfterTarget)
                } else {
                    value
                }
            };
            if let Some(value) = parse_add_mana_equal_amount_value(tokens)
                .or_else(|| parse_equal_to_aggregate_filter_value(tokens))
                .or_else(|| parse_equal_to_number_of_filter_value(tokens))
            {
                return Ok((preserve_surface(value), 3));
            }
            if let Some(value) = parse_devotion_value_from_add_clause(tokens)? {
                return Ok((preserve_surface(value), 3));
            }
            if let Some(value) = parse_dynamic_cost_modifier_value(tokens)? {
                return Ok((preserve_surface(value), 3));
            }
            if let Some(value_tokens) = value_tokens {
                if let Some((value, used)) = parse_value(&value_tokens)
                    && used == value_tokens.len()
                {
                    return Ok((preserve_surface(value), 3));
                }
                if let Some(value) = parse_named_source_power_value(&value_tokens) {
                    return Ok((preserve_surface(value), 3));
                }
            }
            return Err(CardTextError::ParseError(format!(
                "missing counter amount (clause: '{}')",
                clause
            )));
        }
        shapes::CounterCountPrefixShape::ExistingCounterEqual { value_tokens } => {
            if let Some((value, used)) = parse_value(&value_tokens)
                && used == value_tokens.len()
            {
                return Ok((value, 0));
            }
            if let Some(value) = parse_named_source_power_value(&value_tokens) {
                return Ok((value, 0));
            }
        }
        shapes::CounterCountPrefixShape::Plain => {}
    }

    parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!("missing counter amount (clause: '{}')", clause))
    })
}

fn parse_named_source_power_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    shapes::is_named_source_power_shape(tokens)
        .then(|| Value::PowerOf(Box::new(ChooseSpec::Source)))
}

fn target_from_counter_source_spec(spec: &ChooseSpec, span: Option<TextSpan>) -> Option<TargetAst> {
    match spec {
        ChooseSpec::Source => Some(TargetAst::Source(span)),
        ChooseSpec::Tagged(tag) => Some(TargetAst::Tagged(tag.clone(), span)),
        ChooseSpec::Target(inner) => target_from_counter_source_spec(inner, span),
        _ => None,
    }
}

pub(crate) fn target_object_filter_mut(target: &mut TargetAst) -> Option<&mut ObjectFilter> {
    match target {
        TargetAst::Object(filter, _, _) => Some(filter),
        TargetAst::WithCount(inner, _) => target_object_filter_mut(inner),
        _ => None,
    }
}

pub(crate) fn merge_it_match_filter_into_target(
    target: &mut TargetAst,
    it_filter: &ObjectFilter,
) -> bool {
    if let TargetAst::Tagged(tag, span) = target {
        let mut filter = ObjectFilter::default();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        *target = TargetAst::Object(filter, span.clone(), None);
    }

    let Some(filter) = target_object_filter_mut(target) else {
        return false;
    };
    if !it_filter.card_types.is_empty() {
        filter.card_types = it_filter.card_types.clone();
    }
    if !it_filter.subtypes.is_empty() {
        filter.subtypes = it_filter.subtypes.clone();
    }
    if let Some(power) = &it_filter.power {
        filter.power = Some(power.clone());
        filter.power_reference = it_filter.power_reference;
    }
    if let Some(toughness) = &it_filter.toughness {
        filter.toughness = Some(toughness.clone());
        filter.toughness_reference = it_filter.toughness_reference;
    }
    if let Some(mana_value) = &it_filter.mana_value {
        filter.mana_value = Some(mana_value.clone());
    }
    true
}

fn parse_counter_target_phrase(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    if shapes::is_him_or_her_counter_target(tokens) {
        return Ok(TargetAst::Source(span_from_tokens(tokens)));
    }
    parse_target_phrase(tokens)
}

pub(crate) fn parse_put_counters(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let tokens = shapes::strip_optional_put_prefix(tokens);
    let (mut count_value, used) = parse_put_counter_count_value(tokens)?;
    let rest = &tokens[used..];
    let clause_text = render_clause_words(tokens);
    let target_shape = shapes::parse_put_counter_target_shape(rest).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter target (clause: '{}')",
            clause_text
        ))
    })?;
    let equal_to_difference = target_shape.equal_to_difference;
    let mut target_tokens = target_shape.target_tokens.to_vec();
    let mut trailing_predicate: Option<PredicateAst> = None;
    if let Some(spec) = split_trailing_if_clause_lexed(&target_tokens) {
        trailing_predicate = Some(spec.predicate);
        target_tokens = spec.leading_tokens.to_vec();
    }
    target_tokens = shapes::strip_trailing_instead(&target_tokens).to_vec();

    let wrap_conditional = |effect: EffectAst| {
        if let Some(predicate) = trailing_predicate.clone() {
            EffectAst::TrailingIf {
                predicate,
                effects: vec![effect],
            }
        } else {
            effect
        }
    };

    // An unqualified referential count ("its counters" / "this's counters")
    // names the complete counter collection to move.  Interpret that typed
    // fact before inspecting the remaining target phrase: words in the target
    // such as "creature" are otherwise valid named-counter surfaces and can
    // be mistaken for a counter descriptor.
    if let Value::CountersOn(spec, None) = &count_value {
        let target = parse_counter_target_phrase(&target_tokens)?;
        let from = target_from_counter_source_spec(spec.as_ref(), span_from_tokens(tokens))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported counter source reference (clause: '{}')",
                    render_clause_words(tokens)
                ))
            })?;
        return Ok(wrap_conditional(EffectAst::subject_verb_move_all_counters(
            from, target,
        )));
    }

    let counter_type = if let Some(counter_type) = parse_counter_type_from_tokens(rest) {
        counter_type
    } else if let Value::CountersOn(_, Some(counter_type)) = &count_value {
        *counter_type
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter type (clause: '{}')",
            render_clause_words(tokens)
        )));
    };

    if let Value::Fixed(fixed_count) = count_value
        && fixed_count >= 0
        && let Some(mut effect) = parse_put_or_remove_counter_choice(
            fixed_count as u32,
            counter_type,
            &target_tokens,
            tokens,
        )?
    {
        let mut predicate = trailing_predicate.clone();
        if let Some(PredicateAst::ItMatches(filter)) = predicate.as_ref()
            && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutOrRemoveCounters { target, .. },
                ..
            }) = &mut effect
            && merge_it_match_filter_into_target(target, filter)
        {
            predicate = None;
        }
        return Ok(if let Some(predicate) = predicate {
            EffectAst::Conditional {
                predicate,
                if_true: vec![effect],
                if_false: Vec::new(),
            }
        } else {
            effect
        });
    }

    if let Some((target_count, used)) = parse_counter_target_count_prefix(&target_tokens)? {
        let target_phrase = &target_tokens[used..];
        if target_phrase.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing counter target after count clause (clause: '{}')",
                render_clause_words(tokens)
            )));
        }
        let mut target = parse_counter_target_phrase(target_phrase)?;
        let mut predicate = trailing_predicate.clone();
        if let Some(PredicateAst::ItMatches(filter)) = predicate.as_ref()
            && merge_it_match_filter_into_target(&mut target, filter)
        {
            predicate = None;
        }
        let effect = EffectAst::subject_verb_put_counters(
            counter_type,
            count_value.clone(),
            target,
            Some(target_count),
            false,
        );
        return Ok(if let Some(predicate) = predicate {
            EffectAst::Conditional {
                predicate,
                if_true: vec![effect],
                if_false: Vec::new(),
            }
        } else {
            effect
        });
    }

    if let Some(filter_tokens) = shapes::strip_each_counter_prefix(&target_tokens) {
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(wrap_conditional(EffectAst::subject_verb_put_counters_all(
            counter_type,
            count_value,
            filter,
        )));
    }
    if let Some((base_target_tokens, count_filter_tokens)) =
        shapes::split_for_each_counter_target(&target_tokens)
    {
        let mut target = parse_counter_target_phrase(base_target_tokens)?;
        let mut predicate = trailing_predicate.clone();
        if let Some(PredicateAst::ItMatches(filter)) = predicate.as_ref()
            && merge_it_match_filter_into_target(&mut target, filter)
        {
            predicate = None;
        }
        let mut count =
            if let Some(dynamic) = parse_create_for_each_dynamic_count(count_filter_tokens) {
                dynamic
            } else {
                Value::Count(parse_object_filter(count_filter_tokens, false)?)
            };
        if let Value::Fixed(multiplier) = count_value.clone()
            && multiplier > 1
        {
            let base = count.clone();
            for _ in 1..multiplier {
                count = Value::Add(Box::new(count), Box::new(base.clone()));
            }
        }
        count = count.with_surface_hint(ValueSurfaceHint::ForEach);
        let effect = EffectAst::subject_verb_put_counters(counter_type, count, target, None, false);
        return Ok(if let Some(predicate) = predicate {
            EffectAst::Conditional {
                predicate,
                if_true: vec![effect],
                if_false: Vec::new(),
            }
        } else {
            effect
        });
    }
    let mut target = parse_counter_target_phrase(&target_tokens)?;
    if equal_to_difference {
        let target_spec =
            crate::runtime_backend::references::reference_helpers::choose_spec_for_target(&target);
        count_value = Value::Add(
            Box::new(Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                IT_TAG,
            ))))),
            Box::new(Value::Scaled(
                Box::new(Value::PowerOf(Box::new(target_spec))),
                -1,
            )),
        );
    }
    let mut predicate = trailing_predicate.clone();
    if let Some(PredicateAst::ItMatches(filter)) = predicate.as_ref()
        && merge_it_match_filter_into_target(&mut target, filter)
    {
        predicate = None;
    }
    let effect =
        EffectAst::subject_verb_put_counters(counter_type, count_value, target, None, false);
    Ok(if let Some(predicate) = predicate {
        EffectAst::Conditional {
            predicate,
            if_true: vec![effect],
            if_false: Vec::new(),
        }
    } else {
        effect
    })
}

pub(crate) fn parse_sentence_put_multiple_counters_on_target(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = shapes::parse_shared_counter_target_shape(tokens) else {
        return Ok(None);
    };
    let [first, second] = shape.descriptors.as_slice() else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;
    if matches!(target, TargetAst::WithCount(_, _)) {
        return Ok(None);
    }

    let first_effect = EffectAst::subject_verb_put_counters(
        first.counter_type,
        Value::Fixed(first.count as i32),
        target.clone(),
        None,
        false,
    );
    let second_effect = EffectAst::subject_verb_put_counters(
        second.counter_type,
        Value::Fixed(second.count as i32),
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        None,
        false,
    );

    Ok(Some(vec![first_effect, second_effect]))
}

fn parse_put_or_remove_counter_choice(
    put_count: u32,
    put_counter_type: CounterType,
    target_tokens: &[OwnedLexToken],
    clause_tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = shapes::parse_put_or_remove_counter_shape(target_tokens) else {
        return Ok(None);
    };
    let remove_counter_type = shape.remove_counter_type.unwrap_or(put_counter_type);

    let (target, target_count) = if let Some((target_count, used_target_count)) =
        parse_counter_target_count_prefix(shape.base_target_tokens)?
    {
        let target_phrase = &shape.base_target_tokens[used_target_count..];
        if target_phrase.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing counter target before put-or-remove remove clause (clause: '{}')",
                render_clause_words(clause_tokens)
            )));
        }
        (
            parse_counter_target_phrase(target_phrase)?,
            Some(target_count),
        )
    } else {
        (parse_counter_target_phrase(shape.base_target_tokens)?, None)
    };

    let target_phrase = render_clause_words(shape.base_target_tokens);
    let put_mode_text = format!(
        "Put {} on {}",
        describe_counter_phrase_for_mode(put_count, put_counter_type),
        target_phrase
    );
    let remove_mode_text = sentence_case_mode_text(&render_clause_words(shape.remove_mode_tokens));

    Ok(Some(EffectAst::subject_verb_put_or_remove_counters(
        put_counter_type,
        Value::Fixed(put_count as i32),
        remove_counter_type,
        shape.remove_count,
        put_mode_text,
        remove_mode_text,
        target,
        target_count,
    )))
}

pub(crate) fn parse_counter_target_count_prefix(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ChoiceCount, usize)>, CardTextError> {
    Ok(shapes::parse_counter_target_count_shape(tokens))
}

pub(crate) fn split_until_source_leaves_tail(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    shapes::split_until_source_leaves_shape(tokens)
}

pub(crate) fn split_until_target_leaves_tail(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    shapes::split_until_target_leaves_shape(tokens)
}

pub(crate) fn parse_half_starting_life_total_value(
    tokens: &[OwnedLexToken],
    player: PlayerAst,
) -> Option<Value> {
    let shape = shapes::parse_half_starting_life_shape(tokens)?;
    let player_filter =
        shapes::player_filter_for_half_reference(player).unwrap_or_else(|| shape.player.clone());
    if player_filter != shape.player {
        return None;
    }
    Some(match shape.rounding {
        shapes::HalfStartingLifeRounding::Up => {
            Value::HalfStartingLifeTotalRoundedUp(player_filter)
        }
        shapes::HalfStartingLifeRounding::Down => {
            Value::HalfStartingLifeTotalRoundedDown(player_filter)
        }
    })
}

fn parse_transform_like(
    tokens: &[OwnedLexToken],
    action: fn(TargetAst) -> EffectAst,
) -> Result<EffectAst, CardTextError> {
    match shapes::parse_transform_target_shape(tokens) {
        shapes::TransformTargetShape::ImplicitSource => Ok(action(TargetAst::Source(None))),
        shapes::TransformTargetShape::EachObject { filter_tokens } => {
            let filter = parse_object_filter(filter_tokens, false)?;
            Ok(EffectAst::ForEachObject {
                filter,
                effects: vec![action(TargetAst::Tagged(
                    TagKey::from(IT_TAG),
                    span_from_tokens(tokens),
                ))],
            })
        }
        shapes::TransformTargetShape::Source { surface } => {
            let span = span_from_tokens(tokens);
            if let Some(surface) = surface {
                record_source_reference_surface(span, surface);
            }
            Ok(action(TargetAst::Source(span)))
        }
        shapes::TransformTargetShape::Target {
            target_tokens,
            fallback_to_source,
        } => match parse_target_phrase(target_tokens) {
            Ok(target) => Ok(action(target)),
            Err(_) if fallback_to_source => Ok(action(TargetAst::Source(span_from_tokens(tokens)))),
            Err(err) => Err(err),
        },
    }
}

pub(crate) fn parse_transform(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    parse_transform_like(tokens, EffectAst::subject_verb_transform)
}

pub(crate) fn parse_convert(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    parse_transform_like(tokens, EffectAst::subject_verb_convert)
}

pub(crate) fn exile_subject_owner_filter(subject: Option<SubjectAst>) -> Option<PlayerFilter> {
    match subject {
        Some(SubjectAst::Player(PlayerAst::Target)) => Some(PlayerFilter::target_player()),
        Some(SubjectAst::Player(PlayerAst::TargetOpponent)) => {
            Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)))
        }
        Some(SubjectAst::Player(PlayerAst::That)) => Some(PlayerFilter::IteratedPlayer),
        Some(SubjectAst::Player(PlayerAst::You)) => Some(PlayerFilter::You),
        _ => None,
    }
}

pub(crate) fn apply_exile_subject_owner_context(
    filter: &mut ObjectFilter,
    subject: Option<SubjectAst>,
) {
    let Some(owner_filter) = exile_subject_owner_filter(subject) else {
        return;
    };
    let direct_zone_ok = matches!(
        filter.zone,
        Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
    );
    let any_of_zone_ok = filter.any_of.iter().any(|nested| {
        matches!(
            nested.zone,
            Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
        )
    });
    if !direct_zone_ok && !any_of_zone_ok {
        return;
    }
    match filter.owner {
        Some(PlayerFilter::Target(_)) | Some(PlayerFilter::IteratedPlayer) | None => {
            filter.owner = Some(owner_filter);
        }
        _ => {}
    }
}

pub(crate) fn apply_exile_subject_hand_owner_context(
    target: &mut TargetAst,
    subject: Option<SubjectAst>,
) {
    let Some(filter) = target_object_filter_mut(target) else {
        return;
    };
    if filter.zone != Some(Zone::Hand) {
        return;
    }
    apply_exile_subject_owner_context(filter, subject);
}

pub(crate) fn apply_shuffle_subject_graveyard_owner_context(
    target: &mut TargetAst,
    subject: SubjectAst,
) {
    let Some(filter) = target_object_filter_mut(target) else {
        return;
    };
    if filter.zone != Some(Zone::Graveyard) {
        return;
    }

    let owner_filter = match subject {
        SubjectAst::Player(PlayerAst::Target) => Some(PlayerFilter::target_player()),
        SubjectAst::Player(PlayerAst::TargetOpponent) => Some(PlayerFilter::target_opponent()),
        SubjectAst::Player(PlayerAst::You) => Some(PlayerFilter::You),
        _ => None,
    };
    let Some(owner_filter) = owner_filter else {
        return;
    };

    match filter.owner {
        Some(PlayerFilter::IteratedPlayer) | None => {
            filter.owner = Some(owner_filter);
        }
        _ => {}
    }
}

#[cfg(test)]
mod filtered_prior_action_counter_tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn counter_count_preserves_exiled_creature_filter() {
        let tokens = lex_line("creature card exiled this way", 0).unwrap();
        let Some(Value::PendingPriorEffectMetric(query)) =
            parse_create_for_each_dynamic_count(&tokens)
        else {
            panic!("expected a typed prior-effect metric");
        };
        assert_eq!(
            query.source,
            ironsmith_core::EffectMetricSource::AffectedObjects
        );
        assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Exiled)
        );
        let filter = query
            .filter
            .expect("metric should retain its object filter");
        assert!(
            filter
                .card_types
                .contains(&crate::types::CardType::Creature)
        );
        assert!(filter.union_surface.explicit_card_noun());
    }

    #[test]
    fn counter_count_preserves_destroyed_permanent_action() {
        let tokens = lex_line("permanent destroyed this way", 0).unwrap();
        let Some(Value::PendingPriorEffectMetric(query)) =
            parse_create_for_each_dynamic_count(&tokens)
        else {
            panic!("expected a typed prior-effect metric");
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Destroyed)
        );
        let filter = query.filter.expect("metric should retain permanent scope");
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        for card_type in [
            crate::types::CardType::Artifact,
            crate::types::CardType::Creature,
            crate::types::CardType::Enchantment,
            crate::types::CardType::Land,
            crate::types::CardType::Planeswalker,
            crate::types::CardType::Battle,
        ] {
            assert!(filter.card_types.contains(&card_type));
        }
    }

    #[test]
    fn counter_count_preserves_named_counters_on_typed_source() {
        let tokens = lex_line("invasion counter on this enchantment", 0).unwrap();
        let value = parse_create_for_each_dynamic_count(&tokens)
            .expect("named counters on a source should be a dynamic count");

        let Value::CountersOn(spec, Some(CounterType::Named("invasion"))) = value else {
            panic!("expected invasion counters on source, got {value:?}");
        };
        assert!(matches!(spec.unhinted(), ChooseSpec::Source));
        assert_eq!(
            spec.source_reference_surface()
                .map(crate::target::SourceReferenceSurface::display_text),
            Some("this enchantment".to_string())
        );
    }
}

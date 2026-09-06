use ironsmith_compiler_semantic::model::ForEachEffectAst;
use crate::cards::builders::{
    EffectAst, PredicateAst, SubjectVerbActionAst, SubjectVerbEffectAst, TargetAst, CounterActionAst, GrantActionAst, LibraryActionAst, CharacteristicActionAst, ZoneMoveActionAst, RevealLookActionAst, LifeResourceActionAst, ChoiceActionAst, DamageActionAst, StatChangeActionAst, DelayedEffectAst, ObjectChoiceEffectAst, VoteEffectAst, ConditionalEffectAst, PermissionEffectAst,
};
use crate::effect::Value;
use ironsmith_core::ValueSurfaceHint;

fn source_counter_removal(effect: &EffectAst) -> Option<crate::object::CounterType> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters {
                amount,
                target: TargetAst::Source(_),
                counter_type: Some(counter_type),
                up_to: false,
                distributed_across_all: false,
                all_of_them: false,
            }),
        ..
    }) = effect
    else {
        return None;
    };
    matches!(amount.unhinted(), Value::CountersOnSource(kind) if kind == counter_type)
        .then_some(*counter_type)
}

fn bind_damage_amount_to_removed_counter_count(
    effect: &mut EffectAst,
    counter_type: crate::object::CounterType,
) -> usize {
    let mut bound = 0;
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect {
        let amount = match action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { amount, .. }) => Some(amount),
            _ => None,
        };
        if let Some(amount) = amount
            && matches!(
                amount.unhinted(),
                Value::EventValue(crate::effect::EventValueSpec::Amount)
            )
        {
            let hints = amount.surface_hints().to_vec();
            *amount = Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::Outcome,
                    ironsmith_core::EffectMetric::Count,
                )
                .with_action(ironsmith_core::PriorEffectAction::Removed)
                .with_counter_type(Some(counter_type)),
            )
            .with_surface_hints(hints);
            bound += 1;
        }
    }
    super::effect_ast_traversal::for_each_nested_effects_mut(effect, true, |nested| {
        for child in nested {
            bound += bind_damage_amount_to_removed_counter_count(child, counter_type);
        }
    });
    bound
}

fn is_removed_counter_damage_fanout_member(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) => matches!(
            action,
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { .. })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { .. })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { .. })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { .. })
        ),
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. }) => {
            !effects.is_empty() && effects.iter().all(is_removed_counter_damage_fanout_member)
        }
        _ => false,
    }
}

fn bind_removed_counter_damage_fanout(effects: &mut [EffectAst]) -> bool {
    let [removal, damage @ ..] = effects else {
        return false;
    };
    let Some(counter_type) = source_counter_removal(removal) else {
        return false;
    };
    if damage.is_empty() || !damage.iter().all(is_removed_counter_damage_fanout_member) {
        return false;
    }
    let mut rebound = damage.to_vec();
    let bound = rebound
        .iter_mut()
        .map(|effect| bind_damage_amount_to_removed_counter_count(effect, counter_type))
        .sum::<usize>();
    if bound < 2 {
        return false;
    }
    damage.clone_from_slice(&rebound);
    true
}

pub fn normalize_effects_ast(effects: &[EffectAst]) -> Vec<EffectAst> {
    let mut normalized = effects.to_vec();
    normalize_effects_ast_in_place(&mut normalized);
    normalized
}

pub fn normalize_effects_ast_in_place(effects: &mut Vec<EffectAst>) {
    bind_typed_where_x_references(effects, None);
    normalize_effects_vec(effects);
}

fn typed_where_x_binding(effect: &EffectAst) -> Option<Value> {
    let EffectAst::SubjectVerb(subject_verb) = effect else {
        return None;
    };
    let SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { count, .. }) = &subject_verb.action else {
        return None;
    };
    let Value::SurfaceHinted { value, hints } = count else {
        return None;
    };
    hints
        .contains(&ValueSurfaceHint::WhereXIs)
        .then(|| value.as_ref().clone())
}

fn replace_bound_x_in_value(value: &mut Value, replacement: &Value) {
    match value {
        Value::X => *value = replacement.clone(),
        Value::XTimes(multiplier) => {
            let multiplier = *multiplier;
            *value = if multiplier == 1 {
                replacement.clone()
            } else if let Value::Fixed(fixed) = replacement {
                Value::Fixed(fixed * multiplier)
            } else {
                Value::Scaled(Box::new(replacement.clone()), multiplier)
            };
        }
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => replace_bound_x_in_value(value, replacement),
        Value::Add(left, right) | Value::Min(left, right) => {
            replace_bound_x_in_value(left, replacement);
            replace_bound_x_in_value(right, replacement);
        }
        _ => {}
    }
}

fn replace_bound_x_in_predicate(predicate: &mut PredicateAst, replacement: &Value) {
    match predicate {
        PredicateAst::ValueComparison { left, right, .. } => {
            replace_bound_x_in_value(left, replacement);
            replace_bound_x_in_value(right, replacement);
        }
        PredicateAst::ValueIsPrime(value) => replace_bound_x_in_value(value, replacement),
        PredicateAst::Not(inner) => replace_bound_x_in_predicate(inner, replacement),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            replace_bound_x_in_predicate(left, replacement);
            replace_bound_x_in_predicate(right, replacement);
        }
        _ => {}
    }
}

fn bind_typed_where_x_references(effects: &mut [EffectAst], inherited: Option<Value>) {
    let mut binding = inherited;
    for effect in effects {
        match effect {
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            })
            | EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                ..
            } => {
                if let Some(replacement) = binding.as_ref() {
                    replace_bound_x_in_predicate(predicate, replacement);
                }
                bind_typed_where_x_references(if_true, binding.clone());
                bind_typed_where_x_references(if_false, binding.clone());
            }
            EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { predicate, effects })
            | EffectAst::Conditionals(ConditionalEffectAst::TrailingUnless { predicate, effects }) => {
                if let Some(replacement) = binding.as_ref() {
                    replace_bound_x_in_predicate(predicate, replacement);
                }
                bind_typed_where_x_references(effects, binding.clone());
            }
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { modes }) | EffectAst::ObjectChoices(ObjectChoiceEffectAst::VillainousChoice { modes, .. }) => {
                for mode in modes {
                    bind_typed_where_x_references(&mut mode.effects, binding.clone());
                }
            }
            EffectAst::Conditionals(ConditionalEffectAst::IfEffectDidNotHappen { effect, otherwise }) => {
                bind_typed_where_x_references(
                    std::slice::from_mut(effect.as_mut()),
                    binding.clone(),
                );
                bind_typed_where_x_references(otherwise, binding.clone());
            }
            EffectAst::Conditionals(ConditionalEffectAst::IfEffectResult {
                effect, if_true, ..
            }) => {
                bind_typed_where_x_references(
                    std::slice::from_mut(effect.as_mut()),
                    binding.clone(),
                );
                bind_typed_where_x_references(if_true, binding.clone());
            }
            EffectAst::TagAffected { effect, .. } => bind_typed_where_x_references(
                std::slice::from_mut(effect.as_mut()),
                binding.clone(),
            ),
            _ => super::effect_ast_traversal::for_each_nested_effects_mut(effect, true, |nested| {
                bind_typed_where_x_references(nested, binding.clone())
            }),
        }

        if let Some(next_binding) = typed_where_x_binding(effect) {
            binding = Some(next_binding);
        }
    }
}

fn normalize_effects_vec(effects: &mut Vec<EffectAst>) {
    for effect in effects.iter_mut() {
        normalize_nested_effects(effect);
        collapse_single_nested_coordination(effect);
        normalize_singular_source_exiled_move(effect);
    }
    // A full-card parse can normalize a named source reference only after the
    // narrow removal/damage sentence recognizer has run. Recover the same
    // typed shared-result provenance at this common AST boundary once the
    // producer is provably a source-bound counter removal and every following
    // member belongs to the damage fanout.
    bind_removed_counter_damage_fanout(effects);
    bind_explicit_chosen_object_followups(effects);
    correlate_conditional_quantified_choice_followups(effects);
    correlate_split_for_each_player_choice_complements(effects);
    bind_all_players_subtype_choices_to_destroy_exclusion(effects);
    bind_all_players_subtype_choices_to_return_inclusion(effects);
    bind_quantified_choice_collections_to_destroy_followups(effects);
    bind_counted_set_followups(effects);
    bind_until_next_turn_permissions_to_prior_exiled_collection(effects);
    bind_choice_remainder_to_choice_domain(effects);
    bind_consult_remainder_to_revealed_collection(effects);
    if let Some(rewritten) = rewrite_repeat_process(effects) {
        *effects = rewritten;
    }
    if let Some(rewritten) = rewrite_repeat_process_may(effects) {
        *effects = rewritten;
    }
    if let Some(rewritten) = rewrite_repeat_process_once(effects) {
        *effects = rewritten;
    }
    if let Some(rewritten) = rewrite_return_as_aura(effects) {
        *effects = rewritten;
    }
    effects.retain(|effect| !is_noop_effect(effect));
}

/// Resolve “the rest of the revealed cards” against the two collections
/// exported by the nearest typed library consult. The generic move grammar
/// deliberately leaves `rest` unresolved; this canonical AST pass owns the
/// cross-sentence set difference and never consults source text.
fn bind_consult_remainder_to_revealed_collection(effects: &mut [EffectAst]) {
    let mut latest_consult = None;
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                    all_tag,
                    match_tag,
                    player,
                    ..
                }),
            ..
        }) = effect
        {
            latest_consult = Some((all_tag.clone(), match_tag.clone(), *player));
            continue;
        }

        let Some((all_tag, match_tag, player)) = latest_consult.clone() else {
            continue;
        };
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            continue;
        };
        let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
            target: TargetAst::Tagged(tag, _),
            zone,
            library_order,
            library_order_chooser,
            ..
        }) = &subject_verb.action
        else {
            continue;
        };
        if tag.as_str() != crate::tag::CompilerReferenceTag::Rest.as_str() {
            continue;
        }
        subject_verb.action = if *zone == crate::zone::Zone::Library {
            let Some(order) = *library_order else {
                continue;
            };
            SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag: all_tag,
                keep_tagged: Some(match_tag),
                order,
                player: if matches!(
                    library_order_chooser,
                    crate::cards::builders::PlayerAst::Implicit
                ) {
                    player
                } else {
                    *library_order_chooser
                },
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            })
        } else {
            SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {
                tag: all_tag,
                keep_tagged: match_tag,
                zone: *zone,
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            })
        };
    }
}

/// Bind an adjacent object choice's `the rest` consumer to the exact
/// complement of that choice domain. Inside a quantified-player loop, the
/// chosen tag is iteration-local, so the complement must retain the same
/// owner/controller and zone constraints as its producer.
fn bind_choice_remainder_to_choice_domain(effects: &mut [EffectAst]) {
    for index in 1..effects.len() {
        let (before, after) = effects.split_at_mut(index);
        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, tag, .. }) = &before[index - 1] else {
            continue;
        };
        let EffectAst::SubjectVerb(subject_verb) = &mut after[0] else {
            continue;
        };
        let target = match &mut subject_verb.action {
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { target, .. }) => target,
            _ => continue,
        };
        if !matches!(
            target,
            TargetAst::Tagged(rest, _)
                if rest.as_str() == crate::tag::CompilerReferenceTag::Rest.as_str()
        ) {
            continue;
        }
        *target = TargetAst::Object(filter.clone().not_tagged(tag.clone()), None, None);
    }
}

/// A document-level clause can preserve coordination around a sentence that
/// was already recognized as one coordinated typed effect. The outer
/// one-member wrapper carries no additional operator or scope; retaining it
/// produces a nested runtime sequence and hides the actual executable
/// members from consumers. Canonicalize that redundant wrapper before
/// reference annotation and lowering.
fn collapse_single_nested_coordination(effect: &mut EffectAst) {
    loop {
        let EffectAst::Coordinated { effects, .. } = effect else {
            return;
        };
        let replacement = match effects.as_slice() {
            [nested @ EffectAst::Coordinated { .. }] => Some(nested.clone()),
            [
                sentence @ EffectAst::SourceSentence {
                    effects: sentence_effects,
                    ..
                },
            ] if matches!(sentence_effects.as_slice(), [EffectAst::Coordinated { .. }]) => {
                Some(sentence.clone())
            }
            _ => None,
        };
        let Some(replacement) = replacement else {
            return;
        };
        *effect = replacement;
    }
}

/// Bind a later temporary play permission to the exact collection produced by
/// a prior top-of-library exile, including when that producer lives inside a
/// delayed or quantified wrapper.
///
/// Standalone permission sentences initially use `crate::tag::CompilerReferenceTag::It.as_str()`.  Resolving that
/// through the generic last-object channel is wrong when the intervening
/// producer is wrapped: the wrapper's watched/iterated object remains the
/// generic antecedent even though "it" / "those cards" refers to the newly
/// exiled collection.  Preserve explicit tags and require one unambiguous
/// exile collection before carrying the tag across the boundary.
fn bind_until_next_turn_permissions_to_prior_exiled_collection(effects: &mut [EffectAst]) {
    fn collect_exiled_tags(effect: &EffectAst, tags: &mut Vec<crate::tag::TagKey>) {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                    tags: moved_tags,
                    accumulated_tags,
                    ..
                }),
            ..
        }) = effect
        {
            for tag in moved_tags.iter().chain(accumulated_tags) {
                if !tags.contains(tag) {
                    tags.push(tag.clone().into());
                }
            }
        }
        super::effect_ast_traversal::for_each_nested_effects(effect, true, |nested| {
            for child in nested {
                collect_exiled_tags(child, tags);
            }
        });
    }

    fn rebind_unresolved_permissions(effect: &mut EffectAst, exiled_tag: &crate::tag::TagKey) {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn { tag, .. }),
            ..
        }) = effect
            && (tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                || tag.as_str().starts_with("damaged_")
                || tag.as_str().starts_with("pumped_"))
        {
            *tag = ironsmith_compiler_semantic::tag::TagRef::of(exiled_tag.clone());
        }
        super::effect_ast_traversal::for_each_nested_effects_mut(effect, true, |nested| {
            for child in nested {
                rebind_unresolved_permissions(child, exiled_tag);
            }
        });
    }

    let mut prior_exiled_tag = None;
    for effect in effects {
        if let Some(tag) = prior_exiled_tag.as_ref() {
            rebind_unresolved_permissions(effect, tag);
        }

        let mut tags = Vec::new();
        collect_exiled_tags(effect, &mut tags);
        prior_exiled_tag = match tags.as_slice() {
            [tag] => Some(tag.clone()),
            [] => prior_exiled_tag,
            _ => None,
        };
    }
}

/// one-object move even though the generic non-target subject path defaults
/// object filters to `all`. The explicit singular surface plus the typed
/// source-exile identity are both required before removing that broadening.
fn normalize_singular_source_exiled_move(effect: &mut EffectAst) {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                target: TargetAst::Object(filter, ..),
                zone,
                target_plural_surface,
                all,
                ..
            }),
        ..
    }) = effect
        && *all
        && !*target_plural_surface
        && *zone == crate::zone::Zone::Graveyard
        && let [constraint] = filter.tagged_constraints.as_slice()
        && constraint.tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
        && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    {
        let mut remainder = filter.clone();
        remainder.zone = None;
        remainder.tagged_constraints.clear();
        remainder.union_surface = Default::default();
        if remainder == crate::filter::ObjectFilter::default() {
            *all = false;
        }
        return;
    }

    super::effect_ast_traversal::for_each_nested_effects_mut(effect, true, |nested| {
        for child in nested {
            normalize_singular_source_exiled_move(child);
        }
    });
}

fn single_subtype_choice_family(effect: &EffectAst) -> Option<crate::types::SubtypeFamily> {
    match effect {
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. } => {
            let [effect] = effects.as_slice() else {
                return None;
            };
            single_subtype_choice_family(effect)
        }
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCreatureType { family, .. }) => Some(*family),
            _ => None,
        },
        _ => None,
    }
}

fn all_players_choose_one_subtype_family(
    effect: &EffectAst,
) -> Option<crate::types::SubtypeFamily> {
    match effect {
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. } => {
            let [effect] = effects.as_slice() else {
                return None;
            };
            all_players_choose_one_subtype_family(effect)
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }) => {
            let [effect] = effects.as_slice() else {
                return None;
            };
            single_subtype_choice_family(effect)
        }
        _ => None,
    }
}

fn filter_is_misbound_chosen_subtype_result(
    filter: &crate::filter::ObjectFilter,
    family: crate::types::SubtypeFamily,
) -> bool {
    if family != crate::types::SubtypeFamily::Creature
        || !matches!(
            filter.card_types.as_slice(),
            [crate::types::CardType::Creature]
        )
        || filter.tagged_constraints.len() != 1
    {
        return false;
    }
    let mut expected = crate::filter::ObjectFilter::creature().match_tagged(
        crate::tag::CompilerReferenceTag::It.bind(),
        crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    );
    // The ordinary object-filter route may leave the implicit permanent zone
    // unstated until lowering. Treat those two forms as the same exact shape.
    if filter.zone.is_none() {
        expected.zone = None;
    }
    filter == &expected
}

/// A subtype chosen "this way" is characteristic data stored on the source,
/// not an object-result collection. Some public multi-sentence routes see the
/// terminal words first and temporarily bind `chosen this way` to the generic
/// object tag. Repair only the exact all-players subtype-choice followed by an
/// otherwise-plain destroy-all object filter, retaining ordinary chosen-object
/// procedures unchanged.
fn bind_all_players_subtype_choices_to_destroy_exclusion(effects: &mut [EffectAst]) {
    for consumer_index in 1..effects.len() {
        let Some(family) = all_players_choose_one_subtype_family(&effects[consumer_index - 1])
        else {
            continue;
        };
        let Some(filter) = direct_destroy_filter_mut(&mut effects[consumer_index]) else {
            continue;
        };
        if !filter_is_misbound_chosen_subtype_result(filter, family) {
            continue;
        }

        filter.tagged_constraints.clear();
        filter.excluded_any_chosen_creature_type = true;
        filter.set_chosen_type_this_way_surface(true);
    }
}

fn all_players_return_all_filter_mut(
    effect: &mut EffectAst,
) -> Option<&mut crate::filter::ObjectFilter> {
    match effect {
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. } => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            all_players_return_all_filter_mut(effect)
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }) => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            match effect {
                EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
                    SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToBattlefield { filter, .. }) => Some(filter),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

/// Bind an all-players subtype choice to a following all-players return. The
/// source stores every simultaneously chosen subtype, so the consumer must
/// match the union of those choices rather than only the last submitted one.
fn bind_all_players_subtype_choices_to_return_inclusion(effects: &mut [EffectAst]) {
    for consumer_index in 1..effects.len() {
        if all_players_choose_one_subtype_family(&effects[consumer_index - 1])
            != Some(crate::types::SubtypeFamily::Creature)
        {
            continue;
        }
        let Some(filter) = all_players_return_all_filter_mut(&mut effects[consumer_index]) else {
            continue;
        };
        if filter.zone != Some(crate::zone::Zone::Graveyard)
            || filter.owner != Some(crate::target::PlayerFilter::IteratedPlayer)
            || !matches!(
                filter.card_types.as_slice(),
                [crate::types::CardType::Creature]
            )
            || filter.prior_effect_action_surface()
                != Some(ironsmith_core::PriorEffectAction::Chosen)
        {
            continue;
        }
        filter.chosen_creature_type = true;
        filter.set_chosen_type_this_way_surface(true);
    }
}

fn quantified_player_choice_effects_mut(effect: &mut EffectAst) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. }) => Some(effects),
        EffectAst::SourceSentence { effects, .. } => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            quantified_player_choice_effects_mut(effect)
        }
        _ => None,
    }
}

fn retag_quantified_choice_collection(effect: &mut EffectAst) -> bool {
    let Some(choice_effects) = quantified_player_choice_effects_mut(effect) else {
        return false;
    };
    let Some(original_tag) = common_object_choice_tag(choice_effects) else {
        return false;
    };
    if !choice_collection_tag_can_accumulate(&original_tag) {
        return false;
    }
    for effect in choice_effects {
        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. }) = effect else {
            return false;
        };
        *tag = crate::tag::CompilerReferenceTag::ChosenObjects.bind();
    }
    true
}

/// Return whether `effect` is made entirely from object-choice producers and
/// whether at least one of those choices is repeated/quantified. This keeps
/// the later binding conservative: ordinary one-off target choices continue
/// to use the normal antecedent resolver, while a union built across players
/// or repetitions receives a durable accumulating tag.
fn choice_collection_producer_is_quantified(effect: &EffectAst) -> Option<bool> {
    fn sequence_kind(effects: &[EffectAst]) -> Option<bool> {
        if effects.is_empty() {
            return None;
        }
        let mut quantified = false;
        for effect in effects {
            quantified |= choice_collection_producer_is_quantified(effect)?;
        }
        Some(quantified)
    }

    match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary { .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone { .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { .. }) => Some(false),
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. }) => sequence_kind(effects).map(|_| true),
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ResultBranchLabel { effects, .. }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. }) => sequence_kind(effects),
        EffectAst::TagAffected { effect, .. } => choice_collection_producer_is_quantified(effect),
        EffectAst::Coordination(coordination) => {
            let mut quantified = false;
            let mut any = false;
            for effect in coordination.effects() {
                any = true;
                quantified |= choice_collection_producer_is_quantified(effect)?;
            }
            any.then_some(quantified)
        }
        _ => None,
    }
}

fn choice_collection_tag_can_accumulate(tag: &crate::tag::TagKey) -> bool {
    tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        || tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        || tag.as_str().starts_with("participant_choice_l")
}

fn choice_collection_producer_has_accumulating_tags(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { tag, .. }) => {
            choice_collection_tag_can_accumulate(tag)
        }
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ResultBranchLabel { effects, .. }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. }) => {
            !effects.is_empty()
                && effects
                    .iter()
                    .all(choice_collection_producer_has_accumulating_tags)
        }
        EffectAst::TagAffected { effect, .. } => {
            choice_collection_producer_has_accumulating_tags(effect)
        }
        EffectAst::Coordination(coordination) => {
            let mut effects = coordination.effects();
            effects
                .next()
                .is_some_and(choice_collection_producer_has_accumulating_tags)
                && effects.all(choice_collection_producer_has_accumulating_tags)
        }
        _ => false,
    }
}

fn retag_choice_collection_producer(effect: &mut EffectAst, durable_tag: &crate::tag::TagKey) {
    match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { tag, .. }) => {
            if choice_collection_tag_can_accumulate(tag) {
                *tag = ironsmith_compiler_semantic::tag::TagRef::of(durable_tag.clone());
            }
        }
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ResultBranchLabel { effects, .. }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. }) => {
            for effect in effects {
                retag_choice_collection_producer(effect, durable_tag);
            }
        }
        EffectAst::TagAffected { effect, .. } => {
            retag_choice_collection_producer(effect, durable_tag)
        }
        EffectAst::Coordination(coordination) => {
            for effect in coordination.effects_mut() {
                retag_choice_collection_producer(effect, durable_tag);
            }
        }
        _ => unreachable!("choice producer shape changed between inspection and retagging"),
    }
}

fn target_only_collection_tag_mut(effect: &mut EffectAst) -> Option<&mut crate::tag::TagKey> {
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TargetOnly { .. },
            ..
        })
    ) {
        let target_only = std::mem::replace(
            effect,
            EffectAst::Sequence {
                effects: Vec::new(),
            },
        );
        *effect = EffectAst::TagAffected {
            effect: Box::new(target_only),
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        };
    }
    match effect {
        EffectAst::TagAffected { effect, tag }
            if matches!(
                effect.as_ref(),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::TargetOnly { .. },
                    ..
                })
            ) =>
        {
            Some(&mut tag.key)
        }
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::SourceSentence { effects, .. } => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            target_only_collection_tag_mut(effect)
        }
        EffectAst::Coordination(coordination) => {
            let mut effects = coordination.effects_mut();
            let effect = effects.next()?;
            if effects.next().is_some() {
                return None;
            }
            target_only_collection_tag_mut(effect)
        }
        _ => None,
    }
}

/// Give an ordinary object choice the durable chosen-set tag when an
/// immediately following effect explicitly refers to "the chosen objects".
/// The parser uses `__it__` for standalone choices, while the consumer uses
/// the reserved chosen-set alias; assigning the producer that same durable
/// tag preserves both runtime collection identity and authored rendering.
fn bind_explicit_chosen_object_followups(effects: &mut [EffectAst]) {
    for consumer_index in 1..effects.len() {
        if !super::compile_support::effect_references_tag(
            &effects[consumer_index],
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str(),
        ) && !iterated_object_effect_uses_prior_choice(&effects[consumer_index])
        {
            continue;
        }
        if choice_collection_producer_has_accumulating_tags(&effects[consumer_index - 1]) {
            retag_choice_collection_producer(
                &mut effects[consumer_index - 1],
                &crate::tag::CompilerReferenceTag::ChosenObjects.bind(),
            );
            continue;
        }

        // Explicit target declarations are also choice producers. A later
        // "the chosen cards" consumer can be separated by an additional cost
        // and its result branch, so bind the nearest preceding declaration
        // rather than requiring adjacency.
        if let Some(tag) = effects[..consumer_index]
            .iter_mut()
            .rev()
            .find_map(target_only_collection_tag_mut)
        {
            *tag = (crate::tag::CompilerReferenceTag::ChosenObjects.bind()).into();
        }
    }
}

fn iterated_object_effect_uses_prior_choice(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. }) => effects.iter().any(|effect| match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy { source, .. }),
                ..
            }) => target_has_demonstrative_it_reference(source),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { target, .. }),
                ..
            }) => target_has_demonstrative_it_reference(target),
            _ => false,
        }),
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::SourceSentence { effects, .. } => {
            effects.iter().any(iterated_object_effect_uses_prior_choice)
        }
        EffectAst::Coordination(coordination) => coordination
            .effects()
            .any(iterated_object_effect_uses_prior_choice),
        _ => false,
    }
}

fn target_has_demonstrative_it_reference(target: &TargetAst) -> bool {
    let TargetAst::Object(filter, _, _) = target else {
        return false;
    };
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) && matches!(
        filter.source_surface.as_ref(),
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(surface))
            if surface.starts_with("that ")
    )
}

fn normalized_choice_collection_object_kind(
    filter: &crate::filter::ObjectFilter,
) -> crate::filter::ObjectFilter {
    let mut kind = filter.clone();
    kind.zone = None;
    kind.controller = None;
    kind.owner = None;
    kind.other = false;
    kind.tagged_constraints.clear();
    kind
}

fn choice_collection_producer_matches_object_kind(
    effect: &EffectAst,
    expected: &crate::filter::ObjectFilter,
) -> bool {
    match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { filter, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary { filter, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfZone { filter, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { filter, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { filter, .. }) => {
            normalized_choice_collection_object_kind(filter) == *expected
        }
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ResultBranchLabel { effects, .. }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. }) => {
            !effects.is_empty()
                && effects
                    .iter()
                    .all(|effect| choice_collection_producer_matches_object_kind(effect, expected))
        }
        EffectAst::TagAffected { effect, .. } => {
            choice_collection_producer_matches_object_kind(effect, expected)
        }
        EffectAst::Coordination(coordination) => {
            let mut effects = coordination.effects();
            effects.next().is_some_and(|effect| {
                choice_collection_producer_matches_object_kind(effect, expected)
            }) && effects
                .all(|effect| choice_collection_producer_matches_object_kind(effect, expected))
        }
        _ => false,
    }
}

fn direct_destroy_filter_mut(effect: &mut EffectAst) -> Option<&mut crate::filter::ObjectFilter> {
    match effect {
        EffectAst::SourceSentence { effects, .. } => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            direct_destroy_filter_mut(effect)
        }
        EffectAst::TagAffected { effect, .. } => direct_destroy_filter_mut(effect),
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. }) => Some(filter),
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
                target: TargetAst::Object(filter, _, _),
                ..
            }) => Some(filter),
            _ => None,
        },
        _ => None,
    }
}

fn direct_destroy_filter(effect: &EffectAst) -> Option<&crate::filter::ObjectFilter> {
    match effect {
        EffectAst::SourceSentence { effects, .. } => {
            let [effect] = effects.as_slice() else {
                return None;
            };
            direct_destroy_filter(effect)
        }
        EffectAst::TagAffected { effect, .. } => direct_destroy_filter(effect),
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. }) => Some(filter),
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
                target: TargetAst::Object(filter, _, _),
                ..
            }) => Some(filter),
            _ => None,
        },
        _ => None,
    }
}

fn direct_destroy_consumes_choice_collection(effect: &EffectAst) -> bool {
    let Some(filter) = direct_destroy_filter(effect) else {
        return false;
    };
    filter.other
        || filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                || constraint.tag.as_str()
                    == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        })
}

/// Bind a quantified/repeated choice union to the immediately following
/// destroy instruction. `ChooseObjectsEffect` accumulates explicit tags at
/// runtime, so changing the producer from the ephemeral `it` tag to the
/// reserved chosen-set tag preserves every iteration. A surface `all other`
/// destroy is then made explicit as the complement of that same union.
fn bind_quantified_choice_collections_to_destroy_followups(effects: &mut [EffectAst]) {
    for consumer_index in 1..effects.len() {
        if !direct_destroy_consumes_choice_collection(&effects[consumer_index]) {
            continue;
        }

        let mut producer_start = consumer_index;
        let mut has_quantified_producer = false;
        while producer_start > 0 {
            let Some(quantified) =
                choice_collection_producer_is_quantified(&effects[producer_start - 1])
            else {
                break;
            };
            has_quantified_producer |= quantified;
            producer_start -= 1;
        }
        if producer_start == consumer_index || !has_quantified_producer {
            continue;
        }

        let producers = &effects[producer_start..consumer_index];
        if !producers
            .iter()
            .all(choice_collection_producer_has_accumulating_tags)
        {
            continue;
        }
        let Some(destroy_filter) = direct_destroy_filter(&effects[consumer_index]) else {
            continue;
        };
        if destroy_filter.other {
            let destroy_kind = normalized_choice_collection_object_kind(destroy_filter);
            if !producers.iter().all(|producer| {
                choice_collection_producer_matches_object_kind(producer, &destroy_kind)
            }) {
                continue;
            }
        }

        let durable_tag = crate::tag::CompilerReferenceTag::ChosenObjects.bind();
        for producer in &mut effects[producer_start..consumer_index] {
            retag_choice_collection_producer(producer, &durable_tag);
        }

        let Some(filter) = direct_destroy_filter_mut(&mut effects[consumer_index]) else {
            continue;
        };
        for constraint in &mut filter.tagged_constraints {
            if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
                constraint.tag = durable_tag.clone().into();
            }
        }
        if filter.other {
            filter.other = false;
            if !filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == durable_tag.key.clone()
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            }) {
                filter
                    .tagged_constraints
                    .push(crate::filter::TaggedObjectConstraint {
                        tag: durable_tag.key.clone(),
                        relation: crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
                    });
            }
        }
    }
}

fn direct_destroy_references_chosen_collection(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SourceSentence { effects, .. } => {
            let [effect] = effects.as_slice() else {
                return false;
            };
            direct_destroy_references_chosen_collection(effect)
        }
        EffectAst::TagAffected { effect, .. } => {
            direct_destroy_references_chosen_collection(effect)
        }
        EffectAst::SubjectVerb(subject_verb)
            if matches!(&subject_verb.action, SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { .. })) =>
        {
            super::compile_support::effect_references_tag(
                effect,
                crate::tag::CompilerReferenceTag::ChosenObjects.as_str(),
            )
        }
        _ => false,
    }
}

/// A plural chosen-set continuation can be authored as a new sentence after
/// a leading conditional. If that action consumes the reserved aggregate
/// choice tag, it is both a semantic continuation of the quantified choice
/// and a no-op when the condition is false. Keep the producer and consumer in
/// one branch and give them the same durable tag before reference lowering.
pub fn correlate_conditional_quantified_choice_followups(effects: &mut Vec<EffectAst>) -> bool {
    let mut index = 0usize;
    let mut changed = false;
    while index + 1 < effects.len() {
        let follows_conditional_choice = {
            let (before, after) = effects.split_at_mut(index + 1);
            let EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                if_true, if_false, ..
            }) = &mut before[index]
            else {
                index += 1;
                continue;
            };
            if !if_false.is_empty()
                || if_true.len() != 1
                || !direct_destroy_references_chosen_collection(&after[0])
            {
                false
            } else {
                retag_quantified_choice_collection(&mut if_true[0])
            }
        };
        if follows_conditional_choice {
            let followup = effects.remove(index + 1);
            let EffectAst::Conditionals(ConditionalEffectAst::Conditional { if_true, .. }) = &mut effects[index] else {
                unreachable!("the checked effect must remain conditional")
            };
            if_true.push(followup);
            changed = true;
        }
        index += 1;
    }
    changed
}

fn source_sentence_for_each_player_effects_mut(
    effect: &mut EffectAst,
) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }) => Some(effects),
        EffectAst::SourceSentence { effects, .. } => {
            let [EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })] = effects.as_mut_slice() else {
                return None;
            };
            Some(effects)
        }
        _ => None,
    }
}

fn common_object_choice_tag(effects: &[EffectAst]) -> Option<crate::tag::TagKey> {
    let mut common = None;
    for effect in effects {
        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. }) = effect else {
            return None;
        };
        if let Some(expected) = common.as_ref()
            && expected != tag
        {
            return None;
        }
        common = Some(tag.clone());
    }
    common.map(Into::into)
}

fn replace_correlated_filter_tag(
    filter: &mut crate::filter::ObjectFilter,
    old: &crate::tag::TagKey,
    new: &crate::tag::TagKey,
) -> bool {
    let mut replaced = false;
    for constraint in &mut filter.tagged_constraints {
        if constraint.tag == *old {
            constraint.tag = new.clone();
            replaced = true;
        }
    }
    if let Some(crate::filter::ObjectRef::Tagged(tag)) = &mut filter.in_combat_with
        && tag == old
    {
        *tag = new.clone();
        replaced = true;
    }
    for comparison in &mut filter.no_shared_creature_types_with {
        let comparison_replaced = replace_correlated_filter_tag(comparison, old, new);
        if comparison_replaced && comparison.controller.is_none() {
            // The durable tag contains one choice set per player.  Restrict a
            // relational comparison to the choice made for the active player
            // iteration instead of comparing against every player's choice.
            comparison.controller = Some(crate::filter::PlayerFilter::IteratedPlayer);
        }
        replaced |= comparison_replaced;
    }
    for relation in &mut filter.characteristic_relations {
        let comparison_replaced = replace_correlated_filter_tag(&mut relation.comparison, old, new);
        if comparison_replaced && relation.comparison.controller.is_none() {
            // The durable tag contains one choice set per player. Restrict a
            // relational comparison to the choice made for the active player
            // iteration instead of comparing against every player's choice.
            relation.comparison.controller = Some(crate::filter::PlayerFilter::IteratedPlayer);
        }
        replaced |= comparison_replaced;
    }
    if let Some(targets) = filter.targets_object.as_deref_mut() {
        replaced |= replace_correlated_filter_tag(targets, old, new);
    }
    if let Some(targets) = filter.targets_only_object.as_deref_mut() {
        replaced |= replace_correlated_filter_tag(targets, old, new);
    }
    if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
        replaced |= replace_correlated_filter_tag(attached_to, old, new);
    }
    if let Some(combat_partner) = filter.blocked_or_was_blocked_by_this_turn.as_deref_mut() {
        replaced |= replace_correlated_filter_tag(combat_partner, old, new);
    }
    for branch in &mut filter.any_of {
        replaced |= replace_correlated_filter_tag(branch, old, new);
    }
    replaced
}

fn split_player_complement_filter_mut(
    effects: &mut [EffectAst],
) -> Option<&mut crate::filter::ObjectFilter> {
    let [EffectAst::SubjectVerb(subject_verb)] = effects else {
        return None;
    };
    match &mut subject_verb.action {
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice { filter, .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter }) => Some(filter),
        _ => None,
    }
}

/// Link a player-by-player choice sentence to a following player-by-player
/// complement sentence.  A durable tag accumulates all locked-in choices;
/// the complement filter then excludes that chosen set.  This preserves the
/// required two-phase ordering (all choices first, then the action) without
/// collapsing the sentences into a sequential choose/action loop.
fn correlate_split_for_each_player_choice_complements(effects: &mut [EffectAst]) {
    for index in 0..effects.len().saturating_sub(1) {
        let (before, after) = effects.split_at_mut(index + 1);
        let Some(choice_effects) = source_sentence_for_each_player_effects_mut(&mut before[index])
        else {
            continue;
        };
        if choice_effects.is_empty() {
            continue;
        }
        let Some(original_tag) = common_object_choice_tag(choice_effects) else {
            continue;
        };
        let Some(complement_effects) = source_sentence_for_each_player_effects_mut(&mut after[0])
        else {
            continue;
        };
        let Some(complement_filter) = split_player_complement_filter_mut(complement_effects) else {
            continue;
        };
        if complement_filter.controller != Some(crate::filter::PlayerFilter::IteratedPlayer)
            || !complement_filter.other
        {
            continue;
        }

        let durable_tag = if original_tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        {
            crate::tag::CompilerReferenceTag::ChosenForEachPlayer.bind()
        } else {
            ironsmith_compiler_semantic::tag::TagRef::of(original_tag.clone())
        };
        for effect in choice_effects {
            let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, tag, .. }) = effect else {
                continue;
            };
            replace_correlated_filter_tag(filter, &original_tag, &durable_tag);
            *tag = durable_tag.clone();
        }
        replace_correlated_filter_tag(complement_filter, &original_tag, &durable_tag);
        if !complement_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag == durable_tag.key.clone()
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            })
        {
            complement_filter
                .tagged_constraints
                .push(crate::filter::TaggedObjectConstraint {
                    tag: durable_tag.key.clone(),
                    relation: crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
                });
        }
        // `other` was an unresolved surface relation to the chosen set.  Once
        // encoded explicitly it must not also exempt the source permanent.
        complement_filter.other = false;
    }
}

fn count_filter(value: &Value) -> Option<&crate::filter::ObjectFilter> {
    match value {
        Value::Count(filter) => Some(filter),
        Value::SurfaceHinted { value, .. } => count_filter(value),
        _ => None,
    }
}

/// Bind a demonstrative plural grant to the set counted by the immediately
/// preceding draw. For example, in "Draw a card for each creature ... Those
/// creatures gain indestructible," the grant applies to the counted creatures,
/// not to the source spell represented by the parser's unresolved `it` target.
fn bind_counted_set_followups(effects: &mut [EffectAst]) {
    for index in 1..effects.len() {
        let (before, after) = effects.split_at_mut(index);
        let EffectAst::SubjectVerb(draw) = &before[index - 1] else {
            continue;
        };
        let SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count }) = &draw.action else {
            continue;
        };
        let Some(filter) = count_filter(count).cloned() else {
            continue;
        };

        let EffectAst::SubjectVerb(grant) = &mut after[0] else {
            continue;
        };
        let SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
            target,
            set_quantifier_surface:
                Some(
                    ironsmith_core::SetQuantifierSurface::Each
                    | ironsmith_core::SetQuantifierSurface::Those,
                ),
            ..
        }) = &mut grant.action
        else {
            continue;
        };
        let unresolved_set_reference = match target {
            TargetAst::Source(_) => true,
            TargetAst::Tagged(tag, _) => {
                tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            }
            _ => false,
        };
        if unresolved_set_reference {
            *target = TargetAst::Object(filter, None, None);
        }
    }
}

fn normalize_nested_effects(effect: &mut EffectAst) {
    match effect {
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            if_true, if_false, ..
        })
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            normalize_effects_vec(if_true);
            normalize_effects_vec(if_false);
        }
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::ResultBranchLabel { effects, .. }
        | EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::TrailingUnless { effects, .. })
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Conditionals(ConditionalEffectAst::UnlessPays { effects, .. })
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
        | EffectAst::Permissions(PermissionEffectAst::AnyPlayerMay { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::ResolvedIfResult { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::ResolvedWhenResult { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::WhenResult { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTargetPlayers { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTagged { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTaggedWithControllerAtLastBlockedBy { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponentDoesNot { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDoesNot { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachOpponentDid { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDid { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::ForEachTaggedPlayer { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::RepeatProcess { effects, .. })
        | EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. })
        | EffectAst::Votes(VoteEffectAst::BidLife {
            winner_effects: effects,
            ..
        })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextCleanupStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUntapStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUpkeep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextDrawStep { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextMainPhase { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextFirstMainPhase { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndOfCombat { effects })
        | EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectLeavesBattlefield { effects, .. })
        | EffectAst::Votes(VoteEffectAst::VoteOption { effects, .. })
        | EffectAst::ManaRestricted { effects, .. } => normalize_effects_vec(effects),
        EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
            effects,
            alternative,
            ..
        }) => {
            normalize_effects_vec(effects);
            normalize_effects_vec(alternative);
        }
        // NOTE: this walker stays hand-rolled (rather than routing through
        // effect_ast_traversal's shared helper) because normalize_effects_vec
        // resizes/replaces the Vec (retain + whole-Vec rewrites), which the
        // slice-exposing helper cannot express. New wrapper variants must be
        // added here and kept in sync with the traversal macro.
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { modes }) | EffectAst::ObjectChoices(ObjectChoiceEffectAst::VillainousChoice { modes, .. }) => {
            for mode in modes {
                normalize_effects_vec(&mut mode.effects);
            }
        }
        EffectAst::Conditionals(ConditionalEffectAst::IfEffectDidNotHappen { effect, otherwise }) => {
            normalize_nested_effects(effect);
            normalize_singular_source_exiled_move(effect);
            normalize_effects_vec(otherwise);
        }
        EffectAst::Conditionals(ConditionalEffectAst::IfEffectResult {
            effect, if_true, ..
        }) => {
            normalize_nested_effects(effect);
            normalize_singular_source_exiled_move(effect);
            normalize_effects_vec(if_true);
        }
        EffectAst::TagAffected { effect, .. } => {
            normalize_nested_effects(effect);
            normalize_singular_source_exiled_move(effect);
        }
        EffectAst::Coordination(coordination) => {
            for member in &mut coordination.members {
                normalize_effects_vec(&mut member.effects);
            }
        }
        EffectAst::ControlFlow(control) => {
            for program in &mut control.programs {
                normalize_effects_vec(&mut program.effects);
            }
        }
        _ => {}
    }
}

fn rewrite_repeat_process(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    if effects.len() < 2 {
        return None;
    }

    let last_index = effects.len() - 1;
    let EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate,
        effects: tail_effects,
    }) = &effects[last_index]
    else {
        return None;
    };
    let marker_is_direct = matches!(tail_effects.last(), Some(EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess)));
    let marker_is_coordinated = matches!(
        tail_effects.last(),
        Some(EffectAst::Coordinated { effects, .. })
            if matches!(effects.last(), Some(EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess)))
    );
    if !marker_is_direct && !marker_is_coordinated {
        return None;
    }
    // In "<action> unless <player> pays ... and repeat this process", paying is
    // the alternative action that both prevents the consequence and repeats
    // the process. Track the complete final IfResult: it is declined only when
    // this branch is selected and the UnlessPays payment succeeds.
    let repeat_follows_unless_payment = matches!(
        tail_effects.last(),
        Some(EffectAst::Coordinated { effects, .. })
            if matches!(
                effects.as_slice(),
                [EffectAst::Conditionals(ConditionalEffectAst::UnlessPays { .. }), EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess)]
            )
    );
    let continue_effect_index = if repeat_follows_unless_payment {
        last_index
    } else {
        last_index.saturating_sub(1)
    };
    let continue_predicate = if repeat_follows_unless_payment {
        crate::cards::builders::IfResultPredicate::WasDeclined
    } else {
        predicate.clone()
    };
    let mut body = effects.to_vec();
    let EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. }) = &mut body[last_index] else {
        return None;
    };
    if marker_is_direct {
        effects.pop();
    } else if let Some(EffectAst::Coordinated { effects, .. }) = effects.last_mut() {
        effects.pop();
    }
    if effects.is_empty() {
        body.pop();
    }

    Some(vec![EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
        effects: body,
        continue_effect_index,
        continue_predicate,
    })])
}

fn rewrite_repeat_process_once(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    if effects.len() < 2 || !matches!(effects.last(), Some(EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessOnce))) {
        return None;
    }

    let body = effects[..effects.len() - 1].to_vec();
    Some(vec![EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
        count: Value::Fixed(2)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::RepeatThisProcessOnce),
        effects: body,
    })])
}

fn rewrite_repeat_process_may(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    if effects.len() < 2 || !matches!(effects.last(), Some(EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessMay))) {
        return None;
    }

    Some(vec![EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
        effects: effects.to_vec(),
        continue_effect_index: effects.len() - 1,
        continue_predicate: crate::cards::builders::IfResultPredicate::Did,
    })])
}

fn rewrite_return_as_aura(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    use crate::cards::builders::{ReturnAsAuraAst, SubjectVerbActionAst, TargetAst};

    let has_return_aura_pair = effects.windows(2).any(|pair| {
        let [
            EffectAst::SubjectVerb(return_subject_verb),
            EffectAst::SubjectVerb(aura_subject_verb),
        ] = pair
        else {
            return false;
        };
        matches!(
            &return_subject_verb.action,
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield { as_aura: None, .. })
        ) && matches!(
            &aura_subject_verb.action,
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
                target: TargetAst::Tagged(tag, _),
                ..
            }) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        )
    });
    if !has_return_aura_pair {
        return None;
    }

    let mut rewritten = Vec::with_capacity(effects.len());
    let mut index = 0;
    let mut changed = false;
    while index < effects.len() {
        let Some(EffectAst::SubjectVerb(return_subject_verb)) = effects.get(index) else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield { as_aura: None, .. }) =
            &return_subject_verb.action
        else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        let Some(EffectAst::SubjectVerb(aura_subject_verb)) = effects.get(index + 1) else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        let SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
            target,
            attachment_filter,
            granted_abilities,
            ..
        }) = &aura_subject_verb.action
        else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        if !matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
        {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        }

        let mut remove_all_abilities = false;
        let mut consumed = 2;
        if let Some(EffectAst::SubjectVerb(remove_subject_verb)) = effects.get(index + 2)
            && is_return_as_aura_remove_all_marker(&remove_subject_verb.action)
        {
            remove_all_abilities = true;
            consumed = 3;
        }

        let mut combined = effects[index].clone();
        if let EffectAst::SubjectVerb(subject_verb) = &mut combined
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield { as_aura, .. }) =
                &mut subject_verb.action
        {
            *as_aura = Some(ReturnAsAuraAst {
                attachment_filter: attachment_filter.clone(),
                remove_all_abilities,
                granted_abilities: granted_abilities.clone(),
            });
        }
        rewritten.push(combined);
        index += consumed;
        changed = true;
    }

    changed.then_some(rewritten)
}

fn is_return_as_aura_remove_all_marker(action: &SubjectVerbActionAst) -> bool {
    match action {
        SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll {
            abilities,
            duration,
            ..
        }) => abilities.is_empty() && matches!(duration, crate::effect::Until::Forever),
        SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
            target,
            abilities,
            duration,
        }) => {
            abilities.is_empty()
                && matches!(duration, crate::effect::Until::Forever)
                && matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
        }
        _ => false,
    }
}

fn is_noop_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                crate::cards::builders::SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { abilities, .. })
                | crate::cards::builders::SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceAll {
                    abilities, ..
                }),
            ..
        }) => abilities.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use ironsmith_compiler_semantic::model::PermissionEffectAst;
    use ironsmith_compiler_semantic::model::ConditionalEffectAst;
    use ironsmith_compiler_semantic::model::ObjectChoiceEffectAst;
    use ironsmith_compiler_semantic::model::ForEachEffectAst;
    use ironsmith_compiler_semantic::model::DelayedEffectAst;
    use ironsmith_compiler_semantic::model::LifeResourceActionAst;
    use ironsmith_compiler_semantic::model::RandomActionAst;
    use ironsmith_compiler_semantic::model::ZoneMoveActionAst;
    use ironsmith_compiler_semantic::model::LibraryActionAst;
    use ironsmith_compiler_semantic::model::GrantActionAst;
    use crate::cards::builders::IfResultPredicate;
    use crate::cards::builders::{
        EffectAst, PlayerAst, PredicateAst, SubjectVerbActionAst, TagKey, TargetAst,
    };
    use crate::effect::{ChoiceCount, Until, Value};
    use crate::filter::{
        ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
    };
    use crate::zone::Zone;
    use ironsmith_core::ValueSurfaceHint;

    use super::normalize_effects_ast;

    #[test]
    fn normalize_removes_empty_global_grant_effect() {
        let effects = vec![EffectAst::subject_verb_grant_abilities_all(
            ObjectFilter::default(),
            Vec::new(),
            Until::EndOfTurn,
        )];

        let normalized = normalize_effects_ast(&effects);
        assert!(normalized.is_empty());
    }

    #[test]
    fn normalize_removes_empty_global_grant_effect_inside_wrappers() {
        let effects = vec![EffectAst::Permissions(PermissionEffectAst::May {
            effects: vec![
                EffectAst::subject_verb_grant_abilities_all(
                    ObjectFilter::default(),
                    Vec::new(),
                    Until::EndOfTurn,
                ),
                EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    crate::cards::builders::SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                        count: Value::Fixed(1),
                    }),
                ),
            ],
        })];

        let normalized = normalize_effects_ast(&effects);
        let EffectAst::Permissions(PermissionEffectAst::May { effects }) = &normalized[0] else {
            panic!("expected wrapped may effect");
        };
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { .. }),
                ..
            })
        ));
    }

    #[test]
    fn normalize_treats_all_players_chosen_subtypes_as_characteristics_not_objects() {
        let choose_type = EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_choose_creature_type(
                PlayerAst::That,
                Vec::new(),
            )],
        });
        let misbound = ObjectFilter::creature().match_tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            TaggedOpbjectRelation::IsTaggedObject,
        );

        let normalized =
            normalize_effects_ast(&[choose_type, EffectAst::subject_verb_destroy_all(misbound)]);
        let filter = super::direct_destroy_filter(&normalized[1]).expect("destroy filter");
        assert!(filter.tagged_constraints.is_empty(), "{filter:#?}");
        assert!(filter.excluded_any_chosen_creature_type, "{filter:#?}");
        assert!(filter.has_chosen_type_this_way_surface(), "{filter:#?}");
    }

    #[test]
    fn normalize_keeps_all_players_chosen_object_destroy_procedures_tagged() {
        let choose_object = EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: ObjectFilter::creature(),
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::That,
                tag: crate::tag::CompilerReferenceTag::It.bind(),
            })],
        });
        let chosen = ObjectFilter::creature().match_tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            TaggedOpbjectRelation::IsTaggedObject,
        );

        let normalized =
            normalize_effects_ast(&[choose_object, EffectAst::subject_verb_destroy_all(chosen)]);
        let filter = super::direct_destroy_filter(&normalized[1]).expect("destroy filter");
        assert!(!filter.excluded_any_chosen_creature_type, "{filter:#?}");
        assert_eq!(filter.tagged_constraints.len(), 1, "{filter:#?}");
    }

    #[test]
    fn normalize_binds_direct_choice_to_explicit_chosen_set_value() {
        let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: ObjectFilter::creature().controlled_by(PlayerFilter::You),
            count: ChoiceCount::exactly(2),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        });
        let chosen_filter = ObjectFilter::creature().match_tagged(
            crate::tag::CompilerReferenceTag::ChosenObjects.bind(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
        let difference = Value::absolute_difference(
            Value::GreatestPower(chosen_filter.clone()),
            Value::LeastPower(chosen_filter),
        )
        .with_surface_hint(ValueSurfaceHint::Difference);
        let draw = EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count: difference }),
        );

        let normalized = normalize_effects_ast(&[choose, draw]);
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. }), _] = normalized.as_slice() else {
            panic!("expected choice followed by draw: {normalized:#?}");
        };
        assert_eq!(
            tag.as_str(),
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        );
    }

    #[test]
    fn normalize_correlates_conditional_quantified_choice_with_chosen_set_destroy() {
        let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: ObjectFilter::permanent().controlled_by(PlayerFilter::IteratedPlayer),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        });
        let mut chosen_permanents = ObjectFilter::permanent();
        chosen_permanents
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: (crate::tag::CompilerReferenceTag::ChosenObjects.bind()).into(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        let effects = vec![
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::ThisSpellWasCastFromZone(Zone::Exile),
                if_true: vec![EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
                    effects: vec![choose],
                })],
                if_false: Vec::new(),
            }),
            EffectAst::subject_verb_destroy(TargetAst::Object(chosen_permanents, None, None)),
        ];

        let normalized = normalize_effects_ast(&effects);
        let [
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                if_true, if_false, ..
            }),
        ] = normalized.as_slice()
        else {
            panic!("expected one correlated conditional: {normalized:#?}");
        };
        assert!(if_false.is_empty());
        let [EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects }), destroy] = if_true.as_slice() else {
            panic!("expected choice and destroy in the true branch: {if_true:#?}");
        };
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })] = effects.as_slice() else {
            panic!("expected one quantified object choice: {effects:#?}");
        };
        assert_eq!(
            tag.as_str(),
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        );
        assert!(super::direct_destroy_references_chosen_collection(destroy));
    }

    #[test]
    fn normalize_binds_repeated_choices_to_destroy_complement() {
        let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: ObjectFilter::creature(),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        });
        let mut complement = ObjectFilter::creature();
        complement.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
        let normalized = normalize_effects_ast(&[
            EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
                count: Value::DistinctPowers(ObjectFilter::creature()),
                effects: vec![choose],
            }),
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. }), destroy] = normalized.as_slice() else {
            panic!("expected repeated choice followed by destroy: {normalized:#?}");
        };
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })] = effects.as_slice() else {
            panic!("expected repeated object choice: {effects:#?}");
        };
        assert_eq!(
            tag.as_str(),
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        );
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn normalize_unions_direct_and_per_player_choices_before_destroying_others() {
        let choice = || EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: ObjectFilter::permanent(),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        });
        let mut complement = ObjectFilter::permanent();
        complement.other = true;
        let normalized = normalize_effects_ast(&[
            choice(),
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::NotYou,
                effects: vec![choice()],
            }),
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag: direct, .. }),
            quantified,
            destroy,
        ] = normalized.as_slice()
        else {
            panic!("expected two choice producers and a destroy: {normalized:#?}");
        };
        let EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { effects, .. }) = quantified else {
            panic!("expected quantified choice: {quantified:#?}");
        };
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag: repeated, .. })] = effects.as_slice() else {
            panic!("expected quantified object choice: {effects:#?}");
        };
        assert_eq!(
            direct.as_str(),
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        );
        assert_eq!(
            repeated.as_str(),
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        );
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(!filter.other);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn normalize_does_not_bind_an_unrelated_destroy_after_a_repeated_choice() {
        let mut unrelated_complement = ObjectFilter::artifact();
        unrelated_complement.other = true;
        let normalized = normalize_effects_ast(&[
            EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
                count: Value::Fixed(2),
                effects: vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter: ObjectFilter::creature(),
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: crate::tag::CompilerReferenceTag::It.bind(),
                })],
            }),
            EffectAst::subject_verb_destroy_all(unrelated_complement),
        ]);

        let [EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. }), destroy] = normalized.as_slice() else {
            panic!("expected unchanged repeated choice: {normalized:#?}");
        };
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })] = effects.as_slice() else {
            panic!("expected object choice: {effects:#?}");
        };
        assert_eq!(tag.as_str(), crate::tag::CompilerReferenceTag::It.as_str());
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(filter.other);
        assert!(filter.tagged_constraints.is_empty());
    }

    #[test]
    fn normalize_preserves_custom_choice_collection_tags() {
        let custom_tag = ironsmith_compiler_semantic::tag::declared_key("custom_choice_collection");
        let mut complement = ObjectFilter::creature();
        complement.other = true;
        let normalized = normalize_effects_ast(&[
            EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
                count: Value::Fixed(2),
                effects: vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter: ObjectFilter::creature(),
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: custom_tag.clone(),
                })],
            }),
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [EffectAst::ForEach(ForEachEffectAst::RepeatEffects { effects, .. }), destroy] = normalized.as_slice() else {
            panic!("expected unchanged repeated choice: {normalized:#?}");
        };
        let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })] = effects.as_slice() else {
            panic!("expected object choice: {effects:#?}");
        };
        assert_eq!(tag, &custom_tag);
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(filter.other);
        assert!(filter.tagged_constraints.is_empty());
    }

    #[test]
    fn normalize_binds_demonstrative_grant_to_draw_counted_set() {
        let counted = ObjectFilter::creature().you_control();
        let draw = EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                count: Value::Count(counted.clone()),
            }),
        );
        let mut grant = EffectAst::subject_verb_grant_abilities_to_target(
            TargetAst::Source(None),
            Vec::new(),
            Until::EndOfTurn,
        );
        let EffectAst::SubjectVerb(grant_subject) = &mut grant else {
            panic!("expected targeted grant");
        };
        let SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
            set_quantifier_surface,
            ..
        }) = &mut grant_subject.action
        else {
            panic!("expected targeted grant action");
        };
        *set_quantifier_surface = Some(ironsmith_core::SetQuantifierSurface::Each);

        let normalized = normalize_effects_ast(&[draw, grant]);
        assert!(matches!(
            &normalized[1],
            EffectAst::SubjectVerb(subject)
                if matches!(
                    &subject.action,
                    SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                        target: TargetAst::Object(filter, _, _),
                        ..
                    }) if filter == &counted
                )
        ));
    }

    #[test]
    fn normalize_binds_later_predicate_x_to_typed_where_x_value() {
        let where_x =
            Value::CardsInHand(PlayerFilter::You).with_surface_hint(ValueSurfaceHint::WhereXIs);
        let effects = vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                where_x,
                ironsmith_compiler_semantic::tag::declared_key("looked"),
            ),
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::ValueComparison {
                    left: Value::X,
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                },
                if_true: Vec::new(),
                if_false: Vec::new(),
            }),
        ];

        let normalized = normalize_effects_ast(&effects);
        let EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::ValueComparison { left, .. },
            ..
        }) = &normalized[1]
        else {
            panic!("expected typed value comparison");
        };
        assert_eq!(*left, Value::CardsInHand(PlayerFilter::You));
    }

    #[test]
    fn normalize_rewrites_repeat_this_process_tail_into_loop_effect() {
        let effects = vec![
            EffectAst::Permissions(PermissionEffectAst::May {
                effects: vec![EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    crate::cards::builders::SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                        count: Value::Fixed(1),
                    }),
                )],
            }),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![
                    EffectAst::subject_verb(
                        crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        crate::cards::builders::SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                            amount: Value::Fixed(1),
                        }),
                    ),
                    EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess),
                ],
            }),
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
                continue_effect_index: 0,
                continue_predicate: IfResultPredicate::Did,
                ..
            })]
        ));
    }

    #[test]
    fn normalize_unless_payment_as_the_repeat_continuation_gate() {
        let effects = vec![
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                SubjectVerbActionAst::Random(RandomActionAst::FlipCoin),
            ),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                        count: Value::Fixed(1),
                    }),
                )],
            }),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                effects: vec![EffectAst::Coordinated {
                    effects: vec![
                        EffectAst::Conditionals(ConditionalEffectAst::UnlessPays {
                            effects: vec![EffectAst::subject_verb(
                                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                                PlayerAst::You,
                                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife {
                                    amount: Value::Fixed(1),
                                }),
                            )],
                            player: PlayerAst::You,
                            cost: ironsmith_core::TotalCost::from_cost(
                                crate::model::CompilerCost::Mana(
                                    crate::mana::ManaCost::from_symbols(vec![
                                        crate::mana::ManaSymbol::Generic(3),
                                    ]),
                                ),
                            ),
                            before_delayed_step: false,
                        }),
                        EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess),
                    ],
                    leading_duration: false,
                    result_conjunction: false,
                }],
            }),
        ];

        let normalized = normalize_effects_ast(&effects);
        let [
            EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
                effects,
                continue_effect_index,
                continue_predicate,
            }),
        ] = normalized.as_slice()
        else {
            panic!("expected one typed repeat process: {normalized:#?}");
        };
        assert_eq!(*continue_effect_index, 2);
        assert_eq!(*continue_predicate, IfResultPredicate::WasDeclined);
        assert!(matches!(
            effects.as_slice(),
            [
                EffectAst::SubjectVerb(_),
                EffectAst::Conditionals(ConditionalEffectAst::IfResult { .. }),
                EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                    effects: loss_effects,
                    ..
                })
            ] if matches!(
                loss_effects.as_slice(),
                [EffectAst::Coordinated {
                    effects,
                    ..
                }] if matches!(effects.as_slice(), [EffectAst::Conditionals(ConditionalEffectAst::UnlessPays { .. })])
            )
        ));
    }

    #[test]
    fn normalize_removes_empty_clash_result_marker_from_repeat_body() {
        let effects = vec![
            EffectAst::subject_verb_clash(crate::cards::builders::ClashOpponentAst::Opponent),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::WonClash,
                effects: vec![EffectAst::ForEach(ForEachEffectAst::RepeatThisProcess)],
            }),
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
                effects,
                continue_effect_index: 0,
                continue_predicate: IfResultPredicate::WonClash,
            })] if effects.len() == 1
        ));
    }

    #[test]
    fn normalize_rewrites_optional_repeat_this_process_tail_into_loop_effect() {
        let effects = vec![
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                    count: Value::Fixed(1),
                }),
            ),
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife {
                    amount: Value::Fixed(1),
                }),
            ),
            EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessMay),
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
                continue_effect_index: 2,
                continue_predicate: IfResultPredicate::Did,
                ..
            })]
        ));
    }

    #[test]
    fn normalize_preserves_repeat_this_process_once_as_a_typed_repeat() {
        let effects = vec![
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                    count: Value::Fixed(1),
                }),
            ),
            EffectAst::ForEach(ForEachEffectAst::RepeatThisProcessOnce),
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count, effects })]
                if count.unhinted() == &Value::Fixed(2)
                    && count.has_surface_hint(ValueSurfaceHint::RepeatThisProcessOnce)
                    && effects.len() == 1
        ));
    }

    #[test]
    fn normalize_binds_player_choice_remainder_to_the_same_graveyard_domain() {
        let tag = crate::tag::CompilerReferenceTag::It.bind();
        let choice_filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(crate::target::PlayerFilter::IteratedPlayer);
        let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: choice_filter,
            count: ChoiceCount::exactly(2),
            count_value: None,
            player: PlayerAst::That,
            tag: tag.clone(),
        });
        let exile_rest = EffectAst::subject_verb_exile(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Rest.bind(), None),
            false,
        );

        let normalized = normalize_effects_ast(&[choose, exile_rest]);
        let EffectAst::SubjectVerb(subject_verb) = &normalized[1] else {
            panic!("expected move-to-exile consumer");
        };
        let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
            target: TargetAst::Object(filter, ..),
            ..
        }) = &subject_verb.action
        else {
            panic!("the rest must be an executable complement: {subject_verb:#?}");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(
            filter.owner,
            Some(crate::target::PlayerFilter::IteratedPlayer)
        );
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == tag.key.clone() && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn normalize_binds_consult_remainder_to_revealed_minus_matched_collection() {
        let revealed = ironsmith_compiler_semantic::tag::declared_key("consult_revealed");
        let matched = ironsmith_compiler_semantic::tag::declared_key("consult_matched");
        let consult = EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::You,
            crate::cards::builders::LibraryConsultModeAst::Reveal,
            ObjectFilter::creature(),
            crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
            revealed.clone(),
            matched.clone(),
        );
        let remainder = EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Rest.bind(), None),
            Zone::Library,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )
        .with_library_order(
            Some(crate::cards::builders::LibraryBottomOrderAst::Random),
            PlayerAst::You,
        );

        let normalized = normalize_effects_ast(&[consult, remainder]);
        assert!(matches!(
            normalized.as_slice(),
            [
                _,
                EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        tag,
                        keep_tagged: Some(keep_tagged),
                        order: crate::cards::builders::LibraryBottomOrderAst::Random,
                        player: PlayerAst::You,
                        ..
                    }),
                    ..
                })
            ] if tag == &revealed && keep_tagged == &matched
        ));
    }

    #[test]
    fn normalize_binds_next_turn_permission_to_exile_inside_delayed_trigger() {
        let exiled_tag = ironsmith_compiler_semantic::tag::declared_key("delayed_exiled_cards");
        let delayed = EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration {
            trigger: crate::cards::builders::TriggerSpec::Dies(ObjectFilter::creature()),
            effects: vec![EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::You,
                Value::Fixed(1),
                vec![exiled_tag.clone()],
                Vec::new(),
            )],
            one_shot: false,
            duration: Until::EndOfTurn,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: None,
        });
        let grant = EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
            crate::tag::CompilerReferenceTag::It.bind(),
            PlayerAst::You,
            true,
            false,
        );

        let normalized = normalize_effects_ast(&[delayed, grant]);
        assert!(matches!(
            normalized.get(1),
            Some(EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn { tag, .. }),
                ..
            })) if tag == &exiled_tag
        ));
    }

    #[test]
    fn normalize_keeps_explicit_next_turn_permission_tag() {
        let delayed = EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration {
            trigger: crate::cards::builders::TriggerSpec::Dies(ObjectFilter::creature()),
            effects: vec![EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::You,
                Value::Fixed(1),
                vec![ironsmith_compiler_semantic::tag::declared_key("delayed_exiled_cards")],
                Vec::new(),
            )],
            one_shot: false,
            duration: Until::EndOfTurn,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: None,
        });
        let explicit_tag = ironsmith_compiler_semantic::tag::declared_key("explicit_permission_pool");
        let grant = EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
            explicit_tag.clone(),
            PlayerAst::You,
            true,
            false,
        );

        let normalized = normalize_effects_ast(&[delayed, grant]);
        assert!(matches!(
            normalized.get(1),
            Some(EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn { tag, .. }),
                ..
            })) if tag == &explicit_tag
        ));
    }
}

use crate::cards::builders::{
    CHOSEN_OBJECTS_TAG, EffectAst, IT_TAG, PredicateAst, SubjectVerbActionAst, TargetAst,
};
use crate::effect::Value;
use ironsmith_core::ValueSurfaceHint;

pub(crate) fn normalize_effects_ast(effects: &[EffectAst]) -> Vec<EffectAst> {
    let mut normalized = effects.to_vec();
    bind_typed_where_x_references(&mut normalized, None);
    normalize_effects_vec(&mut normalized);
    normalized
}

fn typed_where_x_binding(effect: &EffectAst) -> Option<Value> {
    let EffectAst::SubjectVerb(subject_verb) = effect else {
        return None;
    };
    let SubjectVerbActionAst::LookAtTopCards { count, .. } = &subject_verb.action else {
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
            EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            }
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
            EffectAst::TrailingIf { predicate, effects }
            | EffectAst::TrailingUnless { predicate, effects } => {
                if let Some(replacement) = binding.as_ref() {
                    replace_bound_x_in_predicate(predicate, replacement);
                }
                bind_typed_where_x_references(effects, binding.clone());
            }
            EffectAst::ChooseOneOf { modes } | EffectAst::VillainousChoice { modes, .. } => {
                for mode in modes {
                    bind_typed_where_x_references(&mut mode.effects, binding.clone());
                }
            }
            EffectAst::IfEffectDidNotHappen { effect, otherwise } => {
                bind_typed_where_x_references(
                    std::slice::from_mut(effect.as_mut()),
                    binding.clone(),
                );
                bind_typed_where_x_references(otherwise, binding.clone());
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
    }
    correlate_conditional_quantified_choice_followups(effects);
    correlate_split_for_each_player_choice_complements(effects);
    bind_quantified_choice_collections_to_destroy_followups(effects);
    bind_counted_set_followups(effects);
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

fn quantified_player_choice_effects_mut(effect: &mut EffectAst) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. } => Some(effects),
        EffectAst::SourceSentence { effects } => {
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
    if original_tag.as_str() != IT_TAG && original_tag.as_str() != CHOSEN_OBJECTS_TAG {
        return false;
    }
    for effect in choice_effects {
        let EffectAst::ChooseObjects { tag, .. } = effect else {
            return false;
        };
        *tag = crate::tag::TagKey::from(CHOSEN_OBJECTS_TAG);
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
        EffectAst::ChooseObjects { .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { .. }
        | EffectAst::ChooseObjectsBottomOfLibrary { .. }
        | EffectAst::ChooseObjectsTopOfLibrary { .. }
        | EffectAst::ChooseTaggedObjectsInZone { .. }
        | EffectAst::ChooseObjectsAcrossZones { .. } => Some(false),
        EffectAst::RepeatEffects { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachObject { effects, .. } => sequence_kind(effects).map(|_| true),
        EffectAst::Sequence { effects }
        | EffectAst::SourceSentence { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. } => sequence_kind(effects),
        EffectAst::TagAffected { effect, .. } => choice_collection_producer_is_quantified(effect),
        _ => None,
    }
}

fn choice_collection_tag_can_accumulate(tag: &crate::tag::TagKey) -> bool {
    matches!(tag.as_str(), IT_TAG | CHOSEN_OBJECTS_TAG)
}

fn choice_collection_producer_has_accumulating_tags(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::ChooseObjects { tag, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { tag, .. }
        | EffectAst::ChooseObjectsBottomOfLibrary { tag, .. }
        | EffectAst::ChooseObjectsTopOfLibrary { tag, .. }
        | EffectAst::ChooseTaggedObjectsInZone { tag, .. }
        | EffectAst::ChooseObjectsAcrossZones { tag, .. } => {
            choice_collection_tag_can_accumulate(tag)
        }
        EffectAst::RepeatEffects { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::SourceSentence { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. } => {
            !effects.is_empty()
                && effects
                    .iter()
                    .all(choice_collection_producer_has_accumulating_tags)
        }
        EffectAst::TagAffected { effect, .. } => {
            choice_collection_producer_has_accumulating_tags(effect)
        }
        _ => false,
    }
}

fn retag_choice_collection_producer(effect: &mut EffectAst, durable_tag: &crate::tag::TagKey) {
    match effect {
        EffectAst::ChooseObjects { tag, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { tag, .. }
        | EffectAst::ChooseObjectsBottomOfLibrary { tag, .. }
        | EffectAst::ChooseObjectsTopOfLibrary { tag, .. }
        | EffectAst::ChooseTaggedObjectsInZone { tag, .. }
        | EffectAst::ChooseObjectsAcrossZones { tag, .. } => {
            if choice_collection_tag_can_accumulate(tag) {
                *tag = durable_tag.clone();
            }
        }
        EffectAst::RepeatEffects { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::SourceSentence { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. } => {
            for effect in effects {
                retag_choice_collection_producer(effect, durable_tag);
            }
        }
        EffectAst::TagAffected { effect, .. } => {
            retag_choice_collection_producer(effect, durable_tag)
        }
        _ => unreachable!("choice producer shape changed between inspection and retagging"),
    }
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
        EffectAst::ChooseObjects { filter, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { filter, .. }
        | EffectAst::ChooseObjectsBottomOfLibrary { filter, .. }
        | EffectAst::ChooseObjectsTopOfLibrary { filter, .. }
        | EffectAst::ChooseTaggedObjectsInZone { filter, .. }
        | EffectAst::ChooseObjectsAcrossZones { filter, .. } => {
            normalized_choice_collection_object_kind(filter) == *expected
        }
        EffectAst::RepeatEffects { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::SourceSentence { effects }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. } => {
            !effects.is_empty()
                && effects
                    .iter()
                    .all(|effect| choice_collection_producer_matches_object_kind(effect, expected))
        }
        EffectAst::TagAffected { effect, .. } => {
            choice_collection_producer_matches_object_kind(effect, expected)
        }
        _ => false,
    }
}

fn direct_destroy_filter_mut(effect: &mut EffectAst) -> Option<&mut crate::filter::ObjectFilter> {
    match effect {
        EffectAst::SourceSentence { effects } => {
            let [effect] = effects.as_mut_slice() else {
                return None;
            };
            direct_destroy_filter_mut(effect)
        }
        EffectAst::TagAffected { effect, .. } => direct_destroy_filter_mut(effect),
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::DestroyAll { filter, .. } => Some(filter),
            SubjectVerbActionAst::Destroy {
                target: TargetAst::Object(filter, _, _),
                ..
            } => Some(filter),
            _ => None,
        },
        _ => None,
    }
}

fn direct_destroy_filter(effect: &EffectAst) -> Option<&crate::filter::ObjectFilter> {
    match effect {
        EffectAst::SourceSentence { effects } => {
            let [effect] = effects.as_slice() else {
                return None;
            };
            direct_destroy_filter(effect)
        }
        EffectAst::TagAffected { effect, .. } => direct_destroy_filter(effect),
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::DestroyAll { filter, .. } => Some(filter),
            SubjectVerbActionAst::Destroy {
                target: TargetAst::Object(filter, _, _),
                ..
            } => Some(filter),
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
            constraint.tag.as_str() == IT_TAG || constraint.tag.as_str() == CHOSEN_OBJECTS_TAG
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

        let durable_tag = crate::tag::TagKey::from(CHOSEN_OBJECTS_TAG);
        for producer in &mut effects[producer_start..consumer_index] {
            retag_choice_collection_producer(producer, &durable_tag);
        }

        let Some(filter) = direct_destroy_filter_mut(&mut effects[consumer_index]) else {
            continue;
        };
        for constraint in &mut filter.tagged_constraints {
            if constraint.tag.as_str() == IT_TAG {
                constraint.tag = durable_tag.clone();
            }
        }
        if filter.other {
            filter.other = false;
            if !filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == durable_tag
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            }) {
                filter
                    .tagged_constraints
                    .push(crate::filter::TaggedObjectConstraint {
                        tag: durable_tag,
                        relation: crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
                    });
            }
        }
    }
}

fn direct_destroy_references_chosen_collection(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SourceSentence { effects } => {
            let [effect] = effects.as_slice() else {
                return false;
            };
            direct_destroy_references_chosen_collection(effect)
        }
        EffectAst::TagAffected { effect, .. } => {
            direct_destroy_references_chosen_collection(effect)
        }
        EffectAst::SubjectVerb(subject_verb)
            if matches!(&subject_verb.action, SubjectVerbActionAst::Destroy { .. }) =>
        {
            super::compile_support::effect_references_tag(effect, CHOSEN_OBJECTS_TAG)
        }
        _ => false,
    }
}

/// A plural chosen-set continuation can be authored as a new sentence after
/// a leading conditional. If that action consumes the reserved aggregate
/// choice tag, it is both a semantic continuation of the quantified choice
/// and a no-op when the condition is false. Keep the producer and consumer in
/// one branch and give them the same durable tag before reference lowering.
pub(crate) fn correlate_conditional_quantified_choice_followups(
    effects: &mut Vec<EffectAst>,
) -> bool {
    let mut index = 0usize;
    let mut changed = false;
    while index + 1 < effects.len() {
        let follows_conditional_choice = {
            let (before, after) = effects.split_at_mut(index + 1);
            let EffectAst::Conditional {
                if_true, if_false, ..
            } = &mut before[index]
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
            let EffectAst::Conditional { if_true, .. } = &mut effects[index] else {
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
        EffectAst::ForEachPlayer { effects } => Some(effects),
        EffectAst::SourceSentence { effects } => {
            let [EffectAst::ForEachPlayer { effects }] = effects.as_mut_slice() else {
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
        let EffectAst::ChooseObjects { tag, .. } = effect else {
            return None;
        };
        if let Some(expected) = common.as_ref()
            && expected != tag
        {
            return None;
        }
        common = Some(tag.clone());
    }
    common
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
    if let Some(targets) = filter.targets_object.as_deref_mut() {
        replaced |= replace_correlated_filter_tag(targets, old, new);
    }
    if let Some(targets) = filter.targets_only_object.as_deref_mut() {
        replaced |= replace_correlated_filter_tag(targets, old, new);
    }
    if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
        replaced |= replace_correlated_filter_tag(attached_to, old, new);
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
        SubjectVerbActionAst::Sacrifice { filter, .. }
        | SubjectVerbActionAst::SacrificeAll { filter } => Some(filter),
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

        let durable_tag = if original_tag.as_str() == IT_TAG {
            crate::tag::TagKey::from("chosen_for_each_player")
        } else {
            original_tag.clone()
        };
        for effect in choice_effects {
            let EffectAst::ChooseObjects { filter, tag, .. } = effect else {
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
                constraint.tag == durable_tag
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            })
        {
            complement_filter
                .tagged_constraints
                .push(crate::filter::TaggedObjectConstraint {
                    tag: durable_tag,
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
        let SubjectVerbActionAst::Draw { count } = &draw.action else {
            continue;
        };
        let Some(filter) = count_filter(count).cloned() else {
            continue;
        };

        let EffectAst::SubjectVerb(grant) = &mut after[0] else {
            continue;
        };
        let SubjectVerbActionAst::GrantAbilitiesToTarget {
            target,
            set_quantifier_surface: Some(ironsmith_core::SetQuantifierSurface::Each),
            ..
        } = &mut grant.action
        else {
            continue;
        };
        let unresolved_set_reference = match target {
            TargetAst::Source(_) => true,
            TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
            _ => false,
        };
        if unresolved_set_reference {
            *target = TargetAst::Object(filter, None, None);
        }
    }
}

fn normalize_nested_effects(effect: &mut EffectAst) {
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            normalize_effects_vec(if_true);
            normalize_effects_vec(if_false);
        }
        EffectAst::TrailingIf { effects, .. }
        | EffectAst::TrailingUnless { effects, .. }
        | EffectAst::SourceSentence { effects }
        | EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::AnyPlayerMay { effects }
        | EffectAst::ResolvedIfResult { effects, .. }
        | EffectAst::ResolvedWhenResult { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::WhenResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::RepeatProcess { effects, .. }
        | EffectAst::RepeatEffects { effects, .. }
        | EffectAst::BidLife {
            winner_effects: effects,
            ..
        }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilNextUpkeep { effects, .. }
        | EffectAst::DelayedUntilNextDrawStep { effects, .. }
        | EffectAst::DelayedUntilNextMainPhase { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedTriggerForDuration { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectLeavesBattlefield { effects, .. }
        | EffectAst::VoteOption { effects, .. }
        | EffectAst::ManaRestricted { effects, .. } => normalize_effects_vec(effects),
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            normalize_effects_vec(effects);
            normalize_effects_vec(alternative);
        }
        // NOTE: this walker stays hand-rolled (rather than routing through
        // effect_ast_traversal's shared helper) because normalize_effects_vec
        // resizes/replaces the Vec (retain + whole-Vec rewrites), which the
        // slice-exposing helper cannot express. New wrapper variants must be
        // added here and kept in sync with the traversal macro.
        EffectAst::ChooseOneOf { modes } | EffectAst::VillainousChoice { modes, .. } => {
            for mode in modes {
                normalize_effects_vec(&mut mode.effects);
            }
        }
        EffectAst::IfEffectDidNotHappen { effect, otherwise } => {
            normalize_nested_effects(effect);
            normalize_effects_vec(otherwise);
        }
        EffectAst::TagAffected { effect, .. } => {
            normalize_nested_effects(effect);
        }
        _ => {}
    }
}

fn rewrite_repeat_process(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    if effects.len() < 2 {
        return None;
    }

    let last_index = effects.len() - 1;
    let EffectAst::IfResult {
        predicate,
        effects: tail_effects,
    } = &effects[last_index]
    else {
        return None;
    };
    if !matches!(tail_effects.last(), Some(EffectAst::RepeatThisProcess)) {
        return None;
    }

    let continue_effect_index = last_index.saturating_sub(1);
    let mut body = effects.to_vec();
    let EffectAst::IfResult { effects, .. } = &mut body[last_index] else {
        return None;
    };
    effects.pop();
    if effects.is_empty() {
        body.pop();
    }

    Some(vec![EffectAst::RepeatProcess {
        effects: body,
        continue_effect_index,
        continue_predicate: predicate.clone(),
    }])
}

fn rewrite_repeat_process_once(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    if effects.len() < 2 || !matches!(effects.last(), Some(EffectAst::RepeatThisProcessOnce)) {
        return None;
    }

    let body = effects[..effects.len() - 1].to_vec();
    let mut duplicated = body.clone();
    duplicated.extend(body);
    Some(duplicated)
}

fn rewrite_repeat_process_may(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    if effects.len() < 2 || !matches!(effects.last(), Some(EffectAst::RepeatThisProcessMay)) {
        return None;
    }

    Some(vec![EffectAst::RepeatProcess {
        effects: effects.to_vec(),
        continue_effect_index: effects.len() - 1,
        continue_predicate: crate::cards::builders::IfResultPredicate::Did,
    }])
}

fn rewrite_return_as_aura(effects: &[EffectAst]) -> Option<Vec<EffectAst>> {
    use crate::cards::builders::{IT_TAG, ReturnAsAuraAst, SubjectVerbActionAst, TargetAst};

    let mut rewritten = Vec::with_capacity(effects.len());
    let mut index = 0;
    let mut changed = false;
    while index < effects.len() {
        let Some(EffectAst::SubjectVerb(return_subject_verb)) = effects.get(index) else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        let SubjectVerbActionAst::ReturnToBattlefield { as_aura: None, .. } =
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
        let SubjectVerbActionAst::BecomeAuraEnchantment {
            target,
            attachment_filter,
            granted_abilities,
            ..
        } = &aura_subject_verb.action
        else {
            rewritten.push(effects[index].clone());
            index += 1;
            continue;
        };
        if !matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG) {
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
            && let SubjectVerbActionAst::ReturnToBattlefield { as_aura, .. } =
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
        SubjectVerbActionAst::RemoveAbilitiesAll {
            abilities,
            duration,
            ..
        } => abilities.is_empty() && matches!(duration, crate::effect::Until::Forever),
        SubjectVerbActionAst::RemoveAbilitiesFromTarget {
            target,
            abilities,
            duration,
        } => {
            abilities.is_empty()
                && matches!(duration, crate::effect::Until::Forever)
                && matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG)
        }
        _ => false,
    }
}

fn is_noop_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action:
                crate::cards::builders::SubjectVerbActionAst::GrantAbilitiesAll { abilities, .. }
                | crate::cards::builders::SubjectVerbActionAst::GrantAbilitiesChoiceAll {
                    abilities, ..
                },
            ..
        }) => abilities.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::{CHOSEN_OBJECTS_TAG, IT_TAG, IfResultPredicate};
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
        let effects = vec![EffectAst::May {
            effects: vec![
                EffectAst::subject_verb_grant_abilities_all(
                    ObjectFilter::default(),
                    Vec::new(),
                    Until::EndOfTurn,
                ),
                EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    crate::cards::builders::SubjectVerbActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                ),
            ],
        }];

        let normalized = normalize_effects_ast(&effects);
        let EffectAst::May { effects } = &normalized[0] else {
            panic!("expected wrapped may effect");
        };
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0],
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::Draw { .. },
                ..
            })
        ));
    }

    #[test]
    fn normalize_correlates_conditional_quantified_choice_with_chosen_set_destroy() {
        let choose = EffectAst::ChooseObjects {
            filter: ObjectFilter::permanent().controlled_by(PlayerFilter::IteratedPlayer),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        };
        let mut chosen_permanents = ObjectFilter::permanent();
        chosen_permanents
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(CHOSEN_OBJECTS_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        let effects = vec![
            EffectAst::Conditional {
                predicate: PredicateAst::ThisSpellWasCastFromZone(Zone::Exile),
                if_true: vec![EffectAst::ForEachOpponent {
                    effects: vec![choose],
                }],
                if_false: Vec::new(),
            },
            EffectAst::subject_verb_destroy(TargetAst::Object(chosen_permanents, None, None)),
        ];

        let normalized = normalize_effects_ast(&effects);
        let [
            EffectAst::Conditional {
                if_true, if_false, ..
            },
        ] = normalized.as_slice()
        else {
            panic!("expected one correlated conditional: {normalized:#?}");
        };
        assert!(if_false.is_empty());
        let [EffectAst::ForEachOpponent { effects }, destroy] = if_true.as_slice() else {
            panic!("expected choice and destroy in the true branch: {if_true:#?}");
        };
        let [EffectAst::ChooseObjects { tag, .. }] = effects.as_slice() else {
            panic!("expected one quantified object choice: {effects:#?}");
        };
        assert_eq!(tag.as_str(), CHOSEN_OBJECTS_TAG);
        assert!(super::direct_destroy_references_chosen_collection(destroy));
    }

    #[test]
    fn normalize_binds_repeated_choices_to_destroy_complement() {
        let choose = EffectAst::ChooseObjects {
            filter: ObjectFilter::creature(),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        };
        let mut complement = ObjectFilter::creature();
        complement.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
        let normalized = normalize_effects_ast(&[
            EffectAst::RepeatEffects {
                count: Value::DistinctPowers(ObjectFilter::creature()),
                effects: vec![choose],
            },
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [EffectAst::RepeatEffects { effects, .. }, destroy] = normalized.as_slice() else {
            panic!("expected repeated choice followed by destroy: {normalized:#?}");
        };
        let [EffectAst::ChooseObjects { tag, .. }] = effects.as_slice() else {
            panic!("expected repeated object choice: {effects:#?}");
        };
        assert_eq!(tag.as_str(), CHOSEN_OBJECTS_TAG);
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == CHOSEN_OBJECTS_TAG
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn normalize_unions_direct_and_per_player_choices_before_destroying_others() {
        let choice = || EffectAst::ChooseObjects {
            filter: ObjectFilter::permanent(),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        };
        let mut complement = ObjectFilter::permanent();
        complement.other = true;
        let normalized = normalize_effects_ast(&[
            choice(),
            EffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::NotYou,
                effects: vec![choice()],
            },
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [
            EffectAst::ChooseObjects { tag: direct, .. },
            quantified,
            destroy,
        ] = normalized.as_slice()
        else {
            panic!("expected two choice producers and a destroy: {normalized:#?}");
        };
        let EffectAst::ForEachPlayersFiltered { effects, .. } = quantified else {
            panic!("expected quantified choice: {quantified:#?}");
        };
        let [EffectAst::ChooseObjects { tag: repeated, .. }] = effects.as_slice() else {
            panic!("expected quantified object choice: {effects:#?}");
        };
        assert_eq!(direct.as_str(), CHOSEN_OBJECTS_TAG);
        assert_eq!(repeated.as_str(), CHOSEN_OBJECTS_TAG);
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(!filter.other);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == CHOSEN_OBJECTS_TAG
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    #[test]
    fn normalize_does_not_bind_an_unrelated_destroy_after_a_repeated_choice() {
        let mut unrelated_complement = ObjectFilter::artifact();
        unrelated_complement.other = true;
        let normalized = normalize_effects_ast(&[
            EffectAst::RepeatEffects {
                count: Value::Fixed(2),
                effects: vec![EffectAst::ChooseObjects {
                    filter: ObjectFilter::creature(),
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: TagKey::from(IT_TAG),
                }],
            },
            EffectAst::subject_verb_destroy_all(unrelated_complement),
        ]);

        let [EffectAst::RepeatEffects { effects, .. }, destroy] = normalized.as_slice() else {
            panic!("expected unchanged repeated choice: {normalized:#?}");
        };
        let [EffectAst::ChooseObjects { tag, .. }] = effects.as_slice() else {
            panic!("expected object choice: {effects:#?}");
        };
        assert_eq!(tag.as_str(), IT_TAG);
        let filter = super::direct_destroy_filter(destroy).expect("destroy filter");
        assert!(filter.other);
        assert!(filter.tagged_constraints.is_empty());
    }

    #[test]
    fn normalize_preserves_custom_choice_collection_tags() {
        let custom_tag = TagKey::from("custom_choice_collection");
        let mut complement = ObjectFilter::creature();
        complement.other = true;
        let normalized = normalize_effects_ast(&[
            EffectAst::RepeatEffects {
                count: Value::Fixed(2),
                effects: vec![EffectAst::ChooseObjects {
                    filter: ObjectFilter::creature(),
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: custom_tag.clone(),
                }],
            },
            EffectAst::subject_verb_destroy_all(complement),
        ]);

        let [EffectAst::RepeatEffects { effects, .. }, destroy] = normalized.as_slice() else {
            panic!("expected unchanged repeated choice: {normalized:#?}");
        };
        let [EffectAst::ChooseObjects { tag, .. }] = effects.as_slice() else {
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
            SubjectVerbActionAst::Draw {
                count: Value::Count(counted.clone()),
            },
        );
        let mut grant = EffectAst::subject_verb_grant_abilities_to_target(
            TargetAst::Source(None),
            Vec::new(),
            Until::EndOfTurn,
        );
        let EffectAst::SubjectVerb(grant_subject) = &mut grant else {
            panic!("expected targeted grant");
        };
        let SubjectVerbActionAst::GrantAbilitiesToTarget {
            set_quantifier_surface,
            ..
        } = &mut grant_subject.action
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
                    SubjectVerbActionAst::GrantAbilitiesToTarget {
                        target: TargetAst::Object(filter, _, _),
                        ..
                    } if filter == &counted
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
                TagKey::from("looked"),
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::ValueComparison {
                    left: Value::X,
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                },
                if_true: Vec::new(),
                if_false: Vec::new(),
            },
        ];

        let normalized = normalize_effects_ast(&effects);
        let EffectAst::Conditional {
            predicate: PredicateAst::ValueComparison { left, .. },
            ..
        } = &normalized[1]
        else {
            panic!("expected typed value comparison");
        };
        assert_eq!(*left, Value::CardsInHand(PlayerFilter::You));
    }

    #[test]
    fn normalize_rewrites_repeat_this_process_tail_into_loop_effect() {
        let effects = vec![
            EffectAst::May {
                effects: vec![EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    crate::cards::builders::SubjectVerbActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                )],
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![
                    EffectAst::subject_verb(
                        crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        crate::cards::builders::SubjectVerbActionAst::GainLife {
                            amount: Value::Fixed(1),
                        },
                    ),
                    EffectAst::RepeatThisProcess,
                ],
            },
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::RepeatProcess {
                continue_effect_index: 0,
                continue_predicate: IfResultPredicate::Did,
                ..
            }]
        ));
    }

    #[test]
    fn normalize_removes_empty_clash_result_marker_from_repeat_body() {
        let effects = vec![
            EffectAst::subject_verb_clash(crate::cards::builders::ClashOpponentAst::Opponent),
            EffectAst::IfResult {
                predicate: IfResultPredicate::WonClash,
                effects: vec![EffectAst::RepeatThisProcess],
            },
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::RepeatProcess {
                effects,
                continue_effect_index: 0,
                continue_predicate: IfResultPredicate::WonClash,
            }] if effects.len() == 1
        ));
    }

    #[test]
    fn normalize_rewrites_optional_repeat_this_process_tail_into_loop_effect() {
        let effects = vec![
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::Draw {
                    count: Value::Fixed(1),
                },
            ),
            EffectAst::subject_verb(
                crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                crate::cards::builders::SubjectVerbActionAst::LoseLife {
                    amount: Value::Fixed(1),
                },
            ),
            EffectAst::RepeatThisProcessMay,
        ];

        let normalized = normalize_effects_ast(&effects);
        assert!(matches!(
            normalized.as_slice(),
            [EffectAst::RepeatProcess {
                continue_effect_index: 2,
                continue_predicate: IfResultPredicate::Did,
                ..
            }]
        ));
    }
}

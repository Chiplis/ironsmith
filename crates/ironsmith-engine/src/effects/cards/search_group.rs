//! CR 701.23h groups search instructions that share a concluding shuffle.
//! A group belongs to one resolution and one searcher/library pair.
use crate::effect::Effect;
use crate::effects::{
    ChooseObjectsEffect, SearchLibraryEffect, SearchLibrarySlotsEffect, ShuffleLibraryEffect,
};
use crate::ids::{ObjectId, PlayerId};
use std::collections::{HashMap, HashSet};

/// A search for all cards sharing a name with each member of a set, followed
/// by one shuffle, searches once for the union of those names (CR 701.23h).
pub(crate) fn grouped_name_search(
    each: &crate::effects::ForEachTaggedEffect,
    game: &crate::game_state::GameState,
    ctx: &mut crate::effects::ExecutionContext,
) -> Option<crate::effects::SequenceEffect> {
    use crate::effects::{
        ConditionalEffect, ForEachTaggedEffect, MoveToZoneEffect, SequenceEffect,
    };
    use crate::filter::{ObjectFilter, ObjectFilterExt, TaggedOpbjectRelation};
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::zone::Zone;

    let [guard] = each.effects.as_slice() else {
        return None;
    };
    let guard = guard.downcast_ref::<ConditionalEffect>()?;
    let crate::effect::Condition::TaggedObjectMatchedLastKnown(iterand, filter) = &guard.condition
    else {
        return None;
    };
    let [body] = guard.if_true.as_slice() else {
        return None;
    };
    let body = body.downcast_ref::<SequenceEffect>()?;
    let [choice, destinations] = body.effects.as_slice() else {
        return None;
    };
    let search = choice.downcast_ref::<ChooseObjectsEffect>()?;
    let [name] = search.filter.tagged_constraints.as_slice() else {
        return None;
    };
    if iterand.as_str() != "__it__"
        || !guard.if_false.is_empty()
        || guard.capture_condition_result
        || each.controller_at_last_blocked_by.is_some()
        || !search.is_search
        || !matches!(
            search.search_mode,
            crate::effect::SearchSelectionMode::AllMatching
                | crate::effect::SearchSelectionMode::Optional
        )
        || search.count != crate::effect::ChoiceCount::any_number()
        || (search.zone != Some(Zone::Library) && !search.additional_zones.contains(&Zone::Library))
        || search.count_value.is_some()
        || search.aggregate_constraint.is_some()
        || search.chooser != PlayerFilter::You
        || !matches!(
            search.filter.owner,
            Some(PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_) | PlayerFilter::You)
        )
        || name.tag != *iterand
        || name.relation != TaggedOpbjectRelation::SameNameAsTagged
        || !ctx.library_search_groups.has_concluding_shuffle(choice)
    {
        return None;
    }
    // Other predicates could depend on the current iteration. Only the name
    // set varies in this instruction.
    let mut guard_filter = filter.clone();
    guard_filter.set_explicit_card_noun(false);
    let mut search_filter = search.filter.clone();
    search_filter.owner = None;
    search_filter.zone = None;
    search_filter.tagged_constraints.clear();
    search_filter.set_same_name_antecedent_surface(None);
    if guard_filter != ObjectFilter::default() || search_filter != ObjectFilter::default() {
        return None;
    }
    let movement = if let Some(each_found) = destinations.downcast_ref::<ForEachTaggedEffect>() {
        let [movement] = each_found.effects.as_slice() else {
            return None;
        };
        let movement = movement.downcast_ref::<MoveToZoneEffect>()?;
        if each_found.tag != search.tag
            || each_found.controller_at_last_blocked_by.is_some()
            || movement != &MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Exile, false)
        {
            return None;
        }
        let mut movement = movement.clone();
        movement.target = ChooseSpec::Tagged(search.tag.clone());
        Effect::new(movement)
    } else {
        let inner = destinations
            .downcast_ref::<crate::effects::TaggedEffect>()
            .map_or(destinations, |tagged| tagged.effect.as_ref());
        let exile = inner.downcast_ref::<crate::effects::ExileEffect>()?;
        if exile.spec != ChooseSpec::Tagged(search.tag.clone()) {
            return None;
        }
        destinations.clone()
    };
    let filter_ctx = ctx.filter_context(game);
    let names: Vec<_> = ctx
        .get_tagged_all(&each.tag)
        .into_iter()
        .flatten()
        .filter(|card| filter.matches_snapshot(card, &filter_ctx, game))
        .cloned()
        .collect();
    if names.is_empty() {
        return Some(SequenceEffect::new(Vec::new()));
    }
    let names_tag = crate::tag::TagKey::from(format!(
        "__pre_move_history__search_names__{}",
        each.tag.as_str()
    ));
    ctx.set_tagged_objects(names_tag.clone(), names);
    let mut search = search.clone();
    search.filter.tagged_constraints[0].tag = names_tag;
    Some(SequenceEffect::new(vec![Effect::new(search), movement]))
}

/// An anaphoric "that player shuffles" after a search of the target's
/// library still refers to the target when the object iteration has ended.
pub(crate) fn bind_target_library_shuffles<'a>(effects: impl Iterator<Item = &'a mut Effect>) {
    use crate::effects::{ForEachObject, MoveToZoneEffect, SequenceEffect, TaggedEffect};
    use crate::target::{ChooseSpec, PlayerFilter};
    let mut prior: Option<(crate::tag::TagKey, PlayerFilter)> = None;
    for effect in effects {
        if let Some((tag, owner)) = prior.take()
            && let Some(tail) = effect.downcast_ref::<SequenceEffect>()
            && let [put, shuffle] = tail.effects.as_slice()
            && let Some(shuffle) = shuffle.downcast_ref::<ShuffleLibraryEffect>()
            && shuffle.player == PlayerFilter::IteratedPlayer
        {
            let put = put
                .downcast_ref::<TaggedEffect>()
                .map_or(put, |tagged| tagged.effect.as_ref());
            if put
                .downcast_ref::<MoveToZoneEffect>()
                .is_some_and(|movement| {
                    movement.target == ChooseSpec::Tagged(tag)
                        && movement.zone == crate::zone::Zone::Battlefield
                })
            {
                let mut shuffle = shuffle.clone();
                shuffle.player = owner;
                let mut tail = tail.clone();
                tail.effects[1] = Effect::new(shuffle);
                *effect = Effect::new(tail);
            }
        }
        if let Some(each) = effect.downcast_ref::<ForEachObject>()
            && let Some(PlayerFilter::Target(controller) | PlayerFilter::AliasedTarget(controller)) =
                &each.filter.controller
            && let [choice] = each.effects.as_slice()
            && let Some(search) = choice.downcast_ref::<ChooseObjectsEffect>()
            && search.is_search
            && search.zone == Some(crate::zone::Zone::Library)
            && let Some(
                owner @ (PlayerFilter::AliasedTarget(target) | PlayerFilter::Target(target)),
            ) = &search.filter.owner
            && target == controller
        {
            prior = Some((search.tag.clone(), owner.clone()));
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct LibrarySearchGroups {
    searches_with_shuffle: HashSet<usize>,
    instructions: HashMap<usize, usize>,
    views: HashMap<(usize, PlayerId, PlayerId), HashSet<ObjectId>>,
}

impl LibrarySearchGroups {
    pub(crate) fn for_program(program: &crate::resolution::ResolutionProgram) -> Self {
        let mut groups = Self::default();
        let mut pending = Vec::new();
        for segment in &program.segments {
            for branch in &segment.self_replacements {
                let mut branch_pending = pending.clone();
                for effect in &branch.replacement_effects {
                    groups.visit(effect, &mut branch_pending);
                }
            }
            for effect in &segment.default_effects {
                groups.visit(effect, &mut pending);
            }
        }
        groups
    }

    fn visit(&mut self, effect: &Effect, pending: &mut Vec<usize>) {
        let implicit_shuffle = effect.downcast_ref::<SearchLibraryEffect>().is_some()
            || effect.downcast_ref::<SearchLibrarySlotsEffect>().is_some();
        if implicit_shuffle
            || effect
                .downcast_ref::<ChooseObjectsEffect>()
                .is_some_and(|item| item.is_search)
        {
            pending.push(std::sync::Arc::as_ptr(&effect.0) as *const () as usize);
        }
        if implicit_shuffle || effect.downcast_ref::<ShuffleLibraryEffect>().is_some() {
            self.searches_with_shuffle.extend(pending.iter().copied());
            if pending.len() > 1 {
                let group = pending[0];
                for instruction in pending.drain(..) {
                    self.instructions.insert(instruction, group);
                }
            } else {
                pending.clear();
            }
        } else {
            effect
                .0
                .visit_resolution_child_effects(&mut |child| self.visit(child, pending));
        }
    }

    pub(crate) fn has_concluding_shuffle(&self, search: &Effect) -> bool {
        let id = std::sync::Arc::as_ptr(&search.0) as *const () as usize;
        self.searches_with_shuffle.contains(&id)
    }

    /// Later selections see the original search space, even after found cards move.
    pub(crate) fn begin(
        &mut self,
        instruction: Option<usize>,
        chooser: PlayerId,
        owner: PlayerId,
        cards: &mut Vec<ObjectId>,
        restrict: impl FnOnce(&mut Vec<ObjectId>),
    ) -> bool {
        let Some(group) = instruction.and_then(|id| self.instructions.get(&id)) else {
            restrict(cards);
            return true;
        };
        match self.views.entry((*group, chooser, owner)) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                restrict(cards);
                entry.insert(cards.iter().copied().collect());
                true
            }
            std::collections::hash_map::Entry::Occupied(entry) => {
                cards.retain(|id| entry.get().contains(id));
                false
            }
        }
    }

    pub(crate) fn shuffled(&mut self, owner: PlayerId) {
        self.views.retain(|(_, _, library), _| *library != owner);
    }
}

//! CR 701.23h groups search instructions that share a concluding shuffle.
//! A group belongs to one resolution and one searcher/library pair.
use crate::effect::Effect;
use crate::effects::{
    ChooseObjectsEffect, SearchLibraryEffect, SearchLibrarySlotsEffect, ShuffleLibraryEffect,
};
use crate::ids::{ObjectId, PlayerId};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Default)]
pub(crate) struct LibrarySearchGroups {
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

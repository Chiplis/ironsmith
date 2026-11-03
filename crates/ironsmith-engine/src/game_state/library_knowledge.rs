//! Presentation knowledge about exact library incarnations. A random placement
//! grants one interval to the entire group, never the permutation inside it.
use super::*;

#[derive(Clone, Debug)]
pub(super) struct LibraryGroup {
    members: im::HashSet<ObjectId>,
    unordered: bool,
}

impl GameState {
    pub fn remember_library_group(&mut self, viewer: PlayerId, owner: PlayerId, cards: &[ObjectId], unordered: bool) {
        if cards.is_empty() { return; }
        let members: im::HashSet<_> = cards.iter().copied().collect();
        let groups = self.metadata.library_knowledge.entry((viewer, owner)).or_default();
        for group in groups.iter_mut() {
            group.members.retain(|id| !members.contains(id));
        }
        groups.retain(|group| !group.members.is_empty());
        groups.push_back(LibraryGroup { members, unordered });
    }

    pub(crate) fn forget_library_positions(&mut self, owner: PlayerId, cards: &[ObjectId]) {
        for ((_, subject), groups) in self.metadata.library_knowledge.iter_mut() {
            if *subject == owner {
                for group in groups.iter_mut() { group.members.retain(|id| !cards.contains(id)); }
                groups.retain(|group| !group.members.is_empty());
            }
        }
    }

    /// Zero-based inclusive bounds, from the top. No within-group ordering is exported.
    pub fn known_library_positions(&self, viewer: PlayerId, owner: PlayerId) -> HashMap<ObjectId, (usize, usize)> {
        let mut output = HashMap::new();
        let Some(groups) = self.metadata.library_knowledge.get(&(viewer, owner)) else { return output; };
        let Some(player) = self.player(owner) else { return output; };
        let slots: HashMap<_,_> = player.library.iter().rev().enumerate().map(|(i,id)| (*id,i)).collect();
        for group in groups {
            let mut members: Vec<_> = group.members.iter().filter_map(|id| Some((*id,*slots.get(id)?))).collect();
            if group.unordered {
                // Losing a card to someone else's hidden hand doesn't disclose which one left.
                if viewer != owner && members.len() != group.members.len() { continue; }
                let Some(first) = members.iter().map(|(_,i)| *i).min() else { continue; };
                let last = members.iter().map(|(_,i)| *i).max().unwrap_or(first);
                for (id,_) in members { output.insert(id,(first,last)); }
            } else {
                output.extend(members.drain(..).map(|(id,i)| (id,(i,i))));
            }
        }
        output
    }
}

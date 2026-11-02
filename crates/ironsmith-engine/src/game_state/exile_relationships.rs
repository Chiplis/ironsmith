//! Exact exile-to-battlefield associations, independent of source abilities.
use super::*;
impl GameState {
    pub(crate) fn link_exiled_card(&mut self, relationship: &str, card: ObjectId, host: ObjectId) -> bool {
        if !self.object(host).is_some_and(|object| object.zone == Zone::Battlefield)
            || self.is_phased_out(host)
            || !self.object(card).is_some_and(|object| object.zone == Zone::Exile && object.kind == crate::object::ObjectKind::Card)
        { return false; }
        let key=(relationship.trim().to_ascii_lowercase(),host);
        if key.0.is_empty() { return false; }
        let members=self.exile_tracking_mut().named_exile_relationships.entry(key).or_default();
        if members.contains(&card) { return false; }
        members.push(card);
        self.mark_continuous_state_dirty();
        true
    }
    pub(crate) fn related_exiled_cards(&self, relationship:&str, host:ObjectId)->Vec<ObjectId>{
        if !self.object(host).is_some_and(|object|object.zone==Zone::Battlefield){return Vec::new();}
        self.exile_tracking.named_exile_relationships.get(&(relationship.trim().to_ascii_lowercase(),host))
            .into_iter().flatten().copied().filter(|id|self.object(*id).is_some_and(|object|object.zone==Zone::Exile&&object.kind==crate::object::ObjectKind::Card)).collect()
    }
    pub(crate) fn related_exile_relationship_names(&self,host:ObjectId)->Vec<String>{
        self.exile_tracking.named_exile_relationships.keys().filter(|(_,anchor)|*anchor==host).map(|(name,_)|name.clone()).collect()
    }
    pub(crate) fn has_related_exiled_card(&self,relationship:&str,host:ObjectId,card:ObjectId)->bool{
        self.related_exiled_cards(relationship,host).contains(&card)
    }
}

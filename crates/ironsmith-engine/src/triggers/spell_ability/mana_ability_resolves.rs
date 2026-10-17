//! "Whenever a mana ability of this creature resolves" trigger (Tyvar the
//! Bellicose). Mana abilities don't use the stack (CR 605.3b); they resolve
//! when they add mana, so the trigger watches mana a mana ability of the
//! source produced. The produced amount is the event value.

use crate::events::mana::ManaProductionProvenance;
use crate::events::{EventKind, ManaAddedEvent};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SourceManaAbilityResolvesTrigger;

impl SourceManaAbilityResolvesTrigger {
    pub fn new() -> Self {
        Self
    }

    fn mana_ability_event<'a>(event: &'a TriggerEvent) -> Option<&'a ManaAddedEvent> {
        if event.kind() != EventKind::ManaAdded {
            return None;
        }
        let added = event.downcast::<ManaAddedEvent>()?;
        (!added.mana.is_empty()
            && matches!(
                added.provenance,
                ManaProductionProvenance::TappedSourceForMana
                    | ManaProductionProvenance::ManaAbility
            ))
        .then_some(added)
    }
}

impl TriggerMatcher for SourceManaAbilityResolvesTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        Self::mana_ability_event(event).is_some_and(|added| added.source == ctx.source_id)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::ManaAdded])
    }

    fn event_value_amount(&self, event: &TriggerEvent, _ctx: &TriggerContext) -> Option<i32> {
        Self::mana_ability_event(event).map(|added| added.mana.len() as i32)
    }

    fn display(&self) -> String {
        "Whenever a mana ability of this creature resolves".to_string()
    }
}

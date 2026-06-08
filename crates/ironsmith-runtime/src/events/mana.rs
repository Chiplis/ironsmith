//! Mana-added event implementation.

use std::any::Any;

use crate::events::raw_event::RawEvent;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher, ReplacementPriority};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::snapshot::ObjectSnapshot;
use crate::target::ObjectFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ManaProductionProvenance {
    #[default]
    Unknown,
    TappedSourceForMana,
}

/// Mana was added to a player's mana pool.
#[derive(Debug, Clone)]
pub struct ManaAddedEvent {
    /// Object whose ability or effect added the mana.
    pub source: ObjectId,
    /// Controller of the source ability or effect.
    pub controller: PlayerId,
    /// Player who received the mana.
    pub player: PlayerId,
    /// Mana symbols added by this event.
    pub mana: Vec<ManaSymbol>,
    /// Last-known snapshot of the source when the mana was added.
    pub snapshot: Option<ObjectSnapshot>,
    /// How this mana was produced, when relevant to replacement effects.
    pub provenance: ManaProductionProvenance,
}

impl ManaAddedEvent {
    pub fn new(
        source: ObjectId,
        controller: PlayerId,
        player: PlayerId,
        mana: Vec<ManaSymbol>,
    ) -> Self {
        Self {
            source,
            controller,
            player,
            mana,
            snapshot: None,
            provenance: ManaProductionProvenance::Unknown,
        }
    }

    pub fn with_snapshot(mut self, snapshot: Option<ObjectSnapshot>) -> Self {
        self.snapshot = snapshot;
        self
    }

    pub fn with_production_provenance(mut self, provenance: ManaProductionProvenance) -> Self {
        self.provenance = provenance;
        self
    }

    pub fn with_mana(mut self, mana: Vec<ManaSymbol>) -> Self {
        self.mana = mana;
        self
    }

    pub fn into_trigger_event(self) -> RawEvent {
        RawEvent::new_with_provenance(self, crate::provenance::ProvNodeId::default())
    }

    pub fn trigger_event(
        source: ObjectId,
        controller: PlayerId,
        player: PlayerId,
        mana: Vec<ManaSymbol>,
    ) -> RawEvent {
        Self::new(source, controller, player, mana).into_trigger_event()
    }
}

pub(crate) fn apply_mana_replacements(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    player: PlayerId,
    mana: Vec<ManaSymbol>,
    production_provenance: ManaProductionProvenance,
    snapshot: Option<ObjectSnapshot>,
    decision_maker: &mut (impl crate::decision::DecisionMaker + ?Sized),
) -> Vec<ManaSymbol> {
    if mana.is_empty() {
        return mana;
    }

    let event = crate::events::Event::new_with_provenance(
        ManaAddedEvent::new(source, controller, player, mana.clone())
            .with_production_provenance(production_provenance)
            .with_snapshot(snapshot),
        crate::provenance::ProvNodeId::default(),
    );
    let applied_effects = std::collections::HashSet::new();
    let applied_effect_keys = std::collections::HashSet::new();
    match crate::events::processing::process_trait_event_with_dm_and_applied_effects(
        game,
        event,
        decision_maker,
        &applied_effects,
        &applied_effect_keys,
    )
    .into_event()
    {
        Some(event) => crate::events::downcast_event::<ManaAddedEvent>(event.inner())
            .map(|event| event.mana.clone())
            .unwrap_or(mana),
        None => Vec::new(),
    }
}

pub mod matchers {
    use super::*;
    use crate::events::context::EventContext;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ManaProducedBySourceMatcher {
        source_filter: ObjectFilter,
        required_provenance: Option<ManaProductionProvenance>,
    }

    impl ManaProducedBySourceMatcher {
        pub fn new(source_filter: ObjectFilter) -> Self {
            Self {
                source_filter,
                required_provenance: None,
            }
        }

        pub fn tapped_source_for_mana(source_filter: ObjectFilter) -> Self {
            Self {
                source_filter,
                required_provenance: Some(ManaProductionProvenance::TappedSourceForMana),
            }
        }
    }

    impl ReplacementMatcher for ManaProducedBySourceMatcher {
        fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
            if event.event_kind() != EventKind::ManaAdded {
                return false;
            }
            let Some(mana_event) = event.as_any().downcast_ref::<ManaAddedEvent>() else {
                return false;
            };
            if mana_event.mana.is_empty() {
                return false;
            }
            if let Some(required) = self.required_provenance
                && mana_event.provenance != required
            {
                return false;
            }

            if let Some(object) = ctx.game.object(mana_event.source) {
                return self
                    .source_filter
                    .matches(object, &ctx.filter_ctx, ctx.game);
            }
            if let Some(snapshot) = mana_event.snapshot.as_ref() {
                return self
                    .source_filter
                    .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game);
            }
            false
        }

        fn priority(&self) -> ReplacementPriority {
            ReplacementPriority::Other
        }

        fn display(&self) -> String {
            format!("If {} would produce mana", self.source_filter.description())
        }
    }
}

impl GameEventType for ManaAddedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::ManaAdded
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }

    fn display(&self) -> String {
        "Mana added".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

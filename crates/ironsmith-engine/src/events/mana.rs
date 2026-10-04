//! Mana-added event implementation.

use std::any::Any;

use crate::events::raw_event::RawEvent;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher, ReplacementPriority};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::snapshot::ObjectSnapshot;
use crate::target::ObjectFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature="serialization",derive(serde::Serialize,serde::Deserialize))]
#[cfg_attr(feature="serialization",serde(deny_unknown_fields))]
pub enum ManaProductionProvenance {
    #[default]
    Unknown,
    TappedSourceForMana,
}

/// A pure rewrite of one pending production event. Applying this instruction
/// does not credit a pool or emit a trigger; the replacement procedure owns
/// ordering, optional choices and the once-per-event application history.
#[derive(Debug, Clone, Copy)]
pub enum ManaTransformation<'a> {
    ReplaceTypes(&'a [ManaSymbol]),
    ReplaceExact(&'a [ManaSymbol]),
    Multiply(u32),
    Rewrite { input: ironsmith_core::ManaRewriteInput, symbol: ManaSymbol,
        quantity: ironsmith_core::ManaRewriteQuantity },
}

impl ManaTransformation<'_> {
    pub fn apply(self, original: &[ManaSymbol]) -> Vec<ManaSymbol> {
        match self {
            Self::ReplaceTypes([symbol]) => vec![*symbol; original.len()],
            Self::ReplaceTypes(symbols) | Self::ReplaceExact(symbols) => symbols.to_vec(),
            Self::Rewrite { input, symbol, quantity } => {
                if !original.iter().any(|mana| input.matches(*mana)) { return original.to_vec(); }
                match quantity {
                    ironsmith_core::ManaRewriteQuantity::Preserve => original.iter()
                        .map(|mana| if input.matches(*mana) { symbol } else { *mana }).collect(),
                    ironsmith_core::ManaRewriteQuantity::Exact(amount) => {
                        let mut output: Vec<_> = original.iter().copied().filter(|mana| !input.matches(*mana)).collect();
                        output.extend(std::iter::repeat_n(symbol, amount as usize));
                        output
                    }
                }
            }
            Self::Multiply(factor) => {
                let mut output = Vec::with_capacity(original.len() * factor as usize);
                for _ in 0..factor { output.extend_from_slice(original); }
                output
            }
        }
    }
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

pub(crate) fn mana_rewrite_output_choices(output: ironsmith_core::ManaRewriteOutput,
    event: &ManaAddedEvent, game: &GameState) -> Vec<ManaSymbol> {
    match output {
        ironsmith_core::ManaRewriteOutput::Symbol(symbol) => vec![symbol],
        ironsmith_core::ManaRewriteOutput::ChooseColor => crate::color::Color::ALL.into_iter()
            .map(ManaSymbol::from_color).collect(),
        ironsmith_core::ManaRewriteOutput::ChosenColor => Vec::new(),
        ironsmith_core::ManaRewriteOutput::ByBasicLandType(mapping) => {
            let subtypes = game.object(event.source).filter(|_| !game.is_phased_out(event.source))
                .and_then(|_| game.current_subtypes(event.source))
                .or_else(|| event.snapshot.as_ref().map(|snapshot| snapshot.subtypes.clone())).unwrap_or_default();
            let mut colors = Vec::new();
            for (land_type, output) in [crate::types::Subtype::Plains, crate::types::Subtype::Island,
                crate::types::Subtype::Swamp, crate::types::Subtype::Mountain, crate::types::Subtype::Forest]
                .into_iter().zip(mapping) {
                if subtypes.contains(&land_type) && let Some(output) = output
                    && !colors.contains(&output) { colors.push(output); }
            }
            colors
        }
    }
}

/// Declarative predicate over one pending mana production. No replacement is
/// applied by matching; amounts and provenance are tested before each rewrite.
#[derive(Debug, Clone, Copy)]
pub struct ManaEventPredicate<'a> {
    pub source_filter: &'a ObjectFilter,
    pub required_provenance: Option<ManaProductionProvenance>,
    pub minimum_amount: usize,
    pub controller: Option<&'a crate::target::PlayerFilter>,
    pub input: ironsmith_core::ManaRewriteInput,
    pub land_type_outputs: Option<&'a [Option<ManaSymbol>; 5]>,
}

impl ManaEventPredicate<'_> {
    pub(crate) fn matches(self, event: &ManaAddedEvent, game: &GameState, filter_ctx: &crate::target::FilterContext) -> bool {
        if event.mana.is_empty() || event.mana.len() < self.minimum_amount
            || self.required_provenance.is_some_and(|required| event.provenance != required)
            || self.controller.is_some_and(|controller| !controller.matches_player(event.controller, filter_ctx))
            || !event.mana.iter().any(|symbol| self.input.matches(*symbol))
            || self.land_type_outputs.is_some_and(|mapping|
                mana_rewrite_output_choices(ironsmith_core::ManaRewriteOutput::ByBasicLandType(*mapping), event, game).is_empty()) {
            return false;
        }
        if let Some(object) = game.object(event.source).filter(|_| !game.is_phased_out(event.source)) {
            self.source_filter.matches(object, filter_ctx, game)
        } else if let Some(snapshot) = event.snapshot.as_ref() {
            self.source_filter.matches_snapshot(snapshot, filter_ctx, game)
        } else { false }
    }
}

pub const POOL_SYMBOLS: [ManaSymbol; 6] = [ManaSymbol::White, ManaSymbol::Blue,
    ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless];

/// The exact aggregate of existing units proposed for loss. A conversion ends
/// the loss proposal; a second "would lose" effect no longer matches it.
/// Unit metadata is held by the original transaction, never reconstructed from
/// these type counts. This event is not a ManaAdded event.
#[derive(Debug, Clone)]
pub struct ManaLostEvent {
    pub player: PlayerId,
    pub mana: crate::player::ManaPool,
    pub converted_to: Option<ManaSymbol>,
}
impl GameEventType for ManaLostEvent {
    fn event_kind(&self) -> EventKind { EventKind::ManaLost }
    fn affected_player(&self, _game: &GameState) -> PlayerId { self.player }
    fn player(&self) -> Option<PlayerId> { Some(self.player) }
    fn display(&self) -> String { "Unspent mana lost".into() }
    fn as_any(&self) -> &dyn Any { self }
}

pub mod matchers {
    use super::*;
    #[derive(Debug, Clone, PartialEq)]
    pub struct ManaLossMatcher { pub player: crate::target::PlayerFilter }
    impl ReplacementMatcher for ManaLossMatcher {
        fn may_match_event_kind(&self, kind: EventKind) -> bool { kind == EventKind::ManaLost }
        fn matches_prepared_event(&self, event: &dyn GameEventType, ctx: &crate::events::context::PreparedEventContext) -> bool {
            event.as_any().downcast_ref::<ManaLostEvent>().is_some_and(|loss|
                loss.converted_to.is_none() && POOL_SYMBOLS.iter().any(|symbol| loss.mana.amount(*symbol) > 0)
                && self.player.matches_player(loss.player, &ctx.filter_ctx))
        }
        fn display(&self) -> String { "If a player would lose unspent mana".into() }
    }

    use crate::events::context::EventContext;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ManaRewriteMatcher { pub rule: ironsmith_core::ManaOutputRewrite }
    impl ReplacementMatcher for ManaRewriteMatcher {
        fn may_match_event_kind(&self, kind: EventKind) -> bool { kind == EventKind::ManaAdded }
        fn mana_predicate(&self) -> Option<ManaEventPredicate<'_>> {
            Some(ManaEventPredicate {
                source_filter: &self.rule.source_filter,
                required_provenance: self.rule.tapped_for_mana.then_some(ManaProductionProvenance::TappedSourceForMana),
                minimum_amount: 1, controller: self.rule.controller.as_ref(), input: self.rule.input,
                land_type_outputs: match &self.rule.output { ironsmith_core::ManaRewriteOutput::ByBasicLandType(mapping) => Some(mapping), _ => None },
            })
        }
        fn matches_prepared_event(&self, event: &dyn GameEventType, ctx: &crate::events::context::PreparedEventContext) -> bool {
            event.as_any().downcast_ref::<ManaAddedEvent>().is_some_and(|mana|
                self.mana_predicate().is_some_and(|predicate| predicate.matches(mana, ctx.game, &ctx.filter_ctx)))
        }
        fn priority(&self) -> ReplacementPriority { ReplacementPriority::Other }
        fn display(&self) -> String { format!("Mana output rewrite: {:?}", self.rule) }
    }

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

        fn may_match_event_kind(&self, kind: EventKind) -> bool {
            kind == EventKind::ManaAdded
        }

        fn mana_predicate(&self) -> Option<ManaEventPredicate<'_>> {
            Some(ManaEventPredicate {
                source_filter: &self.source_filter,
                required_provenance: self.required_provenance,
                minimum_amount: 1,
                controller: None,
                input: ironsmith_core::ManaRewriteInput::Any,
                land_type_outputs: None,
            })
        }

        fn matches_prepared_event(&self, event: &dyn GameEventType, ctx: &crate::events::context::PreparedEventContext) -> bool {
            let Some(mana_event) = event.as_any().downcast_ref::<ManaAddedEvent>() else {
                return false;
            };
            self.mana_predicate().is_some_and(|predicate| predicate.matches(mana_event, ctx.game, &ctx.filter_ctx))
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

/// A concrete, provenance-preserving mana unit was spent on a transaction.
#[derive(Debug, Clone)]
pub struct ManaUnitSpentEvent {
    pub player: PlayerId,
    pub mana_source: ObjectId,
    pub payment_source: Option<ObjectId>,
    pub symbol: ManaSymbol,
    pub purpose: crate::ability::ManaPaymentPurpose,
    pub source_snapshot: Option<ObjectSnapshot>,
}

impl GameEventType for ManaUnitSpentEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::ManaSpent
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.mana_source)
    }

    fn object_id(&self) -> Option<ObjectId> {
        self.payment_source
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.source_snapshot.as_ref()
    }

    fn display(&self) -> String {
        "Mana unit spent".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

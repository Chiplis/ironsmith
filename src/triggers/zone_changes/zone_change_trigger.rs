//! Composable zone change trigger.
//!
//! This unified trigger expresses zone-change patterns with a single
//! composable type.
//!
//! # Examples
//!
//! ```ignore
//! // "Whenever a creature dies"
//! ZoneChangeTrigger::new()
//!     .from(Zone::Battlefield)
//!     .to(Zone::Graveyard)
//!     .filter(ObjectFilter::creature())
//!
//! // "Whenever you discard a card"
//! ZoneChangeTrigger::new()
//!     .from(Zone::Hand)
//!     .to(Zone::Graveyard)
//!     .player(PlayerRelation::You)
//!
//! // "Whenever a card is put into your graveyard from anywhere"
//! ZoneChangeTrigger::new()
//!     .to(Zone::Graveyard)
//!     .player(PlayerRelation::You)
//! ```

use crate::events::EventKind;
use crate::events::cause::CauseFilter;
use crate::events::zones::ZoneChangeEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::zone::Zone;

/// Pattern for matching zones in zone change events.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ZonePattern {
    /// Match any zone.
    #[default]
    Any,
    /// Match a specific zone.
    Specific(Zone),
    /// Match any of these zones.
    OneOf(Vec<Zone>),
    /// Match any zone except this one.
    AnyExcept(Zone),
}

impl ZonePattern {
    /// Check if a zone matches this pattern.
    pub fn matches(&self, zone: Zone) -> bool {
        match self {
            ZonePattern::Any => true,
            ZonePattern::Specific(z) => zone == *z,
            ZonePattern::OneOf(zones) => zones.contains(&zone),
            ZonePattern::AnyExcept(z) => zone != *z,
        }
    }
}

impl From<Zone> for ZonePattern {
    fn from(zone: Zone) -> Self {
        ZonePattern::Specific(zone)
    }
}

/// How the player relates to the zone change (owner/controller of the object).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PlayerRelation {
    /// Match any player's objects.
    #[default]
    Any,
    /// Match objects owned/controlled by the trigger's controller.
    You,
    /// Match objects owned/controlled by an opponent of the trigger's controller.
    Opponent,
}

/// How many times the trigger fires for batch events.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CountMode {
    /// Fire once per object ("Whenever a creature dies").
    #[default]
    Each,
    /// Fire once for the batch ("Whenever one or more creatures die").
    OneOrMore,
}

/// A composable trigger for zone change events.
///
/// This single type can express:
/// - "Whenever a creature dies" (battlefield -> graveyard, creature filter)
/// - "Whenever you discard a card" (hand -> graveyard, you)
/// - "Whenever a permanent enters the battlefield" (any -> battlefield)
/// - "Whenever a card is put into your graveyard" (any -> graveyard, you)
/// - And many more combinations
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneChangeTrigger {
    /// The zone the object is leaving.
    pub from: ZonePattern,
    /// The zone the object is entering.
    pub to: ZonePattern,
    /// Filter for matching objects.
    pub object_filter: ObjectFilter,
    /// Who owns/controls the object.
    pub player: PlayerRelation,
    /// Optional filter on what caused the zone change.
    pub cause_filter: Option<CauseFilter>,
    /// How many times to fire for batch events.
    pub count_mode: CountMode,
    /// If true, only trigger for the source object ("When ~ dies").
    pub this_object: bool,
}

impl Default for ZoneChangeTrigger {
    fn default() -> Self {
        Self {
            from: ZonePattern::Any,
            to: ZonePattern::Any,
            object_filter: ObjectFilter::default(),
            player: PlayerRelation::Any,
            cause_filter: None,
            count_mode: CountMode::Each,
            this_object: false,
        }
    }
}

impl ZoneChangeTrigger {
    /// Create a new zone change trigger with default settings (matches everything).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source zone pattern.
    pub fn from(mut self, zone: impl Into<ZonePattern>) -> Self {
        self.from = zone.into();
        self
    }

    /// Set the destination zone pattern.
    pub fn to(mut self, zone: impl Into<ZonePattern>) -> Self {
        self.to = zone.into();
        self
    }

    /// Set the object filter.
    pub fn filter(mut self, filter: ObjectFilter) -> Self {
        self.object_filter = filter;
        self
    }

    /// Set the player relation.
    pub fn player(mut self, player: PlayerRelation) -> Self {
        self.player = player;
        self
    }

    /// Set the cause filter.
    pub fn cause(mut self, cause: CauseFilter) -> Self {
        self.cause_filter = Some(cause);
        self
    }

    /// Set the count mode.
    pub fn count(mut self, mode: CountMode) -> Self {
        self.count_mode = mode;
        self
    }

    /// Make this trigger only match the source object ("When ~ dies").
    pub fn this(mut self) -> Self {
        self.this_object = true;
        self
    }

    // === Convenience constructors for common patterns ===

    /// "Whenever a [filter] dies" (battlefield -> graveyard)
    pub fn dies(filter: ObjectFilter) -> Self {
        Self::new()
            .from(Zone::Battlefield)
            .to(Zone::Graveyard)
            .filter(filter)
    }

    /// "When ~ dies"
    pub fn this_dies() -> Self {
        // "dies" is a creature-only game term; encode that in the trigger metadata.
        Self::dies(ObjectFilter::creature()).this()
    }

    /// "Whenever a [filter] enters the battlefield"
    pub fn enters_battlefield(filter: ObjectFilter) -> Self {
        Self::new().to(Zone::Battlefield).filter(filter)
    }

    /// "When ~ enters the battlefield"
    pub fn this_enters_battlefield() -> Self {
        Self::enters_battlefield(ObjectFilter::default()).this()
    }

    /// "Whenever a [filter] leaves the battlefield"
    pub fn leaves_battlefield(filter: ObjectFilter) -> Self {
        Self::new().from(Zone::Battlefield).filter(filter)
    }

    /// "When ~ leaves the battlefield"
    pub fn this_leaves_battlefield() -> Self {
        Self::leaves_battlefield(ObjectFilter::default()).this()
    }

    /// "Whenever you discard a card"
    pub fn you_discard() -> Self {
        Self::new()
            .from(Zone::Hand)
            .to(Zone::Graveyard)
            .player(PlayerRelation::You)
    }

    /// "Whenever a card is put into your graveyard from anywhere"
    pub fn card_enters_your_graveyard() -> Self {
        Self::new().to(Zone::Graveyard).player(PlayerRelation::You)
    }

    /// "Whenever a [filter] is exiled"
    pub fn exiled(filter: ObjectFilter) -> Self {
        Self::new().to(Zone::Exile).filter(filter)
    }

    /// Generate display text for this trigger.
    fn generate_display(&self) -> String {
        if self.this_object {
            let battlefield_subject = self.this_subject("permanent");
            let card_subject = self.this_subject("card");
            return match (&self.from, &self.to) {
                (
                    ZonePattern::Specific(Zone::Battlefield),
                    ZonePattern::Specific(Zone::Graveyard),
                ) => {
                    format!("When this {} dies", battlefield_subject)
                }
                (_, ZonePattern::Specific(Zone::Battlefield)) => {
                    format!("When this {} enters the battlefield", battlefield_subject)
                }
                (ZonePattern::Specific(Zone::Battlefield), _) => {
                    format!("When this {} leaves the battlefield", battlefield_subject)
                }
                (ZonePattern::Specific(Zone::Hand), ZonePattern::Specific(Zone::Graveyard)) => {
                    "When this card is discarded".to_string()
                }
                (_, ZonePattern::Specific(Zone::Graveyard)) => {
                    format!("When this {} is put into a graveyard", card_subject)
                }
                (_, ZonePattern::Specific(Zone::Exile)) => {
                    format!("When this {} is exiled", battlefield_subject)
                }
                _ => "When this object changes zones".to_string(),
            };
        }

        let mut parts = vec!["Whenever".to_string()];

        // Player relation
        match &self.player {
            PlayerRelation::You => parts.push("you".to_string()),
            PlayerRelation::Opponent => parts.push("an opponent".to_string()),
            PlayerRelation::Any => {}
        }

        // Count mode for batch
        if self.count_mode == CountMode::OneOrMore {
            parts.push("one or more".to_string());
        } else {
            parts.push("a".to_string());
        }

        // Object filter description
        let filter_desc = self.object_filter.description();
        if filter_desc != "object" {
            parts.push(filter_desc);
        } else {
            parts.push("card".to_string());
        }

        // Zone change description
        match (&self.from, &self.to) {
            (ZonePattern::Specific(Zone::Battlefield), ZonePattern::Specific(Zone::Graveyard)) => {
                parts.push("dies".to_string());
            }
            (ZonePattern::Specific(Zone::Hand), ZonePattern::Specific(Zone::Graveyard)) => {
                parts.push("is discarded".to_string());
            }
            (_, ZonePattern::Specific(Zone::Battlefield)) => {
                parts.push("enters the battlefield".to_string());
            }
            (ZonePattern::Specific(Zone::Battlefield), _) => {
                parts.push("leaves the battlefield".to_string());
            }
            (_, ZonePattern::Specific(Zone::Graveyard)) => {
                parts.push("is put into a graveyard".to_string());
            }
            (_, ZonePattern::Specific(Zone::Exile)) => {
                parts.push("is exiled".to_string());
            }
            _ => {
                parts.push("changes zones".to_string());
            }
        }

        parts.join(" ")
    }

    fn this_subject(&self, fallback: &'static str) -> &'static str {
        use crate::types::CardType;
        if self.object_filter.card_types.contains(&CardType::Creature) {
            return "creature";
        }
        if self.object_filter.card_types.len() == 1 {
            return match self.object_filter.card_types[0] {
                CardType::Land => "land",
                CardType::Artifact => "artifact",
                CardType::Enchantment => "enchantment",
                CardType::Planeswalker => "planeswalker",
                CardType::Battle => "battle",
                CardType::Kindred => "kindred",
                CardType::Instant | CardType::Sorcery => fallback,
                CardType::Creature => "creature",
            };
        }
        fallback
    }
}

fn snapshot_matches_filter(
    snapshot: &crate::snapshot::ObjectSnapshot,
    filter: &ObjectFilter,
    ctx: &TriggerContext,
) -> bool {
    if !filter.card_types.is_empty()
        && !filter
            .card_types
            .iter()
            .any(|t| snapshot.card_types.contains(t))
    {
        return false;
    }

    if !filter.subtypes.is_empty()
        && !filter
            .subtypes
            .iter()
            .any(|t| snapshot.subtypes.contains(t))
    {
        return false;
    }

    if let Some(ref required_colors) = filter.colors
        && snapshot.colors.intersection(*required_colors).is_empty()
    {
        return false;
    }

    if let Some(ref controller_filter) = filter.controller {
        use crate::target::PlayerFilter;
        let matches = match controller_filter {
            PlayerFilter::You => ctx.filter_ctx.you == Some(snapshot.controller),
            PlayerFilter::Opponent => ctx.filter_ctx.opponents.contains(&snapshot.controller),
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => *id == snapshot.controller,
            _ => true,
        };
        if !matches {
            return false;
        }
    }

    if let Some(ref comparison) = filter.power {
        let Some(power) = snapshot.power else {
            return false;
        };
        if !comparison.satisfies(power) {
            return false;
        }
    }

    if let Some(ref comparison) = filter.toughness {
        let Some(toughness) = snapshot.toughness else {
            return false;
        };
        if !comparison.satisfies(toughness) {
            return false;
        }
    }

    if let Some(ref required_name) = filter.name
        && snapshot.name != *required_name
    {
        return false;
    }

    if filter.other && ctx.filter_ctx.source == Some(snapshot.object_id) {
        return false;
    }

    true
}

impl TriggerMatcher for ZoneChangeTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        // Must be a zone change event
        if event.kind() != EventKind::ZoneChange {
            return false;
        }

        let Some(zc) = event.downcast::<ZoneChangeEvent>() else {
            return false;
        };

        // Check zone patterns
        if !self.from.matches(zc.from) {
            return false;
        }
        if !self.to.matches(zc.to) {
            return false;
        }

        // For "this object" triggers, check if any object is the source
        if self.this_object && !zc.objects.contains(&ctx.source_id) {
            return false;
        }

        // Check player relation using snapshot if available, otherwise use game state
        if self.player != PlayerRelation::Any {
            let player_matches = if let Some(ref snapshot) = zc.snapshot {
                match &self.player {
                    PlayerRelation::You => snapshot.controller == ctx.controller,
                    PlayerRelation::Opponent => snapshot.controller != ctx.controller,
                    PlayerRelation::Any => true,
                }
            } else {
                // Check the first object in game state
                zc.objects
                    .first()
                    .and_then(|&id| ctx.game.object(id))
                    .map(|obj| match &self.player {
                        PlayerRelation::You => obj.controller == ctx.controller,
                        PlayerRelation::Opponent => obj.controller != ctx.controller,
                        PlayerRelation::Any => true,
                    })
                    .unwrap_or(false)
            };

            if !player_matches {
                return false;
            }
        }

        // Check object filter using snapshot for LKI
        if let Some(ref snapshot) = zc.snapshot {
            if !snapshot_matches_filter(snapshot, &self.object_filter, ctx) {
                return false;
            }
        } else {
            // Check filter against live object (for ETB triggers, etc.)
            let filter_matches = zc
                .objects
                .first()
                .and_then(|&id| ctx.game.object(id))
                .map(|obj| self.object_filter.matches(obj, &ctx.filter_ctx, ctx.game))
                .unwrap_or(true); // If no object, allow (conservative)

            if !filter_matches {
                return false;
            }
        }

        // Check cause filter if specified
        if let Some(ref cause_filter) = self.cause_filter {
            let affected = zc
                .snapshot
                .as_ref()
                .map(|s| s.controller)
                .or_else(|| {
                    zc.objects
                        .first()
                        .and_then(|&id| ctx.game.object(id))
                        .map(|o| o.controller)
                })
                .unwrap_or(ctx.controller);

            if !cause_filter.matches(&zc.cause, ctx.game, affected) {
                return false;
            }
        }

        true
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        match self.count_mode {
            CountMode::OneOrMore => 1,
            CountMode::Each => {
                if let Some(zc) = event.downcast::<ZoneChangeEvent>() {
                    zc.count() as u32
                } else {
                    1
                }
            }
        }
    }

    fn uses_snapshot(&self) -> bool {
        // Zone change triggers often need LKI for the object's characteristics
        // at the moment it left its origin zone
        self.from != ZonePattern::Any
    }

    fn display(&self) -> String {
        self.generate_display()
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cause::EventCause;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature_snapshot(
        object_id: ObjectId,
        controller: PlayerId,
        name: &str,
    ) -> ObjectSnapshot {
        ObjectSnapshot::for_testing(object_id, controller, name)
            .with_card_types(vec![CardType::Creature])
            .with_pt(2, 2)
    }

    #[test]
    fn test_zone_pattern_matching() {
        assert!(ZonePattern::Any.matches(Zone::Battlefield));
        assert!(ZonePattern::Any.matches(Zone::Graveyard));

        assert!(ZonePattern::Specific(Zone::Battlefield).matches(Zone::Battlefield));
        assert!(!ZonePattern::Specific(Zone::Battlefield).matches(Zone::Graveyard));

        assert!(ZonePattern::OneOf(vec![Zone::Hand, Zone::Graveyard]).matches(Zone::Hand));
        assert!(!ZonePattern::OneOf(vec![Zone::Hand, Zone::Graveyard]).matches(Zone::Battlefield));

        assert!(ZonePattern::AnyExcept(Zone::Battlefield).matches(Zone::Graveyard));
        assert!(!ZonePattern::AnyExcept(Zone::Battlefield).matches(Zone::Battlefield));
    }

    #[test]
    fn test_dies_trigger() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let creature_id = ObjectId::from_raw(2);

        let trigger = ZoneChangeTrigger::dies(ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // Creature dying should match
        let event = TriggerEvent::new(ZoneChangeEvent::with_cause(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            EventCause::from_sba(),
            Some(make_creature_snapshot(creature_id, alice, "Bear")),
        ));
        assert!(trigger.matches(&event, &ctx));

        // Creature being exiled should not match
        let exile_event = TriggerEvent::new(ZoneChangeEvent::with_cause(
            creature_id,
            Zone::Battlefield,
            Zone::Exile,
            EventCause::from_sba(),
            Some(make_creature_snapshot(creature_id, alice, "Bear")),
        ));
        assert!(!trigger.matches(&exile_event, &ctx));
    }

    #[test]
    fn test_this_dies_trigger() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let other_id = ObjectId::from_raw(2);

        let trigger = ZoneChangeTrigger::this_dies();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // Source dying should match
        let event = TriggerEvent::new(ZoneChangeEvent::with_cause(
            source_id,
            Zone::Battlefield,
            Zone::Graveyard,
            EventCause::from_sba(),
            Some(make_creature_snapshot(source_id, alice, "Self")),
        ));
        assert!(trigger.matches(&event, &ctx));

        // Other creature dying should not match
        let other_event = TriggerEvent::new(ZoneChangeEvent::with_cause(
            other_id,
            Zone::Battlefield,
            Zone::Graveyard,
            EventCause::from_sba(),
            Some(make_creature_snapshot(other_id, alice, "Other")),
        ));
        assert!(!trigger.matches(&other_event, &ctx));
    }

    #[test]
    fn test_etb_trigger() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let creature_id = ObjectId::from_raw(2);

        let trigger = ZoneChangeTrigger::enters_battlefield(ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // ETB from hand
        let from_hand = TriggerEvent::new(ZoneChangeEvent::new(
            creature_id,
            Zone::Hand,
            Zone::Battlefield,
            Some(make_creature_snapshot(creature_id, alice, "Bear")),
        ));
        assert!(trigger.matches(&from_hand, &ctx));

        // ETB from graveyard (reanimate)
        let from_graveyard = TriggerEvent::new(ZoneChangeEvent::new(
            creature_id,
            Zone::Graveyard,
            Zone::Battlefield,
            Some(make_creature_snapshot(creature_id, alice, "Bear")),
        ));
        assert!(trigger.matches(&from_graveyard, &ctx));
    }

    #[test]
    fn test_you_discard_trigger() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let card_id = ObjectId::from_raw(2);

        let trigger = ZoneChangeTrigger::you_discard();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // Alice discarding should match
        let alice_discard = TriggerEvent::new(ZoneChangeEvent::with_cause(
            card_id,
            Zone::Hand,
            Zone::Graveyard,
            EventCause::from_effect(source_id, alice),
            Some(ObjectSnapshot::for_testing(card_id, alice, "Card")),
        ));
        assert!(trigger.matches(&alice_discard, &ctx));

        // Bob discarding should not match
        let bob_discard = TriggerEvent::new(ZoneChangeEvent::with_cause(
            card_id,
            Zone::Hand,
            Zone::Graveyard,
            EventCause::from_effect(source_id, bob),
            Some(ObjectSnapshot::for_testing(card_id, bob, "Card")),
        ));
        assert!(!trigger.matches(&bob_discard, &ctx));
    }

    #[test]
    fn test_batch_trigger_count() {
        let objects = vec![
            ObjectId::from_raw(1),
            ObjectId::from_raw(2),
            ObjectId::from_raw(3),
        ];
        let event = TriggerEvent::new(ZoneChangeEvent::batch(
            objects,
            Zone::Battlefield,
            Zone::Graveyard,
            EventCause::from_sba(),
        ));

        // "Whenever a creature dies" fires 3 times
        let each_trigger = ZoneChangeTrigger::dies(ObjectFilter::creature());
        assert_eq!(each_trigger.trigger_count(&event), 3);

        // "Whenever one or more creatures die" fires once
        let batch_trigger =
            ZoneChangeTrigger::dies(ObjectFilter::creature()).count(CountMode::OneOrMore);
        assert_eq!(batch_trigger.trigger_count(&event), 1);
    }

    #[test]
    fn test_display() {
        let dies = ZoneChangeTrigger::dies(ObjectFilter::creature());
        assert!(dies.display().contains("dies"));

        let etb = ZoneChangeTrigger::enters_battlefield(ObjectFilter::default());
        assert!(etb.display().contains("enters the battlefield"));

        let discard = ZoneChangeTrigger::you_discard();
        assert!(discard.display().contains("discard"));
    }
}

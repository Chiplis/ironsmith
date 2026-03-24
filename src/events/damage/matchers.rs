//! Damage replacement effect matchers.

use crate::events::context::EventContext;
use crate::events::traits::{
    EventKind, GameEventType, ReplacementMatcher, ReplacementPriority, downcast_event,
};
use crate::game_event::DamageTarget;
use crate::ids::{ObjectId, PlayerId};
use crate::target::{ObjectFilter, PlayerFilter};

use super::DamageEvent;

/// Matches damage events where the target is a player matching the filter.
#[derive(Debug, Clone)]
pub struct DamageToPlayerMatcher {
    pub player_filter: PlayerFilter,
}

impl DamageToPlayerMatcher {
    pub fn new(player_filter: PlayerFilter) -> Self {
        Self { player_filter }
    }

    /// Matches damage to "you" (the controller of the replacement effect).
    pub fn to_you() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Matches damage to any player.
    pub fn to_any_player() -> Self {
        Self::new(PlayerFilter::Any)
    }

    /// Matches damage to any opponent.
    pub fn to_opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

impl ReplacementMatcher for DamageToPlayerMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        match damage.target {
            DamageTarget::Player(player_id) => self
                .player_filter
                .matches_player(player_id, &ctx.filter_ctx),
            DamageTarget::Object(_) => false,
        }
    }

    fn display(&self) -> String {
        match &self.player_filter {
            PlayerFilter::You => "When damage would be dealt to you".to_string(),
            PlayerFilter::Any => "When damage would be dealt to any player".to_string(),
            PlayerFilter::Opponent => "When damage would be dealt to an opponent".to_string(),
            _ => "When damage would be dealt to a player".to_string(),
        }
    }
}

/// Matches damage events where the target is an object matching the filter.
#[derive(Debug, Clone)]
pub struct DamageToObjectMatcher {
    pub filter: ObjectFilter,
}

impl DamageToObjectMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Matches damage to any creature.
    pub fn to_creature() -> Self {
        Self::new(ObjectFilter::creature())
    }

    /// Matches damage to any permanent.
    pub fn to_permanent() -> Self {
        Self::new(ObjectFilter::permanent())
    }
}

impl ReplacementMatcher for DamageToObjectMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        match damage.target {
            DamageTarget::Object(object_id) => {
                if let Some(obj) = ctx.game.object(object_id) {
                    self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
                } else {
                    false
                }
            }
            DamageTarget::Player(_) => false,
        }
    }

    fn display(&self) -> String {
        "When damage would be dealt to a permanent".to_string()
    }
}

/// Matches damage events where the target is a player or object matching the filters.
#[derive(Debug, Clone)]
pub struct DamageToPlayerOrObjectMatcher {
    pub player_filter: PlayerFilter,
    pub object_filter: ObjectFilter,
}

impl DamageToPlayerOrObjectMatcher {
    pub fn new(player_filter: PlayerFilter, object_filter: ObjectFilter) -> Self {
        Self {
            player_filter,
            object_filter,
        }
    }
}

impl ReplacementMatcher for DamageToPlayerOrObjectMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        match damage.target {
            DamageTarget::Player(player_id) => self
                .player_filter
                .matches_player(player_id, &ctx.filter_ctx),
            DamageTarget::Object(object_id) => {
                if let Some(obj) = ctx.game.object(object_id) {
                    self.object_filter.matches(obj, &ctx.filter_ctx, ctx.game)
                } else {
                    false
                }
            }
        }
    }

    fn display(&self) -> String {
        "When damage would be dealt to a player or permanent".to_string()
    }
}

/// Matches combat damage events.
#[derive(Debug, Clone)]
pub struct CombatDamageMatcher;

impl ReplacementMatcher for CombatDamageMatcher {
    fn matches_event(&self, event: &dyn GameEventType, _ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        damage.is_combat
    }

    fn display(&self) -> String {
        "When combat damage would be dealt".to_string()
    }
}

/// Matches noncombat damage events.
#[derive(Debug, Clone)]
pub struct NoncombatDamageMatcher;

impl ReplacementMatcher for NoncombatDamageMatcher {
    fn matches_event(&self, event: &dyn GameEventType, _ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        !damage.is_combat
    }

    fn display(&self) -> String {
        "When noncombat damage would be dealt".to_string()
    }
}

/// Matches damage events from a source matching the filter.
#[derive(Debug, Clone)]
pub struct DamageFromSourceMatcher {
    pub filter: ObjectFilter,
}

impl DamageFromSourceMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Matches damage from any creature.
    pub fn from_creature() -> Self {
        Self::new(ObjectFilter::creature())
    }
}

impl ReplacementMatcher for DamageFromSourceMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        if let Some(obj) = ctx.game.object(damage.source) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        "When damage would be dealt by a source".to_string()
    }
}

/// Matches preventable damage events dealt by the source of the replacement effect.
///
/// Used for abilities like "Prevent all damage that would be dealt by this creature."
#[derive(Debug, Clone)]
pub struct DamageFromSelfMatcher;

impl DamageFromSelfMatcher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DamageFromSelfMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplacementMatcher for DamageFromSelfMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        // "Damage can't be prevented" bypasses prevention effects.
        if damage.is_unpreventable {
            return false;
        }

        ctx.source == Some(damage.source)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When damage would be dealt by this permanent".to_string()
    }
}

/// Constraint for matching damage sources.
#[derive(Debug, Clone)]
pub enum DamageSourceConstraint {
    /// Damage is dealt by a specific object.
    Specific(ObjectId),
    /// Damage is dealt by a source matching this filter.
    Filter(ObjectFilter),
}

/// Constraint for matching damage targets.
#[derive(Debug, Clone)]
pub enum DamageTargetConstraint {
    /// Any damage target.
    Any,
    /// Damage is dealt to a specific player.
    Player(PlayerId),
}

/// Matches preventable damage events with optional source/target constraints.
///
/// Intended for "prevent that damage" style replacement effects.
#[derive(Debug, Clone)]
pub struct PreventableDamageConstraintMatcher {
    pub source: DamageSourceConstraint,
    pub target: DamageTargetConstraint,
}

impl PreventableDamageConstraintMatcher {
    pub fn from_specific_source(source: ObjectId, target: DamageTargetConstraint) -> Self {
        Self {
            source: DamageSourceConstraint::Specific(source),
            target,
        }
    }

    pub fn from_filter(filter: ObjectFilter, target: DamageTargetConstraint) -> Self {
        Self {
            source: DamageSourceConstraint::Filter(filter),
            target,
        }
    }
}

impl ReplacementMatcher for PreventableDamageConstraintMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        // "Damage can't be prevented" bypasses prevention effects.
        if damage.is_unpreventable {
            return false;
        }

        // Source constraint.
        match &self.source {
            DamageSourceConstraint::Specific(id) => {
                if &damage.source != id {
                    return false;
                }
            }
            DamageSourceConstraint::Filter(filter) => {
                let Some(obj) = ctx.game.object(damage.source) else {
                    return false;
                };
                if !filter.matches(obj, &ctx.filter_ctx, ctx.game) {
                    return false;
                }
            }
        }

        // Target constraint.
        match &self.target {
            DamageTargetConstraint::Any => {}
            DamageTargetConstraint::Player(player_id) => match damage.target {
                DamageTarget::Player(pid) => {
                    if &pid != player_id {
                        return false;
                    }
                }
                DamageTarget::Object(_) => return false,
            },
        }

        true
    }

    fn display(&self) -> String {
        "When damage would be dealt (preventable)".to_string()
    }
}

/// Matches damage events to the source of the replacement effect.
#[derive(Debug, Clone)]
pub struct DamageToSelfMatcher;

impl DamageToSelfMatcher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DamageToSelfMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplacementMatcher for DamageToSelfMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        match damage.target {
            DamageTarget::Object(object_id) => ctx.source == Some(object_id),
            DamageTarget::Player(_) => false,
        }
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When damage would be dealt to this permanent".to_string()
    }
}

/// Matches preventable damage to the source of the replacement effect with optional constraints.
#[derive(Debug, Clone, Default)]
pub struct DamageToSelfConstraintMatcher {
    pub source_filter: Option<ObjectFilter>,
    pub combat_only: Option<bool>,
}

impl DamageToSelfConstraintMatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_source_filter(source_filter: ObjectFilter) -> Self {
        Self {
            source_filter: Some(source_filter),
            combat_only: None,
        }
    }

    pub fn noncombat() -> Self {
        Self {
            source_filter: None,
            combat_only: Some(false),
        }
    }

    pub fn combat_from_source_filter(source_filter: ObjectFilter) -> Self {
        Self {
            source_filter: Some(source_filter),
            combat_only: Some(true),
        }
    }
}

impl ReplacementMatcher for DamageToSelfConstraintMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        if damage.is_unpreventable {
            return false;
        }

        let DamageTarget::Object(target_id) = damage.target else {
            return false;
        };
        if ctx.source != Some(target_id) {
            return false;
        }

        if let Some(combat_only) = self.combat_only
            && damage.is_combat != combat_only
        {
            return false;
        }

        if let Some(source_filter) = &self.source_filter {
            let Some(source_obj) = ctx.game.object(damage.source) else {
                return false;
            };
            if !source_filter.matches(source_obj, &ctx.filter_ctx, ctx.game) {
                return false;
            }
        }

        true
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When constrained damage would be dealt to this permanent".to_string()
    }
}

/// Matches preventable combat damage events to the source of the replacement effect.
#[derive(Debug, Clone)]
pub struct DamageToSelfCombatMatcher;

impl DamageToSelfCombatMatcher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DamageToSelfCombatMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplacementMatcher for DamageToSelfCombatMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        if damage.is_unpreventable || !damage.is_combat {
            return false;
        }

        match damage.target {
            DamageTarget::Object(object_id) => ctx.source == Some(object_id),
            DamageTarget::Player(_) => false,
        }
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When combat damage would be dealt to this permanent".to_string()
    }
}

/// Matches damage events dealt to another creature you control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DamageToOtherCreatureYouControlMatcher;

impl ReplacementMatcher for DamageToOtherCreatureYouControlMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        if damage.is_unpreventable {
            return false;
        }

        let DamageTarget::Object(target_id) = damage.target else {
            return false;
        };
        if ctx.source == Some(target_id) {
            return false;
        }

        let Some(target_obj) = ctx.game.object(target_id) else {
            return false;
        };
        target_obj.controller == ctx.controller
            && target_obj
                .card_types
                .contains(&crate::types::CardType::Creature)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When damage would be dealt to another creature you control".to_string()
    }
}

/// Matches preventable damage events dealt to the source of the replacement effect
/// by sources that satisfy a filter.
#[derive(Debug, Clone)]
pub struct DamageToSelfFromSourceFilterMatcher {
    pub source_filter: ObjectFilter,
}

impl DamageToSelfFromSourceFilterMatcher {
    pub fn new(source_filter: ObjectFilter) -> Self {
        Self { source_filter }
    }

    pub fn from_creature() -> Self {
        Self::new(ObjectFilter::creature())
    }
}

impl ReplacementMatcher for DamageToSelfFromSourceFilterMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        if damage.is_unpreventable {
            return false;
        }

        let DamageTarget::Object(target_id) = damage.target else {
            return false;
        };
        if ctx.source != Some(target_id) {
            return false;
        }

        let Some(source_obj) = ctx.game.object(damage.source) else {
            return false;
        };

        self.source_filter
            .matches(source_obj, &ctx.filter_ctx, ctx.game)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When damage would be dealt to this permanent by a matching source".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn damage(source: ObjectId, target: DamageTarget, amount: u32, is_combat: bool) -> DamageEvent {
        let cause = if is_combat {
            crate::events::cause::EventCause::combat_damage(source)
        } else {
            crate::events::cause::EventCause::effect()
        };
        DamageEvent::with_cause(source, target, amount, is_combat, cause)
    }

    fn unpreventable_damage(
        source: ObjectId,
        target: DamageTarget,
        amount: u32,
        is_combat: bool,
    ) -> DamageEvent {
        let cause = if is_combat {
            crate::events::cause::EventCause::combat_damage(source)
        } else {
            crate::events::cause::EventCause::effect()
        };
        DamageEvent::unpreventable_with_cause(source, target, amount, is_combat, cause)
    }

    #[test]
    fn test_damage_to_player_matcher() {
        let game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let bob = crate::ids::PlayerId::from_index(1);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = DamageToPlayerMatcher::to_you();

        // Damage to Alice (the controller) should match
        let event_to_alice = damage(ObjectId::from_raw(1), DamageTarget::Player(alice), 3, false);
        assert!(matcher.matches_event(&event_to_alice, &ctx));

        // Damage to Bob should not match "you"
        let event_to_bob = damage(ObjectId::from_raw(1), DamageTarget::Player(bob), 3, false);
        assert!(!matcher.matches_event(&event_to_bob, &ctx));
    }

    #[test]
    fn test_combat_damage_matcher() {
        let game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = CombatDamageMatcher;

        let combat_damage = damage(ObjectId::from_raw(1), DamageTarget::Player(alice), 3, true);
        assert!(matcher.matches_event(&combat_damage, &ctx));

        let noncombat_damage = damage(ObjectId::from_raw(1), DamageTarget::Player(alice), 3, false);
        assert!(!matcher.matches_event(&noncombat_damage, &ctx));
    }

    #[test]
    fn test_noncombat_damage_matcher() {
        let game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = NoncombatDamageMatcher;

        let noncombat_damage = damage(ObjectId::from_raw(1), DamageTarget::Player(alice), 3, false);
        assert!(matcher.matches_event(&noncombat_damage, &ctx));

        let combat_damage = damage(ObjectId::from_raw(1), DamageTarget::Player(alice), 3, true);
        assert!(!matcher.matches_event(&combat_damage, &ctx));
    }

    #[test]
    fn test_damage_to_self_matcher_priority() {
        let matcher = DamageToSelfMatcher::new();
        assert_eq!(matcher.priority(), ReplacementPriority::Other);
    }

    #[test]
    fn test_damage_to_self_combat_matcher() {
        let game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let src = ObjectId::from_raw(42);

        let matcher = DamageToSelfCombatMatcher::new();
        let ctx = EventContext::for_replacement_effect(alice, src, &game);

        let combat_to_self = damage(src, DamageTarget::Object(src), 3, true);
        assert!(matcher.matches_event(&combat_to_self, &ctx));

        let noncombat_to_self = damage(src, DamageTarget::Object(src), 3, false);
        assert!(!matcher.matches_event(&noncombat_to_self, &ctx));

        let combat_to_other = damage(src, DamageTarget::Object(ObjectId::from_raw(7)), 3, true);
        assert!(!matcher.matches_event(&combat_to_other, &ctx));

        let combat_to_player = damage(src, DamageTarget::Player(alice), 3, true);
        assert!(!matcher.matches_event(&combat_to_player, &ctx));

        let unpreventable = unpreventable_damage(src, DamageTarget::Object(src), 3, true);
        assert!(!matcher.matches_event(&unpreventable, &ctx));
    }

    #[test]
    fn test_damage_from_self_matcher() {
        let game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let src = ObjectId::from_raw(42);

        let ctx = EventContext::for_replacement_effect(alice, src, &game);
        let matcher = DamageFromSelfMatcher::new();

        // Damage from the replacement effect's source should match.
        let from_src = damage(src, DamageTarget::Player(alice), 3, false);
        assert!(matcher.matches_event(&from_src, &ctx));

        // Damage from a different source should not match.
        let other = damage(ObjectId::from_raw(7), DamageTarget::Player(alice), 3, false);
        assert!(!matcher.matches_event(&other, &ctx));

        // Unpreventable damage should not match (prevention can't apply).
        let unpreventable = unpreventable_damage(src, DamageTarget::Player(alice), 3, false);
        assert!(!matcher.matches_event(&unpreventable, &ctx));
    }

    #[test]
    fn test_damage_to_self_from_source_filter_matcher() {
        let mut game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);

        let target_card = CardBuilder::new(CardId::new(), "Protected Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let target = game.create_object_from_card(&target_card, alice, Zone::Battlefield);

        let creature_source_card = CardBuilder::new(CardId::new(), "Creature Source")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_source =
            game.create_object_from_card(&creature_source_card, alice, Zone::Battlefield);

        let artifact_source_card = CardBuilder::new(CardId::new(), "Artifact Source")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_source =
            game.create_object_from_card(&artifact_source_card, alice, Zone::Battlefield);

        let matcher = DamageToSelfFromSourceFilterMatcher::from_creature();
        let ctx = EventContext::for_replacement_effect(alice, target, &game);

        let creature_damage = damage(creature_source, DamageTarget::Object(target), 3, false);
        assert!(matcher.matches_event(&creature_damage, &ctx));

        let noncreature_damage = damage(artifact_source, DamageTarget::Object(target), 3, false);
        assert!(!matcher.matches_event(&noncreature_damage, &ctx));

        let wrong_target_damage = damage(
            creature_source,
            DamageTarget::Object(artifact_source),
            3,
            false,
        );
        assert!(!matcher.matches_event(&wrong_target_damage, &ctx));

        let unpreventable =
            unpreventable_damage(creature_source, DamageTarget::Object(target), 3, false);
        assert!(!matcher.matches_event(&unpreventable, &ctx));
    }
}

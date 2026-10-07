//! "Whenever [filter] deals combat damage to [player]" trigger.

use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::EventKind;
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{SimultaneousTriggerKey, TriggerContext, TriggerMatcher};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct DealsCombatDamageToPlayerTrigger {
    pub filter: ObjectFilter,
    pub player: PlayerFilter,
    pub one_or_more: bool,
    /// "One or more creatures deal combat damage to a player": one event for
    /// each damaged player (CR 603.2c, 510.2), whose "that player" is that
    /// player and whose "that much damage" is the total dealt to them.
    /// Without it, "to one or more players" is one event for the whole step.
    pub each_damaged_player: bool,
    pub per_source_controller: bool,
}

impl DealsCombatDamageToPlayerTrigger {
    pub fn new(filter: ObjectFilter, player: PlayerFilter) -> Self {
        Self {
            filter,
            player,
            one_or_more: false,
            each_damaged_player: false,
            per_source_controller: false,
        }
    }

    /// "Whenever one or more [filter] deal combat damage to one or more
    /// players / of your opponents": once per combat damage step.
    pub fn one_or_more(filter: ObjectFilter, player: PlayerFilter) -> Self {
        Self {
            filter,
            player,
            one_or_more: true,
            each_damaged_player: false,
            per_source_controller: false,
        }
    }

    /// "Whenever one or more [filter] deal combat damage to a player / an
    /// opponent": once for each player dealt damage.
    pub fn one_or_more_each_player(filter: ObjectFilter, player: PlayerFilter) -> Self {
        Self {
            filter,
            player,
            one_or_more: true,
            each_damaged_player: true,
            per_source_controller: false,
        }
    }

    pub fn per_source_controller(filter: ObjectFilter, player: PlayerFilter, each_damaged_player: bool) -> Self {
        Self { filter, player, one_or_more: true, each_damaged_player, per_source_controller: true }
    }

    fn first_matching_hit_to_player_in_batch(
        &self,
        player: crate::ids::PlayerId,
        ctx: &TriggerContext,
    ) -> bool {
        for (source, damaged_player) in ctx.game.combat_damage_player_batch_hits() {
            if *damaged_player != player {
                continue;
            }
            let Some(source_obj) = ctx.game.object(*source) else {
                continue;
            };
            if self.filter.matches(source_obj, &ctx.filter_ctx, ctx.game) {
                return false;
            }
        }
        true
    }
}

impl TriggerMatcher for DealsCombatDamageToPlayerTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(e) = event.downcast::<DamageEvent>() else {
            return false;
        };
        // Must be combat damage to a player; prevented damage isn't dealt
        // (CR 615.1).
        if !e.is_combat || e.amount == 0 {
            return false;
        }
        let DamageTarget::Player(damaged_player) = e.target else {
            return false;
        };
        if self.per_source_controller {
            let Some(source) = event.source_snapshot().filter(|snapshot| snapshot.object_id == e.source) else {
                ctx.game.record_token_resource_failure(&crate::effects::ExecutionError::IncompleteEvidence(
                    "controller-grouped combat damage requires its exact damage-time source receipt".into()));
                return false;
            };
            let mut player_ctx = ctx.filter_ctx.clone();
            player_ctx.filter_candidate_players = Some((source.controller, source.owner));
            // Every matching physical assignment participates; the simultaneous
            // key groups only this controller's assignments, not another's.
            return self.filter.matches_snapshot(source, &ctx.filter_ctx, ctx.game)
                && self.player.matches_player(damaged_player, &player_ctx);
        }
        let Some(obj) = ctx.game.object(e.source) else {
            return false;
        };
        if !self.filter.matches(obj, &ctx.filter_ctx, ctx.game) {
            return false;
        }
        // "... deals combat damage to its owner": the player filter is
        // relative to the damage source (the filter candidate).
        let player_matches = if matches!(
            self.player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::FilterCandidate)
                | PlayerFilter::ControllerOf(crate::filter::ObjectRef::FilterCandidate)
        ) {
            let mut player_ctx = ctx.filter_ctx.clone();
            player_ctx.filter_candidate_players = Some((ctx.game.controller_of(obj), obj.owner));
            self.player.matches_player(damaged_player, &player_ctx)
        } else {
            self.player.matches_player(damaged_player, &ctx.filter_ctx)
        };
        if !player_matches {
            return false;
        }
        // Per damaged player, every matching assignment belongs to that
        // player's event; the per-recipient grouping queues it once and sums
        // the damage.
        if !self.one_or_more || self.each_damaged_player {
            return true;
        }
        self.first_matching_hit_to_player_in_batch(damaged_player, ctx)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Damage])
    }

    fn simultaneous_trigger_key(&self, event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
        if !self.one_or_more {
            return None;
        }
        let damage = event.downcast::<DamageEvent>()?;
        if self.per_source_controller {
            let controller = event.source_snapshot().filter(|snapshot| snapshot.object_id == damage.source)?.controller;
            return Some(SimultaneousTriggerKey::DamageSourceController(controller,
                self.each_damaged_player.then_some(damage.target)));
        }
        if self.each_damaged_player {
            return Some(SimultaneousTriggerKey::DamageTarget(damage.target));
        }
        Some(SimultaneousTriggerKey::DamageBatch)
    }

    fn display(&self) -> String {
        if !self.one_or_more && self.filter == ObjectFilter::default() {
            let player = self.player.description();
            if player == "you" {
                return "Whenever you're dealt combat damage".to_string();
            }
            return format!("Whenever {player} is dealt combat damage");
        }
        // Combat damage already implies a creature; oracle says "a Vehicle
        // you control deals combat damage", not "a Vehicle creature ...".
        let surface_filter = if self.filter.card_types == [crate::types::CardType::Creature]
            && !self.filter.subtypes.is_empty()
            && self.filter.all_card_types.is_empty()
        {
            let mut stripped = self.filter.clone();
            stripped.card_types.clear();
            stripped
        } else {
            self.filter.clone()
        };
        if self.one_or_more {
            // The plural form keeps the authored noun: "one or more Ninja or
            // Rogue creatures you control".
            let mut subject = crate::static_abilities::pluralized_subject_text(&self.filter);
            if self.filter.controller == Some(PlayerFilter::Opponent) {
                subject = if self.per_source_controller {
                    subject.replace("your opponents control", "an opponent controls")
                } else {
                    subject.replace("an opponent controls", "your opponents control")
                };
            }
            let subject = subject.strip_prefix("All ").unwrap_or(&subject);
            let player = match (&self.player, self.each_damaged_player) {
                (PlayerFilter::Opponent, false) => "one or more of your opponents".to_string(),
                (PlayerFilter::Any, false) => "one or more players".to_string(),
                (PlayerFilter::Opponent, true) => "an opponent".to_string(),
                _ => self.player.description(),
            };
            return format!("Whenever one or more {subject} deal combat damage to {player}");
        }
        let player = if matches!(self.player, PlayerFilter::Opponent) {
            "one of your opponents".to_string()
        } else if matches!(
            self.player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::FilterCandidate)
        ) {
            "its owner".to_string()
        } else if matches!(
            self.player,
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::FilterCandidate)
        ) {
            "its controller".to_string()
        } else {
            self.player.description()
        };
        let subject = with_indefinite_article(surface_filter.description());
        format!("Whenever {} deals combat damage to {}", subject, player)
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        if !self.one_or_more || !self.matches(event, ctx) {
            return None;
        }
        let damage = event.downcast::<DamageEvent>()?;
        let DamageTarget::Player(current_player) = damage.target else {
            return None;
        };
        if self.per_source_controller {
            return match i32::try_from(damage.amount) {
                Ok(amount) => Some(amount),
                Err(_) => {
                    ctx.game.record_token_resource_failure(&crate::effects::ExecutionError::ResourceLimitExceeded {
                        resource: "grouped combat damage amount", requested: u128::from(damage.amount),
                        maximum: i32::MAX as u128,
                    });
                    None
                }
            };
        }
        // "That much damage" / "the amount of damage those creatures dealt to
        // that player": this assignment's share of the per-player total.
        if self.each_damaged_player {
            return i32::try_from(damage.amount).ok();
        }

        let mut damaged_players = HashSet::new();
        damaged_players.insert(current_player);
        for (source, player) in ctx.game.combat_damage_player_batch_hits() {
            if !self.player.matches_player(*player, &ctx.filter_ctx) {
                continue;
            }
            let Some(source_obj) = ctx.game.object(*source) else {
                continue;
            };
            if self.filter.matches(source_obj, &ctx.filter_ctx, ctx.game) {
                damaged_players.insert(*player);
            }
        }
        i32::try_from(damaged_players.len()).ok()
    }
}

fn with_indefinite_article(subject: String) -> String {
    let trimmed = subject.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
        || lower.starts_with("target ")
        || lower.starts_with("your ")
        || lower.starts_with("their ")
    {
        return trimmed.to_string();
    }
    let article = if trimmed
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn combat_damage(source: ObjectId, player: PlayerId) -> DamageEvent {
        DamageEvent::with_cause(
            source,
            DamageTarget::Player(player),
            2,
            true,
            crate::events::cause::EventCause::combat_damage(source),
        )
    }

    #[test]
    fn test_display() {
        let trigger =
            DealsCombatDamageToPlayerTrigger::new(ObjectFilter::creature(), PlayerFilter::Any);
        assert!(trigger.display().contains("deals combat damage"));
    }

    #[test]
    fn singular_controller_group_uses_completed_source_snapshots_and_does_not_suppress_a_later_actor() {
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        let a = PlayerId(0); let b = PlayerId(1); let c = PlayerId(2);
        let one = create_creature(&mut game, "B one", b);
        let two = create_creature(&mut game, "B two", b);
        let three = create_creature(&mut game, "C one", c);
        let events = [one, two, three].into_iter().map(|id| {
            TriggerEvent::new_with_provenance(combat_damage(id, a), Default::default())
                .with_source_snapshot(crate::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(), &game))
        }).collect::<Vec<_>>();
        let trigger = DealsCombatDamageToPlayerTrigger::per_source_controller(
            ObjectFilter::creature().opponent_controls(), PlayerFilter::You, true);
        game.record_combat_damage_player_batch_hit(one, a);
        for id in [one, two, three] { game.set_current_controller(id, a).unwrap(); }
        let (resource_root, resource_meter) = game.begin_token_resource_scope();
        let ctx = TriggerContext::for_source(ObjectId::from_raw(1000), a, &game);
        for event in &events { assert!(trigger.matches(event, &ctx)); }
        assert_eq!(trigger.simultaneous_trigger_key(&events[0]), trigger.simultaneous_trigger_key(&events[1]));
        assert_ne!(trigger.simultaneous_trigger_key(&events[0]), trigger.simultaneous_trigger_key(&events[2]));
        let missing = TriggerEvent::new_with_provenance(combat_damage(one, a), Default::default());
        assert!(!trigger.matches(&missing, &ctx));
        assert!(matches!(game.token_resource_failure(), Some(crate::effects::ExecutionError::IncompleteEvidence(_))));
        game.end_token_resource_scope(resource_root, &resource_meter);
    }

    #[test]
    fn test_one_or_more_opponent_display_uses_plural_opponents_phrase() {
        let trigger = DealsCombatDamageToPlayerTrigger::one_or_more(
            ObjectFilter::creature(),
            PlayerFilter::Opponent,
        );
        assert_eq!(
            trigger.display(),
            "Whenever one or more creatures deal combat damage to one or more of your opponents"
        );
    }

    #[test]
    fn possessive_subject_does_not_gain_an_indefinite_article() {
        assert_eq!(
            with_indefinite_article("your commander".to_string()),
            "your commander"
        );
    }

    #[test]
    fn one_or_more_subtype_subject_keeps_the_typed_subtype_scope() {
        let trigger = DealsCombatDamageToPlayerTrigger::one_or_more(
            ObjectFilter::default()
                .with_subtype(crate::types::Subtype::Assassin)
                .you_control(),
            PlayerFilter::Any,
        );
        assert!(trigger.one_or_more);
        assert_eq!(trigger.filter.controller, Some(PlayerFilter::You));
        assert_eq!(
            trigger.filter.subtypes,
            vec![crate::types::Subtype::Assassin]
        );
        assert_eq!(trigger.player, PlayerFilter::Any);
    }

    #[test]
    fn unrestricted_source_display_uses_passive_recipient_surface() {
        let you = DealsCombatDamageToPlayerTrigger::new(ObjectFilter::default(), PlayerFilter::You);
        assert_eq!(you.display(), "Whenever you're dealt combat damage");

        let opponent =
            DealsCombatDamageToPlayerTrigger::new(ObjectFilter::default(), PlayerFilter::Opponent);
        assert_eq!(
            opponent.display(),
            "Whenever an opponent is dealt combat damage"
        );
    }

    #[test]
    fn test_one_or_more_matches_only_first_matching_hit_per_player_in_batch() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", alice);
        let attacker_two = create_creature(&mut game, "B", alice);

        let trigger = DealsCombatDamageToPlayerTrigger::one_or_more(
            ObjectFilter::creature(),
            PlayerFilter::Any,
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let first_event = TriggerEvent::new_with_provenance(
            combat_damage(attacker_one, bob),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&first_event, &ctx));

        game.record_combat_damage_player_batch_hit(attacker_one, bob);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let second_event = TriggerEvent::new_with_provenance(
            combat_damage(attacker_two, bob),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&second_event, &ctx));
    }

    #[test]
    fn test_one_or_more_groups_the_damage_batch_and_counts_distinct_damaged_players() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", alice);
        let attacker_two = create_creature(&mut game, "B", alice);
        let trigger = DealsCombatDamageToPlayerTrigger::one_or_more(
            ObjectFilter::creature(),
            PlayerFilter::Opponent,
        );

        game.record_combat_damage_player_batch_hit(attacker_one, bob);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            combat_damage(attacker_two, charlie),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(
            trigger.simultaneous_trigger_key(&event),
            Some(SimultaneousTriggerKey::DamageBatch)
        );
        assert_eq!(trigger.event_value_amount(&event, &ctx), Some(2));
    }

    #[test]
    fn test_matches_respects_damaged_player_filter() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let source_id = ObjectId::from_raw(100);
        let attacker = create_creature(&mut game, "Attacker", bob);
        let trigger =
            DealsCombatDamageToPlayerTrigger::new(ObjectFilter::creature(), PlayerFilter::You);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let hits_charlie = TriggerEvent::new_with_provenance(
            combat_damage(attacker, charlie),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&hits_charlie, &ctx));

        let hits_alice = TriggerEvent::new_with_provenance(
            combat_damage(attacker, alice),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&hits_alice, &ctx));
    }
}

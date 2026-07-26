//! "Whenever [filter] deals combat damage to [player]" trigger.

use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::EventKind;
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{SimultaneousTriggerKey, TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct DealsCombatDamageToPlayerTrigger {
    pub filter: ObjectFilter,
    pub player: PlayerFilter,
    pub one_or_more: bool,
}

impl DealsCombatDamageToPlayerTrigger {
    pub fn new(filter: ObjectFilter, player: PlayerFilter) -> Self {
        Self {
            filter,
            player,
            one_or_more: false,
        }
    }

    pub fn one_or_more(filter: ObjectFilter, player: PlayerFilter) -> Self {
        Self {
            filter,
            player,
            one_or_more: true,
        }
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
        // Must be combat damage to a player.
        if !e.is_combat {
            return false;
        }
        let DamageTarget::Player(damaged_player) = e.target else {
            return false;
        };
        let Some(obj) = ctx.game.object(e.source) else {
            return false;
        };
        if !self.filter.matches(obj, &ctx.filter_ctx, ctx.game) {
            return false;
        }
        if !self.player.matches_player(damaged_player, &ctx.filter_ctx) {
            return false;
        }
        if !self.one_or_more {
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
        Some(SimultaneousTriggerKey::DamageTarget(damage.target))
    }

    fn display(&self) -> String {
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
            let mut subject = self.filter.description();
            if let Some(stripped) = subject.strip_prefix("a ") {
                subject = stripped.to_string();
            } else if let Some(stripped) = subject.strip_prefix("an ") {
                subject = stripped.to_string();
            }
            if subject == "creature" {
                subject = "creatures".to_string();
            } else if let Some(rest) = subject.strip_prefix("creature ") {
                subject = format!("creatures {rest}");
            } else if subject.contains(" creature ") {
                subject = subject.replacen(" creature ", " creatures ", 1);
            }
            let player = if matches!(self.player, PlayerFilter::Opponent) {
                "one or more of your opponents".to_string()
            } else {
                self.player.description()
            };
            return format!("Whenever one or more {subject} deal combat damage to {player}");
        }
        let player = if matches!(self.player, PlayerFilter::Opponent) {
            "one of your opponents".to_string()
        } else {
            self.player.description()
        };
        let subject = with_indefinite_article(surface_filter.description());
        format!("Whenever {} deals combat damage to {}", subject, player)
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

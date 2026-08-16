//! "Whenever you gain life" trigger.

use crate::events::EventKind;
use crate::events::life::LifeGainEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{
    TriggerContext, TriggerMatcher, current_turn_matches_player_filter,
};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct YouGainLifeTrigger {
    pub during_turn: Option<PlayerFilter>,
    pub cause_filter: Option<ObjectFilter>,
}

impl Default for YouGainLifeTrigger {
    fn default() -> Self {
        Self::new()
    }
}

impl YouGainLifeTrigger {
    pub fn new() -> Self {
        Self {
            during_turn: None,
            cause_filter: None,
        }
    }

    pub fn during_turn(during_turn: PlayerFilter) -> Self {
        Self {
            during_turn: Some(during_turn),
            cause_filter: None,
        }
    }

    pub fn caused_by(cause_filter: ObjectFilter) -> Self {
        Self {
            during_turn: None,
            cause_filter: Some(cause_filter),
        }
    }
}

impl TriggerMatcher for YouGainLifeTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::LifeGain {
            return false;
        }
        let Some(e) = event.downcast::<LifeGainEvent>() else {
            return false;
        };
        if e.player != ctx.controller {
            return false;
        }
        if let Some(cause_filter) = &self.cause_filter {
            let Some(source) = e.source else {
                return false;
            };
            let mut cause_filter = cause_filter.clone();
            // The event itself establishes that this object caused the gain as
            // a spell. Match its card characteristics even when the stack
            // entry has already been removed during trigger processing.
            if cause_filter.zone == Some(Zone::Stack) {
                cause_filter.zone = None;
            }
            if matches!(
                cause_filter.stack_kind,
                Some(crate::filter::StackObjectKind::Spell)
            ) {
                cause_filter.stack_kind = None;
            }
            let matches_source = ctx
                .game
                .object(source)
                .is_some_and(|object| cause_filter.matches(object, &ctx.filter_ctx, ctx.game))
                || event.source_snapshot().is_some_and(|snapshot| {
                    cause_filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
                });
            if !matches_source {
                return false;
            }
        }
        if let Some(during_turn) = &self.during_turn {
            return current_turn_matches_player_filter(during_turn, ctx, None);
        }
        true
    }

    fn display(&self) -> String {
        if let Some(cause_filter) = &self.cause_filter {
            format!(
                "Whenever {} causes you to gain life",
                with_indefinite_article(&cause_filter.description())
            )
        } else if let Some(during_turn) = &self.during_turn {
            let suffix = match during_turn {
                PlayerFilter::You => " during your turn",
                PlayerFilter::Opponent => " during an opponent's turn",
                PlayerFilter::Specific(_) => " during that player's turn",
                _ => "",
            };
            format!("Whenever you gain life{suffix}")
        } else {
            "Whenever you gain life".to_string()
        }
    }
}

fn with_indefinite_article(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with("a ") || trimmed.starts_with("an ") || trimmed.starts_with("the ") {
        return trimmed.to_string();
    }
    let article = if matches!(
        trimmed.chars().next().map(|ch| ch.to_ascii_lowercase()),
        Some('a' | 'e' | 'i' | 'o' | 'u')
    ) {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn test_matches_own_life_gain() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = YouGainLifeTrigger::new();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            LifeGainEvent::new(alice, 3),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_opponent_life_gain() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = YouGainLifeTrigger::new();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            LifeGainEvent::new(bob, 3),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn causal_life_gain_matches_the_spells_characteristics() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let trigger_source = ObjectId::from_raw(500);
        let spell = CardBuilder::new(CardId::from_raw(501), "White Lifegain Spell")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);
        game.push_to_stack(crate::game_state::StackEntry::new(spell_id, alice));

        let mut filter = ObjectFilter::instant_or_sorcery();
        filter.colors = Some(crate::color::ColorSet::WHITE);
        let trigger = YouGainLifeTrigger::caused_by(filter);
        let ctx = TriggerContext::for_source(trigger_source, alice, &game);

        let matching = TriggerEvent::new_with_provenance(
            LifeGainEvent::new(alice, 3).with_source(spell_id),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&matching, &ctx));
        assert_eq!(
            trigger.display(),
            "Whenever a white instant or sorcery spell causes you to gain life"
        );

        let source_less = TriggerEvent::new_with_provenance(
            LifeGainEvent::new(alice, 3),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&source_less, &ctx));
    }
}

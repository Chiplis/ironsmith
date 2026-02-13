//! "Whenever [player] casts [spell]" trigger.

use crate::events::EventKind;
use crate::events::spells::SpellCastEvent;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct SpellCastTrigger {
    pub filter: Option<ObjectFilter>,
    pub caster: PlayerFilter,
    pub during_turn: Option<PlayerFilter>,
    pub min_spells_this_turn: Option<u32>,
    pub from_not_hand: bool,
}

impl SpellCastTrigger {
    pub fn new(filter: Option<ObjectFilter>, caster: PlayerFilter) -> Self {
        Self {
            filter,
            caster,
            during_turn: None,
            min_spells_this_turn: None,
            from_not_hand: false,
        }
    }

    pub fn qualified(
        filter: Option<ObjectFilter>,
        caster: PlayerFilter,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    ) -> Self {
        Self {
            filter,
            caster,
            during_turn,
            min_spells_this_turn,
            from_not_hand,
        }
    }

    pub fn you_cast_any() -> Self {
        Self::new(None, PlayerFilter::You)
    }

    pub fn any_cast_any() -> Self {
        Self::new(None, PlayerFilter::Any)
    }
}

impl TriggerMatcher for SpellCastTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::SpellCast {
            return false;
        }
        let Some(e) = event.downcast::<SpellCastEvent>() else {
            return false;
        };

        // Check caster filter
        let caster_matches = match &self.caster {
            PlayerFilter::You => e.caster == ctx.controller,
            PlayerFilter::Opponent => e.caster != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.caster == *id,
            _ => true,
        };

        if !caster_matches {
            return false;
        }

        if let Some(turn_filter) = &self.during_turn {
            let active_player = ctx.game.turn.active_player;
            let turn_matches = match turn_filter {
                PlayerFilter::You => active_player == ctx.controller,
                PlayerFilter::Opponent => active_player != ctx.controller,
                PlayerFilter::Any | PlayerFilter::Active => true,
                PlayerFilter::Specific(id) => active_player == *id,
                _ => true,
            };
            if !turn_matches {
                return false;
            }
        }

        if let Some(min_spells) = self.min_spells_this_turn {
            let cast_count = ctx
                .game
                .spells_cast_this_turn
                .get(&e.caster)
                .copied()
                .unwrap_or(0);
            if cast_count < min_spells {
                return false;
            }
        }
        if self.from_not_hand && e.from_zone == Zone::Hand {
            return false;
        }

        // Check spell filter if present
        if let Some(ref filter) = self.filter {
            let mut stack_filter = filter.clone();
            if let Some(zone) = filter.zone
                && zone != Zone::Stack
            {
                if e.from_zone != zone {
                    return false;
                }
                stack_filter.zone = Some(Zone::Stack);
            }
            if let Some(obj) = ctx.game.object(e.spell) {
                stack_filter.matches(obj, &ctx.filter_ctx, ctx.game)
            } else {
                false
            }
        } else {
            true
        }
    }

    fn display(&self) -> String {
        let caster_text = match &self.caster {
            PlayerFilter::You => "you cast",
            PlayerFilter::Any => "a player casts",
            PlayerFilter::Opponent => "an opponent casts",
            _ => "someone casts",
        };
        let mut spell_text = self
            .filter
            .as_ref()
            .map(describe_spell_filter)
            .unwrap_or_else(|| "a spell".to_string());
        let mut suffix = String::new();
        if self.min_spells_this_turn == Some(2) && spell_text == "a spell" {
            spell_text = "another spell".to_string();
        } else if self.min_spells_this_turn == Some(2) {
            suffix.push_str(" as your second spell this turn");
        }
        if let Some(turn_filter) = &self.during_turn {
            let turn_text = match turn_filter {
                PlayerFilter::You => " during your turn",
                PlayerFilter::Opponent => " during an opponent's turn",
                PlayerFilter::Specific(_) => " during that player's turn",
                _ => "",
            };
            suffix.push_str(turn_text);
        }
        if self.from_not_hand {
            suffix.push_str(" from anywhere other than your hand");
        }
        format!("Whenever {} {}{}", caster_text, spell_text, suffix)
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

fn describe_spell_filter(filter: &ObjectFilter) -> String {
    if filter.zone == Some(Zone::Graveyard) {
        let owner_text = match filter.owner.as_ref().unwrap_or(&PlayerFilter::Any) {
            PlayerFilter::You => "your",
            PlayerFilter::Opponent => "an opponent's",
            _ => "a",
        };
        if owner_text == "a" {
            return "a spell from a graveyard".to_string();
        }
        return format!("a spell from {owner_text} graveyard");
    }
    if filter.zone == Some(Zone::Exile) {
        return "a spell from exile".to_string();
    }
    if filter.card_types.is_empty()
        && filter
            .excluded_card_types
            .contains(&crate::types::CardType::Creature)
        && filter
            .excluded_card_types
            .contains(&crate::types::CardType::Land)
    {
        return "a noncreature spell".to_string();
    }

    let fallback = filter.description();
    if fallback == "permanent" {
        "a spell".to_string()
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::ids::{ObjectId, PlayerId};
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn test_matches_own_spell() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let spell_id = ObjectId::from_raw(2);

        let trigger = SpellCastTrigger::you_cast_any();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(SpellCastEvent::new(spell_id, alice, Zone::Hand));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = SpellCastTrigger::you_cast_any();
        assert!(trigger.display().contains("you cast"));
    }

    #[test]
    fn test_display_noncreature_spell_filter() {
        let trigger =
            SpellCastTrigger::new(Some(ObjectFilter::noncreature_spell()), PlayerFilter::You);
        assert_eq!(trigger.display(), "Whenever you cast a noncreature spell");
    }

    #[test]
    fn test_matches_spell_cast_from_graveyard_zone_filter() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let spell = CardBuilder::new(CardId::new(), "Graveyard Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);

        let trigger = SpellCastTrigger::new(
            Some(
                ObjectFilter::spell()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You),
            ),
            PlayerFilter::You,
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let from_graveyard =
            TriggerEvent::new(SpellCastEvent::new(spell_id, alice, Zone::Graveyard));
        assert!(trigger.matches(&from_graveyard, &ctx));

        let from_hand = TriggerEvent::new(SpellCastEvent::new(spell_id, alice, Zone::Hand));
        assert!(!trigger.matches(&from_hand, &ctx));
    }

    #[test]
    fn test_display_spell_from_graveyard_filter() {
        let trigger = SpellCastTrigger::new(
            Some(
                ObjectFilter::spell()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You),
            ),
            PlayerFilter::You,
        );
        assert_eq!(
            trigger.display(),
            "Whenever you cast a spell from your graveyard"
        );
    }

    #[test]
    fn test_qualified_second_spell_during_your_turn_display() {
        let trigger = SpellCastTrigger::qualified(
            None,
            PlayerFilter::You,
            Some(PlayerFilter::You),
            Some(2),
            false,
        );
        assert_eq!(
            trigger.display(),
            "Whenever you cast another spell during your turn"
        );
    }
}

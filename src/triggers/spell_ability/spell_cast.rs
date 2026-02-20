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
    pub exact_spells_this_turn: Option<u32>,
    pub from_not_hand: bool,
}

impl SpellCastTrigger {
    pub fn new(filter: Option<ObjectFilter>, caster: PlayerFilter) -> Self {
        Self {
            filter,
            caster,
            during_turn: None,
            min_spells_this_turn: None,
            exact_spells_this_turn: None,
            from_not_hand: false,
        }
    }

    pub fn qualified(
        filter: Option<ObjectFilter>,
        caster: PlayerFilter,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    ) -> Self {
        Self {
            filter,
            caster,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
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
            PlayerFilter::Active => e.caster == ctx.game.turn.active_player,
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

        let cast_count = ctx
            .game
            .spells_cast_this_turn
            .get(&e.caster)
            .copied()
            .unwrap_or(0);
        if let Some(exact_spells) = self.exact_spells_this_turn {
            if cast_count != exact_spells {
                return false;
            }
        } else if let Some(min_spells) = self.min_spells_this_turn {
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
            // `ObjectFilter::spell()` historically used `has_mana_cost` as a rough
            // spell-vs-ability proxy on stack filters. SpellCastEvent already
            // guarantees this is a spell, and real spells can have no mana cost
            // (e.g. suspend cards), so drop that extra gate here.
            if stack_filter.zone == Some(Zone::Stack) {
                stack_filter.has_mana_cost = false;
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
            PlayerFilter::Active => "the active player casts",
            _ => "someone casts",
        };
        let mut spell_text = self
            .filter
            .as_ref()
            .map(describe_spell_filter)
            .unwrap_or_else(|| "a spell".to_string());
        let mut suffix = String::new();
        if let Some(exact_spells) = self.exact_spells_this_turn {
            let ordinal = ordinal_word(exact_spells);
            if spell_text == "a spell" || spell_text == "spell" {
                spell_text = match &self.caster {
                    PlayerFilter::You => format!("your {ordinal} spell each turn"),
                    PlayerFilter::Any => format!("their {ordinal} spell each turn"),
                    PlayerFilter::Active => format!("their {ordinal} spell each turn"),
                    PlayerFilter::Opponent | PlayerFilter::Specific(_) => {
                        format!("that player's {ordinal} spell each turn")
                    }
                    _ => format!("the {ordinal} spell each turn"),
                };
            } else {
                let exact_suffix = match &self.caster {
                    PlayerFilter::You => format!(" as your {ordinal} spell this turn"),
                    PlayerFilter::Any => format!(" as the {ordinal} spell this turn"),
                    PlayerFilter::Active => format!(" as that player's {ordinal} spell this turn"),
                    PlayerFilter::Opponent | PlayerFilter::Specific(_) => {
                        format!(" as that player's {ordinal} spell this turn")
                    }
                    _ => format!(" as the {ordinal} spell this turn"),
                };
                suffix.push_str(&exact_suffix);
            }
        } else if self.min_spells_this_turn == Some(2)
            && matches!(self.caster, PlayerFilter::Any)
            && (spell_text == "a spell" || spell_text == "spell")
        {
            spell_text = "their second spell each turn".to_string();
        } else if self.min_spells_this_turn == Some(2) && spell_text == "a spell" {
            spell_text = "another spell".to_string();
        } else if self.min_spells_this_turn == Some(2)
            && matches!(
                self.caster,
                PlayerFilter::Opponent | PlayerFilter::Specific(_)
            )
        {
            spell_text = format!(
                "{spell_text} other than the first {spell_text} that player casts each turn"
            );
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
    if filter.targets_player.is_some() || filter.targets_object.is_some() {
        let mut base_filter = filter.clone();
        let targets_player = base_filter.targets_player.take();
        let targets_object = base_filter.targets_object.take();

        let mut base_text = describe_spell_filter(&base_filter);
        if base_text == "spell" {
            base_text = "a spell".to_string();
        } else if !base_text.to_ascii_lowercase().contains("spell") {
            base_text.push_str(" spell");
        }

        let mut target_parts = Vec::new();
        if let Some(player_filter) = targets_player {
            target_parts.push(match player_filter {
                PlayerFilter::You => "you".to_string(),
                PlayerFilter::NotYou => "a player other than you".to_string(),
                PlayerFilter::Opponent => "an opponent".to_string(),
                PlayerFilter::Any => "a player".to_string(),
                PlayerFilter::Specific(_) => "that player".to_string(),
                PlayerFilter::Teammate => "a teammate".to_string(),
                PlayerFilter::Active => "the active player".to_string(),
                PlayerFilter::Defending => "the defending player".to_string(),
                PlayerFilter::Attacking => "an attacking player".to_string(),
                PlayerFilter::DamagedPlayer => "the damaged player".to_string(),
                PlayerFilter::IteratedPlayer => "that player".to_string(),
                PlayerFilter::Target(inner) => match inner.as_ref() {
                    PlayerFilter::You => "you".to_string(),
                    PlayerFilter::NotYou => "a player other than you".to_string(),
                    PlayerFilter::Opponent => "an opponent".to_string(),
                    PlayerFilter::Any => "a player".to_string(),
                    _ => "target player".to_string(),
                },
                PlayerFilter::ControllerOf(_) => "that object's controller".to_string(),
                PlayerFilter::OwnerOf(_) => "that object's owner".to_string(),
            });
        }
        if let Some(object_filter) = targets_object {
            let mut object_desc = object_filter.description();
            if object_desc == "this source" {
                object_desc = "this creature".to_string();
            } else if object_desc == "that source" {
                object_desc = "that creature".to_string();
            }
            target_parts.push(object_desc);
        }

        if !target_parts.is_empty() {
            let targets = if target_parts.len() == 2 {
                format!("{} and {}", target_parts[0], target_parts[1])
            } else {
                target_parts[0].clone()
            };
            return format!("{base_text} that targets {targets}");
        }
        return base_text;
    }

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
    if filter.zone == Some(Zone::Stack) {
        match fallback.as_str() {
            "you own"
            | "you don't own"
            | "an opponent owns"
            | "a player owns"
            | "active player owns"
            | "defending player owns"
            | "attacking player owns"
            | "damaged player owns"
            | "a teammate owns"
            | "that player owns"
            | "they own" => return format!("a spell {fallback}"),
            _ => {}
        }
    }
    if fallback == "permanent" {
        "a spell".to_string()
    } else if fallback.to_ascii_lowercase().contains("spell") {
        fallback
    } else {
        format!("{fallback} spell")
    }
}

fn ordinal_word(value: u32) -> &'static str {
    match value {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        5 => "fifth",
        6 => "sixth",
        7 => "seventh",
        8 => "eighth",
        9 => "ninth",
        10 => "tenth",
        _ => "nth",
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
            None,
            false,
        );
        assert_eq!(
            trigger.display(),
            "Whenever you cast another spell during your turn"
        );
    }

    #[test]
    fn test_qualified_second_spell_any_player_display() {
        let trigger = SpellCastTrigger::qualified(
            Some(ObjectFilter::spell().in_zone(Zone::Stack)),
            PlayerFilter::Any,
            None,
            Some(2),
            None,
            false,
        );
        assert_eq!(
            trigger.display(),
            "Whenever a player casts their second spell each turn"
        );
    }

    #[test]
    fn test_qualified_third_spell_you_display() {
        let trigger =
            SpellCastTrigger::qualified(None, PlayerFilter::You, None, None, Some(3), false);
        assert_eq!(
            trigger.display(),
            "Whenever you cast your third spell each turn"
        );
    }

    #[test]
    fn test_display_spell_filter_with_targeted_object_clause() {
        let trigger = SpellCastTrigger::new(
            Some(ObjectFilter::spell().targeting_object(ObjectFilter::source())),
            PlayerFilter::You,
        );
        assert_eq!(
            trigger.display(),
            "Whenever you cast a spell that targets this creature"
        );
    }
}

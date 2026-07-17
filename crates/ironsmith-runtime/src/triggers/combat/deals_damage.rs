//! "Whenever [filter] deals damage" trigger.

use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::EventKind;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::target::{PlayerFilter, PlayerFilterExt};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{SimultaneousTriggerKey, TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct DealsDamageTrigger {
    pub filter: ObjectFilter,
    pub damaged_player: Option<PlayerFilter>,
    pub combat_only: bool,
    pub noncombat_only: bool,
    pub source_surface: ironsmith_core::trigger_model::DamageSourceSurface,
}

impl DealsDamageTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            damaged_player: None,
            combat_only: false,
            noncombat_only: false,
            source_surface: ironsmith_core::trigger_model::DamageSourceSurface::Filter,
        }
    }

    pub fn with_source_surface(
        filter: ObjectFilter,
        source_surface: ironsmith_core::trigger_model::DamageSourceSurface,
    ) -> Self {
        Self {
            source_surface,
            ..Self::new(filter)
        }
    }

    pub fn to_player(
        filter: ObjectFilter,
        damaged_player: PlayerFilter,
        source_surface: ironsmith_core::trigger_model::DamageSourceSurface,
    ) -> Self {
        Self {
            damaged_player: Some(damaged_player),
            ..Self::with_source_surface(filter, source_surface)
        }
    }

    pub fn combat_only(filter: ObjectFilter) -> Self {
        Self {
            filter,
            damaged_player: None,
            combat_only: true,
            noncombat_only: false,
            source_surface: ironsmith_core::trigger_model::DamageSourceSurface::Filter,
        }
    }

    pub fn noncombat_to_player(
        filter: ObjectFilter,
        damaged_player: PlayerFilter,
        source_surface: ironsmith_core::trigger_model::DamageSourceSurface,
    ) -> Self {
        Self {
            noncombat_only: true,
            ..Self::to_player(filter, damaged_player, source_surface)
        }
    }
}

impl TriggerMatcher for DealsDamageTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(e) = event.downcast::<DamageEvent>() else {
            return false;
        };
        if self.combat_only && !e.is_combat {
            return false;
        }
        if self.noncombat_only && e.is_combat {
            return false;
        }
        if let Some(player_filter) = &self.damaged_player {
            let DamageTarget::Player(player) = e.target else {
                return false;
            };
            if !player_filter.matches_player(player, &ctx.filter_ctx) {
                return false;
            }
        }
        let Some(obj) = ctx.game.object(e.source) else {
            return false;
        };
        if !self.filter.matches(obj, &ctx.filter_ctx, ctx.game) {
            return false;
        }
        if self.filter.union_is_one_or_more()
            && e.is_combat
            && let DamageTarget::Player(damaged_player) = e.target
        {
            let already_matched = ctx.game.combat_damage_player_batch_hits().iter().any(
                |(prior_source, prior_player)| {
                    *prior_player == damaged_player
                        && ctx.game.object(*prior_source).is_some_and(|source| {
                            self.filter.matches(source, &ctx.filter_ctx, ctx.game)
                        })
                },
            );
            if already_matched {
                return false;
            }
        }
        true
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Damage])
    }

    fn simultaneous_trigger_key(&self, event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
        if !self.filter.union_is_one_or_more() {
            return None;
        }
        let damage = event.downcast::<DamageEvent>()?;
        if self.damaged_player.is_some() {
            Some(SimultaneousTriggerKey::DamageTarget(damage.target))
        } else {
            Some(SimultaneousTriggerKey::DamageBatch)
        }
    }

    fn display(&self) -> String {
        let grouped_sources = self.filter.union_is_one_or_more();
        let mut surface_filter = self.filter.clone();
        surface_filter.set_union_one_or_more(false);
        let source_description =
            if self.source_surface == ironsmith_core::trigger_model::DamageSourceSurface::Source {
                generic_source_description(&surface_filter)
            } else if surface_filter == ObjectFilter::default() {
                "a source".to_string()
            } else {
                surface_filter.description()
            };
        let source_description = if grouped_sources {
            format!(
                "one or more {}",
                pluralize_damage_source(&source_description)
            )
        } else {
            source_description
        };
        let verb = if grouped_sources { "deal" } else { "deals" };
        if self.combat_only {
            format!("Whenever {source_description} {verb} combat damage")
        } else if self.noncombat_only {
            if let Some(player) = &self.damaged_player {
                format!(
                    "Whenever {} {} noncombat damage to {}",
                    source_description,
                    verb,
                    player.description()
                )
            } else {
                format!("Whenever {source_description} {verb} noncombat damage")
            }
        } else if let Some(player) = &self.damaged_player {
            if self.source_surface == ironsmith_core::trigger_model::DamageSourceSurface::Filter
                && self.filter == ObjectFilter::default()
            {
                let player_description = player.description();
                if player_description == "you" {
                    return "Whenever you are dealt damage".to_string();
                }
                return format!("Whenever {} is dealt damage", player_description);
            }
            format!(
                "Whenever {} {} damage to {}",
                source_description,
                verb,
                player.description()
            )
        } else {
            format!("Whenever {source_description} {verb} damage")
        }
    }
}

fn pluralize_damage_source(description: &str) -> String {
    let description = description
        .strip_prefix("a ")
        .or_else(|| description.strip_prefix("an "))
        .unwrap_or(description);
    let suffixes = [
        " you control",
        " you don't control",
        " an opponent controls",
        " your opponent controls",
        " controlled by an opponent",
        " controlled by you",
    ];
    let (subject, suffix) = suffixes
        .into_iter()
        .find_map(|suffix| {
            description
                .strip_suffix(suffix)
                .map(|subject| (subject, suffix))
        })
        .unwrap_or((description, ""));
    let mut words = subject
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let Some(noun) = words.last_mut() else {
        return description.to_string();
    };
    let lower = noun.to_ascii_lowercase();
    let plural = if lower.ends_with('y')
        && !lower
            .chars()
            .rev()
            .nth(1)
            .is_some_and(|ch| matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        format!("{}ies", &noun[..noun.len().saturating_sub(1)])
    } else if lower.ends_with('s')
        || lower.ends_with('x')
        || lower.ends_with('z')
        || lower.ends_with("ch")
        || lower.ends_with("sh")
        || lower.ends_with('o')
    {
        format!("{noun}es")
    } else {
        format!("{noun}s")
    };
    *noun = plural;
    let result = words.join(" ");
    format!("{result}{suffix}")
}

pub(super) fn generic_source_description(filter: &ObjectFilter) -> String {
    let uses_default_permanent_noun = filter.zone.is_none()
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && !filter.token
        && !filter.nontoken
        && filter.stack_kind.is_none();
    if uses_default_permanent_noun {
        let description = filter.description();
        if description.contains("permanent") {
            let source = description.replacen("permanent", "source", 1);
            if source == "source" {
                return "a source".to_string();
            }
            return source;
        }
    }
    let mut remaining = filter.clone();
    let controller = remaining.controller.take();
    if remaining != ObjectFilter::default() {
        return filter.description();
    }
    match controller {
        Some(PlayerFilter::You) => "a source you control".to_string(),
        Some(PlayerFilter::Opponent) => "a source an opponent controls".to_string(),
        Some(PlayerFilter::NotYou) => "a source you don't control".to_string(),
        None | Some(PlayerFilter::Any) => "a source".to_string(),
        _ => filter.description(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::events::cause::EventCause;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::provenance::ProvNodeId;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    #[test]
    fn test_display() {
        let trigger = DealsDamageTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("deals damage"));
    }

    #[test]
    fn generic_source_surface_and_opponent_recipient_are_preserved() {
        let mut filter = ObjectFilter::default();
        filter.controller = Some(PlayerFilter::You);
        let trigger = DealsDamageTrigger::noncombat_to_player(
            filter,
            PlayerFilter::Opponent,
            ironsmith_core::trigger_model::DamageSourceSurface::Source,
        );
        assert_eq!(
            trigger.display(),
            "Whenever a source you control deals noncombat damage to an opponent"
        );
    }

    #[test]
    fn grouped_hero_sources_preserve_plural_surface_and_recipient_batch_key() {
        let mut heroes = ObjectFilter::default()
            .with_subtype(Subtype::Hero)
            .controlled_by(PlayerFilter::You);
        heroes.set_union_one_or_more(true);
        let trigger = DealsDamageTrigger::to_player(
            heroes,
            PlayerFilter::Any,
            ironsmith_core::trigger_model::DamageSourceSurface::Filter,
        );
        assert_eq!(
            trigger.display(),
            "Whenever one or more Heroes you control deal damage to a player"
        );

        let damaged_player = PlayerId::from_index(1);
        let event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                ObjectId::from_raw(9),
                DamageTarget::Player(damaged_player),
                2,
                false,
                EventCause::effect(),
            ),
            ProvNodeId::default(),
        );
        assert_eq!(
            trigger.simultaneous_trigger_key(&event),
            Some(SimultaneousTriggerKey::DamageTarget(DamageTarget::Player(
                damaged_player
            )))
        );
    }

    #[test]
    fn authored_generic_source_to_you_does_not_collapse_to_passive_voice() {
        let trigger = DealsDamageTrigger::to_player(
            ObjectFilter::default(),
            PlayerFilter::You,
            ironsmith_core::trigger_model::DamageSourceSurface::Source,
        );

        assert_eq!(trigger.display(), "Whenever a source deals damage to you");
    }

    #[test]
    fn opponent_controlled_source_matches_damage_from_the_stack() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let spell = CardBuilder::new(CardId::new(), "Damage Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&spell, bob, Zone::Stack);
        let trigger = DealsDamageTrigger::to_player(
            ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
            PlayerFilter::You,
            ironsmith_core::trigger_model::DamageSourceSurface::Source,
        );
        let ctx = TriggerContext::for_source(ObjectId::from_raw(100), alice, &game);
        let event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                spell_id,
                DamageTarget::Player(alice),
                3,
                false,
                EventCause::effect(),
            ),
            ProvNodeId::default(),
        );

        assert_eq!(
            trigger.display(),
            "Whenever a source an opponent controls deals damage to you"
        );
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn controlled_noncreature_source_matches_stack_spell_but_not_creature() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let spell = CardBuilder::new(CardId::new(), "Damage Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let creature = CardBuilder::new(CardId::new(), "Damage Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let mut filter = ObjectFilter::default().controlled_by(PlayerFilter::You);
        filter.excluded_card_types.push(CardType::Creature);
        let trigger = DealsDamageTrigger::with_source_surface(
            filter,
            ironsmith_core::trigger_model::DamageSourceSurface::Source,
        );
        let ctx = TriggerContext::for_source(ObjectId::from_raw(100), alice, &game);
        let event_from = |source| {
            TriggerEvent::new_with_provenance(
                DamageEvent::with_cause(
                    source,
                    DamageTarget::Player(bob),
                    3,
                    false,
                    EventCause::effect(),
                ),
                ProvNodeId::default(),
            )
        };

        assert_eq!(
            trigger.display(),
            "Whenever a noncreature source you control deals damage"
        );
        assert!(trigger.matches(&event_from(spell_id), &ctx));
        assert!(!trigger.matches(&event_from(creature_id), &ctx));
    }
}

//! "Whenever this creature and at least N other creatures attack" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when the source creature attacks with at least N other creatures.
///
/// This captures battalion-style wording:
/// "Whenever this creature and at least two other creatures attack, ..."
#[derive(Debug, Clone, PartialEq)]
pub struct ThisAttacksWithNOthersTrigger {
    /// Minimum number of *other* attacking creatures required.
    pub other_count: usize,
    /// Whether the trigger requires exactly that many other attackers.
    pub exact: bool,
    /// Optional rendered subject, used when the parser saw a named source.
    pub display_subject: Option<String>,
    /// Optional filter for the required other attackers.
    pub other_filter: Option<ObjectFilter>,
}

impl ThisAttacksWithNOthersTrigger {
    pub const fn new(other_count: usize) -> Self {
        Self {
            other_count,
            exact: false,
            display_subject: None,
            other_filter: None,
        }
    }

    pub fn with_display_subject(
        other_count: usize,
        display_subject: Option<String>,
        other_filter: Option<ObjectFilter>,
    ) -> Self {
        Self {
            other_count,
            exact: false,
            display_subject,
            other_filter,
        }
    }

    pub const fn exact(other_count: usize) -> Self {
        Self {
            other_count,
            exact: true,
            display_subject: None,
            other_filter: None,
        }
    }

    fn matching_other_attackers(&self, ctx: &TriggerContext) -> Option<usize> {
        let other_filter = self.other_filter.as_ref()?;
        let combat = ctx.game.combat.as_ref()?;
        Some(
            combat
                .attackers
                .iter()
                .filter(|info| info.creature != ctx.source_id)
                .filter(|info| {
                    ctx.game.object(info.creature).is_some_and(|object| {
                        other_filter.matches(object, &ctx.filter_ctx, ctx.game)
                    })
                })
                .count(),
        )
    }

    fn other_subject(&self) -> String {
        let Some(filter) = &self.other_filter else {
            return if self.other_count == 1 {
                "creature".to_string()
            } else {
                "creatures".to_string()
            };
        };
        let mut display_filter = filter.clone();
        if display_filter.card_types == [crate::types::CardType::Creature]
            && !display_filter.subtypes.is_empty()
            && display_filter.all_card_types.is_empty()
        {
            display_filter.card_types.clear();
        }
        let mut subject = display_filter.description();
        for prefix in ["a ", "an "] {
            if let Some(stripped) = subject.strip_prefix(prefix) {
                subject = stripped.to_string();
                break;
            }
        }
        subject
    }

    fn displayed_other_subject(&self) -> String {
        pluralize_other_subject(self.other_subject(), self.other_count)
    }
}

fn pluralize_other_subject(subject: String, count: usize) -> String {
    if count == 1 {
        return subject;
    }
    if subject == "creature" {
        return "creatures".to_string();
    }
    if let Some(rest) = subject.strip_prefix("creature ") {
        return format!("creatures {rest}");
    }
    if let Some((head, tail)) = subject.split_once(' ') {
        return format!("{}s {tail}", head.trim_end_matches('s'));
    }
    format!("{}s", subject.trim_end_matches('s'))
}

impl TriggerMatcher for ThisAttacksWithNOthersTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        // The source itself must be one of the attackers, and the declared
        // attackers must satisfy the requested source-plus-others threshold.
        if e.attacker != ctx.source_id {
            return false;
        }
        if let Some(matching_others) = self.matching_other_attackers(ctx) {
            if self.exact {
                matching_others == self.other_count
            } else {
                matching_others >= self.other_count
            }
        } else if self.exact {
            e.total_attackers == self.other_count.saturating_add(1)
        } else {
            e.total_attackers >= self.other_count.saturating_add(1)
        }
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureAttacked])
    }

    fn source_must_match_event_object(&self, event_kind: EventKind) -> bool {
        event_kind == EventKind::CreatureAttacked
    }

    fn display(&self) -> String {
        let subject = self.displayed_other_subject();
        if self.exact {
            let count = ironsmith_core::cardinal_word(self.other_count as u32)
                .unwrap_or_else(|| self.other_count.to_string());
            format!(
                "Whenever this creature attacks, if you attacked with exactly {count} other {subject} this combat"
            )
        } else {
            let source_subject = self.display_subject.as_deref().unwrap_or("this creature");
            if self.other_count == 1
                && self
                    .other_filter
                    .as_ref()
                    .is_some_and(|filter| filter.other)
            {
                return format!("Whenever {source_subject} and {subject} attack");
            }
            let count = ironsmith_core::cardinal_word(self.other_count as u32)
                .unwrap_or_else(|| self.other_count.to_string());
            format!("Whenever {source_subject} and at least {count} other {subject} attack",)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::events::combat::AttackEventTarget;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature_with_subtype(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        subtype: Subtype,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![subtype])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    #[test]
    fn matches_when_source_attacks_with_enough_others() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(11);
        let trigger = ThisAttacksWithNOthersTrigger::new(2);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                source_id,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_source_is_not_attacker() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(11);
        let other_id = ObjectId::from_raw(12);
        let trigger = ThisAttacksWithNOthersTrigger::new(2);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                other_id,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_not_enough_attackers() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(11);
        let trigger = ThisAttacksWithNOthersTrigger::new(2);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                source_id,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn exact_mode_requires_exact_other_count() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(11);
        let trigger = ThisAttacksWithNOthersTrigger::exact(1);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let exact_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                source_id,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&exact_event, &ctx));

        let too_many_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                source_id,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&too_many_event, &ctx));
    }

    #[test]
    fn paladin_elizabeth_taggerdy_named_battalion_display_keeps_threshold_subject() {
        let trigger = ThisAttacksWithNOthersTrigger::with_display_subject(
            2,
            Some("Paladin Elizabeth Taggerdy".to_string()),
            None,
        );

        assert_eq!(
            trigger.display(),
            "Whenever Paladin Elizabeth Taggerdy and at least two other creatures attack"
        );
    }

    #[test]
    fn named_source_and_another_filtered_attacker_preserves_oracle_surface() {
        let trigger = ThisAttacksWithNOthersTrigger::with_display_subject(
            1,
            Some("Merry".to_string()),
            Some(
                ObjectFilter::creature()
                    .with_supertype(crate::types::Supertype::Legendary)
                    .other(),
            ),
        );

        assert_eq!(
            trigger.display(),
            "Whenever Merry and another legendary creature attack"
        );
    }

    #[test]
    fn filtered_other_attackers_count_only_matching_subtype() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id =
            create_creature_with_subtype(&mut game, "Paired Tactician", alice, Subtype::Warrior);
        let other_warrior =
            create_creature_with_subtype(&mut game, "Other Warrior", alice, Subtype::Warrior);
        let other_scout =
            create_creature_with_subtype(&mut game, "Other Scout", alice, Subtype::Scout);
        game.combat = Some(CombatState {
            attackers: vec![
                AttackerInfo {
                    creature: source_id,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: other_warrior,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: other_scout,
                    target: AttackTarget::Player(bob),
                },
            ],
            ..Default::default()
        });
        let trigger = ThisAttacksWithNOthersTrigger::with_display_subject(
            2,
            Some("this creature".to_string()),
            Some(
                ObjectFilter::creature()
                    .with_subtype(Subtype::Warrior)
                    .in_zone(Zone::Battlefield),
            ),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                source_id,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(
            !trigger.matches(&event, &ctx),
            "only one other Warrior is attacking"
        );

        let trigger = ThisAttacksWithNOthersTrigger::with_display_subject(
            1,
            Some("this creature".to_string()),
            Some(
                ObjectFilter::creature()
                    .with_subtype(Subtype::Warrior)
                    .in_zone(Zone::Battlefield),
            ),
        );

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(
            trigger.display(),
            "Whenever this creature and at least one other Warrior attack"
        );
    }
}

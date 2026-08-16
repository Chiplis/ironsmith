//! "Whenever [source] deals exactly N damage to [object] or [player]" trigger.

use crate::events::{DamageEvent, DamageTarget, EventKind};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use ironsmith_core::trigger_model::DamageSourceSurface;

#[derive(Debug, Clone, PartialEq)]
pub struct DealsExactDamageToObjectOrPlayerTrigger {
    pub source_filter: ObjectFilter,
    pub object_filter: ObjectFilter,
    pub player_filter: PlayerFilter,
    pub player_first: bool,
    pub amount: u32,
    pub source_surface: DamageSourceSurface,
}

impl DealsExactDamageToObjectOrPlayerTrigger {
    pub fn new(
        source_filter: ObjectFilter,
        object_filter: ObjectFilter,
        player_filter: PlayerFilter,
        player_first: bool,
        amount: u32,
        source_surface: DamageSourceSurface,
    ) -> Self {
        Self {
            source_filter,
            object_filter,
            player_filter,
            player_first,
            amount,
            source_surface,
        }
    }

    fn source_description(&self) -> String {
        let description = if self.source_surface == DamageSourceSurface::Source {
            super::deals_damage::generic_source_description(&self.source_filter)
        } else {
            self.source_filter.description()
        };
        super::deals_damage::correct_damage_source_indefinite_article(description)
    }

    fn recipient_description(&self) -> String {
        if self.object_filter.union_is_one_or_more()
            && self.object_filter.union_connective()
                == crate::filter::ObjectFilterUnionConnective::AndOr
            && self.player_filter == PlayerFilter::Any
        {
            let mut object = self.object_filter.clone();
            object.set_union_one_or_more(false);
            let object_description = object.description();
            let object = strip_indefinite_article(&object_description);
            return format!("one or more {object} and/or players");
        }

        let object = damage_object_description(&self.object_filter);
        let player = self.player_filter.description();
        if self.player_first {
            format!("{player} or {}", strip_indefinite_article(&object))
        } else {
            format!("{object} or {}", strip_indefinite_article(&player))
        }
    }
}

impl TriggerMatcher for DealsExactDamageToObjectOrPlayerTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(damage) = event.downcast::<DamageEvent>() else {
            return false;
        };
        if damage.amount != self.amount {
            return false;
        }
        let Some(source) = ctx.game.object(damage.source) else {
            return false;
        };
        if !self
            .source_filter
            .matches(source, &ctx.filter_ctx, ctx.game)
        {
            return false;
        }
        match damage.target {
            DamageTarget::Object(target) => ctx.game.object(target).is_some_and(|object| {
                self.object_filter
                    .matches(object, &ctx.filter_ctx, ctx.game)
            }),
            DamageTarget::Player(player) => {
                self.player_filter.matches_player(player, &ctx.filter_ctx)
            }
        }
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Damage])
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} deals exactly {} damage to {}",
            self.source_description(),
            self.amount,
            self.recipient_description()
        )
    }
}

fn strip_indefinite_article(description: &str) -> &str {
    description
        .strip_prefix("a ")
        .or_else(|| description.strip_prefix("an "))
        .unwrap_or(description)
}

fn damage_object_description(filter: &ObjectFilter) -> String {
    let description = filter.description();
    if description.starts_with("a ")
        || description.starts_with("an ")
        || description.starts_with("the ")
        || description.starts_with("another ")
    {
        return description;
    }
    let article = if description
        .chars()
        .next()
        .is_some_and(|first| matches!(first.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {description}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_creature(game: &mut crate::GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn damage(source: ObjectId, target: DamageTarget, amount: u32) -> TriggerEvent {
        TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source,
                target,
                amount,
                false,
                crate::events::cause::EventCause::effect(),
            ),
            crate::provenance::ProvNodeId::default(),
        )
    }

    #[test]
    fn exact_amount_and_same_event_recipient_are_enforced() {
        let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let ghyrson = create_creature(&mut game, "Ghyrson", alice);
        let source = create_creature(&mut game, "Pinger", alice);
        let opposing_source = create_creature(&mut game, "Opposing Pinger", bob);
        let target = create_creature(&mut game, "Target", bob);
        let trigger = DealsExactDamageToObjectOrPlayerTrigger::new(
            ObjectFilter::default()
                .controlled_by(PlayerFilter::You)
                .other(),
            ObjectFilter::default(),
            PlayerFilter::Any,
            false,
            1,
            DamageSourceSurface::Source,
        );
        let ctx = TriggerContext::for_source(ghyrson, alice, &game);

        assert!(trigger.matches(&damage(source, DamageTarget::Object(target), 1), &ctx));
        assert!(trigger.matches(&damage(source, DamageTarget::Player(bob), 1), &ctx));
        assert!(!trigger.matches(&damage(source, DamageTarget::Object(target), 2), &ctx));
        assert!(!trigger.matches(&damage(opposing_source, DamageTarget::Player(bob), 1), &ctx));
        assert!(!trigger.matches(&damage(ghyrson, DamageTarget::Player(bob), 1), &ctx));
    }

    #[test]
    fn exact_amount_surface_is_not_inferred_from_a_generic_damage_trigger() {
        let trigger = DealsExactDamageToObjectOrPlayerTrigger::new(
            ObjectFilter::default()
                .controlled_by(PlayerFilter::You)
                .other(),
            ObjectFilter::default(),
            PlayerFilter::Any,
            false,
            1,
            DamageSourceSurface::Source,
        );
        assert_eq!(
            trigger.display(),
            "Whenever another source you control deals exactly 1 damage to a permanent or player"
        );
    }
}

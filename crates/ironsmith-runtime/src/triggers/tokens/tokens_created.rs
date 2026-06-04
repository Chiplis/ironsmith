//! "Whenever [player] creates [tokens]" trigger.

use crate::events::{CreateTokensEvent, EventKind};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct TokensCreatedTrigger {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub one_or_more: bool,
}

impl TokensCreatedTrigger {
    pub fn new(player: PlayerFilter, filter: ObjectFilter, one_or_more: bool) -> Self {
        Self {
            player,
            filter,
            one_or_more,
        }
    }
}

impl TriggerMatcher for TokensCreatedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreateTokens {
            return false;
        }
        let Some(created) = event.downcast::<CreateTokensEvent>() else {
            return false;
        };
        if created.count == 0
            || !self
                .player
                .matches_player(created.controller, &ctx.filter_ctx)
        {
            return false;
        }
        let Some(token) = &created.token else {
            return self.filter == ObjectFilter::default();
        };
        self.filter.matches(token, &ctx.filter_ctx, ctx.game)
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        if self.one_or_more {
            return 1;
        }
        event
            .downcast::<CreateTokensEvent>()
            .map_or(1, |created| created.count.max(1))
    }

    fn display(&self) -> String {
        let subject = match &self.player {
            PlayerFilter::You => "you".to_string(),
            PlayerFilter::Opponent => "an opponent".to_string(),
            PlayerFilter::Any => "a player".to_string(),
            other => other.description(),
        };
        let verb = if self.player == PlayerFilter::You {
            "create"
        } else {
            "creates"
        };
        let token = self.filter.description();
        let object_phrase = if self.one_or_more {
            format!("one or more {token}s")
        } else {
            format!("a {token}")
        };
        format!("Whenever {subject} {verb} {object_phrase}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardDefinitionBuilder;
    use crate::events::cause::EventCause;
    use crate::ids::{ObjectId, PlayerId};
    use crate::object::Object;
    use crate::provenance::ProvNodeId;
    use crate::types::{CardType, Subtype};

    #[test]
    fn matches_creature_token_created_by_controller_once_for_one_or_more() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let soldier = CardDefinitionBuilder::new(crate::CardId::new(), "Soldier")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Soldier])
            .build();
        let token = Object::from_token_definition(ObjectId::from_raw(100), &soldier, alice);
        let event = TriggerEvent::new_with_provenance(
            CreateTokensEvent::with_token_cause(alice, 3, token, EventCause::effect()),
            ProvNodeId::default(),
        );
        let trigger = TokensCreatedTrigger::new(
            PlayerFilter::You,
            ObjectFilter::creature().token(),
            true,
        );
        let ctx = TriggerContext::for_source(ObjectId::from_raw(1), alice, &game);

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(trigger.trigger_count(&event), 1);
    }

    #[test]
    fn rejects_noncreature_token_for_creature_token_filter() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let clue = CardDefinitionBuilder::new(crate::CardId::new(), "Clue")
            .token()
            .card_types(vec![CardType::Artifact])
            .build();
        let token = Object::from_token_definition(ObjectId::from_raw(100), &clue, alice);
        let event = TriggerEvent::new_with_provenance(
            CreateTokensEvent::with_token_cause(alice, 1, token, EventCause::effect()),
            ProvNodeId::default(),
        );
        let trigger = TokensCreatedTrigger::new(
            PlayerFilter::You,
            ObjectFilter::creature().token(),
            true,
        );
        let ctx = TriggerContext::for_source(ObjectId::from_raw(1), alice, &game);

        assert!(!trigger.matches(&event, &ctx));
    }
}

//! Token creation event implementation.

use std::any::Any;

use crate::events::cause::EventCause;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::object::Object;
use ironsmith_core::AdditionalTokenKind;

/// An event representing one effect creating one or more tokens for a player.
#[derive(Debug, Clone)]
pub struct CreateTokensEvent {
    /// Player under whose control the tokens would be created.
    pub controller: PlayerId,
    /// Number of tokens that would be created.
    pub count: u32,
    /// What caused the token creation.
    pub cause: EventCause,
    /// Characteristics of the token being created, when known before creation.
    pub token: Option<Object>,
    /// Separately defined tokens added by replacement effects.
    pub additional_tokens: Vec<(AdditionalTokenKind, u32)>,
}

impl CreateTokensEvent {
    pub fn with_cause(controller: PlayerId, count: u32, cause: EventCause) -> Self {
        Self {
            controller,
            count,
            cause,
            token: None,
            additional_tokens: Vec::new(),
        }
    }

    pub fn with_token_cause(
        controller: PlayerId,
        count: u32,
        token: Object,
        cause: EventCause,
    ) -> Self {
        Self {
            controller,
            count,
            cause,
            token: Some(token),
            additional_tokens: Vec::new(),
        }
    }

    /// Every token group doubled, including tokens an earlier replacement
    /// added (CR 616.1: each replacement applies to the modified event).
    pub fn doubled(&self) -> Self {
        self.scaled_groups(|_| true, |count| count.saturating_mul(2))
    }

    /// Total number of tokens this event would create, across the original
    /// token and every group added by an earlier replacement.
    pub fn total_count(&self) -> u32 {
        self.additional_tokens
            .iter()
            .fold(self.count, |total, (_, count)| total.saturating_add(*count))
    }

    /// Rewrite the count of each token group `matches` accepts. The original
    /// group is `None`; added groups pass their kind.
    pub fn scaled_groups(
        &self,
        matches: impl Fn(Option<AdditionalTokenKind>) -> bool,
        scale: impl Fn(u32) -> u32,
    ) -> Self {
        let mut next = self.clone();
        if matches(None) {
            next.count = scale(next.count);
        }
        for (kind, count) in &mut next.additional_tokens {
            if matches(Some(*kind)) {
                *count = scale(*count);
            }
        }
        next.additional_tokens.retain(|(_, count)| *count > 0);
        next
    }

    /// Rewrite the combined count of the token groups `matches` accepts
    /// ("those tokens plus an additional one", "that many minus one"). Tokens
    /// an earlier replacement added are part of the modified event (CR 616.1),
    /// so they count toward the total. Added tokens go to the first covered
    /// group; removed tokens come off the covered groups in order.
    pub fn adjusted_covered_total(
        &self,
        matches: impl Fn(Option<AdditionalTokenKind>) -> bool,
        adjust: impl Fn(u32) -> u32,
    ) -> Self {
        let mut next = self.clone();
        let covered_added = next
            .additional_tokens
            .iter()
            .enumerate()
            .filter(|(_, (kind, count))| *count > 0 && matches(Some(*kind)))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        // Only positive groups are part of the modified creation event. An
        // earlier replacement may remove the original group while leaving
        // added groups; an increase must use the first surviving covered group.
        // When the positive original template is unknown and no added group
        // matches, preserve the existing fallback to that original group.
        let covers_original = next.count > 0 && (matches(None) || covered_added.is_empty());
        if !covers_original && covered_added.is_empty() {
            return next;
        }
        let total = covered_added
            .iter()
            .fold(
                if covers_original { next.count } else { 0 },
                |total, index| total.saturating_add(next.additional_tokens[*index].1),
            );
        let target = adjust(total);
        if target > total {
            let extra = target - total;
            if covers_original {
                next.count = next.count.saturating_add(extra);
            } else {
                let count = &mut next.additional_tokens[covered_added[0]].1;
                *count = count.saturating_add(extra);
            }
        } else {
            let mut remove = total - target;
            if covers_original {
                let taken = remove.min(next.count);
                next.count -= taken;
                remove -= taken;
            }
            for index in covered_added {
                if remove == 0 {
                    break;
                }
                let count = &mut next.additional_tokens[index].1;
                let taken = remove.min(*count);
                *count -= taken;
                remove -= taken;
            }
        }
        next.additional_tokens.retain(|(_, count)| *count > 0);
        next
    }

    pub fn with_count(&self, count: u32) -> Self {
        Self {
            count,
            ..self.clone()
        }
    }

    pub fn with_additional_tokens(&self, token: AdditionalTokenKind, count: u32) -> Self {
        let mut next = self.clone();
        if count > 0 {
            next.additional_tokens.push((token, count));
        }
        next
    }
}

impl GameEventType for CreateTokensEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CreateTokens
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn display(&self) -> String {
        format!("Create {} token(s)", self.count)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Token definition for a token kind a replacement effect adds.
pub fn additional_token_definition(kind: AdditionalTokenKind) -> crate::cards::CardDefinition {
    match kind {
        AdditionalTokenKind::Treasure => crate::cards::tokens::treasure_token_definition(),
        AdditionalTokenKind::Food => crate::cards::tokens::food_token_definition(),
        AdditionalTokenKind::Clue => crate::cards::tokens::clue_token_definition(),
        AdditionalTokenKind::Squirrel => crate::cards::tokens::squirrel_token_definition(),
    }
}

/// The characteristics of an added token group, for matching later
/// replacements' token filters ("If you would create one or more Treasure
/// tokens" sees a Treasure an earlier replacement added).
pub fn additional_token_object(kind: AdditionalTokenKind, controller: PlayerId) -> Object {
    Object::from_token_definition(
        crate::ids::ObjectId::from_raw(0),
        &additional_token_definition(kind),
        controller,
    )
}

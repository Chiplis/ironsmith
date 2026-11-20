//! Token creation event implementation.

use std::any::Any;

use crate::events::cause::EventCause;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::object::Object;
use ironsmith_core::AdditionalTokenKind;
use crate::effects::ExecutionError;
use crate::effects::tokens::resources::token_count;

/// Stable keys within one immutable token-creation proposal. Definitions are
/// first-class groups, not a finite list of card-specific token kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenGroupKey { Original, Named(usize), Template(usize), Copy(usize) }

#[derive(Debug, Clone)]
pub struct TemplateTokenGroup {
    pub definition: crate::cards::CardDefinition,
    pub count: u32,
}

/// A distinct copiable object in the same token-creation instruction.
#[derive(Debug, Clone)]
pub struct CopyTokenGroup {
    pub token: Object,
    pub count: u32,
    pub initial_counters: Vec<(crate::object::CounterType, u32)>,
}

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
    /// Arbitrary templates appended or substituted by creation replacements.
    pub additional_templates: Vec<TemplateTokenGroup>,
    pub additional_copies: Vec<CopyTokenGroup>,
}

impl CreateTokensEvent {
    pub fn with_cause(controller: PlayerId, count: u32, cause: EventCause) -> Self {
        Self {
            controller,
            count,
            cause,
            token: None,
            additional_tokens: Vec::new(),
            additional_templates: Vec::new(),
            additional_copies: Vec::new(),
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
            additional_templates: Vec::new(),
            additional_copies: Vec::new(),
        }
    }

    /// Every token group doubled, including tokens an earlier replacement
    /// added (CR 616.1: each replacement applies to the modified event).
    pub fn doubled(&self) -> Result<Self, ExecutionError> {
        self.scaled_groups(|_| true, |count| u128::from(count).saturating_mul(2))
    }

    /// Total number of tokens this event would create, across the original
    /// token and every group added by an earlier replacement.
    pub fn total_count(&self) -> u128 {
        // Each count fits u32; the sum saturates rather than wrapping.
        self.group_keys().into_iter().fold(0u128, |total, key| {
            total.saturating_add(u128::from(self.group_count(key)))
        })
    }

    /// This proposal limited to its first `granted` tokens, in group order.
    /// Creation past the token budget creates what fits and then stops.
    pub(crate) fn truncated_to(&self, granted: u128) -> Self {
        let mut next = self.clone();
        let mut remaining = granted;
        for key in self.group_keys() {
            let count = next.group_count_mut(key);
            let kept = token_count(remaining.min(u128::from(*count)));
            *count = kept;
            remaining = remaining.saturating_sub(u128::from(kept));
        }
        next.additional_tokens.retain(|(_, count)| *count > 0);
        next.additional_templates.retain(|group| group.count > 0);
        next.additional_copies.retain(|group| group.count > 0);
        next
    }

    pub fn group_keys(&self) -> Vec<TokenGroupKey> {
        std::iter::once(TokenGroupKey::Original)
            .chain((0..self.additional_tokens.len()).map(TokenGroupKey::Named))
            .chain((0..self.additional_templates.len()).map(TokenGroupKey::Template))
            .chain((0..self.additional_copies.len()).map(TokenGroupKey::Copy))
            .collect()
    }
    pub fn group_count(&self, key: TokenGroupKey) -> u32 {
        match key {
            TokenGroupKey::Original => self.count,
            TokenGroupKey::Named(index) => self.additional_tokens[index].1,
            TokenGroupKey::Template(index) => self.additional_templates[index].count,
            TokenGroupKey::Copy(index) => self.additional_copies[index].count,
        }
    }
    pub(crate) fn group_count_mut(&mut self, key: TokenGroupKey) -> &mut u32 {
        match key {
            TokenGroupKey::Original => &mut self.count,
            TokenGroupKey::Named(index) => &mut self.additional_tokens[index].1,
            TokenGroupKey::Template(index) => &mut self.additional_templates[index].count,
            TokenGroupKey::Copy(index) => &mut self.additional_copies[index].count,
        }
    }
    pub fn group_object(&self, key: TokenGroupKey) -> Option<Object> {
        match key {
            TokenGroupKey::Original => self.token.clone(),
            TokenGroupKey::Copy(index) => Some(self.additional_copies[index].token.clone()),
            TokenGroupKey::Named(index) => Some(additional_token_object(self.additional_tokens[index].0, self.controller)),
            TokenGroupKey::Template(index) => Some(Object::from_token_definition(
                crate::ids::ObjectId::from_raw(0), &self.additional_templates[index].definition, self.controller,
            )),
        }
    }
    pub fn matching_count(&self, matches: impl Fn(TokenGroupKey) -> bool) -> u128 {
        self.group_keys().into_iter().filter(|key| matches(*key))
            .fold(0u128, |total, key| total.saturating_add(u128::from(self.group_count(key))))
    }
    pub fn scaled_token_groups(&self, matches: impl Fn(TokenGroupKey) -> bool, scale: impl Fn(u32) -> u128) -> Result<Self, ExecutionError> {
        let mut next = self.clone();
        for key in self.group_keys() {
            if matches(key) { *next.group_count_mut(key) = token_count(scale(self.group_count(key))); }
        }
        next.additional_tokens.retain(|(_, count)| *count > 0);
        next.additional_templates.retain(|group| group.count > 0);
        next.additional_copies.retain(|group| group.count > 0);
        Ok(next)
    }
    pub fn adjusted_token_total(&self, matches: impl Fn(TokenGroupKey) -> bool, adjust: impl Fn(u32) -> u128) -> Result<Self, ExecutionError> {
        let mut keys: Vec<_> = self.group_keys().into_iter().filter(|key| self.group_count(*key) > 0 && matches(*key)).collect();
        // Preserve the original unknown-prototype fallback of the legacy API.
        if keys.is_empty() && self.count > 0 && self.token.is_none() { keys.push(TokenGroupKey::Original); }
        let Some(first) = keys.first().copied() else { return Ok(self.clone()); };
        // The covered total saturates for the adjustment, but the change is
        // applied against the exact total so "none instead" removes every group.
        let exact = keys.iter().fold(0u128, |total, key| total.saturating_add(u128::from(self.group_count(*key))));
        let desired = adjust(token_count(exact));
        let mut next = self.clone();
        if desired > exact {
            let count = next.group_count_mut(first);
            *count = token_count(u128::from(*count).saturating_add(desired.saturating_sub(exact)));
        } else {
            let mut remove = exact.saturating_sub(desired);
            for key in keys {
                let count = next.group_count_mut(key);
                let removed = token_count(remove.min(u128::from(*count)));
                *count = count.saturating_sub(removed);
                remove = remove.saturating_sub(u128::from(removed));
            }
        }
        next.additional_tokens.retain(|(_, count)| *count > 0);
        next.additional_templates.retain(|group| group.count > 0);
        next.additional_copies.retain(|group| group.count > 0);
        Ok(next)
    }
    /// Compatibility adapter for predefined-kind callers. Native replacement
    /// matching uses the complete group-key API, including arbitrary templates.
    pub fn scaled_groups(&self, matches: impl Fn(Option<AdditionalTokenKind>) -> bool, scale: impl Fn(u32) -> u128) -> Result<Self, ExecutionError> {
        self.scaled_token_groups(|key| matches(match key {
            TokenGroupKey::Named(index) => Some(self.additional_tokens[index].0),
            _ => None,
        }), scale)
    }
    pub fn adjusted_covered_total(&self, matches: impl Fn(Option<AdditionalTokenKind>) -> bool, adjust: impl Fn(u32) -> u128) -> Result<Self, ExecutionError> {
        self.adjusted_token_total(|key| matches(match key {
            TokenGroupKey::Named(index) => Some(self.additional_tokens[index].0),
            _ => None,
        }), adjust)
    }
    pub fn with_template(&self, definition: crate::cards::CardDefinition, count: u32) -> Result<Self, ExecutionError> {
        let mut next = self.clone();
        if count > 0 { next.additional_templates.push(TemplateTokenGroup { definition, count }); }
        Ok(next)
    }

    pub fn with_count(&self, count: u32) -> Result<Self, ExecutionError> {
        Ok(Self { count, ..self.clone() })
    }

    pub fn with_additional_tokens(&self, token: AdditionalTokenKind, count: u32) -> Result<Self, ExecutionError> {
        let mut next = self.clone();
        if count > 0 {
            next.additional_tokens.push((token, count));
        }
        Ok(next)
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
        format!("Create {} token(s)", self.total_count())
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

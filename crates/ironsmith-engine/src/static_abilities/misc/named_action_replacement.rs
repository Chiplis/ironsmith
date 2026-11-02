//! Replacement of an independently declared named operation program.
use crate::static_abilities::{StaticAbilityId, StaticAbilityKind};
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::{Effect, ObjectId, PlayerId};

#[derive(Debug, Clone, PartialEq)]
pub struct NamedActionReplacement {
    pub label: String,
    pub source_filter: ObjectFilter,
    pub performer_filter: Option<PlayerFilter>,
    pub object_requirement: Option<ironsmith_core::filter_model::NamedActionObjectRequirement>,
    pub replacement_effects: Vec<Effect>,
    pub optional: bool,
    pub display: String,
}
impl StaticAbilityKind for NamedActionReplacement {
    fn id(&self) -> StaticAbilityId { StaticAbilityId::KeywordActionReplacement }
    fn display(&self) -> String { self.display.clone() }
    fn generate_replacement_effect(&self, source: ObjectId, controller: PlayerId) -> Option<ReplacementEffect> {
        let replacement = ReplacementEffect::with_matcher(source, controller,
            crate::events::other::WouldNamedActionMatcher {
                label: self.label.clone(), source_filter: self.source_filter.clone(), performer_filter: self.performer_filter.clone(), object_requirement:self.object_requirement.clone(),
            }, ReplacementAction::Instead(self.replacement_effects.clone()));
        Some(if self.optional { replacement.optional() } else { replacement })
    }
}

/// An operation plan modification that preserves the operation's original menu.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedActionFirstThenOtherMode {
    pub label: String,
    pub player: PlayerFilter,
    pub first_mode: u32,
    pub display: String,
}
impl StaticAbilityKind for NamedActionFirstThenOtherMode {
    fn id(&self) -> StaticAbilityId { StaticAbilityId::KeywordActionReplacement }
    fn display(&self) -> String { self.display.clone() }
    fn generate_replacement_effect(&self, source: ObjectId, controller: PlayerId) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(source, controller,
            crate::events::other::WouldNamedActionMatcher {
                label:self.label.clone(), source_filter:ObjectFilter::default(), performer_filter:Some(self.player.clone()), object_requirement:None,
            }, ReplacementAction::Modify(crate::replacement::EventModification::NamedActionFirstThenOtherMode { first_mode:self.first_mode })))
    }
}

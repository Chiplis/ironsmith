//! Definition substitution preserves creation actor, count and destination.
use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
use crate::static_abilities::{StaticAbilityId, StaticAbilityKind};
use crate::target::PlayerFilter;
use crate::{ObjectId, PlayerId};
#[derive(Debug, Clone, PartialEq)]
pub struct CardCreationSelectionReplacement {
    pub operation_label: Option<String>,
    pub player: PlayerFilter,
    pub replacement: crate::effect::Effect,
    pub display: String,
}
impl StaticAbilityKind for CardCreationSelectionReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CardCreationSelectionReplacement
    }
    fn display(&self) -> String {
        self.display.clone()
    }
    fn canonical_model(&self) -> Option<crate::static_abilities::CompiledStaticAbility> {
        Some(crate::static_abilities::CompiledStaticAbility::card_creation_selection_replacement(
            self.operation_label.clone(), self.player.clone(), self.replacement.clone(), self.display.clone(),
        ))
    }
    fn supplied_replacement_definition(&self) -> Option<crate::effect::Effect> {
        Some(self.replacement.clone())
    }
    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        let mut template = &self.replacement;
        while let Some(child) = template.transparent_child_effect() {
            template = child;
        }
        let replacement = template.downcast_ref::<crate::effects::CreateCardsEffect>()?;
        let ironsmith_core::CardCreationSelection::Named(name) = &replacement.selection else {
            return None;
        };
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::WouldCreateCardsMatcher {
                operation_label: self.operation_label.clone(),
                player: Some(self.player.clone()),
            },
            ReplacementAction::Modify(EventModification::ReplaceCardDefinition(
                name.clone(),
            )),
        ))
    }
}

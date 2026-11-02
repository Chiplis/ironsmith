//! Source-definition replacement preserves the original copy operation.
use crate::static_abilities::{StaticAbilityId,StaticAbilityKind};
use crate::replacement::{EventModification,ReplacementAction,ReplacementEffect};
use crate::{ObjectId,PlayerId};
#[derive(Debug,Clone,PartialEq)]
pub struct CopySourceSelectionReplacement {pub player:crate::target::PlayerFilter,pub replacement:crate::effect::Effect,pub display:String}
impl StaticAbilityKind for CopySourceSelectionReplacement {
 fn id(&self)->StaticAbilityId{StaticAbilityId::CopySourceSelectionReplacement}
 fn display(&self)->String{self.display.clone()}
 fn canonical_model(&self)->Option<crate::static_abilities::CompiledStaticAbility>{
  Some(crate::static_abilities::CompiledStaticAbility::copy_source_selection_replacement(self.player.clone(),self.replacement.clone(),self.display.clone()))
 }
 fn supplied_replacement_definition(&self)->Option<crate::effect::Effect>{Some(self.replacement.clone())}
 fn generate_replacement_effect(&self,source:ObjectId,controller:PlayerId)->Option<ReplacementEffect>{
  let mut operand=&self.replacement;while let Some(child)=operand.transparent_child_effect(){operand=child;}
  let copy=operand.downcast_ref::<crate::effects::CopyFromDefinitionEffect>()?;
  Some(ReplacementEffect::with_matcher(source,controller,crate::events::WouldSelectCopySourceMatcher{player:self.player.clone()},ReplacementAction::Modify(EventModification::ReplaceCardDefinition(copy.definition.card.name.clone()))))
 }
}

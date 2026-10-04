//! Exact native replacement predicate captures. This is not a checkpoint wire codec.
//! Snapshot mapping is mandatory for the owning canonical codec: ordinary native
//! snapshot serde intentionally skips abilities and must not be used here.
#[derive(Debug, Clone)]
pub enum ReplacementMatcherDescriptor<S> {
DamageToSpecificTargetMatcher { target: crate::events::DamageTarget },
DamageSourceToSpecificTargetMatcher { source: crate::events::damage::matchers::DamageSourceConstraint, target: crate::events::DamageTarget },
DamageToExactTargetMatcher { target: crate::events::DamageTarget },
ManaProducedBySourceMatcher { source_filter: crate::target::ObjectFilter, required_provenance: Option<crate::events::mana::ManaProductionProvenance> },
DamageToPlayerMatcher { player_filter: crate::target::PlayerFilter },
PreventableDamageToPlayerMatcher { player_filter: crate::target::PlayerFilter },
DamageToObjectMatcher { filter: crate::target::ObjectFilter },
DamageToPlayerOrObjectMatcher { player_filter: crate::target::PlayerFilter, object_filter: crate::target::ObjectFilter },
CombatDamageMatcher,
PreventableCombatDamageToObjectMatcher { filter: crate::target::ObjectFilter },
PreventableNoncombatDamageToObjectMatcher { filter: crate::target::ObjectFilter },
NoncombatDamageMatcher,
DamageFromSourceMatcher { filter: crate::target::ObjectFilter },
DamageFromSourceToPlayerMatcher { source_filter: crate::target::ObjectFilter, player_filter: crate::target::PlayerFilter },
DamageFromSourceToObjectMatcher { source_filter: crate::target::ObjectFilter, target_filter: crate::target::ObjectFilter, combat_only: Option<bool>, preventable_only: bool },
DamageFromSelfMatcher,
DamageToOrFromSelfMatcher,
DamageFromSelfCombatMatcher,
PreventableDamageConstraintMatcher { source: crate::events::damage::matchers::DamageSourceConstraint, target: crate::events::damage::matchers::DamageTargetConstraint },
DamageToSelfMatcher,
DamageToAttachedObjectMatcher,
DamageToSelfConstraintMatcher { source_filter: Option<crate::target::ObjectFilter>, combat_only: Option<bool> },
DamageToSelfCombatMatcher,
DamageToOtherCreatureYouControlMatcher { noncombat_only: bool },
DamageToSelfFromSourceFilterMatcher { source_filter: crate::target::ObjectFilter, combat_only: bool, source_relation: ironsmith_core::StaticDamageSourceRelation },
WouldCreateTokensUnderControlMatcher { controller_filter: crate::target::PlayerFilter, cause_filter: crate::events::cause::CauseFilter, token_filter: Option<crate::target::ObjectFilter> },
WouldPutCountersMatcher { filter: crate::target::ObjectFilter, counter_type: Option<crate::object::CounterType>, cause_filter: crate::events::cause::CauseFilter },
WouldRemoveCountersMatcher { filter: crate::target::ObjectFilter, counter_type: Option<crate::object::CounterType> },
WouldBecomeTappedMatcher { filter: crate::target::ObjectFilter },
WouldBecomeUntappedMatcher { filter: crate::target::ObjectFilter },
WouldBeDestroyedMatcher { filter: crate::target::ObjectFilter },
ThisWouldBeDestroyedMatcher,
AttachedPermanentWouldBeDestroyedMatcher { aura: crate::ids::ObjectId },
WouldBeSacrificedMatcher { filter: crate::target::ObjectFilter },
RegenerationShieldMatcher { protected: crate::ids::ObjectId },
WouldGainLifeMatcher { player_filter: crate::target::PlayerFilter },
WouldLoseLifeMatcher { player_filter: crate::target::PlayerFilter },
WouldKeywordActionMatcher { action: crate::events::KeywordActionKind, source_filter: crate::target::ObjectFilter, performer_filter: Option<crate::target::PlayerFilter> },
WouldLoseGameMatcher,
WouldDrawCardMatcher { player_filter: crate::target::PlayerFilter },
WouldDrawCardWhileLibraryEmptyMatcher { player_filter: crate::target::PlayerFilter },
WouldDrawFirstCardMatcher { player_filter: crate::target::PlayerFilter },
WouldDiscardMatcher { player_filter: crate::target::PlayerFilter, cause_filter: crate::events::cause::CauseFilter, card_filter: Option<crate::target::ObjectFilter>, destination: Option<crate::zone::Zone> },
WouldEnterBattlefieldMatcher { filter: crate::target::ObjectFilter, stable_id: Option<crate::ids::StableId> },
ThisWouldEnterBattlefieldMatcher,
WouldDieMatcher { filter: crate::target::ObjectFilter },
WouldDieDamagedBySourceThisTurnMatcher { filter: crate::target::ObjectFilter, damaged_by: ironsmith_core::DamagedBySource, victims: Option<Vec<crate::ids::StableId>> },
WouldDieDamagedByFilteredSourceThisTurnMatcher { victim_filter: crate::target::ObjectFilter, damager_filter: crate::target::ObjectFilter },
ThisWouldDieMatcher,
WouldGoToGraveyardMatcher { filter: crate::target::ObjectFilter },
WouldChangeZoneMatcher { filter: crate::target::ObjectFilter, from_zone: Option<crate::zone::Zone>, to_zone: Option<crate::zone::Zone>, cause_filter: Option<crate::events::cause::CauseFilter>, require_cause_source_match: bool, frozen_tagged_objects: std::collections::HashMap<crate::tag::TagKey, Vec<S>> },
WouldBeExiledMatcher { filter: crate::target::ObjectFilter },
ThisWouldGoToGraveyardMatcher,
WouldGoToHandMatcher { player_filter: crate::target::PlayerFilter },
WouldLeaveBattlefieldMatcher { filter: crate::target::ObjectFilter },
ThisWouldEnterTappedUnlessControlTwoOrMoreOtherLandsMatcher,
ThisWouldEnterTappedUnlessControlTwoOrFewerOtherLandsMatcher,
ThisWouldEnterTappedUnlessControlTwoOrMoreBasicLandsMatcher,
ThisWouldEnterTappedUnlessAPlayerHas13OrLessLifeMatcher,
ThisWouldEnterTappedUnlessTwoOrMoreOpponentsMatcher,
ThisWouldEnterTappedUnlessConditionMatcher { condition: crate::effect::Condition, display: String },
ThisWouldEnterWithBloodthirstMatcher,
ThisWouldEnterWithCountersIfConditionMatcher { condition: crate::effect::Condition, condition_display: String },
PreventableCombatDamageToOrByObjectMatcher { to: crate::events::damage::matchers::PreventableCombatDamageToObjectMatcher, by: crate::events::damage::matchers::DamageFromSourceMatcher },
PreventableAnyDamageToObjectMatcher { combat: crate::events::damage::matchers::PreventableCombatDamageToObjectMatcher, noncombat: crate::events::damage::matchers::PreventableNoncombatDamageToObjectMatcher },
ConditionalWouldEnterBattlefieldMatcher { enter_matcher: crate::events::zones::matchers::WouldEnterBattlefieldMatcher, condition: Option<crate::ConditionExpr> },
ChosenTypeDamageSourceMatcher { ability_source: crate::ids::ObjectId },
DamageAmountReplacementMatcher { source_filter: crate::target::ObjectFilter, target_player_filter: Option<crate::target::PlayerFilter>, target_object_filter: Option<crate::target::ObjectFilter>, condition: Option<crate::ConditionExpr>, combat_only: bool, noncombat_only: bool, amount_less_than: Option<crate::effect::Value> },
WouldPutCountersOrEnterWithCountersMatcher { ability_source: crate::ids::ObjectId, controller: crate::ids::PlayerId, filter: crate::target::ObjectFilter, player_filter: Option<crate::target::PlayerFilter>, counter_type: Option<crate::object::CounterType>, actor: Option<crate::target::PlayerFilter>, includes_permanents: bool, effect_only: bool },
DredgeDrawMatcher { amount: u32 },
ConditionalWouldDrawCardMatcher { condition: crate::effect::Condition, display: String },
WouldDrawInstructionMatcher { condition: Option<crate::effect::Condition>, except_first_of_draw_step: bool, per_instruction: bool, display: String },
WouldDrawByPlayerMatcher { drawer: crate::target::PlayerFilter, except_first_of_draw_step: bool, display: String },
WouldGoToGraveyardFromAnywhereMatcher { filter: crate::target::ObjectFilter, exclude_cycled: bool },
WouldEnterFromZoneMatcher { enter_matcher: crate::events::zones::matchers::WouldEnterBattlefieldMatcher, not_cast: bool },
TappedForMinimumManaMatcher { inner: crate::events::mana::matchers::ManaProducedBySourceMatcher, minimum_amount: u32 },
ConditionalWouldChangeLifeMatcher { player: crate::target::PlayerFilter, loss: bool, condition: Option<crate::effect::Condition>, display: String },
}
pub type NativeReplacementMatcherDescriptor = ReplacementMatcherDescriptor<crate::snapshot::ObjectSnapshot>;
impl NativeReplacementMatcherDescriptor {
    /// Select only frozen roots read by the predicate before invoking a
    /// perspective history policy. This does not authorize disclosure.
    /// Full checkpoint encoding must retain the complete native descriptor.
    pub fn retain_predicate_history(mut self) -> Self {
        use ironsmith_core::tag::TagKeyWalk;
        if let Self::WouldChangeZoneMatcher { filter, frozen_tagged_objects, .. } = &mut self {
            let mut tags = std::collections::BTreeSet::new();
            filter.for_each_tag_key(&mut |tag| { tags.insert(tag.clone()); });
            let mut objects = std::collections::BTreeSet::new();
            filter.for_each_object_ref(&mut |reference| {
                if let ironsmith_core::filter_model::ObjectRef::Specific(id) = reference {
                    objects.insert(*id);
                }
            });
            frozen_tagged_objects.retain(|tag, snapshots| {
                if tags.contains(tag) { return true; }
                // Player owner/controller references search all captured roots
                // by object ID even when no tag key occurs in the predicate.
                snapshots.retain(|snapshot| objects.contains(&snapshot.object_id));
                !snapshots.is_empty()
            });
        }
        self
    }
}
impl<S> ReplacementMatcherDescriptor<S> {
    pub fn try_map_snapshots<T, E>(self, mut map:impl FnMut(S)->Result<T,E>) -> Result<ReplacementMatcherDescriptor<T>, E> {
        Ok(match self {
Self::DamageToSpecificTargetMatcher { target } => ReplacementMatcherDescriptor::DamageToSpecificTargetMatcher { target: target },
Self::DamageSourceToSpecificTargetMatcher { source, target } => ReplacementMatcherDescriptor::DamageSourceToSpecificTargetMatcher { source: source, target: target },
Self::DamageToExactTargetMatcher { target } => ReplacementMatcherDescriptor::DamageToExactTargetMatcher { target: target },
Self::ManaProducedBySourceMatcher { source_filter, required_provenance } => ReplacementMatcherDescriptor::ManaProducedBySourceMatcher { source_filter: source_filter, required_provenance: required_provenance },
Self::DamageToPlayerMatcher { player_filter } => ReplacementMatcherDescriptor::DamageToPlayerMatcher { player_filter: player_filter },
Self::PreventableDamageToPlayerMatcher { player_filter } => ReplacementMatcherDescriptor::PreventableDamageToPlayerMatcher { player_filter: player_filter },
Self::DamageToObjectMatcher { filter } => ReplacementMatcherDescriptor::DamageToObjectMatcher { filter: filter },
Self::DamageToPlayerOrObjectMatcher { player_filter, object_filter } => ReplacementMatcherDescriptor::DamageToPlayerOrObjectMatcher { player_filter: player_filter, object_filter: object_filter },
Self::CombatDamageMatcher => ReplacementMatcherDescriptor::CombatDamageMatcher,
Self::PreventableCombatDamageToObjectMatcher { filter } => ReplacementMatcherDescriptor::PreventableCombatDamageToObjectMatcher { filter: filter },
Self::PreventableNoncombatDamageToObjectMatcher { filter } => ReplacementMatcherDescriptor::PreventableNoncombatDamageToObjectMatcher { filter: filter },
Self::NoncombatDamageMatcher => ReplacementMatcherDescriptor::NoncombatDamageMatcher,
Self::DamageFromSourceMatcher { filter } => ReplacementMatcherDescriptor::DamageFromSourceMatcher { filter: filter },
Self::DamageFromSourceToPlayerMatcher { source_filter, player_filter } => ReplacementMatcherDescriptor::DamageFromSourceToPlayerMatcher { source_filter: source_filter, player_filter: player_filter },
Self::DamageFromSourceToObjectMatcher { source_filter, target_filter, combat_only, preventable_only } => ReplacementMatcherDescriptor::DamageFromSourceToObjectMatcher { source_filter: source_filter, target_filter: target_filter, combat_only: combat_only, preventable_only: preventable_only },
Self::DamageFromSelfMatcher => ReplacementMatcherDescriptor::DamageFromSelfMatcher,
Self::DamageToOrFromSelfMatcher => ReplacementMatcherDescriptor::DamageToOrFromSelfMatcher,
Self::DamageFromSelfCombatMatcher => ReplacementMatcherDescriptor::DamageFromSelfCombatMatcher,
Self::PreventableDamageConstraintMatcher { source, target } => ReplacementMatcherDescriptor::PreventableDamageConstraintMatcher { source: source, target: target },
Self::DamageToSelfMatcher => ReplacementMatcherDescriptor::DamageToSelfMatcher,
Self::DamageToAttachedObjectMatcher => ReplacementMatcherDescriptor::DamageToAttachedObjectMatcher,
Self::DamageToSelfConstraintMatcher { source_filter, combat_only } => ReplacementMatcherDescriptor::DamageToSelfConstraintMatcher { source_filter: source_filter, combat_only: combat_only },
Self::DamageToSelfCombatMatcher => ReplacementMatcherDescriptor::DamageToSelfCombatMatcher,
Self::DamageToOtherCreatureYouControlMatcher { noncombat_only } => ReplacementMatcherDescriptor::DamageToOtherCreatureYouControlMatcher { noncombat_only: noncombat_only },
Self::DamageToSelfFromSourceFilterMatcher { source_filter, combat_only, source_relation } => ReplacementMatcherDescriptor::DamageToSelfFromSourceFilterMatcher { source_filter: source_filter, combat_only: combat_only, source_relation: source_relation },
Self::WouldCreateTokensUnderControlMatcher { controller_filter, cause_filter, token_filter } => ReplacementMatcherDescriptor::WouldCreateTokensUnderControlMatcher { controller_filter: controller_filter, cause_filter: cause_filter, token_filter: token_filter },
Self::WouldPutCountersMatcher { filter, counter_type, cause_filter } => ReplacementMatcherDescriptor::WouldPutCountersMatcher { filter: filter, counter_type: counter_type, cause_filter: cause_filter },
Self::WouldRemoveCountersMatcher { filter, counter_type } => ReplacementMatcherDescriptor::WouldRemoveCountersMatcher { filter: filter, counter_type: counter_type },
Self::WouldBecomeTappedMatcher { filter } => ReplacementMatcherDescriptor::WouldBecomeTappedMatcher { filter: filter },
Self::WouldBecomeUntappedMatcher { filter } => ReplacementMatcherDescriptor::WouldBecomeUntappedMatcher { filter: filter },
Self::WouldBeDestroyedMatcher { filter } => ReplacementMatcherDescriptor::WouldBeDestroyedMatcher { filter: filter },
Self::ThisWouldBeDestroyedMatcher => ReplacementMatcherDescriptor::ThisWouldBeDestroyedMatcher,
Self::AttachedPermanentWouldBeDestroyedMatcher { aura } => ReplacementMatcherDescriptor::AttachedPermanentWouldBeDestroyedMatcher { aura: aura },
Self::WouldBeSacrificedMatcher { filter } => ReplacementMatcherDescriptor::WouldBeSacrificedMatcher { filter: filter },
Self::RegenerationShieldMatcher { protected } => ReplacementMatcherDescriptor::RegenerationShieldMatcher { protected: protected },
Self::WouldGainLifeMatcher { player_filter } => ReplacementMatcherDescriptor::WouldGainLifeMatcher { player_filter: player_filter },
Self::WouldLoseLifeMatcher { player_filter } => ReplacementMatcherDescriptor::WouldLoseLifeMatcher { player_filter: player_filter },
Self::WouldKeywordActionMatcher { action, source_filter, performer_filter } => ReplacementMatcherDescriptor::WouldKeywordActionMatcher { action: action, source_filter: source_filter, performer_filter: performer_filter },
Self::WouldLoseGameMatcher => ReplacementMatcherDescriptor::WouldLoseGameMatcher,
Self::WouldDrawCardMatcher { player_filter } => ReplacementMatcherDescriptor::WouldDrawCardMatcher { player_filter: player_filter },
Self::WouldDrawCardWhileLibraryEmptyMatcher { player_filter } => ReplacementMatcherDescriptor::WouldDrawCardWhileLibraryEmptyMatcher { player_filter: player_filter },
Self::WouldDrawFirstCardMatcher { player_filter } => ReplacementMatcherDescriptor::WouldDrawFirstCardMatcher { player_filter: player_filter },
Self::WouldDiscardMatcher { player_filter, cause_filter, card_filter, destination } => ReplacementMatcherDescriptor::WouldDiscardMatcher { player_filter: player_filter, cause_filter: cause_filter, card_filter: card_filter, destination: destination },
Self::WouldEnterBattlefieldMatcher { filter, stable_id } => ReplacementMatcherDescriptor::WouldEnterBattlefieldMatcher { filter: filter, stable_id: stable_id },
Self::ThisWouldEnterBattlefieldMatcher => ReplacementMatcherDescriptor::ThisWouldEnterBattlefieldMatcher,
Self::WouldDieMatcher { filter } => ReplacementMatcherDescriptor::WouldDieMatcher { filter: filter },
Self::WouldDieDamagedBySourceThisTurnMatcher { filter, damaged_by, victims } => ReplacementMatcherDescriptor::WouldDieDamagedBySourceThisTurnMatcher { filter: filter, damaged_by: damaged_by, victims: victims },
Self::WouldDieDamagedByFilteredSourceThisTurnMatcher { victim_filter, damager_filter } => ReplacementMatcherDescriptor::WouldDieDamagedByFilteredSourceThisTurnMatcher { victim_filter: victim_filter, damager_filter: damager_filter },
Self::ThisWouldDieMatcher => ReplacementMatcherDescriptor::ThisWouldDieMatcher,
Self::WouldGoToGraveyardMatcher { filter } => ReplacementMatcherDescriptor::WouldGoToGraveyardMatcher { filter: filter },
Self::WouldChangeZoneMatcher { filter, from_zone, to_zone, cause_filter, require_cause_source_match, frozen_tagged_objects } => {
    // Mapping may disclose history or stop on a policy error. Visit tag groups
    // deterministically while preserving the capture order within each group.
    let mut tagged_objects: Vec<_> = frozen_tagged_objects.into_iter().collect();
    tagged_objects.sort_by(|left, right| left.0.cmp(&right.0));
    ReplacementMatcherDescriptor::WouldChangeZoneMatcher {
        filter, from_zone, to_zone, cause_filter, require_cause_source_match,
        frozen_tagged_objects: tagged_objects.into_iter().map(|(tag, snapshots)| {
            Ok((tag, snapshots.into_iter().map(&mut map).collect::<Result<Vec<_>, E>>()?))
        }).collect::<Result<_, E>>()?,
    }
},
Self::WouldBeExiledMatcher { filter } => ReplacementMatcherDescriptor::WouldBeExiledMatcher { filter: filter },
Self::ThisWouldGoToGraveyardMatcher => ReplacementMatcherDescriptor::ThisWouldGoToGraveyardMatcher,
Self::WouldGoToHandMatcher { player_filter } => ReplacementMatcherDescriptor::WouldGoToHandMatcher { player_filter: player_filter },
Self::WouldLeaveBattlefieldMatcher { filter } => ReplacementMatcherDescriptor::WouldLeaveBattlefieldMatcher { filter: filter },
Self::ThisWouldEnterTappedUnlessControlTwoOrMoreOtherLandsMatcher => ReplacementMatcherDescriptor::ThisWouldEnterTappedUnlessControlTwoOrMoreOtherLandsMatcher,
Self::ThisWouldEnterTappedUnlessControlTwoOrFewerOtherLandsMatcher => ReplacementMatcherDescriptor::ThisWouldEnterTappedUnlessControlTwoOrFewerOtherLandsMatcher,
Self::ThisWouldEnterTappedUnlessControlTwoOrMoreBasicLandsMatcher => ReplacementMatcherDescriptor::ThisWouldEnterTappedUnlessControlTwoOrMoreBasicLandsMatcher,
Self::ThisWouldEnterTappedUnlessAPlayerHas13OrLessLifeMatcher => ReplacementMatcherDescriptor::ThisWouldEnterTappedUnlessAPlayerHas13OrLessLifeMatcher,
Self::ThisWouldEnterTappedUnlessTwoOrMoreOpponentsMatcher => ReplacementMatcherDescriptor::ThisWouldEnterTappedUnlessTwoOrMoreOpponentsMatcher,
Self::ThisWouldEnterTappedUnlessConditionMatcher { condition, display } => ReplacementMatcherDescriptor::ThisWouldEnterTappedUnlessConditionMatcher { condition: condition, display: display },
Self::ThisWouldEnterWithBloodthirstMatcher => ReplacementMatcherDescriptor::ThisWouldEnterWithBloodthirstMatcher,
Self::ThisWouldEnterWithCountersIfConditionMatcher { condition, condition_display } => ReplacementMatcherDescriptor::ThisWouldEnterWithCountersIfConditionMatcher { condition: condition, condition_display: condition_display },
Self::PreventableCombatDamageToOrByObjectMatcher { to, by } => ReplacementMatcherDescriptor::PreventableCombatDamageToOrByObjectMatcher { to: to, by: by },
Self::PreventableAnyDamageToObjectMatcher { combat, noncombat } => ReplacementMatcherDescriptor::PreventableAnyDamageToObjectMatcher { combat: combat, noncombat: noncombat },
Self::ConditionalWouldEnterBattlefieldMatcher { enter_matcher, condition } => ReplacementMatcherDescriptor::ConditionalWouldEnterBattlefieldMatcher { enter_matcher: enter_matcher, condition: condition },
Self::ChosenTypeDamageSourceMatcher { ability_source } => ReplacementMatcherDescriptor::ChosenTypeDamageSourceMatcher { ability_source: ability_source },
Self::DamageAmountReplacementMatcher { source_filter, target_player_filter, target_object_filter, condition, combat_only, noncombat_only, amount_less_than } => ReplacementMatcherDescriptor::DamageAmountReplacementMatcher { source_filter: source_filter, target_player_filter: target_player_filter, target_object_filter: target_object_filter, condition: condition, combat_only: combat_only, noncombat_only: noncombat_only, amount_less_than: amount_less_than },
Self::WouldPutCountersOrEnterWithCountersMatcher { ability_source, controller, filter, player_filter, counter_type, actor, includes_permanents, effect_only } => ReplacementMatcherDescriptor::WouldPutCountersOrEnterWithCountersMatcher { ability_source: ability_source, controller: controller, filter: filter, player_filter: player_filter, counter_type: counter_type, actor: actor, includes_permanents: includes_permanents, effect_only: effect_only },
Self::DredgeDrawMatcher { amount } => ReplacementMatcherDescriptor::DredgeDrawMatcher { amount: amount },
Self::ConditionalWouldDrawCardMatcher { condition, display } => ReplacementMatcherDescriptor::ConditionalWouldDrawCardMatcher { condition: condition, display: display },
Self::WouldDrawInstructionMatcher { condition, except_first_of_draw_step, per_instruction, display } => ReplacementMatcherDescriptor::WouldDrawInstructionMatcher { condition: condition, except_first_of_draw_step: except_first_of_draw_step, per_instruction: per_instruction, display: display },
Self::WouldDrawByPlayerMatcher { drawer, except_first_of_draw_step, display } => ReplacementMatcherDescriptor::WouldDrawByPlayerMatcher { drawer: drawer, except_first_of_draw_step: except_first_of_draw_step, display: display },
Self::WouldGoToGraveyardFromAnywhereMatcher { filter, exclude_cycled } => ReplacementMatcherDescriptor::WouldGoToGraveyardFromAnywhereMatcher { filter: filter, exclude_cycled: exclude_cycled },
Self::WouldEnterFromZoneMatcher { enter_matcher, not_cast } => ReplacementMatcherDescriptor::WouldEnterFromZoneMatcher { enter_matcher: enter_matcher, not_cast: not_cast },
Self::TappedForMinimumManaMatcher { inner, minimum_amount } => ReplacementMatcherDescriptor::TappedForMinimumManaMatcher { inner: inner, minimum_amount: minimum_amount },
Self::ConditionalWouldChangeLifeMatcher { player, loss, condition, display } => ReplacementMatcherDescriptor::ConditionalWouldChangeLifeMatcher { player: player, loss: loss, condition: condition, display: display },
        })
    }
}
pub fn restore_replacement_matcher_descriptor(model: NativeReplacementMatcherDescriptor) -> Result<Box<dyn crate::events::ReplacementMatcher>, String> {
    if let Some(matcher) = crate::effects::damage::redirect_next_damage_to_target::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::effects::damage::redirect_next_time_damage_to_source::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::effects::damage::replace_next_damage_to_target::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::events::cards::matchers::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::events::counters::matchers::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::events::damage::matchers::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::events::life::matchers::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::events::mana::matchers::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::events::other::keyword_action::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::events::other::player_loses_game::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::events::permanents::matchers::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::events::tokens::matchers::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::events::zones::matchers::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::static_abilities::misc::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    if let Some(matcher) = crate::static_abilities::misc::replacements_and_rules::restore_replacement_matcher_descriptor(&model) { return Ok(matcher); }
    Err("native replacement predicate descriptor has no restoring adapter".into())
}

#[cfg(test)]
mod native_replacement_matcher_descriptor_contract_tests {
    use super::*;
    use crate::{GameState,PlayerId,CardId,Zone};
    use crate::events::{ReplacementMatcher,EventContext,LifeGainEvent};
    use crate::events::life::matchers::WouldGainLifeMatcher;
    use crate::events::zones::matchers::WouldChangeZoneMatcher;
    use crate::target::{PlayerFilter,ObjectFilter};
    #[test]fn same_display_predicates_preserve_distinct_player_applicability(){let _g=crate::tests::test_helpers::setup_two_player_game();let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);let ctx=EventContext::for_controller(alice,&_g);let event=LifeGainEvent::new(alice,3);let left=WouldGainLifeMatcher::new(PlayerFilter::Specific(alice));let right=WouldGainLifeMatcher::new(PlayerFilter::Specific(bob));assert_eq!(left.display(),right.display());let left=restore_replacement_matcher_descriptor(left.export_descriptor().unwrap()).unwrap();let right=restore_replacement_matcher_descriptor(right.export_descriptor().unwrap()).unwrap();assert!(left.matches_event(&event,&ctx).unwrap());assert!(!right.matches_event(&event,&ctx).unwrap());}
    fn captured()->NativeReplacementMatcherDescriptor {let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let definition=crate::cards::builders::CardDefinitionBuilder::new(CardId::new(),"Complete captured predicate snapshot").card_types(vec![crate::types::CardType::Artifact]).with_ability(crate::Ability::static_ability(crate::static_abilities::StaticAbility::flying())).build();let id=game.create_object_from_definition(&definition,alice,Zone::Battlefield);let snapshot=crate::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(),&game);assert_eq!(snapshot.abilities.len(),1);assert_eq!(snapshot.copiable_values.abilities.len(),1);let tags=std::collections::HashMap::from([(crate::tag::TagKey::from("capture"),vec![snapshot])]);WouldChangeZoneMatcher::new(ObjectFilter::permanent(),Some(Zone::Battlefield),Some(Zone::Exile)).with_frozen_tagged_objects(tags).export_descriptor().unwrap()}
    #[test]fn snapshot_mapping_preserves_native_executables_before_codec_binding(){let model=captured();let mut calls=0;let model=model.try_map_snapshots(|snapshot|{calls+=1;assert_eq!(snapshot.abilities.len(),1);assert_eq!(snapshot.copiable_values.abilities.len(),1);Ok::<_,String>(snapshot)}).unwrap();assert_eq!(calls,1);let native=restore_replacement_matcher_descriptor(model).unwrap();let matcher=native.downcast_ref::<WouldChangeZoneMatcher>().unwrap();let snapshot=&matcher.frozen_tagged_objects[&crate::tag::TagKey::from("capture")][0];assert_eq!(snapshot.abilities.len(),1);assert_eq!(snapshot.copiable_values.abilities.len(),1);assert_eq!(matcher.from_zone,Some(Zone::Battlefield));assert_eq!(matcher.to_zone,Some(Zone::Exile));}
    #[test]fn snapshot_binding_failure_is_explicit(){let error=captured().try_map_snapshots::<(),_>(|_|Err("snapshot graph binding failed")).unwrap_err();assert_eq!(error,"snapshot graph binding failed");}
    #[derive(Debug,Clone)]struct UnknownPredicate;
    impl ReplacementMatcher for UnknownPredicate {fn matches_prepared_event(&self,_:&dyn crate::events::GameEventType,_:&crate::events::context::PreparedEventContext)->bool {true}fn display(&self)->String{"When a player would gain life".into()}}
    #[test]fn unknown_native_predicate_is_not_guessed_from_display_text(){assert_eq!(UnknownPredicate.display(),WouldGainLifeMatcher::new(PlayerFilter::Specific(PlayerId::from_index(0))).display());assert!(UnknownPredicate.export_descriptor().unwrap_err().contains("UnknownPredicate"));}
}

#[cfg(test)]
mod frozen_matcher_history_order_contract_tests {
    use super::*;
    fn unordered_tags()->std::collections::HashMap<crate::tag::TagKey,Vec<u32>> {
        // Select a real native map with noncanonical iteration order; this makes
        // the old failure explicit instead of relying on a lucky random seed.
        for _ in 0..128 {
            let tags=std::collections::HashMap::from([(crate::tag::TagKey::from("z-history"),vec![9,10]),(crate::tag::TagKey::from("a-history"),vec![1,2])]);
            if tags.keys().next()==Some(&crate::tag::TagKey::from("z-history")){return tags;}
        }
        panic!("could not construct noncanonical native map fixture");
    }
    fn descriptor(tags:std::collections::HashMap<crate::tag::TagKey,Vec<u32>>)->ReplacementMatcherDescriptor<u32>{
        ReplacementMatcherDescriptor::WouldChangeZoneMatcher{filter:crate::target::ObjectFilter::permanent(),from_zone:Some(crate::Zone::Battlefield),to_zone:Some(crate::Zone::Graveyard),cause_filter:None,require_cause_source_match:false,frozen_tagged_objects:tags}
    }
    #[test]
    fn frozen_history_callbacks_follow_tag_order_and_preserve_each_capture_vector(){
        let tags=unordered_tags();let mut visited=Vec::new();let restored=descriptor(tags.clone()).try_map_snapshots(|snapshot|{visited.push(snapshot);Ok::<_, &'static str>(snapshot)}).unwrap();
        assert_eq!(visited,vec![1,2,9,10],"policy order must not depend on native hash iteration");
        let ReplacementMatcherDescriptor::WouldChangeZoneMatcher{frozen_tagged_objects,..}=restored else{panic!("zone matcher")};assert_eq!(frozen_tagged_objects,tags);
    }
    #[test]
    fn frozen_history_failure_stops_before_lexically_later_tags(){
        let mut visited=Vec::new();let error=descriptor(unordered_tags()).try_map_snapshots(|snapshot|{visited.push(snapshot);if snapshot==1{Err("first disclosure required")}else{Ok(snapshot)}}).unwrap_err();
        assert_eq!(error,"first disclosure required");assert_eq!(visited,vec![1],"later-tag policy must not run before earliest disclosure failure");
    }
}

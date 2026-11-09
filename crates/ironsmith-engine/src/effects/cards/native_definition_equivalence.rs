//! Structural definition proof over typed native payloads. No rendering or wire
//! codec participates in equality. Unsupported opaque executors fail closed.
use crate::effects::cards::definition_scope::CardDefinitionScopeEffect;
use crate::{CardDefinition, Effect};

#[derive(Clone, Debug)]
struct EffectProof(Effect);
impl PartialEq for EffectProof {
    fn eq(&self, other: &Self) -> bool {
        effect_equal(&self.0, &other.0)
    }
}
type ProofCost = ironsmith_core::Cost<EffectProof>;
type ProofStatic = ironsmith_core::StaticAbility<
    ironsmith_core::trigger_model::Trigger,
    EffectProof,
    ProofCost,
    crate::static_abilities::ThisSpellCostCondition,
>;

fn cost_proof(cost: crate::costs::Cost) -> Result<ProofCost, String> {
    cost.compiled_model()
        .ok_or_else(|| "native cost has no structural definition proof".to_string())?
        .clone()
        .try_map_effect(|effect| Ok::<_, String>(EffectProof(effect)))
}
fn trigger_proof(
    trigger: crate::triggers::Trigger,
) -> Result<ironsmith_core::trigger_model::Trigger, String> {
    trigger
        .compiled_model()
        .cloned()
        .ok_or_else(|| "native trigger has no structural definition proof".to_string())
}
fn static_proof(ability: crate::static_abilities::StaticAbility) -> Result<ProofStatic, String> {
    ability
        .canonical_model()
        .ok_or_else(|| "native static ability has no structural definition proof".to_string())?
        .try_map(
            trigger_proof,
            |effect| Ok::<_, String>(EffectProof(effect)),
            cost_proof,
            Ok::<_, String>,
        )
}

pub(crate) fn definitions_equal(
    left: &CardDefinition,
    right: &CardDefinition,
) -> Result<bool, String> {
    // Replacing only executable wrappers preserves every non-executable field
    // of the definition, ability, resolution segment, cost and cast route.
    let normalize = |definition: CardDefinition| {
        definition.try_map(
            |ability| {
                ability.try_map(
                    static_proof,
                    trigger_proof,
                    |effect| Ok::<_, String>(EffectProof(effect)),
                    cost_proof,
                    Ok::<_, String>,
                )
            },
            |effect| Ok::<_, String>(EffectProof(effect)),
            cost_proof,
            |method| method.try_map(|effect| Ok::<_, String>(EffectProof(effect)), cost_proof),
            |cost| cost.try_map(cost_proof),
        )
    };
    Ok(normalize(left.clone())? == normalize(right.clone())?)
}
fn effects_equal(left: &[Effect], right: &[Effect]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(a, b)| effect_equal(a, b))
}

macro_rules! with_native_scalar_types {
    ($consumer:ident) => {
        $consumer!(
            crate::effects::AddManaEffect,
            crate::effects::AdditionalLandPlaysEffect,
            crate::effects::AddManaFromCommanderColorIdentityEffect,
            crate::effects::AddManaOfAnyColorEffect,
            crate::effects::AddManaOfAnyOneColorEffect,
            crate::effects::AddManaOfChosenColorEffect,
            crate::effects::AddManaOfColorsAmongEffect,
            crate::effects::AddManaEqualToManaCostEffect,
            crate::effects::AddManaLostThisWayEffect,
            crate::effects::AddManaOfNotedTypeEffect,
            crate::effects::AddOneManaOfAnyColorAmongEffect,
            crate::effects::AddScaledManaEffect,
            crate::effects::AdditionalPhasesEffect,
            crate::effects::AdditionalStepsEffect,
            crate::effects::AmassEffect,
            crate::effects::CollectEvidenceEffect,
            crate::effects::EmpowerJaceEffect,
            crate::effects::AmplifyEffect,
            crate::effects::AscendEffect,
            crate::effects::AssignNoCombatDamageEffect,
            crate::effects::AttachObjectsEffect,
            crate::effects::AttachToEffect,
            crate::effects::AuraSwapEffect,
            crate::effects::BecomeBasicLandTypeChoiceEffect,
            crate::effects::BecomeColorChoiceEffect,
            crate::effects::BecomeCreatureTypeChoiceEffect,
            crate::effects::BolsterEffect,
            crate::effects::CantEffect,
            crate::effects::CastSourceEffect,
            crate::effects::RemoveOptionalTextFragmentEffect,
            crate::effects::ChangeTextEffect,
            crate::effects::ChooseCardNameEffect,
            crate::effects::ChooseCardTypeEffect,
            crate::effects::RippleEffect,
            crate::effects::ChooseNumberEffect,
            crate::effects::ChooseNewTargetsEffect,
            crate::effects::ChooseObjectsEffect,
            crate::effects::ChooseSpellCastHistoryEffect,
            crate::effects::CipherEffect,
            crate::effects::ClashEffect,
            crate::effects::ConspireCostEffect,
            crate::effects::ConsultTopOfLibraryEffect,
            crate::effects::ControlPlayerEffect,
            crate::effects::TriggerAbilitiesByClassEffect,
            crate::effects::CopySpellEffect,
            crate::effects::CopySpellForEachTargetEffect,
            crate::effects::CounterEffect,
            crate::effects::CrewCostEffect,
            crate::effects::DealDamageEffect,
            crate::effects::DealDamageToRecipientsEffect,
            crate::effects::DealDamageBySourcesEffect,
            crate::effects::DealDamageEachEffect,
            crate::effects::DevourEffect,
            crate::effects::DirectionalAdjacentPlayerControlEffect,
            crate::effects::DiscardHandEffect,
            crate::effects::DiscoverEffect,
            crate::effects::NoteObjectNamesEffect,
            crate::effects::DrawCardsEffect,
            crate::effects::EachPlayerScryEffect,
            crate::effects::EarthbendEffect,
            crate::effects::PerformNamedActionEffect,
            crate::effects::ActivateAbilityWithoutCostsEffect,
            crate::effects::EmitKeywordActionEffect,
            crate::effects::EnergyCountersEffect,
            crate::effects::EvolveEffect,
            crate::effects::ExchangeControlEffect,
            crate::effects::ExchangeTextBoxesEffect,
            crate::effects::ExertCostEffect,
            crate::effects::ExileEffect,
            crate::effects::ExileInsteadOfGraveyardEffect,
            crate::effects::ExileUntilEffect,
            crate::effects::ExtraTurnAfterNextTurnEffect,
            crate::effects::ExtraTurnEffect,
            crate::effects::FlipEffect,
            crate::effects::GainLifeEffect,
            crate::effects::GrantEndThisEffectPaymentEffect,
            crate::effects::GrantNextSpellCostReductionEffect,
            crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
            crate::effects::GrantTaggedSpellLifeCostByManaValueEffect,
            crate::effects::HealDamageEffect,
            crate::effects::IncreaseSpeedEffect,
            crate::effects::IncubateEffect,
            crate::effects::LearnEffect,
            crate::effects::LookAtHandEffect,
            crate::effects::LookAtObjectsEffect,
            crate::effects::LookAtTopCardsEffect,
            crate::effects::DrawTheGameEffect,
            crate::effects::LoseTheGameEffect,
            crate::effects::ManifestObjectsEffect,
            crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect,
            crate::effects::MayMoveToZoneEffect,
            crate::effects::CreateLinkedFaceTokenEffect,
            crate::effects::LinkExiledCardsEffect,
            crate::effects::MeldEffect,
            crate::effects::MergePermanentsEffect,
            crate::effects::MillEffect,
            crate::effects::ModifyPowerToughnessEffect,
            crate::effects::ModifyPowerToughnessForEachEffect,
            crate::effects::GrantActivationCostAlternativeEffect,
            crate::effects::SetPermanentDesignationEffect,
            crate::effects::GrantExileControllerInspectionEffect,
            ironsmith_core::effect::RememberOriginalMovementArrivalsEffect,
            crate::effects::MonstrosityEffect,
            crate::effects::MoveAllCountersEffect,
            crate::effects::MoveCountersEffect,
            crate::effects::MoveOneCounterEffect,
            crate::effects::MoveToLibraryNthFromTopEffect,
            crate::effects::MoveToLibraryTopOrBottomChoiceEffect,
            crate::effects::NinjutsuCostEffect,
            crate::effects::NinjutsuEffect,
            crate::effects::CastFaceDownUsingActivationManaEffect,
            crate::effects::RegisterExtraDieReplacementEffect,
            crate::effects::NoteActivationManaTypeEffect,
            crate::effects::NoteActivationManaPaymentEffect,
            crate::effects::NoteTriggeringCreatureTypeEffect,
            crate::effects::NoteCountersEffect,
            crate::effects::PayAnyEnergyEffect,
            crate::effects::PayAnyLifeEffect,
            crate::effects::TagPlayersEffect,
            crate::effects::KeepGreatestManaValuePlayersEffect,
            crate::effects::RollDieKeepHighestPlayersEffect,
            crate::effects::RetainSpellChoicesEffect,
            crate::effects::PutRetainedSpellCopyOntoStackEffect,
            crate::effects::PayEnergyEffect,
            crate::effects::PayLifeEffect,
            crate::effects::RedistributeLifeTotalsEffect,
            crate::effects::CoinFlipLifeStakesEffect,
            crate::effects::RedirectEnteringSourceEffect,
            crate::effects::PayManaEffect,
            crate::effects::PopulateEffect,
            crate::effects::PreventAllCombatDamageEffect,
            crate::effects::ProliferateEffect,
            crate::effects::PutCountersEffect,
            crate::effects::PutOntoBattlefieldEffect,
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
            crate::effects::RearrangeLookedCardsInLibraryEffect,
            crate::effects::ReconfigureEffect,
            crate::effects::ReduceSpeedEffect,
            crate::effects::RegisterCounterPlacementReplacementEffect,
            crate::effects::RegisterDamageMultiplierEffect,
            crate::effects::RegisterTokenCreationMultiplierEffect,
            crate::effects::RegisterDamageCounterReplacementEffect,
            crate::effects::RegisterDamageAdditionEffect,
            crate::effects::RegisterEnterTappedReplacementEffect,
            crate::effects::RegisterFutureZoneReplacementEffect,
            crate::effects::RegisterManaReplacementEffect,
            crate::effects::RegisterManaRewriteEffect,
            crate::effects::RegisterManaSpendPermissionEffect,
            crate::effects::RegisterNextBatchEnterAsCopyEffect,
            crate::effects::RegisterNextBatchEnterWithCountersEffect,
            crate::effects::RemoveAnyCountersAmongEffect,
            crate::effects::RemoveAnyCountersFromSourceEffect,
            crate::effects::RemoveCountersEffect,
            crate::effects::RemoveUpToAnyCountersEffect,
            crate::effects::RenownEffect,
            crate::effects::ReorderLibraryTopEffect,
            crate::effects::ReorderTopPlanarDeckEffect,
            crate::effects::RestartGameEffect,
            crate::effects::RetainManaUntilEndOfTurnEffect,
            crate::effects::RetargetStackObjectEffect,
            crate::effects::ReturnAllToBattlefieldEffect,
            crate::effects::ReturnFromGraveyardToBattlefieldEffect,
            crate::effects::ReturnFromGraveyardToHandEffect,
            crate::effects::ReturnToHandEffect,
            crate::effects::RevealFromHandEffect,
            crate::effects::RevealSourceFromHandEffect,
            crate::effects::RevealTaggedEffect,
            crate::effects::RevealTopEffect,
            crate::effects::ReverseTurnOrderEffect,
            crate::effects::RollToVisitAttractionsEffect,
            crate::effects::RollPlanarDieEffect,
            crate::effects::SacrificeTargetEffect,
            crate::effects::ScryEffect,
            crate::effects::SearchLibraryEffect,
            crate::effects::SearchLibrarySlotsEffect,
            crate::effects::SetBasePowerToughnessEffect,
            crate::effects::SetClassLevelEffect,
            crate::effects::SetDayNightEffect,
            crate::effects::ChoosePlayerOptionEffect,
            crate::effects::ControlVotesThisTurnEffect,
            crate::effects::PutCounterOfKindChosenFromEffect,
            crate::effects::PutCounterOfRandomKindEffect,
            crate::effects::ShuffleHandAndGraveyardIntoLibraryEffect,
            crate::effects::ShuffleLibraryEffect,
            crate::effects::ShuffleObjectsIntoLibraryEffect,
            crate::effects::SneakCostEffect,
            crate::effects::SolveCaseEffect,
            crate::effects::LinkPermanentsEffect,
            crate::effects::SoulbondPairEffect,
            crate::effects::TagAttachedToSourceEffect,
            crate::effects::TagMatchingObjectsEffect,
            crate::effects::TagOtherBlockParticipantEffect,
            crate::effects::TagTriggeringAttackerEffect,
            crate::effects::TagTriggeringBlockersEffect,
            crate::effects::TagTriggeringObjectEffect,
            crate::effects::TagSplicedCardsEffect,
            crate::effects::TagLastDamageSourceEffect,
            crate::effects::LinkObjectsToGroupEffect,
            crate::effects::TagTriggeringSourceEffect,
            crate::effects::TapEffect,
            crate::effects::TargetOnlyEffect,
            crate::effects::TicketCountersEffect,
            crate::effects::TransformEffect,
            crate::effects::TurnFaceDownEffect,
            crate::effects::TurnFaceUpEffect,
            crate::effects::ReselectAttackTargetEffect,
            crate::effects::ExchangeBlocksEffect,
            crate::effects::UnblockSoleBlockedAttackersEffect,
            crate::effects::BlockAttackerOfChoiceEffect,
            crate::effects::PlayWithHandRevealedEffect,
            crate::effects::RevealSourceInHandUntilOwnersNextTurnEffect,
            crate::effects::ChooseFriendsOrFoesEffect,
            crate::effects::UnattachObjectsEffect,
            crate::effects::UnearthEffect,
            crate::effects::UntapEffect,
            crate::effects::VariableCasualtyPlaneswalkerCopyEffect,
            crate::effects::WinTheGameEffect,
            crate::effects::composition::ResolvesDespiteIllegalTargetsEffect,
            crate::effects::mana::AddManaOfImprintedColorsEffect,
            crate::effects::player::MayCastForMiracleCostEffect,
            crate::effects::zones::SacrificePlayerEffect,
        );
    };
}

/// Exact native scalar admission: visitors must confirm that no executable
/// children or owned definitions are hidden behind a scalar registration.
pub(crate) fn trusted_native_scalar_effect(effect: &Effect) -> bool {
    let mut nested = false;
    effect.visit_child_effects(&mut |_| nested = true);
    effect.visit_card_definitions(&mut |_| nested = true);
    if nested {
        return false;
    }
    macro_rules! recognized { ($($ty:path),* $(,)?) => { $(
        if effect.downcast_ref::<$ty>().is_some() { return true; }
    )* }; }
    with_native_scalar_types!(recognized);
    false
}

fn effect_equal(left: &Effect, right: &Effect) -> bool {
    // Shared immutable executor identity is a complete proof, unlike a card
    // name, display string, or a reused CardId.
    if std::sync::Arc::ptr_eq(&left.0, &right.0) {
        return true;
    }
    if let Some(a) = left.downcast_ref::<CardDefinitionScopeEffect>() {
        let Some(b) = right.downcast_ref::<CardDefinitionScopeEffect>() else {
            return false;
        };
        return a.scope == b.scope
            && a.definitions.len() == b.definitions.len()
            && a.definitions
                .iter()
                .zip(&b.definitions)
                .all(|(a, b)| definitions_equal(a, b) == Ok(true))
            && effect_equal(&a.effect, &b.effect);
    }
    if let Some(a) = left.downcast_ref::<super::CreateCardsEffect>() {
        let Some(b) = right.downcast_ref::<super::CreateCardsEffect>() else {
            return false;
        };
        return a.definition_scope == b.definition_scope
            && a.definition_refs == b.definition_refs
            && a.operation_label == b.operation_label
            && a.selection == b.selection
            && a.count == b.count
            && a.player == b.player
            && a.zone == b.zone
            && a.library_position == b.library_position
            && a.definitions.len() == b.definitions.len()
            && a.definitions
                .iter()
                .zip(&b.definitions)
                .all(|(a, b)| definitions_equal(a, b) == Ok(true));
    }
    if let Some(a) = left.downcast_ref::<crate::effects::SequenceEffect>() {
        let Some(b) = right.downcast_ref::<crate::effects::SequenceEffect>() else {
            return false;
        };
        return a.surface == b.surface
            && a.result_label == b.result_label
            && effects_equal(&a.effects, &b.effects);
    }
    if let Some(a) = left.downcast_ref::<crate::effects::MayEffect>() {
        let Some(b) = right.downcast_ref::<crate::effects::MayEffect>() else {
            return false;
        };
        return a.payment_receipt == b.payment_receipt
            && a.decider == b.decider
            && a.fallback == b.fallback
            && a.pay_as_cost == b.pay_as_cost
            && effects_equal(&a.effects, &b.effects);
    }
    if let Some(a) = left.downcast_ref::<crate::effects::TaggedEffect>() {
        let Some(b) = right.downcast_ref::<crate::effects::TaggedEffect>() else {
            return false;
        };
        return a.tag == b.tag
            && a.outcome_only == b.outcome_only
            && a.action == b.action
            && effect_equal(&a.effect, &b.effect);
    }
    if let Some(a) = left.downcast_ref::<crate::effects::WithIdEffect>() {
        let Some(b) = right.downcast_ref::<crate::effects::WithIdEffect>() else {
            return false;
        };
        return a.id == b.id && effect_equal(&a.effect, &b.effect);
    }
    let mut owned_children = false;
    left.visit_child_effects(&mut |_| owned_children = true);
    left.visit_card_definitions(&mut |_| owned_children = true);
    right.visit_child_effects(&mut |_| owned_children = true);
    right.visit_card_definitions(&mut |_| owned_children = true);
    if owned_children {
        return false;
    }
    macro_rules! compare_scalar {
        ($($ty:path),* $(,)?) => { $(
            if let Some(a) = left.downcast_ref::<$ty>() {
                return right.downcast_ref::<$ty>().is_some_and(|b| a == b);
            }
        )* };
    }
    with_native_scalar_types!(compare_scalar);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sequence(effects: Vec<Effect>) -> Effect {
        Effect::new(crate::effects::SequenceEffect::new(effects))
    }
    fn definition(effect: Effect) -> CardDefinition {
        crate::CardDefinitionBuilder::new(ironsmith_core::CardId::new(), "Native identity")
            .with_spell_effect(vec![effect])
            .build()
    }
    #[test]
    fn independent_native_leaf_payloads_compare_structurally_and_all_definition_metadata_matters() {
        let a = definition(Effect::draw(1));
        let mut b = a.clone();
        b.spell_effect = Some(vec![Effect::draw(1)].into());
        assert_eq!(definitions_equal(&a, &b), Ok(true));
        b.spell_effect = Some(vec![Effect::draw(2)].into());
        assert_eq!(definitions_equal(&a, &b), Ok(false));
        b = a.clone();
        b.card.name = "Different name".into();
        assert_eq!(definitions_equal(&a, &b), Ok(false));
        b = a.clone();
        b.ability_labels.push("unmatched label".into());
        assert_eq!(definitions_equal(&a, &b), Ok(false));
    }
    #[test]
    fn native_wrapper_receipts_and_sequential_child_order_are_not_erased() {
        let a = definition(Effect::new(crate::effects::TaggedEffect::new(
            "result",
            sequence(vec![Effect::draw(1), Effect::gain_life(2)]),
        )));
        let mut b = a.clone();
        b.spell_effect = Some(
            vec![Effect::new(crate::effects::TaggedEffect::new(
                "result",
                sequence(vec![Effect::draw(1), Effect::gain_life(2)]),
            ))]
            .into(),
        );
        assert_eq!(definitions_equal(&a, &b), Ok(true));
        b.spell_effect = Some(
            vec![Effect::new(crate::effects::TaggedEffect::new(
                "result",
                sequence(vec![Effect::gain_life(2), Effect::draw(1)]),
            ))]
            .into(),
        );
        assert_eq!(definitions_equal(&a, &b), Ok(false));
        b.spell_effect = Some(
            vec![Effect::new(crate::effects::TaggedEffect::new(
                "other-result",
                sequence(vec![Effect::draw(1), Effect::gain_life(2)]),
            ))]
            .into(),
        );
        assert_eq!(definitions_equal(&a, &b), Ok(false));
    }
}

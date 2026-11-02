//! Checked typed execution-reference rewriting, independent of artifact codecs.
//! Immutable native instructions are cloned in full; opaque executors fail closed.
use crate::effect::{Effect, EffectId};
use crate::effects::*;
use crate::tag::TagKey;
use ironsmith_core::tag::TagKeyWalk;

pub(crate) struct ReferenceMapper<'a> {
    pub tags: &'a mut dyn FnMut(&mut TagKey),
    pub ids: &'a mut dyn FnMut(&mut EffectId),
}
impl ReferenceMapper<'_> {
    pub fn pure<T: TagKeyWalk>(&mut self, value: &mut T) {
        value.map_execution_nodes(&mut |node| {
            if let Some(tag) = node.downcast_mut::<TagKey>() { (self.tags)(tag); return true; }
            if let Some(id) = node.downcast_mut::<EffectId>() { (self.ids)(id); return true; }
            false
        });
    }
    pub fn effects(&mut self, effects: &[Effect]) -> Result<Vec<Effect>, String> {
        effects.iter().map(|effect| self.effect(effect)).collect()
    }
    pub fn cost(&mut self, cost: &crate::costs::Cost) -> Result<crate::costs::Cost, String> {
        let model = cost.compiled_model().cloned().ok_or("native cost has no typed reference model")?;
        // Map the actual native child through its checked owner, then traverse
        // the remaining pure fields without giving Effect a lossy walk impl.
        let mut child = None;
        let mut pure = model.try_map_effect(|effect| {
            child = Some(self.effect(&effect)?);
            Ok::<_, String>(())
        })?;
        self.pure(&mut pure);
        let mapped = pure.try_map_effect(|()| child.take().ok_or_else(|| "missing native cost child".to_string()))?;
        crate::costs::Cost::from_model(mapped)
    }
    pub fn total_cost(&mut self, cost: &crate::cost::TotalCost) -> Result<crate::cost::TotalCost, String> {
        cost.clone().try_map(|cost| self.cost(&cost))
    }
    pub fn effect(&mut self, effect: &Effect) -> Result<Effect, String> {
        macro_rules! pure {
            ($($ty:ty),* $(,)?) => {$({
                if let Some(model) = effect.downcast_ref::<$ty>() {
                    let mut mapped = model.clone(); self.pure(&mut mapped);
                    return Ok(Effect::new(mapped));
                }
            })*};
        }
        macro_rules! fields {
            ($ty:ty, [$($field:ident),*], [$($children:ident),*]) => {
                if let Some(model) = effect.downcast_ref::<$ty>() {
                    let mut mapped = model.clone();
                    $(self.pure(&mut mapped.$field);)*
                    $(mapped.$children = self.effects(&mapped.$children)?;)*
                    return Ok(Effect::new(mapped));
                }
            };
        }
        pure!(
            ActivateAbilityWithoutCostsEffect,
            ActivateManaAbilityOfEachLandEffect,
            AddManaEffect,
            AddManaEqualToManaCostEffect,
            AddManaFromCommanderColorIdentityEffect,
            AddManaLostThisWayEffect,
            AddManaOfAnyColorEffect,
            AddManaOfAnyOneColorEffect,
            AddManaOfChosenColorEffect,
            AddManaOfColorsAmongEffect,
            AddManaOfNotedTypeEffect,
            AddOneManaOfAnyColorAmongEffect,
            AddScaledManaEffect,
            AmassEffect,
            AmplifyEffect,
            AscendEffect,
            AssignNoCombatDamageEffect,
            AttachObjectsEffect,
            AttachToEffect,
            AuraSwapEffect,
            BecomeBasicLandTypeChoiceEffect,
            BecomeBlockedEffect,
            BecomeColorChoiceEffect,
            BecomeCreatureTypeChoiceEffect,
            BecomeForetoldEffect,
            BecomePlottedEffect,
            BlockAttackerOfChoiceEffect,
            CantEffect,
            CastFaceDownUsingActivationManaEffect,
            CastSourceEffect,
            ChangeTextEffect,
            ChooseCardNameEffect,
            ChooseCardTypeEffect,
            ChooseFriendsOrFoesEffect,
            ChooseLandTypeEffect,
            ChooseNewTargetsEffect,
            ChooseNumberEffect,
            ChooseObjectsEffect,
            ChooseSpellCastHistoryEffect,
            ClashEffect,
            ClearGoadEffect,
            CoinFlipLifeStakesEffect,
            CollectEvidenceEffect,
            ControlPlayerEffect,
            CopySpellEffect,
            CopySpellForEachTargetEffect,
            CounterEffect,
            CreateLinkedFaceTokenEffect,
            CrewCostEffect,
            DealDamageBySourcesEffect,
            DealDamageEachEffect,
            DealDamageEffect,
            DealDamageToRecipientsEffect,
            DirectionalAdjacentPlayerControlEffect,
            DiscardHandEffect,
            DiscoverEffect,
            DoubleCountersEffect,
            DrawCardsEffect,
            EachPlayerScryEffect,
            EarthbendEffect,
            EmitKeywordActionEffect,
            EmpowerJaceEffect,
            EnergyCountersEffect,
            ExchangeBlocksEffect,
            ExchangeControlEffect,
            ExchangeTextBoxesEffect,
            ExertCostEffect,
            ExileEffect,
            ExileInsteadOfGraveyardEffect,
            ExileUntilEffect,
            ExtraTurnAfterNextTurnEffect,
            ExtraTurnEffect,
            FlipEffect,
            GainLifeEffect,
            GrantEndThisEffectPaymentEffect,
            GrantNextSpellCostReductionEffect,
            GrantPayLifeManaAbilityUntilEndOfTurnEffect,
            GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
            GrantTaggedSpellLifeCostByManaValueEffect,
            GrantTapUncontrolledLandsForManaEffect,
            HealDamageEffect,
            IncubateEffect,
            LearnEffect,
            LinkExiledCardsEffect,
            LinkObjectsToGroupEffect,
            LinkPermanentsEffect,
            LookAtHandEffect,
            LookAtObjectsEffect,
            LookAtTopCardsEffect,
            LoseTheGameEffect,
            ManifestObjectsEffect,
            MayCastMatchingSpellWithoutPayingManaCostEffect,
            MayMoveToZoneEffect,
            MeldEffect,
            MergePermanentsEffect,
            MillEffect,
            ModifyPowerToughnessEffect,
            ModifyPowerToughnessForEachEffect,
            MonstrosityEffect,
            MoveAllCountersEffect,
            MoveCountersEffect,
            MoveOneCounterEffect,
            MoveToLibraryNthFromTopEffect,
            MoveToLibraryTopOrBottomChoiceEffect,
            MustAttackPlayerThisTurnEffect,
            NoteCountersEffect,
            NoteObjectNamesEffect,
            PayAnyEnergyEffect,
            PayAnyLifeEffect,
            PayEnergyEffect,
            PayLifeEffect,
            PayManaEffect,
            PerformNamedActionEffect,
            PlayWithHandRevealedEffect,
            PopulateEffect,
            PrepareEffect,
            ProliferateEffect,
            PutCounterOfChosenKindEffect,
            PutCounterOfKindChosenFromEffect,
            PutCounterOfRandomKindEffect,
            PutCountersEffect,
            PutOntoBattlefieldEffect,
            PutTaggedRemainderOnLibraryBottomEffect,
            RearrangeLookedCardsInLibraryEffect,
            ReconfigureEffect,
            RedirectAllDamageThisTurnToTargetEffect,
            RedirectEnteringSourceEffect,
            RedistributeLifeTotalsEffect,
            RegisterCounterPlacementReplacementEffect,
            RegisterDamageAdditionEffect,
            RegisterDamageCounterReplacementEffect,
            RegisterDamageMultiplierEffect,
            RegisterDamagedBySourceZoneReplacementEffect,
            RegisterEnterTappedReplacementEffect,
            RegisterEnterUnderControlReplacementEffect,
            RegisterEnterWithCountersReplacementEffect,
            RegisterExtraDieReplacementEffect,
            RegisterFutureZoneReplacementEffect,
            RegisterManaReplacementEffect,
            RegisterManaRewriteEffect,
            RegisterManaSpendPermissionEffect,
            RegisterNextBatchEnterAsCopyEffect,
            RegisterNextBatchEnterWithCountersEffect,
            RegisterTokenCreationMultiplierEffect,
            RemoveAnyCountersAmongEffect,
            RemoveAnyCountersFromSourceEffect,
            RemoveCountersEffect,
            RemoveOptionalTextFragmentEffect,
            RemoveUpToAnyCountersEffect,
            RenownEffect,
            ReorderLibraryTopEffect,
            ReorderTopPlanarDeckEffect,
            RerollStoredDieResultsEffect,
            ReselectAttackTargetEffect,
            RestartGameEffect,
            RetainManaUntilEndOfTurnEffect,
            ReturnAllToBattlefieldEffect,
            ReturnFromGraveyardOrExileToBattlefieldEffect,
            ReturnFromGraveyardToHandEffect,
            ReturnToHandEffect,
            RevealFromHandEffect,
            RevealSourceFromHandEffect,
            RevealSourceInHandUntilOwnersNextTurnEffect,
            RevealTaggedEffect,
            RevealTopEffect,
            RippleEffect,
            RollAndStoreDieResultsEffect,
            RollPlanarDieEffect,
            RollToVisitAttractionsEffect,
            ironsmith_core::SacrificePlayerEffect,
            SacrificeTargetEffect,
            ScryEffect,
            SearchLibraryEffect,
            SearchLibrarySlotsEffect,
            SetBasePowerToughnessEffect,
            SetClassLevelEffect,
            ShuffleLibraryEffect,
            ShuffleObjectsIntoLibraryEffect,
            SoulbondPairEffect,
            TagAttachedToSourceEffect,
            TagLastDamageSourceEffect,
            TagMatchingObjectsEffect,
            TagOtherBlockParticipantEffect,
            TagSplicedCardsEffect,
            TagTriggeringAttackerEffect,
            TagTriggeringBlockersEffect,
            TagTriggeringObjectEffect,
            TagTriggeringSourceEffect,
            TapEffect,
            TargetOnlyEffect,
            TicketCountersEffect,
            TransformEffect,
            TriggerAbilitiesByClassEffect,
            TurnFaceDownEffect,
            TurnFaceUpEffect,
            UnattachObjectsEffect,
            UnblockSoleBlockedAttackersEffect,
            UnearthEffect,
            UnlockRoomDoorEffect,
            UntapEffect,
            VariableCasualtyPlaneswalkerCopyEffect,
            WinTheGameEffect,
        );
        if let Some(model)=effect.downcast_ref::<MoveToZoneEffect>() {
            let mut mapped=model.clone().try_map_entry_modifications(|_| Err::<(),_>("entry modification has no native reference owner".to_string()))?;
            self.pure(&mut mapped);
            let mapped=mapped.try_map_entry_modifications(|()| Err::<crate::continuous::Modification,_>("unexpected entry modification".to_string()))?;
            return Ok(Effect::new(mapped));
        }
        fields!(SequenceEffect, [], [effects]);
        fields!(MayEffect, [decider], [effects]);
        fields!(ForPlayersEffect, [filter, announcement_condition], [effects]);
        fields!(SacrificeEffect, [filter, count, player, event_object_tags, event_source_tags], []);
        fields!(DestroyEffect, [spec], []);
        fields!(LoseLifeEffect, [amount, player], []);
        fields!(ForEachObject, [filter], [effects]);
        fields!(ConditionalEffect, [condition], [if_true, if_false]);
        fields!(IfEffect, [condition, predicate], [then, else_]);
        fields!(NamedActionProgramEffect, [objects, player], [effects]);
        fields!(UnlessActionEffect, [player], [effects, alternative]);
        fields!(RepeatEffectsEffect, [count], [effects]);
        fields!(RepeatProcessEffect, [condition, predicate, choice_history], [effects]);
        if let Some(model) = effect.downcast_ref::<WithIdEffect>() {
            let mut mapped=model.clone(); self.pure(&mut mapped.id);
            mapped.effect=Box::new(self.effect(&mapped.effect)?);
            return Ok(Effect::new(mapped));
        }
        if let Some(model) = effect.downcast_ref::<TaggedEffect>() {
            let mut mapped=model.clone(); self.pure(&mut mapped.tag);
            mapped.effect=Box::new(self.effect(&mapped.effect)?);
            return Ok(Effect::new(mapped));
        }
        if let Some(model)=effect.downcast_ref::<ChooseModeEffect>() {
            let mut mapped=model.clone();
            self.pure(&mut mapped.chooser); self.pure(&mut mapped.min); self.pure(&mut mapped.max);
            self.pure(&mut mapped.choose_count); self.pure(&mut mapped.min_choose_count);
            self.pure(&mut mapped.mode_additional_mana_costs);
            self.pure(&mut mapped.conditional_mode_range); self.pure(&mut mapped.cast_chooser);
            mapped.common_prefix_effects=self.effects(&mapped.common_prefix_effects)?;
            for mode in &mut mapped.modes { mode.effects=self.effects(&mode.effects)?; }
            return Ok(Effect::new(mapped));
        }
        Err(format!("native effect has no typed execution-reference rewrite: {:?}", effect.0))
    }
}

/// Read declarations through native immutable ownership. Merely referenced
/// IDs are not declarations, and opaque display/retained JSON is never read.
pub(crate) fn visit_declared_effect_ids(effect: &Effect, visitor: &mut dyn FnMut(u32)) {
    if let Some(model)=effect.downcast_ref::<WithIdEffect>() { visitor(model.id.0); }
    effect.visit_child_effects(&mut |child| visit_declared_effect_ids(child, visitor));
    effect.visit_card_definitions(&mut |definition| visit_definition_effect_ids(definition, visitor));
}
pub(crate) fn visit_definition_effect_ids(definition: &crate::cards::CardDefinition, visitor: &mut dyn FnMut(u32)) {
    if let Some(program)=&definition.spell_effect {
        for effect in program.all_effects() { visit_declared_effect_ids(effect, visitor); }
    }
    for ability in &definition.abilities {
        crate::ability::visit_owned_effects(ability, &mut |effect| visit_declared_effect_ids(effect, visitor));
    }
    for abilities in definition.ability_counter_definitions.values() { for ability in abilities {
        crate::ability::visit_owned_effects(ability, &mut |effect| visit_declared_effect_ids(effect, visitor));
    }}
    crate::ability::visit_total_cost_owned_effects(&definition.additional_cost, &mut |effect| visit_declared_effect_ids(effect, visitor));
    for method in &definition.alternative_casts {
        let visitor=std::cell::RefCell::new(&mut *visitor);
        let _=method.clone().try_map(
            |effect| {visit_declared_effect_ids(&effect,&mut **visitor.borrow_mut()); Ok::<_,std::convert::Infallible>(effect)},
            |cost| {crate::ability::visit_cost_owned_effects(&cost,&mut |effect|visit_declared_effect_ids(effect,&mut **visitor.borrow_mut())); Ok(cost)},
        );
    }
    for cost in &definition.optional_costs {
        crate::ability::visit_total_cost_owned_effects(&cost.cost, &mut |effect| visit_declared_effect_ids(effect, visitor));
    }
}

/// Exact admitted native rewrite domain; unknown executors never imply no refs.
pub(crate) fn native_reference_rewrite_supported(effect: &Effect) -> bool {
    ReferenceMapper { tags: &mut |_| {}, ids: &mut |_| {} }.effect(effect).is_ok()
}

#[derive(Default)]
pub(crate) struct ReferenceInventory { pub ids: std::collections::BTreeSet<u32> }
impl ReferenceInventory {
    fn pure<T:TagKeyWalk>(&mut self,value:&mut T) {
        ReferenceMapper {tags:&mut |_|{},ids:&mut |id| {self.ids.insert(id.0);} }.pure(value);
    }
    fn effect(&mut self,effect:&Effect)->Result<(),String> {
        ReferenceMapper {tags:&mut |_|{},ids:&mut |id| {self.ids.insert(id.0);} }.effect(effect)?;
        Ok(())
    }
    fn cost(&mut self,cost:&crate::costs::Cost)->Result<ironsmith_core::Cost<()>,String> {
        let model=cost.compiled_model().cloned().ok_or("host cost lacks typed reference evidence")?;
        let mut pure=model.try_map_effect(|effect| {self.effect(&effect)?;Ok::<_,String>(())})?;
        self.pure(&mut pure);Ok(pure)
    }
    fn program(&mut self,program:&crate::resolution::ResolutionProgram)->Result<(),String> {
        let mut pure=program.clone().try_map_effects(|effect| {self.effect(&effect)?;Ok::<_,String>(())})?;
        self.pure(&mut pure);Ok(())
    }
    fn trigger(&mut self,trigger:&crate::triggers::Trigger)->Result<(),String> {
        let mut model=trigger.compiled_model().cloned().ok_or("host trigger lacks typed reference evidence")?;
        self.pure(&mut model);Ok(())
    }
    pub fn static_ability(&mut self,ability:&crate::static_abilities::StaticAbility)->Result<(),String> {
        let model=ability.canonical_model().ok_or("host static ability lacks typed reference evidence")?;
        let cell=std::cell::RefCell::new(&mut *self);
        let mut pure=model.try_map(
            |trigger| {cell.borrow_mut().trigger(&trigger)?;Ok::<_,String>(())},
            |effect| {cell.borrow_mut().effect(&effect)?;Ok(())},
            |cost| cell.borrow_mut().cost(&cost),
            |condition| Ok(condition),
        )?;
        drop(cell);self.pure(&mut pure);Ok(())
    }
    fn ability(&mut self,ability:&crate::ability::Ability)->Result<(),String> {
        let cell=std::cell::RefCell::new(&mut *self);
        let mut pure=ability.clone().try_map(
            |ability| {cell.borrow_mut().static_ability(&ability)?;Ok::<_,String>(())},
            |trigger| {cell.borrow_mut().trigger(&trigger)?;Ok(())},
            |effect| {cell.borrow_mut().effect(&effect)?;Ok(())},
            |cost| cell.borrow_mut().cost(&cost),
            |condition| Ok(condition),
        )?;
        drop(cell);self.pure(&mut pure);Ok(())
    }
    pub fn definition(&mut self,definition:&crate::cards::CardDefinition)->Result<(),String> {
        self.pure(&mut definition.card.clone());self.pure(&mut definition.aura_attach_filter.clone());
        for ability in &definition.abilities {self.ability(ability)?;}
        for abilities in definition.ability_counter_definitions.values() {for ability in abilities {self.ability(ability)?;}}
        if let Some(program)=&definition.spell_effect {self.program(program)?;}
        definition.additional_cost.clone().try_map(|cost|self.cost(&cost))?;
        for optional in &definition.optional_costs {
            let mut pure=optional.clone().try_map(|cost|self.cost(&cost))?;self.pure(&mut pure);
        }
        for alternative in &definition.alternative_casts {
            let cell=std::cell::RefCell::new(&mut *self);
            let mut pure=alternative.clone().try_map(
                |effect| {cell.borrow_mut().effect(&effect)?;Ok::<_,String>(())},
                |cost| cell.borrow_mut().cost(&cost),
            )?;
            drop(cell);self.pure(&mut pure);
        }
        Ok(())
    }
}

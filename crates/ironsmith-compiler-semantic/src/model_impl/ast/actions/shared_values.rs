use super::*;

impl SubjectVerbActionAst {
    /// Visits numeric operands shared by X substitution and effect-result binding.
    /// Each pass handles its additional operands and nested binding scopes separately.
    pub fn try_for_each_shared_value_mut<E>(
        &mut self,
        mut visit: impl FnMut(&mut Value) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            Self::Characteristics(action) => match action {
                CharacteristicActionAst::ReduceSpeed { amount, .. }
                | CharacteristicActionAst::SetLifeTotal { amount } => visit(amount)?,
                CharacteristicActionAst::SetBasePowerToughness { power, toughness, .. } => {
                    visit(power)?;
                    visit(toughness)?;
                }
                CharacteristicActionAst::SetBasePower { power, .. }
                | CharacteristicActionAst::SetBaseToughness { toughness: power, .. } => visit(power)?,
                CharacteristicActionAst::AddAllSubtypesOfFamily { .. }
                | CharacteristicActionAst::AddCardTypes { .. }
                | CharacteristicActionAst::AddColors { .. }
                | CharacteristicActionAst::AddSubtypes { .. }
                | CharacteristicActionAst::BecomeAuraEnchantment { .. }
                | CharacteristicActionAst::BecomeBasePtCreature { .. }
                | CharacteristicActionAst::BecomeBasicLandType { .. }
                | CharacteristicActionAst::BecomeBasicLandTypeChoice { .. }
                | CharacteristicActionAst::BecomeColorChoice { .. }
                | CharacteristicActionAst::BecomeCopy { .. }
                | CharacteristicActionAst::BecomeCreatureTypeChoice { .. }
                | CharacteristicActionAst::BecomeMonarch
                | CharacteristicActionAst::BecomeSaddledUntilEndOfTurn { .. }
                | CharacteristicActionAst::ChangeText { .. }
                | CharacteristicActionAst::RemoveOptionalTextFragment { .. }
                | CharacteristicActionAst::SetCardTypes { .. }
                | CharacteristicActionAst::SetColors { .. }
                | CharacteristicActionAst::SetCreatureSubtypes { .. } => {}
            },
            Self::Choices(
                ChoiceActionAst::ChooseCardName { .. }
                | ChoiceActionAst::ChooseCardType { .. }
                | ChoiceActionAst::ChooseColor
                | ChoiceActionAst::ChooseColorAmong { .. }
                | ChoiceActionAst::ChooseCreatureType { .. }
                | ChoiceActionAst::ChooseLandType { .. }
                | ChoiceActionAst::ChooseNamedOption { .. }
                | ChoiceActionAst::ChooseNumber { .. }
                | ChoiceActionAst::ChoosePlayer { .. }
                | ChoiceActionAst::ChooseSpellCastHistory { .. },
            ) => {}
            Self::Control(
                ControlActionAst::Attach { .. }
                | ControlActionAst::ControlCombatChoicesThisTurn { .. }
                | ControlActionAst::ControlPlayer { .. }
                | ControlActionAst::Enchant { .. }
                | ControlActionAst::GainControl { .. }
                | ControlActionAst::Unattach { .. },
            ) => {}
            Self::Counters(action) => match action {
                CounterActionAst::EnergyCounters { count: amount }
                | CounterActionAst::ExperienceCounters { count: amount }
                | CounterActionAst::PoisonCounters { count: amount }
                | CounterActionAst::PutCounterChoice { count: amount, .. }
                | CounterActionAst::PutCounters { count: amount, .. }
                | CounterActionAst::PutCountersAll { count: amount, .. }
                | CounterActionAst::PutCountersThenBecomeMonstrous { amount }
                | CounterActionAst::RadCounters { count: amount }
                | CounterActionAst::RemoveCountersAll { amount, .. }
                | CounterActionAst::RemoveUpToAnyCounters { amount, .. }
                | CounterActionAst::TicketCounters { count: amount } => visit(amount)?,
                CounterActionAst::PutOrRemoveCounters {
                    put_count,
                    remove_count,
                    ..
                } => {
                    visit(put_count)?;
                    visit(remove_count)?;
                }
                CounterActionAst::DoubleCountersOnEach { .. }
                | CounterActionAst::DoubleCountersOnTarget { .. }
                | CounterActionAst::ForEachCounterKindPutOrRemove { .. }
                | CounterActionAst::GrowCountersOnChosenHolders
                | CounterActionAst::MoveAllCounters { .. }
                | CounterActionAst::MoveCounters { .. }
                | CounterActionAst::MoveOneCounter { .. }
                | CounterActionAst::NextAdaptIgnoresCounters { .. }
                | CounterActionAst::PayForEachSourceCounter { .. }
                | CounterActionAst::PutCounterOfChosenKind { .. }
                | CounterActionAst::PutCounterOfKindChosenFrom { .. }
                | CounterActionAst::PutCounterOfRandomKind { .. }
                | CounterActionAst::PutCountersThenBecomeRenowned { .. } => {}
            },
            Self::Damage(action) => match action {
                DamageActionAst::DealDamage { amount, .. }
                | DamageActionAst::DealDamageBySources { amount, .. }
                | DamageActionAst::DealDamageEach { amount, .. }
                | DamageActionAst::DealDamageToRecipients { amount, .. }
                | DamageActionAst::DealDistributedDamage { amount, .. }
                | DamageActionAst::HealDamage {
                    amount: Some(amount), ..
                } => visit(amount)?,
                DamageActionAst::DealDamageEqualToPower { .. }
                | DamageActionAst::ExcessDamageToController { .. }
                | DamageActionAst::HealDamage { amount: None, .. } => {}
            },
            Self::DamagePrevention(action) => match action {
                DamagePreventionActionAst::PreventDamage { amount, .. }
                | DamagePreventionActionAst::PreventDamageEach { amount, .. }
                | DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget { amount, .. } => visit(amount)?,
                DamagePreventionActionAst::PreventDamageToTargetPutCounters {
                    amount: Some(amount), ..
                } => visit(amount)?,
                DamagePreventionActionAst::AssignNoCombatDamage { .. }
                | DamagePreventionActionAst::PreventAllCombatDamage { .. }
                | DamagePreventionActionAst::PreventAllCombatDamageFromSource { .. }
                | DamagePreventionActionAst::PreventAllCombatDamageFromSourceFilter { .. }
                | DamagePreventionActionAst::PreventAllCombatDamageToPlayers { .. }
                | DamagePreventionActionAst::PreventAllCombatDamageToYou { .. }
                | DamagePreventionActionAst::PreventAllDamageFromSourceFilter { .. }
                | DamagePreventionActionAst::PreventAllDamageToTarget { .. }
                | DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter { .. }
                | DamagePreventionActionAst::PreventDamageToTargetPutCounters { amount: None, .. }
                | DamagePreventionActionAst::PreventNextTimeDamage { .. }
                | DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController { .. }
                | DamagePreventionActionAst::RedirectAllDamageThisTurnToTarget { .. }
                | DamagePreventionActionAst::RedirectNextTimeDamageToSource { .. }
                | DamagePreventionActionAst::ReplaceNextDamageToTarget { .. } => {}
            },
            Self::Exchanges(
                ExchangeActionAst::ExchangeControl { .. }
                | ExchangeActionAst::ExchangeControlHeterogeneous { .. }
                | ExchangeActionAst::ExchangeLifeTotals { .. }
                | ExchangeActionAst::ExchangeTextBoxes { .. }
                | ExchangeActionAst::ExchangeValues { .. }
                | ExchangeActionAst::ExchangeZones { .. },
            ) => {}
            Self::Game(
                GameActionAst::DrawGame
                | GameActionAst::EndCombatPhase
                | GameActionAst::EndTurn
                | GameActionAst::ExtraTurnAfterTurn { .. }
                | GameActionAst::LoseGame
                | GameActionAst::RedirectEnteringSource { .. }
                | GameActionAst::ReverseTurnOrder
                | GameActionAst::WinGame,
            ) => {}
            Self::Grants(
                GrantActionAst::GrantAbilitiesAll { .. }
                | GrantActionAst::GrantAbilitiesChoiceAll { .. }
                | GrantActionAst::GrantAbilitiesChoiceToTarget { .. }
                | GrantActionAst::GrantAbilitiesToTarget { .. }
                | GrantActionAst::GrantAbilityToSource { .. }
                | GrantActionAst::GrantActivatedAbilitiesFrom { .. }
                | GrantActionAst::GrantActivationCostAlternative { .. }
                | GrantActionAst::GrantBySpec { .. }
                | GrantActionAst::GrantKeywordAbilitiesFrom { .. }
                | GrantActionAst::GrantNextSpellAbilityThisTurn { .. }
                | GrantActionAst::GrantPlayTaggedForAsLongAsExiled { .. }
                | GrantActionAst::GrantPlayTaggedForAsLongAsYouControlSource { .. }
                | GrantActionAst::GrantPlayTaggedUntilEndOfTurn { .. }
                | GrantActionAst::GrantPlayTaggedUntilYourNextTurn { .. }
                | GrantActionAst::GrantPlayTaggedWhileSourceOnBattlefield { .. }
                | GrantActionAst::GrantProtectionChoice { .. }
                | GrantActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn { .. }
                | GrantActionAst::GrantToTarget { .. },
            ) => {}
            Self::KeywordActions(action) => match action {
                KeywordActionAst::CollectEvidence { amount }
                | KeywordActionAst::Connive { count: amount, .. }
                | KeywordActionAst::Discover { count: amount }
                | KeywordActionAst::EmpowerJace { amount }
                | KeywordActionAst::Fateseal { count: amount }
                | KeywordActionAst::Investigate { count: amount }
                | KeywordActionAst::Monstrosity { amount }
                | KeywordActionAst::Populate { count: amount, .. }
                | KeywordActionAst::Proliferate { count: amount }
                | KeywordActionAst::Scry { count: amount }
                | KeywordActionAst::Surveil { count: amount } => visit(amount)?,
                KeywordActionAst::Incubate { amount, count, .. } => {
                    visit(amount)?;
                    visit(count)?;
                }
                KeywordActionAst::ActivateAbilityWithoutCosts { .. }
                | KeywordActionAst::Adapt { .. }
                | KeywordActionAst::Airbend { .. }
                | KeywordActionAst::Amass { .. }
                | KeywordActionAst::BecomeForetold { .. }
                | KeywordActionAst::BecomePlotted { .. }
                | KeywordActionAst::Behold { .. }
                | KeywordActionAst::Bolster { .. }
                | KeywordActionAst::Casualty { .. }
                | KeywordActionAst::Clash { .. }
                | KeywordActionAst::ClearGoad { .. }
                | KeywordActionAst::ClearSuspected { .. }
                | KeywordActionAst::ConniveIterated
                | KeywordActionAst::CopiesAssignedToOpponents
                | KeywordActionAst::CumulativeUpkeep { .. }
                | KeywordActionAst::Detain { .. }
                | KeywordActionAst::Earthbend { .. }
                | KeywordActionAst::EmitKeywordAction { .. }
                | KeywordActionAst::Endure { .. }
                | KeywordActionAst::EnterAttackingReturnedDefender
                | KeywordActionAst::ExileResolvedSpellForCombatCopies
                | KeywordActionAst::Exploit
                | KeywordActionAst::Explore { .. }
                | KeywordActionAst::Fight { .. }
                | KeywordActionAst::FightIterated { .. }
                | KeywordActionAst::FirstResolutionEachTurn { .. }
                | KeywordActionAst::Goad { .. }
                | KeywordActionAst::GrantExileControllerInspection { .. }
                | KeywordActionAst::Learn
                | KeywordActionAst::LinkPermanents { .. }
                | KeywordActionAst::ManifestCardFromHand
                | KeywordActionAst::ManifestDread
                | KeywordActionAst::Meld { .. }
                | KeywordActionAst::MustAttackPlayerThisTurn { .. }
                | KeywordActionAst::OpenAttraction { .. }
                | KeywordActionAst::OptionalSourceCopiesForControllerAndOpponent
                | KeywordActionAst::PairUnpairedCreatures
                | KeywordActionAst::PerformNamedAction { .. }
                | KeywordActionAst::Prepare { .. }
                | KeywordActionAst::Reconfigure { .. }
                | KeywordActionAst::RecordSourceCollection { .. }
                | KeywordActionAst::Regenerate { .. }
                | KeywordActionAst::RegenerateAll { .. }
                | KeywordActionAst::RememberOriginalMovementArrivals { .. }
                | KeywordActionAst::ReturnUnblockedAttackerForCost
                | KeywordActionAst::RingTemptsYou
                | KeywordActionAst::RollPlanarDie
                | KeywordActionAst::RollToVisitAttractions
                | KeywordActionAst::SetPermanentDesignation { .. }
                | KeywordActionAst::Support { .. }
                | KeywordActionAst::Suspect { .. }
                | KeywordActionAst::TakeInitiative
                | KeywordActionAst::TriggerAbilitiesByClass { .. }
                | KeywordActionAst::TriggerAdditionalTime { .. }
                | KeywordActionAst::UnlockRoomDoor
                | KeywordActionAst::UnlockTargetRoomDoor { .. }
                | KeywordActionAst::VentureIntoDungeon { .. } => {}
            },
            Self::Library(action) => match action {
                LibraryActionAst::ExileGraveyardForTotalManaValue { amount }
                | LibraryActionAst::ExileTopOfLibrary { count: amount, .. }
                | LibraryActionAst::Mill { count: amount }
                | LibraryActionAst::MoveToLibraryNthFromTop { position: amount, .. } => visit(amount)?,
                LibraryActionAst::CloakTopCardOfLibrary
                | LibraryActionAst::ConsultTopOfLibrary { .. }
                | LibraryActionAst::DescribedTopCardFaceDown { .. }
                | LibraryActionAst::ManifestTopCardOfLibrary
                | LibraryActionAst::MoveToLibraryTopOrBottomChoice { .. }
                | LibraryActionAst::PutRestOnBottomOfLibrary
                | LibraryActionAst::PutTaggedRemainderInZone { .. }
                | LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary { .. }
                | LibraryActionAst::ReorderGraveyard
                | LibraryActionAst::ReorderGraveyardAtRandom
                | LibraryActionAst::ReorderTopOfLibrary { .. }
                | LibraryActionAst::RetrieveOutsideOrRummage { .. }
                | LibraryActionAst::ShuffleGraveyardIntoLibrary { .. }
                | LibraryActionAst::ShuffleHandAndGraveyardIntoLibrary
                | LibraryActionAst::ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary
                | LibraryActionAst::ShuffleLibrary
                | LibraryActionAst::ShuffleObjectsIntoLibrary { .. } => {}
            },
            Self::LifeResources(action) => match action {
                LifeResourceActionAst::Draw { count: amount }
                | LifeResourceActionAst::GainLife { amount }
                | LifeResourceActionAst::LoseLife { amount }
                | LifeResourceActionAst::PayEnergy { amount }
                | LifeResourceActionAst::PayLife { amount } => visit(amount)?,
                LifeResourceActionAst::DrawForEachTaggedMatching { .. }
                | LifeResourceActionAst::NoteLifeTotal
                | LifeResourceActionAst::PayAnyEnergy { .. }
                | LifeResourceActionAst::PayAnyLife { .. }
                | LifeResourceActionAst::RedistributeLifeTotals => {}
            },
            Self::Mana(action) => match action {
                ManaActionAst::AddManaAnyColor { amount, .. }
                | ManaActionAst::AddManaAnyColorAmong { amount, .. }
                | ManaActionAst::AddManaAnyOneColor { amount }
                | ManaActionAst::AddManaChosenColor { amount, .. }
                | ManaActionAst::AddManaCommanderIdentity { amount }
                | ManaActionAst::AddManaDraftChosenColors { amount, .. }
                | ManaActionAst::AddManaFromLandCouldProduce { amount, .. }
                | ManaActionAst::AddManaLostThisWay { amount, .. }
                | ManaActionAst::AddManaNotedType { amount, .. }
                | ManaActionAst::AddManaScaled { amount, .. } => visit(amount)?,
                ManaActionAst::ActivateManaAbilityOfEachLand
                | ManaActionAst::AddMana { .. }
                | ManaActionAst::AddManaColorsAmong { .. }
                | ManaActionAst::AddManaEqualToManaCost { .. }
                | ManaActionAst::AddManaImprintedColors
                | ManaActionAst::AddOneManaAnyColorAmong { .. }
                | ManaActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn { .. }
                | ManaActionAst::DoubleManaPool
                | ManaActionAst::EmptyManaPool
                | ManaActionAst::PayMana { .. } => {}
            },
            Self::PermanentState(
                PermanentStateActionAst::BecomeBlocked { .. }
                | PermanentStateActionAst::Convert { .. }
                | PermanentStateActionAst::Flip { .. }
                | PermanentStateActionAst::PhaseIn { .. }
                | PermanentStateActionAst::PhaseInAll { .. }
                | PermanentStateActionAst::PhaseOut { .. }
                | PermanentStateActionAst::PhaseOutAll { .. }
                | PermanentStateActionAst::RemoveFromCombat { .. }
                | PermanentStateActionAst::ReselectAttackTarget { .. }
                | PermanentStateActionAst::ScalePowerToughnessAll { .. }
                | PermanentStateActionAst::SwitchPowerToughness { .. }
                | PermanentStateActionAst::Tap { .. }
                | PermanentStateActionAst::TapAll { .. }
                | PermanentStateActionAst::TapOrUntap { .. }
                | PermanentStateActionAst::TapOrUntapAll { .. }
                | PermanentStateActionAst::Transform { .. }
                | PermanentStateActionAst::TurnFaceDown { .. }
                | PermanentStateActionAst::TurnFaceUp { .. }
                | PermanentStateActionAst::Untap { .. }
                | PermanentStateActionAst::UntapAll { .. },
            ) => {}
            Self::Random(
                RandomActionAst::ChooseNumberAtRandom { .. }
                | RandomActionAst::FlipCoin
                | RandomActionAst::FlipCoinFaceOnly
                | RandomActionAst::FlipCoins { .. }
                | RandomActionAst::RollDiceChooseResult { .. }
                | RandomActionAst::RollDiceTotal { .. }
                | RandomActionAst::RollDie { .. },
            ) => {}
            Self::Replacements(
                ReplacementActionAst::RegisterCounterPlacementReplacement { .. }
                | ReplacementActionAst::RegisterDamageAddition { .. }
                | ReplacementActionAst::RegisterDamageMultiplier { .. }
                | ReplacementActionAst::RegisterDamagedBySourceZoneReplacement { .. }
                | ReplacementActionAst::RegisterDrawReplacement { .. }
                | ReplacementActionAst::RegisterEnterTappedReplacement { .. }
                | ReplacementActionAst::RegisterEnterUnderControlReplacement { .. }
                | ReplacementActionAst::RegisterEnterWithCountersReplacement { .. }
                | ReplacementActionAst::RegisterFutureZoneReplacement { .. }
                | ReplacementActionAst::RegisterManaReplacement { .. }
                | ReplacementActionAst::RegisterManaRewrite { .. }
                | ReplacementActionAst::RegisterManaSpendPermission { .. }
                | ReplacementActionAst::RegisterNextBatchEnterAsCopy { .. }
                | ReplacementActionAst::RegisterNextBatchEnterWithCounters { .. }
                | ReplacementActionAst::RegisterTokenCreationMultiplier { .. }
                | ReplacementActionAst::RegisterZoneReplacement { .. },
            ) => {}
            Self::RevealLook(action) => match action {
                RevealLookActionAst::LookAtTopCards { count: amount, .. } => visit(amount)?,
                RevealLookActionAst::LookAtHand { .. }
                | RevealLookActionAst::LookAtObjects { .. }
                | RevealLookActionAst::LookAtTarget { .. }
                | RevealLookActionAst::RevealCardsFromHand { .. }
                | RevealLookActionAst::RevealHand
                | RevealLookActionAst::RevealTagged { .. }
                | RevealLookActionAst::RevealTop => {}
            },
            Self::Stack(action) => match action {
                StackActionAst::CopySpell { count: amount, .. } => visit(amount)?,
                StackActionAst::CastTagged { .. }
                | StackActionAst::CopySpellForEachTarget { .. }
                | StackActionAst::Counter { .. }
                | StackActionAst::CounterUnlessPays { .. }
                | StackActionAst::FreeCastNextSpellThisTurn { .. }
                | StackActionAst::ReduceMatchingSpellCostThisTurn { .. }
                | StackActionAst::ReduceNextSpellCostThisTurn { .. }
                | StackActionAst::RetargetStackObject { .. }
                | StackActionAst::ScaleXValue { .. } => {}
            },
            Self::StatChanges(action) => match action {
                StatChangeActionAst::Pump { power, toughness, .. }
                | StatChangeActionAst::PumpAll { power, toughness, .. } => {
                    visit(power)?;
                    visit(toughness)?;
                }
                StatChangeActionAst::PumpForEach { count, .. } => visit(count)?,
                StatChangeActionAst::MakeColorless { .. }
                | StatChangeActionAst::PumpByLastEffect { .. }
                | StatChangeActionAst::RemoveAbilitiesAll { .. }
                | StatChangeActionAst::RemoveAbilitiesFromTarget { .. }
                | StatChangeActionAst::RemoveAllSubtypesOfFamily { .. }
                | StatChangeActionAst::RemoveCardTypes { .. }
                | StatChangeActionAst::RemoveSubtypes { .. }
                | StatChangeActionAst::RemoveSupertypes { .. } => {}
            },
            Self::Tokens(action) => match action {
                TokenActionAst::CreateTokenCopy { count: amount, .. } => visit(amount)?,
                TokenActionAst::CreateTokenWithMods {
                    count: amount,
                    dynamic_power_toughness,
                    ..
                } => {
                    visit(amount)?;
                    if let Some((power, toughness)) = dynamic_power_toughness {
                        visit(power)?;
                        visit(toughness)?;
                    }
                }
                TokenActionAst::CreateEmblem { .. } | TokenActionAst::CreateTokenChoice { .. } => {}
            },
            Self::TurnStructure(action) => match action {
                TurnStructureActionAst::AdditionalLandPlays { count: amount, .. }
                | TurnStructureActionAst::AdditionalSteps { count: amount, .. } => visit(amount)?,
                TurnStructureActionAst::AdditionalPhases { .. }
                | TurnStructureActionAst::SkipCombatPhases
                | TurnStructureActionAst::SkipCombatPhasesThisTurn
                | TurnStructureActionAst::SkipDrawStep
                | TurnStructureActionAst::SkipMainPhasesThisTurn
                | TurnStructureActionAst::SkipNextCombatPhaseThisTurn
                | TurnStructureActionAst::SkipScheduled { .. }
                | TurnStructureActionAst::SkipTurn => {}
            },
            Self::ZoneMoves(action) => match action {
                ZoneMoveActionAst::Discard { count: amount, .. } => visit(amount)?,
                ZoneMoveActionAst::Destroy { .. }
                | ZoneMoveActionAst::DestroyAll { .. }
                | ZoneMoveActionAst::DestroyAllAttachedTo { .. }
                | ZoneMoveActionAst::DestroyAllOfChosenColor { .. }
                | ZoneMoveActionAst::DiscardHand
                | ZoneMoveActionAst::ExchangeSourceWithHandAura
                | ZoneMoveActionAst::Exile { .. }
                | ZoneMoveActionAst::ExileAll { .. }
                | ZoneMoveActionAst::ExileAllAttachedTo { .. }
                | ZoneMoveActionAst::ExileInsteadOfGraveyardThisTurn
                | ZoneMoveActionAst::ExileUntilSourceLeaves { .. }
                | ZoneMoveActionAst::ExileWhenSourceLeaves { .. }
                | ZoneMoveActionAst::MayMoveToZone { .. }
                | ZoneMoveActionAst::MoveToZone { .. }
                | ZoneMoveActionAst::PlayFromGraveyardUntilEot
                | ZoneMoveActionAst::PutOntoBattlefield { .. }
                | ZoneMoveActionAst::ReturnAllToBattlefield { .. }
                | ZoneMoveActionAst::ReturnAllToHand { .. }
                | ZoneMoveActionAst::ReturnAllToHandOfChosenColor { .. }
                | ZoneMoveActionAst::ReturnSourceTransformedFromExile
                | ZoneMoveActionAst::ReturnSourceWithHasteAndExileDeadline
                | ZoneMoveActionAst::ReturnToBattlefield { .. }
                | ZoneMoveActionAst::ReturnToHand { .. }
                | ZoneMoveActionAst::Sacrifice { .. }
                | ZoneMoveActionAst::SacrificeAll { .. }
                | ZoneMoveActionAst::SacrificeSourceWhenLeaves { .. }
                | ZoneMoveActionAst::SearchLibrary { .. }
                | ZoneMoveActionAst::SearchLibrarySlotsToHand { .. } => {}
            },
            Self::Cant { .. }
            | Self::PutSticker { .. }
            | Self::ReorderTopPlanarDeck { .. }
            | Self::TagMatchingObjects { .. }
            | Self::TargetOnly { .. } => {}
        }
        Ok(())
    }
}

use super::EffectAst;

// Keep the list of wrapper variants with `effects: Vec<EffectAst>` in one place.
// This avoids drift between immutable/mutable/fallible traversal helpers.
macro_rules! nested_effects_variants {
    ($effects:ident) => {
        EffectAst::UnlessPays {
            effects: $effects,
            ..
        } | EffectAst::May { effects: $effects }
            | EffectAst::MayByPlayer {
                effects: $effects,
                ..
            }
            | EffectAst::MayByTaggedController {
                effects: $effects,
                ..
            }
            | EffectAst::IfResult {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachOpponent { effects: $effects }
            | EffectAst::ForEachPlayersFiltered {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachPlayer { effects: $effects }
            | EffectAst::ForEachTargetPlayers {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachObject {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachTagged {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachOpponentDoesNot { effects: $effects }
            | EffectAst::ForEachPlayerDoesNot { effects: $effects }
            | EffectAst::ForEachOpponentDid {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachPlayerDid {
                effects: $effects,
                ..
            }
            | EffectAst::ForEachTaggedPlayer {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilNextEndStep {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilNextUpkeep {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilEndStepOfExtraTurn {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedUntilEndOfCombat { effects: $effects }
            | EffectAst::DelayedTriggerThisTurn {
                effects: $effects,
                ..
            }
            | EffectAst::DelayedWhenLastObjectDiesThisTurn {
                effects: $effects,
                ..
            }
            | EffectAst::VoteOption {
                effects: $effects,
                ..
            }
    };
}

pub(super) fn assert_effect_ast_variant_coverage(effect: &EffectAst) {
    match effect {
        EffectAst::DealDamage { .. } => {}
        EffectAst::DealDamageEqualToPower { .. } => {}
        EffectAst::Fight { .. } => {}
        EffectAst::FightIterated { .. } => {}
        EffectAst::Clash { .. } => {}
        EffectAst::DealDamageEach { .. } => {}
        EffectAst::Draw { .. } => {}
        EffectAst::Counter { .. } => {}
        EffectAst::CounterUnlessPays { .. } => {}
        EffectAst::UnlessPays { .. } => {}
        EffectAst::UnlessAction { .. } => {}
        EffectAst::PutCounters { .. } => {}
        EffectAst::PutOrRemoveCounters { .. } => {}
        EffectAst::ForEachCounterKindPutOrRemove { .. } => {}
        EffectAst::PutCountersAll { .. } => {}
        EffectAst::DoubleCountersOnEach { .. } => {}
        EffectAst::Proliferate => {}
        EffectAst::Tap { .. } => {}
        EffectAst::TapAll { .. } => {}
        EffectAst::Untap { .. } => {}
        EffectAst::RemoveFromCombat { .. } => {}
        EffectAst::TapOrUntap { .. } => {}
        EffectAst::UntapAll { .. } => {}
        EffectAst::LoseLife { .. } => {}
        EffectAst::GainLife { .. } => {}
        EffectAst::LoseGame { .. } => {}
        EffectAst::WinGame { .. } => {}
        EffectAst::PreventAllCombatDamage { .. } => {}
        EffectAst::PreventAllCombatDamageFromSource { .. } => {}
        EffectAst::PreventAllCombatDamageToPlayers { .. } => {}
        EffectAst::PreventAllCombatDamageToYou { .. } => {}
        EffectAst::PreventDamage { .. } => {}
        EffectAst::PreventAllDamageToTarget { .. } => {}
        EffectAst::PreventNextTimeDamage { .. } => {}
        EffectAst::RedirectNextDamageFromSourceToTarget { .. } => {}
        EffectAst::RedirectNextTimeDamageToSource { .. } => {}
        EffectAst::PreventDamageEach { .. } => {}
        EffectAst::GrantProtectionChoice { .. } => {}
        EffectAst::Earthbend { .. } => {}
        EffectAst::Explore { .. } => {}
        EffectAst::OpenAttraction => {}
        EffectAst::ManifestDread => {}
        EffectAst::Bolster { .. } => {}
        EffectAst::Support { .. } => {}
        EffectAst::Adapt { .. } => {}
        EffectAst::CounterActivatedOrTriggeredAbility => {}
        EffectAst::AddMana { .. } => {}
        EffectAst::AddManaScaled { .. } => {}
        EffectAst::AddManaAnyColor { .. } => {}
        EffectAst::AddManaAnyOneColor { .. } => {}
        EffectAst::AddManaChosenColor { .. } => {}
        EffectAst::AddManaFromLandCouldProduce { .. } => {}
        EffectAst::AddManaCommanderIdentity { .. } => {}
        EffectAst::AddManaImprintedColors => {}
        EffectAst::Scry { .. } => {}
        EffectAst::Discover { .. } => {}
        EffectAst::BecomeBasicLandTypeChoice { .. } => {}
        EffectAst::BecomeCreatureTypeChoice { .. } => {}
        EffectAst::BecomeColorChoice { .. } => {}
        EffectAst::Surveil { .. } => {}
        EffectAst::PayMana { .. } => {}
        EffectAst::PayEnergy { .. } => {}
        EffectAst::Cant { .. } => {}
        EffectAst::PlayFromGraveyardUntilEot { .. } => {}
        EffectAst::GrantPlayTaggedUntilEndOfTurn { .. } => {}
        EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn { .. } => {}
        EffectAst::GrantPlayTaggedUntilYourNextTurn { .. } => {}
        EffectAst::CastTagged { .. } => {}
        EffectAst::ExileInsteadOfGraveyardThisTurn { .. } => {}
        EffectAst::GainControl { .. } => {}
        EffectAst::ControlPlayer { .. } => {}
        EffectAst::ExtraTurnAfterTurn { .. } => {}
        EffectAst::DelayedUntilNextEndStep { .. } => {}
        EffectAst::DelayedUntilNextUpkeep { .. } => {}
        EffectAst::DelayedUntilEndStepOfExtraTurn { .. } => {}
        EffectAst::DelayedUntilEndOfCombat { .. } => {}
        EffectAst::DelayedTriggerThisTurn { .. } => {}
        EffectAst::DelayedWhenLastObjectDiesThisTurn { .. } => {}
        EffectAst::RevealTop { .. } => {}
        EffectAst::RevealTopChooseCardTypePutToHandRestBottom { .. } => {}
        EffectAst::RevealTagged { .. } => {}
        EffectAst::LookAtTopCards { .. } => {}
        EffectAst::RevealHand { .. } => {}
        EffectAst::PutIntoHand { .. } => {}
        EffectAst::PutSomeIntoHandRestIntoGraveyard { .. } => {}
        EffectAst::PutSomeIntoHandRestOnBottomOfLibrary { .. } => {}
        EffectAst::PutRestOnBottomOfLibrary => {}
        EffectAst::CopySpell { .. } => {}
        EffectAst::RetargetStackObject { .. } => {}
        EffectAst::Conditional { .. } => {}
        EffectAst::ChooseObjects { .. } => {}
        EffectAst::Sacrifice { .. } => {}
        EffectAst::SacrificeAll { .. } => {}
        EffectAst::DiscardHand { .. } => {}
        EffectAst::Discard { .. } => {}
        EffectAst::Connive { .. } => {}
        EffectAst::ConniveIterated => {}
        EffectAst::Goad { .. } => {}
        EffectAst::Transform { .. } => {}
        EffectAst::Flip { .. } => {}
        EffectAst::Regenerate { .. } => {}
        EffectAst::RegenerateAll { .. } => {}
        EffectAst::Mill { .. } => {}
        EffectAst::ReturnToHand { .. } => {}
        EffectAst::ReturnToBattlefield { .. } => {}
        EffectAst::MoveToZone { .. } => {}
        EffectAst::MoveToLibrarySecondFromTop { .. } => {}
        EffectAst::ReturnAllToHand { .. } => {}
        EffectAst::ReturnAllToHandOfChosenColor { .. } => {}
        EffectAst::ReturnAllToBattlefield { .. } => {}
        EffectAst::ExchangeControl { .. } => {}
        EffectAst::SetLifeTotal { .. } => {}
        EffectAst::SkipTurn { .. } => {}
        EffectAst::SkipCombatPhases { .. } => {}
        EffectAst::SkipNextCombatPhaseThisTurn { .. } => {}
        EffectAst::SkipDrawStep { .. } => {}
        EffectAst::PoisonCounters { .. } => {}
        EffectAst::EnergyCounters { .. } => {}
        EffectAst::May { .. } => {}
        EffectAst::MayByPlayer { .. } => {}
        EffectAst::MayByTaggedController { .. } => {}
        EffectAst::IfResult { .. } => {}
        EffectAst::ForEachOpponent { .. } => {}
        EffectAst::ForEachPlayersFiltered { .. } => {}
        EffectAst::ForEachPlayer { .. } => {}
        EffectAst::ForEachTargetPlayers { .. } => {}
        EffectAst::ForEachObject { .. } => {}
        EffectAst::ForEachTagged { .. } => {}
        EffectAst::ForEachOpponentDoesNot { .. } => {}
        EffectAst::ForEachPlayerDoesNot { .. } => {}
        EffectAst::ForEachOpponentDid { .. } => {}
        EffectAst::ForEachPlayerDid { .. } => {}
        EffectAst::ForEachTaggedPlayer { .. } => {}
        EffectAst::Enchant { .. } => {}
        EffectAst::Attach { .. } => {}
        EffectAst::Investigate { .. } => {}
        EffectAst::Amass { .. } => {}
        EffectAst::Destroy { .. } => {}
        EffectAst::DestroyNoRegeneration { .. } => {}
        EffectAst::DestroyAll { .. } => {}
        EffectAst::DestroyAllNoRegeneration { .. } => {}
        EffectAst::DestroyAllOfChosenColor { .. } => {}
        EffectAst::DestroyAllOfChosenColorNoRegeneration { .. } => {}
        EffectAst::DestroyAllAttachedTo { .. } => {}
        EffectAst::Exile { .. } => {}
        EffectAst::ExileWhenSourceLeaves { .. } => {}
        EffectAst::SacrificeSourceWhenLeaves { .. } => {}
        EffectAst::ExileUntilSourceLeaves { .. } => {}
        EffectAst::ExileAll { .. } => {}
        EffectAst::LookAtHand { .. } => {}
        EffectAst::TargetOnly { .. } => {}
        EffectAst::CreateToken { .. } => {}
        EffectAst::CreateTokenCopy { .. } => {}
        EffectAst::CreateTokenCopyFromSource { .. } => {}
        EffectAst::CreateTokenWithMods { .. } => {}
        EffectAst::ExileThatTokenAtEndOfCombat => {}
        EffectAst::SacrificeThatTokenAtEndOfCombat => {}
        EffectAst::Monstrosity { .. } => {}
        EffectAst::RemoveUpToAnyCounters { .. } => {}
        EffectAst::RemoveCountersAll { .. } => {}
        EffectAst::MoveAllCounters { .. } => {}
        EffectAst::Pump { .. } => {}
        EffectAst::SwitchPowerToughness { .. } => {}
        EffectAst::SetBasePowerToughness { .. } => {}
        EffectAst::BecomeBasePtCreature { .. } => {}
        EffectAst::AddCardTypes { .. } => {}
        EffectAst::AddSubtypes { .. } => {}
        EffectAst::SetColors { .. } => {}
        EffectAst::MakeColorless { .. } => {}
        EffectAst::SetBasePower { .. } => {}
        EffectAst::PumpForEach { .. } => {}
        EffectAst::PumpAll { .. } => {}
        EffectAst::PumpByLastEffect { .. } => {}
        EffectAst::GrantAbilitiesAll { .. } => {}
        EffectAst::RemoveAbilitiesAll { .. } => {}
        EffectAst::GrantAbilitiesChoiceAll { .. } => {}
        EffectAst::GrantAbilitiesToTarget { .. } => {}
        EffectAst::RemoveAbilitiesFromTarget { .. } => {}
        EffectAst::GrantAbilitiesChoiceToTarget { .. } => {}
        EffectAst::GrantAbilityToSource { .. } => {}
        EffectAst::SearchLibrary { .. } => {}
        EffectAst::ShuffleGraveyardIntoLibrary { .. } => {}
        EffectAst::ReorderGraveyard { .. } => {}
        EffectAst::ReorderTopOfLibrary { .. } => {}
        EffectAst::ShuffleLibrary { .. } => {}
        EffectAst::VoteStart { .. } => {}
        EffectAst::VoteOption { .. } => {}
        EffectAst::VoteExtra { .. } => {}
        EffectAst::TokenCopyHasHaste => {}
        EffectAst::TokenCopyGainHasteUntilEot => {}
        EffectAst::TokenCopySacrificeAtNextEndStep => {}
        EffectAst::TokenCopyExileAtNextEndStep => {}
    }
}

pub(super) fn for_each_nested_effects(
    effect: &EffectAst,
    include_unless_action_alternative: bool,
    mut visit: impl FnMut(&[EffectAst]),
) {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            visit(if_true);
            visit(if_false);
        }
        nested_effects_variants!(effects) => {
            visit(effects);
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            visit(effects);
            if include_unless_action_alternative {
                visit(alternative);
            }
        }
        _ => {}
    }
}

pub(super) fn for_each_nested_effects_mut(
    effect: &mut EffectAst,
    include_unless_action_alternative: bool,
    mut visit: impl FnMut(&mut [EffectAst]),
) {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            visit(if_true);
            visit(if_false);
        }
        nested_effects_variants!(effects) => {
            visit(effects);
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            visit(effects);
            if include_unless_action_alternative {
                visit(alternative);
            }
        }
        _ => {}
    }
}

pub(super) fn try_for_each_nested_effects_mut<E>(
    effect: &mut EffectAst,
    include_unless_action_alternative: bool,
    mut visit: impl FnMut(&mut [EffectAst]) -> Result<(), E>,
) -> Result<(), E> {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            visit(if_true)?;
            visit(if_false)?;
        }
        nested_effects_variants!(effects) => {
            visit(effects)?;
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            visit(effects)?;
            if include_unless_action_alternative {
                visit(alternative)?;
            }
        }
        _ => {}
    }
    Ok(())
}

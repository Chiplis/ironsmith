use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum PredicateAst {
    ItIsNight,
    FirstCombatPhaseOfTurn,
    SourceControllersMainPhase,
    ItIsLandCard,
    ItIsSoulbondPaired,
    SourceChosenOption(String),
    ItMatches(ObjectFilter),
    /// The implicit object matched this filter immediately before its zone change.
    ///
    /// This is distinct from `ItMatches`: phrases such as "if it was a creature"
    /// use last-known information and must not become true from the object's
    /// characteristics in its new zone.
    ItMatchedLastKnown(ObjectFilter),
    TargetMatches(ObjectFilter),
    TaggedMatches(TagKey, ObjectFilter),
    TaggedWasCast(TagKey),
    EnchantedPermanentAttackedThisTurn,
    EnchantedPermanentAttackedOrBlockedSinceLastUpkeep,
    SourceBlockedOrBecameBlockedSinceLastUpkeep,
    TargetObjectsHaveDifferentColorSets,
    PlayerTaggedObjectMatches {
        player: PlayerAst,
        tag: TagKey,
        filter: ObjectFilter,
        mode: ironsmith_core::TaggedObjectMatchMode,
    },
    PlayerTaggedObjectEnteredBattlefieldThisTurn {
        player: PlayerAst,
        tag: TagKey,
    },
    PlayerControls {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerHasAtLeast {
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerControlsExactly {
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerHasAtLeastWithDifferentPowers {
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerControlsOrHasCardInGraveyard {
        player: PlayerAst,
        control_filter: ObjectFilter,
        graveyard_filter: ObjectFilter,
    },
    PlayerOwnsCardNamedInZones {
        player: PlayerAst,
        name: String,
        zones: Vec<Zone>,
    },
    PlayerControlsNo {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerControlsMost {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerControlsMoreThanEachOtherPlayer {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    AnOpponentHasFewerThanPlayer {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerControlsMoreThanYou {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerLifeAtMostHalfStartingLifeTotal {
        player: PlayerAst,
    },
    PlayerLifeLessThanHalfStartingLifeTotal {
        player: PlayerAst,
    },
    PlayerHasLessLifeThanYou {
        player: PlayerAst,
    },
    PlayerHasMoreLifeThanYou {
        player: PlayerAst,
    },
    PlayerHasNoOpponentWithMoreLifeThan {
        player: PlayerAst,
    },
    PlayerHasMoreLifeThanEachOtherPlayer {
        player: PlayerAst,
    },
    CountParity {
        count: crate::static_abilities::AnthemCountExpression,
        even: bool,
        display: Option<String>,
    },
    PlayerIsMonarch {
        player: PlayerAst,
    },
    PlayerHasInitiative {
        player: PlayerAst,
    },
    PlayerHasCitysBlessing {
        player: PlayerAst,
    },
    SourceIsRingBearer {
        player: PlayerAst,
    },
    PlayerRingTemptedThisGameOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCompletedDungeon {
        player: PlayerAst,
        dungeon_name: Option<String>,
    },
    PlayerTappedLandForManaThisTurn {
        player: PlayerAst,
    },
    PlayerGainedLifeThisTurnOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerHadLandEnterBattlefieldThisTurn {
        player: PlayerAst,
    },
    PlayerDescendedThisTurn {
        player: PlayerAst,
    },
    PlayerControlsBasicLandTypesAmongLandsOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerHasCardTypesInGraveyardOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandOrFewer {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandAtTurnStartOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandAtTurnStartOrFewer {
        player: PlayerAst,
        count: u32,
    },
    PlayerHasMoreCardsInHandThanYou {
        player: PlayerAst,
    },
    PlayerHasMoreCardsInHandThanEachOtherPlayer {
        player: PlayerAst,
    },
    PlayerHasPoisonCountersOrMore {
        player: PlayerAst,
        count: u32,
    },
    VoteOptionGetsMoreVotes {
        option: String,
    },
    SecretChoicesMatch,
    VoteOptionGetsMoreVotesOrTied {
        option: String,
    },
    NoVoteObjectsMatched {
        filter: ObjectFilter,
    },
    PlayerCastSpellsThisTurnOrMore {
        player: PlayerAst,
        count: u32,
    },
    OpponentLostLifeThisTurn,
    AnyPlayerLostLifeThisTurnOrMore {
        count: u32,
    },
    OpponentWasDealtDamageThisTurn,
    YouHaveNoCardsInHand,
    PlayerWouldDrawCard {
        player: PlayerAst,
    },
    PlayerWouldProliferate {
        player: PlayerAst,
    },
    PlayerWouldBeginExtraTurn {
        player: PlayerAst,
    },
    SourceIsTapped,
    SourceIsEquipped,
    SourceIsEnchanted,
    SourceIsSaddled,
    SourceIsRenowned,
    SourceCrewedByExactly {
        count: u32,
        filter: ObjectFilter,
    },
    SourceMatches(ObjectFilter),
    /// The battlefield object this Aura or Equipment source is attached to
    /// matches the filter at the time the trigger is checked.
    AttachedToSourceMatches(ObjectFilter),
    /// The object in the surrounding tap event is becoming tapped for the
    /// first time this turn. This is per object, not per triggered ability.
    TriggeringObjectBecameTappedFirstTimeThisTurn,
    /// The object in the surrounding counter event is receiving counters for
    /// the first time this turn. This is per object, not per triggered
    /// ability.
    TriggeringObjectHadCountersPutFirstTimeThisTurn,
    TriggeringObjectHadToAttackThisCombat,

    SourceHasNoCounter(CounterType),
    TriggeringObjectHadNoCounter(CounterType),
    TriggeringObjectHadCounterAtLeast {
        counter_type: CounterType,
        count: u32,
    },
    SourceHasCounterAtLeast {
        counter_type: CounterType,
        count: u32,
        surface: crate::SourceCounterThresholdSurface,
    },
    SourceHasCountersAtLeast(u32),
    SourceHasAttachmentsMatching {
        filter: ObjectFilter,
        comparison: crate::effect::Comparison,
        display: String,
    },
    SourcePowerAtLeast(u32),
    SourceDealtCombatDamageToPlayerThisTurn,
    PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
        player: PlayerAst,
        subtype: Subtype,
    },
    SourceAttackedThisTurn,
    SourceSuspected,
    SourceCameUnderYourControlThisTurn,
    SourceAttackedOrBlockedThisTurn,
    SourceInGraveyardWithCardsAbove {
        filter: ObjectFilter,
        count: u32,
    },
    SourceIsInZone(Zone),
    YourTurn,
    YouAttackedWithExactlyNOtherCreaturesThisCombat(u32),
    CreatureDiedThisTurn,
    CreatureDiedThisTurnOrMore(u32),
    CreatureDealtDamageBySourceDiedThisTurn {
        victim: ObjectFilter,
        damager: DamageBySpec,
        count: u32,
    },
    CreatureCardPutIntoYourGraveyardThisTurn,
    PermanentLeftBattlefieldThisTurn,
    NonlandPermanentLeftBattlefieldThisTurn,
    SpellWasWarpedThisTurn,
    PermanentLeftBattlefieldUnderYourControlThisTurn {
        surface: crate::PermanentLeftBattlefieldControlSurface,
    },
    ObjectEnteredBattlefieldThisTurn(ObjectFilter),
    ObjectEnteredBattlefieldLastTurn(ObjectFilter),
    ObjectPutIntoGraveyardFromBattlefieldThisTurn(ObjectFilter),
    YouHaveFullParty,
    YouAttackedThisTurn,
    YouAttackedWithNOrMoreCreaturesThisTurn(u32),
    SourceWasCast,
    ThisSpellWasCastAtSorceryTiming,
    ThisSpellEscaped,
    NoSpellsWereCastLastTurn,
    ThisSpellWasKicked,
    ThisSpellPaidLabel(OptionalCostRef),
    TargetWasKicked,
    ThisAbilityResolvedThisTurnExactly(u32),
    TargetSpellCastOrderThisTurn(u32),
    TargetSpellControllerIsPoisoned,
    TargetSpellNoManaSpentToCast,
    YouControlMoreCreaturesThanTargetSpellController,
    TargetIsBlocked,
    TargetHasGreatestPowerAmongCreatures,
    TargetManaValueLteColorsSpentToCastThisSpell,
    ManaSpentToCastThisSpellAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },
    TriggeringSpellManaSpentToCastAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },
    ColoredManaSpentToCastThisSpellAtLeast(u32),
    TriggeringSpellColoredManaSpentToCastAtLeast(u32),
    SnowManaOfAnySpellColorSpentToCastThisSpell,
    SameColorManaSpentToCastThisSpellAtLeast(u32),
    ThisSpellWasCastFromZone(Zone),
    ThisSpellWasCastFromNonHand,
    TurnHistory(TurnHistoryPredicateAst),
    ValueComparison {
        left: Value,
        operator: crate::effect::ValueComparisonOperator,
        right: Value,
    },
    Not(Box<PredicateAst>),
    And(Box<PredicateAst>, Box<PredicateAst>),
    Or(Box<PredicateAst>, Box<PredicateAst>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TurnHistoryPredicateAst {
    SpellsCastLastTurnAtLeast(u32),
    SourceCrewedByAtLeast {
        count: u32,
        filter: ObjectFilter,
    },
    SourceWasCast {
        surface: SourceReferenceSurface,
    },
    SourceWasCastByController {
        surface: SourceReferenceSurface,
    },
    SourceWasKicked {
        surface: SourceReferenceSurface,
    },
    SourceEnteredBattlefieldThisTurn {
        surface: SourceReferenceSurface,
    },
    SourceAttackedThisTurn {
        surface: SourceReferenceSurface,
    },
    TriggeringObjectEnlistedThisCombat,
    TriggeringObjectWasCast,
    TriggeringObjectWasCastFromZone(Zone),
    PlayerPlayedLandThisTurn(PlayerAst),
    TriggeringObjectDied,
    PlayerPlayedCardFromZoneThisTurn {
        player: PlayerAst,
        zone: Zone,
    },
    PlayerCastSpellFromZoneThisTurn {
        player: PlayerAst,
        zone: Zone,
    },
    PlayerActivatedAbilityOfCardInZoneThisTurn {
        player: PlayerAst,
        zone: Zone,
    },
    PlayerVisitedAttractionThisTurn(PlayerAst),
    TriggeringPlayerAttackedControllerLastTurn,
    PlayerLostLifeLastTurn(PlayerAst),
    TriggeringPlayersTurn {
        definite_player: bool,
    },
    ControllerTeamGainedLifeThisTurn,
    TriggeringObjectsNoneWereCastOrNoManaSpent,
    ManaFromSourceSpentOnTriggeringAction {
        source_filter: ObjectFilter,
    },
    AllPlayersLifeAtMost(i32),
    AnotherOpponentControlsPotentialTarget {
        filter: ObjectFilter,
    },
    TriggeringAttackerBlockers {
        required: ObjectFilter,
        required_count: u32,
        prohibited: ObjectFilter,
    },
    TriggeringAbilityIsManaAbility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateReferenceAntecedent {
    SourceObject,
}

impl PredicateAst {
    /// Whether this predicate evaluates the object denoted by an implicit
    /// `it`/`that object` reference supplied by the surrounding effect.
    pub fn uses_implicit_object_reference(&self) -> bool {
        match self {
            PredicateAst::ItIsLandCard
            | PredicateAst::ItIsSoulbondPaired
            | PredicateAst::ItMatches(_)
            | PredicateAst::ItMatchedLastKnown(_)
            | PredicateAst::TargetMatches(_) => true,
            PredicateAst::Not(inner) => inner.uses_implicit_object_reference(),
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
                left.uses_implicit_object_reference() || right.uses_implicit_object_reference()
            }
            _ => false,
        }
    }

    pub fn reference_antecedent(&self) -> Option<PredicateReferenceAntecedent> {
        match self {
            PredicateAst::SourceChosenOption(_)
            | PredicateAst::SourceIsTapped
            | PredicateAst::SourceIsEquipped
            | PredicateAst::SourceIsEnchanted
            | PredicateAst::SourceIsSaddled
            | PredicateAst::SourceIsRenowned
            | PredicateAst::SourceCrewedByExactly { .. }
            | PredicateAst::SourceMatches(_)
            | PredicateAst::AttachedToSourceMatches(_)
            | PredicateAst::SourceHasNoCounter(_)
            | PredicateAst::SourceHasCounterAtLeast { .. }
            | PredicateAst::SourceHasCountersAtLeast(_)
            | PredicateAst::SourceHasAttachmentsMatching { .. }
            | PredicateAst::SourcePowerAtLeast(_)
            | PredicateAst::SourceAttackedThisTurn
            | PredicateAst::SourceSuspected
            | PredicateAst::SourceCameUnderYourControlThisTurn
            | PredicateAst::SourceAttackedOrBlockedThisTurn
            | PredicateAst::SourceInGraveyardWithCardsAbove { .. }
            | PredicateAst::SourceIsInZone(_)
            | PredicateAst::SourceWasCast
            | PredicateAst::ThisSpellWasCastAtSorceryTiming
            | PredicateAst::ThisSpellEscaped
            | PredicateAst::ThisSpellWasKicked
            | PredicateAst::ThisSpellPaidLabel(_)
            | PredicateAst::ThisSpellWasCastFromZone(_)
            | PredicateAst::ThisSpellWasCastFromNonHand
            | PredicateAst::TurnHistory(
                TurnHistoryPredicateAst::SourceCrewedByAtLeast { .. }
                | TurnHistoryPredicateAst::SourceWasCast { .. }
                | TurnHistoryPredicateAst::SourceWasCastByController { .. }
                | TurnHistoryPredicateAst::SourceWasKicked { .. }
                | TurnHistoryPredicateAst::SourceEnteredBattlefieldThisTurn { .. }
                | TurnHistoryPredicateAst::SourceAttackedThisTurn { .. },
            ) => Some(PredicateReferenceAntecedent::SourceObject),
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => left
                .reference_antecedent()
                .or_else(|| right.reference_antecedent()),
            PredicateAst::Not(inner) => inner.reference_antecedent(),
            _ => None,
        }
    }

    pub fn establishes_source_object_antecedent(&self) -> bool {
        matches!(
            self.reference_antecedent(),
            Some(PredicateReferenceAntecedent::SourceObject)
        )
    }
}

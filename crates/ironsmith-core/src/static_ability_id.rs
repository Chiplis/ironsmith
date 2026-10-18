//! Static ability identity enum.
//!
//! This enum provides unique identifiers for each type of static ability.

/// Unique identifier for each type of static ability.
use crate::tag::TagKeyWalk;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TagKeyWalk)]
pub enum StaticAbilityId {
    Flying,
    FirstStrike,
    DoubleStrike,
    Deathtouch,
    Defender,
    Flash,
    Haste,
    Hexproof,
    HexproofFrom,
    Indestructible,
    Intimidate,
    Lifelink,
    Menace,
    Banding,
    BandsWithOther,
    Protection,
    Reach,
    Shroud,
    Trample,
    Vigilance,
    Ward,
    Fear,
    Skulk,
    Prowess,
    Flanking,
    UmbraArmor,
    Landwalk,
    CantBeBlockedAsLongAsDefendingPlayerControlsCardType,
    CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes,
    CantBeBlockedWhileDefendingPlayerControlsMostCreatures,
    Bloodthirst,
    Tribute,
    Daybound,
    Nightbound,
    DayNightStartsDayAsEnters,
    Morph,
    Disguise,
    Megamorph,
    Shadow,
    Horsemanship,
    Phasing,
    Wither,
    Infect,
    Changeling,
    LivingMetal,
    Companion,
    Partner,
    PartnerWith,
    StartYourEngines,
    SpaceSculptor,
    DoctorsCompanion,
    Assist,
    Ascend,
    /// "Storied": if you control three or more artifacts, legendaries, and/or
    /// Sagas, you have an enduring story for the rest of the game.
    Storied,
    SplitSecond,
    Rebound,
    Cascade,
    CascadeLandDrop,
    Splice,
    Escalate,
    ReadAhead,
    Unleash,
    ConditionalSpellKeyword,
    ThisSpellCastRestriction,
    ThisSpellXMaximum,
    ThisSpellXMinimum,
    Unblockable,
    FlyingRestriction,
    FlyingOnlyRestriction,
    CanBlockFlying,
    CanBlockAsThoughNoShadow,
    CanBlockOnlyFlying,
    CanBlockAdditionalCreatureEachCombat,
    MaxCreaturesCanAttackEachCombat,
    MaxCreaturesCanAttackYouEachCombat,
    MaxCreaturesCanBlockEachCombat,
    CantBeBlockedByPowerOrLess,
    CantBeBlockedByPowerOrGreater,
    CantBeBlockedByLowerPowerThanSource,
    CantBeBlockedByMoreThan,
    CantBeBlockedExceptByNOrMore,
    CanAttackAsThoughNoDefender,
    CanAttackAsThoughHaste,
    /// "You may activate abilities of <objects> as though those creatures had haste."
    ActivateAbilitiesAsThoughHaste,
    MustAttack,
    GoadedBySourceController,
    MustAttackAttachedController,
    AllCreaturesAttackAttachedControllerEachCombatIfAble,
    AttachedGoadedBySourceController,
    GoadMatching,
    AttachedControllerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn,
    AnyPlayerMayPayManaToIgnoreSourceEffectUntilEndOfTurn,
    ExertAttack,
    EnlistAttack,
    MustBlock,
    CantAttack,
    CantAttackItsOwner,
    /// "This creature can't attack a player it has already attacked this
    /// turn." (Port Razer)
    CantAttackPlayerAlreadyAttackedThisTurn,
    /// "can't be destroyed by lethal damage unless lethal damage dealt by a
    /// single source is marked on it" (Ogre Enforcer; CR 704.5g).
    LethalDamageMustBeFromSingleSource,
    CantAttackUnlessControllerCastCreatureSpellThisTurn,
    CantAttackUnlessControllerCastNonCreatureSpellThisTurn,
    CantAttackUnlessCondition,
    CantAttackYouUnlessControllerPaysPerAttacker,
    CantAttackYouOrPlaneswalkersUnlessControllerPaysPerAttacker,
    CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl,
    BlockCost,
    AttackCost,
    CantBlock,
    MayAssignDamageAsUnblocked,
    YouAssignCombatDamageOfCreaturesAttackingYou,
    ThisCreatureAssignsCombatDamageUsingToughness,
    CreaturesAssignCombatDamageUsingToughness,
    CreaturesYouControlAssignCombatDamageUsingToughness,
    LethalDamageToCreaturesYouControlUsesPower,
    Anthem,
    GrantAbility,
    RemoveAbilityForFilter,
    RemoveAllAbilitiesForFilter,
    RemoveAllAbilitiesExceptManaForFilter,
    SetBasePowerToughnessForFilter,
    SourceCharacteristicsOfLastExiledCreatureCard,
    EquipmentGrant,
    CharacteristicDefiningPT,
    AddCardTypes,
    RemoveCardTypes,
    SetCardTypes,
    AddSubtypes,
    AddAllSubtypesOfFamily,
    SetLandSubtypes,
    SetCreatureSubtypes,
    AddColors,
    CopyActivatedAbilities,
    CopyStaticAbilityVariants,
    CopyTriggeredAbilities,
    SoulbondSharedBonus,
    AttachedAbilityGrant,
    AttachedChosenLandwalkGrant,
    ControlAttachedPermanent,
    GrantObjectAbilityForFilter,
    SetColors,
    SetName,
    CountAsCardNamedForSpellEffect,
    MakeColorless,
    AddSupertypes,
    RemoveSupertypes,
    CostReduction,
    ActivatedAbilityCostReduction,
    ActivatedAbilityCostIncrease,
    ThisSpellCostReduction,
    ThisSpellCostReductionManaCost,
    CostIncrease,
    CostIncreaseLife,
    CostReductionManaCost,
    CostIncreaseManaCost,
    CostIncreasePerAdditionalTarget,
    CostIncreaseManaCostPerAdditionalTarget,
    CommanderTaxLifeSubstitution,
    Affinity,
    AffinityForArtifacts,
    Delve,
    Dredge,
    Convoke,
    Improvise,
    PlayersCantGainLife,
    PlayersCantSearch,
    DamageCantBePrevented,
    YouCantLoseGame,
    OpponentsCantWinGame,
    YourLifeTotalCantChange,
    PermanentsCantBeSacrificed,
    OpponentsCantCastSpells,
    OpponentsCantDrawExtraCards,
    CantHaveCountersPlaced,
    CounterLimit,
    CountersRemainAcrossZoneChanges,
    CantBeCountered,
    PlayersCantCycle,
    PlayersSkipUpkeep,
    PlayerSkipsDrawStep,
    PlayersSkipUntapStep,
    PlayersSkipExtraTurns,
    DamageNotRemovedDuringCleanup,
    BlackManaMayBePaidWithLife,
    DieRollResultAdjustment,
    MinimumSpellTotalMana,
    CantPayLifeOrSacrificeNonlandForCastOrActivate,
    ChooseColorAsEnters,
    ChooseColorAsBecomesAttached,
    ChoosePlayerAsEnters,
    NoteLifeTotalAsEnters,
    DiscardHandAsEnters,
    RevealFromHandAsEnters,
    ChooseCardNameAsEnters,
    ChooseBasicLandTypeAsEnters,
    ChooseLandTypeAsEnters,
    ChooseNamedOptionAsEnters,
    ChoosePowerToughnessAsEntersOrTurnsFaceUp,
    BoastTwiceEachTurn,
    /// "You may activate the loyalty abilities of planeswalkers you control
    /// twice each turn rather than only once." (Oath of Teferi)
    LoyaltyAbilitiesTwiceEachTurn,
    FirstEquipCostAlternative,
    EquipAbilitiesAnyTime,
    LoyaltyAbilitiesAnyTime,
    ExhaustAbilitiesAsThoughUnactivatedThisTurn,
    VoteAdditionalTimeWhileVoting,
    VoteAdditionalVoteWhileVoting,
    Enchant,
    EnchantedLandIsChosenType,
    SourceLandIsChosenType,
    AddChosenCreatureType,
    AddChosenBasicLandType,
    AddChosenColor,
    SetChosenColor,
    RedirectDamageToSource,
    RedirectDamageToSourceController,
    PreventAllDamageDealtToAndByThisPermanent,
    PreventAllDamageDealtByThisPermanent,
    PreventAllCombatDamageDealtByThisPermanent,
    PreventAllDamageDealtToCreatures,
    PreventAllDamageToSelf,
    PreventAllCombatDamageToSelf,
    PreventAllCombatDamageToPermanentsMatching,
    PreventAllCombatDamageToAndByPermanentsMatching,
    PreventAllNoncombatDamageToPermanentsMatching,
    PreventAllDamageToPermanentsMatching,
    PreventAllDamageToSelfFromSourcesMatching,
    PreventAllDamageToSelfByCreatures,
    PreventDamageToYouFromSourceFilter,
    PreventDamageToSelfRemoveCounter,
    PreventDamageToSelfPutCountersInstead,
    PreventConstrainedDamageToSelfPutCountersInstead,
    DamagePreventionWithFollowUp,
    ReplaceDamageWithCountersInstead,
    PreventDamageToOtherCreatureYouControlPutCountersInstead,
    PreventAllNoncombatDamageToOtherCreaturesYouControl,
    DoesntUntap,
    UntapDuringEachOtherPlayersUntapStep,
    UntapStepLimit,
    SearchLimitedToTopCards,
    PreventAllDamageToYou,
    PlayerProtectionFrom,
    OpponentsMustTargetFlagbearers,
    MayChooseNotToUntapDuringUntapStep,
    ChooseCreatureTypeAsEnters,
    EntersTapped,
    EntersPrepared,
    EntersTappedUnlessControlTwoOrMoreOtherLands,
    EntersTappedUnlessControlTwoOrFewerOtherLands,
    EntersTappedUnlessControlTwoOrMoreBasicLands,
    EntersTappedUnlessAPlayerHas13OrLessLife,
    EntersTappedUnlessTwoOrMoreOpponents,
    EntersTappedUnlessCondition,
    EnterWithCounters,
    EnterWithCountersIfCondition,
    ShuffleIntoLibraryFromGraveyard,
    AllPermanentsEnterTapped,
    EnterTappedForFilter,
    EnterUntappedForFilter,
    EnterAsCopyAsEnters,
    AsEntersEffectProgram,
    EnterWithCountersForFilter,
    EnterWithCharacteristicsForFilter,
    CanBeCommander,
    LevelAbilities,
    NoMaximumHandSize,
    SetMaximumHandSize,
    ReduceMaximumHandSize,
    IncreaseMaximumHandSize,
    MaximumHandSizeSevenMinusYourGraveyardCardTypes,
    RevealFirstCardYouDrawEachTurn,
    LookAtTopCardOfLibrary,
    LookAtFaceDownCreaturesYouDontControl,
    AllPlayersLookAtTopCardsOfLibraries,
    AllPlayersLookAtYourTopLibraryCard,
    OpponentsPlayWithHandsRevealed,
    ControlOpponentsWhileSearchingLibraries,
    OpponentSearchExileFoundCards,
    CastThisCardFromLibraryWhileSearching,
    EffectDiscardToLibraryReplacement,
    OpponentEffectDiscardThisToBattlefieldReplacement,
    /// "If a spell or ability an opponent controls causes you to discard a
    /// card, you may reveal that card and put it on top of your library
    /// instead of putting it anywhere else." (Nephalia Academy)
    OpponentEffectDiscardToLibraryReplacement,
    DrawReplacementExileTopFaceDown,
    DrawReplacementExileTopAndPlay,
    DrawReplacementRevealTopMatchingToHandRestBottom,
    DrawReplacementDouble,
    DrawReplacementSkipEmptyLibrary,
    ConditionalDrawReplacement,
    DrawExtraCardsReplacement,
    LoseGameReplacement,
    KeywordActionReplacement,
    ExileToCounteredExileInsteadOfGraveyard,
    ExileToExileInsteadOfGraveyard,
    ExileWouldDieInstead,
    RedirectZoneChange,
    ModifyDamageAmountReplacement,
    PreventHalfDamageReplacement,
    DoubleCountersReplacement,
    AddCountersPlacementReplacement,
    PlayerCounterPerTurnLimitReplacement,
    DoubleTokenCreationReplacement,
    MultiplyTokenCreationReplacement,
    RedirectDrawReplacement,
    DrawReplacementWithEffects,
    CreateOneOfEachTokenReplacement,
    AddTokenCreationReplacement,
    CreaturesEnteringDontCauseAbilitiesToTrigger,
    DuplicateMatchingTriggeredAbilities,
    DungeonRoomTriggerDuplication,
    /// "This effect doesn't remove this Aura." on an Aura that grants
    /// protection (CR 702.16n): this Aura isn't put into its owner's
    /// graveyard for being attached to a permanent with that protection.
    ProtectionDoesntRemoveThisAura,
    /// "This effect doesn't remove Auras." (CR 702.16n): no Aura is removed
    /// from the enchanted permanent by this Aura's protection grant.
    ProtectionDoesntRemoveAuras,
    SuppressMatchingTriggeredAbilities,
    DoubleDamageFromSourcesYouControlOfChosenType,
    StartingLifeBonus,
    BuybackCostReduction,
    LegendRuleDoesntApply,
    LegendRuleDoesntApplyToController,
    LegendRuleDoesntApplyToControllerTokens,
    ManaSpendPermission,
    SpendManaAsAnyColor,
    SpendManaAsAnyColorActivationCosts,
    RuleRestriction,
    TargetingAsThoughNoAbility,
    DiscardOrRedirectReplacement,
    SacrificeOrRedirectReplacement,
    PayLifeOrEnterTappedReplacement,
    RevealCardOrEnterTappedReplacement,
    RedirectWouldEnterReplacement,
    ManaProductionReplacement,
    ManaProductionMultiplierReplacement,
    DoubleLifeChangeReplacement,
    PregameAction,
    DeckConstructionRuleText,
    DungeonEntryRestriction,
    DraftRuleText,
    HiddenAgenda,
    DoubleAgenda,
    KeywordText,
    KeywordMarker,
    /// "If this enchantment leaves the battlefield, this effect continues
    /// until end of turn" (Titania's Song): when the permanent leaves the
    /// battlefield, the continuous effects of its other static abilities keep
    /// applying until end of turn.
    StaticEffectsContinueUntilEndOfTurnAfterLeaving,
    SourceLineKeywordGroup,
    SourceLineStaticGroup,
    KeywordFallbackText,
    RuleFallbackText,
    UnsupportedParserLine,
    Grants,
    NativeAlternativeCastFromZone,
    EntersUnderChosenControl,
    /// Toxic N (CR 702.164). The amount is carried in the ability's label.
    Toxic,
    /// Trample over planeswalkers (CR 702.19c). A variant of trample, not an
    /// instance of it: "has trample" checks don't match it.
    TrampleOverPlaneswalkers,
    /// A blocking-only permission that preserves the attacker's landwalk abilities.
    BlockingAsThoughNoLandwalk,
    /// Unbounded blocker capacity; appended to preserve serialized variant ordinals.
    CanBlockAnyNumber,
    /// Additional capacity scaled by a live typed permanent filter.
    CanBlockAdditionalForEach,
    /// Filtered, amount-based damage prevention. Appended for wire compatibility.
    PreventMatchingDamage,
    /// A controller-scoped exception to CR 704.5i.
    PlaneswalkersYouControlDontDieAtZeroLoyalty,
    /// Add to one life-gain event; appended for serialized ID compatibility.
    AddLifeGainReplacement,
    TokenCreationTemplates,
    ControllerPlaysWithHandRevealed,
    PlayersPlayWithHandsRevealed,
    /// A spell-only copy prohibition; appended for wire compatibility.
    CantBeCopied,
    RedirectMatchingDamage,
    SpellManaSpendingRestriction,
    /// Typed mana output rewrite; appended to preserve existing ordinals.
    ManaProductionRewrite,
    /// Extra numerical dice with ignored low rolls, appended for wire stability.
    ExtraDieIgnoreLowest,
    /// Conversion of mana that would be lost, preserving the existing units.
    ConvertUnspentMana,
    /// Appended to preserve existing published enum discriminants.
    ForetellSpecialActionModifier,
    ExtraCoinIgnoreOne,
    FirstCoinBatchHeadsWin,
    /// Standalone paired private inspection; appended for wire compatibility.
    LookAtSourceExiledCards,
    /// Generic "<event> instead" replacement; appended for wire compatibility.
    EventReplacementWithEffects,
    /// "You may pay <mana> rather than pay the echo cost for permanents you
    /// control"; appended for wire compatibility.
    EchoCostAlternative,
    /// A keyword granted to matching spells as they are cast (CR 601.2b);
    /// appended for wire compatibility.
    GrantSpellKeyword,
    /// Filtered permission to cast from a zone with a keyword's alternative
    /// cost; appended for wire compatibility.
    AlternativeCastFromZoneForFilter,
    /// "No more than N creatures can attack this planeswalker each combat";
    /// appended for wire compatibility.
    MaxCreaturesCanAttackSourceEachCombat,
    /// "can block as though it were untapped"; appended for wire compatibility.
    CanBlockAsThoughUntapped,
    /// Blocker-side "can block creatures with landwalk abilities as though
    /// they didn't have those abilities"; appended for wire compatibility.
    CanBlockAsThoughNoLandwalk,
    /// "If <creature> attacks, <creatures> attack if able"; appended for wire
    /// compatibility.
    ConditionalAttackRequirement,
    /// Generic amount-modifying replacement; appended for wire compatibility.
    EventAmountReplacement,
    /// "If an opponent would face a villainous choice, they face that choice
    /// an additional time." (The Valeyard); appended for wire compatibility.
    OpponentsFaceVillainousChoiceAdditionalTime,
    /// "A player losing unspent mana causes that player to lose that much
    /// life." (Yurlok of Scorch Thrash); appended for wire compatibility.
    LoseLifeForUnspentMana,
    /// "Each player chooses artifact, creature, or land during their untap
    /// step. That player can untap only permanents of the chosen type this
    /// step." (Storage Matrix); appended for wire compatibility.
    UntapStepChosenCardTypeOnly,
    /// "You may activate the loyalty abilities of Urza twice each turn rather
    /// than only once." (Urza, Planeswalker; CR 606.3); appended for wire
    /// compatibility.
    SourceLoyaltyAbilitiesTwiceEachTurn,
    /// "Each power-up ability of permanents you control can be activated an
    /// additional time." (Wonder Man, Hollywood Hero); appended for wire
    /// compatibility.
    PowerUpAbilitiesAdditionalTime,
    /// "Each opponent must attack you or a planeswalker you control with at
    /// least one creature each combat if able." (Trove of Temptation;
    /// CR 508.1d); appended for wire compatibility.
    OpponentsAttackYouWithAtLeastOneCreature,
    /// "Can't be blocked unless all creatures defending player controls block
    /// it." (Tromokratis); appended for wire compatibility.
    CantBeBlockedUnlessAllDefendingCreaturesBlock,
    /// "Enchanted creature is a copy of the chosen creature." (Metamorphic
    /// Alteration); appended for wire compatibility.
    AttachedIsCopyOfChosenObject,
    /// "If you would begin your turn while this artifact is tapped, you may
    /// skip that turn instead. If you do, untap this artifact." (Time Vault;
    /// CR 614.10): offered to the controller before their turn begins;
    /// appended for wire compatibility.
    MaySkipTurnWhileSourceTappedThenUntap,
    /// "Each opponent must attack with at least one creature each combat if
    /// able." (Seeker of Slaanesh; CR 508.1d): a requirement on the whole
    /// attack declaration, obeyed by any attack; appended for wire
    /// compatibility.
    OpponentsAttackWithAtLeastOneCreature,
    /// "assign their combat damage this turn as though they weren't blocked"
    /// (Outmaneuver): a mandatory unblocked assignment (CR 510.1c); appended
    /// for wire compatibility.
    AssignsDamageAsUnblocked,
    /// "Equipped creature has all abilities of ability stickers on this
    /// Equipment." (Pin Collection); appended for wire compatibility.
    AttachedHasSourceAbilityStickerAbilities,
    /// Adds to devotion for each queried color or combination, once per query.
    DevotionIncrease,
    /// "As this Equipment becomes attached to a creature, choose a creature
    /// card name and a creature type." (Psychic Paper); appended for wire
    /// compatibility.
    ChooseCreatureCardNameAndTypeAsBecomesAttached,
    /// "its name and creature type are the last chosen name and creature
    /// type" (Psychic Paper): the attached object's name and creature types
    /// become the source's last choices (layers 3 and 4); appended for wire
    /// compatibility.
    AttachedHasSourceChosenNameAndCreatureType,
    /// "If damage would be dealt to this creature, instead that damage is
    /// dealt, but all other damage already dealt to it is healed." (Wolverine,
    /// Fierce Fighter); appended for wire compatibility.
    HealDamageBeforeDamageToSelf,
    /// "Equipped creature ... has all names of nonlegendary creature cards in
    /// addition to its name." (Spy Kit): every name check of the attached
    /// object also matches each nonlegendary creature card name; appended for
    /// wire compatibility.
    AttachedHasAllNonlegendaryCreatureCardNames,
    /// "The monarch controls enchanted creature." (Fealty to the Realm;
    /// CR 613.1b, 724): the current monarch controls the permanent this
    /// source is attached to; appended for wire compatibility.
    MonarchControlsAttachedPermanent,
    /// "All instances of color words in the text of spells and permanents are
    /// changed to the chosen color word." (Swirl the Mists; CR 612.2): layer-3
    /// substitution of every other color word by the source's chosen color;
    /// appended for wire compatibility.
    AllColorWordsBecomeChosenColor,
    /// "If this creature is unblocked, you may have it assign its combat
    /// damage to a creature defending player controls." (Cunning Giant): an
    /// unblocked attacker may assign all its combat damage to one creature
    /// the defending player controls instead; appended for wire
    /// compatibility.
    MayAssignUnblockedDamageToDefendingCreature,
    /// "This creature has all abilities of ability stickers on other
    /// permanents you own and cards in your graveyard." (Clandestine
    /// Chameleon): the source gains the abilities of the ability stickers on
    /// every object matching a filter; appended for wire compatibility.
    SourceHasAbilityStickerAbilitiesOf,
    /// "As long as the top card of your graveyard is a creature card, this
    /// creature has the full text of that card and has the text ..."
    /// (Volrath's Shapeshifter): a self-sustaining layer-1 full-text
    /// overwrite; appended for wire compatibility.
    SourceHasFullTextOfTopGraveyardCard,
    /// "If you would begin your draw step, you may skip that step instead.
    /// If you do, <effects>." (Fasting; CR 614.10): offered to the
    /// controller as their draw step would begin; appended for wire
    /// compatibility.
    MaySkipDrawStepWithFollowup,
    /// "That player may sacrifice a permanent of their choice for that
    /// player to ignore this effect until end of turn." (Damping Engine): a
    /// special action for the player this source's restriction names;
    /// appended for wire compatibility.
    PlayerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn,
    /// "You may discard this card any time you could cast an instant."
    /// (Circling Vultures): a special action from its owner's hand that does
    /// not use the stack (CR 116.2); appended for wire compatibility.
    MayDiscardSourceAnyTimeYouCouldCastInstant,
    /// "All creatures can attack your opponents and planeswalkers your
    /// opponents control as though those creatures had haste." (Frenzied
    /// Saddlebrute): a battlefield-wide attack permission scoped to defenders
    /// who are opponents of this source's controller; appended for wire
    /// compatibility.
    CreaturesCanAttackOpponentsAsThoughHaste,
    /// "This creature attacks a player you noted for cards named <name> each
    /// combat if able." (Cogwork Tracker): an attack requirement (CR 508.1d)
    /// toward the players its controller noted while drafting cards of that
    /// name. The label is the card name; appended for wire compatibility.
    MustAttackDraftNotedPlayers,
    /// "You may plot nonland cards from the top of your library." (Fblthp,
    /// Lost on the Range): its controller may take the plot special action
    /// (CR 702.170a) for the top card of their library if that card has plot;
    /// appended for wire compatibility.
    MayPlotNonlandCardsFromTopOfLibrary,
    /// "As this Equipment enters, choose a creature you control it could be
    /// attached to. If you do, it enters attached to that creature."
    /// (Grifter's Blade): a non-Aura's own entry choice of its host;
    /// appended for wire compatibility.
    EntersAttachedToChosenObject,
    /// Extra "Sacrifice a <permanent>" costs per mana symbol of a color in
    /// spells' mana costs or activated abilities' costs (Drought); appended
    /// for wire compatibility.
    AdditionalSacrificePerManaSymbol,
    /// "If a spell or ability you control would counter a spell, instead
    /// exile that spell and you may play that card without paying its mana
    /// cost." (Guile): appended for wire compatibility.
    ControllerCounterExilePermission,
    /// "The attacking player chooses how each creature blocks each combat."
    /// (Invasion Plans): appended for wire compatibility.
    AttackingPlayerChoosesBlocks,
    /// "Its controller may exile three cards from their graveyard for that
    /// player to ignore this effect until end of turn." (Lost in Thought):
    /// the attached object's controller's special action with an authored
    /// cost; appended for wire compatibility.
    AttachedControllerMayPayCostToIgnoreSourceEffectUntilEndOfTurn,
    /// "Can't be blocked unless it's attacking its owner or a permanent its
    /// owner controls." (Become the Pilot): evasion that lapses only while
    /// the attack target is the attacker's owner or a permanent that owner
    /// controls (CR 509.1b); appended for wire compatibility.
    CantBeBlockedUnlessAttackingOwnerOrOwnersPermanent,
    /// "Creatures can't attack a player unless that player cast a spell or
    /// put a nontoken permanent onto the battlefield during their last turn."
    /// (Arboria): a global per-defending-player attack restriction;
    /// appended for wire compatibility.
    CreaturesCantAttackPlayersInactiveLastTurn,
    /// "You may assign this creature's combat damage divided as you choose
    /// among defending player and/or any number of creatures they control."
    /// (Butcher Orgg): an optional free division with no lethal-damage
    /// ordering (CR 510.1); appended for wire compatibility.
    MayDivideCombatDamageAmongDefendingPlayerAndCreatures,
    /// Mandatory public reveal of every card drawn by matching players.
    RevealCardsDrawn,
    ExtraPlanarDieIgnoreOne,
}

impl StaticAbilityId {
    fn exhaustive_classification_guard(id: StaticAbilityId) {
        use StaticAbilityId::*;
        match id {
            Flying
            | FirstStrike
            | DoubleStrike
            | Deathtouch
            | Defender
            | Flash
            | Haste
            | Hexproof
            | HexproofFrom
            | Indestructible
            | Intimidate
            | Lifelink
            | Menace
            | Banding
            | BandsWithOther
            | Protection
            | Reach
            | Shroud
            | Trample
            | Vigilance
            | Ward
            | Fear
            | Skulk
            | Prowess
            | Flanking
            | UmbraArmor
            | Landwalk
            | CantBeBlockedAsLongAsDefendingPlayerControlsCardType
            | CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
            | CantBeBlockedWhileDefendingPlayerControlsMostCreatures
            | CantBeBlockedUnlessAttackingOwnerOrOwnersPermanent
            | Bloodthirst
            | Tribute
            | Daybound
            | Nightbound
            | DayNightStartsDayAsEnters
            | Morph
            | Disguise
            | Megamorph
            | Shadow
            | Horsemanship
            | Phasing
            | Wither
            | Infect
            | Changeling
            | LivingMetal
            | Companion
            | Partner
            | PartnerWith
            | StartYourEngines
            | SpaceSculptor
            | DoctorsCompanion
            | Assist
            | Ascend
            | Storied
            | SplitSecond
            | Rebound
            | Cascade
            | CascadeLandDrop
            | Splice
            | Escalate
            | ReadAhead
            | Unleash
            | ConditionalSpellKeyword
            | ThisSpellCastRestriction
            | ThisSpellXMaximum
            | ThisSpellXMinimum
            | Unblockable
            | FlyingRestriction
            | FlyingOnlyRestriction
            | CanBlockFlying
            | CanBlockAsThoughNoShadow
            | CanBlockOnlyFlying
            | CanBlockAdditionalCreatureEachCombat
            | CanBlockAnyNumber
            | CanBlockAdditionalForEach
            | MaxCreaturesCanAttackEachCombat
            | MaxCreaturesCanAttackYouEachCombat
            | MaxCreaturesCanAttackSourceEachCombat
            | CanBlockAsThoughUntapped
            | CanBlockAsThoughNoLandwalk
            | MaxCreaturesCanBlockEachCombat
            | CantBeBlockedByPowerOrLess
            | CantBeBlockedByPowerOrGreater
            | CantBeBlockedByLowerPowerThanSource
            | CantBeBlockedByMoreThan
            | CantBeBlockedExceptByNOrMore
            | CanAttackAsThoughNoDefender
            | CanAttackAsThoughHaste
            | ActivateAbilitiesAsThoughHaste
            | MustAttack
            | GoadedBySourceController
            | MustAttackAttachedController
            | AllCreaturesAttackAttachedControllerEachCombatIfAble
            | AttachedGoadedBySourceController
            | GoadMatching
            | ConditionalAttackRequirement
            | AttachedControllerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn
            | AttachedControllerMayPayCostToIgnoreSourceEffectUntilEndOfTurn
            | AnyPlayerMayPayManaToIgnoreSourceEffectUntilEndOfTurn
            | PlayerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn
            | ExertAttack
            | EnlistAttack
            | MustBlock
            | CantAttack
            | CantAttackItsOwner
            | CantAttackPlayerAlreadyAttackedThisTurn
            | CreaturesCantAttackPlayersInactiveLastTurn
            | LethalDamageMustBeFromSingleSource
            | CantAttackUnlessControllerCastCreatureSpellThisTurn
            | CantAttackUnlessControllerCastNonCreatureSpellThisTurn
            | CantAttackUnlessCondition
            | CantAttackYouUnlessControllerPaysPerAttacker
            | CantAttackYouOrPlaneswalkersUnlessControllerPaysPerAttacker
            | CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
            | BlockCost
            | AttackCost
            | CantBlock
            | MayAssignDamageAsUnblocked
            | AssignsDamageAsUnblocked
            | MayAssignUnblockedDamageToDefendingCreature
            | MayDivideCombatDamageAmongDefendingPlayerAndCreatures
            | YouAssignCombatDamageOfCreaturesAttackingYou
            | ThisCreatureAssignsCombatDamageUsingToughness
            | CreaturesAssignCombatDamageUsingToughness
            | CreaturesYouControlAssignCombatDamageUsingToughness
            | LethalDamageToCreaturesYouControlUsesPower
            | Anthem
            | GrantAbility
            | RemoveAbilityForFilter
            | RemoveAllAbilitiesForFilter
            | RemoveAllAbilitiesExceptManaForFilter
            | SetBasePowerToughnessForFilter
            | SourceCharacteristicsOfLastExiledCreatureCard
            | EquipmentGrant
            | AttachedHasSourceAbilityStickerAbilities
            | SourceHasAbilityStickerAbilitiesOf
            | SourceHasFullTextOfTopGraveyardCard
            | AttachedHasSourceChosenNameAndCreatureType
            | AttachedHasAllNonlegendaryCreatureCardNames
            | MonarchControlsAttachedPermanent
            | AllColorWordsBecomeChosenColor
            | DevotionIncrease
            | CharacteristicDefiningPT
            | AddCardTypes
            | RemoveCardTypes
            | SetCardTypes
            | AddSubtypes
            | AddAllSubtypesOfFamily
            | SetLandSubtypes
            | SetCreatureSubtypes
            | AddColors
            | CopyActivatedAbilities
            | CopyStaticAbilityVariants
            | CopyTriggeredAbilities
            | SoulbondSharedBonus
            | AttachedAbilityGrant
            | AttachedChosenLandwalkGrant
            | ControlAttachedPermanent
            | GrantObjectAbilityForFilter
            | SetColors
            | SetName
            | CountAsCardNamedForSpellEffect
            | MakeColorless
            | AddSupertypes
            | RemoveSupertypes
            | CostReduction
            | ActivatedAbilityCostReduction
            | ActivatedAbilityCostIncrease
            | ThisSpellCostReduction
            | ThisSpellCostReductionManaCost
            | CostIncrease
            | CostIncreaseLife
            | CostReductionManaCost
            | CostIncreaseManaCost
            | CostIncreasePerAdditionalTarget
            | CostIncreaseManaCostPerAdditionalTarget
            | CommanderTaxLifeSubstitution
            | Affinity
            | AffinityForArtifacts
            | Delve
            | Dredge
            | Convoke
            | Improvise
            | PlayersCantGainLife
            | PlayersCantSearch
            | DamageCantBePrevented
            | YouCantLoseGame
            | OpponentsCantWinGame
            | YourLifeTotalCantChange
            | PermanentsCantBeSacrificed
            | OpponentsCantCastSpells
            | OpponentsCantDrawExtraCards
            | CantHaveCountersPlaced
            | CounterLimit
            | CountersRemainAcrossZoneChanges
            | CantBeCountered
            | CantBeCopied
            | PlayersCantCycle
            | PlayersSkipUpkeep
            | PlayerSkipsDrawStep
            | PlayersSkipUntapStep
            | PlayersSkipExtraTurns
            | DamageNotRemovedDuringCleanup
            | BlackManaMayBePaidWithLife
            | DieRollResultAdjustment
            | MinimumSpellTotalMana
            | CantPayLifeOrSacrificeNonlandForCastOrActivate
            | ChooseColorAsEnters
            | ChooseColorAsBecomesAttached
            | ChooseCreatureCardNameAndTypeAsBecomesAttached
            | ChoosePlayerAsEnters
            | NoteLifeTotalAsEnters
            | DiscardHandAsEnters
            | RevealFromHandAsEnters
            | ChooseCardNameAsEnters
            | ChooseBasicLandTypeAsEnters
            | ChooseLandTypeAsEnters
            | ChooseNamedOptionAsEnters
            | ChoosePowerToughnessAsEntersOrTurnsFaceUp
            | BoastTwiceEachTurn
            | LoyaltyAbilitiesTwiceEachTurn
            | SourceLoyaltyAbilitiesTwiceEachTurn
            | PowerUpAbilitiesAdditionalTime
            | OpponentsAttackYouWithAtLeastOneCreature
            | CantBeBlockedUnlessAllDefendingCreaturesBlock
            | MaySkipTurnWhileSourceTappedThenUntap
            | MaySkipDrawStepWithFollowup
            | MayDiscardSourceAnyTimeYouCouldCastInstant
            | MayPlotNonlandCardsFromTopOfLibrary
            | EntersAttachedToChosenObject
            | ControllerCounterExilePermission
            | AttackingPlayerChoosesBlocks
            | MustAttackDraftNotedPlayers
            | CreaturesCanAttackOpponentsAsThoughHaste
            | OpponentsAttackWithAtLeastOneCreature
            | FirstEquipCostAlternative
            | EquipAbilitiesAnyTime
            | LoyaltyAbilitiesAnyTime
            | PlaneswalkersYouControlDontDieAtZeroLoyalty
            | ExhaustAbilitiesAsThoughUnactivatedThisTurn
            | VoteAdditionalTimeWhileVoting
            | VoteAdditionalVoteWhileVoting
            | OpponentsFaceVillainousChoiceAdditionalTime
            | LoseLifeForUnspentMana
            | Enchant
            | EnchantedLandIsChosenType
            | AttachedIsCopyOfChosenObject
            | SourceLandIsChosenType
            | AddChosenCreatureType
            | AddChosenBasicLandType
            | AddChosenColor
            | SetChosenColor
            | RedirectDamageToSource
            | RedirectDamageToSourceController
            | PreventAllDamageDealtToAndByThisPermanent
            | PreventAllDamageDealtByThisPermanent
            | PreventAllCombatDamageDealtByThisPermanent
            | PreventAllDamageDealtToCreatures
            | PreventAllDamageToSelf
            | PreventAllCombatDamageToSelf
            | PreventAllCombatDamageToPermanentsMatching
            | PreventAllCombatDamageToAndByPermanentsMatching
            | PreventAllNoncombatDamageToPermanentsMatching
            | PreventAllDamageToPermanentsMatching
            | PreventAllDamageToSelfFromSourcesMatching
            | PreventAllDamageToSelfByCreatures
            | PreventDamageToYouFromSourceFilter
            | PreventDamageToSelfRemoveCounter
            | PreventDamageToSelfPutCountersInstead
            | HealDamageBeforeDamageToSelf
            | PreventConstrainedDamageToSelfPutCountersInstead
            | DamagePreventionWithFollowUp
            | ReplaceDamageWithCountersInstead
            | PreventDamageToOtherCreatureYouControlPutCountersInstead
            | PreventAllNoncombatDamageToOtherCreaturesYouControl
            | DoesntUntap
            | UntapDuringEachOtherPlayersUntapStep
            | UntapStepLimit
            | UntapStepChosenCardTypeOnly
            | SearchLimitedToTopCards
            | PreventAllDamageToYou
            | PlayerProtectionFrom
            | OpponentsMustTargetFlagbearers
            | MayChooseNotToUntapDuringUntapStep
            | ChooseCreatureTypeAsEnters
            | EntersTapped
            | EntersPrepared
            | EntersTappedUnlessControlTwoOrMoreOtherLands
            | EntersTappedUnlessControlTwoOrFewerOtherLands
            | EntersTappedUnlessControlTwoOrMoreBasicLands
            | EntersTappedUnlessAPlayerHas13OrLessLife
            | EntersTappedUnlessTwoOrMoreOpponents
            | EntersTappedUnlessCondition
            | EnterWithCounters
            | EnterWithCountersIfCondition
            | ShuffleIntoLibraryFromGraveyard
            | AllPermanentsEnterTapped
            | EnterTappedForFilter
            | EnterUntappedForFilter
            | EnterAsCopyAsEnters
            | AsEntersEffectProgram
            | EnterWithCountersForFilter
            | EnterWithCharacteristicsForFilter
            | CanBeCommander
            | LevelAbilities
            | NoMaximumHandSize
            | SetMaximumHandSize
            | ReduceMaximumHandSize
            | IncreaseMaximumHandSize
            | MaximumHandSizeSevenMinusYourGraveyardCardTypes
            | RevealFirstCardYouDrawEachTurn
            | RevealCardsDrawn
            | LookAtTopCardOfLibrary
            | LookAtFaceDownCreaturesYouDontControl
            | AllPlayersLookAtTopCardsOfLibraries
            | AllPlayersLookAtYourTopLibraryCard
            | OpponentsPlayWithHandsRevealed
            | ControllerPlaysWithHandRevealed
            | PlayersPlayWithHandsRevealed
            | ControlOpponentsWhileSearchingLibraries
            | OpponentSearchExileFoundCards
            | CastThisCardFromLibraryWhileSearching
            | EffectDiscardToLibraryReplacement
            | OpponentEffectDiscardThisToBattlefieldReplacement
            | OpponentEffectDiscardToLibraryReplacement
            | DrawReplacementExileTopFaceDown
            | DrawReplacementExileTopAndPlay
            | DrawReplacementRevealTopMatchingToHandRestBottom
            | DrawReplacementDouble
            | DrawReplacementSkipEmptyLibrary
            | ConditionalDrawReplacement
            | DrawExtraCardsReplacement
            | LoseGameReplacement
            | KeywordActionReplacement
            | ExileToCounteredExileInsteadOfGraveyard
            | ExileToExileInsteadOfGraveyard
            | ExileWouldDieInstead
            | RedirectZoneChange
            | ModifyDamageAmountReplacement
            | PreventHalfDamageReplacement
            | PreventMatchingDamage
            | SpellManaSpendingRestriction
            | ExtraCoinIgnoreOne
            | FirstCoinBatchHeadsWin
            | LookAtSourceExiledCards
            | ExtraDieIgnoreLowest
            | ExtraPlanarDieIgnoreOne
            | RedirectMatchingDamage
            | AddLifeGainReplacement
            | TokenCreationTemplates
            | DoubleCountersReplacement
            | AddCountersPlacementReplacement
            | PlayerCounterPerTurnLimitReplacement
            | DoubleTokenCreationReplacement
            | MultiplyTokenCreationReplacement
            | RedirectDrawReplacement
            | DrawReplacementWithEffects
            | EventReplacementWithEffects
            | EchoCostAlternative
            | EventAmountReplacement
            | CreateOneOfEachTokenReplacement
            | AddTokenCreationReplacement
            | CreaturesEnteringDontCauseAbilitiesToTrigger
            | DuplicateMatchingTriggeredAbilities
            | DungeonRoomTriggerDuplication
            | ProtectionDoesntRemoveThisAura
            | ProtectionDoesntRemoveAuras
            | SuppressMatchingTriggeredAbilities
            | DoubleDamageFromSourcesYouControlOfChosenType
            | StartingLifeBonus
            | BuybackCostReduction
            | LegendRuleDoesntApply
            | LegendRuleDoesntApplyToController
            | LegendRuleDoesntApplyToControllerTokens
            | ManaSpendPermission
            | SpendManaAsAnyColor
            | SpendManaAsAnyColorActivationCosts
            | RuleRestriction
            | TargetingAsThoughNoAbility
            | BlockingAsThoughNoLandwalk
            | DiscardOrRedirectReplacement
            | SacrificeOrRedirectReplacement
            | PayLifeOrEnterTappedReplacement
            | RevealCardOrEnterTappedReplacement
            | RedirectWouldEnterReplacement
            | ManaProductionReplacement
            | ConvertUnspentMana
            | ManaProductionRewrite
            | ManaProductionMultiplierReplacement
            | DoubleLifeChangeReplacement
            | PregameAction
            | DeckConstructionRuleText
            | DungeonEntryRestriction
            | DraftRuleText
            | HiddenAgenda
            | DoubleAgenda
            | KeywordText
            | KeywordMarker
            | StaticEffectsContinueUntilEndOfTurnAfterLeaving
            | SourceLineKeywordGroup
            | SourceLineStaticGroup
            | KeywordFallbackText
            | RuleFallbackText
            | UnsupportedParserLine
            | Grants
            | EntersUnderChosenControl
            | Toxic
            | TrampleOverPlaneswalkers
            | ForetellSpecialActionModifier
            | NativeAlternativeCastFromZone
            | GrantSpellKeyword
            | AlternativeCastFromZoneForFilter
            | AdditionalSacrificePerManaSymbol => {}
        }
    }

    pub fn is_keyword(&self) -> bool {
        Self::exhaustive_classification_guard(*self);
        use StaticAbilityId::*;
        matches!(
            self,
            Flying
                | FirstStrike
                | DoubleStrike
                | Deathtouch
                | Defender
                | Flash
                | Haste
                | Hexproof
                | HexproofFrom
                | Indestructible
                | Intimidate
                | Lifelink
                | Menace
                | Banding
                | BandsWithOther
                | Protection
                | Reach
                | Shroud
                | Trample
                | Vigilance
                | Ward
                | Fear
                | Skulk
                | Prowess
                | Flanking
                | UmbraArmor
                | Landwalk
                | Bloodthirst
                | Tribute
                | Morph
                | Disguise
                | Megamorph
                | Shadow
                | Horsemanship
                | Phasing
                | Wither
                | Infect
                | Changeling
                | LivingMetal
                | Companion
                | Partner
                | PartnerWith
                | StartYourEngines
                | SpaceSculptor
                | DoctorsCompanion
                | Assist
                | SplitSecond
                | Rebound
                | Cascade
                | Splice
                | Escalate
                | Dredge
                | EnlistAttack
                | ReadAhead
                | Unleash
                | Toxic
                | TrampleOverPlaneswalkers
                | KeywordText
                | KeywordMarker
                | KeywordFallbackText
        )
    }

    pub fn grants_evasion(&self) -> bool {
        Self::exhaustive_classification_guard(*self);
        use StaticAbilityId::*;
        matches!(
            self,
            Flying
                | Shadow
                | Horsemanship
                | Fear
                | Intimidate
                | Skulk
                | FlyingRestriction
                | FlyingOnlyRestriction
                | CantBeBlockedByPowerOrLess
                | CantBeBlockedByPowerOrGreater
                | CantBeBlockedByLowerPowerThanSource
                | CantBeBlockedByMoreThan
                | CantBeBlockedExceptByNOrMore
                | Landwalk
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardType
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
                | CantBeBlockedWhileDefendingPlayerControlsMostCreatures
                | CantBeBlockedUnlessAttackingOwnerOrOwnersPermanent
        )
    }

    pub fn affects_combat(&self) -> bool {
        Self::exhaustive_classification_guard(*self);
        use StaticAbilityId::*;
        matches!(
            self,
            Flying
                | FirstStrike
                | DoubleStrike
                | Deathtouch
                | Defender
                | Lifelink
                | Menace
                | Banding
                | BandsWithOther
                | Reach
                | Trample
                | Vigilance
                | Fear
                | Skulk
                | Flanking
                | Landwalk
                | Shadow
                | Horsemanship
                | Unblockable
                | FlyingRestriction
                | FlyingOnlyRestriction
                | CanBlockFlying
                | CanBlockAsThoughNoShadow
                | BlockingAsThoughNoLandwalk
                | CanBlockOnlyFlying
                | CanBlockAnyNumber
                | CanBlockAdditionalForEach
                | MaxCreaturesCanAttackEachCombat
                | MaxCreaturesCanBlockEachCombat
                | CantBeBlockedByPowerOrLess
                | CantBeBlockedByPowerOrGreater
                | CantBeBlockedByLowerPowerThanSource
                | CantBeBlockedByMoreThan
                | CantBeBlockedExceptByNOrMore
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardType
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
                | CantBeBlockedWhileDefendingPlayerControlsMostCreatures
                | CantBeBlockedUnlessAttackingOwnerOrOwnersPermanent
                | CanAttackAsThoughNoDefender
                | CanAttackAsThoughHaste
                | CreaturesCanAttackOpponentsAsThoughHaste
                | ActivateAbilitiesAsThoughHaste
                | MustAttack
                | ConditionalAttackRequirement
                | MustBlock
                | CantAttack
                | CantAttackItsOwner
                | CantAttackPlayerAlreadyAttackedThisTurn
                | CreaturesCantAttackPlayersInactiveLastTurn
                | CantAttackUnlessControllerCastCreatureSpellThisTurn
                | CantAttackUnlessControllerCastNonCreatureSpellThisTurn
                | CantAttackUnlessCondition
                | CantAttackYouUnlessControllerPaysPerAttacker
                | CantAttackYouOrPlaneswalkersUnlessControllerPaysPerAttacker
                | CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
                | BlockCost
                | AttackCost
                | CantBlock
                | MayAssignDamageAsUnblocked
                | AssignsDamageAsUnblocked
                | MayAssignUnblockedDamageToDefendingCreature
                | MayDivideCombatDamageAmongDefendingPlayerAndCreatures
                | YouAssignCombatDamageOfCreaturesAttackingYou
                | ThisCreatureAssignsCombatDamageUsingToughness
                | CreaturesAssignCombatDamageUsingToughness
                | CreaturesYouControlAssignCombatDamageUsingToughness
                | LethalDamageToCreaturesYouControlUsesPower
                | Toxic
                | TrampleOverPlaneswalkers
        )
    }

    pub fn generates_continuous_effects(&self) -> bool {
        Self::exhaustive_classification_guard(*self);
        use StaticAbilityId::*;
        matches!(
            self,
            Anthem
                | GrantAbility
                | AttachedAbilityGrant
                | AttachedChosenLandwalkGrant
                | RemoveAllAbilitiesForFilter
                | RemoveAllAbilitiesExceptManaForFilter
                | SetBasePowerToughnessForFilter
                | EquipmentGrant
                | AttachedHasSourceAbilityStickerAbilities
                | SourceHasAbilityStickerAbilitiesOf
                | AttachedHasSourceChosenNameAndCreatureType
                | GrantObjectAbilityForFilter
                | ControlAttachedPermanent
                | MonarchControlsAttachedPermanent
                | AllColorWordsBecomeChosenColor
                | CharacteristicDefiningPT
                | LivingMetal
                | AddCardTypes
                | RemoveCardTypes
                | SetCardTypes
                | AddSubtypes
                | SetLandSubtypes
                | AddColors
                | AddChosenColor
                | SetColors
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_identification_is_stable() {
        assert!(StaticAbilityId::Flying.is_keyword());
        assert!(StaticAbilityId::Trample.is_keyword());
        assert!(StaticAbilityId::Protection.is_keyword());
        assert!(!StaticAbilityId::Anthem.is_keyword());
        assert!(!StaticAbilityId::SetLandSubtypes.is_keyword());
    }

    #[test]
    fn evasion_identification_is_stable() {
        assert!(StaticAbilityId::Flying.grants_evasion());
        assert!(StaticAbilityId::Shadow.grants_evasion());
        assert!(!StaticAbilityId::Trample.grants_evasion());
        assert!(!StaticAbilityId::Lifelink.grants_evasion());
    }

    #[test]
    fn continuous_effect_identification_is_stable() {
        assert!(StaticAbilityId::Anthem.generates_continuous_effects());
        assert!(StaticAbilityId::SetLandSubtypes.generates_continuous_effects());
        assert!(!StaticAbilityId::Flying.generates_continuous_effects());
        assert!(!StaticAbilityId::Hexproof.generates_continuous_effects());
    }
}

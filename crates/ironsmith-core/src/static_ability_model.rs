use std::any::Any;

use crate::{
    Ability, AbilityKind, ActivatedAbility, AlternativeCastingMethod, AnthemValue, CardType, Color,
    ColorSet, Condition, CostComponent, CounterType, DamagedBySource, DerivedAlternativeCast,
    GrantSpec, Grantable, KeywordActionKind, ManaCost, ManaSpendPermission, ObjectFilter,
    PlayerFilter, ProtectionFrom, Restriction, StaticAbilityId, Subtype, SubtypeFamily, Supertype,
    TotalCost, TriggeredAbility, Value, Zone,
};

type AbilityModel<T, E, C, Cond> = Ability<StaticAbility<T, E, C, Cond>, T, E, C>;
type LevelAbilityModel<T, E, C, Cond> = crate::LevelAbility<StaticAbility<T, E, C, Cond>>;
type GrantSpecModel<T, E, C, Cond> = GrantSpec<StaticAbility<T, E, C, Cond>, E, C, Cond>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalSpellKeywordKind {
    Flash,
    Cascade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraveyardCountMetric {
    CardTypes,
    ManaValues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalSpellKeywordSpec {
    pub keyword: ConditionalSpellKeywordKind,
    pub metric: GraveyardCountMetric,
    pub threshold: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdditionalTokenKind {
    Treasure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PregameActionKind {
    BeginOnBattlefield(PregameBeginOnBattlefieldSpec),
    MulliganExileHandDrawSameCount,
    ChooseColor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PregameBeginOnBattlefieldSpec {
    pub require_not_starting_player: bool,
    pub counters: Vec<(CounterType, u32)>,
    pub exile_cards_from_hand: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCastRestrictionKind {
    pub label: String,
}

impl ThisSpellCastRestrictionKind {
    fn named(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }

    pub fn during_declare_attackers_step() -> Self {
        Self::named("during declare attackers step")
    }

    pub fn during_declare_attackers_step_if_you_were_attacked_this_step() -> Self {
        Self::named("during declare attackers step if you were attacked")
    }

    pub fn during_combat() -> Self {
        Self::named("during combat")
    }

    pub fn during_combat_before_blockers_are_declared() -> Self {
        Self::named("during combat before blockers")
    }

    pub fn during_combat_after_blockers_are_declared() -> Self {
        Self::named("during combat after blockers")
    }

    pub fn during_combat_on_your_turn_before_blockers_are_declared() -> Self {
        Self::named("during combat on your turn before blockers")
    }

    pub fn during_combat_on_opponents_turn() -> Self {
        Self::named("during combat on opponents turn")
    }

    pub fn before_attackers_are_declared() -> Self {
        Self::named("before attackers are declared")
    }

    pub fn before_combat_damage_step() -> Self {
        Self::named("before combat damage step")
    }

    pub fn during_opponents_upkeep() -> Self {
        Self::named("during opponents upkeep")
    }

    pub fn during_opponents_turn_after_upkeep() -> Self {
        Self::named("during opponents turn after upkeep")
    }

    pub fn during_your_end_step() -> Self {
        Self::named("during your end step")
    }

    pub fn if_you_cast_another_spell_this_turn() -> Self {
        Self::named("if you cast another spell this turn")
    }

    pub fn if_you_cast_another_green_spell_this_turn() -> Self {
        Self::named("if you cast another green spell this turn")
    }

    pub fn if_opponent_cast_creature_spell_this_turn() -> Self {
        Self::named("if opponent cast creature spell this turn")
    }

    pub fn if_creature_is_attacking_you() -> Self {
        Self::named("if creature is attacking you")
    }

    pub fn after_combat() -> Self {
        Self::named("after combat")
    }

    pub fn if_no_permanents_named_on_battlefield(name: impl AsRef<str>) -> Self {
        Self::named(format!("if no permanents named {}", name.as_ref()))
    }

    pub fn if_you_control_snow_land() -> Self {
        Self::named("if you control snow land")
    }

    pub fn if_you_control_fewer_creatures_than_each_opponent() -> Self {
        Self::named("if you control fewer creatures than each opponent")
    }

    pub fn if_you_control_subtype_or_more(subtype: Subtype, count: u32) -> Self {
        Self::named(format!("if you control {count}+ {subtype}"))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticAbility<T, E, C, Cond> {
    pub id: Option<StaticAbilityId>,
    pub label: String,
    pub payload: StaticAbilityPayload<T, E, C, Cond>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerToughnessChoiceOption<T, E, C, Cond> {
    pub power: i32,
    pub toughness: i32,
    pub abilities: Vec<StaticAbility<T, E, C, Cond>>,
}

impl<T, E, C, Cond> PowerToughnessChoiceOption<T, E, C, Cond> {
    pub fn new(power: i32, toughness: i32) -> Self {
        Self {
            power,
            toughness,
            abilities: Vec::new(),
        }
    }

    pub fn with_abilities(
        power: i32,
        toughness: i32,
        abilities: Vec<StaticAbility<T, E, C, Cond>>,
    ) -> Self {
        Self {
            power,
            toughness,
            abilities,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum StaticAbilityPayload<T, E, C, Cond> {
    #[default]
    None,
    Anthem(Anthem),
    AttachedAbilityGrant(Box<AttachedAbilityGrant<T, E, C, Cond>>),
    AttachedChosenLandwalkGrant(AttachedChosenLandwalkGrant),
    Conditional {
        ability: Box<StaticAbility<T, E, C, Cond>>,
        condition: Condition,
    },
    GrantAbility(Box<GrantAbility<T, E, C, Cond>>),
    GrantObjectAbilityForFilter(Box<GrantObjectAbilityForFilter<T, E, C, Cond>>),
    CopyActivatedAbilities(CopyActivatedAbilities),
    CopyTriggeredAbilities(CopyTriggeredAbilities),
    CostReduction(CostReduction),
    CostReductionManaCost(CostReductionManaCost),
    CostIncrease(CostIncrease),
    CostIncreaseManaCost(CostIncreaseManaCost),
    ThisSpellCostReduction(ThisSpellCostReduction<Cond>),
    ThisSpellCostReductionManaCost(ThisSpellCostReductionManaCost<Cond>),
    ThisSpellCastRestriction {
        kind: ThisSpellCastRestrictionKind,
        display: String,
    },
    ThisSpellXMaximum {
        maximum: Value,
        display: String,
    },
    LevelAbility(Box<LevelAbilityModel<T, E, C, Cond>>),
    HexproofFrom(ObjectFilter),
    Protection(ProtectionFrom),
    PreventAllCombatDamageToPermanentsMatching(ObjectFilter),
    RuleRestriction {
        restriction: Restriction,
        display: String,
    },
    PregameAction {
        kind: PregameActionKind,
        text: String,
    },
    Ward(TotalCost<C>),
    Morph(TotalCost<C>),
    Disguise(TotalCost<C>),
    Megamorph(TotalCost<C>),
    CanBlockAdditionalCreatureEachCombat(usize),
    CanBlockAsThoughReachForSubtype(Subtype),
    CantBeBlockedByMoreThan(usize),
    CantBeBlockedExceptByNOrMore(usize),
    CantBeBlockedByPowerOrLess(i32),
    CantBeBlockedByPowerOrGreater(i32),
    CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes(Vec<CardType>),
    CantAttackUnlessCondition {
        condition: CantAttackUnlessConditionSpec,
        display: String,
    },
    MayChooseNotToUntapDuringUntapStep(String),
    UntapDuringEachOtherPlayersUntapStep {
        filter: ObjectFilter,
        display: String,
    },
    FirstEquipCostAlternative(String),
    ControlAttachedPermanent(String),
    SetColors {
        filter: ObjectFilter,
        colors: ColorSet,
    },
    AddColors {
        filter: ObjectFilter,
        colors: ColorSet,
    },
    SetName {
        filter: ObjectFilter,
        name: String,
    },
    AddSupertypes {
        filter: ObjectFilter,
        supertypes: Vec<Supertype>,
    },
    RemoveSupertypes {
        filter: ObjectFilter,
        supertypes: Vec<Supertype>,
    },
    MaxCreaturesCanAttackEachCombat(usize),
    MaxCreaturesCanAttackYouEachCombat(usize),
    MaxCreaturesCanBlockEachCombat(usize),
    ChooseBasicLandTypeAsEnters(String),
    ChooseLandTypeAsEnters(String),
    EnchantedLandIsChosenType(String),
    AddChosenCreatureType {
        filter: ObjectFilter,
        display: String,
    },
    AddChosenColor {
        filter: ObjectFilter,
        display: String,
    },
    SetChosenColor {
        filter: ObjectFilter,
        display: String,
    },
    SetMaximumHandSize {
        player: PlayerFilter,
        amount: u32,
    },
    ReduceMaximumHandSize {
        player: PlayerFilter,
        by: u32,
    },
    MaximumHandSizeSevenMinusYourGraveyardCardTypes {
        player: PlayerFilter,
        min_card_types: u32,
    },
    DuplicateMatchingTriggeredAbilities {
        source_filter: Option<ObjectFilter>,
        event_matcher: Option<T>,
        count: u32,
        display: String,
    },
    SuppressMatchingTriggeredAbilities {
        source_filter: Option<ObjectFilter>,
        event_matcher: Option<T>,
        display: String,
    },
    ExertAttack {
        only_if_not_exerted_this_turn: bool,
        linked_trigger: Option<TriggeredAbility<T, E>>,
        display: String,
    },
    EquipmentGrant(Vec<StaticAbility<T, E, C, Cond>>),
    SoulbondSharedPowerToughness {
        power: i32,
        toughness: i32,
    },
    SoulbondSharedAbility(Box<StaticAbility<T, E, C, Cond>>),
    SoulbondSharedObjectAbility(Box<AbilityModel<T, E, C, Cond>>),
    RemoveAbilityForFilter {
        filter: ObjectFilter,
        ability: Box<StaticAbility<T, E, C, Cond>>,
    },
    RemoveAllAbilities(ObjectFilter),
    RemoveAllAbilitiesExceptMana(ObjectFilter),
    SetBasePowerToughness {
        filter: ObjectFilter,
        power: i32,
        toughness: i32,
    },
    AddCardTypes {
        filter: ObjectFilter,
        card_types: Vec<CardType>,
    },
    RemoveCardTypes {
        filter: ObjectFilter,
        card_types: Vec<CardType>,
        condition: Option<Condition>,
    },
    SetCardTypes {
        filter: ObjectFilter,
        card_types: Vec<CardType>,
    },
    AddSubtypes {
        filter: ObjectFilter,
        subtypes: Vec<Subtype>,
    },
    AddAllSubtypesOfFamily {
        filter: ObjectFilter,
        family: SubtypeFamily,
    },
    SetLandSubtypes {
        filter: ObjectFilter,
        subtypes: Vec<Subtype>,
    },
    SetCreatureSubtypes {
        filter: ObjectFilter,
        subtypes: Vec<Subtype>,
    },
    MakeColorless(ObjectFilter),
    CostIncreasePerTargetBeyondFirst(u32),
    CostIncreaseManaCostPerTargetBeyondFirst(ManaCost),
    MinimumSpellTotalMana(u32),
    ActivatedAbilityCostReduction {
        filter: ObjectFilter,
        reduction: u32,
        replacement_mana_cost: Option<ManaCost>,
        display: Option<String>,
        condition: Option<ActivatedAbilityCostCondition>,
        per_matching_objects: Option<ObjectFilter>,
        per_basic_land_types_among: Option<ObjectFilter>,
        minimum_total_mana: Option<u32>,
    },
    ActivatedAbilityCostIncrease {
        filter: ObjectFilter,
        increase: TotalCost<C>,
        activator: Option<PlayerFilter>,
        non_mana_only: bool,
        condition: Option<Condition>,
    },
    ChoosePlayerAsEnters(String),
    EntersUnderOpponentControlAsEnters(String),
    ChooseCardNameAsEnters(String),
    ChooseCreatureTypeAsEnters(String),
    ChooseNamedOptionAsEnters {
        options: Vec<String>,
        display: String,
    },
    ChoosePowerToughnessAsEntersOrTurnsFaceUp {
        options: Vec<PowerToughnessChoiceOption<T, E, C, Cond>>,
        display: String,
    },
    EnterAsCopyAsEnters {
        spec: EnterAsCopyAsEntersSpec<T, E, C, Cond>,
        display: String,
    },
    EntersWithCharacteristicsForFilter {
        filter: ObjectFilter,
        card_types: Vec<CardType>,
        subtypes: Vec<Subtype>,
        power: i32,
        toughness: i32,
    },
    DoubleDamageFromSourcesYouControlOfChosenType(String),
    RedirectDamageToSourceController {
        source_filter: ObjectFilter,
        target_player_filter: PlayerFilter,
        display: String,
    },
    AdditionalLandPlays(u32),
    RevealFirstCardYouDrawEachTurn {
        optional: bool,
        your_turns_only: bool,
    },
    ExileToCounteredExileInsteadOfGraveyard {
        player: PlayerFilter,
        counter_type: CounterType,
    },
    ExileToExileInsteadOfGraveyard {
        filter: ObjectFilter,
        graveyard_owner: PlayerFilter,
        exclude_cycled: bool,
    },
    ExileWouldDieInstead {
        filter: ObjectFilter,
        damaged_by: Option<DamagedBySource>,
        exile_with_counters: Vec<(CounterType, u32)>,
        follow_up_effects: Vec<E>,
    },
    ModifyDamageAmountReplacement {
        source_filter: ObjectFilter,
        target_player_filter: Option<PlayerFilter>,
        target_object_filter: Option<ObjectFilter>,
        delta: i32,
        display: String,
    },
    MinimumDamageAmountReplacement {
        source_filter: ObjectFilter,
        target_player_filter: Option<PlayerFilter>,
        target_object_filter: Option<ObjectFilter>,
        floor: Value,
        noncombat_only: bool,
        display: String,
    },
    DoubleDamageAmountReplacement {
        source_filter: ObjectFilter,
        target_player_filter: Option<PlayerFilter>,
        target_object_filter: Option<ObjectFilter>,
        display: String,
    },
    DoubleCountersReplacement {
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        display: String,
    },
    DoubleTokenCreationReplacement {
        controller: PlayerFilter,
        display: String,
    },
    AddTokenCreationReplacement {
        controller: PlayerFilter,
        token_filter: ObjectFilter,
        additional_token: AdditionalTokenKind,
        additional: i32,
        display: String,
    },
    KeywordActionReplacement {
        action: KeywordActionKind,
        source_filter: ObjectFilter,
        replacement_effects: Vec<E>,
        display: String,
    },
    ConditionalDrawReplacement {
        condition: Condition,
        replacement_effects: Vec<E>,
        display: String,
    },
    CharacteristicDefiningPt {
        power: Value,
        toughness: Value,
    },
    DiscardOrRedirectReplacement {
        filter: ObjectFilter,
        redirect_zone: Zone,
    },
    PayLifeOrEnterTapped(u32),
    ManaSpendPermission {
        permission: ManaSpendPermission,
        display: String,
    },
    Landwalk(LandwalkKind),
    Bloodthirst(u32),
    PreventDamageToSelfRemoveCounter {
        counter_type: CounterType,
        amount: u32,
    },
    PreventDamageToSelfPutCountersInstead {
        counter_type: CounterType,
        display: String,
    },
    PreventConstrainedDamageToSelfPutCountersInstead {
        counter_type: CounterType,
        display: String,
        source_filter: Option<ObjectFilter>,
        combat_only: Option<bool>,
    },
    PreventDamageToYouFromSourceFilter {
        amount: u32,
        source_filter: ObjectFilter,
        display: String,
    },
    ReplaceDamageWithCountersInstead {
        counter_type: CounterType,
        display: String,
        source_filter: ObjectFilter,
        target_filter: ObjectFilter,
        combat_only: Option<bool>,
    },
    CantAttackYouUnlessControllerPaysPerAttacker(u32),
    CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl,
    Grants(Box<GrantSpecModel<T, E, C, Cond>>),
    EntersTappedUnlessCondition {
        condition: Condition,
        display: String,
    },
    EntersWithCountersIfCondition {
        counter: CounterType,
        count: Value,
        condition: Condition,
        display: String,
        added_abilities: Vec<AbilityModel<T, E, C, Cond>>,
    },
    EntersWithCountersValue {
        counter: CounterType,
        count: Value,
    },
    EntersTappedForFilter(ObjectFilter),
    EntersUntappedForFilter(ObjectFilter),
    EntersWithCountersAndSubtypesForFilter {
        filter: ObjectFilter,
        counter: CounterType,
        count: Value,
        subtypes: Vec<Subtype>,
    },
}

impl<T, E, C, Cond> StaticAbility<T, E, C, Cond>
where
    C: Clone,
{
    pub fn try_map<T2, E2, C2, Err, FT, FE, FC>(
        self,
        mut map_trigger: FT,
        mut map_effect: FE,
        mut map_cost: FC,
    ) -> Result<StaticAbility<T2, E2, C2, Cond>, Err>
    where
        E2: Clone,
        C2: CostComponent,
        FT: FnMut(T) -> Result<T2, Err>,
        FE: FnMut(E) -> Result<E2, Err>,
        FC: FnMut(C) -> Result<C2, Err>,
    {
        fn map_total_cost<C, C2, Err, FC>(
            cost: TotalCost<C>,
            map_cost: &mut FC,
        ) -> Result<TotalCost<C2>, Err>
        where
            C: Clone,
            FC: FnMut(C) -> Result<C2, Err>,
        {
            let mut mapped = Vec::new();
            for cost in cost.costs().iter().cloned() {
                mapped.push(map_cost(cost)?);
            }
            Ok(TotalCost::from_costs(mapped))
        }

        fn map_triggered<T, E, T2, E2, Err, FT, FE>(
            triggered: TriggeredAbility<T, E>,
            map_trigger: &mut FT,
            map_effect: &mut FE,
        ) -> Result<TriggeredAbility<T2, E2>, Err>
        where
            E2: Clone,
            FT: FnMut(T) -> Result<T2, Err>,
            FE: FnMut(E) -> Result<E2, Err>,
        {
            Ok(TriggeredAbility {
                trigger: map_trigger(triggered.trigger)?,
                effects: triggered.effects.try_map_effects(map_effect)?,
                choices: triggered.choices,
                intervening_if: triggered.intervening_if,
                presentation_label: None,
            })
        }

        fn map_activated<E, C, E2, C2, Err, FE, FC>(
            activated: ActivatedAbility<E, C>,
            map_effect: &mut FE,
            map_cost: &mut FC,
        ) -> Result<ActivatedAbility<E2, C2>, Err>
        where
            C: Clone,
            E2: Clone,
            FE: FnMut(E) -> Result<E2, Err>,
            FC: FnMut(C) -> Result<C2, Err>,
        {
            Ok(ActivatedAbility {
                mana_cost: map_total_cost(activated.mana_cost, map_cost)?,
                effects: activated.effects.try_map_effects(map_effect)?,
                choices: activated.choices,
                timing: activated.timing,
                additional_restrictions: activated.additional_restrictions,
                activation_restrictions: activated.activation_restrictions,
                mana_output: activated.mana_output,
                activation_condition: activated.activation_condition,
                mana_usage_restrictions: activated.mana_usage_restrictions,
                is_loyalty_ability: activated.is_loyalty_ability,
            })
        }

        fn map_ability<T, E, C, Cond, T2, E2, C2, Err, FT, FE, FC>(
            ability: Ability<StaticAbility<T, E, C, Cond>, T, E, C>,
            map_trigger: &mut FT,
            map_effect: &mut FE,
            map_cost: &mut FC,
        ) -> Result<Ability<StaticAbility<T2, E2, C2, Cond>, T2, E2, C2>, Err>
        where
            C: Clone,
            E2: Clone,
            C2: CostComponent,
            FT: FnMut(T) -> Result<T2, Err>,
            FE: FnMut(E) -> Result<E2, Err>,
            FC: FnMut(C) -> Result<C2, Err>,
        {
            let kind = match ability.kind {
                AbilityKind::Static(static_ability) => AbilityKind::Static(map_static_ability(
                    static_ability,
                    map_trigger,
                    map_effect,
                    map_cost,
                )?),
                AbilityKind::Triggered(triggered) => {
                    AbilityKind::Triggered(map_triggered(triggered, map_trigger, map_effect)?)
                }
                AbilityKind::Activated(activated) => {
                    AbilityKind::Activated(map_activated(activated, map_effect, map_cost)?)
                }
            };
            Ok(Ability {
                kind,
                functional_zones: ability.functional_zones,
            })
        }

        fn map_alternative_cast<E, C, Cond, E2, C2, Err, FE, FC>(
            method: AlternativeCastingMethod<E, C, Cond>,
            map_effect: &mut FE,
            map_cost: &mut FC,
        ) -> Result<AlternativeCastingMethod<E2, C2, Cond>, Err>
        where
            C: Clone,
            E2: Clone,
            C2: CostComponent,
            FE: FnMut(E) -> Result<E2, Err>,
            FC: FnMut(C) -> Result<C2, Err>,
        {
            method.try_map(map_effect, map_cost)
        }

        fn map_derived_alternative_cast<C, C2, Err, FC>(
            spec: DerivedAlternativeCast<C>,
            map_cost: &mut FC,
        ) -> Result<DerivedAlternativeCast<C2>, Err>
        where
            C: Clone,
            FC: FnMut(C) -> Result<C2, Err>,
        {
            Ok(match spec {
                DerivedAlternativeCast::FlashbackFromCardManaCost { additional_costs } => {
                    let mut mapped = Vec::with_capacity(additional_costs.len());
                    for cost in additional_costs {
                        mapped.push(map_cost(cost)?);
                    }
                    DerivedAlternativeCast::FlashbackFromCardManaCost {
                        additional_costs: mapped,
                    }
                }
                DerivedAlternativeCast::EscapeFromCardManaCost { exile_count } => {
                    DerivedAlternativeCast::EscapeFromCardManaCost { exile_count }
                }
                DerivedAlternativeCast::RetraceFromCardManaCost => {
                    DerivedAlternativeCast::RetraceFromCardManaCost
                }
                DerivedAlternativeCast::BlitzFromCardManaCost => {
                    DerivedAlternativeCast::BlitzFromCardManaCost
                }
                DerivedAlternativeCast::EmergeFromCardManaCost => {
                    DerivedAlternativeCast::EmergeFromCardManaCost
                }
                DerivedAlternativeCast::MiracleFromCardManaCostReducedBy { reduction } => {
                    DerivedAlternativeCast::MiracleFromCardManaCostReducedBy { reduction }
                }
                DerivedAlternativeCast::ManaValueAsGenericFromHand => {
                    DerivedAlternativeCast::ManaValueAsGenericFromHand
                }
                DerivedAlternativeCast::LifeEqualManaValueFromHand { usage_limit } => {
                    DerivedAlternativeCast::LifeEqualManaValueFromHand { usage_limit }
                }
                DerivedAlternativeCast::LifeEqualManaValueFromZone { zone, usage_limit } => {
                    DerivedAlternativeCast::LifeEqualManaValueFromZone { zone, usage_limit }
                }
                DerivedAlternativeCast::GraveyardCastFromCardManaCost {
                    additional_costs,
                    usage_limit,
                    condition,
                    exiles_after_resolution,
                } => {
                    let mut mapped = Vec::with_capacity(additional_costs.len());
                    for cost in additional_costs {
                        mapped.push(map_cost(cost)?);
                    }
                    DerivedAlternativeCast::GraveyardCastFromCardManaCost {
                        additional_costs: mapped,
                        usage_limit,
                        condition,
                        exiles_after_resolution,
                    }
                }
            })
        }

        fn map_grantable<T, E, C, Cond, T2, E2, C2, Err, FT, FE, FC>(
            grantable: Grantable<StaticAbility<T, E, C, Cond>, E, C, Cond>,
            map_trigger: &mut FT,
            map_effect: &mut FE,
            map_cost: &mut FC,
        ) -> Result<Grantable<StaticAbility<T2, E2, C2, Cond>, E2, C2, Cond>, Err>
        where
            C: Clone,
            E2: Clone,
            C2: CostComponent,
            FT: FnMut(T) -> Result<T2, Err>,
            FE: FnMut(E) -> Result<E2, Err>,
            FC: FnMut(C) -> Result<C2, Err>,
        {
            Ok(match grantable {
                Grantable::Ability(static_ability) => Grantable::Ability(map_static_ability(
                    static_ability,
                    map_trigger,
                    map_effect,
                    map_cost,
                )?),
                Grantable::AlternativeCast(method) => {
                    Grantable::AlternativeCast(map_alternative_cast(method, map_effect, map_cost)?)
                }
                Grantable::DerivedAlternativeCast(spec) => {
                    Grantable::DerivedAlternativeCast(map_derived_alternative_cast(spec, map_cost)?)
                }
                Grantable::PlayFrom => Grantable::PlayFrom,
            })
        }

        fn map_grant_spec<T, E, C, Cond, T2, E2, C2, Err, FT, FE, FC>(
            spec: GrantSpec<StaticAbility<T, E, C, Cond>, E, C, Cond>,
            map_trigger: &mut FT,
            map_effect: &mut FE,
            map_cost: &mut FC,
        ) -> Result<GrantSpec<StaticAbility<T2, E2, C2, Cond>, E2, C2, Cond>, Err>
        where
            C: Clone,
            E2: Clone,
            C2: CostComponent,
            FT: FnMut(T) -> Result<T2, Err>,
            FE: FnMut(E) -> Result<E2, Err>,
            FC: FnMut(C) -> Result<C2, Err>,
        {
            Ok(GrantSpec {
                grantable: map_grantable(spec.grantable, map_trigger, map_effect, map_cost)?,
                filter: spec.filter,
                zone: spec.zone,
                beneficiary: spec.beneficiary,
                cast_this_way_grants: spec
                    .cast_this_way_grants
                    .into_iter()
                    .map(|ability| map_static_ability(ability, map_trigger, map_effect, map_cost))
                    .collect::<Result<Vec<_>, _>>()?,
            })
        }

        fn map_static_ability<T, E, C, Cond, T2, E2, C2, Err, FT, FE, FC>(
            ability: StaticAbility<T, E, C, Cond>,
            map_trigger: &mut FT,
            map_effect: &mut FE,
            map_cost: &mut FC,
        ) -> Result<StaticAbility<T2, E2, C2, Cond>, Err>
        where
            C: Clone,
            E2: Clone,
            C2: CostComponent,
            FT: FnMut(T) -> Result<T2, Err>,
            FE: FnMut(E) -> Result<E2, Err>,
            FC: FnMut(C) -> Result<C2, Err>,
        {
            let payload = match ability.payload {
            StaticAbilityPayload::None => StaticAbilityPayload::None,
            StaticAbilityPayload::Anthem(anthem) => StaticAbilityPayload::Anthem(anthem),
            StaticAbilityPayload::AttachedAbilityGrant(grant) => {
                let grant = *grant;
                StaticAbilityPayload::AttachedAbilityGrant(Box::new(AttachedAbilityGrant {
                    ability: map_ability(grant.ability, map_trigger, map_effect, map_cost)?,
                    display: grant.display,
                    condition: grant.condition,
                }))
            }
            StaticAbilityPayload::AttachedChosenLandwalkGrant(grant) => {
                StaticAbilityPayload::AttachedChosenLandwalkGrant(grant)
            }
            StaticAbilityPayload::Conditional { ability, condition } => {
                StaticAbilityPayload::Conditional {
                    ability: Box::new(map_static_ability(
                        *ability,
                        map_trigger,
                        map_effect,
                        map_cost,
                    )?),
                    condition,
                }
            }
            StaticAbilityPayload::GrantAbility(grant) => {
                let grant = *grant;
                StaticAbilityPayload::GrantAbility(Box::new(GrantAbility {
                    filter: grant.filter,
                    ability: map_ability(grant.ability, map_trigger, map_effect, map_cost)?,
                    condition: grant.condition,
                }))
            }
            StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
                let grant = *grant;
                StaticAbilityPayload::GrantObjectAbilityForFilter(Box::new(
                    GrantObjectAbilityForFilter {
                        filter: grant.filter,
                        ability: map_ability(
                            grant.ability,
                            map_trigger,
                            map_effect,
                            map_cost,
                        )?,
                        display: grant.display,
                        condition: grant.condition,
                    },
                ))
            }
            StaticAbilityPayload::CopyActivatedAbilities(copy) => {
                StaticAbilityPayload::CopyActivatedAbilities(copy)
            }
            StaticAbilityPayload::CopyTriggeredAbilities(copy) => {
                StaticAbilityPayload::CopyTriggeredAbilities(copy)
            }
            StaticAbilityPayload::CostReduction(reduction) => {
                StaticAbilityPayload::CostReduction(reduction)
            }
            StaticAbilityPayload::CostReductionManaCost(reduction) => {
                StaticAbilityPayload::CostReductionManaCost(reduction)
            }
            StaticAbilityPayload::CostIncrease(increase) => {
                StaticAbilityPayload::CostIncrease(increase)
            }
            StaticAbilityPayload::CostIncreaseManaCost(increase) => {
                StaticAbilityPayload::CostIncreaseManaCost(increase)
            }
            StaticAbilityPayload::ThisSpellCostReduction(reduction) => {
                StaticAbilityPayload::ThisSpellCostReduction(reduction)
            }
            StaticAbilityPayload::ThisSpellCostReductionManaCost(reduction) => {
                StaticAbilityPayload::ThisSpellCostReductionManaCost(reduction)
            }
            StaticAbilityPayload::ThisSpellCastRestriction { kind, display } => {
                StaticAbilityPayload::ThisSpellCastRestriction { kind, display }
            }
            StaticAbilityPayload::ThisSpellXMaximum { maximum, display } => {
                StaticAbilityPayload::ThisSpellXMaximum { maximum, display }
            }
            StaticAbilityPayload::LevelAbility(level) => {
                let level = *level;
                let mut abilities = Vec::with_capacity(level.abilities.len());
                for ability in level.abilities {
                    abilities.push(map_static_ability(
                        ability,
                        map_trigger,
                        map_effect,
                        map_cost,
                    )?);
                }
                StaticAbilityPayload::LevelAbility(Box::new(crate::LevelAbility {
                    min_level: level.min_level,
                    max_level: level.max_level,
                    power_toughness: level.power_toughness,
                    abilities,
                }))
            }
            StaticAbilityPayload::HexproofFrom(filter) => StaticAbilityPayload::HexproofFrom(filter),
            StaticAbilityPayload::Protection(from) => StaticAbilityPayload::Protection(from),
            StaticAbilityPayload::PreventAllCombatDamageToPermanentsMatching(filter) => {
                StaticAbilityPayload::PreventAllCombatDamageToPermanentsMatching(filter)
            }
            StaticAbilityPayload::RuleRestriction {
                restriction,
                display,
            } => StaticAbilityPayload::RuleRestriction {
                restriction,
                display,
            },
            StaticAbilityPayload::PregameAction { kind, text } => {
                StaticAbilityPayload::PregameAction { kind, text }
            }
            StaticAbilityPayload::Ward(cost) => {
                StaticAbilityPayload::Ward(map_total_cost(cost, map_cost)?)
            }
            StaticAbilityPayload::Morph(cost) => {
                StaticAbilityPayload::Morph(map_total_cost(cost, map_cost)?)
            }
            StaticAbilityPayload::Disguise(cost) => {
                StaticAbilityPayload::Disguise(map_total_cost(cost, map_cost)?)
            }
            StaticAbilityPayload::Megamorph(cost) => {
                StaticAbilityPayload::Megamorph(map_total_cost(cost, map_cost)?)
            }
            StaticAbilityPayload::CanBlockAdditionalCreatureEachCombat(count) => {
                StaticAbilityPayload::CanBlockAdditionalCreatureEachCombat(count)
            }
            StaticAbilityPayload::CanBlockAsThoughReachForSubtype(subtype) => {
                StaticAbilityPayload::CanBlockAsThoughReachForSubtype(subtype)
            }
            StaticAbilityPayload::CantBeBlockedByMoreThan(count) => {
                StaticAbilityPayload::CantBeBlockedByMoreThan(count)
            }
            StaticAbilityPayload::CantBeBlockedExceptByNOrMore(count) => {
                StaticAbilityPayload::CantBeBlockedExceptByNOrMore(count)
            }
            StaticAbilityPayload::CantBeBlockedByPowerOrLess(power) => {
                StaticAbilityPayload::CantBeBlockedByPowerOrLess(power)
            }
            StaticAbilityPayload::CantBeBlockedByPowerOrGreater(power) => {
                StaticAbilityPayload::CantBeBlockedByPowerOrGreater(power)
            }
            StaticAbilityPayload::CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes(
                card_types,
            ) => StaticAbilityPayload::CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes(
                card_types,
            ),
            StaticAbilityPayload::CantAttackUnlessCondition { condition, display } => {
                StaticAbilityPayload::CantAttackUnlessCondition { condition, display }
            }
            StaticAbilityPayload::MayChooseNotToUntapDuringUntapStep(subject) => {
                StaticAbilityPayload::MayChooseNotToUntapDuringUntapStep(subject)
            }
            StaticAbilityPayload::UntapDuringEachOtherPlayersUntapStep { filter, display } => {
                StaticAbilityPayload::UntapDuringEachOtherPlayersUntapStep { filter, display }
            }
            StaticAbilityPayload::FirstEquipCostAlternative(display) => {
                StaticAbilityPayload::FirstEquipCostAlternative(display)
            }
            StaticAbilityPayload::ControlAttachedPermanent(display) => {
                StaticAbilityPayload::ControlAttachedPermanent(display)
            }
            StaticAbilityPayload::SetColors { filter, colors } => {
                StaticAbilityPayload::SetColors { filter, colors }
            }
            StaticAbilityPayload::AddColors { filter, colors } => {
                StaticAbilityPayload::AddColors { filter, colors }
            }
            StaticAbilityPayload::SetName { filter, name } => {
                StaticAbilityPayload::SetName { filter, name }
            }
            StaticAbilityPayload::AddSupertypes { filter, supertypes } => {
                StaticAbilityPayload::AddSupertypes { filter, supertypes }
            }
            StaticAbilityPayload::RemoveSupertypes { filter, supertypes } => {
                StaticAbilityPayload::RemoveSupertypes { filter, supertypes }
            }
            StaticAbilityPayload::MaxCreaturesCanAttackEachCombat(maximum) => {
                StaticAbilityPayload::MaxCreaturesCanAttackEachCombat(maximum)
            }
            StaticAbilityPayload::MaxCreaturesCanAttackYouEachCombat(maximum) => {
                StaticAbilityPayload::MaxCreaturesCanAttackYouEachCombat(maximum)
            }
            StaticAbilityPayload::MaxCreaturesCanBlockEachCombat(maximum) => {
                StaticAbilityPayload::MaxCreaturesCanBlockEachCombat(maximum)
            }
            StaticAbilityPayload::ChooseBasicLandTypeAsEnters(display) => {
                StaticAbilityPayload::ChooseBasicLandTypeAsEnters(display)
            }
            StaticAbilityPayload::ChooseLandTypeAsEnters(display) => {
                StaticAbilityPayload::ChooseLandTypeAsEnters(display)
            }
            StaticAbilityPayload::EnchantedLandIsChosenType(display) => {
                StaticAbilityPayload::EnchantedLandIsChosenType(display)
            }
            StaticAbilityPayload::AddChosenCreatureType { filter, display } => {
                StaticAbilityPayload::AddChosenCreatureType { filter, display }
            }
            StaticAbilityPayload::AddChosenColor { filter, display } => {
                StaticAbilityPayload::AddChosenColor { filter, display }
            }
            StaticAbilityPayload::SetChosenColor { filter, display } => {
                StaticAbilityPayload::SetChosenColor { filter, display }
            }
            StaticAbilityPayload::SetMaximumHandSize { player, amount } => {
                StaticAbilityPayload::SetMaximumHandSize { player, amount }
            }
            StaticAbilityPayload::ReduceMaximumHandSize { player, by } => {
                StaticAbilityPayload::ReduceMaximumHandSize { player, by }
            }
            StaticAbilityPayload::MaximumHandSizeSevenMinusYourGraveyardCardTypes {
                player,
                min_card_types,
            } => StaticAbilityPayload::MaximumHandSizeSevenMinusYourGraveyardCardTypes {
                player,
                min_card_types,
            },
            StaticAbilityPayload::DuplicateMatchingTriggeredAbilities {
                source_filter,
                event_matcher,
                count,
                display,
            } => StaticAbilityPayload::DuplicateMatchingTriggeredAbilities {
                source_filter,
                event_matcher: event_matcher.map(|matcher| map_trigger(matcher)).transpose()?,
                count,
                display,
            },
            StaticAbilityPayload::SuppressMatchingTriggeredAbilities {
                source_filter,
                event_matcher,
                display,
            } => StaticAbilityPayload::SuppressMatchingTriggeredAbilities {
                source_filter,
                event_matcher: event_matcher.map(|matcher| map_trigger(matcher)).transpose()?,
                display,
            },
            StaticAbilityPayload::ExertAttack {
                only_if_not_exerted_this_turn,
                linked_trigger,
                display,
            } => StaticAbilityPayload::ExertAttack {
                only_if_not_exerted_this_turn,
                linked_trigger: linked_trigger
                    .map(|triggered| map_triggered(triggered, map_trigger, map_effect))
                    .transpose()?,
                display,
            },
            StaticAbilityPayload::EquipmentGrant(abilities) => {
                let mut mapped = Vec::with_capacity(abilities.len());
                for ability in abilities {
                    mapped.push(map_static_ability(
                        ability,
                        map_trigger,
                        map_effect,
                        map_cost,
                    )?);
                }
                StaticAbilityPayload::EquipmentGrant(mapped)
            }
            StaticAbilityPayload::SoulbondSharedPowerToughness { power, toughness } => {
                StaticAbilityPayload::SoulbondSharedPowerToughness { power, toughness }
            }
            StaticAbilityPayload::SoulbondSharedAbility(ability) => {
                StaticAbilityPayload::SoulbondSharedAbility(Box::new(map_static_ability(
                    *ability,
                    map_trigger,
                    map_effect,
                    map_cost,
                )?))
            }
            StaticAbilityPayload::SoulbondSharedObjectAbility(ability) => {
                StaticAbilityPayload::SoulbondSharedObjectAbility(Box::new(map_ability(
                    *ability,
                    map_trigger,
                    map_effect,
                    map_cost,
                )?))
            }
            StaticAbilityPayload::RemoveAbilityForFilter { filter, ability } => {
                StaticAbilityPayload::RemoveAbilityForFilter {
                    filter,
                    ability: Box::new(map_static_ability(
                        *ability,
                        map_trigger,
                        map_effect,
                        map_cost,
                    )?),
                }
            }
            StaticAbilityPayload::RemoveAllAbilities(filter) => {
                StaticAbilityPayload::RemoveAllAbilities(filter)
            }
            StaticAbilityPayload::RemoveAllAbilitiesExceptMana(filter) => {
                StaticAbilityPayload::RemoveAllAbilitiesExceptMana(filter)
            }
            StaticAbilityPayload::SetBasePowerToughness {
                filter,
                power,
                toughness,
            } => StaticAbilityPayload::SetBasePowerToughness {
                filter,
                power,
                toughness,
            },
            StaticAbilityPayload::AddCardTypes { filter, card_types } => {
                StaticAbilityPayload::AddCardTypes { filter, card_types }
            }
            StaticAbilityPayload::RemoveCardTypes {
                filter,
                card_types,
                condition,
            } => StaticAbilityPayload::RemoveCardTypes {
                filter,
                card_types,
                condition,
            },
            StaticAbilityPayload::SetCardTypes { filter, card_types } => {
                StaticAbilityPayload::SetCardTypes { filter, card_types }
            }
            StaticAbilityPayload::AddSubtypes { filter, subtypes } => {
                StaticAbilityPayload::AddSubtypes { filter, subtypes }
            }
            StaticAbilityPayload::AddAllSubtypesOfFamily { filter, family } => {
                StaticAbilityPayload::AddAllSubtypesOfFamily { filter, family }
            }
            StaticAbilityPayload::SetLandSubtypes { filter, subtypes } => {
                StaticAbilityPayload::SetLandSubtypes { filter, subtypes }
            }
            StaticAbilityPayload::SetCreatureSubtypes { filter, subtypes } => {
                StaticAbilityPayload::SetCreatureSubtypes { filter, subtypes }
            }
            StaticAbilityPayload::MakeColorless(filter) => StaticAbilityPayload::MakeColorless(filter),
            StaticAbilityPayload::CostIncreasePerTargetBeyondFirst(amount) => {
                StaticAbilityPayload::CostIncreasePerTargetBeyondFirst(amount)
            }
            StaticAbilityPayload::CostIncreaseManaCostPerTargetBeyondFirst(cost) => {
                StaticAbilityPayload::CostIncreaseManaCostPerTargetBeyondFirst(cost)
            }
            StaticAbilityPayload::MinimumSpellTotalMana(amount) => {
                StaticAbilityPayload::MinimumSpellTotalMana(amount)
            }
            StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction,
                replacement_mana_cost,
                display,
                condition,
                per_matching_objects,
                per_basic_land_types_among,
                minimum_total_mana,
            } => StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction,
                replacement_mana_cost,
                display,
                condition,
                per_matching_objects,
                per_basic_land_types_among,
                minimum_total_mana,
            },
            StaticAbilityPayload::ActivatedAbilityCostIncrease {
                filter,
                increase,
                activator,
                non_mana_only,
                condition,
            } => StaticAbilityPayload::ActivatedAbilityCostIncrease {
                filter,
                increase: map_total_cost(increase, map_cost)?,
                activator,
                non_mana_only,
                condition,
            },
            StaticAbilityPayload::ChoosePlayerAsEnters(display) => {
                StaticAbilityPayload::ChoosePlayerAsEnters(display)
            }
            StaticAbilityPayload::EntersUnderOpponentControlAsEnters(display) => {
                StaticAbilityPayload::EntersUnderOpponentControlAsEnters(display)
            }
            StaticAbilityPayload::ChooseCardNameAsEnters(display) => {
                StaticAbilityPayload::ChooseCardNameAsEnters(display)
            }
            StaticAbilityPayload::ChooseCreatureTypeAsEnters(display) => {
                StaticAbilityPayload::ChooseCreatureTypeAsEnters(display)
            }
            StaticAbilityPayload::ChooseNamedOptionAsEnters { options, display } => {
                StaticAbilityPayload::ChooseNamedOptionAsEnters { options, display }
            }
            StaticAbilityPayload::ChoosePowerToughnessAsEntersOrTurnsFaceUp {
                options,
                display,
            } => StaticAbilityPayload::ChoosePowerToughnessAsEntersOrTurnsFaceUp {
                options: options
                    .into_iter()
                    .map(|option| {
                        Ok(PowerToughnessChoiceOption {
                            power: option.power,
                            toughness: option.toughness,
                            abilities: option
                                .abilities
                                .into_iter()
                                .map(|ability| {
                                    map_static_ability(ability, map_trigger, map_effect, map_cost)
                                })
                                .collect::<Result<Vec<_>, _>>()?,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                display,
            },
            StaticAbilityPayload::EnterAsCopyAsEnters { spec, display } => {
                let mut added_abilities = Vec::with_capacity(spec.added_abilities.len());
                for ability in spec.added_abilities {
                    added_abilities.push(map_ability(
                        ability,
                        map_trigger,
                        map_effect,
                        map_cost,
                    )?);
                }
                StaticAbilityPayload::EnterAsCopyAsEnters {
                    spec: EnterAsCopyAsEntersSpec {
                        filter: spec.filter,
                        affected_filter: spec.affected_filter,
                        may: spec.may,
                        enters_tapped_if_chosen: spec.enters_tapped_if_chosen,
                        linked_exile_pair: spec.linked_exile_pair,
                        copy_source_self: spec.copy_source_self,
                        copy_source_enchanted: spec.copy_source_enchanted,
                        name_override: spec.name_override,
                        added_card_types: spec.added_card_types,
                        removed_supertypes: spec.removed_supertypes,
                        added_subtypes: spec.added_subtypes,
                        added_abilities,
                        set_base_power_toughness: spec.set_base_power_toughness,
                        set_base_power_toughness_from_self: spec
                            .set_base_power_toughness_from_self,
                    },
                    display,
                }
            }
            StaticAbilityPayload::EntersWithCharacteristicsForFilter {
                filter,
                card_types,
                subtypes,
                power,
                toughness,
            } => StaticAbilityPayload::EntersWithCharacteristicsForFilter {
                filter,
                card_types,
                subtypes,
                power,
                toughness,
            },
            StaticAbilityPayload::DoubleDamageFromSourcesYouControlOfChosenType(display) => {
                StaticAbilityPayload::DoubleDamageFromSourcesYouControlOfChosenType(display)
            }
            StaticAbilityPayload::RedirectDamageToSourceController {
                source_filter,
                target_player_filter,
                display,
            } => StaticAbilityPayload::RedirectDamageToSourceController {
                source_filter,
                target_player_filter,
                display,
            },
            StaticAbilityPayload::AdditionalLandPlays(count) => {
                StaticAbilityPayload::AdditionalLandPlays(count)
            }
            StaticAbilityPayload::RevealFirstCardYouDrawEachTurn {
                optional,
                your_turns_only,
            } => StaticAbilityPayload::RevealFirstCardYouDrawEachTurn {
                optional,
                your_turns_only,
            },
            StaticAbilityPayload::ExileToCounteredExileInsteadOfGraveyard {
                player,
                counter_type,
            } => StaticAbilityPayload::ExileToCounteredExileInsteadOfGraveyard {
                player,
                counter_type,
            },
            StaticAbilityPayload::ExileToExileInsteadOfGraveyard {
                filter,
                graveyard_owner,
                exclude_cycled,
            } => StaticAbilityPayload::ExileToExileInsteadOfGraveyard {
                filter,
                graveyard_owner,
                exclude_cycled,
            },
            StaticAbilityPayload::ExileWouldDieInstead {
                filter,
                damaged_by,
                exile_with_counters,
                follow_up_effects,
            } => StaticAbilityPayload::ExileWouldDieInstead {
                filter,
                damaged_by,
                exile_with_counters,
                follow_up_effects: follow_up_effects
                    .into_iter()
                    .map(map_effect)
                    .collect::<Result<Vec<_>, _>>()?,
            },
            StaticAbilityPayload::ModifyDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                delta,
                display,
            } => StaticAbilityPayload::ModifyDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                delta,
                display,
            },
            StaticAbilityPayload::MinimumDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                floor,
                noncombat_only,
                display,
            } => StaticAbilityPayload::MinimumDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                floor,
                noncombat_only,
                display,
            },
            StaticAbilityPayload::DoubleDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                display,
            } => StaticAbilityPayload::DoubleDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                display,
            },
            StaticAbilityPayload::DoubleCountersReplacement {
                filter,
                counter_type,
                display,
            } => StaticAbilityPayload::DoubleCountersReplacement {
                filter,
                counter_type,
                display,
            },
            StaticAbilityPayload::DoubleTokenCreationReplacement {
                controller,
                display,
            } => StaticAbilityPayload::DoubleTokenCreationReplacement {
                controller,
                display,
            },
            StaticAbilityPayload::AddTokenCreationReplacement {
                controller,
                token_filter,
                additional_token,
                additional,
                display,
            } => StaticAbilityPayload::AddTokenCreationReplacement {
                controller,
                token_filter,
                additional_token,
                additional,
                display,
            },
            StaticAbilityPayload::KeywordActionReplacement {
                action,
                source_filter,
                replacement_effects,
                display,
            } => StaticAbilityPayload::KeywordActionReplacement {
                action,
                source_filter,
                replacement_effects: replacement_effects
                    .into_iter()
                    .map(map_effect)
                    .collect::<Result<Vec<_>, _>>()?,
                display,
            },
            StaticAbilityPayload::ConditionalDrawReplacement {
                condition,
                replacement_effects,
                display,
            } => StaticAbilityPayload::ConditionalDrawReplacement {
                condition,
                replacement_effects: replacement_effects
                    .into_iter()
                    .map(map_effect)
                    .collect::<Result<Vec<_>, _>>()?,
                display,
            },
            StaticAbilityPayload::CharacteristicDefiningPt { power, toughness } => {
                StaticAbilityPayload::CharacteristicDefiningPt { power, toughness }
            }
            StaticAbilityPayload::DiscardOrRedirectReplacement {
                filter,
                redirect_zone,
            } => StaticAbilityPayload::DiscardOrRedirectReplacement {
                filter,
                redirect_zone,
            },
            StaticAbilityPayload::PayLifeOrEnterTapped(value) => {
                StaticAbilityPayload::PayLifeOrEnterTapped(value)
            }
            StaticAbilityPayload::ManaSpendPermission {
                permission,
                display,
            } => StaticAbilityPayload::ManaSpendPermission {
                permission,
                display,
            },
            StaticAbilityPayload::Landwalk(kind) => StaticAbilityPayload::Landwalk(kind),
            StaticAbilityPayload::Bloodthirst(amount) => {
                StaticAbilityPayload::Bloodthirst(amount)
            }
            StaticAbilityPayload::PreventDamageToSelfRemoveCounter {
                counter_type,
                amount,
            } => StaticAbilityPayload::PreventDamageToSelfRemoveCounter {
                counter_type,
                amount,
            },
            StaticAbilityPayload::PreventDamageToSelfPutCountersInstead {
                counter_type,
                display,
            } => StaticAbilityPayload::PreventDamageToSelfPutCountersInstead {
                counter_type,
                display,
            },
            StaticAbilityPayload::PreventConstrainedDamageToSelfPutCountersInstead {
                counter_type,
                display,
                source_filter,
                combat_only,
            } => StaticAbilityPayload::PreventConstrainedDamageToSelfPutCountersInstead {
                counter_type,
                display,
                source_filter,
                combat_only,
            },
            StaticAbilityPayload::PreventDamageToYouFromSourceFilter {
                amount,
                source_filter,
                display,
            } => StaticAbilityPayload::PreventDamageToYouFromSourceFilter {
                amount,
                source_filter,
                display,
            },
            StaticAbilityPayload::ReplaceDamageWithCountersInstead {
                counter_type,
                display,
                source_filter,
                target_filter,
                combat_only,
            } => StaticAbilityPayload::ReplaceDamageWithCountersInstead {
                counter_type,
                display,
                source_filter,
                target_filter,
                combat_only,
            },
            StaticAbilityPayload::CantAttackYouUnlessControllerPaysPerAttacker(amount) => {
                StaticAbilityPayload::CantAttackYouUnlessControllerPaysPerAttacker(amount)
            }
            StaticAbilityPayload::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl => {
                StaticAbilityPayload::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
            }
            StaticAbilityPayload::Grants(spec) => {
                StaticAbilityPayload::Grants(Box::new(map_grant_spec(
                    *spec,
                    map_trigger,
                    map_effect,
                    map_cost,
                )?))
            }
            StaticAbilityPayload::EntersTappedUnlessCondition { condition, display } => {
                StaticAbilityPayload::EntersTappedUnlessCondition { condition, display }
            }
            StaticAbilityPayload::EntersWithCountersIfCondition {
                counter,
                count,
                condition,
                display,
                added_abilities,
            } => StaticAbilityPayload::EntersWithCountersIfCondition {
                counter,
                count,
                condition,
                display,
                added_abilities: added_abilities
                    .into_iter()
                    .map(|ability| map_ability(ability, map_trigger, map_effect, map_cost))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            StaticAbilityPayload::EntersWithCountersValue { counter, count } => {
                StaticAbilityPayload::EntersWithCountersValue { counter, count }
            }
            StaticAbilityPayload::EntersTappedForFilter(filter) => {
                StaticAbilityPayload::EntersTappedForFilter(filter)
            }
            StaticAbilityPayload::EntersUntappedForFilter(filter) => {
                StaticAbilityPayload::EntersUntappedForFilter(filter)
            }
            StaticAbilityPayload::EntersWithCountersAndSubtypesForFilter {
                filter,
                counter,
                count,
                subtypes,
            } => StaticAbilityPayload::EntersWithCountersAndSubtypesForFilter {
                filter,
                counter,
                count,
                subtypes,
            },
            };

            Ok(StaticAbility {
                id: ability.id,
                label: ability.label,
                payload,
            })
        }

        map_static_ability(self, &mut map_trigger, &mut map_effect, &mut map_cost)
    }
}

impl<
    T: Clone + PartialEq + std::fmt::Debug + 'static,
    E: Clone + PartialEq + std::fmt::Debug + 'static,
    C: Clone + PartialEq + std::fmt::Debug + 'static,
    Cond: Clone + PartialEq + std::fmt::Debug + 'static,
> StaticAbility<T, E, C, Cond>
{
    fn identified(id: StaticAbilityId, label: impl Into<String>) -> Self {
        Self {
            id: Some(id),
            label: label.into(),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn new(label: impl std::fmt::Debug + 'static) -> Self {
        let label_any = &label as &dyn Any;
        if let Some(id) = label_any.downcast_ref::<StaticAbilityId>() {
            return Self::identified(*id, format!("{id:?}"));
        }
        if let Some(payload) = label_any.downcast_ref::<Anthem>() {
            return Self {
                id: Some(StaticAbilityId::Anthem),
                label: "anthem".to_string(),
                payload: StaticAbilityPayload::Anthem(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<AttachedAbilityGrant<T, E, C, Cond>>() {
            return Self {
                id: Some(StaticAbilityId::AttachedAbilityGrant),
                label: payload.display.clone(),
                payload: StaticAbilityPayload::AttachedAbilityGrant(Box::new(payload.clone())),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<AttachedChosenLandwalkGrant>() {
            return Self {
                id: Some(StaticAbilityId::AttachedChosenLandwalkGrant),
                label: payload.display.clone(),
                payload: StaticAbilityPayload::AttachedChosenLandwalkGrant(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<GrantAbility<T, E, C, Cond>>() {
            return Self {
                id: Some(StaticAbilityId::GrantAbility),
                label: "grant ability".to_string(),
                payload: StaticAbilityPayload::GrantAbility(Box::new(payload.clone())),
            };
        }
        if let Some(payload) =
            label_any.downcast_ref::<GrantObjectAbilityForFilter<T, E, C, Cond>>()
        {
            return Self {
                id: Some(StaticAbilityId::GrantObjectAbilityForFilter),
                label: payload.display.clone(),
                payload: StaticAbilityPayload::GrantObjectAbilityForFilter(Box::new(
                    payload.clone(),
                )),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<RemoveCardTypesForFilter>() {
            return Self {
                id: Some(StaticAbilityId::RemoveCardTypes),
                label: "remove card types".to_string(),
                payload: StaticAbilityPayload::RemoveCardTypes {
                    filter: payload.filter.clone(),
                    card_types: payload.types.clone(),
                    condition: payload.condition.clone(),
                },
            };
        }
        if let Some(payload) = label_any.downcast_ref::<CopyActivatedAbilities>() {
            return Self {
                id: Some(StaticAbilityId::CopyActivatedAbilities),
                label: "copy activated abilities".to_string(),
                payload: StaticAbilityPayload::CopyActivatedAbilities(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<CopyTriggeredAbilities>() {
            return Self {
                id: Some(StaticAbilityId::CopyTriggeredAbilities),
                label: payload.display.clone(),
                payload: StaticAbilityPayload::CopyTriggeredAbilities(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<CostReduction>() {
            return Self {
                id: Some(StaticAbilityId::CostReduction),
                label: "cost reduction".to_string(),
                payload: StaticAbilityPayload::CostReduction(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<CostReductionManaCost>() {
            return Self {
                id: Some(StaticAbilityId::CostReductionManaCost),
                label: "cost reduction mana cost".to_string(),
                payload: StaticAbilityPayload::CostReductionManaCost(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<CostIncrease>() {
            return Self {
                id: Some(StaticAbilityId::CostIncrease),
                label: "cost increase".to_string(),
                payload: StaticAbilityPayload::CostIncrease(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<CostIncreaseManaCost>() {
            return Self {
                id: Some(StaticAbilityId::CostIncreaseManaCost),
                label: "cost increase mana cost".to_string(),
                payload: StaticAbilityPayload::CostIncreaseManaCost(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<ThisSpellCostReduction<Cond>>() {
            return Self {
                id: Some(StaticAbilityId::ThisSpellCostReduction),
                label: "this spell cost reduction".to_string(),
                payload: StaticAbilityPayload::ThisSpellCostReduction(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<ThisSpellCostReductionManaCost<Cond>>() {
            return Self {
                id: Some(StaticAbilityId::ThisSpellCostReductionManaCost),
                label: "this spell cost reduction mana cost".to_string(),
                payload: StaticAbilityPayload::ThisSpellCostReductionManaCost(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<LandwalkKind>() {
            return Self {
                id: Some(StaticAbilityId::Landwalk),
                label: payload.display(),
                payload: StaticAbilityPayload::Landwalk(*payload),
            };
        }
        Self {
            id: None,
            label: format!("{label:?}"),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn level(ability: LevelAbilityModel<T, E, C, Cond>) -> Self {
        Self {
            id: Some(StaticAbilityId::LevelAbilities),
            label: "level".to_string(),
            payload: StaticAbilityPayload::LevelAbility(Box::new(ability)),
        }
    }

    pub fn flash() -> Self {
        Self {
            id: Some(StaticAbilityId::Flash),
            label: "flash".to_string(),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn this_spell_cast_restriction(
        kind: ThisSpellCastRestrictionKind,
        text: impl Into<String>,
    ) -> Self {
        let display = text.into();
        Self {
            id: Some(StaticAbilityId::ThisSpellCastRestriction),
            label: display.clone(),
            payload: StaticAbilityPayload::ThisSpellCastRestriction { kind, display },
        }
    }

    pub fn this_spell_x_maximum(maximum: Value, text: impl Into<String>) -> Self {
        let display = text.into();
        Self {
            id: Some(StaticAbilityId::ThisSpellXMaximum),
            label: display.clone(),
            payload: StaticAbilityPayload::ThisSpellXMaximum { maximum, display },
        }
    }

    pub fn morph(cost: TotalCost<C>) -> Self {
        Self {
            id: Some(StaticAbilityId::Morph),
            label: "morph".to_string(),
            payload: StaticAbilityPayload::Morph(cost),
        }
    }

    pub fn disguise(cost: TotalCost<C>) -> Self {
        Self {
            id: Some(StaticAbilityId::Disguise),
            label: "disguise".to_string(),
            payload: StaticAbilityPayload::Disguise(cost),
        }
    }

    pub fn daybound() -> Self {
        Self::identified(StaticAbilityId::Daybound, "Daybound")
    }

    pub fn nightbound() -> Self {
        Self::identified(StaticAbilityId::Nightbound, "Nightbound")
    }

    pub fn day_night_starts_day_as_enters() -> Self {
        Self::identified(
            StaticAbilityId::DayNightStartsDayAsEnters,
            "If it's neither day nor night, it becomes day as this creature enters",
        )
    }

    pub fn megamorph(cost: TotalCost<C>) -> Self {
        Self {
            id: Some(StaticAbilityId::Megamorph),
            label: "megamorph".to_string(),
            payload: StaticAbilityPayload::Megamorph(cost),
        }
    }

    pub fn keyword_marker(marker: impl std::fmt::Debug) -> Self {
        let text = format!("{marker:?}").trim_matches('"').to_string();
        if let Some(ability) = Self::known_keyword_marker(&text) {
            return ability;
        }
        Self::identified(StaticAbilityId::KeywordMarker, text)
    }

    pub fn keyword_fallback_text(text: impl Into<String>) -> Self {
        Self::identified(StaticAbilityId::KeywordFallbackText, text)
    }

    pub fn draft_rule_text(text: impl Into<String>) -> Self {
        Self::identified(StaticAbilityId::DraftRuleText, text)
    }

    pub fn deck_construction_rule_text(text: impl Into<String>) -> Self {
        Self::identified(StaticAbilityId::DeckConstructionRuleText, text)
    }

    pub fn rule_fallback_text(text: impl Into<String>) -> Self {
        Self::identified(StaticAbilityId::RuleFallbackText, text)
    }

    fn known_keyword_marker(text: &str) -> Option<Self> {
        let normalized = text.trim().trim_end_matches('.').to_ascii_lowercase();
        if normalized.ends_with(" can't be blocked") || normalized.ends_with(" cant be blocked") {
            return Some(Self::unblockable());
        }
        Some(match normalized.as_str() {
            "flying" => Self::flying(),
            "first strike" => Self::first_strike(),
            "double strike" => Self::double_strike(),
            "deathtouch" => Self::deathtouch(),
            "lifelink" => Self::lifelink(),
            "vigilance" => Self::vigilance(),
            "trample" => Self::trample(),
            "reach" => Self::reach(),
            "defender" => Self::defender(),
            "flash" => Self::flash(),
            "haste" => Self::haste(),
            "menace" => Self::menace(),
            "hexproof" => Self::hexproof(),
            "indestructible" => Self::indestructible(),
            "shroud" => Self::shroud(),
            "wither" => Self::wither(),
            "infect" => Self::infect(),
            "skulk" => Self::skulk(),
            "prowess" => Self::prowess(),
            "cascade" => Self::cascade(),
            "unleash" => Self::unleash(),
            "split second" => Self::split_second(),
            "rebound" => Self::rebound(),
            "fear" => Self::fear(),
            "intimidate" => Self::intimidate(),
            "shadow" => Self::shadow(),
            "horsemanship" => Self::horsemanship(),
            "flanking" => Self::flanking(),
            "umbra armor" => Self::umbra_armor(),
            "phasing" => Self::phasing(),
            "improvise" => Self::improvise(),
            "convoke" => Self::convoke(),
            "affinity for artifacts" => Self::affinity_for_artifacts(),
            "delve" => Self::delve(),
            "changeling" => Self::changeling(),
            "this creature can't be blocked" | "can't be blocked" => Self::unblockable(),
            "plainswalk" => Self::landwalk(Subtype::Plains),
            "islandwalk" => Self::landwalk(Subtype::Island),
            "swampwalk" => Self::landwalk(Subtype::Swamp),
            "mountainwalk" => Self::landwalk(Subtype::Mountain),
            "forestwalk" => Self::landwalk(Subtype::Forest),
            "snow plainswalk" => Self::snow_landwalk(Subtype::Plains),
            "snow islandwalk" => Self::snow_landwalk(Subtype::Island),
            "snow swampwalk" => Self::snow_landwalk(Subtype::Swamp),
            "snow mountainwalk" => Self::snow_landwalk(Subtype::Mountain),
            "snow forestwalk" => Self::snow_landwalk(Subtype::Forest),
            "landwalk" => Self::any_landwalk(),
            "nonbasic landwalk" => Self::nonbasic_landwalk(),
            "artifact landwalk" => Self::artifact_landwalk(),
            "protection from white" => Self::protection(ProtectionFrom::Color(ColorSet::WHITE)),
            "protection from blue" => Self::protection(ProtectionFrom::Color(ColorSet::BLUE)),
            "protection from black" => Self::protection(ProtectionFrom::Color(ColorSet::BLACK)),
            "protection from red" => Self::protection(ProtectionFrom::Color(ColorSet::RED)),
            "protection from green" => Self::protection(ProtectionFrom::Color(ColorSet::GREEN)),
            "protection from all colors" => Self::protection(ProtectionFrom::AllColors),
            "protection from colorless" => Self::protection(ProtectionFrom::Colorless),
            "protection from everything" => Self::protection(ProtectionFrom::Everything),
            "protection from human" | "protection from humans" => Self::protection(
                ProtectionFrom::Permanents(ObjectFilter::creature().with_subtype(Subtype::Human)),
            ),
            _ => return None,
        })
    }

    pub fn restriction(restriction: Restriction, detail: impl std::fmt::Debug) -> Self {
        let display = format!("{detail:?}").trim_matches('"').to_string();
        Self {
            id: Some(StaticAbilityId::RuleRestriction),
            label: display.clone(),
            payload: StaticAbilityPayload::RuleRestriction {
                restriction,
                display,
            },
        }
    }

    pub fn protection(filter: ProtectionFrom) -> Self {
        Self {
            id: Some(StaticAbilityId::Protection),
            label: "protection".to_string(),
            payload: StaticAbilityPayload::Protection(filter),
        }
    }

    pub fn must_attack() -> Self {
        Self::identified(StaticAbilityId::MustAttack, "must attack")
    }

    pub fn attached_goaded_by_source_controller(display: impl Into<String>) -> Self {
        Self::identified(StaticAbilityId::AttachedGoadedBySourceController, display)
    }

    pub fn all_creatures_attack_attached_controller_each_combat_if_able() -> Self {
        Self::identified(
            StaticAbilityId::AllCreaturesAttackAttachedControllerEachCombatIfAble,
            "All creatures attack enchanted creature's controller each combat if able",
        )
    }

    pub fn must_block() -> Self {
        Self::identified(StaticAbilityId::MustBlock, "must block")
    }

    pub fn unblockable() -> Self {
        Self::identified(StaticAbilityId::Unblockable, "unblockable")
    }

    pub fn make_colorless(filter: ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::MakeColorless),
            label: "make colorless".to_string(),
            payload: StaticAbilityPayload::MakeColorless(filter),
        }
    }

    pub fn cant_block() -> Self {
        Self {
            id: Some(StaticAbilityId::CantBlock),
            label: "cant block".to_string(),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn grants(spec: GrantSpecModel<T, E, C, Cond>) -> Self {
        Self {
            id: Some(StaticAbilityId::Grants),
            label: "grants".to_string(),
            payload: StaticAbilityPayload::Grants(Box::new(spec)),
        }
    }

    pub fn set_base_power_toughness(filter: ObjectFilter, power: i32, toughness: i32) -> Self {
        Self {
            id: Some(StaticAbilityId::SetBasePowerToughnessForFilter),
            label: "set base power toughness".to_string(),
            payload: StaticAbilityPayload::SetBasePowerToughness {
                filter,
                power,
                toughness,
            },
        }
    }

    pub fn can_attack_as_though_no_defender() -> Self {
        Self::identified(
            StaticAbilityId::CanAttackAsThoughNoDefender,
            "can attack as though no defender",
        )
    }

    pub fn set_colors(filter: ObjectFilter, colors: ColorSet) -> Self {
        Self {
            id: Some(StaticAbilityId::SetColors),
            label: "set colors".to_string(),
            payload: StaticAbilityPayload::SetColors { filter, colors },
        }
    }

    pub fn add_card_types(filter: ObjectFilter, card_types: Vec<CardType>) -> Self {
        Self {
            id: Some(StaticAbilityId::AddCardTypes),
            label: "add card types".to_string(),
            payload: StaticAbilityPayload::AddCardTypes { filter, card_types },
        }
    }

    pub fn remove_card_types(
        filter: ObjectFilter,
        card_types: Vec<CardType>,
        condition: Option<Condition>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::RemoveCardTypes),
            label: "remove card types".to_string(),
            payload: StaticAbilityPayload::RemoveCardTypes {
                filter,
                card_types,
                condition,
            },
        }
    }

    pub fn add_subtypes(filter: ObjectFilter, subtypes: Vec<Subtype>) -> Self {
        Self {
            id: Some(StaticAbilityId::AddSubtypes),
            label: "add subtypes".to_string(),
            payload: StaticAbilityPayload::AddSubtypes { filter, subtypes },
        }
    }

    pub fn changeling() -> Self {
        Self::identified(StaticAbilityId::Changeling, "changeling")
    }

    pub fn ward(amount: impl Into<TotalCost<C>>) -> Self {
        Self {
            id: Some(StaticAbilityId::Ward),
            label: "ward".to_string(),
            payload: StaticAbilityPayload::Ward(amount.into()),
        }
    }

    pub fn can_block_additional_creature_each_combat(additional: usize) -> Self {
        Self {
            id: Some(StaticAbilityId::CanBlockAdditionalCreatureEachCombat),
            label: "can block additional creature".to_string(),
            payload: StaticAbilityPayload::CanBlockAdditionalCreatureEachCombat(additional),
        }
    }

    pub fn can_block_subtype_as_though_reach(subtype: Subtype) -> Self {
        let subtype_text = subtype.to_string();
        let plural = if subtype_text.ends_with('s') {
            subtype_text
        } else {
            format!("{subtype_text}s")
        };
        Self {
            id: Some(StaticAbilityId::CanBlockFlying),
            label: format!("This creature can block {plural} as though it had reach"),
            payload: StaticAbilityPayload::CanBlockAsThoughReachForSubtype(subtype),
        }
    }

    pub fn can_block_as_though_reach_subtype(&self) -> Option<Subtype> {
        match self.payload {
            StaticAbilityPayload::CanBlockAsThoughReachForSubtype(subtype) => Some(subtype),
            _ => None,
        }
    }

    pub fn doesnt_untap() -> Self {
        Self {
            id: Some(StaticAbilityId::DoesntUntap),
            label: "doesnt untap".to_string(),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn cant_attack() -> Self {
        Self::identified(StaticAbilityId::CantAttack, "cant attack")
    }

    pub fn enters_tapped_unless_condition(
        condition: Condition,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::EntersTappedUnlessCondition),
            label: display.clone(),
            payload: StaticAbilityPayload::EntersTappedUnlessCondition { condition, display },
        }
    }

    pub fn haste() -> Self {
        Self::identified(StaticAbilityId::Haste, "haste")
    }

    pub fn flying() -> Self {
        Self::identified(StaticAbilityId::Flying, "flying")
    }

    pub fn trample() -> Self {
        Self::identified(StaticAbilityId::Trample, "trample")
    }

    pub fn menace() -> Self {
        Self::identified(StaticAbilityId::Menace, "menace")
    }

    pub fn banding() -> Self {
        Self::identified(StaticAbilityId::Banding, "banding")
    }

    pub fn hexproof() -> Self {
        Self::identified(StaticAbilityId::Hexproof, "hexproof")
    }

    pub fn improvise() -> Self {
        Self::identified(StaticAbilityId::Improvise, "improvise")
    }

    pub fn convoke() -> Self {
        Self::identified(StaticAbilityId::Convoke, "convoke")
    }

    pub fn affinity_for_artifacts() -> Self {
        Self::identified(
            StaticAbilityId::AffinityForArtifacts,
            "affinity for artifacts",
        )
    }

    pub fn delve() -> Self {
        Self::identified(StaticAbilityId::Delve, "delve")
    }

    pub fn first_strike() -> Self {
        Self::identified(StaticAbilityId::FirstStrike, "first strike")
    }

    pub fn double_strike() -> Self {
        Self::identified(StaticAbilityId::DoubleStrike, "double strike")
    }

    pub fn deathtouch() -> Self {
        Self::identified(StaticAbilityId::Deathtouch, "deathtouch")
    }

    pub fn lifelink() -> Self {
        Self::identified(StaticAbilityId::Lifelink, "lifelink")
    }

    pub fn vigilance() -> Self {
        Self::identified(StaticAbilityId::Vigilance, "vigilance")
    }

    pub fn reach() -> Self {
        Self::identified(StaticAbilityId::Reach, "reach")
    }

    pub fn defender() -> Self {
        Self::identified(StaticAbilityId::Defender, "defender")
    }

    pub fn phasing() -> Self {
        Self::identified(StaticAbilityId::Phasing, "phasing")
    }

    pub fn indestructible() -> Self {
        Self::identified(StaticAbilityId::Indestructible, "indestructible")
    }

    pub fn shroud() -> Self {
        Self::identified(StaticAbilityId::Shroud, "shroud")
    }

    pub fn wither() -> Self {
        Self::identified(StaticAbilityId::Wither, "wither")
    }

    pub fn infect() -> Self {
        Self::identified(StaticAbilityId::Infect, "infect")
    }

    pub fn cascade() -> Self {
        Self::identified(StaticAbilityId::Cascade, "cascade")
    }

    pub fn read_ahead() -> Self {
        Self::identified(StaticAbilityId::ReadAhead, "read ahead")
    }

    pub fn skulk() -> Self {
        Self::identified(StaticAbilityId::Skulk, "skulk")
    }

    pub fn prowess() -> Self {
        Self::identified(StaticAbilityId::Prowess, "prowess")
    }

    pub fn granted_inline_ability(&self) -> Option<&AbilityModel<T, E, C, Cond>> {
        None
    }

    pub fn toxic(_amount: u32) -> Self {
        Self::new("toxic")
    }

    pub fn unleash() -> Self {
        Self::identified(StaticAbilityId::Unleash, "unleash")
    }

    pub fn any_landwalk() -> Self {
        Self::new(LandwalkKind::AnyLand)
    }

    pub fn nonbasic_landwalk() -> Self {
        Self::new(LandwalkKind::NonbasicLand)
    }

    pub fn artifact_landwalk() -> Self {
        Self::new(LandwalkKind::ArtifactLand)
    }

    pub fn landwalk(kind: Subtype) -> Self {
        Self::new(LandwalkKind::Subtype {
            subtype: kind,
            snow: false,
        })
    }

    pub fn snow_landwalk(subtype: Subtype) -> Self {
        Self::new(LandwalkKind::Subtype {
            subtype,
            snow: true,
        })
    }

    pub fn hexproof_from(filter: ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::HexproofFrom),
            label: "hexproof from".into(),
            payload: StaticAbilityPayload::HexproofFrom(filter),
        }
    }

    pub fn id(&self) -> StaticAbilityId {
        self.id.unwrap_or(StaticAbilityId::RuleFallbackText)
    }

    pub fn display(&self) -> String {
        self.label.clone()
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.label = text.into();
        self
    }

    pub fn with_condition(self, condition: Condition) -> Self {
        match self.payload {
            StaticAbilityPayload::CostIncrease(mut increase) => {
                increase.condition = Some(match increase.condition {
                    Some(existing) => Condition::And(Box::new(existing), Box::new(condition)),
                    None => condition,
                });
                StaticAbility {
                    id: self.id,
                    label: self.label,
                    payload: StaticAbilityPayload::CostIncrease(increase),
                }
            }
            StaticAbilityPayload::ActivatedAbilityCostIncrease {
                filter,
                increase,
                activator,
                non_mana_only,
                condition: existing,
            } => StaticAbility {
                id: self.id,
                label: self.label,
                payload: StaticAbilityPayload::ActivatedAbilityCostIncrease {
                    filter,
                    increase,
                    activator,
                    non_mana_only,
                    condition: Some(match existing {
                        Some(existing) => Condition::And(Box::new(existing), Box::new(condition)),
                        None => condition,
                    }),
                },
            },
            StaticAbilityPayload::Conditional {
                ability,
                condition: existing,
            } => {
                let combined = Condition::And(Box::new(existing), Box::new(condition));
                StaticAbility {
                    id: ability.id,
                    label: ability.label.clone(),
                    payload: StaticAbilityPayload::Conditional {
                        ability,
                        condition: combined,
                    },
                }
            }
            payload => {
                let ability = StaticAbility {
                    id: self.id,
                    label: self.label,
                    payload,
                };
                StaticAbility {
                    id: ability.id,
                    label: ability.label.clone(),
                    payload: StaticAbilityPayload::Conditional {
                        ability: Box::new(ability),
                        condition,
                    },
                }
            }
        }
    }
    pub fn unwrap_or(self, _fallback: Self) -> Self {
        self
    }
    pub fn unwrap_or_else<F: FnOnce() -> Self>(self, _fallback: F) -> Self {
        self
    }

    pub fn unsupported_parser_line(text: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::new(format!("{} ({})", text.into(), reason.into()))
    }

    pub fn characteristic_defining_pt(
        power: impl Into<Value>,
        toughness: impl Into<Value>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::CharacteristicDefiningPT),
            label: "characteristic defining pt".to_string(),
            payload: StaticAbilityPayload::CharacteristicDefiningPt {
                power: power.into(),
                toughness: toughness.into(),
            },
        }
    }

    pub fn cant_be_blocked_by_more_than(count: usize) -> Self {
        Self {
            id: Some(StaticAbilityId::CantBeBlockedByMoreThan),
            label: format!("can't be blocked by more than {count} creature"),
            payload: StaticAbilityPayload::CantBeBlockedByMoreThan(count),
        }
    }

    pub fn cant_be_blocked_except_by_n_or_more(count: usize) -> Self {
        Self {
            id: Some(StaticAbilityId::CantBeBlockedExceptByNOrMore),
            label: format!("can't be blocked except by {count} or more creatures"),
            payload: StaticAbilityPayload::CantBeBlockedExceptByNOrMore(count),
        }
    }
    pub fn enters_tapped_ability() -> Self {
        Self::identified(StaticAbilityId::EntersTapped, "enters tapped")
    }
    pub fn remove_all_abilities(filter: ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::RemoveAllAbilitiesForFilter),
            label: "remove all abilities".to_string(),
            payload: StaticAbilityPayload::RemoveAllAbilities(filter),
        }
    }
    pub fn can_block_only_flying() -> Self {
        Self::identified(StaticAbilityId::CanBlockOnlyFlying, "can block only flying")
    }
    pub fn partner() -> Self {
        Self::identified(StaticAbilityId::Partner, "partner")
    }
    pub fn partner_with(partner_name: impl AsRef<str>) -> Self {
        Self::identified(
            StaticAbilityId::PartnerWith,
            format!("partner with {}", partner_name.as_ref().trim()),
        )
    }
    pub fn start_your_engines() -> Self {
        Self::identified(StaticAbilityId::StartYourEngines, "start your engines")
    }
    pub fn assist() -> Self {
        Self::identified(StaticAbilityId::Assist, "assist")
    }
    pub fn split_second() -> Self {
        Self::identified(StaticAbilityId::SplitSecond, "split second")
    }
    pub fn rebound() -> Self {
        Self::identified(StaticAbilityId::Rebound, "rebound")
    }
    pub fn fear() -> Self {
        Self::identified(StaticAbilityId::Fear, "fear")
    }
    pub fn intimidate() -> Self {
        Self::identified(StaticAbilityId::Intimidate, "intimidate")
    }
    pub fn shadow() -> Self {
        Self::identified(StaticAbilityId::Shadow, "shadow")
    }
    pub fn horsemanship() -> Self {
        Self::identified(StaticAbilityId::Horsemanship, "horsemanship")
    }
    pub fn flanking() -> Self {
        Self::identified(StaticAbilityId::Flanking, "flanking")
    }
    pub fn umbra_armor() -> Self {
        Self::identified(StaticAbilityId::UmbraArmor, "umbra armor")
    }
    pub fn bloodthirst(amount: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::Bloodthirst),
            label: format!("bloodthirst {amount}"),
            payload: StaticAbilityPayload::Bloodthirst(amount),
        }
    }
    pub fn krrik_black_mana_may_be_paid_with_life() -> Self {
        Self {
            id: Some(StaticAbilityId::BlackManaMayBePaidWithLife),
            label: "krrik".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn minimum_spell_total_mana(amount: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::MinimumSpellTotalMana),
            label: "minimum spell total mana".into(),
            payload: StaticAbilityPayload::MinimumSpellTotalMana(amount),
        }
    }
    pub fn cant_pay_life_or_sacrifice_nonland_for_cast_or_activate() -> Self {
        Self {
            id: Some(StaticAbilityId::CantPayLifeOrSacrificeNonlandForCastOrActivate),
            label: "cant pay life or sac nonland".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn cant_attack_unless_condition(
        condition: CantAttackUnlessConditionSpec,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::CantAttackUnlessCondition),
            label: display.clone(),
            payload: StaticAbilityPayload::CantAttackUnlessCondition { condition, display },
        }
    }
    pub fn cant_attack_its_owner() -> Self {
        Self {
            id: Some(StaticAbilityId::CantAttackItsOwner),
            label: "cant attack its owner".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn reduce_activated_ability_costs(
        filter: ObjectFilter,
        reduction: u32,
        minimum_total_mana: Option<u32>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostReduction),
            label: "reduce activated ability costs".into(),
            payload: StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction,
                replacement_mana_cost: None,
                display: None,
                condition: None,
                per_matching_objects: None,
                per_basic_land_types_among: None,
                minimum_total_mana,
            },
        }
    }
    pub fn reduce_activated_ability_costs_with_display(
        filter: ObjectFilter,
        reduction: u32,
        minimum_total_mana: Option<u32>,
        display: impl Into<String>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostReduction),
            label: "reduce activated ability costs".into(),
            payload: StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction,
                replacement_mana_cost: None,
                display: Some(display.into()),
                condition: None,
                per_matching_objects: None,
                per_basic_land_types_among: None,
                minimum_total_mana,
            },
        }
    }
    pub fn reduce_activated_ability_costs_if_targets(
        filter: ObjectFilter,
        reduction: u32,
        condition: ActivatedAbilityCostCondition,
        minimum_total_mana: Option<u32>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostReduction),
            label: "reduce activated ability costs if targets".into(),
            payload: StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction,
                replacement_mana_cost: None,
                display: None,
                condition: Some(condition),
                per_matching_objects: None,
                per_basic_land_types_among: None,
                minimum_total_mana,
            },
        }
    }
    pub fn reduce_activated_ability_costs_for_each(
        filter: ObjectFilter,
        reduction: u32,
        per_filter: ObjectFilter,
        minimum_total_mana: Option<u32>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostReduction),
            label: "reduce activated ability costs for each".into(),
            payload: StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction,
                replacement_mana_cost: None,
                display: None,
                condition: None,
                per_matching_objects: Some(per_filter),
                per_basic_land_types_among: None,
                minimum_total_mana,
            },
        }
    }
    pub fn reduce_activated_ability_costs_for_each_basic_land_type(
        filter: ObjectFilter,
        reduction: u32,
        lands_filter: ObjectFilter,
        minimum_total_mana: Option<u32>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostReduction),
            label: "reduce activated ability costs for each basic land type".into(),
            payload: StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction,
                replacement_mana_cost: None,
                display: None,
                condition: None,
                per_matching_objects: None,
                per_basic_land_types_among: Some(lands_filter),
                minimum_total_mana,
            },
        }
    }
    pub fn replace_activated_ability_mana_cost(
        filter: ObjectFilter,
        replacement_mana_cost: ManaCost,
        display: impl Into<String>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostReduction),
            label: "replace activated ability mana cost".into(),
            payload: StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction: 0,
                replacement_mana_cost: Some(replacement_mana_cost),
                display: Some(display.into()),
                condition: None,
                per_matching_objects: None,
                per_basic_land_types_among: None,
                minimum_total_mana: None,
            },
        }
    }
    pub fn remove_all_abilities_except_mana(filter: ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::RemoveAllAbilitiesExceptManaForFilter),
            label: "remove all abilities except mana".to_string(),
            payload: StaticAbilityPayload::RemoveAllAbilitiesExceptMana(filter),
        }
    }
    pub fn set_card_types(filter: ObjectFilter, card_types: Vec<CardType>) -> Self {
        Self {
            id: Some(StaticAbilityId::SetCardTypes),
            label: "set card types".to_string(),
            payload: StaticAbilityPayload::SetCardTypes { filter, card_types },
        }
    }
    pub fn set_land_subtypes(filter: ObjectFilter, subtypes: Vec<Subtype>) -> Self {
        Self {
            id: Some(StaticAbilityId::SetLandSubtypes),
            label: "set land subtypes".to_string(),
            payload: StaticAbilityPayload::SetLandSubtypes { filter, subtypes },
        }
    }
    pub fn set_creature_subtypes(filter: ObjectFilter, subtypes: Vec<Subtype>) -> Self {
        Self {
            id: Some(StaticAbilityId::SetCreatureSubtypes),
            label: "set creature subtypes".to_string(),
            payload: StaticAbilityPayload::SetCreatureSubtypes { filter, subtypes },
        }
    }
    pub fn add_all_subtypes_of_family(filter: ObjectFilter, family: SubtypeFamily) -> Self {
        Self {
            id: Some(StaticAbilityId::AddAllSubtypesOfFamily),
            label: "add all subtypes of family".to_string(),
            payload: StaticAbilityPayload::AddAllSubtypesOfFamily { filter, family },
        }
    }
    pub fn prevent_constrained_damage_to_self_put_counters_instead(
        counter_type: CounterType,
        display: impl Into<String>,
        source_filter: Option<ObjectFilter>,
        combat_only: Option<bool>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::PreventConstrainedDamageToSelfPutCountersInstead),
            label: display.clone(),
            payload: StaticAbilityPayload::PreventConstrainedDamageToSelfPutCountersInstead {
                counter_type,
                display,
                source_filter,
                combat_only,
            },
        }
    }
    pub fn prevent_damage_to_you_from_source_filter(
        amount: u32,
        source_filter: ObjectFilter,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::PreventDamageToYouFromSourceFilter),
            label: display.clone(),
            payload: StaticAbilityPayload::PreventDamageToYouFromSourceFilter {
                amount,
                source_filter,
                display,
            },
        }
    }
    pub fn untap_during_each_other_players_untap_step(
        filter: ObjectFilter,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::UntapDuringEachOtherPlayersUntapStep),
            label: display.clone(),
            payload: StaticAbilityPayload::UntapDuringEachOtherPlayersUntapStep { filter, display },
        }
    }
    pub fn pregame_action(kind: PregameActionKind, display: impl Into<String>) -> Self {
        let text = display.into();
        Self {
            id: Some(StaticAbilityId::PregameAction),
            label: text.clone(),
            payload: StaticAbilityPayload::PregameAction { kind, text },
        }
    }
    pub fn reduce_maximum_hand_size(player: PlayerFilter, by: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::ReduceMaximumHandSize),
            label: "reduce maximum hand size".into(),
            payload: StaticAbilityPayload::ReduceMaximumHandSize { player, by },
        }
    }
    pub fn set_maximum_hand_size(player: PlayerFilter, amount: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::SetMaximumHandSize),
            label: "set maximum hand size".into(),
            payload: StaticAbilityPayload::SetMaximumHandSize { player, amount },
        }
    }
    pub fn equipment_grant(abilities: Vec<StaticAbility<T, E, C, Cond>>) -> Self {
        Self {
            id: Some(StaticAbilityId::EquipmentGrant),
            label: "equipment grant".to_string(),
            payload: StaticAbilityPayload::EquipmentGrant(abilities),
        }
    }
    pub fn soulbond_shared_object_ability(ability: AbilityModel<T, E, C, Cond>) -> Self {
        let text = match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.label.clone(),
            AbilityKind::Triggered(_) | AbilityKind::Activated(_) => "an ability".to_string(),
        };
        Self {
            id: Some(StaticAbilityId::SoulbondSharedBonus),
            label: format!(
                "As long as this creature is paired with another creature, both creatures have \"{text}\""
            ),
            payload: StaticAbilityPayload::SoulbondSharedObjectAbility(Box::new(ability)),
        }
    }
    pub fn grant_object_ability_for_filter(
        filter: ObjectFilter,
        ability: AbilityModel<T, E, C, Cond>,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self::new(GrantObjectAbilityForFilter::new(filter, ability, display))
    }
    pub fn boast_twice_each_turn() -> Self {
        Self::identified(StaticAbilityId::BoastTwiceEachTurn, "boast twice each turn")
    }
    pub fn first_equip_cost_alternative(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::FirstEquipCostAlternative),
            label: display.clone(),
            payload: StaticAbilityPayload::FirstEquipCostAlternative(display),
        }
    }
    pub fn equip_abilities_any_time() -> Self {
        Self::identified(
            StaticAbilityId::EquipAbilitiesAnyTime,
            "activate equip abilities any time",
        )
    }
    pub fn exhaust_abilities_as_though_unactivated_this_turn() -> Self {
        Self::identified(
            StaticAbilityId::ExhaustAbilitiesAsThoughUnactivatedThisTurn,
            "activate exhaust abilities as though unactivated this turn",
        )
    }
    pub fn vote_additional_time_while_voting() -> Self {
        Self::identified(
            StaticAbilityId::VoteAdditionalTimeWhileVoting,
            "vote additional time while voting",
        )
    }
    pub fn vote_additional_vote_while_voting() -> Self {
        Self::identified(
            StaticAbilityId::VoteAdditionalVoteWhileVoting,
            "vote additional vote while voting",
        )
    }
    pub fn exert_attack(
        only_if_not_exerted_this_turn: bool,
        linked_trigger: Option<TriggeredAbility<T, E>>,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ExertAttack),
            label: display.clone(),
            payload: StaticAbilityPayload::ExertAttack {
                only_if_not_exerted_this_turn,
                linked_trigger,
                display,
            },
        }
    }
    pub fn cant_attack_you_unless_controller_pays_per_attacker(cost: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttacker),
            label: "cant attack you unless pays per attacker".to_string(),
            payload: StaticAbilityPayload::CantAttackYouUnlessControllerPaysPerAttacker(cost),
        }
    }
    pub fn cant_attack_you_unless_controller_pays_per_attacker_basic_land_types_among_lands_you_control()
    -> Self {
        Self {
            id: Some(
                StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl,
            ),
            label: "cant attack you unless pays per attacker basic land types".to_string(),
            payload:
                StaticAbilityPayload::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl,
        }
    }
    pub fn cant_be_blocked_by_power_or_less(power: i32) -> Self {
        Self {
            id: Some(StaticAbilityId::CantBeBlockedByPowerOrLess),
            label: format!("can't be blocked by creatures with power {power} or less"),
            payload: StaticAbilityPayload::CantBeBlockedByPowerOrLess(power),
        }
    }
    pub fn cant_be_blocked_by_power_or_greater(power: i32) -> Self {
        Self {
            id: Some(StaticAbilityId::CantBeBlockedByPowerOrGreater),
            label: format!("can't be blocked by creatures with power {power} or greater"),
            payload: StaticAbilityPayload::CantBeBlockedByPowerOrGreater(power),
        }
    }
    pub fn cant_attack_unless_controller_cast_creature_spell_this_turn() -> Self {
        Self::identified(
            StaticAbilityId::CantAttackUnlessControllerCastCreatureSpellThisTurn,
            "cant attack unless controller cast creature spell",
        )
    }
    pub fn cant_attack_unless_controller_cast_noncreature_spell_this_turn() -> Self {
        Self::identified(
            StaticAbilityId::CantAttackUnlessControllerCastNonCreatureSpellThisTurn,
            "cant attack unless controller cast noncreature spell",
        )
    }
    pub fn players_cant_gain_life() -> Self {
        Self::identified(
            StaticAbilityId::PlayersCantGainLife,
            "players cant gain life",
        )
    }
    pub fn players_cant_search() -> Self {
        Self::identified(StaticAbilityId::PlayersCantSearch, "players cant search")
    }
    pub fn damage_cant_be_prevented() -> Self {
        Self::identified(
            StaticAbilityId::DamageCantBePrevented,
            "damage cant be prevented",
        )
    }
    pub fn you_cant_lose_game() -> Self {
        Self::identified(StaticAbilityId::YouCantLoseGame, "you cant lose game")
    }
    pub fn opponents_cant_win_game() -> Self {
        Self::identified(
            StaticAbilityId::OpponentsCantWinGame,
            "opponents cant win game",
        )
    }
    pub fn your_life_total_cant_change() -> Self {
        Self::identified(
            StaticAbilityId::YourLifeTotalCantChange,
            "your life total cant change",
        )
    }
    pub fn opponents_cant_cast_spells() -> Self {
        Self::identified(
            StaticAbilityId::OpponentsCantCastSpells,
            "opponents cant cast spells",
        )
    }
    pub fn opponents_cant_draw_extra_cards() -> Self {
        Self::identified(
            StaticAbilityId::OpponentsCantDrawExtraCards,
            "opponents cant draw extra cards",
        )
    }
    pub fn cant_have_counters_placed() -> Self {
        Self::identified(
            StaticAbilityId::CantHaveCountersPlaced,
            "cant have counters placed",
        )
    }
    pub fn cant_be_countered_ability() -> Self {
        Self::identified(
            StaticAbilityId::CantBeCountered,
            "cant be countered ability",
        )
    }
    pub fn permanents_you_control_cant_be_sacrificed() -> Self {
        Self::identified(
            StaticAbilityId::PermanentsCantBeSacrificed,
            "permanents you control cant be sacrificed",
        )
    }
    pub fn cant_be_blocked_as_long_as_defending_player_controls_card_type(
        card_type: CardType,
    ) -> Self {
        Self::cant_be_blocked_as_long_as_defending_player_controls_card_types(vec![card_type])
    }
    pub fn cant_be_blocked_as_long_as_defending_player_controls_card_types(
        card_types: Vec<CardType>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::RuleRestriction),
            label: "cant be blocked as long as defending controls card types".into(),
            payload: StaticAbilityPayload::CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes(
                card_types,
            ),
        }
    }
    pub fn set_name(filter: ObjectFilter, name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: Some(StaticAbilityId::SetName),
            label: format!("set name {name}"),
            payload: StaticAbilityPayload::SetName { filter, name },
        }
    }
    pub fn soulbond_shared_power_toughness(power: i32, toughness: i32) -> Self {
        let signed = |value: i32| {
            if value >= 0 {
                format!("+{value}")
            } else {
                value.to_string()
            }
        };
        Self {
            id: Some(StaticAbilityId::SoulbondSharedBonus),
            label: format!(
                "As long as this creature is paired with another creature, each of those creatures gets {}/{}",
                signed(power),
                signed(toughness)
            ),
            payload: StaticAbilityPayload::SoulbondSharedPowerToughness { power, toughness },
        }
    }
    pub fn soulbond_shared_ability(ability: StaticAbility<T, E, C, Cond>) -> Self {
        let label = ability.display();
        Self {
            id: Some(StaticAbilityId::SoulbondSharedBonus),
            label: format!(
                "As long as this creature is paired with another creature, both creatures have {label}"
            ),
            payload: StaticAbilityPayload::SoulbondSharedAbility(Box::new(ability)),
        }
    }
    pub fn add_colors(filter: ObjectFilter, colors: ColorSet) -> Self {
        Self {
            id: Some(StaticAbilityId::AddColors),
            label: "add colors".to_string(),
            payload: StaticAbilityPayload::AddColors { filter, colors },
        }
    }
    pub fn control_attached_permanent(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ControlAttachedPermanent),
            label: display.clone(),
            payload: StaticAbilityPayload::ControlAttachedPermanent(display),
        }
    }
    pub fn prevent_damage_to_self_remove_counter(counter_type: CounterType, amount: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::PreventDamageToSelfRemoveCounter),
            label: "prevent damage to self remove counter".to_string(),
            payload: StaticAbilityPayload::PreventDamageToSelfRemoveCounter {
                counter_type,
                amount,
            },
        }
    }
    pub fn prevent_damage_to_self_put_counters_instead(
        counter_type: CounterType,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::PreventDamageToSelfPutCountersInstead),
            label: display.clone(),
            payload: StaticAbilityPayload::PreventDamageToSelfPutCountersInstead {
                counter_type,
                display,
            },
        }
    }
    pub fn replace_damage_with_counters_instead(
        counter_type: CounterType,
        source_filter: ObjectFilter,
        target_filter: ObjectFilter,
        combat_only: Option<bool>,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ReplaceDamageWithCountersInstead),
            label: display.clone(),
            payload: StaticAbilityPayload::ReplaceDamageWithCountersInstead {
                counter_type,
                display,
                source_filter,
                target_filter,
                combat_only,
            },
        }
    }
    pub fn add_supertypes(filter: ObjectFilter, supertypes: Vec<Supertype>) -> Self {
        Self {
            id: Some(StaticAbilityId::AddSupertypes),
            label: "add supertypes".to_string(),
            payload: StaticAbilityPayload::AddSupertypes { filter, supertypes },
        }
    }
    pub fn reveal_first_card_you_draw_each_turn(optional: bool, your_turns_only: bool) -> Self {
        Self {
            id: Some(StaticAbilityId::RevealFirstCardYouDrawEachTurn),
            label: "reveal first card".into(),
            payload: StaticAbilityPayload::RevealFirstCardYouDrawEachTurn {
                optional,
                your_turns_only,
            },
        }
    }
    pub fn increase_activated_ability_costs(filter: ObjectFilter, increase: TotalCost<C>) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostIncrease),
            label: "increase activated ability costs".to_string(),
            payload: StaticAbilityPayload::ActivatedAbilityCostIncrease {
                filter,
                increase,
                activator: None,
                non_mana_only: false,
                condition: None,
            },
        }
    }
    pub fn increase_activated_ability_costs_for_activator(
        activator: PlayerFilter,
        increase: TotalCost<C>,
        non_mana_only: bool,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostIncrease),
            label: "increase activated ability costs".to_string(),
            payload: StaticAbilityPayload::ActivatedAbilityCostIncrease {
                filter: ObjectFilter::default(),
                increase,
                activator: Some(activator),
                non_mana_only,
                condition: None,
            },
        }
    }
    pub fn cant_be_blocked_by_lower_power_than_source() -> Self {
        Self {
            id: Some(StaticAbilityId::CantBeBlockedByLowerPowerThanSource),
            label: "cant be blocked by lower power than source".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn doctors_companion() -> Self {
        Self {
            id: Some(StaticAbilityId::DoctorsCompanion),
            label: "doctors companion".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn conditional_spell_keyword(_spec: ConditionalSpellKeywordSpec) -> Self {
        Self {
            id: Some(StaticAbilityId::ConditionalSpellKeyword),
            label: "conditional spell keyword".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn damage_not_removed_during_cleanup() -> Self {
        Self::identified(
            StaticAbilityId::DamageNotRemovedDuringCleanup,
            "damage not removed during cleanup",
        )
    }
    pub fn choose_basic_land_type_as_enters(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ChooseBasicLandTypeAsEnters),
            label: display.clone(),
            payload: StaticAbilityPayload::ChooseBasicLandTypeAsEnters(display),
        }
    }
    pub fn choose_land_type_as_enters(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ChooseLandTypeAsEnters),
            label: display.clone(),
            payload: StaticAbilityPayload::ChooseLandTypeAsEnters(display),
        }
    }
    pub fn enchanted_land_is_chosen_type(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::EnchantedLandIsChosenType),
            label: display.clone(),
            payload: StaticAbilityPayload::EnchantedLandIsChosenType(display),
        }
    }
    pub fn add_chosen_creature_type(filter: ObjectFilter, display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::AddChosenCreatureType),
            label: display.clone(),
            payload: StaticAbilityPayload::AddChosenCreatureType { filter, display },
        }
    }
    pub fn add_chosen_color(filter: ObjectFilter, display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::AddChosenColor),
            label: display.clone(),
            payload: StaticAbilityPayload::AddChosenColor { filter, display },
        }
    }
    pub fn set_chosen_color(filter: ObjectFilter, display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::SetChosenColor),
            label: display.clone(),
            payload: StaticAbilityPayload::SetChosenColor { filter, display },
        }
    }
    pub fn choose_creature_type_as_enters(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ChooseCreatureTypeAsEnters),
            label: display.clone(),
            payload: StaticAbilityPayload::ChooseCreatureTypeAsEnters(display),
        }
    }
    pub fn choose_named_option_as_enters(options: Vec<String>, display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ChooseNamedOptionAsEnters),
            label: display.clone(),
            payload: StaticAbilityPayload::ChooseNamedOptionAsEnters { options, display },
        }
    }

    pub fn choose_power_toughness_as_enters_or_turns_face_up(
        options: Vec<(i32, i32)>,
        display: impl Into<String>,
    ) -> Self {
        let options = options
            .into_iter()
            .map(|(power, toughness)| PowerToughnessChoiceOption::new(power, toughness))
            .collect();
        Self::choose_power_toughness_options_as_enters_or_turns_face_up(options, display)
    }

    pub fn choose_power_toughness_options_as_enters_or_turns_face_up(
        options: Vec<PowerToughnessChoiceOption<T, E, C, Cond>>,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ChoosePowerToughnessAsEntersOrTurnsFaceUp),
            label: display.clone(),
            payload: StaticAbilityPayload::ChoosePowerToughnessAsEntersOrTurnsFaceUp {
                options,
                display,
            },
        }
    }

    pub fn duplicate_matching_triggered_abilities(
        source_filter: Option<ObjectFilter>,
        event_matcher: Option<T>,
        count: u32,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::DuplicateMatchingTriggeredAbilities),
            label: display.clone(),
            payload: StaticAbilityPayload::DuplicateMatchingTriggeredAbilities {
                source_filter,
                event_matcher,
                count,
                display,
            },
        }
    }
    pub fn suppress_matching_triggered_abilities(
        source_filter: Option<ObjectFilter>,
        event_matcher: Option<T>,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::SuppressMatchingTriggeredAbilities),
            label: display.clone(),
            payload: StaticAbilityPayload::SuppressMatchingTriggeredAbilities {
                source_filter,
                event_matcher,
                display,
            },
        }
    }
    pub fn double_damage_from_sources_you_control_of_chosen_type(
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::DoubleDamageFromSourcesYouControlOfChosenType),
            label: display.clone(),
            payload: StaticAbilityPayload::DoubleDamageFromSourcesYouControlOfChosenType(display),
        }
    }

    pub fn redirect_damage_to_source_controller(
        source_filter: ObjectFilter,
        target_player_filter: PlayerFilter,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::RedirectDamageToSourceController),
            label: display.clone(),
            payload: StaticAbilityPayload::RedirectDamageToSourceController {
                source_filter,
                target_player_filter,
                display,
            },
        }
    }
    pub fn with_enter_as_copy_as_enters(
        spec: EnterAsCopyAsEntersSpec<T, E, C, Cond>,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::EnterAsCopyAsEnters),
            label: display.clone(),
            payload: StaticAbilityPayload::EnterAsCopyAsEnters { spec, display },
        }
    }
    pub fn choose_color_as_enters(_excluded: Option<Color>, _display: impl Into<String>) -> Self {
        Self {
            id: Some(StaticAbilityId::ChooseColorAsEnters),
            label: "choose color as enters".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn choose_color_as_becomes_attached(_display: impl Into<String>) -> Self {
        Self {
            id: Some(StaticAbilityId::ChooseColorAsBecomesAttached),
            label: "choose color as becomes attached".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn choose_player_as_enters(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ChoosePlayerAsEnters),
            label: display.clone(),
            payload: StaticAbilityPayload::ChoosePlayerAsEnters(display),
        }
    }
    pub fn enters_under_opponent_control_as_enters(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::EntersUnderOpponentControlAsEnters),
            label: display.clone(),
            payload: StaticAbilityPayload::EntersUnderOpponentControlAsEnters(display),
        }
    }
    pub fn choose_card_name_as_enters(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ChooseCardNameAsEnters),
            label: display.clone(),
            payload: StaticAbilityPayload::ChooseCardNameAsEnters(display),
        }
    }
    pub fn redirect_damage_from_you_and_other_permanents_to_source() -> Self {
        Self::identified(
            StaticAbilityId::RedirectDamageToSource,
            "redirect damage from you and other permanents to source",
        )
    }
    pub fn max_attackers_each_combat(n: usize) -> Self {
        Self {
            id: Some(StaticAbilityId::MaxCreaturesCanAttackEachCombat),
            label: format!("no more than {n} creatures can attack each combat"),
            payload: StaticAbilityPayload::MaxCreaturesCanAttackEachCombat(n),
        }
    }
    pub fn max_attackers_can_attack_you_each_combat(n: usize) -> Self {
        Self {
            id: Some(StaticAbilityId::MaxCreaturesCanAttackYouEachCombat),
            label: format!("no more than {n} creatures can attack you each combat"),
            payload: StaticAbilityPayload::MaxCreaturesCanAttackYouEachCombat(n),
        }
    }
    pub fn max_blockers_each_combat(n: usize) -> Self {
        Self {
            id: Some(StaticAbilityId::MaxCreaturesCanBlockEachCombat),
            label: format!("no more than {n} creatures can block each combat"),
            payload: StaticAbilityPayload::MaxCreaturesCanBlockEachCombat(n),
        }
    }
    pub fn shuffle_into_library_from_graveyard() -> Self {
        Self {
            id: Some(StaticAbilityId::ShuffleIntoLibraryFromGraveyard),
            label: "shuffle into library from graveyard".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn permanents_enter_tapped() -> Self {
        Self {
            id: Some(StaticAbilityId::AllPermanentsEnterTapped),
            label: "permanents enter tapped".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn creatures_entering_dont_cause_abilities_to_trigger() -> Self {
        Self {
            id: Some(StaticAbilityId::CreaturesEnteringDontCauseAbilitiesToTrigger),
            label: "creatures entering dont cause abilities to trigger".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn creatures_assign_combat_damage_using_toughness() -> Self {
        Self::identified(
            StaticAbilityId::CreaturesAssignCombatDamageUsingToughness,
            "creatures assign combat damage using toughness",
        )
    }
    pub fn this_creature_assigns_combat_damage_using_toughness() -> Self {
        Self::identified(
            StaticAbilityId::ThisCreatureAssignsCombatDamageUsingToughness,
            "this creature assigns combat damage using toughness",
        )
    }
    pub fn creatures_you_control_assign_combat_damage_using_toughness() -> Self {
        Self {
            id: Some(StaticAbilityId::CreaturesYouControlAssignCombatDamageUsingToughness),
            label: "creatures you control assign damage using toughness".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn lethal_damage_to_creatures_you_control_uses_power() -> Self {
        Self::identified(
            StaticAbilityId::LethalDamageToCreaturesYouControlUsesPower,
            "lethal damage to creatures you control uses power",
        )
    }
    pub fn players_cant_cycle() -> Self {
        Self {
            id: Some(StaticAbilityId::PlayersCantCycle),
            label: "players cant cycle".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn starting_life_bonus(_amount: i32) -> Self {
        Self::identified(StaticAbilityId::StartingLifeBonus, "starting life bonus")
    }
    pub fn buyback_cost_reduction(_amount: impl Into<Value>) -> Self {
        Self::identified(
            StaticAbilityId::BuybackCostReduction,
            "buyback cost reduction",
        )
    }
    pub fn cost_increase_per_target_beyond_first(cost: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::CostIncreasePerAdditionalTarget),
            label: "cost increase per target beyond first".to_string(),
            payload: StaticAbilityPayload::CostIncreasePerTargetBeyondFirst(cost),
        }
    }
    pub fn cost_increase_mana_cost_per_target_beyond_first(cost: ManaCost) -> Self {
        Self {
            id: Some(StaticAbilityId::CostIncreaseManaCostPerAdditionalTarget),
            label: "mana cost increase per target beyond first".to_string(),
            payload: StaticAbilityPayload::CostIncreaseManaCostPerTargetBeyondFirst(cost),
        }
    }
    pub fn players_skip_upkeep() -> Self {
        Self {
            id: Some(StaticAbilityId::PlayersSkipUpkeep),
            label: "players skip upkeep".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn legend_rule_doesnt_apply() -> Self {
        Self::identified(
            StaticAbilityId::LegendRuleDoesntApply,
            "legend rule doesnt apply",
        )
    }
    pub fn remove_supertypes(filter: ObjectFilter, supertypes: Vec<Supertype>) -> Self {
        Self {
            id: Some(StaticAbilityId::RemoveSupertypes),
            label: "remove supertypes".into(),
            payload: StaticAbilityPayload::RemoveSupertypes { filter, supertypes },
        }
    }
    pub fn prevent_all_damage_dealt_to_creatures() -> Self {
        Self {
            id: Some(StaticAbilityId::PreventAllDamageDealtToCreatures),
            label: "prevent all damage dealt to creatures".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn prevent_damage_to_other_creature_you_control_put_counters_instead(
        _counter_type: CounterType,
        _display: impl Into<String>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::PreventDamageToOtherCreatureYouControlPutCountersInstead),
            label: "prevent damage to other creature you control put counters instead".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn prevent_all_combat_damage_to_self() -> Self {
        Self {
            id: Some(StaticAbilityId::PreventAllCombatDamageToSelf),
            label: "prevent all combat damage to self".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn prevent_all_combat_damage_to_permanents_matching(filter: ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::PreventAllCombatDamageToPermanentsMatching),
            label: "prevent all combat damage to permanents matching filter".into(),
            payload: StaticAbilityPayload::PreventAllCombatDamageToPermanentsMatching(filter),
        }
    }
    pub fn prevent_all_damage_to_self() -> Self {
        Self {
            id: Some(StaticAbilityId::PreventAllDamageToSelf),
            label: "prevent all damage to self".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn prevent_all_noncombat_damage_to_other_creatures_you_control() -> Self {
        Self {
            id: Some(StaticAbilityId::PreventAllNoncombatDamageToOtherCreaturesYouControl),
            label: "prevent all noncombat damage to other creatures you control".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn prevent_all_damage_to_self_by_creatures() -> Self {
        Self {
            id: Some(StaticAbilityId::PreventAllDamageToSelfByCreatures),
            label: "prevent all damage to self by creatures".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn may_choose_not_to_untap_during_untap_step(subject: impl Into<String>) -> Self {
        let subject = subject.into();
        Self {
            id: Some(StaticAbilityId::MayChooseNotToUntapDuringUntapStep),
            label: format!("You may choose not to untap {subject} during your untap step"),
            payload: StaticAbilityPayload::MayChooseNotToUntapDuringUntapStep(subject),
        }
    }
    pub fn flying_only_restriction() -> Self {
        Self {
            id: Some(StaticAbilityId::FlyingOnlyRestriction),
            label: "flying only restriction".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn flying_restriction() -> Self {
        Self {
            id: Some(StaticAbilityId::FlyingRestriction),
            label: "flying restriction".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn may_assign_damage_as_unblocked() -> Self {
        Self {
            id: Some(StaticAbilityId::MayAssignDamageAsUnblocked),
            label: "may assign damage as unblocked".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn remove_ability(filter: ObjectFilter, ability: StaticAbility<T, E, C, Cond>) -> Self {
        Self {
            id: Some(StaticAbilityId::RemoveAbilityForFilter),
            label: format!("remove {}", ability.display()),
            payload: StaticAbilityPayload::RemoveAbilityForFilter {
                filter,
                ability: Box::new(ability),
            },
        }
    }
    pub fn look_at_top_card_of_library() -> Self {
        Self {
            id: Some(StaticAbilityId::LookAtTopCardOfLibrary),
            label: "You may look at the top card of your library any time.".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn look_at_face_down_creatures_you_dont_control() -> Self {
        Self {
            id: Some(StaticAbilityId::LookAtFaceDownCreaturesYouDontControl),
            label: "You may look at face-down creatures you don't control any time.".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn all_players_look_at_top_cards_of_libraries() -> Self {
        Self {
            id: Some(StaticAbilityId::AllPlayersLookAtTopCardsOfLibraries),
            label: "Players play with the top card of their libraries revealed.".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn all_players_look_at_your_top_library_card() -> Self {
        Self {
            id: Some(StaticAbilityId::AllPlayersLookAtYourTopLibraryCard),
            label: "Play with the top card of your library revealed.".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn opponents_play_with_hands_revealed() -> Self {
        Self {
            id: Some(StaticAbilityId::OpponentsPlayWithHandsRevealed),
            label: "Your opponents play with their hands revealed.".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn control_opponents_while_searching_libraries() -> Self {
        Self {
            id: Some(StaticAbilityId::ControlOpponentsWhileSearchingLibraries),
            label: "You control your opponents while they're searching their libraries.".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn opponent_search_exile_found_cards() -> Self {
        Self {
            id: Some(StaticAbilityId::OpponentSearchExileFoundCards),
            label: "While an opponent is searching their library, they exile each card they find. You may play those cards for as long as they remain exiled, and you may spend mana as though it were mana of any color to cast them.".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn cast_this_card_from_library_while_searching() -> Self {
        Self {
            id: Some(StaticAbilityId::CastThisCardFromLibraryWhileSearching),
            label: "While you're searching your library, you may cast this card from your library."
                .into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn additional_land_plays(count: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::RuleRestriction),
            label: "additional land plays".to_string(),
            payload: StaticAbilityPayload::AdditionalLandPlays(count),
        }
    }
    pub fn no_maximum_hand_size() -> Self {
        Self {
            id: Some(StaticAbilityId::NoMaximumHandSize),
            label: "no maximum hand size".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn can_be_commander() -> Self {
        Self {
            id: Some(StaticAbilityId::CanBeCommander),
            label: "can be commander".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn max_hand_size_seven_minus_your_graveyard_card_types(
        player: PlayerFilter,
        min_card_types: u32,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::MaximumHandSizeSevenMinusYourGraveyardCardTypes),
            label: "max hand size seven minus your graveyard card types".into(),
            payload: StaticAbilityPayload::MaximumHandSizeSevenMinusYourGraveyardCardTypes {
                player,
                min_card_types,
            },
        }
    }
    pub fn effect_discard_to_library_replacement() -> Self {
        Self {
            id: Some(StaticAbilityId::EffectDiscardToLibraryReplacement),
            label: "effect discard to library replacement".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn opponent_effect_discard_this_to_battlefield_replacement() -> Self {
        Self {
            id: Some(StaticAbilityId::OpponentEffectDiscardThisToBattlefieldReplacement),
            label: "opponent effect discard this to battlefield replacement".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn draw_replacement_exile_top_face_down() -> Self {
        Self {
            id: Some(StaticAbilityId::DrawReplacementExileTopFaceDown),
            label: "draw replacement exile top face down".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn draw_replacement_double() -> Self {
        Self {
            id: Some(StaticAbilityId::DrawReplacementDouble),
            label: "draw replacement double".into(),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn draw_replacement_skip_empty_library() -> Self {
        Self {
            id: Some(StaticAbilityId::DrawReplacementSkipEmptyLibrary),
            label: "draw replacement skip empty library".into(),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn conditional_draw_replacement(
        condition: Condition,
        replacement_effects: Vec<E>,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ConditionalDrawReplacement),
            label: display.clone(),
            payload: StaticAbilityPayload::ConditionalDrawReplacement {
                condition,
                replacement_effects,
                display,
            },
        }
    }

    pub fn draw_replacement_exile_top_and_play(count: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::DrawReplacementExileTopAndPlay),
            label: format!("draw replacement exile top {count} and play"),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn keyword_action_replacement(
        action: KeywordActionKind,
        source_filter: ObjectFilter,
        replacement_effects: Vec<E>,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::KeywordActionReplacement),
            label: display.clone(),
            payload: StaticAbilityPayload::KeywordActionReplacement {
                action,
                source_filter,
                replacement_effects,
                display,
            },
        }
    }

    pub fn exile_to_countered_exile_instead_of_graveyard(
        player: PlayerFilter,
        counter_type: CounterType,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard),
            label: "exile to countered exile instead of graveyard".into(),
            payload: StaticAbilityPayload::ExileToCounteredExileInsteadOfGraveyard {
                player,
                counter_type,
            },
        }
    }

    pub fn exile_to_exile_instead_of_graveyard(
        filter: ObjectFilter,
        graveyard_owner: PlayerFilter,
    ) -> Self {
        Self::exile_to_exile_instead_of_graveyard_with_options(filter, graveyard_owner, false)
    }

    pub fn exile_to_exile_instead_of_graveyard_unless_cycled(
        filter: ObjectFilter,
        graveyard_owner: PlayerFilter,
    ) -> Self {
        Self::exile_to_exile_instead_of_graveyard_with_options(filter, graveyard_owner, true)
    }

    fn exile_to_exile_instead_of_graveyard_with_options(
        filter: ObjectFilter,
        graveyard_owner: PlayerFilter,
        exclude_cycled: bool,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ExileToExileInsteadOfGraveyard),
            label: "exile instead of graveyard".into(),
            payload: StaticAbilityPayload::ExileToExileInsteadOfGraveyard {
                filter,
                graveyard_owner,
                exclude_cycled,
            },
        }
    }

    pub fn exile_would_die_instead(filter: ObjectFilter) -> Self {
        Self::exile_would_die_instead_with_damage_source(filter, None)
    }

    pub fn exile_would_die_instead_with_damage_source(
        filter: ObjectFilter,
        damaged_by: Option<DamagedBySource>,
    ) -> Self {
        Self::exile_would_die_instead_with_damage_source_and_follow_up(filter, damaged_by, vec![])
    }

    pub fn exile_would_die_instead_with_damage_source_and_follow_up(
        filter: ObjectFilter,
        damaged_by: Option<DamagedBySource>,
        follow_up_effects: Vec<E>,
    ) -> Self {
        Self::exile_would_die_instead_with_damage_source_counters_and_follow_up(
            filter,
            damaged_by,
            Vec::new(),
            follow_up_effects,
        )
    }

    pub fn exile_would_die_instead_with_damage_source_counters_and_follow_up(
        filter: ObjectFilter,
        damaged_by: Option<DamagedBySource>,
        exile_with_counters: Vec<(CounterType, u32)>,
        follow_up_effects: Vec<E>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ExileWouldDieInstead),
            label: "exile would die instead".into(),
            payload: StaticAbilityPayload::ExileWouldDieInstead {
                filter,
                damaged_by,
                exile_with_counters,
                follow_up_effects,
            },
        }
    }
    pub fn modify_damage_amount_replacement(
        source_filter: ObjectFilter,
        target_player_filter: Option<PlayerFilter>,
        target_object_filter: Option<ObjectFilter>,
        delta: i32,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ModifyDamageAmountReplacement),
            label: display.clone(),
            payload: StaticAbilityPayload::ModifyDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                delta,
                display,
            },
        }
    }
    pub fn minimum_damage_amount_replacement(
        source_filter: ObjectFilter,
        target_player_filter: Option<PlayerFilter>,
        target_object_filter: Option<ObjectFilter>,
        floor: Value,
        noncombat_only: bool,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ModifyDamageAmountReplacement),
            label: display.clone(),
            payload: StaticAbilityPayload::MinimumDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                floor,
                noncombat_only,
                display,
            },
        }
    }
    pub fn double_damage_amount_replacement(
        source_filter: ObjectFilter,
        target_player_filter: Option<PlayerFilter>,
        target_object_filter: Option<ObjectFilter>,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ModifyDamageAmountReplacement),
            label: display.clone(),
            payload: StaticAbilityPayload::DoubleDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                display,
            },
        }
    }

    pub fn double_counters_replacement(
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::DoubleCountersReplacement),
            label: display.clone(),
            payload: StaticAbilityPayload::DoubleCountersReplacement {
                filter,
                counter_type,
                display,
            },
        }
    }

    pub fn double_token_creation_replacement(
        controller: PlayerFilter,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::DoubleTokenCreationReplacement),
            label: display.clone(),
            payload: StaticAbilityPayload::DoubleTokenCreationReplacement {
                controller,
                display,
            },
        }
    }

    pub fn add_token_creation_replacement(
        controller: PlayerFilter,
        token_filter: ObjectFilter,
        additional_token: AdditionalTokenKind,
        additional: i32,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::AddTokenCreationReplacement),
            label: display.clone(),
            payload: StaticAbilityPayload::AddTokenCreationReplacement {
                controller,
                token_filter,
                additional_token,
                additional,
                display,
            },
        }
    }

    pub fn discard_or_redirect_replacement(filter: ObjectFilter, redirect_zone: Zone) -> Self {
        Self {
            id: Some(StaticAbilityId::DiscardOrRedirectReplacement),
            label: "discard or redirect replacement".into(),
            payload: StaticAbilityPayload::DiscardOrRedirectReplacement {
                filter,
                redirect_zone,
            },
        }
    }
    pub fn pay_life_or_enter_tapped(value: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::PayLifeOrEnterTappedReplacement),
            label: "pay life or enter tapped".to_string(),
            payload: StaticAbilityPayload::PayLifeOrEnterTapped(value),
        }
    }
    pub fn copy_activated_abilities(copy: CopyActivatedAbilities) -> Self {
        Self {
            id: Some(StaticAbilityId::CopyActivatedAbilities),
            label: "copy activated abilities".to_string(),
            payload: StaticAbilityPayload::CopyActivatedAbilities(copy),
        }
    }
    pub fn copy_triggered_abilities(copy: CopyTriggeredAbilities) -> Self {
        Self {
            id: Some(StaticAbilityId::CopyTriggeredAbilities),
            label: copy.display.clone(),
            payload: StaticAbilityPayload::CopyTriggeredAbilities(copy),
        }
    }
    pub fn mana_spend_permission(perm: ManaSpendPermission, display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ManaSpendPermission),
            label: display.clone(),
            payload: StaticAbilityPayload::ManaSpendPermission {
                permission: perm,
                display,
            },
        }
    }
    pub fn enters_with_counters_if_condition(
        counter: CounterType,
        count: Value,
        condition: Condition,
        display: impl Into<String>,
    ) -> Self {
        Self::enters_with_counters_and_abilities_if_condition(
            counter,
            count,
            condition,
            display,
            Vec::new(),
        )
    }

    pub fn enters_with_counters_and_abilities_if_condition(
        counter: CounterType,
        count: Value,
        condition: Condition,
        display: impl Into<String>,
        added_abilities: Vec<AbilityModel<T, E, C, Cond>>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::EnterWithCountersIfCondition),
            label: display.clone(),
            payload: StaticAbilityPayload::EntersWithCountersIfCondition {
                counter,
                count,
                condition,
                display,
                added_abilities,
            },
        }
    }
    pub fn enters_with_counters_value(counter: CounterType, count: Value) -> Self {
        Self {
            id: Some(StaticAbilityId::EnterWithCounters),
            label: "enters with counters value".to_string(),
            payload: StaticAbilityPayload::EntersWithCountersValue { counter, count },
        }
    }
    pub fn enters_tapped_for_filter(filter: ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::EnterTappedForFilter),
            label: "enters tapped for filter".to_string(),
            payload: StaticAbilityPayload::EntersTappedForFilter(filter),
        }
    }
    pub fn enters_untapped_for_filter(filter: ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::EnterUntappedForFilter),
            label: "enters untapped for filter".to_string(),
            payload: StaticAbilityPayload::EntersUntappedForFilter(filter),
        }
    }
    pub fn enters_tapped_unless_control_two_or_more_other_lands() -> Self {
        Self::identified(
            StaticAbilityId::EntersTappedUnlessControlTwoOrMoreOtherLands,
            "enters tapped unless control two or more other lands",
        )
    }
    pub fn enters_tapped_unless_control_two_or_fewer_other_lands() -> Self {
        Self::identified(
            StaticAbilityId::EntersTappedUnlessControlTwoOrFewerOtherLands,
            "enters tapped unless control two or fewer other lands",
        )
    }
    pub fn enters_tapped_unless_control_two_or_more_basic_lands() -> Self {
        Self::identified(
            StaticAbilityId::EntersTappedUnlessControlTwoOrMoreBasicLands,
            "enters tapped unless control two or more basic lands",
        )
    }
    pub fn enters_tapped_unless_a_player_has_13_or_less_life() -> Self {
        Self::identified(
            StaticAbilityId::EntersTappedUnlessAPlayerHas13OrLessLife,
            "enters tapped unless a player has 13 or less life",
        )
    }
    pub fn enters_tapped_unless_two_or_more_opponents() -> Self {
        Self::identified(
            StaticAbilityId::EntersTappedUnlessTwoOrMoreOpponents,
            "enters tapped unless two or more opponents",
        )
    }
    pub fn enters_with_counters_and_subtypes_for_filter(
        filter: ObjectFilter,
        counter: CounterType,
        count: Value,
        subtypes: Vec<Subtype>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::EnterWithCountersForFilter),
            label: "enters with counters and subtypes for filter".to_string(),
            payload: StaticAbilityPayload::EntersWithCountersAndSubtypesForFilter {
                filter,
                counter,
                count,
                subtypes,
            },
        }
    }

    pub fn enters_with_characteristics_for_filter(
        filter: ObjectFilter,
        card_types: Vec<CardType>,
        subtypes: Vec<Subtype>,
        power: i32,
        toughness: i32,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::EnterWithCharacteristicsForFilter),
            label: "enters with characteristics for filter".to_string(),
            payload: StaticAbilityPayload::EntersWithCharacteristicsForFilter {
                filter,
                card_types,
                subtypes,
                power,
                toughness,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandwalkKind {
    Subtype { subtype: Subtype, snow: bool },
    AnyLand,
    NonbasicLand,
    ArtifactLand,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anthem {
    pub filter: Option<ObjectFilter>,
    pub power: AnthemValue,
    pub toughness: AnthemValue,
    pub condition: Option<Condition>,
}

impl Anthem {
    pub fn new(filter: ObjectFilter, power: i32, toughness: i32) -> Self {
        Self {
            filter: Some(filter),
            power: AnthemValue::Fixed(power),
            toughness: AnthemValue::Fixed(toughness),
            condition: None,
        }
    }
    pub fn for_source(power: i32, toughness: i32) -> Self {
        Self {
            filter: None,
            power: AnthemValue::Fixed(power),
            toughness: AnthemValue::Fixed(toughness),
            condition: None,
        }
    }
    pub fn with_values(mut self, power: AnthemValue, toughness: AnthemValue) -> Self {
        self.power = power;
        self.toughness = toughness;
        self
    }
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttachedAbilityGrant<T, E, C, Cond> {
    pub ability: AbilityModel<T, E, C, Cond>,
    pub display: String,
    pub condition: Option<Condition>,
}

impl<T, E, C, Cond> AttachedAbilityGrant<T, E, C, Cond> {
    pub fn new(ability: AbilityModel<T, E, C, Cond>, display: impl Into<String>) -> Self {
        Self {
            ability,
            display: display.into(),
            condition: None,
        }
    }
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttachedChosenLandwalkGrant {
    pub display: String,
    pub snow: bool,
}

impl AttachedChosenLandwalkGrant {
    pub fn new(display: impl Into<String>, snow: bool) -> Self {
        Self {
            display: display.into(),
            snow,
        }
    }
    pub fn with_condition(self, _condition: Condition) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantAbility<T, E, C, Cond> {
    pub filter: ObjectFilter,
    pub ability: AbilityModel<T, E, C, Cond>,
    pub condition: Option<Condition>,
}

impl<T, E, C, Cond> GrantAbility<T, E, C, Cond> {
    pub fn new(filter: ObjectFilter, ability: AbilityModel<T, E, C, Cond>) -> Self {
        Self {
            filter,
            ability,
            condition: None,
        }
    }
    pub fn source(ability: impl Into<AbilityModel<T, E, C, Cond>>) -> Self {
        Self {
            filter: ObjectFilter::source(),
            ability: ability.into(),
            condition: None,
        }
    }
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[derive(Clone, PartialEq)]
pub struct GrantObjectAbilityForFilter<T, E, C, Cond> {
    pub filter: ObjectFilter,
    pub ability: AbilityModel<T, E, C, Cond>,
    pub display: String,
    pub condition: Option<Condition>,
}

impl<T, E, C, Cond> std::fmt::Debug for GrantObjectAbilityForFilter<T, E, C, Cond>
where
    T: std::fmt::Debug,
    E: std::fmt::Debug,
    C: std::fmt::Debug,
    Cond: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrantObjectAbilityForFilter")
            .field("filter", &self.filter)
            .field("ability", &self.ability)
            .field(
                "generated_modification",
                &format!("AddAbilityGeneric({:?})", self.ability),
            )
            .field("display", &self.display)
            .field("condition", &self.condition)
            .finish()
    }
}

impl<T, E, C, Cond> GrantObjectAbilityForFilter<T, E, C, Cond> {
    pub fn new(
        filter: ObjectFilter,
        ability: AbilityModel<T, E, C, Cond>,
        display: impl Into<String>,
    ) -> Self {
        Self {
            filter,
            ability,
            display: display.into(),
            condition: None,
        }
    }
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CopyActivatedAbilities {
    pub filter: ObjectFilter,
    pub counter: Option<CounterType>,
    pub exclude_source_name: bool,
    pub exclude_source_id: bool,
    pub force_once_each_turn: bool,
    pub display: String,
}

impl CopyActivatedAbilities {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            counter: None,
            exclude_source_name: false,
            exclude_source_id: true,
            force_once_each_turn: false,
            display: "Has all activated abilities of matching objects".to_string(),
        }
    }
    pub fn with_exclude_source_name(mut self, exclude: bool) -> Self {
        self.exclude_source_name = exclude;
        self
    }
    pub fn with_exclude_source_id(mut self, exclude: bool) -> Self {
        self.exclude_source_id = exclude;
        self
    }
    pub fn with_display(mut self, display: impl Into<String>) -> Self {
        self.display = display.into();
        self
    }
    pub fn with_counter(mut self, counter: CounterType) -> Self {
        self.counter = Some(counter);
        self
    }
    pub fn with_once_each_turn(mut self) -> Self {
        self.force_once_each_turn = true;
        self
    }
    pub fn with_condition(self, _condition: Condition) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CopyTriggeredAbilities {
    pub filter: ObjectFilter,
    pub exclude_source_name: bool,
    pub display: String,
}

impl CopyTriggeredAbilities {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            exclude_source_name: false,
            display: "Has all triggered abilities of matching objects".to_string(),
        }
    }
    pub fn with_exclude_source_name(mut self, exclude: bool) -> Self {
        self.exclude_source_name = exclude;
        self
    }
    pub fn with_display(mut self, display: impl Into<String>) -> Self {
        self.display = display.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostReduction {
    pub filter: ObjectFilter,
    pub amount: Value,
}

impl CostReduction {
    pub fn new(filter: ObjectFilter, amount: Value) -> Self {
        Self { filter, amount }
    }
    pub fn with_condition(self, _condition: Condition) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionalLifeAdditionalCost {
    pub label: String,
    pub life_cost: u32,
}

impl OptionalLifeAdditionalCost {
    pub fn new(label: impl Into<String>, life_cost: u32) -> Self {
        Self {
            label: label.into(),
            life_cost,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostReductionManaCost {
    pub filter: ObjectFilter,
    pub cost: ManaCost,
    pub optional_life_additional_cost: Option<OptionalLifeAdditionalCost>,
}

impl CostReductionManaCost {
    pub fn new(filter: ObjectFilter, cost: ManaCost) -> Self {
        Self {
            filter,
            cost,
            optional_life_additional_cost: None,
        }
    }

    pub fn with_optional_life_additional_cost(
        mut self,
        label: impl Into<String>,
        life_cost: u32,
    ) -> Self {
        self.optional_life_additional_cost =
            Some(OptionalLifeAdditionalCost::new(label, life_cost));
        self
    }
    pub fn with_condition(self, _condition: Condition) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostIncrease {
    pub filter: ObjectFilter,
    pub amount: Value,
    pub condition: Option<Condition>,
}

impl CostIncrease {
    pub fn new(filter: ObjectFilter, amount: Value) -> Self {
        Self {
            filter,
            amount,
            condition: None,
        }
    }
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostIncreaseManaCost {
    pub filter: ObjectFilter,
    pub cost: ManaCost,
}

impl CostIncreaseManaCost {
    pub fn new(filter: ObjectFilter, cost: ManaCost) -> Self {
        Self { filter, cost }
    }
    pub fn with_condition(self, _condition: Condition) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCostReduction<Cond> {
    pub amount: Value,
    pub condition: Cond,
}

impl<Cond> ThisSpellCostReduction<Cond> {
    pub fn new(amount: Value, condition: Cond) -> Self {
        Self { amount, condition }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCostReductionManaCost<Cond> {
    pub cost: ManaCost,
    pub condition: Cond,
}

impl<Cond> ThisSpellCostReductionManaCost<Cond> {
    pub fn new(cost: ManaCost, condition: Cond) -> Self {
        Self { cost, condition }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetColorsForFilter {
    pub filter: ObjectFilter,
    pub color: ColorSet,
}

impl SetColorsForFilter {
    pub fn new(filter: ObjectFilter, color: ColorSet) -> Self {
        Self { filter, color }
    }
    pub fn with_condition(self, _condition: Condition) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveCardTypesForFilter {
    pub filter: ObjectFilter,
    pub types: Vec<CardType>,
    pub condition: Option<Condition>,
}

impl RemoveCardTypesForFilter {
    pub fn new(filter: ObjectFilter, types: Vec<CardType>) -> Self {
        Self {
            filter,
            types,
            condition: None,
        }
    }
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActivatedAbilityCostCondition {
    TargetsExactly { count: usize, filter: ObjectFilter },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttackCostCondition {
    PayGenericPerSourceCounter {
        counter_type: CounterType,
        amount_per_counter: u32,
    },
    ReturnPermanentsToOwnersHand {
        filter: ObjectFilter,
        count: u32,
    },
    SacrificePermanents {
        filter: ObjectFilter,
        count: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttackingGroupAttackCondition {
    AtLeastNOtherCreaturesAttack(u32),
    BlackOrGreenCreatureAlsoAttacks,
    CreatureWithGreaterPowerAlsoAttacks,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefendingPlayerAttackCondition {
    Controls(ObjectFilter),
    ControlsEnchantmentOrEnchantedPermanent,
    HasCardsInGraveyardOrMore(u32),
    IsMonarch,
    IsPoisoned,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CantAttackUnlessConditionSpec {
    AttackCost(AttackCostCondition),
    AttackingGroupCondition(AttackingGroupAttackCondition),
    BattlefieldCountAtLeast { filter: ObjectFilter, count: u32 },
    ControllerControlsMoreThanDefendingPlayer(ObjectFilter),
    ControllerGraveyardHasCardsAtLeast(u32),
    DefendingPlayerCondition(DefendingPlayerAttackCondition),
    OpponentWasDealtDamageThisTurn,
    SourceCondition(Condition),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnterAsCopyAsEntersSpec<T, E, C, Cond> {
    pub filter: ObjectFilter,
    pub affected_filter: Option<ObjectFilter>,
    pub may: bool,
    pub enters_tapped_if_chosen: bool,
    pub linked_exile_pair: Option<EnterAsCopyLinkedExilePairSpec>,
    pub copy_source_self: bool,
    pub copy_source_enchanted: bool,
    pub name_override: Option<String>,
    pub added_card_types: Vec<CardType>,
    pub removed_supertypes: Vec<Supertype>,
    pub added_subtypes: Vec<Subtype>,
    pub added_abilities: Vec<AbilityModel<T, E, C, Cond>>,
    pub set_base_power_toughness: Option<(i32, i32)>,
    pub set_base_power_toughness_from_self: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnterAsCopyLinkedExilePairSpec {
    pub counter_type: CounterType,
}

impl<T, E, C, Cond> crate::GrantStaticAbility for StaticAbility<T, E, C, Cond>
where
    T: Clone + PartialEq + std::fmt::Debug + 'static,
    E: Clone + PartialEq + std::fmt::Debug + 'static,
    C: Clone + PartialEq + std::fmt::Debug + 'static,
    Cond: Clone + PartialEq + std::fmt::Debug + 'static,
{
    fn grant_flash() -> Self {
        Self::flash()
    }

    fn grant_display(&self) -> String {
        self.display()
    }

    fn grant_has_flash(&self) -> bool {
        self.id() == StaticAbilityId::Flash
    }
}

impl LandwalkKind {
    pub fn display(self) -> String {
        match self {
            Self::Subtype {
                subtype,
                snow: false,
            } => format!("{subtype}walk"),
            Self::Subtype {
                subtype,
                snow: true,
            } => format!("Snow {subtype}walk"),
            Self::AnyLand => "Landwalk".to_string(),
            Self::NonbasicLand => "Nonbasic landwalk".to_string(),
            Self::ArtifactLand => "Artifact landwalk".to_string(),
        }
    }
}

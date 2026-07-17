use crate::cost_model::{CoreCostComponent, TotalCost};
use crate::{
    CardType, ColorSet, Condition, ManaSymbol, ObjectFilter, ObjectId, ResolutionProgram,
    StaticAbilityId, Subtype, Zone,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivationTiming {
    #[default]
    AnyTime,
    SorcerySpeed,
    DuringCombat,
    OncePerTurn,
    DuringYourTurn,
    DuringOpponentsTurn,
    /// Only during the upkeep of the player who owns the source object.
    ///
    /// This is distinct from `DuringYourTurn`: Forecast is activated from a
    /// hidden zone and CR 702.57b keys its permission to the card's owner,
    /// not to the ability's controller.
    DuringSourceOwnersUpkeep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaUsageSubtypeRequirement {
    Exact(Subtype),
    ChosenTypeOfSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaSpendBonusCondition {
    IfThisManaIsSpentToCast,
    IfThatManaIsSpentToCast,
    IfThisManaIsSpentOn,
    IfThatManaIsSpentOn,
    WhenYouSpendThisManaToCast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaSpendAbilityGrantDuration {
    UntilEndOfTurn,
    UntilYourNextTurn,
}

/// A keyword ability that can be granted to a spell by mana used to cast it.
///
/// These are kept separate from `StaticAbilityId`: some keywords, such as
/// riot, lower to triggered gameplay abilities rather than a static marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaSpendGrantedKeyword {
    Riot,
}

/// The engine-facing reason for a complete mana-payment transaction.
///
/// This deliberately lives in the shared model rather than the runtime cost
/// payer so a mana unit's predicate can be compiled without depending on the
/// runtime crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaPaymentPurpose {
    CastSpell,
    ActivateAbility,
    ActivateManaAbility,
    CumulativeUpkeep,
    UnlockDoor,
    TurnFaceUp,
    Effect,
    Other,
}

/// A composable predicate over the complete transaction a mana unit would pay.
#[derive(Debug, Clone, PartialEq)]
pub enum ManaPaymentPredicate {
    Any,
    Purpose(ManaPaymentPurpose),
    SourceMatches(ObjectFilter),
    CostContains(ManaSymbol),
    CostContainsX,
    SharesCreatureTypeWithPayersCommander,
    All(Vec<ManaPaymentPredicate>),
    AnyOf(Vec<ManaPaymentPredicate>),
    Not(Box<ManaPaymentPredicate>),
}

impl Eq for ManaPaymentPredicate {}

/// A generic payload carried by one produced mana unit.
///
/// Each matching unit creates its own copy of this program when spent, which
/// is the per-unit multiplicity required by CR 106.6a.
#[derive(Debug, Clone, PartialEq)]
pub struct ManaSpendPayload<E> {
    pub predicate: ManaPaymentPredicate,
    pub effects: ResolutionProgram<E>,
    pub choices: Vec<crate::ChooseSpec>,
}

impl<E: PartialEq> Eq for ManaSpendPayload<E> {}

#[derive(Debug, Clone, PartialEq)]
pub enum ManaUsageRestriction<E> {
    CastSpell {
        card_types: Vec<CardType>,
        subtype_requirement: Option<ManaUsageSubtypeRequirement>,
        restrict_to_matching_spell: bool,
        grant_uncounterable: bool,
        enters_with_counters: Vec<(crate::CounterType, u32)>,
        granted_abilities: Vec<StaticAbilityId>,
    },
    CastSpellMatching {
        filter: ObjectFilter,
        restrict_to_matching_spell: bool,
        grant_uncounterable: bool,
        enters_with_counters: Vec<(crate::CounterType, u32)>,
        granted_abilities: Vec<StaticAbilityId>,
    },
    /// A bonus carried by a produced mana unit and applied only when that unit
    /// is spent on a spell matching `filter`. Unlike a spending restriction,
    /// this never prevents the mana from being used for a different purpose.
    CastSpellWithManaBonus {
        filter: ObjectFilter,
        condition: ManaSpendBonusCondition,
        grant_uncounterable: bool,
        enters_with_counters: Vec<(crate::CounterType, u32)>,
        granted_abilities: Vec<(StaticAbilityId, ManaSpendAbilityGrantDuration)>,
        granted_keywords: Vec<ManaSpendGrantedKeyword>,
    },
    CastSpellOrActivateAbilitySourceMatching {
        spell_filter: ObjectFilter,
        ability_source_filter: ObjectFilter,
    },
    CastSpellOrUnlockDoorOrTurnFaceUp {
        spell_filter: ObjectFilter,
    },
    ActivateAbility,
    /// Generic CR 106.6 transaction rule. An empty `on_spend` list is a pure
    /// spending restriction; a predicate of `Any` with payloads is an
    /// unrestricted mana unit carrying only additional effects.
    PaymentTransaction {
        restriction: Option<ManaPaymentPredicate>,
        on_spend: Vec<ManaSpendPayload<E>>,
    },
}

impl<E: PartialEq> Eq for ManaUsageRestriction<E> {}

impl<E> ManaUsageRestriction<E> {
    pub fn try_map_effects<E2: Clone, Error>(
        self,
        map_effect: &mut impl FnMut(E) -> Result<E2, Error>,
    ) -> Result<ManaUsageRestriction<E2>, Error> {
        Ok(match self {
            Self::CastSpell {
                card_types,
                subtype_requirement,
                restrict_to_matching_spell,
                grant_uncounterable,
                enters_with_counters,
                granted_abilities,
            } => ManaUsageRestriction::CastSpell {
                card_types,
                subtype_requirement,
                restrict_to_matching_spell,
                grant_uncounterable,
                enters_with_counters,
                granted_abilities,
            },
            Self::CastSpellMatching {
                filter,
                restrict_to_matching_spell,
                grant_uncounterable,
                enters_with_counters,
                granted_abilities,
            } => ManaUsageRestriction::CastSpellMatching {
                filter,
                restrict_to_matching_spell,
                grant_uncounterable,
                enters_with_counters,
                granted_abilities,
            },
            Self::CastSpellWithManaBonus {
                filter,
                condition,
                grant_uncounterable,
                enters_with_counters,
                granted_abilities,
                granted_keywords,
            } => ManaUsageRestriction::CastSpellWithManaBonus {
                filter,
                condition,
                grant_uncounterable,
                enters_with_counters,
                granted_abilities,
                granted_keywords,
            },
            Self::CastSpellOrActivateAbilitySourceMatching {
                spell_filter,
                ability_source_filter,
            } => ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching {
                spell_filter,
                ability_source_filter,
            },
            Self::CastSpellOrUnlockDoorOrTurnFaceUp { spell_filter } => {
                ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp { spell_filter }
            }
            Self::ActivateAbility => ManaUsageRestriction::ActivateAbility,
            Self::PaymentTransaction {
                restriction,
                on_spend,
            } => ManaUsageRestriction::PaymentTransaction {
                restriction,
                on_spend: on_spend
                    .into_iter()
                    .map(|payload| {
                        Ok(ManaSpendPayload {
                            predicate: payload.predicate,
                            effects: payload.effects.try_map_effects(&mut *map_effect)?,
                            choices: payload.choices,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RestrictedManaUnit<E> {
    pub symbol: ManaSymbol,
    pub source: ObjectId,
    pub source_chosen_creature_type: Option<Subtype>,
    pub restrictions: Vec<ManaUsageRestriction<E>>,
}

impl<E: PartialEq> Eq for RestrictedManaUnit<E> {}

#[derive(Debug, Clone, PartialEq)]
pub struct Ability<SA, T, E, C> {
    pub kind: AbilityKind<SA, T, E, C>,
    pub functional_zones: Vec<Zone>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AbilityKind<SA, T, E, C> {
    Static(SA),
    Triggered(TriggeredAbility<T, E>),
    Activated(ActivatedAbility<E, C>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProtectionFrom {
    Color(ColorSet),
    Colorless,
    AllColors,
    Creatures,
    CardType(CardType),
    Permanents(ObjectFilter),
    EachManaValueAmong(ObjectFilter),
    ChosenPlayer,
    ChosenColor,
    Everything,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LevelAbility<SA> {
    pub min_level: u32,
    pub max_level: Option<u32>,
    pub power_toughness: Option<(i32, i32)>,
    pub abilities: Vec<SA>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PresentationKeyword {
    Prowess,
    Firebending(String),
    Toxic(u32),
    Poisonous(u32),
    Afflict(u32),
    Amplify(u32),
    Devour(u32),
    Suspend,
    Recover(String),
    Casualty(u32),
    Soulshift(String),
}

impl PresentationKeyword {
    pub fn from_legacy_keyword(label: &str) -> Option<Self> {
        let payload = label.trim().strip_prefix("keyword:")?.trim();
        let lower = payload.to_ascii_lowercase();
        if lower == "prowess" {
            return Some(Self::Prowess);
        }
        if lower == "suspend" {
            return Some(Self::Suspend);
        }
        if let Some(rest) = lower.strip_prefix("firebending ") {
            return Some(Self::Firebending(rest.trim().to_string()));
        }
        if let Some(rest) = lower.strip_prefix("toxic ") {
            return rest.parse().ok().map(Self::Toxic);
        }
        if let Some(rest) = lower.strip_prefix("poisonous ") {
            return rest.parse().ok().map(Self::Poisonous);
        }
        if let Some(rest) = lower.strip_prefix("afflict ") {
            return rest.parse().ok().map(Self::Afflict);
        }
        if let Some(rest) = lower.strip_prefix("amplify ") {
            return rest.parse().ok().map(Self::Amplify);
        }
        if let Some(rest) = lower.strip_prefix("devour ") {
            return rest.parse().ok().map(Self::Devour);
        }
        if let Some(rest) = payload.strip_prefix("recover ") {
            return Some(Self::Recover(rest.trim().to_string()));
        }
        if let Some(rest) = lower.strip_prefix("casualty ") {
            return rest.parse().ok().map(Self::Casualty);
        }
        if let Some(rest) = payload.strip_prefix("soulshift ") {
            return Some(Self::Soulshift(rest.trim().to_string()));
        }
        None
    }

    pub fn display(&self) -> String {
        match self {
            Self::Prowess => "Prowess".to_string(),
            Self::Firebending(amount) => format!("Firebending {amount}"),
            Self::Toxic(amount) => format!("Toxic {amount}"),
            Self::Poisonous(amount) => format!("Poisonous {amount}"),
            Self::Afflict(amount) => format!("Afflict {amount}"),
            Self::Amplify(amount) => format!("Amplify {amount}"),
            Self::Devour(amount) => format!("Devour {amount}"),
            Self::Suspend => "Suspend".to_string(),
            Self::Recover(cost) => format!("Recover {cost}"),
            Self::Casualty(power) => format!("Casualty {power}"),
            Self::Soulshift(amount) => format!("Soulshift {amount}"),
        }
    }

    pub fn matches_name(&self, name: &str) -> bool {
        let lower = name.trim().to_ascii_lowercase();
        matches!(
            (self, lower.as_str()),
            (Self::Prowess, "prowess")
                | (Self::Suspend, "suspend")
                | (Self::Firebending(_), "firebending")
                | (Self::Toxic(_), "toxic")
                | (Self::Poisonous(_), "poisonous")
                | (Self::Afflict(_), "afflict")
                | (Self::Amplify(_), "amplify")
                | (Self::Devour(_), "devour")
                | (Self::Recover(_), "recover")
                | (Self::Casualty(_), "casualty")
                | (Self::Soulshift(_), "soulshift")
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivatedPresentationLabel {
    ThrowEllipsis,
    Boast,
    Exhaust,
    Renew,
    Channel,
    Cohort,
    Teleport,
    Transmute,
}

impl ActivatedPresentationLabel {
    pub fn from_label(label: &str) -> Option<Self> {
        let trimmed = label.trim();
        if trimmed.eq_ignore_ascii_case("Throw ...") || trimmed.eq_ignore_ascii_case("Throw") {
            return Some(Self::ThrowEllipsis);
        }
        let head = trimmed.split_whitespace().next().unwrap_or(trimmed);
        match head.to_ascii_lowercase().as_str() {
            "boast" => Some(Self::Boast),
            "exhaust" => Some(Self::Exhaust),
            "renew" => Some(Self::Renew),
            "channel" => Some(Self::Channel),
            "cohort" => Some(Self::Cohort),
            "teleport" => Some(Self::Teleport),
            "transmute" => Some(Self::Transmute),
            _ => None,
        }
    }

    pub fn display(self) -> &'static str {
        match self {
            Self::ThrowEllipsis => "Throw ...",
            Self::Boast => "Boast",
            Self::Exhaust => "Exhaust",
            Self::Renew => "Renew",
            Self::Channel => "Channel",
            Self::Cohort => "Cohort",
            Self::Teleport => "Teleport",
            Self::Transmute => "Transmute",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PresentationLabel {
    AbilityWord(String),
    Keyword(PresentationKeyword),
    CaseSolved,
    CaseToSolve,
    Activated(ActivatedPresentationLabel),
}

impl PresentationLabel {
    pub fn from_ability_word(label: impl Into<String>) -> Self {
        let label = label.into();
        let trimmed = label.trim();
        if trimmed.eq_ignore_ascii_case("solved") {
            return Self::CaseSolved;
        }
        if trimmed.eq_ignore_ascii_case("__ironsmith_case_solved") {
            return Self::CaseSolved;
        }
        if trimmed.eq_ignore_ascii_case("to solve") {
            return Self::CaseToSolve;
        }
        if trimmed.eq_ignore_ascii_case("__ironsmith_case_to_solve") {
            return Self::CaseToSolve;
        }
        if let Some(keyword) = PresentationKeyword::from_legacy_keyword(trimmed) {
            return Self::Keyword(keyword);
        }
        if let Some(activated) = ActivatedPresentationLabel::from_label(trimmed) {
            return Self::Activated(activated);
        }
        Self::AbilityWord(trimmed.to_string())
    }

    pub fn display_prefix(&self) -> Option<String> {
        match self {
            Self::AbilityWord(label) if label.trim().is_empty() => None,
            Self::AbilityWord(label) => Some(label.clone()),
            Self::Keyword(keyword) => Some(keyword.display()),
            Self::CaseSolved => Some("Solved".to_string()),
            Self::CaseToSolve => Some("To solve".to_string()),
            Self::Activated(label) => Some(label.display().to_string()),
        }
    }

    pub fn activated_display(&self) -> Option<&'static str> {
        match self {
            Self::Activated(label) => Some(label.display()),
            _ => None,
        }
    }

    pub fn is_keyword(&self, keyword: &str) -> bool {
        matches!(self, Self::Keyword(label) if label.matches_name(keyword))
    }

    pub fn recover_cost(&self) -> Option<&str> {
        match self {
            Self::Keyword(PresentationKeyword::Recover(cost)) => Some(cost.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TriggeredAbility<T, E> {
    pub trigger: T,
    pub effects: ResolutionProgram<E>,
    pub choices: Vec<crate::ChooseSpec>,
    pub intervening_if: Option<Condition>,
    pub presentation_label: Option<PresentationLabel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivatedAbility<E, C> {
    pub mana_cost: TotalCost<C>,
    pub effects: ResolutionProgram<E>,
    pub choices: Vec<crate::ChooseSpec>,
    pub timing: ActivationTiming,
    pub is_loyalty_ability: bool,
    pub additional_restrictions: Vec<String>,
    pub activation_restrictions: Vec<Condition>,
    pub mana_output: Option<Vec<ManaSymbol>>,
    pub activation_condition: Option<Condition>,
    pub mana_usage_restrictions: Vec<ManaUsageRestriction<E>>,
}

impl<SA, T, E, C> Ability<SA, T, E, C>
where
    E: Clone,
    C: CoreCostComponent,
{
    pub fn static_ability(effect: SA) -> Self {
        Self {
            kind: AbilityKind::Static(effect),
            functional_zones: vec![Zone::Battlefield],
        }
    }

    pub fn triggered(trigger: T, effects: impl Into<ResolutionProgram<E>>) -> Self {
        Self {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger,
                effects: effects.into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        }
    }

    pub fn triggered_optional(trigger: T, effects: impl Into<ResolutionProgram<E>>) -> Self {
        Self::triggered(trigger, effects)
    }

    pub fn activated(mana_cost: TotalCost<C>, effects: impl Into<ResolutionProgram<E>>) -> Self {
        Self::activated_with_timing(mana_cost, effects, ActivationTiming::AnyTime)
    }

    pub fn activated_with_timing(
        mana_cost: TotalCost<C>,
        effects: impl Into<ResolutionProgram<E>>,
        timing: ActivationTiming,
    ) -> Self {
        Self {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost,
                effects: effects.into(),
                choices: vec![],
                timing,
                is_loyalty_ability: false,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
        }
    }

    pub fn activated_with_costs(
        cost: TotalCost<C>,
        additional_costs: Vec<C>,
        effects: impl Into<ResolutionProgram<E>>,
    ) -> Self {
        let mut costs = cost.costs().to_vec();
        costs.extend(additional_costs);
        Self {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::from_costs(costs),
                effects: effects.into(),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                is_loyalty_ability: false,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
        }
    }

    pub fn mana(cost: TotalCost<C>, mana: Vec<ManaSymbol>) -> Self {
        let mut costs = cost.costs().to_vec();
        if !costs.iter().any(|c| c.requires_tap()) {
            costs.push(C::tap_cost());
        }
        Self {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::from_costs(costs),
                effects: ResolutionProgram::default(),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                is_loyalty_ability: false,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: Some(mana),
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
        }
    }

    pub fn mana_with_effects(cost: TotalCost<C>, effects: impl Into<ResolutionProgram<E>>) -> Self {
        let mut costs = cost.costs().to_vec();
        costs.push(C::tap_cost());
        Self {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::from_costs(costs),
                effects: effects.into(),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                is_loyalty_ability: false,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: Some(vec![]),
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
        }
    }

    pub fn basic_land_mana(subtype: Subtype) -> Option<Self> {
        let symbol = match subtype {
            Subtype::Plains => ManaSymbol::White,
            Subtype::Island => ManaSymbol::Blue,
            Subtype::Swamp => ManaSymbol::Black,
            Subtype::Mountain => ManaSymbol::Red,
            Subtype::Forest => ManaSymbol::Green,
            _ => return None,
        };

        Some(Self {
            kind: AbilityKind::Activated(ActivatedAbility::basic_mana(symbol)),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    pub fn in_zones(mut self, zones: Vec<Zone>) -> Self {
        self.functional_zones = zones;
        self
    }

    pub fn is_mana_ability(&self) -> bool {
        matches!(&self.kind, AbilityKind::Activated(a) if a.is_mana_ability())
    }

    pub fn functions_in(&self, zone: &Zone) -> bool {
        self.functional_zones.contains(zone)
    }

    pub fn try_map<SA2, T2, E2, C2, Error>(
        self,
        mut map_static: impl FnMut(SA) -> Result<SA2, Error>,
        mut map_trigger: impl FnMut(T) -> Result<T2, Error>,
        mut map_effect: impl FnMut(E) -> Result<E2, Error>,
        mut map_cost: impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<Ability<SA2, T2, E2, C2>, Error>
    where
        E2: Clone,
    {
        let kind = match self.kind {
            AbilityKind::Static(static_ability) => AbilityKind::Static(map_static(static_ability)?),
            AbilityKind::Triggered(triggered) => AbilityKind::Triggered(TriggeredAbility {
                trigger: map_trigger(triggered.trigger)?,
                effects: triggered.effects.try_map_effects(&mut map_effect)?,
                choices: triggered.choices,
                intervening_if: triggered.intervening_if,
                presentation_label: triggered.presentation_label,
            }),
            AbilityKind::Activated(activated) => AbilityKind::Activated(ActivatedAbility {
                mana_cost: activated.mana_cost.try_map(&mut map_cost)?,
                effects: activated.effects.try_map_effects(&mut map_effect)?,
                choices: activated.choices,
                timing: activated.timing,
                is_loyalty_ability: activated.is_loyalty_ability,
                additional_restrictions: activated.additional_restrictions,
                activation_restrictions: activated.activation_restrictions,
                mana_output: activated.mana_output,
                activation_condition: activated.activation_condition,
                mana_usage_restrictions: activated
                    .mana_usage_restrictions
                    .into_iter()
                    .map(|restriction| restriction.try_map_effects(&mut map_effect))
                    .collect::<Result<Vec<_>, Error>>()?,
            }),
        };

        Ok(Ability {
            kind,
            functional_zones: self.functional_zones,
        })
    }
}

impl<SA, T, E, C> From<SA> for Ability<SA, T, E, C>
where
    E: Clone,
    C: CoreCostComponent,
{
    fn from(value: SA) -> Self {
        Self::static_ability(value)
    }
}

impl<SA> LevelAbility<SA> {
    pub fn new(min_level: u32, max_level: Option<u32>) -> Self {
        Self {
            min_level,
            max_level,
            power_toughness: None,
            abilities: Vec::new(),
        }
    }

    pub fn with_pt(mut self, power: i32, toughness: i32) -> Self {
        self.power_toughness = Some((power, toughness));
        self
    }

    pub fn with_ability(mut self, ability: SA) -> Self {
        self.abilities.push(ability);
        self
    }

    pub fn with_abilities(mut self, abilities: Vec<SA>) -> Self {
        self.abilities.extend(abilities);
        self
    }

    pub fn applies_at_level(&self, level_count: u32) -> bool {
        level_count >= self.min_level && self.max_level.is_none_or(|max| level_count <= max)
    }
}

impl<T, E: Clone> TriggeredAbility<T, E> {
    pub fn with_targets(mut self, targets: Vec<crate::ChooseSpec>) -> Self {
        self.choices = targets;
        self
    }

    pub fn with_intervening_if(mut self, condition: Condition) -> Self {
        self.intervening_if = Some(condition);
        self
    }
}

impl<E: Clone, C: CoreCostComponent> ActivatedAbility<E, C> {
    pub fn produces_mana(&self) -> bool {
        self.mana_output.is_some()
    }

    pub fn has_targets(&self) -> bool {
        self.choices.iter().any(crate::ChooseSpec::is_target)
    }

    pub fn is_loyalty_ability(&self) -> bool {
        self.is_loyalty_ability || self.mana_cost.has_loyalty_activation_cost()
    }

    pub fn is_mana_ability(&self) -> bool {
        self.produces_mana() && !self.has_targets() && !self.is_loyalty_ability()
    }

    pub fn mana_symbols(&self) -> &[ManaSymbol] {
        self.mana_output.as_deref().unwrap_or(&[])
    }

    pub fn with_targets(mut self, targets: Vec<crate::ChooseSpec>) -> Self {
        self.choices = targets;
        self
    }

    pub fn sorcery_speed(mut self) -> Self {
        self.timing = ActivationTiming::SorcerySpeed;
        self
    }

    pub fn once_per_turn(mut self) -> Self {
        self.timing = ActivationTiming::OncePerTurn;
        self
    }

    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.activation_condition = Some(condition);
        self
    }

    pub fn with_costs(mut self, additional_costs: Vec<C>) -> Self {
        let mut costs = self.mana_cost.costs().to_vec();
        costs.extend(additional_costs);
        self.mana_cost = TotalCost::from_costs(costs);
        self
    }

    pub fn has_tap_cost(&self) -> bool {
        fn contains_tap<C: CoreCostComponent>(cost: &TotalCost<C>) -> bool {
            match cost.kind() {
                crate::TotalCostKind::All(costs) => costs.iter().any(|cost| cost.requires_tap()),
                crate::TotalCostKind::OneOf(branches) => branches.iter().any(contains_tap),
            }
        }

        contains_tap(&self.mana_cost)
    }

    pub fn has_sacrifice_self_cost(&self) -> bool {
        fn contains_sacrifice_self<C: CoreCostComponent>(cost: &TotalCost<C>) -> bool {
            match cost.kind() {
                crate::TotalCostKind::All(costs) => {
                    costs.iter().any(|cost| cost.is_sacrifice_self())
                }
                crate::TotalCostKind::OneOf(branches) => {
                    branches.iter().any(contains_sacrifice_self)
                }
            }
        }

        contains_sacrifice_self(&self.mana_cost)
    }

    pub fn life_cost_amount(&self) -> Option<u32> {
        fn first_life_cost<C: CoreCostComponent>(cost: &TotalCost<C>) -> Option<u32> {
            match cost.kind() {
                crate::TotalCostKind::All(costs) => {
                    costs.iter().find_map(|cost| cost.life_amount())
                }
                crate::TotalCostKind::OneOf(branches) => branches.iter().find_map(first_life_cost),
            }
        }

        first_life_cost(&self.mana_cost)
    }

    pub fn is_exhaust_ability(&self) -> bool {
        self.additional_restrictions.iter().any(|restriction| {
            let lower = restriction.to_ascii_lowercase();
            lower.contains("activate each exhaust ability only once")
                || lower.contains("activate this exhaust ability only once")
        })
    }

    /// Minimum X announced for this activation. Oracle currently expresses
    /// this activated-ability constraint as the standalone sentence
    /// "X can't be 0."; keeping it on the ability lets the decision flow
    /// enforce the restriction while retaining the authored surface.
    pub fn activation_x_minimum(&self) -> u32 {
        self.additional_restrictions
            .iter()
            .any(|restriction| {
                let normalized = restriction
                    .trim()
                    .trim_end_matches('.')
                    .replace('’', "'")
                    .to_ascii_lowercase();
                matches!(normalized.as_str(), "x can't be 0" | "x cant be 0")
            })
            .then_some(1)
            .unwrap_or(0)
    }

    pub fn max_activations_per_turn(&self) -> Option<u32> {
        fn min_cap(current: Option<u32>, next: u32) -> Option<u32> {
            Some(current.map_or(next, |existing| existing.min(next)))
        }

        let mut cap = None;
        if self.timing == ActivationTiming::OncePerTurn {
            cap = min_cap(cap, 1);
        }

        if let Some(Condition::MaxActivationsPerTurn(limit)) = self.activation_condition.as_ref() {
            cap = min_cap(cap, *limit);
        }

        for restriction in &self.activation_restrictions {
            if let Condition::MaxActivationsPerTurn(limit) = restriction {
                cap = min_cap(cap, *limit);
            }
        }

        if cap.is_some() {
            return cap;
        }

        self.additional_restrictions
            .iter()
            .find_map(|restriction| parse_activation_max_times_per_turn(restriction))
    }

    pub fn basic_mana(mana: ManaSymbol) -> Self {
        Self {
            mana_cost: TotalCost::from_cost(C::tap_cost()),
            effects: ResolutionProgram::default(),
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            is_loyalty_ability: false,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: Some(vec![mana]),
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }
    }

    pub fn mana_with_costs(
        cost: TotalCost<C>,
        additional_costs: Vec<C>,
        mana: Vec<ManaSymbol>,
    ) -> Self {
        let mut costs = cost.costs().to_vec();
        costs.extend(additional_costs);
        Self {
            mana_cost: TotalCost::from_costs(costs),
            effects: ResolutionProgram::default(),
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            is_loyalty_ability: false,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: Some(mana),
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }
    }

    pub fn conditional_mana(mana: ManaSymbol, required_subtypes: Vec<Subtype>) -> Self {
        let mut condition: Option<Condition> = None;
        for subtype in required_subtypes {
            let next = Condition::YouControl(
                ObjectFilter::default()
                    .with_type(CardType::Land)
                    .with_subtype(subtype),
            );
            condition = Some(match condition {
                Some(existing) => Condition::Or(Box::new(existing), Box::new(next)),
                None => next,
            });
        }

        Self {
            mana_cost: TotalCost::from_cost(C::tap_cost()),
            effects: ResolutionProgram::default(),
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            is_loyalty_ability: false,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: Some(vec![mana]),
            activation_condition: condition,
            mana_usage_restrictions: vec![],
        }
    }
}

fn parse_activation_max_times_per_turn(restriction: &str) -> Option<u32> {
    let normalized = restriction
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c.is_ascii_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>();

    let words: Vec<&str> = normalized.split_whitespace().collect();
    if words.len() < 4 || !words.contains(&"activate") {
        return None;
    }

    let each_turn_pos = words
        .windows(2)
        .position(|window| window[0] == "each" && window[1] == "turn")?;
    if each_turn_pos == 0 {
        return None;
    }

    if each_turn_pos >= 4 {
        for idx in 0..=each_turn_pos - 4 {
            if words[idx] == "no" && words[idx + 1] == "more" && words[idx + 2] == "than" {
                if let Some(parsed) = parse_named_count_word(words[idx + 3]) {
                    return Some(parsed);
                }
            }
        }
    }

    let mut count_word = words[each_turn_pos - 1];
    if (count_word == "time" || count_word == "times") && each_turn_pos >= 2 {
        count_word = words[each_turn_pos - 2];
    }

    parse_named_count_word(count_word)
}

fn parse_named_count_word(word: &str) -> Option<u32> {
    match word {
        "once" => Some(1),
        "twice" => Some(2),
        _ => crate::parse_cardinal_word(word),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChooseSpec, Cost, CounterType};

    #[test]
    fn targeted_mana_production_is_not_a_mana_ability() {
        let mut ability = ActivatedAbility::<(), Cost<()>>::basic_mana(ManaSymbol::Green);
        ability.choices.push(ChooseSpec::target_player());

        assert!(ability.produces_mana());
        assert!(ability.has_targets());
        assert!(!ability.is_mana_ability());
    }

    #[test]
    fn loyalty_mana_production_is_not_a_mana_ability() {
        let mut flagged = ActivatedAbility::<(), Cost<()>>::basic_mana(ManaSymbol::Green);
        flagged.is_loyalty_ability = true;
        assert!(flagged.produces_mana());
        assert!(flagged.is_loyalty_ability());
        assert!(!flagged.is_mana_ability());

        let mut inferred = ActivatedAbility::<(), Cost<()>>::basic_mana(ManaSymbol::Green);
        inferred.mana_cost = TotalCost::from_cost(Cost::add_counters(CounterType::Loyalty, 1));
        assert!(inferred.is_loyalty_ability());
        assert!(!inferred.is_mana_ability());
    }
}

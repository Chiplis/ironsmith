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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaUsageSubtypeRequirement {
    Exact(Subtype),
    ChosenTypeOfSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ManaUsageRestriction {
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
    ActivateAbility,
    PayCumulativeUpkeepCosts,
}

impl Eq for ManaUsageRestriction {}

#[derive(Debug, Clone, PartialEq)]
pub struct RestrictedManaUnit {
    pub symbol: ManaSymbol,
    pub source: ObjectId,
    pub source_chosen_creature_type: Option<Subtype>,
    pub restrictions: Vec<ManaUsageRestriction>,
}

impl Eq for RestrictedManaUnit {}

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

#[derive(Debug, Clone, PartialEq)]
pub struct TriggeredAbility<T, E> {
    pub trigger: T,
    pub effects: ResolutionProgram<E>,
    pub choices: Vec<crate::ChooseSpec>,
    pub intervening_if: Option<Condition>,
    pub presentation_label: Option<String>,
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
    pub mana_usage_restrictions: Vec<ManaUsageRestriction>,
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
                mana_usage_restrictions: activated.mana_usage_restrictions,
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
        self.mana_cost.costs().iter().any(|c| c.requires_tap())
    }

    pub fn has_sacrifice_self_cost(&self) -> bool {
        self.mana_cost.costs().iter().any(|c| c.is_sacrifice_self())
    }

    pub fn life_cost_amount(&self) -> Option<u32> {
        self.mana_cost.costs().iter().find_map(|c| c.life_amount())
    }

    pub fn is_exhaust_ability(&self) -> bool {
        self.additional_restrictions.iter().any(|restriction| {
            let lower = restriction.to_ascii_lowercase();
            lower.contains("activate each exhaust ability only once")
                || lower.contains("activate this exhaust ability only once")
        })
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

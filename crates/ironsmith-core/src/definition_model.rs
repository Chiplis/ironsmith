use crate::tag::TagKeyWalk;

use crate::{AuraAttachmentFilter, Card, CostComponent, ResolutionProgram, TotalCost};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct CardDefinition<A, E, C, AC, OC> {
    pub card: Card,
    /// Canonical rules text rendered by the compiler-side presentation layer.
    ///
    /// Definitions embedded in executable payloads carry their own metadata.
    /// Both fields are required on the wire, including explicit empty values.
    pub canonical_text: String,
    /// Canonical labels for the executable abilities in `abilities`.
    pub ability_labels: Vec<String>,
    pub abilities: Vec<A>,
    pub spell_effect: Option<ResolutionProgram<E>>,
    pub aura_attach_filter: Option<AuraAttachmentFilter>,
    pub alternative_casts: Vec<AC>,
    pub has_fuse: bool,
    pub optional_costs: Vec<OC>,
    pub additional_cost: TotalCost<C>,
    /// True when the card's rules text refers to ante. CR 407.3 makes such a
    /// card illegal outside a game played for ante.
    pub refers_to_ante: bool,
}

impl<A, E, C, AC, OC> CardDefinition<A, E, C, AC, OC>
where
    C: CostComponent,
{
    pub fn new(card: Card) -> Self {
        Self {
            card,
            canonical_text: String::new(),
            ability_labels: Vec::new(),
            abilities: Vec::new(),
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            has_fuse: false,
            optional_costs: Vec::new(),
            additional_cost: TotalCost::free(),
            refers_to_ante: false,
        }
    }

    pub fn with_abilities(card: Card, abilities: Vec<A>) -> Self {
        Self {
            card,
            canonical_text: String::new(),
            ability_labels: Vec::new(),
            abilities,
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            has_fuse: false,
            optional_costs: Vec::new(),
            additional_cost: TotalCost::free(),
            refers_to_ante: false,
        }
    }

    pub fn spell(card: Card, effects: Vec<E>) -> Self
    where
        E: Clone,
    {
        Self {
            card,
            canonical_text: String::new(),
            ability_labels: Vec::new(),
            abilities: Vec::new(),
            spell_effect: Some(ResolutionProgram::from_effects(effects)),
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            has_fuse: false,
            optional_costs: Vec::new(),
            additional_cost: TotalCost::free(),
            refers_to_ante: false,
        }
    }

    pub fn spell_with_abilities(card: Card, effects: Vec<E>, abilities: Vec<A>) -> Self
    where
        E: Clone,
    {
        Self {
            card,
            canonical_text: String::new(),
            ability_labels: Vec::new(),
            abilities,
            spell_effect: Some(ResolutionProgram::from_effects(effects)),
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            has_fuse: false,
            optional_costs: Vec::new(),
            additional_cost: TotalCost::free(),
            refers_to_ante: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.card.name
    }

    pub fn is_creature(&self) -> bool {
        self.card.is_creature()
    }

    pub fn is_spell(&self) -> bool {
        self.card.is_instant() || self.card.is_sorcery()
    }

    pub fn is_permanent(&self) -> bool {
        self.card.is_creature()
            || self.card.is_artifact()
            || self.card.is_enchantment()
            || self.card.is_land()
            || self.card.is_planeswalker()
    }

    pub fn try_map<A2, E2, C2, AC2, OC2, Error>(
        self,
        mut map_ability: impl FnMut(A) -> Result<A2, Error>,
        mut map_effect: impl FnMut(E) -> Result<E2, Error>,
        map_cost: impl FnMut(C) -> Result<C2, Error>,
        mut map_alternative_cast: impl FnMut(AC) -> Result<AC2, Error>,
        mut map_optional_cost: impl FnMut(OC) -> Result<OC2, Error>,
    ) -> Result<CardDefinition<A2, E2, C2, AC2, OC2>, Error>
    where
        E2: Clone,
    {
        let mut abilities = Vec::with_capacity(self.abilities.len());
        for ability in self.abilities {
            abilities.push(map_ability(ability)?);
        }

        let mut alternative_casts = Vec::with_capacity(self.alternative_casts.len());
        for method in self.alternative_casts {
            alternative_casts.push(map_alternative_cast(method)?);
        }

        let mut optional_costs = Vec::with_capacity(self.optional_costs.len());
        for cost in self.optional_costs {
            optional_costs.push(map_optional_cost(cost)?);
        }

        Ok(CardDefinition {
            card: self.card,
            canonical_text: self.canonical_text,
            ability_labels: self.ability_labels,
            abilities,
            spell_effect: self
                .spell_effect
                .map(|effects| effects.try_map_effects(&mut map_effect))
                .transpose()?,
            aura_attach_filter: self.aura_attach_filter,
            alternative_casts,
            has_fuse: self.has_fuse,
            optional_costs,
            additional_cost: self.additional_cost.try_map(map_cost)?,
            refers_to_ante: self.refers_to_ante,
        })
    }
}

/// Generic payment shapes produced from a custom keyword's reminder text.
pub const GENERIC_ALTERNATIVE_COST_NAME: &str = "Parsed alternative cost";

impl<A, E, C, Cond> CardDefinition<A, E, C, crate::AlternativeCastingMethod<E, C, Cond>, crate::OptionalCost<C>>
where
    C: CostComponent + Clone,
    E: Clone,
    Cond: Clone,
{
    /// Attribute the single generic payment a custom keyword's definition
    /// created to that keyword, so "a spell using <keyword>" can read its
    /// receipt. Official payments are never renamed. Returns whether a unique
    /// generic payment existed.
    pub fn attribute_custom_keyword_payment(&mut self, keyword: &str) -> bool {
        let keyword = keyword.trim();
        let generic_alternatives = self
            .alternative_casts
            .iter()
            .filter(|method| method.name().eq_ignore_ascii_case(GENERIC_ALTERNATIVE_COST_NAME))
            .count();
        let generic_additional = self
            .optional_costs
            .iter()
            .filter(|cost| cost.reference.kind == crate::OptionalCostKind::Additional && cost.reference.discriminator.is_none())
            .count();
        match (generic_alternatives, generic_additional) {
            (1, 0) => {
                for method in &mut self.alternative_casts {
                    if let crate::AlternativeCastingMethod::Composed { name, .. }
                    | crate::AlternativeCastingMethod::FromZone { name, .. } = method
                        && name.eq_ignore_ascii_case(GENERIC_ALTERNATIVE_COST_NAME)
                    {
                        *name = keyword.to_string().into();
                        return true;
                    }
                }
                false
            }
            (0, 1) => {
                let cost = self
                    .optional_costs
                    .iter_mut()
                    .find(|cost| cost.reference.kind == crate::OptionalCostKind::Additional)
                    .expect("counted above");
                cost.reference.discriminator = Some(keyword.to_string());
                true
            }
            _ => false,
        }
    }
}

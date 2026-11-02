use crate::tag::TagKeyWalk;

use crate::{AuraAttachmentFilter, Card, CostComponent, ResolutionProgram, TotalCost};

// A missing vocabulary means an empty map, independent of the ability type.
// Explicit default functions prevent serde derive from imposing A: Default.
#[cfg(feature = "serde")]
fn empty_counter_programs<A>() -> std::collections::BTreeMap<String, Vec<A>> {
    std::collections::BTreeMap::new()
}

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
    /// Supplied counter definitions are executable programs, never prose.
    /// They are admitted into a game's counter vocabulary with this definition.
    #[cfg_attr(feature = "serde", serde(default = "empty_counter_programs", skip_serializing_if = "std::collections::BTreeMap::is_empty"))]
    pub ability_counter_definitions: std::collections::BTreeMap<String, Vec<A>>,
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
            ability_counter_definitions: std::collections::BTreeMap::new(),
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
            ability_counter_definitions: std::collections::BTreeMap::new(),
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
            ability_counter_definitions: std::collections::BTreeMap::new(),
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
            ability_counter_definitions: std::collections::BTreeMap::new(),
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

        let mut ability_counter_definitions = std::collections::BTreeMap::new();
        for (kind, program) in self.ability_counter_definitions {
            ability_counter_definitions.insert(kind, program.into_iter().map(&mut map_ability).collect::<Result<Vec<_>, Error>>()?);
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
            ability_counter_definitions,
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

/// Adapter shared by compiler and runtime abilities for custom source-owned prices.
pub trait CustomKeywordPaymentAttribution {
    fn unattributed_text_addition_payments(&self) -> usize;
    fn attribute_text_addition_payment(&mut self, keyword: &str) -> bool;
}
impl<T,E,C,Cond,ICond> CustomKeywordPaymentAttribution for crate::StaticAbility<T,E,C,Cond,ICond> {
    fn unattributed_text_addition_payments(&self)->usize {
        usize::from(matches!(&self.payload,crate::StaticAbilityPayload::Splice(spec) if spec.descriptive && spec.custom_keyword.is_none()))
    }
    fn attribute_text_addition_payment(&mut self,keyword:&str)->bool {
        if let crate::StaticAbilityPayload::Splice(spec)=&mut self.payload
            && spec.descriptive && spec.custom_keyword.is_none() {
            spec.custom_keyword=Some(keyword.to_owned());return true;
        }
        false
    }
}
impl<SA,T,E,C,Cond> CustomKeywordPaymentAttribution for crate::Ability<SA,T,E,C,Cond>
where SA:CustomKeywordPaymentAttribution {
    fn unattributed_text_addition_payments(&self)->usize {
        match &self.kind {crate::AbilityKind::Static(ability)=>ability.unattributed_text_addition_payments(),_=>0}
    }
    fn attribute_text_addition_payment(&mut self,keyword:&str)->bool {
        match &mut self.kind {crate::AbilityKind::Static(ability)=>ability.attribute_text_addition_payment(keyword),_=>false}
    }
}

/// Generic payment shapes produced from a custom keyword's reminder text.
pub const GENERIC_ALTERNATIVE_COST_NAME: &str = "Parsed alternative cost";
/// The described form of a timed exile ("exile it with N progress counters
/// on it"), which a custom keyword such as "Forge 4—{B}{B}" may define.
pub const GENERIC_TIMED_EXILE_NAME: &str = "Parsed timed exile";

fn is_generic_alternative_name(name: &str) -> bool {
    name.eq_ignore_ascii_case(GENERIC_ALTERNATIVE_COST_NAME) || name.eq_ignore_ascii_case(GENERIC_TIMED_EXILE_NAME)
        || name.eq_ignore_ascii_case("Parsed delayed casting preparation")
}

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
    pub fn attribute_custom_keyword_payment(&mut self, keyword: &str) -> bool
    where A:CustomKeywordPaymentAttribution {
        let keyword = keyword.trim();
        let attributed=self.optional_costs.iter().filter(|cost|cost.reference.custom_keyword.as_deref()
            .is_some_and(|name|name.eq_ignore_ascii_case(keyword))).count();
        if attributed==1{return true;}
        let generic_alternatives = self
            .alternative_casts
            .iter()
            .filter(|method| is_generic_alternative_name(method.name()))
            .count();
        let generic_additional = self
            .optional_costs
            .iter()
            .filter(|cost| cost.reference.kind == crate::OptionalCostKind::Additional
                && cost.reference.custom_keyword.is_none()
                && (cost.reference.custom_keyword_attributable || cost.reference.discriminator.is_none()))
            .count();
        let generic_text_additions=self.abilities.iter().map(CustomKeywordPaymentAttribution::unattributed_text_addition_payments).sum::<usize>();
        match (generic_alternatives, generic_additional, generic_text_additions) {
            (1, 0, 0) => {
                for method in &mut self.alternative_casts {
                    if let crate::AlternativeCastingMethod::Composed { name, .. }
                    | crate::AlternativeCastingMethod::FromZone { name, .. } = method
                        && is_generic_alternative_name(name)
                    {
                        *name = keyword.to_string().into();
                        return true;
                    }
                }
                false
            }
            (0, 1, 0) => {
                let cost = self
                    .optional_costs
                    .iter_mut()
                    .find(|cost| cost.reference.kind == crate::OptionalCostKind::Additional
                        && cost.reference.custom_keyword.is_none()
                        && (cost.reference.custom_keyword_attributable || cost.reference.discriminator.is_none()))
                    .expect("counted above");
                cost.reference.custom_keyword = Some(keyword.to_string());
                // Preserve the legacy unscoped payment surface. A scoped
                // trigger still refers to its exact original discriminator.
                if cost.reference.discriminator.is_none() {
                    cost.reference.discriminator = Some(keyword.to_string());
                }
                true
            }
            (0, 0, 1) => self.abilities.iter_mut().any(|ability|ability.attribute_text_addition_payment(keyword)),
            _ => false,
        }
    }
}

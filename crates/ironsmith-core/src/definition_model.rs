use crate::{AuraAttachmentFilter, Card, CostComponent, ResolutionProgram, TotalCost};

#[derive(Debug, Clone, PartialEq)]
pub struct CardDefinition<A, E, C, AC, OC> {
    pub card: Card,
    pub abilities: Vec<A>,
    pub spell_effect: Option<ResolutionProgram<E>>,
    pub aura_attach_filter: Option<AuraAttachmentFilter>,
    pub alternative_casts: Vec<AC>,
    pub has_fuse: bool,
    pub optional_costs: Vec<OC>,
    pub max_saga_chapter: Option<u32>,
    pub additional_cost: TotalCost<C>,
}

impl<A, E, C, AC, OC> CardDefinition<A, E, C, AC, OC>
where
    C: CostComponent,
{
    pub fn new(card: Card) -> Self {
        Self {
            card,
            abilities: Vec::new(),
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            has_fuse: false,
            optional_costs: Vec::new(),
            max_saga_chapter: None,
            additional_cost: TotalCost::free(),
        }
    }

    pub fn with_abilities(card: Card, abilities: Vec<A>) -> Self {
        Self {
            card,
            abilities,
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            has_fuse: false,
            optional_costs: Vec::new(),
            max_saga_chapter: None,
            additional_cost: TotalCost::free(),
        }
    }

    pub fn spell(card: Card, effects: Vec<E>) -> Self
    where
        E: Clone,
    {
        Self {
            card,
            abilities: Vec::new(),
            spell_effect: Some(ResolutionProgram::from_effects(effects)),
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            has_fuse: false,
            optional_costs: Vec::new(),
            max_saga_chapter: None,
            additional_cost: TotalCost::free(),
        }
    }

    pub fn spell_with_abilities(card: Card, effects: Vec<E>, abilities: Vec<A>) -> Self
    where
        E: Clone,
    {
        Self {
            card,
            abilities,
            spell_effect: Some(ResolutionProgram::from_effects(effects)),
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            has_fuse: false,
            optional_costs: Vec::new(),
            max_saga_chapter: None,
            additional_cost: TotalCost::free(),
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
}

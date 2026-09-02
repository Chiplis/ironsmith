use ironsmith::ability::{Ability, LevelAbility};
use ironsmith::card::{LinkedFaceLayout, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::cost::OptionalCost;
use ironsmith::effect::Effect;
use ironsmith::ids::CardId;
use ironsmith::mana::ManaCost;
use ironsmith::object::AuraAttachmentFilter;
use ironsmith::types::{CardType, Subtype, Supertype};
use ironsmith_compiler::CardDefinitionBuilder as CompilerCardDefinitionBuilder;
use ironsmith_compiler::{KeywordAction, MetadataLine};

pub type CardTextError = crate::CompilerIntegrationError;
pub type ParseAnnotations = ironsmith_compiler::ParseAnnotations;
pub type TextSpan = ironsmith_compiler::TextSpan;

#[derive(Debug, Clone)]
pub struct CardDefinitionBuilder {
    runtime: ironsmith::cards::builders::CardDefinitionBuilder,
    compiler: CompilerCardDefinitionBuilder,
}

impl CardDefinitionBuilder {
    pub fn new(id: CardId, name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            runtime: ironsmith::cards::builders::CardDefinitionBuilder::new(id, name.clone()),
            compiler: CompilerCardDefinitionBuilder::new(id, name),
        }
    }

    pub fn mana_cost(mut self, cost: ManaCost) -> Self {
        self.runtime = self.runtime.mana_cost(cost.clone());
        self.compiler = self.compiler.mana_cost(cost);
        self
    }

    pub fn color_indicator(mut self, colors: ColorSet) -> Self {
        self.runtime = self.runtime.color_indicator(colors);
        self.compiler = self.compiler.color_indicator(colors);
        self
    }

    pub fn supertypes(mut self, supertypes: Vec<Supertype>) -> Self {
        self.runtime = self.runtime.supertypes(supertypes.clone());
        self.compiler = self.compiler.supertypes(supertypes);
        self
    }

    pub fn card_types(mut self, types: Vec<CardType>) -> Self {
        self.runtime = self.runtime.card_types(types.clone());
        self.compiler = self.compiler.card_types(types);
        self
    }

    pub fn subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.runtime = self.runtime.subtypes(subtypes.clone());
        self.compiler = self.compiler.subtypes(subtypes);
        self
    }

    pub fn oracle_text(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.runtime = self.runtime.oracle_text(text.clone());
        self.compiler = self.compiler.oracle_text(text);
        self
    }

    pub fn other_face(mut self, face: CardId) -> Self {
        self.runtime = self.runtime.other_face(face);
        self.compiler = self.compiler.other_face(face);
        self
    }

    pub fn other_face_name(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.runtime = self.runtime.other_face_name(name.clone());
        self.compiler = self.compiler.other_face_name(name);
        self
    }

    pub fn linked_face_layout(mut self, layout: LinkedFaceLayout) -> Self {
        self.runtime = self.runtime.linked_face_layout(layout);
        self.compiler = self.compiler.linked_face_layout(layout);
        self
    }

    pub fn has_fuse(mut self) -> Self {
        self.runtime = self.runtime.has_fuse();
        self.compiler = self.compiler.has_fuse();
        self
    }

    pub fn power_toughness(mut self, pt: PowerToughness) -> Self {
        self.runtime = self.runtime.power_toughness(pt);
        self.compiler = self.compiler.power_toughness(pt);
        self
    }

    pub fn loyalty(mut self, loyalty: u32) -> Self {
        self.runtime = self.runtime.loyalty(loyalty);
        self.compiler = self.compiler.loyalty(loyalty);
        self
    }

    pub fn defense(mut self, defense: u32) -> Self {
        self.runtime = self.runtime.defense(defense);
        self.compiler = self.compiler.defense(defense);
        self
    }

    pub fn token(mut self) -> Self {
        self.runtime = self.runtime.token();
        self.compiler = self.compiler.token();
        self
    }

    pub fn enchants(mut self, filter: impl Into<AuraAttachmentFilter>) -> Self {
        let filter = filter.into();
        self.runtime = self.runtime.enchants(filter.clone());
        self.compiler = self.compiler.enchants(filter);
        self
    }

    pub fn with_abilities(mut self, abilities: Vec<Ability>) -> Self {
        self.runtime = self.runtime.with_abilities(abilities);
        self
    }

    pub fn with_ability(mut self, ability: Ability) -> Self {
        self.runtime = self.runtime.with_ability(ability);
        self
    }

    pub fn with_spell_effect(mut self, effects: Vec<Effect>) -> Self {
        self.runtime = self.runtime.with_spell_effect(effects);
        self
    }

    pub fn with_level_abilities(mut self, abilities: Vec<LevelAbility>) -> Self {
        self.runtime = self.runtime.with_level_abilities(abilities);
        self
    }

    pub fn optional_cost(mut self, cost: OptionalCost) -> Self {
        self.runtime = self.runtime.optional_cost(cost);
        self
    }

    pub fn apply_keyword_action(mut self, action: KeywordAction) -> Self {
        self.compiler = self.compiler.apply_keyword_action(action);
        self
    }

    pub fn apply_metadata(mut self, meta: impl Into<MetadataLine>) -> Result<Self, CardTextError> {
        let face = ironsmith_compiler::card_metadata::apply_metadata_line(
            self.compiler.card_builder.clone(),
            meta,
        )
        .map_err(CardTextError::from)?;
        self.compiler = self.compiler.with_face(face);
        Ok(self)
    }

    pub fn flying(mut self) -> Self {
        self.runtime = self.runtime.flying();
        self
    }
    pub fn defender(mut self) -> Self {
        self.runtime = self.runtime.defender();
        self
    }
    pub fn vigilance(mut self) -> Self {
        self.runtime = self.runtime.vigilance();
        self
    }
    pub fn prowess(mut self) -> Self {
        self.runtime = self.runtime.prowess();
        self
    }
    pub fn trample(mut self) -> Self {
        self.runtime = self.runtime.trample();
        self
    }
    pub fn lifelink(mut self) -> Self {
        self.runtime = self.runtime.lifelink();
        self
    }
    pub fn deathtouch(mut self) -> Self {
        self.runtime = self.runtime.deathtouch();
        self
    }
    pub fn haste(mut self) -> Self {
        self.runtime = self.runtime.haste();
        self
    }
    pub fn menace(mut self) -> Self {
        self.runtime = self.runtime.menace();
        self
    }
    pub fn reach(mut self) -> Self {
        self.runtime = self.runtime.reach();
        self
    }
    pub fn hexproof(mut self) -> Self {
        self.runtime = self.runtime.hexproof();
        self
    }
    pub fn indestructible(mut self) -> Self {
        self.runtime = self.runtime.indestructible();
        self
    }
    pub fn toxic(mut self, amount: u32) -> Self {
        self.runtime = self.runtime.toxic(amount);
        self
    }
    pub fn first_strike(mut self) -> Self {
        self.runtime = self.runtime.first_strike();
        self
    }
    pub fn double_strike(mut self) -> Self {
        self.runtime = self.runtime.double_strike();
        self
    }

    pub fn build(self) -> CardDefinition {
        self.runtime.build()
    }

    pub fn parse_text(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError> {
        crate::compile_builder_to_runtime_definition(self.compiler, text, false)
    }

    pub fn parse_text_allow_unsupported(
        self,
        text: impl Into<String>,
    ) -> Result<CardDefinition, CardTextError> {
        crate::compile_builder_to_runtime_definition(self.compiler, text, true)
    }
}

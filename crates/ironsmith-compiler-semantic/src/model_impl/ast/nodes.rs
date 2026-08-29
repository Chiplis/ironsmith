use crate::ability::ActivationTiming;
use crate::model::provenance::{ProvenanceStore, SemanticProvenance, SourceUnitId};
use crate::model::symbols::SymbolTable;
use crate::zone::Zone;

/// Canonical compiler document. Runtime builders and materialized abilities
/// are deliberately absent: lowering consumes this tree and produces them.
#[derive(Debug, Clone)]
pub struct CompilerDocument<Item> {
    pub source: SourceUnitId,
    pub items: Vec<Item>,
    pub provenance: ProvenanceStore,
    pub symbols: SymbolTable,
    pub allow_unsupported: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompilerDocumentItem<Line, Ability, Modal, Level> {
    Line(Line),
    Ability(Ability),
    Modal(Modal),
    Level(Level),
}

/// Compiler-owned ability shell. Domain PRs replace the generic components
/// with the canonical cost, trigger, static, and effect nodes without changing
/// document ownership again.
#[derive(Debug, Clone, PartialEq)]
pub struct CompilerAbility<Static, Triggered, Activated> {
    pub kind: CompilerAbilityKind<Static, Triggered, Activated>,
    pub functional_zones: Vec<Zone>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompilerAbilityKind<Static, Triggered, Activated> {
    Static(Static),
    Triggered(Triggered),
    Activated(Activated),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerTriggeredAbility<Trigger, Effect, Choice, Condition, Presentation> {
    pub trigger: Trigger,
    pub effects: Vec<Effect>,
    pub choices: Vec<Choice>,
    pub intervening_if: Option<Condition>,
    pub presentation: Option<Presentation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerActivatedAbility<Cost, Effect, Choice, Condition, Mana, ManaRestriction> {
    pub cost: Cost,
    pub effects: Vec<Effect>,
    pub choices: Vec<Choice>,
    pub timing: ActivationTiming,
    pub is_loyalty_ability: bool,
    pub restrictions: Vec<Condition>,
    pub mana_output: Option<Vec<Mana>>,
    pub mana_restrictions: Vec<ManaRestriction>,
}

/// Transitional compiler-owned parser payload. It keeps a structural clone of
/// legacy ability state while parser families move to the canonical ability
/// enums in PR-09 through PR-12. The wrapped type is generic and cannot be
/// accessed by front-end code; materialization owns extraction.
#[derive(Debug, Clone, PartialEq)]
pub struct CompilerAbilityPayload<LegacyAbility, Effect, ReferenceImports, Trigger> {
    legacy: LegacyAbility,
    pub effects: Option<Vec<Effect>>,
    pub reference_imports: ReferenceImports,
    pub trigger: Option<Trigger>,
}

impl<LegacyAbility, Effect, ReferenceImports, Trigger>
    CompilerAbilityPayload<LegacyAbility, Effect, ReferenceImports, Trigger>
{
    pub fn from_legacy(
        legacy: LegacyAbility,
        effects: Option<Vec<Effect>>,
        reference_imports: ReferenceImports,
        trigger: Option<Trigger>,
    ) -> Self {
        Self {
            legacy,
            effects,
            reference_imports,
            trigger,
        }
    }

    pub fn legacy(&self) -> &LegacyAbility {
        &self.legacy
    }

    pub fn legacy_mut(&mut self) -> &mut LegacyAbility {
        &mut self.legacy
    }

    pub fn into_legacy(self) -> LegacyAbility {
        self.legacy
    }
}

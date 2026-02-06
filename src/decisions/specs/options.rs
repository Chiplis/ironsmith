//! Option selection decision specifications.
//!
//! These specs are for decisions where the player selects from indexed options,
//! such as modal spell modes, generic choices, priority actions, etc.

use crate::alternative_cast::CastingMethod;
use crate::decision::{FallbackStrategy, LegalAction};
use crate::decisions::context::{DecisionContext, SelectOptionsContext, SelectableOption};
use crate::decisions::spec::{DecisionPrimitive, DecisionSpec, DisplayOption};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::zone::Zone;

// ============================================================================
// ModesSpec - Modal spell mode selection
// ============================================================================

/// A mode option for a modal spell/ability.
#[derive(Debug, Clone)]
pub struct ModeOption {
    /// Index of this mode.
    pub index: usize,
    /// Description of what this mode does.
    pub description: String,
    /// Whether this mode is currently legal to choose.
    pub legal: bool,
}

impl ModeOption {
    /// Create a new legal mode option.
    pub fn new(index: usize, description: impl Into<String>) -> Self {
        Self {
            index,
            description: description.into(),
            legal: true,
        }
    }

    /// Create a mode option with explicit legality.
    pub fn with_legality(index: usize, description: impl Into<String>, legal: bool) -> Self {
        Self {
            index,
            description: description.into(),
            legal,
        }
    }
}

/// Specification for choosing mode(s) for a modal spell/ability.
#[derive(Debug, Clone)]
pub struct ModesSpec {
    /// The source spell or ability.
    pub source: ObjectId,
    /// Available modes.
    pub modes: Vec<ModeOption>,
    /// Minimum number of modes to choose.
    pub min_modes: usize,
    /// Maximum number of modes to choose.
    pub max_modes: usize,
}

impl ModesSpec {
    /// Create a new ModesSpec.
    pub fn new(
        source: ObjectId,
        modes: Vec<ModeOption>,
        min_modes: usize,
        max_modes: usize,
    ) -> Self {
        Self {
            source,
            modes,
            min_modes,
            max_modes,
        }
    }

    /// Create a spec for choosing exactly one mode.
    pub fn single(source: ObjectId, modes: Vec<ModeOption>) -> Self {
        Self::new(source, modes, 1, 1)
    }

    /// Create a spec for "choose one or more" modes.
    pub fn one_or_more(source: ObjectId, modes: Vec<ModeOption>) -> Self {
        let max = modes.len();
        Self::new(source, modes, 1, max)
    }
}

impl DecisionSpec for ModesSpec {
    type Response = Vec<usize>;

    fn description(&self) -> String {
        if self.min_modes == self.max_modes {
            if self.min_modes == 1 {
                "Choose a mode".to_string()
            } else {
                format!("Choose {} modes", self.min_modes)
            }
        } else {
            format!("Choose {}-{} modes", self.min_modes, self.max_modes)
        }
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectOptions {
            min: self.min_modes,
            max: self.max_modes,
        }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> Vec<usize> {
        // Default: choose first legal mode(s)
        self.modes
            .iter()
            .filter(|m| m.legal)
            .take(self.min_modes)
            .map(|m| m.index)
            .collect()
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        let options: Vec<SelectableOption> = self
            .modes
            .iter()
            .map(|m| SelectableOption::with_legality(m.index, m.description.clone(), m.legal))
            .collect();

        DecisionContext::SelectOptions(SelectOptionsContext::new(
            player,
            Some(self.source),
            self.description(),
            options,
            self.min_modes,
            self.max_modes,
        ))
    }
}

// ============================================================================
// ChoiceSpec - Generic choice from options
// ============================================================================

/// Specification for a generic choice from options.
#[derive(Debug, Clone)]
pub struct ChoiceSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// Available options.
    pub options: Vec<DisplayOption>,
    /// Minimum choices required.
    pub min_choices: usize,
    /// Maximum choices allowed.
    pub max_choices: usize,
}

impl ChoiceSpec {
    /// Create a new ChoiceSpec.
    pub fn new(
        source: ObjectId,
        options: Vec<DisplayOption>,
        min_choices: usize,
        max_choices: usize,
    ) -> Self {
        Self {
            source,
            options,
            min_choices,
            max_choices,
        }
    }

    /// Create a spec for choosing exactly one option.
    pub fn single(source: ObjectId, options: Vec<DisplayOption>) -> Self {
        Self::new(source, options, 1, 1)
    }
}

impl DecisionSpec for ChoiceSpec {
    type Response = Vec<usize>;

    fn description(&self) -> String {
        if self.min_choices == 1 && self.max_choices == 1 {
            "Choose one".to_string()
        } else {
            format!("Choose {}-{}", self.min_choices, self.max_choices)
        }
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectOptions {
            min: self.min_choices,
            max: self.max_choices,
        }
    }

    fn default_response(&self, strategy: FallbackStrategy) -> Vec<usize> {
        let take_count = match strategy {
            FallbackStrategy::Maximum => self.max_choices,
            _ => self.min_choices,
        };
        self.options
            .iter()
            .filter(|o| o.legal)
            .take(take_count)
            .map(|o| o.index)
            .collect()
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        let options: Vec<SelectableOption> = self
            .options
            .iter()
            .map(|o| SelectableOption::with_legality(o.index, &o.description, o.legal))
            .collect();

        let description = if self.min_choices == 1 && self.max_choices == 1 {
            "Choose one".to_string()
        } else {
            format!("Choose {}-{}", self.min_choices, self.max_choices)
        };

        DecisionContext::SelectOptions(SelectOptionsContext::new(
            player,
            Some(self.source),
            description,
            options,
            self.min_choices,
            self.max_choices,
        ))
    }
}

// ============================================================================
// ReplacementSpec - Choose replacement effect to apply
// ============================================================================

/// A replacement effect option.
#[derive(Debug, Clone)]
pub struct ReplacementOption {
    /// Index of this option.
    pub index: usize,
    /// Source of the replacement effect.
    pub source: ObjectId,
    /// Description of what this replacement does.
    pub description: String,
}

impl ReplacementOption {
    /// Create a new replacement option.
    pub fn new(index: usize, source: ObjectId, description: impl Into<String>) -> Self {
        Self {
            index,
            source,
            description: description.into(),
        }
    }
}

/// Specification for choosing a replacement effect to apply.
#[derive(Debug, Clone)]
pub struct ReplacementSpec {
    /// The replacement effect options.
    pub options: Vec<ReplacementOption>,
}

impl ReplacementSpec {
    /// Create a new ReplacementSpec.
    pub fn new(options: Vec<ReplacementOption>) -> Self {
        Self { options }
    }
}

impl DecisionSpec for ReplacementSpec {
    type Response = usize;

    fn description(&self) -> String {
        "Choose which replacement effect to apply".to_string()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectOptions { min: 1, max: 1 }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> usize {
        0 // Default to first replacement effect
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        let options: Vec<SelectableOption> = self
            .options
            .iter()
            .map(|o| SelectableOption::new(o.index, &o.description))
            .collect();

        DecisionContext::SelectOptions(SelectOptionsContext::new(
            player,
            None,
            "Choose which replacement effect to apply",
            options,
            1,
            1,
        ))
    }
}

// ============================================================================
// OptionalCostsSpec - Choose optional costs to pay (kicker, etc.)
// ============================================================================

/// An optional cost option.
#[derive(Debug, Clone)]
pub struct OptionalCostOption {
    /// Index of this optional cost.
    pub index: usize,
    /// Label for this cost (e.g., "Kicker", "Buyback").
    pub label: String,
    /// Whether this cost can be paid multiple times (multikicker).
    pub repeatable: bool,
    /// Whether the player can currently afford this cost.
    pub affordable: bool,
    /// Description of the cost to pay.
    pub cost_description: String,
}

impl OptionalCostOption {
    /// Create a new optional cost option.
    pub fn new(
        index: usize,
        label: impl Into<String>,
        repeatable: bool,
        affordable: bool,
        cost_description: impl Into<String>,
    ) -> Self {
        Self {
            index,
            label: label.into(),
            repeatable,
            affordable,
            cost_description: cost_description.into(),
        }
    }
}

/// Specification for choosing optional costs to pay.
#[derive(Debug, Clone)]
pub struct OptionalCostsSpec {
    /// The source spell.
    pub source: ObjectId,
    /// Available optional costs.
    pub options: Vec<OptionalCostOption>,
}

impl OptionalCostsSpec {
    /// Create a new OptionalCostsSpec.
    pub fn new(source: ObjectId, options: Vec<OptionalCostOption>) -> Self {
        Self { source, options }
    }
}

/// Response for optional costs: (cost_index, times_paid) pairs.
pub type OptionalCostsResponse = Vec<(usize, u32)>;

impl DecisionSpec for OptionalCostsSpec {
    type Response = OptionalCostsResponse;

    fn description(&self) -> String {
        "Choose optional costs to pay".to_string()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectOptions {
            min: 0,
            max: self.options.len(),
        }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> OptionalCostsResponse {
        // Default: don't pay any optional costs
        Vec::new()
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        let options: Vec<SelectableOption> = self
            .options
            .iter()
            .map(|o| {
                let description = format!("{}: {}", o.label, o.cost_description);
                SelectableOption::with_legality(o.index, description, o.affordable)
            })
            .collect();

        DecisionContext::SelectOptions(SelectOptionsContext::new(
            player,
            Some(self.source),
            "Choose optional costs to pay",
            options,
            0,
            self.options.len(),
        ))
    }
}

// ============================================================================
// CastingMethodSpec - Choose how to cast a spell
// ============================================================================

/// A casting method option.
#[derive(Debug, Clone)]
pub struct CastingMethodOption {
    /// The casting method.
    pub method: CastingMethod,
    /// Display name for this method.
    pub name: String,
    /// Description of the cost.
    pub cost_description: String,
}

impl CastingMethodOption {
    /// Create a new casting method option.
    pub fn new(
        method: CastingMethod,
        name: impl Into<String>,
        cost_description: impl Into<String>,
    ) -> Self {
        Self {
            method,
            name: name.into(),
            cost_description: cost_description.into(),
        }
    }
}

/// Specification for choosing how to cast a spell.
#[derive(Debug, Clone)]
pub struct CastingMethodSpec {
    /// The spell being cast.
    pub spell_id: ObjectId,
    /// The zone the spell is being cast from.
    pub from_zone: Zone,
    /// Available casting methods.
    pub options: Vec<CastingMethodOption>,
}

impl CastingMethodSpec {
    /// Create a new CastingMethodSpec.
    pub fn new(spell_id: ObjectId, from_zone: Zone, options: Vec<CastingMethodOption>) -> Self {
        Self {
            spell_id,
            from_zone,
            options,
        }
    }
}

impl DecisionSpec for CastingMethodSpec {
    type Response = usize;

    fn description(&self) -> String {
        "Choose how to cast this spell".to_string()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectOptions { min: 1, max: 1 }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> usize {
        0 // Default to first (usually Normal) casting method
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        let options: Vec<SelectableOption> = self
            .options
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let description = format!("{} ({})", o.name, o.cost_description);
                SelectableOption::new(i, description)
            })
            .collect();

        DecisionContext::SelectOptions(SelectOptionsContext::new(
            player,
            Some(self.spell_id),
            "Choose how to cast this spell",
            options,
            1,
            1,
        ))
    }
}

// ============================================================================
// ManaPaymentSpec - Choose mana payment option
// ============================================================================

/// A mana payment option.
#[derive(Debug, Clone)]
pub struct ManaPaymentOption {
    /// Index of this option.
    pub index: usize,
    /// Description of this payment method.
    pub description: String,
}

impl ManaPaymentOption {
    /// Create a new mana payment option.
    pub fn new(index: usize, description: impl Into<String>) -> Self {
        Self {
            index,
            description: description.into(),
        }
    }
}

/// Specification for choosing a mana payment option.
#[derive(Debug, Clone)]
pub struct ManaPaymentSpec {
    /// The source being paid for.
    pub source: ObjectId,
    /// Description of what is being paid for.
    pub context: String,
    /// Payment options.
    pub options: Vec<ManaPaymentOption>,
}

impl ManaPaymentSpec {
    /// Create a new ManaPaymentSpec.
    pub fn new(
        source: ObjectId,
        context: impl Into<String>,
        options: Vec<ManaPaymentOption>,
    ) -> Self {
        Self {
            source,
            context: context.into(),
            options,
        }
    }
}

impl DecisionSpec for ManaPaymentSpec {
    type Response = usize;

    fn description(&self) -> String {
        format!("Choose mana payment for {}", self.context)
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectOptions { min: 1, max: 1 }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> usize {
        0 // Default to first payment option
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        let options: Vec<SelectableOption> = self
            .options
            .iter()
            .map(|o| SelectableOption::new(o.index, &o.description))
            .collect();

        DecisionContext::SelectOptions(SelectOptionsContext::new(
            player,
            Some(self.source),
            format!("Choose mana payment for {}", self.context),
            options,
            1,
            1,
        ))
    }
}

// ============================================================================
// PrioritySpec - Choose priority action
// ============================================================================

/// Specification for priority decisions.
#[derive(Debug, Clone)]
pub struct PrioritySpec {
    /// Legal actions available.
    pub legal_actions: Vec<LegalAction>,
    /// Commander actions available (accessed via 'C').
    pub commander_actions: Vec<LegalAction>,
}

impl PrioritySpec {
    /// Create a new PrioritySpec.
    pub fn new(legal_actions: Vec<LegalAction>, commander_actions: Vec<LegalAction>) -> Self {
        Self {
            legal_actions,
            commander_actions,
        }
    }
}

impl DecisionSpec for PrioritySpec {
    type Response = LegalAction;

    fn description(&self) -> String {
        "Choose an action".to_string()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectOptions { min: 1, max: 1 }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> LegalAction {
        LegalAction::PassPriority
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        let options: Vec<SelectableOption> = self
            .legal_actions
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let description = format!("{:?}", action);
                SelectableOption::new(i, description)
            })
            .collect();

        DecisionContext::SelectOptions(SelectOptionsContext::new(
            player,
            None,
            "Choose an action",
            options,
            1,
            1,
        ))
    }
}

// ============================================================================
// DiscardDestinationSpec - Choose where discarded card goes
// ============================================================================

/// Specification for choosing discard destination (Library of Leng interaction).
#[derive(Debug, Clone)]
pub struct DiscardDestinationSpec {
    /// The card being discarded.
    pub card: ObjectId,
    /// Available destination zones.
    pub options: Vec<Zone>,
    /// Description of the choice.
    pub description: String,
}

impl DiscardDestinationSpec {
    /// Create a new DiscardDestinationSpec.
    pub fn new(card: ObjectId, options: Vec<Zone>, description: impl Into<String>) -> Self {
        Self {
            card,
            options,
            description: description.into(),
        }
    }
}

impl DecisionSpec for DiscardDestinationSpec {
    type Response = Zone;

    fn description(&self) -> String {
        self.description.clone()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectOptions { min: 1, max: 1 }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> Zone {
        // Default to graveyard if available, otherwise first option
        if self.options.contains(&Zone::Graveyard) {
            Zone::Graveyard
        } else {
            self.options.first().copied().unwrap_or(Zone::Graveyard)
        }
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        let options: Vec<SelectableOption> = self
            .options
            .iter()
            .enumerate()
            .map(|(i, zone)| SelectableOption::new(i, format!("{:?}", zone)))
            .collect();

        DecisionContext::SelectOptions(SelectOptionsContext::new(
            player,
            Some(self.card),
            self.description.clone(),
            options,
            1,
            1,
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modes_spec_single() {
        let source = ObjectId::from_raw(1);
        let modes = vec![
            ModeOption::new(0, "Counter target spell"),
            ModeOption::new(1, "Draw a card"),
        ];
        let spec = ModesSpec::single(source, modes);

        assert_eq!(spec.description(), "Choose a mode");
        assert!(matches!(
            spec.primitive(),
            DecisionPrimitive::SelectOptions { min: 1, max: 1 }
        ));
    }

    #[test]
    fn test_choice_spec() {
        let source = ObjectId::from_raw(1);
        let options = vec![
            DisplayOption::new(0, "Option A"),
            DisplayOption::new(1, "Option B"),
        ];
        let spec = ChoiceSpec::single(source, options);

        assert_eq!(spec.description(), "Choose one");
        assert_eq!(
            spec.default_response(FallbackStrategy::FirstOption),
            vec![0]
        );
    }

    #[test]
    fn test_optional_costs_spec() {
        let source = ObjectId::from_raw(1);
        let options = vec![OptionalCostOption::new(0, "Kicker", false, true, "{2}")];
        let spec = OptionalCostsSpec::new(source, options);

        // Default is to not pay optional costs
        assert!(spec.default_response(FallbackStrategy::Decline).is_empty());
    }

    #[test]
    fn test_priority_spec() {
        let spec = PrioritySpec::new(vec![LegalAction::PassPriority], vec![]);

        assert!(matches!(
            spec.default_response(FallbackStrategy::Decline),
            LegalAction::PassPriority
        ));
    }

    #[test]
    fn test_discard_destination_spec() {
        let card = ObjectId::from_raw(1);
        let options = vec![Zone::Graveyard, Zone::Library];
        let spec = DiscardDestinationSpec::new(card, options, "Choose where card goes");

        assert_eq!(
            spec.default_response(FallbackStrategy::FirstOption),
            Zone::Graveyard
        );
    }
}

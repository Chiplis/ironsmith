//! Special decision specifications.
//!
//! These specs are for specialized decisions that don't fit neatly into
//! the other categories: scry, surveil, distribute, mana colors, counter removal.

use crate::color::Color;
use crate::decision::FallbackStrategy;
use crate::decisions::context::{
    ColorsContext, CountersContext, DecisionContext, DistributeContext, DistributeTarget,
    PartitionContext, SelectOptionsContext, SelectableOption,
};
use crate::decisions::spec::{DecisionPrimitive, DecisionSpec};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;

// ============================================================================
// ScrySpec - Scry N cards
// ============================================================================

/// Specification for scrying cards.
/// Player looks at top N cards and puts any number on bottom in any order.
#[derive(Debug, Clone)]
pub struct ScrySpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// The cards being looked at (top of library).
    pub cards: Vec<ObjectId>,
}

impl ScrySpec {
    /// Create a new ScrySpec.
    pub fn new(source: ObjectId, cards: Vec<ObjectId>) -> Self {
        Self { source, cards }
    }
}

/// Response for scry: cards to put on bottom (in order).
/// Cards not in this list stay on top.
pub type ScryResponse = Vec<ObjectId>;

impl DecisionSpec for ScrySpec {
    type Response = ScryResponse;

    fn description(&self) -> String {
        format!("Scry {} card(s)", self.cards.len())
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::Partition
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> ScryResponse {
        // Default: keep all cards on top
        Vec::new()
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let cards: Vec<(ObjectId, String)> = self
            .cards
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                (id, name)
            })
            .collect();

        DecisionContext::Partition(PartitionContext::scry(player, Some(self.source), cards))
    }
}

// ============================================================================
// SurveilSpec - Surveil N cards
// ============================================================================

/// Specification for surveilling cards.
/// Player looks at top N cards and puts any number in graveyard.
#[derive(Debug, Clone)]
pub struct SurveilSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// The cards being looked at (top of library).
    pub cards: Vec<ObjectId>,
}

impl SurveilSpec {
    /// Create a new SurveilSpec.
    pub fn new(source: ObjectId, cards: Vec<ObjectId>) -> Self {
        Self { source, cards }
    }
}

/// Response for surveil: cards to put in graveyard.
/// Cards not in this list stay on top of library.
pub type SurveilResponse = Vec<ObjectId>;

impl DecisionSpec for SurveilSpec {
    type Response = SurveilResponse;

    fn description(&self) -> String {
        format!("Surveil {} card(s)", self.cards.len())
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::Partition
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> SurveilResponse {
        // Default: keep all cards on top of library
        Vec::new()
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let cards: Vec<(ObjectId, String)> = self
            .cards
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                (id, name)
            })
            .collect();

        DecisionContext::Partition(PartitionContext::surveil(player, Some(self.source), cards))
    }
}

// ============================================================================
// DistributeSpec - Distribute amount among targets
// ============================================================================

/// Specification for distributing an amount among targets.
/// Used for damage distribution, counter distribution, etc.
#[derive(Debug, Clone)]
pub struct DistributeSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// Total amount to distribute.
    pub total: u32,
    /// Valid targets to distribute among.
    pub targets: Vec<Target>,
    /// Minimum amount per target (usually 1 for damage).
    pub min_per_target: u32,
}

impl DistributeSpec {
    /// Create a new DistributeSpec.
    pub fn new(source: ObjectId, total: u32, targets: Vec<Target>, min_per_target: u32) -> Self {
        Self {
            source,
            total,
            targets,
            min_per_target,
        }
    }

    /// Create a spec for distributing damage (min 1 per target).
    pub fn damage(source: ObjectId, total: u32, targets: Vec<Target>) -> Self {
        Self::new(source, total, targets, 1)
    }

    /// Create a spec for distributing counters (can put 0 on some targets).
    pub fn counters(source: ObjectId, total: u32, targets: Vec<Target>) -> Self {
        Self::new(source, total, targets, 0)
    }
}

/// Response for distribute: (target, amount) pairs.
pub type DistributeResponse = Vec<(Target, u32)>;

impl DecisionSpec for DistributeSpec {
    type Response = DistributeResponse;

    fn description(&self) -> String {
        format!("Distribute {} among targets", self.total)
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::Distribute {
            total: self.total,
            min_per_target: self.min_per_target,
        }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> DistributeResponse {
        // Default: put all on first target
        if let Some(first) = self.targets.first() {
            vec![(*first, self.total)]
        } else {
            Vec::new()
        }
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let targets: Vec<DistributeTarget> = self
            .targets
            .iter()
            .map(|target| {
                let name = match target {
                    Target::Object(id) => game
                        .object(*id)
                        .map(|o| o.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    Target::Player(pid) => format!("Player {}", pid.index() + 1),
                };
                DistributeTarget {
                    target: *target,
                    name,
                }
            })
            .collect();

        DecisionContext::Distribute(DistributeContext::new(
            player,
            Some(self.source),
            format!("Distribute {} among targets", self.total),
            self.total,
            targets,
            self.min_per_target,
        ))
    }
}

// ============================================================================
// ManaColorsSpec - Choose mana colors
// ============================================================================

/// Specification for choosing mana colors.
/// Used by abilities like Birds of Paradise, City of Brass, etc.
#[derive(Debug, Clone)]
pub struct ManaColorsSpec {
    /// The source of the ability.
    pub source: ObjectId,
    /// Number of mana to add.
    pub count: u32,
    /// If true, all mana must be the same color.
    pub same_color: bool,
    /// Available colors (None = all five colors).
    pub available_colors: Option<Vec<Color>>,
}

impl ManaColorsSpec {
    /// Create a new ManaColorsSpec for any color.
    pub fn any_color(source: ObjectId, count: u32, same_color: bool) -> Self {
        Self {
            source,
            count,
            same_color,
            available_colors: None,
        }
    }

    /// Create a new ManaColorsSpec with restricted colors.
    pub fn restricted(
        source: ObjectId,
        count: u32,
        same_color: bool,
        available_colors: Vec<Color>,
    ) -> Self {
        Self {
            source,
            count,
            same_color,
            available_colors: Some(available_colors),
        }
    }
}

impl DecisionSpec for ManaColorsSpec {
    type Response = Vec<Color>;

    fn description(&self) -> String {
        if self.same_color {
            format!("Choose a color for {} mana", self.count)
        } else {
            format!("Choose {} mana color(s)", self.count)
        }
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectColors {
            count: self.count,
            same_color: self.same_color,
        }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> Vec<Color> {
        // Default: green (arbitrary but consistent)
        let default_color = self
            .available_colors
            .as_ref()
            .and_then(|colors| colors.first().copied())
            .unwrap_or(Color::Green);
        vec![default_color; self.count as usize]
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        _game: &GameState,
    ) -> DecisionContext {
        let ctx = if let Some(colors) = &self.available_colors {
            ColorsContext::restricted(
                player,
                Some(self.source),
                self.count,
                self.same_color,
                colors.clone(),
            )
        } else {
            ColorsContext::any_color(player, Some(self.source), self.count, self.same_color)
        };

        DecisionContext::Colors(ctx)
    }
}

// ============================================================================
// CounterRemovalSpec - Choose counters to remove
// ============================================================================

/// Specification for choosing counters to remove from a permanent.
/// Used by effects like Hex Parasite.
#[derive(Debug, Clone)]
pub struct CounterRemovalSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// The permanent to remove counters from.
    pub target: ObjectId,
    /// Maximum total counters that can be removed.
    pub max_total: u32,
    /// Available counters: (counter_type, count_available).
    pub available_counters: Vec<(CounterType, u32)>,
}

impl CounterRemovalSpec {
    /// Create a new CounterRemovalSpec.
    pub fn new(
        source: ObjectId,
        target: ObjectId,
        max_total: u32,
        available_counters: Vec<(CounterType, u32)>,
    ) -> Self {
        Self {
            source,
            target,
            max_total,
            available_counters,
        }
    }
}

/// Response for counter removal: (counter_type, count_to_remove) pairs.
pub type CounterRemovalResponse = Vec<(CounterType, u32)>;

impl DecisionSpec for CounterRemovalSpec {
    type Response = CounterRemovalResponse;

    fn description(&self) -> String {
        format!("Choose up to {} counters to remove", self.max_total)
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectCounters {
            max_total: self.max_total,
        }
    }

    fn default_response(&self, strategy: FallbackStrategy) -> CounterRemovalResponse {
        match strategy {
            FallbackStrategy::Maximum => {
                // Remove as many counters as possible
                let mut remaining = self.max_total;
                let mut selections = Vec::new();
                for (counter_type, available) in &self.available_counters {
                    if remaining == 0 {
                        break;
                    }
                    let to_remove = (*available).min(remaining);
                    if to_remove > 0 {
                        selections.push((*counter_type, to_remove));
                        remaining -= to_remove;
                    }
                }
                selections
            }
            _ => Vec::new(), // Default: don't remove any counters
        }
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let target_name = game
            .object(self.target)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        DecisionContext::Counters(CountersContext::new(
            player,
            Some(self.source),
            self.target,
            target_name,
            self.max_total,
            self.available_counters.clone(),
        ))
    }
}

// ============================================================================
// ManaPipSpec - Choose how to pay a single mana pip
// ============================================================================

/// An action for paying a mana pip.
#[derive(Debug, Clone)]
pub enum ManaPipAction {
    /// Use mana from the pool.
    UseFromPool(crate::mana::ManaSymbol),
    /// Activate a mana ability.
    ActivateManaAbility {
        source_id: ObjectId,
        ability_index: usize,
    },
    /// Pay life (for Phyrexian mana).
    PayLife(u32),
}

/// An option for paying a mana pip.
#[derive(Debug, Clone)]
pub struct ManaPipOption {
    /// Index of this option.
    pub index: usize,
    /// Description of this payment method.
    pub description: String,
    /// The action to take.
    pub action: ManaPipAction,
}

impl ManaPipOption {
    /// Create a new ManaPipOption.
    pub fn new(index: usize, description: impl Into<String>, action: ManaPipAction) -> Self {
        Self {
            index,
            description: description.into(),
            action,
        }
    }
}

/// Specification for paying a single mana pip.
#[derive(Debug, Clone)]
pub struct ManaPipSpec {
    /// The source being paid for.
    pub source: ObjectId,
    /// Description of what is being paid for.
    pub context: String,
    /// The pip being paid (as alternatives).
    pub pip: Vec<crate::mana::ManaSymbol>,
    /// Human-readable description of this pip.
    pub pip_description: String,
    /// How many pips remain after this one.
    pub remaining_pips: usize,
    /// Payment options for this pip.
    pub options: Vec<ManaPipOption>,
}

impl ManaPipSpec {
    /// Create a new ManaPipSpec.
    pub fn new(
        source: ObjectId,
        context: impl Into<String>,
        pip: Vec<crate::mana::ManaSymbol>,
        pip_description: impl Into<String>,
        remaining_pips: usize,
        options: Vec<ManaPipOption>,
    ) -> Self {
        Self {
            source,
            context: context.into(),
            pip,
            pip_description: pip_description.into(),
            remaining_pips,
            options,
        }
    }
}

impl DecisionSpec for ManaPipSpec {
    type Response = usize;

    fn description(&self) -> String {
        format!(
            "Pay {} for {} ({} remaining)",
            self.pip_description, self.context, self.remaining_pips
        )
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
            .map(|opt| SelectableOption::new(opt.index, &opt.description))
            .collect();

        DecisionContext::SelectOptions(SelectOptionsContext::new(
            player,
            Some(self.source),
            format!(
                "Pay {} for {} ({} remaining)",
                self.pip_description, self.context, self.remaining_pips
            ),
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
    use crate::ids::PlayerId;

    #[test]
    fn test_scry_spec() {
        let source = ObjectId::from_raw(1);
        let cards = vec![ObjectId::from_raw(2), ObjectId::from_raw(3)];
        let spec = ScrySpec::new(source, cards);

        assert!(spec.description().contains("Scry 2"));
        // Default keeps all on top
        assert!(spec.default_response(FallbackStrategy::Decline).is_empty());
    }

    #[test]
    fn test_surveil_spec() {
        let source = ObjectId::from_raw(1);
        let cards = vec![ObjectId::from_raw(2)];
        let spec = SurveilSpec::new(source, cards);

        assert!(spec.description().contains("Surveil 1"));
        // Default keeps all on library
        assert!(spec.default_response(FallbackStrategy::Decline).is_empty());
    }

    #[test]
    fn test_distribute_spec_damage() {
        let source = ObjectId::from_raw(1);
        let targets = vec![
            Target::Player(PlayerId::from_index(0)),
            Target::Player(PlayerId::from_index(1)),
        ];
        let spec = DistributeSpec::damage(source, 5, targets.clone());

        assert!(spec.description().contains("Distribute 5"));
        assert!(matches!(
            spec.primitive(),
            DecisionPrimitive::Distribute {
                total: 5,
                min_per_target: 1
            }
        ));

        // Default puts all on first target
        let response = spec.default_response(FallbackStrategy::FirstOption);
        assert_eq!(response.len(), 1);
        assert_eq!(response[0].1, 5);
    }

    #[test]
    fn test_mana_colors_spec() {
        let source = ObjectId::from_raw(1);
        let spec = ManaColorsSpec::any_color(source, 2, false);

        assert!(spec.description().contains("2 mana color"));
        let response = spec.default_response(FallbackStrategy::FirstOption);
        assert_eq!(response.len(), 2);
    }

    #[test]
    fn test_counter_removal_spec() {
        let source = ObjectId::from_raw(1);
        let target = ObjectId::from_raw(2);
        let available = vec![(CounterType::PlusOnePlusOne, 3), (CounterType::Loyalty, 2)];
        let spec = CounterRemovalSpec::new(source, target, 4, available);

        assert!(spec.description().contains("up to 4"));

        // Default removes nothing
        assert!(spec.default_response(FallbackStrategy::Decline).is_empty());

        // Maximum removes as much as possible
        let max = spec.default_response(FallbackStrategy::Maximum);
        let total_removed: u32 = max.iter().map(|(_, count)| count).sum();
        assert_eq!(total_removed, 4);
    }
}

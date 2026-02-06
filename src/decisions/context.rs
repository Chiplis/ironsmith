//! Decision context structs for the new DecisionMaker trait methods.
//!
//! Each context struct contains all information needed by a DecisionMaker
//! to render and process a specific type of decision.

use crate::color::Color;
use crate::combat_state::AttackTarget;
use crate::game_state::Target;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::zone::Zone;

// ============================================================================
// Boolean Context
// ============================================================================

/// Context for boolean (yes/no) decisions.
///
/// Used for: may effects, ward payment, miracle trigger, madness trigger,
/// assign damage as unblocked, etc.
#[derive(Debug, Clone)]
pub struct BooleanContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source of the effect (optional).
    pub source: Option<ObjectId>,
    /// Description of what the player may do.
    pub description: String,
    /// Name of the source card (for display).
    pub source_name: Option<String>,
}

impl BooleanContext {
    /// Create a new BooleanContext.
    pub fn new(player: PlayerId, source: Option<ObjectId>, description: impl Into<String>) -> Self {
        Self {
            player,
            source,
            description: description.into(),
            source_name: None,
        }
    }

    /// Set the source name for display.
    pub fn with_source_name(mut self, name: impl Into<String>) -> Self {
        self.source_name = Some(name.into());
        self
    }
}

// ============================================================================
// Number Context
// ============================================================================

/// Context for number selection decisions.
///
/// Used for: X value, "choose a number", "up to N" effects, etc.
#[derive(Debug, Clone)]
pub struct NumberContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source of the effect.
    pub source: Option<ObjectId>,
    /// Description of what the number represents.
    pub description: String,
    /// Minimum value (inclusive).
    pub min: u32,
    /// Maximum value (inclusive).
    pub max: u32,
    /// Whether this is an X value decision (affects response type).
    pub is_x_value: bool,
}

impl NumberContext {
    /// Create a new NumberContext.
    pub fn new(
        player: PlayerId,
        source: Option<ObjectId>,
        min: u32,
        max: u32,
        description: impl Into<String>,
    ) -> Self {
        Self {
            player,
            source,
            min,
            max,
            description: description.into(),
            is_x_value: false,
        }
    }

    /// Create a NumberContext for an X value decision.
    pub fn x_value(player: PlayerId, source: ObjectId, max: u32) -> Self {
        Self {
            player,
            source: Some(source),
            min: 0,
            max,
            description: "Choose value for X".to_string(),
            is_x_value: true,
        }
    }
}

// ============================================================================
// View Cards Context
// ============================================================================

/// Context for viewing cards in a private zone.
///
/// Used for effects like "Look at target player's hand."
#[derive(Debug, Clone)]
pub struct ViewCardsContext {
    /// The player who is viewing the cards.
    pub viewer: PlayerId,
    /// The player whose cards are being viewed.
    pub subject: PlayerId,
    /// The source of the effect (optional).
    pub source: Option<ObjectId>,
    /// The zone being viewed.
    pub zone: Zone,
    /// Description of why the cards are being viewed.
    pub description: String,
}

impl ViewCardsContext {
    /// Create a new ViewCardsContext.
    pub fn new(
        viewer: PlayerId,
        subject: PlayerId,
        source: Option<ObjectId>,
        zone: Zone,
        description: impl Into<String>,
    ) -> Self {
        Self {
            viewer,
            subject,
            source,
            zone,
            description: description.into(),
        }
    }

    /// Convenience for "look at target player's hand".
    pub fn look_at_hand(viewer: PlayerId, subject: PlayerId, source: Option<ObjectId>) -> Self {
        Self::new(
            viewer,
            subject,
            source,
            Zone::Hand,
            "Look at target player's hand",
        )
    }
}

// ============================================================================
// Select Objects Context
// ============================================================================

/// An object that can be selected.
#[derive(Debug, Clone)]
pub struct SelectableObject {
    /// The object ID.
    pub id: ObjectId,
    /// Display name for this object.
    pub name: String,
    /// Whether this object is currently legal to select.
    pub legal: bool,
}

impl SelectableObject {
    /// Create a new legal selectable object.
    pub fn new(id: ObjectId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            legal: true,
        }
    }

    /// Create a selectable object with explicit legality.
    pub fn with_legality(id: ObjectId, name: impl Into<String>, legal: bool) -> Self {
        Self {
            id,
            name: name.into(),
            legal,
        }
    }
}

/// Context for object selection decisions.
///
/// Used for: sacrifice, discard, search library, exile, choose legend, etc.
#[derive(Debug, Clone)]
pub struct SelectObjectsContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source of the effect.
    pub source: Option<ObjectId>,
    /// Description of what kind of objects to select.
    pub description: String,
    /// Objects that can be selected.
    pub candidates: Vec<SelectableObject>,
    /// Minimum objects to select (0 for optional).
    pub min: usize,
    /// Maximum objects to select (None = unlimited).
    pub max: Option<usize>,
}

impl SelectObjectsContext {
    /// Create a new SelectObjectsContext.
    pub fn new(
        player: PlayerId,
        source: Option<ObjectId>,
        description: impl Into<String>,
        candidates: Vec<SelectableObject>,
        min: usize,
        max: Option<usize>,
    ) -> Self {
        Self {
            player,
            source,
            description: description.into(),
            candidates,
            min,
            max,
        }
    }
}

// ============================================================================
// Select Options Context
// ============================================================================

/// An option that can be selected.
#[derive(Debug, Clone)]
pub struct SelectableOption {
    /// Index of this option.
    pub index: usize,
    /// Description of this option.
    pub description: String,
    /// Whether this option is currently legal to select.
    pub legal: bool,
}

impl SelectableOption {
    /// Create a new legal selectable option.
    pub fn new(index: usize, description: impl Into<String>) -> Self {
        Self {
            index,
            description: description.into(),
            legal: true,
        }
    }

    /// Create a selectable option with explicit legality.
    pub fn with_legality(index: usize, description: impl Into<String>, legal: bool) -> Self {
        Self {
            index,
            description: description.into(),
            legal,
        }
    }
}

/// Context for option selection decisions.
///
/// Used for: modes, choices, priority actions, mana payment, replacement effects, etc.
#[derive(Debug, Clone)]
pub struct SelectOptionsContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source of the effect.
    pub source: Option<ObjectId>,
    /// Description of the choice being made.
    pub description: String,
    /// Options to choose from.
    pub options: Vec<SelectableOption>,
    /// Minimum options to select.
    pub min: usize,
    /// Maximum options to select.
    pub max: usize,
}

impl SelectOptionsContext {
    /// Create a new SelectOptionsContext.
    pub fn new(
        player: PlayerId,
        source: Option<ObjectId>,
        description: impl Into<String>,
        options: Vec<SelectableOption>,
        min: usize,
        max: usize,
    ) -> Self {
        Self {
            player,
            source,
            description: description.into(),
            options,
            min,
            max,
        }
    }

    /// Create a mana payment context.
    /// The description includes "mana" to trigger ManaPayment response type.
    pub fn mana_payment(
        player: PlayerId,
        source: ObjectId,
        spell_name: impl Into<String>,
        options: Vec<SelectableOption>,
    ) -> Self {
        Self {
            player,
            source: Some(source),
            description: format!("Pay mana for {}", spell_name.into()),
            options,
            min: 1,
            max: 1,
        }
    }

    /// Create a mana pip payment context.
    /// The description includes "mana pip" to trigger ManaPipPayment response type.
    pub fn mana_pip_payment(
        player: PlayerId,
        source: ObjectId,
        spell_name: impl Into<String>,
        pip_description: impl Into<String>,
        remaining_pips: usize,
        options: Vec<SelectableOption>,
    ) -> Self {
        Self {
            player,
            source: Some(source),
            description: format!(
                "Pay mana pip {} for {} ({} remaining)",
                pip_description.into(),
                spell_name.into(),
                remaining_pips
            ),
            options,
            min: 1,
            max: 1,
        }
    }
}

// ============================================================================
// Modes Context
// ============================================================================

/// Context for modal spell mode selection (per MTG rule 601.2b).
///
/// Used during spell casting to select modes before targets are chosen.
#[derive(Debug, Clone)]
pub struct ModesContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source spell on the stack.
    pub source: Option<ObjectId>,
    /// Name of the spell being cast.
    pub spell_name: String,
    /// The modes specification with options, min/max counts.
    pub spec: crate::decisions::ModesSpec,
}

impl ModesContext {
    /// Create a new ModesContext.
    pub fn new(
        player: PlayerId,
        source: Option<ObjectId>,
        spell_name: impl Into<String>,
        spec: crate::decisions::ModesSpec,
    ) -> Self {
        Self {
            player,
            source,
            spell_name: spell_name.into(),
            spec,
        }
    }
}

// ============================================================================
// Hybrid Choice Context
// ============================================================================

/// An option for paying a hybrid/Phyrexian mana pip.
#[derive(Debug, Clone)]
pub struct HybridOption {
    /// The index of this option.
    pub index: usize,
    /// Display label for this option (e.g., "{W} (White mana)").
    pub label: String,
    /// The mana symbol this option represents.
    pub symbol: crate::mana::ManaSymbol,
}

/// Context for hybrid/Phyrexian mana payment choice (per MTG rule 601.2b).
///
/// Used during spell casting to announce how hybrid/Phyrexian costs will be paid
/// before targets are chosen.
#[derive(Debug, Clone)]
pub struct HybridChoiceContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source spell on the stack.
    pub source: Option<ObjectId>,
    /// Name of the spell being cast.
    pub spell_name: String,
    /// The pip index (1-based for display).
    pub pip_number: usize,
    /// The available payment options.
    pub options: Vec<HybridOption>,
}

impl HybridChoiceContext {
    /// Create a new HybridChoiceContext.
    pub fn new(
        player: PlayerId,
        source: Option<ObjectId>,
        spell_name: impl Into<String>,
        pip_number: usize,
        options: Vec<HybridOption>,
    ) -> Self {
        Self {
            player,
            source,
            spell_name: spell_name.into(),
            pip_number,
            options,
        }
    }
}

// ============================================================================
// Order Context
// ============================================================================

/// Context for ordering decisions.
///
/// Used for: order blockers, order attackers, scry ordering, surveil ordering, etc.
#[derive(Debug, Clone)]
pub struct OrderContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source of the effect.
    pub source: Option<ObjectId>,
    /// Description of what is being ordered.
    pub description: String,
    /// Items to order (as object IDs with display names).
    pub items: Vec<(ObjectId, String)>,
}

impl OrderContext {
    /// Create a new OrderContext.
    pub fn new(
        player: PlayerId,
        source: Option<ObjectId>,
        description: impl Into<String>,
        items: Vec<(ObjectId, String)>,
    ) -> Self {
        Self {
            player,
            source,
            description: description.into(),
            items,
        }
    }
}

// ============================================================================
// Attackers Context
// ============================================================================

/// An attacker option with its valid targets.
#[derive(Debug, Clone)]
pub struct AttackerOptionContext {
    /// The creature that can attack.
    pub creature: ObjectId,
    /// Display name of the creature.
    pub creature_name: String,
    /// Valid targets this creature can attack.
    pub valid_targets: Vec<AttackTarget>,
    /// Whether this creature must attack if able.
    pub must_attack: bool,
}

/// Context for declaring attackers.
#[derive(Debug, Clone)]
pub struct AttackersContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// Creatures that can attack with their valid targets.
    pub attacker_options: Vec<AttackerOptionContext>,
}

impl AttackersContext {
    /// Create a new AttackersContext.
    pub fn new(player: PlayerId, attacker_options: Vec<AttackerOptionContext>) -> Self {
        Self {
            player,
            attacker_options,
        }
    }
}

// ============================================================================
// Blockers Context
// ============================================================================

/// Options for blocking a specific attacker.
#[derive(Debug, Clone)]
pub struct BlockerOptionContext {
    /// The attacking creature.
    pub attacker: ObjectId,
    /// Display name of the attacker.
    pub attacker_name: String,
    /// Creatures that can legally block this attacker (ID and name).
    pub valid_blockers: Vec<(ObjectId, String)>,
    /// Minimum number of blockers required (for menace, etc.).
    pub min_blockers: usize,
}

/// Context for declaring blockers.
#[derive(Debug, Clone)]
pub struct BlockersContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// Options for each attacker.
    pub blocker_options: Vec<BlockerOptionContext>,
}

impl BlockersContext {
    /// Create a new BlockersContext.
    pub fn new(player: PlayerId, blocker_options: Vec<BlockerOptionContext>) -> Self {
        Self {
            player,
            blocker_options,
        }
    }
}

// ============================================================================
// Distribute Context
// ============================================================================

/// A target for distribution with display information.
#[derive(Debug, Clone)]
pub struct DistributeTarget {
    /// The target.
    pub target: Target,
    /// Display name for this target.
    pub name: String,
}

/// Context for distribution decisions.
///
/// Used for: damage distribution, counter distribution, etc.
#[derive(Debug, Clone)]
pub struct DistributeContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source of the effect.
    pub source: Option<ObjectId>,
    /// Description of what is being distributed.
    pub description: String,
    /// Total amount to distribute.
    pub total: u32,
    /// Valid targets to distribute among.
    pub targets: Vec<DistributeTarget>,
    /// Minimum amount per target (usually 1 for damage).
    pub min_per_target: u32,
}

impl DistributeContext {
    /// Create a new DistributeContext.
    pub fn new(
        player: PlayerId,
        source: Option<ObjectId>,
        description: impl Into<String>,
        total: u32,
        targets: Vec<DistributeTarget>,
        min_per_target: u32,
    ) -> Self {
        Self {
            player,
            source,
            description: description.into(),
            total,
            targets,
            min_per_target,
        }
    }
}

// ============================================================================
// Colors Context
// ============================================================================

/// Context for color selection decisions.
///
/// Used for: mana color selection, protection color choice, etc.
#[derive(Debug, Clone)]
pub struct ColorsContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source of the effect.
    pub source: Option<ObjectId>,
    /// Description of the color choice.
    pub description: String,
    /// Number of colors to select.
    pub count: u32,
    /// If true, all selections must be the same color.
    pub same_color: bool,
    /// Available colors (None = all five colors).
    pub available_colors: Option<Vec<Color>>,
}

impl ColorsContext {
    /// Create a new ColorsContext for any color.
    pub fn any_color(
        player: PlayerId,
        source: Option<ObjectId>,
        count: u32,
        same_color: bool,
    ) -> Self {
        Self {
            player,
            source,
            description: if same_color {
                format!("Choose a color for {} mana", count)
            } else {
                format!("Choose {} mana color(s)", count)
            },
            count,
            same_color,
            available_colors: None,
        }
    }

    /// Create a new ColorsContext with restricted colors.
    pub fn restricted(
        player: PlayerId,
        source: Option<ObjectId>,
        count: u32,
        same_color: bool,
        available_colors: Vec<Color>,
    ) -> Self {
        Self {
            player,
            source,
            description: if same_color {
                format!("Choose a color for {} mana", count)
            } else {
                format!("Choose {} mana color(s)", count)
            },
            count,
            same_color,
            available_colors: Some(available_colors),
        }
    }
}

// ============================================================================
// Counters Context
// ============================================================================

/// Context for counter removal decisions.
///
/// Used for: Hex Parasite-style "remove up to X counters" effects.
#[derive(Debug, Clone)]
pub struct CountersContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source of the effect.
    pub source: Option<ObjectId>,
    /// The target permanent to remove counters from.
    pub target: ObjectId,
    /// Display name of the target.
    pub target_name: String,
    /// Maximum total counters that can be removed.
    pub max_total: u32,
    /// Available counters: (counter_type, count_available).
    pub available_counters: Vec<(CounterType, u32)>,
}

impl CountersContext {
    /// Create a new CountersContext.
    pub fn new(
        player: PlayerId,
        source: Option<ObjectId>,
        target: ObjectId,
        target_name: impl Into<String>,
        max_total: u32,
        available_counters: Vec<(CounterType, u32)>,
    ) -> Self {
        Self {
            player,
            source,
            target,
            target_name: target_name.into(),
            max_total,
            available_counters,
        }
    }
}

// ============================================================================
// Partition Context
// ============================================================================

/// Context for partition decisions (scry, surveil).
///
/// The response is a list of object IDs to put in the "secondary" destination:
/// - For scry: cards to put on bottom (rest stay on top)
/// - For surveil: cards to put in graveyard (rest stay on top)
#[derive(Debug, Clone)]
pub struct PartitionContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source of the effect.
    pub source: Option<ObjectId>,
    /// Description of the partition (e.g., "Scry 2", "Surveil 3").
    pub description: String,
    /// Cards to partition (ID and name).
    pub cards: Vec<(ObjectId, String)>,
    /// Label for primary destination (e.g., "top of library").
    pub primary_label: String,
    /// Label for secondary destination (e.g., "bottom of library", "graveyard").
    pub secondary_label: String,
}

impl PartitionContext {
    /// Create a new PartitionContext.
    pub fn new(
        player: PlayerId,
        source: Option<ObjectId>,
        description: impl Into<String>,
        cards: Vec<(ObjectId, String)>,
        primary_label: impl Into<String>,
        secondary_label: impl Into<String>,
    ) -> Self {
        Self {
            player,
            source,
            description: description.into(),
            cards,
            primary_label: primary_label.into(),
            secondary_label: secondary_label.into(),
        }
    }

    /// Create a scry context.
    pub fn scry(
        player: PlayerId,
        source: Option<ObjectId>,
        cards: Vec<(ObjectId, String)>,
    ) -> Self {
        Self::new(
            player,
            source,
            format!("Scry {}", cards.len()),
            cards,
            "top of library",
            "bottom of library",
        )
    }

    /// Create a surveil context.
    pub fn surveil(
        player: PlayerId,
        source: Option<ObjectId>,
        cards: Vec<(ObjectId, String)>,
    ) -> Self {
        Self::new(
            player,
            source,
            format!("Surveil {}", cards.len()),
            cards,
            "top of library",
            "graveyard",
        )
    }
}

// ============================================================================
// Proliferate Context
// ============================================================================

/// Context for proliferate decisions.
///
/// Player chooses any number of permanents and/or players with counters.
/// Each chosen object/player gets one counter of each type it already has.
#[derive(Debug, Clone)]
pub struct ProliferateContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source of the effect.
    pub source: Option<ObjectId>,
    /// Permanents with counters that can be proliferated (ID and name).
    pub eligible_permanents: Vec<(ObjectId, String)>,
    /// Players with counters that can be proliferated (ID and name).
    pub eligible_players: Vec<(PlayerId, String)>,
}

impl ProliferateContext {
    /// Create a new ProliferateContext.
    pub fn new(
        player: PlayerId,
        source: Option<ObjectId>,
        eligible_permanents: Vec<(ObjectId, String)>,
        eligible_players: Vec<(PlayerId, String)>,
    ) -> Self {
        Self {
            player,
            source,
            eligible_permanents,
            eligible_players,
        }
    }
}

// ============================================================================
// Priority Context
// ============================================================================

/// Context for priority decisions.
///
/// This includes the full legal actions so responses can be converted back.
#[derive(Debug, Clone)]
pub struct PriorityContext {
    /// The player with priority.
    pub player: PlayerId,
    /// Legal actions available.
    pub legal_actions: Vec<crate::decision::LegalAction>,
    /// Commander actions available (accessed via 'C').
    pub commander_actions: Vec<crate::decision::LegalAction>,
}

impl PriorityContext {
    /// Create a new PriorityContext.
    pub fn new(
        player: PlayerId,
        legal_actions: Vec<crate::decision::LegalAction>,
        commander_actions: Vec<crate::decision::LegalAction>,
    ) -> Self {
        Self {
            player,
            legal_actions,
            commander_actions,
        }
    }
}

// ============================================================================
// Targets Context
// ============================================================================

/// A targeting requirement with legal targets.
#[derive(Debug, Clone)]
pub struct TargetRequirementContext {
    /// Description of what's being targeted.
    pub description: String,
    /// Legal targets for this requirement.
    pub legal_targets: Vec<crate::game_state::Target>,
    /// Minimum number of targets to choose.
    pub min_targets: usize,
    /// Maximum number of targets to choose (None = unlimited).
    pub max_targets: Option<usize>,
}

impl TargetRequirementContext {
    /// Create a requirement for exactly one target.
    pub fn single(
        description: impl Into<String>,
        legal_targets: Vec<crate::game_state::Target>,
    ) -> Self {
        Self {
            description: description.into(),
            legal_targets,
            min_targets: 1,
            max_targets: Some(1),
        }
    }
}

/// Context for choosing targets for a spell or ability.
#[derive(Debug, Clone)]
pub struct TargetsContext {
    /// The player making the decision.
    pub player: PlayerId,
    /// The source spell or ability.
    pub source: ObjectId,
    /// Description of what is being targeted (spell/ability name).
    pub context: String,
    /// The targeting requirements.
    pub requirements: Vec<TargetRequirementContext>,
}

impl TargetsContext {
    /// Create a new TargetsContext.
    pub fn new(
        player: PlayerId,
        source: ObjectId,
        context: impl Into<String>,
        requirements: Vec<TargetRequirementContext>,
    ) -> Self {
        Self {
            player,
            source,
            context: context.into(),
            requirements,
        }
    }
}

// ============================================================================
// DecisionContext Enum
// ============================================================================

/// Unified enum for all decision context types.
///
/// This enum allows the `build_context()` method to return any context type.
#[derive(Debug, Clone)]
pub enum DecisionContext {
    Boolean(BooleanContext),
    Number(NumberContext),
    SelectObjects(SelectObjectsContext),
    SelectOptions(SelectOptionsContext),
    /// Mode selection for modal spells (per MTG rule 601.2b).
    Modes(ModesContext),
    /// Hybrid/Phyrexian mana payment choice (per MTG rule 601.2b).
    HybridChoice(HybridChoiceContext),
    Order(OrderContext),
    Attackers(AttackersContext),
    Blockers(BlockersContext),
    Distribute(DistributeContext),
    Colors(ColorsContext),
    Counters(CountersContext),
    Partition(PartitionContext),
    Proliferate(ProliferateContext),
    /// Priority decisions with full legal action list for response conversion.
    Priority(PriorityContext),
    /// Target selection for spells and abilities.
    Targets(TargetsContext),
}

impl DecisionContext {
    /// Convert to BooleanContext, panicking if wrong type.
    pub fn into_boolean(self) -> BooleanContext {
        match self {
            DecisionContext::Boolean(ctx) => ctx,
            _ => panic!("Expected BooleanContext"),
        }
    }

    /// Convert to NumberContext, panicking if wrong type.
    pub fn into_number(self) -> NumberContext {
        match self {
            DecisionContext::Number(ctx) => ctx,
            _ => panic!("Expected NumberContext"),
        }
    }

    /// Convert to SelectObjectsContext, panicking if wrong type.
    pub fn into_objects(self) -> SelectObjectsContext {
        match self {
            DecisionContext::SelectObjects(ctx) => ctx,
            _ => panic!("Expected SelectObjectsContext"),
        }
    }

    /// Convert to SelectOptionsContext, panicking if wrong type.
    pub fn into_options(self) -> SelectOptionsContext {
        match self {
            DecisionContext::SelectOptions(ctx) => ctx,
            _ => panic!("Expected SelectOptionsContext"),
        }
    }

    /// Convert to OrderContext, panicking if wrong type.
    pub fn into_order(self) -> OrderContext {
        match self {
            DecisionContext::Order(ctx) => ctx,
            _ => panic!("Expected OrderContext"),
        }
    }

    /// Convert to AttackersContext, panicking if wrong type.
    pub fn into_attackers(self) -> AttackersContext {
        match self {
            DecisionContext::Attackers(ctx) => ctx,
            _ => panic!("Expected AttackersContext"),
        }
    }

    /// Convert to BlockersContext, panicking if wrong type.
    pub fn into_blockers(self) -> BlockersContext {
        match self {
            DecisionContext::Blockers(ctx) => ctx,
            _ => panic!("Expected BlockersContext"),
        }
    }

    /// Convert to DistributeContext, panicking if wrong type.
    pub fn into_distribute(self) -> DistributeContext {
        match self {
            DecisionContext::Distribute(ctx) => ctx,
            _ => panic!("Expected DistributeContext"),
        }
    }

    /// Convert to ColorsContext, panicking if wrong type.
    pub fn into_colors(self) -> ColorsContext {
        match self {
            DecisionContext::Colors(ctx) => ctx,
            _ => panic!("Expected ColorsContext"),
        }
    }

    /// Convert to CountersContext, panicking if wrong type.
    pub fn into_counters(self) -> CountersContext {
        match self {
            DecisionContext::Counters(ctx) => ctx,
            _ => panic!("Expected CountersContext"),
        }
    }

    /// Convert to PartitionContext, panicking if wrong type.
    pub fn into_partition(self) -> PartitionContext {
        match self {
            DecisionContext::Partition(ctx) => ctx,
            _ => panic!("Expected PartitionContext"),
        }
    }

    /// Convert to ProliferateContext, panicking if wrong type.
    pub fn into_proliferate(self) -> ProliferateContext {
        match self {
            DecisionContext::Proliferate(ctx) => ctx,
            _ => panic!("Expected ProliferateContext"),
        }
    }

    /// Convert to PriorityContext, panicking if wrong type.
    pub fn into_priority(self) -> PriorityContext {
        match self {
            DecisionContext::Priority(ctx) => ctx,
            _ => panic!("Expected PriorityContext"),
        }
    }

    /// Convert to TargetsContext, panicking if wrong type.
    pub fn into_targets(self) -> TargetsContext {
        match self {
            DecisionContext::Targets(ctx) => ctx,
            _ => panic!("Expected TargetsContext"),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boolean_context() {
        let player = PlayerId::from_index(0);
        let ctx =
            BooleanContext::new(player, None, "draw a card").with_source_name("Wall of Omens");

        assert_eq!(ctx.description, "draw a card");
        assert_eq!(ctx.source_name, Some("Wall of Omens".to_string()));
    }

    #[test]
    fn test_number_context() {
        let player = PlayerId::from_index(0);
        let ctx = NumberContext::new(player, None, 0, 5, "Choose X");

        assert_eq!(ctx.min, 0);
        assert_eq!(ctx.max, 5);
    }

    #[test]
    fn test_select_objects_context() {
        let player = PlayerId::from_index(0);
        let candidates = vec![
            SelectableObject::new(ObjectId::from_raw(1), "Forest"),
            SelectableObject::new(ObjectId::from_raw(2), "Mountain"),
        ];
        let ctx = SelectObjectsContext::new(
            player,
            None,
            "Choose a land to sacrifice",
            candidates,
            1,
            Some(1),
        );

        assert_eq!(ctx.candidates.len(), 2);
        assert_eq!(ctx.min, 1);
        assert_eq!(ctx.max, Some(1));
    }

    #[test]
    fn test_partition_context_scry() {
        let player = PlayerId::from_index(0);
        let cards = vec![
            (ObjectId::from_raw(1), "Forest".to_string()),
            (ObjectId::from_raw(2), "Lightning Bolt".to_string()),
        ];
        let ctx = PartitionContext::scry(player, None, cards);

        assert!(ctx.description.contains("Scry 2"));
        assert_eq!(ctx.secondary_label, "bottom of library");
    }

    #[test]
    fn test_decision_context_conversions() {
        let player = PlayerId::from_index(0);
        let ctx = DecisionContext::Boolean(BooleanContext::new(player, None, "test"));

        let boolean = ctx.into_boolean();
        assert_eq!(boolean.description, "test");
    }
}

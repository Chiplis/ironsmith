use super::*;

// ============================================================================
// Fallback Strategies
// ============================================================================

/// Strategy for how effects should behave when no decision maker is present.
///
/// Different effects have different default behaviors when the player cannot
/// be prompted for a decision (e.g., in tests, AI, or auto-resolve scenarios).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FallbackStrategy {
    /// Decline optional actions ("may" effects).
    /// The effect does not occur. This is the safest default for optional effects.
    #[default]
    Decline,

    /// Choose the first legal option available.
    /// Good for mandatory choices where any option is equally valid.
    FirstOption,

    /// Choose the maximum value for "up to" effects.
    /// Maximizes the effect's impact.
    Maximum,

    /// Choose the minimum value for "up to" effects (usually 0).
    /// Minimizes the effect's impact.
    Minimum,

    /// Accept/perform the action (opposite of Decline).
    /// For "may" effects where the default should be to do it.
    Accept,
}

// ============================================================================
// Action Types
// ============================================================================

/// A legal action a player can take when they have priority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegalAction {
    /// Pass priority to the next player.
    PassPriority,

    /// Keep the current opening hand during pregame mulligans.
    KeepOpeningHand,

    /// Take a normal mulligan during pregame.
    TakeMulligan,

    /// Finish the current player's pregame actions and move to the next player.
    ContinuePregame,

    /// Finish pregame actions and begin the first turn.
    BeginGame,

    /// Use a parser-backed pregame action from a card in hand.
    UsePregameAction {
        card_id: ObjectId,
        ability_index: usize,
    },

    /// Cast a spell from a zone.
    CastSpell {
        spell_id: ObjectId,
        from_zone: Zone,
        /// The casting method (normal or alternative like flashback).
        casting_method: CastingMethod,
    },

    /// Activate an ability on a permanent.
    ActivateAbility {
        source: ObjectId,
        ability_index: usize,
    },

    /// Play a land from hand.
    PlayLand { land_id: ObjectId },

    /// Activate a mana ability (doesn't use stack).
    ActivateManaAbility {
        source: ObjectId,
        ability_index: usize,
    },

    /// Turn a face-down creature face up (e.g., morph/megamorph/manifest).
    TurnFaceUp {
        creature_id: ObjectId,
        method: crate::special_actions::TurnFaceUpMethod,
    },

    /// Special action (suspend, foretell, etc.).
    SpecialAction(SpecialAction),
}

/// An option for declaring an attacker.
#[derive(Debug, Clone)]
pub struct AttackerOption {
    /// The creature that can attack.
    pub creature: ObjectId,
    /// Valid targets this creature can attack.
    pub valid_targets: Vec<AttackTarget>,
    /// Whether this creature must attack if able.
    pub must_attack: bool,
}

/// A declared attacker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackerDeclaration {
    /// The attacking creature.
    pub creature: ObjectId,
    /// What the creature is attacking.
    pub target: AttackTarget,
}

/// Options for blocking a specific attacker.
#[derive(Debug, Clone)]
pub struct BlockerOption {
    /// The attacking creature.
    pub attacker: ObjectId,
    /// Creatures that can legally block this attacker.
    pub valid_blockers: Vec<ObjectId>,
    /// Minimum number of blockers required (for menace, etc.).
    pub min_blockers: usize,
}

/// A declared blocker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerDeclaration {
    /// The blocking creature.
    pub blocker: ObjectId,
    /// The attacker being blocked.
    pub blocking: ObjectId,
}

/// A targeting requirement for a spell or ability.
#[derive(Debug, Clone)]
pub struct TargetRequirement {
    /// The target specification.
    pub spec: ChooseSpec,
    /// Player assigned to make this target choice. `None` uses the spell or
    /// ability controller.
    pub chooser: Option<crate::target::PlayerFilter>,
    /// Legal targets that match this specification.
    pub legal_targets: Vec<Target>,
    /// Legal target groups for constraints that apply to the selected set.
    /// If empty, any combination of legal targets is allowed.
    pub legal_target_sets: Vec<Vec<Target>>,
    /// Resolved restriction on the selected target set as a whole.
    pub aggregate_constraint: Option<crate::targeting::ResolvedTargetAggregateConstraint>,
    /// Description of what's being targeted.
    pub description: String,
    /// Minimum number of targets to choose (default 1).
    pub min_targets: usize,
    /// Maximum number of targets to choose (None = unlimited, i.e., "any number").
    pub max_targets: Option<usize>,
    /// Requirements in the same group must select different player targets.
    pub distinct_player_group: Option<usize>,
    /// Amount to divide among this requirement's selected targets during announcement.
    pub distribution_value: Option<crate::effect::Value>,
    /// Minimum amount assigned to each selected target.
    pub distribution_min_per_target: u32,
}

impl TargetRequirement {
    /// Create a new targeting requirement for exactly one target.
    pub fn single(spec: ChooseSpec, legal_targets: Vec<Target>, description: String) -> Self {
        Self {
            spec,
            chooser: None,
            legal_targets,
            legal_target_sets: Vec::new(),
            aggregate_constraint: None,
            description,
            min_targets: 1,
            max_targets: Some(1),
            distinct_player_group: None,
            distribution_value: None,
            distribution_min_per_target: 1,
        }
    }

    /// Create a new targeting requirement for any number of targets (0 or more).
    pub fn any_number(spec: ChooseSpec, legal_targets: Vec<Target>, description: String) -> Self {
        Self {
            spec,
            chooser: None,
            legal_targets,
            legal_target_sets: Vec::new(),
            aggregate_constraint: None,
            description,
            min_targets: 0,
            max_targets: None,
            distinct_player_group: None,
            distribution_value: None,
            distribution_min_per_target: 1,
        }
    }

    /// Create a new targeting requirement for a specific range of targets.
    pub fn range(
        spec: ChooseSpec,
        legal_targets: Vec<Target>,
        description: String,
        min: usize,
        max: Option<usize>,
    ) -> Self {
        Self {
            spec,
            chooser: None,
            legal_targets,
            legal_target_sets: Vec::new(),
            aggregate_constraint: None,
            description,
            min_targets: min,
            max_targets: max,
            distinct_player_group: None,
            distribution_value: None,
            distribution_min_per_target: 1,
        }
    }

    /// Returns true if this allows choosing any number of targets.
    pub fn is_any_number(&self) -> bool {
        self.min_targets == 0 && self.max_targets.is_none()
    }
}

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

/// A generic choice option.
#[derive(Debug, Clone)]
pub struct ChoiceOption {
    /// Index of this option.
    pub index: usize,
    /// Description of this option.
    pub description: String,
}

/// An optional cost that can be paid when casting.
#[derive(Debug, Clone)]
pub struct OptionalCostOption {
    /// Index of this optional cost in the spell's optional_costs list.
    pub index: usize,
    /// Label for this cost (e.g., "Kicker", "Buyback").
    pub label: String,
    /// Whether this cost can be paid multiple times (multikicker).
    pub repeatable: bool,
    /// Whether the player can currently afford this cost.
    pub affordable: bool,
    /// Description of the cost to pay (e.g., "{2}{R}").
    pub cost_description: String,
}

/// An option for choosing how to cast a spell.
#[derive(Debug, Clone)]
pub struct CastingMethodOption {
    /// The casting method.
    pub method: crate::alternative_cast::CastingMethod,
    /// Display name for this method (e.g., "Normal", "Flashback", "Force of Will").
    pub name: String,
    /// Description of the cost (e.g., "{3}{U}{U}" or "Pay 1 life, exile a blue card").
    pub cost_description: String,
}

/// An option for choosing a replacement effect.
#[derive(Debug, Clone)]
pub struct ReplacementOption {
    /// Index of this option.
    pub index: usize,
    /// Source of the replacement effect.
    pub source: ObjectId,
    /// Description of what this replacement does.
    pub description: String,
}

/// Pip-level alternative payment effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlternativePaymentEffect {
    Convoke,
    Improvise,
}

/// Tracks a keyword ability payment contribution made while casting a spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordPaymentContribution {
    pub permanent_id: ObjectId,
    pub effect: AlternativePaymentEffect,
}

#[derive(Clone, Copy)]
pub(crate) struct HandCardSummary<'a> {
    pub(crate) card_id: ObjectId,
    pub(crate) card: &'a crate::object::Object,
    pub(crate) is_land: bool,
    pub(crate) has_normal_mana_cost: bool,
    pub(crate) has_foretell: bool,
    pub(crate) has_suspend: bool,
    pub(crate) has_plot: bool,
    pub(crate) can_cast_face_down: bool,
    pub(crate) has_split_other_half: bool,
    pub(crate) has_fuse: bool,
    pub(crate) has_hand_native_alternatives: bool,
}

impl<'a> HandCardSummary<'a> {
    pub(crate) fn has_any_hand_special_action(&self) -> bool {
        self.has_foretell || self.has_suspend || self.has_plot
    }

    pub(crate) fn has_any_alternative_branch(&self, has_hand_grants: bool) -> bool {
        self.can_cast_face_down
            || self.has_split_other_half
            || self.has_fuse
            || self.has_hand_native_alternatives
            || has_hand_grants
    }
}

pub(crate) fn spell_has_intrinsic_cost_adjustments(spell: &crate::object::Object) -> bool {
    use crate::ability::AbilityKind;

    spell.abilities.iter().any(|ability| {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        static_ability.has_affinity()
            || static_ability.has_delve()
            || static_ability.has_convoke()
            || static_ability.has_improvise()
            || static_ability.this_spell_cost_reduction().is_some()
            || static_ability
                .this_spell_cost_reduction_mana_cost()
                .is_some()
            || static_ability.cost_reduction().is_some()
            || static_ability.cost_increase().is_some()
            || static_ability.cost_reduction_mana_cost().is_some()
            || static_ability.cost_increase_mana_cost().is_some()
            || static_ability
                .cost_increase_per_additional_target()
                .is_some()
    })
}

#[derive(Clone, Default)]
pub(crate) struct CastLegalityPerfBreakdown {
    pub(crate) total_ms: f64,
    pub(crate) timing_ms: f64,
    pub(crate) restrictions_ms: f64,
    pub(crate) target_legality_ms: f64,
    pub(crate) cost_adjustment_ms: f64,
    pub(crate) affordability_ms: f64,
}

pub(crate) struct CastLegalityContext<'a> {
    pub(crate) game: &'a GameState,
    pub(crate) player: PlayerId,
    pub(crate) view: &'a DerivedGameView<'a>,
    pub(crate) allow_library_search_cast_timing: bool,
    pub(crate) has_battlefield_spell_cost_modifiers: bool,
    pub(crate) has_temporary_spell_cost_reductions: bool,
    pub(crate) minimum_total_spell_mana_payment: Option<u32>,
    pub(crate) strips_life_pips_for_casts: bool,
    pub(crate) perf: RefCell<CastLegalityPerfBreakdown>,
}

impl<'a> CastLegalityContext<'a> {
    pub(crate) fn new(
        game: &'a GameState,
        player: PlayerId,
        view: &'a DerivedGameView<'a>,
    ) -> Self {
        Self {
            game,
            player,
            view,
            allow_library_search_cast_timing: false,
            has_battlefield_spell_cost_modifiers: view.has_battlefield_spell_cost_modifiers(),
            has_temporary_spell_cost_reductions: game
                .effect_store
                .temporary_spell_cost_reductions
                .iter()
                .any(|effect| effect.player == player && !effect.is_expired(game)),
            minimum_total_spell_mana_payment: view.minimum_total_spell_mana_payment(),
            strips_life_pips_for_casts: view.player_cant_pay_life_to_cast_or_activate(player),
            perf: RefCell::new(CastLegalityPerfBreakdown::default()),
        }
    }

    pub(crate) fn with_library_search_cast_timing(mut self) -> Self {
        self.allow_library_search_cast_timing = true;
        self
    }

    pub(crate) fn snapshot_perf(&self) -> CastLegalityPerfBreakdown {
        self.perf.borrow().clone()
    }

    pub(crate) fn add_total_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().total_ms += elapsed_ms;
    }

    pub(crate) fn add_timing_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().timing_ms += elapsed_ms;
    }

    pub(crate) fn add_restrictions_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().restrictions_ms += elapsed_ms;
    }

    pub(crate) fn add_target_legality_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().target_legality_ms += elapsed_ms;
    }

    pub(crate) fn add_cost_adjustment_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().cost_adjustment_ms += elapsed_ms;
    }

    pub(crate) fn add_affordability_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().affordability_ms += elapsed_ms;
    }

    pub(crate) fn spell_cost_needs_adjustment(&self, has_intrinsic_cost_adjustments: bool) -> bool {
        has_intrinsic_cost_adjustments
            || self.has_battlefield_spell_cost_modifiers
            || self.has_temporary_spell_cost_reductions
            || self.minimum_total_spell_mana_payment.is_some()
    }

    pub(crate) fn can_use_printed_cost_directly(
        &self,
        has_intrinsic_cost_adjustments: bool,
    ) -> bool {
        !has_intrinsic_cost_adjustments
            && !self.has_battlefield_spell_cost_modifiers
            && !self.has_temporary_spell_cost_reductions
            && self.minimum_total_spell_mana_payment.is_none()
            && !self.strips_life_pips_for_casts
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ActivationSourceFacts {
    pub(crate) controller: PlayerId,
    pub(crate) can_activate_abilities: bool,
    pub(crate) can_activate_tap_abilities: bool,
    pub(crate) can_activate_non_mana_abilities_of_source: bool,
    pub(crate) is_tapped: bool,
    pub(crate) is_creature: bool,
    pub(crate) is_summoning_sick: bool,
    pub(crate) has_haste: bool,
}

impl ActivationSourceFacts {
    pub(crate) fn for_source(
        game: &GameState,
        source: ObjectId,
        view: &DerivedGameView<'_>,
    ) -> Self {
        let controller = game
            .object(source)
            .map(|obj| game.controller_of(obj))
            .unwrap_or(game.turn.active_player);
        Self {
            controller,
            can_activate_abilities: game.can_activate_abilities_of(source),
            can_activate_tap_abilities: game.can_activate_tap_abilities_of(source),
            can_activate_non_mana_abilities_of_source: game
                .can_activate_non_mana_abilities_of(source),
            is_tapped: game.is_tapped(source),
            is_creature: view.object_has_card_type(source, crate::types::CardType::Creature),
            is_summoning_sick: game.is_summoning_sick(source),
            has_haste: view.object_has_static_ability_id(
                source,
                crate::static_abilities::StaticAbilityId::Haste,
            ),
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct BattlefieldAbilityPerfBreakdown {
    pub(crate) total_ms: f64,
    pub(crate) precheck_ms: f64,
    pub(crate) target_legality_ms: f64,
    pub(crate) cost_build_ms: f64,
    pub(crate) affordability_ms: f64,
}

pub(crate) struct BattlefieldAbilityContext {
    perf: RefCell<BattlefieldAbilityPerfBreakdown>,
    has_activation_cost_modifiers: bool,
}

impl BattlefieldAbilityContext {
    pub(crate) fn new(view: &DerivedGameView<'_>) -> Self {
        Self {
            perf: RefCell::new(BattlefieldAbilityPerfBreakdown::default()),
            has_activation_cost_modifiers: view.has_activated_ability_cost_modifiers(),
        }
    }

    pub(crate) fn snapshot_perf(&self) -> BattlefieldAbilityPerfBreakdown {
        self.perf.borrow().clone()
    }

    pub(crate) fn add_total_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().total_ms += elapsed_ms;
    }

    pub(crate) fn add_precheck_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().precheck_ms += elapsed_ms;
    }

    pub(crate) fn add_target_legality_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().target_legality_ms += elapsed_ms;
    }

    pub(crate) fn add_cost_build_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().cost_build_ms += elapsed_ms;
    }

    pub(crate) fn add_affordability_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().affordability_ms += elapsed_ms;
    }

    pub(crate) fn has_activation_cost_modifiers(&self) -> bool {
        self.has_activation_cost_modifiers
    }
}

pub(crate) fn mana_cost_is_obviously_unpayable(
    potential: &crate::player::ManaPool,
    cost: &crate::mana::ManaCost,
    allow_any_color: bool,
    allow_black_life: bool,
) -> bool {
    use crate::mana::ManaSymbol;

    let mut minimum_total = 0u32;
    let mut required_white = 0u32;
    let mut required_blue = 0u32;
    let mut required_black = 0u32;
    let mut required_red = 0u32;
    let mut required_green = 0u32;
    let mut required_colorless = 0u32;

    for pip in cost.pips() {
        let mut minimum_for_pip: Option<u32> = None;
        let mut has_mana_option = false;

        for symbol in pip {
            let mana_needed = match symbol {
                ManaSymbol::Life(_) => Some(0),
                ManaSymbol::White
                | ManaSymbol::Blue
                | ManaSymbol::Black
                | ManaSymbol::Red
                | ManaSymbol::Green
                | ManaSymbol::Colorless
                | ManaSymbol::Snow => Some(1),
                ManaSymbol::Generic(amount) => Some(*amount as u32),
                ManaSymbol::X => Some(0),
            };
            if let Some(amount) = mana_needed {
                has_mana_option = true;
                minimum_for_pip = Some(match minimum_for_pip {
                    Some(current) => current.min(amount),
                    None => amount,
                });
            }
        }

        if has_mana_option {
            minimum_total += minimum_for_pip.unwrap_or(0);
        }

        if pip.len() == 1 {
            match pip[0] {
                ManaSymbol::White => required_white += 1,
                ManaSymbol::Blue => required_blue += 1,
                ManaSymbol::Black if !allow_black_life => required_black += 1,
                ManaSymbol::Red => required_red += 1,
                ManaSymbol::Green => required_green += 1,
                ManaSymbol::Colorless => required_colorless += 1,
                _ => {}
            }
        }
    }

    if potential.total() < minimum_total {
        return true;
    }

    if !allow_any_color
        && (potential.white < required_white
            || potential.blue < required_blue
            || potential.black < required_black
            || potential.red < required_red
            || potential.green < required_green
            || potential.colorless < required_colorless)
    {
        return true;
    }

    false
}

pub(crate) fn activated_ability_uses_simple_precheck(
    activated: &crate::ability::ActivatedAbility,
) -> bool {
    matches!(activated.timing, crate::ability::ActivationTiming::AnyTime)
        && activated.activation_condition.is_none()
        && activated.activation_restrictions.is_empty()
        && activated.effects.iter().all(|effect| {
            effect
                .modal_effect_spec()
                .is_none_or(|modal| !modal.disallow_previously_chosen_modes)
        })
}

// ============================================================================
// Game Progress
// ============================================================================

/// Result of advancing the game.
#[derive(Debug, Clone)]
pub enum GameProgress {
    /// Game needs a player decision using the new context-based system.
    /// This variant uses typed contexts that go directly to decide_* methods.
    NeedsDecisionCtx(crate::decisions::context::DecisionContext),
    /// Current phase/step has ended, game can continue.
    Continue,
    /// Game has ended.
    GameOver(GameResult),
    /// Stack item resolved, need to re-advance priority with decision maker.
    /// Used to signal the outer loop to handle triggers with proper targeting.
    StackResolved,
}

/// Result of a completed game.
#[derive(Debug, Clone)]
pub enum GameResult {
    /// A player won the game.
    Winner(PlayerId),
    /// The game ended in a draw.
    Draw,
    /// Multiple players remain (multiplayer game ended early).
    Remaining(Vec<PlayerId>),
}

// ============================================================================
// Error Types
// ============================================================================

/// Error when applying a player response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseError {
    /// No decision is pending.
    NoDecisionPending,
    /// Response type doesn't match the pending decision.
    WrongResponseType,
    /// The response is not a legal choice.
    IllegalChoice(String),
    /// Invalid target selection.
    InvalidTargets(String),
    /// Invalid attacker declaration.
    InvalidAttackers(String),
    /// Invalid blocker declaration.
    InvalidBlockers(String),
}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseError::NoDecisionPending => write!(f, "No decision is pending"),
            ResponseError::WrongResponseType => {
                write!(f, "Response type doesn't match pending decision")
            }
            ResponseError::IllegalChoice(msg) => write!(f, "Illegal choice: {}", msg),
            ResponseError::InvalidTargets(msg) => write!(f, "Invalid targets: {}", msg),
            ResponseError::InvalidAttackers(msg) => write!(f, "Invalid attackers: {}", msg),
            ResponseError::InvalidBlockers(msg) => write!(f, "Invalid blockers: {}", msg),
        }
    }
}

impl std::error::Error for ResponseError {}

// ============================================================================

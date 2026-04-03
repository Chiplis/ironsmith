//! Player decision system for MTG.
//!
//! This module provides:
//! - `DecisionMaker` and typed decision contexts for player input
//! - `LegalAction` and related types for describing legal game actions
//! - Helper functions to compute legal actions

use crate::alternative_cast::CastingMethod;
use crate::combat_state::{AttackTarget, CombatState};
use crate::derived_view::DerivedGameView;
use crate::effects::helpers::resolve_value;
use crate::executor::ExecutionContext;
use crate::game_state::{GameState, Phase, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::perf::PerfTimer;
use crate::special_actions::{SpecialAction, can_activate_mana_ability_check_with_view};
use crate::target::ChooseSpec;
use crate::targeting::normalize_targets_for_requirements;
use crate::zone::Zone;
use crate::{CounterType, ManaSymbol, Step};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::rc::Rc;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ComputeLegalActionsPerfMetrics {
    pub prewarm_ms: f64,
    pub lands_ms: f64,
    pub hand_casts_ms: f64,
    pub can_cast_spell_with_view_ms: f64,
    pub spell_has_legal_targets_ms: f64,
    pub compute_potential_mana_with_view_ms: f64,
    pub hand_casts_timing_ms: f64,
    pub hand_casts_restrictions_ms: f64,
    pub hand_casts_target_legality_ms: f64,
    pub hand_casts_cost_adjustment_ms: f64,
    pub hand_casts_affordability_ms: f64,
    pub hand_special_actions_ms: f64,
    pub graveyard_casts_ms: f64,
    pub exile_casts_ms: f64,
    pub hand_alternatives_ms: f64,
    pub battlefield_abilities_ms: f64,
    pub can_activate_ability_with_restrictions_with_view_ms: f64,
    pub battlefield_ability_precheck_ms: f64,
    pub battlefield_ability_target_legality_ms: f64,
    pub battlefield_ability_cost_build_ms: f64,
    pub battlefield_ability_affordability_ms: f64,
    pub non_battlefield_abilities_ms: f64,
    pub total_ms: f64,
    pub action_count: usize,
}

thread_local! {
    static LAST_COMPUTE_LEGAL_ACTIONS_PERF: RefCell<Option<ComputeLegalActionsPerfMetrics>> =
        const { RefCell::new(None) };
}

fn store_compute_legal_actions_perf(metrics: ComputeLegalActionsPerfMetrics) {
    LAST_COMPUTE_LEGAL_ACTIONS_PERF.with(|slot| {
        *slot.borrow_mut() = Some(metrics);
    });
}

pub fn last_compute_legal_actions_perf() -> Option<ComputeLegalActionsPerfMetrics> {
    LAST_COMPUTE_LEGAL_ACTIONS_PERF.with(|slot| slot.borrow().clone())
}

fn generic_mana_cost(amount: u32) -> crate::mana::ManaCost {
    if amount == 0 {
        return crate::mana::ManaCost::new();
    }

    let mut pips = Vec::new();
    let mut remaining = amount;
    while remaining > 0 {
        let chunk = remaining.min(u8::MAX as u32) as u8;
        pips.push(vec![ManaSymbol::Generic(chunk)]);
        remaining -= chunk as u32;
    }
    crate::mana::ManaCost::from_pips(pips)
}

fn static_abilities_for_attack_preview(
    view: &DerivedGameView<'_>,
    attacker: &crate::object::Object,
) -> Vec<crate::static_abilities::StaticAbility> {
    view.calculated_characteristics(attacker.id)
        .map(|chars| chars.static_abilities)
        .unwrap_or_else(|| {
            attacker
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    crate::ability::AbilityKind::Static(static_ability) => {
                        Some(static_ability.clone())
                    }
                    _ => None,
                })
                .collect()
        })
}

fn defending_player_for_attack_target(game: &GameState, target: &AttackTarget) -> Option<PlayerId> {
    match target {
        AttackTarget::Player(player) => Some(*player),
        AttackTarget::Planeswalker(planeswalker) => {
            game.object(*planeswalker).map(|object| object.controller)
        }
    }
}

fn generic_attack_tax_preview(
    game: &GameState,
    defending_player: PlayerId,
    view: &DerivedGameView<'_>,
) -> u32 {
    let mut tax = 0u32;

    for &object_id in &game.battlefield {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        if object.controller != defending_player {
            continue;
        }

        let abilities = view
            .calculated_characteristics(object_id)
            .map(|chars| chars.static_abilities)
            .unwrap_or_else(|| {
                object
                    .abilities
                    .iter()
                    .filter_map(|ability| match &ability.kind {
                        crate::ability::AbilityKind::Static(static_ability) => {
                            Some(static_ability.clone())
                        }
                        _ => None,
                    })
                    .collect()
            });

        tax = abilities.into_iter().fold(tax, |acc, ability| {
            acc.saturating_add(
                ability
                    .generic_attack_tax_per_attacker_against_you(game, object_id, defending_player)
                    .unwrap_or(0),
            )
        });
    }

    tax
}

fn can_declare_attack_target_preview(
    game: &GameState,
    attacker: &crate::object::Object,
    target: &AttackTarget,
    view: &DerivedGameView<'_>,
) -> bool {
    let Some(defending_player) = defending_player_for_attack_target(game, target) else {
        return false;
    };
    if !crate::rules::combat::can_attack_defending_player_with_view(
        attacker,
        defending_player,
        game,
        view,
    ) {
        return false;
    }

    let abilities = static_abilities_for_attack_preview(view, attacker);
    if abilities.iter().any(|ability| {
        ability
            .can_pay_attack_cost(game, attacker.id, attacker.controller)
            .is_some_and(|can_pay| !can_pay)
    }) {
        return false;
    }

    let total_generic_cost = abilities.iter().fold(
        generic_attack_tax_preview(game, defending_player, view),
        |acc, ability| {
            acc.saturating_add(
                ability
                    .generic_attack_mana_cost_for_source(game, attacker.id, attacker.controller)
                    .unwrap_or(0),
            )
        },
    );

    total_generic_cost == 0
        || game.can_pay_mana_cost(
            attacker.controller,
            None,
            &generic_mana_cost(total_generic_cost),
            0,
        )
}

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

    /// Use Serum Powder's pregame mulligan replacement from hand.
    SerumPowderMulligan { card_id: ObjectId },

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
    /// Legal targets that match this specification.
    pub legal_targets: Vec<Target>,
    /// Description of what's being targeted.
    pub description: String,
    /// Minimum number of targets to choose (default 1).
    pub min_targets: usize,
    /// Maximum number of targets to choose (None = unlimited, i.e., "any number").
    pub max_targets: Option<usize>,
}

impl TargetRequirement {
    /// Create a new targeting requirement for exactly one target.
    pub fn single(spec: ChooseSpec, legal_targets: Vec<Target>, description: String) -> Self {
        Self {
            spec,
            legal_targets,
            description,
            min_targets: 1,
            max_targets: Some(1),
        }
    }

    /// Create a new targeting requirement for any number of targets (0 or more).
    pub fn any_number(spec: ChooseSpec, legal_targets: Vec<Target>, description: String) -> Self {
        Self {
            spec,
            legal_targets,
            description,
            min_targets: 0,
            max_targets: None,
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
            legal_targets,
            description,
            min_targets: min,
            max_targets: max,
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

/// An option for paying mana.
#[derive(Debug, Clone)]
pub struct ManaPaymentOption {
    /// Index of this option.
    pub index: usize,
    /// Description of this payment method.
    pub description: String,
}

/// An option for paying a single mana pip.
///
/// This is used in the pip-by-pip payment flow, where each pip in a mana cost
/// is paid individually with explicit player choice.
#[derive(Debug, Clone)]
pub struct ManaPipPaymentOption {
    /// Index of this option.
    pub index: usize,
    /// Description of this payment method.
    pub description: String,
    /// The type of payment action.
    pub action: ManaPipPaymentAction,
}

/// The action to take when paying a mana pip.
#[derive(Debug, Clone)]
pub enum ManaPipPaymentAction {
    /// Use mana already in the pool.
    UseFromPool(crate::mana::ManaSymbol),
    /// Activate a mana ability on a permanent.
    ActivateManaAbility {
        /// The permanent with the mana ability.
        source_id: ObjectId,
        /// Index of the ability on that permanent.
        ability_index: usize,
    },
    /// Pay life (for Phyrexian mana).
    PayLife(u32),
    /// Pay this pip using a non-mana alternative (e.g., Convoke/Improvise).
    PayViaAlternative {
        /// The permanent used to pay the pip.
        permanent_id: ObjectId,
        /// Which alternative payment effect is being used.
        effect: AlternativePaymentEffect,
    },
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
struct HandCardSummary<'a> {
    card_id: ObjectId,
    card: &'a crate::object::Object,
    is_land: bool,
    has_normal_mana_cost: bool,
    has_foretell: bool,
    has_suspend: bool,
    has_plot: bool,
    can_cast_face_down: bool,
    has_split_other_half: bool,
    has_fuse: bool,
    has_hand_native_alternatives: bool,
}

impl<'a> HandCardSummary<'a> {
    fn has_any_hand_special_action(&self) -> bool {
        self.has_foretell || self.has_suspend || self.has_plot
    }

    fn has_any_alternative_branch(&self, has_hand_grants: bool) -> bool {
        self.can_cast_face_down
            || self.has_split_other_half
            || self.has_fuse
            || self.has_hand_native_alternatives
            || has_hand_grants
    }
}

fn spell_has_intrinsic_cost_adjustments(spell: &crate::object::Object) -> bool {
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
struct CastLegalityPerfBreakdown {
    total_ms: f64,
    timing_ms: f64,
    restrictions_ms: f64,
    target_legality_ms: f64,
    cost_adjustment_ms: f64,
    affordability_ms: f64,
}

struct CastLegalityContext<'a> {
    game: &'a GameState,
    player: PlayerId,
    view: &'a DerivedGameView<'a>,
    has_battlefield_spell_cost_modifiers: bool,
    has_temporary_spell_cost_reductions: bool,
    minimum_total_spell_mana_payment: Option<u32>,
    strips_life_pips_for_casts: bool,
    perf: RefCell<CastLegalityPerfBreakdown>,
}

impl<'a> CastLegalityContext<'a> {
    fn new(game: &'a GameState, player: PlayerId, view: &'a DerivedGameView<'a>) -> Self {
        Self {
            game,
            player,
            view,
            has_battlefield_spell_cost_modifiers: view.has_battlefield_spell_cost_modifiers(),
            has_temporary_spell_cost_reductions: game
                .temporary_spell_cost_reductions
                .iter()
                .any(|effect| effect.player == player && !effect.is_expired(game.turn.turn_number)),
            minimum_total_spell_mana_payment: game.minimum_total_spell_mana_payment(),
            strips_life_pips_for_casts: game.player_cant_pay_life_to_cast_or_activate(player),
            perf: RefCell::new(CastLegalityPerfBreakdown::default()),
        }
    }

    fn snapshot_perf(&self) -> CastLegalityPerfBreakdown {
        self.perf.borrow().clone()
    }

    fn add_total_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().total_ms += elapsed_ms;
    }

    fn add_timing_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().timing_ms += elapsed_ms;
    }

    fn add_restrictions_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().restrictions_ms += elapsed_ms;
    }

    fn add_target_legality_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().target_legality_ms += elapsed_ms;
    }

    fn add_cost_adjustment_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().cost_adjustment_ms += elapsed_ms;
    }

    fn add_affordability_ms(&self, elapsed_ms: f64) {
        self.perf.borrow_mut().affordability_ms += elapsed_ms;
    }

    fn spell_cost_needs_adjustment(&self, has_intrinsic_cost_adjustments: bool) -> bool {
        has_intrinsic_cost_adjustments
            || self.has_battlefield_spell_cost_modifiers
            || self.has_temporary_spell_cost_reductions
            || self.minimum_total_spell_mana_payment.is_some()
    }

    fn can_use_printed_cost_directly(&self, has_intrinsic_cost_adjustments: bool) -> bool {
        !has_intrinsic_cost_adjustments
            && !self.has_battlefield_spell_cost_modifiers
            && !self.has_temporary_spell_cost_reductions
            && self.minimum_total_spell_mana_payment.is_none()
            && !self.strips_life_pips_for_casts
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ActivationSourceFacts {
    controller: PlayerId,
    can_activate_abilities: bool,
    can_activate_tap_abilities: bool,
    can_activate_non_mana_abilities_of_source: bool,
    is_tapped: bool,
    is_creature: bool,
    is_summoning_sick: bool,
    has_haste: bool,
}

impl ActivationSourceFacts {
    fn for_source(game: &GameState, source: ObjectId, view: &DerivedGameView<'_>) -> Self {
        let controller = game
            .object(source)
            .map(|obj| obj.controller)
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
struct BattlefieldAbilityPerfBreakdown {
    total_ms: f64,
    precheck_ms: f64,
    target_legality_ms: f64,
    cost_build_ms: f64,
    affordability_ms: f64,
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

    fn snapshot_perf(&self) -> BattlefieldAbilityPerfBreakdown {
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

fn mana_cost_is_obviously_unpayable(
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

    if !allow_any_color {
        if potential.white < required_white
            || potential.blue < required_blue
            || potential.black < required_black
            || potential.red < required_red
            || potential.green < required_green
            || potential.colorless < required_colorless
        {
            return true;
        }
    }

    false
}

fn activated_ability_uses_simple_precheck(activated: &crate::ability::ActivatedAbility) -> bool {
    matches!(activated.timing, crate::ability::ActivationTiming::AnyTime)
        && activated.activation_condition.is_none()
        && activated.activation_restrictions.is_empty()
        && activated.effects.iter().all(|effect| {
            effect
                .downcast_ref::<crate::effects::ChooseModeEffect>()
                .is_none_or(|choose_mode| !choose_mode.disallow_previously_chosen_modes)
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
// Helper Functions
// ============================================================================

fn append_granted_play_from_actions_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    source_zone: Zone,
    view: &DerivedGameView<'_>,
) {
    let play_from_grants = view.granted_play_from_for_card(card_id, source_zone, player);
    for grant in play_from_grants {
        // PlayFrom (e.g., Yawgmoth's Will): can cast from zone as if from hand.
        let from_zone = grant.zone;
        let granted_alternatives =
            view.granted_alternative_casts_for_card(card_id, from_zone, player);
        let has_same_source_granted_alternative = granted_alternatives
            .iter()
            .any(|granted_alt| granted_alt.source_id == grant.source_id);

        if !has_same_source_granted_alternative
            && !card.is_land()
            && let Some(mana_cost) = &card.mana_cost
            && can_cast_with_cost_with_view(
                game,
                player,
                card,
                card_id,
                Some(mana_cost),
                None,
                &AdditionalCastRequirements::default(),
                view,
            )
        {
            actions.push(LegalAction::CastSpell {
                spell_id: card_id,
                from_zone,
                casting_method: CastingMethod::PlayFrom {
                    source: grant.source_id,
                    zone: from_zone,
                    use_alternative: None,
                },
            });
        }

        for (idx, alt_cast) in card.alternative_casts.iter().enumerate() {
            if alt_cast.cast_from_zone() == Zone::Hand
                && can_cast_with_alternative_from_hand_with_view(
                    game, player, card, card_id, alt_cast, view,
                )
            {
                actions.push(LegalAction::CastSpell {
                    spell_id: card_id,
                    from_zone,
                    casting_method: CastingMethod::PlayFrom {
                        source: grant.source_id,
                        zone: from_zone,
                        use_alternative: Some(idx),
                    },
                });
            }
        }

        let base_alt_idx = card.alternative_casts.len();
        for (offset, granted_alt) in granted_alternatives.iter().enumerate() {
            if can_cast_with_alternative_with_view(game, player, card, &granted_alt.method, view) {
                actions.push(LegalAction::CastSpell {
                    spell_id: card_id,
                    from_zone,
                    casting_method: CastingMethod::PlayFrom {
                        source: granted_alt.source_id,
                        zone: from_zone,
                        use_alternative: Some(base_alt_idx + offset),
                    },
                });
            }
        }
    }
}

fn append_native_alternative_cast_actions_for_card_from_zone(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    from_zone: Zone,
    view: &DerivedGameView<'_>,
) {
    for (idx, alt_cast) in card.alternative_casts.iter().enumerate() {
        if alt_cast.cast_from_zone() == from_zone
            && can_cast_with_alternative_with_view(game, player, card, alt_cast, view)
        {
            actions.push(LegalAction::CastSpell {
                spell_id: card_id,
                from_zone,
                casting_method: CastingMethod::Alternative(idx),
            });
        }
    }
}

fn append_graveyard_granted_alternative_cast_actions_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    view: &DerivedGameView<'_>,
) {
    let granted_casts = view.granted_alternative_casts_for_card(card_id, Zone::Graveyard, player);

    for grant in granted_casts {
        let method = &grant.method;
        let requirements = build_requirements_for_method(method);
        let mana_cost = get_mana_cost_for_method(method, card);

        if !can_cast_with_cost_with_view(
            game,
            player,
            card,
            card_id,
            mana_cost,
            None,
            &requirements,
            view,
        ) {
            continue;
        }

        let casting_method = match method {
            crate::alternative_cast::AlternativeCastingMethod::Escape { exile_count, .. } => {
                CastingMethod::GrantedEscape {
                    source: grant.source_id,
                    exile_count: *exile_count,
                }
            }
            crate::alternative_cast::AlternativeCastingMethod::Flashback { .. } => {
                CastingMethod::GrantedFlashback
            }
            _ => continue,
        };

        actions.push(LegalAction::CastSpell {
            spell_id: card_id,
            from_zone: Zone::Graveyard,
            casting_method,
        });
    }
}

fn append_hand_granted_alternative_cast_actions_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    view: &DerivedGameView<'_>,
) {
    if card.is_land() {
        return;
    }

    let granted_casts = view.granted_alternative_casts_for_card(card_id, Zone::Hand, player);
    let base_alt_idx = card.alternative_casts.len();

    for (offset, grant) in granted_casts.iter().enumerate() {
        if grant.method.cast_from_zone() != Zone::Hand
            || !can_cast_with_alternative_from_hand_with_view(
                game,
                player,
                card,
                card_id,
                &grant.method,
                view,
            )
        {
            continue;
        }

        actions.push(LegalAction::CastSpell {
            spell_id: card_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::PlayFrom {
                source: grant.source_id,
                zone: Zone::Hand,
                use_alternative: Some(base_alt_idx + offset),
            },
        });
    }
}

fn append_cast_actions_from_zone_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    from_zone: Zone,
    view: &DerivedGameView<'_>,
    zone_has_active_grants: bool,
) {
    append_native_alternative_cast_actions_for_card_from_zone(
        game, actions, player, card_id, card, from_zone, view,
    );
    if zone_has_active_grants && from_zone == Zone::Graveyard {
        append_graveyard_granted_alternative_cast_actions_for_card(
            game, actions, player, card_id, card, view,
        );
    }
    if zone_has_active_grants {
        append_granted_play_from_actions_for_card(
            game, actions, player, card_id, card, from_zone, view,
        );
    }
}

fn append_granted_land_play_actions_from_public_zone(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    zone: Zone,
    view: &DerivedGameView<'_>,
) {
    for card_id in crate::object_query::candidate_ids_for_zone(game, Some(zone)) {
        let Some(card) = game.object(card_id) else {
            continue;
        };
        if !card.is_land() {
            continue;
        }
        if view
            .granted_play_from_for_card(card_id, zone, player)
            .is_empty()
        {
            continue;
        }

        let action = SpecialAction::PlayLand { card_id };
        if crate::special_actions::can_perform_check(&action, game, player).is_ok() {
            actions.push(LegalAction::PlayLand { land_id: card_id });
        }
    }
}

/// Compute legal actions for a player who has priority.
///
/// This validates each potential action by testing it against the actual game rules.
/// Only actions that would succeed are included in the result.
pub fn compute_legal_actions(game: &GameState, player: PlayerId) -> Vec<LegalAction> {
    use crate::special_actions::{SpecialAction, can_perform_check};
    use std::collections::HashSet;

    let total_started_at = PerfTimer::start();
    let mut perf = ComputeLegalActionsPerfMetrics::default();
    let empty_zone: &[ObjectId] = &[];
    let (hand, graveyard) = game
        .player(player)
        .map_or((empty_zone, empty_zone), |player_obj| {
            (player_obj.hand.as_slice(), player_obj.graveyard.as_slice())
        });
    let mut actions = Vec::with_capacity(
        1 + hand.len() * 6
            + graveyard.len() * 2
            + game.exile.len() * 2
            + game.battlefield.len() * 4,
    );
    let mut hand_cards_with_normal_cast = HashSet::new();
    let view = DerivedGameView::new(game);
    let cast_ctx = CastLegalityContext::new(game, player, &view);
    let battlefield_ability_ctx = BattlefieldAbilityContext::new(&view);
    let hand_has_active_grants = view.player_has_active_grants_for_zone(player, Zone::Hand);
    let graveyard_has_active_grants =
        view.player_has_active_grants_for_zone(player, Zone::Graveyard);
    let exile_has_active_grants = view.player_has_active_grants_for_zone(player, Zone::Exile);
    let hand_summaries: Vec<_> = hand
        .iter()
        .filter_map(|&card_id| {
            let card = game.object(card_id)?;
            let mut has_foretell = false;
            let mut has_suspend = false;
            let mut has_plot = false;
            let mut has_hand_native_alternatives = false;
            for method in &card.alternative_casts {
                match method {
                    crate::alternative_cast::AlternativeCastingMethod::Foretell { .. } => {
                        has_foretell = true;
                    }
                    crate::alternative_cast::AlternativeCastingMethod::Suspend { .. } => {
                        has_suspend = true;
                    }
                    crate::alternative_cast::AlternativeCastingMethod::Plot { .. } => {
                        has_plot = true;
                    }
                    _ => {}
                }
                if method.cast_from_zone() == Zone::Hand {
                    has_hand_native_alternatives = true;
                }
            }
            Some(HandCardSummary {
                card_id,
                card,
                is_land: card.is_land(),
                has_normal_mana_cost: card.mana_cost.is_some(),
                has_foretell,
                has_suspend,
                has_plot,
                can_cast_face_down: spell_can_be_cast_face_down(card),
                has_split_other_half: card.linked_face_layout
                    == crate::card::LinkedFaceLayout::Split,
                has_fuse: card.has_fuse,
                has_hand_native_alternatives,
            })
        })
        .collect();
    let controlled_battlefield: Vec<_> = game
        .battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id)
                .is_some_and(|object| object.controller == player)
        })
        .collect();
    let prewarm_started_at = PerfTimer::start();
    view.prewarm_characteristics(&controlled_battlefield);
    perf.prewarm_ms = prewarm_started_at.elapsed_ms();

    // Can always pass priority
    actions.push(LegalAction::PassPriority);

    // Check for lands that can be played - validate each using special_actions::can_perform_check
    let lands_started_at = PerfTimer::start();
    for summary in &hand_summaries {
        if summary.is_land {
            let action = SpecialAction::PlayLand {
                card_id: summary.card_id,
            };
            if can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::PlayLand {
                    land_id: summary.card_id,
                });
            }
        }
    }
    if graveyard_has_active_grants {
        append_granted_land_play_actions_from_public_zone(
            game,
            &mut actions,
            player,
            Zone::Graveyard,
            &view,
        );
    }
    if exile_has_active_grants {
        append_granted_land_play_actions_from_public_zone(
            game,
            &mut actions,
            player,
            Zone::Exile,
            &view,
        );
    }
    perf.lands_ms = lands_started_at.elapsed_ms();

    // Check for spells that can be cast from hand
    let hand_casts_started_at = PerfTimer::start();
    for summary in &hand_summaries {
        if summary.is_land || !summary.has_normal_mana_cost {
            continue;
        }
        let can_cast_normal =
            can_cast_spell_with_context(summary.card, &CastingMethod::Normal, &cast_ctx);
        if can_cast_normal {
            actions.push(LegalAction::CastSpell {
                spell_id: summary.card_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            });
            hand_cards_with_normal_cast.insert(summary.card_id);
        }
    }
    perf.hand_casts_ms = hand_casts_started_at.elapsed_ms();

    // Check for foretell special actions from hand.
    let hand_special_actions_started_at = PerfTimer::start();
    for summary in &hand_summaries {
        if !summary.has_any_hand_special_action() {
            continue;
        }
        if summary.has_foretell {
            let action = SpecialAction::Foretell {
                card_id: summary.card_id,
            };
            if can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::SpecialAction(action));
            }
        }
        if summary.has_suspend {
            let action = SpecialAction::Suspend {
                card_id: summary.card_id,
            };
            if can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::SpecialAction(action));
            }
        }
        if summary.has_plot {
            let action = SpecialAction::Plot {
                card_id: summary.card_id,
            };
            if can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::SpecialAction(action));
            }
        }
    }
    perf.hand_special_actions_ms = hand_special_actions_started_at.elapsed_ms();

    // Check for spells that can be cast from graveyard.
    // Includes native alternatives, granted alternatives, and PlayFrom grants.
    let graveyard_casts_started_at = PerfTimer::start();
    for &card_id in graveyard {
        if let Some(card) = game.object(card_id) {
            append_cast_actions_from_zone_for_card(
                game,
                &mut actions,
                player,
                card_id,
                card,
                Zone::Graveyard,
                &view,
                graveyard_has_active_grants,
            );
        }
    }
    perf.graveyard_casts_ms = graveyard_casts_started_at.elapsed_ms();

    // Check for cards that can be played/cast from exile.
    let exile_casts_started_at = PerfTimer::start();
    for &card_id in &game.exile {
        let Some(card) = game.object(card_id) else {
            continue;
        };
        append_cast_actions_from_zone_for_card(
            game,
            &mut actions,
            player,
            card_id,
            card,
            Zone::Exile,
            &view,
            exile_has_active_grants,
        );
    }
    perf.exile_casts_ms = exile_casts_started_at.elapsed_ms();

    // Check for alternative casting methods from hand (e.g., Force of Will's alternative cost)
    // Only add these if the Normal method is NOT available for this spell.
    // When both are available, the game will prompt for method selection via ChooseCastingMethod.
    let hand_alternatives_started_at = PerfTimer::start();
    for summary in &hand_summaries {
        // Only add alternative casts if Normal is not available
        if hand_cards_with_normal_cast.contains(&summary.card_id)
            || summary.is_land
            || !summary.has_any_alternative_branch(hand_has_active_grants)
        {
            continue;
        }
        if summary.can_cast_face_down
            && can_cast_spell_with_context(summary.card, &CastingMethod::FaceDown, &cast_ctx)
        {
            actions.push(LegalAction::CastSpell {
                spell_id: summary.card_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::FaceDown,
            });
        }
        if summary.has_split_other_half
            && can_cast_spell_with_context(summary.card, &CastingMethod::SplitOtherHalf, &cast_ctx)
        {
            actions.push(LegalAction::CastSpell {
                spell_id: summary.card_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::SplitOtherHalf,
            });
        }
        if summary.has_split_other_half
            && summary.has_fuse
            && can_cast_spell_with_context(summary.card, &CastingMethod::Fuse, &cast_ctx)
        {
            actions.push(LegalAction::CastSpell {
                spell_id: summary.card_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Fuse,
            });
        }
        if summary.has_hand_native_alternatives {
            for (idx, alt_cast) in summary.card.alternative_casts.iter().enumerate() {
                if alt_cast.cast_from_zone() == Zone::Hand
                    && can_cast_with_alternative_from_hand_with_context(
                        summary.card,
                        summary.card_id,
                        alt_cast,
                        &cast_ctx,
                    )
                {
                    actions.push(LegalAction::CastSpell {
                        spell_id: summary.card_id,
                        from_zone: Zone::Hand,
                        casting_method: CastingMethod::Alternative(idx),
                    });
                }
            }
        }
        if hand_has_active_grants {
            append_hand_granted_alternative_cast_actions_for_card(
                game,
                &mut actions,
                player,
                summary.card_id,
                summary.card,
                &view,
            );
        }
    }
    perf.hand_alternatives_ms = hand_alternatives_started_at.elapsed_ms();

    // Check for face-down permanents that can be turned face up.
    let battlefield_abilities_started_at = PerfTimer::start();
    for &perm_id in &controlled_battlefield {
        if game.is_face_down(perm_id) {
            for method in crate::special_actions::available_turn_face_up_methods(game, perm_id) {
                let action = SpecialAction::TurnFaceUp {
                    permanent_id: perm_id,
                    method,
                };
                if can_perform_check(&action, game, player).is_ok() {
                    actions.push(LegalAction::TurnFaceUp {
                        creature_id: perm_id,
                        method,
                    });
                }
            }
        }
    }

    // Check for mana and activated abilities on relevant permanents.
    let simple_mana_analysis = view.simple_battlefield_mana_analysis(player);
    for &perm_id in simple_mana_analysis.relevant_source_ids() {
        if let Some(perm) = game.object(perm_id) {
            let source_facts = ActivationSourceFacts::for_source(game, perm_id, &view);
            let cached_abilities = view.abilities_rc(perm_id);
            let abilities = cached_abilities.as_deref().unwrap_or(&perm.abilities);
            let mana_ability_indices = simple_mana_analysis.mana_ability_indices_for(perm_id);
            let activated_ability_indices =
                simple_mana_analysis.activated_ability_indices_for(perm_id);
            if mana_ability_indices.is_empty() && activated_ability_indices.is_empty() {
                continue;
            };

            for &ability_index in mana_ability_indices {
                let Some(ability) = abilities.get(ability_index) else {
                    continue;
                };
                if simple_mana_analysis
                    .activatable_indices_for(perm_id)
                    .contains(&ability_index)
                {
                    actions.push(LegalAction::ActivateManaAbility {
                        source: perm_id,
                        ability_index,
                    });
                } else if can_activate_mana_ability_check_with_view(
                    game,
                    player,
                    perm_id,
                    ability_index,
                    ability,
                    &view,
                    Some(&battlefield_ability_ctx),
                )
                .is_ok()
                {
                    actions.push(LegalAction::ActivateManaAbility {
                        source: perm_id,
                        ability_index,
                    });
                }
            }

            if game.can_activate_non_mana_abilities(player) {
                for &ability_index in activated_ability_indices {
                    let Some(ability) = abilities.get(ability_index) else {
                        continue;
                    };
                    let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
                        continue;
                    };
                    if can_activate_ability_with_restrictions_with_view(
                        game,
                        perm_id,
                        ability_index,
                        activated,
                        &view,
                        Some(&battlefield_ability_ctx),
                        Some(&source_facts),
                    ) {
                        actions.push(LegalAction::ActivateAbility {
                            source: perm_id,
                            ability_index,
                        });
                    }
                }
            }
        }
    }
    perf.battlefield_abilities_ms = battlefield_abilities_started_at.elapsed_ms();
    let battlefield_breakdown = battlefield_ability_ctx.snapshot_perf();
    perf.can_activate_ability_with_restrictions_with_view_ms = battlefield_breakdown.total_ms;
    perf.battlefield_ability_precheck_ms = battlefield_breakdown.precheck_ms;
    perf.battlefield_ability_target_legality_ms = battlefield_breakdown.target_legality_ms;
    perf.battlefield_ability_cost_build_ms = battlefield_breakdown.cost_build_ms;
    perf.battlefield_ability_affordability_ms = battlefield_breakdown.affordability_ms;

    // Check for activated abilities that function outside the battlefield
    // (e.g., Unearth from graveyard, Cycling/Ninjutsu from hand).
    let non_battlefield_abilities_started_at = PerfTimer::start();
    let mut non_battlefield_ids = Vec::with_capacity(
        hand.len() + graveyard.len() + game.exile.len() + game.command_zone.len(),
    );
    non_battlefield_ids.extend(hand.iter().copied());
    non_battlefield_ids.extend(graveyard.iter().copied());
    non_battlefield_ids.extend(
        game.exile
            .iter()
            .copied()
            .filter(|id| game.object(*id).is_some_and(|obj| obj.owner == player)),
    );
    non_battlefield_ids.extend(
        game.command_zone
            .iter()
            .copied()
            .filter(|id| game.object(*id).is_some_and(|obj| obj.owner == player)),
    );
    non_battlefield_ids.sort_by_key(|id| id.0);
    non_battlefield_ids.dedup();

    for source_id in non_battlefield_ids {
        let Some(obj) = game.object(source_id) else {
            continue;
        };
        if obj.zone == Zone::Battlefield || obj.controller != player {
            continue;
        }

        let Some(ability_summary) = view.ability_index_summary(source_id) else {
            continue;
        };
        if !ability_summary.has_any_relevant_abilities() {
            continue;
        }

        for &ability_index in ability_summary.mana_ability_indices() {
            // Mana abilities from non-battlefield zones (e.g. Simian Spirit Guide)
            let action = SpecialAction::ActivateManaAbility {
                permanent_id: source_id,
                ability_index,
            };
            if can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::ActivateManaAbility {
                    source: source_id,
                    ability_index,
                });
            }
        }

        if game.can_activate_non_mana_abilities(player) {
            for &ability_index in ability_summary.activated_ability_indices() {
                let Some(ability) = obj.abilities.get(ability_index) else {
                    continue;
                };
                let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
                    continue;
                };
                if can_activate_ability_with_restrictions_with_view(
                    game,
                    source_id,
                    ability_index,
                    activated,
                    &view,
                    None,
                    None,
                ) {
                    actions.push(LegalAction::ActivateAbility {
                        source: source_id,
                        ability_index,
                    });
                }
            }
        }
    }

    perf.non_battlefield_abilities_ms = non_battlefield_abilities_started_at.elapsed_ms();
    let cast_breakdown = cast_ctx.snapshot_perf();
    perf.can_cast_spell_with_view_ms = cast_breakdown.total_ms;
    perf.spell_has_legal_targets_ms = cast_breakdown.target_legality_ms;
    perf.compute_potential_mana_with_view_ms = view.potential_mana_compute_ms();
    perf.hand_casts_timing_ms = cast_breakdown.timing_ms;
    perf.hand_casts_restrictions_ms = cast_breakdown.restrictions_ms;
    perf.hand_casts_target_legality_ms = cast_breakdown.target_legality_ms;
    perf.hand_casts_cost_adjustment_ms = cast_breakdown.cost_adjustment_ms;
    perf.hand_casts_affordability_ms = cast_breakdown.affordability_ms;
    perf.total_ms = total_started_at.elapsed_ms();
    perf.action_count = actions.len();
    store_compute_legal_actions_perf(perf);
    actions
}

/// Returns whether an activated ability can be used right now based on per-turn
/// limits and textual activation restrictions parsed from Oracle text.
pub(crate) fn can_activate_ability_with_restrictions(
    game: &GameState,
    source: ObjectId,
    ability_index: usize,
    activated: &crate::ability::ActivatedAbility,
) -> bool {
    let view = DerivedGameView::new(game);
    can_activate_ability_with_restrictions_with_view(
        game,
        source,
        ability_index,
        activated,
        &view,
        None,
        None,
    )
}

fn activated_ability_has_legal_targets_with_view(
    activated: &crate::ability::ActivatedAbility,
    controller: PlayerId,
    source: ObjectId,
    view: &DerivedGameView<'_>,
) -> bool {
    let effects = activated.effects.flattened_default_effects();
    effects.is_empty() || view.spell_has_legal_targets(effects, controller, Some(source), None)
}

fn activation_timing_allows(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    ability_index: usize,
    timing: &crate::ability::ActivationTiming,
) -> bool {
    match timing {
        crate::ability::ActivationTiming::AnyTime => true,
        crate::ability::ActivationTiming::DuringCombat => matches!(game.turn.phase, Phase::Combat),
        crate::ability::ActivationTiming::SorcerySpeed => {
            game.turn.active_player == controller
                && matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain)
                && game.stack_is_empty()
        }
        crate::ability::ActivationTiming::OncePerTurn => {
            game.ability_activation_count_this_turn(source, ability_index) == 0
        }
        crate::ability::ActivationTiming::DuringYourTurn => game.turn.active_player == controller,
        crate::ability::ActivationTiming::DuringOpponentsTurn => {
            game.turn.active_player != controller
        }
    }
}

fn activation_cost_component_precheck_with_view(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    cost: &crate::costs::Cost,
    reason: crate::costs::PaymentReason,
    _view: &DerivedGameView<'_>,
) -> bool {
    if let Some(amount) = cost.life_amount() {
        return game.can_pay_life_with_reason(controller, amount, reason);
    }

    if let Some((count, card_type)) = cost.discard_details() {
        let Some(player) = game.player(controller) else {
            return false;
        };
        let available = player
            .hand
            .iter()
            .filter_map(|object_id| game.object(*object_id))
            .filter(|object| {
                card_type.is_none_or(|required_type| object.card_types.contains(&required_type))
            })
            .count();
        return available >= count as usize;
    }

    game.validate_cost_for_payment_reason(controller, source, cost, reason)
        .is_ok()
}

fn activation_precheck_with_view(
    game: &GameState,
    source: ObjectId,
    ability_index: usize,
    activated: &crate::ability::ActivatedAbility,
    view: &DerivedGameView<'_>,
    perf_ctx: Option<&BattlefieldAbilityContext>,
    source_facts: Option<&ActivationSourceFacts>,
) -> Option<PlayerId> {
    let started_at = PerfTimer::start();
    let owned_facts;
    let source_facts = if let Some(source_facts) = source_facts {
        source_facts
    } else {
        owned_facts = ActivationSourceFacts::for_source(game, source, view);
        &owned_facts
    };
    let controller = source_facts.controller;

    if let Some(obj) = game.object(source)
        && !game.can_activate_non_mana_abilities(obj.controller)
    {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if !source_facts.can_activate_abilities {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    if activated.has_tap_cost() && !source_facts.can_activate_tap_abilities {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    if activated.has_tap_cost() && source_facts.is_tapped {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    let has_untap_cost = activated
        .mana_cost
        .costs()
        .iter()
        .any(|cost| cost.requires_untap());
    if (activated.has_tap_cost() || has_untap_cost)
        && source_facts.is_creature
        && source_facts.is_summoning_sick
        && !source_facts.has_haste
    {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    if activated
        .mana_cost
        .costs()
        .iter()
        .any(|cost| cost.requires_untap())
        && !source_facts.is_tapped
    {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    if !activated.is_mana_ability() && !source_facts.can_activate_non_mana_abilities_of_source {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if activated_ability_uses_simple_precheck(activated) {
        if !activation_timing_allows(game, controller, source, ability_index, &activated.timing) {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }

        if let Some(max_activations) = activated.max_activations_per_turn()
            && game.ability_activation_count_this_turn(source, ability_index) >= max_activations
        {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }

        for cost in activated.mana_cost.costs() {
            if !activation_cost_component_precheck_with_view(
                game,
                controller,
                source,
                cost,
                crate::costs::PaymentReason::ActivateAbility,
                view,
            ) {
                if let Some(perf_ctx) = perf_ctx {
                    perf_ctx.add_precheck_ms(started_at.elapsed_ms());
                }
                return None;
            }
        }

        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return Some(controller);
    }

    let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
        controller,
        source,
        defending_player: None,
        attacking_player: None,
        filter_source: Some(source),
        triggering_event: None,
        trigger_identity: None,
        ability_index: Some(ability_index),
        options: Default::default(),
    };

    if !activation_timing_allows(game, controller, source, ability_index, &activated.timing) {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if !matches!(
        activated.timing,
        crate::ability::ActivationTiming::OncePerTurn
    ) && let Some(max_activations) = activated.max_activations_per_turn()
        && game.ability_activation_count_this_turn(source, ability_index) >= max_activations
    {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if let Some(condition) = &activated.activation_condition
        && !crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
    {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    for cost in activated.mana_cost.costs() {
        if !activation_cost_component_precheck_with_view(
            game,
            controller,
            source,
            cost,
            crate::costs::PaymentReason::ActivateAbility,
            view,
        ) {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }
    }

    for condition in &activated.activation_restrictions {
        if !crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx) {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }
    }

    for effect in &activated.effects {
        if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>()
            && choose_mode.disallow_previously_chosen_modes
            && !game.ability_has_unchosen_mode(
                source,
                ability_index,
                choose_mode.modes.len(),
                choose_mode.disallow_previously_chosen_modes_this_turn,
            )
        {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }
    }

    if let Some(perf_ctx) = perf_ctx {
        perf_ctx.add_precheck_ms(started_at.elapsed_ms());
    }
    Some(eval_ctx.controller)
}

pub(crate) fn can_activate_ability_with_restrictions_with_view(
    game: &GameState,
    source: ObjectId,
    ability_index: usize,
    activated: &crate::ability::ActivatedAbility,
    view: &DerivedGameView<'_>,
    perf_ctx: Option<&BattlefieldAbilityContext>,
    source_facts: Option<&ActivationSourceFacts>,
) -> bool {
    let total_started_at = PerfTimer::start();
    let Some(controller) = activation_precheck_with_view(
        game,
        source,
        ability_index,
        activated,
        view,
        perf_ctx,
        source_facts,
    ) else {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_total_ms(total_started_at.elapsed_ms());
        }
        return false;
    };

    let target_started_at = PerfTimer::start();
    let has_legal_targets =
        activated_ability_has_legal_targets_with_view(activated, controller, source, view);
    if let Some(perf_ctx) = perf_ctx {
        perf_ctx.add_target_legality_ms(target_started_at.elapsed_ms());
    }
    if !has_legal_targets {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_total_ms(total_started_at.elapsed_ms());
        }
        return false;
    }

    let cost_started_at = PerfTimer::start();
    let has_activation_cost_modifiers = perf_ctx
        .map(BattlefieldAbilityContext::has_activation_cost_modifiers)
        .unwrap_or_else(|| view.has_activated_ability_cost_modifiers());
    if !has_activation_cost_modifiers {
        // The precheck already validated the printed activation costs, so when
        // nothing can modify them we can stop after target legality.
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_total_ms(total_started_at.elapsed_ms());
        }
        return true;
    }

    let total_cost = {
        calculate_effective_activation_total_cost_with_view(
            game,
            controller,
            source,
            &activated.mana_cost,
            &[],
            view,
        )
    };
    if let Some(perf_ctx) = perf_ctx {
        perf_ctx.add_cost_build_ms(cost_started_at.elapsed_ms());
    }
    for cost in total_cost.costs() {
        if game
            .validate_cost_for_payment_reason(
                controller,
                source,
                cost,
                crate::costs::PaymentReason::ActivateAbility,
            )
            .is_err()
        {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_total_ms(total_started_at.elapsed_ms());
            }
            return false;
        }
    }

    if let Some(perf_ctx) = perf_ctx {
        perf_ctx.add_total_ms(total_started_at.elapsed_ms());
    }
    true
}

/// Compute legal commander actions for a player (casting from command zone).
///
/// These are kept separate from regular legal actions so they can be accessed
/// via 'C' input rather than numeric indices.
pub fn compute_commander_actions(game: &GameState, player: PlayerId) -> Vec<LegalAction> {
    let mut actions = Vec::new();
    let view = DerivedGameView::new(game);

    // Check for commanders that can be cast from command zone
    if let Some(player_obj) = game.player(player) {
        for &commander_id in player_obj.get_commanders() {
            if let Some(current_id) = game.current_commander_object(commander_id)
                && let Some(commander) = game.object(current_id)
            {
                // Only if the commander is in the command zone
                if commander.zone == Zone::Command
                    && can_cast_spell_with_view(
                        game,
                        player,
                        commander,
                        &CastingMethod::Normal,
                        &view,
                    )
                {
                    actions.push(LegalAction::CastSpell {
                        spell_id: current_id,
                        from_zone: Zone::Command,
                        casting_method: CastingMethod::Normal,
                    });
                }
            }
        }
    }

    actions
}

fn commander_action_indices(actions: &[LegalAction]) -> Vec<usize> {
    actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| match action {
            LegalAction::CastSpell {
                from_zone: Zone::Command,
                ..
            } => Some(index),
            _ => None,
        })
        .collect()
}

/// Calculate activated-ability cost after applying battlefield static cost modifiers.
pub fn calculate_effective_activation_total_cost(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    cost: &crate::cost::TotalCost,
) -> crate::cost::TotalCost {
    calculate_effective_activation_total_cost_with_chosen_targets(
        game,
        activator,
        ability_source,
        cost,
        &[],
    )
}

pub fn calculate_effective_activation_total_cost_with_chosen_targets(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    cost: &crate::cost::TotalCost,
    chosen_targets: &[Target],
) -> crate::cost::TotalCost {
    let view = DerivedGameView::new(game);
    calculate_effective_activation_total_cost_with_view(
        game,
        activator,
        ability_source,
        cost,
        chosen_targets,
        &view,
    )
}

pub(crate) fn calculate_effective_activation_total_cost_with_view(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    cost: &crate::cost::TotalCost,
    chosen_targets: &[Target],
    view: &DerivedGameView<'_>,
) -> crate::cost::TotalCost {
    let mut costs = Vec::with_capacity(cost.costs().len());
    for component in cost.costs() {
        if let Some(mana_cost) = component.mana_cost_ref() {
            let reduced = calculate_effective_activation_mana_cost_with_view(
                game,
                activator,
                ability_source,
                mana_cost,
                chosen_targets,
                view,
            );
            costs.push(crate::costs::Cost::mana(reduced));
        } else {
            costs.push(component.clone());
        }
    }
    crate::cost::TotalCost::from_costs(costs)
}

/// Calculate the effective mana portion of an activated ability's cost.
pub fn calculate_effective_activation_mana_cost(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    base_cost: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_activation_mana_cost_with_view(
        game,
        activator,
        ability_source,
        base_cost,
        &[],
        &view,
    )
}

fn calculate_effective_activation_mana_cost_with_view(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;
    use crate::filter::FilterContext;

    fn opponents_of(game: &GameState, player: PlayerId) -> Vec<PlayerId> {
        game.turn_order
            .iter()
            .copied()
            .filter(|p| *p != player)
            .collect()
    }

    let mut adjusted = base_cost.clone();
    let Some(ability_source_object) = game.object(ability_source) else {
        return adjusted;
    };

    let mut cost_modifier_sources = view.activated_ability_cost_modifier_sources();
    if ability_source_object.zone != Zone::Battlefield {
        cost_modifier_sources.push(ability_source);
    }

    for source_id in cost_modifier_sources {
        let Some(perm) = game.object(source_id) else {
            continue;
        };
        let controller = perm.controller;
        let filter_ctx = FilterContext::new(controller)
            .with_source(source_id)
            .with_active_player(game.turn.active_player)
            .with_opponents(opponents_of(game, controller));

        let static_abilities = if perm.zone == Zone::Battlefield {
            view.static_abilities_rc(source_id).unwrap_or_else(|| {
                Rc::new(
                    perm.abilities
                        .iter()
                        .filter_map(|a| match &a.kind {
                            AbilityKind::Static(sa) => Some(sa.clone()),
                            _ => None,
                        })
                        .collect(),
                )
            })
        } else {
            Rc::new(
                perm.abilities
                    .iter()
                    .filter_map(|a| match &a.kind {
                        AbilityKind::Static(sa) if a.functions_in(&perm.zone) => Some(sa.clone()),
                        _ => None,
                    })
                    .collect(),
            )
        };

        for static_ability in static_abilities.iter() {
            let Some(reduction) = static_ability.activated_ability_cost_reduction() else {
                continue;
            };
            if !static_ability.is_active(game, source_id) {
                continue;
            }
            if !reduction
                .filter
                .matches(ability_source_object, &filter_ctx, game)
            {
                continue;
            }
            if let Some(condition) = &reduction.condition
                && !crate::static_abilities::activated_ability_cost_condition_is_active_for_activation(
                    game,
                    ability_source,
                    condition,
                    chosen_targets,
                )
            {
                continue;
            }

            let multiplier = if let Some(per_filter) = &reduction.per_matching_objects {
                game.objects_in_deterministic_order()
                    .into_iter()
                    .filter(|obj| per_filter.matches(obj, &filter_ctx, game))
                    .count() as u32
            } else {
                1
            };
            if multiplier == 0 {
                continue;
            }

            let before = adjusted.clone();
            adjusted = adjusted.reduce_generic(reduction.reduction.saturating_mul(multiplier));
            if let Some(minimum_total_mana) = reduction.minimum_total_mana
                && before.mana_value() > 0
                && adjusted.mana_value() < minimum_total_mana
            {
                let missing = minimum_total_mana - adjusted.mana_value();
                adjusted = add_generic_mana_cost(&adjusted, missing);
            }
        }
    }

    apply_payment_reason_mana_adjustments(
        game,
        activator,
        Some(ability_source),
        &adjusted,
        crate::costs::PaymentReason::ActivateAbility,
    )
}

/// Resolve an alternative method index for `CastingMethod::PlayFrom`.
///
/// The index space is:
/// 1) Card intrinsic alternatives (`card.alternative_casts`)
/// 2) Granted alternatives for this card/zone/player (appended after intrinsic methods)
pub(crate) fn resolve_play_from_alternative_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    zone: Zone,
    idx: usize,
) -> Option<crate::alternative_cast::AlternativeCastingMethod> {
    if let Some(method) = spell.alternative_casts.get(idx) {
        return Some(method.clone());
    }

    let granted = game
        .grant_registry
        .granted_alternative_casts_for_card(game, spell.id, zone, player);
    let granted_idx = idx.checked_sub(spell.alternative_casts.len())?;
    granted.get(granted_idx).map(|entry| entry.method.clone())
}

fn spell_matches_cast_filter(
    game: &GameState,
    spell: &crate::object::Object,
    spell_filter: &crate::target::ObjectFilter,
) -> bool {
    spell_filter.matches(spell, &crate::target::FilterContext::default(), game)
}

fn snapshot_matches_cast_filter(
    game: &GameState,
    snapshot: &crate::snapshot::ObjectSnapshot,
    spell_filter: &crate::target::ObjectFilter,
) -> bool {
    spell_filter.matches_snapshot(snapshot, &crate::target::FilterContext::default(), game)
}

fn spells_cast_this_turn_matching_filter(
    game: &GameState,
    player: PlayerId,
    spell_filter: &crate::target::ObjectFilter,
) -> u32 {
    if spell_filter == &crate::target::ObjectFilter::default() {
        return game.turn_history.spells_cast_by_player(player);
    }

    game.turn_history
        .spell_cast_snapshot_history()
        .iter()
        .filter(|snapshot| {
            snapshot.controller == player
                && snapshot_matches_cast_filter(game, snapshot, spell_filter)
        })
        .count() as u32
}

fn violates_cast_limit(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_filter: &crate::target::ObjectFilter,
) -> bool {
    spell_matches_cast_filter(game, spell, spell_filter)
        && spells_cast_this_turn_matching_filter(game, player, spell_filter) >= 1
}

fn violates_any_cast_limit(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    game.cant_effects
        .cast_limit_filters_for_player(player)
        .is_some_and(|filters| {
            filters
                .iter()
                .any(|spell_filter| violates_cast_limit(game, player, spell, spell_filter))
        })
}

fn violates_any_cant_cast_restriction(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    game.cant_effects
        .cast_filters_for_player(player)
        .is_some_and(|filters| {
            filters
                .iter()
                .any(|spell_filter| spell_matches_cast_filter(game, spell, spell_filter))
        })
}

fn is_sorcery_speed_spell(spell: &crate::object::Object) -> bool {
    use crate::types::CardType;

    spell.has_card_type(CardType::Sorcery)
        || spell.has_card_type(CardType::Creature)
        || spell.has_card_type(CardType::Artifact)
        || spell.has_card_type(CardType::Enchantment)
        || spell.has_card_type(CardType::Planeswalker)
}

fn spell_has_active_flash_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    view: &DerivedGameView<'_>,
) -> bool {
    spell.abilities.iter().any(|a| {
        if let crate::ability::AbilityKind::Static(s) = &a.kind {
            if s.has_flash() {
                return true;
            }
            if let Some(spec) = s.conditional_spell_keyword_spec()
                && spec.keyword == crate::static_abilities::ConditionalSpellKeywordKind::Flash
            {
                return crate::static_abilities::conditional_spell_keyword_active(
                    spec, game, player,
                );
            }
        }
        false
    }) || view.card_has_granted_static_ability_id(
        spell_id,
        Zone::Hand,
        player,
        crate::static_abilities::StaticAbilityId::Flash,
    )
}

fn player_was_attacked_this_step(game: &GameState, player: PlayerId) -> bool {
    use crate::combat_state::AttackTarget;
    use crate::game_state::{Phase, Step};

    if !matches!(game.turn.phase, Phase::Combat) || game.turn.step != Some(Step::DeclareAttackers) {
        return false;
    }

    let Some(combat) = game.combat.as_ref() else {
        return false;
    };

    combat
        .attackers
        .iter()
        .any(|attacker| match attacker.target {
            AttackTarget::Player(defender) => defender == player,
            AttackTarget::Planeswalker(planeswalker_id) => game
                .object(planeswalker_id)
                .is_some_and(|planeswalker| planeswalker.controller == player),
        })
}

fn this_spell_cast_restriction_allows(
    game: &GameState,
    player: PlayerId,
    kind: &crate::static_abilities::ThisSpellCastRestrictionKind,
) -> bool {
    let timing_allows = kind
        .timing
        .is_none_or(|timing| this_spell_cast_timing_allows(game, player, timing));
    if !timing_allows {
        return false;
    }
    kind.condition
        .as_ref()
        .is_none_or(|condition| this_spell_cast_condition_allows(game, player, condition))
}

fn this_spell_cast_timing_allows(
    game: &GameState,
    player: PlayerId,
    timing: crate::static_abilities::ThisSpellCastTiming,
) -> bool {
    use crate::game_state::{Phase, Step};
    use crate::static_abilities::ThisSpellCastTiming;

    match timing {
        ThisSpellCastTiming::DuringDeclareAttackersStep => {
            matches!(game.turn.phase, Phase::Combat)
                && game.turn.step == Some(Step::DeclareAttackers)
        }
        ThisSpellCastTiming::DuringCombat => matches!(game.turn.phase, Phase::Combat),
        ThisSpellCastTiming::DuringCombatBeforeBlockersAreDeclared => {
            matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::BeginCombat | Step::DeclareAttackers)
                )
        }
        ThisSpellCastTiming::DuringCombatAfterBlockersAreDeclared => {
            matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::DeclareBlockers | Step::CombatDamage | Step::EndCombat)
                )
        }
        ThisSpellCastTiming::DuringCombatOnYourTurnBeforeBlockersAreDeclared => {
            game.turn.active_player == player
                && matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::BeginCombat | Step::DeclareAttackers)
                )
        }
        ThisSpellCastTiming::DuringCombatOnOpponentsTurn => {
            game.turn.active_player != player && matches!(game.turn.phase, Phase::Combat)
        }
        ThisSpellCastTiming::BeforeAttackersAreDeclared => {
            matches!(game.turn.phase, Phase::Combat) && game.turn.step == Some(Step::BeginCombat)
        }
        ThisSpellCastTiming::BeforeCombatDamageStep => {
            matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::BeginCombat | Step::DeclareAttackers | Step::DeclareBlockers)
                )
        }
        ThisSpellCastTiming::DuringOpponentsUpkeep => {
            game.turn.active_player != player
                && matches!(game.turn.phase, Phase::Beginning)
                && game.turn.step == Some(Step::Upkeep)
        }
        ThisSpellCastTiming::DuringOpponentsTurnAfterUpkeep => {
            if game.turn.active_player == player {
                return false;
            }
            !matches!(
                (game.turn.phase, game.turn.step),
                (Phase::Beginning, Some(Step::Untap | Step::Upkeep))
            )
        }
        ThisSpellCastTiming::DuringYourEndStep => {
            game.turn.active_player == player
                && matches!(game.turn.phase, Phase::Ending)
                && game.turn.step == Some(Step::End)
        }
        ThisSpellCastTiming::AfterCombat => {
            matches!(game.turn.phase, Phase::NextMain | Phase::Ending)
        }
    }
}

fn players_matching_cast_restriction_filter(
    game: &GameState,
    player: PlayerId,
    filter: &crate::target::PlayerFilter,
) -> Vec<PlayerId> {
    let filter_ctx = game.filter_context_for(player, None);
    match filter {
        crate::target::PlayerFilter::You => vec![player],
        crate::target::PlayerFilter::Opponent => filter_ctx.opponents.clone(),
        crate::target::PlayerFilter::Teammate => filter_ctx.teammates.clone(),
        crate::target::PlayerFilter::Specific(id) => vec![*id],
        crate::target::PlayerFilter::Any => game
            .players
            .iter()
            .filter(|candidate| candidate.is_in_game())
            .map(|candidate| candidate.id)
            .collect(),
        crate::target::PlayerFilter::NotYou => game
            .players
            .iter()
            .filter_map(|candidate| {
                (candidate.is_in_game() && candidate.id != player).then_some(candidate.id)
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn this_spell_cast_condition_allows(
    game: &GameState,
    player: PlayerId,
    condition: &crate::static_abilities::ThisSpellCastCondition,
) -> bool {
    match condition {
        crate::static_abilities::ThisSpellCastCondition::YouWereAttackedThisStep => {
            player_was_attacked_this_step(game, player)
        }
        crate::static_abilities::ThisSpellCastCondition::PlayerCastSpellThisTurnOrMore {
            player: player_filter,
            spell_filter,
            count,
        } => players_matching_cast_restriction_filter(game, player, player_filter)
            .into_iter()
            .map(|matched_player| {
                spells_cast_this_turn_matching_filter(game, matched_player, spell_filter)
            })
            .sum::<u32>()
            >= *count,
        crate::static_abilities::ThisSpellCastCondition::CreatureIsAttackingYou => {
            player_was_attacked_this_step(game, player)
                || game.combat.as_ref().is_some_and(|combat| {
                    combat.attackers.iter().any(|attacker| match attacker.target {
                        crate::combat_state::AttackTarget::Player(defender) => defender == player,
                        crate::combat_state::AttackTarget::Planeswalker(planeswalker_id) => game
                            .object(planeswalker_id)
                            .is_some_and(|planeswalker| planeswalker.controller == player),
                    })
                })
        }
        crate::static_abilities::ThisSpellCastCondition::NoPermanentsNamedOnBattlefield(name) => {
            !game.battlefield.iter().any(|&id| {
                game.object(id)
                    .is_some_and(|object| object.name.eq_ignore_ascii_case(name))
            })
        }
        crate::static_abilities::ThisSpellCastCondition::YouControlAtLeast { filter, count } => {
            let mut required_filter = filter.clone();
            required_filter.zone = Some(Zone::Battlefield);
            let filter_ctx = game.filter_context_for(player, None);
            game.battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|object| {
                    object.controller == player
                        && required_filter.matches(object, &filter_ctx, game)
                })
                .count()
                >= *count as usize
        }
        crate::static_abilities::ThisSpellCastCondition::YouControlFewerCreaturesThanEachOpponent => {
            let your_creatures = game.creatures_controlled_by(player).len();
            game.players
                .iter()
                .filter(|opponent| opponent.is_in_game() && opponent.id != player)
                .all(|opponent| your_creatures < game.creatures_controlled_by(opponent.id).len())
        }
        crate::static_abilities::ThisSpellCastCondition::YouControlNameWordOrMore {
            word,
            count,
        } => game
            .permanents_controlled_by(player)
            .iter()
            .filter(|id| {
                game.object(**id).is_some_and(|object| {
                    object
                        .name
                        .to_ascii_lowercase()
                        .contains(&word.to_ascii_lowercase())
                })
            })
            .count()
            >= *count as usize,
    }
}

fn spell_cast_restrictions_allow(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    spell.abilities.iter().all(|ability| {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return true;
        };
        let Some(kind) = static_ability.this_spell_cast_restriction_kind() else {
            return true;
        };
        this_spell_cast_restriction_allows(game, player, &kind)
    })
}

fn has_valid_spell_timing(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: ObjectId,
) -> bool {
    let view = DerivedGameView::new(game);
    has_valid_spell_timing_with_view(game, player, spell, spell_id, &view)
}

fn has_valid_spell_timing_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    view: &DerivedGameView<'_>,
) -> bool {
    if !is_sorcery_speed_spell(spell)
        || spell_has_active_flash_with_view(game, player, spell, spell_id, view)
    {
        return true;
    }

    // Sorcery-speed spells require: active player, main phase, empty stack.
    game.turn.active_player == player && crate::turn::is_sorcery_timing(game)
}

fn face_down_cast_mana_cost() -> crate::mana::ManaCost {
    crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Generic(3)]])
}

fn spell_can_be_cast_face_down(spell: &crate::object::Object) -> bool {
    spell.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            crate::ability::AbilityKind::Static(static_ability)
                if static_ability.turn_face_up_cost().is_some()
        )
    })
}

/// Resolve the mana cost for a spell cast from a specific zone and method.
pub fn spell_mana_cost_for_cast(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    from_zone: Zone,
) -> Option<crate::mana::ManaCost> {
    let base_cost = match casting_method {
        CastingMethod::Normal => spell.mana_cost.clone(),
        CastingMethod::FaceDown => Some(face_down_cast_mana_cost()),
        CastingMethod::SplitOtherHalf => {
            linked_face_definition(game, spell).and_then(|def| def.card.mana_cost)
        }
        CastingMethod::Fuse => {
            spell_view_for_fused_split_cast(game, spell).and_then(|view| view.mana_cost)
        }
        CastingMethod::Alternative(idx) => {
            if let Some(method) = spell.alternative_casts.get(*idx) {
                if matches!(
                    method,
                    crate::alternative_cast::AlternativeCastingMethod::Plot { .. }
                ) {
                    Some(crate::mana::ManaCost::new())
                } else if method.total_cost().is_some() {
                    Some(method.mana_cost().cloned().unwrap_or_default())
                } else {
                    method
                        .mana_cost()
                        .cloned()
                        .or_else(|| spell.mana_cost.clone())
                }
            } else {
                spell.mana_cost.clone()
            }
        }
        CastingMethod::GrantedEscape { .. } => spell.mana_cost.clone(),
        CastingMethod::GrantedFlashback => spell.mana_cost.clone(),
        CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => spell.mana_cost.clone(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => {
            if let Some(method) =
                resolve_play_from_alternative_method(game, player, spell, *zone, *idx)
            {
                if matches!(
                    method,
                    crate::alternative_cast::AlternativeCastingMethod::Plot { .. }
                ) {
                    Some(crate::mana::ManaCost::new())
                } else if method.total_cost().is_some() {
                    Some(method.mana_cost().cloned().unwrap_or_default())
                } else {
                    method
                        .mana_cost()
                        .cloned()
                        .or_else(|| spell.mana_cost.clone())
                }
            } else {
                spell.mana_cost.clone()
            }
        }
    };

    if from_zone == Zone::Command {
        let tax = game.commander_cast_count(spell.id).saturating_mul(2);
        base_cost.map(|cost| cost.add_generic(tax))
    } else {
        base_cost
    }
}

fn alternative_method_uses_printed_mana_cost(
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> bool {
    matches!(
        method,
        crate::alternative_cast::AlternativeCastingMethod::JumpStart
            | crate::alternative_cast::AlternativeCastingMethod::Escape { cost: None, .. }
    )
}

fn casting_method_requires_printed_mana_cost(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    match casting_method {
        CastingMethod::Normal
        | CastingMethod::GrantedEscape { .. }
        | CastingMethod::GrantedFlashback
        | CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => true,
        CastingMethod::Alternative(idx) => spell
            .alternative_casts
            .get(*idx)
            .is_some_and(alternative_method_uses_printed_mana_cost),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => resolve_play_from_alternative_method(game, player, spell, *zone, *idx)
            .as_ref()
            .is_some_and(alternative_method_uses_printed_mana_cost),
        _ => false,
    }
}

/// Check if a spell can be cast by a player using the given casting method.
pub fn can_cast_spell(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    let view = DerivedGameView::new(game);
    can_cast_spell_with_view(game, player, spell, casting_method, &view)
}

/// Check whether a player could begin casting a spell from hand for suspend.
///
/// This enforces cast prohibitions, cast limits, timing, and explicit
/// "cast this spell only ..." restrictions without requiring a printable mana
/// cost or legal targets yet.
pub fn can_begin_to_cast_from_hand_for_suspend(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    if violates_any_cant_cast_restriction(game, player, spell) {
        return false;
    }

    if violates_any_cast_limit(game, player, spell) {
        return false;
    }

    if spell.is_land() {
        return false;
    }

    if !has_valid_spell_timing(game, player, spell, spell.id) {
        return false;
    }

    spell_cast_restrictions_allow(game, player, spell)
}

fn spell_has_legal_targets_for_cast_with_view(
    spell: &crate::object::Object,
    spell_id: ObjectId,
    effects_override: Option<&[crate::effect::Effect]>,
    player: PlayerId,
    view: &DerivedGameView<'_>,
) -> bool {
    let synthesized_aura_effects = if effects_override.is_none()
        && spell.spell_effect.is_none()
        && spell.subtypes.contains(&crate::types::Subtype::Aura)
    {
        spell
            .aura_attach_filter
            .clone()
            .map(|filter| vec![crate::effect::Effect::attach_to(filter.target_spec())])
    } else {
        None
    };
    let effects = effects_override.unwrap_or_else(|| {
        synthesized_aura_effects
            .as_deref()
            .or(spell.spell_effect.as_deref())
            .unwrap_or(&[])
    });
    effects.is_empty() || view.spell_has_legal_targets(effects, player, Some(spell_id), None)
}

fn can_cast_spell_with_context(
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    let total_started_at = PerfTimer::start();
    let game = ctx.game;
    let player = ctx.player;
    let view = ctx.view;
    let split_view = match casting_method {
        CastingMethod::FaceDown => {
            if !spell_can_be_cast_face_down(spell) {
                return false;
            }
            Some(spell_view_for_face_down_cast(spell))
        }
        CastingMethod::SplitOtherHalf => match spell_view_for_split_other_half_cast(game, spell) {
            Some(view) => Some(view),
            None => return false,
        },
        CastingMethod::Fuse => match spell_view_for_fused_split_cast(game, spell) {
            Some(view) => Some(view),
            None => return false,
        },
        _ => None,
    };
    let spell_for_checks = split_view.as_ref().unwrap_or(spell);

    let restrictions_started_at = PerfTimer::start();
    if violates_any_cant_cast_restriction(game, player, spell_for_checks) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    if violates_any_cast_limit(game, player, spell_for_checks) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    if spell_for_checks.is_land() {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    if !spell_cast_restrictions_allow(game, player, spell_for_checks) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());

    let timing_started_at = PerfTimer::start();
    if !has_valid_spell_timing_with_view(game, player, spell_for_checks, spell.id, view) {
        ctx.add_timing_ms(timing_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    ctx.add_timing_ms(timing_started_at.elapsed_ms());

    let base_mana_cost = spell_mana_cost_for_cast(game, player, spell, casting_method, spell.zone);
    if base_mana_cost.is_none()
        && casting_method_requires_printed_mana_cost(game, player, spell, casting_method)
    {
        return false;
    }

    let target_started_at = PerfTimer::start();
    let effects = split_view
        .as_ref()
        .and_then(|view| view.spell_effect.as_deref())
        .or(spell.spell_effect.as_deref());
    let has_legal_targets = spell_has_legal_targets_for_cast_with_view(
        spell_for_checks,
        spell.id,
        effects,
        player,
        view,
    );
    ctx.add_target_legality_ms(target_started_at.elapsed_ms());
    if !has_legal_targets {
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }

    if let Some(base_cost) = base_mana_cost.as_ref() {
        let cost_started_at = PerfTimer::start();
        let effective_cost = if ctx
            .can_use_printed_cost_directly(spell_has_intrinsic_cost_adjustments(spell_for_checks))
        {
            base_cost.clone()
        } else if ctx
            .spell_cost_needs_adjustment(spell_has_intrinsic_cost_adjustments(spell_for_checks))
        {
            calculate_effective_mana_cost_with_view(game, player, spell_for_checks, base_cost, view)
        } else {
            apply_minimum_spell_total_mana(
                game,
                &apply_payment_reason_mana_adjustments(
                    game,
                    player,
                    Some(spell.id),
                    base_cost,
                    crate::costs::PaymentReason::CastSpell,
                ),
            )
        };
        ctx.add_cost_adjustment_ms(cost_started_at.elapsed_ms());

        let affordability_started_at = PerfTimer::start();
        let potential = view.potential_mana(player);
        let allow_any_color = game.can_spend_mana_as_any_color(player, Some(spell.id));
        let allow_black_life = game.player_can_pay_black_with_life_for_reason(
            player,
            Some(spell.id),
            crate::costs::PaymentReason::CastSpell,
        );
        if mana_cost_is_obviously_unpayable(
            &potential,
            &effective_cost,
            allow_any_color,
            allow_black_life,
        ) {
            ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
            ctx.add_total_ms(total_started_at.elapsed_ms());
            return false;
        }
        if !view.can_potentially_pay_with_reason(
            player,
            Some(spell.id),
            &effective_cost,
            0,
            crate::costs::PaymentReason::CastSpell,
        ) {
            ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
            ctx.add_total_ms(total_started_at.elapsed_ms());
            return false;
        }
        ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
    }

    ctx.add_total_ms(total_started_at.elapsed_ms());
    true
}

fn can_cast_spell_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_spell_with_context(spell, casting_method, &ctx)
}

// ============================================================================
// Unified Spell Casting Validation
// ============================================================================

/// Additional requirements for casting a spell beyond mana.
#[derive(Debug, Clone, Default)]
pub struct AdditionalCastRequirements {
    /// Cards that must be exiled from graveyard (excluding the spell itself).
    pub exile_from_graveyard: u32,
    /// Cards that must be discarded from hand.
    pub discard_from_hand: u32,
    /// A TotalCost that must be paid (for alternative costs like Force of Will).
    /// This is checked with spell exclusion (the spell being cast is excluded from hand).
    pub total_cost: Option<crate::cost::TotalCost>,
    /// If true, spell must be instant or sorcery only.
    pub must_be_instant_or_sorcery: bool,
}

fn can_cast_with_cost_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    mana_cost: Option<&crate::mana::ManaCost>,
    effects_override: Option<&[crate::effect::Effect]>,
    requirements: &AdditionalCastRequirements,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_with_cost_with_context(
        spell,
        spell_id,
        mana_cost,
        effects_override,
        requirements,
        &ctx,
    )
}

fn can_cast_with_cost_with_context(
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    mana_cost: Option<&crate::mana::ManaCost>,
    effects_override: Option<&[crate::effect::Effect]>,
    requirements: &AdditionalCastRequirements,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    use crate::types::CardType;
    let game = ctx.game;
    let player = ctx.player;
    let view = ctx.view;

    let restrictions_started_at = PerfTimer::start();
    if violates_any_cant_cast_restriction(game, player, spell) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if violates_any_cast_limit(game, player, spell) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if spell.is_land() {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if requirements.must_be_instant_or_sorcery
        && !spell.has_card_type(CardType::Instant)
        && !spell.has_card_type(CardType::Sorcery)
    {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if !spell_cast_restrictions_allow(game, player, spell) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());

    let timing_started_at = PerfTimer::start();
    if !has_valid_spell_timing_with_view(game, player, spell, spell_id, view) {
        ctx.add_timing_ms(timing_started_at.elapsed_ms());
        return false;
    }
    ctx.add_timing_ms(timing_started_at.elapsed_ms());

    let target_started_at = PerfTimer::start();
    let has_legal_targets =
        spell_has_legal_targets_for_cast_with_view(spell, spell_id, effects_override, player, view);
    ctx.add_target_legality_ms(target_started_at.elapsed_ms());
    if !has_legal_targets {
        return false;
    }

    let Some(player_obj) = game.player(player) else {
        return false;
    };

    // Check exile from graveyard requirement
    if requirements.exile_from_graveyard > 0 {
        let other_cards_in_graveyard = player_obj
            .graveyard
            .iter()
            .filter(|&&id| id != spell_id)
            .count();
        if other_cards_in_graveyard < requirements.exile_from_graveyard as usize {
            return false;
        }
    }

    // Check discard from hand requirement
    // For Jump-Start, need at least discard_from_hand cards in hand
    if requirements.discard_from_hand > 0
        && (player_obj.hand.len() as u32) < requirements.discard_from_hand
    {
        return false;
    }

    // Check TotalCost requirement (for Force of Will style costs)
    if let Some(ref total_cost) = requirements.total_cost {
        for individual_cost in total_cost.costs() {
            if !can_pay_cost_with_spell_exclusion(game, player, individual_cost, Some(spell_id)) {
                return false;
            }
        }
    }

    if let Some(cost) = mana_cost {
        let cost_started_at = PerfTimer::start();
        let adjusted =
            if ctx.can_use_printed_cost_directly(spell_has_intrinsic_cost_adjustments(spell)) {
                cost.clone()
            } else if ctx.spell_cost_needs_adjustment(spell_has_intrinsic_cost_adjustments(spell)) {
                calculate_effective_mana_cost_with_view(game, player, spell, cost, view)
            } else {
                apply_minimum_spell_total_mana(
                    game,
                    &apply_payment_reason_mana_adjustments(
                        game,
                        player,
                        Some(spell_id),
                        cost,
                        crate::costs::PaymentReason::CastSpell,
                    ),
                )
            };
        ctx.add_cost_adjustment_ms(cost_started_at.elapsed_ms());

        let affordability_started_at = PerfTimer::start();
        let potential = view.potential_mana(player);
        let allow_any_color = game.can_spend_mana_as_any_color(player, Some(spell_id));
        let allow_black_life = game.player_can_pay_black_with_life_for_reason(
            player,
            Some(spell_id),
            crate::costs::PaymentReason::CastSpell,
        );
        if mana_cost_is_obviously_unpayable(
            &potential,
            &adjusted,
            allow_any_color,
            allow_black_life,
        ) {
            ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
            return false;
        }
        if !view.can_potentially_pay_with_reason(
            player,
            Some(spell_id),
            &adjusted,
            0,
            crate::costs::PaymentReason::CastSpell,
        ) {
            ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
            return false;
        }
        ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
    }

    true
}

/// Build additional cast requirements from an alternative casting method.
fn build_requirements_for_method(
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> AdditionalCastRequirements {
    let method_requirements = method.requirements();
    AdditionalCastRequirements {
        exile_from_graveyard: method_requirements.exile_from_graveyard,
        discard_from_hand: method_requirements.discard_from_hand,
        ..Default::default()
    }
}

/// Get the mana cost for an alternative casting method.
fn get_mana_cost_for_method<'a>(
    method: &'a crate::alternative_cast::AlternativeCastingMethod,
    spell: &'a crate::object::Object,
) -> Option<&'a crate::mana::ManaCost> {
    // Composed costs can intentionally represent "without paying its mana cost"
    // by omitting a mana component, so do not fall back to the card's printed cost.
    if method.is_composed_cost() {
        return method.mana_cost();
    }

    // Method's cost takes priority, fallback to spell's cost for methods that
    // explicitly say they reuse the spell's normal mana cost.
    method.mana_cost().or(spell.mana_cost.as_ref())
}

fn spell_view_for_disturb_cast(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::object::Object> {
    let other_def = game
        .linked_face_definition_by_name_or_id(spell.other_face_name.as_deref(), spell.other_face)?;
    let mut view = spell.clone();
    view.apply_definition_face(&other_def);
    view.ensure_aura_cast_spell_effect();
    Some(view)
}

fn spell_view_for_face_down_cast(spell: &crate::object::Object) -> crate::object::Object {
    let mut view = spell.clone();
    view.apply_face_down_cast_overlay();
    view
}

fn linked_face_definition(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::cards::CardDefinition> {
    game.linked_face_definition_by_name_or_id(spell.other_face_name.as_deref(), spell.other_face)
}

fn spell_view_for_split_other_half_cast(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::object::Object> {
    if spell.linked_face_layout != crate::card::LinkedFaceLayout::Split {
        return None;
    }
    let other_def = linked_face_definition(game, spell)?;
    let mut view = spell.clone();
    view.apply_definition_face(&other_def);
    view.ensure_aura_cast_spell_effect();
    Some(view)
}

fn spell_view_for_fused_split_cast(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::object::Object> {
    if spell.linked_face_layout != crate::card::LinkedFaceLayout::Split || !spell.has_fuse {
        return None;
    }
    let other_def = linked_face_definition(game, spell)?;
    let mut view = spell.clone();
    view.apply_fused_split_spell_overlay(&other_def);
    Some(view)
}

fn can_cast_with_alternative_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_with_alternative_with_context(spell, method, &ctx)
}

fn can_cast_with_alternative_with_context(
    spell: &crate::object::Object,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    let game = ctx.game;
    let player = ctx.player;

    let disturbed_view = match method {
        AlternativeCastingMethod::Disturb { .. } => {
            match spell_view_for_disturb_cast(game, spell) {
                Some(view) => Some(view),
                None => return false,
            }
        }
        _ => None,
    };
    let spell_for_checks = disturbed_view.as_ref().unwrap_or(spell);
    let effects_override = method.overload_effects().or_else(|| {
        disturbed_view
            .as_ref()
            .and_then(|view| view.spell_effect.as_deref())
    });
    let free_plot_cost = crate::mana::ManaCost::new();
    let mana_cost = match method {
        AlternativeCastingMethod::Foretell { .. } => {
            if !game.is_foretold(spell.id) {
                return false;
            }
            get_mana_cost_for_method(method, spell_for_checks)
        }
        AlternativeCastingMethod::Plot { .. } => {
            if !game.is_plotted_by(spell.id, player) {
                return false;
            }
            let Some(plotted_turn) = game.plotted_turn(spell.id) else {
                return false;
            };
            if plotted_turn >= game.turn.turn_number {
                return false;
            }
            if game.turn.active_player != player || !crate::turn::is_sorcery_timing(game) {
                return false;
            }
            Some(&free_plot_cost)
        }
        AlternativeCastingMethod::Suspend { .. } => return false,
        _ => get_mana_cost_for_method(method, spell_for_checks),
    };
    if mana_cost.is_none() && alternative_method_uses_printed_mana_cost(method) {
        return false;
    }

    let requirements = build_requirements_for_method(method);
    if !can_cast_with_cost_with_context(
        spell_for_checks,
        spell.id,
        mana_cost,
        effects_override,
        &requirements,
        ctx,
    ) {
        return false;
    }

    let check_ctx = crate::costs::CostCheckContext::new(spell.id, player)
        .with_reason(crate::costs::PaymentReason::CastSpell);
    for cost in method.non_mana_costs() {
        if game
            .validate_cost_for_payment_reason(player, spell.id, &cost, check_ctx.reason)
            .is_err()
        {
            return false;
        }
        if crate::costs::can_pay_with_check_context(&*cost.0, game, &check_ctx).is_err() {
            return false;
        }
    }

    true
}

/// Check if a spell can be cast with an alternative cost from hand (e.g., Force of Will).
pub fn can_cast_with_alternative_from_hand(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> bool {
    let view = DerivedGameView::new(game);
    can_cast_with_alternative_from_hand_with_view(game, player, spell, spell_id, method, &view)
}

fn can_cast_with_alternative_from_hand_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_with_alternative_from_hand_with_context(spell, spell_id, method, &ctx)
}

fn can_cast_with_alternative_from_hand_with_context(
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    let game = ctx.game;
    let player = ctx.player;

    match method {
        method if method.is_composed_cost() => {
            let zero_cost = crate::mana::ManaCost::new();
            if let Some(condition) = method.cast_condition()
                && !crate::static_abilities::this_spell_cost_condition_is_active_for_cast(
                    game,
                    spell_id,
                    condition,
                    &[],
                )
            {
                return false;
            }

            if !can_cast_with_cost_with_context(
                spell,
                spell_id,
                method.mana_cost().or(Some(&zero_cost)),
                None,
                &AdditionalCastRequirements::default(),
                ctx,
            ) {
                return false;
            }

            let check_ctx = crate::costs::CostCheckContext::new(spell_id, player)
                .with_reason(crate::costs::PaymentReason::CastSpell);
            for cost in method.non_mana_costs() {
                if game
                    .validate_cost_for_payment_reason(player, spell_id, &cost, check_ctx.reason)
                    .is_err()
                {
                    return false;
                }
                if crate::costs::can_pay_with_check_context(&*cost.0, game, &check_ctx).is_err() {
                    return false;
                }
            }
            true
        }
        AlternativeCastingMethod::Bestow { total_cost } => {
            let Some(cost) = total_cost.mana_cost() else {
                return false;
            };

            if !can_cast_with_cost_with_context(
                spell,
                spell_id,
                Some(cost),
                None,
                &AdditionalCastRequirements::default(),
                ctx,
            ) {
                return false;
            }

            let check_ctx = crate::costs::CostCheckContext::new(spell_id, player)
                .with_reason(crate::costs::PaymentReason::CastSpell);
            for cost in method.non_mana_costs() {
                if game
                    .validate_cost_for_payment_reason(player, spell_id, &cost, check_ctx.reason)
                    .is_err()
                {
                    return false;
                }
                if crate::costs::can_pay_with_check_context(&*cost.0, game, &check_ctx).is_err() {
                    return false;
                }
            }

            let bestow_spec = ChooseSpec::Object(crate::target::ObjectFilter::creature());
            let bestow_targets =
                crate::targeting::compute_legal_targets_with_tagged_objects_with_view(
                    game,
                    &bestow_spec,
                    player,
                    Some(spell_id),
                    None,
                    ctx.view,
                );
            !bestow_targets.is_empty()
        }
        AlternativeCastingMethod::MindbreakTrap {
            cost, condition, ..
        } => {
            // Check if the trap condition is met
            if !is_trap_condition_met(game, player, condition) {
                return false;
            }
            // Check if player can pay the trap cost (usually {0})
            can_cast_with_cost_with_context(
                spell,
                spell_id,
                Some(cost),
                None,
                &AdditionalCastRequirements::default(),
                ctx,
            )
        }
        _ => can_cast_with_alternative_with_context(spell, method, ctx),
    }
}

/// Check if a trap condition is met for the given player.
fn is_trap_condition_met(
    game: &GameState,
    player: PlayerId,
    condition: &crate::alternative_cast::TrapCondition,
) -> bool {
    use crate::alternative_cast::TrapCondition;

    // Get all opponents
    let opponents: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|p| p.id != player && p.is_in_game())
        .map(|p| p.id)
        .collect();

    match condition {
        TrapCondition::OpponentCastSpells { count } => {
            // Check if any opponent cast N or more spells this turn
            opponents
                .iter()
                .any(|&opp| game.turn_history.spells_cast_by_player(opp) >= *count)
        }
        TrapCondition::OpponentSearchedLibrary => {
            // Check if any opponent searched their library this turn
            opponents
                .iter()
                .any(|opp| game.turn_history.player_searched_library_this_turn(*opp))
        }
        TrapCondition::OpponentCreatureEntered => {
            // Check if any opponent had a creature enter the battlefield this turn
            opponents.iter().any(|&opp| {
                game.turn_history
                    .player_had_creature_enter_battlefield_this_turn(opp)
            })
        }
        TrapCondition::CreatureDealtDamageToYou => {
            // Check if any creature dealt damage to the player this turn
            game.turn_history
                .player_was_dealt_damage_by_creature_this_turn(player)
        }
    }
}

/// Check if a player can pay a specific cost, excluding a specific card from hand (the spell being cast).
fn can_pay_cost_with_spell_exclusion(
    game: &GameState,
    player: PlayerId,
    cost: &crate::costs::Cost,
    spell_to_exclude: Option<crate::ids::ObjectId>,
) -> bool {
    use crate::costs::CostProcessingMode;

    let source = spell_to_exclude.or_else(|| {
        game.player(player).and_then(|p| {
            p.hand
                .first()
                .copied()
                .or_else(|| p.graveyard.first().copied())
        })
    });
    let Some(source) = source else {
        return false;
    };

    let mut dm = crate::decision::CliDecisionMaker;
    let ctx = crate::costs::CostContext::new(source, player, &mut dm)
        .with_reason(crate::costs::PaymentReason::CastSpell);
    if game
        .validate_cost_for_payment_reason(player, source, cost, ctx.reason)
        .is_err()
    {
        return false;
    }

    match cost.processing_mode() {
        CostProcessingMode::ManaPayment { .. } => cost.can_potentially_pay(game, &ctx).is_ok(),
        CostProcessingMode::Immediate
        | CostProcessingMode::InlineWithTriggers
        | CostProcessingMode::SacrificeTarget { .. }
        | CostProcessingMode::DiscardCards { .. }
        | CostProcessingMode::ExileFromHand { .. }
        | CostProcessingMode::ExileFromGraveyard { .. }
        | CostProcessingMode::RevealFromHand { .. }
        | CostProcessingMode::ReturnToHandTarget { .. } => cost.can_pay(game, &ctx).is_ok(),
    }
}

fn apply_payment_reason_mana_adjustments(
    game: &GameState,
    payer: PlayerId,
    source: Option<ObjectId>,
    cost: &crate::mana::ManaCost,
    reason: crate::costs::PaymentReason,
) -> crate::mana::ManaCost {
    game.adjust_mana_cost_for_payment_reason(payer, source, cost, reason)
}

fn apply_minimum_spell_total_mana(
    game: &GameState,
    cost: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    if let Some(minimum) = game.minimum_total_spell_mana_payment()
        && cost.mana_value() < minimum
    {
        return cost.add_generic(minimum - cost.mana_value());
    }

    cost.clone()
}

// ============================================================================
// Cost Modifier Helpers (Tier 9)
// ============================================================================

/// Calculate the effective mana cost after applying cost reduction abilities.
///
/// This handles abilities like:
/// - Affinity for artifacts: Reduce generic cost by 1 for each artifact you control
/// - Delve: Reduce generic cost by 1 for each card exiled from graveyard (automatic maximum)
/// - Convoke: Tap creatures to pay for mana (colored or generic)
///
/// Returns the reduced mana cost.
pub fn calculate_effective_mana_cost(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        1,
        &[],
        true,
        &view,
    )
}

fn calculate_effective_mana_cost_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        1,
        &[],
        true,
        view,
    )
}

/// Calculate the effective mana cost with explicit chosen target count.
pub fn calculate_effective_mana_cost_with_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_target_count,
        &[],
        true,
        &view,
    )
}

/// Calculate the effective mana cost using the exact chosen targets.
pub fn calculate_effective_mana_cost_with_chosen_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_targets.len(),
        chosen_targets,
        true,
        &view,
    )
}

/// Calculate effective cost for payment stage where Convoke/Improvise are handled
/// as pip alternatives instead of up-front reductions.
pub fn calculate_effective_mana_cost_for_payment_with_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_target_count,
        &[],
        false,
        &view,
    )
}

/// Calculate payment-stage effective cost using exact chosen targets.
pub fn calculate_effective_mana_cost_for_payment_with_chosen_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_targets.len(),
        chosen_targets,
        false,
        &view,
    )
}

fn calculate_effective_mana_cost_with_targets_internal(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    chosen_targets: &[Target],
    include_convoke_improvise_reductions: bool,
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;

    let mut current_cost = base_cost.clone();

    // Check for Affinity for artifacts
    let has_affinity = spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_affinity()
        } else {
            false
        }
    });

    if has_affinity {
        // Count artifacts controlled by the player
        let artifact_count = count_artifacts_controlled_with_view(game, player, view);
        current_cost = current_cost.reduce_generic(artifact_count);
    }

    // Apply explicit cost reductions/increases on the spell itself.
    current_cost = apply_spell_cost_modifiers(
        game,
        player,
        spell,
        &current_cost,
        chosen_target_count,
        chosen_targets,
    );

    // Apply global cost modifiers from battlefield permanents (Sphere of Resistance, leeches, etc.).
    current_cost = apply_battlefield_spell_cost_modifiers(
        game,
        player,
        spell,
        &current_cost,
        chosen_target_count,
        view,
    );

    // Check for Delve
    let has_delve_ability = has_delve(spell);

    if has_delve_ability {
        // For Delve, we assume maximum usage (exile all cards up to generic cost remaining)
        let graveyard_count = count_cards_in_graveyard(game, player);
        current_cost = current_cost.reduce_generic(graveyard_count);
    }

    if include_convoke_improvise_reductions {
        // Check for Convoke
        let has_convoke_ability = has_convoke(spell);
        if has_convoke_ability {
            // For Convoke, calculate the optimal creature tapping
            let (_, convoked_cost) = calculate_convoke_cost(game, player, &current_cost);
            current_cost = convoked_cost;
        }

        // Check for Improvise
        let has_improvise_ability = has_improvise(spell);
        if has_improvise_ability {
            // For Improvise, calculate the optimal artifact tapping
            let (_, improvised_cost) = calculate_improvise_cost(game, player, &current_cost);
            current_cost = improvised_cost;
        }
    }

    let current_cost = apply_payment_reason_mana_adjustments(
        game,
        player,
        Some(spell.id),
        &current_cost,
        crate::costs::PaymentReason::CastSpell,
    );

    apply_minimum_spell_total_mana(game, &current_cost)
}

fn apply_spell_cost_modifiers(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    chosen_targets: &[Target],
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;
    use crate::filter::FilterContext;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::target::ObjectFilter;

    fn opponents_of(game: &GameState, player: PlayerId) -> Vec<PlayerId> {
        game.turn_order
            .iter()
            .copied()
            .filter(|p| *p != player)
            .collect()
    }

    fn spell_matches_filter(
        game: &GameState,
        spell: &crate::object::Object,
        caster: PlayerId,
        filter: &ObjectFilter,
        ctx: &FilterContext,
    ) -> bool {
        if filter.targets_object.is_some() || filter.targets_player.is_some() {
            // Target-dependent cost modifiers require target selection context.
            return false;
        }
        if filter.alternative_cast.is_some() {
            // Alternative casting method isn't tracked for cost computation yet.
            return false;
        }
        let mut cast_filter = filter.clone();
        cast_filter.targets_player = None;
        cast_filter.targets_object = None;
        cast_filter.alternative_cast = None;
        cast_filter.matches(spell, &ctx.clone().with_caster(Some(caster)), game)
    }

    let mut total_increase: i32 = 0;
    let mut total_reduction: i32 = 0;
    let mut increase_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut reduction_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let ctx = FilterContext::new(player)
        .with_source(spell.id)
        .with_active_player(game.turn.active_player)
        .with_opponents(opponents_of(game, player));

    for ability in &spell.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let functions_in_current_zone = ability.functions_in(&spell.zone);
        if let Some(reduction) = static_ability.this_spell_cost_reduction() {
            if crate::static_abilities::this_spell_cost_condition_is_active_for_cast(
                game,
                spell.id,
                &reduction.condition,
                chosen_targets,
            ) {
                let amount =
                    resolve_this_spell_cost_reduction_value(game, player, spell, reduction);
                if amount > 0 {
                    total_reduction = total_reduction.saturating_add(amount);
                }
            }
        }
        if let Some(reduction) = static_ability.this_spell_cost_reduction_mana_cost() {
            if crate::static_abilities::this_spell_cost_condition_is_active_for_cast(
                game,
                spell.id,
                &reduction.condition,
                chosen_targets,
            ) {
                reduction_pips.extend(reduction.reduction.pips().iter().cloned());
            }
        }
        if !functions_in_current_zone {
            continue;
        }
        if !static_ability.is_active(game, spell.id) {
            continue;
        }
        if let Some(reduction) = static_ability.cost_reduction()
            && spell_matches_filter(game, spell, player, &reduction.filter, &ctx)
        {
            let amount = resolve_cost_modifier_value(game, player, spell, &reduction.reduction);
            if amount > 0 {
                total_reduction = total_reduction.saturating_add(amount);
            }
        }
        if let Some(increase) = static_ability.cost_increase()
            && spell_matches_filter(game, spell, player, &increase.filter, &ctx)
        {
            let amount = resolve_cost_modifier_value(game, player, spell, &increase.increase);
            if amount > 0 {
                total_increase = total_increase.saturating_add(amount);
            }
        }
        if let Some(increase) = static_ability.cost_increase_mana_cost()
            && spell_matches_filter(game, spell, player, &increase.filter, &ctx)
        {
            increase_pips.extend(increase.increase.pips().iter().cloned());
        }
        if let Some(reduction) = static_ability.cost_reduction_mana_cost()
            && spell_matches_filter(game, spell, player, &reduction.filter, &ctx)
        {
            reduction_pips.extend(reduction.reduction.pips().iter().cloned());
        }
        if let Some(per_target_amount) = static_ability.cost_increase_per_additional_target() {
            let additional_targets = chosen_target_count.saturating_sub(1);
            if additional_targets > 0 {
                let extra = (per_target_amount as i32).saturating_mul(additional_targets as i32);
                total_increase = total_increase.saturating_add(extra);
            }
        }
    }

    let current_turn = game.turn.turn_number;
    for effect in &game.temporary_spell_cost_reductions {
        if effect.player != player || effect.is_expired(current_turn) {
            continue;
        }
        if spell_matches_filter(game, spell, player, &effect.filter, &ctx) {
            reduction_pips.extend(effect.reduction.pips().iter().cloned());
        }
    }

    let mut adjusted = cost.clone();
    if !increase_pips.is_empty() {
        adjusted = add_mana_cost(&adjusted, &ManaCost::from_pips(increase_pips));
    }
    if total_increase > 0 {
        adjusted = add_generic_mana_cost(&adjusted, total_increase as u32);
    }
    if total_reduction > 0 {
        adjusted = adjusted.reduce_generic(total_reduction as u32);
    }
    if !reduction_pips.is_empty() {
        adjusted = reduce_mana_cost(&adjusted, &ManaCost::from_pips(reduction_pips));
    }
    adjusted
}

fn apply_battlefield_spell_cost_modifiers(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;
    use crate::filter::FilterContext;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::target::ObjectFilter;

    fn opponents_of(game: &GameState, player: PlayerId) -> Vec<PlayerId> {
        game.turn_order
            .iter()
            .copied()
            .filter(|p| *p != player)
            .collect()
    }

    fn spell_matches_filter(
        game: &GameState,
        spell: &crate::object::Object,
        caster: PlayerId,
        filter: &ObjectFilter,
        ctx: &FilterContext,
        _chosen_target_count: usize,
    ) -> bool {
        if filter.targets_object.is_some() || filter.targets_player.is_some() {
            // Target-dependent cost modifiers require target selection context.
            return false;
        }
        if filter.alternative_cast.is_some() {
            // Alternative casting method isn't tracked for cost computation yet.
            return false;
        }
        let mut cast_filter = filter.clone();
        cast_filter.targets_player = None;
        cast_filter.targets_object = None;
        cast_filter.alternative_cast = None;
        cast_filter.matches(spell, &ctx.clone().with_caster(Some(caster)), game)
    }

    let mut total_increase: i32 = 0;
    let mut total_reduction: i32 = 0;
    let mut increase_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut reduction_pips: Vec<Vec<ManaSymbol>> = Vec::new();

    for perm_id in view.battlefield_spell_cost_modifier_sources() {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        let controller = perm.controller;
        let ctx = FilterContext::new(controller)
            .with_source(perm_id)
            .with_active_player(game.turn.active_player)
            .with_opponents(opponents_of(game, controller));

        if let Some(static_abilities) = view.static_abilities_rc(perm_id) {
            for static_ability in static_abilities.iter() {
                if let Some(reduction) = static_ability.cost_reduction()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        chosen_target_count,
                    )
                {
                    let amount = resolve_cost_modifier_value_for_source(
                        game,
                        perm_id,
                        controller,
                        &reduction.reduction,
                    );
                    if amount > 0 {
                        total_reduction = total_reduction.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        chosen_target_count,
                    )
                {
                    let amount = resolve_cost_modifier_value_for_source(
                        game,
                        perm_id,
                        controller,
                        &increase.increase,
                    );
                    if amount > 0 {
                        total_increase = total_increase.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        chosen_target_count,
                    )
                {
                    increase_pips.extend(increase.increase.pips().iter().cloned());
                }
                if let Some(reduction) = static_ability.cost_reduction_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        chosen_target_count,
                    )
                {
                    reduction_pips.extend(reduction.reduction.pips().iter().cloned());
                }
            }
        } else {
            for static_ability in perm
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    AbilityKind::Static(static_ability) => Some(static_ability),
                    _ => None,
                })
            {
                if let Some(reduction) = static_ability.cost_reduction()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        chosen_target_count,
                    )
                {
                    let amount = resolve_cost_modifier_value_for_source(
                        game,
                        perm_id,
                        controller,
                        &reduction.reduction,
                    );
                    if amount > 0 {
                        total_reduction = total_reduction.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        chosen_target_count,
                    )
                {
                    let amount = resolve_cost_modifier_value_for_source(
                        game,
                        perm_id,
                        controller,
                        &increase.increase,
                    );
                    if amount > 0 {
                        total_increase = total_increase.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        chosen_target_count,
                    )
                {
                    increase_pips.extend(increase.increase.pips().iter().cloned());
                }
                if let Some(reduction) = static_ability.cost_reduction_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        chosen_target_count,
                    )
                {
                    reduction_pips.extend(reduction.reduction.pips().iter().cloned());
                }
            }
        }
    }

    let mut adjusted = cost.clone();
    if !increase_pips.is_empty() {
        adjusted = add_mana_cost(&adjusted, &ManaCost::from_pips(increase_pips));
    }
    if total_increase > 0 {
        adjusted = add_generic_mana_cost(&adjusted, total_increase as u32);
    }
    if total_reduction > 0 {
        adjusted = adjusted.reduce_generic(total_reduction as u32);
    }
    if !reduction_pips.is_empty() {
        adjusted = reduce_mana_cost(&adjusted, &ManaCost::from_pips(reduction_pips));
    }
    adjusted
}

fn resolve_this_spell_cost_reduction_value(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    reduction: &crate::static_abilities::ThisSpellCostReduction,
) -> i32 {
    if matches!(
        (&reduction.condition, &reduction.reduction),
        (
            crate::static_abilities::ThisSpellCostCondition::LifeTotalLessThanStarting,
            crate::effect::Value::X
        )
    ) {
        if let Some(player_state) = game.player(player) {
            return player_state
                .starting_life
                .saturating_sub(player_state.life)
                .max(0);
        }
    }

    resolve_cost_modifier_value(game, player, spell, &reduction.reduction)
}

fn add_generic_mana_cost(cost: &crate::mana::ManaCost, increase: u32) -> crate::mana::ManaCost {
    if increase == 0 {
        return cost.clone();
    }
    use crate::mana::ManaSymbol;

    let mut new_pips = cost.pips().to_vec();
    let mut remaining = increase;
    while remaining > 0 {
        let chunk = remaining.min(u8::MAX as u32) as u8;
        new_pips.push(vec![ManaSymbol::Generic(chunk)]);
        remaining -= chunk as u32;
    }

    crate::mana::ManaCost::from_pips(new_pips)
}

fn add_mana_cost(
    cost: &crate::mana::ManaCost,
    add: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    if add.pips().is_empty() {
        return cost.clone();
    }
    let mut new_pips = cost.pips().to_vec();
    new_pips.extend(add.pips().iter().cloned());
    crate::mana::ManaCost::from_pips(new_pips)
}

pub(crate) fn reduce_mana_cost(
    cost: &crate::mana::ManaCost,
    reduction: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    use crate::mana::ManaSymbol;

    if reduction.pips().is_empty() {
        return cost.clone();
    }
    let mut pips = cost.pips().to_vec();
    let mut generic_reduction: u32 = 0;
    for red_pip in reduction.pips() {
        if red_pip.len() == 1
            && let ManaSymbol::Generic(amount) = red_pip[0]
        {
            generic_reduction = generic_reduction.saturating_add(amount as u32);
            continue;
        }
        if let Some(pos) = pips.iter().position(|pip| pip == red_pip) {
            pips.remove(pos);
        }
    }
    let reduced = crate::mana::ManaCost::from_pips(pips);
    if generic_reduction > 0 {
        reduced.reduce_generic(generic_reduction)
    } else {
        reduced
    }
}

fn resolve_cost_modifier_value(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    value: &crate::effect::Value,
) -> i32 {
    let mut dm = SelectFirstDecisionMaker;
    let ctx = ExecutionContext::new(spell.id, player, &mut dm);
    resolve_value(game, value, &ctx).unwrap_or(0)
}

fn resolve_cost_modifier_value_for_source(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    value: &crate::effect::Value,
) -> i32 {
    let mut dm = SelectFirstDecisionMaker;
    let ctx = ExecutionContext::new(source, controller, &mut dm);
    resolve_value(game, value, &ctx).unwrap_or(0)
}

/// Calculate the number of cards that need to be exiled for Delve.
///
/// Returns how many cards from graveyard should be exiled based on:
/// - The generic mana remaining in the cost after other reductions
/// - The player's available mana
/// - Cards available in graveyard
pub fn calculate_delve_exile_count(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> u32 {
    calculate_delve_exile_count_with_targets(game, player, spell, base_cost, 1)
}

/// Calculate the number of cards to exile for Delve with explicit target count.
pub fn calculate_delve_exile_count_with_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
) -> u32 {
    use crate::ability::AbilityKind;

    // Only calculate Delve if the spell actually has Delve
    let has_delve_ability = spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_delve()
        } else {
            false
        }
    });
    if !has_delve_ability {
        return 0;
    }

    // First apply other cost reductions (like Affinity)
    let mut cost_after_reductions = base_cost.clone();

    let has_affinity = spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_affinity()
        } else {
            false
        }
    });

    if has_affinity {
        let artifact_count = count_artifacts_controlled(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(artifact_count);
    }

    cost_after_reductions = apply_spell_cost_modifiers(
        game,
        player,
        spell,
        &cost_after_reductions,
        chosen_target_count,
        &[],
    );

    // Now calculate how much generic mana remains
    let generic_remaining = cost_after_reductions.generic_mana_total();

    // Get graveyard count and calculate exile amount
    let graveyard_count = count_cards_in_graveyard(game, player);

    // Exile up to the generic mana cost (maximum Delve)
    generic_remaining.min(graveyard_count)
}

/// Count the number of artifacts controlled by a player.
pub fn count_artifacts_controlled(game: &GameState, player: PlayerId) -> u32 {
    let view = DerivedGameView::new(game);
    count_artifacts_controlled_with_view(game, player, &view)
}

fn count_artifacts_controlled_with_view(
    game: &GameState,
    player: PlayerId,
    view: &DerivedGameView<'_>,
) -> u32 {
    game.battlefield
        .iter()
        .filter(|&&id| {
            if let Some(obj) = game.object(id) {
                obj.controller == player
                    && view.object_has_card_type(id, crate::types::CardType::Artifact)
            } else {
                false
            }
        })
        .count() as u32
}

/// Check if a spell has the Delve ability.
pub fn has_delve(spell: &crate::object::Object) -> bool {
    use crate::ability::AbilityKind;
    spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_delve()
        } else {
            false
        }
    })
}

/// Count cards in a player's graveyard (for Delve calculation).
pub fn count_cards_in_graveyard(game: &GameState, player: PlayerId) -> u32 {
    game.player(player)
        .map(|p| p.graveyard.len() as u32)
        .unwrap_or(0)
}

/// Compute potential mana available to a player.
///
/// This includes:
/// - Current mana pool
/// - Mana from all untapped lands and mana sources that can be activated
///
/// Returns a ManaPool representing the maximum mana the player could produce.
pub fn compute_potential_mana(game: &GameState, player: PlayerId) -> crate::player::ManaPool {
    let view = DerivedGameView::new(game);
    compute_potential_mana_with_view(game, player, &view)
}

pub(crate) fn compute_potential_mana_with_view(
    game: &GameState,
    player: PlayerId,
    view: &DerivedGameView<'_>,
) -> crate::player::ManaPool {
    use crate::ability::AbilityKind;
    use crate::costs::{CostCheckContext, can_pay_with_check_context};

    // Start with current mana pool
    let mut potential = game
        .player(player)
        .map(|p| p.mana_pool.clone())
        .unwrap_or_default();
    let simple_mana_analysis = view.simple_battlefield_mana_analysis(player);

    // Add mana from all available mana abilities.
    // The pass-local analysis already found the controlled mana sources.
    for &perm_id in simple_mana_analysis.mana_source_ids() {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };

        if let Some(symbols) = simple_mana_analysis.first_output_for(perm_id) {
            for mana in symbols {
                potential.add(*mana, 1);
            }
            continue;
        }

        let cached_abilities = view.abilities_rc(perm_id);
        let abilities = cached_abilities.as_deref().unwrap_or(&perm.abilities);
        for &ability_idx in simple_mana_analysis.mana_ability_indices_for(perm_id) {
            let Some(ability) = abilities.get(ability_idx) else {
                continue;
            };
            let AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if !mana_ability.is_mana_ability() {
                continue;
            }
            if mana_ability.has_tap_cost() && !game.can_activate_tap_abilities_of(perm_id) {
                continue;
            }
            // Do a simple non-recursive check for whether this mana ability
            // could be activated. We intentionally skip mana cost checks here
            // to avoid infinite recursion (mana ability with mana cost would
            // call compute_potential_mana again).
            let simple_taplike_costs_only = mana_ability.mana_cost.costs().iter().all(|cost| {
                cost.processing_mode().is_mana_payment()
                    || cost.requires_tap()
                    || cost.requires_untap()
            });

            let can_activate = if simple_taplike_costs_only {
                mana_ability.mana_cost.costs().iter().all(|cost| {
                    if cost.requires_tap() {
                        return !game.is_tapped(perm_id)
                            && (!view
                                .object_has_card_type(perm_id, crate::types::CardType::Creature)
                                || !game.is_summoning_sick(perm_id)
                                || view.object_has_static_ability_id(
                                    perm_id,
                                    crate::static_abilities::StaticAbilityId::Haste,
                                ));
                    }
                    if cost.requires_untap() {
                        return game.is_tapped(perm_id)
                            && (!view
                                .object_has_card_type(perm_id, crate::types::CardType::Creature)
                                || !game.is_summoning_sick(perm_id)
                                || view.object_has_static_ability_id(
                                    perm_id,
                                    crate::static_abilities::StaticAbilityId::Haste,
                                ));
                    }
                    true
                })
            } else {
                let ctx = CostCheckContext::new(perm_id, player)
                    .with_reason(crate::costs::PaymentReason::ActivateManaAbility);
                mana_ability.mana_cost.costs().iter().all(|cost| {
                    // Skip mana cost check to avoid recursion - we only check
                    // non-mana costs like tap, life, sacrifice
                    if cost.processing_mode().is_mana_payment() {
                        // Assume mana costs could be paid from other sources
                        // This is an approximation but prevents infinite recursion
                        true
                    } else {
                        if game
                            .validate_cost_for_payment_reason(player, perm_id, cost, ctx.reason)
                            .is_err()
                        {
                            return false;
                        }
                        can_pay_with_check_context(&*cost.0, game, &ctx).is_ok()
                    }
                })
            };

            // Also check activation condition if present
            let condition_met = mana_ability
                .activation_condition
                .as_ref()
                .is_none_or(|cond| {
                    check_mana_ability_condition_for_potential(
                        game,
                        player,
                        perm_id,
                        ability_idx,
                        cond,
                    )
                });

            if can_activate && condition_met {
                // Add the mana this ability could produce, preserving
                // multiplicity for effects like Black Lotus.
                for mana in
                    inferred_potential_mana_symbols_for_ability(game, perm_id, player, mana_ability)
                {
                    potential.add(mana, 1);
                }
                // Only count one mana ability per permanent (can only tap once)
                break;
            }
        }
    }

    potential
}

pub(crate) fn simple_battlefield_mana_ability_output(
    game: &GameState,
    player: PlayerId,
    permanent_id: ObjectId,
    ability_index: usize,
    ability: &crate::ability::Ability,
    view: &DerivedGameView<'_>,
) -> Option<Vec<ManaSymbol>> {
    use crate::ability::AbilityKind;

    let object = game.object(permanent_id)?;
    if object.controller != player
        || object.zone != Zone::Battlefield
        || !ability.functions_in(&object.zone)
        || !ability.is_mana_ability()
    {
        return None;
    }

    let AbilityKind::Activated(mana_ability) = &ability.kind else {
        return None;
    };
    if !mana_ability.is_mana_ability() || !game.can_activate_abilities_of(permanent_id) {
        return None;
    }
    if mana_ability.has_tap_cost() && !game.can_activate_tap_abilities_of(permanent_id) {
        return None;
    }
    if !mana_ability
        .mana_cost
        .costs()
        .iter()
        .all(|cost| cost.requires_tap() || cost.requires_untap())
    {
        return None;
    }

    for cost in mana_ability.mana_cost.costs() {
        if cost.requires_tap() {
            if game.is_tapped(permanent_id) {
                return None;
            }
            if view.object_has_card_type(permanent_id, crate::types::CardType::Creature)
                && game.is_summoning_sick(permanent_id)
                && !view.object_has_static_ability_id(
                    permanent_id,
                    crate::static_abilities::StaticAbilityId::Haste,
                )
            {
                return None;
            }
        }
        if cost.requires_untap() && !game.is_tapped(permanent_id) {
            return None;
        }
        if cost.requires_untap()
            && view.object_has_card_type(permanent_id, crate::types::CardType::Creature)
            && game.is_summoning_sick(permanent_id)
            && !view.object_has_static_ability_id(
                permanent_id,
                crate::static_abilities::StaticAbilityId::Haste,
            )
        {
            return None;
        }
    }

    if let Some(condition) = &mana_ability.activation_condition
        && !check_mana_ability_condition_for_potential(
            game,
            player,
            permanent_id,
            ability_index,
            condition,
        )
    {
        return None;
    }

    Some(inferred_potential_mana_symbols_for_ability(
        game,
        permanent_id,
        player,
        mana_ability,
    ))
}

fn inferred_potential_mana_symbols_for_ability(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    mana_ability: &crate::ability::ActivatedAbility,
) -> Vec<ManaSymbol> {
    use crate::effects::{
        AddColorlessManaEffect, AddManaEffect, AddManaOfAnyColorEffect, AddManaOfAnyOneColorEffect,
        AddScaledManaEffect,
    };

    if let Some(mana_output) = mana_ability.mana_output.as_ref()
        && !mana_output.is_empty()
    {
        return mana_output.clone();
    }

    let resolve_amount = |value: &crate::effect::Value| -> usize {
        let mut dm = SelectFirstDecisionMaker;
        let ctx = ExecutionContext::new(source, controller, &mut dm);
        resolve_value(game, value, &ctx).unwrap_or(0).max(0) as usize
    };

    let mut inferred = Vec::new();
    for effect in &mana_ability.effects {
        if let Some(add_mana) = effect.downcast_ref::<AddManaEffect>() {
            inferred.extend(add_mana.mana.iter().copied());
            continue;
        }
        if let Some(add_colorless) = effect.downcast_ref::<AddColorlessManaEffect>() {
            inferred.extend(std::iter::repeat_n(
                ManaSymbol::Colorless,
                resolve_amount(&add_colorless.amount),
            ));
            continue;
        }
        if let Some(add_scaled) = effect.downcast_ref::<AddScaledManaEffect>() {
            let repeats = resolve_amount(&add_scaled.amount);
            for _ in 0..repeats {
                inferred.extend(add_scaled.mana.iter().copied());
            }
            continue;
        }
        if let Some(add_any_color) = effect.downcast_ref::<AddManaOfAnyColorEffect>() {
            let amount = resolve_amount(&add_any_color.amount);
            let colors = add_any_color.available_colors.as_deref().unwrap_or(&[
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ]);
            for color in colors {
                inferred.extend(std::iter::repeat_n(ManaSymbol::from_color(*color), amount));
            }
            continue;
        }
        if let Some(add_any_one_color) = effect.downcast_ref::<AddManaOfAnyOneColorEffect>() {
            let amount = resolve_amount(&add_any_one_color.amount);
            for color in [
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ] {
                inferred.extend(std::iter::repeat_n(ManaSymbol::from_color(color), amount));
            }
            continue;
        }

        if let Some(symbols) = effect.producible_mana_symbols(game, source, controller) {
            inferred.extend(symbols.into_iter().filter(|symbol| {
                matches!(
                    symbol,
                    ManaSymbol::White
                        | ManaSymbol::Blue
                        | ManaSymbol::Black
                        | ManaSymbol::Red
                        | ManaSymbol::Green
                        | ManaSymbol::Colorless
                )
            }));
        }
    }

    inferred
}

/// Check mana ability condition for potential mana computation.
fn check_mana_ability_condition_for_potential(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    ability_index: usize,
    condition: &crate::ConditionExpr,
) -> bool {
    let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
        controller: player,
        source,
        defending_player: None,
        attacking_player: None,
        filter_source: Some(source),
        triggering_event: None,
        trigger_identity: None,
        ability_index: Some(ability_index),
        options: crate::condition_eval::ExternalEvaluationOptions::default(),
    };
    crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
}

/// Check if a player could pay a mana cost using potential mana.
///
/// This considers mana currently in pool plus mana from untapped sources.
pub fn can_potentially_pay(
    game: &GameState,
    player: PlayerId,
    cost: &crate::mana::ManaCost,
    x_value: u32,
) -> bool {
    let potential = compute_potential_mana(game, player);
    potential.can_pay(cost, x_value)
}

/// Calculate the effective mana cost for a spell with Delve, given available graveyard cards.
///
/// For Delve, each card exiled from graveyard pays for {1} of generic mana.
/// This function calculates the minimum mana needed given maximum Delve usage.
pub fn calculate_delve_effective_cost(
    base_cost: &crate::mana::ManaCost,
    available_graveyard_cards: u32,
) -> crate::mana::ManaCost {
    let generic_in_cost = base_cost.generic_mana_total();
    let delve_amount = generic_in_cost.min(available_graveyard_cards);
    base_cost.reduce_generic(delve_amount)
}

/// Calculate how many cards to exile for Delve to minimize mana cost while being castable.
///
/// Returns (cards_to_exile, effective_mana_cost).
/// This greedily exiles cards to pay generic mana.
pub fn calculate_optimal_delve(
    game: &GameState,
    player: PlayerId,
    base_cost: &crate::mana::ManaCost,
) -> (u32, crate::mana::ManaCost) {
    let graveyard_count = count_cards_in_graveyard(game, player);
    let generic_in_cost = base_cost.generic_mana_total();

    // Exile up to the generic mana cost
    let delve_amount = generic_in_cost.min(graveyard_count);
    let effective_cost = base_cost.reduce_generic(delve_amount);

    (delve_amount, effective_cost)
}

/// Check if a spell has the Convoke ability.
pub fn has_convoke(spell: &crate::object::Object) -> bool {
    use crate::ability::AbilityKind;
    spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_convoke()
        } else {
            false
        }
    })
}

/// Calculate which creatures to tap for Convoke.
///
/// Returns the creature IDs to tap for maximum Convoke usage.
/// This takes into account Affinity and Delve reductions first.
pub fn calculate_convoke_creatures_to_tap(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> Vec<crate::ids::ObjectId> {
    use crate::ability::AbilityKind;

    if !has_convoke(spell) {
        return Vec::new();
    }

    // First apply other cost reductions (like Affinity and Delve)
    let mut cost_after_reductions = base_cost.clone();

    let has_affinity = spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_affinity()
        } else {
            false
        }
    });

    if has_affinity {
        let artifact_count = count_artifacts_controlled(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(artifact_count);
    }

    cost_after_reductions =
        apply_spell_cost_modifiers(game, player, spell, &cost_after_reductions, 1, &[]);

    let has_delve_ability = has_delve(spell);

    if has_delve_ability {
        let graveyard_count = count_cards_in_graveyard(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(graveyard_count);
    }

    // Now calculate Convoke creatures to tap
    let (creatures_to_tap, _) = calculate_convoke_cost(game, player, &cost_after_reductions);
    creatures_to_tap
}

/// Check if a spell has the Improvise ability.
pub fn has_improvise(spell: &crate::object::Object) -> bool {
    use crate::ability::AbilityKind;
    spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_improvise()
        } else {
            false
        }
    })
}

/// Get untapped artifacts controlled by a player that can be tapped for Improvise.
///
/// Returns a list of artifact ObjectIds.
pub fn get_improvise_artifacts(game: &GameState, player: PlayerId) -> Vec<crate::ids::ObjectId> {
    game.battlefield
        .iter()
        .filter_map(|&id| {
            let obj = game.object(id)?;
            // Must be an artifact controlled by player
            if obj.controller != player || !obj.has_card_type(crate::types::CardType::Artifact) {
                return None;
            }
            // Must be untapped
            if game.is_tapped(id) {
                return None;
            }
            Some(id)
        })
        .collect()
}

/// Calculate the effective mana cost for a spell with Improvise.
///
/// For Improvise, each artifact tapped pays for {1} of generic mana.
/// Returns (artifacts_to_tap, effective_mana_cost).
pub fn calculate_improvise_cost(
    game: &GameState,
    player: PlayerId,
    cost: &crate::mana::ManaCost,
) -> (Vec<crate::ids::ObjectId>, crate::mana::ManaCost) {
    use crate::mana::ManaSymbol;

    let improvise_artifacts = get_improvise_artifacts(game, player);
    if improvise_artifacts.is_empty() {
        return (Vec::new(), cost.clone());
    }

    let mut artifacts_to_tap = Vec::new();
    let mut remaining_pips: Vec<Vec<ManaSymbol>> = cost.pips().to_vec();

    // Improvise only pays generic mana
    let mut i = 0;
    while i < remaining_pips.len() && artifacts_to_tap.len() < improvise_artifacts.len() {
        let pip = &remaining_pips[i];

        // Check if this is a generic pip
        if pip.len() == 1
            && let ManaSymbol::Generic(n) = pip[0]
        {
            let available = improvise_artifacts.len() - artifacts_to_tap.len();
            let to_tap = (n as usize).min(available);

            for j in 0..to_tap {
                artifacts_to_tap.push(improvise_artifacts[artifacts_to_tap.len()]);
                let _ = j; // Suppress unused warning
            }

            // Reduce or remove the generic pip
            let paid = to_tap as u8;
            if paid >= n {
                remaining_pips.remove(i);
                continue;
            } else {
                remaining_pips[i] = vec![ManaSymbol::Generic(n - paid)];
            }
        }
        i += 1;
    }

    let effective_cost = crate::mana::ManaCost::from_pips(remaining_pips);
    (artifacts_to_tap, effective_cost)
}

/// Calculate which artifacts to tap for Improvise.
///
/// Returns the artifact IDs to tap for maximum Improvise usage.
/// This takes into account Affinity, Delve, and Convoke reductions first.
pub fn calculate_improvise_artifacts_to_tap(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> Vec<crate::ids::ObjectId> {
    use crate::ability::AbilityKind;

    if !has_improvise(spell) {
        return Vec::new();
    }

    // First apply other cost reductions (Affinity, Delve, Convoke)
    let mut cost_after_reductions = base_cost.clone();

    let has_affinity = spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_affinity()
        } else {
            false
        }
    });

    if has_affinity {
        let artifact_count = count_artifacts_controlled(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(artifact_count);
    }

    cost_after_reductions =
        apply_spell_cost_modifiers(game, player, spell, &cost_after_reductions, 1, &[]);

    let has_delve_ability = has_delve(spell);

    if has_delve_ability {
        let graveyard_count = count_cards_in_graveyard(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(graveyard_count);
    }

    let has_convoke_ability = has_convoke(spell);

    if has_convoke_ability {
        let (_, convoked_cost) = calculate_convoke_cost(game, player, &cost_after_reductions);
        cost_after_reductions = convoked_cost;
    }

    // Now calculate Improvise artifacts to tap
    let (artifacts_to_tap, _) = calculate_improvise_cost(game, player, &cost_after_reductions);
    artifacts_to_tap
}

/// Count untapped creatures controlled by a player that can be tapped for convoke.
///
/// Returns a tuple of (total_untapped_creatures, creature_ids_with_colors).
pub fn get_convoke_creatures(
    game: &GameState,
    player: PlayerId,
) -> Vec<(crate::ids::ObjectId, crate::color::ColorSet)> {
    game.battlefield
        .iter()
        .filter_map(|&id| {
            let obj = game.object(id)?;
            // Must be a creature controlled by player
            if obj.controller != player || !game.current_is_creature(id) {
                return None;
            }
            // Must be untapped
            if game.is_tapped(id) {
                return None;
            }
            Some((id, game.current_colors(id).unwrap_or_else(|| obj.colors())))
        })
        .collect()
}

/// Calculate the effective mana cost for a spell with Convoke.
///
/// For Convoke, each creature tapped can pay for {1} or one mana of its colors.
/// This function calculates the minimum mana needed given maximum Convoke usage.
///
/// Returns (creatures_to_tap, effective_mana_cost).
pub fn calculate_convoke_cost(
    game: &GameState,
    player: PlayerId,
    cost: &crate::mana::ManaCost,
) -> (Vec<crate::ids::ObjectId>, crate::mana::ManaCost) {
    use crate::mana::ManaSymbol;

    let convoke_creatures = get_convoke_creatures(game, player);
    if convoke_creatures.is_empty() {
        return (Vec::new(), cost.clone());
    }

    let mut creatures_to_tap = Vec::new();
    let mut remaining_pips: Vec<Vec<ManaSymbol>> = cost.pips().to_vec();
    let mut available_creatures = convoke_creatures;

    // First pass: pay colored mana with matching creatures
    let mut i = 0;
    while i < remaining_pips.len() {
        let pip = &remaining_pips[i];

        // Check if this is a single colored pip
        if pip.len() == 1 {
            let color_opt = match pip[0] {
                ManaSymbol::White => Some(crate::color::Color::White),
                ManaSymbol::Blue => Some(crate::color::Color::Blue),
                ManaSymbol::Black => Some(crate::color::Color::Black),
                ManaSymbol::Red => Some(crate::color::Color::Red),
                ManaSymbol::Green => Some(crate::color::Color::Green),
                _ => None,
            };

            if let Some(color) = color_opt {
                // Find a creature with this color
                if let Some(idx) = available_creatures
                    .iter()
                    .position(|(_, colors)| colors.contains(color))
                {
                    let (creature_id, _) = available_creatures.remove(idx);
                    creatures_to_tap.push(creature_id);
                    remaining_pips.remove(i);
                    continue;
                }
            }
        }
        i += 1;
    }

    // Second pass: pay generic mana with any remaining creatures
    let mut i = 0;
    while i < remaining_pips.len() && !available_creatures.is_empty() {
        let pip = &remaining_pips[i];

        // Check if this is a generic pip
        if pip.len() == 1
            && let ManaSymbol::Generic(n) = pip[0]
        {
            let creatures_needed = (n as usize).min(available_creatures.len());
            for _ in 0..creatures_needed {
                let (creature_id, _) = available_creatures.remove(0);
                creatures_to_tap.push(creature_id);
            }

            // Reduce or remove the generic pip
            let paid = creatures_needed as u8;
            if paid >= n {
                remaining_pips.remove(i);
                continue;
            } else {
                remaining_pips[i] = vec![ManaSymbol::Generic(n - paid)];
            }
        }
        i += 1;
    }

    let effective_cost = crate::mana::ManaCost::from_pips(remaining_pips);
    (creatures_to_tap, effective_cost)
}

/// Compute legal attackers for the active player.
pub fn compute_legal_attackers(game: &GameState, _combat: &CombatState) -> Vec<AttackerOption> {
    let mut options = Vec::new();
    let active_player = game.turn.active_player;
    let mut attack_capable = Vec::new();
    let view = DerivedGameView::new(game);

    for &perm_id in &game.battlefield {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        if perm.controller != active_player {
            continue;
        }
        if !view.object_has_card_type(perm_id, crate::types::CardType::Creature) {
            continue;
        }
        if crate::rules::combat::can_attack_with_view(perm, game, &view) {
            attack_capable.push(perm_id);
        }
    }
    let has_other_attacker = attack_capable.len() >= 2;

    // Find all creatures controlled by active player that can attack
    for &perm_id in &attack_capable {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        if !has_other_attacker && !game.can_attack_alone(perm_id) {
            continue;
        }

        let goaded_by = game.active_goaders_for(perm.id);

        // Determine valid attack targets
        let mut valid_targets = Vec::new();

        // Can attack each opponent
        let mut goad_targets = Vec::new();
        let mut nongoad_targets = Vec::new();

        for opponent in &game.players {
            if opponent.id != active_player && opponent.is_in_game() {
                let target = AttackTarget::Player(opponent.id);
                if can_declare_attack_target_preview(game, perm, &target, &view) {
                    if goaded_by.contains(&opponent.id) {
                        goad_targets.push(target);
                    } else {
                        nongoad_targets.push(target);
                    }
                }
            }
        }

        // Can attack planeswalkers controlled by opponents
        for &other_perm_id in &game.battlefield {
            if let Some(other_perm) = game.object(other_perm_id)
                && other_perm.controller != active_player
                && view.object_has_card_type(other_perm_id, crate::types::CardType::Planeswalker)
            {
                let target = AttackTarget::Planeswalker(other_perm_id);
                if can_declare_attack_target_preview(game, perm, &target, &view) {
                    if goaded_by.contains(&other_perm.controller) {
                        goad_targets.push(target);
                    } else {
                        nongoad_targets.push(target);
                    }
                }
            }
        }

        if !nongoad_targets.is_empty() {
            valid_targets.extend(nongoad_targets);
        } else {
            valid_targets.extend(goad_targets);
        }

        let must_attack = crate::rules::combat::must_attack_with_view(perm, game, &view);

        if !valid_targets.is_empty() {
            options.push(AttackerOption {
                creature: perm_id,
                valid_targets,
                must_attack,
            });
        }
    }

    options
}

/// Compute legal blockers for the defending player.
pub fn compute_legal_blockers(
    game: &GameState,
    combat: &CombatState,
    defending_player: PlayerId,
) -> Vec<BlockerOption> {
    use std::collections::HashSet;

    let mut options = Vec::new();
    let mut potential_blockers = HashSet::new();
    let view = DerivedGameView::new(game);

    // For each attacker, find creatures that can block it
    for attacker_info in &combat.attackers {
        let attacker_id = attacker_info.creature;
        let Some(attacker) = game.object(attacker_id) else {
            continue;
        };

        let mut valid_blockers = Vec::new();

        // Find creatures controlled by defending player that can block this attacker
        for &perm_id in &game.battlefield {
            let Some(blocker) = game.object(perm_id) else {
                continue;
            };

            if blocker.controller != defending_player {
                continue;
            }

            if !view.object_has_card_type(perm_id, crate::types::CardType::Creature) {
                continue;
            }

            // Check if this creature can block this attacker
            if crate::rules::combat::can_block_with_view(attacker, blocker, game, &view) {
                valid_blockers.push(perm_id);
                potential_blockers.insert(perm_id);
            }
        }

        let min_blockers = crate::rules::combat::minimum_blockers_with_view(attacker, &view);

        options.push(BlockerOption {
            attacker: attacker_id,
            valid_blockers,
            min_blockers,
        });
    }

    if potential_blockers.len() == 1
        && let Some(&only_blocker) = potential_blockers.iter().next()
        && !game.can_block_alone(only_blocker)
    {
        for option in &mut options {
            option.valid_blockers.clear();
        }
    }

    options
}

// ============================================================================
// Decision Maker Trait (for convenience)
// ============================================================================

/// Trait for something that can make player decisions.
///
/// This is a convenience trait for driving the game loop synchronously.
/// Implementations could be: AI, test harness, etc.
///
/// The trait is fully primitive-based: each `decide_*` method takes a typed
/// context and returns a typed response.
///
/// Default implementations provide deterministic minimal behavior, and
/// implementors can override the relevant methods for interactive or AI control.
pub trait DecisionMaker {
    /// Called when a player auto-passes (had no actions available).
    /// Default implementation does nothing.
    fn on_auto_pass(&mut self, _game: &GameState, _player: PlayerId) {}

    /// Called when an action chain is cancelled due to an invalid choice.
    /// The game state is restored to the checkpoint before the action started.
    /// Default implementation does nothing.
    fn on_action_cancelled(&mut self, _game: &GameState, _reason: &str) {}

    /// True when the decision maker is only surfacing a prompt and execution
    /// should avoid committing fallback choices to game state.
    fn awaiting_choice(&self) -> bool {
        false
    }

    // ========================================================================
    // Primitive-specific methods
    // ========================================================================
    // These methods take typed context structs and return typed responses.
    // Default implementations return minimal/declining choices.
    // Implementers should override these methods for meaningful behavior.

    /// Boolean decisions (may, ward, miracle, madness).
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        // Default: decline optional actions
        false
    }

    /// Number selection (X value, choose number).
    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        // Default: choose minimum value
        ctx.min
    }

    /// Free-form text entry decisions.
    fn decide_text(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        ctx.initial_value.clone().unwrap_or_default()
    }

    /// Object selection (sacrifice, discard, search, etc.).
    /// Returns IDs of selected objects.
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        // Default: select minimum required from legal candidates
        ctx.candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .take(ctx.min)
            .collect()
    }

    /// Option selection (modes, choices, priority actions, mana payment).
    /// Returns indices of selected options.
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        // Default: select minimum required from legal options
        ctx.options
            .iter()
            .filter(|o| o.legal)
            .map(|o| o.index)
            .take(ctx.min)
            .collect()
    }

    /// Ordering (blockers, attackers, scry, surveil).
    /// Returns the items in the desired order.
    fn decide_order(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        // Default: keep original order
        ctx.items.iter().map(|(id, _)| *id).collect()
    }

    /// View cards in a private zone (e.g., look at a player's hand).
    /// Default implementation does nothing.
    fn view_cards(
        &mut self,
        _game: &GameState,
        _viewer: PlayerId,
        _cards: &[ObjectId],
        _ctx: &crate::decisions::context::ViewCardsContext,
    ) {
    }

    /// Combat - attackers.
    fn decide_attackers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        // Default: don't attack with anything
        Vec::new()
    }

    /// Combat - blockers.
    fn decide_blockers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        // Default: don't block with anything
        Vec::new()
    }

    /// Distribution (damage, counters).
    fn decide_distribute(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        // Default: empty distribution
        Vec::new()
    }

    /// Color selection.
    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        // Default: green for all requested colors
        vec![crate::color::Color::Green; ctx.count as usize]
    }

    /// Counter removal.
    fn decide_counters(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        // Default: don't remove any counters
        Vec::new()
    }

    /// Partition (scry top/bottom, surveil library/graveyard).
    /// Returns the items to put in the "secondary" destination.
    fn decide_partition(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        // Default: keep all cards in primary destination
        Vec::new()
    }

    /// Proliferate (mixed objects and players).
    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        // Default: don't proliferate anything
        crate::decisions::specs::ProliferateResponse::default()
    }

    /// Priority decisions (choose action).
    fn decide_priority(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        // Default: pass priority if possible, otherwise take first action
        ctx.actions
            .iter()
            .find(|a| matches!(a, LegalAction::PassPriority))
            .cloned()
            .unwrap_or_else(|| {
                ctx.actions
                    .first()
                    .cloned()
                    .unwrap_or(LegalAction::PassPriority)
            })
    }

    /// Target selection for spells and abilities.
    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        normalize_targets_for_requirements(&ctx.requirements, Vec::new()).unwrap_or_default()
    }
}

/// Routes decisions to the controlling player's DecisionMaker.
///
/// This is used for effects that let one player control another player's decisions.
pub struct DecisionRouter {
    per_player: HashMap<PlayerId, Box<dyn DecisionMaker>>,
    default: Box<dyn DecisionMaker>,
}

impl DecisionRouter {
    /// Create a router with a default DecisionMaker.
    pub fn new(default: Box<dyn DecisionMaker>) -> Self {
        Self {
            per_player: HashMap::new(),
            default,
        }
    }

    /// Register a DecisionMaker for a specific player.
    pub fn with_player(mut self, player: PlayerId, dm: Box<dyn DecisionMaker>) -> Self {
        self.per_player.insert(player, dm);
        self
    }

    /// Replace the DecisionMaker for a specific player.
    pub fn set_player(&mut self, player: PlayerId, dm: Box<dyn DecisionMaker>) {
        self.per_player.insert(player, dm);
    }

    fn dm_for<'a>(&'a mut self, game: &GameState, player: PlayerId) -> &'a mut dyn DecisionMaker {
        let controller = game.controlling_player_for(player);
        if let Some(dm) = self.per_player.get_mut(&controller) {
            return dm.as_mut();
        }
        self.default.as_mut()
    }
}

impl DecisionMaker for DecisionRouter {
    fn on_auto_pass(&mut self, game: &GameState, player: PlayerId) {
        self.dm_for(game, player).on_auto_pass(game, player);
    }

    fn on_action_cancelled(&mut self, game: &GameState, reason: &str) {
        self.default.on_action_cancelled(game, reason);
        for dm in self.per_player.values_mut() {
            dm.on_action_cancelled(game, reason);
        }
    }

    fn decide_boolean(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.dm_for(game, ctx.player).decide_boolean(game, ctx)
    }

    fn decide_number(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        self.dm_for(game, ctx.player).decide_number(game, ctx)
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.dm_for(game, ctx.player).decide_objects(game, ctx)
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.dm_for(game, ctx.player).decide_options(game, ctx)
    }

    fn decide_text(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        self.dm_for(game, ctx.player).decide_text(game, ctx)
    }

    fn view_cards(
        &mut self,
        game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        self.dm_for(game, viewer)
            .view_cards(game, viewer, cards, ctx);
    }

    fn decide_order(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        self.dm_for(game, ctx.player).decide_order(game, ctx)
    }

    fn decide_attackers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        self.dm_for(game, ctx.player).decide_attackers(game, ctx)
    }

    fn decide_blockers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        self.dm_for(game, ctx.player).decide_blockers(game, ctx)
    }

    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        self.dm_for(game, ctx.player).decide_distribute(game, ctx)
    }

    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        self.dm_for(game, ctx.player).decide_colors(game, ctx)
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        self.dm_for(game, ctx.player).decide_counters(game, ctx)
    }

    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        self.dm_for(game, ctx.player).decide_partition(game, ctx)
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        self.dm_for(game, ctx.player).decide_proliferate(game, ctx)
    }

    fn decide_priority(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        self.dm_for(game, ctx.player).decide_priority(game, ctx)
    }

    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        self.dm_for(game, ctx.player).decide_targets(game, ctx)
    }
}

// ============================================================================
// Blanket implementations
// ============================================================================

/// Blanket impl so `&mut D` implements `DecisionMaker` where `D: DecisionMaker`.
/// This allows passing `&mut dyn DecisionMaker` to functions expecting `impl DecisionMaker`.
impl<D: DecisionMaker + ?Sized> DecisionMaker for &mut D {
    fn on_auto_pass(&mut self, game: &GameState, player: PlayerId) {
        (*self).on_auto_pass(game, player)
    }

    fn on_action_cancelled(&mut self, game: &GameState, reason: &str) {
        (*self).on_action_cancelled(game, reason)
    }

    fn awaiting_choice(&self) -> bool {
        (**self).awaiting_choice()
    }

    fn decide_boolean(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        (*self).decide_boolean(game, ctx)
    }

    fn decide_number(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        (*self).decide_number(game, ctx)
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        (*self).decide_objects(game, ctx)
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        (*self).decide_options(game, ctx)
    }

    fn decide_text(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        (*self).decide_text(game, ctx)
    }

    fn decide_order(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        (*self).decide_order(game, ctx)
    }

    fn decide_attackers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        (*self).decide_attackers(game, ctx)
    }

    fn decide_blockers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        (*self).decide_blockers(game, ctx)
    }

    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        (*self).decide_distribute(game, ctx)
    }

    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        (*self).decide_colors(game, ctx)
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        (*self).decide_counters(game, ctx)
    }

    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        (*self).decide_partition(game, ctx)
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        (*self).decide_proliferate(game, ctx)
    }

    fn decide_priority(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        (*self).decide_priority(game, ctx)
    }

    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        (*self).decide_targets(game, ctx)
    }
}

/// Blanket impl so `Box<D>` implements `DecisionMaker` where `D: DecisionMaker`.
/// This allows using `Box<dyn DecisionMaker>` in struct fields.
impl<D: DecisionMaker + ?Sized> DecisionMaker for Box<D> {
    fn on_auto_pass(&mut self, game: &GameState, player: PlayerId) {
        (**self).on_auto_pass(game, player)
    }

    fn on_action_cancelled(&mut self, game: &GameState, reason: &str) {
        (**self).on_action_cancelled(game, reason)
    }

    fn awaiting_choice(&self) -> bool {
        (**self).awaiting_choice()
    }

    fn decide_boolean(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        (**self).decide_boolean(game, ctx)
    }

    fn decide_number(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        (**self).decide_number(game, ctx)
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        (**self).decide_objects(game, ctx)
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        (**self).decide_options(game, ctx)
    }

    fn decide_text(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        (**self).decide_text(game, ctx)
    }

    fn decide_order(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        (**self).decide_order(game, ctx)
    }

    fn decide_attackers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        (**self).decide_attackers(game, ctx)
    }

    fn decide_blockers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        (**self).decide_blockers(game, ctx)
    }

    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        (**self).decide_distribute(game, ctx)
    }

    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        (**self).decide_colors(game, ctx)
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        (**self).decide_counters(game, ctx)
    }

    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        (**self).decide_partition(game, ctx)
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        (**self).decide_proliferate(game, ctx)
    }

    fn decide_priority(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        (**self).decide_priority(game, ctx)
    }

    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        (**self).decide_targets(game, ctx)
    }
}

/// A decision maker that always passes priority and makes minimal choices.
///
/// Useful for testing basic game flow.
#[derive(Debug, Default)]
pub struct AutoPassDecisionMaker;

fn auto_select_option_indices(
    ctx: &crate::decisions::context::SelectOptionsContext,
    desired_count: usize,
) -> Vec<usize> {
    let legal: Vec<&crate::decisions::context::SelectableOption> =
        ctx.options.iter().filter(|o| o.legal).collect();
    if legal.is_empty() || desired_count == 0 {
        return Vec::new();
    }

    let mut selected = Vec::new();
    let mut counts: HashMap<usize, usize> = HashMap::new();

    while selected.len() < desired_count {
        let mut added = false;
        for option in &legal {
            let current = counts.get(&option.index).copied().unwrap_or(0);
            let limit = if option.repeatable {
                option
                    .max_count
                    .map(|count| count as usize)
                    .unwrap_or(usize::MAX)
            } else {
                1
            };
            if current >= limit {
                continue;
            }
            selected.push(option.index);
            counts.insert(option.index, current + 1);
            added = true;
            break;
        }
        if !added {
            break;
        }
    }

    selected
}

impl DecisionMaker for AutoPassDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        // Auto-pass: decline all optional actions
        false
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        // Auto-pass: choose maximum value (most common for "up to" effects)
        ctx.max
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        // Auto-pass: select minimum required, using first legal candidates
        let legal: Vec<ObjectId> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();
        legal.into_iter().take(ctx.min).collect()
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        // Auto-pass: select minimum required, using first legal options
        auto_select_option_indices(ctx, ctx.min)
    }

    fn decide_text(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        ctx.initial_value.clone().unwrap_or_default()
    }

    fn decide_order(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        // Auto-pass: keep original order
        ctx.items.iter().map(|(id, _)| *id).collect()
    }

    fn decide_attackers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        // Auto-pass: don't attack
        Vec::new()
    }

    fn decide_blockers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        // Auto-pass: don't block
        Vec::new()
    }

    fn decide_distribute(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(crate::game_state::Target, u32)> {
        // Auto-pass: don't distribute anything
        Vec::new()
    }

    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        // Auto-pass: default to green for each mana
        let default_color = ctx
            .available_colors
            .as_ref()
            .and_then(|colors| colors.first().copied())
            .unwrap_or(crate::color::Color::Green);
        vec![default_color; ctx.count as usize]
    }

    fn decide_counters(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        // Auto-pass: remove as many counters as possible
        let mut remaining = ctx.max_total;
        let mut selections = Vec::new();
        for (counter_type, available) in &ctx.available_counters {
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

    fn decide_partition(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        // Auto-pass: keep all cards in primary destination (top of library)
        Vec::new()
    }

    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        // Auto-pass: proliferate all eligible targets
        crate::decisions::specs::ProliferateResponse {
            permanents: ctx.eligible_permanents.iter().map(|(id, _)| *id).collect(),
            players: ctx.eligible_players.iter().map(|(id, _)| *id).collect(),
        }
    }

    fn decide_priority(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        // Auto-pass: always pass priority
        LegalAction::PassPriority
    }
}

/// A decision maker that always selects the first available option.
///
/// Unlike `AutoPassDecisionMaker` which selects the minimum required (often 0),
/// this decision maker always selects the first legal option when available.
/// Useful for testing effects where you want to verify behavior when a choice is made.
#[derive(Debug, Default)]
pub struct SelectFirstDecisionMaker;

impl DecisionMaker for SelectFirstDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        // Select first: accept optional actions
        true
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        // Select first: choose maximum value
        ctx.max
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        // Select first: select first legal option (up to max)
        let legal: Vec<ObjectId> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();
        let count = ctx.max.unwrap_or(1).min(legal.len());
        legal.into_iter().take(count).collect()
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        // Select first: select first legal option (up to max)
        auto_select_option_indices(ctx, ctx.max)
    }

    fn decide_order(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        // Keep original order
        ctx.items.iter().map(|(id, _)| *id).collect()
    }

    fn decide_attackers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        Vec::new()
    }

    fn decide_blockers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        Vec::new()
    }

    fn decide_distribute(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(crate::game_state::Target, u32)> {
        Vec::new()
    }

    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        let default_color = ctx
            .available_colors
            .as_ref()
            .and_then(|colors| colors.first().copied())
            .unwrap_or(crate::color::Color::Green);
        vec![default_color; ctx.count as usize]
    }

    fn decide_counters(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        let mut remaining = ctx.max_total;
        let mut selections = Vec::new();
        for (counter_type, available) in &ctx.available_counters {
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

    fn decide_partition(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        Vec::new()
    }

    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        crate::decisions::specs::ProliferateResponse {
            permanents: ctx.eligible_permanents.iter().map(|(id, _)| *id).collect(),
            players: ctx.eligible_players.iter().map(|(id, _)| *id).collect(),
        }
    }

    fn decide_priority(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        LegalAction::PassPriority
    }
}

/// A decision maker that interprets numeric string inputs the same way the CLI does.
///
/// This allows tests to use the same input format as recorded sessions from `--record`.
/// Empty strings are treated as "pass" or "no selection" depending on context.
///
/// # Example
///
/// ```ignore
/// // Simulate: pass priority, then take action 0, then pass again
/// let mut dm = NumericInputDecisionMaker::new(vec!["".to_string(), "0".to_string(), "".to_string()]);
/// ```
#[derive(Debug)]
pub struct NumericInputDecisionMaker {
    inputs: Vec<String>,
    index: usize,
    debug: bool,
}

impl NumericInputDecisionMaker {
    /// Create a new numeric input decision maker with the given inputs.
    pub fn new(inputs: Vec<String>) -> Self {
        Self {
            inputs,
            index: 0,
            debug: false,
        }
    }

    /// Create from a slice of string references.
    pub fn from_strs(inputs: &[&str]) -> Self {
        Self {
            inputs: inputs.iter().map(|s| s.to_string()).collect(),
            index: 0,
            debug: false,
        }
    }

    /// Enable debug output for tracing decisions.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Get the next input, or empty string if exhausted.
    fn next_input(&mut self) -> String {
        if self.index < self.inputs.len() {
            let input = self.inputs[self.index].clone();
            self.index += 1;
            input
        } else {
            String::new()
        }
    }
}

impl DecisionMaker for NumericInputDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision boolean[{}]: {} | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                input
            );
        }

        // "y"/"yes"/"1" = true, empty/"n"/"no" = false
        matches!(trimmed.to_lowercase().as_str(), "y" | "yes" | "1")
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision number[{}]: {} (min={}, max={}) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.min,
                ctx.max,
                input
            );
        }

        if let Ok(n) = trimmed.parse::<u32>()
            && n >= ctx.min
            && n <= ctx.max
        {
            return n;
        }
        ctx.min
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision objects[{}]: {} ({} candidates, min={}, max={}) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.candidates.len(),
                ctx.min,
                ctx.max
                    .map(|max| max.to_string())
                    .unwrap_or_else(|| "any".to_string()),
                input
            );
        }

        if trimmed.is_empty() && (ctx.min == 0 || ctx.allow_partial_completion) {
            return Vec::new();
        }

        let legal: Vec<ObjectId> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();

        let mut selected = Vec::new();
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>()
                && idx < legal.len()
            {
                if let Some(max) = ctx.max {
                    if selected.len() < max {
                        selected.push(legal[idx]);
                    }
                } else {
                    selected.push(legal[idx]);
                }
            }
        }

        // If we didn't select enough, auto-select from beginning
        if !ctx.allow_partial_completion {
            while selected.len() < ctx.min && selected.len() < legal.len() {
                if !selected.contains(&legal[selected.len()]) {
                    selected.push(legal[selected.len()]);
                } else {
                    break;
                }
            }
        }

        selected
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision options[{}]: {} ({} options, min={}, max={}) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.options.len(),
                ctx.min,
                ctx.max,
                input
            );
        }

        if trimmed.is_empty() && ctx.min == 0 {
            return Vec::new();
        }

        let legal: Vec<usize> = ctx
            .options
            .iter()
            .filter(|o| o.legal)
            .map(|o| o.index)
            .collect();

        let mut selected = Vec::new();
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>()
                && idx < legal.len()
                && selected.len() < ctx.max
            {
                selected.push(legal[idx]);
            }
        }

        // If we didn't select enough, auto-select from beginning
        while selected.len() < ctx.min && selected.len() < legal.len() {
            if !selected.contains(&legal[selected.len()]) {
                selected.push(legal[selected.len()]);
            } else {
                break;
            }
        }

        selected
    }

    fn decide_order(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision order[{}]: {} ({} items) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.items.len(),
                input
            );
        }

        let items: Vec<ObjectId> = ctx.items.iter().map(|(id, _)| *id).collect();

        if trimmed.is_empty() {
            return items; // Keep original order
        }

        // Parse comma-separated indices to reorder
        let mut ordered = Vec::new();
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>()
                && idx < items.len()
                && !ordered.contains(&items[idx])
            {
                ordered.push(items[idx]);
            }
        }

        // Add any remaining items not specified
        for id in items {
            if !ordered.contains(&id) {
                ordered.push(id);
            }
        }

        ordered
    }

    fn decide_attackers(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision attackers[{}]: {} options | input '{}'",
                self.index.saturating_sub(1),
                ctx.attacker_options.len(),
                input
            );
        }

        if trimmed.is_empty() {
            return Vec::new();
        }

        // Parse comma-separated indices
        let mut declarations = Vec::new();
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>()
                && idx < ctx.attacker_options.len()
            {
                let opt = &ctx.attacker_options[idx];
                // Pick first valid target (usually opponent)
                if let Some(target) = opt.valid_targets.first() {
                    declarations.push(crate::decisions::spec::AttackerDeclaration {
                        creature: opt.creature,
                        target: target.clone(),
                    });
                }
            }
        }
        declarations
    }

    fn decide_blockers(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision blockers[{}]: {} attacker options | input '{}'",
                self.index.saturating_sub(1),
                ctx.blocker_options.len(),
                input
            );
        }

        if trimmed.is_empty() {
            return Vec::new();
        }

        // Parse "blocker_idx:attacker_idx,..." format
        let mut declarations = Vec::new();
        for part in trimmed.split(',') {
            if let Some((b_str, a_str)) = part.split_once(':')
                && let (Ok(b_idx), Ok(a_idx)) =
                    (b_str.trim().parse::<usize>(), a_str.trim().parse::<usize>())
                && a_idx < ctx.blocker_options.len()
            {
                let opt = &ctx.blocker_options[a_idx];
                if b_idx < opt.valid_blockers.len() {
                    declarations.push(crate::decisions::spec::BlockerDeclaration {
                        blocker: opt.valid_blockers[b_idx].0,
                        blocking: opt.attacker,
                    });
                }
            }
        }
        declarations
    }

    fn decide_distribute(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(crate::game_state::Target, u32)> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision distribute[{}]: {} (total={}, {} targets) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.total,
                ctx.targets.len(),
                input
            );
        }

        if trimmed.is_empty() || ctx.targets.is_empty() {
            // Default: put all on first target
            if let Some(first) = ctx.targets.first() {
                return vec![(first.target, ctx.total)];
            }
            return Vec::new();
        }

        // Parse "amount:target_idx,amount:target_idx,..." format
        let mut distribution = Vec::new();
        let mut remaining = ctx.total;

        for part in trimmed.split(',') {
            if remaining == 0 {
                break;
            }
            if let Some((amt_str, idx_str)) = part.split_once(':')
                && let (Ok(amount), Ok(idx)) = (
                    amt_str.trim().parse::<u32>(),
                    idx_str.trim().parse::<usize>(),
                )
                && idx < ctx.targets.len()
            {
                let to_distribute = amount.min(remaining);
                if to_distribute >= ctx.min_per_target {
                    distribution.push((ctx.targets[idx].target, to_distribute));
                    remaining -= to_distribute;
                }
            }
        }

        // If nothing was distributed, put all on first target
        if distribution.is_empty() && !ctx.targets.is_empty() {
            distribution.push((ctx.targets[0].target, ctx.total));
        }

        distribution
    }

    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision colors[{}]: count={}, same_color={} | input '{}'",
                self.index.saturating_sub(1),
                ctx.count,
                ctx.same_color,
                input
            );
        }

        use crate::color::Color;
        let mut colors = Vec::new();

        for c in trimmed.to_uppercase().chars() {
            match c {
                'W' => colors.push(Color::White),
                'U' => colors.push(Color::Blue),
                'B' => colors.push(Color::Black),
                'R' => colors.push(Color::Red),
                'G' => colors.push(Color::Green),
                ' ' => continue,
                _ => {} // Ignore invalid characters
            }
        }

        // Determine default color
        let default_color = ctx
            .available_colors
            .as_ref()
            .and_then(|colors| colors.first().copied())
            .unwrap_or(Color::Green);

        // Pad with default if not enough colors provided
        while colors.len() < ctx.count as usize {
            colors.push(default_color);
        }

        // Truncate if too many
        colors.truncate(ctx.count as usize);
        colors
    }

    fn decide_counters(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision counters[{}]: max_total={}, {} types | input '{}'",
                self.index.saturating_sub(1),
                ctx.max_total,
                ctx.available_counters.len(),
                input
            );
        }

        if trimmed.is_empty() {
            return Vec::new();
        }

        // Parse "count:type_idx,count:type_idx,..." format
        let mut selections = Vec::new();
        let mut remaining = ctx.max_total;

        for part in trimmed.split(',') {
            if remaining == 0 {
                break;
            }
            if let Some((count_str, idx_str)) = part.split_once(':')
                && let (Ok(count), Ok(idx)) = (
                    count_str.trim().parse::<u32>(),
                    idx_str.trim().parse::<usize>(),
                )
                && idx < ctx.available_counters.len()
            {
                let (counter_type, available) = ctx.available_counters[idx];
                let to_remove = count.min(available).min(remaining);
                if to_remove > 0 {
                    selections.push((counter_type, to_remove));
                    remaining -= to_remove;
                }
            }
        }

        selections
    }

    fn decide_partition(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision partition[{}]: {} ({} cards) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.cards.len(),
                input
            );
        }

        if trimmed.is_empty() {
            return Vec::new(); // Keep all in primary destination
        }

        let cards: Vec<ObjectId> = ctx.cards.iter().map(|(id, _)| *id).collect();

        // Parse comma-separated indices for secondary destination
        let mut to_secondary = Vec::new();
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>()
                && idx < cards.len()
                && !to_secondary.contains(&cards[idx])
            {
                to_secondary.push(cards[idx]);
            }
        }

        to_secondary
    }

    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision proliferate[{}]: {} permanents, {} players | input '{}'",
                self.index.saturating_sub(1),
                ctx.eligible_permanents.len(),
                ctx.eligible_players.len(),
                input
            );
        }

        if trimmed.is_empty() {
            // Default: select all
            return crate::decisions::specs::ProliferateResponse {
                permanents: ctx.eligible_permanents.iter().map(|(id, _)| *id).collect(),
                players: ctx.eligible_players.iter().map(|(id, _)| *id).collect(),
            };
        }

        // Parse "p:idx,o:idx,..." format where p=permanent, o=player
        let mut permanents = Vec::new();
        let mut players = Vec::new();

        for part in trimmed.split(',') {
            if let Some((kind, idx_str)) = part.split_once(':')
                && let Ok(idx) = idx_str.trim().parse::<usize>()
            {
                match kind.trim().to_lowercase().as_str() {
                    "p" | "perm" | "permanent" => {
                        if idx < ctx.eligible_permanents.len() {
                            permanents.push(ctx.eligible_permanents[idx].0);
                        }
                    }
                    "o" | "player" => {
                        if idx < ctx.eligible_players.len() {
                            players.push(ctx.eligible_players[idx].0);
                        }
                    }
                    _ => {}
                }
            }
        }

        crate::decisions::specs::ProliferateResponse {
            permanents,
            players,
        }
    }

    fn decide_priority(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision priority[{}]: {} actions | input '{}'",
                self.index.saturating_sub(1),
                ctx.actions.len(),
                input
            );
        }

        // Empty input means pass priority
        if trimmed.is_empty() {
            return LegalAction::PassPriority;
        }

        // Check for commander action (C, c, C0, c0, C1, c1, etc.)
        let lower = trimmed.to_lowercase();
        let commander_indices = commander_action_indices(&ctx.actions);
        if lower == "c" && commander_indices.len() == 1 {
            return ctx.actions[commander_indices[0]].clone();
        }
        if lower.starts_with('c')
            && let Ok(idx) = lower[1..].parse::<usize>()
            && idx < commander_indices.len()
        {
            return ctx.actions[commander_indices[idx]].clone();
        }

        // Parse as index
        if let Ok(idx) = trimmed.parse::<usize>()
            && idx < ctx.actions.len()
        {
            return ctx.actions[idx].clone();
        }

        // Fallback to pass
        LegalAction::PassPriority
    }

    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision targets[{}]: {} requirements | input '{}'",
                self.index.saturating_sub(1),
                ctx.requirements.len(),
                input
            );
            for (i, req) in ctx.requirements.iter().enumerate() {
                eprintln!(
                    "  target requirement[{}]: {} legal targets",
                    i,
                    req.legal_targets.len()
                );
            }
        }

        // Each requirement gets a target selection
        // Input format: "target_idx" for single target, or "idx1,idx2,..." for multiple requirements
        let mut targets = Vec::new();
        let indices: Vec<usize> = trimmed
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .collect();

        for (req_idx, req) in ctx.requirements.iter().enumerate() {
            if req.min_targets == 0 && (indices.is_empty() || req_idx >= indices.len()) {
                // Optional target with no input, skip
                continue;
            }

            // Get the target index for this requirement
            let target_idx = indices.get(req_idx).copied().unwrap_or(0);

            // Select the target at that index
            if target_idx < req.legal_targets.len() {
                targets.push(req.legal_targets[target_idx]);
            } else if !req.legal_targets.is_empty() && req.min_targets > 0 {
                // Fallback to first legal target if required
                targets.push(req.legal_targets[0]);
            }
        }

        targets
    }
}

// ============================================================================
// CLI Decision Maker
// ============================================================================

/// A decision maker that prompts the user via CLI.
pub struct CliDecisionMaker;

impl DecisionMaker for CliDecisionMaker {
    fn on_auto_pass(&mut self, game: &GameState, player: PlayerId) {
        let phase = format_phase(&game.turn.phase, &game.turn.step);
        println!("({} auto-passes: {})", player_name(game, player), phase);
    }

    fn on_action_cancelled(&mut self, _game: &GameState, reason: &str) {
        println!("\n*** Action cancelled: {} ***", reason);
        println!("(Game state restored to before the action started)\n");
    }

    fn decide_priority(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        display_game_state(game);
        println!("\n--- {} has priority ---", player_name(game, ctx.player));
        prompt_priority_action(game, &ctx.actions)
    }

    fn decide_boolean(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        let source_info = if let Some(name) = &ctx.source_name {
            format!(" ({})", name)
        } else if let Some(source_id) = ctx.source {
            game.object(source_id)
                .map(|o| format!(" ({})", o.name))
                .unwrap_or_default()
        } else {
            String::new()
        };
        println!(
            "\n--- {} chooses{}: {} ---",
            player_name(game, ctx.player),
            source_info,
            ctx.description
        );
        prompt_boolean_choice()
    }

    fn decide_number(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        let source_info = ctx
            .source
            .and_then(|id| game.object(id))
            .map(|o| format!(" for {}", o.name))
            .unwrap_or_default();
        println!(
            "\n--- {} chooses a number{} ---",
            player_name(game, ctx.player),
            source_info
        );
        println!("{}", ctx.description);
        prompt_number_choice(ctx.min, ctx.max)
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        println!(
            "\n--- {} selects objects ---",
            player_name(game, ctx.player)
        );
        println!("{}", ctx.description);
        prompt_select_objects(
            game,
            &ctx.candidates,
            ctx.min,
            ctx.max,
            ctx.allow_partial_completion,
        )
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        println!(
            "\n--- {} chooses option(s) ---",
            player_name(game, ctx.player)
        );
        println!("{}", ctx.description);
        prompt_select_options(&ctx.options, ctx.min, ctx.max)
    }

    fn view_cards(
        &mut self,
        game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        let viewer_name = player_name(game, viewer);
        let subject_name = player_name(game, ctx.subject);
        let zone_label = ctx.zone.to_string();

        println!(
            "\n--- {} looks at {}'s {} ---",
            viewer_name, subject_name, zone_label
        );
        println!("{}", ctx.description);

        if cards.is_empty() {
            println!("(no cards)");
            return;
        }

        for (idx, card_id) in cards.iter().enumerate() {
            let name = game
                .object(*card_id)
                .map(|obj| obj.name.clone())
                .unwrap_or_else(|| format!("Unknown ({})", card_id.0));
            println!("{}. {}", idx + 1, name);
        }
    }

    fn decide_order(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        println!("\n--- {} orders items ---", player_name(game, ctx.player));
        println!("{}", ctx.description);
        // For simplicity, use the default order (items as given)
        ctx.items.iter().map(|(id, _)| *id).collect()
    }

    fn decide_attackers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        display_game_state(game);
        println!(
            "\n--- {} declares attackers ---",
            player_name(game, ctx.player)
        );
        prompt_declare_attackers(game, ctx)
    }

    fn decide_blockers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        display_game_state(game);
        println!(
            "\n--- {} declares blockers ---",
            player_name(game, ctx.player)
        );
        prompt_declare_blockers(game, ctx)
    }

    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        println!(
            "\n--- {} distributes {} ---",
            player_name(game, ctx.player),
            ctx.total
        );
        println!("{}", ctx.description);
        prompt_distribute(game, &ctx.targets, ctx.total, ctx.min_per_target)
    }

    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        println!(
            "\n--- {} chooses {} mana color(s){} ---",
            player_name(game, ctx.player),
            ctx.count,
            if ctx.same_color {
                " (must be same)"
            } else {
                ""
            }
        );
        prompt_choose_colors(ctx.count, ctx.same_color)
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(CounterType, u32)> {
        let target_name = game
            .object(ctx.target)
            .map(|o| o.name.as_str())
            .unwrap_or("permanent");
        println!(
            "\n--- {} chooses counters to remove from {} (up to {} total) ---",
            player_name(game, ctx.player),
            target_name,
            ctx.max_total
        );
        prompt_choose_counters(&ctx.available_counters, ctx.max_total)
    }

    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        println!(
            "\n--- {} partitions {} card(s) ---",
            player_name(game, ctx.player),
            ctx.cards.len()
        );
        println!("{}", ctx.description);
        prompt_partition(game, &ctx.cards, &ctx.primary_label, &ctx.secondary_label)
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        println!(
            "\n--- {} chooses proliferate targets ---",
            player_name(game, ctx.player)
        );
        prompt_proliferate(game, &ctx.eligible_permanents, &ctx.eligible_players)
    }

    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        let source_name = game
            .object(ctx.source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell/ability".to_string());
        println!(
            "\n--- {} chooses targets for {} ---",
            player_name(game, ctx.player),
            source_name
        );
        prompt_choose_targets(game, &ctx.requirements)
    }
}

fn player_name(game: &GameState, player: PlayerId) -> &str {
    game.player(player)
        .map(|p| p.name.as_str())
        .unwrap_or("Unknown")
}

fn display_game_state(game: &GameState) {
    println!(
        "\n=== Turn {}: {} ({}) ===",
        game.turn.turn_number,
        player_name(game, game.turn.active_player),
        format_phase(&game.turn.phase, &game.turn.step)
    );

    // Display players side by side
    for player in &game.players {
        let status = if player.has_lost {
            " [LOST]"
        } else if player.has_won {
            " [WON]"
        } else {
            ""
        };
        let mana = format_mana_pool(&player.mana_pool);
        print!(
            "[{}]{} Life:{} Mana:{} Hand:{} Lib:{} | ",
            player.name,
            status,
            player.life,
            mana,
            player.hand.len(),
            player.library.len()
        );
    }
    println!();

    // Show active player's hand compactly
    let active = game.turn.active_player;
    if let Some(player) = game.player(active) {
        let hand: Vec<String> = player
            .hand
            .iter()
            .filter_map(|&id| {
                game.object(id)
                    .map(|o| format!("{}({})", o.name, format_mana_cost(o)))
            })
            .collect();
        if !hand.is_empty() {
            println!("Hand: {}", hand.join(", "));
        }
    }

    // Show graveyards compactly (if non-empty)
    for player in &game.players {
        if !player.graveyard.is_empty() {
            let gy: Vec<String> = player
                .graveyard
                .iter()
                .filter_map(|&id| game.object(id).map(|o| o.name.clone()))
                .collect();
            println!(
                "{}'s Graveyard ({}): {}",
                player.name,
                gy.len(),
                gy.join(", ")
            );
        }
    }

    // Display battlefield compactly
    if !game.battlefield.is_empty() {
        let perms: Vec<String> = game
            .battlefield
            .iter()
            .filter_map(|&id| {
                game.object(id).map(|obj| {
                    let tapped = if game.is_tapped(id) { "[T]" } else { "" };
                    let pt = if game.current_is_creature(id) {
                        // Use calculated power/toughness (includes +1/+1 counters, anthems, etc.)
                        let power = game.current_power(id).unwrap_or(0);
                        let toughness = game.current_toughness(id).unwrap_or(0);
                        format!(" {}/{}", power, toughness)
                    } else {
                        String::new()
                    };
                    format!(
                        "{}{}{}({})",
                        obj.name,
                        pt,
                        tapped,
                        player_name(game, obj.controller)
                            .chars()
                            .next()
                            .unwrap_or('?')
                    )
                })
            })
            .collect();
        println!("Field: {}", perms.join(", "));
    }

    // Display stack compactly
    if !game.stack.is_empty() {
        let stack: Vec<String> = game
            .stack
            .iter()
            .rev()
            .map(|entry| {
                // Use source_name if available (for abilities), otherwise look up the object
                if entry.is_ability {
                    if let Some(name) = &entry.source_name {
                        format!("{} (ability)", name)
                    } else if let Some(obj) = game.object(entry.object_id) {
                        format!("{} (ability)", obj.name)
                    } else {
                        "[Triggered Ability]".to_string()
                    }
                } else if let Some(obj) = game.object(entry.object_id) {
                    obj.name.clone()
                } else {
                    "[Unknown]".to_string()
                }
            })
            .collect();
        println!("Stack: {}", stack.join(" -> "));
    }
}

fn format_phase(phase: &Phase, step: &Option<Step>) -> String {
    let phase_str = match phase {
        Phase::Beginning => "Beginning",
        Phase::FirstMain => "Precombat Main",
        Phase::Combat => "Combat",
        Phase::NextMain => "Postcombat Main",
        Phase::Ending => "Ending",
    };

    if let Some(step) = step {
        let step_str = match step {
            Step::Untap => "Untap",
            Step::Upkeep => "Upkeep",
            Step::Draw => "Draw",
            Step::BeginCombat => "Begin Combat",
            Step::DeclareAttackers => "Declare Attackers",
            Step::DeclareBlockers => "Declare Blockers",
            Step::CombatDamage => "Combat Damage",
            Step::EndCombat => "End Combat",
            Step::End => "End",
            Step::Cleanup => "Cleanup",
        };
        format!("{} - {}", phase_str, step_str)
    } else {
        phase_str.to_string()
    }
}

fn format_mana_pool(pool: &crate::ManaPool) -> String {
    let mut parts = Vec::new();
    if pool.white > 0 {
        parts.push(format!("{}W", pool.white));
    }
    if pool.blue > 0 {
        parts.push(format!("{}U", pool.blue));
    }
    if pool.black > 0 {
        parts.push(format!("{}B", pool.black));
    }
    if pool.red > 0 {
        parts.push(format!("{}R", pool.red));
    }
    if pool.green > 0 {
        parts.push(format!("{}G", pool.green));
    }
    if pool.colorless > 0 {
        parts.push(format!("{}C", pool.colorless));
    }
    if parts.is_empty() {
        "empty".to_string()
    } else {
        parts.join(" ")
    }
}

fn format_mana_cost(obj: &crate::Object) -> String {
    if let Some(ref cost) = obj.mana_cost {
        let mut parts = Vec::new();
        for pip in cost.pips() {
            if pip.len() == 1 {
                parts.push(format_symbol(&pip[0]));
            } else {
                // Hybrid - show alternatives
                let alts: Vec<String> = pip.iter().map(format_symbol).collect();
                parts.push(format!("({})", alts.join("/")));
            }
        }
        if parts.is_empty() {
            "0".to_string()
        } else {
            parts.join("")
        }
    } else {
        "0".to_string()
    }
}

fn format_symbol(symbol: &ManaSymbol) -> String {
    match symbol {
        ManaSymbol::White => "W".to_string(),
        ManaSymbol::Blue => "U".to_string(),
        ManaSymbol::Black => "B".to_string(),
        ManaSymbol::Red => "R".to_string(),
        ManaSymbol::Green => "G".to_string(),
        ManaSymbol::Colorless => "C".to_string(),
        ManaSymbol::Generic(n) => n.to_string(),
        ManaSymbol::Snow => "S".to_string(),
        ManaSymbol::Life(n) => format!("P{}", n),
        ManaSymbol::X => "X".to_string(),
    }
}

fn format_mana_cost_from_cost(cost: &crate::ManaCost) -> String {
    let mut parts = Vec::new();
    for pip in cost.pips() {
        if pip.len() == 1 {
            parts.push(format_symbol(&pip[0]));
        } else {
            let alts: Vec<String> = pip.iter().map(format_symbol).collect();
            parts.push(format!("({})", alts.join("/")));
        }
    }
    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join("")
    }
}

fn format_non_mana_costs(costs: &[crate::costs::Cost]) -> String {
    if costs.is_empty() {
        return "Free".to_string();
    }
    costs
        .iter()
        .map(|cost| cost.display())
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

/// New version of prompt_priority_action that returns LegalAction directly
/// (used by the new decide_priority method).
fn prompt_priority_action(game: &GameState, actions: &[LegalAction]) -> LegalAction {
    let commander_indices = commander_action_indices(actions);

    // Format actions compactly
    let action_strs: Vec<String> = actions
        .iter()
        .enumerate()
        .map(|(i, a)| format!("{}:{}", i, format_action_short(game, a)))
        .collect();
    println!("Actions: {}", action_strs.join(" | "));

    // Display commander actions separately with 'C' prefix
    if !commander_indices.is_empty() {
        let commander_strs: Vec<String> = commander_indices
            .iter()
            .enumerate()
            .map(|(i, action_index)| {
                let action = &actions[*action_index];
                if commander_indices.len() == 1 {
                    format!("C:{}", format_action_short(game, action))
                } else {
                    format!("C{}:{}", i, format_action_short(game, action))
                }
            })
            .collect();
        println!("Commander: {}", commander_strs.join(" | "));
    }

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        // Empty input = pass priority if available
        if trimmed.is_empty()
            && let Some(pass_action) = actions
                .iter()
                .find(|a| matches!(a, LegalAction::PassPriority))
        {
            return pass_action.clone();
        }

        // Check for commander action (C, c, C0, c0, C1, c1, etc.)
        let lower = trimmed.to_lowercase();
        if lower == "c" && commander_indices.len() == 1 {
            return actions[commander_indices[0]].clone();
        }
        if lower.starts_with('c')
            && let Ok(idx) = lower[1..].parse::<usize>()
            && idx < commander_indices.len()
        {
            return actions[commander_indices[idx]].clone();
        }

        if let Ok(idx) = trimmed.parse::<usize>()
            && idx < actions.len()
        {
            return actions[idx].clone();
        }
        println!("Invalid (0-{})", actions.len() - 1);
    }
}

fn zone_label(zone: Zone) -> &'static str {
    match zone {
        Zone::Battlefield => "battlefield",
        Zone::Hand => "hand",
        Zone::Library => "library",
        Zone::Graveyard => "graveyard",
        Zone::Exile => "exile",
        Zone::Stack => "stack",
        Zone::Command => "command zone",
    }
}

pub(crate) fn format_action_short(game: &GameState, action: &LegalAction) -> String {
    match action {
        LegalAction::PassPriority => "Pass".to_string(),
        LegalAction::KeepOpeningHand => "Keep hand".to_string(),
        LegalAction::TakeMulligan => "Mulligan".to_string(),
        LegalAction::SerumPowderMulligan { card_id } => {
            let name = game
                .object(*card_id)
                .map(|o| o.name.as_str())
                .unwrap_or("Serum Powder");
            format!("Use {}", name)
        }
        LegalAction::ContinuePregame => "Continue".to_string(),
        LegalAction::BeginGame => "Begin game".to_string(),
        LegalAction::UsePregameAction { card_id, .. } => {
            let name = game
                .object(*card_id)
                .map(|o| o.name.as_str())
                .unwrap_or("pregame card");
            format!("Begin with {}", name)
        }
        LegalAction::PlayLand { land_id } => {
            let name = game
                .object(*land_id)
                .map(|o| o.name.as_str())
                .unwrap_or("?");
            format!("Play {}", name)
        }
        LegalAction::CastSpell {
            spell_id,
            casting_method,
            ..
        } => {
            if let Some(obj) = game.object(*spell_id) {
                match casting_method {
                    crate::alternative_cast::CastingMethod::Normal => {
                        format!("{} ({})", obj.name, format_mana_cost(obj))
                    }
                    crate::alternative_cast::CastingMethod::FaceDown => {
                        format!("{} [Face down] ({})", obj.name, "{3}")
                    }
                    crate::alternative_cast::CastingMethod::SplitOtherHalf => {
                        if let Some(other_def) = game.linked_face_definition_by_name_or_id(
                            obj.other_face_name.as_deref(),
                            obj.other_face,
                        ) {
                            let cost = other_def
                                .card
                                .mana_cost
                                .as_ref()
                                .map(format_mana_cost_from_cost)
                                .unwrap_or_else(|| "0".to_string());
                            format!("{} ({})", other_def.card.name, cost)
                        } else {
                            format!("{} [other half]", obj.name)
                        }
                    }
                    crate::alternative_cast::CastingMethod::Fuse => {
                        let fused_cost = spell_mana_cost_for_cast(
                            game,
                            game.turn.priority_player.unwrap_or(obj.owner),
                            obj,
                            casting_method,
                            Zone::Hand,
                        )
                        .as_ref()
                        .map(format_mana_cost_from_cost)
                        .unwrap_or_else(|| format_mana_cost(obj));
                        if let Some(other_def) = game.linked_face_definition_by_name_or_id(
                            obj.other_face_name.as_deref(),
                            obj.other_face,
                        ) {
                            format!(
                                "{} // {} [Fuse] ({})",
                                obj.name, other_def.card.name, fused_cost
                            )
                        } else {
                            format!("{} [Fuse] ({})", obj.name, fused_cost)
                        }
                    }
                    crate::alternative_cast::CastingMethod::Alternative(idx) => {
                        // Get the alternative cost description
                        if let Some(alt_method) = obj.alternative_casts.get(*idx) {
                            let costs = alt_method.non_mana_costs();
                            let cost_desc = if !costs.is_empty() {
                                let effects_desc = format_non_mana_costs(&costs);
                                if let Some(mana) = alt_method.mana_cost() {
                                    format!(
                                        "{}, {}",
                                        format_mana_cost_from_cost(mana),
                                        effects_desc
                                    )
                                } else {
                                    effects_desc
                                }
                            } else if let Some(mana_cost) = alt_method.mana_cost() {
                                // For flashback/escape/etc., show the mana cost
                                format_mana_cost_from_cost(mana_cost)
                            } else {
                                format_mana_cost(obj)
                            };
                            format!("{} [{}] ({})", obj.name, alt_method.name(), cost_desc)
                        } else {
                            format!("{} [Alt] ({})", obj.name, format_mana_cost(obj))
                        }
                    }
                    crate::alternative_cast::CastingMethod::GrantedEscape { .. } => {
                        format!("{} [Escape] ({})", obj.name, format_mana_cost(obj))
                    }
                    crate::alternative_cast::CastingMethod::GrantedFlashback => {
                        format!("{} [Flashback] ({})", obj.name, format_mana_cost(obj))
                    }
                    crate::alternative_cast::CastingMethod::PlayFrom {
                        zone,
                        use_alternative: None,
                        ..
                    } => {
                        format!(
                            "{} [from {}] ({})",
                            obj.name,
                            zone_label(*zone),
                            format_mana_cost(obj)
                        )
                    }
                    crate::alternative_cast::CastingMethod::PlayFrom {
                        zone,
                        use_alternative: Some(idx),
                        ..
                    } => {
                        let acting_player = game.turn.priority_player.unwrap_or(obj.owner);
                        if let Some(alt_method) = resolve_play_from_alternative_method(
                            game,
                            acting_player,
                            obj,
                            *zone,
                            *idx,
                        ) {
                            let costs = alt_method.non_mana_costs();
                            let cost_desc = if !costs.is_empty() {
                                let effects_desc = format_non_mana_costs(&costs);
                                if let Some(mana) = alt_method.mana_cost() {
                                    format!(
                                        "{}, {}",
                                        format_mana_cost_from_cost(mana),
                                        effects_desc
                                    )
                                } else {
                                    effects_desc
                                }
                            } else if let Some(mana_cost) = alt_method.mana_cost() {
                                format_mana_cost_from_cost(mana_cost)
                            } else {
                                format_mana_cost(obj)
                            };
                            format!(
                                "{} [from {}, {}] ({})",
                                obj.name,
                                zone_label(*zone),
                                alt_method.name(),
                                cost_desc
                            )
                        } else {
                            format!(
                                "{} [from {}, Alt] ({})",
                                obj.name,
                                zone_label(*zone),
                                format_mana_cost(obj)
                            )
                        }
                    }
                }
            } else {
                "Cast".to_string()
            }
        }
        LegalAction::ActivateAbility { source, .. } => {
            let name = game.object(*source).map(|o| o.name.as_str()).unwrap_or("?");
            format!("Activate {}", name)
        }
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => {
            let name = game.object(*source).map(|o| o.name.as_str()).unwrap_or("?");

            // Check if this ability requires tapping
            if let Some(ability) = game.current_ability(*source, *ability_index) {
                if let crate::AbilityKind::Activated(mana_ability) = &ability.kind
                    && mana_ability.is_mana_ability()
                {
                    if mana_ability.has_tap_cost() {
                        format!("Tap {}", name)
                    } else if let Some(text) = &ability.text {
                        format!("{}: {}", name, text)
                    } else {
                        format!("Activate {}", name)
                    }
                } else {
                    format!("Activate {}", name)
                }
            } else {
                format!("Tap {}", name)
            }
        }
        LegalAction::TurnFaceUp {
            creature_id,
            method,
        } => {
            let name = game
                .object(*creature_id)
                .map(|o| o.name.as_str())
                .unwrap_or("?");
            format!("Turn face up {} for its {}", name, method.description())
        }
        LegalAction::SpecialAction(special) => match special {
            crate::special_actions::SpecialAction::PlayLand { .. } => "Play land".to_string(),
            crate::special_actions::SpecialAction::TurnFaceUp { method, .. } => {
                format!("Turn face up for its {}", method.description())
            }
            crate::special_actions::SpecialAction::Suspend { .. } => "Suspend".to_string(),
            crate::special_actions::SpecialAction::Foretell { .. } => "Foretell".to_string(),
            crate::special_actions::SpecialAction::Plot { .. } => "Plot".to_string(),
            crate::special_actions::SpecialAction::ActivateManaAbility { .. } => {
                "Activate mana ability".to_string()
            }
        },
    }
}

fn prompt_declare_attackers(
    _game: &GameState,
    ctx: &crate::decisions::context::AttackersContext,
) -> Vec<crate::decisions::spec::AttackerDeclaration> {
    if ctx.attacker_options.is_empty() {
        println!("No creatures can attack.");
        return Vec::new();
    }

    println!("\nCreatures that can attack:");
    for (i, opt) in ctx.attacker_options.iter().enumerate() {
        let must = if opt.must_attack {
            " [MUST ATTACK]"
        } else {
            ""
        };
        println!("  {}: {}{}", i, opt.creature_name, must);
    }

    println!("\nEnter attacking creatures (comma-separated indices, or empty for none):");
    print!("> ");
    io::stdout().flush().unwrap();

    let input = read_input().unwrap_or_default();
    let input = input.trim();

    if input.is_empty() {
        return Vec::new();
    }

    let mut declarations = Vec::new();
    for part in input.split(',') {
        if let Ok(idx) = part.trim().parse::<usize>()
            && idx < ctx.attacker_options.len()
        {
            // Default to attacking the first opponent
            if let Some(target) = ctx.attacker_options[idx].valid_targets.first() {
                declarations.push(crate::decisions::spec::AttackerDeclaration {
                    creature: ctx.attacker_options[idx].creature,
                    target: target.clone(),
                });
            }
        }
    }

    declarations
}

fn prompt_declare_blockers(
    _game: &GameState,
    ctx: &crate::decisions::context::BlockersContext,
) -> Vec<crate::decisions::spec::BlockerDeclaration> {
    if ctx.blocker_options.is_empty() {
        println!("No attackers to block.");
        return Vec::new();
    }

    // Build a list of all valid blockers (creatures that can block at least one attacker)
    let mut all_valid_blockers: Vec<crate::ObjectId> = Vec::new();
    for opt in &ctx.blocker_options {
        for &(blocker_id, _) in &opt.valid_blockers {
            if !all_valid_blockers.contains(&blocker_id) {
                all_valid_blockers.push(blocker_id);
            }
        }
    }

    println!("\nAttackers:");
    for (i, opt) in ctx.blocker_options.iter().enumerate() {
        println!("  Attacker {}: {}", i, opt.attacker_name);
    }

    println!("\nAvailable blockers:");
    for (i, blocker_id) in all_valid_blockers.iter().enumerate() {
        let blocker_name = ctx
            .blocker_options
            .iter()
            .flat_map(|opt| opt.valid_blockers.iter())
            .find(|(id, _)| id == blocker_id)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| format!("Object {}", blocker_id.0));
        // Show which attackers this creature can block
        let can_block: Vec<usize> = ctx
            .blocker_options
            .iter()
            .enumerate()
            .filter(|(_, opt)| opt.valid_blockers.iter().any(|(id, _)| id == blocker_id))
            .map(|(i, _)| i)
            .collect();
        let can_block_text = if can_block.is_empty() {
            "none".to_string()
        } else {
            can_block
                .iter()
                .map(|idx| idx.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!(
            "  Blocker {}: {} (can block attackers: {})",
            i, blocker_name, can_block_text
        );
    }

    println!(
        "\nEnter blocks as 'blocker_idx:attacker_idx' pairs (comma-separated, or empty for none):"
    );
    println!(
        "Example: '0:0,1:0' means blocker 0 blocks attacker 0, blocker 1 also blocks attacker 0"
    );
    print!("> ");
    io::stdout().flush().unwrap();

    let input = read_input().unwrap_or_default();
    let input = input.trim();

    if input.is_empty() {
        return Vec::new();
    }

    let mut declarations = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if let Some((blocker_str, attacker_str)) = part.split_once(':') {
            if let (Ok(blocker_idx), Ok(attacker_idx)) = (
                blocker_str.trim().parse::<usize>(),
                attacker_str.trim().parse::<usize>(),
            ) {
                // Validate indices
                if blocker_idx < all_valid_blockers.len()
                    && attacker_idx < ctx.blocker_options.len()
                {
                    let blocker_id = all_valid_blockers[blocker_idx];
                    let attacker_id = ctx.blocker_options[attacker_idx].attacker;

                    // Check if this blocker can actually block this attacker
                    if ctx.blocker_options[attacker_idx]
                        .valid_blockers
                        .iter()
                        .any(|(id, _)| *id == blocker_id)
                    {
                        declarations.push(crate::decisions::spec::BlockerDeclaration {
                            blocker: blocker_id,
                            blocking: attacker_id,
                        });
                    } else {
                        println!(
                            "Warning: Blocker {} cannot block attacker {}, skipping",
                            blocker_idx, attacker_idx
                        );
                    }
                } else {
                    println!("Warning: Invalid indices {}, skipping", part);
                }
            } else {
                println!("Warning: Could not parse '{}', skipping", part);
            }
        } else {
            println!(
                "Warning: Invalid format '{}', expected 'blocker:attacker'",
                part
            );
        }
    }

    declarations
}

// ============================================================================
// New typed prompt functions for primitive-specific DecisionMaker methods
// ============================================================================

/// Prompt for a boolean (yes/no) choice, returning bool directly.
fn prompt_boolean_choice() -> bool {
    loop {
        print!("Choose (y/n): ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" | "1" => return true,
            "n" | "no" | "0" => return false,
            _ => println!("Please enter 'y' or 'n'."),
        }
    }
}

/// Prompt for a number in a range, returning u32 directly.
fn prompt_number_choice(min: u32, max: u32) -> u32 {
    println!("Choose a number from {} to {}", min, max);
    loop {
        print!("Enter number: ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();

        if let Ok(n) = input.trim().parse::<u32>()
            && n >= min
            && n <= max
        {
            return n;
        }
        println!("Please enter a number between {} and {}.", min, max);
    }
}

/// Prompt for selecting objects from a list, returning Vec<ObjectId> directly.
fn prompt_select_objects(
    game: &GameState,
    candidates: &[crate::decisions::context::SelectableObject],
    min: usize,
    max: Option<usize>,
    allow_partial_completion: bool,
) -> Vec<ObjectId> {
    if candidates.is_empty() {
        return vec![];
    }

    println!("Selectable objects:");
    for (i, candidate) in candidates.iter().enumerate() {
        // Use the candidate's name if available, otherwise look it up
        let name = if candidate.name.is_empty() {
            game.object(candidate.id)
                .map(|o| o.name.as_str())
                .unwrap_or("?")
                .to_string()
        } else {
            candidate.name.clone()
        };
        let legal_marker = if candidate.legal { "" } else { " [ILLEGAL]" };
        println!("  {}: {}{}", i, name, legal_marker);
    }

    let max_display = max
        .map(|m| m.to_string())
        .unwrap_or_else(|| "any".to_string());
    if allow_partial_completion {
        println!(
            "Select up to {} objects (comma-separated indices, or empty for none):",
            max_display
        );
    } else {
        println!(
            "Select {} to {} objects (comma-separated indices, or empty for none):",
            min, max_display
        );
    }

    loop {
        print!("Selection: ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        // Handle empty input
        if trimmed.is_empty() {
            if min == 0 || allow_partial_completion {
                return vec![];
            }
            println!("Must select at least {} object(s).", min);
            continue;
        }

        // Parse comma-separated indices
        let mut selected = vec![];
        let mut valid = true;
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>() {
                if idx < candidates.len() {
                    selected.push(candidates[idx].id);
                } else {
                    println!("Invalid index: {}", idx);
                    valid = false;
                    break;
                }
            } else {
                println!("Invalid input: {}", part);
                valid = false;
                break;
            }
        }

        if !valid {
            continue;
        }

        // Validate count
        if !allow_partial_completion && selected.len() < min {
            println!("Must select at least {} object(s).", min);
            continue;
        }
        if let Some(m) = max
            && selected.len() > m
        {
            println!("Cannot select more than {} object(s).", m);
            continue;
        }

        return selected;
    }
}

/// Prompt for selecting options by index, returning Vec<usize> directly.
fn prompt_select_options(
    options: &[crate::decisions::context::SelectableOption],
    min: usize,
    max: usize,
) -> Vec<usize> {
    println!("Available options:");
    for opt in options {
        let legal_marker = if opt.legal { "" } else { " [ILLEGAL]" };
        println!("  {}: {}{}", opt.index, opt.description, legal_marker);
    }

    if min == max && min == 1 {
        println!("Select one option:");
    } else {
        println!(
            "Select {} to {} option(s) (comma-separated indices):",
            min, max
        );
    }

    loop {
        print!("Selection: ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        // Parse indices
        let mut selected = vec![];
        let mut valid = true;

        if trimmed.is_empty() {
            if min == 0 {
                return vec![];
            }
            println!("Must select at least {} option(s).", min);
            continue;
        }

        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>() {
                // Find option with this index
                if options.iter().any(|o| o.index == idx && o.legal) {
                    selected.push(idx);
                } else {
                    println!("Invalid or illegal option: {}", idx);
                    valid = false;
                    break;
                }
            } else {
                println!("Invalid input: {}", part);
                valid = false;
                break;
            }
        }

        if !valid {
            continue;
        }

        // Validate count
        if selected.len() < min {
            println!("Must select at least {} option(s).", min);
            continue;
        }
        if selected.len() > max {
            println!("Cannot select more than {} option(s).", max);
            continue;
        }

        return selected;
    }
}

/// Prompt for distributing an amount among targets, returning Vec<(Target, u32)> directly.
fn prompt_distribute(
    game: &GameState,
    targets: &[crate::decisions::context::DistributeTarget],
    total: u32,
    min_per_target: u32,
) -> Vec<(Target, u32)> {
    if targets.is_empty() {
        return vec![];
    }

    println!(
        "Distribute {} total (min {} per target):",
        total, min_per_target
    );
    for (i, target) in targets.iter().enumerate() {
        // Use the target's name if available, otherwise look it up
        let name = if !target.name.is_empty() {
            target.name.as_str()
        } else {
            match target.target {
                Target::Object(id) => game.object(id).map(|o| o.name.as_str()).unwrap_or("?"),
                Target::Player(pid) => game.player(pid).map(|p| p.name.as_str()).unwrap_or("?"),
            }
        };
        println!("  {}: {}", i, name);
    }

    // For simplicity, put all on the first target.
    // A full implementation would prompt for amounts per target
    if let Some(first) = targets.first() {
        vec![(first.target, total)]
    } else {
        vec![]
    }
}

/// Prompt for choosing colors, returning Vec<Color> directly.
fn prompt_choose_colors(count: u32, same_color: bool) -> Vec<crate::color::Color> {
    use crate::color::Color;

    println!("Choose {} color(s):", count);
    println!("  0: White");
    println!("  1: Blue");
    println!("  2: Black");
    println!("  3: Red");
    println!("  4: Green");

    let mut result = vec![];

    for i in 0..count {
        loop {
            print!("Color {}: ", i + 1);
            io::stdout().flush().unwrap();

            let input = read_input().unwrap_or_default();

            let normalized = input.trim().to_ascii_lowercase();
            let color = match normalized.as_str() {
                "0" => Some(Color::White),
                "1" => Some(Color::Blue),
                "2" => Some(Color::Black),
                "3" => Some(Color::Red),
                "4" => Some(Color::Green),
                _ => Color::from_mana_code_or_name(normalized.as_str()),
            };

            if let Some(c) = color {
                // Check same_color constraint
                if same_color && !result.is_empty() && result[0] != c {
                    println!("All colors must be the same.");
                    continue;
                }
                result.push(c);
                break;
            }
            println!("Invalid color. Please enter 0-4 or w/u/b/r/g.");
        }
    }

    result
}

/// Prompt for choosing counters to remove, returning Vec<(CounterType, u32)> directly.
fn prompt_choose_counters(
    available_counters: &[(CounterType, u32)],
    max_total: u32,
) -> Vec<(CounterType, u32)> {
    if available_counters.is_empty() {
        return vec![];
    }

    println!("Available counters (up to {} total to remove):", max_total);
    for (i, (counter_type, count)) in available_counters.iter().enumerate() {
        println!(
            "  {}: {} ({} available)",
            i,
            counter_type.description(),
            count
        );
    }

    println!("Enter index and amount pairs (e.g., '0:2,1:1' for 2 of type 0 and 1 of type 1):");
    println!("Or press enter to remove none.");

    loop {
        print!("Counters: ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return vec![];
        }

        let mut result = vec![];
        let mut total_removed = 0u32;
        let mut valid = true;

        for part in trimmed.split(',') {
            let parts: Vec<&str> = part.trim().split(':').collect();
            if parts.len() != 2 {
                println!("Invalid format. Use 'index:amount'.");
                valid = false;
                break;
            }

            let idx: usize = match parts[0].parse() {
                Ok(i) => i,
                Err(_) => {
                    println!("Invalid index: {}", parts[0]);
                    valid = false;
                    break;
                }
            };

            let amount: u32 = match parts[1].parse() {
                Ok(a) => a,
                Err(_) => {
                    println!("Invalid amount: {}", parts[1]);
                    valid = false;
                    break;
                }
            };

            if idx >= available_counters.len() {
                println!("Index {} out of range.", idx);
                valid = false;
                break;
            }

            if amount > available_counters[idx].1 {
                println!(
                    "Cannot remove {} counters, only {} available.",
                    amount, available_counters[idx].1
                );
                valid = false;
                break;
            }

            total_removed += amount;
            result.push((available_counters[idx].0, amount));
        }

        if !valid {
            continue;
        }

        if total_removed > max_total {
            println!("Total {} exceeds maximum {}.", total_removed, max_total);
            continue;
        }

        return result;
    }
}

/// Prompt for partitioning cards, returning Vec<ObjectId> for the secondary destination.
fn prompt_partition(
    game: &GameState,
    cards: &[(ObjectId, String)],
    primary_label: &str,
    secondary_label: &str,
) -> Vec<ObjectId> {
    if cards.is_empty() {
        return vec![];
    }

    println!("Cards to partition:");
    for (i, (id, name)) in cards.iter().enumerate() {
        let display_name = if name.is_empty() {
            game.object(*id)
                .map(|o| o.name.as_str())
                .unwrap_or("?")
                .to_string()
        } else {
            name.clone()
        };
        println!("  {}: {}", i, display_name);
    }

    println!(
        "Enter indices of cards to put on {} (comma-separated):",
        secondary_label
    );
    println!("Remaining cards go to {}.", primary_label);
    println!("Press enter to put all on {}.", primary_label);

    loop {
        print!("To {}: ", secondary_label);
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return vec![];
        }

        let mut result = vec![];
        let mut valid = true;

        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>() {
                if idx < cards.len() {
                    result.push(cards[idx].0);
                } else {
                    println!("Invalid index: {}", idx);
                    valid = false;
                    break;
                }
            } else {
                println!("Invalid input: {}", part);
                valid = false;
                break;
            }
        }

        if valid {
            return result;
        }
    }
}

/// Prompt for proliferate targets, returning ProliferateResponse directly.
fn prompt_proliferate(
    game: &GameState,
    eligible_permanents: &[(ObjectId, String)],
    eligible_players: &[(PlayerId, String)],
) -> crate::decisions::specs::ProliferateResponse {
    println!("Eligible permanents:");
    for (i, (id, name)) in eligible_permanents.iter().enumerate() {
        let display_name = if name.is_empty() {
            game.object(*id)
                .map(|o| o.name.as_str())
                .unwrap_or("?")
                .to_string()
        } else {
            name.clone()
        };
        println!("  p{}: {}", i, display_name);
    }

    println!("Eligible players:");
    for (i, (id, name)) in eligible_players.iter().enumerate() {
        let display_name = if name.is_empty() {
            game.player(*id)
                .map(|p| p.name.as_str())
                .unwrap_or("?")
                .to_string()
        } else {
            name.clone()
        };
        println!("  P{}: {}", i, display_name);
    }

    println!("Enter targets to proliferate (e.g., 'p0,p2,P1' for permanents 0,2 and player 1):");
    println!("Press enter to proliferate nothing.");

    loop {
        print!("Proliferate: ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return crate::decisions::specs::ProliferateResponse::default();
        }

        let mut permanents = vec![];
        let mut players = vec![];
        let mut valid = true;

        for part in trimmed.split(',') {
            let part = part.trim();
            if part.starts_with('p') && part.len() > 1 {
                // Permanent
                if let Ok(idx) = part[1..].parse::<usize>() {
                    if idx < eligible_permanents.len() {
                        permanents.push(eligible_permanents[idx].0);
                    } else {
                        println!("Invalid permanent index: {}", idx);
                        valid = false;
                        break;
                    }
                } else {
                    println!("Invalid permanent: {}", part);
                    valid = false;
                    break;
                }
            } else if part.starts_with('P') && part.len() > 1 {
                // Player
                if let Ok(idx) = part[1..].parse::<usize>() {
                    if idx < eligible_players.len() {
                        players.push(eligible_players[idx].0);
                    } else {
                        println!("Invalid player index: {}", idx);
                        valid = false;
                        break;
                    }
                } else {
                    println!("Invalid player: {}", part);
                    valid = false;
                    break;
                }
            } else {
                println!(
                    "Invalid target: {}. Use p# for permanents, P# for players.",
                    part
                );
                valid = false;
                break;
            }
        }

        if valid {
            return crate::decisions::specs::ProliferateResponse {
                permanents,
                players,
            };
        }
    }
}

/// Prompt for target selection, returning Vec<Target> directly.
fn prompt_choose_targets(
    game: &GameState,
    requirements: &[crate::decisions::context::TargetRequirementContext],
) -> Vec<Target> {
    let mut selected_targets = Vec::new();

    for req in requirements.iter() {
        if req.min_targets == 0 && req.legal_targets.is_empty() {
            // Optional targeting with no legal targets - skip
            continue;
        }

        println!("Select target for: {}", req.description);
        println!("Available targets:");

        for (i, target) in req.legal_targets.iter().enumerate() {
            let display = match target {
                Target::Object(id) => {
                    if let Some(obj) = game.object(*id) {
                        let controller_name = game
                            .player(obj.controller)
                            .map(|p| p.name.chars().next().unwrap_or('?'))
                            .unwrap_or('?');
                        if game.current_is_creature(*id) {
                            let power = game.current_power(*id).unwrap_or(0);
                            let toughness = game.current_toughness(*id).unwrap_or(0);
                            format!("{} {}/{} ({})", obj.name, power, toughness, controller_name)
                        } else {
                            format!("{} ({})", obj.name, controller_name)
                        }
                    } else {
                        format!("Object #{}", id.0)
                    }
                }
                Target::Player(id) => game
                    .player(*id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("Player {}", id.0)),
            };
            println!("  {}: {}", i, display);
        }

        let max_display = req
            .max_targets
            .map(|m| m.to_string())
            .unwrap_or_else(|| "any".to_string());
        println!(
            "Select {} to {} target(s) (enter index, or comma-separated for multiple):",
            req.min_targets, max_display
        );

        loop {
            print!("Selection: ");
            io::stdout().flush().unwrap();

            let input = read_input().unwrap_or_default();
            let trimmed = input.trim();

            // Handle empty input
            if trimmed.is_empty() {
                if req.min_targets == 0 {
                    // Optional targeting - skip this requirement
                    break;
                } else {
                    println!("Must select at least {} target(s).", req.min_targets);
                    continue;
                }
            }

            // Parse selected indices
            let mut valid = true;
            let mut req_targets = Vec::new();

            for part in trimmed.split(',') {
                if let Ok(idx) = part.trim().parse::<usize>() {
                    if idx < req.legal_targets.len() {
                        req_targets.push(req.legal_targets[idx]);
                    } else {
                        println!("Invalid index: {}", idx);
                        valid = false;
                        break;
                    }
                } else {
                    println!("Invalid input: {}", part);
                    valid = false;
                    break;
                }
            }

            if !valid {
                continue;
            }

            // Validate count
            if req_targets.len() < req.min_targets {
                println!(
                    "Must select at least {} target(s), got {}.",
                    req.min_targets,
                    req_targets.len()
                );
                continue;
            }
            if let Some(max) = req.max_targets
                && req_targets.len() > max
            {
                println!(
                    "Can select at most {} target(s), got {}.",
                    max,
                    req_targets.len()
                );
                continue;
            }

            selected_targets.extend(req_targets);
            break;
        }
    }

    selected_targets
}

/// Read a line using the global input manager.
/// In replay mode, exits the program when inputs are exhausted.
pub fn read_input() -> io::Result<String> {
    INPUT_MANAGER.with(|im| {
        let result = im.borrow_mut().read_line();
        if result.is_err() && im.borrow().is_replay_exhausted() {
            println!("\n=== Replay inputs exhausted, exiting ===");
            std::process::exit(0);
        }
        result
    })
}

// ============================================================================
// Input Manager for recording/replaying inputs
// ============================================================================

thread_local! {
    static INPUT_MANAGER: RefCell<InputManager> = RefCell::new(InputManager::new_interactive());
}

/// Manages input for the CLI - can read from stdin, record to file, or replay from file.
struct InputManager {
    mode: InputMode,
}

enum InputMode {
    /// Normal interactive mode - read from stdin
    Interactive,
    /// Record mode - read from stdin and write to file
    Record { file: BufWriter<File> },
    /// Replay mode - read from file
    Replay { lines: Vec<String>, index: usize },
}

impl InputManager {
    fn new_interactive() -> Self {
        Self {
            mode: InputMode::Interactive,
        }
    }

    fn new_record(path: &str) -> io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            mode: InputMode::Record {
                file: BufWriter::new(file),
            },
        })
    }

    fn new_replay(path: &str) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        // Keep empty lines (they're meaningful - e.g., "no attackers"), only skip comments
        let lines: Vec<String> = reader
            .lines()
            .map_while(Result::ok)
            .filter(|l| !l.trim().starts_with('#'))
            .collect();
        Ok(Self {
            mode: InputMode::Replay { lines, index: 0 },
        })
    }

    /// Read a line of input (from stdin or replay file).
    /// In record mode, also writes to the record file.
    fn read_line(&mut self) -> io::Result<String> {
        match &mut self.mode {
            InputMode::Interactive => {
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                Ok(input)
            }
            InputMode::Record { file } => {
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                // Write the trimmed input to the record file
                writeln!(file, "{}", input.trim())?;
                file.flush()?;
                Ok(input)
            }
            InputMode::Replay { lines, index } => {
                if *index < lines.len() {
                    let line = lines[*index].clone();
                    *index += 1;
                    // Print the replayed input for visibility
                    println!("{}", line);
                    Ok(format!("{}\n", line))
                } else {
                    // Out of replay inputs - return empty to trigger end
                    Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Replay inputs exhausted",
                    ))
                }
            }
        }
    }

    /// Check if we're in replay mode and have exhausted inputs.
    fn is_replay_exhausted(&self) -> bool {
        matches!(&self.mode, InputMode::Replay { lines, index } if *index >= lines.len())
    }
}

/// Initialize the global input manager.
pub fn init_input_manager(record_file: Option<&str>, replay_file: Option<&str>) {
    INPUT_MANAGER.with(|im| {
        let manager = if let Some(path) = replay_file {
            InputManager::new_replay(path).unwrap_or_else(|e| {
                eprintln!("Failed to open replay file '{}': {}", path, e);
                std::process::exit(1);
            })
        } else if let Some(path) = record_file {
            InputManager::new_record(path).unwrap_or_else(|e| {
                eprintln!("Failed to create record file '{}': {}", path, e);
                std::process::exit(1);
            })
        } else {
            InputManager::new_interactive()
        };
        *im.borrow_mut() = manager;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardDefinitionBuilder;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::definitions::{basic_island, counterspell, force_of_will, lightning_bolt};
    use crate::color::ColorSet;
    use crate::costs::PaymentReason;
    use crate::decisions::context::{TargetRequirementContext, TargetsContext};
    use crate::effect::{Effect, Value};
    use crate::filter::Comparison;
    use crate::grant::Grantable;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn stage_spell_cast_for_test(
        game: &mut GameState,
        spell_id: ObjectId,
        caster: PlayerId,
        from_zone: Zone,
    ) {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(spell_id, caster, from_zone),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    fn stage_cards_drawn_for_test(game: &mut GameState, player: PlayerId, count: u32) {
        let cards = (0..count).map(|_| game.new_object_id()).collect();
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::other::CardsDrawnEvent::new(player, cards, count > 0),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    fn stage_commit_crime_for_test(game: &mut GameState, player: PlayerId) {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::other::KeywordActionEvent::new(
                crate::events::other::KeywordActionKind::CommitCrime,
                player,
                ObjectId::from_raw(0),
                1,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    fn stage_life_gain_for_test(game: &mut GameState, player: PlayerId, amount: u32) {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LifeGainEvent::new(player, amount),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    fn stage_life_loss_for_test(game: &mut GameState, player: PlayerId, amount: u32) {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LifeLossEvent::from_effect(player, amount),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    #[test]
    fn krrik_keeps_black_spell_costs_as_black_pips() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = CardBuilder::new(CardId::from_raw(7000), "Krrik Cost Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("helper permanent should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::krrik_black_mana_may_be_paid_with_life(),
            ));

        let spell = CardBuilder::new(CardId::from_raw(7001), "Black Cost Probe")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell should exist");
        let base_cost = spell_obj
            .mana_cost
            .as_ref()
            .expect("spell should have a cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{1}{B}{B}");
    }

    #[test]
    fn trinisphere_raises_single_black_spell_to_three_total_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = CardBuilder::new(CardId::from_raw(7002), "Trinisphere Helper")
            .card_types(vec![CardType::Artifact])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("helper permanent should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::minimum_spell_total_mana(3),
            ));

        let spell = CardBuilder::new(CardId::from_raw(7003), "Cheap Black Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Black]))
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell should exist");
        let base_cost = spell_obj
            .mana_cost
            .as_ref()
            .expect("spell should have a cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{B}{2}");
        assert_eq!(effective.mana_value(), 3);
    }

    #[test]
    fn trinisphere_counts_krrik_life_paid_black_pips_toward_floor() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let krrik = CardBuilder::new(CardId::from_raw(7004), "Krrik Cost Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let krrik_id = game.create_object_from_card(&krrik, alice, Zone::Battlefield);
        game.object_mut(krrik_id)
            .expect("krrik helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::krrik_black_mana_may_be_paid_with_life(),
            ));

        let trini = CardBuilder::new(CardId::from_raw(7005), "Trinisphere Helper")
            .card_types(vec![CardType::Artifact])
            .build();
        let trini_id = game.create_object_from_card(&trini, alice, Zone::Battlefield);
        game.object_mut(trini_id)
            .expect("trinisphere helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::minimum_spell_total_mana(3),
            ));

        let spell = CardBuilder::new(CardId::from_raw(7006), "Necro Probe")
            .card_types(vec![CardType::Enchantment])
            .mana_cost(ManaCost::from_symbols(vec![
                ManaSymbol::Black,
                ManaSymbol::Black,
                ManaSymbol::Black,
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
        let effective = {
            let spell_obj = game.object(spell_id).expect("spell should exist");
            let base_cost = spell_obj
                .mana_cost
                .as_ref()
                .expect("spell should have a cost");
            calculate_effective_mana_cost(&game, alice, spell_obj, base_cost)
        };

        assert_eq!(effective.to_oracle(), "{B}{B}{B}");
        assert_eq!(effective.mana_value(), 3);
        assert!(
            game.try_pay_mana_cost_with_reason(
                alice,
                Some(spell_id),
                &effective,
                0,
                PaymentReason::CastSpell
            ),
            "three black pips should already satisfy Trinisphere even when Krrik pays them with life"
        );
        assert_eq!(game.player(alice).expect("alice exists").life, 14);
    }

    #[test]
    fn yasharn_blocks_krrik_life_payment_without_rewriting_spell_costs() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let krrik = CardBuilder::new(CardId::from_raw(7007), "Krrik Cost Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let krrik_id = game.create_object_from_card(&krrik, alice, Zone::Battlefield);
        game.object_mut(krrik_id)
            .expect("krrik helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::krrik_black_mana_may_be_paid_with_life(),
            ));

        let yasharn = CardBuilder::new(CardId::from_raw(7008), "Yasharn Cost Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let yasharn_id = game.create_object_from_card(&yasharn, alice, Zone::Battlefield);
        game.object_mut(yasharn_id)
            .expect("yasharn helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
            ));

        let spell = CardBuilder::new(CardId::from_raw(7009), "Yasharn Probe")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_symbols(vec![
                ManaSymbol::Black,
                ManaSymbol::Black,
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell should exist");
        let base_cost = spell_obj
            .mana_cost
            .as_ref()
            .expect("spell should have a cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{B}{B}");
        assert!(
            !game.can_pay_mana_cost_with_reason(
                alice,
                Some(spell_id),
                &effective,
                0,
                PaymentReason::CastSpell
            ),
            "without black mana in the pool, Yasharn should remove Krrik's life-payment option"
        );
    }

    #[test]
    fn yasharn_blocks_force_of_will_alternative_cost() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let bolt_id = game.create_object_from_definition(&lightning_bolt(), bob, Zone::Stack);
        game.stack.push(crate::StackEntry::new(bolt_id, bob));

        let yasharn = CardBuilder::new(CardId::from_raw(7010), "Yasharn Cost Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let yasharn_id = game.create_object_from_card(&yasharn, alice, Zone::Battlefield);
        game.object_mut(yasharn_id)
            .expect("yasharn helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
            ));

        let fow_id = game.create_object_from_definition(&force_of_will(), alice, Zone::Hand);
        game.create_object_from_definition(&counterspell(), alice, Zone::Hand);

        let fow_obj = game.object(fow_id).expect("force of will should exist");
        let method = &fow_obj.alternative_casts[0];
        assert!(
            !can_cast_with_alternative_from_hand(&game, alice, fow_obj, fow_id, method),
            "Yasharn should stop Force of Will's alternative cost because it includes paying life"
        );
    }

    #[test]
    fn trinisphere_requires_three_mana_for_force_of_will_alternative_cost() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let bolt_id = game.create_object_from_definition(&lightning_bolt(), bob, Zone::Stack);
        game.stack.push(crate::StackEntry::new(bolt_id, bob));

        let trini = CardBuilder::new(CardId::from_raw(7011), "Trinisphere Helper")
            .card_types(vec![CardType::Artifact])
            .build();
        let trini_id = game.create_object_from_card(&trini, alice, Zone::Battlefield);
        game.object_mut(trini_id)
            .expect("trinisphere helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::minimum_spell_total_mana(3),
            ));

        let fow_id = game.create_object_from_definition(&force_of_will(), alice, Zone::Hand);
        game.create_object_from_definition(&counterspell(), alice, Zone::Hand);

        for _ in 0..2 {
            game.create_object_from_definition(&basic_island(), alice, Zone::Battlefield);
        }
        let fow_obj = game.object(fow_id).expect("force of will should exist");
        let method = &fow_obj.alternative_casts[0];
        assert!(
            !can_cast_with_alternative_from_hand(&game, alice, fow_obj, fow_id, method),
            "Trinisphere should make Force of Will's free alternative cost require three mana"
        );

        game.create_object_from_definition(&basic_island(), alice, Zone::Battlefield);
        let fow_obj = game.object(fow_id).expect("force of will should exist");
        let method = &fow_obj.alternative_casts[0];
        assert!(
            can_cast_with_alternative_from_hand(&game, alice, fow_obj, fow_id, method),
            "with three Islands available, the alternative cost should become legal again"
        );
    }

    fn stage_noncombat_damage_to_player_for_test(
        game: &mut GameState,
        source: ObjectId,
        player: PlayerId,
        amount: u32,
    ) {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                source,
                crate::game_event::DamageTarget::Player(player),
                amount,
                false,
                crate::events::cause::EventCause::effect(),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    fn stage_artifact_sacrifice_for_test(game: &mut GameState, player: PlayerId) {
        let artifact = CardBuilder::new(CardId::new(), "Sacrificed Artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_id = game.create_object_from_card(&artifact, player, Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(artifact_id).expect("artifact exists"),
            game,
        );
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::permanents::SacrificeEvent::new(artifact_id, None)
                .with_snapshot(Some(snapshot), Some(player)),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    #[test]
    fn test_compute_legal_actions_basic() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let actions = compute_legal_actions(&game, alice);

        // Should at least have pass priority
        assert!(actions.contains(&LegalAction::PassPriority));
    }

    #[test]
    fn test_compute_legal_actions_with_land() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Add a land to hand
        let land = CardBuilder::new(CardId::from_raw(1), "Forest")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Hand);

        let actions = compute_legal_actions(&game, alice);

        // Should have play land action
        assert!(actions.contains(&LegalAction::PlayLand { land_id }));
    }

    #[test]
    fn test_compute_legal_actions_includes_graveyard_land_with_play_from_grant() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let land = CardBuilder::new(CardId::from_raw(71_018), "Ash Barrens")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Graveyard);

        let source_id = game.new_object_id();
        game.grant_registry.grant_to_filter_until_end_of_turn(
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Graveyard,
            alice,
            Grantable::play_from(),
            source_id,
            game.turn.turn_number,
        );

        let actions = compute_legal_actions(&game, alice);

        assert!(
            actions.contains(&LegalAction::PlayLand { land_id }),
            "play-from-graveyard grants should surface playable lands as land actions"
        );
    }

    #[test]
    fn test_compute_legal_actions_excludes_graveyard_land_after_land_play_used() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let land = CardBuilder::new(CardId::from_raw(71_019), "Haunted Mire")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Graveyard);

        let source_id = game.new_object_id();
        game.grant_registry.grant_to_filter_until_end_of_turn(
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Graveyard,
            alice,
            Grantable::play_from(),
            source_id,
            game.turn.turn_number,
        );

        game.player_mut(alice)
            .expect("alice should exist")
            .record_land_play();

        let actions = compute_legal_actions(&game, alice);

        assert!(
            !actions.contains(&LegalAction::PlayLand { land_id }),
            "granted graveyard land plays must still respect the per-turn land limit"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_exile_land_with_play_from_grant() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let land = CardBuilder::new(CardId::from_raw(71_020), "Forgotten Cave")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Exile);

        let source_id = game.new_object_id();
        game.grant_registry.grant_to_filter_until_end_of_turn(
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Exile,
            alice,
            Grantable::play_from(),
            source_id,
            game.turn.turn_number,
        );

        let actions = compute_legal_actions(&game, alice);

        assert!(
            actions.contains(&LegalAction::PlayLand { land_id }),
            "public-zone play-from grants should continue to surface exile lands"
        );
    }

    #[test]
    fn simple_battlefield_mana_ability_output_recognizes_basic_land_tap() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let forest = CardDefinitionBuilder::new(CardId::from_raw(700_901), "Forest")
            .card_types(vec![CardType::Land])
            .parse_text("{T}: Add {G}.")
            .expect("forest mana text should parse");
        let forest_id = game.create_object_from_definition(&forest, alice, Zone::Battlefield);
        let ability = game
            .current_ability(forest_id, 0)
            .expect("forest should expose a mana ability");
        let view = DerivedGameView::new(&game);

        assert_eq!(
            simple_battlefield_mana_ability_output(&game, alice, forest_id, 0, &ability, &view),
            Some(vec![ManaSymbol::Green]),
        );
    }

    #[test]
    fn simple_battlefield_mana_ability_output_ignores_non_mana_activated_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let yawgmoth = crate::cards::definitions::yawgmoth_thran_physician();
        let yawgmoth_id = game.create_object_from_definition(&yawgmoth, alice, Zone::Battlefield);
        let ability = game
            .current_ability(yawgmoth_id, 0)
            .expect("Yawgmoth should expose its first activated ability");
        let view = DerivedGameView::new(&game);

        assert_eq!(
            simple_battlefield_mana_ability_output(&game, alice, yawgmoth_id, 0, &ability, &view),
            None,
        );
    }

    #[test]
    fn test_select_first_decision_maker_supports_multi_target_requirement() {
        let first = Target::Object(ObjectId::from_raw(1));
        let second = Target::Object(ObjectId::from_raw(2));
        let ctx = TargetsContext::new(
            PlayerId::from_index(0),
            ObjectId::from_raw(99),
            "test spell",
            vec![TargetRequirementContext {
                description: "two targets".to_string(),
                legal_targets: vec![first, second],
                min_targets: 2,
                max_targets: Some(2),
            }],
        );

        let mut dm = SelectFirstDecisionMaker;
        let chosen = dm.decide_targets(&setup_game(), &ctx);

        assert_eq!(chosen, vec![first, second]);
    }

    /// Tests computation of legal attackers during declare attackers step.
    ///
    /// Scenario: Alice controls a Grizzly Bears that has been on the battlefield
    /// since the beginning of her turn (no summoning sickness). When computing
    /// legal attackers, it should be available to attack Bob (player 1).
    #[test]
    fn test_compute_legal_attackers() {
        use crate::cards::definitions::grizzly_bears;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Grizzly Bears on battlefield
        let bears_def = grizzly_bears();
        let creature_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);

        // Remove summoning sickness (creature has been on battlefield since turn start)
        game.remove_summoning_sickness(creature_id);

        let combat = CombatState::default();
        let options = compute_legal_attackers(&game, &combat);

        assert_eq!(options.len(), 1, "Should have one legal attacker");
        assert_eq!(options[0].creature, creature_id);
        assert!(
            !options[0].must_attack,
            "Grizzly Bears doesn't have 'must attack'"
        );
        // Should be able to attack Bob (player 1)
        assert!(
            options[0]
                .valid_targets
                .contains(&AttackTarget::Player(bob)),
            "Should be able to attack the opponent"
        );
    }

    #[test]
    fn test_compute_legal_attackers_respects_cant_attack_restriction_tracker() {
        use crate::cards::definitions::grizzly_bears;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let bears_def = grizzly_bears();
        let creature_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(creature_id);
        game.cant_effects.cant_attack.insert(creature_id);

        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.is_empty(),
            "cant-attack tracker should prevent declaring attackers, got {options:?}"
        );
    }

    #[test]
    fn test_compute_legal_attackers_respects_cant_attack_alone_with_single_attacker() {
        use crate::cards::definitions::grizzly_bears;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let bears_def = grizzly_bears();
        let creature_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(creature_id);
        game.cant_effects.cant_attack_alone.insert(creature_id);

        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.is_empty(),
            "single creature with can't-attack-alone should not be legal attacker, got {options:?}"
        );
    }

    #[test]
    fn test_compute_legal_attackers_respects_cast_creature_spell_attack_restriction() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let cohort_card = CardBuilder::new(CardId::from_raw(901), "Goblin Cohort Variant")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::new(
                crate::card::PtValue::Fixed(2),
                crate::card::PtValue::Fixed(2),
            ))
            .build();
        let cohort_id = game.create_object_from_card(&cohort_card, alice, Zone::Battlefield);
        game.object_mut(cohort_id)
            .expect("cohort exists")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn(),
            ));
        game.remove_summoning_sickness(cohort_id);

        game.refresh_continuous_state();
        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.iter().all(|option| option.creature != cohort_id),
            "cohort should not be legal attacker before controller casts a creature spell this turn"
        );

        let prior_creature = CardBuilder::new(CardId::from_raw(902), "Prior Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let prior_id = game.create_object_from_card(&prior_creature, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_id).expect("prior creature exists"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);

        game.refresh_continuous_state();
        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.iter().any(|option| option.creature == cohort_id),
            "cohort should become a legal attacker after controller casts a creature spell this turn"
        );
    }

    #[test]
    fn test_compute_legal_attackers_respects_graveyard_threshold_attack_restriction() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let threshold_card = CardBuilder::new(CardId::from_raw(903), "Threshold Raider Variant")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::new(
                crate::card::PtValue::Fixed(3),
                crate::card::PtValue::Fixed(2),
            ))
            .build();
        let attacker_id = game.create_object_from_card(&threshold_card, alice, Zone::Battlefield);
        game.object_mut(attacker_id)
            .expect("threshold attacker exists")
                .abilities
                .push(Ability::static_ability(
                    StaticAbility::cant_attack_unless_condition(
                    crate::static_abilities::CantAttackUnlessConditionSpec::ControllerGraveyardHasCardsAtLeast(5),
                    "Can't attack unless there are five or more cards in your graveyard",
                ),
            ));
        game.remove_summoning_sickness(attacker_id);

        game.refresh_continuous_state();
        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.iter().all(|option| option.creature != attacker_id),
            "attacker should not be legal before threshold is met"
        );

        for idx in 0..5 {
            let filler =
                CardBuilder::new(CardId::from_raw(1000 + idx), &format!("Filler {}", idx + 1))
                    .card_types(vec![CardType::Creature])
                    .build();
            let _ = game.create_object_from_card(&filler, alice, Zone::Graveyard);
        }

        game.refresh_continuous_state();
        let options = compute_legal_attackers(&game, &CombatState::default());
        assert!(
            options.iter().any(|option| option.creature == attacker_id),
            "attacker should become legal after graveyard threshold is met"
        );
    }

    #[test]
    fn test_compute_legal_blockers_respects_cant_block_alone_with_single_blocker() {
        use crate::cards::definitions::grizzly_bears;
        use crate::combat_state::AttackerInfo;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let blocker_def = grizzly_bears();
        let blocker_id = game.create_object_from_definition(&blocker_def, alice, Zone::Battlefield);
        game.cant_effects.cant_block_alone.insert(blocker_id);

        let attacker_def = grizzly_bears();
        let attacker_id = game.create_object_from_definition(&attacker_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(alice),
        });

        let options = compute_legal_blockers(&game, &combat, alice);
        assert_eq!(options.len(), 1, "expected one attacker option");
        assert!(
            options[0].valid_blockers.is_empty(),
            "single creature with can't-block-alone should not be a legal blocker, got {options:?}"
        );
    }

    #[test]
    fn test_compute_legal_blockers_excludes_tapped_creatures() {
        use crate::cards::definitions::grizzly_bears;
        use crate::combat_state::AttackerInfo;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let blocker_def = grizzly_bears();
        let blocker_id = game.create_object_from_definition(&blocker_def, alice, Zone::Battlefield);
        game.tap(blocker_id);

        let attacker_def = grizzly_bears();
        let attacker_id = game.create_object_from_definition(&attacker_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(alice),
        });

        let options = compute_legal_blockers(&game, &combat, alice);
        assert_eq!(options.len(), 1, "expected one attacker option");
        assert!(
            options[0].valid_blockers.is_empty(),
            "tapped creature should not be a legal blocker, got {options:?}"
        );
    }

    #[test]
    fn global_colored_spell_cost_increase_adds_pips_to_effective_cost() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Battlefield permanent that taxes black spells you cast by {B}.
        let tax_card = CardBuilder::new(CardId::from_raw(10), "Derelor Variant")
            .card_types(vec![CardType::Creature])
            .build();
        let tax_id = game.create_object_from_card(&tax_card, alice, Zone::Battlefield);
        let mut filter = ObjectFilter::default();
        filter.colors = Some(ColorSet::BLACK);
        filter.cast_by = Some(PlayerFilter::You);
        let tax = StaticAbility::new(crate::static_abilities::CostIncreaseManaCost::new(
            filter,
            ManaCost::from_symbols(vec![ManaSymbol::Black]),
        ));
        game.object_mut(tax_id)
            .expect("tax permanent exists")
            .abilities
            .push(Ability::static_ability(tax));

        // A black spell with base cost {1}{B}.
        let black_spell_card = CardBuilder::new(CardId::from_raw(11), "Black Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&black_spell_card, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{1}{B}{B}");
    }

    #[test]
    fn global_spell_cost_increase_matches_spell_filter_power() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let tax_card = CardBuilder::new(CardId::from_raw(12), "Power Tax")
            .card_types(vec![CardType::Creature])
            .build();
        let tax_id = game.create_object_from_card(&tax_card, alice, Zone::Battlefield);
        let mut filter = ObjectFilter::default();
        filter.power = Some(Comparison::GreaterThanOrEqual(4));
        let tax = StaticAbility::new(crate::static_abilities::CostIncrease::new(
            filter,
            Value::Fixed(1),
        ));
        game.object_mut(tax_id)
            .expect("tax permanent exists")
            .abilities
            .push(Ability::static_ability(tax));

        let creature_spell = CardBuilder::new(CardId::from_raw(13), "Large Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Green],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&creature_spell, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{G}{1}");
    }

    #[test]
    fn global_spell_cost_increase_uses_caster_for_spell_filter_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let tax_card = CardBuilder::new(CardId::from_raw(14), "Caster Tax")
            .card_types(vec![CardType::Creature])
            .build();
        let tax_id = game.create_object_from_card(&tax_card, alice, Zone::Battlefield);
        let mut filter = ObjectFilter::default();
        filter.cast_by = Some(PlayerFilter::You);
        let tax = StaticAbility::new(crate::static_abilities::CostIncrease::new(
            filter,
            Value::Fixed(1),
        ));
        game.object_mut(tax_id)
            .expect("tax permanent exists")
            .abilities
            .push(Ability::static_ability(tax));

        let spell_card = CardBuilder::new(CardId::from_raw(15), "Borrowed Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        // Bob owns/controls the card object, but we evaluate castability for Alice.
        let spell_id = game.create_object_from_card(&spell_card, bob, Zone::Exile);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

        let effective_for_alice = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective_for_alice.to_oracle(), "{2}{U}{1}");

        let effective_for_bob = calculate_effective_mana_cost(&game, bob, spell_obj, base_cost);
        assert_eq!(effective_for_bob.to_oracle(), "{2}{U}");
    }

    #[test]
    fn spell_attached_global_cost_reduction_requires_functional_zone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(16), "Zone Scoped Reducer")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .build();
        let mut filter = ObjectFilter::default();
        filter.cast_by = Some(PlayerFilter::You);

        // A battlefield-only static modifier on a spell card in hand should not apply.
        let battlefield_only_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
            filter.clone(),
            Value::Fixed(1),
        ));
        game.object_mut(battlefield_only_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(reduction));
        let battlefield_only_obj = game.object(battlefield_only_id).expect("spell exists");
        let battlefield_only_base = battlefield_only_obj
            .mana_cost
            .as_ref()
            .expect("spell has mana cost");
        let battlefield_only_effective = calculate_effective_mana_cost(
            &game,
            alice,
            battlefield_only_obj,
            battlefield_only_base,
        );
        assert_eq!(
            battlefield_only_effective.to_oracle(),
            "{2}",
            "battlefield-only modifiers must not apply while the spell is in hand"
        );

        // A hand/stack-scoped modifier still applies (e.g. Undaunted-style implementations).
        let hand_scoped_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let hand_scoped_reduction = StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, Value::Fixed(1)),
        );
        game.object_mut(hand_scoped_id)
            .expect("spell exists")
            .abilities
            .push(
                Ability::static_ability(hand_scoped_reduction)
                    .in_zones(vec![Zone::Hand, Zone::Stack]),
            );
        let hand_scoped_obj = game.object(hand_scoped_id).expect("spell exists");
        let hand_scoped_base = hand_scoped_obj
            .mana_cost
            .as_ref()
            .expect("spell has mana cost");
        let hand_scoped_effective =
            calculate_effective_mana_cost(&game, alice, hand_scoped_obj, hand_scoped_base);
        assert_eq!(
            hand_scoped_effective.to_oracle(),
            "{1}",
            "zone-scoped spell modifiers should still apply"
        );
    }

    #[test]
    fn spell_attached_global_cost_reduction_respects_color_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mut red_filter = ObjectFilter::default();
        red_filter.cast_by = Some(PlayerFilter::You);
        red_filter.colors = Some(ColorSet::RED);

        let colorless_spell = CardBuilder::new(CardId::from_raw(17), "Colorless Probe")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .build();
        let colorless_id = game.create_object_from_card(&colorless_spell, alice, Zone::Hand);
        let reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
            red_filter.clone(),
            Value::Fixed(1),
        ));
        game.object_mut(colorless_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(reduction).in_zones(vec![Zone::Hand, Zone::Stack]));
        let colorless_obj = game.object(colorless_id).expect("spell exists");
        let colorless_base = colorless_obj
            .mana_cost
            .as_ref()
            .expect("spell has mana cost");
        let colorless_effective =
            calculate_effective_mana_cost(&game, alice, colorless_obj, colorless_base);
        assert_eq!(
            colorless_effective.to_oracle(),
            "{2}",
            "red-only filter must not reduce non-red spell costs"
        );

        let red_spell = CardBuilder::new(CardId::from_raw(18), "Red Probe")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let red_id = game.create_object_from_card(&red_spell, alice, Zone::Hand);
        let red_reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
            red_filter,
            Value::Fixed(1),
        ));
        game.object_mut(red_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(red_reduction).in_zones(vec![Zone::Hand, Zone::Stack]));
        let red_obj = game.object(red_id).expect("spell exists");
        let red_base = red_obj.mana_cost.as_ref().expect("spell has mana cost");
        let red_effective = calculate_effective_mana_cost(&game, alice, red_obj, red_base);
        assert_eq!(
            red_effective.to_oracle(),
            "{R}",
            "red-only filter should reduce matching red spell costs"
        );
    }

    #[test]
    fn dynamic_spell_cost_reduction_distinct_names_reduces_generic_cost() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Two differently named lands you control.
        let forest = CardBuilder::new(CardId::from_raw(20), "Forest Variant")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&forest, alice, Zone::Battlefield);
        let island = CardBuilder::new(CardId::from_raw(21), "Island Variant")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&island, alice, Zone::Battlefield);

        // A spell with base cost {6}{G} that costs {X} less where X is distinct land names.
        let spell_card = CardBuilder::new(CardId::from_raw(22), "Fungal Colossus Variant")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::Green],
            ]))
            .build();
        let mut filter = ObjectFilter::land().you_control();
        filter.zone = Some(Zone::Battlefield);
        let reduction = StaticAbility::new(crate::static_abilities::CostReduction::new(
            ObjectFilter::default(),
            Value::DistinctNames(filter),
        ));

        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(reduction).in_zones(vec![Zone::Hand, Zone::Stack]));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{G}");
    }

    #[test]
    fn conditional_this_spell_cost_reduction_only_applies_when_active() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(30), "Avatar Cost Variant")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(6),
            crate::static_abilities::ThisSpellCostCondition::YouLifeTotalOrLess(3),
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Condition not met.
        game.player_mut(alice).expect("alice exists").life = 4;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{6}{B}{B}");

        // Condition met.
        game.player_mut(alice).expect("alice exists").life = 3;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{B}{B}");
    }

    #[test]
    fn conditional_this_spell_mana_cost_reduction_checks_opponent_drawn_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell_card = CardBuilder::new(CardId::from_raw(31), "Even the Score Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let reduction = ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]);
        let ability = StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(
                reduction,
                crate::static_abilities::ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(4),
            ),
        );
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Condition not met.
        stage_cards_drawn_for_test(&mut game, bob, 3);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{U}{U}{U}");

        // Condition met.
        game.turn_history.clear_for_new_turn();
        stage_cards_drawn_for_test(&mut game, bob, 4);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}");
    }

    #[test]
    fn conditional_this_spell_mana_cost_reduction_checks_opponent_cast_spells() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell_card = CardBuilder::new(CardId::from_raw(32), "Ertai's Scorn Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let reduction = ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]);
        let ability = StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(
                reduction,
                crate::static_abilities::ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(
                    2,
                ),
            ),
        );
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Condition not met.
        stage_spell_cast_for_test(&mut game, ObjectId::from_raw(3201), bob, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{U}");

        // Condition met.
        game.turn_history.clear_for_new_turn();
        stage_spell_cast_for_test(&mut game, ObjectId::from_raw(3201), bob, Zone::Hand);
        stage_spell_cast_for_test(&mut game, ObjectId::from_raw(3202), bob, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}");
    }

    #[test]
    fn conditional_this_spell_mana_cost_reduction_with_generic_and_colored_pips() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell_card = CardBuilder::new(CardId::from_raw(33), "Discontinuity Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let reduction = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]);
        let ability = StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(
                reduction,
                crate::static_abilities::ThisSpellCostCondition::YourTurn,
            ),
        );
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Condition met (it's your turn).
        game.turn.active_player = alice;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}");

        // Condition not met.
        game.turn.active_player = bob;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{6}{U}{U}");
    }

    #[test]
    fn this_spell_cost_reduction_with_target_condition_uses_chosen_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(133), "Target Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition = crate::static_abilities::ThisSpellCostCondition::TargetsObject(
            ObjectFilter::creature().tapped(),
        );
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let creature_card = CardBuilder::new(CardId::from_raw(134), "Target Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

        // Untapped target does not satisfy condition.
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost_for_payment_with_chosen_targets(
            &game,
            alice,
            spell_obj,
            base_cost,
            &[Target::Object(creature_id)],
        );
        assert_eq!(effective.to_oracle(), "{3}{R}");

        // Tapped target satisfies condition.
        game.tap(creature_id);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost_for_payment_with_chosen_targets(
            &game,
            alice,
            spell_obj,
            base_cost,
            &[Target::Object(creature_id)],
        );
        assert_eq!(effective.to_oracle(), "{1}{R}");
    }

    #[test]
    fn this_spell_cost_reduction_cast_another_instant_or_sorcery_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(135), "Spell History Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition =
            crate::static_abilities::ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
                count: 1,
                card_types: vec![CardType::Instant, CardType::Sorcery],
            };
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // No prior instant/sorcery this turn.
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{U}");

        // One instant cast this turn enables reduction.
        let prior_card = CardBuilder::new(CardId::from_raw(136), "Prior Instant")
            .card_types(vec![CardType::Instant])
            .build();
        let prior_id = game.create_object_from_card(&prior_card, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_id).expect("prior instant exists"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{U}");
    }

    #[test]
    fn this_spell_cost_reduction_graveyard_card_count_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card =
            CardBuilder::new(CardId::from_raw(137), "Graveyard Cards Discount Variant")
                .card_types(vec![CardType::Creature])
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(8)],
                    vec![ManaSymbol::Black],
                ]))
                .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition =
            crate::static_abilities::ThisSpellCostCondition::YouHaveCardsInYourGraveyardOrMore(9);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(3),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Not enough cards.
        for idx in 0..8 {
            let filler = CardBuilder::new(CardId::from_raw(200 + idx), format!("GY Card {idx}"))
                .card_types(vec![CardType::Instant])
                .build();
            game.create_object_from_card(&filler, alice, Zone::Graveyard);
        }
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{8}{B}");

        // Ninth card enables reduction.
        let extra = CardBuilder::new(CardId::from_raw(300), "GY Extra")
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_card(&extra, alice, Zone::Graveyard);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{5}{B}");
    }

    #[test]
    fn this_spell_cost_reduction_creature_attacking_you_condition() {
        use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let spell_card = CardBuilder::new(CardId::from_raw(138), "Attack Trap Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Black],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition = crate::static_abilities::ThisSpellCostCondition::CreatureIsAttackingYou;
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // No attackers: no reduction.
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{B}");

        // One attacker attacking Alice enables reduction.
        let attacker_card = CardBuilder::new(CardId::from_raw(139), "Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let attacker_id = game.create_object_from_card(&attacker_card, bob, Zone::Battlefield);
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(alice),
        });
        game.combat = Some(combat);

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{B}");
    }

    #[test]
    fn this_spell_cost_reduction_is_night_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(140), "Night Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition = crate::static_abilities::ThisSpellCostCondition::IsNight;
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{R}");

        game.is_night = true;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{R}");
    }

    #[test]
    fn this_spell_cost_reduction_sacrificed_artifact_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(141), "Artifact Sac Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(5)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition =
            crate::static_abilities::ThisSpellCostCondition::YouSacrificedArtifactThisTurn;
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(3),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{5}{R}");

        stage_artifact_sacrifice_for_test(&mut game, alice);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{R}");
    }

    #[test]
    fn this_spell_cost_reduction_creature_left_battlefield_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(142), "Creature Left Discount Variant")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Green],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition = crate::static_abilities::ThisSpellCostCondition::
            CreatureLeftBattlefieldUnderYourControlThisTurn;
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{G}");

        let departed_creature = CardBuilder::new(CardId::from_raw(5000), "Fallen Helper")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let departed_id =
            game.create_object_from_card(&departed_creature, alice, Zone::Battlefield);
        game.move_object_by_effect(departed_id, Zone::Graveyard);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{G}");
    }

    #[test]
    fn this_spell_cost_reduction_committed_crime_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(143), "Crime Discount Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition = crate::static_abilities::ThisSpellCostCondition::YouCommittedCrimeThisTurn;
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(1),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{U}");

        stage_commit_crime_for_test(&mut game, alice);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{U}");
    }

    #[test]
    fn this_spell_cost_reduction_only_named_creatures_in_hand_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card = CardBuilder::new(CardId::from_raw(144), "Mothrider Cavalry")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::White],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let condition =
            crate::static_abilities::ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(
                "mothrider cavalry".to_string(),
            );
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Fixed(2),
            condition,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // Only this card in hand (named Mothrider Cavalry): reduction applies.
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{4}{W}");

        // Another creature with a different name disables the reduction.
        let other_creature = CardBuilder::new(CardId::from_raw(145), "Not Mothrider")
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&other_creature, alice, Zone::Hand);
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{6}{W}");
    }

    #[test]
    fn this_spell_cost_reduction_x_uses_life_difference_from_starting() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_card =
            CardBuilder::new(CardId::from_raw(146), "Starting Life X Discount Variant")
                .card_types(vec![CardType::Creature])
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(13)],
                    vec![ManaSymbol::Black],
                ]))
                .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::X,
            crate::static_abilities::ThisSpellCostCondition::LifeTotalLessThanStarting,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        // At starting life, no reduction.
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{13}{B}");

        // Reduced by life lost from starting life total.
        game.player_mut(alice).expect("player exists").life = 12;
        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{5}{B}");
    }

    #[test]
    fn this_spell_cost_reduction_supports_devotion_where_x_is_clause() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Devotion to black = 3.
        let perm1 = CardBuilder::new(CardId::from_raw(40), "BB Permanent")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ]))
            .build();
        game.create_object_from_card(&perm1, alice, Zone::Battlefield);
        let perm2 = CardBuilder::new(CardId::from_raw(41), "1B Permanent")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
            ]))
            .build();
        game.create_object_from_card(&perm2, alice, Zone::Battlefield);

        let spell_card = CardBuilder::new(CardId::from_raw(42), "Devotion Cost Variant")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(6)],
                vec![ManaSymbol::Black],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::Devotion {
                player: PlayerFilter::You,
                color: crate::color::Color::Black,
            },
            crate::static_abilities::ThisSpellCostCondition::Always,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");

        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{B}");
    }

    #[test]
    fn this_spell_cost_reduction_supports_total_power_where_x_is_clause() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let bear = CardBuilder::new(CardId::from_raw(43), "Cost Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        game.create_object_from_card(&bear, alice, Zone::Battlefield);
        let giant = CardBuilder::new(CardId::from_raw(44), "Cost Giant")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        game.create_object_from_card(&giant, alice, Zone::Battlefield);

        let spell_card = CardBuilder::new(CardId::from_raw(45), "Power Discount Variant")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(10)],
                vec![ManaSymbol::Green],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::TotalPower(ObjectFilter::creature().you_control()),
            crate::static_abilities::ThisSpellCostCondition::Always,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{G}");
    }

    #[test]
    fn this_spell_cost_reduction_supports_life_gained_this_turn_where_x_is_clause() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        stage_life_gain_for_test(&mut game, alice, 5);

        let spell_card = CardBuilder::new(CardId::from_raw(46), "Life Discount Variant")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(7)],
                vec![ManaSymbol::Green],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::LifeGainedThisTurn(PlayerFilter::You),
            crate::static_abilities::ThisSpellCostCondition::Always,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{G}");
    }

    #[test]
    fn this_spell_cost_reduction_supports_noncombat_damage_to_opponents_where_x_is_clause() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        stage_noncombat_damage_to_player_for_test(&mut game, ObjectId::from_raw(4701), bob, 6);

        let spell_card = CardBuilder::new(CardId::from_raw(47), "Damage Discount Variant")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(8)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);
        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::NoncombatDamageDealtToPlayersThisTurn(PlayerFilter::Opponent),
            crate::static_abilities::ThisSpellCostCondition::Always,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{2}{R}");
    }

    #[test]
    fn this_spell_cost_reduction_supports_greatest_commander_mana_value_where_x_is_clause() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let commander_battlefield = CardBuilder::new(CardId::from_raw(48), "Battlefield Commander")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Green],
            ]))
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let battlefield_id =
            game.create_object_from_card(&commander_battlefield, alice, Zone::Battlefield);
        game.set_as_commander(battlefield_id, alice);

        let commander_command_zone =
            CardBuilder::new(CardId::from_raw(49), "Command Zone Commander")
                .card_types(vec![CardType::Creature])
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(5)],
                    vec![ManaSymbol::Blue],
                ]))
                .power_toughness(PowerToughness::fixed(5, 5))
                .build();
        let command_id =
            game.create_object_from_card(&commander_command_zone, alice, Zone::Command);
        game.set_as_commander(command_id, alice);

        let spell_card = CardBuilder::new(CardId::from_raw(50), "Commander Discount Variant")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(9)],
                vec![ManaSymbol::White],
            ]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Hand);

        let mut battlefield_filter = ObjectFilter::default();
        battlefield_filter.zone = Some(Zone::Battlefield);
        battlefield_filter.owner = Some(PlayerFilter::You);
        battlefield_filter.is_commander = true;
        let mut command_filter = battlefield_filter.clone();
        command_filter.zone = Some(Zone::Command);
        let mut commander_filter = ObjectFilter::default();
        commander_filter.any_of = vec![battlefield_filter, command_filter];

        let ability = StaticAbility::new(crate::static_abilities::ThisSpellCostReduction::new(
            Value::GreatestManaValue(commander_filter),
            crate::static_abilities::ThisSpellCostCondition::Always,
        ));
        game.object_mut(spell_id)
            .expect("spell exists")
            .abilities
            .push(Ability::static_ability(ability));

        let spell_obj = game.object(spell_id).expect("spell exists");
        let base_cost = spell_obj.mana_cost.as_ref().expect("spell has mana cost");
        let effective = calculate_effective_mana_cost(&game, alice, spell_obj, base_cost);
        assert_eq!(effective.to_oracle(), "{3}{W}");
    }

    #[test]
    fn test_can_cast_spell_respects_cant_cast_creature_spells_restriction() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;

        let creature = CardBuilder::new(CardId::from_raw(77), "Restriction Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);
        let creature_obj = game
            .object(creature_id)
            .expect("creature in hand must exist")
            .clone();

        game.cant_effects.add_cant_cast_filter(
            alice,
            crate::target::ObjectFilter::default().with_type(CardType::Creature),
        );
        assert!(
            !can_cast_spell(&game, alice, &creature_obj, &CastingMethod::Normal),
            "creature spell should be uncastable when player can't cast creature spells"
        );
    }

    #[test]
    fn test_can_cast_spell_respects_cast_limit_one_per_turn_restriction() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let instant = CardBuilder::new(CardId::from_raw(78), "Restriction Spark")
            .card_types(vec![CardType::Instant])
            .build();
        let instant_id = game.create_object_from_card(&instant, alice, Zone::Hand);
        let instant_obj = game
            .object(instant_id)
            .expect("instant in hand must exist")
            .clone();

        game.cant_effects
            .add_cast_limit_filter(alice, crate::target::ObjectFilter::default());
        stage_spell_cast_for_test(&mut game, ObjectId::from_raw(7801), alice, Zone::Hand);

        assert!(
            !can_cast_spell(&game, alice, &instant_obj, &CastingMethod::Normal),
            "second spell in same turn should be blocked by one-spell limit"
        );
    }

    #[test]
    fn test_can_cast_spell_respects_noncreature_cast_limit() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let instant = CardBuilder::new(CardId::from_raw(79), "Restriction Snuff")
            .card_types(vec![CardType::Instant])
            .build();
        let instant_id = game.create_object_from_card(&instant, alice, Zone::Hand);
        let instant_obj = game
            .object(instant_id)
            .expect("instant in hand must exist")
            .clone();

        let prior_noncreature = CardBuilder::new(CardId::from_raw(80), "Prior Noncreature")
            .card_types(vec![CardType::Sorcery])
            .build();
        let prior_noncreature_id =
            game.create_object_from_card(&prior_noncreature, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_noncreature_id)
                .expect("prior noncreature must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_type(CardType::Creature),
        );

        assert!(
            !can_cast_spell(&game, alice, &instant_obj, &CastingMethod::Normal),
            "second noncreature spell in same turn should be blocked by noncreature cast limit"
        );
    }

    #[test]
    fn test_can_cast_spell_noncreature_limit_still_allows_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;

        let creature = CardBuilder::new(CardId::from_raw(81), "Restriction Beast")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::new())
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);
        let creature_obj = game
            .object(creature_id)
            .expect("creature in hand must exist")
            .clone();

        let prior_noncreature = CardBuilder::new(CardId::from_raw(82), "Prior Noncreature")
            .card_types(vec![CardType::Instant])
            .build();
        let prior_noncreature_id =
            game.create_object_from_card(&prior_noncreature, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_noncreature_id)
                .expect("prior noncreature must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_type(CardType::Creature),
        );

        assert!(
            can_cast_spell(&game, alice, &creature_obj, &CastingMethod::Normal),
            "noncreature cast limit should still allow creature spell"
        );
    }

    #[test]
    fn test_can_cast_spell_respects_nonartifact_cast_limit() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let nonartifact_spell = CardBuilder::new(CardId::from_raw(83), "Restriction Chant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::new())
            .build();
        let nonartifact_spell_id =
            game.create_object_from_card(&nonartifact_spell, alice, Zone::Hand);
        let nonartifact_spell_obj = game
            .object(nonartifact_spell_id)
            .expect("nonartifact spell in hand must exist")
            .clone();

        let prior_nonartifact = CardBuilder::new(CardId::from_raw(84), "Prior Nonartifact")
            .card_types(vec![CardType::Sorcery])
            .build();
        let prior_nonartifact_id =
            game.create_object_from_card(&prior_nonartifact, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_nonartifact_id)
                .expect("prior nonartifact must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_type(CardType::Artifact),
        );

        assert!(
            !can_cast_spell(&game, alice, &nonartifact_spell_obj, &CastingMethod::Normal),
            "second nonartifact spell in same turn should be blocked by nonartifact cast limit"
        );
    }

    #[test]
    fn test_can_cast_spell_nonartifact_limit_allows_artifact() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;

        let artifact_spell = CardBuilder::new(CardId::from_raw(85), "Restriction Relic")
            .card_types(vec![CardType::Artifact])
            .mana_cost(ManaCost::new())
            .build();
        let artifact_spell_id = game.create_object_from_card(&artifact_spell, alice, Zone::Hand);
        let artifact_spell_obj = game
            .object(artifact_spell_id)
            .expect("artifact spell in hand must exist")
            .clone();

        let prior_nonartifact = CardBuilder::new(CardId::from_raw(86), "Prior Nonartifact")
            .card_types(vec![CardType::Instant])
            .build();
        let prior_nonartifact_id =
            game.create_object_from_card(&prior_nonartifact, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_nonartifact_id)
                .expect("prior nonartifact must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_type(CardType::Artifact),
        );

        assert!(
            can_cast_spell(&game, alice, &artifact_spell_obj, &CastingMethod::Normal),
            "nonartifact cast limit should still allow artifact spell"
        );
    }

    #[test]
    fn test_can_cast_spell_respects_nonphyrexian_cast_limit() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let nonphyrexian_spell = CardBuilder::new(CardId::from_raw(87), "Restriction Spell")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::new())
            .subtypes(vec![Subtype::Elf])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let nonphyrexian_spell_id =
            game.create_object_from_card(&nonphyrexian_spell, alice, Zone::Hand);
        let nonphyrexian_spell_obj = game
            .object(nonphyrexian_spell_id)
            .expect("non-Phyrexian spell in hand must exist")
            .clone();

        let prior_nonphyrexian = CardBuilder::new(CardId::from_raw(88), "Prior Nonphyrexian")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let prior_nonphyrexian_id =
            game.create_object_from_card(&prior_nonphyrexian, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_nonphyrexian_id)
                .expect("prior non-Phyrexian must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_subtype(Subtype::Phyrexian),
        );

        assert!(
            !can_cast_spell(
                &game,
                alice,
                &nonphyrexian_spell_obj,
                &CastingMethod::Normal
            ),
            "second non-Phyrexian spell in same turn should be blocked by non-Phyrexian cast limit"
        );
    }

    #[test]
    fn test_can_cast_spell_nonphyrexian_limit_allows_phyrexian() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;

        let phyrexian_spell = CardBuilder::new(CardId::from_raw(89), "Restriction Horror")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::new())
            .subtypes(vec![Subtype::Phyrexian])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let phyrexian_spell_id = game.create_object_from_card(&phyrexian_spell, alice, Zone::Hand);
        let phyrexian_spell_obj = game
            .object(phyrexian_spell_id)
            .expect("Phyrexian spell in hand must exist")
            .clone();

        let prior_nonphyrexian = CardBuilder::new(CardId::from_raw(90), "Prior Nonphyrexian")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let prior_nonphyrexian_id =
            game.create_object_from_card(&prior_nonphyrexian, alice, Zone::Graveyard);
        let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(prior_nonphyrexian_id)
                .expect("prior non-Phyrexian must exist"),
            &game,
        );
        stage_spell_cast_for_test(&mut game, prior_snapshot.object_id, alice, Zone::Hand);
        game.cant_effects.add_cast_limit_filter(
            alice,
            crate::target::ObjectFilter::default().without_subtype(Subtype::Phyrexian),
        );

        assert!(
            can_cast_spell(&game, alice, &phyrexian_spell_obj, &CastingMethod::Normal),
            "non-Phyrexian cast limit should still allow a Phyrexian spell"
        );
    }

    #[test]
    fn test_can_cast_spell_uses_conditional_spell_flash_threshold() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.turn.active_player = bob;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::BeginCombat);

        let sorcery = CardBuilder::new(CardId::from_raw(1200), "Threshold Flash Sorcery")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::new())
            .build();
        let spell_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);
        let spec = crate::static_abilities::ConditionalSpellKeywordSpec {
            keyword: crate::static_abilities::ConditionalSpellKeywordKind::Flash,
            metric: crate::static_abilities::GraveyardCountMetric::ManaValues,
            threshold: 5,
        };
        game.object_mut(spell_id)
            .expect("spell should exist")
            .abilities
            .push(
                Ability::static_ability(StaticAbility::conditional_spell_keyword(spec))
                    .in_zones(vec![Zone::Hand, Zone::Stack]),
            );

        for (idx, mv) in [1u8, 2, 3, 4].into_iter().enumerate() {
            let card = CardBuilder::new(
                CardId::from_raw(1300 + idx as u32),
                &format!("MV{mv} Graveyard Card"),
            )
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(mv)]]))
            .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }

        let spell_obj = game.object(spell_id).expect("spell should exist").clone();
        assert!(
            !can_cast_spell(&game, alice, &spell_obj, &CastingMethod::Normal),
            "sorcery should remain sorcery-speed before mana-value threshold is met"
        );

        let fifth = CardBuilder::new(CardId::from_raw(1399), "MV5 Graveyard Card")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
            .build();
        game.create_object_from_card(&fifth, alice, Zone::Graveyard);

        let spell_obj = game.object(spell_id).expect("spell should exist").clone();
        assert!(
            can_cast_spell(&game, alice, &spell_obj, &CastingMethod::Normal),
            "conditional flash should allow casting once the mana-value threshold is met"
        );
    }

    #[test]
    fn test_can_cast_spell_with_non_targeted_prevent_all_damage_without_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;

        let definition =
            crate::cards::CardDefinitionBuilder::new(CardId::from_raw(13000), "Sivvi Cast Probe")
                .card_types(vec![CardType::Instant])
                .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
                .parse_text(
                    "Prevent all damage that would be dealt this turn to creatures you control.",
                )
                .expect("prevent-all damage line should parse as a non-targeted effect");

        let spell_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
        game.player_mut(alice)
            .expect("player should exist")
            .mana_pool
            .add(ManaSymbol::White, 1);

        let spell_obj = game.object(spell_id).expect("spell should exist").clone();
        assert!(
            can_cast_spell(&game, alice, &spell_obj, &CastingMethod::Normal),
            "spell should be castable without creatures because effect is non-targeted"
        );
    }

    #[test]
    fn test_compute_legal_targets_respects_cant_target_player_restriction() {
        use crate::target::{ChooseSpec, PlayerFilter};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.cant_effects.cant_target_players.insert(bob);
        let targets = crate::game_loop::compute_legal_targets(
            &game,
            &ChooseSpec::Player(PlayerFilter::Any),
            alice,
            None,
        );

        assert!(
            !targets.contains(&crate::Target::Player(bob)),
            "untargetable player should not appear in legal target set: {targets:?}"
        );
        assert!(
            targets.contains(&crate::Target::Player(alice)),
            "other legal players should remain targetable: {targets:?}"
        );
    }

    #[test]
    fn test_auto_pass_decision_maker() {
        use crate::decisions::context::PriorityContext;

        let game = setup_game();
        let mut dm = AutoPassDecisionMaker;

        let ctx = PriorityContext::new(PlayerId::from_index(0), vec![LegalAction::PassPriority]);

        let response = dm.decide_priority(&game, &ctx);
        assert!(matches!(response, LegalAction::PassPriority));
    }

    #[test]
    fn test_numeric_input_decision_maker() {
        use crate::decisions::context::PriorityContext;

        let game = setup_game();

        // Test priority decisions with numeric input
        let mut dm = NumericInputDecisionMaker::from_strs(&["0", "1", ""]);

        let legal_actions = vec![
            LegalAction::PassPriority,
            LegalAction::PlayLand {
                land_id: ObjectId::from_raw(1),
            },
        ];

        let ctx = PriorityContext::new(PlayerId::from_index(0), legal_actions.clone());

        // "0" should select PassPriority
        assert!(matches!(
            dm.decide_priority(&game, &ctx),
            LegalAction::PassPriority
        ));

        // "1" should select PlayLand
        let ctx2 = PriorityContext::new(PlayerId::from_index(0), legal_actions.clone());
        assert!(matches!(
            dm.decide_priority(&game, &ctx2),
            LegalAction::PlayLand { .. }
        ));

        // "" (empty) should default to PassPriority
        let ctx3 = PriorityContext::new(PlayerId::from_index(0), legal_actions);
        assert!(matches!(
            dm.decide_priority(&game, &ctx3),
            LegalAction::PassPriority
        ));
    }

    #[test]
    fn test_numeric_input_priority_commander_shortcut_single() {
        use crate::alternative_cast::CastingMethod;
        use crate::decisions::context::PriorityContext;
        use crate::zone::Zone;

        let game = setup_game();
        let mut dm = NumericInputDecisionMaker::from_strs(&["c"]);

        let actions = vec![
            LegalAction::PassPriority,
            LegalAction::CastSpell {
                spell_id: ObjectId::from_raw(100),
                from_zone: Zone::Command,
                casting_method: CastingMethod::Normal,
            },
        ];

        let ctx = PriorityContext::new(PlayerId::from_index(0), actions);
        assert!(matches!(
            dm.decide_priority(&game, &ctx),
            LegalAction::CastSpell {
                from_zone: Zone::Command,
                ..
            }
        ));
    }

    #[test]
    fn test_numeric_input_priority_commander_shortcut_indexed() {
        use crate::alternative_cast::CastingMethod;
        use crate::decisions::context::PriorityContext;
        use crate::zone::Zone;

        let game = setup_game();
        let mut dm = NumericInputDecisionMaker::from_strs(&["c1"]);

        let actions = vec![
            LegalAction::PassPriority,
            LegalAction::CastSpell {
                spell_id: ObjectId::from_raw(101),
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            },
            LegalAction::CastSpell {
                spell_id: ObjectId::from_raw(102),
                from_zone: Zone::Command,
                casting_method: CastingMethod::Normal,
            },
            LegalAction::CastSpell {
                spell_id: ObjectId::from_raw(103),
                from_zone: Zone::Command,
                casting_method: CastingMethod::Normal,
            },
        ];

        let ctx = PriorityContext::new(PlayerId::from_index(0), actions);
        assert!(matches!(
            dm.decide_priority(&game, &ctx),
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Command,
                ..
            } if spell_id == ObjectId::from_raw(103)
        ));
    }

    #[test]
    fn test_commander_tax_applies_to_recasts_from_command_zone() {
        use crate::ability::Ability;
        use crate::cost::TotalCost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let commander = CardBuilder::new(CardId::from_raw(2000), "Test Commander")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let commander_id = game.create_object_from_card(&commander, alice, Zone::Command);
        game.set_as_commander(commander_id, alice);

        for idx in 0..2 {
            let land = CardBuilder::new(
                CardId::from_raw(2100 + idx),
                &format!("Green Source {}", idx),
            )
            .card_types(vec![CardType::Land])
            .build();
            let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
            game.object_mut(land_id)
                .expect("green source should exist")
                .abilities
                .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
        }

        let commander_obj = game
            .object(commander_id)
            .expect("commander should remain in command zone")
            .clone();
        assert!(
            can_cast_spell(&game, alice, &commander_obj, &CastingMethod::Normal),
            "initial cast should be affordable with two mana"
        );

        game.record_commander_cast_from_command_zone(commander_id);
        let commander_obj = game
            .object(commander_id)
            .expect("commander should remain in command zone")
            .clone();
        assert!(
            !can_cast_spell(&game, alice, &commander_obj, &CastingMethod::Normal),
            "recast should require commander tax"
        );

        for idx in 0..2 {
            let land = CardBuilder::new(
                CardId::from_raw(2200 + idx),
                &format!("Extra Green Source {}", idx),
            )
            .card_types(vec![CardType::Land])
            .build();
            let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
            game.object_mut(land_id)
                .expect("extra green source should exist")
                .abilities
                .push(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]));
        }

        let commander_obj = game
            .object(commander_id)
            .expect("commander should remain in command zone")
            .clone();
        assert!(
            can_cast_spell(&game, alice, &commander_obj, &CastingMethod::Normal),
            "four mana should pay the taxed commander cost"
        );
    }

    #[test]
    fn test_numeric_input_may_choice() {
        use crate::decisions::context::BooleanContext;

        let game = setup_game();
        let mut dm = NumericInputDecisionMaker::from_strs(&["y", "n", "", "1"]);

        let ctx = BooleanContext {
            player: PlayerId::from_index(0),
            source: Some(ObjectId::from_raw(1)),
            description: "Test?".to_string(),
            source_name: None,
            ui_hints: crate::decisions::context::DecisionUiHints::default(),
        };

        // "y" = true
        assert!(dm.decide_boolean(&game, &ctx));

        // "n" = false
        assert!(!dm.decide_boolean(&game, &ctx));

        // "" = false
        assert!(!dm.decide_boolean(&game, &ctx));

        // "1" = true
        assert!(dm.decide_boolean(&game, &ctx));
    }

    /// Tests that tapped creatures cannot activate mana abilities with tap costs.
    ///
    /// Scenario: Alice controls an untapped Llanowar Elves (which has "{T}: Add {G}").
    /// When untapped, she should be able to activate the mana ability. After tapping it,
    /// she should no longer be able to activate the ability.
    #[test]
    fn test_activated_ability_tap_cost_validation() {
        use crate::cards::definitions::llanowar_elves;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase (for priority)
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Create Llanowar Elves on battlefield (has {T}: Add {G} - a mana ability)
        let elves_def = llanowar_elves();
        let creature_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);

        // Remove summoning sickness so it can tap
        game.remove_summoning_sickness(creature_id);

        // Check legal actions - should include the mana ability
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Should be able to activate untapped creature's tap mana ability"
        );

        // Now tap the creature (simulating it was already tapped for mana earlier)
        game.tap(creature_id);

        // Check legal actions again - should NOT include the mana ability
        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Should NOT be able to activate already-tapped creature's tap mana ability"
        );
    }

    #[test]
    fn test_activated_ability_mana_cost_validation() {
        use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::effect::Effect;
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Create a creature with an activated ability that costs {1}{G}
        let creature = CardBuilder::new(CardId::from_raw(1), "Pump Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);

        // Add an activated ability: {1}{G}: +2/+2 until EOT
        let mana_cost =
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)], vec![ManaSymbol::Green]]);
        let activated_ability = Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(mana_cost),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::pump(
                    2,
                    2,
                    crate::target::ChooseSpec::Source,
                    crate::effect::Until::EndOfTurn,
                )]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
            text: None,
        };
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(activated_ability);
        game.remove_summoning_sickness(creature_id);

        // Cost payment is validated during the activation flow, so the action
        // should still surface even before the player floats mana.
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)),
            "Should surface the activation even before mana is available"
        );

        // Add mana to pool
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        // Now should be able to activate
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)),
            "Should be able to activate with sufficient mana"
        );
    }

    #[test]
    fn test_tayam_wall_of_roots_activation_uses_mana_sequence_solver() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::{tayam_luminous_enigma, wall_of_roots};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let tayam_id =
            game.create_object_from_definition(&tayam_luminous_enigma(), alice, Zone::Battlefield);
        let wall_id =
            game.create_object_from_definition(&wall_of_roots(), alice, Zone::Battlefield);

        if let Some(wall) = game.object_mut(wall_id) {
            wall.counters.insert(CounterType::MinusOneMinusOne, 2);
        }

        // Start with only 2 mana and 2 counters; activation should still be legal
        // because Wall of Roots can be activated during cost payment.
        if let Some(player) = game.player_mut(alice) {
            player.mana_pool.add(ManaSymbol::Colorless, 2);
        }

        let tayam_ability_index = game
            .object(tayam_id)
            .expect("Tayam should exist")
            .abilities
            .iter()
            .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .expect("Tayam should have an activated ability");

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index }
                    if *source == tayam_id && *ability_index == tayam_ability_index
            )),
            "Tayam activation should be legal when Wall of Roots can provide the 3rd mana and 3rd counter during payment"
        );
    }

    #[test]
    fn test_tayam_wall_of_roots_activation_blocked_when_wall_already_used() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::{tayam_luminous_enigma, wall_of_roots};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let tayam_id =
            game.create_object_from_definition(&tayam_luminous_enigma(), alice, Zone::Battlefield);
        let wall_id =
            game.create_object_from_definition(&wall_of_roots(), alice, Zone::Battlefield);

        if let Some(wall) = game.object_mut(wall_id) {
            wall.counters.insert(CounterType::MinusOneMinusOne, 2);
        }

        if let Some(player) = game.player_mut(alice) {
            player.mana_pool.add(ManaSymbol::Colorless, 2);
        }

        let wall_mana_ability_index = game
            .object(wall_id)
            .expect("Wall of Roots should exist")
            .abilities
            .iter()
            .position(|ability| ability.is_mana_ability())
            .expect("Wall of Roots should have a mana ability");
        game.record_ability_activation(wall_id, wall_mana_ability_index);

        let tayam_ability_index = game
            .object(tayam_id)
            .expect("Tayam should exist")
            .abilities
            .iter()
            .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .expect("Tayam should have an activated ability");

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index }
                    if *source == tayam_id && *ability_index == tayam_ability_index
            )),
            "Tayam activation should still surface even when the payment flow will reject it"
        );
    }

    #[test]
    fn test_activated_ability_cost_reduction_respects_minimum_one_mana() {
        use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::effect::Effect;
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::static_abilities::StaticAbility;
        use crate::target::ObjectFilter;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Creature with two activated abilities: one costs {2}, one costs {1}.
        let creature = CardBuilder::new(CardId::from_raw(11), "Reducer Target")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.remove_summoning_sickness(creature_id);

        let cost_two = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]);
        let cost_one = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]);
        let activated = |cost: ManaCost| Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(cost),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
            text: None,
        };
        game.object_mut(creature_id)
            .expect("creature exists")
            .abilities
            .extend([activated(cost_two), activated(cost_one)]);

        // Training Grounds-style static ability.
        let reducer = CardBuilder::new(CardId::from_raw(12), "Training Grounds Effect")
            .card_types(vec![CardType::Enchantment])
            .build();
        let reducer_id = game.create_object_from_card(&reducer, alice, Zone::Battlefield);
        game.object_mut(reducer_id)
            .expect("reducer exists")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::reduce_activated_ability_costs(
                    ObjectFilter::creature().you_control(),
                    2,
                    Some(1),
                ),
            ));

        let actions_without_mana = compute_legal_actions(&game, alice);
        assert!(
            actions_without_mana.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility {
                    source,
                    ability_index: 0
                } if *source == creature_id
            )),
            "reduced activated abilities should still surface before mana is floated"
        );

        game.player_mut(alice)
            .expect("player exists")
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        let actions_with_one = compute_legal_actions(&game, alice);
        assert!(
            actions_with_one.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility {
                    source,
                    ability_index: 0
                } if *source == creature_id
            )),
            "with one mana, {{2}} ability should be reduced to {{1}}"
        );
        assert!(
            actions_with_one.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility {
                    source,
                    ability_index: 1
                } if *source == creature_id
            )),
            "minimum-one-mana floor should keep {{1}} ability at {{1}}"
        );

        let reduced_one = calculate_effective_activation_total_cost(
            &game,
            alice,
            creature_id,
            &TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]])),
        );
        assert_eq!(
            reduced_one
                .mana_cost()
                .expect("reduced cost keeps mana component")
                .generic_mana_total(),
            1,
            "minimum-one-mana floor should not reduce a {{1}} activation cost to zero"
        );
    }

    #[test]
    fn test_self_hand_activated_ability_cost_reduction_counts_matching_battlefield_objects() {
        use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::effect::Effect;
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::static_abilities::StaticAbility;
        use crate::target::ObjectFilter;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let legend = CardBuilder::new(CardId::from_raw(21), "Legendary Scout")
            .supertypes(vec![crate::types::Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&legend, alice, Zone::Battlefield);
        game.create_object_from_card(&legend, alice, Zone::Battlefield);

        let card = CardBuilder::new(CardId::from_raw(22), "Hand Reducer")
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_card(&card, alice, Zone::Hand);
        game.object_mut(source_id)
            .expect("source exists")
            .abilities
            .extend([
                Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost: TotalCost::mana(ManaCost::from_pips(vec![vec![
                            ManaSymbol::Generic(3),
                        ]])),
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![
                            Effect::draw(1),
                        ]),
                        choices: vec![],
                        timing: ActivationTiming::AnyTime,
                        additional_restrictions: vec![],
                        activation_restrictions: vec![],
                        mana_output: None,
                        activation_condition: None,
                        mana_usage_restrictions: vec![],
                    }),
                    functional_zones: vec![Zone::Hand],
                    text: Some("Hand ability".to_string()),
                },
                Ability::static_ability(StaticAbility::reduce_activated_ability_costs_for_each(
                    ObjectFilter::source(),
                    1,
                    ObjectFilter::creature()
                        .you_control()
                        .with_supertype(crate::types::Supertype::Legendary),
                    Some(1),
                ))
                .in_zones(vec![Zone::Battlefield, Zone::Hand]),
            ]);

        let reduced = calculate_effective_activation_mana_cost(
            &game,
            alice,
            source_id,
            &ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]),
        );
        assert_eq!(
            reduced.generic_mana_total(),
            1,
            "two matching legendary creatures should reduce a hand-zone activation from {{3}} to {{1}}"
        );
    }

    /// Tests that summoning sick creatures cannot activate mana abilities with tap costs.
    ///
    /// Scenario: Alice casts Llanowar Elves. On the same turn, the creature has
    /// summoning sickness, so she should not be able to activate its "{T}: Add {G}"
    /// mana ability.
    #[test]
    fn test_activated_ability_summoning_sickness_blocks_tap() {
        use crate::cards::definitions::llanowar_elves;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Create Llanowar Elves on battlefield with summoning sickness
        let elves_def = llanowar_elves();
        let creature_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);

        // Creature just entered battlefield, so it has summoning sickness
        game.set_summoning_sick(creature_id);

        // Should NOT be able to activate tap mana ability due to summoning sickness
        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Summoning sick creature should not be able to use tap mana abilities"
        );
    }

    /// Tests that creatures with haste can use tap mana abilities despite summoning sickness.
    ///
    /// Scenario: Alice has given her Llanowar Elves haste (e.g., via an effect like
    /// Swiftfoot Boots). Even though the creature just entered the battlefield and
    /// has summoning sickness, haste allows it to activate its "{T}: Add {G}" mana ability.
    #[test]
    fn test_activated_ability_haste_bypasses_summoning_sickness() {
        use crate::ability::Ability;
        use crate::cards::definitions::llanowar_elves;
        use crate::static_abilities::StaticAbility;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Create Llanowar Elves with summoning sickness but also with haste
        let elves_def = llanowar_elves();
        let creature_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);

        // Add haste (e.g., from equipment or an enchantment)
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability::static_ability(StaticAbility::haste()));

        // Creature just entered battlefield, so it has summoning sickness
        game.set_summoning_sick(creature_id);

        // Should be able to activate tap mana ability despite summoning sickness (has haste)
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == creature_id)),
            "Creature with haste should be able to use tap mana abilities despite summoning sickness"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_turn_face_up_for_morph() {
        use crate::ability::Ability;
        use crate::static_abilities::StaticAbility;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        let creature = CardBuilder::new(CardId::from_raw(101), "Morph Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability::static_ability(StaticAbility::morph(
                crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Green]]),
            )));
        game.set_face_down(creature_id);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Green, 1);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(
                |a| matches!(a, LegalAction::TurnFaceUp { creature_id: id, .. } if *id == creature_id)
            ),
            "face-down creature with payable morph cost should have TurnFaceUp legal action"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_face_down_cast_for_morph_when_normal_cast_is_too_expensive()
     {
        use crate::ability::Ability;
        use crate::static_abilities::StaticAbility;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        let creature = CardBuilder::new(CardId::from_raw(102), "Costly Morph Bear")
            .mana_cost(crate::mana::ManaCost::from_pips(vec![
                vec![crate::mana::ManaSymbol::Generic(5)],
                vec![crate::mana::ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability::static_ability(StaticAbility::morph(
                crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Green]]),
            )));
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Colorless, 3);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::FaceDown,
                } if *spell_id == creature_id
            )),
            "morph card should be castable face down when {{3}} is payable"
        );
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == creature_id
            )),
            "normal cast should stay unavailable when the printed mana cost is too expensive"
        );
    }

    #[test]
    fn test_activated_ability_sorcery_speed_timing() {
        use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::effect::Effect;
        use crate::game_state::Step;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature with sorcery-speed activated ability
        let creature = CardBuilder::new(CardId::from_raw(1), "Sorcery Speed Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);

        // Add sorcery-speed activated ability (no cost, just free)
        let activated_ability = Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::gain_life(1),
                ]),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
            text: None,
        };
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(activated_ability);
        game.remove_summoning_sickness(creature_id);

        // Main phase, empty stack - should be able to activate
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)),
            "Should be able to activate sorcery-speed ability during main phase with empty stack"
        );

        // Combat phase - should NOT be able to activate
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)),
            "Should NOT be able to activate sorcery-speed ability during combat"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_hand_activated_ability() {
        use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::effect::Effect;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        let card = CardBuilder::new(CardId::from_raw(777), "Hand Ability Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let source_id = game.create_object_from_card(&card, alice, Zone::Hand);
        game.object_mut(source_id)
            .expect("source card should exist")
            .abilities
            .push(Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: TotalCost::free(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::gain_life(1),
                    ]),
                    choices: vec![],
                    timing: ActivationTiming::AnyTime,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones: vec![Zone::Hand],
                text: Some("Hand ability".to_string()),
            });

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(
                |a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == source_id)
            ),
            "hand-zone activated ability should be discoverable as a legal action"
        );
    }

    #[test]
    fn test_compute_legal_actions_excludes_hand_only_ability_from_battlefield() {
        use crate::cards::CardDefinitionBuilder;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let def = CardDefinitionBuilder::new(CardId::from_raw(779), "Boseiju Regression Probe")
            .card_types(vec![CardType::Land])
            .mana_cost(ManaCost::new())
            .parse_text(
                "{T}: Add {G}.\nChannel — {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls.\nThis ability costs {1} less to activate for each legendary creature you control.",
            )
            .expect("channel land probe should parse");

        let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(source_id);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateManaAbility { source, .. } if *source == source_id
            )),
            "battlefield Boseiju should still expose its tap-for-mana ability"
        );
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == source_id
            )),
            "battlefield Boseiju should not expose its hand-only channel ability"
        );
    }

    #[test]
    fn test_tap_only_activation_skips_empty_next_cost_prompt() {
        use crate::cost::TotalCost;
        use crate::costs::Cost;
        use crate::game_loop::{
            PriorityLoopState, PriorityResponse, apply_priority_response_with_dm,
        };
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::from_raw(780), "Tap Probe")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(source_id)
            .expect("tap probe should exist")
            .abilities
            .push(Ability::activated_with_costs(
                TotalCost::free(),
                vec![Cost::tap()],
                vec![Effect::gain_life(1)],
            ));

        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = AutoPassDecisionMaker;

        let progress = apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
                source: source_id,
                ability_index: 0,
            }),
            &mut dm,
        )
        .expect("tap-only activation should resolve its cost flow");

        assert!(
            state.pending_activation.is_none(),
            "tap-only activation should not get stuck in a pending cost prompt"
        );
        assert!(
            game.is_tapped(source_id),
            "tap-only activation should pay the tap cost immediately"
        );
        assert_eq!(
            game.stack.len(),
            1,
            "tap-only activation should place the ability on the stack"
        );
        assert!(
            matches!(
                progress,
                crate::decision::GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Priority(_)
                )
            ),
            "after a tap-only activation resolves its cost, priority should continue normally"
        );
    }

    #[test]
    fn test_compute_legal_actions_excludes_tapped_non_mana_tap_ability() {
        use crate::cost::TotalCost;
        use crate::costs::Cost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::from_raw(781), "Tapped Ability Probe")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(source_id)
            .expect("probe should exist")
            .abilities
            .push(Ability::activated_with_costs(
                TotalCost::free(),
                vec![Cost::tap()],
                vec![Effect::gain_life(1)],
            ));
        game.tap(source_id);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == source_id
            )),
            "tapped permanents should not expose non-mana tap abilities as legal actions"
        );
    }

    #[test]
    fn test_compute_legal_actions_excludes_summoning_sick_non_mana_untap_ability() {
        use crate::cost::TotalCost;
        use crate::costs::Cost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::from_raw(782), "Untap Ability Probe")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .build(),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(source_id)
            .expect("probe should exist")
            .abilities
            .push(Ability::activated_with_costs(
                TotalCost::free(),
                vec![Cost::untap()],
                vec![Effect::gain_life(1)],
            ));
        game.tap(source_id);
        game.set_summoning_sick(source_id);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == source_id
            )),
            "summoning-sick creatures should not expose non-mana untap abilities as legal actions"
        );

        game.remove_summoning_sickness(source_id);
        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == source_id
            )),
            "once summoning sickness is removed, the untap ability should become legal"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_foretell_special_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(778), "Foretell Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![CardType::Instant])
            .with_spell_effect(vec![Effect::gain_life(1)])
            .foretell(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Blue],
            ]))
            .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::SpecialAction(crate::special_actions::SpecialAction::Foretell {
                    card_id: found
                }) if *found == card_id
            )),
            "expected foretell special action in legal actions, got {actions:?}"
        );
    }

    #[test]
    fn test_compute_legal_actions_includes_suspend_special_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Green, 1);

        let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(779), "Suspend Probe")
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::gain_life(1)])
            .suspend(2, ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
            .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::SpecialAction(crate::special_actions::SpecialAction::Suspend {
                    card_id: found
                }) if *found == card_id
            )),
            "expected suspend special action in legal actions, got {actions:?}"
        );
    }

    #[test]
    fn test_suspend_special_action_respects_cant_cast_restrictions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Green, 1);

        let def = crate::cards::CardDefinitionBuilder::new(
            CardId::from_raw(7791),
            "Suspend Restriction Probe",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .suspend(2, ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        game.cant_effects.add_cant_cast_filter(
            alice,
            crate::target::ObjectFilter::default().with_type(CardType::Creature),
        );

        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::SpecialAction(crate::special_actions::SpecialAction::Suspend {
                    card_id: found
                }) if *found == card_id
            )),
            "suspend should not be offered when a cast prohibition would stop starting the cast, got {actions:?}"
        );
    }

    #[test]
    fn test_suspend_only_card_does_not_offer_normal_cast_from_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(782), "Lotus Bloom")
            .parse_text(
                "Type: Artifact\n\
                 Suspend 3—{0} (Rather than cast this card from your hand, pay {0} and exile it with three time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)\n\
                 {T}, Sacrifice this artifact: Add three mana of any one color.",
            )
            .expect("Lotus Bloom text should parse");
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == card_id
            )),
            "suspend-only card should not offer a normal cast action, got {actions:?}"
        );
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::SpecialAction(crate::special_actions::SpecialAction::Suspend {
                    card_id: found
                }) if *found == card_id
            )),
            "suspend-only card should still offer suspend, got {actions:?}"
        );
    }

    #[test]
    fn test_suspend_special_action_exiles_with_time_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Green, 1);

        let def =
            crate::cards::CardDefinitionBuilder::new(CardId::from_raw(781), "Suspend Runtime")
                .card_types(vec![CardType::Sorcery])
                .with_spell_effect(vec![Effect::gain_life(1)])
                .suspend(2, ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
                .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let mut dm = crate::decision::SelectFirstDecisionMaker;
        crate::special_actions::perform(
            crate::special_actions::SpecialAction::Suspend { card_id },
            &mut game,
            alice,
            &mut dm,
        )
        .expect("suspend special action should resolve");

        let exiled_id = *game.exile.first().expect("card should be exiled");
        let exiled = game.object(exiled_id).expect("exiled card should exist");
        assert_eq!(exiled.zone, Zone::Exile);
        assert_eq!(
            game.counter_count(exiled_id, crate::object::CounterType::Time),
            2
        );
    }

    #[test]
    fn test_plot_special_action_enables_cast_on_later_turn_only() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Red, 3);

        let def = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(780), "Plot Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::gain_life(1)])
            .plot(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Red],
            ]))
            .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let mut dm = crate::decision::SelectFirstDecisionMaker;
        crate::special_actions::perform(
            crate::special_actions::SpecialAction::Plot { card_id },
            &mut game,
            alice,
            &mut dm,
        )
        .expect("plot special action should resolve");

        let exiled_id = *game.exile.first().expect("card should be in exile");
        let same_turn_actions = compute_legal_actions(&game, alice);
        assert!(
            !same_turn_actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == exiled_id
            )),
            "plotted card should not be castable the same turn it was plotted"
        );

        game.next_turn();
        game.next_turn();
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let later_actions = compute_legal_actions(&game, alice);
        assert!(
            later_actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == exiled_id
            )),
            "plotted card should be castable from exile on a later turn"
        );
    }

    #[test]
    fn test_spectacle_condition_controls_alternative_cast_legality() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Red, 1);

        let def =
            crate::cards::CardDefinitionBuilder::new(CardId::from_raw(782), "Spectacle Probe")
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(2)],
                    vec![ManaSymbol::Red],
                ]))
                .card_types(vec![CardType::Sorcery])
                .with_spell_effect(vec![Effect::gain_life(1)])
                .spectacle(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
                .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);
        let card = game.object(card_id).expect("spectacle card should exist");
        assert!(
            !can_cast_with_alternative_from_hand(
                &game,
                alice,
                card,
                card_id,
                &card.alternative_casts[0]
            ),
            "spectacle alternative should not be available before an opponent loses life"
        );

        stage_life_loss_for_test(&mut game, bob, 1);
        let card = game
            .object(card_id)
            .expect("spectacle card should still exist");
        assert!(
            can_cast_with_alternative_from_hand(
                &game,
                alice,
                card,
                card_id,
                &card.alternative_casts[0]
            ),
            "spectacle alternative should become available once an opponent has lost life"
        );
    }

    #[test]
    fn test_foretell_special_action_enables_cast_from_exile() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Blue, 4);

        let def = crate::cards::CardDefinitionBuilder::new(
            CardId::from_raw(779),
            "Foretell Runtime Probe",
        )
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::gain_life(1)])
        .foretell(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
        let card_id = game.create_object_from_definition(&def, alice, Zone::Hand);

        let mut dm = SelectFirstDecisionMaker;
        crate::special_actions::perform(
            crate::special_actions::SpecialAction::Foretell { card_id },
            &mut game,
            alice,
            &mut dm,
        )
        .expect("foretell special action should succeed");

        let foretold_id = *game.exile.last().expect("card should be in exile");
        assert!(game.is_face_down(foretold_id));
        assert!(game.is_foretold(foretold_id));

        let actions = compute_legal_actions(&game, alice);
        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == foretold_id
            )),
            "expected foretold card to be castable from exile, got {actions:?}"
        );
    }

    /// Tests that compute_potential_mana correctly calculates mana from untapped sources.
    ///
    /// Scenario: Player has empty mana pool but 4 untapped Mountains on battlefield.
    /// compute_potential_mana should return a pool with 4 red mana.
    #[test]
    fn test_compute_potential_mana_with_untapped_lands() {
        use crate::cards::definitions::basic_mountain;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Verify mana pool is empty
        assert_eq!(
            game.player(alice).unwrap().mana_pool.total(),
            0,
            "Mana pool should start empty"
        );

        // Create 4 Mountains on battlefield
        let mountain_def = basic_mountain();
        for _ in 0..4 {
            game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);
        }

        // compute_potential_mana should include mana from untapped lands
        let potential = compute_potential_mana(&game, alice);
        assert_eq!(
            potential.red, 4,
            "Should have 4 potential red mana from Mountains"
        );
        assert_eq!(potential.total(), 4, "Total potential mana should be 4");
    }

    /// Tests that max_x_for_cost works correctly with potential mana.
    ///
    /// Scenario: Player has empty mana pool but 4 untapped Mountains.
    /// For a Fireball ({X}{R}), max X should be 3 (4 total mana - 1 for {R} = 3 for X).
    #[test]
    fn test_max_x_with_potential_mana() {
        use crate::cards::definitions::basic_mountain;
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Verify mana pool is empty
        assert_eq!(
            game.player(alice).unwrap().mana_pool.total(),
            0,
            "Mana pool should start empty"
        );

        // Create 4 Mountains on battlefield
        let mountain_def = basic_mountain();
        for _ in 0..4 {
            game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);
        }

        // Fireball cost: {X}{R}
        let fireball_cost = ManaCost::from_pips(vec![vec![ManaSymbol::X], vec![ManaSymbol::Red]]);

        // Using just the mana pool (which is empty), max_x would be 0
        let max_x_from_pool = game
            .player(alice)
            .unwrap()
            .mana_pool
            .max_x_for_cost(&fireball_cost);
        assert_eq!(max_x_from_pool, 0, "max_x from empty pool should be 0");

        // Using potential mana (including untapped lands), max_x should be 3
        let potential = compute_potential_mana(&game, alice);
        let max_x_from_potential = potential.max_x_for_cost(&fireball_cost);
        assert_eq!(
            max_x_from_potential, 3,
            "max_x from potential mana should be 3 (4 mana - 1 for R = 3 for X)"
        );
    }

    /// Tests that potential mana includes mana dorks (creatures with mana abilities).
    ///
    /// Scenario: Player has 1 Mountain and 1 Llanowar Elves (untapped, no summoning sickness).
    /// For Fireball ({X}{R}), max X should be 1 (2 total mana - 1 for {R} = 1 for X).
    #[test]
    fn test_max_x_with_mana_dork() {
        use crate::cards::definitions::{basic_mountain, llanowar_elves};
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Create Mountain and Llanowar Elves
        let mountain_def = basic_mountain();
        game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);

        let elves_def = llanowar_elves();
        let elves_id = game.create_object_from_definition(&elves_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(elves_id);

        // Fireball cost: {X}{R}
        let fireball_cost = ManaCost::from_pips(vec![vec![ManaSymbol::X], vec![ManaSymbol::Red]]);

        // Potential mana: 1R from Mountain + 1G from Elves = 2 total
        let potential = compute_potential_mana(&game, alice);
        assert_eq!(potential.red, 1, "Should have 1 potential red mana");
        assert_eq!(potential.green, 1, "Should have 1 potential green mana");
        assert_eq!(potential.total(), 2, "Total potential mana should be 2");

        // max_x should be 1: pay {R} with Mountain, {X}=1 with Elves' green mana
        let max_x = potential.max_x_for_cost(&fireball_cost);
        assert_eq!(max_x, 1, "max_x should be 1 (2 total - 1 for R = 1 for X)");
    }

    #[test]
    fn test_graveyard_play_from_actions_include_variable_mana_sources() {
        use crate::cards::definitions::lightning_bolt;
        use crate::cards::tokens::treasure_token_definition;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        // Treasure's mana ability is effect-backed ("any color"), so this specifically
        // verifies variable mana producers are considered in castability checks.
        let treasure = treasure_token_definition();
        game.create_object_from_definition(&treasure, alice, Zone::Battlefield);

        let bolt = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Graveyard);

        let source_id = game.new_object_id();
        game.grant_registry.grant_to_filter_until_end_of_turn(
            ObjectFilter::nonland(),
            Zone::Graveyard,
            alice,
            Grantable::play_from(),
            source_id,
            game.turn.turn_number,
        );

        let actions = compute_legal_actions(&game, alice);
        let can_cast_from_graveyard = actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::PlayFrom {
                        zone: Zone::Graveyard,
                        ..
                    },
                    ..
                } if *spell_id == bolt_id
            )
        });

        assert!(
            can_cast_from_graveyard,
            "variable mana sources should allow castability inference for play-from-graveyard actions"
        );
    }

    #[test]
    fn test_counter_unless_pays_spell_not_castable_without_stack_target() {
        use crate::cards::definitions::{basic_island, mana_tithe};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Give Alice the mana to cast Mana Tithe.
        let island = basic_island();
        game.create_object_from_definition(&island, alice, Zone::Battlefield);

        // Put Mana Tithe in hand and leave stack empty.
        let mana_tithe_def = mana_tithe();
        let mana_tithe_id = game.create_object_from_definition(&mana_tithe_def, alice, Zone::Hand);

        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == mana_tithe_id
            )
        });

        assert!(
            !can_cast,
            "counter-unless-pays spells must not be castable without a legal spell target on stack"
        );
    }

    #[test]
    fn test_conditional_counter_spell_not_castable_without_stack_target() {
        use crate::cards::definitions::basic_island;
        use crate::effect::Condition;
        use crate::game_state::StackEntry;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Give Alice the mana to cast the conditional counterspell.
        let island = basic_island();
        game.create_object_from_definition(&island, alice, Zone::Battlefield);

        // Corrupted Resolve-shaped payload:
        // "Counter target spell if its controller is poisoned."
        let card = CardBuilder::new(CardId::from_raw(91), "Corrupted Resolve Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build();
        let spell_id = game.create_object_from_card(&card, alice, Zone::Hand);
        game.object_mut(spell_id)
            .expect("spell exists")
            .spell_effect = Some(crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::conditional(
                Condition::TargetSpellControllerIsPoisoned,
                vec![Effect::counter(ChooseSpec::target_spell())],
                vec![],
            ),
        ]));

        // With no spell on stack, the counterspell must not be castable.
        let actions_without_stack = compute_legal_actions(&game, alice);
        let can_cast_without_stack = actions_without_stack.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )
        });
        assert!(
            !can_cast_without_stack,
            "conditional counterspell should not be castable without a legal spell target on stack"
        );

        // Add a dummy spell to the stack and verify the cast action appears.
        let dummy_spell = CardBuilder::new(CardId::from_raw(92), "Stack Dummy")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build();
        let dummy_id = game.create_object_from_card(&dummy_spell, bob, Zone::Stack);
        game.push_to_stack(StackEntry::new(dummy_id, bob));

        let actions_with_stack = compute_legal_actions(&game, alice);
        let can_cast_with_stack = actions_with_stack.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )
        });
        assert!(
            can_cast_with_stack,
            "conditional counterspell should be castable once a legal spell target exists on stack"
        );
    }

    #[test]
    fn test_if_effect_counter_spell_not_castable_without_stack_target() {
        use crate::cards::definitions::basic_island;
        use crate::game_state::StackEntry;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Give Alice the mana to cast the spell.
        let island = basic_island();
        game.create_object_from_definition(&island, alice, Zone::Battlefield);

        // "If you do, counter target spell." shape:
        // represented as IfEffect branching on a prior effect result.
        let card = CardBuilder::new(CardId::from_raw(93), "If Counter Variant")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build();
        let spell_id = game.create_object_from_card(&card, alice, Zone::Hand);
        game.object_mut(spell_id)
            .expect("spell exists")
            .spell_effect = Some(crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::if_then(
                crate::effect::EffectId(0),
                crate::effect::EffectPredicate::Happened,
                vec![Effect::counter(ChooseSpec::target_spell())],
            ),
        ]));

        // With no spell on stack, the spell must not be castable.
        let actions_without_stack = compute_legal_actions(&game, alice);
        let can_cast_without_stack = actions_without_stack.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )
        });
        assert!(
            !can_cast_without_stack,
            "if-effect counterspell should not be castable without a legal spell target on stack"
        );

        // Add a legal stack spell; cast action should appear.
        let dummy_spell = CardBuilder::new(CardId::from_raw(94), "If Stack Dummy")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build();
        let dummy_id = game.create_object_from_card(&dummy_spell, bob, Zone::Stack);
        game.push_to_stack(StackEntry::new(dummy_id, bob));

        let actions_with_stack = compute_legal_actions(&game, alice);
        let can_cast_with_stack = actions_with_stack.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )
        });
        assert!(
            can_cast_with_stack,
            "if-effect counterspell should be castable once a legal spell target exists on stack"
        );
    }
}

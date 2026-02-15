//! Player decision system for MTG.
//!
//! This module provides:
//! - `DecisionMaker` and typed decision contexts for player input
//! - `LegalAction` and related types for describing legal game actions
//! - Helper functions to compute legal actions

use crate::alternative_cast::CastingMethod;
use crate::combat_state::{AttackTarget, CombatState};
use crate::cost::can_pay_cost;
use crate::effects::helpers::resolve_value;
use crate::executor::ExecutionContext;
use crate::game_state::{GameState, Phase, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::special_actions::SpecialAction;
use crate::target::ChooseSpec;
use crate::zone::Zone;
use crate::{CounterType, ManaSymbol, Step};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, BufWriter, Write};

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

    /// Turn a face-down creature face up (e.g., morph/megamorph).
    TurnFaceUp { creature_id: ObjectId },

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
    pub label: &'static str,
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

/// Compute legal actions for a player who has priority.
///
/// This validates each potential action by testing it against the actual game rules.
/// Only actions that would succeed are included in the result.
pub fn compute_legal_actions(game: &GameState, player: PlayerId) -> Vec<LegalAction> {
    use crate::special_actions::{SpecialAction, can_perform_check};

    let mut actions = Vec::new();

    // Can always pass priority
    actions.push(LegalAction::PassPriority);

    // Check for lands that can be played - validate each using special_actions::can_perform_check
    if let Some(player_obj) = game.player(player) {
        for &card_id in player_obj.hand.clone().iter() {
            if let Some(card) = game.object(card_id)
                && card.is_land()
            {
                let action = SpecialAction::PlayLand { card_id };
                if can_perform_check(&action, game, player).is_ok() {
                    actions.push(LegalAction::PlayLand { land_id: card_id });
                }
            }
        }
    }

    // Check for spells that can be cast from hand
    if let Some(player_obj) = game.player(player) {
        for &card_id in player_obj.hand.clone().iter() {
            if let Some(card) = game.object(card_id)
                && can_cast_spell(game, player, card, &CastingMethod::Normal)
            {
                actions.push(LegalAction::CastSpell {
                    spell_id: card_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                });
            }
        }
    }

    // Check for spells that can be cast from graveyard (flashback, escape, jump-start)
    if let Some(player_obj) = game.player(player) {
        for &card_id in player_obj.graveyard.clone().iter() {
            if let Some(card) = game.object(card_id) {
                for (idx, alt_cast) in card.alternative_casts.iter().enumerate() {
                    if alt_cast.cast_from_zone() == Zone::Graveyard
                        && can_cast_with_alternative(game, player, card, alt_cast)
                    {
                        actions.push(LegalAction::CastSpell {
                            spell_id: card_id,
                            from_zone: Zone::Graveyard,
                            casting_method: CastingMethod::Alternative(idx),
                        });
                    }
                }
            }
        }
    }

    // Check for granted alternative casts from the registry (escape, flashback, etc.)
    // This handles both static ability grants (Underworld Breach) and effect grants (Snapcaster Mage)
    if let Some(player_obj) = game.player(player) {
        for &card_id in player_obj.graveyard.clone().iter() {
            if let Some(card) = game.object(card_id) {
                let granted_casts = game.grant_registry.granted_alternative_casts_for_card(
                    game,
                    card_id,
                    Zone::Graveyard,
                    player,
                );

                for grant in granted_casts {
                    let method = &grant.method;
                    let requirements = build_requirements_for_method(method);
                    let mana_cost = get_mana_cost_for_method(method, card);

                    if can_cast_with_cost(game, player, card, card_id, mana_cost, &requirements) {
                        let casting_method = match method {
                            crate::alternative_cast::AlternativeCastingMethod::Escape {
                                exile_count,
                                ..
                            } => CastingMethod::GrantedEscape {
                                source: grant.source_id,
                                exile_count: *exile_count,
                            },
                            crate::alternative_cast::AlternativeCastingMethod::Flashback {
                                ..
                            } => CastingMethod::GrantedFlashback,
                            _ => continue,
                        };

                        actions.push(LegalAction::CastSpell {
                            spell_id: card_id,
                            from_zone: Zone::Graveyard,
                            casting_method,
                        });
                    }
                }

                let play_from_grants = game.grant_registry.granted_play_from_for_card(
                    game,
                    card_id,
                    Zone::Graveyard,
                    player,
                );

                for grant in play_from_grants {
                    // PlayFrom (e.g., Yawgmoth's Will): can cast from zone as if from hand
                    // This allows both normal cost and alternative costs
                    use crate::types::CardType;
                    let from_zone = grant.zone;

                    let is_castable = card.has_card_type(CardType::Instant)
                        || card.has_card_type(CardType::Sorcery);
                    if is_castable
                        && let Some(mana_cost) = &card.mana_cost
                        && can_cast_with_cost(
                            game,
                            player,
                            card,
                            card_id,
                            Some(mana_cost),
                            &AdditionalCastRequirements::default(),
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
                            && can_cast_with_alternative_from_hand(
                                game, player, card, card_id, alt_cast,
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
                }
            }
        }
    }

    // Check for alternative casting methods from hand (e.g., Force of Will's alternative cost)
    // Only add these if the Normal method is NOT available for this spell.
    // When both are available, the game will prompt for method selection via ChooseCastingMethod.
    if let Some(player_obj) = game.player(player) {
        for &card_id in player_obj.hand.clone().iter() {
            if let Some(card) = game.object(card_id) {
                // Check if we already added a Normal cast for this spell
                let has_normal_cast = actions.iter().any(|a| {
                    matches!(
                        a,
                        LegalAction::CastSpell {
                            spell_id,
                            from_zone: Zone::Hand,
                            casting_method: CastingMethod::Normal,
                        } if *spell_id == card_id
                    )
                });

                // Only add alternative casts if Normal is not available
                if !has_normal_cast {
                    for (idx, alt_cast) in card.alternative_casts.iter().enumerate() {
                        if alt_cast.cast_from_zone() == Zone::Hand
                            && can_cast_with_alternative_from_hand(
                                game, player, card, card_id, alt_cast,
                            )
                        {
                            actions.push(LegalAction::CastSpell {
                                spell_id: card_id,
                                from_zone: Zone::Hand,
                                casting_method: CastingMethod::Alternative(idx),
                            });
                        }
                    }
                }
            }
        }
    }

    // Check for mana abilities on permanents - validate using special_actions::can_perform
    let battlefield = game.battlefield.clone();
    for perm_id in battlefield {
        if let Some(perm) = game.object(perm_id) {
            if perm.controller != player {
                continue;
            }

            if game.is_face_down(perm_id) {
                let action = SpecialAction::TurnFaceUp {
                    permanent_id: perm_id,
                };
                if can_perform_check(&action, game, player).is_ok() {
                    actions.push(LegalAction::TurnFaceUp {
                        creature_id: perm_id,
                    });
                }
            }

            for (i, ability) in perm.abilities.iter().enumerate() {
                // Check mana abilities
                if matches!(ability.kind, crate::ability::AbilityKind::Mana(_)) {
                    let action = SpecialAction::ActivateManaAbility {
                        permanent_id: perm_id,
                        ability_index: i,
                    };
                    if can_perform_check(&action, game, player).is_ok() {
                        actions.push(LegalAction::ActivateManaAbility {
                            source: perm_id,
                            ability_index: i,
                        });
                    }
                }

                // Check activated abilities (non-mana)
                if let crate::ability::AbilityKind::Activated(activated) = &ability.kind {
                    // Validate the ability's cost can be paid
                    if can_pay_ability_cost(game, perm_id, player, &activated.mana_cost) {
                        // Check timing restrictions
                        let timing_ok = match activated.timing {
                            crate::ability::ActivationTiming::AnyTime => true,
                            crate::ability::ActivationTiming::DuringCombat => {
                                matches!(game.turn.phase, Phase::Combat)
                            }
                            crate::ability::ActivationTiming::SorcerySpeed => {
                                // Sorcery speed: main phase, empty stack, active player
                                game.turn.active_player == player
                                    && matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain)
                                    && game.stack_is_empty()
                            }
                            crate::ability::ActivationTiming::OncePerTurn => {
                                // Check if this ability has been activated this turn
                                !game.ability_activated_this_turn(perm_id, i)
                            }
                            crate::ability::ActivationTiming::DuringYourTurn => {
                                game.turn.active_player == player
                            }
                            crate::ability::ActivationTiming::DuringOpponentsTurn => {
                                game.turn.active_player != player
                            }
                        };

                        if timing_ok {
                            actions.push(LegalAction::ActivateAbility {
                                source: perm_id,
                                ability_index: i,
                            });
                        }
                    }
                }
            }
        }
    }

    actions
}

/// Compute legal commander actions for a player (casting from command zone).
///
/// These are kept separate from regular legal actions so they can be accessed
/// via 'C' input rather than numeric indices.
pub fn compute_commander_actions(game: &GameState, player: PlayerId) -> Vec<LegalAction> {
    let mut actions = Vec::new();

    // Check for commanders that can be cast from command zone
    if let Some(player_obj) = game.player(player) {
        for &commander_id in player_obj.get_commanders() {
            if let Some(commander) = game.object(commander_id) {
                // Only if the commander is in the command zone
                if commander.zone == Zone::Command
                    && can_cast_spell(game, player, commander, &CastingMethod::Normal)
                {
                    actions.push(LegalAction::CastSpell {
                        spell_id: commander_id,
                        from_zone: Zone::Command,
                        casting_method: CastingMethod::Normal,
                    });
                }
            }
        }
    }

    actions
}

/// Check if a player can pay an activated ability's cost.
///
/// This is a thin wrapper around `can_pay_cost` that returns a bool
/// instead of a Result for convenience in computing legal actions.
fn can_pay_ability_cost(
    game: &GameState,
    source_id: ObjectId,
    player: PlayerId,
    cost: &crate::cost::TotalCost,
) -> bool {
    can_pay_cost(game, source_id, player, cost).is_ok()
}

/// Check if a player could potentially pay a TotalCost.
///
/// This uses the new costs module to check each cost component, considering:
/// - Mana costs: includes potential mana from untapped sources
/// - Non-mana costs: checks if requirements can be met
///
/// This is useful for UI to show actions that could be afforded after
/// tapping mana sources.
pub fn can_potentially_pay_total_cost(
    game: &GameState,
    source_id: ObjectId,
    player: PlayerId,
    cost: &crate::cost::TotalCost,
) -> bool {
    use crate::costs::{CostCheckContext, can_potentially_pay_with_check_context};

    let ctx = CostCheckContext::new(source_id, player);

    for c in cost.costs() {
        if can_potentially_pay_with_check_context(&*c.0, game, &ctx).is_err() {
            return false;
        }
    }
    true
}

/// Check if a spell can be cast by a player using the given casting method.
pub fn can_cast_spell(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    use crate::game_loop::spell_has_legal_targets;
    use crate::types::CardType;

    // Check if player is prevented from casting spells
    if !game.can_cast_spells(player) {
        return false;
    }

    // Lands cannot be cast - they are played as a special action
    if spell.is_land() {
        return false;
    }

    // Check timing restrictions
    let is_sorcery_speed = spell.has_card_type(CardType::Sorcery)
        || spell.has_card_type(CardType::Creature)
        || spell.has_card_type(CardType::Artifact)
        || spell.has_card_type(CardType::Enchantment)
        || spell.has_card_type(CardType::Planeswalker);

    // Check if has flash (either intrinsically or granted)
    let has_flash = spell.abilities.iter().any(|a| {
        if let crate::ability::AbilityKind::Static(s) = &a.kind {
            s.has_flash()
        } else {
            false
        }
    }) || game.grant_registry.card_has_granted_ability(
        game,
        spell.id,
        Zone::Hand,
        player,
        &crate::static_abilities::StaticAbility::flash(),
    );

    if is_sorcery_speed && !has_flash {
        // Sorcery-speed spells require: active player, main phase, empty stack
        if game.turn.active_player != player {
            return false;
        }
        if !matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain) {
            return false;
        }
        if !game.stack_is_empty() {
            return false;
        }
    }

    // Determine which mana cost to check based on casting method
    let base_mana_cost = match casting_method {
        CastingMethod::Normal => spell.mana_cost.as_ref(),
        CastingMethod::Alternative(idx) => {
            // Get the alternative cost
            spell
                .alternative_casts
                .get(*idx)
                .and_then(|method| method.mana_cost())
                .or(spell.mana_cost.as_ref()) // Fallback to normal cost for jump-start
        }
        CastingMethod::GrantedEscape { .. } => spell.mana_cost.as_ref(), // Uses card's own cost
        CastingMethod::GrantedFlashback => spell.mana_cost.as_ref(),     // Uses card's own cost
        CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => {
            // PlayFrom with normal cost - uses card's own mana cost
            spell.mana_cost.as_ref()
        }
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            ..
        } => spell
            .alternative_casts
            .get(*idx)
            .and_then(|method| method.mana_cost()),
    };

    // Check mana availability with cost reductions applied
    if let Some(base_cost) = base_mana_cost {
        // Calculate effective cost after applying cost reductions (Affinity, etc.)
        let effective_cost = calculate_effective_mana_cost(game, player, spell, base_cost);

        // For X spells, check if they can pay at least X=0
        // For non-X spells, check if they can pay the full cost (x_value=0)
        // Use potential mana (current pool + untapped mana sources)
        if !can_potentially_pay(game, player, &effective_cost, 0) {
            return false;
        }
    }

    // Check if legal targets exist for targeted spells
    let effects = spell.spell_effect.as_deref().unwrap_or(&[]);
    if !spell_has_legal_targets(game, effects, player, Some(spell.id)) {
        return false;
    }

    true
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

/// Check if a spell can be cast with the given mana cost and additional requirements.
///
/// This is the unified function for checking spell castability across all casting methods:
/// - Normal casting
/// - Alternative casting methods (flashback, escape, jump-start)
/// - Granted abilities (from Snapcaster Mage, Underworld Breach, etc.)
fn can_cast_with_cost(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    mana_cost: Option<&crate::mana::ManaCost>,
    requirements: &AdditionalCastRequirements,
) -> bool {
    use crate::game_loop::spell_has_legal_targets;
    use crate::types::CardType;

    // Lands cannot be cast
    if spell.is_land() {
        return false;
    }

    // Check type restriction if required
    if requirements.must_be_instant_or_sorcery
        && !spell.has_card_type(CardType::Instant)
        && !spell.has_card_type(CardType::Sorcery)
    {
        return false;
    }

    // Check timing restrictions
    let is_sorcery_speed = spell.has_card_type(CardType::Sorcery)
        || spell.has_card_type(CardType::Creature)
        || spell.has_card_type(CardType::Artifact)
        || spell.has_card_type(CardType::Enchantment)
        || spell.has_card_type(CardType::Planeswalker);

    // Check if has flash (either intrinsically or granted)
    let has_flash = spell.abilities.iter().any(|a| {
        if let crate::ability::AbilityKind::Static(s) = &a.kind {
            s.has_flash()
        } else {
            false
        }
    }) || game.grant_registry.card_has_granted_ability(
        game,
        spell_id,
        Zone::Hand,
        player,
        &crate::static_abilities::StaticAbility::flash(),
    );

    if is_sorcery_speed && !has_flash {
        if game.turn.active_player != player {
            return false;
        }
        if !matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain) {
            return false;
        }
        if !game.stack_is_empty() {
            return false;
        }
    }

    // Check mana availability (using potential mana from untapped sources)
    if let Some(cost) = mana_cost
        && !can_potentially_pay(game, player, cost, 0)
    {
        return false;
    }

    // Check additional cost requirements
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

    // Check if legal targets exist
    let effects = spell.spell_effect.as_deref().unwrap_or(&[]);
    if !spell_has_legal_targets(game, effects, player, Some(spell_id)) {
        return false;
    }

    true
}

/// Build additional cast requirements from an alternative casting method.
fn build_requirements_for_method(
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> AdditionalCastRequirements {
    use crate::alternative_cast::AlternativeCastingMethod;

    let mut requirements = AdditionalCastRequirements::default();

    match method {
        AlternativeCastingMethod::JumpStart => {
            requirements.discard_from_hand = 1;
        }
        AlternativeCastingMethod::Escape { exile_count, .. } => {
            requirements.exile_from_graveyard = *exile_count;
        }
        AlternativeCastingMethod::Flashback { .. } => {
            // Flashback has no additional requirements beyond mana
        }
        _ => {}
    }

    requirements
}

/// Get the mana cost for an alternative casting method.
fn get_mana_cost_for_method<'a>(
    method: &'a crate::alternative_cast::AlternativeCastingMethod,
    spell: &'a crate::object::Object,
) -> Option<&'a crate::mana::ManaCost> {
    // Method's cost takes priority, fallback to spell's cost
    method.mana_cost().or(spell.mana_cost.as_ref())
}

/// Check if a spell can be cast using an alternative casting method.
fn can_cast_with_alternative(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> bool {
    let requirements = build_requirements_for_method(method);
    let mana_cost = get_mana_cost_for_method(method, spell);
    can_cast_with_cost(game, player, spell, spell.id, mana_cost, &requirements)
}

/// Check if a spell can be cast with an alternative cost from hand (e.g., Force of Will).
pub fn can_cast_with_alternative_from_hand(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;

    match method {
        AlternativeCastingMethod::AlternativeCost {
            mana_cost,
            cost_effects,
            ..
        } => {
            // Check mana cost if present
            if let Some(mana) = mana_cost
                && !can_cast_with_cost(
                    game,
                    player,
                    spell,
                    spell_id,
                    Some(mana),
                    &AdditionalCastRequirements::default(),
                )
            {
                return false;
            }

            // Check cost effects can be paid using the generic can_execute_as_cost method
            for effect in cost_effects {
                if effect
                    .0
                    .can_execute_as_cost(game, spell_id, player)
                    .is_err()
                {
                    return false;
                }
            }

            // Check if legal targets exist for targeted spells
            use crate::game_loop::spell_has_legal_targets;
            let effects = spell.spell_effect.as_deref().unwrap_or(&[]);
            if !spell_has_legal_targets(game, effects, player, Some(spell_id)) {
                return false;
            }

            true
        }
        AlternativeCastingMethod::MindbreakTrap {
            cost, condition, ..
        } => {
            // Check if the trap condition is met
            if !is_trap_condition_met(game, player, condition) {
                return false;
            }
            // Check if player can pay the trap cost (usually {0})
            can_cast_with_cost(
                game,
                player,
                spell,
                spell_id,
                Some(cost),
                &AdditionalCastRequirements::default(),
            )
        }
        _ => false,
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
                .any(|&opp| game.spells_cast_this_turn.get(&opp).copied().unwrap_or(0) >= *count)
        }
        TrapCondition::OpponentSearchedLibrary => {
            // Check if any opponent searched their library this turn
            opponents
                .iter()
                .any(|opp| game.library_searches_this_turn.contains(opp))
        }
        TrapCondition::OpponentCreatureEntered => {
            // Check if any opponent had a creature enter the battlefield this turn
            opponents.iter().any(|&opp| {
                game.creatures_entered_this_turn
                    .get(&opp)
                    .copied()
                    .unwrap_or(0)
                    > 0
            })
        }
        TrapCondition::CreatureDealtDamageToYou => {
            // Check if any creature dealt damage to the player this turn
            game.creature_damage_to_players_this_turn
                .get(&player)
                .copied()
                .unwrap_or(0)
                > 0
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

    let Some(player_obj) = game.player(player) else {
        return false;
    };

    match cost.processing_mode() {
        CostProcessingMode::Immediate => {
            // For immediate costs like life payment, check via the cost's can_pay
            if let Some(life_amount) = cost.life_amount() {
                player_obj.life > life_amount as i32
            } else {
                // For other immediate costs (tap, untap, etc.), assume payable
                true
            }
        }
        CostProcessingMode::ExileFromHand {
            count,
            color_filter,
        } => {
            // Check if player has enough cards in hand matching the color filter
            // Exclude the spell being cast from the count
            let matching_cards = player_obj
                .hand
                .iter()
                .filter(|&&card_id| {
                    // Exclude the spell being cast
                    if spell_to_exclude == Some(card_id) {
                        return false;
                    }
                    // Check color filter if specified
                    if let Some(required_colors) = color_filter {
                        if let Some(card) = game.object(card_id) {
                            let card_colors = card.colors();
                            // Card must have at least one of the required colors
                            !card_colors.intersection(required_colors).is_empty()
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                })
                .count();
            matching_cards >= count as usize
        }
        // For other processing modes, assume payable
        _ => true,
    }
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
    calculate_effective_mana_cost_with_targets(game, player, spell, base_cost, 1)
}

/// Calculate the effective mana cost with explicit chosen target count.
pub fn calculate_effective_mana_cost_with_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_target_count,
        true,
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
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_target_count,
        false,
    )
}

fn calculate_effective_mana_cost_with_targets_internal(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    include_convoke_improvise_reductions: bool,
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
        let artifact_count = count_artifacts_controlled(game, player);
        current_cost = current_cost.reduce_generic(artifact_count);
    }

    // Apply explicit cost reductions/increases on the spell itself.
    current_cost =
        apply_spell_cost_modifiers(game, player, spell, &current_cost, chosen_target_count);

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

    current_cost
}

fn apply_spell_cost_modifiers(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;

    let mut total_increase: i32 = 0;
    let mut total_reduction: i32 = 0;

    for ability in &spell.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        if let Some(reduction) = static_ability.cost_reduction() {
            let amount = resolve_cost_modifier_value(game, player, spell, &reduction.reduction);
            if amount > 0 {
                total_reduction = total_reduction.saturating_add(amount);
            }
        }
        if let Some(increase) = static_ability.cost_increase() {
            let amount = resolve_cost_modifier_value(game, player, spell, &increase.increase);
            if amount > 0 {
                total_increase = total_increase.saturating_add(amount);
            }
        }
        if let Some(per_target_amount) = static_ability.cost_increase_per_additional_target() {
            let additional_targets = chosen_target_count.saturating_sub(1);
            if additional_targets > 0 {
                let extra = (per_target_amount as i32).saturating_mul(additional_targets as i32);
                total_increase = total_increase.saturating_add(extra);
            }
        }
    }

    let mut adjusted = cost.clone();
    if total_increase > 0 {
        adjusted = add_generic_mana_cost(&adjusted, total_increase as u32);
    }
    if total_reduction > 0 {
        adjusted = adjusted.reduce_generic(total_reduction as u32);
    }
    adjusted
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
    game.battlefield
        .iter()
        .filter(|&&id| {
            if let Some(obj) = game.object(id) {
                obj.controller == player && obj.has_card_type(crate::types::CardType::Artifact)
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
    use crate::ability::AbilityKind;
    use crate::costs::{CostCheckContext, can_pay_with_check_context};

    // Start with current mana pool
    let mut potential = game
        .player(player)
        .map(|p| p.mana_pool.clone())
        .unwrap_or_default();

    // Add mana from all available mana abilities
    for &perm_id in &game.battlefield {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };

        if perm.controller != player {
            continue;
        }

        for ability in perm.abilities.iter() {
            if let AbilityKind::Mana(mana_ability) = &ability.kind {
                // Do a simple non-recursive check for whether this mana ability
                // could be activated. We intentionally skip mana cost checks here
                // to avoid infinite recursion (mana ability with mana cost would
                // call compute_potential_mana again).
                let ctx = CostCheckContext::new(perm_id, player);

                let can_activate = mana_ability.mana_cost.costs().iter().all(|cost| {
                    // Skip mana cost check to avoid recursion - we only check
                    // non-mana costs like tap, life, sacrifice
                    if cost.processing_mode().is_mana_payment() {
                        // Assume mana costs could be paid from other sources
                        // This is an approximation but prevents infinite recursion
                        true
                    } else {
                        can_pay_with_check_context(&*cost.0, game, &ctx).is_ok()
                    }
                });

                // Also check activation condition if present
                let condition_met = mana_ability
                    .activation_condition
                    .as_ref()
                    .is_none_or(|cond| {
                        check_mana_ability_condition_for_potential(game, player, cond)
                    });

                if can_activate && condition_met {
                    // Add the mana this ability would produce
                    for mana in &mana_ability.mana {
                        potential.add(*mana, 1);
                    }
                    // Only count one mana ability per permanent (can only tap once)
                    break;
                }
            }
        }
    }

    potential
}

/// Check mana ability condition for potential mana computation.
fn check_mana_ability_condition_for_potential(
    game: &GameState,
    player: PlayerId,
    condition: &crate::ability::ManaAbilityCondition,
) -> bool {
    match condition {
        crate::ability::ManaAbilityCondition::ControlLandWithSubtype(required_subtypes) => {
            // Check if the player controls a land with at least one of the required subtypes
            game.battlefield.iter().any(|&perm_id| {
                if let Some(perm) = game.object(perm_id) {
                    perm.controller == player
                        && perm.is_land()
                        && required_subtypes.iter().any(|st| perm.has_subtype(*st))
                } else {
                    false
                }
            })
        }
        crate::ability::ManaAbilityCondition::ControlAtLeastArtifacts(required_count) => {
            let controlled_artifacts = game
                .battlefield
                .iter()
                .filter_map(|&perm_id| game.object(perm_id))
                .filter(|perm| {
                    perm.controller == player
                        && perm.card_types.contains(&crate::types::CardType::Artifact)
                })
                .count() as u32;
            controlled_artifacts >= *required_count
        }
        crate::ability::ManaAbilityCondition::ControlAtLeastLands(required_count) => {
            let controlled_lands = game
                .battlefield
                .iter()
                .filter_map(|&perm_id| game.object(perm_id))
                .filter(|perm| perm.controller == player && perm.is_land())
                .count() as u32;
            controlled_lands >= *required_count
        }
        crate::ability::ManaAbilityCondition::ControlCreatureWithPowerAtLeast(required_power) => {
            game.battlefield.iter().any(|&perm_id| {
                game.object(perm_id).is_some_and(|perm| {
                    perm.controller == player
                        && perm.is_creature()
                        && perm.power().is_some_and(|power| power >= *required_power as i32)
                })
            })
        }
        crate::ability::ManaAbilityCondition::ControlCreaturesTotalPowerAtLeast(required_power) => {
            let total_power = game
                .battlefield
                .iter()
                .filter_map(|&perm_id| game.object(perm_id))
                .filter(|perm| perm.controller == player && perm.is_creature())
                .map(|perm| perm.power().unwrap_or(0).max(0))
                .sum::<i32>();
            total_power >= *required_power as i32
        }
        crate::ability::ManaAbilityCondition::CardInYourGraveyard {
            card_types,
            subtypes,
        } => game.player(player).is_some_and(|player_state| {
            player_state.graveyard.iter().any(|&card_id| {
                let Some(card) = game.object(card_id) else {
                    return false;
                };
                let card_type_match = card_types.is_empty()
                    || card_types
                        .iter()
                        .any(|card_type| card.card_types.contains(card_type));
                let subtype_match = subtypes.is_empty()
                    || subtypes.iter().any(|subtype| card.has_subtype(*subtype));
                card_type_match && subtype_match
            })
        }),
        crate::ability::ManaAbilityCondition::Timing(timing) => match timing {
            crate::ability::ActivationTiming::AnyTime => true,
            crate::ability::ActivationTiming::DuringCombat => {
                matches!(game.turn.phase, Phase::Combat)
            }
            crate::ability::ActivationTiming::SorcerySpeed => {
                game.turn.active_player == player
                    && matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain)
                    && game.stack_is_empty()
            }
            crate::ability::ActivationTiming::OncePerTurn => true,
            crate::ability::ActivationTiming::DuringYourTurn => game.turn.active_player == player,
            crate::ability::ActivationTiming::DuringOpponentsTurn => {
                game.turn.active_player != player
            }
        },
        crate::ability::ManaAbilityCondition::All(conditions) => conditions
            .iter()
            .all(|inner| check_mana_ability_condition_for_potential(game, player, inner)),
    }
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
        apply_spell_cost_modifiers(game, player, spell, &cost_after_reductions, 1);

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
        apply_spell_cost_modifiers(game, player, spell, &cost_after_reductions, 1);

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
    use crate::ability::AbilityKind;

    game.battlefield
        .iter()
        .filter_map(|&id| {
            let obj = game.object(id)?;
            // Must be a creature controlled by player
            if obj.controller != player
                || !game.object_has_card_type(id, crate::types::CardType::Creature)
            {
                return None;
            }
            // Must be untapped
            if game.is_tapped(id) {
                return None;
            }
            // Must not have summoning sickness (unless has haste)
            if game.is_summoning_sick(id) {
                let has_haste = obj.abilities.iter().any(|a| {
                    if let AbilityKind::Static(s) = &a.kind {
                        s.has_haste()
                    } else {
                        false
                    }
                });
                if !has_haste {
                    return None;
                }
            }
            Some((id, obj.colors()))
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

    // Find all creatures controlled by active player that can attack
    for &perm_id in &game.battlefield {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };

        if perm.controller != active_player {
            continue;
        }

        if !game.object_has_card_type(perm_id, crate::types::CardType::Creature) {
            continue;
        }

        // Check if creature can attack
        if !crate::rules::combat::can_attack(perm, game) {
            continue;
        }

        // Determine valid attack targets
        let mut valid_targets = Vec::new();

        // Can attack each opponent
        for opponent in &game.players {
            if opponent.id != active_player && opponent.is_in_game() {
                valid_targets.push(AttackTarget::Player(opponent.id));
            }
        }

        // Can attack planeswalkers controlled by opponents
        for &other_perm_id in &game.battlefield {
            if let Some(other_perm) = game.object(other_perm_id)
                && other_perm.controller != active_player
                && game.object_has_card_type(other_perm_id, crate::types::CardType::Planeswalker)
            {
                valid_targets.push(AttackTarget::Planeswalker(other_perm_id));
            }
        }

        let must_attack = crate::rules::combat::must_attack_with_game(perm, game);

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
    let mut options = Vec::new();

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

            if !game.object_has_card_type(perm_id, crate::types::CardType::Creature) {
                continue;
            }

            // Check if this creature can block this attacker
            if crate::rules::combat::can_block(attacker, blocker, game) {
                valid_blockers.push(perm_id);
            }
        }

        let min_blockers = crate::rules::combat::minimum_blockers_with_game(attacker, game);

        options.push(BlockerOption {
            attacker: attacker_id,
            valid_blockers,
            min_blockers,
        });
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
        ctx.legal_actions
            .iter()
            .find(|a| matches!(a, LegalAction::PassPriority))
            .cloned()
            .unwrap_or_else(|| {
                ctx.legal_actions
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
        // Default: select first valid target for each requirement with min > 0
        ctx.requirements
            .iter()
            .filter(|r| r.min_targets > 0)
            .filter_map(|r| r.legal_targets.first().cloned())
            .collect()
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
        let legal: Vec<usize> = ctx
            .options
            .iter()
            .filter(|o| o.legal)
            .map(|o| o.index)
            .collect();
        legal.into_iter().take(ctx.min).collect()
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
        let legal: Vec<usize> = ctx
            .options
            .iter()
            .filter(|o| o.legal)
            .map(|o| o.index)
            .collect();
        let count = ctx.max.min(legal.len());
        legal.into_iter().take(count).collect()
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
                "DEBUG: Boolean[{}] = {:?}, input = {:?}",
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
                "DEBUG: Number[{}] = {:?} (min={}, max={}), input = {:?}",
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
                "DEBUG: Objects[{}] = {:?} ({} candidates, min={}, max={:?}), input = {:?}",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.candidates.len(),
                ctx.min,
                ctx.max,
                input
            );
        }

        if trimmed.is_empty() && ctx.min == 0 {
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
        while selected.len() < ctx.min && selected.len() < legal.len() {
            if !selected.contains(&legal[selected.len()]) {
                selected.push(legal[selected.len()]);
            } else {
                break;
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
                "DEBUG: Options[{}] = {:?} ({} options, min={}, max={}), input = {:?}",
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
                "DEBUG: Order[{}] = {:?} ({} items), input = {:?}",
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
                "DEBUG: Attackers[{}] ({} options), input = {:?}",
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
                "DEBUG: Blockers[{}] ({} attacker options), input = {:?}",
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
                "DEBUG: Distribute[{}] = {:?} (total={}, {} targets), input = {:?}",
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
                "DEBUG: Colors[{}] (count={}, same_color={}), input = {:?}",
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
                "DEBUG: Counters[{}] (max_total={}, {} types), input = {:?}",
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
                "DEBUG: Partition[{}] = {:?} ({} cards), input = {:?}",
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
                "DEBUG: Proliferate[{}] ({} permanents, {} players), input = {:?}",
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
                "DEBUG: Priority[{}] = {} actions, input = {:?}",
                self.index.saturating_sub(1),
                ctx.legal_actions.len(),
                input
            );
        }

        // Empty input means pass priority
        if trimmed.is_empty() {
            return LegalAction::PassPriority;
        }

        // Check for commander action (C, c, C0, c0, C1, c1, etc.)
        let lower = trimmed.to_lowercase();
        if lower == "c" && ctx.commander_actions.len() == 1 {
            return ctx.commander_actions[0].clone();
        }
        if lower.starts_with('c')
            && let Ok(idx) = lower[1..].parse::<usize>()
            && idx < ctx.commander_actions.len()
        {
            return ctx.commander_actions[idx].clone();
        }

        // Parse as index
        if let Ok(idx) = trimmed.parse::<usize>()
            && idx < ctx.legal_actions.len()
        {
            return ctx.legal_actions[idx].clone();
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
                "DEBUG: Targets[{}] = {} requirements, input = {:?}",
                self.index.saturating_sub(1),
                ctx.requirements.len(),
                input
            );
            for (i, req) in ctx.requirements.iter().enumerate() {
                eprintln!(
                    "DEBUG:   req[{}]: {} legal targets",
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
        prompt_priority_action(game, &ctx.legal_actions, &ctx.commander_actions)
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
        prompt_select_objects(game, &ctx.candidates, ctx.min, ctx.max)
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
        let zone_label = format!("{:?}", ctx.zone).to_lowercase();

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
                    let pt = if obj.is_creature() {
                        // Use calculated power/toughness (includes +1/+1 counters, anthems, etc.)
                        let power = game.calculated_power(id).unwrap_or(0);
                        let toughness = game.calculated_toughness(id).unwrap_or(0);
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

fn format_cost_effects(cost_effects: &[crate::effect::Effect]) -> String {
    if cost_effects.is_empty() {
        return "Free".to_string();
    }
    cost_effects
        .iter()
        .filter_map(|e| {
            // Use the cost_description method if available, otherwise skip
            // (ExileEffect after ChooseObjects is redundant in the description)
            e.0.cost_description()
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// New version of prompt_priority_action that returns LegalAction directly
/// (used by the new decide_priority method).
fn prompt_priority_action(
    game: &GameState,
    actions: &[LegalAction],
    commander_actions: &[LegalAction],
) -> LegalAction {
    // Format actions compactly
    let action_strs: Vec<String> = actions
        .iter()
        .enumerate()
        .map(|(i, a)| format!("{}:{}", i, format_action_short(game, a)))
        .collect();
    println!("Actions: {}", action_strs.join(" | "));

    // Display commander actions separately with 'C' prefix
    if !commander_actions.is_empty() {
        let commander_strs: Vec<String> = commander_actions
            .iter()
            .enumerate()
            .map(|(i, a)| {
                if commander_actions.len() == 1 {
                    format!("C:{}", format_action_short(game, a))
                } else {
                    format!("C{}:{}", i, format_action_short(game, a))
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
        if lower == "c" && commander_actions.len() == 1 {
            return commander_actions[0].clone();
        }
        if lower.starts_with('c')
            && let Ok(idx) = lower[1..].parse::<usize>()
            && idx < commander_actions.len()
        {
            return commander_actions[idx].clone();
        }

        if let Ok(idx) = trimmed.parse::<usize>()
            && idx < actions.len()
        {
            return actions[idx].clone();
        }
        println!("Invalid (0-{})", actions.len() - 1);
    }
}

fn format_action_short(game: &GameState, action: &LegalAction) -> String {
    match action {
        LegalAction::PassPriority => "Pass".to_string(),
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
                    crate::alternative_cast::CastingMethod::Alternative(idx) => {
                        // Get the alternative cost description
                        if let Some(alt_method) = obj.alternative_casts.get(*idx) {
                            let cost_effects = alt_method.cost_effects();
                            let cost_desc = if !cost_effects.is_empty() {
                                // For AlternativeCost (like Force of Will), show the cost effects
                                let effects_desc = format_cost_effects(cost_effects);
                                if let Some(mana) = alt_method.mana_cost() {
                                    // Has both mana and cost effects
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
                        format!("{} [from {:?}] ({})", obj.name, zone, format_mana_cost(obj))
                    }
                    crate::alternative_cast::CastingMethod::PlayFrom {
                        zone,
                        use_alternative: Some(idx),
                        ..
                    } => {
                        if let Some(alt_method) = obj.alternative_casts.get(*idx) {
                            let cost_effects = alt_method.cost_effects();
                            let cost_desc = if !cost_effects.is_empty() {
                                let effects_desc = format_cost_effects(cost_effects);
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
                                "{} [from {:?}, {}] ({})",
                                obj.name,
                                zone,
                                alt_method.name(),
                                cost_desc
                            )
                        } else {
                            format!(
                                "{} [from {:?}, Alt] ({})",
                                obj.name,
                                zone,
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
            if let Some(obj) = game.object(*source) {
                if let Some(ability) = obj.abilities.get(*ability_index) {
                    if let crate::AbilityKind::Mana(mana_ability) = &ability.kind {
                        if mana_ability.has_tap_cost() {
                            format!("Tap {}", name)
                        } else {
                            // Show the ability text if available, otherwise show cost
                            if let Some(text) = &ability.text {
                                format!("{}: {}", name, text)
                            } else {
                                format!("Activate {}", name)
                            }
                        }
                    } else {
                        format!("Activate {}", name)
                    }
                } else {
                    format!("Tap {}", name)
                }
            } else {
                format!("Tap {}", name)
            }
        }
        LegalAction::TurnFaceUp { creature_id } => {
            let name = game
                .object(*creature_id)
                .map(|o| o.name.as_str())
                .unwrap_or("?");
            format!("Flip {}", name)
        }
        LegalAction::SpecialAction(special) => format!("{:?}", special),
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
        println!(
            "  Blocker {}: {} (can block attackers: {:?})",
            i, blocker_name, can_block
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
    println!(
        "Select {} to {} objects (comma-separated indices, or empty for none):",
        min, max_display
    );

    loop {
        print!("Selection: ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        // Handle empty input
        if trimmed.is_empty() {
            if min == 0 {
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
        if selected.len() < min {
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

            let color = match input.trim() {
                "0" | "w" | "white" => Some(Color::White),
                "1" | "u" | "blue" => Some(Color::Blue),
                "2" | "b" | "black" => Some(Color::Black),
                "3" | "r" | "red" => Some(Color::Red),
                "4" | "g" | "green" => Some(Color::Green),
                _ => None,
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
        println!("  {}: {:?} ({} available)", i, counter_type, count);
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
                        if obj.is_creature() {
                            let power = game.calculated_power(*id).unwrap_or(0);
                            let toughness = game.calculated_toughness(*id).unwrap_or(0);
                            format!("{} {}/{} ({})", obj.name, power, toughness, controller_name)
                        } else {
                            format!("{} ({})", obj.name, controller_name)
                        }
                    } else {
                        format!("Object {:?}", id)
                    }
                }
                Target::Player(id) => game
                    .player(*id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("Player {:?}", id)),
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
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::CardId;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
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
    fn test_auto_pass_decision_maker() {
        use crate::decisions::context::PriorityContext;

        let game = setup_game();
        let mut dm = AutoPassDecisionMaker;

        let ctx = PriorityContext::new(
            PlayerId::from_index(0),
            vec![LegalAction::PassPriority],
            vec![],
        );

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

        let ctx = PriorityContext::new(PlayerId::from_index(0), legal_actions.clone(), vec![]);

        // "0" should select PassPriority
        assert!(matches!(
            dm.decide_priority(&game, &ctx),
            LegalAction::PassPriority
        ));

        // "1" should select PlayLand
        let ctx2 = PriorityContext::new(PlayerId::from_index(0), legal_actions.clone(), vec![]);
        assert!(matches!(
            dm.decide_priority(&game, &ctx2),
            LegalAction::PlayLand { .. }
        ));

        // "" (empty) should default to PassPriority
        let ctx3 = PriorityContext::new(PlayerId::from_index(0), legal_actions, vec![]);
        assert!(matches!(
            dm.decide_priority(&game, &ctx3),
            LegalAction::PassPriority
        ));
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
                effects: vec![Effect::pump(
                    2,
                    2,
                    crate::target::ChooseSpec::Source,
                    crate::effect::Until::EndOfTurn,
                )],
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
            }),
            functional_zones: vec![crate::zone::Zone::Battlefield],
            text: None,
        };
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(activated_ability);
        game.remove_summoning_sickness(creature_id);

        // Without mana, should not be able to activate
        let actions = compute_legal_actions(&game, alice);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == creature_id)),
            "Should NOT be able to activate without mana"
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
                |a| matches!(a, LegalAction::TurnFaceUp { creature_id: id } if *id == creature_id)
            ),
            "face-down creature with payable morph cost should have TurnFaceUp legal action"
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
                effects: vec![Effect::gain_life(1)],
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
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
}

//! Combat state management for MTG.
//!
//! This module handles combat declaration and state tracking including:
//! - Attacker declarations
//! - Blocker declarations
//! - Damage assignment order
//! - Combat queries

use std::collections::HashMap;

use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::rules::combat::{
    can_attack_defending_player, can_block, has_vigilance_with_game, maximum_blockers,
    minimum_blockers_with_game,
};
use crate::static_abilities::StaticAbility;
use crate::zone::Zone;

/// Combat state tracking.
#[derive(Debug, Clone, Default)]
pub struct CombatState {
    /// All declared attackers with their targets.
    pub attackers: Vec<AttackerInfo>,
    /// Mapping from attacker to their blockers.
    pub blockers: HashMap<ObjectId, Vec<ObjectId>>,
    /// Damage assignment order: attacker -> ordered list of blockers.
    pub damage_assignment_order: HashMap<ObjectId, Vec<ObjectId>>,
}

/// Information about an attacking creature.
#[derive(Debug, Clone)]
pub struct AttackerInfo {
    /// The attacking creature's ObjectId.
    pub creature: ObjectId,
    /// What the creature is attacking.
    pub target: AttackTarget,
}

/// The target of an attack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttackTarget {
    /// Attacking a player.
    Player(PlayerId),
    /// Attacking a planeswalker.
    Planeswalker(ObjectId),
}

/// Errors that can occur during combat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombatError {
    /// The creature cannot attack (defender, summoning sickness without haste, etc.).
    CreatureCannotAttack(ObjectId),
    /// The creature cannot block the specified attacker (evasion, protection, etc.).
    CreatureCannotBlock {
        blocker: ObjectId,
        attacker: ObjectId,
    },
    /// Not enough blockers were assigned to an attacker with menace.
    NotEnoughBlockers {
        attacker: ObjectId,
        required: usize,
        provided: usize,
    },
    /// Too many blockers were assigned to an attacker with a max-blockers restriction.
    TooManyBlockers {
        attacker: ObjectId,
        maximum: usize,
        provided: usize,
    },
    /// Too many creatures were declared as attackers this combat.
    TooManyAttackers { maximum: usize, provided: usize },
    /// Too many creatures were declared as blockers this combat.
    TooManyBlockingCreatures { maximum: usize, provided: usize },
    /// The attack target is invalid (player not in game, planeswalker doesn't exist, etc.).
    InvalidAttackTarget(AttackTarget),
    /// The creature is tapped and cannot attack or block.
    CreatureTapped(ObjectId),
    /// The creature is not in combat.
    NotInCombat(ObjectId),
    /// The creature is not on the battlefield.
    NotOnBattlefield(ObjectId),
    /// The creature is not a creature.
    NotACreature(ObjectId),
    /// The creature is not controlled by the specified player.
    NotControlledBy {
        creature: ObjectId,
        expected: PlayerId,
    },
    /// The blocker order doesn't match the assigned blockers.
    InvalidBlockerOrder {
        attacker: ObjectId,
        expected_blockers: Vec<ObjectId>,
        provided_blockers: Vec<ObjectId>,
    },
    /// A creature was declared multiple times as an attacker.
    DuplicateAttacker(ObjectId),
    /// A creature was declared as blocking multiple attackers.
    DuplicateBlocker(ObjectId),
    /// The attacker doesn't exist.
    AttackerNotFound(ObjectId),
    /// A creature with "must attack if able" was not declared as an attacker.
    MustAttackNotDeclared(ObjectId),
    /// A creature that must block a specific attacker if able was not declared as doing so.
    MustBlockRequirementNotMet {
        blocker: ObjectId,
        attacker: ObjectId,
    },
}

impl std::fmt::Display for CombatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CombatError::CreatureCannotAttack(id) => {
                write!(f, "Creature {:?} cannot attack", id)
            }
            CombatError::CreatureCannotBlock { blocker, attacker } => {
                write!(f, "Creature {:?} cannot block {:?}", blocker, attacker)
            }
            CombatError::NotEnoughBlockers {
                attacker,
                required,
                provided,
            } => {
                write!(
                    f,
                    "Attacker {:?} requires {} blockers but only {} provided",
                    attacker, required, provided
                )
            }
            CombatError::TooManyBlockers {
                attacker,
                maximum,
                provided,
            } => {
                write!(
                    f,
                    "Attacker {:?} allows at most {} blockers but {} provided",
                    attacker, maximum, provided
                )
            }
            CombatError::TooManyAttackers { maximum, provided } => {
                write!(
                    f,
                    "At most {} creatures can attack this combat but {} were declared",
                    maximum, provided
                )
            }
            CombatError::TooManyBlockingCreatures { maximum, provided } => {
                write!(
                    f,
                    "At most {} creatures can block this combat but {} were declared",
                    maximum, provided
                )
            }
            CombatError::InvalidAttackTarget(target) => {
                write!(f, "Invalid attack target: {:?}", target)
            }
            CombatError::CreatureTapped(id) => {
                write!(f, "Creature {:?} is tapped", id)
            }
            CombatError::NotInCombat(id) => {
                write!(f, "Creature {:?} is not in combat", id)
            }
            CombatError::NotOnBattlefield(id) => {
                write!(f, "Creature {:?} is not on the battlefield", id)
            }
            CombatError::NotACreature(id) => {
                write!(f, "Object {:?} is not a creature", id)
            }
            CombatError::NotControlledBy { creature, expected } => {
                write!(
                    f,
                    "Creature {:?} is not controlled by player {:?}",
                    creature, expected
                )
            }
            CombatError::InvalidBlockerOrder {
                attacker,
                expected_blockers,
                provided_blockers,
            } => {
                write!(
                    f,
                    "Invalid blocker order for attacker {:?}: expected {:?}, got {:?}",
                    attacker, expected_blockers, provided_blockers
                )
            }
            CombatError::DuplicateAttacker(id) => {
                write!(
                    f,
                    "Creature {:?} was declared as an attacker multiple times",
                    id
                )
            }
            CombatError::DuplicateBlocker(id) => {
                write!(
                    f,
                    "Creature {:?} was declared as blocking multiple attackers",
                    id
                )
            }
            CombatError::AttackerNotFound(id) => {
                write!(f, "Attacker {:?} not found", id)
            }
            CombatError::MustAttackNotDeclared(id) => {
                write!(
                    f,
                    "Creature {:?} must attack this combat if able but was not declared",
                    id
                )
            }
            CombatError::MustBlockRequirementNotMet { blocker, attacker } => {
                write!(
                    f,
                    "Creature {:?} must block {:?} this combat if able but was not declared",
                    blocker, attacker
                )
            }
        }
    }
}

impl std::error::Error for CombatError {}

/// Creates a new, empty combat state.
pub fn new_combat() -> CombatState {
    CombatState::default()
}

/// Clears all combat state at end of combat.
pub fn end_combat(combat: &mut CombatState) {
    combat.attackers.clear();
    combat.blockers.clear();
    combat.damage_assignment_order.clear();
}

fn battlefield_static_abilities(game: &GameState) -> Vec<StaticAbility> {
    let mut out = Vec::new();
    for &object_id in &game.battlefield {
        if let Some(calc) = game.calculated_characteristics(object_id) {
            out.extend(calc.static_abilities);
            continue;
        }
        if let Some(object) = game.object(object_id) {
            out.extend(
                object
                    .abilities
                    .iter()
                    .filter_map(|ability| match &ability.kind {
                        crate::ability::AbilityKind::Static(static_ability) => {
                            Some(static_ability.clone())
                        }
                        _ => None,
                    }),
            );
        }
    }
    out
}

fn max_creatures_can_attack_each_combat(game: &GameState) -> Option<usize> {
    battlefield_static_abilities(game)
        .iter()
        .filter_map(|ability| ability.max_creatures_can_attack_each_combat())
        .min()
}

fn max_creatures_can_block_each_combat(game: &GameState) -> Option<usize> {
    battlefield_static_abilities(game)
        .iter()
        .filter_map(|ability| ability.max_creatures_can_block_each_combat())
        .min()
}

/// Declares attackers for combat.
///
/// This function validates all attackers and taps those without vigilance.
/// The active player should be the attacker.
///
/// # Arguments
/// * `game` - Mutable reference to the game state (for tapping attackers)
/// * `combat` - The combat state to update
/// * `declarations` - List of (creature, target) pairs
///
/// # Returns
/// * `Ok(())` if all declarations are valid
/// * `Err(CombatError)` if any declaration is invalid
pub fn declare_attackers(
    game: &mut GameState,
    combat: &mut CombatState,
    declarations: Vec<(ObjectId, AttackTarget)>,
) -> Result<(), CombatError> {
    let active_player = game.turn.active_player;

    // First pass: validate all declarations
    let mut seen_attackers = std::collections::HashSet::new();
    for (creature_id, target) in &declarations {
        // Check for duplicate attackers
        if !seen_attackers.insert(*creature_id) {
            return Err(CombatError::DuplicateAttacker(*creature_id));
        }

        let creature = game
            .object(*creature_id)
            .ok_or(CombatError::NotOnBattlefield(*creature_id))?;

        // Must be on battlefield
        if creature.zone != Zone::Battlefield {
            return Err(CombatError::NotOnBattlefield(*creature_id));
        }

        // Must be a creature
        if !game.object_has_card_type(*creature_id, crate::types::CardType::Creature) {
            return Err(CombatError::NotACreature(*creature_id));
        }

        // Must be controlled by active player
        if creature.controller != active_player {
            return Err(CombatError::NotControlledBy {
                creature: *creature_id,
                expected: active_player,
            });
        }

        // Must be untapped
        if game.is_tapped(*creature_id) {
            return Err(CombatError::CreatureTapped(*creature_id));
        }

        // Validate attack target
        let defending_player = match target {
            AttackTarget::Player(player_id) => {
                let player = game
                    .player(*player_id)
                    .ok_or_else(|| CombatError::InvalidAttackTarget(target.clone()))?;
                if !player.is_in_game() {
                    return Err(CombatError::InvalidAttackTarget(target.clone()));
                }
                *player_id
            }
            AttackTarget::Planeswalker(pw_id) => {
                let pw = game
                    .object(*pw_id)
                    .ok_or_else(|| CombatError::InvalidAttackTarget(target.clone()))?;
                if pw.zone != Zone::Battlefield
                    || !game.object_has_card_type(*pw_id, crate::types::CardType::Planeswalker)
                {
                    return Err(CombatError::InvalidAttackTarget(target.clone()));
                }
                pw.controller
            }
        };

        // Must be able to attack (no defender, no summoning sickness unless haste, etc.)
        // Check both rules-based restrictions and effect-based restrictions.
        if !can_attack_defending_player(creature, defending_player, game)
            || !game.can_attack(*creature_id)
        {
            return Err(CombatError::CreatureCannotAttack(*creature_id));
        }

        // Validate attack target
        match target {
            AttackTarget::Player(_) | AttackTarget::Planeswalker(_) => {}
        }
    }

    if let Some(max_attackers) = max_creatures_can_attack_each_combat(game)
        && declarations.len() > max_attackers
    {
        return Err(CombatError::TooManyAttackers {
            maximum: max_attackers,
            provided: declarations.len(),
        });
    }

    // Second pass: apply declarations and tap attackers without vigilance
    for (creature_id, target) in declarations {
        // Add to attackers list
        combat.attackers.push(AttackerInfo {
            creature: creature_id,
            target,
        });

        // Initialize empty blocker list
        combat.blockers.insert(creature_id, Vec::new());

        // Tap the creature unless it has vigilance
        let creature = game.object(creature_id).unwrap();
        if !has_vigilance_with_game(creature, game) {
            game.tap(creature_id);
        }
    }

    Ok(())
}

/// Declares blockers for combat.
///
/// This function validates all blockers and enforces blocking restrictions.
/// The defending player should declare blockers.
///
/// # Arguments
/// * `game` - Reference to the game state
/// * `combat` - The combat state to update
/// * `declarations` - List of (blocker, attacker) pairs
///
/// # Returns
/// * `Ok(())` if all declarations are valid
/// * `Err(CombatError)` if any declaration is invalid
pub fn declare_blockers(
    game: &GameState,
    combat: &mut CombatState,
    declarations: Vec<(ObjectId, ObjectId)>,
) -> Result<(), CombatError> {
    // Group blockers by attacker for menace validation
    let mut blockers_by_attacker: HashMap<ObjectId, Vec<ObjectId>> = HashMap::new();
    let mut attackers_by_blocker: HashMap<ObjectId, Vec<ObjectId>> = HashMap::new();
    let mut blocker_counts: HashMap<ObjectId, usize> = HashMap::new();

    fn max_attackers_this_blocker_can_block(game: &GameState, blocker_id: ObjectId) -> usize {
        let static_abilities = game
            .calculated_characteristics(blocker_id)
            .map(|c| c.static_abilities)
            .unwrap_or_else(|| {
                game.object(blocker_id)
                    .map(|o| {
                        o.abilities
                            .iter()
                            .filter_map(|a| match &a.kind {
                                crate::ability::AbilityKind::Static(sa) => Some(sa.clone()),
                                _ => None,
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            });

        let extra: usize = static_abilities
            .iter()
            .filter_map(|a| a.additional_blockable_attackers())
            .sum();
        1usize.saturating_add(extra)
    }

    // First pass: validate all blockers
    for (blocker_id, attacker_id) in &declarations {
        // Validate blocker exists and is on battlefield
        let blocker = game
            .object(*blocker_id)
            .ok_or(CombatError::NotOnBattlefield(*blocker_id))?;

        // Check for blockers declared against too many attackers.
        let max_attackers = max_attackers_this_blocker_can_block(game, *blocker_id);
        let entry = blocker_counts.entry(*blocker_id).or_insert(0);
        *entry += 1;
        if *entry > max_attackers {
            return Err(CombatError::DuplicateBlocker(*blocker_id));
        }

        if blocker.zone != Zone::Battlefield {
            return Err(CombatError::NotOnBattlefield(*blocker_id));
        }

        // Must be a creature
        if !game.object_has_card_type(*blocker_id, crate::types::CardType::Creature) {
            return Err(CombatError::NotACreature(*blocker_id));
        }

        // Must be untapped
        if game.is_tapped(*blocker_id) {
            return Err(CombatError::CreatureTapped(*blocker_id));
        }

        // Validate attacker exists and is attacking
        if !is_attacking(combat, *attacker_id) {
            return Err(CombatError::AttackerNotFound(*attacker_id));
        }

        let attacker = game
            .object(*attacker_id)
            .ok_or(CombatError::NotOnBattlefield(*attacker_id))?;

        // Check if blocker can legally block the attacker (evasion, protection, etc.)
        if !can_block(attacker, blocker, game) {
            return Err(CombatError::CreatureCannotBlock {
                blocker: *blocker_id,
                attacker: *attacker_id,
            });
        }

        // Check if blocker has "can't block" from abilities or effects
        if game.object_has_ability(*blocker_id, &StaticAbility::cant_block())
            || !game.can_block(*blocker_id)
        {
            return Err(CombatError::CreatureCannotBlock {
                blocker: *blocker_id,
                attacker: *attacker_id,
            });
        }

        // Check if attacker can't be blocked (from CantEffectTracker)
        if !game.can_be_blocked(*attacker_id) {
            return Err(CombatError::CreatureCannotBlock {
                blocker: *blocker_id,
                attacker: *attacker_id,
            });
        }

        blockers_by_attacker
            .entry(*attacker_id)
            .or_default()
            .push(*blocker_id);
        attackers_by_blocker
            .entry(*blocker_id)
            .or_default()
            .push(*attacker_id);
    }

    if let Some(max_blockers) = max_creatures_can_block_each_combat(game)
        && blocker_counts.len() > max_blockers
    {
        return Err(CombatError::TooManyBlockingCreatures {
            maximum: max_blockers,
            provided: blocker_counts.len(),
        });
    }

    // Second pass: validate minimum/maximum blockers.
    for (attacker_id, blocker_list) in &blockers_by_attacker {
        let attacker = game.object(*attacker_id).unwrap();
        let min_blockers = minimum_blockers_with_game(attacker, game);

        // If any blockers were assigned, must meet minimum
        if !blocker_list.is_empty() && blocker_list.len() < min_blockers {
            return Err(CombatError::NotEnoughBlockers {
                attacker: *attacker_id,
                required: min_blockers,
                provided: blocker_list.len(),
            });
        }

        if let Some(max_blockers) = maximum_blockers(attacker, game)
            && blocker_list.len() > max_blockers
        {
            return Err(CombatError::TooManyBlockers {
                attacker: *attacker_id,
                maximum: max_blockers,
                provided: blocker_list.len(),
            });
        }
    }

    // Enforce "must block specific attacker if able" requirements.
    for (&blocker_id, required_attackers) in &game.cant_effects.must_block_specific_attackers {
        let Some(blocker) = game.object(blocker_id) else {
            continue;
        };
        if blocker.zone != Zone::Battlefield
            || !game.object_has_card_type(blocker_id, crate::types::CardType::Creature)
            || game.is_tapped(blocker_id)
        {
            continue;
        }

        for &required_attacker in required_attackers {
            if !is_attacking(combat, required_attacker) {
                continue;
            }
            let Some(attacker) = game.object(required_attacker) else {
                continue;
            };

            let can_legally_block_required = can_block(attacker, blocker, game)
                && game.can_block_attacker(blocker_id, required_attacker)
                && game.can_block(blocker_id)
                && game.can_be_blocked(required_attacker)
                && !game.object_has_ability(blocker_id, &StaticAbility::cant_block());
            if !can_legally_block_required {
                continue;
            }

            let declared_required = attackers_by_blocker
                .get(&blocker_id)
                .is_some_and(|attackers| attackers.contains(&required_attacker));
            if !declared_required {
                return Err(CombatError::MustBlockRequirementNotMet {
                    blocker: blocker_id,
                    attacker: required_attacker,
                });
            }
        }
    }

    // Third pass: apply declarations
    for (attacker_id, blocker_list) in blockers_by_attacker {
        combat.blockers.insert(attacker_id, blocker_list);
    }

    Ok(())
}

/// Sets the damage assignment order for an attacker's blockers.
///
/// When an attacker is blocked by multiple creatures, the attacking player
/// chooses the order in which to assign damage.
///
/// # Arguments
/// * `combat` - The combat state to update
/// * `attacker` - The attacking creature
/// * `blocker_order` - The ordered list of blockers
///
/// # Returns
/// * `Ok(())` if the order is valid
/// * `Err(CombatError)` if the order is invalid
pub fn set_damage_assignment_order(
    combat: &mut CombatState,
    attacker: ObjectId,
    blocker_order: Vec<ObjectId>,
) -> Result<(), CombatError> {
    // Check that attacker is in combat
    if !is_attacking(combat, attacker) {
        return Err(CombatError::NotInCombat(attacker));
    }

    // Get the assigned blockers
    let assigned_blockers = combat
        .blockers
        .get(&attacker)
        .ok_or(CombatError::AttackerNotFound(attacker))?;

    // Verify that blocker_order contains exactly the same blockers
    let mut expected: Vec<ObjectId> = assigned_blockers.clone();
    let mut provided: Vec<ObjectId> = blocker_order.clone();
    expected.sort_by_key(|id| id.0);
    provided.sort_by_key(|id| id.0);

    if expected != provided {
        return Err(CombatError::InvalidBlockerOrder {
            attacker,
            expected_blockers: assigned_blockers.clone(),
            provided_blockers: blocker_order,
        });
    }

    // Set the damage assignment order
    combat
        .damage_assignment_order
        .insert(attacker, blocker_order);

    Ok(())
}

/// Returns true if the creature is attacking.
pub fn is_attacking(combat: &CombatState, creature: ObjectId) -> bool {
    combat
        .attackers
        .iter()
        .any(|info| info.creature == creature)
}

/// Returns true if the creature is blocking.
pub fn is_blocking(combat: &CombatState, creature: ObjectId) -> bool {
    combat
        .blockers
        .values()
        .any(|blockers| blockers.contains(&creature))
}

/// Returns the blockers assigned to an attacker.
pub fn get_blockers(combat: &CombatState, attacker: ObjectId) -> &[ObjectId] {
    combat
        .blockers
        .get(&attacker)
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

/// Returns the attacker that a blocker is blocking, if any.
pub fn get_blocked_attacker(combat: &CombatState, blocker: ObjectId) -> Option<ObjectId> {
    for (attacker_id, blockers) in &combat.blockers {
        if blockers.contains(&blocker) {
            return Some(*attacker_id);
        }
    }
    None
}

/// Returns true if the attacker is blocked (has at least one blocker assigned).
pub fn is_blocked(combat: &CombatState, attacker: ObjectId) -> bool {
    combat
        .blockers
        .get(&attacker)
        .is_some_and(|blockers| !blockers.is_empty())
}

/// Returns true if the attacker is unblocked (no blockers assigned and is attacking).
pub fn is_unblocked(combat: &CombatState, attacker: ObjectId) -> bool {
    is_attacking(combat, attacker) && !is_blocked(combat, attacker)
}

/// Returns the attack target for a creature, if it is attacking.
pub fn get_attack_target(combat: &CombatState, attacker: ObjectId) -> Option<&AttackTarget> {
    combat
        .attackers
        .iter()
        .find(|info| info.creature == attacker)
        .map(|info| &info.target)
}

/// Returns all attackers targeting a specific player.
pub fn attackers_targeting_player(combat: &CombatState, player: PlayerId) -> Vec<ObjectId> {
    combat
        .attackers
        .iter()
        .filter(|info| matches!(&info.target, AttackTarget::Player(p) if *p == player))
        .map(|info| info.creature)
        .collect()
}

/// Returns all attackers targeting a specific planeswalker.
pub fn attackers_targeting_planeswalker(
    combat: &CombatState,
    planeswalker: ObjectId,
) -> Vec<ObjectId> {
    combat
        .attackers
        .iter()
        .filter(
            |info| matches!(&info.target, AttackTarget::Planeswalker(pw) if *pw == planeswalker),
        )
        .map(|info| info.creature)
        .collect()
}

/// Returns the damage assignment order for an attacker, or the default blocker order.
pub fn get_damage_assignment_order(combat: &CombatState, attacker: ObjectId) -> Vec<ObjectId> {
    combat
        .damage_assignment_order
        .get(&attacker)
        .cloned()
        .unwrap_or_else(|| get_blockers(combat, attacker).to_vec())
}

/// Returns all players being attacked (defending players).
///
/// In a 2-player game, this is typically just the opponent.
/// In multiplayer, creatures can attack different players.
pub fn defending_players(combat: &CombatState) -> Vec<PlayerId> {
    let mut players: Vec<PlayerId> = combat
        .attackers
        .iter()
        .filter_map(|info| {
            if let AttackTarget::Player(p) = &info.target {
                Some(*p)
            } else {
                None
            }
        })
        .collect();
    players.sort();
    players.dedup();
    players
}

/// Checks if a player is being attacked (is a defending player).
pub fn is_defending_player(combat: &CombatState, player: PlayerId) -> bool {
    combat
        .attackers
        .iter()
        .any(|info| matches!(&info.target, AttackTarget::Player(p) if *p == player))
}

/// Checks if a player is the attacking player (controls attacking creatures).
///
/// Note: In a typical 2-player game, the attacking player is the active player.
/// This function checks if any attacking creature is controlled by the given player.
pub fn is_attacking_player(combat: &CombatState, player: PlayerId, game: &GameState) -> bool {
    combat.attackers.iter().any(|info| {
        game.object(info.creature)
            .is_some_and(|obj| obj.controller == player)
    })
}

/// Returns the attacking player (the player who controls attacking creatures).
///
/// Returns None if there are no attackers.
pub fn get_attacking_player(combat: &CombatState, game: &GameState) -> Option<PlayerId> {
    combat
        .attackers
        .first()
        .and_then(|info| game.object(info.creature))
        .map(|obj| obj.controller)
}

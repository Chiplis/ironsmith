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
    let all_effects = game.all_continuous_effects();
    let mut out = Vec::new();
    for &object_id in &game.battlefield {
        out.extend(static_abilities_for_object(game, object_id, &all_effects));
    }
    out
}

fn static_abilities_for_object(
    game: &GameState,
    object_id: ObjectId,
    effects: &[crate::continuous::ContinuousEffect],
) -> Vec<StaticAbility> {
    if let Some(calc) = game.calculated_characteristics_with_effects(object_id, effects) {
        return calc.static_abilities;
    }
    game.object(object_id)
        .map(|object| {
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
        })
        .unwrap_or_default()
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

fn generic_mana_cost(amount: u32) -> crate::mana::ManaCost {
    use crate::mana::ManaSymbol;

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
    let declared_attackers: Vec<ObjectId> = declarations.iter().map(|(id, _)| *id).collect();
    let all_effects = game.all_continuous_effects();
    let mut additional_attack_mana_cost = 0u32;

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
        if !game.object_has_card_type_with_effects(
            *creature_id,
            crate::types::CardType::Creature,
            &all_effects,
        ) {
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
                    || !game.object_has_card_type_with_effects(
                        *pw_id,
                        crate::types::CardType::Planeswalker,
                        &all_effects,
                    )
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

        let abilities = static_abilities_for_object(game, creature.id, &all_effects);
        for ability in &abilities {
            if let Some(can_attack) = ability.can_attack_with_attacking_group(
                game,
                creature.id,
                creature.controller,
                &declared_attackers,
            ) && !can_attack
            {
                return Err(CombatError::CreatureCannotAttack(*creature_id));
            }
            if let Some(can_pay) =
                ability.can_pay_attack_cost(game, creature.id, creature.controller)
                && !can_pay
            {
                return Err(CombatError::CreatureCannotAttack(*creature_id));
            }
            if let Some(cost) =
                ability.generic_attack_mana_cost_for_source(game, creature.id, creature.controller)
            {
                additional_attack_mana_cost = additional_attack_mana_cost.saturating_add(cost);
            }
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

    // Pay non-mana attacker costs from "can't attack unless ..." restrictions.
    for (creature_id, _target) in &declarations {
        let Some(creature) = game.object(*creature_id) else {
            return Err(CombatError::NotOnBattlefield(*creature_id));
        };
        let creature_source = creature.id;
        let creature_controller = creature.controller;
        let abilities = game
            .calculated_characteristics(creature_source)
            .map(|c| c.static_abilities)
            .unwrap_or_else(|| {
                creature
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
        for ability in abilities {
            if let Some(result) =
                ability.pay_non_mana_attack_cost(game, creature_source, creature_controller)
                && result.is_err()
            {
                return Err(CombatError::CreatureCannotAttack(*creature_id));
            }
        }
    }

    // Pay aggregated generic mana attacker costs after validation.
    if additional_attack_mana_cost > 0
        && let Some((first_attacker, _)) = declarations.first()
    {
        let mana_cost = generic_mana_cost(additional_attack_mana_cost);
        if !game.can_pay_mana_cost(active_player, None, &mana_cost, 0)
            || !game.try_pay_mana_cost(active_player, None, &mana_cost, 0)
        {
            return Err(CombatError::CreatureCannotAttack(*first_attacker));
        }
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
    let all_effects = game.all_continuous_effects();

    // Group blockers by attacker for menace validation
    let mut blockers_by_attacker: HashMap<ObjectId, Vec<ObjectId>> = HashMap::new();
    let mut attackers_by_blocker: HashMap<ObjectId, Vec<ObjectId>> = HashMap::new();
    let mut blocker_counts: HashMap<ObjectId, usize> = HashMap::new();

    fn max_attackers_this_blocker_can_block(
        game: &GameState,
        blocker_id: ObjectId,
        effects: &[crate::continuous::ContinuousEffect],
    ) -> usize {
        let static_abilities = static_abilities_for_object(game, blocker_id, effects);

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
        let max_attackers = max_attackers_this_blocker_can_block(game, *blocker_id, &all_effects);
        let entry = blocker_counts.entry(*blocker_id).or_insert(0);
        *entry += 1;
        if *entry > max_attackers {
            return Err(CombatError::DuplicateBlocker(*blocker_id));
        }

        if blocker.zone != Zone::Battlefield {
            return Err(CombatError::NotOnBattlefield(*blocker_id));
        }

        // Must be a creature
        if !game.object_has_card_type_with_effects(
            *blocker_id,
            crate::types::CardType::Creature,
            &all_effects,
        ) {
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
        if game.object_has_ability_with_effects(
            *blocker_id,
            &StaticAbility::cant_block(),
            &all_effects,
        ) || !game.can_block(*blocker_id)
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
            || !game.object_has_card_type_with_effects(
                blocker_id,
                crate::types::CardType::Creature,
                &all_effects,
            )
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
                && !game.object_has_ability_with_effects(
                    blocker_id,
                    &StaticAbility::cant_block(),
                    &all_effects,
                );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness, PtValue};
    use crate::ids::CardId;
    use crate::mana::ManaSymbol;
    use crate::object::CounterType;
    use crate::static_abilities::{CantAttackUnlessConditionSpec, StaticAbility};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn creature_card(name: &str, power: i32, toughness: i32) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::new(
                PtValue::Fixed(power),
                PtValue::Fixed(toughness),
            ))
            .build()
    }

    fn land_card(name: &str, subtype: Option<Subtype>) -> crate::card::Card {
        let mut builder = CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Land]);
        if let Some(subtype) = subtype {
            builder = builder.subtypes(vec![subtype]);
        }
        builder.build()
    }

    fn enchantment_card(name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .build()
    }

    #[test]
    fn declare_attackers_rejects_at_least_two_other_creatures_attack_requirement() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let restricted = creature_card("Gang Source", 2, 2);
        let buddy_one = creature_card("Buddy One", 2, 2);
        let buddy_two = creature_card("Buddy Two", 2, 2);

        let restricted_id = game.create_object_from_card(&restricted, alice, Zone::Battlefield);
        let buddy_one_id = game.create_object_from_card(&buddy_one, alice, Zone::Battlefield);
        let buddy_two_id = game.create_object_from_card(&buddy_two, alice, Zone::Battlefield);
        game.object_mut(restricted_id)
            .expect("restricted creature should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::cant_attack_unless_condition(
                    CantAttackUnlessConditionSpec::AttackingGroupCondition(
                        crate::static_abilities::AttackingGroupAttackCondition::AtLeastNOtherCreaturesAttack(
                            2,
                        ),
                    ),
                    "Can't attack unless at least two other creatures attack",
                ),
            ));

        game.remove_summoning_sickness(restricted_id);
        game.remove_summoning_sickness(buddy_one_id);
        game.remove_summoning_sickness(buddy_two_id);

        let mut combat = CombatState::default();
        let invalid = declare_attackers(
            &mut game,
            &mut combat,
            vec![
                (restricted_id, AttackTarget::Player(bob)),
                (buddy_one_id, AttackTarget::Player(bob)),
            ],
        );
        assert_eq!(
            invalid,
            Err(CombatError::CreatureCannotAttack(restricted_id))
        );

        let valid = declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![
                (restricted_id, AttackTarget::Player(bob)),
                (buddy_one_id, AttackTarget::Player(bob)),
                (buddy_two_id, AttackTarget::Player(bob)),
            ],
        );
        assert!(valid.is_ok(), "expected valid three-creature attack");
    }

    #[test]
    fn declare_attackers_requires_and_pays_sacrifice_land_attack_cost() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = creature_card("Exalted Dragon Variant", 5, 5);
        let attacker_id = game.create_object_from_card(&attacker, alice, Zone::Battlefield);
        game.object_mut(attacker_id)
            .expect("attacker should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::cant_attack_unless_condition(
                    CantAttackUnlessConditionSpec::AttackCost(
                        crate::static_abilities::AttackCostCondition::SacrificeLands {
                            count: 1,
                            subtype: None,
                        },
                    ),
                    "Can't attack unless you sacrifice a land",
                ),
            ));
        game.remove_summoning_sickness(attacker_id);

        let without_land = declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![(attacker_id, AttackTarget::Player(bob))],
        );
        assert_eq!(
            without_land,
            Err(CombatError::CreatureCannotAttack(attacker_id))
        );

        let land = land_card("Forest", Some(Subtype::Forest));
        let _land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
        let with_land = declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![(attacker_id, AttackTarget::Player(bob))],
        );
        assert!(
            with_land.is_ok(),
            "expected attack to succeed after paying cost"
        );
        let graveyard_count = game
            .player(alice)
            .expect("attacking player should exist")
            .graveyard
            .len();
        assert_eq!(
            graveyard_count, 1,
            "expected one land sacrificed as attack cost"
        );
    }

    #[test]
    fn declare_attackers_requires_and_pays_return_enchantment_attack_cost() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = creature_card("Floodtide Serpent Variant", 4, 4);
        let attacker_id = game.create_object_from_card(&attacker, alice, Zone::Battlefield);
        game.object_mut(attacker_id)
            .expect("attacker should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::cant_attack_unless_condition(
                    CantAttackUnlessConditionSpec::AttackCost(
                        crate::static_abilities::AttackCostCondition::ReturnEnchantmentYouControlToOwnersHand,
                    ),
                    "Can't attack unless you return an enchantment you control to its owner's hand",
                ),
            ));
        game.remove_summoning_sickness(attacker_id);

        let without_enchantment = declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![(attacker_id, AttackTarget::Player(bob))],
        );
        assert_eq!(
            without_enchantment,
            Err(CombatError::CreatureCannotAttack(attacker_id))
        );

        let enchantment = enchantment_card("Seal of Return");
        let _enchantment_id = game.create_object_from_card(&enchantment, alice, Zone::Battlefield);
        let with_enchantment = declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![(attacker_id, AttackTarget::Player(bob))],
        );
        assert!(
            with_enchantment.is_ok(),
            "expected attack to succeed after returning enchantment"
        );
        let hand_count = game
            .player(alice)
            .expect("attacking player should exist")
            .hand
            .len();
        assert_eq!(
            hand_count, 1,
            "expected returned enchantment to be in attacker's hand"
        );
    }

    #[test]
    fn declare_attackers_requires_generic_mana_for_plus_one_plus_one_counter_attack_cost() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = creature_card("Phyrexian Marauder Variant", 0, 0);
        let attacker_id = game.create_object_from_card(&attacker, alice, Zone::Battlefield);
        {
            let attacker_obj = game
                .object_mut(attacker_id)
                .expect("attacker should exist on battlefield");
            attacker_obj.add_counters(CounterType::PlusOnePlusOne, 2);
            attacker_obj.abilities.push(Ability::static_ability(
                StaticAbility::cant_attack_unless_condition(
                    CantAttackUnlessConditionSpec::AttackCost(
                        crate::static_abilities::AttackCostCondition::PayOneForEachPlusOnePlusOneCounterOnIt,
                    ),
                    "Can't attack unless you pay {1} for each +1/+1 counter on it",
                ),
            ));
        }
        game.remove_summoning_sickness(attacker_id);

        let without_mana = declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![(attacker_id, AttackTarget::Player(bob))],
        );
        assert_eq!(
            without_mana,
            Err(CombatError::CreatureCannotAttack(attacker_id))
        );

        game.player_mut(alice)
            .expect("attacking player should exist")
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        let with_mana = declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![(attacker_id, AttackTarget::Player(bob))],
        );
        assert!(
            with_mana.is_ok(),
            "expected attack to succeed after paying per-counter mana"
        );
        let remaining = game
            .player(alice)
            .expect("attacking player should exist")
            .mana_pool
            .total();
        assert_eq!(remaining, 0, "expected mana attack cost to be paid");
    }
}

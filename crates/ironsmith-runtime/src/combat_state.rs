//! Combat state management for MTG.
//!
//! This module handles combat declaration and state tracking including:
//! - Attacker declarations
//! - Blocker declarations
//! - Damage assignment order
//! - Combat queries

use std::collections::{HashMap, HashSet};

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
    /// Attacking bands declared for the current combat.
    pub attacking_bands: Vec<Vec<ObjectId>>,
    /// Creatures that were required to attack when they were declared this combat.
    pub had_to_attack_this_combat: HashSet<ObjectId>,
}

impl CombatState {
    pub fn creature_had_to_attack_this_combat(&self, creature: ObjectId) -> bool {
        self.had_to_attack_this_combat.contains(&creature)
    }
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

impl std::fmt::Display for AttackTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttackTarget::Player(id) => write!(f, "player {}", id.0),
            AttackTarget::Planeswalker(id) => write!(f, "planeswalker #{}", id.0),
        }
    }
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
        fn object_label(id: &ObjectId) -> String {
            format!("#{}", id.0)
        }

        fn object_list(ids: &[ObjectId]) -> String {
            ids.iter().map(object_label).collect::<Vec<_>>().join(", ")
        }

        match self {
            CombatError::CreatureCannotAttack(id) => {
                write!(f, "Creature {} cannot attack", object_label(id))
            }
            CombatError::CreatureCannotBlock { blocker, attacker } => {
                write!(
                    f,
                    "Creature {} cannot block {}",
                    object_label(blocker),
                    object_label(attacker)
                )
            }
            CombatError::NotEnoughBlockers {
                attacker,
                required,
                provided,
            } => {
                write!(
                    f,
                    "Attacker {} requires {} blockers but only {} were declared",
                    object_label(attacker),
                    required,
                    provided
                )
            }
            CombatError::TooManyBlockers {
                attacker,
                maximum,
                provided,
            } => {
                write!(
                    f,
                    "Attacker {} allows at most {} blockers but {} were declared",
                    object_label(attacker),
                    maximum,
                    provided
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
                write!(f, "Invalid attack target: {target}")
            }
            CombatError::CreatureTapped(id) => {
                write!(f, "Creature {} is tapped", object_label(id))
            }
            CombatError::NotInCombat(id) => {
                write!(f, "Creature {} is not in combat", object_label(id))
            }
            CombatError::NotOnBattlefield(id) => {
                write!(f, "Creature {} is not on the battlefield", object_label(id))
            }
            CombatError::NotACreature(id) => {
                write!(f, "Object {} is not a creature", object_label(id))
            }
            CombatError::NotControlledBy { creature, expected } => {
                write!(
                    f,
                    "Creature {} is not controlled by player {}",
                    object_label(creature),
                    expected.0
                )
            }
            CombatError::InvalidBlockerOrder {
                attacker,
                expected_blockers,
                provided_blockers,
            } => {
                write!(
                    f,
                    "Invalid blocker order for attacker {}: expected [{}], got [{}]",
                    object_label(attacker),
                    object_list(expected_blockers),
                    object_list(provided_blockers)
                )
            }
            CombatError::DuplicateAttacker(id) => {
                write!(
                    f,
                    "Creature {} was declared as an attacker multiple times",
                    object_label(id)
                )
            }
            CombatError::DuplicateBlocker(id) => {
                write!(
                    f,
                    "Creature {} was declared as blocking multiple attackers",
                    object_label(id)
                )
            }
            CombatError::AttackerNotFound(id) => {
                write!(f, "Attacker {} not found", object_label(id))
            }
            CombatError::MustAttackNotDeclared(id) => {
                write!(
                    f,
                    "Creature {} must attack this combat if able but was not declared",
                    object_label(id)
                )
            }
            CombatError::MustBlockRequirementNotMet { blocker, attacker } => {
                write!(
                    f,
                    "Creature {} must block {} this combat if able but was not declared",
                    object_label(blocker),
                    object_label(attacker)
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
    combat.had_to_attack_this_combat.clear();
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
        return calc.static_abilities.to_vec();
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

pub(crate) fn max_creatures_can_attack_each_combat(game: &GameState) -> Option<usize> {
    battlefield_static_abilities(game)
        .iter()
        .filter_map(|ability| ability.max_creatures_can_attack_each_combat())
        .min()
}

pub(crate) fn max_creatures_can_block_each_combat(game: &GameState) -> Option<usize> {
    battlefield_static_abilities(game)
        .iter()
        .filter_map(|ability| ability.max_creatures_can_block_each_combat())
        .min()
}

pub(crate) fn max_creatures_can_attack_defending_player_each_combat(
    game: &GameState,
    defending_player: PlayerId,
) -> Option<usize> {
    let all_effects = game.all_continuous_effects();
    game.battlefield
        .iter()
        .filter_map(|&object_id| {
            let object = game.object(object_id)?;
            (game.controller_of(object) == defending_player).then_some(object_id)
        })
        .flat_map(|object_id| static_abilities_for_object(game, object_id, &all_effects))
        .filter_map(|ability| ability.max_creatures_can_attack_you_each_combat())
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
        if game.controller_of(creature) != active_player {
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
                game.controller_of(pw)
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
                game.controller_of(creature),
                &declared_attackers,
            ) && !can_attack
            {
                return Err(CombatError::CreatureCannotAttack(*creature_id));
            }
            if let Some(can_pay) =
                ability.can_pay_attack_cost(game, creature.id, game.controller_of(creature))
                && !can_pay
            {
                return Err(CombatError::CreatureCannotAttack(*creature_id));
            }
            if let Some(cost) = ability.generic_attack_mana_cost_for_source(
                game,
                creature.id,
                game.controller_of(creature),
            ) {
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

    let mut attackers_per_defender: HashMap<PlayerId, usize> = HashMap::new();
    for (_attacker, target) in &declarations {
        let defending_player = match target {
            AttackTarget::Player(player_id) => *player_id,
            AttackTarget::Planeswalker(pw_id) => {
                let pw = game
                    .object(*pw_id)
                    .ok_or_else(|| CombatError::InvalidAttackTarget(target.clone()))?;
                game.controller_of(pw)
            }
        };
        *attackers_per_defender.entry(defending_player).or_insert(0) += 1;
    }

    for (defending_player, provided) in attackers_per_defender {
        if let Some(maximum) =
            max_creatures_can_attack_defending_player_each_combat(game, defending_player)
            && provided > maximum
        {
            return Err(CombatError::TooManyAttackers { maximum, provided });
        }
    }

    // Pay non-mana attacker costs from "can't attack unless ..." restrictions.
    for (creature_id, _target) in &declarations {
        let Some(creature) = game.object(*creature_id) else {
            return Err(CombatError::NotOnBattlefield(*creature_id));
        };
        let creature_source = creature.id;
        let creature_controller = game.controller_of(creature);
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

    let had_to_attack: HashSet<ObjectId> = declarations
        .iter()
        .filter_map(|(creature_id, _)| {
            let creature = game.object(*creature_id)?;
            crate::rules::combat::must_attack_with_game(creature, game).then_some(*creature_id)
        })
        .collect();

    // Second pass: apply declarations and tap attackers without vigilance
    for (creature_id, target) in declarations {
        // Add to attackers list
        combat.attackers.push(AttackerInfo {
            creature: creature_id,
            target,
        });

        // Initialize empty blocker list
        combat.blockers.insert(creature_id, Vec::new());
        if had_to_attack.contains(&creature_id) {
            combat.had_to_attack_this_combat.insert(creature_id);
        }

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
    declare_blockers_internal(game, combat, declarations, true)
}

fn declare_blockers_internal(
    game: &GameState,
    combat: &mut CombatState,
    declarations: Vec<(ObjectId, ObjectId)>,
    enforce_requirements: bool,
) -> Result<(), CombatError> {
    let all_effects = game.all_continuous_effects();

    // Group blockers by attacker for menace validation
    let mut blockers_by_attacker: HashMap<ObjectId, Vec<ObjectId>> = HashMap::new();
    let mut attackers_by_blocker: HashMap<ObjectId, Vec<ObjectId>> = HashMap::new();
    let mut blocker_counts: HashMap<ObjectId, usize> = HashMap::new();

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

        if defending_player_for_attacker(game, combat, *attacker_id)
            != Some(game.controller_of(blocker))
        {
            return Err(CombatError::CreatureCannotBlock {
                blocker: *blocker_id,
                attacker: *attacker_id,
            });
        }

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

    propagate_banding_blocks(combat, &mut blockers_by_attacker);

    let blocking_creature_count = blocker_counts.len();
    if blocking_creature_count == 1
        && let Some(&blocker) = blocker_counts.keys().next()
        && !game.can_block_alone(blocker)
    {
        let attacker = attackers_by_blocker
            .get(&blocker)
            .and_then(|attackers| attackers.first())
            .copied()
            .unwrap_or(blocker);
        return Err(CombatError::CreatureCannotBlock { blocker, attacker });
    }
    // With no declared blockers, every global "at most N creatures can
    // block" restriction is vacuously satisfied. Avoid deriving the static
    // abilities of the whole battlefield just to rediscover that fact.
    if blocking_creature_count > 0
        && let Some(max_blockers) = max_creatures_can_block_each_combat(game)
        && blocking_creature_count > max_blockers
    {
        return Err(CombatError::TooManyBlockingCreatures {
            maximum: max_blockers,
            provided: blocking_creature_count,
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

    if enforce_requirements {
        let requirements = blocking_requirements(game, combat, &all_effects);
        let requirements_obeyed = blocking_requirement_score(
            combat,
            &requirements,
            &blockers_by_attacker,
            &attackers_by_blocker,
        );
        if block_declaration_obeying_more_requirements_exists(
            game,
            combat,
            &all_effects,
            &requirements,
            requirements_obeyed,
        ) {
            return Err(first_unsatisfied_block_requirement_error(
                game,
                combat,
                &all_effects,
                &requirements,
                &blockers_by_attacker,
                &attackers_by_blocker,
            ));
        }
    }

    // Third pass: apply declarations
    for (attacker_id, blocker_list) in blockers_by_attacker {
        combat.blockers.insert(attacker_id, blocker_list);
    }

    Ok(())
}

fn max_attackers_this_blocker_can_block(
    game: &GameState,
    blocker_id: ObjectId,
    effects: &[crate::continuous::ContinuousEffect],
) -> usize {
    let extra = static_abilities_for_object(game, blocker_id, effects)
        .iter()
        .filter_map(|ability| ability.additional_blockable_attackers())
        .sum::<usize>();
    1usize.saturating_add(extra)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockingRequirement {
    AttackerMustBeBlocked(ObjectId),
    BlockerMustBlock(ObjectId),
    BlockerMustBlockAttacker {
        blocker: ObjectId,
        attacker: ObjectId,
    },
}

fn game_may_have_must_block_requirements(
    game: &GameState,
    effects: &[crate::continuous::ContinuousEffect],
) -> bool {
    use crate::static_abilities::StaticAbilityId;

    let raw_ability_can_require_or_grant_blocking = game.battlefield.iter().copied().any(|id| {
        game.object(id).is_some_and(|object| {
            object.abilities.iter().any(|ability| {
                let crate::ability::AbilityKind::Static(ability) = &ability.kind else {
                    return false;
                };
                matches!(
                    ability.id(),
                    StaticAbilityId::MustBlock
                        | StaticAbilityId::GrantAbility
                        | StaticAbilityId::AttachedAbilityGrant
                        | StaticAbilityId::GrantObjectAbilityForFilter
                )
            })
        })
    });

    raw_ability_can_require_or_grant_blocking
        || effects
            .iter()
            .any(|effect| effect.modification.layer() == crate::continuous::Layer::Ability)
}

fn blocking_requirements(
    game: &GameState,
    combat: &CombatState,
    effects: &[crate::continuous::ContinuousEffect],
) -> Vec<BlockingRequirement> {
    let mut requirements = Vec::new();
    let defending_players = combat
        .attackers
        .iter()
        .filter_map(|attacker| defending_player_for_attack_target(game, &attacker.target))
        .collect::<HashSet<_>>();

    for attacker in &combat.attackers {
        if game.must_be_blocked(attacker.creature) {
            requirements.push(BlockingRequirement::AttackerMustBeBlocked(
                attacker.creature,
            ));
        }
    }

    if game_may_have_must_block_requirements(game, effects) {
        for &blocker in &game.battlefield {
            let Some(object) = game.object(blocker) else {
                continue;
            };
            if defending_players.contains(&game.controller_of(object)) {
                let requirement_count = static_abilities_for_object(game, blocker, effects)
                    .iter()
                    .filter(|ability| {
                        ability.id() == crate::static_abilities::StaticAbilityId::MustBlock
                    })
                    .count();
                requirements.extend(std::iter::repeat_n(
                    BlockingRequirement::BlockerMustBlock(blocker),
                    requirement_count,
                ));
            }
        }
    }

    for (&blocker, attackers) in &game.effect_store.cant_effects.must_block_specific_attackers {
        for &attacker in attackers {
            if is_attacking(combat, attacker) {
                requirements
                    .push(BlockingRequirement::BlockerMustBlockAttacker { blocker, attacker });
            }
        }
    }

    requirements
}

fn propagated_block_maps(
    combat: &CombatState,
    blockers_by_attacker: &HashMap<ObjectId, Vec<ObjectId>>,
) -> (
    HashMap<ObjectId, Vec<ObjectId>>,
    HashMap<ObjectId, Vec<ObjectId>>,
) {
    let mut propagated = blockers_by_attacker.clone();
    propagate_banding_blocks(combat, &mut propagated);
    let mut attackers_by_blocker: HashMap<ObjectId, Vec<ObjectId>> = HashMap::new();
    for (&attacker, blockers) in &propagated {
        for &blocker in blockers {
            let attackers = attackers_by_blocker.entry(blocker).or_default();
            if !attackers.contains(&attacker) {
                attackers.push(attacker);
            }
        }
    }
    (propagated, attackers_by_blocker)
}

fn block_requirement_is_obeyed(
    requirement: BlockingRequirement,
    blockers_by_attacker: &HashMap<ObjectId, Vec<ObjectId>>,
    attackers_by_blocker: &HashMap<ObjectId, Vec<ObjectId>>,
) -> bool {
    match requirement {
        BlockingRequirement::AttackerMustBeBlocked(attacker) => blockers_by_attacker
            .get(&attacker)
            .is_some_and(|blockers| !blockers.is_empty()),
        BlockingRequirement::BlockerMustBlock(blocker) => attackers_by_blocker
            .get(&blocker)
            .is_some_and(|attackers| !attackers.is_empty()),
        BlockingRequirement::BlockerMustBlockAttacker { blocker, attacker } => attackers_by_blocker
            .get(&blocker)
            .is_some_and(|attackers| attackers.contains(&attacker)),
    }
}

fn blocking_requirement_score(
    combat: &CombatState,
    requirements: &[BlockingRequirement],
    blockers_by_attacker: &HashMap<ObjectId, Vec<ObjectId>>,
    _attackers_by_blocker: &HashMap<ObjectId, Vec<ObjectId>>,
) -> usize {
    let (blockers_by_attacker, attackers_by_blocker) =
        propagated_block_maps(combat, blockers_by_attacker);
    requirements
        .iter()
        .filter(|&&requirement| {
            block_requirement_is_obeyed(requirement, &blockers_by_attacker, &attackers_by_blocker)
        })
        .count()
}

fn legal_block_edge(
    game: &GameState,
    combat: &CombatState,
    effects: &[crate::continuous::ContinuousEffect],
    blocker_id: ObjectId,
    attacker_id: ObjectId,
) -> bool {
    let (Some(blocker), Some(attacker)) = (game.object(blocker_id), game.object(attacker_id))
    else {
        return false;
    };
    blocker.zone == Zone::Battlefield
        && attacker.zone == Zone::Battlefield
        && is_attacking(combat, attacker_id)
        && defending_player_for_attacker(game, combat, attacker_id)
            == Some(game.controller_of(blocker))
        && game.object_has_card_type_with_effects(
            blocker_id,
            crate::types::CardType::Creature,
            effects,
        )
        && !game.is_tapped(blocker_id)
        && !game.object_has_ability_with_effects(blocker_id, &StaticAbility::cant_block(), effects)
        && game.can_block_attacker(blocker_id, attacker_id)
        && game.can_be_blocked(attacker_id)
        && can_block(attacker, blocker, game)
}

fn attackers_share_band(combat: &CombatState, first: ObjectId, second: ObjectId) -> bool {
    first == second
        || combat
            .attacking_bands
            .iter()
            .any(|band| band.contains(&first) && band.contains(&second))
}

fn block_edge_can_obey_requirement(
    combat: &CombatState,
    edge: (ObjectId, ObjectId),
    requirement: BlockingRequirement,
) -> bool {
    let (blocker, attacker) = edge;
    match requirement {
        BlockingRequirement::AttackerMustBeBlocked(required_attacker) => {
            attackers_share_band(combat, attacker, required_attacker)
        }
        BlockingRequirement::BlockerMustBlock(required_blocker) => blocker == required_blocker,
        BlockingRequirement::BlockerMustBlockAttacker {
            blocker: required_blocker,
            attacker: required_attacker,
        } => {
            blocker == required_blocker && attackers_share_band(combat, attacker, required_attacker)
        }
    }
}

fn block_declaration_obeying_more_requirements_exists(
    game: &GameState,
    combat: &CombatState,
    effects: &[crate::continuous::ContinuousEffect],
    requirements: &[BlockingRequirement],
    baseline: usize,
) -> bool {
    if baseline >= requirements.len() {
        return false;
    }

    let mut legal_edges = Vec::new();
    for &blocker in &game.battlefield {
        for attacker in &combat.attackers {
            let edge = (blocker, attacker.creature);
            if legal_block_edge(game, combat, effects, blocker, attacker.creature) {
                legal_edges.push(edge);
            }
        }
    }
    let requirement_relevant_attackers = legal_edges
        .iter()
        .filter(|&&edge| {
            requirements
                .iter()
                .any(|&requirement| block_edge_can_obey_requirement(combat, edge, requirement))
        })
        .map(|&(_, attacker)| attacker)
        .collect::<HashSet<_>>();
    let mut candidates = legal_edges
        .into_iter()
        .filter(|&(blocker, attacker)| {
            requirements.iter().any(|&requirement| {
                block_edge_can_obey_requirement(combat, (blocker, attacker), requirement)
            }) || requirement_relevant_attackers
                .iter()
                .any(|&relevant| attackers_share_band(combat, attacker, relevant))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|&(blocker, attacker)| {
        let covered = requirements
            .iter()
            .filter(|&&requirement| {
                block_edge_can_obey_requirement(combat, (blocker, attacker), requirement)
            })
            .count();
        (std::cmp::Reverse(covered), blocker.0, attacker.0)
    });

    fn maps_for_declarations(
        declarations: &[(ObjectId, ObjectId)],
    ) -> (
        HashMap<ObjectId, Vec<ObjectId>>,
        HashMap<ObjectId, Vec<ObjectId>>,
    ) {
        let mut blockers_by_attacker: HashMap<ObjectId, Vec<ObjectId>> = HashMap::new();
        let mut attackers_by_blocker: HashMap<ObjectId, Vec<ObjectId>> = HashMap::new();
        for &(blocker, attacker) in declarations {
            blockers_by_attacker
                .entry(attacker)
                .or_default()
                .push(blocker);
            attackers_by_blocker
                .entry(blocker)
                .or_default()
                .push(attacker);
        }
        (blockers_by_attacker, attackers_by_blocker)
    }

    fn search(
        game: &GameState,
        combat: &CombatState,
        effects: &[crate::continuous::ContinuousEffect],
        requirements: &[BlockingRequirement],
        candidates: &[(ObjectId, ObjectId)],
        index: usize,
        declarations: &mut Vec<(ObjectId, ObjectId)>,
        baseline: usize,
    ) -> bool {
        let (blockers_by_attacker, attackers_by_blocker) = maps_for_declarations(declarations);
        let score = blocking_requirement_score(
            combat,
            requirements,
            &blockers_by_attacker,
            &attackers_by_blocker,
        );
        if score > baseline {
            let mut candidate_combat = combat.clone();
            if declare_blockers_internal(game, &mut candidate_combat, declarations.clone(), false)
                .is_ok()
            {
                return true;
            }
        }
        if index == candidates.len() {
            return false;
        }

        let (propagated_blockers, propagated_attackers) =
            propagated_block_maps(combat, &blockers_by_attacker);
        let optimistic_score = requirements
            .iter()
            .filter(|&&requirement| {
                block_requirement_is_obeyed(
                    requirement,
                    &propagated_blockers,
                    &propagated_attackers,
                ) || candidates[index..]
                    .iter()
                    .any(|&edge| block_edge_can_obey_requirement(combat, edge, requirement))
            })
            .count();
        if optimistic_score <= baseline {
            return false;
        }

        let (blocker, attacker) = candidates[index];
        let blocker_count = attackers_by_blocker.get(&blocker).map_or(0, Vec::len);
        let attacker_count = blockers_by_attacker.get(&attacker).map_or(0, Vec::len);
        let distinct_blockers = attackers_by_blocker.len();
        let introduces_blocker = !attackers_by_blocker.contains_key(&blocker);
        let within_global_cap = max_creatures_can_block_each_combat(game)
            .is_none_or(|maximum| distinct_blockers + usize::from(introduces_blocker) <= maximum);
        let within_blocker_capacity =
            blocker_count < max_attackers_this_blocker_can_block(game, blocker, effects);
        let within_attacker_capacity = game
            .object(attacker)
            .and_then(|object| maximum_blockers(object, game))
            .is_none_or(|maximum| attacker_count < maximum);

        if within_global_cap && within_blocker_capacity && within_attacker_capacity {
            declarations.push((blocker, attacker));
            if search(
                game,
                combat,
                effects,
                requirements,
                candidates,
                index + 1,
                declarations,
                baseline,
            ) {
                return true;
            }
            declarations.pop();
        }

        search(
            game,
            combat,
            effects,
            requirements,
            candidates,
            index + 1,
            declarations,
            baseline,
        )
    }

    search(
        game,
        combat,
        effects,
        requirements,
        &candidates,
        0,
        &mut Vec::new(),
        baseline,
    )
}

fn first_unsatisfied_block_requirement_error(
    game: &GameState,
    combat: &CombatState,
    effects: &[crate::continuous::ContinuousEffect],
    requirements: &[BlockingRequirement],
    blockers_by_attacker: &HashMap<ObjectId, Vec<ObjectId>>,
    _attackers_by_blocker: &HashMap<ObjectId, Vec<ObjectId>>,
) -> CombatError {
    let (blockers_by_attacker, attackers_by_blocker) =
        propagated_block_maps(combat, blockers_by_attacker);
    for &requirement in requirements {
        if block_requirement_is_obeyed(requirement, &blockers_by_attacker, &attackers_by_blocker) {
            continue;
        }
        match requirement {
            BlockingRequirement::AttackerMustBeBlocked(attacker) => {
                let can_obey = game.battlefield.iter().copied().any(|blocker| {
                    combat.attackers.iter().any(|candidate| {
                        attackers_share_band(combat, candidate.creature, attacker)
                            && legal_block_edge(game, combat, effects, blocker, candidate.creature)
                    })
                });
                if !can_obey {
                    continue;
                }
                let required = game
                    .object(attacker)
                    .map(|object| minimum_blockers_with_game(object, game))
                    .unwrap_or(1);
                return CombatError::NotEnoughBlockers {
                    attacker,
                    required,
                    provided: 0,
                };
            }
            BlockingRequirement::BlockerMustBlockAttacker { blocker, attacker } => {
                let can_obey = combat.attackers.iter().any(|candidate| {
                    attackers_share_band(combat, candidate.creature, attacker)
                        && legal_block_edge(game, combat, effects, blocker, candidate.creature)
                });
                if !can_obey {
                    continue;
                }
                return CombatError::MustBlockRequirementNotMet { blocker, attacker };
            }
            BlockingRequirement::BlockerMustBlock(blocker) => {
                if let Some(attacker) = combat.attackers.iter().find_map(|attacker| {
                    legal_block_edge(game, combat, effects, blocker, attacker.creature)
                        .then_some(attacker.creature)
                }) {
                    return CombatError::MustBlockRequirementNotMet { blocker, attacker };
                }
            }
        }
    }
    unreachable!("a better blocking declaration requires an unsatisfied requirement")
}

/// Records an attacking band for the current combat.
///
/// This covers rule 702.22c for ordinary banding: a band may contain one or
/// more attacking creatures with banding and up to one attacking creature
/// without banding. "Bands with other" has extra quality constraints and is
/// intentionally not modeled here yet.
pub fn set_attacking_band(
    game: &GameState,
    combat: &mut CombatState,
    members: Vec<ObjectId>,
) -> Result<(), CombatError> {
    if members.len() < 2 {
        return Ok(());
    }

    let mut deduped = Vec::new();
    for member in members {
        if !deduped.contains(&member) {
            deduped.push(member);
        }
    }

    for member in &deduped {
        if !is_attacking(combat, *member) {
            return Err(CombatError::NotInCombat(*member));
        }
        if combat
            .attacking_bands
            .iter()
            .any(|band| band.contains(member))
        {
            return Err(CombatError::DuplicateAttacker(*member));
        }
    }

    let banding_count = deduped
        .iter()
        .filter(|&&member| creature_has_banding(game, member))
        .count();
    if banding_count == 0 || deduped.len().saturating_sub(banding_count) > 1 {
        return Err(CombatError::CreatureCannotAttack(deduped[0]));
    }

    let Some(first_target) = get_attack_target(combat, deduped[0]).cloned() else {
        return Err(CombatError::NotInCombat(deduped[0]));
    };
    if deduped
        .iter()
        .skip(1)
        .any(|member| get_attack_target(combat, *member) != Some(&first_target))
    {
        return Err(CombatError::InvalidAttackTarget(first_target));
    }

    combat.attacking_bands.push(deduped);
    Ok(())
}

fn creature_has_banding(game: &GameState, creature: ObjectId) -> bool {
    let all_effects = game.all_continuous_effects();
    static_abilities_for_object(game, creature, &all_effects)
        .iter()
        .any(|ability| ability.id() == crate::static_abilities::StaticAbilityId::Banding)
}

fn propagate_banding_blocks(
    combat: &CombatState,
    blockers_by_attacker: &mut HashMap<ObjectId, Vec<ObjectId>>,
) {
    for band in &combat.attacking_bands {
        let mut shared_blockers = Vec::new();
        for attacker in band {
            if let Some(blockers) = blockers_by_attacker.get(attacker) {
                for blocker in blockers {
                    if !shared_blockers.contains(blocker) {
                        shared_blockers.push(*blocker);
                    }
                }
            }
        }
        if shared_blockers.is_empty() {
            continue;
        }
        for attacker in band {
            let blockers = blockers_by_attacker.entry(*attacker).or_default();
            for blocker in &shared_blockers {
                if !blockers.contains(blocker) {
                    blockers.push(*blocker);
                }
            }
        }
    }
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

/// Return the defending player associated with an attack target.
pub fn defending_player_for_attack_target(
    game: &GameState,
    target: &AttackTarget,
) -> Option<PlayerId> {
    match target {
        AttackTarget::Player(player) => Some(*player),
        AttackTarget::Planeswalker(planeswalker) => game.controller_of_id(*planeswalker),
    }
}

/// Return the defending player associated with an attacking creature.
pub fn defending_player_for_attacker(
    game: &GameState,
    combat: &CombatState,
    attacker: ObjectId,
) -> Option<PlayerId> {
    defending_player_for_attack_target(game, get_attack_target(combat, attacker)?)
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
            .is_some_and(|obj| game.controller_of(obj) == player)
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
        .map(|obj| game.controller_of(obj))
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
    fn declare_blockers_enforces_must_be_blocked_if_able() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = creature_card("Must Be Blocked", 2, 2);
        let blocker = creature_card("Available Blocker", 2, 2);
        let attacker_id = game.create_object_from_card(&attacker, alice, Zone::Battlefield);
        let blocker_id = game.create_object_from_card(&blocker, bob, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);
        game.effect_store
            .cant_effects
            .must_be_blocked
            .insert(attacker_id);

        let mut combat = CombatState::default();
        declare_attackers(
            &mut game,
            &mut combat,
            vec![(attacker_id, AttackTarget::Player(bob))],
        )
        .expect("attacker should be declared");

        let missing_block = declare_blockers(&mut game, &mut combat.clone(), Vec::new());
        assert_eq!(
            missing_block,
            Err(CombatError::NotEnoughBlockers {
                attacker: attacker_id,
                required: 1,
                provided: 0,
            })
        );

        declare_blockers(&mut game, &mut combat, vec![(blocker_id, attacker_id)])
            .expect("blocking the required attacker should satisfy the requirement");
    }

    #[test]
    fn declare_blockers_ignores_must_be_blocked_when_no_blocker_can_block() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = creature_card("Must Be Blocked", 2, 2);
        let tapped_blocker = creature_card("Tapped Blocker", 2, 2);
        let attacker_id = game.create_object_from_card(&attacker, alice, Zone::Battlefield);
        let blocker_id = game.create_object_from_card(&tapped_blocker, bob, Zone::Battlefield);
        game.tap(blocker_id);
        game.remove_summoning_sickness(attacker_id);
        game.effect_store
            .cant_effects
            .must_be_blocked
            .insert(attacker_id);

        let mut combat = CombatState::default();
        declare_attackers(
            &mut game,
            &mut combat,
            vec![(attacker_id, AttackTarget::Player(bob))],
        )
        .expect("attacker should be declared");

        declare_blockers(&mut game, &mut combat, Vec::new())
            .expect("no block should be required when no creature can block");
    }

    #[test]
    fn conflicting_must_be_blocked_requirements_use_the_maximum_satisfiable_count() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first = game.create_object_from_card(
            &creature_card("First Required Block", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let second = game.create_object_from_card(
            &creature_card("Second Required Block", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let blocker = game.create_object_from_card(
            &creature_card("Only Blocker", 2, 2),
            bob,
            Zone::Battlefield,
        );
        game.remove_summoning_sickness(first);
        game.remove_summoning_sickness(second);
        game.effect_store.cant_effects.must_be_blocked.insert(first);
        game.effect_store
            .cant_effects
            .must_be_blocked
            .insert(second);

        let mut combat = CombatState::default();
        declare_attackers(
            &mut game,
            &mut combat,
            vec![
                (first, AttackTarget::Player(bob)),
                (second, AttackTarget::Player(bob)),
            ],
        )
        .expect("both attackers should be declared");

        declare_blockers(&game, &mut combat, vec![(blocker, first)]).expect(
            "one blocker may obey either one of two mutually exclusive blocking requirements",
        );
    }

    #[test]
    fn conflicting_specific_must_block_requirements_respect_blocker_capacity() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first = game.create_object_from_card(
            &creature_card("First Forced Target", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let second = game.create_object_from_card(
            &creature_card("Second Forced Target", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let blocker = game.create_object_from_card(
            &creature_card("One-Capacity Blocker", 2, 2),
            bob,
            Zone::Battlefield,
        );
        game.remove_summoning_sickness(first);
        game.remove_summoning_sickness(second);
        game.effect_store
            .cant_effects
            .must_block_specific_attackers
            .entry(blocker)
            .or_default()
            .extend([first, second]);

        let mut combat = CombatState::default();
        declare_attackers(
            &mut game,
            &mut combat,
            vec![
                (first, AttackTarget::Player(bob)),
                (second, AttackTarget::Player(bob)),
            ],
        )
        .expect("both attackers should be declared");

        declare_blockers(&game, &mut combat, vec![(blocker, first)]).expect(
            "a one-capacity blocker may obey either one of two conflicting specific requirements",
        );
    }

    #[test]
    fn must_block_can_require_an_optional_second_blocker_for_menace() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = game.create_object_from_card(
            &creature_card("Menacing Attacker", 2, 2),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(attacker)
            .expect("attacker exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::menace()));
        let required_blocker = game.create_object_from_card(
            &creature_card("Required Blocker", 2, 2),
            bob,
            Zone::Battlefield,
        );
        game.object_mut(required_blocker)
            .expect("blocker exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::must_block()));
        let optional_blocker = game.create_object_from_card(
            &creature_card("Optional Blocker", 2, 2),
            bob,
            Zone::Battlefield,
        );
        game.remove_summoning_sickness(attacker);
        game.refresh_continuous_state();

        let mut combat = CombatState::default();
        declare_attackers(
            &mut game,
            &mut combat,
            vec![(attacker, AttackTarget::Player(bob))],
        )
        .expect("attacker should be declared");

        assert!(
            declare_blockers(&game, &mut combat.clone(), Vec::new()).is_err(),
            "the must-block requirement can be obeyed by also declaring the optional blocker"
        );
        declare_blockers(
            &game,
            &mut combat,
            vec![(required_blocker, attacker), (optional_blocker, attacker)],
        )
        .expect("both blockers satisfy menace and the maximum blocking requirement count");
    }

    #[test]
    fn blocker_optimizer_counts_multiple_requirements_on_one_creature() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = game.create_object_from_card(
            &creature_card("Attacker", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let two_requirements = game.create_object_from_card(
            &creature_card("Twice Required Blocker", 2, 2),
            bob,
            Zone::Battlefield,
        );
        let one_requirement = game.create_object_from_card(
            &creature_card("Once Required Blocker", 2, 2),
            bob,
            Zone::Battlefield,
        );
        for _ in 0..2 {
            game.object_mut(two_requirements)
                .expect("blocker exists")
                .abilities_mut()
                .push(Ability::static_ability(StaticAbility::must_block()));
        }
        game.object_mut(one_requirement)
            .expect("blocker exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::must_block()));
        let limiter = game.create_object_from_card(
            &enchantment_card("One Blocker Limit"),
            bob,
            Zone::Battlefield,
        );
        game.object_mut(limiter)
            .expect("limiter exists")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::max_blockers_each_combat(1),
            ));
        game.remove_summoning_sickness(attacker);
        game.refresh_continuous_state();

        let mut combat = CombatState::default();
        declare_attackers(
            &mut game,
            &mut combat,
            vec![(attacker, AttackTarget::Player(bob))],
        )
        .expect("attacker should be declared");

        assert!(
            declare_blockers(
                &game,
                &mut combat.clone(),
                vec![(one_requirement, attacker)]
            )
            .is_err(),
            "one obeyed requirement is illegal when two can be obeyed"
        );
        declare_blockers(&game, &mut combat, vec![(two_requirements, attacker)])
            .expect("the blocker obeying two requirements should be the legal blocker");
    }

    #[test]
    fn empty_blocker_declaration_skips_global_blocker_limit_scan() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature = creature_card("Large Board Creature", 2, 2);
        let ids = (0..64)
            .map(|_| game.create_object_from_card(&creature, alice, Zone::Battlefield))
            .collect::<Vec<_>>();
        game.effect_store
            .continuous_effects
            .add_effect(crate::continuous::ContinuousEffect::new(
                ids[0],
                alice,
                crate::continuous::EffectTarget::AllCreatures,
                crate::continuous::Modification::ModifyPowerToughness {
                    power: 1,
                    toughness: 1,
                },
            ));
        game.refresh_continuous_state();
        let before = game.work_counters();
        let mut combat = CombatState {
            attackers: vec![AttackerInfo {
                creature: ids[0],
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        };

        declare_blockers(&game, &mut combat, Vec::new())
            .expect("an empty blocker declaration is always within an upper limit");

        let after = game.work_counters();
        assert_eq!(
            after.characteristics_full_recomputes, before.characteristics_full_recomputes,
            "zero blockers should not derive every battlefield object's abilities"
        );
        assert_eq!(
            after.dependency_sorts, before.dependency_sorts,
            "zero blockers should not run per-object dependency sorting"
        );
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
            .abilities_mut().push(Ability::static_ability(
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
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::cant_attack_unless_condition(
                    CantAttackUnlessConditionSpec::AttackCost(
                        crate::static_abilities::AttackCostCondition::SacrificePermanents {
                            filter: crate::filter::ObjectFilter::land(),
                            count: 1,
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
            .abilities_mut().push(Ability::static_ability(
                StaticAbility::cant_attack_unless_condition(
                    CantAttackUnlessConditionSpec::AttackCost(
                        crate::static_abilities::AttackCostCondition::ReturnPermanentsToOwnersHand {
                            filter: crate::filter::ObjectFilter::enchantment(),
                            count: 1,
                        },
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
            attacker_obj.abilities_mut().push(Ability::static_ability(
                StaticAbility::cant_attack_unless_condition(
                    CantAttackUnlessConditionSpec::AttackCost(
                        crate::static_abilities::AttackCostCondition::PayGenericPerSourceCounter {
                            counter_type: CounterType::PlusOnePlusOne,
                            amount_per_counter: 1,
                        },
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

    #[test]
    fn declare_attackers_enforces_crawlspace_style_cap_against_its_controller() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let crawlspace_like = enchantment_card("Crawlspace");
        let crawlspace_id = game.create_object_from_card(&crawlspace_like, bob, Zone::Battlefield);
        game.object_mut(crawlspace_id)
            .expect("crawlspace object should exist")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::max_attackers_can_attack_you_each_combat(2),
            ));

        let attacker_one = game.create_object_from_card(
            &creature_card("Attacker One", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let attacker_two = game.create_object_from_card(
            &creature_card("Attacker Two", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let attacker_three = game.create_object_from_card(
            &creature_card("Attacker Three", 2, 2),
            alice,
            Zone::Battlefield,
        );
        game.remove_summoning_sickness(attacker_one);
        game.remove_summoning_sickness(attacker_two);
        game.remove_summoning_sickness(attacker_three);

        let result = declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![
                (attacker_one, AttackTarget::Player(bob)),
                (attacker_two, AttackTarget::Player(bob)),
                (attacker_three, AttackTarget::Player(bob)),
            ],
        );
        assert_eq!(
            result,
            Err(CombatError::TooManyAttackers {
                maximum: 2,
                provided: 3,
            })
        );
    }

    #[test]
    fn declare_attackers_crawlspace_style_cap_does_not_limit_other_defenders() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);

        let crawlspace_like = enchantment_card("Crawlspace");
        let crawlspace_id = game.create_object_from_card(&crawlspace_like, bob, Zone::Battlefield);
        game.object_mut(crawlspace_id)
            .expect("crawlspace object should exist")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::max_attackers_can_attack_you_each_combat(2),
            ));

        let attacker_one = game.create_object_from_card(
            &creature_card("Attacker One", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let attacker_two = game.create_object_from_card(
            &creature_card("Attacker Two", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let attacker_three = game.create_object_from_card(
            &creature_card("Attacker Three", 2, 2),
            alice,
            Zone::Battlefield,
        );
        game.remove_summoning_sickness(attacker_one);
        game.remove_summoning_sickness(attacker_two);
        game.remove_summoning_sickness(attacker_three);

        let result = declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![
                (attacker_one, AttackTarget::Player(cara)),
                (attacker_two, AttackTarget::Player(cara)),
                (attacker_three, AttackTarget::Player(cara)),
            ],
        );
        assert!(
            result.is_ok(),
            "expected three attackers against another defender to be legal"
        );
    }
}

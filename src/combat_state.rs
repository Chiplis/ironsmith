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
    let mut seen_blockers = std::collections::HashSet::new();

    // First pass: validate all blockers
    for (blocker_id, attacker_id) in &declarations {
        // Check for duplicate blockers (creature blocking multiple attackers)
        if !seen_blockers.insert(*blocker_id) {
            return Err(CombatError::DuplicateBlocker(*blocker_id));
        }

        // Validate blocker exists and is on battlefield
        let blocker = game
            .object(*blocker_id)
            .ok_or(CombatError::NotOnBattlefield(*blocker_id))?;

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
    use crate::object::Object;
    use crate::ability::Ability;
    use crate::card::PtValue;
    use crate::color::ColorSet;
    use crate::cost::OptionalCostsPaid;
    use crate::ids::StableId;
    use crate::static_abilities::StaticAbility;
    use crate::types::{CardType, Subtype};
    use std::collections::HashMap;

    fn make_creature(id: u64, owner: u8, name: &str) -> Object {
        Object {
            id: ObjectId::from_raw(id),
            stable_id: StableId::from_raw(id),
            kind: crate::object::ObjectKind::Card,
            card: None,
            zone: Zone::Battlefield,
            owner: PlayerId::from_index(owner),
            controller: PlayerId::from_index(owner),
            name: name.to_string(),
            mana_cost: None,
            color_override: None,
            supertypes: vec![],
            card_types: vec![CardType::Creature],
            subtypes: vec![],
            oracle_text: String::new(),
            base_power: Some(PtValue::Fixed(2)),
            base_toughness: Some(PtValue::Fixed(2)),
            base_loyalty: None,
            abilities: vec![],
            counters: HashMap::new(),
            attached_to: None,
            attachments: vec![],
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: vec![],
            optional_costs: vec![],
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: crate::player::ManaPool::default(),
            cost_effects: vec![],
            max_saga_chapter: None,
        }
    }

    fn make_planeswalker(id: u64, owner: u8, name: &str) -> Object {
        Object {
            id: ObjectId::from_raw(id),
            stable_id: StableId::from_raw(id),
            kind: crate::object::ObjectKind::Card,
            card: None,
            zone: Zone::Battlefield,
            owner: PlayerId::from_index(owner),
            controller: PlayerId::from_index(owner),
            name: name.to_string(),
            mana_cost: None,
            color_override: Some(ColorSet::BLUE),
            supertypes: vec![],
            card_types: vec![CardType::Planeswalker],
            subtypes: vec![],
            oracle_text: String::new(),
            base_power: None,
            base_toughness: None,
            base_loyalty: Some(3),
            abilities: vec![],
            counters: HashMap::new(),
            attached_to: None,
            attachments: vec![],
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: vec![],
            optional_costs: vec![],
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: crate::player::ManaPool::default(),
            cost_effects: vec![],
            max_saga_chapter: None,
        }
    }

    fn setup_game_with_creatures() -> (GameState, ObjectId, ObjectId, ObjectId, ObjectId) {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

        // Alice's creatures (player 0)
        let attacker1 = make_creature(1, 0, "Attacker 1");
        let attacker2 = make_creature(2, 0, "Attacker 2");

        // Bob's creatures (player 1)
        let blocker1 = make_creature(3, 1, "Blocker 1");
        let blocker2 = make_creature(4, 1, "Blocker 2");

        game.add_object(attacker1);
        game.add_object(attacker2);
        game.add_object(blocker1);
        game.add_object(blocker2);

        (
            game,
            ObjectId::from_raw(1),
            ObjectId::from_raw(2),
            ObjectId::from_raw(3),
            ObjectId::from_raw(4),
        )
    }

    #[test]
    fn test_new_combat() {
        let combat = new_combat();
        assert!(combat.attackers.is_empty());
        assert!(combat.blockers.is_empty());
        assert!(combat.damage_assignment_order.is_empty());
    }

    #[test]
    fn test_end_combat() {
        let mut combat = new_combat();
        combat.attackers.push(AttackerInfo {
            creature: ObjectId::from_raw(1),
            target: AttackTarget::Player(PlayerId::from_index(1)),
        });
        combat
            .blockers
            .insert(ObjectId::from_raw(1), vec![ObjectId::from_raw(2)]);

        end_combat(&mut combat);

        assert!(combat.attackers.is_empty());
        assert!(combat.blockers.is_empty());
        assert!(combat.damage_assignment_order.is_empty());
    }

    #[test]
    fn test_declare_attackers_success() {
        let (mut game, attacker1, attacker2, _, _) = setup_game_with_creatures();
        let mut combat = new_combat();

        let declarations = vec![
            (attacker1, AttackTarget::Player(PlayerId::from_index(1))),
            (attacker2, AttackTarget::Player(PlayerId::from_index(1))),
        ];

        let result = declare_attackers(&mut game, &mut combat, declarations);
        assert!(result.is_ok());

        assert_eq!(combat.attackers.len(), 2);
        assert!(is_attacking(&combat, attacker1));
        assert!(is_attacking(&combat, attacker2));

        // Check that attackers are tapped (no vigilance)
        assert!(game.is_tapped(attacker1));
        assert!(game.is_tapped(attacker2));
    }

    /// Tests that creatures with vigilance don't tap when attacking.
    ///
    /// Scenario: Alice controls Serra Angel (a creature with vigilance) and declares
    /// it as an attacker. Due to vigilance, the creature should not tap when attacking.
    #[test]
    fn test_declare_attackers_vigilance_not_tapped() {
        use crate::cards::definitions::serra_angel;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Serra Angel on battlefield (has flying and vigilance)
        let serra_def = serra_angel();
        let serra_id = game.create_object_from_definition(&serra_def, alice, Zone::Battlefield);
        // Remove summoning sickness so it can attack
        game.remove_summoning_sickness(serra_id);

        let mut combat = new_combat();
        let declarations = vec![(serra_id, AttackTarget::Player(bob))];

        let result = declare_attackers(&mut game, &mut combat, declarations);
        assert!(result.is_ok());

        // Serra Angel has vigilance, so it should NOT be tapped after attacking
        assert!(
            !game.is_tapped(serra_id),
            "Creature with vigilance should not tap when attacking"
        );
    }

    /// Tests that tapped creatures cannot be declared as attackers.
    ///
    /// Scenario: Alice controls a Grizzly Bears that has been tapped (e.g., from
    /// activating a tap ability or attacking on a previous turn). When she tries
    /// to declare it as an attacker, the declaration should fail.
    #[test]
    fn test_declare_attackers_tapped_creature_fails() {
        use crate::cards::definitions::grizzly_bears;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Grizzly Bears on battlefield
        let bears_def = grizzly_bears();
        let bears_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(bears_id);

        // The creature is tapped (simulating it attacked previously or used a tap ability)
        game.tap(bears_id);

        let mut combat = new_combat();
        let declarations = vec![(bears_id, AttackTarget::Player(bob))];

        let result = declare_attackers(&mut game, &mut combat, declarations);
        assert!(
            matches!(result, Err(CombatError::CreatureTapped(_))),
            "Tapped creatures cannot attack"
        );
    }

    /// Tests that summoning sick creatures cannot attack.
    ///
    /// Scenario: Alice casts Grizzly Bears. On the same turn, before the creature
    /// has been under her control since the beginning of her turn, she tries to
    /// attack with it. The attack should fail due to summoning sickness.
    #[test]
    fn test_declare_attackers_summoning_sick_fails() {
        use crate::cards::definitions::grizzly_bears;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Grizzly Bears on battlefield with summoning sickness
        // (simulating it just entered this turn)
        let bears_def = grizzly_bears();
        let bears_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);
        game.set_summoning_sick(bears_id);

        let mut combat = new_combat();
        let declarations = vec![(bears_id, AttackTarget::Player(bob))];

        let result = declare_attackers(&mut game, &mut combat, declarations);
        assert!(
            matches!(result, Err(CombatError::CreatureCannotAttack(_))),
            "Summoning sick creatures cannot attack"
        );
    }

    /// Tests that summoning sick creatures with haste can attack.
    ///
    /// Scenario: Alice casts Goblin Guide (a creature with haste). On the same turn,
    /// despite having summoning sickness, she can attack with it because haste allows
    /// creatures to attack the turn they come under your control.
    #[test]
    fn test_declare_attackers_summoning_sick_with_haste() {
        use crate::cards::definitions::goblin_guide;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Goblin Guide on battlefield with summoning sickness
        // Goblin Guide has haste, so it should still be able to attack
        let guide_def = goblin_guide();
        let guide_id = game.create_object_from_definition(&guide_def, alice, Zone::Battlefield);
        game.set_summoning_sick(guide_id);

        let mut combat = new_combat();
        let declarations = vec![(guide_id, AttackTarget::Player(bob))];

        let result = declare_attackers(&mut game, &mut combat, declarations);
        assert!(
            result.is_ok(),
            "Creatures with haste can attack even with summoning sickness"
        );
    }

    /// Tests that creatures with defender cannot attack.
    ///
    /// Scenario: Alice controls Wall of Omens (a creature with defender). When she
    /// tries to declare it as an attacker, the declaration should fail because
    /// creatures with defender can't attack.
    #[test]
    fn test_declare_attackers_defender_fails() {
        use crate::cards::definitions::wall_of_omens;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Wall of Omens on battlefield (has defender)
        let wall_def = wall_of_omens();
        let wall_id = game.create_object_from_definition(&wall_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(wall_id);

        let mut combat = new_combat();
        let declarations = vec![(wall_id, AttackTarget::Player(bob))];

        let result = declare_attackers(&mut game, &mut combat, declarations);
        assert!(
            matches!(result, Err(CombatError::CreatureCannotAttack(_))),
            "Creatures with defender cannot attack"
        );
    }

    #[test]
    fn test_declare_attackers_wrong_controller_fails() {
        let (mut game, _, _, blocker1, _) = setup_game_with_creatures();
        let mut combat = new_combat();

        // Try to attack with opponent's creature
        let declarations = vec![(blocker1, AttackTarget::Player(PlayerId::from_index(0)))];

        let result = declare_attackers(&mut game, &mut combat, declarations);
        assert!(matches!(result, Err(CombatError::NotControlledBy { .. })));
    }

    #[test]
    fn test_declare_attackers_duplicate_fails() {
        let (mut game, attacker1, _, _, _) = setup_game_with_creatures();
        let mut combat = new_combat();

        // Try to declare same creature twice
        let declarations = vec![
            (attacker1, AttackTarget::Player(PlayerId::from_index(1))),
            (attacker1, AttackTarget::Player(PlayerId::from_index(1))),
        ];

        let result = declare_attackers(&mut game, &mut combat, declarations);
        assert!(matches!(result, Err(CombatError::DuplicateAttacker(_))));
    }

    #[test]
    fn test_declare_attackers_invalid_target_fails() {
        let (mut game, attacker1, _, _, _) = setup_game_with_creatures();
        let mut combat = new_combat();

        // Try to attack non-existent player
        let declarations = vec![(attacker1, AttackTarget::Player(PlayerId::from_index(99)))];

        let result = declare_attackers(&mut game, &mut combat, declarations);
        assert!(matches!(result, Err(CombatError::InvalidAttackTarget(_))));
    }

    #[test]
    fn test_declare_attackers_planeswalker_target() {
        let (mut game, attacker1, _, _, _) = setup_game_with_creatures();
        let mut combat = new_combat();

        // Add a planeswalker for Bob
        let pw = make_planeswalker(100, 1, "Jace");
        let pw_id = pw.id;
        game.add_object(pw);

        let declarations = vec![(attacker1, AttackTarget::Planeswalker(pw_id))];

        let result = declare_attackers(&mut game, &mut combat, declarations);
        assert!(result.is_ok());
        assert!(matches!(
            get_attack_target(&combat, attacker1),
            Some(AttackTarget::Planeswalker(id)) if *id == pw_id
        ));
    }

    #[test]
    fn test_declare_blockers_success() {
        let (mut game, attacker1, attacker2, blocker1, blocker2) = setup_game_with_creatures();
        let mut combat = new_combat();

        // First declare attackers
        let attacker_declarations = vec![
            (attacker1, AttackTarget::Player(PlayerId::from_index(1))),
            (attacker2, AttackTarget::Player(PlayerId::from_index(1))),
        ];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        // Now declare blockers
        let blocker_declarations = vec![(blocker1, attacker1), (blocker2, attacker2)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(result.is_ok());

        assert!(is_blocking(&combat, blocker1));
        assert!(is_blocking(&combat, blocker2));
        assert_eq!(get_blocked_attacker(&combat, blocker1), Some(attacker1));
        assert_eq!(get_blocked_attacker(&combat, blocker2), Some(attacker2));
    }

    #[test]
    fn test_declare_blockers_multiple_on_one_attacker() {
        let (mut game, attacker1, _, blocker1, blocker2) = setup_game_with_creatures();
        let mut combat = new_combat();

        // Declare one attacker
        let attacker_declarations =
            vec![(attacker1, AttackTarget::Player(PlayerId::from_index(1)))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        // Both blockers block the same attacker
        let blocker_declarations = vec![(blocker1, attacker1), (blocker2, attacker1)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(result.is_ok());

        let blockers = get_blockers(&combat, attacker1);
        assert_eq!(blockers.len(), 2);
        assert!(blockers.contains(&blocker1));
        assert!(blockers.contains(&blocker2));
    }

    /// Tests that tapped creatures cannot block.
    ///
    /// Scenario: Alice attacks Bob with Grizzly Bears. Bob's Grizzly Bears is tapped
    /// (from a previous ability activation or attack). When Bob tries to declare it
    /// as a blocker, the declaration should fail because tapped creatures can't block.
    #[test]
    fn test_declare_blockers_tapped_fails() {
        use crate::cards::definitions::grizzly_bears;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create attacker for Alice
        let bears_def = grizzly_bears();
        let attacker_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        // Create blocker for Bob (tapped)
        let blocker_id = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker_id);
        game.tap(blocker_id);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        let blocker_declarations = vec![(blocker_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            matches!(result, Err(CombatError::CreatureTapped(_))),
            "Tapped creatures cannot block"
        );
    }

    /// Tests that non-flying creatures cannot block flying creatures.
    ///
    /// Scenario: Alice attacks Bob with Serra Angel (a creature with flying). Bob
    /// tries to block with Grizzly Bears (a ground creature). The block should fail
    /// because creatures without flying or reach cannot block flying creatures.
    #[test]
    fn test_declare_blockers_flying_evasion() {
        use crate::cards::definitions::{grizzly_bears, serra_angel};

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Serra Angel (flying) for Alice
        let serra_def = serra_angel();
        let attacker_id = game.create_object_from_definition(&serra_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        // Create Grizzly Bears (ground creature) for Bob
        let bears_def = grizzly_bears();
        let blocker_id = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker_id);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        // Ground creature can't block flyer
        let blocker_declarations = vec![(blocker_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            matches!(result, Err(CombatError::CreatureCannotBlock { .. })),
            "Ground creatures cannot block flying creatures"
        );
    }

    /// Tests that creatures with reach can block flying creatures.
    ///
    /// Scenario: Alice attacks Bob with Serra Angel (a creature with flying). Bob
    /// blocks with Giant Spider (a creature with reach). The block should succeed
    /// because reach allows a creature to block flying creatures.
    #[test]
    fn test_declare_blockers_flying_with_reach() {
        use crate::cards::definitions::{giant_spider, serra_angel};

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Serra Angel (flying) for Alice
        let serra_def = serra_angel();
        let attacker_id = game.create_object_from_definition(&serra_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        // Create Giant Spider (reach) for Bob
        let spider_def = giant_spider();
        let blocker_id = game.create_object_from_definition(&spider_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker_id);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        let blocker_declarations = vec![(blocker_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            result.is_ok(),
            "Creatures with reach can block flying creatures"
        );
    }

    /// Tests that creatures with menace cannot be blocked by a single creature.
    ///
    /// Scenario: Alice attacks Bob with Boggart Brute (a creature with menace). Bob
    /// tries to block with a single Grizzly Bears. The block should fail because
    /// menace requires at least two creatures to block.
    #[test]
    fn test_declare_blockers_menace_not_enough() {
        use crate::cards::definitions::{boggart_brute, grizzly_bears};

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Boggart Brute (menace) for Alice
        let brute_def = boggart_brute();
        let attacker_id = game.create_object_from_definition(&brute_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        // Create single Grizzly Bears for Bob
        let bears_def = grizzly_bears();
        let blocker_id = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker_id);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        // Only one blocker - not enough for menace
        let blocker_declarations = vec![(blocker_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            matches!(
                result,
                Err(CombatError::NotEnoughBlockers {
                    required: 2,
                    provided: 1,
                    ..
                })
            ),
            "Menace requires at least two blockers"
        );
    }

    /// Tests that creatures with menace can be blocked by two or more creatures.
    ///
    /// Scenario: Alice attacks Bob with Boggart Brute (a creature with menace). Bob
    /// blocks with two Grizzly Bears. The block should succeed because menace only
    /// requires at least two blockers.
    #[test]
    fn test_declare_blockers_menace_success() {
        use crate::cards::definitions::{boggart_brute, grizzly_bears};

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Boggart Brute (menace) for Alice
        let brute_def = boggart_brute();
        let attacker_id = game.create_object_from_definition(&brute_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        // Create two Grizzly Bears for Bob
        let bears_def = grizzly_bears();
        let blocker1_id = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker1_id);
        let blocker2_id = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker2_id);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        // Two blockers satisfy menace
        let blocker_declarations = vec![(blocker1_id, attacker_id), (blocker2_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            result.is_ok(),
            "Two creatures can successfully block a creature with menace"
        );
    }

    #[test]
    fn test_declare_blockers_duplicate_blocker_fails() {
        let (mut game, attacker1, attacker2, blocker1, _) = setup_game_with_creatures();
        let mut combat = new_combat();

        let attacker_declarations = vec![
            (attacker1, AttackTarget::Player(PlayerId::from_index(1))),
            (attacker2, AttackTarget::Player(PlayerId::from_index(1))),
        ];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        // Same creature trying to block two attackers
        let blocker_declarations = vec![(blocker1, attacker1), (blocker1, attacker2)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(matches!(result, Err(CombatError::DuplicateBlocker(_))));
    }

    #[test]
    fn test_set_damage_assignment_order() {
        let (mut game, attacker1, _, blocker1, blocker2) = setup_game_with_creatures();
        let mut combat = new_combat();

        let attacker_declarations =
            vec![(attacker1, AttackTarget::Player(PlayerId::from_index(1)))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        let blocker_declarations = vec![(blocker1, attacker1), (blocker2, attacker1)];
        declare_blockers(&game, &mut combat, blocker_declarations).unwrap();

        // Set damage order: blocker2 first, then blocker1
        let result = set_damage_assignment_order(&mut combat, attacker1, vec![blocker2, blocker1]);
        assert!(result.is_ok());

        let order = get_damage_assignment_order(&combat, attacker1);
        assert_eq!(order, vec![blocker2, blocker1]);
    }

    #[test]
    fn test_set_damage_assignment_order_wrong_blockers_fails() {
        let (mut game, attacker1, _, blocker1, blocker2) = setup_game_with_creatures();
        let mut combat = new_combat();

        let attacker_declarations =
            vec![(attacker1, AttackTarget::Player(PlayerId::from_index(1)))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        let blocker_declarations = vec![(blocker1, attacker1)];
        declare_blockers(&game, &mut combat, blocker_declarations).unwrap();

        // Try to set order with wrong blockers
        let result = set_damage_assignment_order(&mut combat, attacker1, vec![blocker2]);
        assert!(matches!(
            result,
            Err(CombatError::InvalidBlockerOrder { .. })
        ));
    }

    #[test]
    fn test_is_blocked_and_unblocked() {
        let (mut game, attacker1, attacker2, blocker1, _) = setup_game_with_creatures();
        let mut combat = new_combat();

        let attacker_declarations = vec![
            (attacker1, AttackTarget::Player(PlayerId::from_index(1))),
            (attacker2, AttackTarget::Player(PlayerId::from_index(1))),
        ];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        // Only block attacker1
        let blocker_declarations = vec![(blocker1, attacker1)];
        declare_blockers(&game, &mut combat, blocker_declarations).unwrap();

        assert!(is_blocked(&combat, attacker1));
        assert!(!is_unblocked(&combat, attacker1));

        assert!(!is_blocked(&combat, attacker2));
        assert!(is_unblocked(&combat, attacker2));
    }

    #[test]
    fn test_attackers_targeting_player() {
        let (mut game, attacker1, attacker2, _, _) = setup_game_with_creatures();
        let mut combat = new_combat();

        // Add a planeswalker for Bob
        let pw = make_planeswalker(100, 1, "Jace");
        let pw_id = pw.id;
        game.add_object(pw);

        let attacker_declarations = vec![
            (attacker1, AttackTarget::Player(PlayerId::from_index(1))),
            (attacker2, AttackTarget::Planeswalker(pw_id)),
        ];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        let player_attackers = attackers_targeting_player(&combat, PlayerId::from_index(1));
        assert_eq!(player_attackers.len(), 1);
        assert!(player_attackers.contains(&attacker1));

        let pw_attackers = attackers_targeting_planeswalker(&combat, pw_id);
        assert_eq!(pw_attackers.len(), 1);
        assert!(pw_attackers.contains(&attacker2));
    }

    /// Tests shadow evasion - creatures without shadow cannot block shadow creatures.
    ///
    /// Scenario: Alice attacks Bob with Dauthi Slayer (a creature with shadow). Bob
    /// tries to block with Grizzly Bears, which should fail. Then Bob tries with
    /// another Dauthi Slayer (also with shadow), which should succeed because shadow
    /// can block shadow.
    #[test]
    fn test_shadow_evasion() {
        use crate::cards::definitions::{dauthi_slayer, grizzly_bears};

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Dauthi Slayer (shadow) for Alice
        let dauthi_def = dauthi_slayer();
        let attacker_id = game.create_object_from_definition(&dauthi_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        // Create Grizzly Bears (no shadow) for Bob
        let bears_def = grizzly_bears();
        let blocker1_id = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker1_id);

        // Create another Dauthi Slayer (shadow) for Bob
        let blocker2_id = game.create_object_from_definition(&dauthi_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker2_id);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        // Non-shadow creature can't block shadow
        let blocker_declarations = vec![(blocker1_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            matches!(result, Err(CombatError::CreatureCannotBlock { .. })),
            "Non-shadow creatures cannot block shadow creatures"
        );

        // Shadow creature can block shadow
        let blocker_declarations = vec![(blocker2_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            result.is_ok(),
            "Shadow creatures can block other shadow creatures"
        );
    }

    /// Tests that creatures with "can't be blocked" cannot be blocked.
    ///
    /// Scenario: Alice attacks Bob with Invisible Stalker (a creature that can't be
    /// blocked). Bob tries to block with Grizzly Bears, which should fail because
    /// unblockable creatures cannot be blocked by any creature.
    #[test]
    fn test_unblockable() {
        use crate::cards::definitions::{grizzly_bears, invisible_stalker};

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Invisible Stalker (unblockable) for Alice
        let stalker_def = invisible_stalker();
        let attacker_id =
            game.create_object_from_definition(&stalker_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        // Create Grizzly Bears for Bob
        let bears_def = grizzly_bears();
        let blocker_id = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker_id);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        let blocker_declarations = vec![(blocker_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            matches!(result, Err(CombatError::CreatureCannotBlock { .. })),
            "Unblockable creatures cannot be blocked"
        );
    }

    /// Tests that creatures with "can't block" cannot block.
    ///
    /// Scenario: Alice attacks Bob with Grizzly Bears. Bob tries to block with
    /// Sightless Ghoul (a creature that can't block). The block should fail because
    /// creatures with "can't block" cannot be declared as blockers.
    #[test]
    fn test_cant_block_ability() {
        use crate::cards::definitions::{grizzly_bears, sightless_ghoul};

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Grizzly Bears for Alice
        let bears_def = grizzly_bears();
        let attacker_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        // Create Sightless Ghoul (can't block) for Bob
        let ghoul_def = sightless_ghoul();
        let blocker_id = game.create_object_from_definition(&ghoul_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker_id);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        let blocker_declarations = vec![(blocker_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            matches!(result, Err(CombatError::CreatureCannotBlock { .. })),
            "Creatures with 'can't block' cannot be declared as blockers"
        );
    }

    /// Tests horsemanship evasion - creatures without horsemanship cannot block
    /// creatures with horsemanship.
    ///
    /// Scenario: Alice attacks Bob with Zodiac Rooster (a creature with horsemanship).
    /// Bob tries to block with Grizzly Bears, which should fail. Then Bob tries with
    /// another Zodiac Rooster (also with horsemanship), which should succeed.
    #[test]
    fn test_horsemanship_evasion() {
        use crate::cards::definitions::{grizzly_bears, zodiac_rooster};

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Zodiac Rooster (horsemanship) for Alice
        let rooster_def = zodiac_rooster();
        let attacker_id =
            game.create_object_from_definition(&rooster_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker_id);

        // Create Grizzly Bears (no horsemanship) for Bob
        let bears_def = grizzly_bears();
        let blocker1_id = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker1_id);

        // Create another Zodiac Rooster (horsemanship) for Bob
        let blocker2_id = game.create_object_from_definition(&rooster_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker2_id);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        // Non-horsemanship creature can't block horsemanship
        let blocker_declarations = vec![(blocker1_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            matches!(result, Err(CombatError::CreatureCannotBlock { .. })),
            "Non-horsemanship creatures cannot block horsemanship creatures"
        );

        // Horsemanship creature can block horsemanship
        let blocker_declarations = vec![(blocker2_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            result.is_ok(),
            "Horsemanship creatures can block other horsemanship creatures"
        );
    }

    #[test]
    fn test_landwalk_unblockable_when_defender_controls_matching_land() {
        use crate::cards::definitions::grizzly_bears;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let bob = PlayerId::from_index(1);

        // Attacker with swampwalk.
        let mut attacker = make_creature(100, 0, "Swampwalker");
        attacker
            .abilities
            .push(Ability::static_ability(StaticAbility::landwalk(
                Subtype::Swamp,
            )));
        let attacker_id = attacker.id;
        game.add_object(attacker);
        game.remove_summoning_sickness(attacker_id);

        // Bob controls a Swamp.
        let mut swamp = make_creature(101, 1, "Swamp");
        swamp.card_types = vec![CardType::Land];
        swamp.subtypes = vec![Subtype::Swamp];
        swamp.base_power = None;
        swamp.base_toughness = None;
        game.add_object(swamp);

        // Bob also controls a normal blocker.
        let bears_def = grizzly_bears();
        let blocker_id = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker_id);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        let blocker_declarations = vec![(blocker_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            matches!(result, Err(CombatError::CreatureCannotBlock { .. })),
            "landwalk attacker should be unblockable when defender controls matching land"
        );

        // If the defending player doesn't control the required subtype, blocking should be legal.
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let mut attacker = make_creature(200, 0, "Swampwalker");
        attacker
            .abilities
            .push(Ability::static_ability(StaticAbility::landwalk(
                Subtype::Swamp,
            )));
        let attacker_id = attacker.id;
        game.add_object(attacker);
        game.remove_summoning_sickness(attacker_id);
        let blocker_id = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker_id);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();
        let blocker_declarations = vec![(blocker_id, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            result.is_ok(),
            "without matching land subtype, landwalk should not prevent blocking"
        );
    }

    #[test]
    fn test_cant_be_blocked_by_more_than_one_creature() {
        use crate::cards::definitions::grizzly_bears;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let bob = PlayerId::from_index(1);

        let mut attacker = make_creature(300, 0, "Solo Target");
        attacker.abilities.push(Ability::static_ability(
            StaticAbility::cant_be_blocked_by_more_than(1),
        ));
        let attacker_id = attacker.id;
        game.add_object(attacker);
        game.remove_summoning_sickness(attacker_id);

        let bears_def = grizzly_bears();
        let blocker1 = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        let blocker2 = game.create_object_from_definition(&bears_def, bob, Zone::Battlefield);
        game.remove_summoning_sickness(blocker1);
        game.remove_summoning_sickness(blocker2);

        let mut combat = new_combat();
        let attacker_declarations = vec![(attacker_id, AttackTarget::Player(bob))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        let blocker_declarations = vec![(blocker1, attacker_id), (blocker2, attacker_id)];
        let result = declare_blockers(&game, &mut combat, blocker_declarations);
        assert!(
            matches!(
                result,
                Err(CombatError::TooManyBlockers {
                    maximum: 1,
                    provided: 2,
                    ..
                })
            ),
            "max-blockers restriction should reject too many blockers"
        );
    }

    #[test]
    fn test_defending_players() {
        let (mut game, attacker1, attacker2, _, _) = setup_game_with_creatures();
        let mut combat = new_combat();

        // Attack player 1 with both creatures
        let attacker_declarations = vec![
            (attacker1, AttackTarget::Player(PlayerId::from_index(1))),
            (attacker2, AttackTarget::Player(PlayerId::from_index(1))),
        ];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        // Should have player 1 as defending player
        let defenders = defending_players(&combat);
        assert_eq!(defenders.len(), 1);
        assert!(defenders.contains(&PlayerId::from_index(1)));

        // is_defending_player checks
        assert!(is_defending_player(&combat, PlayerId::from_index(1)));
        assert!(!is_defending_player(&combat, PlayerId::from_index(0)));
    }

    #[test]
    fn test_attacking_player() {
        let (mut game, attacker1, _, _, _) = setup_game_with_creatures();
        let mut combat = new_combat();

        // Player 0 attacks player 1
        let attacker_declarations =
            vec![(attacker1, AttackTarget::Player(PlayerId::from_index(1)))];
        declare_attackers(&mut game, &mut combat, attacker_declarations).unwrap();

        // Player 0 should be the attacking player
        assert_eq!(
            get_attacking_player(&combat, &game),
            Some(PlayerId::from_index(0))
        );
        assert!(is_attacking_player(&combat, PlayerId::from_index(0), &game));
        assert!(!is_attacking_player(
            &combat,
            PlayerId::from_index(1),
            &game
        ));
    }

    #[test]
    fn test_no_attackers_returns_none() {
        let (game, _, _, _, _) = setup_game_with_creatures();
        let combat = new_combat();

        // No attackers declared
        assert!(defending_players(&combat).is_empty());
        assert_eq!(get_attacking_player(&combat, &game), None);
    }
}

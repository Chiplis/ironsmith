//! Damage calculation rules for MTG.
//!
//! This module handles damage-related rules including:
//! - Deathtouch (any damage is lethal)
//! - Lifelink (controller gains life)
//! - Infect (damage as -1/-1 counters to creatures, poison to players)
//! - Wither (damage as -1/-1 counters to creatures)
//! - Trample (excess damage goes to defending player)

use crate::ability::AbilityKind;
use crate::ids::PlayerId;
use crate::object::Object;
use crate::static_abilities::StaticAbilityId;

/// The target of damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageTarget {
    /// Damage to a creature or planeswalker.
    Permanent,
    /// Damage to a player.
    Player(PlayerId),
}

/// Result of calculating damage, including side effects from keywords.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DamageResult {
    /// Amount of normal damage dealt to the target.
    pub damage_dealt: u32,

    /// Life gained by the controller (from lifelink).
    pub life_gained: u32,

    /// Poison counters given to a player (from infect).
    pub poison_counters: u32,

    /// -1/-1 counters to place on a creature (from wither/infect).
    pub minus_counters: u32,

    /// Excess damage that can go to the defending player (for trample).
    pub excess_damage: u32,

    /// Whether the damage source has deathtouch.
    pub has_deathtouch: bool,

    /// Whether the damage source has infect.
    pub has_infect: bool,

    /// Whether the damage source has wither.
    pub has_wither: bool,

    /// Whether the damage source has lifelink.
    pub has_lifelink: bool,
}

/// Check if a source object has a static ability with the given ID.
fn has_ability_id(source: &Object, ability_id: StaticAbilityId) -> bool {
    source.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.id() == ability_id
        } else {
            false
        }
    })
}

/// Calculate the result of dealing damage.
///
/// # Arguments
/// * `source` - The object dealing damage
/// * `target` - Whether the target is a permanent or player
/// * `amount` - The amount of damage being dealt
/// * `is_combat` - Whether this is combat damage
///
/// # Returns
/// A `DamageResult` describing all effects of the damage.
pub fn calculate_damage(
    source: &Object,
    target: DamageTarget,
    amount: u32,
    _is_combat: bool,
) -> DamageResult {
    if amount == 0 {
        return DamageResult::default();
    }

    let has_deathtouch = has_ability_id(source, StaticAbilityId::Deathtouch);
    let has_infect = has_ability_id(source, StaticAbilityId::Infect);
    let has_wither = has_ability_id(source, StaticAbilityId::Wither);
    let has_lifelink = has_ability_id(source, StaticAbilityId::Lifelink);

    let mut result = DamageResult {
        has_deathtouch,
        has_infect,
        has_wither,
        has_lifelink,
        ..Default::default()
    };

    match target {
        DamageTarget::Permanent => {
            // Damage to a creature or planeswalker
            if has_infect || has_wither {
                // Infect/wither: damage is dealt as -1/-1 counters to creatures
                result.minus_counters = amount;
            } else {
                // Normal damage
                result.damage_dealt = amount;
            }
        }
        DamageTarget::Player(_) => {
            // Damage to a player
            if has_infect {
                // Infect: damage to players is dealt as poison counters
                result.poison_counters = amount;
            } else {
                // Normal damage (or wither, which only affects creatures)
                result.damage_dealt = amount;
            }
        }
    }

    // Lifelink: controller gains life equal to damage dealt
    if has_lifelink {
        // Life is gained based on the total "damage dealt" equivalent
        // For infect to creatures: the -1/-1 counters count as damage dealt
        // For infect to players: the poison counters count as damage dealt
        result.life_gained = amount;
    }

    result
}

/// Check if damage is lethal to a creature.
///
/// Damage is lethal if:
/// - The source has deathtouch and dealt any damage (> 0), OR
/// - The total damage marked (including this damage) >= toughness
///
/// # Arguments
/// * `source` - The object dealing damage
/// * `creature` - The creature receiving damage
/// * `damage` - The amount of damage being dealt
/// * `game` - The game state (for accessing damage_marked)
///
/// # Returns
/// `true` if this damage would be lethal to the creature.
pub fn is_lethal(
    source: &Object,
    creature: &Object,
    damage: u32,
    game: &crate::game_state::GameState,
) -> bool {
    if damage == 0 {
        return false;
    }

    // Deathtouch: any damage is lethal
    if has_ability_id(source, StaticAbilityId::Deathtouch) {
        return true;
    }

    // Normal lethal check: damage >= toughness - existing damage
    let Some(toughness) = creature.toughness() else {
        return false;
    };

    let existing_damage = game.damage_on(creature.id);
    let effective_toughness = (toughness - existing_damage as i32).max(0) as u32;
    damage >= effective_toughness
}

/// Calculate excess damage for trample.
///
/// Trample allows attackers to deal excess damage to the defending player
/// after dealing lethal damage to blockers.
///
/// # Arguments
/// * `attacker` - The attacking creature with trample
/// * `blockers` - The creatures blocking the attacker
/// * `total_damage` - The attacker's power (total damage available)
/// * `game` - The game state (for accessing damage_marked)
///
/// # Returns
/// The amount of excess damage that can trample through to the defending player.
/// Returns 0 if the attacker doesn't have trample.
pub fn calculate_trample_excess(
    attacker: &Object,
    blockers: &[&Object],
    total_damage: u32,
    game: &crate::game_state::GameState,
) -> u32 {
    // Must have trample
    if !has_ability_id(attacker, StaticAbilityId::Trample) {
        return 0;
    }

    let has_deathtouch = has_ability_id(attacker, StaticAbilityId::Deathtouch);

    // Calculate minimum damage needed to kill each blocker
    let mut damage_needed: u32 = 0;

    for blocker in blockers {
        if has_deathtouch {
            // With deathtouch, only need 1 damage to each blocker
            damage_needed += 1;
        } else {
            // Need to deal lethal damage (toughness - existing damage)
            if let Some(toughness) = blocker.toughness() {
                let existing_damage = game.damage_on(blocker.id);
                let remaining = (toughness - existing_damage as i32).max(0) as u32;
                damage_needed += remaining;
            }
        }
    }

    // Excess damage tramples through
    total_damage.saturating_sub(damage_needed)
}

/// Calculate damage distribution for trample with multiple blockers.
///
/// Returns a vector of (damage, is_lethal) tuples for each blocker,
/// plus the excess damage that goes to the defending player.
pub fn distribute_trample_damage(
    attacker: &Object,
    blockers: &[&Object],
    total_damage: u32,
    game: &crate::game_state::GameState,
) -> (Vec<(u32, bool)>, u32) {
    if blockers.is_empty() {
        // No blockers, all damage to player
        return (vec![], total_damage);
    }

    let has_deathtouch = has_ability_id(attacker, StaticAbilityId::Deathtouch);
    let has_trample = has_ability_id(attacker, StaticAbilityId::Trample);

    let mut distribution = Vec::with_capacity(blockers.len());
    let mut remaining_damage = total_damage;

    for blocker in blockers {
        let existing_damage = game.damage_on(blocker.id);
        let lethal = if has_deathtouch {
            1
        } else if let Some(toughness) = blocker.toughness() {
            (toughness - existing_damage as i32).max(0) as u32
        } else {
            0
        };

        let damage_to_blocker = remaining_damage.min(lethal);
        let is_lethal = damage_to_blocker >= lethal && lethal > 0;

        distribution.push((damage_to_blocker, is_lethal));
        remaining_damage = remaining_damage.saturating_sub(damage_to_blocker);

        // If no trample, all damage goes to blockers
        if !has_trample {
            remaining_damage = 0;
        }
    }

    (distribution, remaining_damage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::PtValue;
    use crate::cost::OptionalCostsPaid;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, StableId};
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;
    use std::collections::HashMap;

    fn test_game_state() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

    fn make_creature(name: &str, power: i32, toughness: i32) -> Object {
        let id = ObjectId::from_raw(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        Object {
            id,
            stable_id: StableId::from(id),
            kind: crate::object::ObjectKind::Card,
            card: None,
            zone: Zone::Battlefield,
            owner: PlayerId::from_index(0),
            controller: PlayerId::from_index(0),
            name: name.to_string(),
            mana_cost: None,
            color_override: None,
            supertypes: vec![],
            card_types: vec![CardType::Creature],
            subtypes: vec![],
            oracle_text: String::new(),
            base_power: Some(PtValue::Fixed(power)),
            base_toughness: Some(PtValue::Fixed(toughness)),
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
            cost_effects: vec![],
            max_saga_chapter: None,
        }
    }

    fn add_ability(obj: &mut Object, static_ability: StaticAbility) {
        obj.abilities.push(Ability::static_ability(static_ability));
    }

    #[test]
    fn test_normal_damage_to_creature() {
        let source = make_creature("Attacker", 3, 3);
        let result = calculate_damage(&source, DamageTarget::Permanent, 3, true);

        assert_eq!(result.damage_dealt, 3);
        assert_eq!(result.minus_counters, 0);
        assert_eq!(result.poison_counters, 0);
        assert_eq!(result.life_gained, 0);
    }

    #[test]
    fn test_damage_with_lifelink() {
        let mut source = make_creature("Lifelinker", 3, 3);
        add_ability(&mut source, StaticAbility::lifelink());

        let result = calculate_damage(&source, DamageTarget::Permanent, 3, true);

        assert_eq!(result.damage_dealt, 3);
        assert_eq!(result.life_gained, 3);
    }

    #[test]
    fn test_infect_damage_to_creature() {
        let mut source = make_creature("Infector", 3, 3);
        add_ability(&mut source, StaticAbility::infect());

        let result = calculate_damage(&source, DamageTarget::Permanent, 3, true);

        assert_eq!(result.damage_dealt, 0);
        assert_eq!(result.minus_counters, 3);
    }

    #[test]
    fn test_infect_damage_to_player() {
        let mut source = make_creature("Infector", 3, 3);
        add_ability(&mut source, StaticAbility::infect());

        let result = calculate_damage(
            &source,
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            true,
        );

        assert_eq!(result.damage_dealt, 0);
        assert_eq!(result.poison_counters, 3);
    }

    #[test]
    fn test_wither_damage_to_creature() {
        let mut source = make_creature("Witherer", 3, 3);
        add_ability(&mut source, StaticAbility::wither());

        let result = calculate_damage(&source, DamageTarget::Permanent, 3, true);

        assert_eq!(result.damage_dealt, 0);
        assert_eq!(result.minus_counters, 3);
    }

    #[test]
    fn test_wither_damage_to_player() {
        let mut source = make_creature("Witherer", 3, 3);
        add_ability(&mut source, StaticAbility::wither());

        // Wither only affects creatures, normal damage to players
        let result = calculate_damage(
            &source,
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            true,
        );

        assert_eq!(result.damage_dealt, 3);
        assert_eq!(result.poison_counters, 0);
    }

    #[test]
    fn test_infect_with_lifelink() {
        let mut source = make_creature("Infect Lifelink", 3, 3);
        add_ability(&mut source, StaticAbility::infect());
        add_ability(&mut source, StaticAbility::lifelink());

        // Infect to player
        let result = calculate_damage(
            &source,
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            true,
        );
        assert_eq!(result.poison_counters, 3);
        assert_eq!(result.life_gained, 3); // Lifelink still works

        // Infect to creature
        let result = calculate_damage(&source, DamageTarget::Permanent, 3, true);
        assert_eq!(result.minus_counters, 3);
        assert_eq!(result.life_gained, 3);
    }

    #[test]
    fn test_deathtouch_is_lethal() {
        let game = test_game_state();
        let mut source = make_creature("Deathtoucher", 1, 1);
        add_ability(&mut source, StaticAbility::deathtouch());

        let creature = make_creature("Big", 5, 5);

        // 1 damage with deathtouch is lethal
        assert!(is_lethal(&source, &creature, 1, &game));
    }

    #[test]
    fn test_normal_lethal_damage() {
        let mut game = test_game_state();
        let source = make_creature("Normal", 3, 3);

        let creature = make_creature("Target", 4, 4);

        // 3 damage to 4 toughness is not lethal
        assert!(!is_lethal(&source, &creature, 3, &game));

        // 4 damage to 4 toughness is lethal
        assert!(is_lethal(&source, &creature, 4, &game));

        // With existing damage, less is needed
        game.mark_damage(creature.id, 2);
        assert!(is_lethal(&source, &creature, 2, &game));
    }

    #[test]
    fn test_zero_damage_not_lethal() {
        let game = test_game_state();
        let mut source = make_creature("Deathtoucher", 1, 1);
        add_ability(&mut source, StaticAbility::deathtouch());

        let creature = make_creature("Target", 4, 4);

        // 0 damage is never lethal, even with deathtouch
        assert!(!is_lethal(&source, &creature, 0, &game));
    }

    #[test]
    fn test_trample_excess_damage() {
        let game = test_game_state();
        let mut attacker = make_creature("Trampler", 6, 6);
        add_ability(&mut attacker, StaticAbility::trample());

        let blocker = make_creature("Small", 2, 2);

        // 6 power - 2 toughness = 4 excess
        let excess = calculate_trample_excess(&attacker, &[&blocker], 6, &game);
        assert_eq!(excess, 4);
    }

    #[test]
    fn test_trample_multiple_blockers() {
        let game = test_game_state();
        let mut attacker = make_creature("Trampler", 7, 7);
        add_ability(&mut attacker, StaticAbility::trample());

        let blocker1 = make_creature("Small1", 2, 2);
        let blocker2 = make_creature("Small2", 3, 3);

        // 7 power - (2 + 3) toughness = 2 excess
        let excess = calculate_trample_excess(&attacker, &[&blocker1, &blocker2], 7, &game);
        assert_eq!(excess, 2);
    }

    #[test]
    fn test_trample_with_deathtouch() {
        let game = test_game_state();
        let mut attacker = make_creature("Deathtouch Trampler", 6, 6);
        add_ability(&mut attacker, StaticAbility::trample());
        add_ability(&mut attacker, StaticAbility::deathtouch());

        let blocker1 = make_creature("Big1", 5, 5);
        let blocker2 = make_creature("Big2", 5, 5);

        // With deathtouch, only need 1 damage to each blocker
        // 6 power - (1 + 1) = 4 excess
        let excess = calculate_trample_excess(&attacker, &[&blocker1, &blocker2], 6, &game);
        assert_eq!(excess, 4);
    }

    #[test]
    fn test_no_trample_no_excess() {
        let game = test_game_state();
        let attacker = make_creature("Normal", 6, 6);
        let blocker = make_creature("Small", 2, 2);

        // Without trample, no excess damage
        let excess = calculate_trample_excess(&attacker, &[&blocker], 6, &game);
        assert_eq!(excess, 0);
    }

    #[test]
    fn test_distribute_trample_damage() {
        let game = test_game_state();
        let mut attacker = make_creature("Trampler", 5, 5);
        add_ability(&mut attacker, StaticAbility::trample());

        let blocker1 = make_creature("Small1", 2, 2);
        let blocker2 = make_creature("Small2", 2, 2);

        let (distribution, excess) =
            distribute_trample_damage(&attacker, &[&blocker1, &blocker2], 5, &game);

        assert_eq!(distribution.len(), 2);
        assert_eq!(distribution[0], (2, true)); // 2 damage to blocker1, lethal
        assert_eq!(distribution[1], (2, true)); // 2 damage to blocker2, lethal
        assert_eq!(excess, 1); // 1 damage tramples through
    }

    #[test]
    fn test_distribute_damage_no_trample() {
        let game = test_game_state();
        let attacker = make_creature("Normal", 5, 5);
        let blocker = make_creature("Small", 2, 2);

        let (distribution, excess) = distribute_trample_damage(&attacker, &[&blocker], 5, &game);

        assert_eq!(distribution.len(), 1);
        assert_eq!(distribution[0], (2, true)); // Only lethal damage assigned
        assert_eq!(excess, 0); // No trample, no excess
    }

    #[test]
    fn test_existing_damage_affects_lethal() {
        let mut game = test_game_state();
        let source = make_creature("Attacker", 2, 2);

        let creature = make_creature("Damaged", 4, 4);
        game.mark_damage(creature.id, 2);

        // 2 damage to a 4 toughness creature with 2 damage already is lethal
        assert!(is_lethal(&source, &creature, 2, &game));

        // 1 damage is not quite lethal
        assert!(!is_lethal(&source, &creature, 1, &game));
    }
}

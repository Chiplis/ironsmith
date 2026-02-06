//! Combat rules for MTG.
//!
//! This module handles combat-related rules including:
//! - Blocking restrictions (flying, reach, shadow, etc.)
//! - Minimum blockers (menace)
//! - Attack restrictions (defender, summoning sickness)

use crate::ability::{AbilityKind, ProtectionFrom};
use crate::color::Color;
use crate::object::Object;
use crate::static_abilities::StaticAbilityId;
use crate::target::FilterContext;
use crate::types::CardType;

/// Evasion ability types for convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvasionType {
    Flying,
    Shadow,
    Horsemanship,
    Fear,
    Intimidate,
    Skulk,
    Menace,
    Unblockable,
}

/// Check if an object has a static ability with the given ID.
fn has_ability_id(object: &Object, ability_id: StaticAbilityId) -> bool {
    object.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.id() == ability_id
        } else {
            false
        }
    })
}

/// Check if an object has any of the specified static abilities.
#[allow(dead_code)]
fn has_any_ability_id(object: &Object, ability_ids: &[StaticAbilityId]) -> bool {
    ability_ids.iter().any(|id| has_ability_id(object, *id))
}

/// Check if a creature has an evasion ability.
pub fn has_evasion(object: &Object, evasion: EvasionType) -> bool {
    match evasion {
        EvasionType::Flying => has_ability_id(object, StaticAbilityId::Flying),
        EvasionType::Shadow => has_ability_id(object, StaticAbilityId::Shadow),
        EvasionType::Horsemanship => has_ability_id(object, StaticAbilityId::Horsemanship),
        EvasionType::Fear => has_ability_id(object, StaticAbilityId::Fear),
        EvasionType::Intimidate => has_ability_id(object, StaticAbilityId::Intimidate),
        EvasionType::Skulk => false, // Skulk checks power, not a simple ability
        EvasionType::Menace => has_ability_id(object, StaticAbilityId::Menace),
        EvasionType::Unblockable => has_ability_id(object, StaticAbilityId::Unblockable),
    }
}

/// Extract static abilities from an object's base abilities.
/// Used as a fallback when the object isn't in the game state's characteristics cache.
fn get_static_abilities(object: &Object) -> Vec<crate::static_abilities::StaticAbility> {
    object
        .abilities
        .iter()
        .filter_map(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                Some(s.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Check if a blocker can legally block an attacker.
///
/// Returns true if the blocker can block the attacker, considering
/// all evasion abilities and blocking restrictions.
///
/// Takes `GameState` to check abilities granted by continuous effects (like protection from Akroma's Will).
pub fn can_block(attacker: &Object, blocker: &Object, game: &crate::game_state::GameState) -> bool {
    // Get calculated abilities for both creatures (includes continuous effects)
    // Fall back to the object's base abilities if not in game state (for unit tests)
    let attacker_abilities = game
        .calculated_characteristics(attacker.id)
        .map(|c| c.static_abilities)
        .unwrap_or_else(|| get_static_abilities(attacker));
    let blocker_abilities = game
        .calculated_characteristics(blocker.id)
        .map(|c| c.static_abilities)
        .unwrap_or_else(|| get_static_abilities(blocker));

    // Helper to check if abilities contain a specific ability ID
    let attacker_has = |id: StaticAbilityId| attacker_abilities.iter().any(|a| a.id() == id);
    let blocker_has = |id: StaticAbilityId| blocker_abilities.iter().any(|a| a.id() == id);

    // Unblockable creatures can't be blocked
    if attacker_has(StaticAbilityId::Unblockable) {
        return false;
    }

    // Flying: can only be blocked by flying or reach
    if attacker_has(StaticAbilityId::Flying) {
        let blocker_has_flying = blocker_has(StaticAbilityId::Flying);
        let blocker_has_reach = blocker_has(StaticAbilityId::Reach);
        let blocker_can_block_flying = blocker_has(StaticAbilityId::CanBlockFlying);

        if !blocker_has_flying && !blocker_has_reach && !blocker_can_block_flying {
            return false;
        }
    }

    // Shadow: can only block/be blocked by creatures with shadow
    if attacker_has(StaticAbilityId::Shadow) && !blocker_has(StaticAbilityId::Shadow) {
        return false;
    }
    // Creatures with shadow can only block creatures with shadow
    if blocker_has(StaticAbilityId::Shadow) && !attacker_has(StaticAbilityId::Shadow) {
        return false;
    }

    // Horsemanship: can only be blocked by creatures with horsemanship
    if attacker_has(StaticAbilityId::Horsemanship) && !blocker_has(StaticAbilityId::Horsemanship) {
        return false;
    }

    // Fear: can only be blocked by artifact or black creatures
    if attacker_has(StaticAbilityId::Fear) {
        let blocker_is_artifact = blocker.has_card_type(CardType::Artifact);
        let blocker_is_black = blocker.colors().contains(Color::Black);
        if !blocker_is_artifact && !blocker_is_black {
            return false;
        }
    }

    // Intimidate: can only be blocked by artifact or creatures sharing a color
    if attacker_has(StaticAbilityId::Intimidate) {
        let blocker_is_artifact = blocker.has_card_type(CardType::Artifact);
        let attacker_colors = attacker.colors();
        let blocker_colors = blocker.colors();
        let shares_color = !attacker_colors.intersection(blocker_colors).is_empty();

        if !blocker_is_artifact && !shares_color {
            return false;
        }
    }

    // Protection: can't be blocked by creatures the permanent has protection from
    // Check both the object's base abilities AND abilities from continuous effects
    for ability in &attacker_abilities {
        if let Some(prot) = ability.protection_from()
            && protection_prevents_blocking(prot, blocker, attacker, game)
        {
            return false;
        }
    }

    // "Can't block" ability
    if blocker_has(StaticAbilityId::CantBlock) {
        return false;
    }

    true
}

/// Check if protection prevents a creature from blocking.
fn protection_prevents_blocking(
    protection: &ProtectionFrom,
    blocker: &Object,
    attacker: &Object,
    game: &crate::game_state::GameState,
) -> bool {
    match protection {
        ProtectionFrom::Color(colors) => {
            let blocker_colors = blocker.colors();
            !colors.intersection(blocker_colors).is_empty()
        }
        ProtectionFrom::AllColors => !blocker.colors().is_empty(),
        ProtectionFrom::Creatures => blocker.has_card_type(CardType::Creature),
        ProtectionFrom::CardType(card_type) => blocker.has_card_type(*card_type),
        ProtectionFrom::Permanents(filter) => {
            // Create a filter context for the attacker (who has the protection)
            // "You" is the controller of the attacker, source is the attacker
            let ctx = FilterContext::new(attacker.controller).with_source(attacker.id);
            filter.matches(blocker, &ctx, game)
        }
        ProtectionFrom::Everything => true,
        ProtectionFrom::Colorless => blocker.colors().is_empty(),
    }
}

/// Returns the minimum number of blockers required to block an attacker.
///
/// Most creatures require 1 blocker. Creatures with menace require 2.
pub fn minimum_blockers(attacker: &Object) -> usize {
    if has_ability_id(attacker, StaticAbilityId::Menace) {
        2
    } else {
        1
    }
}

/// Check if a creature can attack this turn.
///
/// Returns true if the creature can attack, considering:
/// - Tapped creatures can't attack
/// - Defender (can't attack unless it has "can attack as though no defender")
/// - Summoning sickness (unless it has haste)
/// - "Can't attack" abilities
pub fn can_attack(creature: &Object, game: &crate::game_state::GameState) -> bool {
    // Tapped creatures can't attack
    if game.is_tapped(creature.id) {
        return false;
    }

    // Defender prevents attacking
    if has_ability_id(creature, StaticAbilityId::Defender) {
        // Unless it has "can attack as though it didn't have defender"
        if !has_ability_id(creature, StaticAbilityId::CanAttackAsThoughNoDefender) {
            return false;
        }
    }

    // Summoning sickness prevents attacking (unless haste)
    if game.is_summoning_sick(creature.id) && !has_ability_id(creature, StaticAbilityId::Haste) {
        return false;
    }

    // "Can't attack" ability
    if has_ability_id(creature, StaticAbilityId::CantAttack) {
        return false;
    }

    true
}

/// Check if a creature must attack this turn if able.
///
/// Returns true for creatures with "must attack each turn if able" effects.
pub fn must_attack(creature: &Object) -> bool {
    has_ability_id(creature, StaticAbilityId::MustAttack)
}

/// Check if a creature must block this turn if able.
pub fn must_block(creature: &Object) -> bool {
    has_ability_id(creature, StaticAbilityId::MustBlock)
}

/// Check if a creature has vigilance (doesn't tap to attack).
pub fn has_vigilance(creature: &Object) -> bool {
    has_ability_id(creature, StaticAbilityId::Vigilance)
}

/// Check if a creature has first strike.
pub fn has_first_strike(creature: &Object) -> bool {
    has_ability_id(creature, StaticAbilityId::FirstStrike)
}

/// Check if a creature has double strike.
pub fn has_double_strike(creature: &Object) -> bool {
    has_ability_id(creature, StaticAbilityId::DoubleStrike)
}

/// Check if a creature deals damage in the first strike damage step.
pub fn deals_first_strike_damage(creature: &Object) -> bool {
    has_first_strike(creature) || has_double_strike(creature)
}

/// Check if a creature deals damage in the regular combat damage step.
pub fn deals_regular_combat_damage(creature: &Object) -> bool {
    // Double strike deals in both steps
    // First strike only deals in first strike step
    !has_first_strike(creature) || has_double_strike(creature)
}

/// Check if a creature deals first strike damage, considering continuous effects.
/// This checks both native abilities and abilities granted by continuous effects.
pub fn deals_first_strike_damage_with_game(
    creature: &Object,
    game: &crate::game_state::GameState,
) -> bool {
    // Get calculated abilities (includes continuous effects)
    let abilities = game
        .calculated_characteristics(creature.id)
        .map(|c| c.static_abilities)
        .unwrap_or_else(|| get_static_abilities(creature));

    abilities
        .iter()
        .any(|a| a.id() == StaticAbilityId::FirstStrike || a.id() == StaticAbilityId::DoubleStrike)
}

/// Check if a creature deals regular combat damage, considering continuous effects.
/// This checks both native abilities and abilities granted by continuous effects.
pub fn deals_regular_combat_damage_with_game(
    creature: &Object,
    game: &crate::game_state::GameState,
) -> bool {
    // Get calculated abilities (includes continuous effects)
    let abilities = game
        .calculated_characteristics(creature.id)
        .map(|c| c.static_abilities)
        .unwrap_or_else(|| get_static_abilities(creature));

    let has_first_strike = abilities
        .iter()
        .any(|a| a.id() == StaticAbilityId::FirstStrike);
    let has_double_strike = abilities
        .iter()
        .any(|a| a.id() == StaticAbilityId::DoubleStrike);

    // Double strike deals in both steps
    // First strike only deals in first strike step
    !has_first_strike || has_double_strike
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{Ability, ProtectionFrom};
    use crate::card::PtValue;
    use crate::color::ColorSet;
    use crate::cost::OptionalCostsPaid;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId, StableId};
    use crate::static_abilities::StaticAbility;
    use crate::zone::Zone;
    use std::collections::HashMap;

    fn test_game_state() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature(name: &str, power: i32, toughness: i32) -> Object {
        Object {
            id: ObjectId::from_raw(1),
            stable_id: StableId::from_raw(1),
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
    fn test_flying_blocks_flying() {
        let game = test_game_state();
        let mut attacker = make_creature("Flyer", 2, 2);
        add_ability(&mut attacker, StaticAbility::flying());

        let mut blocker = make_creature("Ground", 2, 2);

        // Ground creature can't block flying
        assert!(!can_block(&attacker, &blocker, &game));

        // Flying creature can block flying
        add_ability(&mut blocker, StaticAbility::flying());
        assert!(can_block(&attacker, &blocker, &game));
    }

    #[test]
    fn test_reach_blocks_flying() {
        let game = test_game_state();
        let mut attacker = make_creature("Flyer", 2, 2);
        add_ability(&mut attacker, StaticAbility::flying());

        let mut blocker = make_creature("Reacher", 2, 2);
        add_ability(&mut blocker, StaticAbility::reach());

        assert!(can_block(&attacker, &blocker, &game));
    }

    #[test]
    fn test_shadow_only_blocks_shadow() {
        let game = test_game_state();
        let mut shadow_attacker = make_creature("Shadow Attacker", 2, 2);
        add_ability(&mut shadow_attacker, StaticAbility::shadow());

        let normal_blocker = make_creature("Normal", 2, 2);
        let mut shadow_blocker = make_creature("Shadow Blocker", 2, 2);
        add_ability(&mut shadow_blocker, StaticAbility::shadow());

        // Normal can't block shadow
        assert!(!can_block(&shadow_attacker, &normal_blocker, &game));

        // Shadow can block shadow
        assert!(can_block(&shadow_attacker, &shadow_blocker, &game));

        // Shadow can't block normal
        let normal_attacker = make_creature("Normal Attacker", 2, 2);
        assert!(!can_block(&normal_attacker, &shadow_blocker, &game));
    }

    #[test]
    fn test_fear_blocked_by_artifact_or_black() {
        let game = test_game_state();
        let mut fear_attacker = make_creature("Fear", 2, 2);
        add_ability(&mut fear_attacker, StaticAbility::fear());

        // White creature can't block
        let mut white_blocker = make_creature("White", 2, 2);
        white_blocker.color_override = Some(ColorSet::WHITE);
        assert!(!can_block(&fear_attacker, &white_blocker, &game));

        // Black creature can block
        let mut black_blocker = make_creature("Black", 2, 2);
        black_blocker.color_override = Some(ColorSet::BLACK);
        assert!(can_block(&fear_attacker, &black_blocker, &game));

        // Artifact creature can block
        let mut artifact_blocker = make_creature("Artifact", 2, 2);
        artifact_blocker.card_types.push(CardType::Artifact);
        assert!(can_block(&fear_attacker, &artifact_blocker, &game));
    }

    #[test]
    fn test_intimidate_blocked_by_artifact_or_same_color() {
        let game = test_game_state();
        let mut intimidate_attacker = make_creature("Intimidate", 2, 2);
        add_ability(&mut intimidate_attacker, StaticAbility::intimidate());
        intimidate_attacker.color_override = Some(ColorSet::RED);

        // Blue creature can't block red intimidate
        let mut blue_blocker = make_creature("Blue", 2, 2);
        blue_blocker.color_override = Some(ColorSet::BLUE);
        assert!(!can_block(&intimidate_attacker, &blue_blocker, &game));

        // Red creature can block red intimidate
        let mut red_blocker = make_creature("Red", 2, 2);
        red_blocker.color_override = Some(ColorSet::RED);
        assert!(can_block(&intimidate_attacker, &red_blocker, &game));

        // Artifact creature can block
        let mut artifact_blocker = make_creature("Artifact", 2, 2);
        artifact_blocker.card_types.push(CardType::Artifact);
        assert!(can_block(&intimidate_attacker, &artifact_blocker, &game));
    }

    #[test]
    fn test_unblockable() {
        let game = test_game_state();
        let mut unblockable = make_creature("Unblockable", 2, 2);
        add_ability(&mut unblockable, StaticAbility::unblockable());

        let blocker = make_creature("Blocker", 2, 2);
        assert!(!can_block(&unblockable, &blocker, &game));
    }

    #[test]
    fn test_menace_minimum_blockers() {
        let normal = make_creature("Normal", 2, 2);
        assert_eq!(minimum_blockers(&normal), 1);

        let mut menace = make_creature("Menace", 2, 2);
        add_ability(&mut menace, StaticAbility::menace());
        assert_eq!(minimum_blockers(&menace), 2);
    }

    #[test]
    fn test_defender_cant_attack() {
        let game = test_game_state();
        let mut defender = make_creature("Defender", 0, 4);
        add_ability(&mut defender, StaticAbility::defender());
        assert!(!can_attack(&defender, &game));

        // With "can attack as though no defender"
        add_ability(
            &mut defender,
            StaticAbility::can_attack_as_though_no_defender(),
        );
        assert!(can_attack(&defender, &game));
    }

    #[test]
    fn test_summoning_sickness() {
        let mut game = test_game_state();
        let creature = make_creature("New", 2, 2);
        game.set_summoning_sick(creature.id);
        assert!(!can_attack(&creature, &game));

        // With haste - add ability to creature
        let mut creature_with_haste = make_creature("New", 2, 2);
        add_ability(&mut creature_with_haste, StaticAbility::haste());
        game.set_summoning_sick(creature_with_haste.id);
        assert!(can_attack(&creature_with_haste, &game));
    }

    #[test]
    fn test_must_attack() {
        let normal = make_creature("Normal", 2, 2);
        assert!(!must_attack(&normal));

        let mut must = make_creature("Must Attack", 2, 2);
        add_ability(&mut must, StaticAbility::must_attack());
        assert!(must_attack(&must));
    }

    #[test]
    fn test_first_strike_damage_steps() {
        let normal = make_creature("Normal", 2, 2);
        assert!(!deals_first_strike_damage(&normal));
        assert!(deals_regular_combat_damage(&normal));

        let mut first_strike = make_creature("First Strike", 2, 2);
        add_ability(&mut first_strike, StaticAbility::first_strike());
        assert!(deals_first_strike_damage(&first_strike));
        assert!(!deals_regular_combat_damage(&first_strike));

        let mut double_strike = make_creature("Double Strike", 2, 2);
        add_ability(&mut double_strike, StaticAbility::double_strike());
        assert!(deals_first_strike_damage(&double_strike));
        assert!(deals_regular_combat_damage(&double_strike));
    }

    #[test]
    fn test_protection_from_color_blocks() {
        let game = test_game_state();
        let mut protected = make_creature("Protected", 2, 2);
        protected
            .abilities
            .push(Ability::static_ability(StaticAbility::protection(
                ProtectionFrom::Color(ColorSet::RED),
            )));

        let mut red_blocker = make_creature("Red", 2, 2);
        red_blocker.color_override = Some(ColorSet::RED);

        let mut blue_blocker = make_creature("Blue", 2, 2);
        blue_blocker.color_override = Some(ColorSet::BLUE);

        // Red can't block protection from red
        assert!(!can_block(&protected, &red_blocker, &game));

        // Blue can block protection from red
        assert!(can_block(&protected, &blue_blocker, &game));
    }

    #[test]
    fn test_cant_block_ability() {
        let game = test_game_state();
        let attacker = make_creature("Attacker", 2, 2);
        let mut cant_block = make_creature("Can't Block", 2, 2);
        add_ability(&mut cant_block, StaticAbility::cant_block());

        assert!(!can_block(&attacker, &cant_block, &game));
    }
}

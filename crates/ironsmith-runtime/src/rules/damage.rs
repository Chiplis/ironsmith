//! Damage calculation rules for MTG.
//!
//! This module handles damage-related rules including:
//! - Deathtouch (any damage is lethal)
//! - Lifelink (controller gains life)
//! - Infect (damage as -1/-1 counters to creatures, poison to players)
//! - Wither (damage as -1/-1 counters to creatures)
//! - Trample (excess damage goes to defending player)

use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::Object;
use crate::static_abilities::StaticAbilityId;
use crate::types::CardType;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SourceDamageKeywords {
    pub has_deathtouch: bool,
    pub has_infect: bool,
    pub has_wither: bool,
    pub has_lifelink: bool,
}

pub(crate) fn source_damage_keywords(
    game: &GameState,
    source: ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> SourceDamageKeywords {
    if game.object(source).is_some() {
        return SourceDamageKeywords {
            has_deathtouch: game.object_has_static_ability_id(source, StaticAbilityId::Deathtouch),
            has_infect: game.object_has_static_ability_id(source, StaticAbilityId::Infect),
            has_wither: game.object_has_static_ability_id(source, StaticAbilityId::Wither),
            has_lifelink: game.object_has_static_ability_id(source, StaticAbilityId::Lifelink),
        };
    }

    let Some(snapshot) = source_snapshot else {
        return SourceDamageKeywords::default();
    };

    SourceDamageKeywords {
        has_deathtouch: snapshot.has_static_ability_id(StaticAbilityId::Deathtouch),
        has_infect: snapshot.has_static_ability_id(StaticAbilityId::Infect),
        has_wither: snapshot.has_static_ability_id(StaticAbilityId::Wither),
        has_lifelink: snapshot.has_static_ability_id(StaticAbilityId::Lifelink),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct AppliedDamageAssignment {
    pub applied: bool,
    pub life_lost: u32,
}

pub(crate) fn apply_processed_damage_assignment(
    game: &mut GameState,
    source: ObjectId,
    target: crate::game_event::DamageTarget,
    amount: u32,
    keywords: SourceDamageKeywords,
    cause: crate::events::cause::EventCause,
) -> AppliedDamageAssignment {
    match target {
        crate::game_event::DamageTarget::Player(player_id) => {
            let source_controller = game
                .object(source)
                .map(|obj| obj.controller)
                .or(cause.source_controller);
            if keywords.has_infect {
                if let Some(event) = game.add_player_counters_with_source(
                    player_id,
                    crate::object::CounterType::Poison,
                    amount,
                    Some(source),
                    source_controller,
                ) {
                    game.queue_trigger_event(event.provenance(), event);
                }
                return AppliedDamageAssignment {
                    applied: true,
                    life_lost: 0,
                };
            }

            if game.player(player_id).is_none() {
                return AppliedDamageAssignment::default();
            }
            // Damage is still dealt even when life total can't change; track only actual life lost.
            let life_lost = game.lose_life(player_id, amount);
            AppliedDamageAssignment {
                applied: true,
                life_lost,
            }
        }
        crate::game_event::DamageTarget::Object(object_id) => {
            let Some(obj) = game.object(object_id) else {
                return AppliedDamageAssignment::default();
            };

            let is_creature = obj.has_card_type(CardType::Creature);
            let is_planeswalker = obj.has_card_type(CardType::Planeswalker);
            if !is_creature && !is_planeswalker {
                return AppliedDamageAssignment::default();
            }

            if is_creature && (keywords.has_infect || keywords.has_wither) {
                let final_count = crate::event_processor::process_put_counters_with_event(
                    game,
                    object_id,
                    crate::CounterType::MinusOneMinusOne,
                    amount,
                    cause.clone(),
                );
                let source_controller = game
                    .object(source)
                    .map(|obj| obj.controller)
                    .or(cause.source_controller);
                if let Some(event) = game.add_counters_with_source(
                    object_id,
                    crate::CounterType::MinusOneMinusOne,
                    final_count,
                    Some(source),
                    source_controller,
                ) {
                    game.queue_trigger_event(event.provenance(), event);
                }
            } else {
                game.mark_damage(object_id, amount);
            }

            if is_creature && amount > 0 && keywords.has_deathtouch {
                game.mark_deathtouch_damage_since_sba(object_id);
            }

            AppliedDamageAssignment {
                applied: true,
                life_lost: 0,
            }
        }
    }
}

fn build_damage_result(
    target: DamageTarget,
    amount: u32,
    has_deathtouch: bool,
    has_infect: bool,
    has_wither: bool,
    has_lifelink: bool,
) -> DamageResult {
    let mut result = DamageResult {
        has_deathtouch,
        has_infect,
        has_wither,
        has_lifelink,
        ..Default::default()
    };

    match target {
        DamageTarget::Permanent => {
            if has_infect || has_wither {
                result.minus_counters = amount;
            } else {
                result.damage_dealt = amount;
            }
        }
        DamageTarget::Player(_) => {
            if has_infect {
                result.poison_counters = amount;
            } else {
                result.damage_dealt = amount;
            }
        }
    }

    if has_lifelink {
        result.life_gained = amount;
    }

    result
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

    let has_deathtouch = source.has_static_ability_id(StaticAbilityId::Deathtouch);
    let has_infect = source.has_static_ability_id(StaticAbilityId::Infect);
    let has_wither = source.has_static_ability_id(StaticAbilityId::Wither);
    let has_lifelink = source.has_static_ability_id(StaticAbilityId::Lifelink);

    build_damage_result(
        target,
        amount,
        has_deathtouch,
        has_infect,
        has_wither,
        has_lifelink,
    )
}

/// Calculate damage using calculated characteristics (continuous effects included).
pub fn calculate_damage_with_game(
    game: &GameState,
    source: &Object,
    target: DamageTarget,
    amount: u32,
    _is_combat: bool,
) -> DamageResult {
    if amount == 0 {
        return DamageResult::default();
    }

    let has_deathtouch = game.object_has_static_ability_id(source.id, StaticAbilityId::Deathtouch);
    let has_infect = game.object_has_static_ability_id(source.id, StaticAbilityId::Infect);
    let has_wither = game.object_has_static_ability_id(source.id, StaticAbilityId::Wither);
    let has_lifelink = game.object_has_static_ability_id(source.id, StaticAbilityId::Lifelink);

    build_damage_result(
        target,
        amount,
        has_deathtouch,
        has_infect,
        has_wither,
        has_lifelink,
    )
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
    if game.object_has_static_ability_id(source.id, StaticAbilityId::Deathtouch) {
        return true;
    }

    // Normal lethal check: damage >= toughness - existing damage
    let Some(toughness) = game
        .calculated_toughness(creature.id)
        .or_else(|| creature.toughness())
    else {
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
    if !game.object_has_static_ability_id(attacker.id, StaticAbilityId::Trample) {
        return 0;
    }

    let has_deathtouch =
        game.object_has_static_ability_id(attacker.id, StaticAbilityId::Deathtouch);

    // Calculate minimum damage needed to kill each blocker
    let mut damage_needed: u32 = 0;

    for blocker in blockers {
        if has_deathtouch {
            // With deathtouch, only need 1 damage to each blocker
            damage_needed += 1;
        } else {
            // Need to deal lethal damage (toughness - existing damage)
            if let Some(toughness) = game
                .calculated_toughness(blocker.id)
                .or_else(|| blocker.toughness())
            {
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

    let has_deathtouch =
        game.object_has_static_ability_id(attacker.id, StaticAbilityId::Deathtouch);
    let has_trample = game.object_has_static_ability_id(attacker.id, StaticAbilityId::Trample);

    let mut distribution = Vec::with_capacity(blockers.len());
    let mut remaining_damage = total_damage;

    for blocker in blockers {
        let existing_damage = game.damage_on(blocker.id);
        let lethal = if has_deathtouch {
            1
        } else if let Some(toughness) = game
            .calculated_toughness(blocker.id)
            .or_else(|| blocker.toughness())
        {
            (toughness - existing_damage as i32).max(0) as u32
        } else {
            0
        };

        let damage_to_blocker = remaining_damage.min(lethal);
        let is_lethal = damage_to_blocker >= lethal && lethal > 0;

        distribution.push((damage_to_blocker, is_lethal));
        remaining_damage = remaining_damage.saturating_sub(damage_to_blocker);
    }

    if has_trample {
        (distribution, remaining_damage)
    } else {
        (distribution, 0)
    }
}

/// Calculate damage distribution among multiple creature recipients when excess can't trample through.
///
/// This is used for cases like a creature blocking multiple attackers, where the creature assigns
/// all of its combat damage among the creatures it's blocking.
pub fn distribute_combat_damage_to_creatures(
    source: &Object,
    recipients: &[&Object],
    total_damage: u32,
    game: &crate::game_state::GameState,
) -> Vec<(u32, bool)> {
    if recipients.is_empty() {
        return vec![];
    }

    let has_deathtouch = game.object_has_static_ability_id(source.id, StaticAbilityId::Deathtouch);
    let mut distribution = Vec::with_capacity(recipients.len());
    let mut remaining_damage = total_damage;

    for (idx, creature) in recipients.iter().enumerate() {
        let existing_damage = game.damage_on(creature.id);
        let lethal = if has_deathtouch {
            1
        } else if let Some(toughness) = game
            .calculated_toughness(creature.id)
            .or_else(|| creature.toughness())
        {
            (toughness - existing_damage as i32).max(0) as u32
        } else {
            0
        };

        let is_last = idx + 1 == recipients.len();
        let damage_to_creature = if is_last {
            remaining_damage
        } else {
            remaining_damage.min(lethal)
        };
        let is_lethal = damage_to_creature >= lethal && lethal > 0;

        distribution.push((damage_to_creature, is_lethal));
        remaining_damage = remaining_damage.saturating_sub(damage_to_creature);
    }

    distribution
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::PtValue;
    use crate::cost::OptionalCostsPaid;
    use crate::events::EventKind;
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
            other_face: None,
            other_face_name: None,
            linked_face_layout: crate::card::LinkedFaceLayout::None,
            base_power: Some(PtValue::Fixed(power)),
            base_toughness: Some(PtValue::Fixed(toughness)),
            base_loyalty: None,
            base_defense: None,
            abilities: vec![],
            counters: HashMap::new(),
            attached_to: None,
            attachments: vec![],
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: vec![],
            has_fuse: false,
            optional_costs: vec![],
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: crate::player::ManaPool::default(),
            x_value: None,
            keyword_payment_contributions_to_cast: vec![],
            additional_cost: crate::cost::TotalCost::free(),
            max_saga_chapter: None,
            bestow_cast_state: None,
            face_down_cast_state: None,
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
    fn test_apply_infect_damage_to_player_queues_markers_changed_event() {
        let mut game = test_game_state();
        let mut source = make_creature("Infector", 3, 3);
        add_ability(&mut source, StaticAbility::infect());
        let source_id = source.id;
        game.add_object(source);

        let result = apply_processed_damage_assignment(
            &mut game,
            source_id,
            crate::game_event::DamageTarget::Player(PlayerId::from_index(1)),
            3,
            SourceDamageKeywords {
                has_infect: true,
                ..SourceDamageKeywords::default()
            },
            crate::events::cause::EventCause::from_effect(source_id, PlayerId::from_index(0)),
        );

        assert!(result.applied);
        assert_eq!(result.life_lost, 0);
        assert_eq!(
            game.player(PlayerId::from_index(1))
                .expect("player exists")
                .poison_counters,
            3
        );
        assert!(
            game.take_pending_trigger_events()
                .iter()
                .any(|event| event.kind() == EventKind::MarkersChanged),
            "infect damage to a player should queue MarkersChangedEvent"
        );
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
        let mut game = test_game_state();
        let mut source = make_creature("Deathtoucher", 1, 1);
        add_ability(&mut source, StaticAbility::deathtouch());

        let creature = make_creature("Big", 5, 5);

        // Add objects to game so ability lookups work
        game.add_object(source.clone());
        game.add_object(creature.clone());

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
        let mut game = test_game_state();
        let mut attacker = make_creature("Trampler", 6, 6);
        add_ability(&mut attacker, StaticAbility::trample());

        let blocker = make_creature("Small", 2, 2);

        game.add_object(attacker.clone());
        game.add_object(blocker.clone());

        // 6 power - 2 toughness = 4 excess
        let excess = calculate_trample_excess(&attacker, &[&blocker], 6, &game);
        assert_eq!(excess, 4);
    }

    #[test]
    fn test_trample_multiple_blockers() {
        let mut game = test_game_state();
        let mut attacker = make_creature("Trampler", 7, 7);
        add_ability(&mut attacker, StaticAbility::trample());

        let blocker1 = make_creature("Small1", 2, 2);
        let blocker2 = make_creature("Small2", 3, 3);

        game.add_object(attacker.clone());
        game.add_object(blocker1.clone());
        game.add_object(blocker2.clone());

        // 7 power - (2 + 3) toughness = 2 excess
        let excess = calculate_trample_excess(&attacker, &[&blocker1, &blocker2], 7, &game);
        assert_eq!(excess, 2);
    }

    #[test]
    fn test_trample_with_deathtouch() {
        let mut game = test_game_state();
        let mut attacker = make_creature("Deathtouch Trampler", 6, 6);
        add_ability(&mut attacker, StaticAbility::trample());
        add_ability(&mut attacker, StaticAbility::deathtouch());

        let blocker1 = make_creature("Big1", 5, 5);
        let blocker2 = make_creature("Big2", 5, 5);

        game.add_object(attacker.clone());
        game.add_object(blocker1.clone());
        game.add_object(blocker2.clone());

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
        let mut game = test_game_state();
        let mut attacker = make_creature("Trampler", 5, 5);
        add_ability(&mut attacker, StaticAbility::trample());

        let blocker1 = make_creature("Small1", 2, 2);
        let blocker2 = make_creature("Small2", 2, 2);

        game.add_object(attacker.clone());
        game.add_object(blocker1.clone());
        game.add_object(blocker2.clone());

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

//! Combat rules for MTG.
//!
//! This module handles combat-related rules including:
//! - Blocking restrictions (flying, reach, shadow, etc.)
//! - Minimum blockers (menace)
//! - Attack restrictions (defender, summoning sickness)

use crate::ability::{ProtectionFrom, extract_static_abilities};
use crate::color::Color;
use crate::derived_view::DerivedGameView;
use crate::object::Object;
use crate::static_abilities::{LandwalkKind, StaticAbilityId};
use crate::target::FilterContext;
use crate::types::{CardType, Supertype};

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
/// Check if a creature has an evasion ability.
pub fn has_evasion(object: &Object, evasion: EvasionType) -> bool {
    match evasion {
        EvasionType::Flying => object.has_static_ability_id(StaticAbilityId::Flying),
        EvasionType::Shadow => object.has_static_ability_id(StaticAbilityId::Shadow),
        EvasionType::Horsemanship => object.has_static_ability_id(StaticAbilityId::Horsemanship),
        EvasionType::Fear => object.has_static_ability_id(StaticAbilityId::Fear),
        EvasionType::Intimidate => object.has_static_ability_id(StaticAbilityId::Intimidate),
        EvasionType::Skulk => object.has_static_ability_id(StaticAbilityId::Skulk),
        EvasionType::Menace => object.has_static_ability_id(StaticAbilityId::Menace),
        EvasionType::Unblockable => object.has_static_ability_id(StaticAbilityId::Unblockable),
    }
}

/// Extract static abilities from an object's base abilities.
/// Used as a fallback when the object isn't in the game state's characteristics cache.
fn get_static_abilities(object: &Object) -> Vec<crate::static_abilities::StaticAbility> {
    extract_static_abilities(&object.abilities)
}

/// Check if a blocker can legally block an attacker.
///
/// Returns true if the blocker can block the attacker, considering
/// all evasion abilities and blocking restrictions.
///
/// Takes `GameState` to check abilities granted by continuous effects (like protection from Akroma's Will).
pub fn can_block(attacker: &Object, blocker: &Object, game: &crate::game_state::GameState) -> bool {
    let view = DerivedGameView::new(game);
    can_block_with_view(attacker, blocker, game, &view)
}

pub(crate) fn can_block_with_view(
    attacker: &Object,
    blocker: &Object,
    game: &crate::game_state::GameState,
    view: &DerivedGameView<'_>,
) -> bool {
    if !game.can_block_attacker(blocker.id, attacker.id) {
        return false;
    }

    // Get calculated abilities for both creatures (includes continuous effects)
    // Fall back to the object's base abilities if not in game state (for unit tests)
    let attacker_chars = view.calculated_characteristics(attacker.id);
    let blocker_chars = view.calculated_characteristics(blocker.id);

    let attacker_abilities = attacker_chars
        .as_ref()
        .map(|c| c.static_abilities.clone())
        .unwrap_or_else(|| get_static_abilities(attacker));
    let blocker_abilities = blocker_chars
        .as_ref()
        .map(|c| c.static_abilities.clone())
        .unwrap_or_else(|| get_static_abilities(blocker));
    let attacker_colors = attacker_chars
        .as_ref()
        .map(|c| c.colors.clone())
        .unwrap_or_else(|| attacker.colors());
    let blocker_colors = blocker_chars
        .as_ref()
        .map(|c| c.colors.clone())
        .unwrap_or_else(|| blocker.colors());
    let blocker_is_artifact = blocker_chars
        .as_ref()
        .map(|c| c.card_types.contains(&CardType::Artifact))
        .unwrap_or_else(|| blocker.has_card_type(CardType::Artifact));

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
        let blocker_can_block_flying = blocker_has(StaticAbilityId::CanBlockFlying)
            || blocker_has(StaticAbilityId::CanBlockOnlyFlying);

        if !blocker_has_flying && !blocker_has_reach && !blocker_can_block_flying {
            return false;
        }
    }

    // "Can't be blocked except by creatures with flying or reach" (without requiring flying)
    if attacker_has(StaticAbilityId::FlyingRestriction) {
        let blocker_has_flying = blocker_has(StaticAbilityId::Flying);
        let blocker_has_reach = blocker_has(StaticAbilityId::Reach);
        if !blocker_has_flying && !blocker_has_reach {
            return false;
        }
    }

    // "Can't be blocked except by creatures with flying" (reach does not satisfy this clause).
    if attacker_has(StaticAbilityId::FlyingOnlyRestriction) && !blocker_has(StaticAbilityId::Flying)
    {
        return false;
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
        let blocker_is_black = blocker_colors.contains(Color::Black);
        if !blocker_is_artifact && !blocker_is_black {
            return false;
        }
    }

    // Intimidate: can only be blocked by artifact or creatures sharing a color
    if attacker_has(StaticAbilityId::Intimidate) {
        let shares_color = !attacker_colors.intersection(blocker_colors).is_empty();

        if !blocker_is_artifact && !shares_color {
            return false;
        }
    }

    // Skulk: can't be blocked by creatures with greater power.
    if attacker_has(StaticAbilityId::Skulk) {
        let attacker_power = attacker_chars
            .as_ref()
            .and_then(|c| c.power)
            .or_else(|| attacker.power());
        let blocker_power = blocker_chars
            .as_ref()
            .and_then(|c| c.power)
            .or_else(|| blocker.power());
        if let (Some(attacker_power), Some(blocker_power)) = (attacker_power, blocker_power)
            && blocker_power > attacker_power
        {
            return false;
        }
    }

    if let Some(attacker_controller) = game.current_controller(attacker.id)
        && game.ring_level(attacker_controller) >= 1
        && game.current_ring_bearer(attacker_controller) == Some(attacker.id)
    {
        let attacker_power = attacker_chars
            .as_ref()
            .and_then(|c| c.power)
            .or_else(|| attacker.power());
        let blocker_power = blocker_chars
            .as_ref()
            .and_then(|c| c.power)
            .or_else(|| blocker.power());
        if let (Some(attacker_power), Some(blocker_power)) = (attacker_power, blocker_power)
            && blocker_power > attacker_power
        {
            return false;
        }
    }

    // Protection: can't be blocked by creatures the permanent has protection from
    // Check both the object's base abilities AND abilities from continuous effects
    for ability in &attacker_abilities {
        if let Some(prot) = ability.protection_from()
            && protection_prevents_blocking_with_view(prot, blocker, attacker, game, view)
        {
            return false;
        }
    }

    // "Can't block" ability
    if blocker_has(StaticAbilityId::CantBlock) {
        return false;
    }

    // "Can't be blocked by creatures with power N or less"
    if let Some(max_blocker_power) = attacker_abilities
        .iter()
        .filter_map(|ability| ability.blocked_by_power_or_less_threshold())
        .max()
    {
        let blocker_power = blocker_chars
            .as_ref()
            .and_then(|c| c.power)
            .or_else(|| blocker.power());
        if blocker_power.is_some_and(|power| power <= max_blocker_power) {
            return false;
        }
    }

    // "Can't be blocked by creatures with power N or greater"
    if let Some(min_blocker_power) = attacker_abilities
        .iter()
        .filter_map(|ability| ability.blocked_by_power_or_greater_threshold())
        .min()
    {
        let blocker_power = blocker_chars
            .as_ref()
            .and_then(|c| c.power)
            .or_else(|| blocker.power());
        if blocker_power.is_some_and(|power| power >= min_blocker_power) {
            return false;
        }
    }

    // "Creatures with power less than this creature's power can't block it"
    if attacker_has(StaticAbilityId::CantBeBlockedByLowerPowerThanSource) {
        let attacker_power = attacker_chars
            .as_ref()
            .and_then(|c| c.power)
            .or_else(|| attacker.power());
        let blocker_power = blocker_chars
            .as_ref()
            .and_then(|c| c.power)
            .or_else(|| blocker.power());
        if let (Some(attacker_power), Some(blocker_power)) = (attacker_power, blocker_power)
            && blocker_power < attacker_power
        {
            return false;
        }
    }

    for landwalk_kind in attacker_abilities
        .iter()
        .filter_map(|ability| ability.landwalk_kind())
    {
        let defending_has_required_land = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|obj| {
                if obj.controller != blocker.controller
                    || !view.object_has_card_type(obj.id, CardType::Land)
                {
                    return false;
                }

                match landwalk_kind {
                    LandwalkKind::Subtype {
                        subtype,
                        snow: false,
                    } => view.calculated_subtypes(obj.id).contains(&subtype),
                    LandwalkKind::Subtype {
                        subtype,
                        snow: true,
                    } => {
                        obj.has_supertype(Supertype::Snow)
                            && view.calculated_subtypes(obj.id).contains(&subtype)
                    }
                    LandwalkKind::AnyLand => true,
                    LandwalkKind::NonbasicLand => !obj.has_supertype(Supertype::Basic),
                    LandwalkKind::ArtifactLand => {
                        view.object_has_card_type(obj.id, CardType::Artifact)
                    }
                }
            });
        if defending_has_required_land {
            return false;
        }
    }

    for required_card_type in attacker_abilities
        .iter()
        .filter_map(|ability| ability.required_defending_player_card_type_for_unblockable())
    {
        let defending_controls_required_type = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|obj| {
                obj.controller == blocker.controller
                    && view.object_has_card_type(obj.id, required_card_type)
            });
        if defending_controls_required_type {
            return false;
        }
    }

    for required_card_types in attacker_abilities
        .iter()
        .filter_map(|ability| ability.required_defending_player_card_types_for_unblockable())
    {
        let defending_controls_required_types = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|obj| {
                obj.controller == blocker.controller
                    && required_card_types
                        .iter()
                        .all(|required_type| view.object_has_card_type(obj.id, *required_type))
            });
        if defending_controls_required_types {
            return false;
        }
    }

    // "Can block only creatures with flying"
    if blocker_has(StaticAbilityId::CanBlockOnlyFlying) && !attacker_has(StaticAbilityId::Flying) {
        return false;
    }

    true
}

fn protection_prevents_blocking_with_view(
    protection: &ProtectionFrom,
    blocker: &Object,
    attacker: &Object,
    game: &crate::game_state::GameState,
    view: &DerivedGameView<'_>,
) -> bool {
    let blocker_chars = view.calculated_characteristics(blocker.id);
    let blocker_colors = blocker_chars
        .as_ref()
        .map(|c| c.colors.clone())
        .unwrap_or_else(|| blocker.colors());
    let blocker_card_types = blocker_chars
        .as_ref()
        .map(|c| c.card_types.clone())
        .unwrap_or_else(|| blocker.card_types.clone());

    match protection {
        ProtectionFrom::Color(colors) => !colors.intersection(blocker_colors).is_empty(),
        ProtectionFrom::AllColors => !blocker_colors.is_empty(),
        ProtectionFrom::Creatures => blocker_card_types.contains(&CardType::Creature),
        ProtectionFrom::CardType(card_type) => blocker_card_types.contains(card_type),
        ProtectionFrom::Permanents(filter) => {
            // Create a filter context for the attacker (who has the protection)
            // "You" is the controller of the attacker, source is the attacker
            let ctx = FilterContext::new(attacker.controller).with_source(attacker.id);
            filter.matches(blocker, &ctx, game)
        }
        ProtectionFrom::Everything => true,
        ProtectionFrom::Colorless => blocker_colors.is_empty(),
        ProtectionFrom::ChosenPlayer => game
            .chosen_player(attacker.id)
            .is_some_and(|chosen| blocker.controller == chosen),
    }
}

/// Returns the minimum number of blockers required to block an attacker.
///
/// Most creatures require 1 blocker. Creatures with menace require 2.
pub fn minimum_blockers(attacker: &Object) -> usize {
    if attacker.has_static_ability_id(StaticAbilityId::Menace) {
        2
    } else {
        1
    }
}

/// Returns the minimum number of blockers required to block an attacker,
/// accounting for granted abilities from continuous effects.
pub fn minimum_blockers_with_game(attacker: &Object, game: &crate::game_state::GameState) -> usize {
    let view = DerivedGameView::new(game);
    minimum_blockers_with_view(attacker, &view)
}

pub(crate) fn minimum_blockers_with_view(attacker: &Object, view: &DerivedGameView<'_>) -> usize {
    if view.object_has_static_ability_id(attacker.id, StaticAbilityId::Menace) {
        2
    } else {
        1
    }
}

/// Returns the maximum number of blockers allowed for an attacker, if restricted.
pub fn maximum_blockers(attacker: &Object, game: &crate::game_state::GameState) -> Option<usize> {
    let view = DerivedGameView::new(game);
    maximum_blockers_with_view(attacker, &view)
}

pub(crate) fn maximum_blockers_with_view(
    attacker: &Object,
    view: &DerivedGameView<'_>,
) -> Option<usize> {
    let abilities = view
        .calculated_characteristics(attacker.id)
        .map(|c| c.static_abilities)
        .unwrap_or_else(|| get_static_abilities(attacker));

    abilities.iter().filter_map(|a| a.maximum_blockers()).min()
}

/// Check if a creature can attack this turn.
///
/// Returns true if the creature can attack, considering:
/// - Tapped creatures can't attack
/// - Defender (can't attack unless it has "can attack as though no defender")
/// - Summoning sickness (unless it has haste)
/// - "Can't attack" abilities
pub fn can_attack(creature: &Object, game: &crate::game_state::GameState) -> bool {
    let view = DerivedGameView::new(game);
    can_attack_with_view(creature, game, &view)
}

pub(crate) fn can_attack_with_view(
    creature: &Object,
    game: &crate::game_state::GameState,
    view: &DerivedGameView<'_>,
) -> bool {
    if !game.can_attack(creature.id) {
        return false;
    }

    // Tapped creatures can't attack
    if game.is_tapped(creature.id) {
        return false;
    }

    // Defender prevents attacking
    if view.object_has_static_ability_id(creature.id, StaticAbilityId::Defender) {
        // Unless it has "can attack as though it didn't have defender"
        if !view
            .object_has_static_ability_id(creature.id, StaticAbilityId::CanAttackAsThoughNoDefender)
        {
            return false;
        }
    }

    // Summoning sickness prevents attacking (unless haste)
    if game.is_summoning_sick(creature.id)
        && !view.object_has_static_ability_id(creature.id, StaticAbilityId::Haste)
    {
        return false;
    }

    // "Can't attack" ability
    if view.object_has_static_ability_id(creature.id, StaticAbilityId::CantAttack) {
        return false;
    }

    true
}

/// Check if a creature can attack a specific defending player.
///
/// Includes all regular attack legality plus defender-dependent requirements like
/// "can't attack unless defending player controls an Island."
pub fn can_attack_defending_player(
    creature: &Object,
    defending_player: crate::ids::PlayerId,
    game: &crate::game_state::GameState,
) -> bool {
    let view = DerivedGameView::new(game);
    can_attack_defending_player_with_view(creature, defending_player, game, &view)
}

pub(crate) fn can_attack_defending_player_with_view(
    creature: &Object,
    defending_player: crate::ids::PlayerId,
    game: &crate::game_state::GameState,
    view: &DerivedGameView<'_>,
) -> bool {
    if !can_attack_with_view(creature, game, view) {
        return false;
    }

    let abilities = view
        .calculated_characteristics(creature.id)
        .map(|c| c.static_abilities)
        .unwrap_or_else(|| get_static_abilities(creature));

    for ability in &abilities {
        if let Some(can_attack) = ability.can_attack_specific_defender(
            game,
            creature.id,
            creature.controller,
            defending_player,
        ) && !can_attack
        {
            return false;
        }
    }

    true
}

/// Check if a creature must attack this turn if able.
///
/// Returns true for creatures with "must attack each turn if able" effects.
pub fn must_attack(creature: &Object) -> bool {
    creature.has_static_ability_id(StaticAbilityId::MustAttack)
}

/// Check if a creature must attack this turn if able, with continuous effects applied.
pub fn must_attack_with_game(creature: &Object, game: &crate::game_state::GameState) -> bool {
    let view = DerivedGameView::new(game);
    must_attack_with_view(creature, game, &view)
}

pub(crate) fn must_attack_with_view(
    creature: &Object,
    game: &crate::game_state::GameState,
    view: &DerivedGameView<'_>,
) -> bool {
    view.object_has_static_ability_id(creature.id, StaticAbilityId::MustAttack)
        || game.is_goaded(creature.id)
}

/// Check if a creature must block this turn if able.
pub fn must_block(creature: &Object) -> bool {
    creature.has_static_ability_id(StaticAbilityId::MustBlock)
}

/// Check if a creature must block this turn if able, with continuous effects applied.
pub fn must_block_with_game(creature: &Object, game: &crate::game_state::GameState) -> bool {
    game.object_has_static_ability_id(creature.id, StaticAbilityId::MustBlock)
}

/// Check if a creature has vigilance (doesn't tap to attack).
pub fn has_vigilance(creature: &Object) -> bool {
    creature.has_static_ability_id(StaticAbilityId::Vigilance)
}

/// Check if a creature has vigilance (doesn't tap to attack), with continuous effects applied.
pub fn has_vigilance_with_game(creature: &Object, game: &crate::game_state::GameState) -> bool {
    let view = DerivedGameView::new(game);
    has_vigilance_with_view(creature, &view)
}

pub(crate) fn has_vigilance_with_view(creature: &Object, view: &DerivedGameView<'_>) -> bool {
    view.object_has_static_ability_id(creature.id, StaticAbilityId::Vigilance)
}

/// Check if a creature has first strike.
pub fn has_first_strike(creature: &Object) -> bool {
    creature.has_static_ability_id(StaticAbilityId::FirstStrike)
}

/// Check if a creature has double strike.
pub fn has_double_strike(creature: &Object) -> bool {
    creature.has_static_ability_id(StaticAbilityId::DoubleStrike)
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

    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

    fn make_creature(name: &str, power: i32, toughness: i32) -> Object {
        let raw = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Object {
            id: ObjectId::from_raw(raw),
            stable_id: StableId::from_raw(raw),
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
    fn test_can_block_only_flying_restriction() {
        let game = test_game_state();
        let attacker = make_creature("Ground", 2, 2);
        let mut blocker = make_creature("Sky Guard", 2, 2);
        add_ability(&mut blocker, StaticAbility::can_block_only_flying());

        assert!(!can_block(&attacker, &blocker, &game));

        let mut flying_attacker = make_creature("Flyer", 2, 2);
        add_ability(&mut flying_attacker, StaticAbility::flying());
        assert!(can_block(&flying_attacker, &blocker, &game));
    }

    #[test]
    fn test_cant_be_blocked_by_creatures_with_power_or_less() {
        let game = test_game_state();
        let mut attacker = make_creature("Evasive", 3, 3);
        add_ability(
            &mut attacker,
            StaticAbility::cant_be_blocked_by_power_or_less(2),
        );

        let small_blocker = make_creature("Small", 2, 2);
        let big_blocker = make_creature("Big", 3, 3);

        assert!(!can_block(&attacker, &small_blocker, &game));
        assert!(can_block(&attacker, &big_blocker, &game));
    }

    #[test]
    fn test_cant_be_blocked_by_creatures_with_power_or_greater() {
        let game = test_game_state();
        let mut attacker = make_creature("Evasive", 3, 3);
        add_ability(
            &mut attacker,
            StaticAbility::cant_be_blocked_by_power_or_greater(3),
        );

        let small_blocker = make_creature("Small", 2, 2);
        let equal_power_blocker = make_creature("Equal", 3, 3);
        let big_blocker = make_creature("Big", 4, 4);

        assert!(can_block(&attacker, &small_blocker, &game));
        assert!(!can_block(&attacker, &equal_power_blocker, &game));
        assert!(!can_block(&attacker, &big_blocker, &game));
    }

    #[test]
    fn test_cant_be_blocked_by_creatures_with_lower_power_than_source() {
        let game = test_game_state();
        let mut attacker = make_creature("Wandering Wolf", 2, 1);
        add_ability(
            &mut attacker,
            StaticAbility::cant_be_blocked_by_lower_power_than_source(),
        );

        let small_blocker = make_creature("Small", 1, 1);
        let equal_power_blocker = make_creature("Equal", 2, 2);
        let big_blocker = make_creature("Big", 3, 3);

        assert!(!can_block(&attacker, &small_blocker, &game));
        assert!(can_block(&attacker, &equal_power_blocker, &game));
        assert!(can_block(&attacker, &big_blocker, &game));
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
    fn test_skulk_cant_be_blocked_by_greater_power() {
        let game = test_game_state();
        let mut skulk_attacker = make_creature("Skulker", 2, 2);
        add_ability(&mut skulk_attacker, StaticAbility::skulk());

        let equal_power_blocker = make_creature("Equal", 2, 2);
        let smaller_blocker = make_creature("Small", 1, 1);
        let larger_blocker = make_creature("Large", 3, 3);

        assert!(can_block(&skulk_attacker, &equal_power_blocker, &game));
        assert!(can_block(&skulk_attacker, &smaller_blocker, &game));
        assert!(!can_block(&skulk_attacker, &larger_blocker, &game));
    }

    #[test]
    fn test_ring_bearer_cant_be_blocked_by_greater_power() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = test_game_state();

        let mut ring_bearer = make_creature("Bearer", 2, 2);
        ring_bearer.controller = alice;
        ring_bearer.owner = alice;

        let mut equal_power_blocker = make_creature("Equal", 2, 2);
        equal_power_blocker.controller = bob;
        equal_power_blocker.owner = bob;

        let mut larger_blocker = make_creature("Large", 3, 3);
        larger_blocker.controller = bob;
        larger_blocker.owner = bob;

        game.add_object(ring_bearer.clone());
        game.add_object(equal_power_blocker.clone());
        game.add_object(larger_blocker.clone());
        game.increment_ring_temptations(alice);
        game.set_ring_bearer(alice, ring_bearer.id);

        assert!(can_block(&ring_bearer, &equal_power_blocker, &game));
        assert!(!can_block(&ring_bearer, &larger_blocker, &game));
    }

    #[test]
    fn test_cant_be_blocked_when_defending_player_controls_required_card_type() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut attacker = make_creature("Beebles", 2, 2);
        attacker.id = ObjectId::from_raw(10);
        attacker.controller = alice;
        add_ability(
            &mut attacker,
            StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_type(
                CardType::Artifact,
            ),
        );

        let mut blocker = make_creature("Blocker", 2, 2);
        blocker.id = ObjectId::from_raw(11);
        blocker.controller = bob;

        let mut game_without_artifact = test_game_state();
        game_without_artifact.add_object(attacker.clone());
        game_without_artifact.add_object(blocker.clone());
        assert!(
            can_block(&attacker, &blocker, &game_without_artifact),
            "blocker should block when defending player controls no artifact"
        );

        let mut artifact = make_creature("Relic", 0, 1);
        artifact.id = ObjectId::from_raw(12);
        artifact.controller = bob;
        artifact.card_types.push(CardType::Artifact);

        let mut game_with_artifact = test_game_state();
        game_with_artifact.add_object(attacker.clone());
        game_with_artifact.add_object(blocker.clone());
        game_with_artifact.add_object(artifact);
        assert!(
            !can_block(&attacker, &blocker, &game_with_artifact),
            "blocker should fail when defending player controls an artifact"
        );
    }

    #[test]
    fn test_cant_be_blocked_when_defending_player_controls_required_card_type_conjunction() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut attacker = make_creature("Tanglewalker", 2, 2);
        attacker.id = ObjectId::from_raw(110);
        attacker.controller = alice;
        add_ability(
            &mut attacker,
            StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_types(vec![
                CardType::Artifact,
                CardType::Land,
            ]),
        );

        let mut blocker = make_creature("Blocker", 2, 2);
        blocker.id = ObjectId::from_raw(111);
        blocker.controller = bob;

        let mut game_with_only_artifact = test_game_state();
        let mut artifact_only = make_creature("Relic", 0, 1);
        artifact_only.id = ObjectId::from_raw(112);
        artifact_only.controller = bob;
        artifact_only.card_types = vec![CardType::Artifact];
        game_with_only_artifact.add_object(attacker.clone());
        game_with_only_artifact.add_object(blocker.clone());
        game_with_only_artifact.add_object(artifact_only);
        assert!(
            can_block(&attacker, &blocker, &game_with_only_artifact),
            "blocker should still block when defending player controls only an artifact"
        );

        let mut game_with_only_land = test_game_state();
        let mut land_only = make_creature("Field", 0, 1);
        land_only.id = ObjectId::from_raw(113);
        land_only.controller = bob;
        land_only.card_types = vec![CardType::Land];
        game_with_only_land.add_object(attacker.clone());
        game_with_only_land.add_object(blocker.clone());
        game_with_only_land.add_object(land_only);
        assert!(
            can_block(&attacker, &blocker, &game_with_only_land),
            "blocker should still block when defending player controls only a land"
        );

        let mut game_with_artifact_land = test_game_state();
        let mut artifact_land = make_creature("Seat of Synod", 0, 1);
        artifact_land.id = ObjectId::from_raw(114);
        artifact_land.controller = bob;
        artifact_land.card_types = vec![CardType::Artifact, CardType::Land];
        game_with_artifact_land.add_object(attacker);
        game_with_artifact_land.add_object(blocker);
        game_with_artifact_land.add_object(artifact_land);
        assert!(
            !can_block(
                &game_with_artifact_land
                    .object(ObjectId::from_raw(110))
                    .expect("attacker should exist")
                    .clone(),
                &game_with_artifact_land
                    .object(ObjectId::from_raw(111))
                    .expect("blocker should exist")
                    .clone(),
                &game_with_artifact_land
            ),
            "blocker should fail only when defending player controls an artifact land permanent"
        );
    }

    #[test]
    fn test_cant_block_specific_attacker_restriction() {
        let mut game = test_game_state();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let mut attacker = make_creature("Attacker", 2, 2);
        attacker.id = ObjectId::from_raw(20);
        attacker.controller = alice;

        let mut other_attacker = make_creature("Other Attacker", 2, 2);
        other_attacker.id = ObjectId::from_raw(21);
        other_attacker.controller = alice;

        let mut blocker = make_creature("Blocker", 2, 2);
        blocker.id = ObjectId::from_raw(22);
        blocker.controller = bob;

        game.add_object(attacker.clone());
        game.add_object(other_attacker.clone());
        game.add_object(blocker.clone());
        game.cant_effects
            .cant_block_specific_attackers
            .entry(blocker.id)
            .or_default()
            .insert(attacker.id);

        assert!(
            !can_block(&attacker, &blocker, &game),
            "blocker should fail against restricted attacker"
        );
        assert!(
            can_block(&other_attacker, &blocker, &game),
            "blocker should still block other attackers"
        );
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
    fn test_cant_attack_unless_defending_player_controls_island() {
        let mut game = test_game_state();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let mut attacker = make_creature("Serpent", 5, 5);
        attacker.controller = alice;
        add_ability(
            &mut attacker,
            StaticAbility::cant_attack_unless_defending_player_controls_land_subtype(
                crate::types::Subtype::Island,
            ),
        );

        // Bob controls no Island yet.
        assert!(!can_attack_defending_player(&attacker, bob, &game));

        // Add an Island under Bob's control.
        let mut island = make_creature("Island", 0, 0);
        island.id = ObjectId::from_raw(99);
        island.controller = bob;
        island.card_types = vec![CardType::Land];
        island.subtypes = vec![crate::types::Subtype::Island];
        game.add_object(island);

        assert!(can_attack_defending_player(&attacker, bob, &game));
    }

    #[test]
    fn test_cant_attack_unless_defending_player_is_poisoned() {
        let mut game = test_game_state();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let mut attacker = make_creature("Poison Seeker", 3, 2);
        attacker.controller = alice;
        add_ability(
            &mut attacker,
            StaticAbility::cant_attack_unless_condition(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::IsPoisoned,
                ),
                "Can't attack unless defending player is poisoned",
            ),
        );

        assert!(!can_attack_defending_player(&attacker, bob, &game));
        game.player_mut(bob)
            .expect("defending player should exist")
            .add_poison(1);
        assert!(can_attack_defending_player(&attacker, bob, &game));
    }

    #[test]
    fn test_cant_attack_unless_defending_player_is_the_monarch() {
        let mut game = test_game_state();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let mut attacker = make_creature("Crown-Hunter Hireling Variant", 4, 4);
        attacker.controller = alice;
        add_ability(
            &mut attacker,
            StaticAbility::cant_attack_unless_condition(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::IsMonarch,
                ),
                "Can't attack unless defending player is the monarch",
            ),
        );

        game.set_monarch(None);
        assert!(!can_attack_defending_player(&attacker, bob, &game));

        game.set_monarch(Some(alice));
        assert!(!can_attack_defending_player(&attacker, bob, &game));

        game.set_monarch(Some(bob));
        assert!(can_attack_defending_player(&attacker, bob, &game));
    }

    #[test]
    fn test_defender_cant_attack() {
        let mut game = test_game_state();
        let mut defender = make_creature("Defender", 0, 4);
        add_ability(&mut defender, StaticAbility::defender());
        game.add_object(defender.clone());
        assert!(!can_attack(&defender, &game));

        // With "can attack as though no defender"
        add_ability(
            &mut defender,
            StaticAbility::can_attack_as_though_no_defender(),
        );
        *game.object_mut(defender.id).unwrap() = defender.clone();
        assert!(can_attack(&defender, &game));
    }

    #[test]
    fn test_summoning_sickness() {
        let mut game = test_game_state();
        let creature = make_creature("New", 2, 2);
        game.add_object(creature.clone());
        game.set_summoning_sick(creature.id);
        assert!(!can_attack(&creature, &game));

        // With haste - add ability to creature
        let mut creature_with_haste = make_creature("New", 2, 2);
        add_ability(&mut creature_with_haste, StaticAbility::haste());
        game.add_object(creature_with_haste.clone());
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

    #[test]
    fn test_any_landwalk_checks_for_any_land() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let mut attacker = make_creature("Pathwalker", 2, 2);
        attacker.controller = alice;
        add_ability(&mut attacker, StaticAbility::any_landwalk());

        let mut blocker = make_creature("Blocker", 2, 2);
        blocker.controller = bob;

        let mut game = test_game_state();
        game.add_object(attacker.clone());
        game.add_object(blocker.clone());
        assert!(can_block(&attacker, &blocker, &game));

        let mut land = make_creature("Plains", 0, 1);
        land.controller = bob;
        land.card_types = vec![CardType::Land];
        land.subtypes = vec![crate::types::Subtype::Plains];
        game.add_object(land);

        assert!(!can_block(&attacker, &blocker, &game));
    }

    #[test]
    fn test_nonbasic_landwalk_requires_nonbasic_land() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let mut attacker = make_creature("Boots Walker", 2, 2);
        attacker.controller = alice;
        add_ability(&mut attacker, StaticAbility::nonbasic_landwalk());

        let mut blocker = make_creature("Blocker", 2, 2);
        blocker.controller = bob;

        let mut basic_land = make_creature("Forest", 0, 1);
        basic_land.controller = bob;
        basic_land.card_types = vec![CardType::Land];
        basic_land.subtypes = vec![crate::types::Subtype::Forest];
        basic_land.supertypes = vec![Supertype::Basic];

        let mut nonbasic_land = make_creature("Maze", 0, 1);
        nonbasic_land.controller = bob;
        nonbasic_land.card_types = vec![CardType::Land];
        nonbasic_land.subtypes = vec![crate::types::Subtype::Desert];

        let mut game = test_game_state();
        game.add_object(attacker.clone());
        game.add_object(blocker.clone());
        game.add_object(basic_land);
        assert!(can_block(&attacker, &blocker, &game));

        game.add_object(nonbasic_land);
        assert!(!can_block(&attacker, &blocker, &game));
    }

    #[test]
    fn test_snow_landwalk_requires_snow_land_of_matching_type() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let mut attacker = make_creature("Snow Scout", 2, 2);
        attacker.controller = alice;
        add_ability(
            &mut attacker,
            StaticAbility::snow_landwalk(crate::types::Subtype::Forest),
        );

        let mut blocker = make_creature("Blocker", 2, 2);
        blocker.controller = bob;

        let mut forest = make_creature("Forest", 0, 1);
        forest.controller = bob;
        forest.card_types = vec![CardType::Land];
        forest.subtypes = vec![crate::types::Subtype::Forest];

        let mut snow_forest = forest.clone();
        snow_forest.id = ObjectId::from_raw(5000);
        snow_forest.stable_id = StableId::from_raw(5000);
        snow_forest.supertypes = vec![Supertype::Snow];

        let mut game = test_game_state();
        game.add_object(attacker.clone());
        game.add_object(blocker.clone());
        game.add_object(forest);
        assert!(can_block(&attacker, &blocker, &game));

        game.add_object(snow_forest);
        assert!(!can_block(&attacker, &blocker, &game));
    }

    #[test]
    fn test_attached_chosen_landwalk_uses_source_choice() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let mut attacker = make_creature("Traveler", 2, 2);
        attacker.controller = alice;
        attacker.owner = alice;

        let mut blocker = make_creature("Blocker", 2, 2);
        blocker.controller = bob;
        blocker.owner = bob;

        let mut aura = make_creature("Traveler's Cloak", 0, 0);
        aura.controller = alice;
        aura.owner = alice;
        aura.card_types = vec![CardType::Enchantment];
        aura.subtypes = vec![crate::types::Subtype::Aura];
        aura.attached_to = Some(attacker.id);
        aura.abilities.push(Ability::static_ability(
            StaticAbility::attached_chosen_landwalk_grant(
                "enchanted creature has landwalk of the chosen type".to_string(),
                false,
            ),
        ));

        let mut chosen_land = make_creature("Desert", 0, 1);
        chosen_land.controller = bob;
        chosen_land.owner = bob;
        chosen_land.card_types = vec![CardType::Land];
        chosen_land.subtypes = vec![crate::types::Subtype::Desert];

        let mut game = test_game_state();
        let attacker_id = attacker.id;
        let blocker_id = blocker.id;
        let aura_id = aura.id;
        game.add_object(attacker);
        game.add_object(blocker);
        game.add_object(aura);
        game.add_object(chosen_land);
        game.set_chosen_land_type(aura_id, crate::types::Subtype::Desert);

        assert!(!can_block(
            game.object(attacker_id).expect("attacker should exist"),
            game.object(blocker_id).expect("blocker should exist"),
            &game,
        ));
    }
}

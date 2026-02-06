//! Target computation functions.
//!
//! This module provides functions for computing legal targets
//! for spells and abilities.

use crate::ability::AbilityKind;
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::Object;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

use super::types::{TargetingInvalidReason, TargetingResult};

/// Check if a source can target a specific object.
///
/// This function performs all targeting legality checks:
/// - Shroud (can't be targeted by anything)
/// - Hexproof (can't be targeted by opponents)
/// - HexproofFrom (can't be targeted by sources matching filter)
/// - Protection (can't be targeted by sources matching quality)
/// - "Can't be targeted" effects
///
/// Note: This does NOT check ward - ward only triggers when the spell/ability
/// is actually cast/activated with the target, not during target computation.
pub fn can_target_object(
    game: &GameState,
    target_id: ObjectId,
    source_id: ObjectId,
    caster: PlayerId,
) -> TargetingResult {
    let Some(target) = game.object(target_id) else {
        return TargetingResult::Invalid(TargetingInvalidReason::DoesntExist);
    };

    let Some(_source) = game.object(source_id) else {
        // Source no longer exists - targeting can still be legal
        // (we'll use LKI for the source's characteristics)
        return TargetingResult::legal();
    };

    // Check if target is on the battlefield
    if target.zone != Zone::Battlefield && target.zone != Zone::Stack {
        return TargetingResult::Invalid(TargetingInvalidReason::NotOnBattlefield);
    }

    // Get calculated abilities for the target (to account for effects like Humility)
    let target_abilities = game
        .calculated_characteristics(target_id)
        .map(|c| c.static_abilities)
        .unwrap_or_else(|| extract_static_abilities(&target.abilities));

    // Check for shroud
    if target_abilities.iter().any(|a| a.has_shroud()) {
        return TargetingResult::Invalid(TargetingInvalidReason::HasShroud);
    }

    // Check for hexproof (only blocks opponents)
    if target_abilities.iter().any(|a| a.has_hexproof()) && target.controller != caster {
        return TargetingResult::Invalid(TargetingInvalidReason::HasHexproof);
    }

    // Check for HexproofFrom
    for ability in &target_abilities {
        if let Some(filter) = ability.hexproof_from_filter()
            && source_matches_hexproof_from(game, source_id, filter, caster)
        {
            return TargetingResult::Invalid(TargetingInvalidReason::HasHexproofFrom);
        }
    }

    // Check for protection
    if has_protection_from_source(game, target_id, source_id) {
        return TargetingResult::Invalid(TargetingInvalidReason::HasProtection);
    }

    // Check CantEffectTracker for "can't be targeted" effects
    // Note: This includes both shroud and hexproof tracked separately
    if game.is_untargetable(target_id) && target.controller != caster {
        return TargetingResult::Invalid(TargetingInvalidReason::CantBeTargeted);
    }

    TargetingResult::legal()
}

/// Check if a source matches a HexproofFrom filter.
fn source_matches_hexproof_from(
    game: &GameState,
    source_id: ObjectId,
    filter: &ObjectFilter,
    caster: PlayerId,
) -> bool {
    let Some(source) = game.object(source_id) else {
        return false;
    };

    // Build a filter context for the source
    let filter_ctx = game.filter_context_for(caster, Some(source_id));

    filter.matches(source, &filter_ctx, game)
}

/// Check if a permanent has protection from a source.
pub fn has_protection_from_source(
    game: &GameState,
    target_id: ObjectId,
    source_id: ObjectId,
) -> bool {
    let Some(target) = game.object(target_id) else {
        return false;
    };
    let Some(source) = game.object(source_id) else {
        return false;
    };

    // Get calculated abilities for the target
    let target_abilities = game
        .calculated_characteristics(target_id)
        .map(|c| c.static_abilities)
        .unwrap_or_else(|| extract_static_abilities(&target.abilities));

    for ability in target_abilities {
        if ability.has_protection()
            && let Some(protection_from) = ability.protection_from()
            && source_matches_protection(source, protection_from, game)
        {
            return true;
        }
    }

    false
}

/// Check if a source matches a protection quality.
pub fn source_matches_protection(
    source: &Object,
    protection: &crate::ability::ProtectionFrom,
    game: &GameState,
) -> bool {
    use crate::ability::ProtectionFrom;

    // Get calculated characteristics for the source
    let source_colors = game
        .calculated_characteristics(source.id)
        .map(|c| c.colors)
        .unwrap_or_else(|| source.colors());
    let source_types = game
        .calculated_characteristics(source.id)
        .map(|c| c.card_types)
        .unwrap_or_else(|| source.card_types.clone());

    match protection {
        // Protection from a color or set of colors
        ProtectionFrom::Color(color_set) => {
            // Check if source has any of the colors in the set
            !source_colors.intersection(*color_set).is_empty()
        }
        // Protection from all colors
        ProtectionFrom::AllColors => !source_colors.is_empty(),
        // Protection from creatures
        ProtectionFrom::Creatures => source_types.contains(&CardType::Creature),
        // Protection from a card type
        ProtectionFrom::CardType(card_type) => source_types.contains(card_type),
        // Protection from permanents matching a filter
        ProtectionFrom::Permanents(filter) => {
            // Use active player for context since we don't have a specific controller
            let filter_ctx = game.filter_context_for(game.turn.active_player, None);
            filter.matches(source, &filter_ctx, game)
        }
        // Protection from everything
        ProtectionFrom::Everything => true,
        // Protection from colorless (sources with no colors)
        ProtectionFrom::Colorless => source_colors.is_empty(),
    }
}

/// Extract static abilities from a list of abilities.
fn extract_static_abilities(abilities: &[crate::ability::Ability]) -> Vec<StaticAbility> {
    abilities
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

/// Compute all legal targets for a target specification.
///
/// This is the main entry point for determining what can be targeted.
pub fn compute_legal_targets(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<Target> {
    match spec {
        // Target wrapper - recursively compute targets from inner spec
        ChooseSpec::Target(inner) => compute_legal_targets(game, inner, caster, source_id),
        // WithCount wrapper - recursively compute targets from inner spec
        ChooseSpec::WithCount(inner, _) => compute_legal_targets(game, inner, caster, source_id),
        ChooseSpec::AnyTarget => compute_any_targets(game, caster, source_id),
        ChooseSpec::Player(filter) => compute_player_targets(game, filter, caster),
        ChooseSpec::Object(filter) => compute_object_targets(game, filter, caster, source_id),
        // These don't require selection - they're resolved at execution time
        ChooseSpec::Source
        | ChooseSpec::SourceController
        | ChooseSpec::SourceOwner
        | ChooseSpec::SpecificObject(_)
        | ChooseSpec::SpecificPlayer(_)
        | ChooseSpec::Tagged(_)
        | ChooseSpec::All(_)
        | ChooseSpec::EachPlayer(_)
        | ChooseSpec::Iterated => Vec::new(),
    }
}

/// Compute legal targets for "any target" (player or creature/planeswalker).
fn compute_any_targets(
    game: &GameState,
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<Target> {
    let mut targets = Vec::new();

    // All players in the game
    for player in &game.players {
        if player.is_in_game() {
            targets.push(Target::Player(player.id));
        }
    }

    // All creatures and planeswalkers on battlefield
    for &obj_id in &game.battlefield {
        if let Some(obj) = game.object(obj_id) {
            if !obj.has_card_type(CardType::Creature) && !obj.has_card_type(CardType::Planeswalker)
            {
                continue;
            }

            // Check targeting legality
            if let Some(src_id) = source_id {
                match can_target_object(game, obj_id, src_id, caster) {
                    TargetingResult::Legal { .. } => targets.push(Target::Object(obj_id)),
                    TargetingResult::Invalid(_) => {}
                }
            } else {
                // No source - check basic hexproof/shroud
                let is_untargetable = game.is_untargetable(obj_id);
                let is_controlled_by_caster = obj.controller == caster;
                if !is_untargetable || is_controlled_by_caster {
                    targets.push(Target::Object(obj_id));
                }
            }
        }
    }

    targets
}

/// Compute legal player targets.
fn compute_player_targets(
    game: &GameState,
    filter: &PlayerFilter,
    controller: PlayerId,
) -> Vec<Target> {
    let filter_ctx = crate::target::FilterContext::new(controller)
        .with_opponents(
            game.players
                .iter()
                .filter(|p| p.id != controller && p.is_in_game())
                .map(|p| p.id)
                .collect(),
        )
        .with_active_player(game.turn.active_player);

    game.players
        .iter()
        .filter(|p| p.is_in_game())
        .filter(|p| filter.matches_player(p.id, &filter_ctx))
        .map(|p| Target::Player(p.id))
        .collect()
}

/// Compute legal object targets.
fn compute_object_targets(
    game: &GameState,
    filter: &ObjectFilter,
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<Target> {
    let mut targets = Vec::new();

    // Build filter context
    let filter_ctx = game.filter_context_for(caster, source_id);

    // Check battlefield
    for &obj_id in &game.battlefield {
        if let Some(obj) = game.object(obj_id) {
            if !filter.matches(obj, &filter_ctx, game) {
                continue;
            }

            // Check targeting legality
            if let Some(src_id) = source_id {
                match can_target_object(game, obj_id, src_id, caster) {
                    TargetingResult::Legal { .. } => targets.push(Target::Object(obj_id)),
                    TargetingResult::Invalid(_) => {}
                }
            } else {
                // No source - check basic hexproof/shroud
                let is_untargetable = game.is_untargetable(obj_id);
                let is_controlled_by_caster = obj.controller == caster;
                if !is_untargetable || is_controlled_by_caster {
                    targets.push(Target::Object(obj_id));
                }
            }
        }
    }

    // Check stack for spells (for counterspells)
    if filter.zone == Some(Zone::Stack) || filter.zone.is_none() {
        for entry in &game.stack {
            if let Some(obj) = game.object(entry.object_id)
                && filter.matches(obj, &filter_ctx, game)
            {
                targets.push(Target::Object(entry.object_id));
            }
        }
    }

    targets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{Ability, AbilityKind, ProtectionFrom};
    use crate::card::{CardBuilder, PowerToughness};
    use crate::color::{Color, ColorSet};
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(id: u32, name: &str, controller: PlayerId) -> Object {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        let mut obj = Object::from_card(
            ObjectId::from_raw(id as u64),
            &card,
            controller,
            Zone::Battlefield,
        );
        obj.controller = controller;
        obj
    }

    fn add_static_ability(obj: &mut Object, ability: StaticAbility) {
        obj.abilities.push(Ability {
            kind: AbilityKind::Static(ability),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        });
    }

    #[test]
    fn test_can_target_basic_creature() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        let target = create_creature(1, "Target Creature", p1);
        let source = create_creature(2, "Source Creature", p0);

        let target_id = target.id;
        let source_id = source.id;

        game.add_object(target);
        game.add_object(source);

        let result = can_target_object(&game, target_id, source_id, p0);
        assert!(result.is_legal(), "Basic creature should be targetable");
    }

    #[test]
    fn test_shroud_blocks_all_targeting() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        let mut target = create_creature(1, "Shrouded Creature", p1);
        add_static_ability(&mut target, StaticAbility::shroud());

        let source = create_creature(2, "Source Creature", p0);

        let target_id = target.id;
        let source_id = source.id;

        game.add_object(target);
        game.add_object(source);

        // Opponent can't target
        let result = can_target_object(&game, target_id, source_id, p0);
        assert!(matches!(
            result,
            TargetingResult::Invalid(TargetingInvalidReason::HasShroud)
        ));

        // Even controller can't target a shrouded permanent
        let result = can_target_object(&game, target_id, source_id, p1);
        assert!(matches!(
            result,
            TargetingResult::Invalid(TargetingInvalidReason::HasShroud)
        ));
    }

    #[test]
    fn test_hexproof_blocks_opponent_targeting() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        let mut target = create_creature(1, "Hexproof Creature", p1);
        add_static_ability(&mut target, StaticAbility::hexproof());

        let source = create_creature(2, "Source Creature", p0);

        let target_id = target.id;
        let source_id = source.id;

        game.add_object(target);
        game.add_object(source);

        // Opponent can't target
        let result = can_target_object(&game, target_id, source_id, p0);
        assert!(matches!(
            result,
            TargetingResult::Invalid(TargetingInvalidReason::HasHexproof)
        ));

        // Controller CAN target their own hexproof creature
        let result = can_target_object(&game, target_id, source_id, p1);
        assert!(
            result.is_legal(),
            "Controller should be able to target own hexproof creature"
        );
    }

    #[test]
    fn test_hexproof_from_blocks_matching_sources() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        // Target has "Hexproof from black"
        let mut target = create_creature(1, "Protected Creature", p1);
        let black_filter = ObjectFilter {
            colors: Some(ColorSet::from(Color::Black)),
            ..Default::default()
        };
        add_static_ability(&mut target, StaticAbility::hexproof_from(black_filter));

        // Create a black source
        let card = CardBuilder::new(CardId::from_raw(2), "Black Source")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
            .card_types(vec![CardType::Instant])
            .build();
        let mut black_source =
            Object::from_card(ObjectId::from_raw(2), &card, p0, Zone::Battlefield);
        black_source.controller = p0;

        let target_id = target.id;
        let black_source_id = black_source.id;

        game.add_object(target);
        game.add_object(black_source);

        // Black source can't target creature with hexproof from black
        let result = can_target_object(&game, target_id, black_source_id, p0);
        assert!(matches!(
            result,
            TargetingResult::Invalid(TargetingInvalidReason::HasHexproofFrom)
        ));
    }

    #[test]
    fn test_protection_prevents_targeting() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        // Target has protection from red
        let mut target = create_creature(1, "Pro-Red Creature", p1);
        add_static_ability(
            &mut target,
            StaticAbility::protection(ProtectionFrom::Color(ColorSet::from(Color::Red))),
        );

        // Create a red source
        let card = CardBuilder::new(CardId::from_raw(2), "Red Source")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .build();
        let mut red_source = Object::from_card(ObjectId::from_raw(2), &card, p0, Zone::Battlefield);
        red_source.controller = p0;

        let target_id = target.id;
        let red_source_id = red_source.id;

        game.add_object(target);
        game.add_object(red_source);

        // Red source can't target creature with protection from red
        let result = can_target_object(&game, target_id, red_source_id, p0);
        assert!(matches!(
            result,
            TargetingResult::Invalid(TargetingInvalidReason::HasProtection)
        ));
    }

    #[test]
    fn test_protection_from_all_colors() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        // Target has protection from all colors
        let mut target = create_creature(1, "Pro-Colors Creature", p1);
        add_static_ability(
            &mut target,
            StaticAbility::protection(ProtectionFrom::AllColors),
        );

        // Create a blue source
        let card = CardBuilder::new(CardId::from_raw(2), "Blue Source")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Instant])
            .build();
        let mut blue_source =
            Object::from_card(ObjectId::from_raw(2), &card, p0, Zone::Battlefield);
        blue_source.controller = p0;

        let target_id = target.id;
        let blue_source_id = blue_source.id;

        game.add_object(target);
        game.add_object(blue_source);

        // Colored source can't target creature with protection from all colors
        let result = can_target_object(&game, target_id, blue_source_id, p0);
        assert!(matches!(
            result,
            TargetingResult::Invalid(TargetingInvalidReason::HasProtection)
        ));
    }

    #[test]
    fn test_colorless_bypasses_pro_colors() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        // Target has protection from all colors
        let mut target = create_creature(1, "Pro-Colors Creature", p1);
        add_static_ability(
            &mut target,
            StaticAbility::protection(ProtectionFrom::AllColors),
        );

        // Create a colorless source
        let card = CardBuilder::new(CardId::from_raw(2), "Colorless Source")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Instant])
            .build();
        let mut colorless_source =
            Object::from_card(ObjectId::from_raw(2), &card, p0, Zone::Battlefield);
        colorless_source.controller = p0;

        let target_id = target.id;
        let colorless_source_id = colorless_source.id;

        game.add_object(target);
        game.add_object(colorless_source);

        // Colorless source CAN target creature with protection from all colors
        let result = can_target_object(&game, target_id, colorless_source_id, p0);
        assert!(
            result.is_legal(),
            "Colorless source should bypass protection from all colors"
        );
    }

    #[test]
    fn test_protection_from_creatures() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        // Target has protection from creatures
        let mut target = create_creature(1, "Pro-Creatures", p1);
        add_static_ability(
            &mut target,
            StaticAbility::protection(ProtectionFrom::Creatures),
        );

        // Create a creature source
        let source = create_creature(2, "Attacker", p0);

        let target_id = target.id;
        let source_id = source.id;

        game.add_object(target);
        game.add_object(source);

        // Creature source can't target creature with protection from creatures
        let result = can_target_object(&game, target_id, source_id, p0);
        assert!(matches!(
            result,
            TargetingResult::Invalid(TargetingInvalidReason::HasProtection)
        ));
    }

    #[test]
    fn test_nonexistent_target() {
        let game = create_test_game();
        let p0 = PlayerId::from_index(0);

        let nonexistent_target = ObjectId::from_raw(999);
        let source_id = ObjectId::from_raw(1);

        let result = can_target_object(&game, nonexistent_target, source_id, p0);
        assert!(matches!(
            result,
            TargetingResult::Invalid(TargetingInvalidReason::DoesntExist)
        ));
    }

    #[test]
    fn test_target_not_on_battlefield() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        // Create a creature in the graveyard
        let card = CardBuilder::new(CardId::from_raw(1), "Dead Creature")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let target = Object::from_card(ObjectId::from_raw(1), &card, p1, Zone::Graveyard);

        let source = create_creature(2, "Source", p0);

        let target_id = target.id;
        let source_id = source.id;

        game.add_object(target);
        game.add_object(source);

        let result = can_target_object(&game, target_id, source_id, p0);
        assert!(matches!(
            result,
            TargetingResult::Invalid(TargetingInvalidReason::NotOnBattlefield)
        ));
    }
}

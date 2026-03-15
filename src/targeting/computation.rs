//! Target computation functions.
//!
//! This module provides functions for computing legal targets
//! for spells and abilities.

use crate::ability::extract_static_abilities;
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::Object;
use crate::tag::TagKey;
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
    let view = crate::derived_view::DerivedGameView::new(game);
    can_target_object_with_view(game, target_id, source_id, caster, &view)
}

pub(crate) fn can_target_object_with_view(
    game: &GameState,
    target_id: ObjectId,
    source_id: ObjectId,
    caster: PlayerId,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> TargetingResult {
    let Some(target) = game.object(target_id) else {
        return TargetingResult::Invalid(TargetingInvalidReason::DoesntExist);
    };

    let Some(_source) = game.object(source_id) else {
        // Source no longer exists - targeting can still be legal
        // (we'll use LKI for the source's characteristics)
        return TargetingResult::legal();
    };

    // Most targeting restrictions in this function apply only to permanents
    // and stack objects. Cards in other zones are generally targetable unless
    // constrained by the caller's filter.
    if target.zone != Zone::Battlefield && target.zone != Zone::Stack {
        return TargetingResult::legal();
    }

    // Get calculated abilities for the target (to account for effects like Humility)
    let target_abilities = view
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
    if has_protection_from_source_with_view(game, target_id, source_id, view) {
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
    let view = crate::derived_view::DerivedGameView::new(game);
    has_protection_from_source_with_view(game, target_id, source_id, &view)
}

pub(crate) fn has_protection_from_source_with_view(
    game: &GameState,
    target_id: ObjectId,
    source_id: ObjectId,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    let Some(target) = game.object(target_id) else {
        return false;
    };
    let Some(source) = game.object(source_id) else {
        return false;
    };

    // Get calculated abilities for the target
    let target_abilities = view
        .calculated_characteristics(target_id)
        .map(|c| c.static_abilities)
        .unwrap_or_else(|| extract_static_abilities(&target.abilities));

    for ability in target_abilities {
        if ability.has_protection()
            && let Some(protection_from) = ability.protection_from()
            && source_matches_protection_with_view(source, protection_from, game, view)
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
    let view = crate::derived_view::DerivedGameView::new(game);
    source_matches_protection_with_view(source, protection, game, &view)
}

pub(crate) fn source_matches_protection_with_view(
    source: &Object,
    protection: &crate::ability::ProtectionFrom,
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    use crate::ability::ProtectionFrom;

    // Get calculated characteristics for the source
    let source_colors = view
        .calculated_characteristics(source.id)
        .map(|c| c.colors)
        .unwrap_or_else(|| source.colors());
    let source_types = view
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

/// Compute all legal targets for a target specification.
///
/// This is the main entry point for determining what can be targeted.
pub fn compute_legal_targets(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<Target> {
    let view = crate::derived_view::DerivedGameView::new(game);
    compute_legal_targets_with_tagged_objects_with_view(game, spec, caster, source_id, None, &view)
}

/// Compute legal targets with optional tagged-object filter context.
///
/// This is used by effects that target objects based on previously tagged objects
/// (for example, "creatures that crewed it this turn").
pub fn compute_legal_targets_with_tagged_objects(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    tagged_objects: Option<
        &std::collections::HashMap<TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    >,
) -> Vec<Target> {
    let view = crate::derived_view::DerivedGameView::new(game);
    compute_legal_targets_with_tagged_objects_with_view(
        game,
        spec,
        caster,
        source_id,
        tagged_objects,
        &view,
    )
}

pub(crate) fn compute_legal_targets_with_tagged_objects_with_view(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    tagged_objects: Option<
        &std::collections::HashMap<TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    >,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    match spec {
        // Target wrapper - recursively compute targets from inner spec
        ChooseSpec::Target(inner) => compute_legal_targets_with_tagged_objects_with_view(
            game,
            inner,
            caster,
            source_id,
            tagged_objects,
            view,
        ),
        // WithCount wrapper - recursively compute targets from inner spec
        ChooseSpec::WithCount(inner, _) => compute_legal_targets_with_tagged_objects_with_view(
            game,
            inner,
            caster,
            source_id,
            tagged_objects,
            view,
        ),
        ChooseSpec::AnyTarget => compute_any_targets_with_view(game, caster, source_id, view),
        ChooseSpec::AnyOtherTarget => {
            compute_any_other_targets_with_view(game, caster, source_id, view)
        }
        ChooseSpec::PlayerOrPlaneswalker(filter) => {
            compute_player_or_planeswalker_targets_with_view(game, filter, caster, source_id, view)
        }
        ChooseSpec::AttackedPlayerOrPlaneswalker => Vec::new(),
        ChooseSpec::Player(filter) => compute_player_targets(game, filter, caster),
        ChooseSpec::Object(filter) => {
            compute_object_targets_with_view(game, filter, caster, source_id, tagged_objects, view)
        }
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

fn compute_player_or_planeswalker_targets_with_view(
    game: &GameState,
    player_filter: &PlayerFilter,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    let mut targets = compute_player_targets(game, player_filter, caster);

    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        if !view.object_has_card_type(obj_id, CardType::Planeswalker) {
            continue;
        }

        if let Some(src_id) = source_id {
            match can_target_object_with_view(game, obj_id, src_id, caster, view) {
                TargetingResult::Legal { .. } => targets.push(Target::Object(obj_id)),
                TargetingResult::Invalid(_) => {}
            }
        } else {
            let is_untargetable = game.is_untargetable(obj_id);
            let is_controlled_by_caster = obj.controller == caster;
            if !is_untargetable || is_controlled_by_caster {
                targets.push(Target::Object(obj_id));
            }
        }
    }

    targets
}

fn compute_any_targets_with_view(
    game: &GameState,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    let mut targets = Vec::new();

    // All players in the game
    for player in &game.players {
        if player.is_in_game() {
            targets.push(Target::Player(player.id));
        }
    }

    // All creatures, planeswalkers, and battles on the battlefield
    for &obj_id in &game.battlefield {
        if let Some(obj) = game.object(obj_id) {
            if !view.object_has_card_type(obj_id, CardType::Creature)
                && !view.object_has_card_type(obj_id, CardType::Planeswalker)
                && !view.object_has_card_type(obj_id, CardType::Battle)
            {
                continue;
            }

            // Check targeting legality
            if let Some(src_id) = source_id {
                match can_target_object_with_view(game, obj_id, src_id, caster, view) {
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

fn compute_any_other_targets_with_view(
    game: &GameState,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    let mut targets = compute_any_targets_with_view(game, caster, source_id, view);
    if let Some(source_id) = source_id {
        targets.retain(|target| !matches!(target, Target::Object(id) if *id == source_id));
    }
    targets
}

/// Compute legal player targets.
fn compute_player_targets(
    game: &GameState,
    filter: &PlayerFilter,
    controller: PlayerId,
) -> Vec<Target> {
    // Unwrap Target wrapper — during legal target computation we want to know
    // which players *could be* targeted, not which are already targeted.
    let filter = match filter {
        PlayerFilter::Target(inner) => inner.as_ref(),
        other => other,
    };

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
        .filter(|p| game.can_target_player(p.id))
        .filter(|p| filter.matches_player(p.id, &filter_ctx))
        .map(|p| Target::Player(p.id))
        .collect()
}

fn compute_object_targets_with_view(
    game: &GameState,
    filter: &ObjectFilter,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    tagged_objects: Option<
        &std::collections::HashMap<TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    >,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    let mut targets = Vec::new();

    // Build filter context
    let mut filter_ctx = game.filter_context_for(caster, source_id);
    if let Some(tagged) = tagged_objects {
        filter_ctx = filter_ctx.with_tagged_objects(tagged);
    }

    let candidate_ids = view.candidate_ids_for_filter(filter);
    for object_id in candidate_ids {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        if !filter.matches(object, &filter_ctx, game) {
            continue;
        }

        if let Some(src_id) = source_id {
            if can_target_object_with_view(game, object_id, src_id, caster, view).is_legal() {
                targets.push(Target::Object(object_id));
            }
            continue;
        }

        if object.zone != Zone::Battlefield && object.zone != Zone::Stack {
            targets.push(Target::Object(object_id));
            continue;
        }

        let is_untargetable = game.is_untargetable(object_id);
        let is_controlled_by_caster = object.controller == caster;
        if !is_untargetable || is_controlled_by_caster {
            targets.push(Target::Object(object_id));
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
    use crate::effect::Comparison;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::{AnthemCountExpression, GrantAbility, StaticAbility};

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

    fn create_battle(id: u32, name: &str, controller: PlayerId) -> Object {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Battle])
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

    fn create_land(id: u32, name: &str, controller: PlayerId) -> Object {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Land])
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
    fn test_conditional_granted_shroud_blocks_targeting_when_no_untapped_lands() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        let mut target = create_creature(1, "Vintara Snapper Variant", p1);
        add_static_ability(
            &mut target,
            StaticAbility::new(
                GrantAbility::source(StaticAbility::shroud()).with_condition(
                    crate::ConditionExpr::CountComparison {
                        count: AnthemCountExpression::MatchingFilter(
                            ObjectFilter::land().you_control().untapped(),
                        ),
                        comparison: Comparison::LessThanOrEqual(0),
                        display: Some("you control no untapped lands".to_string()),
                    },
                ),
            ),
        );

        let source = create_creature(2, "Source Creature", p0);
        let land = create_land(3, "Forest", p1);

        let target_id = target.id;
        let source_id = source.id;
        let land_id = land.id;

        game.add_object(target);
        game.add_object(source);
        game.add_object(land);

        let result = can_target_object(&game, target_id, source_id, p0);
        assert!(
            result.is_legal(),
            "Creature should be targetable while its controller still has an untapped land"
        );

        game.tap(land_id);

        let result = can_target_object(&game, target_id, source_id, p0);
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
    fn test_any_target_includes_battles() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        let source = create_creature(1, "Source Creature", p0);
        let battle = create_battle(2, "Invasion of Test", p1);
        let battle_id = battle.id;
        let source_id = source.id;

        game.add_object(source);
        game.add_object(battle);

        let legal_targets =
            compute_legal_targets(&game, &ChooseSpec::AnyTarget, p0, Some(source_id));

        assert!(
            legal_targets.contains(&Target::Object(battle_id)),
            "battle permanents should be legal 'any target' choices"
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
        // Zone filtering is now the caller's responsibility (via ObjectFilter),
        // so can_target_object considers non-battlefield objects targetable.
        assert!(matches!(result, TargetingResult::Legal { .. }));
    }
}

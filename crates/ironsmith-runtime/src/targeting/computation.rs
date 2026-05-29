//! Target computation functions.
//!
//! This module provides functions for computing legal targets
//! for spells and abilities.

use crate::ability::extract_static_abilities;
use crate::filter::ObjectFilterExt as _;
use crate::filter::player_filter_matches_game;
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::Object;
use crate::snapshot::ObjectSnapshot;
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
/// - HexproofFrom (can't be targeted by opponents' sources matching filter)
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
    can_target_object_with_view_and_source_snapshot(game, target_id, source_id, None, caster, view)
}

pub(crate) fn can_target_object_with_view_and_source_snapshot(
    game: &GameState,
    target_id: ObjectId,
    source_id: ObjectId,
    source_snapshot: Option<&ObjectSnapshot>,
    caster: PlayerId,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> TargetingResult {
    let Some(target) = game.object(target_id) else {
        return TargetingResult::Invalid(TargetingInvalidReason::DoesntExist);
    };

    let Some(source) = game.object(source_id) else {
        // Rule 608.2b: if the source of an ability has left its expected zone,
        // resolution-time target legality uses that source's last known information.
        let Some(source_snapshot) = source_snapshot else {
            return TargetingResult::legal();
        };
        return can_target_object_from_source_snapshot_with_view(
            game,
            target_id,
            source_snapshot,
            caster,
            view,
        );
    };

    // Most targeting restrictions in this function apply only to permanents
    // and stack objects. Cards in other zones are generally targetable unless
    // constrained by the caller's filter.
    if target.zone != Zone::Battlefield && target.zone != Zone::Stack {
        return TargetingResult::legal();
    }

    // Get calculated abilities for the target (to account for effects like Humility)
    let target_abilities = view
        .static_abilities_rc(target_id)
        .unwrap_or_else(|| std::rc::Rc::new(extract_static_abilities(&target.abilities)));

    // Check for shroud
    if target_abilities.iter().any(|a| a.has_shroud()) {
        return TargetingResult::Invalid(TargetingInvalidReason::HasShroud);
    }

    // Check for hexproof (only blocks opponents)
    if target_abilities.iter().any(|a| a.has_hexproof()) && game.controller_of(target) != caster {
        return TargetingResult::Invalid(TargetingInvalidReason::HasHexproof);
    }

    // Check for HexproofFrom
    if game.controller_of(target) != game.controller_of(source) {
        for ability in target_abilities.iter() {
            if let Some(filter) = ability.hexproof_from_filter()
                && source_matches_hexproof_from(game, source_id, filter, caster)
            {
                return TargetingResult::Invalid(TargetingInvalidReason::HasHexproofFrom);
            }
        }
    }

    // Check for protection
    if has_protection_from_source_with_view(game, target_id, source_id, view) {
        return TargetingResult::Invalid(TargetingInvalidReason::HasProtection);
    }

    // Check CantEffectTracker for "can't be targeted" effects
    // Note: This includes both shroud and hexproof tracked separately
    if game.is_untargetable(target_id) && game.controller_of(target) != caster {
        return TargetingResult::Invalid(TargetingInvalidReason::CantBeTargeted);
    }

    TargetingResult::legal()
}

fn can_target_object_from_source_snapshot_with_view(
    game: &GameState,
    target_id: ObjectId,
    source_snapshot: &ObjectSnapshot,
    caster: PlayerId,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> TargetingResult {
    let Some(target) = game.object(target_id) else {
        return TargetingResult::Invalid(TargetingInvalidReason::DoesntExist);
    };

    if target.zone != Zone::Battlefield && target.zone != Zone::Stack {
        return TargetingResult::legal();
    }

    let target_abilities = view
        .static_abilities_rc(target_id)
        .unwrap_or_else(|| std::rc::Rc::new(extract_static_abilities(&target.abilities)));

    if target_abilities.iter().any(|a| a.has_shroud()) {
        return TargetingResult::Invalid(TargetingInvalidReason::HasShroud);
    }

    if target_abilities.iter().any(|a| a.has_hexproof()) && game.controller_of(target) != caster {
        return TargetingResult::Invalid(TargetingInvalidReason::HasHexproof);
    }

    if game.controller_of(target) != source_snapshot.controller {
        for ability in target_abilities.iter() {
            if let Some(filter) = ability.hexproof_from_filter()
                && source_snapshot_matches_hexproof_from(game, source_snapshot, filter, caster)
            {
                return TargetingResult::Invalid(TargetingInvalidReason::HasHexproofFrom);
            }
        }
    }

    if has_protection_from_source_snapshot_with_view(game, target_id, source_snapshot, view) {
        return TargetingResult::Invalid(TargetingInvalidReason::HasProtection);
    }

    if game.is_untargetable(target_id) && game.controller_of(target) != caster {
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

fn source_snapshot_matches_hexproof_from(
    game: &GameState,
    source_snapshot: &ObjectSnapshot,
    filter: &ObjectFilter,
    caster: PlayerId,
) -> bool {
    let filter_ctx = game.filter_context_for(caster, Some(source_snapshot.object_id));
    filter.matches_snapshot(source_snapshot, &filter_ctx, game)
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
        .static_abilities_rc(target_id)
        .unwrap_or_else(|| std::rc::Rc::new(extract_static_abilities(&target.abilities)));

    for ability in target_abilities.iter() {
        if ability.has_protection()
            && let Some(protection_from) = ability.protection_from()
        {
            let matches = match protection_from {
                crate::ability::ProtectionFrom::ChosenPlayer => game
                    .chosen_player(target_id)
                    .is_some_and(|chosen| game.controller_of(source) == chosen),
                crate::ability::ProtectionFrom::ChosenColor => game
                    .chosen_color(target_id)
                    .is_some_and(|chosen| view.object_colors(source_id).contains(chosen)),
                crate::ability::ProtectionFrom::EachManaValueAmong(filter) => {
                    source_mana_value_matches_scope(game, target_id, source, filter)
                }
                _ => source_matches_protection_with_view(source, protection_from, game, view),
            };
            if matches {
                return true;
            }
        }
    }

    false
}

fn has_protection_from_source_snapshot_with_view(
    game: &GameState,
    target_id: ObjectId,
    source_snapshot: &ObjectSnapshot,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    let Some(target) = game.object(target_id) else {
        return false;
    };

    let target_abilities = view
        .static_abilities_rc(target_id)
        .unwrap_or_else(|| std::rc::Rc::new(extract_static_abilities(&target.abilities)));

    for ability in target_abilities.iter() {
        if ability.has_protection()
            && let Some(protection_from) = ability.protection_from()
        {
            let matches = match protection_from {
                crate::ability::ProtectionFrom::ChosenPlayer => game
                    .chosen_player(target_id)
                    .is_some_and(|chosen| source_snapshot.controller == chosen),
                crate::ability::ProtectionFrom::ChosenColor => game
                    .chosen_color(target_id)
                    .is_some_and(|chosen| source_snapshot.colors.contains(chosen)),
                crate::ability::ProtectionFrom::EachManaValueAmong(filter) => {
                    source_snapshot_mana_value_matches_scope(
                        game,
                        target_id,
                        source_snapshot,
                        filter,
                    )
                }
                _ => source_snapshot_matches_protection(source_snapshot, protection_from, game),
            };
            if matches {
                return true;
            }
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
    let source_colors = view.object_colors(source.id);

    match protection {
        // Protection from a color or set of colors
        ProtectionFrom::Color(color_set) => {
            // Check if source has any of the colors in the set
            !source_colors.intersection(*color_set).is_empty()
        }
        // Protection from all colors
        ProtectionFrom::AllColors => !source_colors.is_empty(),
        // Protection from creatures
        ProtectionFrom::Creatures => view.object_has_card_type(source.id, CardType::Creature),
        // Protection from the chosen player is target-specific and handled by the caller.
        ProtectionFrom::ChosenPlayer => false,
        ProtectionFrom::ChosenColor => false,
        // Protection from a card type
        ProtectionFrom::CardType(card_type) => view.object_has_card_type(source.id, *card_type),
        // Protection from permanents matching a filter
        ProtectionFrom::Permanents(filter) => {
            // Use active player for context since we don't have a specific controller
            let filter_ctx = game.filter_context_for(game.turn.active_player, None);
            filter.matches(source, &filter_ctx, game)
        }
        ProtectionFrom::EachManaValueAmong(_) => false,
        // Protection from everything
        ProtectionFrom::Everything => true,
        // Protection from colorless (sources with no colors)
        ProtectionFrom::Colorless => source_colors.is_empty(),
    }
}

fn source_snapshot_matches_protection(
    source: &ObjectSnapshot,
    protection: &crate::ability::ProtectionFrom,
    game: &GameState,
) -> bool {
    use crate::ability::ProtectionFrom;

    match protection {
        ProtectionFrom::Color(color_set) => !source.colors.intersection(*color_set).is_empty(),
        ProtectionFrom::AllColors => !source.colors.is_empty(),
        ProtectionFrom::Creatures => source.card_types.contains(&CardType::Creature),
        ProtectionFrom::ChosenPlayer => false,
        ProtectionFrom::ChosenColor => false,
        ProtectionFrom::CardType(card_type) => source.card_types.contains(card_type),
        ProtectionFrom::Permanents(filter) => {
            let filter_ctx = game.filter_context_for(game.turn.active_player, None);
            filter.matches_snapshot(source, &filter_ctx, game)
        }
        ProtectionFrom::EachManaValueAmong(_) => false,
        ProtectionFrom::Everything => true,
        ProtectionFrom::Colorless => source.colors.is_empty(),
    }
}

fn source_mana_value_matches_scope(
    game: &GameState,
    protected_id: ObjectId,
    source: &Object,
    scope: &ObjectFilter,
) -> bool {
    object_mana_value(source)
        .is_some_and(|mana_value| mana_value_matches_scope(game, protected_id, mana_value, scope))
}

fn source_snapshot_mana_value_matches_scope(
    game: &GameState,
    protected_id: ObjectId,
    source: &ObjectSnapshot,
    scope: &ObjectFilter,
) -> bool {
    snapshot_mana_value(source)
        .is_some_and(|mana_value| mana_value_matches_scope(game, protected_id, mana_value, scope))
}

fn mana_value_matches_scope(
    game: &GameState,
    protected_id: ObjectId,
    mana_value: i32,
    scope: &ObjectFilter,
) -> bool {
    let Some(protected) = game.object(protected_id) else {
        return false;
    };
    let filter_ctx = game.filter_context_for(game.controller_of(protected), Some(protected_id));
    let zone = scope.zone.unwrap_or(Zone::Battlefield);
    game.objects_in_zone(zone).into_iter().any(|object_id| {
        let Some(object) = game.object(object_id) else {
            return false;
        };
        scope.matches(object, &filter_ctx, game) && object_mana_value(object) == Some(mana_value)
    })
}

fn object_mana_value(object: &Object) -> Option<i32> {
    Some(
        object
            .mana_cost
            .as_ref()
            .map_or(0, |cost| cost.mana_value() as i32),
    )
}

fn snapshot_mana_value(snapshot: &ObjectSnapshot) -> Option<i32> {
    Some(
        snapshot
            .mana_cost
            .as_ref()
            .map_or(0, |cost| cost.mana_value() as i32),
    )
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
    compute_legal_targets_with_tagged_objects_source_snapshot_with_view(
        game,
        spec,
        caster,
        source_id,
        None,
        tagged_objects,
        view,
    )
}

pub(crate) fn compute_legal_targets_with_tagged_objects_combat_context_with_view(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    source_snapshot: Option<&ObjectSnapshot>,
    tagged_objects: Option<
        &std::collections::HashMap<TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    >,
    combat_context: Option<(PlayerId, PlayerId)>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => {
            compute_legal_targets_with_tagged_objects_combat_context_with_view(
                game,
                spec,
                caster,
                source_id,
                source_snapshot,
                tagged_objects,
                combat_context,
                view,
            )
        }
        ChooseSpec::Object(filter) => compute_object_targets_with_view(
            game,
            filter,
            caster,
            source_id,
            source_snapshot,
            tagged_objects,
            combat_context,
            view,
        ),
        _ => compute_legal_targets_with_tagged_objects_source_snapshot_with_view(
            game,
            spec,
            caster,
            source_id,
            source_snapshot,
            tagged_objects,
            view,
        ),
    }
}

pub(crate) fn compute_legal_targets_with_tagged_objects_source_snapshot_with_view(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    source_snapshot: Option<&ObjectSnapshot>,
    tagged_objects: Option<
        &std::collections::HashMap<TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    >,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } => {
            compute_legal_targets_with_tagged_objects_source_snapshot_with_view(
                game,
                spec,
                caster,
                source_id,
                source_snapshot,
                tagged_objects,
                view,
            )
        }
        // Target wrapper - recursively compute targets from inner spec
        ChooseSpec::Target(inner) => {
            compute_legal_targets_with_tagged_objects_source_snapshot_with_view(
                game,
                inner,
                caster,
                source_id,
                source_snapshot,
                tagged_objects,
                view,
            )
        }
        // WithCount wrapper - recursively compute targets from inner spec
        ChooseSpec::WithCount(inner, _) | ChooseSpec::WithCountValue(inner, _, _) => {
            compute_legal_targets_with_tagged_objects_source_snapshot_with_view(
                game,
                inner,
                caster,
                source_id,
                source_snapshot,
                tagged_objects,
                view,
            )
        }
        ChooseSpec::AnyTarget => {
            compute_any_targets_with_view(game, caster, source_id, source_snapshot, view)
        }
        ChooseSpec::AnyOtherTarget => {
            compute_any_other_targets_with_view(game, caster, source_id, source_snapshot, view)
        }
        ChooseSpec::PlayerOrPlaneswalker(filter) => {
            compute_player_or_planeswalker_targets_with_view(
                game,
                filter,
                caster,
                source_id,
                source_snapshot,
                view,
            )
        }
        ChooseSpec::AttackedPlayerOrPlaneswalker => Vec::new(),
        ChooseSpec::Player(filter) => {
            compute_player_targets(game, filter, caster, source_id, source_snapshot)
        }
        ChooseSpec::Object(filter) => compute_object_targets_with_view(
            game,
            filter,
            caster,
            source_id,
            source_snapshot,
            tagged_objects,
            None,
            view,
        ),
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
    source_snapshot: Option<&ObjectSnapshot>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    let mut targets =
        compute_player_targets(game, player_filter, caster, source_id, source_snapshot);

    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        if !view.object_has_card_type(obj_id, CardType::Planeswalker) {
            continue;
        }

        if let Some(src_id) = source_id {
            match can_target_object_with_view_and_source_snapshot(
                game,
                obj_id,
                src_id,
                source_snapshot,
                caster,
                view,
            ) {
                TargetingResult::Legal { .. } => targets.push(Target::Object(obj_id)),
                TargetingResult::Invalid(_) => {}
            }
        } else {
            let is_untargetable = game.is_untargetable(obj_id);
            let is_controlled_by_caster = game.controller_of(obj) == caster;
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
    source_snapshot: Option<&ObjectSnapshot>,
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
                match can_target_object_with_view_and_source_snapshot(
                    game,
                    obj_id,
                    src_id,
                    source_snapshot,
                    caster,
                    view,
                ) {
                    TargetingResult::Legal { .. } => targets.push(Target::Object(obj_id)),
                    TargetingResult::Invalid(_) => {}
                }
            } else {
                // No source - check basic hexproof/shroud
                let is_untargetable = game.is_untargetable(obj_id);
                let is_controlled_by_caster = game.controller_of(obj) == caster;
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
    source_snapshot: Option<&ObjectSnapshot>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    let mut targets = compute_any_targets_with_view(game, caster, source_id, source_snapshot, view);
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
    source_id: Option<ObjectId>,
    source_snapshot: Option<&ObjectSnapshot>,
) -> Vec<Target> {
    // Unwrap Target wrapper — during legal target computation we want to know
    // which players *could be* targeted, not which are already targeted.
    let filter = match filter {
        PlayerFilter::Target(inner) => inner.as_ref(),
        other => other,
    };

    let filter_ctx = target_filter_context(game, controller, source_id);

    game.players
        .iter()
        .filter(|p| p.is_in_game())
        .filter(|p| {
            source_id.map_or_else(
                || game.can_target_player(p.id),
                |source| {
                    can_target_player_from_source_or_snapshot(game, p.id, source, source_snapshot)
                },
            )
        })
        .filter(|p| player_filter_matches_game(filter, p.id, game, &filter_ctx))
        .map(|p| Target::Player(p.id))
        .collect()
}

fn can_target_player_from_source_or_snapshot(
    game: &GameState,
    player: PlayerId,
    source_id: ObjectId,
    source_snapshot: Option<&ObjectSnapshot>,
) -> bool {
    if game.object(source_id).is_some() || source_snapshot.is_none() {
        return game.can_target_player_from_source(player, source_id);
    }
    if !game.can_target_player(player) {
        return false;
    }
    let source_snapshot = source_snapshot.expect("checked above");
    !game
        .effect_store
        .cant_effects
        .cant_target_players_from
        .iter()
        .any(|restriction| {
            if restriction.player != player {
                return false;
            }
            let filter_ctx =
                game.filter_context_for(restriction.controller, Some(source_snapshot.object_id));
            restriction
                .source_filter
                .matches_snapshot(source_snapshot, &filter_ctx, game)
        })
}

fn compute_object_targets_with_view(
    game: &GameState,
    filter: &ObjectFilter,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    source_snapshot: Option<&ObjectSnapshot>,
    tagged_objects: Option<
        &std::collections::HashMap<TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    >,
    combat_context: Option<(PlayerId, PlayerId)>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    let mut targets = Vec::new();

    // Build filter context
    let mut filter_ctx = target_filter_context(game, caster, source_id);
    if let Some((defending_player, attacking_player)) = combat_context {
        filter_ctx.defending_player = Some(defending_player);
        filter_ctx.attacking_player = Some(attacking_player);
    }
    if let Some(tagged) = tagged_objects {
        filter_ctx = filter_ctx.with_tagged_objects(tagged);
    }

    let candidate_ids = view.candidate_ids_for_filter_with_context(filter, &filter_ctx);
    for object_id in candidate_ids {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        if !filter.matches_with_view(object, &filter_ctx, game, view) {
            continue;
        }

        if let Some(src_id) = source_id {
            if can_target_object_with_view_and_source_snapshot(
                game,
                object_id,
                src_id,
                source_snapshot,
                caster,
                view,
            )
            .is_legal()
            {
                targets.push(Target::Object(object_id));
            }
            continue;
        }

        if object.zone != Zone::Battlefield && object.zone != Zone::Stack {
            targets.push(Target::Object(object_id));
            continue;
        }

        let is_untargetable = game.is_untargetable(object_id);
        let is_controlled_by_caster = game.controller_of(object) == caster;
        if !is_untargetable || is_controlled_by_caster {
            targets.push(Target::Object(object_id));
        }
    }

    targets
}

fn target_filter_context(
    game: &GameState,
    controller: PlayerId,
    source_id: Option<ObjectId>,
) -> crate::target::FilterContext {
    let mut filter_ctx = game.filter_context_for(controller, source_id);
    if let Some(source_id) = source_id
        && let Some((defending_player, attacking_player)) =
            combat_players_for_attacking_source(game, source_id)
    {
        filter_ctx.defending_player = Some(defending_player);
        filter_ctx.attacking_player = Some(attacking_player);
    }
    filter_ctx
}

fn combat_players_for_attacking_source(
    game: &GameState,
    source_id: ObjectId,
) -> Option<(PlayerId, PlayerId)> {
    let combat = game.combat.as_ref()?;
    let attack_target = crate::combat_state::get_attack_target(combat, source_id)?;
    let defending_player = match attack_target {
        crate::combat_state::AttackTarget::Player(player_id) => *player_id,
        crate::combat_state::AttackTarget::Planeswalker(planeswalker_id) => {
            let planeswalker = game.object(*planeswalker_id)?;
            game.controller_of(planeswalker)
        }
    };
    let source = game.object(source_id)?;
    Some((defending_player, game.controller_of(source)))
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

    fn add_hand_card(game: &mut GameState, id: u32, owner: PlayerId) {
        let card = CardBuilder::new(CardId::from_raw(id), &format!("Hand Card {id}"))
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand);
    }

    fn create_creature(id: u32, name: &str, controller: PlayerId) -> Object {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        let obj = Object::from_card(
            ObjectId::from_raw(id as u64),
            &card,
            controller,
            Zone::Battlefield,
        );
        obj
    }

    fn create_artifact(id: u32, name: &str, controller: PlayerId, mana_value: u8) -> Object {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
                mana_value,
            )]]))
            .card_types(vec![CardType::Artifact])
            .build();

        Object::from_card(
            ObjectId::from_raw(id as u64),
            &card,
            controller,
            Zone::Battlefield,
        )
    }

    fn create_battle(id: u32, name: &str, controller: PlayerId) -> Object {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Battle])
            .build();

        let obj = Object::from_card(
            ObjectId::from_raw(id as u64),
            &card,
            controller,
            Zone::Battlefield,
        );
        obj
    }

    fn create_planeswalker(id: u32, name: &str, controller: PlayerId) -> Object {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Planeswalker])
            .build();

        Object::from_card(
            ObjectId::from_raw(id as u64),
            &card,
            controller,
            Zone::Battlefield,
        )
    }

    fn create_land(id: u32, name: &str, controller: PlayerId) -> Object {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Land])
            .build();

        let obj = Object::from_card(
            ObjectId::from_raw(id as u64),
            &card,
            controller,
            Zone::Battlefield,
        );
        obj
    }

    fn add_static_ability(obj: &mut Object, ability: StaticAbility) {
        obj.abilities.push(Ability {
            kind: AbilityKind::Static(ability),
            functional_zones: vec![Zone::Battlefield],
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
    fn legal_targets_filter_attackers_attacking_you_or_planeswalker_you_control() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(1, "Trap Door Source", alice);
        let attacking_alice = create_creature(2, "Attacking Alice", bob);
        let attacking_alice_walker = create_creature(3, "Attacking Walker", bob);
        let attacking_bob = create_creature(4, "Attacking Bob", alice);
        let not_attacking = create_creature(5, "Not Attacking", bob);
        let alice_walker = create_planeswalker(6, "Alice Walker", alice);
        let source_id = source.id;
        let attacking_alice_id = attacking_alice.id;
        let attacking_alice_walker_id = attacking_alice_walker.id;
        let attacking_bob_id = attacking_bob.id;
        let not_attacking_id = not_attacking.id;
        let alice_walker_id = alice_walker.id;

        game.add_object(source);
        game.add_object(attacking_alice);
        game.add_object(attacking_alice_walker);
        game.add_object(attacking_bob);
        game.add_object(not_attacking);
        game.add_object(alice_walker);
        game.combat = Some(crate::combat_state::CombatState {
            attackers: vec![
                crate::combat_state::AttackerInfo {
                    creature: attacking_alice_id,
                    target: crate::combat_state::AttackTarget::Player(alice),
                },
                crate::combat_state::AttackerInfo {
                    creature: attacking_alice_walker_id,
                    target: crate::combat_state::AttackTarget::Planeswalker(alice_walker_id),
                },
                crate::combat_state::AttackerInfo {
                    creature: attacking_bob_id,
                    target: crate::combat_state::AttackTarget::Player(bob),
                },
            ],
            blockers: Default::default(),
            damage_assignment_order: Default::default(),
            attacking_bands: Default::default(),
            had_to_attack_this_combat: Default::default(),
        });

        let filter = ObjectFilter::creature()
            .attacking_player_or_planeswalker_controlled_by(PlayerFilter::You);
        let legal_targets =
            compute_legal_targets(&game, &ChooseSpec::Object(filter), alice, Some(source_id));

        assert!(legal_targets.contains(&Target::Object(attacking_alice_id)));
        assert!(legal_targets.contains(&Target::Object(attacking_alice_walker_id)));
        assert!(!legal_targets.contains(&Target::Object(attacking_bob_id)));
        assert!(!legal_targets.contains(&Target::Object(not_attacking_id)));
    }

    #[test]
    fn legal_targets_filter_creatures_defending_player_controls_for_attacking_source() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(1, "Attacking Source", alice);
        let defending_creature = create_creature(2, "Defending Creature", bob);
        let attacking_creature = create_creature(3, "Attacking Creature", alice);
        let defending_creature_id = defending_creature.id;
        let attacking_creature_id = attacking_creature.id;
        let source_id = source.id;

        game.add_object(source);
        game.add_object(defending_creature);
        game.add_object(attacking_creature);
        game.combat = Some(crate::combat_state::CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: source_id,
                target: crate::combat_state::AttackTarget::Player(bob),
            }],
            blockers: Default::default(),
            damage_assignment_order: Default::default(),
            attacking_bands: Default::default(),
            had_to_attack_this_combat: Default::default(),
        });

        let filter = ObjectFilter::creature().controlled_by(PlayerFilter::Defending);
        let legal_targets =
            compute_legal_targets(&game, &ChooseSpec::Object(filter), alice, Some(source_id));

        assert!(legal_targets.contains(&Target::Object(defending_creature_id)));
        assert!(!legal_targets.contains(&Target::Object(attacking_creature_id)));
    }

    #[test]
    fn legal_player_targets_include_planeswalker_controller_as_defending_player() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(1, "Attacking Source", alice);
        let bob_walker = create_planeswalker(2, "Bob Walker", bob);
        let source_id = source.id;
        let bob_walker_id = bob_walker.id;

        game.add_object(source);
        game.add_object(bob_walker);
        game.combat = Some(crate::combat_state::CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: source_id,
                target: crate::combat_state::AttackTarget::Planeswalker(bob_walker_id),
            }],
            blockers: Default::default(),
            damage_assignment_order: Default::default(),
            attacking_bands: Default::default(),
            had_to_attack_this_combat: Default::default(),
        });

        let legal_targets = compute_legal_targets(
            &game,
            &ChooseSpec::Player(PlayerFilter::Defending),
            alice,
            Some(source_id),
        );

        assert!(legal_targets.contains(&Target::Player(bob)));
        assert!(!legal_targets.contains(&Target::Player(alice)));
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
        let black_source = Object::from_card(ObjectId::from_raw(2), &card, p0, Zone::Battlefield);
        let own_black_source =
            Object::from_card(ObjectId::from_raw(3), &card, p1, Zone::Battlefield);

        let target_id = target.id;
        let black_source_id = black_source.id;
        let own_black_source_id = own_black_source.id;

        game.add_object(target);
        game.add_object(black_source);
        game.add_object(own_black_source);

        // Black source can't target creature with hexproof from black
        let result = can_target_object(&game, target_id, black_source_id, p0);
        assert!(matches!(
            result,
            TargetingResult::Invalid(TargetingInvalidReason::HasHexproofFrom)
        ));

        // Hexproof from black still allows the protected creature's controller
        // to target it with their own black source.
        let result = can_target_object(&game, target_id, own_black_source_id, p1);
        assert!(result.is_legal());
    }

    #[test]
    fn test_player_hexproof_from_blocks_matching_sources() {
        let mut game = create_test_game();
        let p0 = PlayerId::from_index(0);
        let p1 = PlayerId::from_index(1);

        let card = CardBuilder::new(CardId::from_raw(1), "Blue Source")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Instant])
            .build();
        let source = Object::from_card(ObjectId::from_raw(1), &card, p0, Zone::Battlefield);
        let own_source = Object::from_card(ObjectId::from_raw(2), &card, p1, Zone::Battlefield);
        let source_id = source.id;
        let own_source_id = own_source.id;
        game.add_object(source);
        game.add_object(own_source);

        game.effect_store
            .cant_effects
            .cant_target_players_from
            .push(crate::game_state::PlayerCantBeTargetedFrom {
                player: p1,
                source_filter: ObjectFilter {
                    colors: Some(ColorSet::from(Color::Blue)),
                    ..Default::default()
                }
                .controlled_by(PlayerFilter::Opponent),
                controller: p1,
            });

        let legal_targets = compute_legal_targets(
            &game,
            &ChooseSpec::Player(PlayerFilter::Any),
            p0,
            Some(source_id),
        );

        assert!(
            !legal_targets.contains(&Target::Player(p1)),
            "player with hexproof-from-blue should not be targetable by blue sources"
        );
        assert!(
            legal_targets.contains(&Target::Player(p0)),
            "other legal players should remain targetable"
        );

        let legal_targets = compute_legal_targets(
            &game,
            &ChooseSpec::Player(PlayerFilter::Any),
            p1,
            Some(own_source_id),
        );

        assert!(
            legal_targets.contains(&Target::Player(p1)),
            "hexproof-from player restrictions should not block that player's own matching source"
        );
    }

    #[test]
    fn test_hand_advantage_player_filter_targets_only_qualified_opponent() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        add_hand_card(&mut game, 101, alice);
        add_hand_card(&mut game, 102, bob);
        add_hand_card(&mut game, 103, bob);

        let filter = PlayerFilter::CardsInHandAtLeastMoreThanYou {
            base: Box::new(PlayerFilter::Opponent),
            count: 2,
        };
        let legal_targets =
            compute_legal_targets(&game, &ChooseSpec::Player(filter.clone()), alice, None);

        assert!(
            !legal_targets.contains(&Target::Player(bob)),
            "Bob should not be legal while only one card ahead"
        );

        add_hand_card(&mut game, 104, bob);
        let legal_targets = compute_legal_targets(&game, &ChooseSpec::Player(filter), alice, None);

        assert!(
            legal_targets.contains(&Target::Player(bob)),
            "Bob should be legal once he has at least two more cards than Alice"
        );
        assert!(
            !legal_targets.contains(&Target::Player(alice)),
            "the base opponent filter should still exclude Alice"
        );
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
        let red_source = Object::from_card(ObjectId::from_raw(2), &card, p0, Zone::Battlefield);

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
    fn rebbec_architect_of_ascension_mana_value_protection_targets_only_matching_values_you_control()
     {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let mut protected = create_artifact(1, "Rebbec-protected artifact", bob, 2);
        add_static_ability(
            &mut protected,
            StaticAbility::protection(ProtectionFrom::EachManaValueAmong(
                ObjectFilter::artifact().controlled_by(PlayerFilter::You),
            )),
        );
        let protected_id = protected.id;

        let matching_source = create_artifact(2, "Mana Value Two Source", alice, 2);
        let matching_source_id = matching_source.id;
        let nonmatching_source = create_artifact(3, "Mana Value Three Source", alice, 3);
        let nonmatching_source_id = nonmatching_source.id;
        let opponent_artifact_same_as_nonmatching =
            create_artifact(4, "Opponent Artifact With Mana Value Three", alice, 3);

        game.add_object(protected);
        game.add_object(matching_source);
        game.add_object(nonmatching_source);
        game.add_object(opponent_artifact_same_as_nonmatching);

        let result = can_target_object(&game, protected_id, matching_source_id, alice);
        assert!(
            matches!(
                result,
                TargetingResult::Invalid(TargetingInvalidReason::HasProtection)
            ),
            "Rebbec, Architect of Ascension should make the artifact illegal to target from a source whose mana value is among artifacts its controller controls"
        );

        let result = can_target_object(&game, protected_id, nonmatching_source_id, alice);
        assert!(
            result.is_legal(),
            "Rebbec, Architect of Ascension should not count artifacts controlled by another player for the protected artifact's mana-value set"
        );
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
        let blue_source = Object::from_card(ObjectId::from_raw(2), &card, p0, Zone::Battlefield);

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
        let colorless_source =
            Object::from_card(ObjectId::from_raw(2), &card, p0, Zone::Battlefield);

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

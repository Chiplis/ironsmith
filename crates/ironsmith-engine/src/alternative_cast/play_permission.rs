//! Exact origin selection for a cast whose rules depend on the chosen grant.
//! This owner is distinct from the legacy source-only PlayFrom lookup.
use super::GrantSelection;
use crate::effects::ExecutionError;
use crate::grant::Grantable;
use crate::grant_registry::{Grant, GrantPermissionIdentity, PlayFromConstraints, grant_usage_limit_allows};
use crate::object::Object;
use crate::{GameState, ObjectId, PlayerId, Zone};

/// Resolve an announcement-local index only when its immutable identity still
/// agrees. A stale index may not adopt another grant from the same host.
pub(crate) fn resolve(
    game: &GameState,
    player: PlayerId,
    face: &Object,
    zone: Zone,
    selection: &GrantSelection,
) -> Result<Grant, ExecutionError> {
    if face.zone == Zone::Stack || face.zone != zone
        || !game.object(face.id).is_some_and(|physical|
            physical.zone == zone && physical.stable_id == face.stable_id && physical.owner == face.owner)
    {
        return Err(ExecutionError::IncompleteEvidence("exact play permission has no live origin card".into()));
    }
    let query = crate::grant_registry::proposed_card_face_query(game, face)?;
    let grants = query.effect_store.grant_registry.get_grants_for_card(&query, face.id, zone, player);
    let grant = grants.get(selection.index).filter(|grant|
        grant.permission_identity.as_ref() == Some(&selection.identity)
            && grant.source.source_id() == selection.source
            && matches!(grant.grantable, Grantable::PlayFrom)
            && grant_usage_limit_allows(&query, player, grant.permission_identity.as_ref(), grant.usage_limit)
    ).cloned().ok_or_else(|| ExecutionError::IncompleteEvidence("selected play permission is no longer available".into()))?;
    if let Some(error) = query.token_resource_failure() { return Err(error); }
    Ok(grant)
}

/// Native receipt frozen before the origin card moves or payment removes a
/// provider. Mana conversion must read this receipt only for its own caster.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayPermissionReceipt {
    pub identity: GrantPermissionIdentity,
    pub source: ObjectId,
    pub origin: ObjectId,
    pub zone: Zone,
    pub player: PlayerId,
    pub constraints: PlayFromConstraints,
    /// Chosen spell face and native alternative cost, fixed at announcement.
    pub origin_method: Option<Box<super::CastingMethod>>,
}

impl PlayPermissionReceipt {
    pub(crate) fn from_resolved_grant(
        grant: &Grant, player: PlayerId, origin: ObjectId, zone: Zone, origin_method: &super::CastingMethod,
    ) -> Result<Self, ExecutionError> {
        if grant.player != player || grant.zone != zone {
            return Err(ExecutionError::IncompleteEvidence("selected permission and casting origin disagree".into()));
        }
        Ok(Self { identity: grant.permission_identity.clone().ok_or_else(||
            ExecutionError::IncompleteEvidence("selected permission lost its immutable identity".into()))?,
            source: grant.source.source_id(), origin, zone, player,
            constraints: grant.play_from_constraints.clone(), origin_method: Some(Box::new(origin_method.clone())),
        })
    }

    pub(crate) fn capture(
        game: &GameState,
        player: PlayerId,
        face: &Object,
        zone: Zone,
        selection: &GrantSelection,
    ) -> Result<Self, ExecutionError> {
        let grant = resolve(game, player, face, zone, selection)?;
        Ok(Self {
            identity: selection.identity.clone(), source: selection.source,
            origin: face.id, zone, player, constraints: grant.play_from_constraints, origin_method: None,
        })
    }
}

/// Rebuild the selected face from the authoritative object and method. The
/// caller's provisional Object never authorizes arbitrary characteristics.
pub(crate) fn selected_face(game: &GameState, player: PlayerId, spell: &Object, method: &super::CastingMethod) -> Result<(Object, ObjectId, Zone), ExecutionError> {
    use super::CastingMethod;
    let origin = if let CastingMethod::ExactPermission { origin, .. } = method {
        super::blind_play::admit_pre_stack_method(game, spell.id, player, method)?;
        origin.as_ref()
    } else {
        super::blind_play::admit_pre_stack_origin(game, spell.id, player, method)?;
        method
    };
    let physical = game.object(spell.id).ok_or(ExecutionError::ObjectNotFound(spell.id))?;
    let (source, zone, alternative) = match origin {
        CastingMethod::PlayFrom { source, zone, use_alternative }
        | CastingMethod::SplitOtherHalfPlayFrom { source, zone, use_alternative } => (*source, *zone, *use_alternative),
        CastingMethod::FaceDownPlayFrom { source, zone } => (*source, *zone, None),
        _ => return Err(ExecutionError::IncompleteEvidence("exact permission requires one ordinary play-from origin".into())),
    };
    if physical.zone != zone { return Err(ExecutionError::IncompleteEvidence("selected permission origin changed".into())); }
    let mut face = match origin {
        CastingMethod::SplitOtherHalfPlayFrom { .. } => crate::decision::spell_view_for_split_other_half_cast(game, physical)
            .ok_or_else(|| ExecutionError::IncompleteEvidence("selected permission has no linked spell face".into()))?,
        CastingMethod::FaceDownPlayFrom { .. } => {
            if !crate::decision::spell_can_be_cast_face_down(game, physical) {
                return Err(ExecutionError::IncompleteEvidence("selected permission does not supply a face-down casting rule".into()));
            }
            crate::decision::spell_view_for_face_down_cast(game, physical)
        }
        _ => physical.clone(),
    };
    if let Some(index) = alternative {
        let method = face.alternative_casts.get(index).filter(|method| method.cast_from_zone() == Zone::Hand)
            .cloned().ok_or_else(|| ExecutionError::IncompleteEvidence("selected permission has no printed alternative from hand".into()))?;
        if method.is_bestow() { face.apply_bestow_cast_overlay(); }
        if let Some(power_toughness) = method.prototype_power_toughness()
            && let Some(cost) = method.mana_cost() { face.apply_prototype_cast_overlay(cost.clone(), power_toughness); }
    }
    Ok((face, source, zone))
}

pub(crate) fn selected_alternative(
    game: &GameState, player: PlayerId, spell: &Object, method: &super::CastingMethod,
) -> Result<Option<super::AlternativeCastingMethod>, ExecutionError> {
    let super::CastingMethod::ExactPermission { origin, .. } = method else { return Ok(None); };
    super::blind_play::admit_pre_stack_method(game, spell.id, player, method)?;
    let index = match origin.as_ref() {
        super::CastingMethod::PlayFrom { use_alternative, .. }
        | super::CastingMethod::SplitOtherHalfPlayFrom { use_alternative, .. } => *use_alternative,
        super::CastingMethod::FaceDownPlayFrom { .. } => None,
        _ => return Err(ExecutionError::IncompleteEvidence("exact permission has an invalid alternative origin".into())),
    };
    let Some(index) = index else { return Ok(None); };
    if game.object(spell.id).is_some_and(|object| object.zone == Zone::Stack) {
        return game.object(spell.id).and_then(|object| object.cast_alternative_method_owned())
            .map(Some).ok_or_else(|| ExecutionError::IncompleteEvidence("selected printed alternative lost its casting receipt".into()));
    }
    let (face, _, _) = selected_face(game, player, spell, method)?;
    Ok(face.alternative_casts.get(index).cloned())
}

pub(crate) fn resolve_method(
    game: &GameState, player: PlayerId, spell: &Object, method: &super::CastingMethod,
) -> Result<Option<Grant>, ExecutionError> {
    let super::CastingMethod::ExactPermission { permission, .. } = method else { return Ok(None); };
    let (face, source, zone) = selected_face(game, player, spell, method)?;
    if source != permission.source { return Err(ExecutionError::IncompleteEvidence("selected permission source does not match its origin".into())); }
    resolve(game, player, &face, zone, permission).map(Some)
}

pub(crate) fn receipt_for_method(
    game: &GameState, player: PlayerId, spell: &Object, method: &super::CastingMethod,
) -> Result<Option<PlayPermissionReceipt>, ExecutionError> {
    let super::CastingMethod::ExactPermission { origin, permission } = method else { return Ok(None); };
    if game.object(spell.id).is_some_and(|object| object.zone == Zone::Stack) {
        let receipt = game.object(spell.id).and_then(|object| object.cast_play_permission.as_deref()).filter(|receipt|
            receipt.player == player && receipt.identity == permission.identity && receipt.source == permission.source
                && receipt.origin_method.as_deref() == Some(origin.as_ref())
                && game.cast_origin_snapshot(spell.id).is_some_and(|snapshot|
                    snapshot.object_id == receipt.origin && snapshot.zone == receipt.zone))
            .ok_or_else(|| ExecutionError::IncompleteEvidence("exact play permission lost its casting receipt".into()))?;
        let (source, zone) = match origin.as_ref() {
            super::CastingMethod::PlayFrom { source, zone, .. } | super::CastingMethod::SplitOtherHalfPlayFrom { source, zone, .. }
            | super::CastingMethod::FaceDownPlayFrom { source, zone } => (*source, *zone),
            _ => return Err(ExecutionError::IncompleteEvidence("exact play permission has a nested or invalid origin".into())),
        };
        if receipt.source != source || receipt.zone != zone { return Err(ExecutionError::IncompleteEvidence("cast receipt and selected origin disagree".into())); }
        return Ok(Some(receipt.clone()));
    }
    let (face, source, zone) = selected_face(game, player, spell, method)?;
    if source != permission.source { return Err(ExecutionError::IncompleteEvidence("selected permission source does not match its origin".into())); }
    let mut receipt = PlayPermissionReceipt::capture(game, player, &face, zone, permission)?;
    receipt.origin_method = Some(origin.clone());
    Ok(Some(receipt))
}

pub(crate) fn receipt_or_latch(
    game: &GameState, player: PlayerId, spell: &Object, method: &super::CastingMethod,
) -> Option<PlayPermissionReceipt> {
    match receipt_for_method(game, player, spell, method) {
        Ok(receipt) => receipt,
        Err(error) => { game.record_token_resource_failure(&error); None }
    }
}

pub(crate) fn casting_spend_policy(
    game: &GameState, player: PlayerId, spell: &Object, method: &super::CastingMethod,
) -> crate::player::ManaSpendPolicy {
    if matches!(method, super::CastingMethod::ExactPermission { .. }) {
        let Some(receipt) = receipt_or_latch(game, player, spell, method) else {
            return game.mana_spend_policy_for_selection(player, Some(spell.id), true, None);
        };
        let mut policy = game.mana_spend_policy_for_selection(player, Some(spell.id), true, Some(&receipt.identity));
        policy.allow_mode(receipt.constraints.cast_mana_spend_mode);
        return policy;
    }
    let actual = game.object(spell.id).unwrap_or(spell);
    if let super::CastingMethod::AlternativePrice { origin_permission: Some(selection), .. } = method
        && let Some(constraints) = super::price_routes::origin_constraints_or_latch(game, player, actual, method)
        && !constraints.cast_mana_spend_mode.is_normal()
    {
        let mut policy = game.mana_spend_policy_for_selection(player, Some(spell.id), true, Some(&selection.identity));
        policy.allow_mode(constraints.cast_mana_spend_mode);
        return policy;
    }
    game.mana_spend_policy_for_cast(player, Some(spell.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::grant_registry::GrantSource;
    use crate::ids::CardId;
    use ironsmith_core::value_model::ManaSpendMode;
    fn fixture() -> (GameState, ObjectId, ObjectId, GrantSelection, GrantSelection) {
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let definition = CardBuilder::new(CardId::from_raw(7), "Permission candidate").build();
        let source = game.create_object_from_card(&definition, player, Zone::Battlefield);
        let card = game.create_object_from_card(&definition, player, Zone::Exile);
        for mode in [ManaSpendMode::AnyColor, ManaSpendMode::Normal] {
            game.effect_store.grant_registry.grant_play_from_to_card(card, Zone::Exile, player,
                PlayFromConstraints { cast_mana_spend_mode: mode, ..Default::default() },
                GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
        }
        let key = |index: usize| GrantSelection {
            identity: game.effect_store.grant_registry.grants[index].permission_identity.clone().unwrap(), source, index,
        };
        let first = key(0); let second = key(1); (game, source, card, first, second)
    }
    #[test]
    fn exact_same_host_readers_keep_independent_modes_and_reject_stale_positions() {
        let (mut game, _, card, first, second) = fixture(); let player = PlayerId::from_index(0);
        let face = game.object(card).unwrap().clone();
        assert_eq!(resolve(&game, player, &face, Zone::Exile, &first).unwrap().play_from_constraints.cast_mana_spend_mode, ManaSpendMode::AnyColor);
        assert_eq!(resolve(&game, player, &face, Zone::Exile, &second).unwrap().play_from_constraints.cast_mana_spend_mode, ManaSpendMode::Normal);
        game.effect_store.grant_registry.grants.remove(0);
        assert!(matches!(resolve(&game, player, &face, Zone::Exile, &first), Err(ExecutionError::IncompleteEvidence(_))));
        assert!(matches!(resolve(&game, player, &face, Zone::Exile, &second), Err(ExecutionError::IncompleteEvidence(_))));
        let relocated = GrantSelection { index: 0, ..second };
        assert_eq!(resolve(&game, player, &face, Zone::Exile, &relocated).unwrap().play_from_constraints.cast_mana_spend_mode, ManaSpendMode::Normal);
    }
    #[test]
    fn receipt_freezes_player_origin_and_rules_before_provider_or_registry_changes() {
        let (mut game, source, card, first, _) = fixture(); let player = PlayerId::from_index(0);
        let receipt = PlayPermissionReceipt::capture(&game, player, game.object(card).unwrap(), Zone::Exile, &first).unwrap();
        let saved = (game.clone(), receipt.clone());
        game.move_object_by_game_rule(source, Zone::Hand).unwrap(); game.effect_store.grant_registry.grants.clear();
        assert_eq!(receipt.player, player); assert_eq!(receipt.origin, card); assert_eq!(receipt.source, source);
        assert_eq!(receipt.constraints.cast_mana_spend_mode, ManaSpendMode::AnyColor);
        game = saved.0; assert_eq!(receipt, saved.1);
        assert_eq!(PlayPermissionReceipt::capture(&game, player, game.object(card).unwrap(), Zone::Exile, &first).unwrap(), receipt);
    }
    #[test]
    fn wrong_player_missing_identity_and_reentered_card_never_adopt_the_grant() {
        let (mut game, _, card, first, _) = fixture(); let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        let face = game.object(card).unwrap().clone();
        assert!(resolve(&game, b, &face, Zone::Exile, &first).is_err());
        let saved = game.clone(); game.effect_store.grant_registry.grants[0].permission_identity = None;
        assert!(matches!(resolve(&game, a, &face, Zone::Exile, &first), Err(ExecutionError::IncompleteEvidence(_))));
        game = saved; let hand = game.move_object_by_game_rule(card, Zone::Hand).unwrap();
        let returned = game.move_object_by_game_rule(hand, Zone::Exile).unwrap();
        assert!(resolve(&game, a, game.object(returned).unwrap(), Zone::Exile, &first).is_err());
    }
    #[cfg(feature = "serialization")]
    #[test]
    fn old_default_constraint_shape_is_unchanged_and_new_modes_round_trip() {
        let old = serde_json::to_value(PlayFromConstraints::default()).unwrap(); assert!(old.get("cast_mana_spend_mode").is_none());
        assert_eq!(serde_json::from_value::<PlayFromConstraints>(old.clone()).unwrap().cast_mana_spend_mode, ManaSpendMode::Normal);
        for mode in [ManaSpendMode::AnyColor, ManaSpendMode::AnyType] {
            let value = PlayFromConstraints { cast_mana_spend_mode: mode, ..Default::default() };
            let json = serde_json::to_value(&value).unwrap(); assert_ne!(json, old);
            assert_eq!(serde_json::from_value::<PlayFromConstraints>(json).unwrap(), value);
        }
    }
}

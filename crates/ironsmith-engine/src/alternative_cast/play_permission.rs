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
pub(crate) struct PlayPermissionReceipt {
    pub identity: GrantPermissionIdentity,
    pub source: ObjectId,
    pub origin: ObjectId,
    pub zone: Zone,
    pub player: PlayerId,
    pub constraints: PlayFromConstraints,
}

impl PlayPermissionReceipt {
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
            origin: face.id, zone, player, constraints: grant.play_from_constraints,
        })
    }
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

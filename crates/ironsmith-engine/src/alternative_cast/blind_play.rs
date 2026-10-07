//! Authority for opening an unseen exiled card immediately before announcing
//! its play (CR 406.3a). No prospective face, cost, type or method is queried.
use super::GrantSelection;
use crate::effects::ExecutionError;
use crate::grant_registry::{Grant, grant_usage_limit_allows};
use crate::{GameState, ObjectId, PlayerId, Zone};

pub fn requires_opening(game: &GameState, card: ObjectId, player: PlayerId) -> bool {
    game.object(card).is_some_and(|object| object.zone == Zone::Exile)
        && game.is_face_down(card) && !game.can_player_look_at_face_down_exiled_card(card, player)
}

/// A tracked card must retain its public zone history. Untracked local cards
/// have no cross-runtime witness and are bound by their exact ObjectId.
pub fn incarnation(game: &GameState, card: ObjectId) -> Result<Option<u64>, ExecutionError> {
    match game.hidden_card_info(card) {
        Some(info) => info.incarnation.map(Some).ok_or_else(|| ExecutionError::IncompleteEvidence(
            "blind play has no public incarnation history".into())),
        None => Ok(None),
    }
}

pub fn validate_incarnation(game: &GameState, card: ObjectId, expected: Option<u64>) -> Result<(), ExecutionError> {
    if incarnation(game, card)? != expected {
        return Err(ExecutionError::IncompleteEvidence("blind play belongs to another exile incarnation".into()));
    }
    Ok(())
}

fn grants(game: &GameState, card: ObjectId, player: PlayerId) -> Result<Vec<Grant>, ExecutionError> {
    if !game.object(card).is_some_and(|object| object.zone == Zone::Exile) { return Ok(Vec::new()); }
    let mut checked = game.continuous_query_snapshot().map_err(ExecutionError::ContinuousDiscovery)?;
    let scope = crate::effects::tokens::resources::TokenQueryScope::new(checked.token_creation_limits());
    checked.bind_token_query_meter(scope.meter());
    let grants = checked.effect_store.grant_registry.unqualified_exile_play_grants(&checked, card, player);
    if let Some(error) = checked.token_resource_failure() { return Err(error); }
    Ok(grants)
}

/// Indices refer to this face-independent list, never to a list filtered by
/// the face that has not been opened. Identity, source and index must all agree.
pub fn selections(game: &GameState, card: ObjectId, player: PlayerId) -> Result<Vec<GrantSelection>, ExecutionError> {
    let grants = grants(game, card, player)?;
    if grants.is_empty() { return Ok(Vec::new()); }
    incarnation(game, card)?;
    grants.into_iter().enumerate().map(|(index, grant)| Ok(GrantSelection {
        source: grant.source.source_id(), index,
        identity: grant.permission_identity.ok_or_else(|| ExecutionError::IncompleteEvidence(
            "blind play permission omitted its immutable acquisition identity".into()))?,
    })).collect()
}

/// The public disclosure actor must match native priority-team admission,
/// including when the context is displayed under another teammate's seat.
pub fn priority_actor(
    game: &GameState, card: ObjectId, selection: &GrantSelection,
) -> Result<Option<PlayerId>, ExecutionError> {
    let checked = game.continuous_query_snapshot().map_err(ExecutionError::ContinuousDiscovery)?;
    for player in checked.priority_team_players() {
        if requires_opening(&checked, card, player) && selections(&checked, card, player)?.contains(selection) {
            return Ok(Some(player));
        }
    }
    Ok(None)
}

pub(crate) fn resolve(game: &GameState, card: ObjectId, player: PlayerId, selection: &GrantSelection) -> Result<Grant, ExecutionError> {
    grants(game, card, player)?.get(selection.index).filter(|grant|
        grant.permission_identity.as_ref() == Some(&selection.identity)
            && grant.source.source_id() == selection.source
            && grant_usage_limit_allows(game, player, grant.permission_identity.as_ref(), grant.usage_limit))
        .cloned().ok_or_else(|| ExecutionError::IncompleteEvidence("blind play permission is stale or unavailable".into()))
}

/// Native authority captured only by the admitted opaque intent and its
/// explicit declaration. A public kind claim alone cannot forge this receipt.
#[derive(Debug, Clone)]
pub(crate) struct BlindFaceDownDeclaration {
    pub player: PlayerId,
    pub incarnation: Option<u64>,
    pub permission: GrantSelection,
    pub kind: crate::game_state::FaceDownCastKind,
}

/// Check only public origin/declaration evidence before constructing any
/// prospective face. This must not call a face query or play-permission reader.
pub(crate) fn admit_pre_stack_origin(
    game: &GameState, card: ObjectId, player: PlayerId, origin: &super::CastingMethod,
) -> Result<(), ExecutionError> {
    if !requires_opening(game, card, player) { return Ok(()); }
    let denied = || ExecutionError::IncompleteEvidence("unseen exile casting requires its exact admitted declaration".into());
    let super::CastingMethod::FaceDownPlayFrom { source, zone: Zone::Exile } = origin else { return Err(denied()); };
    let declaration = game.blind_face_down_declaration(card).ok_or_else(denied)?;
    if declaration.player != player || declaration.permission.source != *source
        || game.hidden_face_down_cast_claim(card) != Some(declaration.kind) { return Err(denied()); }
    if let crate::game_state::FaceDownCastKind::Permission { source } = declaration.kind
        && game.active_face_down_cast_permission(source, player, Zone::Exile).is_none() { return Err(denied()); }
    validate_incarnation(game, card, declaration.incarnation)?;
    resolve(game, card, player, &declaration.permission)?;
    Ok(())
}

/// A full method additionally binds the selected reader. The stored opaque
/// index has its own namespace, so compare identity/source here and let the
/// subsequent ordinary face query validate the method's face-local index.
pub(crate) fn admit_pre_stack_method(
    game: &GameState, card: ObjectId, player: PlayerId, method: &super::CastingMethod,
) -> Result<(), ExecutionError> {
    if !requires_opening(game, card, player) { return Ok(()); }
    let super::CastingMethod::ExactPermission { origin, permission } = method else {
        return Err(ExecutionError::IncompleteEvidence("unseen exile casting requires an exact declaration method".into()));
    };
    admit_pre_stack_origin(game, card, player, origin)?;
    let declaration = game.blind_face_down_declaration(card).ok_or_else(|| ExecutionError::IncompleteEvidence("unseen exile declaration is unavailable".into()))?;
    if declaration.permission.identity != permission.identity || declaration.permission.source != permission.source {
        return Err(ExecutionError::IncompleteEvidence("unseen exile method changed its selected reader".into()));
    }
    Ok(())
}

/// Authoritative announcement also resolves the face-local index after the
/// nonrecursive public admission shared by receipt and cost readers.
pub(crate) fn declared_method_is_authorized(
    game: &GameState, card: ObjectId, player: PlayerId, method: &super::CastingMethod,
) -> Result<bool, ExecutionError> {
    admit_pre_stack_method(game, card, player, method)?;
    let physical = game.object(card).ok_or(ExecutionError::ObjectNotFound(card))?;
    super::play_permission::resolve_method(game, player, physical, method).map(|grant| grant.is_some())
}

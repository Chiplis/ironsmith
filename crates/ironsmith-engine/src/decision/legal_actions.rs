use super::*;
use crate::ability::ActivatedAbilityRuntimeExt as _;
use crate::grant_registry::grant_usage_limit_allows;

#[derive(Clone, Copy)]
enum ActionScope {
    All,
    Globals,
    Source(ObjectId),
}
thread_local! {
    static REQUESTED_ACTION_SOURCE: std::cell::Cell<ActionScope> = const { std::cell::Cell::new(ActionScope::All) };
}
fn requested_action_source(id: ObjectId) -> bool {
    REQUESTED_ACTION_SOURCE.with(|source| match source.get() {
        ActionScope::All => true,
        ActionScope::Globals => false,
        ActionScope::Source(requested) => requested == id,
    })
}
fn requested_global_actions() -> bool {
    REQUESTED_ACTION_SOURCE.with(|source| !matches!(source.get(), ActionScope::Source(_)))
}

/// Check one source without filtering the game: other objects still provide
/// mana, targets, restrictions and grants. Global actions have their own job.
pub fn compute_actions_for_source(
    game: &GameState,
    player: PlayerId,
    source: Option<ObjectId>,
) -> Result<Vec<LegalAction>, crate::effects::ExecutionError> {
    compute_scoped_actions(
        game,
        player,
        source.map_or(ActionScope::All, ActionScope::Source),
    )
}
pub fn compute_global_actions(
    game: &GameState,
    player: PlayerId,
) -> Result<Vec<LegalAction>, crate::effects::ExecutionError> {
    compute_scoped_actions(game, player, ActionScope::Globals)
}
/// Current timing, restrictions, targets and non-mana cost eligibility.
/// These candidates authorize starting an announcement, never completing it:
/// mana payment must still be validated by the payment transaction.
pub fn compute_actions_assuming_mana_for_presentation(
    game: &GameState,
    player: PlayerId,
    source: Option<ObjectId>,
) -> Result<Vec<LegalAction>, crate::effects::ExecutionError> {
    super::mana::with_assumed_mana_for_presentation(|| {
        compute_actions_for_source(game, player, source)
    })
}

fn compute_scoped_actions(
    game: &GameState,
    player: PlayerId,
    scope: ActionScope,
) -> Result<Vec<LegalAction>, crate::effects::ExecutionError> {
    struct Restore(ActionScope);
    impl Drop for Restore {
        fn drop(&mut self) {
            REQUESTED_ACTION_SOURCE.with(|slot| slot.set(self.0));
        }
    }
    let _restore = Restore(REQUESTED_ACTION_SOURCE.with(|slot| slot.replace(scope)));
    let mut actions = compute_legal_actions(game, player)?;
    actions.extend(compute_commander_actions(game, player)?);
    Ok(actions)
}

/// Stable presentation order. Lands in hand go first so spell affordability
/// cannot withhold a cheap land play. Include every zone used by enumeration.
pub fn priority_analysis_sources(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    let mut sources = Vec::new();
    if let Some(p) = game.player(player) {
        sources.extend(
            p.hand
                .iter()
                .copied()
                .filter(|id| game.object(*id).is_some_and(|o| o.is_land())),
        );
        sources.extend(p.hand.iter().copied());
    }
    sources.extend(game.battlefield.iter().copied());
    if let Some(p) = game.player(player) {
        sources.extend(p.graveyard.iter().copied());
    }
    // Land-play grants can refer to cards in another player's public zones.
    for p in game.players.iter() {
        sources.extend(p.graveyard.iter().copied());
    }
    sources.extend(game.exile.iter().copied());
    // Grants can refer to another player's top card, so include all tops.
    for p in game.players.iter() {
        sources.extend(p.library.last().copied());
    }
    sources.extend(game.command_zone.iter().copied());
    if let Some(p) = game.player(player) {
        sources.extend(p.sideboard.iter().copied());
    }
    for p in game.players.iter() {
        sources.extend(p.sideboard.iter().copied());
    }
    sources.extend(game.face_up_planar_objects().iter().copied());
    sources.extend(game.stack.iter().map(|entry| entry.object_id));
    let mut seen = std::collections::HashSet::new();
    sources.retain(|id| seen.insert(*id));
    sources
}

pub fn legal_action_source(action: &LegalAction) -> Option<ObjectId> {
    match action {
        LegalAction::CastSpell { spell_id, .. } => Some(*spell_id),
        LegalAction::OpenExiledCardForPlay { card_id, .. } | LegalAction::CastExiledCardFaceDown { card_id, .. } => Some(*card_id),
        LegalAction::ActivateAbility { source, .. }
        | LegalAction::ActivateManaAbility { source, .. } => Some(*source),
        LegalAction::PlayLand { land_id } | LegalAction::PlayLandBackFace { land_id } => {
            Some(*land_id)
        }
        LegalAction::TurnFaceUp { creature_id, .. } => Some(*creature_id),
        _ => None,
    }
}

/// Enumerate authoritative spell faces before querying marked readers. A
/// permission may match only Bestow/Prototype/the linked face; ordinary absence
/// on another face is not missing evidence and must not poison the menu.
fn append_exact_permission_actions_for_card(
    game: &GameState, actions: &mut Vec<LegalAction>, player: PlayerId,
    card: &crate::object::Object, zone: Zone, view: &DerivedGameView<'_>,
) -> Result<(), crate::effects::ExecutionError> {
    append_exact_permission_actions_for_card_selected(game, actions, player, card, zone, view, None)
}

fn append_exact_permission_actions_for_card_selected(
    game: &GameState, actions: &mut Vec<LegalAction>, player: PlayerId,
    card: &crate::object::Object, zone: Zone, view: &DerivedGameView<'_>,
    selected: Option<&crate::alternative_cast::GrantSelection>,
) -> Result<(), crate::effects::ExecutionError> {
    let mut origins = vec![CastingMethod::PlayFrom { source: card.id, zone, use_alternative: None }];
    origins.extend(card.alternative_casts.iter().enumerate().filter(|(_, method)| method.cast_from_zone() == Zone::Hand)
        .map(|(index, _)| CastingMethod::PlayFrom { source: card.id, zone, use_alternative: Some(index) }));
    // CR 406.3a exempts a cast being made face down from public opening.
    // This selected path belongs only to the mandatory face-up opening owner;
    // already inspected cards keep their separate existing face-down route.
    if selected.is_none() && spell_can_be_cast_face_down(game, card) {
        origins.push(CastingMethod::FaceDownPlayFrom { source: card.id, zone });
    }
    if let Some(face) = spell_view_for_split_other_half_cast(game, card) {
        origins.push(CastingMethod::SplitOtherHalfPlayFrom { source: card.id, zone, use_alternative: None });
        origins.extend(face.alternative_casts.iter().enumerate().filter(|(_, method)| method.cast_from_zone() == Zone::Hand)
            .map(|(index, _)| CastingMethod::SplitOtherHalfPlayFrom { source: card.id, zone, use_alternative: Some(index) }));
    }
    for origin in origins {
        let (face, _, _) = crate::alternative_cast::play_permission::selected_face(game, player, card, &origin)?;
        let query = crate::grant_registry::proposed_card_face_query(game, &face)?;
        let grants = query.effect_store.grant_registry.get_grants_for_card(&query, card.id, zone, player);
        if let Some(error) = query.token_resource_failure() { return Err(error); }
        for (index, grant) in grants.iter().enumerate() {
            if !matches!(grant.grantable, crate::grant::Grantable::PlayFrom)
                || selected.map_or(grant.play_from_constraints.cast_mana_spend_mode.is_normal(), |selected|
                    grant.permission_identity.as_ref() != Some(&selected.identity) || grant.source.source_id() != selected.source)
                || !grant_usage_limit_allows(&query, player, grant.permission_identity.as_ref(), grant.usage_limit) { continue; }
            let identity = grant.permission_identity.clone().ok_or_else(|| crate::effects::ExecutionError::IncompleteEvidence(
                "permission-local mana omitted its immutable acquisition identity".into()))?;
            let source = grant.source.source_id();
            let mut selected_origin = origin.clone();
            match &mut selected_origin {
                CastingMethod::PlayFrom { source: selected, .. }
                | CastingMethod::SplitOtherHalfPlayFrom { source: selected, .. }
                | CastingMethod::FaceDownPlayFrom { source: selected, .. } => *selected = source,
                _ => unreachable!("enumerated ordinary origin"),
            }
            let method = CastingMethod::ExactPermission { origin: Box::new(selected_origin),
                permission: crate::alternative_cast::GrantSelection { source, index, identity } };
            if can_cast_spell_with_view(game, player, card, &method, view) {
                actions.push(LegalAction::CastSpell { spell_id: card.id, from_zone: zone, casting_method: method });
            }
        }
    }
    Ok(())
}

/// Only an explicit public declaration unlocks this generic face-down view.
/// Each exact origin is reindexed in the proposed face's ordinary grant list.
pub(crate) fn declared_blind_face_down_action(
    game: &GameState, player: PlayerId, card_id: ObjectId,
    selected: &crate::alternative_cast::GrantSelection,
) -> Result<Option<LegalAction>, crate::effects::ExecutionError> {
    with_complete_legality_query(game, |game| {
        crate::alternative_cast::blind_play::resolve(game, card_id, player, selected)?;
        if game.hidden_face_down_cast_claim(card_id).is_none() { return Ok(None); }
        let card = game.object(card_id).ok_or(crate::effects::ExecutionError::ObjectNotFound(card_id))?;
        let origin = CastingMethod::FaceDownPlayFrom { source: selected.source, zone: Zone::Exile };
        let (face, _, _) = crate::alternative_cast::play_permission::selected_face(game, player, card, &origin)?;
        let query = crate::grant_registry::proposed_card_face_query(game, &face)?;
        let grants = query.effect_store.grant_registry.get_grants_for_card(&query, card_id, Zone::Exile, player);
        if let Some(error) = query.token_resource_failure() { return Err(error); }
        let Some((index, _)) = grants.iter().enumerate().find(|(_, grant)|
            matches!(grant.grantable, crate::grant::Grantable::PlayFrom)
                && grant.permission_identity.as_ref() == Some(&selected.identity)
                && grant.source.source_id() == selected.source
                && grant_usage_limit_allows(&query, player, grant.permission_identity.as_ref(), grant.usage_limit)) else { return Ok(None); };
        let method = CastingMethod::ExactPermission { origin: Box::new(origin), permission: crate::alternative_cast::GrantSelection {
            identity: selected.identity.clone(), source: selected.source, index,
        }};
        if !crate::alternative_cast::blind_play::declared_method_is_authorized(game, card_id, player, &method)? { return Ok(None); }
        let view = DerivedGameView::new(game);
        Ok(can_cast_spell_with_view(game, player, card, &method, &view).then_some(LegalAction::CastSpell {
            spell_id: card_id, from_zone: Zone::Exile, casting_method: method,
        }))
    })
}

fn append_blind_exile_intents(
    game: &GameState, actions: &mut Vec<LegalAction>, player: PlayerId, card_id: ObjectId, view: &DerivedGameView<'_>,
) -> Result<(), crate::effects::ExecutionError> {
    let permissions = crate::alternative_cast::blind_play::selections(game, card_id, player)?;
    if permissions.is_empty() { return Ok(()); }
    let incarnation = crate::alternative_cast::blind_play::incarnation(game, card_id)?;
    actions.extend(permissions.into_iter().flat_map(|permission| [
        LegalAction::OpenExiledCardForPlay { card_id, incarnation, permission: permission.clone() },
        LegalAction::CastExiledCardFaceDown { card_id, incarnation, permission },
    ]));
    append_declared_blind_face_down_actions(game, actions, player, card_id, view)
}

fn append_declared_blind_face_down_actions(
    game: &GameState, actions: &mut Vec<LegalAction>, player: PlayerId, card_id: ObjectId,
    _view: &DerivedGameView<'_>,
) -> Result<(), crate::effects::ExecutionError> {
    if game.blind_face_down_declaration(card_id).is_none() { return Ok(()); }
    for selection in crate::alternative_cast::blind_play::selections(game, card_id, player)? {
        if let Some(action) = declared_blind_face_down_action(game, player, card_id, &selection)? { actions.push(action); }
    }
    Ok(())
}

/// Only called after the public face-up transition. The opening selected one
/// unqualified authority; this menu may choose a face or price, not another grant.
pub(crate) fn opened_exile_play_actions(
    game: &GameState, player: PlayerId, card_id: ObjectId,
    permission: &crate::alternative_cast::GrantSelection,
) -> Result<Vec<LegalAction>, crate::effects::ExecutionError> {
    with_complete_legality_query(game, |game| {
        let card = game.object(card_id).ok_or(crate::effects::ExecutionError::ObjectNotFound(card_id))?;
        if card.zone != Zone::Exile || game.is_face_down(card_id) {
            return Err(crate::effects::ExecutionError::IncompleteEvidence("opened exile play lost its public origin".into()));
        }
        let authority = crate::alternative_cast::blind_play::selections(game, card_id, player)?;
        if authority.get(permission.index) != Some(permission) { return Ok(Vec::new()); }
        let view = DerivedGameView::new(game); let mut actions = Vec::new();
        append_exact_permission_actions_for_card_selected(game, &mut actions, player, card, Zone::Exile, &view, Some(permission))?;
        for method in crate::alternative_cast::price_routes::candidates(game, player, card)? {
            if matches!(&method, CastingMethod::AlternativePrice { origin_permission: Some(origin), .. }
                if origin.identity == permission.identity && origin.source == permission.source)
                && can_cast_spell_with_view(game, player, card, &method, &view)
            { actions.push(LegalAction::CastSpell { spell_id: card_id, from_zone: Zone::Exile, casting_method: method }); }
        }
        for (special, legal) in [
            (SpecialAction::PlayLand { card_id }, LegalAction::PlayLand { land_id: card_id }),
            (SpecialAction::PlayLandBackFace { card_id }, LegalAction::PlayLandBackFace { land_id: card_id }),
        ] {
            if special_action_is_legal(crate::special_actions::can_perform_check(&special, game, player))? { actions.push(legal); }
        }
        Ok(actions)
    })
}

fn append_granted_play_from_actions_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    source_zone: Zone,
    view: &DerivedGameView<'_>,
) -> Result<(), crate::effects::ExecutionError> {
    append_exact_permission_actions_for_card(game, actions, player, card, source_zone, view)?;
    let play_from_grants = view.granted_play_from_for_card(card_id, source_zone, player);
    for grant in play_from_grants {
        if !grant.constraints.cast_mana_spend_mode.is_normal() { continue; }
        if !grant_usage_limit_allows(
            game,
            player,
            grant.permission_identity.as_ref(),
            grant.usage_limit,
        ) {
            continue;
        }
        // PlayFrom (e.g., Yawgmoth's Will): can cast from zone as if from hand.
        let from_zone = grant.zone;
        let granted_alternatives =
            view.granted_alternative_casts_for_card(card_id, from_zone, player);
        let has_same_source_granted_alternative = granted_alternatives.iter().any(|granted_alt| {
            granted_alt.source_id == grant.source_id
                // Separate static abilities on the same permanent do not
                // make an ordinary permission require its other free cost.
                && !matches!((&grant.permission_identity, &granted_alt.permission_identity),
                    (Some(crate::grant_registry::GrantPermissionIdentity::Static {..}),
                     Some(crate::grant_registry::GrantPermissionIdentity::Static {..}))
                    if grant.permission_identity != granted_alt.permission_identity)
        });

        if !has_same_source_granted_alternative
            && !card.is_land()
            && let Some(mana_cost) = &card.mana_cost
            && can_cast_with_cost_with_view_for_casting_method(
                game,
                player,
                card,
                card_id,
                Some(mana_cost),
                None,
                &AdditionalCastRequirements::default(),
                &CastingMethod::PlayFrom {
                    source: grant.source_id,
                    zone: from_zone,
                    use_alternative: None,
                },
                view,
            )
        {
            actions.push(LegalAction::CastSpell {
                spell_id: card_id,
                from_zone,
                casting_method: CastingMethod::PlayFrom {
                    source: grant.source_id,
                    zone: from_zone,
                    use_alternative: None,
                },
            });
        }

        for (idx, alt_cast) in card.alternative_casts.iter().enumerate() {
            if alt_cast.cast_from_zone() == Zone::Hand
                && can_cast_spell_with_view(
                    game,
                    player,
                    card,
                    &CastingMethod::PlayFrom {
                        source: grant.source_id,
                        zone: from_zone,
                        use_alternative: Some(idx),
                    },
                    view,
                )
            {
                actions.push(LegalAction::CastSpell {
                    spell_id: card_id,
                    from_zone,
                    casting_method: CastingMethod::PlayFrom {
                        source: grant.source_id,
                        zone: from_zone,
                        use_alternative: Some(idx),
                    },
                });
            }
        }

        if source_zone != Zone::Graveyard {
            let base_alt_idx = card.alternative_casts.len();
            for (offset, granted_alt) in granted_alternatives.iter().enumerate() {
                if !grant_usage_limit_allows(
                    game,
                    player,
                    granted_alt.permission_identity.as_ref(),
                    granted_alt.usage_limit,
                ) {
                    continue;
                }
                if can_cast_spell_with_view(
                    game,
                    player,
                    card,
                    &CastingMethod::PlayFrom {
                        source: granted_alt.source_id,
                        zone: from_zone,
                        use_alternative: Some(base_alt_idx + offset),
                    },
                    view,
                ) {
                    actions.push(LegalAction::CastSpell {
                        spell_id: card_id,
                        from_zone,
                        casting_method: CastingMethod::PlayFrom {
                            source: granted_alt.source_id,
                            zone: from_zone,
                            use_alternative: Some(base_alt_idx + offset),
                        },
                    });
                }
            }
        }
    }

    // Morph/disguise is a different proposed spell face. Its public 2/2
    // characteristics, not the printed card's type/power, select permission.
    if spell_can_be_cast_face_down(game, card) {
        let face = spell_view_for_face_down_cast(game, card);
        let face_game = crate::grant_registry::proposed_card_face_query(game, &face)?;
        let face_view = DerivedGameView::new(&face_game);
        let face_card = face_game
            .object(card_id)
            .ok_or(crate::effects::ExecutionError::ObjectNotFound(card_id))?;
        let cost = face_down_cast_mana_cost();
        for grant in face_view.granted_play_from_for_card(card_id, source_zone, player) {
        if !grant.constraints.cast_mana_spend_mode.is_normal() { continue; }
            if !grant_usage_limit_allows(
                game,
                player,
                grant.permission_identity.as_ref(),
                grant.usage_limit,
            ) {
                continue;
            }
            let method = CastingMethod::FaceDownPlayFrom {
                source: grant.source_id,
                zone: grant.zone,
            };
            if can_cast_with_cost_with_view_for_casting_method(
                &face_game,
                player,
                face_card,
                card_id,
                Some(&cost),
                None,
                &AdditionalCastRequirements::default(),
                &method,
                &face_view,
            ) {
                actions.push(LegalAction::CastSpell {
                    spell_id: card_id,
                    from_zone: grant.zone,
                    casting_method: method,
                });
            }
        }
    }

    let Some(adventure_view) = spell_view_for_split_other_half_cast(game, card) else {
        return Ok(());
    };
    let face_game = crate::grant_registry::proposed_card_face_query(game, &adventure_view)?;
    let face_view = DerivedGameView::new(&face_game);
    let face_card = face_game
        .object(card_id)
        .ok_or(crate::effects::ExecutionError::ObjectNotFound(card_id))?;
    let adventure_play_from_grants =
        face_view.granted_play_from_for_card(card_id, source_zone, player);
    let face_alternatives =
        view.granted_alternative_casts_for_card_view(card_id, &adventure_view, source_zone, player);
    let face_alternative_base = card.alternative_casts.len()
        + view
            .granted_alternative_casts_for_card(card_id, source_zone, player)
            .len();
    for grant in adventure_play_from_grants {
        if !grant.constraints.cast_mana_spend_mode.is_normal() { continue; }
        if !grant_usage_limit_allows(
            game,
            player,
            grant.permission_identity.as_ref(),
            grant.usage_limit,
        ) {
            continue;
        }
        let has_same_source_alternative = face_alternatives.iter().any(|alternative| {
            alternative.source_id == grant.source_id
                && !matches!((&grant.permission_identity, &alternative.permission_identity),
                    (Some(crate::grant_registry::GrantPermissionIdentity::Static {..}),
                     Some(crate::grant_registry::GrantPermissionIdentity::Static {..}))
                    if grant.permission_identity != alternative.permission_identity)
        });
        let normal_face_permission = CastingMethod::SplitOtherHalfPlayFrom {
            source: grant.source_id,
            zone: grant.zone,
            use_alternative: None,
        };
        if !has_same_source_alternative
            && can_cast_spell_with_view(
                &face_game,
                player,
                face_card,
                &CastingMethod::PlayFrom {
                    source: grant.source_id,
                    zone: grant.zone,
                    use_alternative: None,
                },
                &face_view,
            )
        {
            actions.push(LegalAction::CastSpell {
                spell_id: card_id,
                from_zone: grant.zone,
                casting_method: normal_face_permission,
            });
        }
        for (offset, alternative) in face_alternatives.iter().enumerate() {
            // Free exile prices have one face-local owner below, including
            // when an independent ordinary PlayFrom permission also exists.
            if matches!(&alternative.method, crate::alternative_cast::AlternativeCastingMethod::FromZone {
                zone: Zone::Exile, total_cost, .. } if total_cost.costs().is_empty()) {
                continue;
            }
            if !grant_usage_limit_allows(
                game,
                player,
                alternative.permission_identity.as_ref(),
                alternative.usage_limit,
            ) {
                continue;
            }
            if alternative.source_id != grant.source_id {
                continue;
            }
            let casting_method = CastingMethod::SplitOtherHalfPlayFrom {
                source: grant.source_id,
                zone: grant.zone,
                use_alternative: Some(face_alternative_base + offset),
            };
            if can_cast_spell_with_view(game, player, card, &casting_method, view) {
                actions.push(LegalAction::CastSpell {
                    spell_id: card_id,
                    from_zone: grant.zone,
                    casting_method,
                });
            }
        }
    }
    Ok(())
}

/// "You may cast creature spells from your graveyard using their sneak
/// abilities." (Ninja Teen): a permanent `player` controls lets a matching
/// card they own be cast from `from_zone` with its `keyword` alternative cost,
/// whether that cost is printed or granted (CR 601.2, 702.190a).
pub(crate) fn filtered_alternative_cast_zone_permission(
    game: &GameState,
    player: PlayerId,
    card: &crate::object::Object,
    from_zone: Zone,
    keyword: Option<ironsmith_core::alternative_cast_model::AlternativeCastKeyword>,
    view: &DerivedGameView<'_>,
) -> bool {
    let Some(keyword) = keyword else {
        return false;
    };
    if card.zone != from_zone || card.owner != player {
        return false;
    }
    game.battlefield.iter().any(|&permanent| {
        let Some(permanent_object) = game.object(permanent) else {
            return false;
        };
        if game.controller_of(permanent_object) != player {
            return false;
        }
        let Some(static_abilities) = view.static_abilities_rc(permanent) else {
            return false;
        };
        static_abilities.iter().any(|static_ability| {
            let Some(ironsmith_core::StaticAbilityPayload::AlternativeCastFromZoneForFilter {
                filter,
                zone,
                method,
            }) = static_ability.compiled_model().map(|model| &model.payload)
            else {
                return false;
            };
            if *zone != from_zone || *method != keyword || !static_ability.is_active(game, permanent)
            {
                return false;
            }
            let ctx = game.filter_context_for(player, Some(permanent));
            let mut filter = filter.clone();
            filter.zone = None;
            filter.matches(card, &ctx, game)
        })
    })
}

fn append_native_alternative_cast_actions_for_card_from_zone(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    from_zone: Zone,
    view: &DerivedGameView<'_>,
) {
    for (idx, alt_cast) in card.alternative_casts.iter().enumerate() {
        let additional_zone_allowed = card.abilities.iter().any(|ability| {
            let crate::ability::AbilityKind::Static(ability) = &ability.kind else { return false; };
            matches!(ability.compiled_model().map(|model| &model.payload),
                Some(ironsmith_core::StaticAbilityPayload::NativeAlternativeCastFromZone { zone, method })
                if *zone == from_zone && alt_cast.keyword() == Some(*method))
        }) || (alt_cast.cast_from_zone() != from_zone
            && filtered_alternative_cast_zone_permission(
                game,
                player,
                card,
                from_zone,
                alt_cast.keyword(),
                view,
            ));
        if (alt_cast.cast_from_zone() == from_zone || additional_zone_allowed)
            && can_cast_with_alternative_with_view(game, player, card, alt_cast, view)
        {
            actions.push(LegalAction::CastSpell {
                spell_id: card_id,
                from_zone,
                casting_method: if alt_cast.cast_from_zone() == from_zone {
                    CastingMethod::Alternative(idx)
                } else {
                    CastingMethod::PlayFrom {
                        source: card_id,
                        zone: from_zone,
                        use_alternative: Some(idx),
                    }
                },
            });
        }
    }
}

fn append_graveyard_granted_alternative_cast_actions_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    view: &DerivedGameView<'_>,
) {
    append_zone_granted_alternative_cast_actions_for_card(
        game,
        actions,
        player,
        card_id,
        card,
        Zone::Graveyard,
        view,
    );
}

/// Granted alternative casts from a public zone: the graveyard, or the top
/// card of the library ("If you cast a spell this way, pay life equal to its
/// mana value rather than pay its mana cost." — Bolas's Citadel).
fn append_zone_granted_alternative_cast_actions_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    zone: Zone,
    view: &DerivedGameView<'_>,
) {
    let granted_casts = view.granted_alternative_casts_for_card(card_id, zone, player);

    let base_alt_idx = card.alternative_casts.len();
    for (offset, grant) in granted_casts.into_iter().enumerate() {
        let method = &grant.method;
        if !grant_usage_limit_allows(
            game,
            player,
            grant.permission_identity.as_ref(),
            grant.usage_limit,
        ) {
            continue;
        }
        // CR 702.35a: a granted madness cost is usable only while the madness
        // trigger resolves, never as an ordinary cast from exile.
        if method.is_madness() && !game.madness_cast_is_authorized(card_id, player) {
            continue;
        }
        let requirements = build_requirements_for_method(method);
        let mana_cost = get_mana_cost_for_method(method, card);
        let casting_method = match method {
            crate::alternative_cast::AlternativeCastingMethod::Escape { exile_count, .. } => {
                CastingMethod::GrantedEscape {
                    source: grant.source_id,
                    exile_count: *exile_count,
                }
            }
            crate::alternative_cast::AlternativeCastingMethod::Flashback { .. } => {
                CastingMethod::GrantedFlashback
            }
            crate::alternative_cast::AlternativeCastingMethod::FromZone {
                zone: method_zone,
                ..
            } if *method_zone == zone => CastingMethod::PlayFrom {
                source: grant.source_id,
                zone,
                use_alternative: Some(base_alt_idx + offset),
            },
            _ if method.cast_from_zone() == zone => CastingMethod::PlayFrom {
                source: grant.source_id,
                zone,
                use_alternative: Some(base_alt_idx + offset),
            },
            // A granted keyword cost that is not itself a cast from this zone
            // ("Creature cards in your graveyard have sneak {3}{B}.") needs a
            // separate permission to be used here (CR 601.2).
            _ if filtered_alternative_cast_zone_permission(
                game,
                player,
                card,
                zone,
                method.keyword(),
                view,
            ) =>
            {
                CastingMethod::PlayFrom {
                    source: grant.source_id,
                    zone,
                    use_alternative: Some(base_alt_idx + offset),
                }
            }
            _ => continue,
        };

        // The zero-component exile price intentionally has no mana component.
        // The complete cast calculation supplies an empty base, then adds
        // mandatory mana costs and cost increases instead of skipping them.
        let can_cast = if matches!(method, crate::alternative_cast::AlternativeCastingMethod::FromZone {
            zone: Zone::Exile, total_cost, .. } if total_cost.costs().is_empty()) {
            can_cast_spell_with_view(game, player, card, &casting_method, view)
        } else {
            can_cast_with_cost_with_view_for_casting_method(
            game,
            player,
            card,
            card_id,
            mana_cost,
            None,
            &requirements,
            &casting_method,
            view,
            )
        };
        if !can_cast {
            continue;
        }
        if !can_pay_non_mana_cost_sequence_for_cast(game, player, card_id, method.non_mana_costs())
        {
            continue;
        }

        actions.push(LegalAction::CastSpell {
            spell_id: card_id,
            from_zone: zone,
            casting_method,
        });
    }
}

/// A free exile permission authorizes each eligible spell face directly;
/// it need not (and must not) also grant an ordinary-price PlayFrom route.
fn append_exile_granted_other_face_alternative_cast_actions_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    view: &DerivedGameView<'_>,
) -> Result<(), crate::effects::ExecutionError> {
    let Some(face) = spell_view_for_split_other_half_cast(game, card) else {
        return Ok(());
    };
    // Preserve discovery failures for the selected face before publishing any
    // actions, including filters which perform object-ID characteristic reads.
    let face_game = crate::grant_registry::proposed_card_face_query(game, &face)?;
    let face_view = DerivedGameView::new(&face_game);
    let face_card = face_game.object(card_id)
        .ok_or(crate::effects::ExecutionError::ObjectNotFound(card_id))?;
    let grants = face_view.granted_alternative_casts_for_card(card_id, Zone::Exile, player);
    let base_alt_idx = card.alternative_casts.len()
        + view.granted_alternative_casts_for_card(card_id, Zone::Exile, player).len();
    for (offset, grant) in grants.into_iter().enumerate() {
        if !matches!(&grant.method, crate::alternative_cast::AlternativeCastingMethod::FromZone {
            zone: Zone::Exile, total_cost, .. } if total_cost.costs().is_empty())
            || !grant_usage_limit_allows(game, player, grant.permission_identity.as_ref(), grant.usage_limit)
        {
            continue;
        }
        let casting_method = CastingMethod::SplitOtherHalfPlayFrom {
            source: grant.source_id,
            zone: Zone::Exile,
            use_alternative: Some(base_alt_idx + offset),
        };
        // Validation runs on the already selected face, so use its local
        // alternative index. The published action retains the original
        // card's combined front/other-face index expected by announcement.
        let face_method = CastingMethod::PlayFrom {
            source: grant.source_id,
            zone: Zone::Exile,
            use_alternative: Some(face_card.alternative_casts.len() + offset),
        };
        if !can_cast_spell_with_view(&face_game, player, face_card, &face_method, &face_view)
            || !can_pay_non_mana_cost_sequence_for_cast(&face_game, player, face_card.id, grant.method.non_mana_costs())
        {
            continue;
        }
        let action = LegalAction::CastSpell {
            spell_id: card_id,
            from_zone: Zone::Exile,
            casting_method,
        };
        if !actions.contains(&action) {
            actions.push(action);
        }
    }
    Ok(())
}

fn append_graveyard_granted_adventure_alternative_cast_actions_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    view: &DerivedGameView<'_>,
) {
    let Some(adventure_view) = spell_view_for_split_other_half_cast(game, card) else {
        return;
    };
    let front_granted_count = view
        .granted_alternative_casts_for_card(card_id, Zone::Graveyard, player)
        .len();
    let granted_casts = view.granted_alternative_casts_for_card_view(
        card_id,
        &adventure_view,
        Zone::Graveyard,
        player,
    );

    let base_alt_idx = card.alternative_casts.len() + front_granted_count;
    for (offset, grant) in granted_casts.into_iter().enumerate() {
        let method = &grant.method;
        if method.cast_from_zone() != Zone::Graveyard
            || !grant_usage_limit_allows(
                game,
                player,
                grant.permission_identity.as_ref(),
                grant.usage_limit,
            )
        {
            continue;
        }

        let requirements = build_requirements_for_method(method);
        let mana_cost = get_mana_cost_for_method(method, &adventure_view);
        let casting_method = CastingMethod::SplitOtherHalfPlayFrom {
            source: grant.source_id,
            zone: Zone::Graveyard,
            use_alternative: Some(base_alt_idx + offset),
        };
        if !can_cast_with_cost_with_view_for_casting_method(
            game,
            player,
            &adventure_view,
            card_id,
            mana_cost,
            adventure_view
                .spell_effect
                .as_deref()
                .map(|program| &**program),
            &requirements,
            &casting_method,
            view,
        ) {
            continue;
        }
        if !can_pay_non_mana_cost_sequence_for_cast(game, player, card_id, method.non_mana_costs())
        {
            continue;
        }

        actions.push(LegalAction::CastSpell {
            spell_id: card_id,
            from_zone: Zone::Graveyard,
            casting_method,
        });
    }
}

fn append_hand_granted_alternative_cast_actions_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    view: &DerivedGameView<'_>,
) {
    if card.is_land() {
        return;
    }

    let granted_casts = view.granted_alternative_casts_for_card(card_id, Zone::Hand, player);
    let base_alt_idx = card.alternative_casts.len();

    for (offset, grant) in granted_casts.iter().enumerate() {
        if grant.method.cast_from_zone() != Zone::Hand
            || !grant_usage_limit_allows(
                game,
                player,
                grant.permission_identity.as_ref(),
                grant.usage_limit,
            )
            || !can_cast_with_alternative_from_hand_with_view(
                game,
                player,
                card,
                card_id,
                &grant.method,
                view,
            )
        {
            continue;
        }

        actions.push(LegalAction::CastSpell {
            spell_id: card_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::PlayFrom {
                source: grant.source_id,
                zone: Zone::Hand,
                use_alternative: Some(base_alt_idx + offset),
            },
        });
    }
}

fn append_cast_actions_from_zone_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    from_zone: Zone,
    view: &DerivedGameView<'_>,
    zone_has_active_grants: bool,
) -> Result<(), crate::effects::ExecutionError> {
    if crate::alternative_cast::blind_play::requires_opening(game, card_id, player) {
        append_blind_exile_intents(game, actions, player, card_id, view)?;
        return Ok(());
    }
    append_native_alternative_cast_actions_for_card_from_zone(
        game, actions, player, card_id, card, from_zone, view,
    );
    if zone_has_active_grants && from_zone == Zone::Graveyard {
        append_graveyard_granted_alternative_cast_actions_for_card(
            game, actions, player, card_id, card, view,
        );
        append_graveyard_granted_adventure_alternative_cast_actions_for_card(
            game, actions, player, card_id, card, view,
        );
    }
    if from_zone == Zone::Exile
        && (zone_has_active_grants
            || game
                .plotted_cast_permission(card_id, from_zone, player)
                .is_some())
    {
        append_zone_granted_alternative_cast_actions_for_card(
            game, actions, player, card_id, card, from_zone, view,
        );
    }
    if zone_has_active_grants && from_zone == Zone::Library && !card.is_land() {
        // Only the top card reaches this path (see `add_library_cast_actions`).
        append_zone_granted_alternative_cast_actions_for_card(
            game,
            actions,
            player,
            card_id,
            card,
            Zone::Library,
            view,
        );
    }
    if zone_has_active_grants {
        append_granted_play_from_actions_for_card(
            game, actions, player, card_id, card, from_zone, view,
        )?;
        if from_zone == Zone::Exile {
            append_exile_granted_other_face_alternative_cast_actions_for_card(
                game, actions, player, card_id, card, view,
            )?;
        }
    }
    Ok(())
}

/// CR 712.12: offer the back face of a land//land modal DFC as its own land
/// play, alongside the front-face `PlayLand`.
fn push_back_face_land_play_action(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
) -> Result<(), crate::effects::ExecutionError> {
    if crate::decision::linked_back_face_land_definition(game, card).is_none() {
        return Ok(());
    }
    let action = SpecialAction::PlayLandBackFace { card_id };
    if special_action_is_legal(crate::special_actions::can_perform_check(
        &action, game, player,
    ))? {
        actions.push(LegalAction::PlayLandBackFace { land_id: card_id });
    }
    Ok(())
}

fn append_granted_land_play_actions_from_public_zone(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    zone: Zone,
    _view: &DerivedGameView<'_>,
) -> Result<(), crate::effects::ExecutionError> {
    for card_id in game.zone_ids(zone) {
        if !requested_action_source(card_id) {
            continue;
        }
        let Some(card) = game.object(card_id) else {
            continue;
        };
        if crate::alternative_cast::blind_play::requires_opening(game, card_id, player) { continue; }
        if !card.is_land()
            && crate::decision::linked_other_face_land_definition(game, card).is_none()
        {
            continue;
        }
        // The authoritative owner evaluates the chosen land face. Filtering
        // only the front card here hides legal subtype-qualified MDFC backs.
        let action = SpecialAction::PlayLand { card_id };
        if special_action_is_legal(crate::special_actions::can_perform_check(
            &action, game, player,
        ))? {
            actions.push(LegalAction::PlayLand { land_id: card_id });
        }
        push_back_face_land_play_action(game, actions, player, card_id, card)?;
    }
    Ok(())
}

fn append_adventure_exiled_land_play_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
) -> Result<(), crate::effects::ExecutionError> {
    for &card_id in &game.exile {
        if !requested_action_source(card_id) {
            continue;
        }
        let Some(card) = game.object(card_id) else {
            continue;
        };
        if crate::alternative_cast::blind_play::requires_opening(game, card_id, player) { continue; }
        if game.adventure_exiled_player(card_id) != Some(player) || !card.is_land() {
            continue;
        }

        let action = SpecialAction::PlayLand { card_id };
        if special_action_is_legal(crate::special_actions::can_perform_check(
            &action, game, player,
        ))? {
            actions.push(LegalAction::PlayLand { land_id: card_id });
        }
        push_back_face_land_play_action(game, actions, player, card_id, card)?;
    }
    Ok(())
}

/// Compute legal actions for a player who has priority.
///
/// This validates each potential action by testing it against the actual game rules.
/// Only actions that would succeed are included in the result.
fn build_hand_summaries<'a>(game: &'a GameState, hand: &[ObjectId]) -> Vec<HandCardSummary<'a>> {
    hand.iter()
        .filter_map(|&card_id| {
            let card = game.object(card_id)?;
            let has_hand_special_actions = card.alternative_casts.iter().any(|method| {
                crate::alternative_cast::hand_special_action(method, card_id).is_some()
            });
            let has_hand_native_alternatives = card
                .alternative_casts
                .iter()
                .any(|method| method.cast_from_zone() == Zone::Hand);
            let has_split_other_half = spell_has_castable_linked_other_half(game, card);
            Some(HandCardSummary {
                card_id,
                card,
                is_land: card.is_land(),
                has_normal_mana_cost: card.mana_cost.is_some(),
                has_hand_special_actions,
                can_cast_face_down: spell_can_be_cast_face_down(game, card),
                has_split_other_half,
                has_fuse: card.has_fuse
                    && card.linked_face_layout == crate::card::LinkedFaceLayout::Split,
                has_hand_native_alternatives,
            })
        })
        .collect()
}

fn collect_controlled_battlefield(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id)
                .is_some_and(|object| game.controller_of(object) == player)
        })
        .collect()
}

fn add_land_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    hand_summaries: &[HandCardSummary<'_>],
    graveyard_has_active_grants: bool,
    exile_has_active_grants: bool,
    library_has_active_grants: bool,
    view: &DerivedGameView<'_>,
) -> Result<(), crate::effects::ExecutionError> {
    use crate::special_actions::{SpecialAction, can_perform_check};

    for summary in hand_summaries {
        if summary.is_land
            || crate::decision::linked_other_face_land_definition(game, summary.card).is_some()
        {
            let action = SpecialAction::PlayLand {
                card_id: summary.card_id,
            };
            if special_action_is_legal(can_perform_check(&action, game, player))? {
                actions.push(LegalAction::PlayLand {
                    land_id: summary.card_id,
                });
            }
            push_back_face_land_play_action(game, actions, player, summary.card_id, summary.card)?;
        }
    }
    if graveyard_has_active_grants {
        append_granted_land_play_actions_from_public_zone(
            game,
            actions,
            player,
            Zone::Graveyard,
            view,
        )?;
    }
    if exile_has_active_grants {
        append_granted_land_play_actions_from_public_zone(
            game,
            actions,
            player,
            Zone::Exile,
            view,
        )?;
    }
    append_adventure_exiled_land_play_actions(game, actions, player)?;
    if library_has_active_grants
        && let Some(card_id) = game
            .player(player)
            .and_then(|player_obj| player_obj.library.last().copied())
        && requested_action_source(card_id)
        && let Some(card) = game.object(card_id)
        && (card.is_land()
            || crate::decision::linked_other_face_land_definition(game, card).is_some())
    {
        let action = SpecialAction::PlayLand { card_id };
        if special_action_is_legal(can_perform_check(&action, game, player))? {
            actions.push(LegalAction::PlayLand { land_id: card_id });
        }
        push_back_face_land_play_action(game, actions, player, card_id, card)?;
    }
    Ok(())
}

fn add_hand_normal_cast_actions(
    actions: &mut Vec<LegalAction>,
    hand_summaries: &[HandCardSummary<'_>],
    cast_ctx: &CastLegalityContext<'_>,
) {
    for summary in hand_summaries {
        if summary.is_land || !summary.has_normal_mana_cost {
            continue;
        }
        let can_cast_normal =
            can_cast_spell_with_context(summary.card, &CastingMethod::Normal, cast_ctx);
        if can_cast_normal {
            actions.push(LegalAction::CastSpell {
                spell_id: summary.card_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            });
        }
    }
}

fn add_hand_special_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    hand_summaries: &[HandCardSummary<'_>],
) -> Result<(), crate::effects::ExecutionError> {
    for summary in hand_summaries {
        if !summary.has_any_hand_special_action() {
            continue;
        }
        let mut offered = Vec::new();
        for method in &summary.card.alternative_casts {
            let Some(action) =
                crate::alternative_cast::hand_special_action(method, summary.card_id)
            else {
                continue;
            };
            if offered.contains(&action) { continue; }
            match crate::special_actions::can_perform_check(&action, game, player) {
                Ok(()) => {
                    offered.push(action.clone());
                    actions.push(LegalAction::SpecialAction(action));
                }
                Err(crate::special_actions::ActionError::ExecutionFailure { error, .. }) => return Err(error),
                Err(_) => {}
            }
        }
    }
    Ok(())
}

fn add_graveyard_cast_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    graveyard: &[ObjectId],
    view: &DerivedGameView<'_>,
    graveyard_has_active_grants: bool,
) -> Result<(), crate::effects::ExecutionError> {
    for &card_id in graveyard {
        if let Some(card) = game.object(card_id) {
            append_cast_actions_from_zone_for_card(
                game,
                actions,
                player,
                card_id,
                card,
                Zone::Graveyard,
                view,
                graveyard_has_active_grants,
            )?;
        }
    }
    Ok(())
}

fn add_library_cast_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    view: &DerivedGameView<'_>,
    library_has_active_grants: bool,
) -> Result<(), crate::effects::ExecutionError> {
    if !library_has_active_grants {
        return Ok(());
    }
    let Some(card_id) = game
        .player(player)
        .and_then(|player_obj| player_obj.library.last().copied())
    else {
        return Ok(());
    };
    if !requested_action_source(card_id) {
        return Ok(());
    }
    let Some(card) = game.object(card_id) else {
        return Ok(());
    };
    append_cast_actions_from_zone_for_card(
        game,
        actions,
        player,
        card_id,
        card,
        Zone::Library,
        view,
        true,
    )?;
    Ok(())
}

/// Native exile designations authorize this exact card/copy for one player.
/// They allow its normal face, never arbitrary cards or its Adventure again.
pub(crate) fn native_exile_normal_cast_origin(
    game: &GameState,
    player: PlayerId,
    card_id: ObjectId,
) -> bool {
    game.object(card_id).is_some_and(|card| {
        card.zone == Zone::Exile
            && (game.adventure_exiled_player(card_id) == Some(player)
                || (game.is_prepared_spell_copy(card_id) && game.controller_of(card) == player))
    })
}

fn add_exile_cast_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    view: &DerivedGameView<'_>,
    exile_has_active_grants: bool,
) -> Result<(), crate::effects::ExecutionError> {
    for &card_id in &game.exile {
        if !requested_action_source(card_id) {
            continue;
        }
        let Some(card) = game.object(card_id) else {
            continue;
        };
        if crate::alternative_cast::blind_play::requires_opening(game, card_id, player) {
            append_blind_exile_intents(game, actions, player, card_id, view)?;
            continue;
        }
        append_cast_actions_from_zone_for_card(
            game,
            actions,
            player,
            card_id,
            card,
            Zone::Exile,
            view,
            exile_has_active_grants,
        )?;
        // A prepare spell copy waits in exile for exactly one caster: whoever
        // controls the prepared permanent right now.
        // CR 715.3d: the Adventure spell's controller may cast the card.
        if native_exile_normal_cast_origin(game, player, card_id)
            && can_cast_spell_with_view(game, player, card, &CastingMethod::Normal, view)
        {
            actions.push(LegalAction::CastSpell {
                spell_id: card_id,
                from_zone: Zone::Exile,
                casting_method: CastingMethod::Normal,
            });
        }
    }
    if exile_has_active_grants {
        append_granted_land_play_actions_from_public_zone(
            game,
            actions,
            player,
            Zone::Exile,
            view,
        )?;
    }
    Ok(())
}

fn add_hand_alternative_cast_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    hand_summaries: &[HandCardSummary<'_>],
    hand_has_active_grants: bool,
    view: &DerivedGameView<'_>,
    cast_ctx: &CastLegalityContext<'_>,
) {
    for summary in hand_summaries {
        if !summary.has_any_alternative_branch(hand_has_active_grants) {
            continue;
        }
        // Granted prices (including no mana cost) get the first payment-search
        // budget, before native alternatives and the printed mana cost.
        if hand_has_active_grants {
            append_hand_granted_alternative_cast_actions_for_card(
                game,
                actions,
                player,
                summary.card_id,
                summary.card,
                view,
            );
        }
        if summary.can_cast_face_down
            && can_cast_spell_with_context(summary.card, &CastingMethod::FaceDown, cast_ctx)
        {
            actions.push(LegalAction::CastSpell {
                spell_id: summary.card_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::FaceDown,
            });
        }
        if summary.has_split_other_half
            && can_cast_spell_with_context(summary.card, &CastingMethod::SplitOtherHalf, cast_ctx)
        {
            actions.push(LegalAction::CastSpell {
                spell_id: summary.card_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::SplitOtherHalf,
            });
        }
        if summary.has_split_other_half
            && summary.has_fuse
            && can_cast_spell_with_context(summary.card, &CastingMethod::Fuse, cast_ctx)
        {
            actions.push(LegalAction::CastSpell {
                spell_id: summary.card_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Fuse,
            });
        }
        if summary.has_hand_native_alternatives {
            for (idx, alt_cast) in summary.card.alternative_casts.iter().enumerate() {
                if alt_cast.cast_from_zone() == Zone::Hand
                    && can_cast_with_alternative_from_hand_with_context(
                        summary.card,
                        summary.card_id,
                        alt_cast,
                        cast_ctx,
                    )
                {
                    actions.push(LegalAction::CastSpell {
                        spell_id: summary.card_id,
                        from_zone: Zone::Hand,
                        casting_method: CastingMethod::Alternative(idx),
                    });
                }
            }
        }
    }
}

fn special_action_is_legal(
    result: Result<(), crate::special_actions::ActionError>,
) -> Result<bool, crate::effects::ExecutionError> {
    match result {
        Ok(()) => Ok(true),
        Err(crate::special_actions::ActionError::ExecutionFailure { error, .. }) => Err(error),
        Err(_) => Ok(false),
    }
}

fn add_battlefield_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    controlled_battlefield: &[ObjectId],
    view: &DerivedGameView<'_>,
    battlefield_ability_ctx: &BattlefieldAbilityContext,
) -> Result<(), crate::effects::ExecutionError> {
    use crate::special_actions::{SpecialAction, can_perform_check};

    // Ignore-effect permissions may be usable by a player who does not
    // control the source. Scan every battlefield source before the ordinary
    // controlled-permanent pass so those special actions remain visible.
    for source_id in game.zone_ids(Zone::Battlefield) {
        if !requested_action_source(source_id) {
            continue;
        }
        let Some(source) = game.object(source_id) else {
            continue;
        };
        for (ability_index, ability) in source.abilities.iter().enumerate() {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            let action = match static_ability.id() {
                crate::static_abilities::StaticAbilityId::AttachedControllerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn => {
                    SpecialAction::IgnoreAttachedRestriction {
                        source_id,
                        ability_index,
                    }
                }
                crate::static_abilities::StaticAbilityId::AnyPlayerMayPayManaToIgnoreSourceEffectUntilEndOfTurn => {
                    SpecialAction::IgnoreSourceEffect {
                        source_id,
                        ability_index,
                    }
                }
                _ => continue,
            };
            if special_action_is_legal(can_perform_check(&action, game, player))? {
                actions.push(LegalAction::SpecialAction(action));
            }
        }
    }

    for &perm_id in controlled_battlefield {
        if game.is_face_down(perm_id) {
            for method in crate::special_actions::available_turn_face_up_methods(game, perm_id)
                .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?
            {
                let action = SpecialAction::TurnFaceUp {
                    permanent_id: perm_id,
                    method,
                };
                if special_action_is_legal(can_perform_check(&action, game, player))? {
                    actions.push(LegalAction::TurnFaceUp {
                        creature_id: perm_id,
                        method,
                    });
                }
            }
        }
        // CR 709.5e: each locked door is its own unlock option (both doors
        // are locked when the Room entered with neither unlocked).
        for door in crate::special_actions::locked_room_doors(game, perm_id) {
            let unlock_action = SpecialAction::UnlockRoomDoor {
                room_id: perm_id,
                door,
            };
            if special_action_is_legal(can_perform_check(&unlock_action, game, player))? {
                actions.push(LegalAction::SpecialAction(unlock_action));
            }
        }
    }

    let simple_mana_analysis = view.simple_battlefield_mana_analysis(player);
    for &perm_id in simple_mana_analysis.relevant_source_ids() {
        if !requested_action_source(perm_id) {
            continue;
        }
        if let Some(perm) = game.object(perm_id) {
            let source_facts = ActivationSourceFacts::for_source(game, perm_id, view);
            let cached_abilities = view.abilities_rc(perm_id);
            let abilities = cached_abilities.as_deref().unwrap_or(&perm.abilities);
            let mana_ability_indices = simple_mana_analysis.mana_ability_indices_for(perm_id);
            let activated_ability_indices =
                simple_mana_analysis.activated_ability_indices_for(perm_id);
            if mana_ability_indices.is_empty() && activated_ability_indices.is_empty() {
                continue;
            };

            for &ability_index in mana_ability_indices {
                let Some(ability) = abilities.get(ability_index) else {
                    continue;
                };
                if simple_mana_analysis
                    .activatable_indices_for(perm_id)
                    .contains(&ability_index)
                {
                    actions.push(LegalAction::ActivateManaAbility {
                        source: perm_id,
                        ability_index,
                    });
                } else if can_activate_mana_ability_check_with_view(
                    game,
                    player,
                    perm_id,
                    ability_index,
                    ability,
                    view,
                    Some(battlefield_ability_ctx),
                )
                .is_ok()
                {
                    actions.push(LegalAction::ActivateManaAbility {
                        source: perm_id,
                        ability_index,
                    });
                }
            }

            if game.can_activate_non_mana_abilities(player) {
                for &ability_index in activated_ability_indices {
                    let Some(ability) = abilities.get(ability_index) else {
                        continue;
                    };
                    let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
                        continue;
                    };
                    if can_activate_ability_with_restrictions_with_view(
                        game,
                        perm_id,
                        ability_index,
                        activated,
                        view,
                        Some(battlefield_ability_ctx),
                        Some(&source_facts),
                    ) {
                        actions.push(LegalAction::ActivateAbility {
                            source: perm_id,
                            ability_index,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn collect_non_battlefield_source_ids(
    game: &GameState,
    player: PlayerId,
    hand: &[ObjectId],
    graveyard: &[ObjectId],
) -> Vec<ObjectId> {
    let mut non_battlefield_ids = Vec::with_capacity(
        hand.len() + graveyard.len() + game.exile.len() + game.command_zone.len(),
    );
    non_battlefield_ids.extend(hand.iter().copied());
    non_battlefield_ids.extend(graveyard.iter().copied());
    non_battlefield_ids.extend(
        game.exile
            .iter()
            .copied()
            .filter(|id| game.object(*id).is_some_and(|obj| obj.owner == player)
                && !crate::alternative_cast::blind_play::requires_opening(game, *id, player)),
    );
    non_battlefield_ids.extend(
        game.command_zone
            .iter()
            .copied()
            .filter(|id| game.object(*id).is_some_and(|obj| obj.owner == player)),
    );
    if game.planar_controller() == Some(player) {
        non_battlefield_ids.extend(
            game.face_up_planar_objects()
                .iter()
                .copied()
                .filter(|object| game.controller_of_id(*object) == Some(player)),
        );
    }
    non_battlefield_ids.extend(game.stack.iter().map(|entry| entry.object_id));
    non_battlefield_ids.sort_by_key(|id| id.0);
    non_battlefield_ids.dedup();
    non_battlefield_ids
}

fn activated_allows_any_player(activated: &crate::ability::ActivatedAbility) -> bool {
    activated.allows_any_player_to_activate()
}

fn add_non_battlefield_ability_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    source_ids: &[ObjectId],
    view: &DerivedGameView<'_>,
) {
    use crate::special_actions::{SpecialAction, can_perform_check};

    for &source_id in source_ids {
        let Some(obj) = game.object(source_id) else {
            continue;
        };
        if obj.zone == Zone::Battlefield {
            continue;
        }

        let Some(ability_summary) = view.ability_index_summary(source_id) else {
            continue;
        };
        if !ability_summary.has_any_relevant_abilities() {
            continue;
        }

        for &ability_index in ability_summary.mana_ability_indices() {
            let action = SpecialAction::ActivateManaAbility {
                permanent_id: source_id,
                ability_index,
            };
            if can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::ActivateManaAbility {
                    source: source_id,
                    ability_index,
                });
            }
        }

        if game.can_activate_non_mana_abilities(player) {
            let current_abilities = view.abilities_rc(source_id);
            let abilities = current_abilities.as_deref().unwrap_or(&obj.abilities);
            for &ability_index in ability_summary.activated_ability_indices() {
                let Some(ability) = abilities.get(ability_index) else {
                    continue;
                };
                if !ability.functions_in(&obj.zone) {
                    continue;
                }
                let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
                    continue;
                };
                if game.controller_of(obj) != player && !activated_allows_any_player(activated) {
                    continue;
                }
                if can_activate_ability_with_restrictions_with_view(
                    game,
                    source_id,
                    ability_index,
                    activated,
                    view,
                    None,
                    None,
                ) {
                    actions.push(LegalAction::ActivateAbility {
                        source: source_id,
                        ability_index,
                    });
                }
            }
        }
    }
}

pub fn compute_legal_actions(
    game: &GameState,
    player: PlayerId,
) -> Result<Vec<LegalAction>, crate::effects::ExecutionError> {
    with_complete_legality_query(game, |checked| {
        compute_legal_actions_checked(checked, player)
    })
}

pub(crate) fn with_complete_legality_query<T>(
    game: &GameState,
    compute: impl FnOnce(&GameState) -> Result<T, crate::effects::ExecutionError>,
) -> Result<T, crate::effects::ExecutionError> {
    let mut checked = game
        .continuous_query_snapshot()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    // Legality is a read-only query, including when entered during an actual
    // payment. Its simulated token work must not consume the real operation's
    // allowance. The owned latch retains failures discarded by boolean cost
    // predicates until this Result-bearing boundary can surface them.
    let scope =
        crate::effects::tokens::resources::TokenQueryScope::new(game.token_creation_limits());
    checked.bind_token_query_meter(scope.meter());
    let result = super::mana::with_checked_query(game, &checked, || compute(&checked));
    if let Some(error) =
        super::mana::analysis_failure().or_else(|| checked.token_resource_failure())
    {
        game.record_token_resource_failure(&error);
        return Err(error);
    }
    result
}

fn compute_legal_actions_checked(
    game: &GameState,
    player: PlayerId,
) -> Result<Vec<LegalAction>, crate::effects::ExecutionError> {
    let total_started_at = PerfTimer::start();
    let mut perf = ComputeLegalActionsPerfMetrics::default();
    let empty_zone: &[ObjectId] = &[];
    let (hand, graveyard) = game
        .player(player)
        .map_or((empty_zone, empty_zone), |player_obj| {
            (player_obj.hand.as_slice(), player_obj.graveyard.as_slice())
        });
    let filtered_hand: Vec<_> = hand
        .iter()
        .copied()
        .filter(|id| requested_action_source(*id))
        .collect();
    let filtered_graveyard: Vec<_> = graveyard
        .iter()
        .copied()
        .filter(|id| requested_action_source(*id))
        .collect();
    let hand = filtered_hand.as_slice();
    let graveyard = filtered_graveyard.as_slice();
    let mut actions = Vec::with_capacity(
        1 + hand.len() * 6
            + graveyard.len() * 2
            + game.exile.len() * 2
            + game.battlefield.len() * 4,
    );
    let view_started_at = PerfTimer::start();
    let view = DerivedGameView::new(game);
    perf.derived_view_ms = view_started_at.elapsed_ms();

    let prewarm_started_at = PerfTimer::start();
    view.prewarm_characteristics(&game.battlefield);
    perf.prewarm_ms = prewarm_started_at.elapsed_ms();

    let cast_context_started_at = PerfTimer::start();
    let cast_ctx = CastLegalityContext::new(game, player, &view);
    perf.cast_context_ms = cast_context_started_at.elapsed_ms();

    let battlefield_context_started_at = PerfTimer::start();
    let battlefield_ability_ctx = BattlefieldAbilityContext::new(&view);
    perf.battlefield_ability_context_ms = battlefield_context_started_at.elapsed_ms();

    let active_grant_zone_started_at = PerfTimer::start();
    let hand_has_active_grants = view.player_has_active_grants_for_zone(player, Zone::Hand);
    let graveyard_has_active_grants =
        view.player_has_active_grants_for_zone(player, Zone::Graveyard);
    let exile_has_active_grants = view.player_has_active_grants_for_zone(player, Zone::Exile);
    let library_has_active_grants = view.player_has_active_grants_for_zone(player, Zone::Library);
    perf.active_grant_zone_checks_ms = active_grant_zone_started_at.elapsed_ms();

    let hand_summary_started_at = PerfTimer::start();
    let hand_summaries = build_hand_summaries(game, hand);
    perf.hand_summary_ms = hand_summary_started_at.elapsed_ms();

    let controlled_battlefield_started_at = PerfTimer::start();
    let controlled_battlefield: Vec<_> = collect_controlled_battlefield(game, player)
        .into_iter()
        .filter(|id| requested_action_source(*id))
        .collect();
    perf.controlled_battlefield_ms = controlled_battlefield_started_at.elapsed_ms();

    actions.push(LegalAction::PassPriority);
    if requested_global_actions() {
        for delayed_trigger_index in 0..game.effect_store.delayed_triggers.len() {
            let action = crate::special_actions::SpecialAction::PayDelayedTrigger {
                delayed_trigger_index,
            };
            if crate::special_actions::can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::SpecialAction(action));
            }
        }
        for action_index in 0..game.effect_store.repeatable_mana_payment_actions.len() {
            let action =
                crate::special_actions::SpecialAction::PerformRepeatableManaPaymentAction {
                    action_index,
                };
            if crate::special_actions::can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::SpecialAction(action));
            }
        }
        let planar_die_action = crate::special_actions::SpecialAction::RollPlanarDie;
        if special_action_is_legal(crate::special_actions::can_perform_check(
            &planar_die_action,
            game,
            player,
        ))? {
            actions.push(LegalAction::SpecialAction(planar_die_action));
        }
        if let Some(companion_id) = game.player(player).and_then(|state| state.companion) {
            let companion_action = crate::special_actions::SpecialAction::Companion {
                card_id: companion_id,
            };
            if crate::special_actions::can_perform_check(&companion_action, game, player).is_ok() {
                actions.push(LegalAction::SpecialAction(companion_action));
            }
        }
        for conspiracy_id in game.conspiracy_cards() {
            let action =
                crate::special_actions::SpecialAction::TurnConspiracyFaceUp { conspiracy_id };
            if crate::special_actions::can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::SpecialAction(action));
            }
        }
    }

    let lands_started_at = PerfTimer::start();
    add_land_actions(
        game,
        &mut actions,
        player,
        &hand_summaries,
        graveyard_has_active_grants,
        exile_has_active_grants,
        library_has_active_grants,
        &view,
    )?;
    perf.lands_ms = lands_started_at.elapsed_ms();

    // Evaluate alternative prices first, while retaining the established menu
    // order (normal before alternative) when the results are displayed.
    let hand_alternatives_started_at = PerfTimer::start();
    let mut hand_alternatives = Vec::new();
    add_hand_alternative_cast_actions(
        game,
        &mut hand_alternatives,
        player,
        &hand_summaries,
        hand_has_active_grants,
        &view,
        &cast_ctx,
    );
    perf.hand_alternatives_ms = hand_alternatives_started_at.elapsed_ms();

    let hand_casts_started_at = PerfTimer::start();
    add_hand_normal_cast_actions(&mut actions, &hand_summaries, &cast_ctx);
    perf.hand_casts_ms = hand_casts_started_at.elapsed_ms();

    let hand_special_actions_started_at = PerfTimer::start();
    add_hand_special_actions(game, &mut actions, player, &hand_summaries)?;
    perf.hand_special_actions_ms = hand_special_actions_started_at.elapsed_ms();

    let graveyard_casts_started_at = PerfTimer::start();
    add_graveyard_cast_actions(
        game,
        &mut actions,
        player,
        graveyard,
        &view,
        graveyard_has_active_grants,
    )?;
    perf.graveyard_casts_ms = graveyard_casts_started_at.elapsed_ms();

    let exile_casts_started_at = PerfTimer::start();
    add_exile_cast_actions(game, &mut actions, player, &view, exile_has_active_grants)?;
    perf.exile_casts_ms = exile_casts_started_at.elapsed_ms();

    add_library_cast_actions(game, &mut actions, player, &view, library_has_active_grants)?;
    if view.player_has_active_grants_for_zone(player, Zone::OutsideGame) {
        for &card_id in &game.player(player).expect("active player").sideboard {
            if !requested_action_source(card_id) {
                continue;
            }
            let Some(card) = game.object(card_id) else {
                continue;
            };
            append_cast_actions_from_zone_for_card(
                game,
                &mut actions,
                player,
                card_id,
                card,
                Zone::OutsideGame,
                &view,
                true,
            )?;
        }
        append_granted_land_play_actions_from_public_zone(
            game,
            &mut actions,
            player,
            Zone::OutsideGame,
            &view,
        )?;
    }

    actions.extend(hand_alternatives);

    // Price grants supply no origins. Build the product of independently
    // authorized origins and eligible prices before ordinary affordability or
    // sorcery timing can filter out a newly payable/flash-enabled proposal.
    if game
        .effect_store
        .grant_registry
        .active_grants(game)
        .iter()
        .any(|grant| {
            grant.player == player
                && matches!(
                    grant.grantable,
                    crate::grant::Grantable::AlternativePrice { .. }
                )
        })
    {
        for id in priority_analysis_sources(game, player)
            .into_iter()
            .filter(|id| requested_action_source(*id))
        {
            let Some(card) = game.object(id) else {
                continue;
            };
            if matches!(card.zone, Zone::Battlefield | Zone::Stack)
                || crate::alternative_cast::blind_play::requires_opening(game, id, player) {
                continue;
            }
            for method in crate::alternative_cast::price_routes::candidates(game, player, card)? {
                if can_cast_spell_with_view(game, player, card, &method, &view) {
                    actions.push(LegalAction::CastSpell {
                        spell_id: id,
                        from_zone: card.zone,
                        casting_method: method,
                    });
                }
            }
        }
    }

    let battlefield_abilities_started_at = PerfTimer::start();
    add_battlefield_actions(
        game,
        &mut actions,
        player,
        &controlled_battlefield,
        &view,
        &battlefield_ability_ctx,
    )?;
    perf.battlefield_abilities_ms = battlefield_abilities_started_at.elapsed_ms();
    let battlefield_breakdown = battlefield_ability_ctx.snapshot_perf();
    perf.can_activate_ability_with_restrictions_with_view_ms = battlefield_breakdown.total_ms;
    perf.battlefield_ability_precheck_ms = battlefield_breakdown.precheck_ms;
    perf.battlefield_ability_target_legality_ms = battlefield_breakdown.target_legality_ms;
    perf.battlefield_ability_cost_build_ms = battlefield_breakdown.cost_build_ms;
    perf.battlefield_ability_affordability_ms = battlefield_breakdown.affordability_ms;

    let non_battlefield_abilities_started_at = PerfTimer::start();
    let non_battlefield_ids: Vec<_> =
        collect_non_battlefield_source_ids(game, player, hand, graveyard)
            .into_iter()
            .filter(|id| requested_action_source(*id))
            .collect();
    add_non_battlefield_ability_actions(game, &mut actions, player, &non_battlefield_ids, &view);

    perf.non_battlefield_abilities_ms = non_battlefield_abilities_started_at.elapsed_ms();
    let cast_breakdown = cast_ctx.snapshot_perf();
    perf.can_cast_spell_with_view_ms = cast_breakdown.total_ms;
    perf.spell_has_legal_targets_ms = cast_breakdown.target_legality_ms;
    perf.compute_potential_mana_with_view_ms = view.potential_mana_compute_ms();
    perf.hand_casts_timing_ms = cast_breakdown.timing_ms;
    perf.hand_casts_restrictions_ms = cast_breakdown.restrictions_ms;
    perf.hand_casts_target_legality_ms = cast_breakdown.target_legality_ms;
    perf.hand_casts_cost_adjustment_ms = cast_breakdown.cost_adjustment_ms;
    perf.hand_casts_affordability_ms = cast_breakdown.affordability_ms;
    perf.total_ms = total_started_at.elapsed_ms();
    perf.action_count = actions.len();
    store_compute_legal_actions_perf(perf);
    Ok(actions)
}

/// Returns whether an activated ability can be used right now based on per-turn
/// limits and textual activation restrictions parsed from Oracle text.
pub(crate) fn can_activate_ability_with_restrictions(
    game: &GameState,
    source: ObjectId,
    ability_index: usize,
    activated: &crate::ability::ActivatedAbility,
) -> bool {
    let view = DerivedGameView::new(game);
    can_activate_ability_with_restrictions_with_view(
        game,
        source,
        ability_index,
        activated,
        &view,
        None,
        None,
    )
}

fn activated_ability_has_legal_targets_with_view(
    activated: &crate::ability::ActivatedAbility,
    controller: PlayerId,
    source: ObjectId,
    view: &DerivedGameView<'_>,
) -> bool {
    let effects = activated.effects.flattened_default_effects();
    effects.is_empty() || view.spell_has_legal_targets(effects, controller, Some(source), None)
}

pub(crate) fn activation_timing_allows(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    ability_index: usize,
    activated: &crate::ability::ActivatedAbility,
    view: &DerivedGameView<'_>,
    timing: &crate::ability::ActivationTiming,
) -> bool {
    match timing {
        crate::ability::ActivationTiming::AnyTime | crate::ability::ActivationTiming::AsInstant => {
            true
        }
        crate::ability::ActivationTiming::DuringCombat => matches!(game.turn.phase, Phase::Combat),
        crate::ability::ActivationTiming::SorcerySpeed => {
            if activated.is_loyalty_ability()
                && player_may_activate_loyalty_abilities_any_time(game, controller, source, view)
            {
                return true;
            }
            if is_equip_ability(game, source, activated)
                && player_may_activate_equip_abilities_any_time(game, controller, view)
            {
                return true;
            }
            game.is_active_player(controller)
                && matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain)
                && game.stack_is_empty()
        }
        crate::ability::ActivationTiming::OncePerTurn => {
            game.ability_activation_count_this_turn(source, ability_index) == 0
        }
        crate::ability::ActivationTiming::DuringYourTurn => game.is_active_player(controller),
        crate::ability::ActivationTiming::DuringOpponentsTurn => !game.is_active_player(controller),
        crate::ability::ActivationTiming::AnyTimeByEnchantedCreatureController => game
            .object(source)
            .and_then(|object| object.attached_to)
            .and_then(|target| target.object_id())
            .is_some_and(|host| {
                game.object(host)
                    .is_some_and(|object| object.zone == Zone::Battlefield)
                    && game.current_has_card_type(host, crate::CardType::Creature)
                    && game.current_controller(host) == Some(controller)
            }),
        crate::ability::ActivationTiming::AnyPlayerDuringTheirTurnBeforeEndStep => {
            game.is_active_player(controller) && game.turn.phase != Phase::Ending
        }
        // "Only your opponents may activate this ability": the activating
        // player must be an opponent of the source's current controller.
        crate::ability::ActivationTiming::AnyTimeByOpponents => game
            .current_controller(source)
            .is_some_and(|source_controller| game.are_opponents(controller, source_controller)),
        crate::ability::ActivationTiming::SorcerySpeedByOpponents => {
            game.current_controller(source)
                .is_some_and(|source_controller| game.are_opponents(controller, source_controller))
                && game.is_active_player(controller)
                && matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain)
                && game.stack_is_empty()
        }
        crate::ability::ActivationTiming::DeclareAttackersStepByAttackedPlayer => {
            game.turn.phase == Phase::Combat
                && game.turn.step == Some(crate::game_state::Step::DeclareAttackers)
                && game.combat.as_ref().is_some_and(|combat| {
                    combat.attackers.iter().any(|info| {
                        info.creature == source
                            && info.target
                                == crate::combat_state::AttackTarget::Player(controller)
                    })
                })
        }
        crate::ability::ActivationTiming::DuringSourceOwnersUpkeep => {
            game.object(source)
                .is_some_and(|object| game.is_active_player(object.owner))
                && game.turn.phase == Phase::Beginning
                && game.turn.step == Some(crate::game_state::Step::Upkeep)
        }
        crate::ability::ActivationTiming::DuringYourUpkeep => {
            game.is_active_player(controller)
                && game.turn.phase == Phase::Beginning
                && game.turn.step == Some(crate::game_state::Step::Upkeep)
        }
        crate::ability::ActivationTiming::DuringOpponentsUpkeep => {
            !game.is_active_player(controller)
                && game.turn.phase == Phase::Beginning
                && game.turn.step == Some(crate::game_state::Step::Upkeep)
        }
        crate::ability::ActivationTiming::DuringAnyUpkeep => {
            game.turn.phase == Phase::Beginning
                && game.turn.step == Some(crate::game_state::Step::Upkeep)
        }
        timing => activation_step_window_allows(game, controller, *timing),
    }
}

/// The step windows of an activation timing: a named step of the turn, or
/// "before" a combat step, which CR 506.8 reads as earlier in the turn than
/// that step of the turn's first combat.
pub(crate) fn activation_step_window_allows(
    game: &GameState,
    controller: PlayerId,
    timing: crate::ability::ActivationTiming,
) -> bool {
    use crate::ability::ActivationTiming;
    use crate::game_state::Step;
    match timing {
        ActivationTiming::DuringYourDrawStep => {
            game.is_active_player(controller)
                && game.turn.phase == Phase::Beginning
                && game.turn.step == Some(Step::Draw)
        }
        ActivationTiming::DuringDeclareAttackersStep => {
            game.turn.phase == Phase::Combat && game.turn.step == Some(Step::DeclareAttackers)
        }
        ActivationTiming::DuringDeclareBlockersStep => {
            game.turn.phase == Phase::Combat && game.turn.step == Some(Step::DeclareBlockers)
        }
        ActivationTiming::BeforeAttackersDeclared => {
            turn_is_before_first_combat_step(game, Step::DeclareAttackers)
        }
        ActivationTiming::DuringYourTurnBeforeAttackersDeclared => {
            game.is_active_player(controller)
                && turn_is_before_first_combat_step(game, Step::DeclareAttackers)
        }
        ActivationTiming::BeforeBlockersDeclared => {
            turn_is_before_first_combat_step(game, Step::DeclareBlockers)
        }
        ActivationTiming::BeforeCombatDamageStep => {
            turn_is_before_first_combat_step(game, Step::CombatDamage)
        }
        ActivationTiming::BeforeEndOfCombatStep => {
            turn_is_before_first_combat_step(game, Step::EndCombat)
        }
        _ => true,
    }
}

/// Whether the game is earlier in the turn than `step` of the turn's first
/// combat phase (CR 506.8): the beginning phase, the precombat main phase, or
/// the first combat before that step.
pub(crate) fn turn_is_before_first_combat_step(
    game: &GameState,
    step: crate::game_state::Step,
) -> bool {
    use crate::game_state::Step;
    fn combat_step_order(step: Step) -> u8 {
        match step {
            Step::BeginCombat => 0,
            Step::DeclareAttackers => 1,
            Step::DeclareBlockers => 2,
            Step::CombatDamage => 3,
            Step::EndCombat => 4,
            _ => 0,
        }
    }
    match game.turn.phase {
        Phase::Beginning | Phase::FirstMain => true,
        Phase::Combat => {
            game.turn_store.combat_phases_started_this_turn <= 1
                && game.turn.step.map_or(0, combat_step_order) < combat_step_order(step)
        }
        Phase::NextMain | Phase::Ending => false,
    }
}

fn loyalty_activation_special_rules_allow(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    activated: &crate::ability::ActivatedAbility,
    view: &DerivedGameView<'_>,
) -> bool {
    if !activated.is_loyalty_ability() {
        return true;
    }

    // CR 606.3: one loyalty activation per permanent each turn, unless an
    // effect this turn allows more ("twice this turn rather than only once").
    let allowed = 1 + crate::effects::player::loyalty_activation_allowance::loyalty_allowance_count(
        game,
        controller,
        source,
        crate::effects::LoyaltyActivationAllowance::ExtraActivation,
    );
    game.loyalty_activations_this_turn(source) < allowed
        && ((game.is_active_player(controller)
            && matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain)
            && game.stack_is_empty())
            || player_may_activate_loyalty_abilities_any_time(game, controller, source, view))
}

fn loyalty_remove_counters_cost_amount(cost: &crate::costs::Cost) -> Option<u32> {
    let effect = cost.effect_ref()?;
    let effect = effect.downcast_ref::<crate::effects::WithIdEffect>().map_or(effect, |observed| &observed.effect);
    let effect = effect.downcast_ref::<crate::effects::RemoveCountersEffect>()?;
    if effect.counter_type != crate::CounterType::Loyalty {
        return None;
    }
    if !matches!(effect.target.base(), crate::target::ChooseSpec::Source) {
        return None;
    }
    let crate::effect::Value::Fixed(count) = &effect.count else {
        return None;
    };
    Some((*count).max(0) as u32)
}

fn loyalty_negative_costs_payable(
    game: &GameState,
    source: ObjectId,
    costs: &[crate::costs::Cost],
) -> bool {
    let required = costs
        .iter()
        .filter_map(loyalty_remove_counters_cost_amount)
        .sum::<u32>();
    required == 0 || game.counter_count(source, crate::CounterType::Loyalty) >= required
}

fn total_cost_branch_is_payable_with_view(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    cost: &crate::cost::TotalCost,
    reason: crate::costs::PaymentReason,
    view: &DerivedGameView<'_>,
) -> bool {
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let mut execution =
        crate::effects::ExecutionContext::new(source, controller, &mut decision_maker);
    if view.has_activated_ability_cost_modifiers()
        || view.source_has_activated_ability_cost_modifiers(source)
    {
        crate::special_actions::can_pay_cost_before_mana_funding_in_context(
            game,
            controller,
            source,
            cost,
            reason,
            &mut execution,
        )
        .is_ok()
    } else {
        crate::special_actions::can_potentially_pay_total_cost_in_context_with_view(
            game,
            controller,
            source,
            cost,
            reason,
            &mut execution,
            view,
        )
        .is_ok()
    }
}

fn every_cost_branch_requires(
    cost: &crate::cost::TotalCost,
    predicate: fn(&crate::costs::Cost) -> bool,
) -> bool {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => costs.iter().any(predicate),
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            !branches.is_empty()
                && branches
                    .iter()
                    .all(|branch| every_cost_branch_requires(branch, predicate))
        }
    }
}

fn activated_minimum_x_cost_is_payable(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    activated: &crate::ability::ActivatedAbility,
) -> bool {
    let minimum = activated.activation_x_minimum();
    if minimum == 0 {
        return true;
    }

    fn branch_maximum_x(
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
        cost: &crate::cost::TotalCost,
    ) -> Option<u32> {
        match cost.kind() {
            ironsmith_core::TotalCostKind::All(costs) => costs
                .iter()
                .filter_map(|cost| {
                    cost.effect_ref()
                        .and_then(|effect| effect.max_cost_x(game, source, controller))
                })
                .min(),
            ironsmith_core::TotalCostKind::OneOf(branches) => branches
                .iter()
                .filter_map(|branch| branch_maximum_x(game, source, controller, branch))
                .max(),
        }
    }

    branch_maximum_x(game, source, controller, &activated.mana_cost)
        .is_none_or(|maximum| maximum >= minimum)
}

pub(crate) fn is_equip_ability(
    _game: &GameState,
    _source: ObjectId,
    activated: &crate::ability::ActivatedAbility,
) -> bool {
    activated.keyword == Some(ironsmith_core::ActivatedAbilityKeyword::Equip)
}

fn player_may_activate_loyalty_abilities_any_time(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    view: &DerivedGameView<'_>,
) -> bool {
    use crate::filter::ObjectFilterExt;
    let Some(activated_object) = game.object(source) else {
        return false;
    };
    // "you may activate loyalty abilities of Jace planeswalkers you control
    // on any player's turn any time you could cast an instant" (an effect
    // lasting this turn).
    if crate::effects::player::loyalty_activation_allowance::loyalty_allowance_count(
        game,
        controller,
        source,
        crate::effects::LoyaltyActivationAllowance::InstantSpeed,
    ) > 0
    {
        return true;
    }
    // Emblems grant the permission from the command zone (CR 114.4), e.g.
    // Teferi, Temporal Archmage's emblem.
    game.battlefield
        .iter()
        .copied()
        .chain(game.command_zone.iter().copied())
        .any(|permission_source| {
            let Some(object) = game.object(permission_source) else {
                return false;
            };
            if game.controller_of(object) != controller {
                return false;
            }
            let permission_zone = object.zone;
            let abilities = view
                .abilities_rc(permission_source)
                .unwrap_or_else(|| std::sync::Arc::new(object.abilities_vec()));
            let ctx = game.filter_context_for(controller, Some(permission_source));
            abilities.iter().any(|ability| {
                if !ability.functional_zones.contains(&permission_zone) {
                    return false;
                }
                let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                    return false;
                };
                let Some(model) = static_ability.compiled_model() else {
                    return false;
                };
                let ironsmith_core::StaticAbilityPayload::LoyaltyAbilitiesAnyTime { filter } =
                    &model.payload
                else {
                    return false;
                };
                filter.matches(activated_object, &ctx, game)
            })
        })
}

fn player_may_activate_equip_abilities_any_time(
    game: &GameState,
    controller: PlayerId,
    view: &DerivedGameView<'_>,
) -> bool {
    game.battlefield.iter().copied().any(|object_id| {
        let Some(object) = game.object(object_id) else {
            return false;
        };
        if game.controller_of(object) != controller {
            return false;
        }
        let abilities = view
            .abilities_rc(object_id)
            .unwrap_or_else(|| std::sync::Arc::new(object.abilities_vec()));
        abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EquipAbilitiesAnyTime
            )
        })
    })
}

/// CR 702.177: Exhaust's restriction applies equally to mana abilities.
pub(crate) fn exhaust_activation_allows(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    ability_index: usize,
    activated: &crate::ability::ActivatedAbility,
    view: &DerivedGameView<'_>,
) -> bool {
    !activated.is_exhaust_ability()
        || !game.exhaust_ability_activated(source, ability_index)
        || player_may_activate_exhaust_abilities_as_unactivated_this_turn(game, controller, view)
}

fn player_may_activate_exhaust_abilities_as_unactivated_this_turn(
    game: &GameState,
    controller: PlayerId,
    view: &DerivedGameView<'_>,
) -> bool {
    if !game.is_active_player(controller)
        || game.exhaust_ability_activation_count_this_turn(controller) > 0
    {
        return false;
    }

    game.battlefield.iter().copied().any(|object_id| {
        let Some(object) = game.object(object_id) else {
            return false;
        };
        if game.controller_of(object) != controller {
            return false;
        }
        let abilities = view
            .abilities_rc(object_id)
            .unwrap_or_else(|| std::sync::Arc::new(object.abilities_vec()));
        abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::ExhaustAbilitiesAsThoughUnactivatedThisTurn
            )
        })
    })
}

fn activation_precheck_with_view(
    game: &GameState,
    source: ObjectId,
    ability_index: usize,
    activated: &crate::ability::ActivatedAbility,
    view: &DerivedGameView<'_>,
    perf_ctx: Option<&BattlefieldAbilityContext>,
    source_facts: Option<&ActivationSourceFacts>,
) -> Option<PlayerId> {
    let started_at = PerfTimer::start();
    let owned_facts;
    let source_facts = if let Some(source_facts) = source_facts {
        source_facts
    } else {
        owned_facts = ActivationSourceFacts::for_source(game, source, view);
        &owned_facts
    };
    let controller = if activated_allows_any_player(activated) {
        game.turn.priority_player.unwrap_or(source_facts.controller)
    } else {
        source_facts.controller
    };

    if !activated_minimum_x_cost_is_payable(game, controller, source, activated) {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if activated.is_loyalty_ability()
        && (controller != source_facts.controller
            || game
                .effect_store
                .cant_effects
                .cant_activate_loyalty_abilities_of
                .contains(&source))
    {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if game.object(source).is_some() && !game.can_activate_non_mana_abilities(controller) {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if !source_facts.can_activate_abilities {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    let every_branch_taps =
        every_cost_branch_requires(&activated.mana_cost, crate::costs::Cost::requires_tap);
    let every_branch_untaps =
        every_cost_branch_requires(&activated.mana_cost, crate::costs::Cost::requires_untap);
    if every_branch_taps && !source_facts.can_activate_tap_abilities {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    if every_branch_taps && source_facts.is_tapped {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    if (every_branch_taps || every_branch_untaps)
        && source_facts.is_creature
        && source_facts.is_summoning_sick
        && !source_facts.has_haste
    {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    if every_branch_untaps && !source_facts.is_tapped {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    if !activated.is_runtime_mana_ability(game, source, controller)
        && !source_facts.can_activate_non_mana_abilities_of_source
    {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    // CR 719.3c: a Case's "Solved — [activated ability]" can be activated
    // only while the Case is solved.
    if activation_requires_solved_case(activated) && !game.is_case_solved(source) {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if !exhaust_activation_allows(game, controller, source, ability_index, activated, view) {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if activated_ability_uses_simple_precheck(activated) {
        if !loyalty_activation_special_rules_allow(game, controller, source, activated, view) {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }

        if !activation_timing_allows(
            game,
            controller,
            source,
            ability_index,
            activated,
            view,
            &activated.timing,
        ) {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }

        if let Some(max_activations) = activated.max_activations_per_turn()
            && game.ability_activation_count_this_turn(source, ability_index) >= max_activations
        {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }

        let reason = activated.payment_reason(game, source, controller);
        let loyalty_costs_payable = match activated.mana_cost.kind() {
            ironsmith_core::TotalCostKind::All(costs) => {
                loyalty_negative_costs_payable(game, source, costs)
            }
            ironsmith_core::TotalCostKind::OneOf(branches) => branches.iter().any(|branch| {
                branch
                    .as_all()
                    .is_some_and(|costs| loyalty_negative_costs_payable(game, source, costs))
            }),
        };
        if activated.is_loyalty_ability() && !loyalty_costs_payable {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }
        if !crate::cost::prospective_references::activation_reference_preflight(
            game,
            source,
            ability_index,
            controller,
            activated,
        )
        .unwrap_or_else(|| {
            total_cost_branch_is_payable_with_view(
                game,
                controller,
                source,
                &activated.mana_cost,
                reason,
                view,
            )
        }) {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }

        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return Some(controller);
    }

    let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
        controller,
        source,
        defending_player: None,
        attacking_player: None,
        filter_source: Some(source),
        iterated_player: None,
        triggering_event: None,
        trigger_identity: None,
        ability_index: Some(ability_index),
        options: Default::default(),
    };

    if !loyalty_activation_special_rules_allow(game, controller, source, activated, view) {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if !activation_timing_allows(
        game,
        controller,
        source,
        ability_index,
        activated,
        view,
        &activated.timing,
    ) {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if !matches!(
        activated.timing,
        crate::ability::ActivationTiming::OncePerTurn
    ) && let Some(max_activations) = activated.max_activations_per_turn()
        && game.ability_activation_count_this_turn(source, ability_index) >= max_activations
    {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    if let Some(condition) = &activated.activation_condition
        && !crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
    {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    let reason = activated.payment_reason(game, source, controller);
    let loyalty_costs_payable = match activated.mana_cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => {
            loyalty_negative_costs_payable(game, source, costs)
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => branches.iter().any(|branch| {
            branch
                .as_all()
                .is_some_and(|costs| loyalty_negative_costs_payable(game, source, costs))
        }),
    };
    if activated.is_loyalty_ability() && !loyalty_costs_payable {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    if !crate::cost::prospective_references::activation_reference_preflight(
        game,
        source,
        ability_index,
        controller,
        activated,
    )
    .unwrap_or_else(|| {
        total_cost_branch_is_payable_with_view(
            game,
            controller,
            source,
            &activated.mana_cost,
            reason,
            view,
        )
    }) {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }

    for condition in &activated.activation_restrictions {
        if !crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx) {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }
    }

    for effect in &activated.effects {
        if let Some(modal) = effect.modal_effect_spec()
            && modal.disallow_previously_chosen_modes
            && !game.ability_has_unchosen_mode(
                source,
                ability_index,
                modal.modes.len(),
                modal.disallow_previously_chosen_modes_this_turn,
            )
        {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }
    }

    if let Some(perf_ctx) = perf_ctx {
        perf_ctx.add_precheck_ms(started_at.elapsed_ms());
    }
    Some(eval_ctx.controller)
}

/// Whether an activated ability is labelled "Solved —" (a Case, CR 719.3c).
fn activation_requires_solved_case(activated: &crate::ability::ActivatedAbility) -> bool {
    activated.additional_restrictions.iter().any(|restriction| {
        restriction
            .strip_prefix("__ironsmith_activation_label:")
            .is_some_and(|label| label.trim().eq_ignore_ascii_case("Solved"))
    })
}

pub(crate) fn activation_total_cost_is_payable_with_view(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    cost: &crate::cost::TotalCost,
    view: &DerivedGameView<'_>,
    reason: crate::costs::PaymentReason,
) -> bool {
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let mut execution =
        crate::effects::ExecutionContext::new(source, controller, &mut decision_maker);
    crate::special_actions::can_potentially_pay_total_cost_in_context_with_view(
        game,
        controller,
        source,
        cost,
        reason,
        &mut execution,
        view,
    )
    .is_ok()
}

pub(crate) fn activation_total_cost_branch_is_payable_with_view(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    cost: &crate::cost::TotalCost,
    view: &DerivedGameView<'_>,
    reason: crate::costs::PaymentReason,
) -> bool {
    activation_total_cost_is_payable_with_view(game, controller, source, cost, view, reason)
}

pub(crate) fn can_activate_ability_with_restrictions_with_view(
    game: &GameState,
    source: ObjectId,
    ability_index: usize,
    activated: &crate::ability::ActivatedAbility,
    view: &DerivedGameView<'_>,
    perf_ctx: Option<&BattlefieldAbilityContext>,
    source_facts: Option<&ActivationSourceFacts>,
) -> bool {
    let total_started_at = PerfTimer::start();
    let Some(controller) = activation_precheck_with_view(
        game,
        source,
        ability_index,
        activated,
        view,
        perf_ctx,
        source_facts,
    ) else {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_total_ms(total_started_at.elapsed_ms());
        }
        return false;
    };
    if !game.object_is_within_range(controller, source, Some(source)) {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_total_ms(total_started_at.elapsed_ms());
        }
        return false;
    }

    // The reference-aware preflight evaluates each public cost choice with
    // its own targets and fully modified price. Never redo it without tags.
    if let Some(payable) = crate::cost::prospective_references::activation_reference_preflight(
        game,
        source,
        ability_index,
        controller,
        activated,
    ) {
        return payable;
    }

    let target_started_at = PerfTimer::start();
    let has_legal_targets =
        activated_ability_has_legal_targets_with_view(activated, controller, source, view);
    if let Some(perf_ctx) = perf_ctx {
        perf_ctx.add_target_legality_ms(target_started_at.elapsed_ms());
    }
    if !has_legal_targets {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_total_ms(total_started_at.elapsed_ms());
        }
        return false;
    }

    let cost_started_at = PerfTimer::start();
    let has_activation_cost_modifiers = perf_ctx
        .map(BattlefieldAbilityContext::has_activation_cost_modifiers)
        .unwrap_or_else(|| view.has_activated_ability_cost_modifiers())
        || view.source_has_activated_ability_cost_modifiers(source);
    if !has_activation_cost_modifiers {
        // The precheck already validated the printed activation costs, so when
        // nothing can modify them we can stop after target legality.
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_total_ms(total_started_at.elapsed_ms());
        }
        return true;
    }

    let total_cost = {
        calculate_effective_activation_total_cost_with_view(
            game,
            controller,
            source,
            &activated.mana_cost,
            &[],
            Some(ActivationCostAbility::of(
                game, controller, source, activated,
            )),
            view,
        )
    };
    if let Some(perf_ctx) = perf_ctx {
        perf_ctx.add_cost_build_ms(cost_started_at.elapsed_ms());
    }
    let loyalty_costs_payable = match total_cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => {
            loyalty_negative_costs_payable(game, source, costs)
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => branches.iter().any(|branch| {
            branch
                .as_all()
                .is_some_and(|costs| loyalty_negative_costs_payable(game, source, costs))
        }),
    };
    if activated.is_loyalty_ability() && !loyalty_costs_payable {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_total_ms(total_started_at.elapsed_ms());
        }
        return false;
    }
    if !activation_total_cost_is_payable_with_view(game, controller, source, &total_cost, view, activated.payment_reason(game, source, controller)) {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_total_ms(total_started_at.elapsed_ms());
        }
        return false;
    }

    if let Some(perf_ctx) = perf_ctx {
        perf_ctx.add_total_ms(total_started_at.elapsed_ms());
    }
    true
}

/// Compute legal commander actions for a player (casting from command zone).
///
/// These are kept separate from regular legal actions so they can be accessed
/// via 'C' input rather than numeric indices.
pub fn compute_commander_actions(
    game: &GameState,
    player: PlayerId,
) -> Result<Vec<LegalAction>, crate::effects::ExecutionError> {
    with_complete_legality_query(game, |checked| {
        Ok(compute_commander_actions_checked(checked, player))
    })
}

fn compute_commander_actions_checked(game: &GameState, player: PlayerId) -> Vec<LegalAction> {
    let mut actions = Vec::new();
    let view = DerivedGameView::new(game);

    // Check for commanders that can be cast from command zone
    if let Some(player_obj) = game.player(player) {
        for &commander_id in player_obj.get_commanders() {
            if let Some(current_id) = game.current_commander_object(commander_id)
                && requested_action_source(current_id)
                && let Some(commander) = game.object(current_id)
            {
                // Only if the commander is in the command zone
                if commander.zone != Zone::Command {
                    continue;
                }
                // CR 903.8 / 601.2b: casting from the command zone offers every
                // choice a cast from hand would (alternative costs, the other
                // face or Adventure half, face down); commander tax is then
                // added to whichever total cost was chosen.
                let mut methods = vec![CastingMethod::Normal];
                if spell_can_be_cast_face_down(game, commander) {
                    methods.push(CastingMethod::FaceDown);
                }
                if spell_has_castable_linked_other_half(game, commander) {
                    methods.push(CastingMethod::SplitOtherHalf);
                }
                for casting_method in methods {
                    if can_cast_spell_with_view(game, player, commander, &casting_method, &view) {
                        actions.push(LegalAction::CastSpell {
                            spell_id: current_id,
                            from_zone: Zone::Command,
                            casting_method,
                        });
                    }
                }
                for (idx, alt_cast) in commander.alternative_casts.iter().enumerate() {
                    // Alternatives that aren't tied to another zone (dash,
                    // blitz, evoke, emerge, prowl...) report the hand as
                    // their casting zone.
                    if alt_cast.cast_from_zone() == Zone::Hand
                        && !alt_cast.requires_cast_from_hand()
                        && can_cast_with_alternative_from_hand_with_view(
                            game, player, commander, current_id, alt_cast, &view,
                        )
                    {
                        actions.push(LegalAction::CastSpell {
                            spell_id: current_id,
                            from_zone: Zone::Command,
                            casting_method: CastingMethod::Alternative(idx),
                        });
                    }
                }
                // Cost-replacement grants from other permanents ("you may pay
                // {W}{U}{B}{R}{G} rather than pay the mana cost for spells you
                // cast": Fist of Suns, Jodah) are registered for hand casts but
                // apply to any spell you cast, the commander included. Lookups
                // for `Zone::Command` read those hand grants
                // (`alternative_cast_grant_zone`), while the cast method keeps
                // the command zone as its origin and commander tax still applies.
                let granted_casts =
                    view.granted_alternative_casts_for_card(current_id, Zone::Command, player);
                let base_alt_idx = commander.alternative_casts.len();
                for (offset, grant) in granted_casts.iter().enumerate() {
                    if grant.method.cast_from_zone() != Zone::Hand
                        || grant.method.requires_cast_from_hand()
                        || !grant_usage_limit_allows(
                            game,
                            player,
                            grant.permission_identity.as_ref(),
                            grant.usage_limit,
                        )
                        || !can_cast_with_alternative_from_hand_with_view(
                            game,
                            player,
                            commander,
                            current_id,
                            &grant.method,
                            &view,
                        )
                    {
                        continue;
                    }
                    actions.push(LegalAction::CastSpell {
                        spell_id: current_id,
                        from_zone: Zone::Command,
                        casting_method: CastingMethod::PlayFrom {
                            source: grant.source_id,
                            zone: Zone::Command,
                            use_alternative: Some(base_alt_idx + offset),
                        },
                    });
                }
            }
        }
    }

    actions
}

pub(crate) fn commander_action_indices(actions: &[LegalAction]) -> Vec<usize> {
    actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| match action {
            LegalAction::CastSpell {
                from_zone: Zone::Command,
                ..
            } => Some(index),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod land_enumeration_failure_tests {
    use super::*;
    use crate::ability::{Ability, AbilityKind};
    use crate::card::LinkedFaceLayout;
    use crate::cards::CardDefinitionBuilder;
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::ids::CardId;
    use crate::static_abilities::{StaticAbility, StaticAbilityId, StaticAbilityKind};

    /// Finite for the real front face, deliberately nonconvergent only after
    /// the isolated land-face proposal has been installed. This ensures an
    /// outer successful discovery cannot hide the inner query's typed error.
    #[derive(Debug, Clone)]
    struct RegrantOnlyForBackFace;
    impl StaticAbilityKind for RegrantOnlyForBackFace {
        fn id(&self) -> StaticAbilityId {
            StaticAbilityId::GrantObjectAbilityForFilter
        }
        fn display(&self) -> String {
            "Back-face discovery failure fixture".into()
        }
        fn generate_effects(
            &self,
            source: ObjectId,
            controller: PlayerId,
            game: &GameState,
        ) -> Vec<ContinuousEffect> {
            if !game
                .objects_in_deterministic_order()
                .iter()
                .any(|object| object.name.as_str() == "Unbounded back")
            {
                return Vec::new();
            }
            let AbilityKind::Static(parent) = &game.object(source).unwrap().abilities[0].kind
            else {
                panic!("fixture parent");
            };
            vec![ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::AddAbility(parent.clone()),
            )]
        }
    }

    #[test]
    fn every_land_origin_propagates_selected_face_discovery_failure_without_publishing_partial_actions()
     {
        // Authored only; no execution before the campaign validation gate.
        for (zone, adventure) in [
            (Zone::Hand, false),
            (Zone::Graveyard, false),
            (Zone::Exile, false),
            (Zone::Library, false),
            (Zone::OutsideGame, false),
            (Zone::Exile, true),
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let player = PlayerId::from_index(0);
            game.turn.active_player = player;
            game.turn.priority_player = Some(player);
            game.turn.phase = crate::game_state::Phase::FirstMain;
            game.turn.step = None;
            let host = CardDefinitionBuilder::new(CardId::new(), "Finite front query host")
                .card_types(vec![crate::types::CardType::Artifact])
                .with_ability(Ability::static_ability(StaticAbility::new(
                    RegrantOnlyForBackFace,
                )))
                .build();
            game.create_object_from_definition(&host, player, Zone::Battlefield);
            if zone != Zone::Hand && !adventure {
                let mut spec = crate::grant::GrantSpec::play_from_graveyard();
                spec.zone = zone;
                let permission = CardDefinitionBuilder::new(CardId::new(), "Exact zone permission")
                    .card_types(vec![crate::types::CardType::Artifact])
                    .with_ability(Ability::static_ability(StaticAbility::grants(spec)))
                    .build();
                game.create_object_from_definition(&permission, player, Zone::Battlefield);
            }
            let front_id = CardId::new();
            let back_id = CardId::new();
            let front = CardDefinitionBuilder::new(front_id, "Finite front")
                .card_types(vec![crate::types::CardType::Land])
                .other_face(back_id)
                .other_face_name("Unbounded back")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            let back = CardDefinitionBuilder::new(back_id, "Unbounded back")
                .card_types(vec![crate::types::CardType::Land])
                .other_face(front_id)
                .other_face_name("Finite front")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            let candidate = game.create_object_from_definition(&front, player, zone);
            if adventure {
                game.set_adventure_exiled_for(candidate, player);
            }
            game.continuous_query_snapshot()
                .expect("unselected face is finite");
            for scoped in [false, true] {
                let result = if scoped {
                    compute_actions_for_source(&game, player, Some(candidate))
                } else {
                    compute_legal_actions(&game, player)
                };
                assert!(matches!(result, Err(crate::effects::ExecutionError::ContinuousDiscovery(
                    crate::static_ability_processor::StaticEffectDiscoveryError::RoundLimit { .. }))),
                    "zone={zone:?}, adventure={adventure}, scoped={scoped}: {result:?}");
                assert_eq!(
                    game.object(candidate).unwrap().name.as_str(),
                    "Finite front"
                );
                assert_eq!(game.object(candidate).unwrap().zone, zone);
            }
        }
    }
    #[test]
    fn granted_spell_enumeration_propagates_selected_face_discovery_failure() {
        for zone in [
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::OutsideGame,
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let player = PlayerId::from_index(0);
            game.turn.active_player = player;
            game.turn.priority_player = Some(player);
            game.turn.phase = crate::game_state::Phase::FirstMain;
            game.turn.step = None;
            let host = CardDefinitionBuilder::new(CardId::new(), "Finite spell query host")
                .card_types(vec![crate::types::CardType::Artifact])
                .with_ability(Ability::static_ability(StaticAbility::new(
                    RegrantOnlyForBackFace,
                )))
                .build();
            game.create_object_from_definition(&host, player, Zone::Battlefield);
            let mut spec = crate::grant::GrantSpec::play_from_graveyard();
            spec.zone = zone;
            spec.filter = crate::target::ObjectFilter::creature();
            spec.top_card_only = zone == Zone::Library;
            let permission = CardDefinitionBuilder::new(CardId::new(), "Chosen-face permission")
                .card_types(vec![crate::types::CardType::Artifact])
                .with_ability(Ability::static_ability(StaticAbility::grants(spec)))
                .build();
            game.create_object_from_definition(&permission, player, Zone::Battlefield);
            let front_id = CardId::new();
            let back_id = CardId::new();
            let front = CardDefinitionBuilder::new(front_id, "Finite front")
                .card_types(vec![crate::types::CardType::Artifact])
                .mana_cost(crate::ManaCost::new())
                .other_face(back_id)
                .other_face_name("Unbounded back")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            let back = CardDefinitionBuilder::new(back_id, "Unbounded back")
                .card_types(vec![crate::types::CardType::Creature])
                .mana_cost(crate::ManaCost::new())
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .other_face(front_id)
                .other_face_name("Finite front")
                .linked_face_layout(LinkedFaceLayout::TransformLike)
                .build();
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            let card = game.create_object_from_definition(&front, player, zone);
            game.continuous_query_snapshot()
                .expect("front query is finite");
            for scoped in [false, true] {
                let result = if scoped {
                    compute_actions_for_source(&game, player, Some(card))
                } else {
                    compute_legal_actions(&game, player)
                };
                assert!(
                    matches!(
                        result,
                        Err(crate::effects::ExecutionError::ContinuousDiscovery(_))
                    ),
                    "{zone:?}: {result:?}"
                );
                assert_eq!(game.object(card).unwrap().name.as_str(), "Finite front");
                assert_eq!(game.object(card).unwrap().zone, zone);
            }
        }
    }
}

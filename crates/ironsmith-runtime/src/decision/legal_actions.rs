use super::*;
use crate::ability::ActivatedAbilityRuntimeExt as _;

fn append_granted_play_from_actions_for_card(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    card_id: ObjectId,
    card: &crate::object::Object,
    source_zone: Zone,
    view: &DerivedGameView<'_>,
) {
    let play_from_grants = view.granted_play_from_for_card(card_id, source_zone, player);
    for grant in play_from_grants {
        // PlayFrom (e.g., Yawgmoth's Will): can cast from zone as if from hand.
        let from_zone = grant.zone;
        let granted_alternatives =
            view.granted_alternative_casts_for_card(card_id, from_zone, player);
        let has_same_source_granted_alternative = granted_alternatives
            .iter()
            .any(|granted_alt| granted_alt.source_id == grant.source_id);

        if !has_same_source_granted_alternative
            && !card.is_land()
            && let Some(mana_cost) = &card.mana_cost
            && can_cast_with_cost_with_view(
                game,
                player,
                card,
                card_id,
                Some(mana_cost),
                None,
                &AdditionalCastRequirements::default(),
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
                && can_cast_with_alternative_from_hand_with_view(
                    game, player, card, card_id, alt_cast, view,
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

        let base_alt_idx = card.alternative_casts.len();
        for (offset, granted_alt) in granted_alternatives.iter().enumerate() {
            if can_cast_with_alternative_with_view(game, player, card, &granted_alt.method, view) {
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
        if alt_cast.cast_from_zone() == from_zone
            && can_cast_with_alternative_with_view(game, player, card, alt_cast, view)
        {
            actions.push(LegalAction::CastSpell {
                spell_id: card_id,
                from_zone,
                casting_method: CastingMethod::Alternative(idx),
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
    let granted_casts = view.granted_alternative_casts_for_card(card_id, Zone::Graveyard, player);

    for grant in granted_casts {
        let method = &grant.method;
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
            _ => continue,
        };

        if !can_cast_with_cost_with_view_for_casting_method(
            game,
            player,
            card,
            card_id,
            mana_cost,
            None,
            &requirements,
            &casting_method,
            view,
        ) {
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
) {
    append_native_alternative_cast_actions_for_card_from_zone(
        game, actions, player, card_id, card, from_zone, view,
    );
    if zone_has_active_grants && from_zone == Zone::Graveyard {
        append_graveyard_granted_alternative_cast_actions_for_card(
            game, actions, player, card_id, card, view,
        );
    }
    if zone_has_active_grants {
        append_granted_play_from_actions_for_card(
            game, actions, player, card_id, card, from_zone, view,
        );
    }
}

fn append_granted_land_play_actions_from_public_zone(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    zone: Zone,
    view: &DerivedGameView<'_>,
) {
    for card_id in crate::object_query::candidate_ids_for_zone(game, Some(zone)) {
        let Some(card) = game.object(card_id) else {
            continue;
        };
        if !card.is_land() {
            continue;
        }
        if view
            .granted_play_from_for_card(card_id, zone, player)
            .is_empty()
        {
            continue;
        }

        let action = SpecialAction::PlayLand { card_id };
        if crate::special_actions::can_perform_check(&action, game, player).is_ok() {
            actions.push(LegalAction::PlayLand { land_id: card_id });
        }
    }
}

/// Compute legal actions for a player who has priority.
///
/// This validates each potential action by testing it against the actual game rules.
/// Only actions that would succeed are included in the result.
fn build_hand_summaries<'a>(game: &'a GameState, hand: &[ObjectId]) -> Vec<HandCardSummary<'a>> {
    hand.iter()
        .filter_map(|&card_id| {
            let card = game.object(card_id)?;
            let mut has_foretell = false;
            let mut has_suspend = false;
            let mut has_plot = false;
            let mut has_hand_native_alternatives = false;
            for method in &card.alternative_casts {
                match method {
                    crate::alternative_cast::AlternativeCastingMethod::Foretell { .. } => {
                        has_foretell = true;
                    }
                    crate::alternative_cast::AlternativeCastingMethod::Suspend { .. } => {
                        has_suspend = true;
                    }
                    crate::alternative_cast::AlternativeCastingMethod::Plot { .. } => {
                        has_plot = true;
                    }
                    _ => {}
                }
                if method.cast_from_zone() == Zone::Hand {
                    has_hand_native_alternatives = true;
                }
            }
            Some(HandCardSummary {
                card_id,
                card,
                is_land: card.is_land(),
                has_normal_mana_cost: card.mana_cost.is_some(),
                has_foretell,
                has_suspend,
                has_plot,
                can_cast_face_down: spell_can_be_cast_face_down(card),
                has_split_other_half: card.linked_face_layout
                    == crate::card::LinkedFaceLayout::Split,
                has_fuse: card.has_fuse,
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
    view: &DerivedGameView<'_>,
) {
    use crate::special_actions::{SpecialAction, can_perform_check};

    for summary in hand_summaries {
        if summary.is_land {
            let action = SpecialAction::PlayLand {
                card_id: summary.card_id,
            };
            if can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::PlayLand {
                    land_id: summary.card_id,
                });
            }
        }
    }
    if graveyard_has_active_grants {
        append_granted_land_play_actions_from_public_zone(
            game,
            actions,
            player,
            Zone::Graveyard,
            view,
        );
    }
    if exile_has_active_grants {
        append_granted_land_play_actions_from_public_zone(game, actions, player, Zone::Exile, view);
    }
}

fn add_hand_normal_cast_actions(
    actions: &mut Vec<LegalAction>,
    hand_summaries: &[HandCardSummary<'_>],
    cast_ctx: &CastLegalityContext<'_>,
) -> std::collections::HashSet<ObjectId> {
    let mut hand_cards_with_normal_cast = std::collections::HashSet::new();

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
            hand_cards_with_normal_cast.insert(summary.card_id);
        }
    }

    hand_cards_with_normal_cast
}

fn add_hand_special_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    hand_summaries: &[HandCardSummary<'_>],
) {
    use crate::special_actions::{SpecialAction, can_perform_check};

    for summary in hand_summaries {
        if !summary.has_any_hand_special_action() {
            continue;
        }
        if summary.has_foretell {
            let action = SpecialAction::Foretell {
                card_id: summary.card_id,
            };
            if can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::SpecialAction(action));
            }
        }
        if summary.has_suspend {
            let action = SpecialAction::Suspend {
                card_id: summary.card_id,
            };
            if can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::SpecialAction(action));
            }
        }
        if summary.has_plot {
            let action = SpecialAction::Plot {
                card_id: summary.card_id,
            };
            if can_perform_check(&action, game, player).is_ok() {
                actions.push(LegalAction::SpecialAction(action));
            }
        }
    }
}

fn add_graveyard_cast_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    graveyard: &[ObjectId],
    view: &DerivedGameView<'_>,
    graveyard_has_active_grants: bool,
) {
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
            );
        }
    }
}

fn add_exile_cast_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    view: &DerivedGameView<'_>,
    exile_has_active_grants: bool,
) {
    for &card_id in &game.exile {
        let Some(card) = game.object(card_id) else {
            continue;
        };
        append_cast_actions_from_zone_for_card(
            game,
            actions,
            player,
            card_id,
            card,
            Zone::Exile,
            view,
            exile_has_active_grants,
        );
    }
    if exile_has_active_grants {
        append_granted_land_play_actions_from_public_zone(game, actions, player, Zone::Exile, view);
    }
}

fn add_hand_alternative_cast_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    hand_summaries: &[HandCardSummary<'_>],
    hand_cards_with_normal_cast: &std::collections::HashSet<ObjectId>,
    hand_has_active_grants: bool,
    view: &DerivedGameView<'_>,
    cast_ctx: &CastLegalityContext<'_>,
) {
    for summary in hand_summaries {
        if hand_cards_with_normal_cast.contains(&summary.card_id)
            || summary.is_land
            || !summary.has_any_alternative_branch(hand_has_active_grants)
        {
            continue;
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
    }
}

fn add_battlefield_actions(
    game: &GameState,
    actions: &mut Vec<LegalAction>,
    player: PlayerId,
    controlled_battlefield: &[ObjectId],
    view: &DerivedGameView<'_>,
    battlefield_ability_ctx: &BattlefieldAbilityContext,
) {
    use crate::special_actions::{SpecialAction, can_perform_check};

    for &perm_id in controlled_battlefield {
        if game.is_face_down(perm_id) {
            for method in crate::special_actions::available_turn_face_up_methods(game, perm_id) {
                let action = SpecialAction::TurnFaceUp {
                    permanent_id: perm_id,
                    method,
                };
                if can_perform_check(&action, game, player).is_ok() {
                    actions.push(LegalAction::TurnFaceUp {
                        creature_id: perm_id,
                        method,
                    });
                }
            }
        }
    }

    let simple_mana_analysis = view.simple_battlefield_mana_analysis(player);
    for &perm_id in simple_mana_analysis.relevant_source_ids() {
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
            .filter(|id| game.object(*id).is_some_and(|obj| obj.owner == player)),
    );
    non_battlefield_ids.extend(
        game.command_zone
            .iter()
            .copied()
            .filter(|id| game.object(*id).is_some_and(|obj| obj.owner == player)),
    );
    non_battlefield_ids.sort_by_key(|id| id.0);
    non_battlefield_ids.dedup();
    non_battlefield_ids
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
        if obj.zone == Zone::Battlefield || game.controller_of(obj) != player {
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
            for &ability_index in ability_summary.activated_ability_indices() {
                let Some(ability) = obj.abilities.get(ability_index) else {
                    continue;
                };
                let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
                    continue;
                };
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

pub fn compute_legal_actions(game: &GameState, player: PlayerId) -> Vec<LegalAction> {
    let total_started_at = PerfTimer::start();
    let mut perf = ComputeLegalActionsPerfMetrics::default();
    let empty_zone: &[ObjectId] = &[];
    let (hand, graveyard) = game
        .player(player)
        .map_or((empty_zone, empty_zone), |player_obj| {
            (player_obj.hand.as_slice(), player_obj.graveyard.as_slice())
        });
    let mut actions = Vec::with_capacity(
        1 + hand.len() * 6
            + graveyard.len() * 2
            + game.exile.len() * 2
            + game.battlefield.len() * 4,
    );
    let view = DerivedGameView::new(game);
    let cast_ctx = CastLegalityContext::new(game, player, &view);
    let battlefield_ability_ctx = BattlefieldAbilityContext::new(&view);
    let hand_has_active_grants = view.player_has_active_grants_for_zone(player, Zone::Hand);
    let graveyard_has_active_grants =
        view.player_has_active_grants_for_zone(player, Zone::Graveyard);
    let exile_has_active_grants = view.player_has_active_grants_for_zone(player, Zone::Exile);
    let hand_summaries = build_hand_summaries(game, hand);
    let controlled_battlefield = collect_controlled_battlefield(game, player);
    let prewarm_started_at = PerfTimer::start();
    view.prewarm_characteristics(&controlled_battlefield);
    perf.prewarm_ms = prewarm_started_at.elapsed_ms();

    actions.push(LegalAction::PassPriority);

    let lands_started_at = PerfTimer::start();
    add_land_actions(
        game,
        &mut actions,
        player,
        &hand_summaries,
        graveyard_has_active_grants,
        exile_has_active_grants,
        &view,
    );
    perf.lands_ms = lands_started_at.elapsed_ms();

    let hand_casts_started_at = PerfTimer::start();
    let hand_cards_with_normal_cast =
        add_hand_normal_cast_actions(&mut actions, &hand_summaries, &cast_ctx);
    perf.hand_casts_ms = hand_casts_started_at.elapsed_ms();

    let hand_special_actions_started_at = PerfTimer::start();
    add_hand_special_actions(game, &mut actions, player, &hand_summaries);
    perf.hand_special_actions_ms = hand_special_actions_started_at.elapsed_ms();

    let graveyard_casts_started_at = PerfTimer::start();
    add_graveyard_cast_actions(
        game,
        &mut actions,
        player,
        graveyard,
        &view,
        graveyard_has_active_grants,
    );
    perf.graveyard_casts_ms = graveyard_casts_started_at.elapsed_ms();

    let exile_casts_started_at = PerfTimer::start();
    add_exile_cast_actions(game, &mut actions, player, &view, exile_has_active_grants);
    perf.exile_casts_ms = exile_casts_started_at.elapsed_ms();

    let hand_alternatives_started_at = PerfTimer::start();
    add_hand_alternative_cast_actions(
        game,
        &mut actions,
        player,
        &hand_summaries,
        &hand_cards_with_normal_cast,
        hand_has_active_grants,
        &view,
        &cast_ctx,
    );
    perf.hand_alternatives_ms = hand_alternatives_started_at.elapsed_ms();

    let battlefield_abilities_started_at = PerfTimer::start();
    add_battlefield_actions(
        game,
        &mut actions,
        player,
        &controlled_battlefield,
        &view,
        &battlefield_ability_ctx,
    );
    perf.battlefield_abilities_ms = battlefield_abilities_started_at.elapsed_ms();
    let battlefield_breakdown = battlefield_ability_ctx.snapshot_perf();
    perf.can_activate_ability_with_restrictions_with_view_ms = battlefield_breakdown.total_ms;
    perf.battlefield_ability_precheck_ms = battlefield_breakdown.precheck_ms;
    perf.battlefield_ability_target_legality_ms = battlefield_breakdown.target_legality_ms;
    perf.battlefield_ability_cost_build_ms = battlefield_breakdown.cost_build_ms;
    perf.battlefield_ability_affordability_ms = battlefield_breakdown.affordability_ms;

    let non_battlefield_abilities_started_at = PerfTimer::start();
    let non_battlefield_ids = collect_non_battlefield_source_ids(game, player, hand, graveyard);
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
    actions
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

fn activation_timing_allows(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    ability_index: usize,
    timing: &crate::ability::ActivationTiming,
) -> bool {
    match timing {
        crate::ability::ActivationTiming::AnyTime => true,
        crate::ability::ActivationTiming::DuringCombat => matches!(game.turn.phase, Phase::Combat),
        crate::ability::ActivationTiming::SorcerySpeed => {
            game.turn.active_player == controller
                && matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain)
                && game.stack_is_empty()
        }
        crate::ability::ActivationTiming::OncePerTurn => {
            game.ability_activation_count_this_turn(source, ability_index) == 0
        }
        crate::ability::ActivationTiming::DuringYourTurn => game.turn.active_player == controller,
        crate::ability::ActivationTiming::DuringOpponentsTurn => {
            game.turn.active_player != controller
        }
    }
}

fn activation_cost_component_precheck_with_view(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    cost: &crate::costs::Cost,
    reason: crate::costs::PaymentReason,
    _view: &DerivedGameView<'_>,
) -> bool {
    if let Some(amount) = cost.life_amount() {
        return game.can_pay_life_with_reason(controller, amount, reason);
    }

    if let Some((count, card_type)) = cost.discard_details() {
        let Some(player) = game.player(controller) else {
            return false;
        };
        let available = player
            .hand
            .iter()
            .filter_map(|object_id| game.object(*object_id))
            .filter(|object| {
                card_type.is_none_or(|required_type| object.card_types.contains(&required_type))
            })
            .count();
        return available >= count as usize;
    }

    game.validate_cost_for_payment_reason(controller, source, cost, reason)
        .is_ok()
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
    let controller = source_facts.controller;

    if let Some(obj) = game.object(source)
        && !game.can_activate_non_mana_abilities(game.controller_of(obj))
    {
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
    if activated.has_tap_cost() && !source_facts.can_activate_tap_abilities {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    if activated.has_tap_cost() && source_facts.is_tapped {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    let has_untap_cost = activated
        .mana_cost
        .costs()
        .iter()
        .any(|cost| cost.requires_untap());
    if (activated.has_tap_cost() || has_untap_cost)
        && source_facts.is_creature
        && source_facts.is_summoning_sick
        && !source_facts.has_haste
    {
        if let Some(perf_ctx) = perf_ctx {
            perf_ctx.add_precheck_ms(started_at.elapsed_ms());
        }
        return None;
    }
    if activated
        .mana_cost
        .costs()
        .iter()
        .any(|cost| cost.requires_untap())
        && !source_facts.is_tapped
    {
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

    if activated_ability_uses_simple_precheck(activated) {
        if !activation_timing_allows(game, controller, source, ability_index, &activated.timing) {
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

        for cost in activated.mana_cost.costs() {
            if !activation_cost_component_precheck_with_view(
                game,
                controller,
                source,
                cost,
                crate::costs::PaymentReason::ActivateAbility,
                view,
            ) {
                if let Some(perf_ctx) = perf_ctx {
                    perf_ctx.add_precheck_ms(started_at.elapsed_ms());
                }
                return None;
            }
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
        triggering_event: None,
        trigger_identity: None,
        ability_index: Some(ability_index),
        options: Default::default(),
    };

    if !activation_timing_allows(game, controller, source, ability_index, &activated.timing) {
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

    for cost in activated.mana_cost.costs() {
        if !activation_cost_component_precheck_with_view(
            game,
            controller,
            source,
            cost,
            crate::costs::PaymentReason::ActivateAbility,
            view,
        ) {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_precheck_ms(started_at.elapsed_ms());
            }
            return None;
        }
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

fn activation_card_cost_choice_cost(
    choice: &crate::game_loop::ActivationCardCostChoice,
) -> &crate::costs::Cost {
    match choice {
        crate::game_loop::ActivationCardCostChoice::Discard { cost, .. }
        | crate::game_loop::ActivationCardCostChoice::ExileFromHand { cost, .. }
        | crate::game_loop::ActivationCardCostChoice::ExileFromGraveyard { cost, .. }
        | crate::game_loop::ActivationCardCostChoice::ExileChosenObject { cost, .. }
        | crate::game_loop::ActivationCardCostChoice::RevealFromHand { cost, .. }
        | crate::game_loop::ActivationCardCostChoice::ReturnToHand { cost, .. } => cost,
    }
}

fn activation_cost_is_payable_with_view(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    cost: &crate::costs::Cost,
    _view: &DerivedGameView<'_>,
) -> bool {
    let reason = crate::costs::PaymentReason::ActivateAbility;
    if game
        .validate_cost_for_payment_reason(controller, source, cost, reason)
        .is_err()
    {
        return false;
    }

    if cost.mana_cost_ref().is_some() {
        // Ability actions should remain visible before mana is floated, even when
        // cost modifiers reprice the mana portion. The payment flow will enforce
        // the actual mana payment later.
        return true;
    }

    let check_ctx = crate::costs::CostCheckContext::new(source, controller).with_reason(reason);
    crate::costs::can_pay_with_check_context(&*cost.0, game, &check_ctx).is_ok()
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
        .unwrap_or_else(|| view.has_activated_ability_cost_modifiers());
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
            view,
        )
    };
    if let Some(perf_ctx) = perf_ctx {
        perf_ctx.add_cost_build_ms(cost_started_at.elapsed_ms());
    }
    let components = total_cost.costs();
    let mut idx = 0usize;
    while idx < components.len() {
        if let Some(choose) = components[idx]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(next) = components.get(idx + 1)
            && let Some(step) = crate::game_loop::choose_tagged_cost_step(choose, next)
        {
            let paired_cost = match &step {
                crate::game_loop::ActivationCostStep::Cost(cost) => cost,
                crate::game_loop::ActivationCostStep::Sacrifice { cost, .. } => cost,
                crate::game_loop::ActivationCostStep::CardChoice(choice) => {
                    activation_card_cost_choice_cost(choice)
                }
            };
            if !activation_cost_is_payable_with_view(game, controller, source, paired_cost, view) {
                if let Some(perf_ctx) = perf_ctx {
                    perf_ctx.add_total_ms(total_started_at.elapsed_ms());
                }
                return false;
            }
            idx += 2;
            continue;
        }

        if !activation_cost_is_payable_with_view(game, controller, source, &components[idx], view) {
            if let Some(perf_ctx) = perf_ctx {
                perf_ctx.add_total_ms(total_started_at.elapsed_ms());
            }
            return false;
        }
        idx += 1;
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
pub fn compute_commander_actions(game: &GameState, player: PlayerId) -> Vec<LegalAction> {
    let mut actions = Vec::new();
    let view = DerivedGameView::new(game);

    // Check for commanders that can be cast from command zone
    if let Some(player_obj) = game.player(player) {
        for &commander_id in player_obj.get_commanders() {
            if let Some(current_id) = game.current_commander_object(commander_id)
                && let Some(commander) = game.object(current_id)
            {
                // Only if the commander is in the command zone
                if commander.zone == Zone::Command
                    && can_cast_spell_with_view(
                        game,
                        player,
                        commander,
                        &CastingMethod::Normal,
                        &view,
                    )
                {
                    actions.push(LegalAction::CastSpell {
                        spell_id: current_id,
                        from_zone: Zone::Command,
                        casting_method: CastingMethod::Normal,
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

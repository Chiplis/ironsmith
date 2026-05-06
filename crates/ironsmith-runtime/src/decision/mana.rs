use super::*;
use crate::ability::ActivatedAbilityRuntimeExt as _;
use crate::filter::ObjectFilterExt as _;

fn count_basic_land_types_among_filter(
    game: &GameState,
    filter: &crate::target::ObjectFilter,
    filter_ctx: &crate::filter::FilterContext,
) -> u32 {
    let mut seen = std::collections::HashSet::new();
    for obj in game.objects_in_deterministic_order() {
        if !filter.matches(obj, filter_ctx, game) {
            continue;
        }
        for subtype in game.calculated_subtypes(obj.id) {
            if subtype.is_basic_land_type() {
                seen.insert(subtype);
            }
        }
    }
    seen.len() as u32
}

/// Calculate activated-ability cost after applying battlefield static cost modifiers.
pub fn calculate_effective_activation_total_cost(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    cost: &crate::cost::TotalCost,
) -> crate::cost::TotalCost {
    calculate_effective_activation_total_cost_with_chosen_targets(
        game,
        activator,
        ability_source,
        cost,
        &[],
    )
}

pub fn calculate_effective_activation_total_cost_with_chosen_targets(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    cost: &crate::cost::TotalCost,
    chosen_targets: &[Target],
) -> crate::cost::TotalCost {
    let view = DerivedGameView::new(game);
    calculate_effective_activation_total_cost_with_view(
        game,
        activator,
        ability_source,
        cost,
        chosen_targets,
        &view,
    )
}

pub(crate) fn calculate_effective_activation_total_cost_with_view(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    cost: &crate::cost::TotalCost,
    chosen_targets: &[Target],
    view: &DerivedGameView<'_>,
) -> crate::cost::TotalCost {
    use crate::ability::AbilityKind;
    use crate::filter::{FilterContext, player_filter_matches_game};

    fn opponents_of(game: &GameState, player: PlayerId) -> Vec<PlayerId> {
        game.turn_store
            .turn_order
            .iter()
            .copied()
            .filter(|p| *p != player)
            .collect()
    }

    let mut costs = Vec::with_capacity(cost.costs().len());
    for component in cost.costs() {
        if let Some(mana_cost) = component.mana_cost_ref() {
            let reduced = calculate_effective_activation_mana_cost_with_view(
                game,
                activator,
                ability_source,
                mana_cost,
                chosen_targets,
                view,
            );
            costs.push(crate::costs::Cost::mana(reduced));
        } else {
            costs.push(component.clone());
        }
    }

    let mut adjusted = crate::cost::TotalCost::from_costs(costs);
    let Some(ability_source_object) = game.object(ability_source) else {
        return adjusted;
    };

    let mut cost_modifier_sources = view.activated_ability_cost_modifier_sources();
    if ability_source_object.zone != Zone::Battlefield {
        cost_modifier_sources.push(ability_source);
    }

    for source_id in cost_modifier_sources {
        let Some(perm) = game.object(source_id) else {
            continue;
        };
        let controller = game.controller_of(perm);
        let filter_ctx = FilterContext::new(controller)
            .with_source(source_id)
            .with_active_player(game.turn.active_player)
            .with_opponents(opponents_of(game, controller));

        let static_abilities = if perm.zone == Zone::Battlefield {
            view.static_abilities_rc(source_id).unwrap_or_else(|| {
                Rc::new(
                    perm.abilities
                        .iter()
                        .filter_map(|a| match &a.kind {
                            AbilityKind::Static(sa) => Some(sa.clone()),
                            _ => None,
                        })
                        .collect(),
                )
            })
        } else {
            Rc::new(
                perm.abilities
                    .iter()
                    .filter_map(|a| match &a.kind {
                        AbilityKind::Static(sa) if a.functions_in(&perm.zone) => Some(sa.clone()),
                        _ => None,
                    })
                    .collect(),
            )
        };

        for static_ability in static_abilities.iter() {
            if !static_ability.is_active(game, source_id) {
                continue;
            }

            if let Some(increase) = static_ability.activated_ability_cost_increase() {
                if let Some(activator_filter) = &increase.activator
                    && !player_filter_matches_game(activator_filter, activator, game, &filter_ctx)
                {
                    continue;
                }
                if !increase
                    .filter
                    .matches(ability_source_object, &filter_ctx, game)
                {
                    continue;
                }

                let mut costs = adjusted.costs().to_vec();
                costs.extend(increase.increase.costs().iter().cloned());
                adjusted = crate::cost::TotalCost::from_costs(costs);
            }
        }
    }

    adjusted
}

/// Calculate the effective mana portion of an activated ability's cost.
pub fn calculate_effective_activation_mana_cost(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    base_cost: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_activation_mana_cost_with_view(
        game,
        activator,
        ability_source,
        base_cost,
        &[],
        &view,
    )
}

pub(crate) fn calculate_effective_activation_mana_cost_with_view(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;
    use crate::filter::FilterContext;

    fn opponents_of(game: &GameState, player: PlayerId) -> Vec<PlayerId> {
        game.turn_store
            .turn_order
            .iter()
            .copied()
            .filter(|p| *p != player)
            .collect()
    }

    let mut adjusted = base_cost.clone();
    let Some(ability_source_object) = game.object(ability_source) else {
        return adjusted;
    };

    let mut cost_modifier_sources = view.activated_ability_cost_modifier_sources();
    if ability_source_object.zone != Zone::Battlefield {
        cost_modifier_sources.push(ability_source);
    }

    for source_id in cost_modifier_sources {
        let Some(perm) = game.object(source_id) else {
            continue;
        };
        let controller = game.controller_of(perm);
        let filter_ctx = FilterContext::new(controller)
            .with_source(source_id)
            .with_active_player(game.turn.active_player)
            .with_opponents(opponents_of(game, controller));

        let static_abilities = if perm.zone == Zone::Battlefield {
            view.static_abilities_rc(source_id).unwrap_or_else(|| {
                Rc::new(
                    perm.abilities
                        .iter()
                        .filter_map(|a| match &a.kind {
                            AbilityKind::Static(sa) => Some(sa.clone()),
                            _ => None,
                        })
                        .collect(),
                )
            })
        } else {
            Rc::new(
                perm.abilities
                    .iter()
                    .filter_map(|a| match &a.kind {
                        AbilityKind::Static(sa) if a.functions_in(&perm.zone) => Some(sa.clone()),
                        _ => None,
                    })
                    .collect(),
            )
        };

        for static_ability in static_abilities.iter() {
            if !static_ability.is_active(game, source_id) {
                continue;
            }

            if let Some(reduction) = static_ability.activated_ability_cost_reduction() {
                if !reduction
                    .filter
                    .matches(ability_source_object, &filter_ctx, game)
                {
                    continue;
                }
                if let Some(condition) = &reduction.condition
                    && !crate::static_abilities::activated_ability_cost_condition_is_active_for_activation(
                        game,
                        ability_source,
                        condition,
                        chosen_targets,
                    )
                {
                    continue;
                }

                let multiplier = if let Some(per_filter) = &reduction.per_matching_objects {
                    game.objects_in_deterministic_order()
                        .into_iter()
                        .filter(|obj| per_filter.matches(obj, &filter_ctx, game))
                        .count() as u32
                } else if let Some(lands_filter) = &reduction.per_basic_land_types_among {
                    count_basic_land_types_among_filter(game, lands_filter, &filter_ctx)
                } else {
                    1
                };
                if multiplier == 0 {
                    continue;
                }

                let before = adjusted.clone();
                adjusted = adjusted.reduce_generic(reduction.reduction.saturating_mul(multiplier));
                if let Some(minimum_total_mana) = reduction.minimum_total_mana
                    && before.mana_value() > 0
                    && adjusted.mana_value() < minimum_total_mana
                {
                    let missing = minimum_total_mana - adjusted.mana_value();
                    adjusted = add_generic_mana_cost(&adjusted, missing);
                }
            }
        }
    }

    apply_payment_reason_mana_adjustments(
        game,
        activator,
        Some(ability_source),
        &adjusted,
        crate::costs::PaymentReason::ActivateAbility,
    )
}

/// Resolve an alternative method index for `CastingMethod::PlayFrom`.
///
/// The index space is:
/// 1) Card intrinsic alternatives (`card.alternative_casts`)
/// 2) Granted alternatives for this card/zone/player (appended after intrinsic methods)
pub fn resolve_play_from_alternative_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    zone: Zone,
    idx: usize,
) -> Option<crate::alternative_cast::AlternativeCastingMethod> {
    if let Some(method) = spell.alternative_casts.get(idx) {
        return Some(method.clone());
    }

    let granted = game
        .effect_store
        .grant_registry
        .granted_alternative_casts_for_card(game, spell.id, zone, player);
    let granted_idx = idx.checked_sub(spell.alternative_casts.len())?;
    granted.get(granted_idx).map(|entry| entry.method.clone())
}

pub(crate) fn alternative_cast_method_matches_kind(
    method: &crate::alternative_cast::AlternativeCastingMethod,
    kind: crate::filter::AlternativeCastKind,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    use crate::filter::AlternativeCastKind;

    match (kind, method) {
        (AlternativeCastKind::Dash, AlternativeCastingMethod::Dash { .. }) => true,
        (AlternativeCastKind::Flashback, AlternativeCastingMethod::Flashback { .. }) => true,
        (AlternativeCastKind::JumpStart, AlternativeCastingMethod::JumpStart) => true,
        (AlternativeCastKind::Escape, AlternativeCastingMethod::Escape { .. }) => true,
        (AlternativeCastKind::Madness, AlternativeCastingMethod::Madness { .. }) => true,
        (AlternativeCastKind::Miracle, AlternativeCastingMethod::Miracle { .. }) => true,
        _ => false,
    }
}

pub(crate) fn casting_method_matches_alternative_kind(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    kind: crate::filter::AlternativeCastKind,
) -> bool {
    match casting_method {
        CastingMethod::Alternative(idx) => spell
            .alternative_casts
            .get(*idx)
            .is_some_and(|method| alternative_cast_method_matches_kind(method, kind)),
        CastingMethod::GrantedEscape { .. } => kind == crate::filter::AlternativeCastKind::Escape,
        CastingMethod::GrantedFlashback => kind == crate::filter::AlternativeCastKind::Flashback,
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => resolve_play_from_alternative_method(game, caster, spell, *zone, *idx)
            .as_ref()
            .is_some_and(|method| alternative_cast_method_matches_kind(method, kind)),
        CastingMethod::Normal
        | CastingMethod::FaceDown
        | CastingMethod::SplitOtherHalf
        | CastingMethod::Fuse
        | CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => false,
    }
}

pub(crate) fn spell_matches_cast_filter(
    game: &GameState,
    spell: &crate::object::Object,
    spell_filter: &crate::target::ObjectFilter,
) -> bool {
    spell_filter.matches(spell, &crate::target::FilterContext::default(), game)
}

pub(crate) fn snapshot_matches_cast_filter(
    game: &GameState,
    snapshot: &crate::snapshot::ObjectSnapshot,
    spell_filter: &crate::target::ObjectFilter,
) -> bool {
    spell_filter.matches_snapshot(snapshot, &crate::target::FilterContext::default(), game)
}

pub(crate) fn spells_cast_this_turn_matching_filter(
    game: &GameState,
    player: PlayerId,
    spell_filter: &crate::target::ObjectFilter,
) -> u32 {
    if spell_filter == &crate::target::ObjectFilter::default() {
        return game.turn_store.turn_history.spells_cast_by_player(player);
    }

    game.turn_store
        .turn_history
        .spell_cast_snapshot_history()
        .iter()
        .filter(|snapshot| {
            snapshot.controller == player
                && snapshot_matches_cast_filter(game, snapshot, spell_filter)
        })
        .count() as u32
}

pub(crate) fn violates_cast_limit(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_filter: &crate::target::ObjectFilter,
) -> bool {
    spell_matches_cast_filter(game, spell, spell_filter)
        && spells_cast_this_turn_matching_filter(game, player, spell_filter) >= 1
}

pub(crate) fn violates_any_cast_limit(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    game.effect_store
        .cant_effects
        .cast_limit_filters_for_player(player)
        .is_some_and(|filters| {
            filters
                .iter()
                .any(|spell_filter| violates_cast_limit(game, player, spell, spell_filter))
        })
}

pub(crate) fn violates_any_cant_cast_restriction(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    game.effect_store
        .cant_effects
        .cast_filters_for_player(player)
        .is_some_and(|filters| {
            filters
                .iter()
                .any(|spell_filter| spell_matches_cast_filter(game, spell, spell_filter))
        })
}

pub(crate) fn is_sorcery_speed_spell(spell: &crate::object::Object) -> bool {
    use crate::types::CardType;

    spell.has_card_type(CardType::Sorcery)
        || spell.has_card_type(CardType::Creature)
        || spell.has_card_type(CardType::Artifact)
        || spell.has_card_type(CardType::Enchantment)
        || spell.has_card_type(CardType::Planeswalker)
}

pub(crate) fn spell_has_active_flash_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    view: &DerivedGameView<'_>,
) -> bool {
    spell.abilities.iter().any(|a| {
        if let crate::ability::AbilityKind::Static(s) = &a.kind {
            if s.has_flash() {
                return true;
            }
            if let Some(spec) = s.conditional_spell_keyword_spec()
                && spec.keyword == crate::static_abilities::ConditionalSpellKeywordKind::Flash
            {
                return crate::static_abilities::conditional_spell_keyword_active(
                    spec, game, player,
                );
            }
        }
        false
    }) || view.card_has_granted_static_ability_id(
        spell_id,
        Zone::Hand,
        player,
        crate::static_abilities::StaticAbilityId::Flash,
    )
}

pub(crate) fn player_was_attacked_this_step(game: &GameState, player: PlayerId) -> bool {
    use crate::combat_state::AttackTarget;
    use crate::game_state::{Phase, Step};

    if !matches!(game.turn.phase, Phase::Combat) || game.turn.step != Some(Step::DeclareAttackers) {
        return false;
    }

    let Some(combat) = game.combat.as_ref() else {
        return false;
    };

    combat
        .attackers
        .iter()
        .any(|attacker| match attacker.target {
            AttackTarget::Player(defender) => defender == player,
            AttackTarget::Planeswalker(planeswalker_id) => game
                .object(planeswalker_id)
                .is_some_and(|planeswalker| game.controller_of(planeswalker) == player),
        })
}

pub(crate) fn this_spell_cast_restriction_allows(
    game: &GameState,
    player: PlayerId,
    kind: &crate::static_abilities::ThisSpellCastRestrictionKind,
) -> bool {
    let timing_allows = kind
        .timing
        .is_none_or(|timing| this_spell_cast_timing_allows(game, player, timing));
    if !timing_allows {
        return false;
    }
    kind.condition
        .as_ref()
        .is_none_or(|condition| this_spell_cast_condition_allows(game, player, condition))
}

pub(crate) fn this_spell_cast_timing_allows(
    game: &GameState,
    player: PlayerId,
    timing: crate::static_abilities::ThisSpellCastTiming,
) -> bool {
    use crate::game_state::{Phase, Step};
    use crate::static_abilities::ThisSpellCastTiming;

    match timing {
        ThisSpellCastTiming::DuringDeclareAttackersStep => {
            matches!(game.turn.phase, Phase::Combat)
                && game.turn.step == Some(Step::DeclareAttackers)
        }
        ThisSpellCastTiming::DuringCombat => matches!(game.turn.phase, Phase::Combat),
        ThisSpellCastTiming::DuringCombatBeforeBlockersAreDeclared => {
            matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::BeginCombat | Step::DeclareAttackers)
                )
        }
        ThisSpellCastTiming::DuringCombatAfterBlockersAreDeclared => {
            matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::DeclareBlockers | Step::CombatDamage | Step::EndCombat)
                )
        }
        ThisSpellCastTiming::DuringCombatOnYourTurnBeforeBlockersAreDeclared => {
            game.turn.active_player == player
                && matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::BeginCombat | Step::DeclareAttackers)
                )
        }
        ThisSpellCastTiming::DuringCombatOnOpponentsTurn => {
            game.turn.active_player != player && matches!(game.turn.phase, Phase::Combat)
        }
        ThisSpellCastTiming::BeforeAttackersAreDeclared => {
            matches!(game.turn.phase, Phase::Combat) && game.turn.step == Some(Step::BeginCombat)
        }
        ThisSpellCastTiming::BeforeCombatDamageStep => {
            matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::BeginCombat | Step::DeclareAttackers | Step::DeclareBlockers)
                )
        }
        ThisSpellCastTiming::DuringOpponentsUpkeep => {
            game.turn.active_player != player
                && matches!(game.turn.phase, Phase::Beginning)
                && game.turn.step == Some(Step::Upkeep)
        }
        ThisSpellCastTiming::DuringOpponentsTurnAfterUpkeep => {
            if game.turn.active_player == player {
                return false;
            }
            !matches!(
                (game.turn.phase, game.turn.step),
                (Phase::Beginning, Some(Step::Untap | Step::Upkeep))
            )
        }
        ThisSpellCastTiming::DuringYourEndStep => {
            game.turn.active_player == player
                && matches!(game.turn.phase, Phase::Ending)
                && game.turn.step == Some(Step::End)
        }
        ThisSpellCastTiming::AfterCombat => {
            matches!(game.turn.phase, Phase::NextMain | Phase::Ending)
        }
    }
}

pub(crate) fn players_matching_cast_restriction_filter(
    game: &GameState,
    player: PlayerId,
    filter: &crate::target::PlayerFilter,
) -> Vec<PlayerId> {
    let filter_ctx = game.filter_context_for(player, None);
    match filter {
        crate::target::PlayerFilter::You => vec![player],
        crate::target::PlayerFilter::Opponent => filter_ctx.opponents.clone(),
        crate::target::PlayerFilter::Teammate => filter_ctx.teammates.clone(),
        crate::target::PlayerFilter::Specific(id) => vec![*id],
        crate::target::PlayerFilter::Any => game
            .players
            .iter()
            .filter(|candidate| candidate.is_in_game())
            .map(|candidate| candidate.id)
            .collect(),
        crate::target::PlayerFilter::NotYou => game
            .players
            .iter()
            .filter_map(|candidate| {
                (candidate.is_in_game() && candidate.id != player).then_some(candidate.id)
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn this_spell_cast_condition_allows(
    game: &GameState,
    player: PlayerId,
    condition: &crate::static_abilities::ThisSpellCastCondition,
) -> bool {
    match condition {
        crate::static_abilities::ThisSpellCastCondition::YouWereAttackedThisStep => {
            player_was_attacked_this_step(game, player)
        }
        crate::static_abilities::ThisSpellCastCondition::PlayerCastSpellThisTurnOrMore {
            player: player_filter,
            spell_filter,
            count,
        } => players_matching_cast_restriction_filter(game, player, player_filter)
            .into_iter()
            .map(|matched_player| {
                spells_cast_this_turn_matching_filter(game, matched_player, spell_filter)
            })
            .sum::<u32>()
            >= *count,
        crate::static_abilities::ThisSpellCastCondition::CreatureIsAttackingYou => {
            player_was_attacked_this_step(game, player)
                || game.combat.as_ref().is_some_and(|combat| {
                    combat.attackers.iter().any(|attacker| match attacker.target {
                        crate::combat_state::AttackTarget::Player(defender) => defender == player,
                        crate::combat_state::AttackTarget::Planeswalker(planeswalker_id) => game
                            .object(planeswalker_id)
                            .is_some_and(|planeswalker| game.controller_of(planeswalker) == player),
                    })
                })
        }
        crate::static_abilities::ThisSpellCastCondition::NoPermanentsNamedOnBattlefield(name) => {
            !game.battlefield.iter().any(|&id| {
                game.object(id)
                    .is_some_and(|object| object.name.eq_ignore_ascii_case(name))
            })
        }
        crate::static_abilities::ThisSpellCastCondition::YouControlAtLeast { filter, count } => {
            let mut required_filter = filter.clone();
            required_filter.zone = Some(Zone::Battlefield);
            let filter_ctx = game.filter_context_for(player, None);
            game.battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|object| {
                    game.controller_of(object) == player
                        && required_filter.matches(object, &filter_ctx, game)
                })
                .count()
                >= *count as usize
        }
        crate::static_abilities::ThisSpellCastCondition::YouControlFewerCreaturesThanEachOpponent => {
            let your_creatures = game.creatures_controlled_by(player).len();
            game.players
                .iter()
                .filter(|opponent| opponent.is_in_game() && opponent.id != player)
                .all(|opponent| your_creatures < game.creatures_controlled_by(opponent.id).len())
        }
        crate::static_abilities::ThisSpellCastCondition::YouControlNameWordOrMore {
            word,
            count,
        } => game
            .permanents_controlled_by(player)
            .iter()
            .filter(|id| {
                game.object(**id).is_some_and(|object| {
                    object
                        .name
                        .to_ascii_lowercase()
                        .contains(&word.to_ascii_lowercase())
                })
            })
            .count()
            >= *count as usize,
    }
}

pub(crate) fn spell_cast_restrictions_allow(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    spell.abilities.iter().all(|ability| {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return true;
        };
        let Some(kind) = static_ability.this_spell_cast_restriction_kind() else {
            return true;
        };
        this_spell_cast_restriction_allows(game, player, &kind)
    })
}

pub(crate) fn has_valid_spell_timing(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: ObjectId,
) -> bool {
    let view = DerivedGameView::new(game);
    has_valid_spell_timing_with_view(game, player, spell, spell_id, &view)
}

pub(crate) fn has_valid_spell_timing_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    view: &DerivedGameView<'_>,
) -> bool {
    if !is_sorcery_speed_spell(spell)
        || spell_has_active_flash_with_view(game, player, spell, spell_id, view)
    {
        return true;
    }

    // Sorcery-speed spells require: active player, main phase, empty stack.
    game.turn.active_player == player && crate::turn::is_sorcery_timing(game)
}

fn casting_method_grants_flash_timing(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    let method = match casting_method {
        CastingMethod::Alternative(idx) => spell.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            zone,
            use_alternative: Some(idx),
            ..
        } => {
            crate::decision::resolve_play_from_alternative_method(game, player, spell, *zone, *idx)
        }
        _ => None,
    };
    matches!(
        method,
        Some(crate::alternative_cast::AlternativeCastingMethod::FlashWithAdditionalCost { .. })
    )
}

fn casting_method_grants_library_search_timing(
    game: &GameState,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    casting_method: &CastingMethod,
) -> bool {
    matches!(
        casting_method,
        CastingMethod::PlayFrom {
            zone: Zone::Library,
            ..
        }
    ) && spell.zone == Zone::Library
        && game.current_has_static_ability_id(
            spell_id,
            crate::static_abilities::StaticAbilityId::CastThisCardFromLibraryWhileSearching,
        )
}

fn casting_method_grants_special_timing(
    ctx: &CastLegalityContext<'_>,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    casting_method: &CastingMethod,
) -> bool {
    casting_method_grants_flash_timing(ctx.game, ctx.player, spell, casting_method)
        || (ctx.allow_library_search_cast_timing
            && casting_method_grants_library_search_timing(
                ctx.game,
                spell,
                spell_id,
                casting_method,
            ))
}

pub(crate) fn face_down_cast_mana_cost() -> crate::mana::ManaCost {
    crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Generic(3)]])
}

pub(crate) fn spell_can_be_cast_face_down(spell: &crate::object::Object) -> bool {
    spell.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            crate::ability::AbilityKind::Static(static_ability)
                if static_ability.turn_face_up_cost().is_some()
        )
    })
}

/// Resolve the mana cost for a spell cast from a specific zone and method.
pub fn spell_mana_cost_for_cast(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    from_zone: Zone,
) -> Option<crate::mana::ManaCost> {
    let base_cost = match casting_method {
        CastingMethod::Normal => spell.mana_cost.clone(),
        CastingMethod::FaceDown => Some(face_down_cast_mana_cost()),
        CastingMethod::SplitOtherHalf => {
            linked_face_definition(game, spell).and_then(|def| def.card.mana_cost)
        }
        CastingMethod::Fuse => {
            spell_view_for_fused_split_cast(game, spell).and_then(|view| view.mana_cost)
        }
        CastingMethod::Alternative(idx) => {
            if let Some(method) = spell.alternative_casts.get(*idx) {
                if matches!(
                    method,
                    crate::alternative_cast::AlternativeCastingMethod::Plot { .. }
                ) {
                    Some(crate::mana::ManaCost::new())
                } else if method.total_cost().is_some() {
                    Some(method.mana_cost().cloned().unwrap_or_default())
                } else {
                    method
                        .mana_cost()
                        .cloned()
                        .or_else(|| spell.mana_cost.clone())
                }
            } else {
                spell.mana_cost.clone()
            }
        }
        CastingMethod::GrantedEscape { .. } => spell.mana_cost.clone(),
        CastingMethod::GrantedFlashback => spell.mana_cost.clone(),
        CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => spell.mana_cost.clone(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => {
            if let Some(method) =
                resolve_play_from_alternative_method(game, player, spell, *zone, *idx)
            {
                if matches!(
                    method,
                    crate::alternative_cast::AlternativeCastingMethod::Plot { .. }
                ) {
                    Some(crate::mana::ManaCost::new())
                } else if method.total_cost().is_some() {
                    Some(method.mana_cost().cloned().unwrap_or_default())
                } else {
                    method
                        .mana_cost()
                        .cloned()
                        .or_else(|| spell.mana_cost.clone())
                }
            } else {
                spell.mana_cost.clone()
            }
        }
    };

    if from_zone == Zone::Command {
        let tax = game.commander_cast_count(spell.id).saturating_mul(2);
        base_cost.map(|cost| cost.add_generic(tax))
    } else {
        base_cost
    }
}

pub(crate) fn alternative_method_uses_printed_mana_cost(
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> bool {
    matches!(
        method,
        crate::alternative_cast::AlternativeCastingMethod::JumpStart
            | crate::alternative_cast::AlternativeCastingMethod::Escape { cost: None, .. }
    )
}

pub(crate) fn casting_method_requires_printed_mana_cost(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    match casting_method {
        CastingMethod::Normal
        | CastingMethod::GrantedEscape { .. }
        | CastingMethod::GrantedFlashback
        | CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => true,
        CastingMethod::Alternative(idx) => spell
            .alternative_casts
            .get(*idx)
            .is_some_and(alternative_method_uses_printed_mana_cost),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => resolve_play_from_alternative_method(game, player, spell, *zone, *idx)
            .as_ref()
            .is_some_and(alternative_method_uses_printed_mana_cost),
        _ => false,
    }
}

/// Check if a spell can be cast by a player using the given casting method.
pub fn can_cast_spell(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    let view = DerivedGameView::new(game);
    can_cast_spell_with_view(game, player, spell, casting_method, &view)
}

/// Check whether a player could begin casting a spell from hand for suspend.
///
/// This enforces cast prohibitions, cast limits, timing, and explicit
/// "cast this spell only ..." restrictions without requiring a printable mana
/// cost or legal targets yet.
pub fn can_begin_to_cast_from_hand_for_suspend(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    if violates_any_cant_cast_restriction(game, player, spell) {
        return false;
    }

    if violates_any_cast_limit(game, player, spell) {
        return false;
    }

    if spell.is_land() {
        return false;
    }

    if !has_valid_spell_timing(game, player, spell, spell.id) {
        return false;
    }

    spell_cast_restrictions_allow(game, player, spell)
}

pub(crate) fn spell_has_legal_targets_for_cast_with_view(
    spell: &crate::object::Object,
    spell_id: ObjectId,
    effects_override: Option<&[crate::effect::Effect]>,
    player: PlayerId,
    view: &DerivedGameView<'_>,
) -> bool {
    let synthesized_aura_effects = if effects_override.is_none()
        && spell.spell_effect.is_none()
        && spell.subtypes.contains(&crate::types::Subtype::Aura)
    {
        spell
            .aura_attach_filter
            .clone()
            .map(|filter| vec![crate::effect::Effect::attach_to(filter.target_spec())])
    } else {
        None
    };
    let effects = effects_override.unwrap_or_else(|| {
        synthesized_aura_effects
            .as_deref()
            .or(spell.spell_effect.as_deref())
            .unwrap_or(&[])
    });
    effects.is_empty() || view.spell_has_legal_targets(effects, player, Some(spell_id), None)
}

pub(crate) fn can_cast_spell_with_context(
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    let total_started_at = PerfTimer::start();
    let game = ctx.game;
    let player = ctx.player;
    let view = ctx.view;
    let split_view = match casting_method {
        CastingMethod::FaceDown => {
            if !spell_can_be_cast_face_down(spell) {
                return false;
            }
            Some(spell_view_for_face_down_cast(spell))
        }
        CastingMethod::SplitOtherHalf => match spell_view_for_split_other_half_cast(game, spell) {
            Some(view) => Some(view),
            None => return false,
        },
        CastingMethod::Fuse => match spell_view_for_fused_split_cast(game, spell) {
            Some(view) => Some(view),
            None => return false,
        },
        _ => None,
    };
    let spell_for_checks = split_view.as_ref().unwrap_or(spell);

    let restrictions_started_at = PerfTimer::start();
    if violates_any_cant_cast_restriction(game, player, spell_for_checks) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    if violates_any_cast_limit(game, player, spell_for_checks) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    if spell_for_checks.is_land() {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    if !spell_cast_restrictions_allow(game, player, spell_for_checks) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());

    let timing_started_at = PerfTimer::start();
    if !has_valid_spell_timing_with_view(game, player, spell_for_checks, spell.id, view)
        && !casting_method_grants_special_timing(ctx, spell_for_checks, spell.id, casting_method)
    {
        ctx.add_timing_ms(timing_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    ctx.add_timing_ms(timing_started_at.elapsed_ms());

    let base_mana_cost = spell_mana_cost_for_cast(game, player, spell, casting_method, spell.zone);
    if base_mana_cost.is_none()
        && casting_method_requires_printed_mana_cost(game, player, spell, casting_method)
    {
        return false;
    }

    let target_started_at = PerfTimer::start();
    let effects = split_view
        .as_ref()
        .and_then(|view| view.spell_effect.as_deref())
        .or(spell.spell_effect.as_deref());
    let has_legal_targets = spell_has_legal_targets_for_cast_with_view(
        spell_for_checks,
        spell.id,
        effects,
        player,
        view,
    );
    ctx.add_target_legality_ms(target_started_at.elapsed_ms());
    if !has_legal_targets {
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }

    if let Some(base_cost) = base_mana_cost.as_ref() {
        let cost_started_at = PerfTimer::start();
        let effective_cost = if ctx
            .can_use_printed_cost_directly(spell_has_intrinsic_cost_adjustments(spell_for_checks))
        {
            base_cost.clone()
        } else if ctx
            .spell_cost_needs_adjustment(spell_has_intrinsic_cost_adjustments(spell_for_checks))
        {
            calculate_effective_mana_cost_with_view_for_casting_method(
                game,
                player,
                spell_for_checks,
                base_cost,
                casting_method,
                view,
            )
        } else {
            apply_minimum_spell_total_mana(
                game,
                &apply_payment_reason_mana_adjustments(
                    game,
                    player,
                    Some(spell.id),
                    base_cost,
                    crate::costs::PaymentReason::CastSpell,
                ),
            )
        };
        ctx.add_cost_adjustment_ms(cost_started_at.elapsed_ms());

        let affordability_started_at = PerfTimer::start();
        let potential = view.potential_mana(player);
        let allow_any_color = game.can_spend_mana_as_any_color(player, Some(spell.id));
        let allow_black_life = view.player_can_pay_black_with_life_for_reason(
            player,
            crate::costs::PaymentReason::CastSpell,
        );
        if mana_cost_is_obviously_unpayable(
            &potential,
            &effective_cost,
            allow_any_color,
            allow_black_life,
        ) {
            ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
            ctx.add_total_ms(total_started_at.elapsed_ms());
            return false;
        }
        if !can_pay_mana_cost_with_available_sources(
            game,
            player,
            Some(spell.id),
            &effective_cost,
            0,
            crate::costs::PaymentReason::CastSpell,
            allow_any_color,
            allow_black_life,
            view,
        ) {
            ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
            ctx.add_total_ms(total_started_at.elapsed_ms());
            return false;
        }
        ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
    }

    ctx.add_total_ms(total_started_at.elapsed_ms());
    true
}

pub(crate) fn can_cast_spell_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_spell_with_context(spell, casting_method, &ctx)
}

// ============================================================================
// Unified Spell Casting Validation
// ============================================================================

/// Additional requirements for casting a spell beyond mana.
#[derive(Debug, Clone, Default)]
pub struct AdditionalCastRequirements {
    /// Cards that must be exiled from graveyard (excluding the spell itself).
    pub exile_from_graveyard: u32,
    /// Cards that must be discarded from hand.
    pub discard_from_hand: u32,
    /// A TotalCost that must be paid (for alternative costs like Force of Will).
    /// This is checked with spell exclusion (the spell being cast is excluded from hand).
    pub total_cost: Option<crate::cost::TotalCost>,
    /// If true, spell must be instant or sorcery only.
    pub must_be_instant_or_sorcery: bool,
}

pub(crate) fn can_cast_with_cost_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    mana_cost: Option<&crate::mana::ManaCost>,
    effects_override: Option<&[crate::effect::Effect]>,
    requirements: &AdditionalCastRequirements,
    view: &DerivedGameView<'_>,
) -> bool {
    can_cast_with_cost_with_view_for_casting_method(
        game,
        player,
        spell,
        spell_id,
        mana_cost,
        effects_override,
        requirements,
        &CastingMethod::Normal,
        view,
    )
}

pub(crate) fn can_cast_with_cost_with_view_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    mana_cost: Option<&crate::mana::ManaCost>,
    effects_override: Option<&[crate::effect::Effect]>,
    requirements: &AdditionalCastRequirements,
    casting_method: &CastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_with_cost_with_context(
        spell,
        spell_id,
        mana_cost,
        effects_override,
        requirements,
        casting_method,
        &ctx,
    )
}

pub(crate) fn can_cast_with_cost_with_context(
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    mana_cost: Option<&crate::mana::ManaCost>,
    effects_override: Option<&[crate::effect::Effect]>,
    requirements: &AdditionalCastRequirements,
    casting_method: &CastingMethod,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    use crate::types::CardType;
    let game = ctx.game;
    let player = ctx.player;
    let view = ctx.view;

    let restrictions_started_at = PerfTimer::start();
    if violates_any_cant_cast_restriction(game, player, spell) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if violates_any_cast_limit(game, player, spell) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if spell.is_land() {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if requirements.must_be_instant_or_sorcery
        && !spell.has_card_type(CardType::Instant)
        && !spell.has_card_type(CardType::Sorcery)
    {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if !spell_cast_restrictions_allow(game, player, spell) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());

    let timing_started_at = PerfTimer::start();
    if !has_valid_spell_timing_with_view(game, player, spell, spell_id, view)
        && !casting_method_grants_special_timing(ctx, spell, spell_id, casting_method)
    {
        ctx.add_timing_ms(timing_started_at.elapsed_ms());
        return false;
    }
    ctx.add_timing_ms(timing_started_at.elapsed_ms());

    let target_started_at = PerfTimer::start();
    let has_legal_targets =
        spell_has_legal_targets_for_cast_with_view(spell, spell_id, effects_override, player, view);
    ctx.add_target_legality_ms(target_started_at.elapsed_ms());
    if !has_legal_targets {
        return false;
    }

    let Some(player_obj) = game.player(player) else {
        return false;
    };

    // Check exile from graveyard requirement
    if requirements.exile_from_graveyard > 0 {
        let other_cards_in_graveyard = player_obj
            .graveyard
            .iter()
            .filter(|&&id| id != spell_id)
            .count();
        if other_cards_in_graveyard < requirements.exile_from_graveyard as usize {
            return false;
        }
    }

    // Check discard from hand requirement
    // For Jump-Start, need at least discard_from_hand cards in hand
    if requirements.discard_from_hand > 0
        && (player_obj.hand.len() as u32) < requirements.discard_from_hand
    {
        return false;
    }

    // Check TotalCost requirement (for Force of Will style costs)
    if let Some(ref total_cost) = requirements.total_cost {
        for individual_cost in total_cost.costs() {
            if !can_pay_cost_with_spell_exclusion(game, player, individual_cost, Some(spell_id)) {
                return false;
            }
        }
    }

    if let Some(cost) = mana_cost {
        let cost_started_at = PerfTimer::start();
        let adjusted =
            if ctx.can_use_printed_cost_directly(spell_has_intrinsic_cost_adjustments(spell)) {
                cost.clone()
            } else if ctx.spell_cost_needs_adjustment(spell_has_intrinsic_cost_adjustments(spell)) {
                calculate_effective_mana_cost_with_view_for_casting_method(
                    game,
                    player,
                    spell,
                    cost,
                    casting_method,
                    view,
                )
            } else {
                apply_minimum_spell_total_mana(
                    game,
                    &apply_payment_reason_mana_adjustments(
                        game,
                        player,
                        Some(spell_id),
                        cost,
                        crate::costs::PaymentReason::CastSpell,
                    ),
                )
            };
        ctx.add_cost_adjustment_ms(cost_started_at.elapsed_ms());

        let affordability_started_at = PerfTimer::start();
        let potential = view.potential_mana(player);
        let allow_any_color = game.can_spend_mana_as_any_color(player, Some(spell_id));
        let allow_black_life = view.player_can_pay_black_with_life_for_reason(
            player,
            crate::costs::PaymentReason::CastSpell,
        );
        if mana_cost_is_obviously_unpayable(
            &potential,
            &adjusted,
            allow_any_color,
            allow_black_life,
        ) {
            ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
            return false;
        }
        if !can_pay_mana_cost_with_available_sources(
            game,
            player,
            Some(spell_id),
            &adjusted,
            0,
            crate::costs::PaymentReason::CastSpell,
            allow_any_color,
            allow_black_life,
            view,
        ) {
            ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
            return false;
        }
        ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
    }

    true
}

pub(crate) fn provisional_casting_method_for_alternative(
    spell: &crate::object::Object,
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> CastingMethod {
    if let Some(index) = spell
        .alternative_casts
        .iter()
        .position(|candidate| candidate == method)
    {
        return CastingMethod::Alternative(index);
    }

    match method {
        crate::alternative_cast::AlternativeCastingMethod::Escape { exile_count, .. } => {
            CastingMethod::GrantedEscape {
                source: spell.id,
                exile_count: *exile_count,
            }
        }
        crate::alternative_cast::AlternativeCastingMethod::Flashback { .. } => {
            CastingMethod::GrantedFlashback
        }
        _ => CastingMethod::Normal,
    }
}

/// Build additional cast requirements from an alternative casting method.
pub(crate) fn build_requirements_for_method(
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> AdditionalCastRequirements {
    let method_requirements = method.requirements();
    AdditionalCastRequirements {
        exile_from_graveyard: method_requirements.exile_from_graveyard,
        discard_from_hand: method_requirements.discard_from_hand,
        ..Default::default()
    }
}

/// Get the mana cost for an alternative casting method.
pub(crate) fn get_mana_cost_for_method<'a>(
    method: &'a crate::alternative_cast::AlternativeCastingMethod,
    spell: &'a crate::object::Object,
) -> Option<&'a crate::mana::ManaCost> {
    // Composed costs can intentionally represent "without paying its mana cost"
    // by omitting a mana component, so do not fall back to the card's printed cost.
    if method.is_composed_cost() {
        return method.mana_cost();
    }

    // Method's cost takes priority, fallback to spell's cost for methods that
    // explicitly say they reuse the spell's normal mana cost.
    method.mana_cost().or(spell.mana_cost.as_ref())
}

pub(crate) fn spell_view_for_disturb_cast(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::object::Object> {
    let other_def = game
        .linked_face_definition_by_name_or_id(spell.other_face_name.as_deref(), spell.other_face)?;
    let mut view = spell.clone();
    view.apply_definition_face(&other_def);
    view.ensure_aura_cast_spell_effect();
    Some(view)
}

pub(crate) fn spell_view_for_face_down_cast(
    spell: &crate::object::Object,
) -> crate::object::Object {
    let mut view = spell.clone();
    view.apply_face_down_cast_overlay();
    view
}

pub(crate) fn linked_face_definition(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::cards::CardDefinition> {
    game.linked_face_definition_by_name_or_id(spell.other_face_name.as_deref(), spell.other_face)
}

pub(crate) fn spell_view_for_split_other_half_cast(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::object::Object> {
    if spell.linked_face_layout != crate::card::LinkedFaceLayout::Split {
        return None;
    }
    let other_def = linked_face_definition(game, spell)?;
    let mut view = spell.clone();
    view.apply_definition_face(&other_def);
    view.ensure_aura_cast_spell_effect();
    Some(view)
}

pub(crate) fn spell_view_for_fused_split_cast(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::object::Object> {
    if spell.linked_face_layout != crate::card::LinkedFaceLayout::Split || !spell.has_fuse {
        return None;
    }
    let other_def = linked_face_definition(game, spell)?;
    let mut view = spell.clone();
    view.apply_fused_split_spell_overlay(&other_def);
    Some(view)
}

pub(crate) fn can_cast_with_alternative_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_with_alternative_with_context(spell, method, &ctx)
}

pub(crate) fn can_cast_with_alternative_with_context(
    spell: &crate::object::Object,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    let game = ctx.game;
    let player = ctx.player;

    let disturbed_view = match method {
        AlternativeCastingMethod::Disturb { .. } => {
            match spell_view_for_disturb_cast(game, spell) {
                Some(view) => Some(view),
                None => return false,
            }
        }
        _ => None,
    };
    let spell_for_checks = disturbed_view.as_ref().unwrap_or(spell);
    let effects_override = method.overload_effects().or_else(|| {
        disturbed_view
            .as_ref()
            .and_then(|view| view.spell_effect.as_deref())
    });
    let free_plot_cost = crate::mana::ManaCost::new();
    let mana_cost = match method {
        AlternativeCastingMethod::Foretell { .. } => {
            if !game.is_foretold(spell.id) {
                return false;
            }
            get_mana_cost_for_method(method, spell_for_checks)
        }
        AlternativeCastingMethod::Plot { .. } => {
            if !game.is_plotted_by(spell.id, player) {
                return false;
            }
            let Some(plotted_turn) = game.plotted_turn(spell.id) else {
                return false;
            };
            if plotted_turn >= game.turn.turn_number {
                return false;
            }
            if game.turn.active_player != player || !crate::turn::is_sorcery_timing(game) {
                return false;
            }
            Some(&free_plot_cost)
        }
        AlternativeCastingMethod::Suspend { .. } => return false,
        _ => get_mana_cost_for_method(method, spell_for_checks),
    };
    if mana_cost.is_none() && alternative_method_uses_printed_mana_cost(method) {
        return false;
    }

    let requirements = build_requirements_for_method(method);
    let casting_method = provisional_casting_method_for_alternative(spell, method);
    if !can_cast_with_cost_with_context(
        spell_for_checks,
        spell.id,
        mana_cost,
        effects_override,
        &requirements,
        &casting_method,
        ctx,
    ) {
        return false;
    }

    let check_ctx = crate::costs::CostCheckContext::new(spell.id, player)
        .with_reason(crate::costs::PaymentReason::CastSpell);
    for cost in method.non_mana_costs() {
        if game
            .validate_cost_for_payment_reason(player, spell.id, &cost, check_ctx.reason)
            .is_err()
        {
            return false;
        }
        if crate::costs::can_pay_with_check_context(&*cost.0, game, &check_ctx).is_err() {
            return false;
        }
    }

    true
}

/// Check if a spell can be cast with an alternative cost from hand (e.g., Force of Will).
pub fn can_cast_with_alternative_from_hand(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> bool {
    let view = DerivedGameView::new(game);
    can_cast_with_alternative_from_hand_with_view(game, player, spell, spell_id, method, &view)
}

pub(crate) fn can_cast_with_alternative_from_hand_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_with_alternative_from_hand_with_context(spell, spell_id, method, &ctx)
}

pub(crate) fn can_cast_with_alternative_from_hand_with_context(
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    let game = ctx.game;
    let player = ctx.player;

    match method {
        method if method.is_composed_cost() => {
            let zero_cost = crate::mana::ManaCost::new();
            let casting_method = provisional_casting_method_for_alternative(spell, method);
            if let Some(condition) = method.cast_condition()
                && !crate::static_abilities::this_spell_cost_condition_is_active_for_cast(
                    game,
                    spell_id,
                    condition,
                    &[],
                )
            {
                return false;
            }

            if !can_cast_with_cost_with_context(
                spell,
                spell_id,
                method.mana_cost().or(Some(&zero_cost)),
                None,
                &AdditionalCastRequirements::default(),
                &casting_method,
                ctx,
            ) {
                return false;
            }

            let check_ctx = crate::costs::CostCheckContext::new(spell_id, player)
                .with_reason(crate::costs::PaymentReason::CastSpell);
            for cost in method.non_mana_costs() {
                if game
                    .validate_cost_for_payment_reason(player, spell_id, &cost, check_ctx.reason)
                    .is_err()
                {
                    return false;
                }
                if crate::costs::can_pay_with_check_context(&*cost.0, game, &check_ctx).is_err() {
                    return false;
                }
            }
            true
        }
        AlternativeCastingMethod::Bestow { total_cost } => {
            let Some(cost) = total_cost.mana_cost() else {
                return false;
            };
            let casting_method = provisional_casting_method_for_alternative(spell, method);

            if !can_cast_with_cost_with_context(
                spell,
                spell_id,
                Some(cost),
                None,
                &AdditionalCastRequirements::default(),
                &casting_method,
                ctx,
            ) {
                return false;
            }

            let check_ctx = crate::costs::CostCheckContext::new(spell_id, player)
                .with_reason(crate::costs::PaymentReason::CastSpell);
            for cost in method.non_mana_costs() {
                if game
                    .validate_cost_for_payment_reason(player, spell_id, &cost, check_ctx.reason)
                    .is_err()
                {
                    return false;
                }
                if crate::costs::can_pay_with_check_context(&*cost.0, game, &check_ctx).is_err() {
                    return false;
                }
            }

            let bestow_spec = ChooseSpec::Object(crate::target::ObjectFilter::creature());
            let bestow_targets =
                crate::targeting::compute_legal_targets_with_tagged_objects_with_view(
                    game,
                    &bestow_spec,
                    player,
                    Some(spell_id),
                    None,
                    ctx.view,
                );
            !bestow_targets.is_empty()
        }
        AlternativeCastingMethod::Trap {
            cost, condition, ..
        } => {
            // Check if the trap condition is met
            if !is_trap_condition_met(game, player, condition) {
                return false;
            }
            // Check if player can pay the trap cost (usually {0})
            let casting_method = provisional_casting_method_for_alternative(spell, method);
            can_cast_with_cost_with_context(
                spell,
                spell_id,
                Some(cost),
                None,
                &AdditionalCastRequirements::default(),
                &casting_method,
                ctx,
            )
        }
        _ => can_cast_with_alternative_with_context(spell, method, ctx),
    }
}

/// Check if a trap condition is met for the given player.
pub(crate) fn is_trap_condition_met(
    game: &GameState,
    player: PlayerId,
    condition: &crate::alternative_cast::TrapCondition,
) -> bool {
    use crate::alternative_cast::TrapCondition;

    // Get all opponents
    let opponents: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|p| p.id != player && p.is_in_game())
        .map(|p| p.id)
        .collect();

    match condition {
        TrapCondition::OpponentCastSpells { count } => {
            // Check if any opponent cast N or more spells this turn
            opponents
                .iter()
                .any(|&opp| game.turn_store.turn_history.spells_cast_by_player(opp) >= *count)
        }
        TrapCondition::OpponentSearchedLibrary => {
            // Check if any opponent searched their library this turn
            opponents.iter().any(|opp| {
                game.turn_store
                    .turn_history
                    .player_searched_library_this_turn(*opp)
            })
        }
        TrapCondition::OpponentCreatureEntered => {
            // Check if any opponent had a creature enter the battlefield this turn
            opponents.iter().any(|&opp| {
                game.turn_store
                    .turn_history
                    .player_had_creature_enter_battlefield_this_turn(opp)
            })
        }
        TrapCondition::CreatureDealtDamageToYou => {
            // Check if any creature dealt damage to the player this turn
            game.turn_store
                .turn_history
                .player_was_dealt_damage_by_creature_this_turn(player)
        }
    }
}

/// Check if a player can pay a specific cost, excluding a specific card from hand (the spell being cast).
pub(crate) fn can_pay_cost_with_spell_exclusion(
    game: &GameState,
    player: PlayerId,
    cost: &crate::costs::Cost,
    spell_to_exclude: Option<crate::ids::ObjectId>,
) -> bool {
    use crate::costs::CostProcessingMode;

    let source = spell_to_exclude.or_else(|| {
        game.player(player).and_then(|p| {
            p.hand
                .first()
                .copied()
                .or_else(|| p.graveyard.first().copied())
        })
    });
    let Some(source) = source else {
        return false;
    };

    let mut dm = crate::decision::CliDecisionMaker;
    let ctx = crate::costs::CostContext::new(source, player, &mut dm)
        .with_reason(crate::costs::PaymentReason::CastSpell);
    if game
        .validate_cost_for_payment_reason(player, source, cost, ctx.reason)
        .is_err()
    {
        return false;
    }

    match cost.processing_mode() {
        CostProcessingMode::ManaPayment { .. } => cost.can_potentially_pay(game, &ctx).is_ok(),
        CostProcessingMode::Immediate
        | CostProcessingMode::InlineWithTriggers
        | CostProcessingMode::SacrificeTarget { .. }
        | CostProcessingMode::DiscardCards { .. }
        | CostProcessingMode::ExileFromHand { .. }
        | CostProcessingMode::ExileFromGraveyard { .. }
        | CostProcessingMode::RevealFromHand { .. }
        | CostProcessingMode::ReturnToHandTarget { .. } => cost.can_pay(game, &ctx).is_ok(),
    }
}

pub(crate) fn apply_payment_reason_mana_adjustments(
    game: &GameState,
    payer: PlayerId,
    source: Option<ObjectId>,
    cost: &crate::mana::ManaCost,
    reason: crate::costs::PaymentReason,
) -> crate::mana::ManaCost {
    game.adjust_mana_cost_for_payment_reason(payer, source, cost, reason)
}

pub(crate) fn apply_minimum_spell_total_mana(
    game: &GameState,
    cost: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    if let Some(minimum) = game.minimum_total_spell_mana_payment()
        && cost.mana_value() < minimum
    {
        return cost.add_generic(minimum - cost.mana_value());
    }

    cost.clone()
}

// ============================================================================
// Cost Modifier Helpers (Tier 9)
// ============================================================================

/// Calculate the effective mana cost after applying cost reduction abilities.
///
/// This handles abilities like:
/// - Affinity for artifacts: Reduce generic cost by 1 for each artifact you control
/// - Delve: Reduce generic cost by 1 for each card exiled from graveyard (automatic maximum)
/// - Convoke: Tap creatures to pay for mana (colored or generic)
///
/// Returns the reduced mana cost.
pub fn calculate_effective_mana_cost(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_for_casting_method(
        game,
        player,
        spell,
        base_cost,
        &CastingMethod::Normal,
    )
}

pub fn calculate_effective_mana_cost_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    casting_method: &CastingMethod,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        1,
        &[],
        true,
        casting_method,
        &view,
    )
}

pub(crate) fn calculate_effective_mana_cost_with_view_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    casting_method: &CastingMethod,
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        1,
        &[],
        true,
        casting_method,
        view,
    )
}

/// Calculate the effective mana cost with explicit chosen target count.
pub fn calculate_effective_mana_cost_with_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_target_count,
        &[],
        true,
        &CastingMethod::Normal,
        &view,
    )
}

/// Calculate the effective mana cost using the exact chosen targets.
pub fn calculate_effective_mana_cost_with_chosen_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_with_chosen_targets_for_casting_method(
        game,
        player,
        spell,
        base_cost,
        chosen_targets,
        &CastingMethod::Normal,
    )
}

pub fn calculate_effective_mana_cost_with_chosen_targets_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
    casting_method: &CastingMethod,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_targets.len(),
        chosen_targets,
        true,
        casting_method,
        &view,
    )
}

/// Calculate effective cost for payment stage where Convoke/Improvise are handled
/// as pip alternatives instead of up-front reductions.
pub fn calculate_effective_mana_cost_for_payment_with_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_for_payment_with_targets_for_casting_method(
        game,
        player,
        spell,
        base_cost,
        chosen_target_count,
        &CastingMethod::Normal,
    )
}

pub fn calculate_effective_mana_cost_for_payment_with_targets_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    casting_method: &CastingMethod,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_target_count,
        &[],
        false,
        casting_method,
        &view,
    )
}

/// Calculate payment-stage effective cost using exact chosen targets.
pub fn calculate_effective_mana_cost_for_payment_with_chosen_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_for_payment_with_chosen_targets_for_casting_method(
        game,
        player,
        spell,
        base_cost,
        chosen_targets,
        &CastingMethod::Normal,
    )
}

pub fn calculate_effective_mana_cost_for_payment_with_chosen_targets_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
    casting_method: &CastingMethod,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_targets.len(),
        chosen_targets,
        false,
        casting_method,
        &view,
    )
}

pub(crate) fn calculate_effective_mana_cost_with_targets_internal(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    chosen_targets: &[Target],
    include_convoke_improvise_reductions: bool,
    casting_method: &CastingMethod,
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;

    let mut current_cost = base_cost.clone();

    // Check for Affinity for artifacts
    let has_affinity = spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_affinity()
        } else {
            false
        }
    });

    if has_affinity {
        // Count artifacts controlled by the player
        let artifact_count = count_artifacts_controlled_with_view(game, player, view);
        current_cost = current_cost.reduce_generic(artifact_count);
    }

    // Apply explicit cost reductions/increases on the spell itself.
    current_cost = apply_spell_cost_modifiers(
        game,
        player,
        spell,
        &current_cost,
        chosen_target_count,
        chosen_targets,
        casting_method,
    );

    // Apply global cost modifiers from battlefield permanents (Sphere of Resistance, leeches, etc.).
    current_cost = apply_battlefield_spell_cost_modifiers(
        game,
        player,
        spell,
        &current_cost,
        chosen_target_count,
        casting_method,
        view,
    );

    // Check for Delve
    let has_delve_ability = has_delve(spell);

    if has_delve_ability {
        // For Delve, we assume maximum usage (exile all cards up to generic cost remaining)
        let graveyard_count = count_cards_in_graveyard(game, player);
        current_cost = current_cost.reduce_generic(graveyard_count);
    }

    if include_convoke_improvise_reductions {
        // Check for Convoke
        let has_convoke_ability = has_convoke(spell);
        if has_convoke_ability {
            // For Convoke, calculate the optimal creature tapping
            let (_, convoked_cost) = calculate_convoke_cost(game, player, &current_cost);
            current_cost = convoked_cost;
        }

        // Check for Improvise
        let has_improvise_ability = has_improvise(spell);
        if has_improvise_ability {
            // For Improvise, calculate the optimal artifact tapping
            let (_, improvised_cost) = calculate_improvise_cost(game, player, &current_cost);
            current_cost = improvised_cost;
        }
    }

    let current_cost = apply_payment_reason_mana_adjustments(
        game,
        player,
        Some(spell.id),
        &current_cost,
        crate::costs::PaymentReason::CastSpell,
    );

    apply_minimum_spell_total_mana(game, &current_cost)
}

pub(crate) fn apply_spell_cost_modifiers(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    chosen_targets: &[Target],
    casting_method: &CastingMethod,
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;
    use crate::filter::FilterContext;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::target::ObjectFilter;

    fn opponents_of(game: &GameState, player: PlayerId) -> Vec<PlayerId> {
        game.turn_store
            .turn_order
            .iter()
            .copied()
            .filter(|p| *p != player)
            .collect()
    }

    fn spell_matches_filter(
        game: &GameState,
        spell: &crate::object::Object,
        caster: PlayerId,
        filter: &ObjectFilter,
        ctx: &FilterContext,
        casting_method: &CastingMethod,
    ) -> bool {
        if filter.targets_object.is_some() || filter.targets_player.is_some() {
            // Target-dependent cost modifiers require target selection context.
            return false;
        }
        let mut cast_filter = filter.clone();
        let alternative_cast = cast_filter.alternative_cast;
        cast_filter.targets_player = None;
        cast_filter.targets_object = None;
        cast_filter.alternative_cast = None;
        cast_filter.matches(spell, &ctx.clone().with_caster(Some(caster)), game)
            && alternative_cast.is_none_or(|kind| {
                casting_method_matches_alternative_kind(game, caster, spell, casting_method, kind)
            })
    }

    let mut total_increase: i32 = 0;
    let mut total_reduction: i32 = 0;
    let mut increase_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut reduction_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let ctx = FilterContext::new(player)
        .with_source(spell.id)
        .with_active_player(game.turn.active_player)
        .with_opponents(opponents_of(game, player));

    for ability in &spell.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let functions_in_current_zone = ability.functions_in(&spell.zone);
        if let Some(reduction) = static_ability.this_spell_cost_reduction() {
            if crate::static_abilities::this_spell_cost_condition_is_active_for_cast(
                game,
                spell.id,
                &reduction.condition,
                chosen_targets,
            ) {
                let amount =
                    resolve_this_spell_cost_reduction_value(game, player, spell, reduction);
                if amount > 0 {
                    total_reduction = total_reduction.saturating_add(amount);
                }
            }
        }
        if let Some(reduction) = static_ability.this_spell_cost_reduction_mana_cost() {
            if crate::static_abilities::this_spell_cost_condition_is_active_for_cast(
                game,
                spell.id,
                &reduction.condition,
                chosen_targets,
            ) {
                reduction_pips.extend(reduction.reduction.pips().iter().cloned());
            }
        }
        if !functions_in_current_zone {
            continue;
        }
        if !static_ability.is_active(game, spell.id) {
            continue;
        }
        if let Some(reduction) = static_ability.cost_reduction()
            && spell_matches_filter(game, spell, player, &reduction.filter, &ctx, casting_method)
        {
            let amount = resolve_cost_modifier_value(game, player, spell, &reduction.reduction);
            if amount > 0 {
                total_reduction = total_reduction.saturating_add(amount);
            }
        }
        if let Some(increase) = static_ability.cost_increase()
            && spell_matches_filter(game, spell, player, &increase.filter, &ctx, casting_method)
        {
            let amount = resolve_cost_modifier_value(game, player, spell, &increase.increase);
            if amount > 0 {
                total_increase = total_increase.saturating_add(amount);
            }
        }
        if let Some(increase) = static_ability.cost_increase_mana_cost()
            && spell_matches_filter(game, spell, player, &increase.filter, &ctx, casting_method)
        {
            increase_pips.extend(increase.increase.pips().iter().cloned());
        }
        if let Some(reduction) = static_ability.cost_reduction_mana_cost()
            && spell_matches_filter(game, spell, player, &reduction.filter, &ctx, casting_method)
        {
            reduction_pips.extend(reduction.reduction.pips().iter().cloned());
        }
        if let Some(per_target_amount) = static_ability.cost_increase_per_additional_target() {
            let additional_targets = chosen_target_count.saturating_sub(1);
            if additional_targets > 0 {
                let extra = (per_target_amount as i32).saturating_mul(additional_targets as i32);
                total_increase = total_increase.saturating_add(extra);
            }
        }
        if let Some(per_target_cost) =
            static_ability.cost_increase_mana_cost_per_additional_target()
        {
            let additional_targets = chosen_target_count.saturating_sub(1);
            for _ in 0..additional_targets {
                increase_pips.extend(per_target_cost.pips().iter().cloned());
            }
        }
    }

    let current_turn = game.turn.turn_number;
    for effect in &game.effect_store.temporary_spell_cost_reductions {
        if effect.player != player || effect.is_expired(current_turn) {
            continue;
        }
        if spell_matches_filter(game, spell, player, &effect.filter, &ctx, casting_method) {
            reduction_pips.extend(effect.reduction.pips().iter().cloned());
        }
    }

    let mut adjusted = cost.clone();
    if !increase_pips.is_empty() {
        adjusted = add_mana_cost(&adjusted, &ManaCost::from_pips(increase_pips));
    }
    if total_increase > 0 {
        adjusted = add_generic_mana_cost(&adjusted, total_increase as u32);
    }
    if total_reduction > 0 {
        adjusted = adjusted.reduce_generic(total_reduction as u32);
    }
    if !reduction_pips.is_empty() {
        adjusted = reduce_mana_cost(&adjusted, &ManaCost::from_pips(reduction_pips));
    }
    adjusted
}

pub(crate) fn apply_battlefield_spell_cost_modifiers(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    casting_method: &CastingMethod,
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;
    use crate::filter::FilterContext;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::target::ObjectFilter;

    fn opponents_of(game: &GameState, player: PlayerId) -> Vec<PlayerId> {
        game.turn_store
            .turn_order
            .iter()
            .copied()
            .filter(|p| *p != player)
            .collect()
    }

    fn spell_matches_filter(
        game: &GameState,
        spell: &crate::object::Object,
        caster: PlayerId,
        filter: &ObjectFilter,
        ctx: &FilterContext,
        casting_method: &CastingMethod,
        _chosen_target_count: usize,
    ) -> bool {
        if filter.targets_object.is_some() || filter.targets_player.is_some() {
            // Target-dependent cost modifiers require target selection context.
            return false;
        }
        let mut cast_filter = filter.clone();
        let alternative_cast = cast_filter.alternative_cast;
        cast_filter.targets_player = None;
        cast_filter.targets_object = None;
        cast_filter.alternative_cast = None;
        cast_filter.matches(spell, &ctx.clone().with_caster(Some(caster)), game)
            && alternative_cast.is_none_or(|kind| {
                casting_method_matches_alternative_kind(game, caster, spell, casting_method, kind)
            })
    }

    let mut total_increase: i32 = 0;
    let mut total_reduction: i32 = 0;
    let mut increase_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut reduction_pips: Vec<Vec<ManaSymbol>> = Vec::new();

    for perm_id in view.battlefield_spell_cost_modifier_sources() {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        let controller = game.controller_of(perm);
        let ctx = FilterContext::new(controller)
            .with_source(perm_id)
            .with_active_player(game.turn.active_player)
            .with_opponents(opponents_of(game, controller));

        if let Some(static_abilities) = view.static_abilities_rc(perm_id) {
            for static_ability in static_abilities.iter() {
                if let Some(reduction) = static_ability.cost_reduction()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        casting_method,
                        chosen_target_count,
                    )
                {
                    let amount = resolve_cost_modifier_value_for_source(
                        game,
                        perm_id,
                        controller,
                        &reduction.reduction,
                    );
                    if amount > 0 {
                        total_reduction = total_reduction.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        casting_method,
                        chosen_target_count,
                    )
                {
                    let amount = resolve_cost_modifier_value_for_source(
                        game,
                        perm_id,
                        controller,
                        &increase.increase,
                    );
                    if amount > 0 {
                        total_increase = total_increase.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        casting_method,
                        chosen_target_count,
                    )
                {
                    increase_pips.extend(increase.increase.pips().iter().cloned());
                }
                if let Some(reduction) = static_ability.cost_reduction_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        casting_method,
                        chosen_target_count,
                    )
                {
                    reduction_pips.extend(reduction.reduction.pips().iter().cloned());
                }
                if let Some(per_target_amount) =
                    static_ability.cost_increase_per_additional_target()
                {
                    let additional_targets = chosen_target_count.saturating_sub(1);
                    if additional_targets > 0 {
                        let extra =
                            (per_target_amount as i32).saturating_mul(additional_targets as i32);
                        total_increase = total_increase.saturating_add(extra);
                    }
                }
                if let Some(per_target_cost) =
                    static_ability.cost_increase_mana_cost_per_additional_target()
                {
                    let additional_targets = chosen_target_count.saturating_sub(1);
                    for _ in 0..additional_targets {
                        increase_pips.extend(per_target_cost.pips().iter().cloned());
                    }
                }
            }
        } else {
            for static_ability in perm
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    AbilityKind::Static(static_ability) => Some(static_ability),
                    _ => None,
                })
            {
                if let Some(reduction) = static_ability.cost_reduction()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        casting_method,
                        chosen_target_count,
                    )
                {
                    let amount = resolve_cost_modifier_value_for_source(
                        game,
                        perm_id,
                        controller,
                        &reduction.reduction,
                    );
                    if amount > 0 {
                        total_reduction = total_reduction.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        casting_method,
                        chosen_target_count,
                    )
                {
                    let amount = resolve_cost_modifier_value_for_source(
                        game,
                        perm_id,
                        controller,
                        &increase.increase,
                    );
                    if amount > 0 {
                        total_increase = total_increase.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        casting_method,
                        chosen_target_count,
                    )
                {
                    increase_pips.extend(increase.increase.pips().iter().cloned());
                }
                if let Some(reduction) = static_ability.cost_reduction_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        casting_method,
                        chosen_target_count,
                    )
                {
                    reduction_pips.extend(reduction.reduction.pips().iter().cloned());
                }
                if let Some(per_target_amount) =
                    static_ability.cost_increase_per_additional_target()
                {
                    let additional_targets = chosen_target_count.saturating_sub(1);
                    if additional_targets > 0 {
                        let extra =
                            (per_target_amount as i32).saturating_mul(additional_targets as i32);
                        total_increase = total_increase.saturating_add(extra);
                    }
                }
                if let Some(per_target_cost) =
                    static_ability.cost_increase_mana_cost_per_additional_target()
                {
                    let additional_targets = chosen_target_count.saturating_sub(1);
                    for _ in 0..additional_targets {
                        increase_pips.extend(per_target_cost.pips().iter().cloned());
                    }
                }
            }
        }
    }

    let mut adjusted = cost.clone();
    if !increase_pips.is_empty() {
        adjusted = add_mana_cost(&adjusted, &ManaCost::from_pips(increase_pips));
    }
    if total_increase > 0 {
        adjusted = add_generic_mana_cost(&adjusted, total_increase as u32);
    }
    if total_reduction > 0 {
        adjusted = adjusted.reduce_generic(total_reduction as u32);
    }
    if !reduction_pips.is_empty() {
        adjusted = reduce_mana_cost(&adjusted, &ManaCost::from_pips(reduction_pips));
    }
    adjusted
}

pub(crate) fn resolve_this_spell_cost_reduction_value(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    reduction: &crate::static_abilities::ThisSpellCostReduction,
) -> i32 {
    if matches!(
        (&reduction.condition, &reduction.reduction),
        (
            crate::static_abilities::ThisSpellCostCondition::LifeTotalLessThanStarting,
            crate::effect::Value::X
        )
    ) {
        if let Some(player_state) = game.player(player) {
            return player_state
                .starting_life
                .saturating_sub(player_state.life)
                .max(0);
        }
    }

    resolve_cost_modifier_value(game, player, spell, &reduction.reduction)
}

pub(crate) fn add_generic_mana_cost(
    cost: &crate::mana::ManaCost,
    increase: u32,
) -> crate::mana::ManaCost {
    if increase == 0 {
        return cost.clone();
    }
    use crate::mana::ManaSymbol;

    let mut new_pips = cost.pips().to_vec();
    let mut remaining = increase;
    while remaining > 0 {
        let chunk = remaining.min(u8::MAX as u32) as u8;
        new_pips.push(vec![ManaSymbol::Generic(chunk)]);
        remaining -= chunk as u32;
    }

    crate::mana::ManaCost::from_pips(new_pips)
}

pub(crate) fn add_mana_cost(
    cost: &crate::mana::ManaCost,
    add: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    if add.pips().is_empty() {
        return cost.clone();
    }
    let mut new_pips = cost.pips().to_vec();
    new_pips.extend(add.pips().iter().cloned());
    crate::mana::ManaCost::from_pips(new_pips)
}

pub(crate) fn reduce_mana_cost(
    cost: &crate::mana::ManaCost,
    reduction: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    use crate::mana::ManaSymbol;

    if reduction.pips().is_empty() {
        return cost.clone();
    }
    let mut pips = cost.pips().to_vec();
    let mut generic_reduction: u32 = 0;
    for red_pip in reduction.pips() {
        if red_pip.len() == 1
            && let ManaSymbol::Generic(amount) = red_pip[0]
        {
            generic_reduction = generic_reduction.saturating_add(amount as u32);
            continue;
        }
        if let Some(pos) = pips.iter().position(|pip| pip == red_pip) {
            pips.remove(pos);
        }
    }
    let reduced = crate::mana::ManaCost::from_pips(pips);
    if generic_reduction > 0 {
        reduced.reduce_generic(generic_reduction)
    } else {
        reduced
    }
}

pub(crate) fn resolve_cost_modifier_value(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    value: &crate::effect::Value,
) -> i32 {
    let mut dm = SelectFirstDecisionMaker;
    let ctx = ExecutionContext::new(spell.id, player, &mut dm);
    resolve_value(game, value, &ctx).unwrap_or(0)
}

pub(crate) fn resolve_cost_modifier_value_for_source(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    value: &crate::effect::Value,
) -> i32 {
    let mut dm = SelectFirstDecisionMaker;
    let ctx = ExecutionContext::new(source, controller, &mut dm);
    resolve_value(game, value, &ctx).unwrap_or(0)
}

/// Calculate the number of cards that need to be exiled for Delve.
///
/// Returns how many cards from graveyard should be exiled based on:
/// - The generic mana remaining in the cost after other reductions
/// - The player's available mana
/// - Cards available in graveyard
pub fn calculate_delve_exile_count(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> u32 {
    calculate_delve_exile_count_with_targets(game, player, spell, base_cost, 1)
}

/// Calculate the number of cards to exile for Delve with explicit target count.
pub fn calculate_delve_exile_count_with_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
) -> u32 {
    use crate::ability::AbilityKind;

    // Only calculate Delve if the spell actually has Delve
    let has_delve_ability = spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_delve()
        } else {
            false
        }
    });
    if !has_delve_ability {
        return 0;
    }

    // First apply other cost reductions (like Affinity)
    let mut cost_after_reductions = base_cost.clone();

    let has_affinity = spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_affinity()
        } else {
            false
        }
    });

    if has_affinity {
        let artifact_count = count_artifacts_controlled(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(artifact_count);
    }

    cost_after_reductions = apply_spell_cost_modifiers(
        game,
        player,
        spell,
        &cost_after_reductions,
        chosen_target_count,
        &[],
        &CastingMethod::Normal,
    );

    // Now calculate how much generic mana remains
    let generic_remaining = cost_after_reductions.generic_mana_total();

    // Get graveyard count and calculate exile amount
    let graveyard_count = count_cards_in_graveyard(game, player);

    // Exile up to the generic mana cost (maximum Delve)
    generic_remaining.min(graveyard_count)
}

/// Count the number of artifacts controlled by a player.
pub fn count_artifacts_controlled(game: &GameState, player: PlayerId) -> u32 {
    let view = DerivedGameView::new(game);
    count_artifacts_controlled_with_view(game, player, &view)
}

pub(crate) fn count_artifacts_controlled_with_view(
    game: &GameState,
    player: PlayerId,
    view: &DerivedGameView<'_>,
) -> u32 {
    game.battlefield
        .iter()
        .filter(|&&id| {
            if let Some(obj) = game.object(id) {
                game.controller_of(obj) == player
                    && view.object_has_card_type(id, crate::types::CardType::Artifact)
            } else {
                false
            }
        })
        .count() as u32
}

/// Check if a spell has the Delve ability.
pub fn has_delve(spell: &crate::object::Object) -> bool {
    use crate::ability::AbilityKind;
    spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_delve()
        } else {
            false
        }
    })
}

/// Count cards in a player's graveyard (for Delve calculation).
pub fn count_cards_in_graveyard(game: &GameState, player: PlayerId) -> u32 {
    game.player(player)
        .map(|p| p.graveyard.len() as u32)
        .unwrap_or(0)
}

/// Compute potential mana available to a player.
///
/// This includes:
/// - Current mana pool
/// - Mana from all untapped lands and mana sources that can be activated
///
/// Returns a ManaPool representing the maximum mana the player could produce.
pub fn compute_potential_mana(game: &GameState, player: PlayerId) -> crate::player::ManaPool {
    let view = DerivedGameView::new(game);
    compute_potential_mana_with_view(game, player, &view)
}

#[derive(Clone)]
struct AvailableManaSource {
    outputs: Vec<Vec<ManaSymbol>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ManaPaymentSearchKey {
    pip_index: usize,
    white: u32,
    blue: u32,
    black: u32,
    red: u32,
    green: u32,
    colorless: u32,
    life_to_pay: u32,
    used_sources_mask: u128,
}

impl ManaPaymentSearchKey {
    fn new(
        pip_index: usize,
        pool: &crate::player::ManaPool,
        life_to_pay: u32,
        used_sources_mask: u128,
    ) -> Self {
        Self {
            pip_index,
            white: pool.white,
            blue: pool.blue,
            black: pool.black,
            red: pool.red,
            green: pool.green,
            colorless: pool.colorless,
            life_to_pay,
            used_sources_mask,
        }
    }
}

fn can_pay_mana_cost_with_available_sources(
    game: &GameState,
    player: PlayerId,
    source: Option<ObjectId>,
    cost: &crate::mana::ManaCost,
    x_value: u32,
    reason: crate::costs::PaymentReason,
    allow_any_color: bool,
    allow_black_life: bool,
    view: &DerivedGameView<'_>,
) -> bool {
    let Some(player_obj) = game.player(player) else {
        return false;
    };

    let mut pips = expand_mana_cost_to_unit_pips(cost, x_value, allow_black_life);
    pips.sort_by_key(|pip| pip_payment_sort_key(pip));

    let sources = available_mana_sources_for_payment(game, player, view);
    if sources.len() > 128 {
        return can_pay_expanded_pips_large_source_count(
            game,
            player,
            reason,
            &pips,
            0,
            player_obj.mana_pool.clone(),
            &sources,
            &mut vec![false; sources.len()],
            0,
            allow_any_color,
            source,
        );
    }

    let mut failed_states = std::collections::HashSet::new();
    can_pay_expanded_pips(
        game,
        player,
        reason,
        &pips,
        0,
        player_obj.mana_pool.clone(),
        &sources,
        0,
        0,
        allow_any_color,
        source,
        &mut failed_states,
    )
}

fn expand_mana_cost_to_unit_pips(
    cost: &crate::mana::ManaCost,
    x_value: u32,
    allow_black_life: bool,
) -> Vec<Vec<ManaSymbol>> {
    let mut pips = Vec::new();
    for pip in cost.pips() {
        if pip.len() == 1 {
            match pip[0] {
                ManaSymbol::Generic(n) => {
                    pips.extend((0..n).map(|_| vec![ManaSymbol::Generic(1)]));
                    continue;
                }
                ManaSymbol::X => {
                    pips.extend((0..x_value).map(|_| vec![ManaSymbol::Generic(1)]));
                    continue;
                }
                ManaSymbol::Black if allow_black_life => {
                    pips.push(vec![ManaSymbol::Black, ManaSymbol::Life(2)]);
                    continue;
                }
                _ => {}
            }
        }
        pips.push(pip.clone());
    }
    pips
}

fn pip_payment_sort_key(pip: &[ManaSymbol]) -> (u8, usize) {
    let has_generic = pip
        .iter()
        .any(|symbol| matches!(symbol, ManaSymbol::Generic(_) | ManaSymbol::X));
    let has_life_only = pip
        .iter()
        .all(|symbol| matches!(symbol, ManaSymbol::Life(_)));
    let has_colored = pip.iter().any(|symbol| {
        matches!(
            symbol,
            ManaSymbol::White
                | ManaSymbol::Blue
                | ManaSymbol::Black
                | ManaSymbol::Red
                | ManaSymbol::Green
        )
    });
    let has_colorless_or_snow = pip
        .iter()
        .any(|symbol| matches!(symbol, ManaSymbol::Colorless | ManaSymbol::Snow));

    let class = if has_colored && !has_generic {
        0
    } else if has_colorless_or_snow && !has_generic {
        1
    } else if has_colored {
        2
    } else if has_generic {
        3
    } else if has_life_only {
        4
    } else {
        5
    };
    (class, pip.len())
}

fn available_mana_sources_for_payment(
    game: &GameState,
    player: PlayerId,
    view: &DerivedGameView<'_>,
) -> Vec<AvailableManaSource> {
    use crate::ability::AbilityKind;

    let mut sources = Vec::new();
    let analysis = view.simple_battlefield_mana_analysis(player);

    for &perm_id in analysis.mana_source_ids() {
        let Some(object) = game.object(perm_id) else {
            continue;
        };
        let abilities = view
            .abilities_rc(perm_id)
            .unwrap_or_else(|| std::rc::Rc::new(object.abilities.clone()));
        let mut outputs_for_permanent = Vec::new();
        for &ability_index in analysis.mana_ability_indices_for(perm_id) {
            let Some(ability) = abilities.get(ability_index) else {
                continue;
            };
            let AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if analysis
                .activatable_indices_for(perm_id)
                .contains(&ability_index)
                || crate::special_actions::can_activate_mana_ability_check_with_view(
                    game,
                    player,
                    perm_id,
                    ability_index,
                    ability,
                    view,
                    None,
                )
                .is_ok()
            {
                let outputs = mana_ability_output_options(game, player, perm_id, mana_ability);
                for output in outputs {
                    if !outputs_for_permanent.contains(&output) {
                        outputs_for_permanent.push(output);
                    }
                }
            }
        }
        if !outputs_for_permanent.is_empty() {
            sources.push(AvailableManaSource {
                outputs: outputs_for_permanent,
            });
        }
    }

    sources
}

fn mana_ability_output_options(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    mana_ability: &crate::ability::ActivatedAbility,
) -> Vec<Vec<ManaSymbol>> {
    use crate::effects::{
        AddColorlessManaEffect, AddManaEffect, AddManaOfAnyColorEffect, AddManaOfAnyOneColorEffect,
        AddScaledManaEffect,
    };

    if let Some(output) = mana_ability.mana_output.as_ref()
        && !output.is_empty()
    {
        return vec![output.clone()];
    }

    let resolve_amount = |value: &crate::effect::Value| -> usize {
        let mut dm = SelectFirstDecisionMaker;
        let ctx = ExecutionContext::new(source, player, &mut dm);
        resolve_value(game, value, &ctx).unwrap_or(0).max(0) as usize
    };

    let mut outputs = vec![Vec::new()];
    for effect in mana_ability.effects.flattened_default_effects() {
        let effect_outputs = if let Some(add_mana) = effect.downcast_ref::<AddManaEffect>() {
            vec![add_mana.mana.clone()]
        } else if let Some(add_colorless) = effect.downcast_ref::<AddColorlessManaEffect>() {
            vec![vec![
                ManaSymbol::Colorless;
                resolve_amount(&add_colorless.amount)
            ]]
        } else if let Some(add_scaled) = effect.downcast_ref::<AddScaledManaEffect>() {
            let repeats = resolve_amount(&add_scaled.amount);
            let mut output = Vec::new();
            for _ in 0..repeats {
                output.extend(add_scaled.mana.iter().copied());
            }
            vec![output]
        } else if let Some(add_any_color) = effect.downcast_ref::<AddManaOfAnyColorEffect>() {
            let colors = add_any_color.available_colors.as_deref().unwrap_or(&[
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ]);
            any_color_output_options(colors, resolve_amount(&add_any_color.amount), false)
        } else if let Some(add_any_one_color) = effect.downcast_ref::<AddManaOfAnyOneColorEffect>()
        {
            any_color_output_options(
                &[
                    crate::color::Color::White,
                    crate::color::Color::Blue,
                    crate::color::Color::Black,
                    crate::color::Color::Red,
                    crate::color::Color::Green,
                ],
                resolve_amount(&add_any_one_color.amount),
                true,
            )
        } else if let Some(symbols) = effect.producible_mana_symbols(game, source, player) {
            symbols
                .into_iter()
                .filter(|symbol| is_payable_mana_symbol(*symbol))
                .map(|symbol| vec![symbol])
                .collect()
        } else {
            Vec::new()
        };

        if effect_outputs.is_empty() {
            continue;
        }
        outputs = combine_mana_output_options(&outputs, &effect_outputs);
    }

    let outputs = outputs
        .into_iter()
        .filter(|output| !output.is_empty())
        .collect::<Vec<_>>();
    if outputs.is_empty() {
        let inferred = mana_ability.inferred_mana_symbols(game, source, player);
        if inferred.is_empty() {
            Vec::new()
        } else {
            vec![inferred]
        }
    } else {
        outputs
    }
}

fn any_color_output_options(
    colors: &[crate::color::Color],
    amount: usize,
    same_color: bool,
) -> Vec<Vec<ManaSymbol>> {
    if amount == 0 {
        return vec![Vec::new()];
    }
    if same_color {
        return colors
            .iter()
            .map(|color| vec![ManaSymbol::from_color(*color); amount])
            .collect();
    }

    let mut outputs = vec![Vec::new()];
    for _ in 0..amount {
        let mut next = Vec::new();
        for output in &outputs {
            for color in colors {
                let mut candidate = output.clone();
                candidate.push(ManaSymbol::from_color(*color));
                next.push(candidate);
                if next.len() >= 128 {
                    return next;
                }
            }
        }
        outputs = next;
    }
    outputs
}

fn combine_mana_output_options(
    base: &[Vec<ManaSymbol>],
    next: &[Vec<ManaSymbol>],
) -> Vec<Vec<ManaSymbol>> {
    let mut combined = Vec::new();
    for left in base {
        for right in next {
            let mut output = left.clone();
            output.extend(right.iter().copied());
            combined.push(output);
            if combined.len() >= 128 {
                return combined;
            }
        }
    }
    combined
}

#[allow(clippy::too_many_arguments)]
fn can_pay_expanded_pips(
    game: &GameState,
    player: PlayerId,
    reason: crate::costs::PaymentReason,
    pips: &[Vec<ManaSymbol>],
    pip_index: usize,
    pool: crate::player::ManaPool,
    sources: &[AvailableManaSource],
    used_sources_mask: u128,
    life_to_pay: u32,
    allow_any_color: bool,
    payment_source: Option<ObjectId>,
    failed_states: &mut std::collections::HashSet<ManaPaymentSearchKey>,
) -> bool {
    if pip_index >= pips.len() {
        return game.can_pay_life_with_reason(player, life_to_pay, reason);
    }

    let key = ManaPaymentSearchKey::new(pip_index, &pool, life_to_pay, used_sources_mask);
    if failed_states.contains(&key) {
        return false;
    }

    let pip = &pips[pip_index];
    for &symbol in pip {
        if let ManaSymbol::Life(amount) = symbol {
            let next_life = life_to_pay.saturating_add(amount as u32);
            if game.can_pay_life_with_reason(player, next_life, reason)
                && can_pay_expanded_pips(
                    game,
                    player,
                    reason,
                    pips,
                    pip_index + 1,
                    pool.clone(),
                    sources,
                    used_sources_mask,
                    next_life,
                    allow_any_color,
                    payment_source,
                    failed_states,
                )
            {
                return true;
            }
            continue;
        }

        let mut pool_after = pool.clone();
        if remove_mana_for_pip(&mut pool_after, symbol, allow_any_color)
            && can_pay_expanded_pips(
                game,
                player,
                reason,
                pips,
                pip_index + 1,
                pool_after,
                sources,
                used_sources_mask,
                life_to_pay,
                allow_any_color,
                payment_source,
                failed_states,
            )
        {
            return true;
        }

        for (source_index, source) in sources.iter().enumerate() {
            let source_mask = 1u128 << source_index;
            if used_sources_mask & source_mask != 0 {
                continue;
            }
            for output in &source.outputs {
                if let Some(pool_from_output) =
                    consume_output_for_pip(output, symbol, allow_any_color)
                {
                    let mut combined_pool = pool.clone();
                    add_pool(&mut combined_pool, &pool_from_output);
                    let can_pay_rest = can_pay_expanded_pips(
                        game,
                        player,
                        reason,
                        pips,
                        pip_index + 1,
                        combined_pool,
                        sources,
                        used_sources_mask | source_mask,
                        life_to_pay,
                        allow_any_color,
                        payment_source,
                        failed_states,
                    );
                    if can_pay_rest {
                        return true;
                    }
                }
            }
        }
    }

    let _ = payment_source;
    failed_states.insert(key);
    false
}

#[allow(clippy::too_many_arguments)]
fn can_pay_expanded_pips_large_source_count(
    game: &GameState,
    player: PlayerId,
    reason: crate::costs::PaymentReason,
    pips: &[Vec<ManaSymbol>],
    pip_index: usize,
    pool: crate::player::ManaPool,
    sources: &[AvailableManaSource],
    used_sources: &mut [bool],
    life_to_pay: u32,
    allow_any_color: bool,
    payment_source: Option<ObjectId>,
) -> bool {
    if pip_index >= pips.len() {
        return game.can_pay_life_with_reason(player, life_to_pay, reason);
    }

    let pip = &pips[pip_index];
    for &symbol in pip {
        if let ManaSymbol::Life(amount) = symbol {
            let next_life = life_to_pay.saturating_add(amount as u32);
            if game.can_pay_life_with_reason(player, next_life, reason)
                && can_pay_expanded_pips_large_source_count(
                    game,
                    player,
                    reason,
                    pips,
                    pip_index + 1,
                    pool.clone(),
                    sources,
                    used_sources,
                    next_life,
                    allow_any_color,
                    payment_source,
                )
            {
                return true;
            }
            continue;
        }

        let mut pool_after = pool.clone();
        if remove_mana_for_pip(&mut pool_after, symbol, allow_any_color)
            && can_pay_expanded_pips_large_source_count(
                game,
                player,
                reason,
                pips,
                pip_index + 1,
                pool_after,
                sources,
                used_sources,
                life_to_pay,
                allow_any_color,
                payment_source,
            )
        {
            return true;
        }

        for (source_index, source) in sources.iter().enumerate() {
            if used_sources[source_index] {
                continue;
            }
            for output in &source.outputs {
                if let Some(pool_from_output) =
                    consume_output_for_pip(output, symbol, allow_any_color)
                {
                    let mut combined_pool = pool.clone();
                    add_pool(&mut combined_pool, &pool_from_output);
                    used_sources[source_index] = true;
                    let can_pay_rest = can_pay_expanded_pips_large_source_count(
                        game,
                        player,
                        reason,
                        pips,
                        pip_index + 1,
                        combined_pool,
                        sources,
                        used_sources,
                        life_to_pay,
                        allow_any_color,
                        payment_source,
                    );
                    used_sources[source_index] = false;
                    if can_pay_rest {
                        return true;
                    }
                }
            }
        }
    }

    let _ = payment_source;
    false
}

fn consume_output_for_pip(
    output: &[ManaSymbol],
    pip: ManaSymbol,
    allow_any_color: bool,
) -> Option<crate::player::ManaPool> {
    for (idx, &produced) in output.iter().enumerate() {
        if mana_symbol_can_pay_pip(produced, pip, allow_any_color) {
            let mut remainder = crate::player::ManaPool::default();
            for (other_idx, &symbol) in output.iter().enumerate() {
                if other_idx != idx && is_payable_mana_symbol(symbol) {
                    remainder.add(symbol, 1);
                }
            }
            return Some(remainder);
        }
    }
    None
}

fn add_pool(pool: &mut crate::player::ManaPool, addition: &crate::player::ManaPool) {
    for symbol in PAYABLE_MANA_SYMBOLS {
        let amount = addition.amount(symbol);
        if amount > 0 {
            pool.add(symbol, amount);
        }
    }
}

const PAYABLE_MANA_SYMBOLS: [ManaSymbol; 6] = [
    ManaSymbol::White,
    ManaSymbol::Blue,
    ManaSymbol::Black,
    ManaSymbol::Red,
    ManaSymbol::Green,
    ManaSymbol::Colorless,
];

fn remove_mana_for_pip(
    pool: &mut crate::player::ManaPool,
    pip: ManaSymbol,
    allow_any_color: bool,
) -> bool {
    match pip {
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green => {
            if !allow_any_color {
                return pool.remove(pip, 1);
            }
            remove_any_payable_mana(pool)
        }
        ManaSymbol::Colorless => pool.remove(ManaSymbol::Colorless, 1),
        ManaSymbol::Generic(_) => remove_any_payable_mana(pool),
        ManaSymbol::Snow => false,
        ManaSymbol::Life(_) | ManaSymbol::X => false,
    }
}

fn remove_any_payable_mana(pool: &mut crate::player::ManaPool) -> bool {
    for symbol in PAYABLE_MANA_SYMBOLS {
        if pool.remove(symbol, 1) {
            return true;
        }
    }
    false
}

fn mana_symbol_can_pay_pip(produced: ManaSymbol, pip: ManaSymbol, allow_any_color: bool) -> bool {
    match pip {
        ManaSymbol::Generic(_) => is_payable_mana_symbol(produced),
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green => {
            produced == pip || (allow_any_color && is_payable_mana_symbol(produced))
        }
        ManaSymbol::Colorless => produced == ManaSymbol::Colorless,
        ManaSymbol::Snow | ManaSymbol::Life(_) | ManaSymbol::X => false,
    }
}

fn is_payable_mana_symbol(symbol: ManaSymbol) -> bool {
    matches!(
        symbol,
        ManaSymbol::White
            | ManaSymbol::Blue
            | ManaSymbol::Black
            | ManaSymbol::Red
            | ManaSymbol::Green
            | ManaSymbol::Colorless
    )
}

pub(crate) fn compute_potential_mana_with_view(
    game: &GameState,
    player: PlayerId,
    view: &DerivedGameView<'_>,
) -> crate::player::ManaPool {
    use crate::ability::AbilityKind;
    use crate::costs::{CostCheckContext, can_pay_with_check_context};

    // Start with current mana pool
    let mut potential = game
        .player(player)
        .map(|p| p.mana_pool.clone())
        .unwrap_or_default();
    let simple_mana_analysis = view.simple_battlefield_mana_analysis(player);

    // Add mana from all available mana abilities.
    // The pass-local analysis already found the controlled mana sources.
    for &perm_id in simple_mana_analysis.mana_source_ids() {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };

        let mana_ability_indices = simple_mana_analysis.mana_ability_indices_for(perm_id);
        if mana_ability_indices.len() == 1
            && let Some(symbols) = simple_mana_analysis.first_output_for(perm_id)
        {
            for mana in symbols {
                potential.add(*mana, 1);
            }
            continue;
        }

        let cached_abilities = view.abilities_rc(perm_id);
        let abilities = cached_abilities.as_deref().unwrap_or(&perm.abilities);
        for &ability_idx in mana_ability_indices {
            let Some(ability) = abilities.get(ability_idx) else {
                continue;
            };
            let AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if !mana_ability.is_runtime_mana_ability(game, perm_id, game.controller_of(perm)) {
                continue;
            }
            if mana_ability.has_tap_cost() && !game.can_activate_tap_abilities_of(perm_id) {
                continue;
            }
            // Do a simple non-recursive check for whether this mana ability
            // could be activated. We intentionally skip mana cost checks here
            // to avoid infinite recursion (mana ability with mana cost would
            // call compute_potential_mana again).
            let simple_taplike_costs_only = mana_ability.mana_cost.costs().iter().all(|cost| {
                cost.processing_mode().is_mana_payment()
                    || cost.requires_tap()
                    || cost.requires_untap()
            });

            let can_activate = if simple_taplike_costs_only {
                mana_ability.mana_cost.costs().iter().all(|cost| {
                    if cost.requires_tap() {
                        return !game.is_tapped(perm_id)
                            && (!view
                                .object_has_card_type(perm_id, crate::types::CardType::Creature)
                                || !game.is_summoning_sick(perm_id)
                                || view.object_has_static_ability_id(
                                    perm_id,
                                    crate::static_abilities::StaticAbilityId::Haste,
                                ));
                    }
                    if cost.requires_untap() {
                        return game.is_tapped(perm_id)
                            && (!view
                                .object_has_card_type(perm_id, crate::types::CardType::Creature)
                                || !game.is_summoning_sick(perm_id)
                                || view.object_has_static_ability_id(
                                    perm_id,
                                    crate::static_abilities::StaticAbilityId::Haste,
                                ));
                    }
                    true
                })
            } else {
                let ctx = CostCheckContext::new(perm_id, player)
                    .with_reason(crate::costs::PaymentReason::ActivateManaAbility);
                let components = mana_ability.mana_cost.costs();
                let mut idx = 0usize;
                let mut payable = true;
                while idx < components.len() {
                    let cost = if let Some(choose) =
                        components[idx].effect_ref().and_then(|effect| {
                            effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
                        })
                        && let Some(next) = components.get(idx + 1)
                        && let Some(step) = crate::game_loop::choose_tagged_cost_step(choose, next)
                    {
                        idx += 2;
                        match step {
                            crate::game_loop::ActivationCostStep::Cost(cost)
                            | crate::game_loop::ActivationCostStep::Sacrifice { cost, .. } => cost,
                            crate::game_loop::ActivationCostStep::CardChoice(choice) => {
                                activation_card_cost_choice_cost(&choice).clone()
                            }
                        }
                    } else {
                        let cost = components[idx].clone();
                        idx += 1;
                        cost
                    };

                    // Skip mana cost check to avoid recursion - we only check
                    // non-mana costs like tap, life, sacrifice.
                    if cost.processing_mode().is_mana_payment() {
                        continue;
                    }

                    if game
                        .validate_cost_for_payment_reason(player, perm_id, &cost, ctx.reason)
                        .is_err()
                    {
                        payable = false;
                        break;
                    }
                    if can_pay_with_check_context(&*cost.0, game, &ctx).is_err() {
                        payable = false;
                        break;
                    }
                }
                payable
            };

            // Also check activation condition if present
            let condition_met = mana_ability
                .activation_condition
                .as_ref()
                .is_none_or(|cond| {
                    check_mana_ability_condition_for_potential(
                        game,
                        player,
                        perm_id,
                        ability_idx,
                        cond,
                    )
                });

            if can_activate && condition_met {
                // Add the mana this ability could produce, preserving
                // multiplicity for effects like Black Lotus.
                for mana in
                    inferred_potential_mana_symbols_for_ability(game, perm_id, player, mana_ability)
                {
                    potential.add(mana, 1);
                }
            }
        }
    }

    potential
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

pub(crate) fn simple_battlefield_mana_ability_output(
    game: &GameState,
    player: PlayerId,
    permanent_id: ObjectId,
    ability_index: usize,
    ability: &crate::ability::Ability,
    view: &DerivedGameView<'_>,
) -> Option<Vec<ManaSymbol>> {
    use crate::ability::AbilityKind;

    let object = game.object(permanent_id)?;
    if game.controller_of(object) != player
        || object.zone != Zone::Battlefield
        || !ability.functions_in(&object.zone)
    {
        return None;
    }

    let AbilityKind::Activated(mana_ability) = &ability.kind else {
        return None;
    };
    if !mana_ability.is_runtime_mana_ability(game, permanent_id, player)
        || !game.can_activate_abilities_of(permanent_id)
    {
        return None;
    }
    if mana_ability.has_tap_cost() && !game.can_activate_tap_abilities_of(permanent_id) {
        return None;
    }
    if !mana_ability
        .mana_cost
        .costs()
        .iter()
        .all(|cost| cost.requires_tap() || cost.requires_untap())
    {
        return None;
    }

    for cost in mana_ability.mana_cost.costs() {
        if cost.requires_tap() {
            if game.is_tapped(permanent_id) {
                return None;
            }
            if view.object_has_card_type(permanent_id, crate::types::CardType::Creature)
                && game.is_summoning_sick(permanent_id)
                && !view.object_has_static_ability_id(
                    permanent_id,
                    crate::static_abilities::StaticAbilityId::Haste,
                )
            {
                return None;
            }
        }
        if cost.requires_untap() && !game.is_tapped(permanent_id) {
            return None;
        }
        if cost.requires_untap()
            && view.object_has_card_type(permanent_id, crate::types::CardType::Creature)
            && game.is_summoning_sick(permanent_id)
            && !view.object_has_static_ability_id(
                permanent_id,
                crate::static_abilities::StaticAbilityId::Haste,
            )
        {
            return None;
        }
    }

    if let Some(condition) = &mana_ability.activation_condition
        && !check_mana_ability_condition_for_potential(
            game,
            player,
            permanent_id,
            ability_index,
            condition,
        )
    {
        return None;
    }

    Some(inferred_potential_mana_symbols_for_ability(
        game,
        permanent_id,
        player,
        mana_ability,
    ))
}

pub(crate) fn inferred_potential_mana_symbols_for_ability(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    mana_ability: &crate::ability::ActivatedAbility,
) -> Vec<ManaSymbol> {
    use crate::effects::{
        AddColorlessManaEffect, AddManaEffect, AddManaOfAnyColorEffect, AddManaOfAnyOneColorEffect,
        AddScaledManaEffect,
    };

    if let Some(mana_output) = mana_ability.mana_output.as_ref()
        && !mana_output.is_empty()
    {
        return mana_output.clone();
    }

    let resolve_amount = |value: &crate::effect::Value| -> usize {
        let mut dm = SelectFirstDecisionMaker;
        let ctx = ExecutionContext::new(source, controller, &mut dm);
        resolve_value(game, value, &ctx).unwrap_or(0).max(0) as usize
    };

    let mut inferred = Vec::new();
    for effect in &mana_ability.effects {
        if let Some(add_mana) = effect.downcast_ref::<AddManaEffect>() {
            inferred.extend(add_mana.mana.iter().copied());
            continue;
        }
        if let Some(add_colorless) = effect.downcast_ref::<AddColorlessManaEffect>() {
            inferred.extend(std::iter::repeat_n(
                ManaSymbol::Colorless,
                resolve_amount(&add_colorless.amount),
            ));
            continue;
        }
        if let Some(add_scaled) = effect.downcast_ref::<AddScaledManaEffect>() {
            let repeats = resolve_amount(&add_scaled.amount);
            for _ in 0..repeats {
                inferred.extend(add_scaled.mana.iter().copied());
            }
            continue;
        }
        if let Some(add_any_color) = effect.downcast_ref::<AddManaOfAnyColorEffect>() {
            let amount = resolve_amount(&add_any_color.amount);
            let colors = add_any_color.available_colors.as_deref().unwrap_or(&[
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ]);
            for color in colors {
                inferred.extend(std::iter::repeat_n(ManaSymbol::from_color(*color), amount));
            }
            continue;
        }
        if let Some(add_any_one_color) = effect.downcast_ref::<AddManaOfAnyOneColorEffect>() {
            let amount = resolve_amount(&add_any_one_color.amount);
            for color in [
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ] {
                inferred.extend(std::iter::repeat_n(ManaSymbol::from_color(color), amount));
            }
            continue;
        }

        if let Some(symbols) = effect.producible_mana_symbols(game, source, controller) {
            inferred.extend(symbols.into_iter().filter(|symbol| {
                matches!(
                    symbol,
                    ManaSymbol::White
                        | ManaSymbol::Blue
                        | ManaSymbol::Black
                        | ManaSymbol::Red
                        | ManaSymbol::Green
                        | ManaSymbol::Colorless
                )
            }));
        }
    }

    if inferred.is_empty() {
        mana_ability.inferred_mana_symbols(game, source, controller)
    } else {
        inferred
    }
}

/// Check mana ability condition for potential mana computation.
pub(crate) fn check_mana_ability_condition_for_potential(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    ability_index: usize,
    condition: &crate::ConditionExpr,
) -> bool {
    let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
        controller: player,
        source,
        defending_player: None,
        attacking_player: None,
        filter_source: Some(source),
        triggering_event: None,
        trigger_identity: None,
        ability_index: Some(ability_index),
        options: crate::condition_eval::ExternalEvaluationOptions::default(),
    };
    crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
}

/// Check if a player could pay a mana cost using potential mana.
///
/// This considers mana currently in pool plus mana from untapped sources.
pub fn can_potentially_pay(
    game: &GameState,
    player: PlayerId,
    cost: &crate::mana::ManaCost,
    x_value: u32,
) -> bool {
    let potential = compute_potential_mana(game, player);
    potential.can_pay(cost, x_value)
}

/// Calculate the effective mana cost for a spell with Delve, given available graveyard cards.
///
/// For Delve, each card exiled from graveyard pays for {1} of generic mana.
/// This function calculates the minimum mana needed given maximum Delve usage.
pub fn calculate_delve_effective_cost(
    base_cost: &crate::mana::ManaCost,
    available_graveyard_cards: u32,
) -> crate::mana::ManaCost {
    let generic_in_cost = base_cost.generic_mana_total();
    let delve_amount = generic_in_cost.min(available_graveyard_cards);
    base_cost.reduce_generic(delve_amount)
}

/// Calculate how many cards to exile for Delve to minimize mana cost while being castable.
///
/// Returns (cards_to_exile, effective_mana_cost).
/// This greedily exiles cards to pay generic mana.
pub fn calculate_optimal_delve(
    game: &GameState,
    player: PlayerId,
    base_cost: &crate::mana::ManaCost,
) -> (u32, crate::mana::ManaCost) {
    let graveyard_count = count_cards_in_graveyard(game, player);
    let generic_in_cost = base_cost.generic_mana_total();

    // Exile up to the generic mana cost
    let delve_amount = generic_in_cost.min(graveyard_count);
    let effective_cost = base_cost.reduce_generic(delve_amount);

    (delve_amount, effective_cost)
}

/// Check if a spell has the Convoke ability.
pub fn has_convoke(spell: &crate::object::Object) -> bool {
    use crate::ability::AbilityKind;
    spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_convoke()
        } else {
            false
        }
    })
}

/// Calculate which creatures to tap for Convoke.
///
/// Returns the creature IDs to tap for maximum Convoke usage.
/// This takes into account Affinity and Delve reductions first.
pub fn calculate_convoke_creatures_to_tap(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> Vec<crate::ids::ObjectId> {
    use crate::ability::AbilityKind;

    if !has_convoke(spell) {
        return Vec::new();
    }

    // First apply other cost reductions (like Affinity and Delve)
    let mut cost_after_reductions = base_cost.clone();

    let has_affinity = spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_affinity()
        } else {
            false
        }
    });

    if has_affinity {
        let artifact_count = count_artifacts_controlled(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(artifact_count);
    }

    cost_after_reductions = apply_spell_cost_modifiers(
        game,
        player,
        spell,
        &cost_after_reductions,
        1,
        &[],
        &CastingMethod::Normal,
    );

    let has_delve_ability = has_delve(spell);

    if has_delve_ability {
        let graveyard_count = count_cards_in_graveyard(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(graveyard_count);
    }

    // Now calculate Convoke creatures to tap
    let (creatures_to_tap, _) = calculate_convoke_cost(game, player, &cost_after_reductions);
    creatures_to_tap
}

/// Check if a spell has the Improvise ability.
pub fn has_improvise(spell: &crate::object::Object) -> bool {
    use crate::ability::AbilityKind;
    spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_improvise()
        } else {
            false
        }
    })
}

/// Get untapped artifacts controlled by a player that can be tapped for Improvise.
///
/// Returns a list of artifact ObjectIds.
pub fn get_improvise_artifacts(game: &GameState, player: PlayerId) -> Vec<crate::ids::ObjectId> {
    game.battlefield
        .iter()
        .filter_map(|&id| {
            let obj = game.object(id)?;
            // Must be an artifact controlled by player
            if game.controller_of(obj) != player
                || !obj.has_card_type(crate::types::CardType::Artifact)
            {
                return None;
            }
            // Must be untapped
            if game.is_tapped(id) {
                return None;
            }
            Some(id)
        })
        .collect()
}

/// Calculate the effective mana cost for a spell with Improvise.
///
/// For Improvise, each artifact tapped pays for {1} of generic mana.
/// Returns (artifacts_to_tap, effective_mana_cost).
pub fn calculate_improvise_cost(
    game: &GameState,
    player: PlayerId,
    cost: &crate::mana::ManaCost,
) -> (Vec<crate::ids::ObjectId>, crate::mana::ManaCost) {
    use crate::mana::ManaSymbol;

    let improvise_artifacts = get_improvise_artifacts(game, player);
    if improvise_artifacts.is_empty() {
        return (Vec::new(), cost.clone());
    }

    let mut artifacts_to_tap = Vec::new();
    let mut remaining_pips: Vec<Vec<ManaSymbol>> = cost.pips().to_vec();

    // Improvise only pays generic mana
    let mut i = 0;
    while i < remaining_pips.len() && artifacts_to_tap.len() < improvise_artifacts.len() {
        let pip = &remaining_pips[i];

        // Check if this is a generic pip
        if pip.len() == 1
            && let ManaSymbol::Generic(n) = pip[0]
        {
            let available = improvise_artifacts.len() - artifacts_to_tap.len();
            let to_tap = (n as usize).min(available);

            for j in 0..to_tap {
                artifacts_to_tap.push(improvise_artifacts[artifacts_to_tap.len()]);
                let _ = j; // Suppress unused warning
            }

            // Reduce or remove the generic pip
            let paid = to_tap as u8;
            if paid >= n {
                remaining_pips.remove(i);
                continue;
            } else {
                remaining_pips[i] = vec![ManaSymbol::Generic(n - paid)];
            }
        }
        i += 1;
    }

    let effective_cost = crate::mana::ManaCost::from_pips(remaining_pips);
    (artifacts_to_tap, effective_cost)
}

/// Calculate which artifacts to tap for Improvise.
///
/// Returns the artifact IDs to tap for maximum Improvise usage.
/// This takes into account Affinity, Delve, and Convoke reductions first.
pub fn calculate_improvise_artifacts_to_tap(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> Vec<crate::ids::ObjectId> {
    use crate::ability::AbilityKind;

    if !has_improvise(spell) {
        return Vec::new();
    }

    // First apply other cost reductions (Affinity, Delve, Convoke)
    let mut cost_after_reductions = base_cost.clone();

    let has_affinity = spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_affinity()
        } else {
            false
        }
    });

    if has_affinity {
        let artifact_count = count_artifacts_controlled(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(artifact_count);
    }

    cost_after_reductions = apply_spell_cost_modifiers(
        game,
        player,
        spell,
        &cost_after_reductions,
        1,
        &[],
        &CastingMethod::Normal,
    );

    let has_delve_ability = has_delve(spell);

    if has_delve_ability {
        let graveyard_count = count_cards_in_graveyard(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(graveyard_count);
    }

    let has_convoke_ability = has_convoke(spell);

    if has_convoke_ability {
        let (_, convoked_cost) = calculate_convoke_cost(game, player, &cost_after_reductions);
        cost_after_reductions = convoked_cost;
    }

    // Now calculate Improvise artifacts to tap
    let (artifacts_to_tap, _) = calculate_improvise_cost(game, player, &cost_after_reductions);
    artifacts_to_tap
}

/// Count untapped creatures controlled by a player that can be tapped for convoke.
///
/// Returns a tuple of (total_untapped_creatures, creature_ids_with_colors).
pub fn get_convoke_creatures(
    game: &GameState,
    player: PlayerId,
) -> Vec<(crate::ids::ObjectId, crate::color::ColorSet)> {
    game.battlefield
        .iter()
        .filter_map(|&id| {
            let obj = game.object(id)?;
            // Must be a creature controlled by player
            if game.controller_of(obj) != player || !game.current_is_creature(id) {
                return None;
            }
            // Must be untapped
            if game.is_tapped(id) {
                return None;
            }
            Some((id, game.current_colors(id).unwrap_or_else(|| obj.colors())))
        })
        .collect()
}

/// Calculate the effective mana cost for a spell with Convoke.
///
/// For Convoke, each creature tapped can pay for {1} or one mana of its colors.
/// This function calculates the minimum mana needed given maximum Convoke usage.
///
/// Returns (creatures_to_tap, effective_mana_cost).
pub fn calculate_convoke_cost(
    game: &GameState,
    player: PlayerId,
    cost: &crate::mana::ManaCost,
) -> (Vec<crate::ids::ObjectId>, crate::mana::ManaCost) {
    use crate::mana::ManaSymbol;

    let convoke_creatures = get_convoke_creatures(game, player);
    if convoke_creatures.is_empty() {
        return (Vec::new(), cost.clone());
    }

    let mut creatures_to_tap = Vec::new();
    let mut remaining_pips: Vec<Vec<ManaSymbol>> = cost.pips().to_vec();
    let mut available_creatures = convoke_creatures;

    // First pass: pay colored mana with matching creatures
    let mut i = 0;
    while i < remaining_pips.len() {
        let pip = &remaining_pips[i];

        // Check if this is a single colored pip
        if pip.len() == 1 {
            let color_opt = match pip[0] {
                ManaSymbol::White => Some(crate::color::Color::White),
                ManaSymbol::Blue => Some(crate::color::Color::Blue),
                ManaSymbol::Black => Some(crate::color::Color::Black),
                ManaSymbol::Red => Some(crate::color::Color::Red),
                ManaSymbol::Green => Some(crate::color::Color::Green),
                _ => None,
            };

            if let Some(color) = color_opt {
                // Find a creature with this color
                if let Some(idx) = available_creatures
                    .iter()
                    .position(|(_, colors)| colors.contains(color))
                {
                    let (creature_id, _) = available_creatures.remove(idx);
                    creatures_to_tap.push(creature_id);
                    remaining_pips.remove(i);
                    continue;
                }
            }
        }
        i += 1;
    }

    // Second pass: pay generic mana with any remaining creatures
    let mut i = 0;
    while i < remaining_pips.len() && !available_creatures.is_empty() {
        let pip = &remaining_pips[i];

        // Check if this is a generic pip
        if pip.len() == 1
            && let ManaSymbol::Generic(n) = pip[0]
        {
            let creatures_needed = (n as usize).min(available_creatures.len());
            for _ in 0..creatures_needed {
                let (creature_id, _) = available_creatures.remove(0);
                creatures_to_tap.push(creature_id);
            }

            // Reduce or remove the generic pip
            let paid = creatures_needed as u8;
            if paid >= n {
                remaining_pips.remove(i);
                continue;
            } else {
                remaining_pips[i] = vec![ManaSymbol::Generic(n - paid)];
            }
        }
        i += 1;
    }

    let effective_cost = crate::mana::ManaCost::from_pips(remaining_pips);
    (creatures_to_tap, effective_cost)
}

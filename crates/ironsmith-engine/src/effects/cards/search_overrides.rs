use crate::alternative_cast::{AlternativeCastingMethod, CastingMethod};
use crate::decision::{
    CastLegalityContext, FallbackStrategy, alternative_method_uses_printed_mana_cost,
    build_requirements_for_method, can_cast_spell_with_context, can_cast_with_cost_with_context,
    resolve_play_from_alternative_method, spell_mana_cost_for_cast,
};
use crate::decisions::make_decision_with_fallback;
use crate::decisions::specs::{MaySpec, ReplacementOption, ReplacementSpec};
use crate::derived_view::DerivedGameView;
use crate::effect::ManaSpendPermission;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::{ActiveManaSpendPermission, GameState, ManaSpendPermissionSource};
use crate::grant::Grantable;
use crate::grant_registry::GrantSource;
use crate::ids::{ObjectId, PlayerId};
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbilityId;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Admission and view policies supplied by a search adapter. Mixed-zone
/// selection may proceed when its library part cannot be searched.
pub(crate) struct LibrarySearchRequest {
    pub chooser: PlayerId,
    pub library_owner: Option<PlayerId>,
    pub search_library: bool,
    pub require_library_access: bool,
    pub restrict_initial_view: bool,
    /// Reevaluate admission at each presentation/offer/observation boundary.
    pub refresh_library_access: bool,
}

/// Actual search bindings supplied once by the shared admission owner.
pub(crate) struct LibrarySearchScope {
    pub chooser: PlayerId,
    pub library_cards: Vec<ObjectId>,
    pub found_card_policy: Option<OppositionAgentSearch>,
    pub event: Option<crate::triggers::TriggerEvent>,
    /// Actual in-search casts, already published at their native boundaries.
    pub completed_outputs: Vec<crate::effects::PublishedEffectOutputs>,
}

/// One owner for search admission, scoped control, initial library presentation,
/// in-search casting offers and the search observation's creation boundary.
/// The caller retains its selection/commit/rollback programme and result shape.
pub(crate) fn execute_library_search_scope<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    request: LibrarySearchRequest,
    body: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        LibrarySearchScope,
    ) -> Result<crate::effect::EffectOutcome, ExecutionError>,
) -> Result<crate::effect::EffectOutcome, ExecutionError> {
    let found_card_policy = request
        .library_owner
        .and_then(|owner| opposition_agent_search(game, request.chooser, owner));
    let can_search = request.library_owner.is_some_and(|owner| {
        game.can_search_library_from_effect(request.chooser, owner, ctx.controller)
    });
    if request.require_library_access && request.library_owner.is_some() && !can_search {
        return Ok(crate::effect::EffectOutcome::prevented());
    }
    let control = begin_opposition_agent_search_control(game, request.chooser, found_card_policy);
    let result = (|| {
        let mut library_cards = Vec::new();
        let mut completed_outputs = Vec::new();
        let access_now = |game: &GameState, ctx: &ExecutionContext| {
            if request.refresh_library_access {
                request.library_owner.is_some_and(|owner| {
                    game.can_search_library_from_effect(request.chooser, owner, ctx.controller)
                })
            } else {
                can_search
            }
        };
        if let Some(owner) = request.library_owner.filter(|_| access_now(game, ctx)) {
            library_cards = game
                .player(owner)
                .map(|player| player.library.to_vec())
                .unwrap_or_default();
            if request.restrict_initial_view {
                game.restrict_library_search_candidates(request.chooser, &mut library_cards);
            }
            crate::effects::helpers::view_hidden_candidate_objects(
                game,
                ctx,
                request.chooser,
                &library_cards,
                "Search library",
                false,
            );
        }
        if let Some(owner) = request.library_owner.filter(|_| access_now(game, ctx)) {
            completed_outputs = offer_library_search_casts(game, ctx, owner)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effect::EffectOutcome::count(0));
            }
        }
        let event = (request.search_library
            && (request.library_owner.is_none() || access_now(game, ctx)))
        .then(|| {
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::SearchLibraryEvent::new(request.chooser, request.library_owner),
                ctx.provenance,
            )
        });
        body(
            game,
            ctx,
            LibrarySearchScope {
                chooser: request.chooser,
                library_cards,
                found_card_policy,
                event,
                completed_outputs,
            },
        )
    })();
    if result.is_ok() && ctx.decision_maker.awaiting_choice() {
        game.capture_pending_decision_controllers();
    }
    // The active control scope always unwinds; only captured pending routing survives.
    finish_opposition_agent_search_control(game, control);
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OppositionAgentSearch {
    pub controller: PlayerId,
    pub source: ObjectId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OppositionAgentFoundCardPermission {
    controller: PlayerId,
    source: ObjectId,
}

#[derive(Debug, Clone)]
struct LibrarySearchCastOption {
    casting_method: CastingMethod,
    method_label: Option<String>,
}

pub(crate) fn opposition_agent_search(
    game: &GameState,
    searching_player: PlayerId,
    library_owner: PlayerId,
) -> Option<OppositionAgentSearch> {
    if searching_player != library_owner {
        return None;
    }

    let mut latest = None;
    let mut latest_timestamp = 0;
    for &source in &game.battlefield {
        let Some(object) = game.object(source) else {
            continue;
        };
        let controller = game.controller_of(object);
        if controller == searching_player {
            continue;
        }
        if game.current_has_static_ability_id(
            source,
            StaticAbilityId::ControlOpponentsWhileSearchingLibraries,
        ) && game
            .current_has_static_ability_id(source, StaticAbilityId::OpponentSearchExileFoundCards)
        {
            let timestamp = game
                .effect_store
                .continuous_effects
                .get_entry_timestamp(source)
                .unwrap_or(0);
            if latest.is_none() || timestamp >= latest_timestamp {
                latest = Some(OppositionAgentSearch { controller, source });
                latest_timestamp = timestamp;
            }
        }
    }

    latest
}

pub(crate) fn begin_opposition_agent_search_control(
    game: &mut GameState,
    searching_player: PlayerId,
    search: Option<OppositionAgentSearch>,
) -> Option<u64> {
    search.map(|search| {
        game.add_scoped_player_control(search.controller, searching_player, Some(search.source))
    })
}

pub(crate) fn finish_opposition_agent_search_control(game: &mut GameState, token: Option<u64>) {
    if let Some(token) = token {
        game.remove_scoped_player_control(token);
    }
}

fn grant_opposition_agent_play_permission(
    game: &mut GameState,
    card_id: ObjectId,
    permission: OppositionAgentFoundCardPermission,
) {
    let Some(object) = game.object(card_id) else {
        return;
    };
    if object.zone != Zone::Exile {
        return;
    }

    let stable_id = object.stable_id;
    let is_land = object.is_land();
    game.effect_store.grant_registry.grant_to_card(
        card_id,
        Zone::Exile,
        permission.controller,
        Grantable::PlayFrom,
        GrantSource::Effect {
            source_id: permission.source,
            expires_end_of_turn: u32::MAX,
        },
    );

    if !is_land {
        game.effect_store
            .mana_spend_effects
            .permissions
            .push(ActiveManaSpendPermission {
                play_permission_identities: None,
                permission: ManaSpendPermission::any_color_for_casting_stable_ids(
                    PlayerFilter::You,
                    vec![stable_id],
                ),
                controller: permission.controller,
                source: ManaSpendPermissionSource::Effect {
                    source_id: permission.source,
                    expires_end_of_turn: u32::MAX,
                },
            });
    }
}

#[derive(Debug)]
#[must_use = "finish found-card movements and retain all replacement instructions"]
pub(crate) struct FoundCardsExileReceipt {
    pub moved_ids: Vec<ObjectId>,
    pub receipts: Vec<(
        ObjectId,
        crate::events::processing::PreparedEventOutcome<crate::effects::zones::AppliedZoneChange>,
    )>,
}

pub(crate) fn move_found_card_for_opposition_agent_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    card_id: ObjectId,
    search: OppositionAgentSearch,
) -> Result<
    crate::events::processing::CommittedZoneChange<crate::effects::zones::AppliedZoneChange>,
    ExecutionError,
> {
    use crate::events::processing::{EventOutcome, PreparedEventOutcome};
    let Some(from) = game.object(card_id).map(|card| card.zone) else {
        return Ok(
            crate::events::processing::CommittedZoneChange::from_receipt(PreparedEventOutcome {
                original: EventOutcome::NotApplicable,
                programs: Vec::new(),
            }),
        );
    };
    let additional = ctx.additional_replacement_effects_snapshot();
    let receipt =
        crate::effects::zones::apply_zone_change_with_context_and_additional_effects_with_outputs(
            game,
            card_id,
            from,
            Zone::Exile,
            ctx.cause.clone(),
            ctx,
            &additional,
        )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(receipt);
    }
    let permission = OppositionAgentFoundCardPermission {
        controller: search.controller,
        source: search.source,
    };
    let ids = crate::effects::zones::movement_arrivals(game, card_id, &receipt.receipt);
    for id in ids {
        if game.object(id).is_some_and(|card| card.zone == Zone::Exile) {
            game.add_exiled_with_source_link(search.source, id);
            grant_opposition_agent_play_permission(game, id, permission);
        }
    }
    Ok(receipt)
}

#[allow(dead_code)]
pub(crate) fn exile_found_cards_for_opposition_agent(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    cards: &[ObjectId],
    searching_player: PlayerId,
) -> Result<FoundCardsExileReceipt, ExecutionError> {
    exile_found_cards_for_opposition_agent_with_outputs(game, ctx, cards, searching_player)
        .map(|(receipt, _)| receipt)
}

pub(crate) fn exile_found_cards_for_opposition_agent_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    cards: &[ObjectId],
    searching_player: PlayerId,
) -> Result<
    (
        FoundCardsExileReceipt,
        Vec<crate::effects::PublishedEffectOutputs>,
    ),
    ExecutionError,
> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok((
            FoundCardsExileReceipt {
                moved_ids: Vec::new(),
                receipts: Vec::new(),
            },
            Vec::new(),
        ));
    }
    let mut published_outputs = Vec::new();
    let result = crate::effects::composition::execute_result_checkpoint_transaction(
        game,
        ctx,
        |game, ctx| {
            let replacements: Vec<_> = game
                .battlefield
                .iter()
                .filter_map(|&source| {
                    let object = game.object(source)?;
                    let controller = game.controller_of(object);
                    (controller != searching_player
                        && game.current_has_static_ability_id(
                            source,
                            StaticAbilityId::OpponentSearchExileFoundCards,
                        ))
                    .then_some(OppositionAgentSearch { controller, source })
                })
                .collect();

            // Each found card has competing exile-and-play-permission replacements.
            // Collect the choices before changing zones, so an interactive pause never
            // commits a partial search. The search's scoped player control is still
            // active when the owner makes this replacement decision.
            let mut selected = Vec::new();
            for &card_id in cards {
                let Some(card) = game.object(card_id) else {
                    continue;
                };
                let owner = card.owner;
                let card_name = card.name.to_string();
                let index = if replacements.len() > 1 {
                    let options = replacements
                .iter()
                .enumerate()
                .map(|(index, replacement)| {
                    let source_name = game
                        .current_name(replacement.source)
                        .unwrap_or_else(|| "Unknown object".into());
                    let controller_name = game
                        .player(replacement.controller)
                        .map(|player| player.name.to_string())
                        .unwrap_or_else(|| "that player".into());
                    ReplacementOption::new(
                        index,
                        replacement.source,
                        format!("{source_name}\nExile {card_name}; {controller_name} may play it while it remains exiled."),
                    )
                    .with_related_objects(vec![replacement.source, card_id])
                })
                .collect();
                    let index = make_decision_with_fallback(
                        game,
                        &mut *ctx.decision_maker,
                        owner,
                        Some(ctx.source),
                        ReplacementSpec::new(options),
                        FallbackStrategy::FirstOption,
                    );
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(FoundCardsExileReceipt {
                            moved_ids: Vec::new(),
                            receipts: Vec::new(),
                        });
                    }
                    match index.as_slice() {
                    [index] => *index,
                    _ => return Err(ExecutionError::InternalError(
                        "found-card exile replacement choice must name exactly one offered effect"
                            .into(),
                    )),
                }
                } else {
                    0
                };
                let replacement = replacements.get(index).ok_or_else(|| {
                    ExecutionError::InternalError(
                        "found-card exile replacement choice is invalid".into(),
                    )
                })?;
                selected.push((card_id, *replacement));
            }
            let opened_batch = game.open_simultaneous_action();
            let movements = (|| -> Result<FoundCardsExileReceipt, ExecutionError> {
                let mut moved_ids = Vec::new();
                let mut receipts = Vec::new();
                for (card, replacement) in selected {
                    let committed = move_found_card_for_opposition_agent_with_outputs(
                        game,
                        ctx,
                        card,
                        replacement,
                    )?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(FoundCardsExileReceipt {
                            moved_ids: Vec::new(),
                            receipts: Vec::new(),
                        });
                    }
                    crate::effects::PublishedEffectOutputs::append_distinct(
                        &mut published_outputs,
                        committed.published_outputs,
                    );
                    let receipt = committed.receipt;
                    moved_ids.extend(crate::effects::zones::movement_arrivals(
                        game, card, &receipt,
                    ));
                    receipts.push((card, receipt));
                }
                Ok(FoundCardsExileReceipt {
                    moved_ids,
                    receipts,
                })
            })();
            game.close_simultaneous_action(opened_batch);
            movements
        },
    );
    if ctx.decision_maker.awaiting_choice() {
        return result.map(|_| {
            (
                FoundCardsExileReceipt {
                    moved_ids: Vec::new(),
                    receipts: Vec::new(),
                },
                Vec::new(),
            )
        });
    }
    result.map(|receipt| (receipt, published_outputs))
}

pub(crate) fn offer_library_search_casts(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    library_owner: PlayerId,
) -> Result<Vec<crate::effects::PublishedEffectOutputs>, ExecutionError> {
    let mut completed_outputs = Vec::new();
    let library_cards = game
        .player(library_owner)
        .map(|player| player.library.clone())
        .unwrap_or_default();

    for card_id in library_cards {
        if !game.current_has_static_ability_id(
            card_id,
            StaticAbilityId::CastThisCardFromLibraryWhileSearching,
        ) {
            continue;
        }
        let cast_options = library_search_cast_options(game, card_id, library_owner);
        if cast_options.is_empty() {
            continue;
        }

        let card_name = game
            .object(card_id)
            .map(|object| object.name.to_string())
            .unwrap_or_else(|| "this card".to_string());
        let owner_name = game
            .player(library_owner)
            .map(|player| player.name.to_string())
            .unwrap_or_else(|| "that player".to_string());

        for option in cast_options {
            let prompt = match option.method_label.as_deref() {
                Some(label) => format!(
                    "cast {card_name} from {owner_name}'s library while searching it using {label}"
                ),
                None => {
                    format!("cast {card_name} from {owner_name}'s library while searching it")
                }
            };
            let accept = make_decision_with_fallback(
                game,
                ctx.decision_maker,
                library_owner,
                Some(card_id),
                MaySpec::new(card_id, prompt),
                FallbackStrategy::Decline,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(completed_outputs);
            }
            if !accept {
                continue;
            }

            if let Some(outputs) = cast_from_library_while_searching(
                game,
                ctx,
                card_id,
                library_owner,
                option.casting_method,
            )? {
                completed_outputs.push(outputs);
            }
            if ctx.decision_maker.awaiting_choice() {
                return Ok(completed_outputs);
            }
            break;
        }
    }

    Ok(completed_outputs)
}

fn library_search_cast_options(
    game: &GameState,
    card_id: ObjectId,
    caster: PlayerId,
) -> Vec<LibrarySearchCastOption> {
    let Some(object) = game.object(card_id) else {
        return Vec::new();
    };
    if !is_library_search_cast_candidate(game, card_id, object) {
        return Vec::new();
    }

    let view = DerivedGameView::new(game);
    let mut options = Vec::new();

    let normal_method = CastingMethod::PlayFrom {
        source: card_id,
        zone: Zone::Library,
        use_alternative: None,
    };
    if library_search_casting_method_is_legal(game, caster, object, &normal_method, &view)
        && library_search_cast_method_supported_for_execution(game, caster, object, &normal_method)
    {
        options.push(LibrarySearchCastOption {
            casting_method: normal_method,
            method_label: None,
        });
    }

    for (idx, method) in object.alternative_casts.iter().enumerate() {
        if method.cast_from_zone() != Zone::Library {
            continue;
        }
        let casting_method = CastingMethod::PlayFrom {
            source: card_id,
            zone: Zone::Library,
            use_alternative: Some(idx),
        };
        if library_search_casting_method_is_legal(game, caster, object, &casting_method, &view)
            && library_search_cast_method_supported_for_execution(
                game,
                caster,
                object,
                &casting_method,
            )
        {
            options.push(LibrarySearchCastOption {
                method_label: Some(format_alternative_library_search_method(
                    game,
                    caster,
                    object,
                    &casting_method,
                )),
                casting_method,
            });
        }
    }

    let granted_alternatives =
        view.granted_alternative_casts_for_card(card_id, Zone::Library, caster);
    let base_alt_idx = object.alternative_casts.len();
    for (offset, grant) in granted_alternatives.iter().enumerate() {
        let casting_method = CastingMethod::PlayFrom {
            source: grant.source_id,
            zone: Zone::Library,
            use_alternative: Some(base_alt_idx + offset),
        };
        if library_search_casting_method_is_legal(game, caster, object, &casting_method, &view)
            && library_search_cast_method_supported_for_execution(
                game,
                caster,
                object,
                &casting_method,
            )
        {
            options.push(LibrarySearchCastOption {
                method_label: Some(format_alternative_library_search_method(
                    game,
                    caster,
                    object,
                    &casting_method,
                )),
                casting_method,
            });
        }
    }

    options
}

fn is_library_search_cast_candidate(
    game: &GameState,
    card_id: ObjectId,
    object: &crate::object::Object,
) -> bool {
    object.zone == Zone::Library
        && !object.is_land()
        && game.current_has_static_ability_id(
            card_id,
            StaticAbilityId::CastThisCardFromLibraryWhileSearching,
        )
}

fn library_search_casting_method_is_legal(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    if !matches!(
        casting_method,
        CastingMethod::PlayFrom {
            zone: Zone::Library,
            ..
        }
    ) {
        return false;
    }

    let ctx = CastLegalityContext::new(game, caster, view).with_library_search_cast_timing();
    match casting_method {
        CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => can_cast_spell_with_context(spell, casting_method, &ctx),
        CastingMethod::PlayFrom {
            use_alternative: Some(_),
            ..
        } => {
            let Some(method) =
                library_search_alternative_method(game, caster, spell, casting_method)
            else {
                return false;
            };
            if !library_search_alternative_condition_allows(game, caster, spell, &method) {
                return false;
            }
            let base_cost =
                spell_mana_cost_for_cast(game, caster, spell, casting_method, Zone::Library);
            if base_cost.is_none() && alternative_method_uses_printed_mana_cost(&method) {
                return false;
            }
            let requirements = build_requirements_for_method(&method);
            can_cast_with_cost_with_context(
                spell,
                spell.id,
                base_cost.as_ref(),
                method
                    .overload_effects()
                    .or_else(|| method.cleave_effects()),
                &requirements,
                casting_method,
                &ctx,
            ) && library_search_non_mana_costs_are_payable(game, caster, spell, &method)
        }
        _ => false,
    }
}

fn library_search_alternative_method(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> Option<AlternativeCastingMethod> {
    match casting_method {
        CastingMethod::PlayFrom {
            zone,
            use_alternative: Some(idx),
            ..
        } => resolve_play_from_alternative_method(game, caster, spell, *zone, *idx),
        _ => None,
    }
}

fn library_search_alternative_condition_allows(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    method: &AlternativeCastingMethod,
) -> bool {
    if let Some(condition) = method.cast_condition() {
        crate::static_abilities::this_spell_cost_condition_is_active_for_player(
            game,
            spell.id,
            caster,
            condition,
            &[],
        )
    } else if let Some(condition) = method.trap_condition() {
        crate::decision::is_trap_condition_met(game, caster, condition)
    } else {
        true
    }
}

fn library_search_non_mana_costs_are_payable(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    method: &AlternativeCastingMethod,
) -> bool {
    let check_ctx = crate::costs::CostCheckContext::new(spell.id, caster)
        .with_reason(crate::costs::PaymentReason::CastSpell);
    method.non_mana_costs().into_iter().all(|cost| {
        game.validate_cost_for_payment_reason(caster, spell.id, &cost, check_ctx.reason)
            .is_ok()
            && crate::costs::can_pay_with_check_context(&*cost.0, game, &check_ctx).is_ok()
    })
}

fn library_search_cast_method_supported_for_execution(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    let Some(base_cost) =
        spell_mana_cost_for_cast(game, caster, spell, casting_method, Zone::Library)
    else {
        return false;
    };
    if base_cost.has_x() || !spell.additional_non_mana_costs().is_empty() {
        return false;
    }
    if library_search_alternative_method(game, caster, spell, casting_method)
        .is_some_and(|method| !method.non_mana_costs().is_empty())
    {
        return false;
    }
    !library_search_cast_requires_target_selection(game, caster, spell, casting_method)
}

fn library_search_cast_requires_target_selection(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    let Some(program) = library_search_cast_effect_program(game, caster, spell, casting_method)
    else {
        return false;
    };
    !crate::game_loop::extract_target_requirements_from_program_with_modes(
        game,
        &program,
        caster,
        Some(spell.id),
        None,
    )
    .is_empty()
}

fn library_search_cast_effect_program(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> Option<ResolutionProgram> {
    if let Some(method) = library_search_alternative_method(game, caster, spell, casting_method)
        && let Some(effects) = method
            .overload_effects()
            .or_else(|| method.cleave_effects())
    {
        return Some(ResolutionProgram::from_effects(effects.to_vec()));
    }
    spell.spell_effect_owned()
}

fn format_alternative_library_search_method(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> String {
    let method_name = library_search_alternative_method(game, caster, spell, casting_method)
        .map(|method| method.name().to_string())
        .unwrap_or_else(|| "alternative cost".to_string());
    let cost = spell_mana_cost_for_cast(game, caster, spell, casting_method, Zone::Library)
        .map(|cost| {
            if cost.is_empty() {
                "free".to_string()
            } else {
                cost.to_oracle()
            }
        })
        .unwrap_or_else(|| "no mana cost".to_string());
    format!("{method_name} ({cost})")
}

fn cast_from_library_while_searching(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    card_id: ObjectId,
    caster: PlayerId,
    casting_method: CastingMethod,
) -> Result<Option<crate::effects::PublishedEffectOutputs>, ExecutionError> {
    {
        let Some(object) = game.object(card_id) else {
            return Ok(None);
        };
        if !is_library_search_cast_candidate(game, card_id, object) {
            return Ok(None);
        }
        let view = DerivedGameView::new(game);
        if !library_search_casting_method_is_legal(game, caster, object, &casting_method, &view)
            || !library_search_cast_method_supported_for_execution(
                game,
                caster,
                object,
                &casting_method,
            )
        {
            return Ok(None);
        }
    }

    let result = crate::game_loop::cast_spell_from_resolving_effect_with_outputs(
        game,
        card_id,
        Zone::Library,
        caster,
        &casting_method,
        false,
        None,
        ctx.provenance,
        &mut ctx.decision_maker,
    )
    .map_err(|error| ExecutionError::Impossible(error.to_string()))?;
    let Some(cast) = result else {
        return Ok(None);
    };

    let (mut outputs, mut captured) = crate::game_loop::capture_completed_spell_cast_with_outputs(
        game,
        cast.new_id,
        caster,
        Zone::Library,
        ctx.provenance,
    )?;
    game.defer_trigger_entries(captured.take_all());
    game.queue_trigger_event(ctx.provenance, outputs.outcome.events[0].clone());
    outputs.retain_published_references([cast.outputs]);

    Ok(Some(crate::effects::PublishedEffectOutputs::retain(
        outputs,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::cards::definitions::basic_forest;
    use crate::color::Color;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectOptionsContext;
    use crate::effect::{ChoiceCount, Effect};
    use crate::effects::{EffectExecutor, ForEachTaggedEffect, PutOntoBattlefieldEffect};
    use crate::filter::ObjectFilter;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::tag::TagKey;
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::types::{CardType, Subtype};

    struct FinalPartingDecisionMaker {
        alice: PlayerId,
        chosen_names: Vec<String>,
        boolean_players: Vec<PlayerId>,
        object_players: Vec<PlayerId>,
    }

    impl FinalPartingDecisionMaker {
        fn new(alice: PlayerId, chosen_names: &[&str]) -> Self {
            Self {
                alice,
                chosen_names: chosen_names
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect(),
                boolean_players: Vec::new(),
                object_players: Vec::new(),
            }
        }
    }

    impl DecisionMaker for FinalPartingDecisionMaker {
        fn decide_boolean(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            let decision_player = game.controlling_player_for(ctx.player);
            self.boolean_players.push(decision_player);
            assert_eq!(decision_player, self.alice);
            true
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let decision_player = game.controlling_player_for(ctx.player);
            self.object_players.push(decision_player);
            assert_eq!(decision_player, self.alice);
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .filter(|candidate| {
                    game.object(candidate.id).is_some_and(|object| {
                        self.chosen_names.iter().any(|name| name == &object.name)
                    })
                })
                .map(|candidate| candidate.id)
                .collect()
        }
    }

    struct PromptOnlyDecisionMaker {
        pending: bool,
        boolean_players: Vec<PlayerId>,
        object_players: Vec<PlayerId>,
    }

    impl PromptOnlyDecisionMaker {
        fn new() -> Self {
            Self {
                pending: false,
                boolean_players: Vec::new(),
                object_players: Vec::new(),
            }
        }
    }

    impl DecisionMaker for PromptOnlyDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.pending
        }

        fn decide_boolean(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.pending = true;
            self.boolean_players
                .push(game.controlling_player_for(ctx.player));
            false
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.object_players
                .push(game.controlling_player_for(ctx.player));
            Vec::new()
        }
    }

    struct SearchCastManaAbilityDecisionMaker {
        controller: PlayerId,
        chosen_name: String,
        boolean_players: Vec<PlayerId>,
        object_players: Vec<PlayerId>,
        mana_payment_players: Vec<PlayerId>,
    }

    impl SearchCastManaAbilityDecisionMaker {
        fn new(controller: PlayerId, chosen_name: &str) -> Self {
            Self {
                controller,
                chosen_name: chosen_name.to_string(),
                boolean_players: Vec::new(),
                object_players: Vec::new(),
                mana_payment_players: Vec::new(),
            }
        }
    }

    impl DecisionMaker for SearchCastManaAbilityDecisionMaker {
        fn decide_boolean(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            let decision_player = game.controlling_player_for(ctx.player);
            self.boolean_players.push(decision_player);
            assert_eq!(decision_player, self.controller);
            true
        }

        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            let decision_player = game.controlling_player_for(ctx.player);
            self.mana_payment_players.push(decision_player);
            assert_eq!(decision_player, self.controller);
            ctx.options
                .iter()
                .find(|option| option.legal)
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let decision_player = game.controlling_player_for(ctx.player);
            self.object_players.push(decision_player);
            assert_eq!(decision_player, self.controller);
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .filter(|candidate| {
                    game.object(candidate.id)
                        .is_some_and(|object| object.name == self.chosen_name)
                })
                .map(|candidate| candidate.id)
                .collect()
        }
    }

    struct SearchCastColorChoiceDecisionMaker {
        controller: PlayerId,
        chosen_name: String,
        boolean_players: Vec<PlayerId>,
        object_players: Vec<PlayerId>,
        mana_payment_players: Vec<PlayerId>,
        color_players: Vec<PlayerId>,
        selected_mana_plans: Vec<crate::mana_payment::ManaPaymentPlan>,
    }

    impl SearchCastColorChoiceDecisionMaker {
        fn new(controller: PlayerId, chosen_name: &str) -> Self {
            Self {
                controller,
                chosen_name: chosen_name.to_string(),
                boolean_players: Vec::new(),
                object_players: Vec::new(),
                mana_payment_players: Vec::new(),
                color_players: Vec::new(),
                selected_mana_plans: Vec::new(),
            }
        }
    }

    impl DecisionMaker for SearchCastColorChoiceDecisionMaker {
        fn decide_boolean(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            let decision_player = game.controlling_player_for(ctx.player);
            self.boolean_players.push(decision_player);
            assert_eq!(decision_player, self.controller);
            true
        }

        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            let decision_player = game.controlling_player_for(ctx.player);
            self.mana_payment_players.push(decision_player);
            assert_eq!(decision_player, self.controller);
            ctx.options
                .iter()
                .find(|option| option.legal)
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }

        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::ManaPaymentContext,
        ) -> crate::mana_payment::ManaPaymentResponse {
            let decision_player = game.controlling_player_for(ctx.player);
            self.mana_payment_players.push(decision_player);
            assert_eq!(decision_player, self.controller);
            self.selected_mana_plans.push(ctx.plan.clone());
            crate::mana_payment::ManaPaymentResponse::Confirm {
                plan_id: ctx.plan.id,
                request_hash: ctx.plan.request_hash,
            }
        }

        fn decide_colors(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::ColorsContext,
        ) -> Vec<Color> {
            let decision_player = game.controlling_player_for(ctx.player);
            self.color_players.push(decision_player);
            if decision_player == self.controller {
                vec![Color::Green; ctx.count as usize]
            } else {
                vec![Color::White; ctx.count as usize]
            }
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let decision_player = game.controlling_player_for(ctx.player);
            self.object_players.push(decision_player);
            assert_eq!(decision_player, self.controller);
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .filter(|candidate| {
                    game.object(candidate.id)
                        .is_some_and(|object| object.name == self.chosen_name)
                })
                .map(|candidate| candidate.id)
                .collect()
        }
    }

    fn opposition_agent_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Opposition Agent")
            .card_types(vec![CardType::Creature])
            .with_ability(Ability::static_ability(
                StaticAbility::control_opponents_while_searching_libraries(),
            ))
            .with_ability(Ability::static_ability(
                StaticAbility::opponent_search_exile_found_cards(),
            ))
            .build()
    }

    struct CompetingAgentsDecisionMaker {
        searching_player: PlayerId,
        controller: PlayerId,
        chosen_source: ObjectId,
        pause: bool,
        pending: bool,
        replacement_choices: usize,
    }

    impl DecisionMaker for CompetingAgentsDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.pending
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates
                .iter()
                .filter(|card| card.legal)
                .map(|card| card.id)
                .collect()
        }

        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            assert_eq!(ctx.player, self.searching_player);
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert!(ctx.description.contains("replacement effect"));
            assert_eq!(ctx.options.len(), 2);
            self.replacement_choices += 1;
            self.pending = self.pause;
            vec![
                ctx.options
                    .iter()
                    .find(|option| option.object_id == Some(self.chosen_source))
                    .unwrap()
                    .index,
            ]
        }
    }

    #[test]
    fn opposition_agent_latest_controls_search_and_chooses_either_exile_permission() {
        for choose_latest in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let older = game.create_object_from_definition(
                &opposition_agent_definition(),
                bob,
                Zone::Battlefield,
            );
            let latest = game.create_object_from_definition(
                &opposition_agent_definition(),
                charlie,
                Zone::Battlefield,
            );
            let card = game.create_object_from_card(
                &library_spell_card("Found card"),
                alice,
                Zone::Library,
            );
            let search = opposition_agent_search(&game, alice, alice).unwrap();
            assert_eq!(
                search.source, latest,
                "the most recent Agent controls the search"
            );
            let mut dm = CompetingAgentsDecisionMaker {
                searching_player: alice,
                controller: charlie,
                chosen_source: if choose_latest { latest } else { older },
                pause: false,
                pending: false,
                replacement_choices: 0,
            };
            let mut ctx = ExecutionContext::new(ObjectId::from_raw(99_997), alice, &mut dm);
            crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default().in_zone(Zone::Library),
                ChoiceCount::exactly(1),
                PlayerFilter::You,
                TagKey::from("searched"),
            )
            .in_zone(Zone::Library)
            .as_search()
            .execute(&mut game, &mut ctx)
            .unwrap();
            assert_eq!(dm.replacement_choices, 1);
            assert!(game.object(card).is_none());
            let exiled = game.exile[0];
            for player in [alice, bob, charlie] {
                assert_eq!(
                    game.effect_store.grant_registry.card_can_play_from_zone(
                        &game,
                        exiled,
                        Zone::Exile,
                        player
                    ),
                    player == if choose_latest { charlie } else { bob },
                );
            }
            assert_eq!(
                game.controlling_player_for(alice),
                alice,
                "search control ends after the choice"
            );
        }
    }

    #[test]
    fn opposition_agent_waits_for_exile_choice_without_moving_found_cards() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let older = game.create_object_from_definition(
            &opposition_agent_definition(),
            bob,
            Zone::Battlefield,
        );
        game.create_object_from_definition(
            &opposition_agent_definition(),
            charlie,
            Zone::Battlefield,
        );
        let card =
            game.create_object_from_card(&library_spell_card("Found card"), alice, Zone::Library);
        let mut dm = CompetingAgentsDecisionMaker {
            searching_player: alice,
            controller: charlie,
            chosen_source: older,
            pause: true,
            pending: false,
            replacement_choices: 0,
        };
        let mut ctx = ExecutionContext::new(ObjectId::from_raw(99_997), alice, &mut dm);
        crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::default().in_zone(Zone::Library),
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            TagKey::from("searched"),
        )
        .in_zone(Zone::Library)
        .as_search()
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert!(dm.pending);
        assert_eq!(game.object(card).unwrap().zone, Zone::Library);
        assert!(game.exile.is_empty());
        assert_eq!(game.controlling_player_for(alice), charlie);
    }

    struct ResumedSearchDecisionMaker {
        inner: CompetingAgentsDecisionMaker,
        invalid: bool,
    }

    impl DecisionMaker for ResumedSearchDecisionMaker {
        fn awaiting_choice(&self) -> bool { self.inner.awaiting_choice() }
        fn decide_objects(&mut self, game: &GameState, ctx: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            self.inner.decide_objects(game, ctx)
        }
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            let selected = self.inner.decide_options(game, ctx);
            if self.invalid && !self.inner.pending { Vec::new() } else { selected }
        }
    }

    #[test]
    fn pending_search_recreated_context_restores_prior_control_on_completion_and_error() {
        for invalid in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let prior_control = game.add_scoped_player_control(bob, alice, None);
            let older = game.create_object_from_definition(&opposition_agent_definition(), bob, Zone::Battlefield);
            game.create_object_from_definition(&opposition_agent_definition(), charlie, Zone::Battlefield);
            let card = game.create_object_from_card(&library_spell_card("Found card"), alice, Zone::Library);
            let source = ObjectId::from_raw(99_997);
            let effect = crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default().in_zone(Zone::Library), ChoiceCount::exactly(1),
                PlayerFilter::You, TagKey::from("searched"),
            ).in_zone(Zone::Library).as_search();
            let mut dm = ResumedSearchDecisionMaker {
                inner: CompetingAgentsDecisionMaker {
                    searching_player: alice, controller: charlie, chosen_source: older,
                    pause: true, pending: false, replacement_choices: 0,
                }, invalid,
            };
            {
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                effect.execute(&mut game, &mut ctx).unwrap();
            }
            assert!(dm.inner.pending);
            assert_eq!(game.object(card).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            dm.inner.pause = false;
            dm.inner.pending = false;
            let result = {
                // Gameplay replay recreates this context; a private context token alone
                // cannot own retained search control across this boundary.
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                effect.execute(&mut game, &mut ctx)
            };
            assert!(!dm.inner.pending);
            assert_eq!(dm.inner.replacement_choices, 2);
            if invalid {
                assert!(result.is_err(), "invalid raw replacement answer must fail");
                assert_eq!(game.object(card).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty());
            } else {
                result.unwrap();
                assert!(game.object(card).is_none());
                assert_eq!(game.exile.len(), 1);
                let exiled = game.exile[0];
                for player in [alice, bob, charlie] {
                    assert_eq!(game.effect_store.grant_registry.card_can_play_from_zone(
                        &game, exiled, Zone::Exile, player,
                    ), player == bob);
                }
            }
            assert_eq!(game.controlling_player_for(alice), bob, "search scope must finish without removing or shadowing the outer scope");
            game.remove_scoped_player_control(prior_control);
            assert_eq!(game.controlling_player_for(alice), alice, "no retained search token may leak after completion or failure");
        }
    }

    #[test]
    fn pending_control_view_keeps_innermost_prompt_when_outer_scopes_unwind() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let outer = game.add_scoped_player_control(bob, alice, None);
        let checkpoint = game.clone();
        let inner = game.add_scoped_player_control(charlie, alice, None);
        game.capture_pending_decision_controllers();
        game.remove_scoped_player_control(inner);
        // Both instruction owners encounter the same pending choice while
        // unwinding. Its actual controller was captured by the inner owner.
        game.capture_pending_decision_controllers();
        game.restore_execution_checkpoint(checkpoint, true);
        assert_eq!(game.controlling_player_for(alice), charlie);
        game.clear_pending_decision_controllers();
        assert_eq!(game.controlling_player_for(alice), bob);
        game.remove_scoped_player_control(outer);
        assert_eq!(game.controlling_player_for(alice), alice);
    }

    #[test]
    fn pending_control_view_does_not_resurrect_a_departed_source_after_rollback() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let outer = game.add_scoped_player_control(bob, alice, None);
        let source = game.create_object_from_card(&library_spell_card("Control source"), charlie, Zone::Battlefield);
        let checkpoint = game.clone();
        let inner = game.add_scoped_player_control(charlie, alice, Some(source));
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(game.controlling_player_for(alice), bob);
        game.capture_pending_decision_controllers();
        game.remove_scoped_player_control(inner);
        game.restore_execution_checkpoint(checkpoint, true);
        assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.controlling_player_for(alice), bob, "routing comes from the prompt state, not reapplying restored source tokens");
        game.clear_pending_decision_controllers();
        assert_eq!(game.controlling_player_for(alice), bob);
        game.remove_scoped_player_control(outer);
        assert_eq!(game.controlling_player_for(alice), alice);
    }

    #[test]
    fn failed_instruction_discards_pending_control_view_and_preserves_outer_scope() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let outer = game.add_scoped_player_control(bob, alice, None);
        let checkpoint = game.clone();
        game.add_scoped_player_control(charlie, alice, None);
        game.capture_pending_decision_controllers();
        game.restore_execution_checkpoint(checkpoint, false);
        assert_eq!(game.controlling_player_for(alice), bob);
        game.remove_scoped_player_control(outer);
        assert_eq!(game.controlling_player_for(alice), alice);
    }

    #[test]
    fn full_stack_search_pending_resume_restores_outer_control_on_completion_and_error() {
        for invalid in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let outer = game.add_scoped_player_control(bob, alice, None);
            let older = game.create_object_from_definition(&opposition_agent_definition(), bob, Zone::Battlefield);
            game.create_object_from_definition(&opposition_agent_definition(), charlie, Zone::Battlefield);
            let card = game.create_object_from_card(&library_spell_card("Found card"), alice, Zone::Library);
            let stable = game.object(card).unwrap().stable_id;
            game.push_to_stack(crate::game_state::StackEntry::ability(older, alice,
                vec![crate::effect::Effect::search_library_to_hand(ObjectFilter::default(), false)]));
            game.take_pending_trigger_events();
            let mut dm = ResumedSearchDecisionMaker {
                inner: CompetingAgentsDecisionMaker {
                    searching_player: alice, controller: charlie, chosen_source: older,
                    pause: true, pending: false, replacement_choices: 0,
                }, invalid,
            };
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(dm.inner.pending);
            assert_eq!(game.stack.len(), 1, "pending stack resolution retains its root");
            assert_eq!(game.object(card).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(game.controlling_player_for(alice), charlie, "outer resolution rollback preserves the actual pending controller");
            dm.inner.pause = false;
            dm.inner.pending = false;
            let result = crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm);
            assert!(!dm.inner.pending);
            assert_eq!(dm.inner.replacement_choices, 2);
            if invalid {
                assert!(result.is_err());
                assert_eq!(game.stack.len(), 1);
                assert_eq!(game.object(card).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty());
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                result.unwrap();
                assert!(game.stack.is_empty());
                let arrival = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
                assert!(game.player(alice).unwrap().hand.is_empty());
                for player in [alice, bob, charlie] {
                    assert_eq!(game.effect_store.grant_registry.card_can_play_from_zone(
                        &game, arrival, Zone::Exile, player), player == bob);
                }
            }
            assert_eq!(game.controlling_player_for(alice), bob);
            game.remove_scoped_player_control(outer);
            assert_eq!(game.controlling_player_for(alice), alice);
        }
    }

    struct NestedSearchAnswers {
        inner_player: PlayerId,
        controller: PlayerId,
        selected_agent: ObjectId,
        pause: bool,
        pending: bool,
        replacement_choices: usize,
    }
    impl DecisionMaker for NestedSearchAnswers {
        fn awaiting_choice(&self) -> bool { self.pending }
        fn decide_objects(&mut self, game: &GameState, ctx: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates.iter().filter(|card| card.legal).map(|card| card.id).collect()
        }
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert_eq!(ctx.options.len(), 2);
            self.replacement_choices += 1;
            self.pending = self.pause && ctx.player == self.inner_player;
            vec![ctx.options.iter().find(|option| option.object_id == Some(self.selected_agent)).unwrap().index]
        }
    }

    #[test]
    fn search_inside_added_replacement_keeps_inner_prompt_control_across_payload_rollback() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into(), "Diana".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let diana = PlayerId::from_index(3);
        let older = game.create_object_from_definition(&opposition_agent_definition(), bob, Zone::Battlefield);
        game.create_object_from_definition(&opposition_agent_definition(), diana, Zone::Battlefield);
        let addition_source = game.create_object_from_card(&library_spell_card("Addition source"), alice, Zone::Battlefield);
        let outer_card = game.create_object_from_card(&library_spell_card("Outer found"), alice, Zone::Library);
        let inner_card = game.create_object_from_card(&library_spell_card("Inner found"), charlie, Zone::Library);
        let outer_stable = game.object(outer_card).unwrap().stable_id;
        let inner_stable = game.object(inner_card).unwrap().stable_id;
        let inner_search = crate::effects::SearchLibraryEffect::to_hand(ObjectFilter::default(), PlayerFilter::Specific(charlie), false);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(
            addition_source, alice,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(outer_card), Some(Zone::Library), Some(Zone::Exile)),
            crate::replacement::ReplacementAction::Additionally(vec![crate::effect::Effect::new(inner_search)]),
        ));
        let effect = crate::effects::ChooseObjectsEffect::new(ObjectFilter::default().in_zone(Zone::Library),
            ChoiceCount::exactly(1), PlayerFilter::You, TagKey::from("searched"))
            .in_zone(Zone::Library).as_search();
        let source = ObjectId::from_raw(99_997);
        let mut dm = NestedSearchAnswers { inner_player: charlie, controller: diana,
            selected_agent: older, pause: true, pending: false, replacement_choices: 0 };
        {
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert!(outcome.events.is_empty());
        }
        assert!(dm.pending);
        assert_eq!(dm.replacement_choices, 2);
        assert_eq!(game.object(outer_card).unwrap().zone, Zone::Library);
        assert_eq!(game.object(inner_card).unwrap().zone, Zone::Library);
        assert!(game.exile.is_empty());
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        assert_eq!(game.controlling_player_for(charlie), diana, "the added-program rollback must retain the actual inner search prompt");
        dm.pause = false;
        dm.pending = false;
        {
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            effect.execute(&mut game, &mut ctx).unwrap();
        }
        assert!(!dm.pending);
        assert_eq!(dm.replacement_choices, 4);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        for stable in [outer_stable, inner_stable] {
            let arrival = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
            for player in [alice, bob, charlie, diana] {
                assert_eq!(game.effect_store.grant_registry.card_can_play_from_zone(&game, arrival, Zone::Exile, player), player == bob);
            }
        }
        assert_eq!(game.controlling_player_for(alice), alice);
        assert_eq!(game.controlling_player_for(charlie), charlie);
    }

    fn panglacial_wurm_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Panglacial Wurm")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(5)],
                vec![ManaSymbol::Green],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(9, 5))
            .with_ability(
                Ability::static_ability(
                    StaticAbility::cast_this_card_from_library_while_searching(),
                )
                .in_zones(vec![Zone::Library]),
            )
            .build()
    }

    fn library_spell_card(name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Artifact])
            .build()
    }

    fn library_plains_island_card(name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Plains, Subtype::Island])
            .build()
    }

    fn colorless_mana_land_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Crystal Vein Stand-In")
            .card_types(vec![CardType::Land])
            .with_ability(Ability::mana(
                crate::cost::TotalCost::free(),
                vec![ManaSymbol::Colorless],
            ))
            .build()
    }

    fn green_white_choice_land_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Green-White Choice Land")
            .card_types(vec![CardType::Land])
            .with_ability(Ability::mana_with_effects(
                crate::cost::TotalCost::free(),
                vec![Effect::add_mana_of_any_color_restricted(
                    1,
                    vec![Color::Green, Color::White],
                )],
            ))
            .build()
    }

    #[test]
    fn panglacial_default_legality_does_not_allow_library_cast_outside_search() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let wurm_id =
            game.create_object_from_definition(&panglacial_wurm_definition(), alice, Zone::Library);
        game.turn.active_player = bob;
        {
            let alice_player = game.player_mut(alice).expect("alice exists");
            alice_player.mana_pool.add(ManaSymbol::Colorless, 5);
            alice_player.mana_pool.add(ManaSymbol::Green, 2);
        }

        let spell = game.object(wurm_id).expect("wurm exists");
        let view = DerivedGameView::new(&game);
        let casting_method = CastingMethod::PlayFrom {
            source: wurm_id,
            zone: Zone::Library,
            use_alternative: None,
        };
        assert!(
            !crate::decision::can_cast_spell_with_view(&game, alice, spell, &casting_method, &view),
            "ordinary legality checks should not expose Panglacial outside a library search"
        );
        assert_eq!(
            library_search_cast_options(&game, wurm_id, alice).len(),
            1,
            "the search-specific legality context should allow Panglacial during a search"
        );
    }

    #[test]
    fn opposition_agent_final_parting_search_exiles_grants_and_allows_panglacial_cast() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let agent_id = game.create_object_from_definition(
            &opposition_agent_definition(),
            alice,
            Zone::Battlefield,
        );
        let _wurm_id =
            game.create_object_from_definition(&panglacial_wurm_definition(), bob, Zone::Library);
        game.create_object_from_card(&library_spell_card("Later Cast A"), bob, Zone::Library);
        game.create_object_from_card(&library_spell_card("Later Cast B"), bob, Zone::Library);
        game.create_object_from_card(&library_spell_card("Unchosen Card"), bob, Zone::Library);

        {
            let bob_player = game.player_mut(bob).expect("bob exists");
            bob_player.mana_pool.add(ManaSymbol::Colorless, 5);
            bob_player.mana_pool.add(ManaSymbol::Green, 2);
        }

        let mut dm = FinalPartingDecisionMaker::new(alice, &["Later Cast A", "Later Cast B"]);
        let source = ObjectId::from_raw(99_999);
        {
            let mut ctx = ExecutionContext::new(source, bob, &mut dm);

            let search_effect = crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default().in_zone(Zone::Library),
                ChoiceCount::exactly(2),
                PlayerFilter::You,
                TagKey::from("searched"),
            )
            .in_zone(Zone::Library)
            .as_search();
            let outcome = search_effect
                .execute(&mut game, &mut ctx)
                .expect("search should resolve");
            assert_eq!(outcome.chosen_objects().unwrap_or_default().len(), 2);
            assert!(
                ctx.get_tagged_all("searched").is_none(),
                "Opposition Agent should consume found cards at search resolution"
            );
        }

        assert!(dm.boolean_players.contains(&alice));
        assert!(dm.object_players.contains(&alice));

        let wurm_stack = game
            .stack
            .iter()
            .find(|entry| {
                game.object(entry.object_id)
                    .is_some_and(|object| object.name == "Panglacial Wurm")
            })
            .expect("Panglacial Wurm should be cast while Bob is searching");
        assert_eq!(wurm_stack.controller, bob);
        assert!(matches!(
            wurm_stack.casting_method,
            CastingMethod::PlayFrom {
                zone: Zone::Library,
                ..
            }
        ));

        let exiled: Vec<_> = game
            .exile
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id).is_some_and(|object| {
                    object.name == "Later Cast A" || object.name == "Later Cast B"
                })
            })
            .collect();
        assert_eq!(exiled.len(), 2);

        for exiled_id in &exiled {
            assert!(
                game.effect_store.grant_registry.card_can_play_from_zone(
                    &game,
                    *exiled_id,
                    Zone::Exile,
                    alice,
                ),
                "Alice should be allowed to play exiled search card"
            );
            assert!(game.can_spend_mana_as_any_color(alice, Some(*exiled_id)));
        }

        game.move_object_by_effect(agent_id, Zone::Graveyard);
        for exiled_id in &exiled {
            assert!(
                game.effect_store.grant_registry.card_can_play_from_zone(
                    &game,
                    *exiled_id,
                    Zone::Exile,
                    alice,
                ),
                "Opposition Agent's play permission should persist after it leaves"
            );
            assert!(
                game.can_spend_mana_as_any_color(alice, Some(*exiled_id)),
                "Opposition Agent's mana permission should persist after it leaves"
            );
        }
    }

    #[test]
    fn opposition_agent_fetchland_put_onto_battlefield_exiles_found_land() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.create_object_from_definition(&opposition_agent_definition(), bob, Zone::Battlefield);
        game.create_object_from_card(
            &library_plains_island_card("Hallowed Fountain"),
            alice,
            Zone::Library,
        );
        game.create_object_from_definition(&panglacial_wurm_definition(), alice, Zone::Library);

        {
            let alice_player = game.player_mut(alice).expect("alice exists");
            alice_player.mana_pool.add(ManaSymbol::Colorless, 5);
            alice_player.mana_pool.add(ManaSymbol::Green, 2);
        }

        let mut dm = FinalPartingDecisionMaker::new(bob, &["Hallowed Fountain"]);
        let source = ObjectId::from_raw(77_777);
        {
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let search_tag = TagKey::from("searched");
            let search_effect = crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default().in_zone(Zone::Library),
                ChoiceCount::exactly(1),
                PlayerFilter::You,
                search_tag.clone(),
            )
            .in_zone(Zone::Library)
            .as_search();
            let outcome = search_effect
                .execute(&mut game, &mut ctx)
                .expect("fetch search should resolve");
            assert_eq!(outcome.chosen_objects().unwrap_or_default().len(), 1);

            let put_onto_battlefield = ForEachTaggedEffect::new(
                search_tag,
                vec![Effect::new(PutOntoBattlefieldEffect::you_control(
                    ChooseSpec::Iterated,
                    false,
                ))],
            );
            put_onto_battlefield
                .execute(&mut game, &mut ctx)
                .expect("printed battlefield follow-up should find no unresolved found cards");
        }

        assert!(dm.boolean_players.contains(&bob));
        assert!(dm.object_players.contains(&bob));
        let wurm_stack = game
            .stack
            .iter()
            .find(|entry| {
                game.object(entry.object_id)
                    .is_some_and(|object| object.name == "Panglacial Wurm")
            })
            .expect(
                "Bob should be able to choose to cast Alice's Panglacial Wurm during her search",
            );
        assert_eq!(wurm_stack.controller, alice);
        assert!(
            game.battlefield.iter().all(|id| {
                game.object(*id)
                    .is_none_or(|object| object.name != "Hallowed Fountain")
            }),
            "found land should not enter Alice's battlefield under Opposition Agent"
        );

        let exiled_id = game
            .exile
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name == "Hallowed Fountain")
            })
            .expect("found land should be exiled");
        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                bob,
            ),
            "Opposition Agent's controller should be able to play the exiled land"
        );
    }

    #[test]
    fn opposition_agent_fetchland_pauses_for_panglacial_prompt_before_land_choice() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.create_object_from_definition(&opposition_agent_definition(), bob, Zone::Battlefield);
        game.create_object_from_card(
            &library_plains_island_card("Hallowed Fountain"),
            alice,
            Zone::Library,
        );
        game.create_object_from_definition(&panglacial_wurm_definition(), alice, Zone::Library);
        {
            let alice_player = game.player_mut(alice).expect("alice exists");
            alice_player.mana_pool.add(ManaSymbol::Colorless, 5);
            alice_player.mana_pool.add(ManaSymbol::Green, 2);
        }

        let mut dm = PromptOnlyDecisionMaker::new();
        {
            let mut ctx = ExecutionContext::new(ObjectId::from_raw(88_888), alice, &mut dm);
            let search_effect = crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default().in_zone(Zone::Library),
                ChoiceCount::exactly(1),
                PlayerFilter::You,
                TagKey::from("searched"),
            )
            .in_zone(Zone::Library)
            .as_search();
            let _ = search_effect
                .execute(&mut game, &mut ctx)
                .expect("fetch search should pause for Panglacial prompt");
        }

        assert_eq!(dm.boolean_players, vec![bob]);
        assert!(
            dm.object_players.is_empty(),
            "search card choice should not be requested until Bob answers the Panglacial prompt"
        );
        assert!(
            game.stack.iter().all(|entry| {
                game.object(entry.object_id)
                    .is_none_or(|object| object.name != "Panglacial Wurm")
            }),
            "Panglacial should not be cast while the prompt is pending"
        );
    }

    #[test]
    fn opposition_agent_fetchland_can_cast_panglacial_using_searching_players_mana_abilities() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.create_object_from_definition(&opposition_agent_definition(), bob, Zone::Battlefield);
        game.create_object_from_card(
            &library_plains_island_card("Hallowed Fountain"),
            alice,
            Zone::Library,
        );
        game.create_object_from_definition(&panglacial_wurm_definition(), alice, Zone::Library);
        for _ in 0..7 {
            game.create_object_from_definition(&basic_forest(), alice, Zone::Battlefield);
        }

        let mut dm = SearchCastManaAbilityDecisionMaker::new(bob, "Hallowed Fountain");
        {
            let mut ctx = ExecutionContext::new(ObjectId::from_raw(99_998), alice, &mut dm);
            let search_effect = crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default().in_zone(Zone::Library),
                ChoiceCount::exactly(1),
                PlayerFilter::You,
                TagKey::from("searched"),
            )
            .in_zone(Zone::Library)
            .as_search();
            let outcome = search_effect
                .execute(&mut game, &mut ctx)
                .expect("fetch search should resolve");
            assert_eq!(outcome.chosen_objects().unwrap_or_default().len(), 1);
        }

        assert_eq!(dm.boolean_players, vec![bob]);
        assert_eq!(dm.object_players, vec![bob]);
        assert_eq!(
            dm.mana_payment_players.len(),
            1,
            "Bob should confirm the authoritative payment plan using Alice's mana abilities"
        );
        assert!(dm.mana_payment_players.iter().all(|player| *player == bob));

        let wurm_stack = game
            .stack
            .iter()
            .find(|entry| {
                game.object(entry.object_id)
                    .is_some_and(|object| object.name == "Panglacial Wurm")
            })
            .expect("Panglacial Wurm should be cast from Alice's library");
        assert_eq!(wurm_stack.controller, alice);

        let tapped_forests = game
            .battlefield
            .iter()
            .filter(|id| {
                game.object(**id)
                    .is_some_and(|object| object.name == "Forest" && game.is_tapped(**id))
            })
            .count();
        assert_eq!(tapped_forests, 7);
    }

    #[test]
    fn opposition_agent_search_cast_mana_color_choices_use_controlling_player() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.create_object_from_definition(&opposition_agent_definition(), bob, Zone::Battlefield);
        game.create_object_from_card(
            &library_plains_island_card("Hallowed Fountain"),
            alice,
            Zone::Library,
        );
        game.create_object_from_definition(&panglacial_wurm_definition(), alice, Zone::Library);
        for _ in 0..5 {
            game.create_object_from_definition(
                &colorless_mana_land_definition(),
                alice,
                Zone::Battlefield,
            );
        }
        for _ in 0..2 {
            game.create_object_from_definition(
                &green_white_choice_land_definition(),
                alice,
                Zone::Battlefield,
            );
        }

        let mut dm = SearchCastColorChoiceDecisionMaker::new(bob, "Hallowed Fountain");
        {
            let mut ctx = ExecutionContext::new(ObjectId::from_raw(99_995), alice, &mut dm);
            let search_effect = crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default().in_zone(Zone::Library),
                ChoiceCount::exactly(1),
                PlayerFilter::You,
                TagKey::from("searched"),
            )
            .in_zone(Zone::Library)
            .as_search();
            let outcome = search_effect
                .execute(&mut game, &mut ctx)
                .expect("fetch search should resolve");
            assert_eq!(outcome.chosen_objects().unwrap_or_default().len(), 1);
        }

        assert_eq!(dm.boolean_players, vec![bob]);
        assert_eq!(dm.object_players, vec![bob]);
        assert_eq!(dm.mana_payment_players.len(), 1);
        assert_eq!(dm.mana_payment_players, vec![bob]);
        // The selected payment already fixes these outputs. Its confirmation
        // belongs to Bob; the same colors must not be asked for a second time.
        assert!(dm.color_players.is_empty(), "selected plans: {:?}; repeated color players: {:?}",
            dm.selected_mana_plans, dm.color_players);
        assert_eq!(game.controlling_player_for(alice), alice,
            "search control must end after the search");

        let wurm_stack = game
            .stack
            .iter()
            .find(|entry| {
                game.object(entry.object_id)
                    .is_some_and(|object| object.name == "Panglacial Wurm")
            })
            .expect("Panglacial Wurm should be cast using Bob's green choices");
        assert_eq!(wurm_stack.controller, alice);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0,
            "the complete selected seven-mana payment must be consumed");
    }

    #[test]
    fn manual_mana_color_choices_use_scoped_controller_and_commit_actual_colors() {
        for controlled in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let lands = (0..2).map(|_| game.create_object_from_definition(
                &green_white_choice_land_definition(), alice, Zone::Battlefield))
                .collect::<Vec<_>>();
            let control = controlled.then(|| game.add_scoped_player_control(bob, alice, None));
            let mut dm = SearchCastColorChoiceDecisionMaker::new(bob, "unused");
            for land in lands {
                crate::special_actions::perform_activate_mana_ability(
                    &mut game, alice, land, 0, &mut dm).unwrap();
            }
            let chooser = if controlled { bob } else { alice };
            assert_eq!(dm.color_players, vec![chooser, chooser]);
            assert!(dm.mana_payment_players.is_empty());
            let pool = &game.player(alice).unwrap().mana_pool;
            assert_eq!(pool.total(), 2);
            assert_eq!(pool.green, if controlled { 2 } else { 0 });
            assert_eq!(pool.white, if controlled { 0 } else { 2 });
            if let Some(control) = control { game.remove_scoped_player_control(control); }
            assert_eq!(game.controlling_player_for(alice), alice);
        }
    }

    #[test]
    fn opposition_agent_fetchland_panglacial_respects_cant_cast_restrictions() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.create_object_from_definition(&opposition_agent_definition(), bob, Zone::Battlefield);
        game.create_object_from_card(
            &library_plains_island_card("Hallowed Fountain"),
            alice,
            Zone::Library,
        );
        game.create_object_from_definition(&panglacial_wurm_definition(), alice, Zone::Library);
        {
            let alice_player = game.player_mut(alice).expect("alice exists");
            alice_player.mana_pool.add(ManaSymbol::Colorless, 5);
            alice_player.mana_pool.add(ManaSymbol::Green, 2);
        }
        game.effect_store
            .cant_effects
            .add_cant_cast_filter(alice, ObjectFilter::default().with_type(CardType::Creature));

        let mut dm = FinalPartingDecisionMaker::new(bob, &["Hallowed Fountain"]);
        {
            let mut ctx = ExecutionContext::new(ObjectId::from_raw(99_997), alice, &mut dm);
            let search_effect = crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default().in_zone(Zone::Library),
                ChoiceCount::exactly(1),
                PlayerFilter::You,
                TagKey::from("searched"),
            )
            .in_zone(Zone::Library)
            .as_search();
            let outcome = search_effect
                .execute(&mut game, &mut ctx)
                .expect("fetch search should resolve");
            assert_eq!(outcome.chosen_objects().unwrap_or_default().len(), 1);
        }

        assert!(
            dm.boolean_players.is_empty(),
            "Bob should not be prompted to cast Panglacial when Alice can't cast creature spells"
        );
        assert_eq!(dm.object_players, vec![bob]);
        assert!(
            game.stack.iter().all(|entry| {
                game.object(entry.object_id)
                    .is_none_or(|object| object.name != "Panglacial Wurm")
            }),
            "Panglacial should not be cast through a cant-cast restriction"
        );
    }

    #[test]
    fn opposition_agent_fetchland_can_cast_panglacial_with_library_free_cast_grant() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.create_object_from_definition(&opposition_agent_definition(), bob, Zone::Battlefield);
        game.create_object_from_card(
            &library_plains_island_card("Hallowed Fountain"),
            alice,
            Zone::Library,
        );
        game.create_object_from_definition(&panglacial_wurm_definition(), alice, Zone::Library);
        let free_cast_source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Library Omniscience")
                .card_types(vec![CardType::Enchantment])
                .build(),
            alice,
            Zone::Battlefield,
        );
        game.effect_store
            .grant_registry
            .grant_alternative_cast_to_filter(
                ObjectFilter::default().with_type(CardType::Creature),
                Zone::Library,
                alice,
                AlternativeCastingMethod::alternative_cost(
                    "without paying its mana cost",
                    None,
                    Vec::new(),
                ),
                GrantSource::Effect {
                    source_id: free_cast_source,
                    expires_end_of_turn: u32::MAX,
                },
            );

        let mut dm = SearchCastManaAbilityDecisionMaker::new(bob, "Hallowed Fountain");
        {
            let mut ctx = ExecutionContext::new(ObjectId::from_raw(99_996), alice, &mut dm);
            let search_effect = crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default().in_zone(Zone::Library),
                ChoiceCount::exactly(1),
                PlayerFilter::You,
                TagKey::from("searched"),
            )
            .in_zone(Zone::Library)
            .as_search();
            let outcome = search_effect
                .execute(&mut game, &mut ctx)
                .expect("fetch search should resolve");
            assert_eq!(outcome.chosen_objects().unwrap_or_default().len(), 1);
        }

        assert_eq!(dm.boolean_players, vec![bob]);
        assert_eq!(dm.object_players, vec![bob]);
        assert!(
            dm.mana_payment_players.is_empty(),
            "the library free-cast grant should avoid Panglacial's printed mana cost"
        );

        let wurm_stack = game
            .stack
            .iter()
            .find(|entry| {
                game.object(entry.object_id)
                    .is_some_and(|object| object.name == "Panglacial Wurm")
            })
            .expect("Panglacial Wurm should be cast for free from Alice's library");
        assert_eq!(wurm_stack.controller, alice);
        assert!(matches!(
            wurm_stack.casting_method,
            CastingMethod::PlayFrom {
                source,
                zone: Zone::Library,
                use_alternative: Some(0),
            } if source == free_cast_source
        ));
    }
}

#[cfg(test)]
mod replacement_invalid_found_card_answer_contract_tests {
    use super::*;
    use crate::decision::DecisionMaker;
    // Keep the identical fixture valid against the legacy vector API and
    // the new fallible receipt. A vector cannot report an invalid-choice error.
    trait InvalidChoiceReceipt { fn invalid_choice_error(&self) -> bool; }
    impl<T> InvalidChoiceReceipt for Result<T, ExecutionError> {
        fn invalid_choice_error(&self) -> bool { matches!(self, Err(ExecutionError::InternalError(_))) }
    }
    impl InvalidChoiceReceipt for Vec<ObjectId> {
        fn invalid_choice_error(&self) -> bool { false }
    }
    struct Answer(Vec<usize>);
    impl DecisionMaker for Answer {
        fn decide_options(&mut self, _: &GameState, _: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> { self.0.clone() }
    }
    fn invalid(indices: Vec<usize>) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20); let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        for name in ["First found-card replacer", "Second found-card replacer"] {
            let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
                .card_types(vec![crate::types::CardType::Creature]).with_ability(crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::opponent_search_exile_found_cards())).build();
            game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        }
        let card = game.create_object_from_card(&crate::card::CardBuilder::new(crate::ids::CardId::new(), "Found library card").card_types(vec![crate::types::CardType::Instant]).build(), alice, Zone::Library);
        game.take_pending_trigger_events(); let ids = game.next_object_id_counter(); let mut dm = Answer(indices);
        let mut ctx = ExecutionContext::new(ObjectId::from_raw(0), alice, &mut dm);
        let result = exile_found_cards_for_opposition_agent(&mut game, &mut ctx, &[card], alice);
        assert!(result.invalid_choice_error(), "found-card replacements must reject malformed required choices before movement or grants");
        assert_eq!(game.object(card).unwrap().zone, Zone::Library); assert_eq!(game.next_object_id_counter(), ids); assert!(game.exile.is_empty()); assert!(game.take_pending_trigger_events().is_empty());
    }
    #[test] fn found_card_empty_answer_is_invalid() { invalid(vec![]); }
    #[test] fn found_card_out_of_range_answer_is_invalid() { invalid(vec![usize::MAX]); }
    #[test] fn found_card_multiple_answers_are_invalid() { invalid(vec![0, 1]); }
}

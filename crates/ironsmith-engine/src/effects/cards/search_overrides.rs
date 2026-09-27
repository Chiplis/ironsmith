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
use crate::events::spells::SpellCastEvent;
use crate::game_state::{ActiveManaSpendPermission, GameState, ManaSpendPermissionSource};
use crate::grant::Grantable;
use crate::grant_registry::GrantSource;
use crate::ids::{ObjectId, PlayerId};
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbilityId;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

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

pub(crate) fn move_found_card_for_opposition_agent(
    game: &mut GameState,
    card_id: ObjectId,
    search: OppositionAgentSearch,
) -> Option<ObjectId> {
    let permission = OppositionAgentFoundCardPermission {
        controller: search.controller,
        source: search.source,
    };
    let new_id = game.move_object_by_effect(card_id, Zone::Exile)?;
    game.add_exiled_with_source_link(search.source, new_id);
    grant_opposition_agent_play_permission(game, new_id, permission);
    Some(new_id)
}

pub(crate) fn exile_found_cards_for_opposition_agent(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    cards: &[ObjectId],
    searching_player: PlayerId,
) -> Vec<ObjectId> {
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
                return Vec::new();
            }
            index
        } else {
            0
        };
        if let Some(replacement) = replacements.get(index).or_else(|| replacements.first()) {
            selected.push((card_id, *replacement));
        }
    }
    selected
        .into_iter()
        .filter_map(|(card_id, replacement)| {
            move_found_card_for_opposition_agent(game, card_id, replacement)
        })
        .collect()
}

pub(crate) fn offer_library_search_casts(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    library_owner: PlayerId,
) -> Result<(), ExecutionError> {
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
                return Ok(());
            }
            if !accept {
                continue;
            }

            cast_from_library_while_searching(
                game,
                ctx,
                card_id,
                library_owner,
                option.casting_method,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
            break;
        }
    }

    Ok(())
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
        crate::static_abilities::this_spell_cost_condition_is_active_for_cast(
            game,
            spell.id,
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
) -> Result<(), ExecutionError> {
    {
        let Some(object) = game.object(card_id) else {
            return Ok(());
        };
        if !is_library_search_cast_candidate(game, card_id, object) {
            return Ok(());
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
            return Ok(());
        }
    }

    let result = crate::game_loop::cast_spell_from_resolving_effect(
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
    let Some(new_id) = result else {
        return Ok(());
    };

    let event = if let Some(object) = game.object(new_id) {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(object, game);
        SpellCastEvent::new_with_snapshot(new_id, caster, Zone::Library, snapshot)
    } else {
        SpellCastEvent::new(new_id, caster, Zone::Library)
    };
    game.queue_trigger_event(
        ctx.provenance,
        TriggerEvent::new_with_provenance(event, ctx.provenance),
    );

    Ok(())
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
        assert_eq!(
            dm.color_players,
            vec![bob, bob],
            "Bob should choose the colors produced by Alice's choice lands while controlling her"
        );

        let wurm_stack = game
            .stack
            .iter()
            .find(|entry| {
                game.object(entry.object_id)
                    .is_some_and(|object| object.name == "Panglacial Wurm")
            })
            .expect("Panglacial Wurm should be cast using Bob's green choices");
        assert_eq!(wurm_stack.controller, alice);
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

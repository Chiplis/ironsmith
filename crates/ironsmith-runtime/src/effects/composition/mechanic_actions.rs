//! Explicit mechanic effects used by parser/rendering for supported wording.
//!
//! These mechanics are represented as first-class effects so parser output does
//! not depend on raw oracle text passthrough for rendering.

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{ChoiceCount, Effect, EffectOutcome, ExecutionFact, OutcomeValue, Until};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{normalize_object_selection, resolve_value};
use crate::effects::player::CastTaggedEffect;
use crate::effects::zones::apply_zone_change;
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::Event;
use crate::events::permanents::SacrificeEvent;
use crate::events::processing::{
    EventOutcome, TraitEventResult, process_trait_event_with_dm_and_applied_effects,
};
use crate::events::zones::ZoneChangeEvent;
use crate::events::{CardRevealedEvent, KeywordActionEvent, KeywordActionKind};
use crate::filter::PlayerFilter;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::object::{CounterType, ObjectKind};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;
use std::collections::HashMap;
pub type AmplifyEffect = ironsmith_core::AmplifyEffect;
pub use ironsmith_core::{BolsterEffect, CipherEffect, DevourEffect};

#[derive(Debug, Clone, PartialEq)]
pub struct BackupEffect {
    pub amount: u32,
    pub granted_abilities: Vec<crate::ability::Ability>,
}

impl BackupEffect {
    pub fn new(amount: u32, granted_abilities: Vec<crate::ability::Ability>) -> Self {
        Self {
            amount,
            granted_abilities,
        }
    }
}

impl EffectExecutor for BackupEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target = crate::effects::helpers::resolve_single_object_from_spec(
            game,
            &ChooseSpec::target_creature(),
            ctx,
        )?;

        let mut outcomes = vec![
            crate::effects::PutCountersEffect::new(
                CounterType::PlusOnePlusOne,
                self.amount,
                ChooseSpec::SpecificObject(target),
            )
            .execute(game, ctx)?,
        ];

        if target != ctx.source {
            for ability in &self.granted_abilities {
                outcomes.push(
                    crate::effects::ApplyContinuousEffect::new(
                        crate::continuous::EffectTarget::Specific(target),
                        crate::continuous::Modification::AddAbilityGeneric(ability.clone()),
                        Until::EndOfTurn,
                    )
                    .execute(game, ctx)?,
                );
            }
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        static TARGET: std::sync::OnceLock<ChooseSpec> = std::sync::OnceLock::new();
        Some(TARGET.get_or_init(ChooseSpec::target_creature))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExploreEffect {
    pub target: ChooseSpec,
}

impl ExploreEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone)]
struct ExploreInstruction {
    object_id: ObjectId,
    controller: PlayerId,
    snapshot: Option<ObjectSnapshot>,
}

fn explore_snapshot_for_object(
    game: &GameState,
    ctx: &ExecutionContext,
    object_id: ObjectId,
) -> Option<ObjectSnapshot> {
    if let Some(object) = game.object(object_id) {
        return Some(ObjectSnapshot::from_object(object, game));
    }
    if let Some(snapshot) = ctx.target_snapshots.get(&object_id) {
        return Some(snapshot.clone());
    }
    if let Some(snapshot) = ctx.source_snapshot.as_ref()
        && snapshot.object_id == object_id
    {
        return Some(snapshot.clone());
    }
    ctx.tagged_objects
        .values()
        .flat_map(|snapshots| snapshots.iter())
        .find(|snapshot| snapshot.object_id == object_id)
        .cloned()
}

fn players_in_apnap_order(game: &GameState) -> Vec<PlayerId> {
    if game.turn_store.turn_order.is_empty() {
        return game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .collect();
    }

    let start = game
        .turn_store
        .turn_order
        .iter()
        .position(|&player_id| player_id == game.turn.active_player)
        .unwrap_or(0);

    (0..game.turn_store.turn_order.len())
        .filter_map(|offset| {
            let player_id =
                game.turn_store.turn_order[(start + offset) % game.turn_store.turn_order.len()];
            game.player(player_id)
                .filter(|player| player.is_in_game())
                .map(|_| player_id)
        })
        .collect()
}

fn execute_keyword_action_replacement_effects(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: Vec<Effect>,
    effect_id: crate::replacement::ReplacementEffectId,
    action_snapshot: Option<ObjectSnapshot>,
) -> Result<EffectOutcome, ExecutionError> {
    let replacement_effect = game
        .effect_store
        .replacement_effects
        .get_effect(effect_id)
        .cloned();
    let (replacement_source, replacement_controller) = replacement_effect
        .as_ref()
        .map(|effect| (effect.source, effect.controller))
        .unwrap_or((ctx.source, ctx.controller));
    let replacement_key = replacement_effect
        .as_ref()
        .map(|effect| effect.application_key());

    let original_source = ctx.source;
    let original_controller = ctx.controller;
    let original_cause = ctx.cause.clone();
    let original_it = ctx.clear_object_tag("__it__");
    let original_plain_it = ctx.clear_object_tag("it");
    let was_suppressed = !ctx
        .replacement
        .suppressed_replacement_effects
        .insert(effect_id);
    let key_was_suppressed = if let Some(key) = replacement_key.as_ref() {
        !ctx.replacement
            .suppressed_replacement_effect_keys
            .insert(key.clone())
    } else {
        true
    };

    ctx.source = replacement_source;
    ctx.controller = replacement_controller;
    ctx.cause =
        crate::events::cause::EventCause::from_effect(replacement_source, replacement_controller);
    if let Some(snapshot) = action_snapshot {
        ctx.set_tagged_objects("__it__", vec![snapshot.clone()]);
        ctx.set_tagged_objects("it", vec![snapshot]);
    }

    let execution_result = (|| -> Result<EffectOutcome, ExecutionError> {
        let mut outcomes = Vec::new();
        for effect in effects {
            outcomes.push(crate::effects::execute_effect(game, &effect, ctx)?);
        }
        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    })();

    ctx.source = original_source;
    ctx.controller = original_controller;
    ctx.cause = original_cause;
    if !was_suppressed {
        ctx.replacement
            .suppressed_replacement_effects
            .remove(&effect_id);
    }
    if !key_was_suppressed && let Some(key) = replacement_key {
        ctx.replacement
            .suppressed_replacement_effect_keys
            .remove(&key);
    }
    match original_it {
        Some(snapshots) => ctx.set_tagged_objects("__it__", snapshots),
        None => {
            ctx.clear_object_tag("__it__");
        }
    }
    match original_plain_it {
        Some(snapshots) => ctx.set_tagged_objects("it", snapshots),
        None => {
            ctx.clear_object_tag("it");
        }
    }

    execution_result
}

impl EffectExecutor for ExploreEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_ids = if let ChooseSpec::Tagged(tag) = self.target.base() {
            ctx.get_tagged_all(tag)
                .map(|snapshots| {
                    snapshots
                        .iter()
                        .map(|snapshot| snapshot.object_id)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            match crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target) {
                Ok(ids) => ids,
                Err(ExecutionError::InvalidTarget) if self.target.is_target() => {
                    return Ok(EffectOutcome::target_invalid());
                }
                Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::count(0)),
                Err(err) => return Err(err),
            }
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if target_ids.is_empty() {
            return Ok(if self.target.is_target() {
                EffectOutcome::target_invalid()
            } else {
                EffectOutcome::count(0)
            });
        }

        let mut remaining = target_ids
            .into_iter()
            .filter_map(|object_id| {
                let snapshot = explore_snapshot_for_object(game, ctx, object_id);
                let controller = game
                    .object(object_id)
                    .map(|object| game.controller_of(object))
                    .or_else(|| snapshot.as_ref().map(|snap| snap.controller))?;
                Some(ExploreInstruction {
                    object_id,
                    controller,
                    snapshot,
                })
            })
            .collect::<Vec<_>>();
        if remaining.is_empty() {
            return Ok(if self.target.is_target() {
                EffectOutcome::target_invalid()
            } else {
                EffectOutcome::count(0)
            });
        }

        let mut events = Vec::new();
        let mut explored_objects = Vec::new();
        let player_order = players_in_apnap_order(game);

        for player in player_order {
            while let Some(candidate_indices) = (!remaining.is_empty()).then(|| {
                remaining
                    .iter()
                    .enumerate()
                    .filter_map(|(index, instruction)| {
                        (instruction.controller == player).then_some(index)
                    })
                    .collect::<Vec<_>>()
            }) {
                if candidate_indices.is_empty() {
                    break;
                }

                let chosen_index = if candidate_indices.len() == 1 {
                    candidate_indices[0]
                } else {
                    let choices = candidate_indices
                        .iter()
                        .map(|&index| remaining[index].object_id)
                        .collect::<Vec<_>>();
                    let spec = ChooseObjectsSpec::new(
                        ctx.source,
                        "Choose a permanent to explore next",
                        choices.clone(),
                        1,
                        Some(1),
                    );
                    let selection: Vec<ObjectId> =
                        make_decision(game, ctx.decision_maker, player, Some(ctx.source), spec);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(
                            EffectOutcome::with_objects(explored_objects).with_events(events)
                        );
                    }
                    let normalized = normalize_object_selection(selection, &choices, 1);
                    let chosen_object = normalized.first().copied().unwrap_or(choices[0]);
                    candidate_indices
                        .into_iter()
                        .find(|index| remaining[*index].object_id == chosen_object)
                        .unwrap_or(0)
                };

                let instruction = remaining.remove(chosen_index);
                let controller = instruction.controller;
                let pre_snapshot = instruction.snapshot.clone();

                let would_event = Event::new_with_provenance(
                    KeywordActionEvent::new(
                        KeywordActionKind::Explore,
                        controller,
                        instruction.object_id,
                        1,
                    )
                    .with_snapshot(pre_snapshot.clone()),
                    ctx.provenance,
                );
                let applied_effects = ctx.replacement.suppressed_replacement_effects.clone();
                let applied_effect_keys =
                    ctx.replacement.suppressed_replacement_effect_keys.clone();
                if applied_effects.is_empty() && applied_effect_keys.is_empty() {
                    game.update_replacement_effects();
                }
                match process_trait_event_with_dm_and_applied_effects(
                    game,
                    would_event,
                    ctx.decision_maker,
                    &applied_effects,
                    &applied_effect_keys,
                ) {
                    TraitEventResult::Replaced {
                        effects, effect_id, ..
                    } => {
                        let replacement_outcome = execute_keyword_action_replacement_effects(
                            game,
                            ctx,
                            effects,
                            effect_id,
                            pre_snapshot.clone(),
                        )?;
                        for event in &replacement_outcome.events {
                            if let Some(keyword) =
                                event.inner().as_any().downcast_ref::<KeywordActionEvent>()
                                && keyword.action == KeywordActionKind::Explore
                            {
                                explored_objects.push(keyword.source);
                            }
                        }
                        events.extend(replacement_outcome.events);
                        continue;
                    }
                    TraitEventResult::Prevented => continue,
                    TraitEventResult::NeedsChoice { .. }
                    | TraitEventResult::NeedsInteraction { .. } => {
                        return Ok(
                            EffectOutcome::with_objects(explored_objects).with_events(events)
                        );
                    }
                    TraitEventResult::Proceed(_) | TraitEventResult::Modified(_) => {}
                }

                let revealed_card_id = game
                    .player(controller)
                    .and_then(|entry| entry.library.last().copied());
                let revealed_snapshot = revealed_card_id.and_then(|card_id| {
                    game.object(card_id)
                        .map(|object| ObjectSnapshot::from_object(object, game))
                });
                if let Some(card_id) = revealed_card_id {
                    for viewer_idx in 0..game.players.len() {
                        let viewer = PlayerId::from_index(viewer_idx as u8);
                        let view_ctx = crate::decisions::context::ViewCardsContext::new(
                            viewer,
                            controller,
                            Some(ctx.source),
                            Zone::Library,
                            "Reveal the top card of a library",
                        )
                        .with_public(true);
                        ctx.decision_maker
                            .view_cards(game, viewer, &[card_id], &view_ctx);
                    }
                    events.push(TriggerEvent::new_with_provenance(
                        CardRevealedEvent::new(
                            controller,
                            card_id,
                            Zone::Library,
                            Some(ctx.source),
                            revealed_snapshot.clone(),
                        ),
                        ctx.provenance,
                    ));
                }

                let revealed_is_land = revealed_card_id
                    .and_then(|card_id| game.object(card_id))
                    .is_some_and(|object| object.has_card_type(crate::types::CardType::Land));

                if let Some(card_id) = revealed_card_id {
                    if revealed_is_land {
                        let _ = apply_zone_change(
                            game,
                            card_id,
                            Zone::Library,
                            Zone::Hand,
                            ctx.cause.clone(),
                            &mut *ctx.decision_maker,
                        );
                    } else {
                        if game.object(instruction.object_id).is_some() {
                            if let Some(event) = game.add_counters_with_source(
                                instruction.object_id,
                                CounterType::PlusOnePlusOne,
                                1,
                                Some(ctx.source),
                                Some(ctx.controller),
                            ) {
                                events.push(event);
                            }
                        }

                        let choice_ctx = crate::decisions::context::BooleanContext::new(
                            controller,
                            Some(ctx.source),
                            "Put the explored card into your graveyard?".to_string(),
                        );
                        let put_into_graveyard =
                            ctx.decision_maker.decide_boolean(game, &choice_ctx);
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(EffectOutcome::count(0));
                        }
                        if put_into_graveyard {
                            let _ = apply_zone_change(
                                game,
                                card_id,
                                Zone::Library,
                                Zone::Graveyard,
                                ctx.cause.clone(),
                                &mut *ctx.decision_maker,
                            );
                        }
                    }
                } else if game.object(instruction.object_id).is_some() {
                    if let Some(event) = game.add_counters_with_source(
                        instruction.object_id,
                        CounterType::PlusOnePlusOne,
                        1,
                        Some(ctx.source),
                        Some(ctx.controller),
                    ) {
                        events.push(event);
                    }
                }

                let action_snapshot = game
                    .object(instruction.object_id)
                    .map(|object| ObjectSnapshot::from_object(object, game))
                    .or(pre_snapshot);
                let object_tags = revealed_snapshot
                    .clone()
                    .map(|snapshot| {
                        HashMap::from([(
                            TagKey::from(crate::effects::PUBLIC_REVEALED_TAG),
                            vec![snapshot],
                        )])
                    })
                    .unwrap_or_default();
                events.push(TriggerEvent::new_with_provenance(
                    KeywordActionEvent::new(
                        KeywordActionKind::Explore,
                        controller,
                        instruction.object_id,
                        1,
                    )
                    .with_snapshot(action_snapshot)
                    .with_object_tags(object_tags),
                    ctx.provenance,
                ));
                explored_objects.push(instruction.object_id);
            }
        }

        Ok(EffectOutcome::with_objects(explored_objects).with_events(events))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent to explore"
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenAttractionEffect;

impl OpenAttractionEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for OpenAttractionEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(source) = game.object(ctx.source) else {
            return Ok(EffectOutcome::resolved());
        };
        let controller = game.controller_of(source);
        Ok(
            EffectOutcome::resolved().with_event(TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::OpenAttraction,
                    controller,
                    ctx.source,
                    1,
                ),
                ctx.provenance,
            )),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestDreadEffect;

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestTopCardOfLibraryEffect {
    pub player: PlayerFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestCardFromHandEffect;

impl ManifestTopCardOfLibraryEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl ManifestDreadEffect {
    pub fn new() -> Self {
        Self
    }
}

impl ManifestCardFromHandEffect {
    pub fn new() -> Self {
        Self
    }
}

fn manifest_card(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    card_id: ObjectId,
    controller: crate::ids::PlayerId,
) -> Result<EffectOutcome, ExecutionError> {
    if game.object(card_id).is_none() {
        return Ok(EffectOutcome::count(0));
    }

    if let Some(card) = game.object_mut(card_id) {
        card.apply_face_down_cast_overlay();
    }

    let outcome = match move_to_battlefield_with_options(
        game,
        ctx,
        card_id,
        BattlefieldEntryOptions::specific(controller, false),
    ) {
        BattlefieldEntryOutcome::Moved(new_id) => {
            game.set_manifested(new_id);
            EffectOutcome::with_objects(vec![new_id]).with_event(TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(KeywordActionKind::Manifest, controller, ctx.source, 1),
                ctx.provenance,
            ))
        }
        BattlefieldEntryOutcome::Prevented => {
            if let Some(card) = game.object_mut(card_id) {
                card.end_face_down_cast_overlay();
            }
            EffectOutcome::count(0)
        }
    };

    Ok(outcome)
}

impl EffectExecutor for ManifestDreadEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::resolved())
    }
}

impl EffectExecutor for ManifestTopCardOfLibraryEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let library_owner =
            crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
        let Some(&card_id) = game
            .player(library_owner)
            .and_then(|player| player.library.last())
        else {
            return Ok(EffectOutcome::count(0));
        };

        manifest_card(game, ctx, card_id, ctx.controller)
    }
}

impl EffectExecutor for ManifestCardFromHandEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let hand = game
            .player(ctx.controller)
            .map(|player| player.hand.clone())
            .unwrap_or_default();
        if hand.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let chosen = make_decision(
            game,
            ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            ChooseObjectsSpec::new(
                ctx.source,
                "Choose a card from your hand to manifest",
                hand,
                1,
                Some(1),
            )
            .require_explicit_choice()
            .with_hidden_card_visibility(
                crate::decisions::context::DecisionHiddenCardVisibility::PrivateToDecisionPlayer,
            ),
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(card_id) = chosen.into_iter().find(|id| {
            game.object(*id)
                .is_some_and(|object| object.zone == Zone::Hand && object.owner == ctx.controller)
        }) else {
            return Ok(EffectOutcome::count(0));
        };

        manifest_card(game, ctx, card_id, ctx.controller)
    }
}

pub type PopulateEffect = ironsmith_core::PopulateEffect;

impl EffectExecutor for PopulateEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        if count == 0 {
            return Ok(EffectOutcome::resolved());
        }

        let mut created_ids = Vec::new();
        let mut events = Vec::new();

        for _ in 0..count {
            let candidates = game
                .battlefield
                .iter()
                .copied()
                .filter(|&id| {
                    game.object(id).is_some_and(|obj| {
                        game.controller_of(obj) == ctx.controller
                            && obj.kind == ObjectKind::Token
                            && game.object_has_card_type(id, crate::types::CardType::Creature)
                    })
                })
                .collect::<Vec<_>>();

            if candidates.is_empty() {
                events.push(TriggerEvent::new_with_provenance(
                    KeywordActionEvent::new(
                        KeywordActionKind::Populate,
                        ctx.controller,
                        ctx.source,
                        1,
                    ),
                    ctx.provenance,
                ));
                continue;
            }

            let chosen = if candidates.len() == 1 {
                candidates[0]
            } else {
                let spec = ChooseObjectsSpec::new(
                    ctx.source,
                    "Choose a creature token you control to populate",
                    candidates.clone(),
                    1,
                    Some(1),
                );
                let selection: Vec<ObjectId> = make_decision(
                    game,
                    ctx.decision_maker,
                    ctx.controller,
                    Some(ctx.source),
                    spec,
                );
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::with_objects(created_ids).with_events(events));
                }
                let normalized = normalize_object_selection(selection, &candidates, 1);
                normalized.first().copied().unwrap_or(candidates[0])
            };

            let outcome =
                crate::effects::CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(chosen))
                    .enters_tapped(self.enters_tapped)
                    .attacking(self.enters_attacking)
                    .haste(self.has_haste)
                    .sacrifice_at_next_end_step(self.sacrifice_at_next_end_step)
                    .exile_at_next_end_step(self.exile_at_next_end_step)
                    .exile_at_eoc(self.exile_at_end_of_combat)
                    .execute(game, ctx)?;
            if let OutcomeValue::Objects(ids) = outcome.value {
                created_ids.extend(ids);
            }
            events.extend(outcome.events);
            events.push(TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(KeywordActionKind::Populate, ctx.controller, ctx.source, 1),
                ctx.provenance,
            ));
        }

        Ok(EffectOutcome::with_objects(created_ids).with_events(events))
    }
}

impl EffectExecutor for BolsterEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut candidates = game
            .battlefield
            .iter()
            .copied()
            .filter(|&id| {
                game.object(id).is_some_and(|obj| {
                    game.controller_of(obj) == ctx.controller
                        && game.object_has_card_type(id, crate::types::CardType::Creature)
                })
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let least_toughness = candidates
            .iter()
            .filter_map(|&id| {
                game.calculated_toughness(id)
                    .or_else(|| game.object(id).and_then(|obj| obj.toughness()))
            })
            .min()
            .unwrap_or(0);
        candidates.retain(|&id| {
            game.calculated_toughness(id)
                .or_else(|| game.object(id).and_then(|obj| obj.toughness()))
                == Some(least_toughness)
        });
        if candidates.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let chosen = if candidates.len() == 1 {
            candidates[0]
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose a creature with the least toughness you control for bolster",
                candidates.clone(),
                1,
                Some(1),
            );
            let selection: Vec<ObjectId> = make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let normalized = normalize_object_selection(selection, &candidates, 1);
            normalized.first().copied().unwrap_or(candidates[0])
        };

        let outcome = crate::effects::PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            self.amount,
            ChooseSpec::SpecificObject(chosen),
        )
        .execute(game, ctx)?;

        Ok(outcome.with_event(TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Bolster, ctx.controller, ctx.source, 1),
            ctx.provenance,
        )))
    }
}

impl EffectExecutor for CipherEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(source_obj) = game.object(ctx.source).cloned() else {
            return Ok(EffectOutcome::target_invalid());
        };
        if source_obj.zone != Zone::Stack || source_obj.card.is_none() {
            return Ok(EffectOutcome::resolved());
        }

        let candidates = game
            .battlefield
            .iter()
            .copied()
            .filter(|&id| {
                game.object(id).is_some_and(|obj| {
                    game.controller_of(obj) == ctx.controller
                        && game.object_has_card_type(id, crate::types::CardType::Creature)
                })
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let choice_ctx = crate::decisions::context::BooleanContext::new(
            ctx.controller,
            Some(ctx.source),
            format!(
                "Exile {} encoded on a creature you control?",
                source_obj.name
            ),
        );
        let encode = ctx.decision_maker.decide_boolean(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if !encode {
            return Ok(EffectOutcome::declined());
        }

        let spec = ChooseObjectsSpec::new(
            ctx.source,
            "Choose a creature you control to encode",
            candidates.clone(),
            1,
            Some(1),
        );
        let selection: Vec<ObjectId> = make_decision(
            game,
            ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            spec,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let normalized = normalize_object_selection(selection, &candidates, 1);
        let Some(chosen_creature) = normalized.first().copied() else {
            return Ok(EffectOutcome::declined());
        };

        let exiled_id = match apply_zone_change(
            game,
            ctx.source,
            source_obj.zone,
            Zone::Exile,
            ctx.cause.clone(),
            &mut *ctx.decision_maker,
        ) {
            EventOutcome::Proceed(result) => {
                let Some(new_id) = result.new_object_id else {
                    return Ok(EffectOutcome::resolved());
                };
                if result.final_zone != Zone::Exile {
                    return Ok(EffectOutcome::resolved());
                }
                new_id
            }
            EventOutcome::Prevented => return Ok(EffectOutcome::prevented()),
            EventOutcome::Replaced => return Ok(EffectOutcome::replaced()),
            EventOutcome::NotApplicable => return Ok(EffectOutcome::target_invalid()),
        };

        let Some(exiled_stable_id) = game.object(exiled_id).map(|obj| obj.stable_id) else {
            return Ok(EffectOutcome::target_invalid());
        };

        game.imprint_card(chosen_creature, exiled_id);
        let ability = crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_deals_combat_damage_to_player(
                crate::target::PlayerFilter::Any,
            ),
            vec![crate::effect::Effect::cast_encoded_card_copy(
                exiled_stable_id,
            )],
        );
        if let Some(creature) = game.object_mut(chosen_creature) {
            creature.abilities.push(ability);
        }

        Ok(
            EffectOutcome::with_objects(vec![exiled_id, chosen_creature])
                .with_execution_fact(ExecutionFact::ChosenObjects(vec![chosen_creature]))
                .with_execution_fact(ExecutionFact::AffectedObjects(vec![exiled_id])),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastEncodedCardCopyEffect {
    pub encoded_card: StableId,
}

impl CastEncodedCardCopyEffect {
    pub fn new(encoded_card: StableId) -> Self {
        Self { encoded_card }
    }
}

impl EffectExecutor for CastEncodedCardCopyEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(encoded_id) = game.find_object_by_stable_id(self.encoded_card) else {
            return Ok(EffectOutcome::target_invalid());
        };
        let Some(encoded_obj) = game.object(encoded_id).cloned() else {
            return Ok(EffectOutcome::target_invalid());
        };
        if encoded_obj.zone != Zone::Exile {
            return Ok(EffectOutcome::target_invalid());
        }

        let choice_ctx = crate::decisions::context::BooleanContext::new(
            ctx.controller,
            Some(ctx.source),
            format!(
                "Cast a copy of {} without paying its mana cost?",
                encoded_obj.name
            ),
        );
        let cast_copy = ctx.decision_maker.decide_boolean(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if !cast_copy {
            return Ok(EffectOutcome::declined());
        }

        let snapshot = ObjectSnapshot::from_object(&encoded_obj, game);
        let prior = ctx.clear_object_tag("cipher_encoded");
        ctx.set_tagged_objects("cipher_encoded", vec![snapshot]);
        let result = CastTaggedEffect::new("cipher_encoded", crate::target::PlayerFilter::You)
            .as_copy()
            .without_paying_mana_cost()
            .execute(game, ctx);
        if let Some(previous) = prior {
            ctx.set_tagged_objects("cipher_encoded", previous);
        } else {
            ctx.clear_object_tag("cipher_encoded");
        }
        result
    }
}

impl EffectExecutor for DevourEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if !game
            .object(ctx.source)
            .is_some_and(|obj| obj.zone == Zone::Battlefield)
        {
            return Ok(EffectOutcome::resolved());
        }

        let candidates = game
            .battlefield
            .iter()
            .copied()
            .filter(|&id| id != ctx.source)
            .filter(|&id| {
                game.object(id).is_some_and(|obj| {
                    game.controller_of(obj) == ctx.controller
                        && game.object_has_card_type(id, crate::types::CardType::Creature)
                        && game.can_be_sacrificed(id)
                })
            })
            .collect::<Vec<_>>();

        let chosen = if candidates.is_empty() {
            Vec::new()
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose any number of other creatures you control to sacrifice for devour",
                candidates.clone(),
                0,
                Some(candidates.len()),
            );
            let selection: Vec<ObjectId> = make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            selection
                .into_iter()
                .filter(|id| candidates.contains(id))
                .fold(Vec::new(), |mut chosen, id| {
                    if !chosen.contains(&id) {
                        chosen.push(id);
                    }
                    chosen
                })
        };

        let pending_start = game.effect_store.pending_trigger_events.len();
        let mut sacrificed_count: i32 = 0;
        let mut sacrifice_events = Vec::new();
        let mut graveyard_zone_changes = Vec::new();
        for id in chosen {
            let pre_snapshot = game
                .object(id)
                .map(|obj| ObjectSnapshot::from_object(obj, game));
            let sacrificing_player = pre_snapshot.as_ref().map(|snapshot| snapshot.controller);

            match apply_zone_change(
                game,
                id,
                Zone::Battlefield,
                Zone::Graveyard,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ) {
                EventOutcome::Prevented | EventOutcome::NotApplicable => {}
                EventOutcome::Proceed(result) => {
                    sacrificed_count += 1;
                    if result.final_zone == Zone::Graveyard {
                        if let Some(snapshot) = pre_snapshot.clone() {
                            graveyard_zone_changes.push((
                                id,
                                result.new_object_ids.clone(),
                                snapshot,
                            ));
                        }
                        sacrifice_events.push(TriggerEvent::new_with_provenance(
                            SacrificeEvent::new(id, Some(ctx.source))
                                .with_snapshot(pre_snapshot, sacrificing_player),
                            ctx.provenance,
                        ));
                    }
                }
                EventOutcome::Replaced => {
                    sacrificed_count += 1;
                }
            }
        }

        if graveyard_zone_changes.len() > 1 {
            let event_objects = graveyard_zone_changes
                .iter()
                .map(|(id, _, _)| *id)
                .collect::<Vec<_>>();
            let result_objects = graveyard_zone_changes
                .iter()
                .flat_map(|(_, result_ids, _)| result_ids.iter().copied())
                .collect::<Vec<_>>();
            let snapshots = graveyard_zone_changes
                .iter()
                .map(|(_, _, snapshot)| snapshot.clone())
                .collect::<Vec<_>>();

            let removed =
                game.remove_pending_trigger_events_matching_from(pending_start, |event| {
                    let Some(zone_change) = event.downcast::<ZoneChangeEvent>() else {
                        return false;
                    };
                    zone_change.from == Zone::Battlefield
                        && zone_change.to == Zone::Graveyard
                        && zone_change.objects.len() == 1
                        && event_objects.contains(&zone_change.objects[0])
                });

            if !removed.is_empty() {
                let mut event = ZoneChangeEvent::batch_with_snapshots(
                    event_objects,
                    Zone::Battlefield,
                    Zone::Graveyard,
                    ctx.cause.clone(),
                    snapshots,
                );
                event.result_objects = result_objects;
                game.queue_trigger_event(
                    ctx.provenance,
                    TriggerEvent::new_with_provenance(event, ctx.provenance),
                );
            }
        }

        if sacrificed_count == 0 {
            game.set_devoured_count(ctx.source, 0);
            return Ok(EffectOutcome::count(0).with_events(sacrifice_events));
        }

        game.set_devoured_count(ctx.source, sacrificed_count as u32);
        let mut counters = crate::effects::PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            sacrificed_count.saturating_mul(self.multiplier as i32),
            ChooseSpec::Source,
        )
        .execute(game, ctx)?;
        counters.events.extend(sacrifice_events);
        Ok(counters)
    }
}

impl EffectExecutor for AmplifyEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if !game
            .object(ctx.source)
            .is_some_and(|obj| obj.zone == Zone::Battlefield)
        {
            return Ok(EffectOutcome::resolved());
        }

        let source_creature_types = game
            .calculated_subtypes(ctx.source)
            .into_iter()
            .filter(|subtype| subtype.is_creature_type())
            .collect::<Vec<_>>();
        if source_creature_types.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let candidates = game
            .player(ctx.controller)
            .map(|player| player.hand.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.zone == Zone::Hand
                        && obj.has_card_type(crate::types::CardType::Creature)
                        && source_creature_types
                            .iter()
                            .any(|&subtype| obj.has_subtype(subtype))
                })
            })
            .collect::<Vec<_>>();

        let chosen = if candidates.is_empty() {
            Vec::new()
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose any number of cards from your hand that share a creature type with this creature to reveal for amplify",
                candidates.clone(),
                0,
                Some(candidates.len()),
            );
            let selection: Vec<ObjectId> = make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            selection
                .into_iter()
                .filter(|id| candidates.contains(id))
                .fold(Vec::new(), |mut chosen, id| {
                    if !chosen.contains(&id) {
                        chosen.push(id);
                    }
                    chosen
                })
        };

        if chosen.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        for viewer_idx in 0..game.players.len() {
            let viewer = PlayerId::from_index(viewer_idx as u8);
            let view_ctx = crate::decisions::context::ViewCardsContext::new(
                viewer,
                ctx.controller,
                Some(ctx.source),
                Zone::Hand,
                "Reveal cards from hand for amplify",
            )
            .with_public(true);
            ctx.decision_maker
                .view_cards(game, viewer, &chosen, &view_ctx);
        }

        let revealed_snapshots = chosen
            .iter()
            .filter_map(|&id| {
                game.object(id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
            .collect::<Vec<_>>();
        if !revealed_snapshots.is_empty() {
            let entry = ctx
                .tagged_objects
                .entry(crate::tag::TagKey::from(
                    crate::effects::PUBLIC_REVEALED_TAG,
                ))
                .or_default();
            for snapshot in revealed_snapshots {
                if !entry
                    .iter()
                    .any(|existing| existing.object_id == snapshot.object_id)
                {
                    entry.push(snapshot);
                }
            }
        }

        let reveal_events = chosen
            .iter()
            .filter_map(|&id| {
                let snapshot = game
                    .object(id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))?;
                Some(TriggerEvent::new_with_provenance(
                    CardRevealedEvent::new(
                        ctx.controller,
                        id,
                        Zone::Hand,
                        Some(ctx.source),
                        Some(snapshot),
                    ),
                    ctx.provenance,
                ))
            })
            .collect::<Vec<_>>();

        let mut counters = crate::effects::PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            (chosen.len() as i32).saturating_mul(self.amount as i32),
            ChooseSpec::Source,
        )
        .execute(game, ctx)?;
        counters.events.extend(reveal_events);
        Ok(counters)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SupportEffect {
    pub amount: u32,
    pub target: ChooseSpec,
}

impl SupportEffect {
    pub fn new(amount: u32) -> Self {
        Self {
            amount,
            target: ChooseSpec::target(ChooseSpec::Object(
                crate::target::ObjectFilter::creature().other(),
            )),
        }
    }
}

impl EffectExecutor for SupportEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut outcome = crate::effects::PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            1,
            self.target.clone(),
        )
        .with_target_count(ChoiceCount::up_to(self.amount as usize))
        .execute(game, ctx)?;
        outcome.events.push(TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(
                KeywordActionKind::Support,
                ctx.controller,
                ctx.source,
                self.amount,
            ),
            ctx.provenance,
        ));
        Ok(outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        Some(ChoiceCount::up_to(self.amount as usize))
    }

    fn target_description(&self) -> &'static str {
        "target creature to support"
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptEffect {
    pub amount: u32,
}

impl AdaptEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl EffectExecutor for AdaptEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_id = ctx.source;
        if game.object(source_id).is_none() {
            return Ok(EffectOutcome::target_invalid());
        }
        if game.counter_count(source_id, CounterType::PlusOnePlusOne) > 0 {
            return Ok(EffectOutcome::count(0));
        }

        if let Some(stable_id) = game.object(source_id).map(|o| o.stable_id) {
            game.record_ui_effect_event(
                "level_up",
                Some(ctx.controller),
                None,
                vec![stable_id],
                Some(i64::from(self.amount)),
                Some("adapt".to_string()),
            );
        }

        crate::effects::PutCountersEffect::on_source(CounterType::PlusOnePlusOne, self.amount)
            .execute(game, ctx)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CounterAbilityEffect;

impl CounterAbilityEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for CounterAbilityEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardDefinitionBuilder;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectObjectsContext;
    use crate::effects::ExecutionContext;
    use crate::events::{CardRevealedEvent, EventKind, KeywordActionEvent, KeywordActionKind};
    use crate::ids::{CardId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use crate::static_abilities::StaticAbilityId;
    use crate::types::{CardType, Subtype};
    use std::collections::VecDeque;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(
        game: &mut GameState,
        controller: PlayerId,
        card_id: u32,
        name: &str,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn create_creature_token(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        power: i32,
        toughness: i32,
        subtype: Subtype,
    ) -> ObjectId {
        let token = CardDefinitionBuilder::new(CardId::new(), name)
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![subtype])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        let source = game.new_object_id();
        crate::effects::CreateTokenEffect::one(token)
            .execute(game, &mut ExecutionContext::new_default(source, controller))
            .expect("token creation should succeed")
            .value
            .objects()
            .and_then(|ids| ids.first().copied())
            .expect("token creation should produce one token")
    }

    fn create_library_card(
        game: &mut GameState,
        owner: PlayerId,
        card_id: u32,
        name: &str,
        card_types: Vec<CardType>,
        mana_cost: Option<crate::mana::ManaCost>,
        power: Option<i32>,
        toughness: Option<i32>,
    ) -> ObjectId {
        let mut builder = CardBuilder::new(CardId::from_raw(card_id), name).card_types(card_types);
        if let Some(cost) = mana_cost {
            builder = builder.mana_cost(cost);
        }
        if let (Some(power), Some(toughness)) = (power, toughness) {
            builder = builder.power_toughness(PowerToughness::fixed(power, toughness));
        }
        let card = builder.build();
        game.create_object_from_card(&card, owner, Zone::Library)
    }

    struct SelectIdsDecisionMaker {
        choices: VecDeque<Vec<ObjectId>>,
    }

    impl DecisionMaker for SelectIdsDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.choices
                .pop_front()
                .unwrap_or_default()
                .into_iter()
                .filter(|id| {
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.legal && candidate.id == *id)
                })
                .collect()
        }
    }

    struct PromptingDecisionMaker;

    impl DecisionMaker for PromptingDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            _ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            Vec::new()
        }

        fn awaiting_choice(&self) -> bool {
            true
        }
    }

    #[derive(Default)]
    struct ExploreDecisionMaker {
        object_choices: VecDeque<Vec<ObjectId>>,
        boolean_choices: VecDeque<bool>,
    }

    impl DecisionMaker for ExploreDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.object_choices
                .pop_front()
                .unwrap_or_default()
                .into_iter()
                .filter(|id| {
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.legal && candidate.id == *id)
                })
                .collect()
        }

        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.boolean_choices.pop_front().unwrap_or(true)
        }
    }

    #[test]
    fn explore_puts_revealed_land_into_hand_without_a_counter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let explorer = create_creature(&mut game, alice, 50, "Explorer", 2, 2);
        let land = create_library_card(
            &mut game,
            alice,
            51,
            "Forest",
            vec![CardType::Land],
            None,
            None,
            None,
        );

        let outcome = ExploreEffect::new(ChooseSpec::SpecificObject(explorer))
            .execute(&mut game, &mut ExecutionContext::new_default(source, alice))
            .expect("explore should execute");

        assert_eq!(game.counter_count(explorer, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.player(alice).expect("alice").hand.len(), 1);
        assert_eq!(game.player(alice).expect("alice").library.len(), 0);
        let reveal = outcome
            .events
            .iter()
            .find_map(|event| event.inner().as_any().downcast_ref::<CardRevealedEvent>())
            .expect("explore should reveal the top card");
        assert_eq!(reveal.card, land);
        let keyword = outcome
            .events
            .iter()
            .find_map(|event| event.inner().as_any().downcast_ref::<KeywordActionEvent>())
            .expect("explore should emit a keyword action");
        assert_eq!(keyword.action, KeywordActionKind::Explore);
        assert_eq!(keyword.source, explorer);
        assert_eq!(keyword.player, alice);
        assert!(
            keyword
                .object_tags
                .get(&TagKey::from(crate::effects::PUBLIC_REVEALED_TAG))
                .is_some_and(|snapshots| snapshots
                    .iter()
                    .any(|snapshot| snapshot.object_id == land)),
            "explore keyword action should remember the revealed card"
        );
    }

    #[test]
    fn explore_can_leave_a_nonland_on_top_and_snapshot_the_post_explore_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let explorer = create_creature(&mut game, alice, 52, "Explorer", 2, 2);
        let spell = create_library_card(
            &mut game,
            alice,
            53,
            "Spell",
            vec![CardType::Instant],
            None,
            None,
            None,
        );
        let mut dm = ExploreDecisionMaker {
            boolean_choices: VecDeque::from([false]),
            ..Default::default()
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = ExploreEffect::new(ChooseSpec::SpecificObject(explorer))
            .execute(&mut game, &mut ctx)
            .expect("explore should execute");

        assert_eq!(game.counter_count(explorer, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.player(alice).expect("alice").library.len(), 1);
        assert_eq!(game.player(alice).expect("alice").graveyard.len(), 0);
        assert_eq!(
            game.player(alice).expect("alice").library.last().copied(),
            Some(spell)
        );
        let keyword = outcome
            .events
            .iter()
            .find_map(|event| event.inner().as_any().downcast_ref::<KeywordActionEvent>())
            .expect("explore should emit a keyword action");
        let snapshot = keyword.snapshot.as_ref().expect("explore snapshot");
        assert_eq!(snapshot.counter_count(CounterType::PlusOnePlusOne), 1);
        assert!(
            keyword
                .object_tags
                .get(&TagKey::from(crate::effects::PUBLIC_REVEALED_TAG))
                .is_some_and(|snapshots| snapshots
                    .iter()
                    .any(|snapshot| snapshot.object_id == spell)),
            "explore keyword action should remember a nonland revealed card left on top"
        );
    }

    #[test]
    fn explore_can_put_a_nonland_into_the_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let explorer = create_creature(&mut game, alice, 54, "Explorer", 2, 2);
        create_library_card(
            &mut game,
            alice,
            55,
            "Spell",
            vec![CardType::Sorcery],
            None,
            None,
            None,
        );

        let outcome = ExploreEffect::new(ChooseSpec::SpecificObject(explorer))
            .execute(&mut game, &mut ExecutionContext::new_default(source, alice))
            .expect("explore should execute");

        assert_eq!(game.counter_count(explorer, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.player(alice).expect("alice").library.len(), 0);
        assert_eq!(game.player(alice).expect("alice").graveyard.len(), 1);
        let graveyard_card = game.player(alice).expect("alice").graveyard[0];
        assert_eq!(
            game.object(graveyard_card).expect("graveyard card").name,
            "Spell"
        );
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::KeywordAction),
            "explore should still emit its keyword action after moving the card"
        );
    }

    #[test]
    fn explore_with_an_empty_library_still_puts_a_counter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let explorer = create_creature(&mut game, alice, 56, "Explorer", 2, 2);

        let outcome = ExploreEffect::new(ChooseSpec::SpecificObject(explorer))
            .execute(&mut game, &mut ExecutionContext::new_default(source, alice))
            .expect("explore should execute");

        assert_eq!(game.counter_count(explorer, CounterType::PlusOnePlusOne), 1);
        assert!(
            outcome.events.iter().all(|event| event
                .inner()
                .as_any()
                .downcast_ref::<CardRevealedEvent>()
                .is_none()),
            "empty-library explore should not reveal a card"
        );
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::KeywordAction),
            "empty-library explore should still count as exploring"
        );
        let keyword = outcome
            .events
            .iter()
            .find_map(|event| event.inner().as_any().downcast_ref::<KeywordActionEvent>())
            .expect("explore should emit a keyword action");
        assert!(
            !keyword
                .object_tags
                .contains_key(&TagKey::from(crate::effects::PUBLIC_REVEALED_TAG)),
            "empty-library explore should not pretend a land or nonland card was revealed"
        );
    }

    #[test]
    fn explore_uses_tagged_lki_and_preserves_the_subject_tag_when_the_permanent_left() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let explorer = create_creature(&mut game, alice, 57, "Explorer", 2, 2);
        let snapshot = ObjectSnapshot::from_object(game.object(explorer).expect("explorer"), &game);
        create_library_card(
            &mut game,
            alice,
            58,
            "Forest",
            vec![CardType::Land],
            None,
            None,
            None,
        );
        game.move_object_by_effect(explorer, Zone::Graveyard)
            .expect("moving to graveyard should succeed");

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects("subject", vec![snapshot.clone()]);
        let effect =
            crate::effect::Effect::explore(ChooseSpec::Tagged("subject".into())).tag("explored");
        let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx)
            .expect("tagged explore should execute");

        assert_eq!(game.player(alice).expect("alice").hand.len(), 1);
        let hand_card = game.player(alice).expect("alice").hand[0];
        assert_eq!(game.object(hand_card).expect("hand card").name, "Forest");
        let explored = ctx
            .get_tagged("explored")
            .expect("explored tag should persist");
        assert_eq!(explored.object_id, snapshot.object_id);
        let keyword = outcome
            .events
            .iter()
            .find_map(|event| event.inner().as_any().downcast_ref::<KeywordActionEvent>())
            .expect("explore should emit a keyword action");
        assert_eq!(keyword.source, snapshot.object_id);
        assert_eq!(
            keyword.snapshot.as_ref().map(|entry| entry.controller),
            Some(alice)
        );
    }

    #[test]
    fn explore_uses_controller_choice_order_for_multiple_instructions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_creature(&mut game, alice, 59, "First", 2, 2);
        let second = create_creature(&mut game, alice, 60, "Second", 2, 2);
        create_library_card(
            &mut game,
            alice,
            61,
            "Forest",
            vec![CardType::Land],
            None,
            None,
            None,
        );
        create_library_card(
            &mut game,
            alice,
            62,
            "Spell",
            vec![CardType::Instant],
            None,
            None,
            None,
        );
        let mut dm = ExploreDecisionMaker {
            object_choices: VecDeque::from([vec![second], vec![first]]),
            boolean_choices: VecDeque::from([true]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        ExploreEffect::new(ChooseSpec::all(
            crate::target::ObjectFilter::creature().you_control(),
        ))
        .execute(&mut game, &mut ctx)
        .expect("explore should execute");

        assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.player(alice).expect("alice").hand.len(), 1);
        assert_eq!(game.player(alice).expect("alice").graveyard.len(), 1);
    }

    #[test]
    fn explore_processes_multiple_controllers_in_apnap_order() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;
        let source = game.new_object_id();
        let alice_creature = create_creature(&mut game, alice, 63, "Alice Explorer", 2, 2);
        let bob_creature = create_creature(&mut game, bob, 64, "Bob Explorer", 2, 2);
        create_library_card(
            &mut game,
            alice,
            65,
            "Forest",
            vec![CardType::Land],
            None,
            None,
            None,
        );
        create_library_card(
            &mut game,
            bob,
            66,
            "Island",
            vec![CardType::Land],
            None,
            None,
            None,
        );

        let outcome = ExploreEffect::new(ChooseSpec::all(crate::target::ObjectFilter::creature()))
            .execute(&mut game, &mut ExecutionContext::new_default(source, alice))
            .expect("explore should execute");

        let reveal_players = outcome
            .events
            .iter()
            .filter_map(|event| event.inner().as_any().downcast_ref::<CardRevealedEvent>())
            .map(|event| event.player)
            .collect::<Vec<_>>();
        assert_eq!(
            reveal_players,
            vec![bob, alice],
            "APNAP order should resolve Bob's explore before Alice's here"
        );
        assert_eq!(
            game.counter_count(alice_creature, CounterType::PlusOnePlusOne),
            0
        );
        assert_eq!(
            game.counter_count(bob_creature, CounterType::PlusOnePlusOne),
            0
        );
    }

    #[test]
    fn populate_copies_the_chosen_creature_token() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let _soldier = create_creature_token(&mut game, alice, "Soldier", 1, 1, Subtype::Soldier);
        let rhino = create_creature_token(&mut game, alice, "Rhino", 4, 4, Subtype::Rhino);

        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![rhino]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = PopulateEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("populate should execute");

        let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
            panic!("populate should return created object ids");
        };
        assert_eq!(ids.len(), 1);
        let copy = game.object(ids[0]).expect("created token should exist");
        assert_eq!(copy.kind, ObjectKind::Token);
        assert_eq!(copy.name, "Rhino");
        assert_eq!(game.calculated_power(ids[0]), Some(4));
        assert_eq!(game.calculated_toughness(ids[0]), Some(4));
        let keyword = outcome
            .events
            .iter()
            .find(|event| event.kind() == EventKind::KeywordAction)
            .expect("expected keyword action event")
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(keyword.action, KeywordActionKind::Populate);
    }

    #[test]
    fn manifest_top_card_of_your_library_enters_face_down_under_your_control() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let _card = create_library_card(
            &mut game,
            alice,
            200,
            "Manifest Test Creature",
            vec![CardType::Creature],
            Some(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::Green,
            ])),
            Some(3),
            Some(3),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ManifestTopCardOfLibraryEffect::new(PlayerFilter::You)
            .execute(&mut game, &mut ctx)
            .expect("manifest should execute");

        let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
            panic!("manifest should return the manifested object id");
        };
        let manifested_id = *ids.first().expect("manifest should create one permanent");
        let manifested = game
            .object(manifested_id)
            .expect("manifested permanent should exist");

        assert_eq!(game.controller_of(manifested), alice);
        assert!(game.is_face_down(manifested_id));
        assert!(game.is_manifested(manifested_id));
        assert_eq!(game.calculated_power(manifested_id), Some(2));
        assert_eq!(game.calculated_toughness(manifested_id), Some(2));
        let keyword = outcome
            .events
            .iter()
            .find(|event| event.kind() == EventKind::KeywordAction)
            .expect("expected keyword action event")
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(keyword.action, KeywordActionKind::Manifest);
    }

    #[test]
    fn manifest_top_card_of_that_players_library_uses_effect_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let _card = create_library_card(
            &mut game,
            bob,
            201,
            "Stolen Manifest Card",
            vec![CardType::Creature],
            Some(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::Blue,
            ])),
            Some(4),
            Some(4),
        );
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);

        let outcome =
            ManifestTopCardOfLibraryEffect::new(PlayerFilter::TargetPlayerOrControllerOfTarget)
                .execute(&mut game, &mut ctx)
                .expect("manifest from that player's library should execute");

        let manifested_id = outcome
            .value
            .objects()
            .and_then(|ids| ids.first().copied())
            .expect("manifest should create one permanent");
        let manifested = game
            .object(manifested_id)
            .expect("manifested permanent should exist");
        assert_eq!(manifested.owner, bob);
        assert_eq!(game.controller_of(manifested), alice);
        assert!(game.is_face_down(manifested_id));
    }

    #[test]
    fn scroll_of_fate_manifest_from_hand_uses_chosen_hand_card() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let first_card = CardBuilder::new(CardId::new(), "Chosen Manifest Card")
            .card_types(vec![CardType::Creature])
            .build();
        let first_id = game.create_object_from_card(&first_card, alice, Zone::Hand);
        let second_card = CardBuilder::new(CardId::new(), "Unchosen Hand Card")
            .card_types(vec![CardType::Creature])
            .build();
        let second_id = game.create_object_from_card(&second_card, alice, Zone::Hand);
        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![first_id]]),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        let outcome = ManifestCardFromHandEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("manifest from hand should execute");

        let manifested_id = outcome
            .value
            .objects()
            .and_then(|ids| ids.first().copied())
            .expect("manifest from hand should create one permanent");
        assert!(game.is_face_down(manifested_id));
        assert!(game.is_manifested(manifested_id));
        assert!(
            game.player(alice)
                .is_some_and(|player| player.hand.contains(&second_id)),
            "unchosen card should remain in hand"
        );
        assert!(
            game.player(alice)
                .is_none_or(|player| !player.hand.contains(&first_id)),
            "chosen card should leave hand"
        );
    }

    #[test]
    fn scroll_of_fate_manifest_from_hand_with_empty_hand_does_nothing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ManifestCardFromHandEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("empty-hand manifest from hand should execute");

        assert!(outcome.value.objects().is_none_or(|ids| ids.is_empty()));
    }

    #[test]
    fn populate_multiple_times_reprompts_and_emits_per_iteration_events() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let soldier = create_creature_token(&mut game, alice, "Soldier", 1, 1, Subtype::Soldier);
        let rhino = create_creature_token(&mut game, alice, "Rhino", 4, 4, Subtype::Rhino);

        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![soldier], vec![rhino]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = PopulateEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("populate twice should execute");

        let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
            panic!("populate should return created object ids");
        };
        assert_eq!(ids.len(), 2);
        let created_names = ids
            .iter()
            .filter_map(|id| game.object(*id))
            .map(|obj| obj.name.clone())
            .collect::<Vec<_>>();
        assert!(created_names.contains(&"Soldier".to_string()));
        assert!(created_names.contains(&"Rhino".to_string()));
        assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| event.kind() == EventKind::KeywordAction)
                .count(),
            2
        );
    }

    #[test]
    fn populate_with_no_creature_tokens_creates_nothing_but_still_performs_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = PopulateEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("populate with no tokens should resolve");

        let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
            panic!("populate should return created object ids");
        };
        assert!(ids.is_empty());
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(outcome.events[0].kind(), EventKind::KeywordAction);
    }

    #[test]
    fn populate_applies_collapsed_token_copy_modifiers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, alice, 30, "Populate Source", 2, 2);
        let rhino = create_creature_token(&mut game, alice, "Rhino", 4, 4, Subtype::Rhino);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: source,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![rhino]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = PopulateEffect::new(1)
            .enters_tapped(true)
            .attacking(true)
            .haste(true)
            .sacrifice_at_next_end_step(true)
            .execute(&mut game, &mut ctx)
            .expect("populate with modifiers should execute");

        let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
            panic!("populate should return created object ids");
        };
        let token_id = *ids.first().expect("populate should create one token");
        assert!(
            game.is_tapped(token_id),
            "populated token should enter tapped"
        );
        assert!(
            game.object_has_static_ability_id(token_id, StaticAbilityId::Haste),
            "populated token should gain haste"
        );
        let combat = game.combat.as_ref().expect("combat should still be active");
        let token_attacker = combat
            .attackers
            .iter()
            .find(|info| info.creature == token_id)
            .expect("populated token should enter attacking");
        assert_eq!(token_attacker.target, AttackTarget::Player(bob));
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        assert_eq!(
            game.effect_store.delayed_triggers[0].target_objects,
            vec![token_id]
        );
    }

    #[test]
    fn support_exposes_up_to_n_other_target_creatures() {
        let effect = SupportEffect::new(3);
        let target = effect
            .get_target_spec()
            .expect("support should expose target metadata");

        assert!(target.is_target(), "support should target creatures");
        assert_eq!(effect.get_target_count(), Some(ChoiceCount::up_to(3)));

        let ChooseSpec::Target(inner) = target else {
            panic!("support should use a targeted ChooseSpec");
        };
        let ChooseSpec::Object(filter) = inner.as_ref() else {
            panic!("support target should resolve to an object filter");
        };
        assert!(
            filter.other,
            "support on permanents must use other creatures"
        );
        assert!(
            filter.card_types.contains(&CardType::Creature),
            "support should only target creatures"
        );
    }

    #[test]
    fn support_puts_one_counter_on_each_chosen_creature_and_emits_keyword_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 40, "Support Source", 2, 2);
        let first = create_creature(&mut game, alice, 41, "First Ally", 2, 2);
        let second = create_creature(&mut game, alice, 42, "Second Ally", 2, 2);
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            crate::effects::ResolvedTarget::Object(first),
            crate::effects::ResolvedTarget::Object(second),
        ]);

        let outcome = SupportEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("support should execute");

        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
        let keyword = outcome
            .events
            .iter()
            .find(|event| event.kind() == EventKind::KeywordAction)
            .expect("expected keyword action event")
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action payload");
        assert_eq!(keyword.action, KeywordActionKind::Support);
        assert_eq!(keyword.amount, 2);
    }

    #[test]
    fn support_on_spell_source_can_target_fewer_than_n_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let target = create_creature(&mut game, alice, 43, "Spell Support Target", 2, 2);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);

        SupportEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("support from a spell source should execute");

        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 1);
    }

    #[test]
    fn support_with_zero_targets_still_resolves_and_emits_keyword_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = SupportEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("support with zero chosen targets should resolve");

        assert_eq!(outcome.events.len(), 1);
        let keyword = outcome.events[0]
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action payload");
        assert_eq!(keyword.action, KeywordActionKind::Support);
        assert_eq!(keyword.amount, 2);
    }

    #[test]
    fn bolster_chooses_among_least_toughness_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_creature(&mut game, alice, 1, "First", 1, 1);
        let second = create_creature(&mut game, alice, 2, "Second", 1, 1);
        let _largest = create_creature(&mut game, alice, 3, "Largest", 4, 4);
        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![second]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = BolsterEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("execute bolster");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 2);
        let keyword = outcome
            .events
            .iter()
            .find(|event| event.kind() == EventKind::KeywordAction)
            .expect("expected keyword action event")
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(keyword.action, KeywordActionKind::Bolster);
    }

    #[test]
    fn bolster_does_nothing_without_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = BolsterEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("bolster without creatures should resolve");

        assert!(!outcome.something_happened());
        assert!(
            outcome.events.is_empty(),
            "bolster should not emit events when no creature can be chosen"
        );
    }

    #[test]
    fn bolster_pauses_for_tied_creature_choice_instead_of_defaulting() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_creature(&mut game, alice, 1, "First", 1, 1);
        let second = create_creature(&mut game, alice, 2, "Second", 1, 1);
        let mut dm = PromptingDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = BolsterEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("bolster should wait for a choice");

        assert!(ctx.decision_maker.awaiting_choice());
        assert!(!outcome.something_happened());
        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 0);
        assert!(
            outcome.events.is_empty(),
            "no bolster event should fire before a choice is made"
        );
    }

    #[test]
    fn devour_sacrifices_exactly_the_chosen_creatures_and_emits_sacrifice_events() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 10, "Devourer", 2, 2);
        let first = create_creature(&mut game, alice, 11, "First Food", 1, 1);
        let second = create_creature(&mut game, alice, 12, "Second Food", 1, 1);
        let keep = create_creature(&mut game, alice, 13, "Keep", 3, 3);
        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![second]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = DevourEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("execute devour");

        assert!(game.battlefield.contains(&source));
        assert!(game.battlefield.contains(&first));
        assert!(!game.battlefield.contains(&second));
        assert!(game.battlefield.contains(&keep));
        assert_eq!(game.players[0].graveyard.len(), 1);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 2);
        assert!(
            outcome
                .events_of_type::<crate::events::permanents::SacrificeEvent>()
                .count()
                == 1,
            "expected devour to emit one sacrifice event"
        );
    }

    #[test]
    fn devour_batches_multiple_deaths_for_lki_triggers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 20, "Devourer", 2, 2);
        let cutthroat_like = CardDefinitionBuilder::new(CardId::from_raw(21), "Cutthroat-Like")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .with_trigger(
                crate::triggers::Trigger::or(vec![
                    crate::triggers::Trigger::this_dies(),
                    crate::triggers::Trigger::dies(
                        crate::target::ObjectFilter::creature().you_control(),
                    ),
                ]),
                Vec::new(),
            )
            .build();
        let first = game.create_object_from_definition(&cutthroat_like, alice, Zone::Battlefield);
        let second = create_creature(&mut game, alice, 22, "Second Food", 1, 1);
        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![first, second]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = DevourEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("execute devour");

        assert_eq!(
            outcome
                .events_of_type::<crate::events::permanents::SacrificeEvent>()
                .count(),
            2,
            "expected one sacrifice event per sacrificed creature"
        );
        let zone_changes = game
            .effect_store
            .pending_trigger_events
            .iter()
            .filter_map(|event| event.downcast::<ZoneChangeEvent>())
            .collect::<Vec<_>>();
        assert_eq!(zone_changes.len(), 1, "expected one batched death event");
        assert_eq!(zone_changes[0].objects.len(), 2);
        assert_eq!(zone_changes[0].result_objects.len(), 2);
        assert_eq!(zone_changes[0].snapshots().len(), 2);

        let triggered =
            crate::triggers::check_triggers(&game, &game.effect_store.pending_trigger_events[0]);
        assert_eq!(
            triggered.len(),
            2,
            "the dying source should see both simultaneous creature deaths"
        );
    }

    #[test]
    fn backup_puts_counter_on_target_and_grants_following_ability_to_another_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 20, "Backup Source", 2, 2);
        let target = create_creature(&mut game, alice, 21, "Backup Target", 1, 1);
        let granted = Ability::static_ability(StaticAbility::flying());
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);

        let outcome = BackupEffect::new(1, vec![granted])
            .execute(&mut game, &mut ctx)
            .expect("execute backup");

        assert!(outcome.something_happened());
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 1);
        assert!(
            game.object_has_static_ability_id(target, StaticAbilityId::Flying),
            "backup target should gain the granted ability until end of turn"
        );
    }

    #[test]
    fn backup_grants_triggered_ability_directly_to_another_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 22, "Backup Source", 2, 2);
        let target = create_creature(&mut game, alice, 23, "Backup Target", 1, 1);
        let granted = Ability::triggered(
            crate::triggers::Trigger::this_deals_combat_damage_to_player(
                crate::target::PlayerFilter::Any,
            ),
            vec![],
        );
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);

        BackupEffect::new(1, vec![granted])
            .execute(&mut game, &mut ctx)
            .expect("execute backup");

        let abilities = game
            .current_abilities(target)
            .expect("target should have calculated abilities");
        assert!(
            abilities
                .iter()
                .any(|ability| matches!(ability.kind, crate::ability::AbilityKind::Triggered(_))),
            "backup should grant the triggered ability itself, not a nested static grant wrapper"
        );
    }

    #[test]
    fn adapt_puts_plus_one_counters_on_source_when_it_has_none() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 50, "Adapt Source", 2, 2);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = AdaptEffect::new(3)
            .execute(&mut game, &mut ctx)
            .expect("adapt should execute");

        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 3);
        assert!(outcome.has_marker_change(|event| {
            event.is_added()
                && event.object() == Some(source)
                && event.marker == CounterType::PlusOnePlusOne.into()
        }));
    }

    #[test]
    fn adapt_does_nothing_when_source_already_has_plus_one_counter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 51, "Already Adapted", 2, 2);
        game.object_mut(source)
            .expect("source should exist")
            .add_counters(CounterType::PlusOnePlusOne, 1);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = AdaptEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("adapt should execute");

        assert_eq!(outcome.value, OutcomeValue::Count(0));
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1);
        assert!(
            outcome.events.is_empty(),
            "adapt should not emit marker events when it is blocked by an existing +1/+1 counter"
        );
    }

    #[test]
    fn adapt_ignores_other_counter_types() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 52, "Charge Counter Creature", 2, 2);
        game.object_mut(source)
            .expect("source should exist")
            .add_counters(CounterType::Charge, 2);
        let mut ctx = ExecutionContext::new_default(source, alice);

        AdaptEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("adapt should ignore non +1/+1 counters");

        assert_eq!(game.counter_count(source, CounterType::Charge), 2);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 2);
    }

    #[test]
    fn adapt_returns_target_invalid_when_source_is_missing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = AdaptEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("adapt should resolve cleanly when the source is gone");

        assert_eq!(outcome, EffectOutcome::target_invalid());
    }
}

//! Explicit mechanic effects used by parser/rendering for supported wording.
//!
//! These mechanics are represented as first-class effects so parser output does
//! not depend on raw oracle text passthrough for rendering.

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{ChoiceCount, Effect, EffectOutcome, ExecutionFact, OutcomeValue, Until};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{normalize_object_selection, resolve_value};
use crate::effects::permanents::face_down_entry::{
    manifest_card_with_outputs, prepare_manifest_card, prepare_manifest_entry,
    rollback_manifest_preparation,
};
use crate::effects::player::CastTaggedEffect;
use crate::effects::zones::apply_zone_change;
use crate::effects::zones::apply_zone_change_with_context_and_additional_effects_with_outputs;
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_batch_with_options,
    move_to_battlefield_with_options,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::Event;
use crate::events::permanents::SacrificeEvent;
use crate::events::processing::{
    EventOutcome, TraitEventResult, process_trait_event_with_execution_context,
};
use crate::events::zones::ZoneChangeEvent;
use crate::events::{CardRevealedEvent, KeywordActionEvent, KeywordActionKind};
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilter;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::object::{CounterType, ObjectKind};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::zone::Zone;
use std::collections::HashMap;
pub type AmplifyEffect = ironsmith_core::AmplifyEffect;
pub use ironsmith_core::{
    BolsterEffect, CipherEffect, DevourEffect, ResolvesDespiteIllegalTargetsEffect,
};

impl EffectExecutor for ResolvesDespiteIllegalTargetsEffect {
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
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        for ability in &self.granted_abilities {
            crate::ability::visit_owned_effects(ability, visitor);
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
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
                    .execute_child_with_outputs(game, ctx)?,
                ];

                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let primary = outcomes[0].outcome.summary_projection();
                if target != ctx.source
                    && game
                        .object(target)
                        .is_some_and(|object| object.zone == Zone::Battlefield)
                    && !game.is_phased_out(target)
                {
                    for ability in &self.granted_abilities {
                        outcomes.push(
                            crate::effects::ApplyContinuousEffect::new(
                                crate::continuous::EffectTarget::Specific(target),
                                crate::continuous::Modification::AddAbilityGeneric(ability.clone()),
                                Until::EndOfTurn,
                            )
                            .execute_child_with_outputs(game, ctx)?,
                        );
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                    }
                }

                Ok(crate::effects::CompletedEffectOutputs::from_children(
                    outcomes,
                    |children| EffectOutcome::aggregate_with_primary_result(primary, children),
                ))
            },
        )
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
    game.team_apnap_player_order()
}

pub(crate) fn execute_keyword_action_replacement_effects_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: Vec<Effect>,
    source: ObjectId,
    controller: PlayerId,
    context: &crate::events::processing::ReplacementEventContext,
    action_snapshot: Option<ObjectSnapshot>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let tags = crate::events::downcast_event::<KeywordActionEvent>(context.event.inner())
        .map(|event| event.object_tags.clone())
        .unwrap_or_default();
    let tags =
        super::keyword_action::keyword_action_object_bindings(tags, action_snapshot.as_ref());
    crate::effects::replacement::execute_replacement_payload_with_outputs(
        game, ctx, &effects, source, controller, context, None, None, tags,
    )
}

/// Put the explore +1/+1 counter through the normal counter-placement
/// pipeline so replacements such as Hardened Scales and "can't have counters"
/// effects apply (CR 614.1, 122.6, 701.44a).
fn put_explore_counter_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    object_id: crate::ids::ObjectId,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::PutCountersEffect::new(
        CounterType::PlusOnePlusOne,
        1,
        ChooseSpec::SpecificObject(object_id),
    )
    .execute_child_with_outputs(game, ctx)
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
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
                    match crate::effects::helpers::resolve_objects_for_effect(
                        game,
                        ctx,
                        &self.target,
                    ) {
                        Ok(ids) => ids,
                        // CR 701.44c: a source that changed zones still explores,
                        // using its last known information.
                        Err(ExecutionError::InvalidTarget)
                            if matches!(self.target.base(), ChooseSpec::Source)
                                && ctx.source_snapshot.is_some() =>
                        {
                            vec![ctx.source]
                        }
                        Err(ExecutionError::InvalidTarget) if self.target.is_target() => {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::target_invalid(),
                            ));
                        }
                        Err(ExecutionError::InvalidTarget) => {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        Err(err) => return Err(err),
                    }
                };
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                if target_ids.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        if self.target.is_target() {
                            EffectOutcome::target_invalid()
                        } else {
                            EffectOutcome::count(0)
                        },
                    ));
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
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        if self.target.is_target() {
                            EffectOutcome::target_invalid()
                        } else {
                            EffectOutcome::count(0)
                        },
                    ));
                }

                let mut children = Vec::new();
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
                            let selection: Vec<ObjectId> = make_decision(
                                game,
                                ctx.decision_maker,
                                player,
                                Some(ctx.source),
                                spec,
                            );
                            if ctx.decision_maker.awaiting_choice() {
                                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                    EffectOutcome::with_objects(explored_objects),
                                ));
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
                        let iteration_outcome =
                            crate::effects::composition::execute_keyword_action_with_outputs(
                                game,
                                ctx,
                                would_event,
                                crate::effects::composition::KeywordActionOutput::Objects,
                                crate::effects::composition::KeywordActionAmount::Repetitions,
                                |game, ctx, action| {
                                    let controller = action.player;
                                    let target_id = action.source;
                                    let mut explored_objects = Vec::new();
                                    let mut children = Vec::new();

                                    let revealed_card_id = game
                                        .player(controller)
                                        .and_then(|entry| entry.library.last().copied());
                                    let revealed_snapshot = revealed_card_id.and_then(|card_id| {
                                        game.object(card_id)
                                            .map(|object| ObjectSnapshot::from_object(object, game))
                                    });
                                    let reveal =
                                        crate::effects::cards::reveal_objects_with_outputs(
                                            game,
                                            ctx,
                                            revealed_snapshot.clone().into_iter().collect(),
                                            Some(controller),
                                            "Reveal the top card of a library",
                                            None,
                                        )?;
                                    children.push(reveal);
                                    if ctx.decision_maker.awaiting_choice() {
                                        return Ok(
                                            crate::effects::CompletedEffectOutputs::aggregate_only(
                                                EffectOutcome::count(0),
                                            ),
                                        );
                                    }

                                    let revealed_is_land = revealed_card_id
                                        .and_then(|card_id| game.object(card_id))
                                        .is_some_and(|object| {
                                            object.has_card_type(crate::types::CardType::Land)
                                        });

                                    if let Some(card_id) = revealed_card_id {
                                        if revealed_is_land {
                                            let movement = crate::effects::MoveToZoneEffect::new(
                                                ChooseSpec::SpecificObject(card_id),
                                                Zone::Hand,
                                                false,
                                            )
                                            .execute_child_with_outputs(game, ctx)?;
                                            if ctx.decision_maker.awaiting_choice() {
                                                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                                            }
                                            children.push(movement);
                                            if ctx.decision_maker.awaiting_choice() {
                                                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                                            }
                                        } else {
                                            if game.object(target_id).is_some() {
                                                let placement = put_explore_counter_with_outputs(
                                                    game, ctx, target_id,
                                                )?;
                                                if ctx.decision_maker.awaiting_choice() {
                                                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                                                }
                                                children.push(placement);
                                            }

                                            let choice_ctx =
                                                crate::decisions::context::BooleanContext::new(
                                                    controller,
                                                    Some(ctx.source),
                                                    "Put the explored card into your graveyard?"
                                                        .to_string(),
                                                );
                                            let put_into_graveyard = ctx
                                                .decision_maker
                                                .decide_boolean(game, &choice_ctx);
                                            if ctx.decision_maker.awaiting_choice() {
                                                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                                            }
                                            if put_into_graveyard {
                                                let movement =
                                                    crate::effects::MoveToZoneEffect::new(
                                                        ChooseSpec::SpecificObject(card_id),
                                                        Zone::Graveyard,
                                                        false,
                                                    )
                                                    .execute_child_with_outputs(game, ctx)?;
                                                if ctx.decision_maker.awaiting_choice() {
                                                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                                                }
                                                children.push(movement);
                                                if ctx.decision_maker.awaiting_choice() {
                                                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                                                }
                                            }
                                        }
                                    } else if game.object(target_id).is_some() {
                                        let placement =
                                            put_explore_counter_with_outputs(game, ctx, target_id)?;
                                        if ctx.decision_maker.awaiting_choice() {
                                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                                        }
                                        children.push(placement);
                                    }

                                    let action_snapshot = game
                                        .object(target_id)
                                        .map(|object| ObjectSnapshot::from_object(object, game))
                                        .or_else(|| action.snapshot.clone());
                                    let object_tags = revealed_snapshot
                                        .clone()
                                        .map(|snapshot| {
                                            HashMap::from([(
                                                TagKey::from(crate::effects::PUBLIC_REVEALED_TAG),
                                                vec![snapshot],
                                            )])
                                        })
                                        .unwrap_or_default();
                                    let completion = KeywordActionEvent::new(
                                        KeywordActionKind::Explore,
                                        controller,
                                        target_id,
                                        1,
                                    )
                                    .with_snapshot(action_snapshot)
                                    .with_object_tags(object_tags);
                                    explored_objects.push(target_id);
                                    let original =
                                        crate::effects::CompletedEffectOutputs::from_children(
                                            children,
                                            |children| {
                                                EffectOutcome::aggregate_with_primary_result(
                                                    EffectOutcome::with_objects(explored_objects),
                                                    children,
                                                )
                                            },
                                        );
                                    super::complete_keyword_action_with_outputs(
                                        game, ctx, original, completion,
                                    )
                                },
                            )?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        if let Some(objects) = iteration_outcome.outcome.objects() {
                            explored_objects.extend_from_slice(objects);
                        }
                        children.push(iteration_outcome);
                    }
                }

                Ok(crate::effects::CompletedEffectOutputs::from_children(
                    children,
                    |children| {
                        EffectOutcome::aggregate_with_primary_result(
                            EffectOutcome::with_objects(explored_objects),
                            children,
                        )
                    },
                ))
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent to explore"
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenAttractionEffect {
    pub reminder: bool,
}

impl Default for OpenAttractionEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAttractionEffect {
    pub fn new() -> Self {
        Self { reminder: false }
    }

    pub fn with_reminder(mut self, reminder: bool) -> Self {
        self.reminder = reminder;
        self
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                // CR 701.51b: "you" is the controller of the instruction, not
                // whoever controls its source now.
                let controller = ctx.controller;

                // CR 701.51a-b: only a player with an Attraction deck can open one,
                // and opening moves that deck's top card face up onto the battlefield
                // under that player's control.
                let Some(attraction) = game.top_attraction(controller) else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                let receipt = move_to_battlefield_with_options(
                    game,
                    ctx,
                    attraction,
                    BattlefieldEntryOptions::specific(controller, false),
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let receipt = receipt.ok_or_else(|| {
                    ExecutionError::InternalError(
                        "attraction entry returned no terminal receipt".into(),
                    )
                })?;
                let original: Result<crate::effects::CompletedEffectOutputs, ExecutionError> =
                    match &receipt.outcome {
                        BattlefieldEntryOutcome::Moved(new_id) => {
                            game.finish_opening_attraction(controller, attraction, Some(*new_id));
                            Ok(super::complete_keyword_action_with_outputs(
                                game,
                                ctx,
                                crate::effects::CompletedEffectOutputs::aggregate_only(
                                    EffectOutcome::with_objects(vec![*new_id]),
                                ),
                                KeywordActionEvent::new(
                                    KeywordActionKind::OpenAttraction,
                                    controller,
                                    ctx.source,
                                    1,
                                ),
                            )?)
                        }
                        BattlefieldEntryOutcome::Redirected(receipt) => {
                            game.finish_opening_attraction(controller, attraction, None);
                            Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0)
                                    .with_affected_objects(receipt.new_object_ids.clone()),
                            ))
                        }
                        BattlefieldEntryOutcome::Prevented => {
                            // The card was moved off the supplementary deck before the
                            // entry attempt (CR 701.51b), even though no open trigger is
                            // created when entry is prevented or replaced (CR 701.51c).
                            game.finish_opening_attraction(controller, attraction, None);
                            Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ))
                        }
                    };
                let original = original?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let mut outputs =
                    crate::effects::zones::finish_battlefield_entry_receipts_with_outputs(
                        game,
                        ctx,
                        original.outcome.clone(),
                        vec![receipt],
                    )?;
                if !ctx.decision_maker.awaiting_choice() {
                    outputs.retain_published_children([original]);
                }
                Ok(outputs)
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestDreadEffect {
    pub player: PlayerFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestTopCardOfLibraryEffect {
    pub player: PlayerFilter,
    pub cloak: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestCardFromHandEffect;

pub type ManifestObjectsEffect = ironsmith_core::ManifestObjectsEffect;

impl ManifestTopCardOfLibraryEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            cloak: false,
        }
    }

    pub fn cloak(player: PlayerFilter) -> Self {
        Self {
            player,
            cloak: true,
        }
    }
}

impl Default for ManifestDreadEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl ManifestDreadEffect {
    pub fn new() -> Self {
        Self {
            player: PlayerFilter::You,
        }
    }
    pub fn for_player(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl Default for ManifestCardFromHandEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl ManifestCardFromHandEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for ManifestObjectsEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let controller =
                    crate::effects::helpers::resolve_player_filter(game, &self.controller, ctx)?;
                let mut object_ids =
                    crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let mut seen = std::collections::HashSet::new();
                object_ids.retain(|object_id| {
                    seen.insert(*object_id)
                        && game
                            .object(*object_id)
                            .is_some_and(|object| object.kind == ObjectKind::Card)
                });
                if object_ids.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        if self.target.is_target() {
                            EffectOutcome::target_invalid()
                        } else {
                            EffectOutcome::count(0)
                        },
                    ));
                }
                if self.shuffle {
                    game.shuffle_slice(&mut object_ids);
                }

                // CR 701.40e / 701.58e: cards from a library enter one at a time.
                // Prepare each face-down object only immediately before its own entry,
                // so replacements and entry events observe earlier manifested cards.
                let library_owner = game.object(object_ids[0]).map(|object| object.owner);
                if object_ids.iter().all(|id| {
                    game.object(*id).is_some_and(|object| {
                        object.zone == Zone::Library && Some(object.owner) == library_owner
                    })
                }) {
                    let mut moved_ids = Vec::new();
                    let mut affected_memory = Vec::new();
                    let mut phase_outcomes = Vec::new();
                    for object_id in object_ids {
                        let mut phase = EffectOutcome::resolved();
                        let mut keyword_outputs = None;
                        let Some(object) = game.object(object_id) else {
                            continue;
                        };
                        let memory = Clone::clone(&ObjectSnapshot::from_object(object, game));
                        let Some(preparation) = prepare_manifest_card(game, object_id, self.cloak)
                        else {
                            continue;
                        };
                        let receipt = move_to_battlefield_with_options(
                            game,
                            ctx,
                            object_id,
                            BattlefieldEntryOptions::specific(controller, self.tapped),
                        )?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        let receipt = receipt.ok_or_else(|| {
                            ExecutionError::InternalError(
                                "sequential manifest entry returned no terminal receipt".into(),
                            )
                        })?;
                        match &receipt.outcome {
                            BattlefieldEntryOutcome::Moved(new_id) => {
                                if self.cloak {
                                    game.set_cloaked(*new_id);
                                } else {
                                    game.set_manifested(*new_id);
                                }
                                moved_ids.push(*new_id);
                                affected_memory.push(memory);
                                let keyword = super::complete_keyword_action_with_outputs(
                                    game,
                                    ctx,
                                    crate::effects::CompletedEffectOutputs::aggregate_only(phase),
                                    KeywordActionEvent::new(
                                        if self.cloak {
                                            KeywordActionKind::Cloak
                                        } else {
                                            KeywordActionKind::Manifest
                                        },
                                        controller,
                                        ctx.source,
                                        1,
                                    ),
                                )?;
                                if ctx.decision_maker.awaiting_choice() {
                                    return Ok(
                                        crate::effects::CompletedEffectOutputs::aggregate_only(
                                            EffectOutcome::count(0),
                                        ),
                                    );
                                }
                                phase = keyword.outcome.clone();
                                keyword_outputs = Some(keyword);
                            }
                            BattlefieldEntryOutcome::Redirected(receipt) => {
                                moved_ids.extend(receipt.new_object_ids.iter().copied());
                                affected_memory.push(memory);
                            }
                            BattlefieldEntryOutcome::Prevented => {
                                rollback_manifest_preparation(game, &preparation);
                            }
                        }
                        let mut phase =
                            crate::effects::zones::finish_battlefield_entry_receipts_with_outputs(
                                game,
                                ctx,
                                phase,
                                vec![receipt],
                            )?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        phase.retain_published_children(keyword_outputs);
                        phase_outcomes.push(phase);
                    }
                    let original = EffectOutcome::with_objects(moved_ids)
                        .with_affected_object_memory(affected_memory);
                    return Ok(crate::effects::CompletedEffectOutputs::from_children(
                        phase_outcomes,
                        |phases| EffectOutcome::aggregate_with_primary_result(original, phases),
                    ));
                }

                let mut entries = Vec::with_capacity(object_ids.len());
                for object_id in object_ids {
                    let Some(object) = game.object(object_id) else {
                        continue;
                    };
                    let memory = Clone::clone(&ObjectSnapshot::from_object(object, game));
                    let Some(preparation) = prepare_manifest_card(game, object_id, self.cloak)
                    else {
                        continue;
                    };
                    entries.push((preparation, memory));
                }

                let requests = entries
                    .iter()
                    .map(|(preparation, _)| {
                        (
                            preparation.object_id,
                            BattlefieldEntryOptions::specific(controller, self.tapped),
                        )
                    })
                    .collect();
                crate::effects::zones::execute_battlefield_entries_with_outputs(
                    game,
                    ctx,
                    requests,
                    false,
                    |game, ctx, receipts| {
                        let mut moved_ids = Vec::new();
                        let mut affected_memory = Vec::new();
                        let mut entered_count = 0u32;
                        for ((preparation, memory), receipt) in entries.into_iter().zip(receipts) {
                            match &receipt.outcome {
                                BattlefieldEntryOutcome::Moved(id) => {
                                    entered_count += 1;
                                    if self.cloak {
                                        game.set_cloaked(*id);
                                    } else {
                                        game.set_manifested(*id);
                                    }
                                    moved_ids.push(*id);
                                    affected_memory.push(memory);
                                }
                                BattlefieldEntryOutcome::Redirected(change) => {
                                    moved_ids.extend(change.new_object_ids.iter().copied());
                                    affected_memory.push(memory);
                                }
                                BattlefieldEntryOutcome::Prevented => {
                                    rollback_manifest_preparation(game, &preparation)
                                }
                            }
                        }
                        if moved_ids.is_empty() {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        let mut original = crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::with_objects(moved_ids)
                                .with_affected_object_memory(affected_memory),
                        );
                        if entered_count > 0 {
                            let action = if self.cloak {
                                KeywordActionKind::Cloak
                            } else {
                                KeywordActionKind::Manifest
                            };
                            original = super::complete_keyword_action_with_outputs(
                                game,
                                ctx,
                                original,
                                KeywordActionEvent::new(
                                    action,
                                    controller,
                                    ctx.source,
                                    entered_count,
                                ),
                            )?;
                        }
                        Ok(original)
                    },
                )
                .map(|commit| commit.outcome)
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target.is_target().then_some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "cards to manifest"
    }
}

impl EffectExecutor for ManifestDreadEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player =
                    crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
                const LOOKED: &str = "__manifest_dread_look";
                let look = crate::effects::LookAtTopCardsEffect::new(
                    PlayerFilter::Specific(player),
                    2,
                    LOOKED,
                )
                .viewed_by(PlayerFilter::Specific(player));
                let observed = ctx.with_object_tag(LOOKED, Vec::new(), |ctx| {
                    look.execute_child_with_outputs(game, ctx)
                })?;
                let observed_outcome = observed.outcome.clone();
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let top_cards = observed_outcome
                    .affected_object_memory()
                    .unwrap_or_default()
                    .iter()
                    .map(|snapshot| snapshot.object_id)
                    .collect::<Vec<_>>();
                if top_cards.is_empty() {
                    let object_tags = HashMap::from([(
                        TagKey::from(crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG),
                        Vec::new(),
                    )]);
                    return Ok(super::complete_keyword_action_with_outputs(
                        game,
                        ctx,
                        crate::effects::CompletedEffectOutputs::from_children(
                            [observed],
                            |children| {
                                EffectOutcome::aggregate_with_primary_result(
                                    EffectOutcome::count(0),
                                    children,
                                )
                            },
                        ),
                        KeywordActionEvent::new(
                            KeywordActionKind::ManifestDread,
                            player,
                            ctx.source,
                            1,
                        )
                        .with_object_tags(object_tags),
                    )?);
                }

                let card_to_manifest = if top_cards.len() == 1 {
                    top_cards[0]
                } else {
                    let selection = make_decision(
                game,
                ctx.decision_maker,
                player,
                Some(ctx.source),
                ChooseObjectsSpec::new(
                    ctx.source,
                    "Choose one of the top two cards of your library to manifest",
                    top_cards.clone(),
                    1,
                    Some(1),
                )
                .require_explicit_choice()
                .with_hidden_card_visibility(
                    crate::decisions::context::DecisionHiddenCardVisibility::PrivateToDecisionPlayer,
                ),
            );
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }

                    normalize_object_selection(selection, &top_cards, 1)
                        .first()
                        .copied()
                        .unwrap_or(top_cards[0])
                };

                let (mut outcome, entry_receipt) =
                    prepare_manifest_entry(game, ctx, card_to_manifest, player, false)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        outcome,
                    ));
                }
                let mut entry_outputs = Vec::new();
                let mut receipts = entry_receipt
                    .into_iter()
                    .map(|receipt| {
                        let (receipt, outputs) = receipt.into_zone_receipt_with_outputs();
                        crate::effects::PublishedEffectOutputs::append_distinct(
                            &mut entry_outputs,
                            outputs,
                        );
                        receipt
                    })
                    .collect::<Vec<_>>();
                let mut graveyard_snapshots = Vec::new();
                // CR 701.62a: every looked-at card that wasn't manifested goes to
                // the graveyard, including the selected card if entry was prohibited.
                // A card moved elsewhere by a replacement is no longer in this group.
                for card_id in top_cards {
                    if !game
                        .object(card_id)
                        .is_some_and(|card| card.zone == Zone::Library)
                    {
                        continue;
                    }
                    let committed = {
                        let zone_additional_effects = ctx.additional_replacement_effects_snapshot();
                        apply_zone_change_with_context_and_additional_effects_with_outputs(
                            game,
                            card_id,
                            Zone::Library,
                            Zone::Graveyard,
                            ctx.cause.clone(),
                            ctx,
                            &zone_additional_effects,
                        )
                    }?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    crate::effects::PublishedEffectOutputs::append_distinct(
                        &mut entry_outputs,
                        committed.published_outputs,
                    );
                    let receipt = committed.receipt;
                    if let EventOutcome::Proceed(result) = &receipt.original
                        && result.final_zone == Zone::Graveyard
                    {
                        graveyard_snapshots.extend(
                            result.new_object_ids.iter().copied().filter_map(|id| {
                                game.object(id).map(|object| {
                                    ObjectSnapshot::from_object_with_calculated_characteristics(
                                        object, game,
                                    )
                                })
                            }),
                        );
                    }
                    receipts.push((card_id, receipt));
                }

                let primary = outcome.summary_projection();
                outcome = EffectOutcome::aggregate_with_primary_result(
                    primary,
                    [observed_outcome, outcome],
                );
                let object_tags = HashMap::from([(
                    TagKey::from(crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG),
                    graveyard_snapshots,
                )]);
                let mut body_outputs =
                    crate::effects::CompletedEffectOutputs::from_children([observed], |_| outcome);
                body_outputs.retain_published_references(entry_outputs);
                let body_outputs = super::complete_keyword_action_with_outputs(
                    game,
                    ctx,
                    body_outputs,
                    KeywordActionEvent::new(
                        KeywordActionKind::ManifestDread,
                        player,
                        ctx.source,
                        1,
                    )
                    .with_object_tags(object_tags),
                )?;

                let outputs = crate::effects::zones::finish_zone_change_receipts_with_outputs(
                    game,
                    ctx,
                    body_outputs,
                    receipts,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                Ok(outputs)
            },
        )
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let library_owner =
                    crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
                let Some(&card_id) = game
                    .player(library_owner)
                    .and_then(|player| player.library.last())
                else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };

                manifest_card_with_outputs(
                    game,
                    ctx,
                    card_id,
                    ctx.controller,
                    self.cloak,
                    if self.cloak {
                        KeywordActionKind::Cloak
                    } else {
                        KeywordActionKind::Manifest
                    },
                )
            },
        )
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let hand = game
                    .player(ctx.controller)
                    .map(|player| player.hand.to_vec())
                    .unwrap_or_default();
                if hand.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
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
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let Some(card_id) = chosen.into_iter().find(|id| {
                    game.object(*id).is_some_and(|object| {
                        object.zone == Zone::Hand && object.owner == ctx.controller
                    })
                }) else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };

                manifest_card_with_outputs(
                    game,
                    ctx,
                    card_id,
                    ctx.controller,
                    false,
                    KeywordActionKind::Manifest,
                )
            },
        )
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::tokens::execute_token_instruction_with_pending_value(
            game,
            ctx,
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| {
                let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
                if count == 0 {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
                }

                game.reserve_token_repetition_work(count)?;
                let mut created_ids = Vec::new();
                let mut outcomes = Vec::new();

                for _ in 0..count {
                    let candidates = game
                        .battlefield
                        .iter()
                        .copied()
                        // CR 702.26b: a phased-out token can't be populated.
                        .filter(|&id| !game.is_phased_out(id))
                        .filter(|&id| {
                            game.object(id).is_some_and(|obj| {
                                game.controller_of(obj) == ctx.controller
                                    && obj.kind == ObjectKind::Token
                                    && game
                                        .object_has_card_type(id, crate::types::CardType::Creature)
                            })
                        })
                        .collect::<Vec<_>>();

                    if candidates.is_empty() {
                        outcomes.push(super::publish_keyword_action_completion_receipt(
                            game,
                            ctx,
                            crate::triggers::TriggerEvent::new_with_provenance(
                                KeywordActionEvent::new(
                                    KeywordActionKind::Populate,
                                    ctx.controller,
                                    ctx.source,
                                    1,
                                ),
                                ctx.provenance,
                            ),
                        )?);
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
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::with_objects(Vec::new()),
                            ));
                        }
                        let normalized = normalize_object_selection(selection, &candidates, 1);
                        normalized.first().copied().unwrap_or(candidates[0])
                    };

                    let outcome = crate::effects::CreateTokenCopyEffect::one(
                        ChooseSpec::SpecificObject(chosen),
                    )
                    .enters_tapped(self.enters_tapped)
                    .attacking(self.enters_attacking)
                    .haste(self.has_haste)
                    .sacrifice_at_next_end_step(self.sacrifice_at_next_end_step)
                    .exile_at_next_end_step(self.exile_at_next_end_step)
                    .next_end_step_player(self.next_end_step_player.clone())
                    .exile_at_eoc(self.exile_at_end_of_combat)
                    .execute_child_with_outputs(game, ctx)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::resolved(),
                        ));
                    }
                    created_ids.extend(outcome.outcome.instruction_result().output_objects());
                    outcomes.push(outcome);
                    outcomes.push(super::publish_keyword_action_completion_receipt(
                        game,
                        ctx,
                        crate::triggers::TriggerEvent::new_with_provenance(
                            KeywordActionEvent::new(
                                KeywordActionKind::Populate,
                                ctx.controller,
                                ctx.source,
                                1,
                            ),
                            ctx.provenance,
                        ),
                    )?);
                }

                Ok(crate::effects::CompletedEffectOutputs::from_children(
                    outcomes,
                    |children| {
                        EffectOutcome::aggregate_with_primary_result(
                            EffectOutcome::with_objects(created_ids),
                            children,
                        )
                    },
                ))
            },
        )
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| {
                game.establish_control_transition_boundary()
                    .map_err(ExecutionError::ContinuousDiscovery)?;
                let amount = match &self.amount_value {
                    Some(value) => {
                        crate::effects::helpers::resolve_nonnegative_u32(game, value, ctx)?
                    }
                    None => self.amount,
                };
                let ids = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|id| !game.is_phased_out(*id))
                    .collect::<Vec<_>>();
                let frame = game
                    .try_current_characteristics_batch(&ids)
                    .map_err(ExecutionError::ContinuousDiscovery)?;
                let mut candidates = Vec::new();
                for id in ids {
                    let chars = frame.get(&id).ok_or_else(|| {
                        ExecutionError::IncompleteEvidence(format!(
                            "bolster has no current characteristics for battlefield object {id:?}"
                        ))
                    })?;
                    if chars.controller == ctx.controller
                        && chars.card_types.contains(&crate::types::CardType::Creature)
                    {
                        let toughness = chars.toughness.ok_or_else(|| {
                            ExecutionError::UnresolvableValue(
                                "bolster creature has no calculated toughness".into(),
                            )
                        })?;
                        candidates.push((id, toughness));
                    }
                }
                let Some(least) = candidates.iter().map(|(_, toughness)| *toughness).min() else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                let candidates = candidates
                    .into_iter()
                    .filter_map(|(id, toughness)| (toughness == least).then_some(id))
                    .collect::<Vec<_>>();
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
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    if selection.len() != 1 || !candidates.contains(&selection[0]) {
                        return Err(ExecutionError::InvalidTarget);
                    }
                    selection[0]
                };

                let outcome = crate::effects::PutCountersEffect::new(
                    CounterType::PlusOnePlusOne,
                    amount,
                    ChooseSpec::SpecificObject(chosen),
                )
                .execute_child_with_outputs(game, ctx)?;

                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let primary = outcome.outcome.summary_projection();
                let original =
                    crate::effects::CompletedEffectOutputs::from_children([outcome], |children| {
                        EffectOutcome::aggregate_with_primary_result(primary, children)
                    });
                super::complete_keyword_action_with_outputs(
                    game,
                    ctx,
                    original,
                    KeywordActionEvent::new(
                        KeywordActionKind::Bolster,
                        ctx.controller,
                        ctx.source,
                        1,
                    ),
                )
            },
        )
    }
}

impl EffectExecutor for CipherEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(*self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let Some(source_obj) = game.object(ctx.source).cloned() else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                };
                if source_obj.zone != Zone::Stack || source_obj.card.is_none() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
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
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
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
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                if !encode {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::declined(),
                    ));
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
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let normalized = normalize_object_selection(selection, &candidates, 1);
                let Some(chosen_creature) = normalized.first().copied() else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::declined(),
                    ));
                };

                let original_source = ctx.source;
                let additional = ctx.additional_replacement_effects_snapshot();
                let committed = apply_zone_change_with_context_and_additional_effects_with_outputs(
                    game,
                    original_source,
                    source_obj.zone,
                    Zone::Exile,
                    ctx.cause.clone(),
                    ctx,
                    &additional,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let receipt = committed.receipt;
                let mut original =
                    (|| -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
                        let exiled_id = match &receipt.original {
                            EventOutcome::Proceed(change) => {
                                if change.final_zone != Zone::Exile {
                                    return Ok(
                                        crate::effects::CompletedEffectOutputs::aggregate_only(
                                            EffectOutcome::with_objects(
                                                change.new_object_ids.clone(),
                                            ),
                                        ),
                                    );
                                }
                                let Some(new_id) = change.new_object_id else {
                                    return Ok(
                                        crate::effects::CompletedEffectOutputs::aggregate_only(
                                            EffectOutcome::resolved(),
                                        ),
                                    );
                                };
                                new_id
                            }
                            EventOutcome::Prevented => {
                                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                    EffectOutcome::prevented(),
                                ));
                            }
                            EventOutcome::Replaced => {
                                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                    EffectOutcome::replaced(),
                                ));
                            }
                            EventOutcome::NotApplicable => {
                                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                    EffectOutcome::target_invalid(),
                                ));
                            }
                        };

                        game.imprint_card(chosen_creature, exiled_id);
                        let ability = crate::ability::Ability::triggered(
                            crate::triggers::Trigger::this_deals_combat_damage_to_player(
                                crate::target::PlayerFilter::Any,
                            ),
                            vec![crate::effect::Effect::cast_encoded_card_copy(exiled_id)],
                        );
                        // CR 702.99a: the encoded card's static ability grants the trigger to
                        // the creature (layer 6). It's a continuous effect, not part of the
                        // creature's copiable values (CR 707.2), and it outlasts earlier
                        // ability-removing effects by timestamp.
                        let grant = crate::effects::ApplyContinuousEffect::new(
                            crate::continuous::EffectTarget::Specific(chosen_creature),
                            crate::continuous::Modification::AddAbilityGeneric(ability),
                            crate::effect::Until::ForAsLongAs(
                                ironsmith_core::ContinuousDurationPredicate::all([
                                    ironsmith_core::ContinuousDurationPredicate::ObjectInZone {
                                        object: ironsmith_core::ContinuousDurationObject::Specific(
                                            exiled_id,
                                        ),
                                        zone: Zone::Exile,
                                    },
                                    ironsmith_core::ContinuousDurationPredicate::ObjectInZone {
                                        object: ironsmith_core::ContinuousDurationObject::Specific(
                                            chosen_creature,
                                        ),
                                        zone: Zone::Battlefield,
                                    },
                                ]),
                            ),
                        )
                        .with_source_type(
                            crate::continuous::EffectSourceType::Resolution {
                                locked_targets: vec![chosen_creature],
                            },
                        );
                        let grant_outcome = grant.execute_child_with_outputs(game, ctx)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        let original =
                            EffectOutcome::with_objects(vec![exiled_id, chosen_creature])
                                .with_execution_fact(ExecutionFact::ChosenObjects(vec![
                                    chosen_creature,
                                ]))
                                .with_execution_fact(ExecutionFact::AffectedObjects(vec![
                                    exiled_id,
                                ]));
                        Ok(crate::effects::CompletedEffectOutputs::from_children(
                            [grant_outcome],
                            |children| {
                                EffectOutcome::aggregate_with_primary_result(original, children)
                            },
                        ))
                    })()?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                original.retain_published_references(committed.published_outputs);
                crate::effects::zones::finish_zone_change_receipts_with_outputs(
                    game,
                    ctx,
                    original,
                    vec![(original_source, receipt)],
                )
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastEncodedCardCopyEffect {
    pub encoded_card: ObjectId,
}

impl CastEncodedCardCopyEffect {
    pub fn new(encoded_card: ObjectId) -> Self {
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let encoded_id = self.encoded_card;
                let Some(encoded_obj) = game.object(encoded_id).cloned() else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                };
                if encoded_obj.zone != Zone::Exile {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
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
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                if !cast_copy {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::declined(),
                    ));
                }

                let snapshot = ObjectSnapshot::from_object(&encoded_obj, game);
                let prior = ctx.clear_object_tag("cipher_encoded");
                ctx.set_tagged_objects("cipher_encoded", vec![snapshot]);
                let result =
                    CastTaggedEffect::new("cipher_encoded", crate::target::PlayerFilter::You)
                        .as_copy()
                        .without_paying_mana_cost()
                        .execute_child_with_outputs(game, ctx);
                if let Some(previous) = prior {
                    ctx.set_tagged_objects("cipher_encoded", previous);
                } else {
                    ctx.clear_object_tag("cipher_encoded");
                }
                result
            },
        )
    }
}

/// Whether an "as this enters" keyword program (devour, amplify) can act:
/// either the source is being prepared to enter the battlefield, or it is
/// already there (a legacy trigger-shaped program).
fn source_is_entering_or_on_battlefield(game: &GameState, ctx: &ExecutionContext) -> bool {
    ctx.replacement.entry_counter_source == Some(ctx.source)
        || game
            .object(ctx.source)
            .is_some_and(|obj| obj.zone == Zone::Battlefield)
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                // Devour applies as the permanent enters (CR 702.82a); the entry
                // program runs against the object before it reaches the battlefield.
                if !source_is_entering_or_on_battlefield(game, ctx) {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
                }

                // CR 702.82c: "Devour [quality]" devours only permanents with
                // that quality; plain devour devours creatures.
                let quality_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
                let candidates = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&id| id != ctx.source && !game.is_phased_out(id))
                    .filter(|&id| {
                        game.object(id).is_some_and(|obj| {
                            game.controller_of(obj) == ctx.controller
                                && match &self.quality {
                                    None => game
                                        .object_has_card_type(id, crate::types::CardType::Creature),
                                    Some(quality) => quality.matches(obj, &quality_ctx, game),
                                }
                                && game.can_be_sacrificed(id)
                        })
                    })
                    .collect::<Vec<_>>();

                let chosen = if candidates.is_empty() {
                    Vec::new()
                } else {
                    let spec = ChooseObjectsSpec::new(
                        ctx.source,
                        if self.quality.is_some() {
                            "Choose any number of other permanents you control to sacrifice for devour"
                        } else {
                            "Choose any number of other creatures you control to sacrifice for devour"
                        },
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
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
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

                crate::effects::zones::sacrifice_selected_objects_with_original_outputs(
                    game,
                    ctx,
                    &[],
                    &[],
                    chosen,
                    |game, ctx, sacrifices| {
                        let devoured = sacrifices
                            .outcome
                            .affected_object_memory()
                            .unwrap_or_default()
                            .to_vec();
                        let count = sacrifices.outcome.count_or_zero();
                        game.set_devoured_objects(ctx.source, devoured);
                        game.set_devoured_count(ctx.source, count.max(0) as u32);
                        let devoured_count = u32::try_from(count).map_err(|_| {
                            ExecutionError::InternalError(
                                "devour count exceeds the counter range".into(),
                            )
                        })?;
                        let counters = crate::effects::PutCountersEffect::new(
                            CounterType::PlusOnePlusOne,
                            self.counters_for(devoured_count),
                            ChooseSpec::Source,
                        )
                        .execute_child_with_outputs(game, ctx)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        let primary = counters.outcome.summary_projection();
                        Ok(crate::effects::CompletedEffectOutputs::from_children(
                            [sacrifices, counters],
                            |children| {
                                EffectOutcome::aggregate_with_primary_result(primary, children)
                            },
                        ))
                    },
                )
            },
        )
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                // Amplify applies as the permanent enters (CR 702.38a).
                if !source_is_entering_or_on_battlefield(game, ctx) {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
                }

                let source_creature_types = game
                    .calculated_subtypes(ctx.source)
                    .into_iter()
                    .filter(|subtype| subtype.is_creature_type())
                    .collect::<Vec<_>>();
                if source_creature_types.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let candidates = game
                    .player(ctx.controller)
                    .map(|player| player.hand.to_vec())
                    .unwrap_or_default()
                    .into_iter()
                    // The entering card itself isn't one of the cards in your hand you
                    // reveal (CR 702.38a), even while the entry program runs.
                    // The same exclusion applies to every card reserved by this entry batch.
                    .filter(|&id| {
                        id != ctx.source && !ctx.replacement.entry_reserved_objects.contains(&id)
                    })
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
                // Only the owner knows which hidden hand cards share a creature type:
                // peers offer their placeholders too, every peer asks, and the chosen
                // cards are opened before the answer replays (see
                // `game_state::hidden_hand_choices`). Opened cards that don't qualify
                // drop out of `candidates` on every peer alike.
                let creature_filter = crate::filter::ObjectFilter::default()
                    .in_zone(Zone::Hand)
                    .owned_by(crate::target::PlayerFilter::Specific(ctx.controller))
                    .with_type(crate::types::CardType::Creature);
                let hand: Vec<ObjectId> = game
                    .player(ctx.controller)
                    .map(|player| {
                        player
                            .hand
                            .iter()
                            .copied()
                            .filter(|&id| {
                                id != ctx.source
                                    && !ctx.replacement.entry_reserved_objects.contains(&id)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let hidden_hand_choice = game
                    .hand_choice_depends_on_hidden_identity(&creature_filter, hand.iter().copied());
                let mut offered = candidates.clone();
                if hidden_hand_choice {
                    let filter_ctx = ctx.filter_context(game);
                    for id in
                        game.hidden_hand_placeholder_candidates(&creature_filter, &filter_ctx, hand)
                    {
                        if !offered.contains(&id) {
                            offered.push(id);
                        }
                    }
                }

                let chosen = if offered.is_empty() {
                    Vec::new()
                } else {
                    let mut spec = ChooseObjectsSpec::new(
                        ctx.source,
                        "Choose any number of cards from your hand that share a creature type with this creature to reveal for amplify",
                        offered.clone(),
                        0,
                        Some(offered.len()),
                    );
                    if hidden_hand_choice {
                        spec = spec.require_explicit_choice().with_selection_reveal_policy(
                            crate::decisions::context::SelectionRevealPolicy::Public,
                        );
                    }
                    let selection: Vec<ObjectId> = make_decision(
                        game,
                        ctx.decision_maker,
                        ctx.controller,
                        Some(ctx.source),
                        spec,
                    );
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    if hidden_hand_choice {
                        let opened: Vec<ObjectId> = selection
                            .iter()
                            .copied()
                            .filter(|id| offered.contains(id))
                            .collect();
                        let filter_ctx = ctx.filter_context(game);
                        game.record_hidden_identity_obligations(
                            &opened,
                            &creature_filter,
                            &filter_ctx,
                            "reveal creature cards for amplify",
                        );
                        game.mark_hidden_cards_publicly_revealed(&opened);
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
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let revealed = chosen
                    .iter()
                    .filter_map(|id| ObjectSnapshot::from_object_id(game, *id))
                    .collect();
                let actor = ctx.controller;
                let reveal = crate::effects::cards::reveal_objects_with_outputs(
                    game,
                    ctx,
                    revealed,
                    Some(actor),
                    "Reveal cards from hand for amplify",
                    None,
                )?;

                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let revealed_count = u32::try_from(
                    reveal
                        .outcome
                        .instruction_result()
                        .action_objects(crate::effect::PriorEffectAction::Revealed, Some(actor))
                        .len(),
                )
                .map_err(|_| {
                    ExecutionError::InternalError("amplify count exceeds the counter range".into())
                })?;
                let counters = crate::effects::PutCountersEffect::new(
                    CounterType::PlusOnePlusOne,
                    revealed_count.saturating_mul(self.amount),
                    ChooseSpec::Source,
                )
                .execute_child_with_outputs(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let primary = counters.outcome.summary_projection();
                Ok(crate::effects::CompletedEffectOutputs::from_children(
                    [reveal, counters],
                    |children| EffectOutcome::aggregate_with_primary_result(primary, children),
                ))
            },
        )
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let outcome = crate::effects::PutCountersEffect::new(
                    CounterType::PlusOnePlusOne,
                    1,
                    self.target.clone(),
                )
                .with_target_count(ChoiceCount::up_to(self.amount as usize))
                .execute_child_with_outputs(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let primary = outcome.outcome.summary_projection();
                let original =
                    crate::effects::CompletedEffectOutputs::from_children([outcome], |children| {
                        EffectOutcome::aggregate_with_primary_result(primary, children)
                    });
                super::complete_keyword_action_with_outputs(
                    game,
                    ctx,
                    original,
                    KeywordActionEvent::new(
                        KeywordActionKind::Support,
                        ctx.controller,
                        ctx.source,
                        self.amount,
                    ),
                )
            },
        )
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let source_id = ctx.source;
                if !game
                    .object(source_id)
                    .is_some_and(|object| object.zone == Zone::Battlefield)
                    || game.is_phased_out(source_id)
                {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                }
                // "The next time target creature adapts this turn, it adapts as though
                // it had no +1/+1 counters on it" is consumed by this adapt.
                let turn = game.turn.turn_number;
                let store = &mut game.turn_store.adapt_ignores_counters;
                let ignores_counters = store.iter().any(|(id, t)| *id == source_id && *t == turn);
                // Every already-active "next time" permission refers to this same
                // adaptation; multiple resolutions do not bank later adaptations.
                store.retain(|(id, t)| *t == turn && *id != source_id);
                if !ignores_counters
                    && game.counter_count(source_id, CounterType::PlusOnePlusOne) > 0
                {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let counters = crate::effects::PutCountersEffect::on_source(
                    CounterType::PlusOnePlusOne,
                    self.amount,
                )
                .execute_child_with_outputs(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
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

                Ok(counters)
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CounterAbilityEffect;

impl Default for CounterAbilityEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl CounterAbilityEffect {
    pub fn new() -> Self {
        Self
    }
}

/// The compatibility effect is a capability restriction on the common stack
/// counter owner. Its fixed child is immutable data, never an execution cache.
fn ability_counter_primitive() -> &'static Effect {
    static OWNER: std::sync::OnceLock<Effect> = std::sync::OnceLock::new();
    OWNER.get_or_init(|| {
        Effect::new(crate::effects::CounterEffect::new(ChooseSpec::target(
            ChooseSpec::Object(crate::target::ObjectFilter::ability()),
        )))
    })
}

impl EffectExecutor for CounterAbilityEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn transparent_child_effect(&self) -> Option<&Effect> {
        Some(ability_counter_primitive())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(ability_counter_primitive());
    }

    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        ability_counter_primitive().0.result_action()
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::execute_effect_with_outputs(game, ability_counter_primitive(), ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        ability_counter_primitive().0.get_target_spec()
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        ability_counter_primitive().0.get_target_count()
    }

    fn target_description(&self) -> &'static str {
        "ability to counter"
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
    fn manifest_dread_chooses_one_top_card_and_puts_the_other_in_the_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let second_from_top = create_library_card(
            &mut game,
            alice,
            198,
            "Chosen Dread Card",
            vec![CardType::Creature],
            None,
            Some(3),
            Some(3),
        );
        let top = create_library_card(
            &mut game,
            alice,
            199,
            "Unchosen Dread Card",
            vec![CardType::Instant],
            None,
            None,
            None,
        );
        let chosen_stable_id = game
            .object(second_from_top)
            .expect("chosen library card")
            .stable_id;
        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![second_from_top]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = ManifestDreadEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("manifest dread should execute");

        let manifested_id = outcome
            .value
            .objects()
            .and_then(|ids| ids.first().copied())
            .expect("manifest dread should return the manifested permanent");
        let manifested = game
            .object(manifested_id)
            .expect("manifested permanent should exist");
        assert_eq!(manifested.stable_id, chosen_stable_id);
        assert!(game.is_face_down(manifested_id));
        assert!(game.is_manifested(manifested_id));
        assert_eq!(game.calculated_power(manifested_id), Some(2));
        assert_eq!(game.calculated_toughness(manifested_id), Some(2));
        assert!(game.player(alice).is_some_and(|player| {
            player.graveyard.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|card| card.name == "Unchosen Dread Card")
            })
        }));
        assert!(
            game.object(top).is_none(),
            "zone change should mint a new object"
        );
        let action_event = outcome
            .events
            .iter()
            .find(|event| {
                event
                    .downcast::<KeywordActionEvent>()
                    .is_some_and(|action| action.action == KeywordActionKind::ManifestDread)
            })
            .cloned()
            .expect("manifest dread should emit its distinct keyword action");
        let action = action_event
            .downcast::<KeywordActionEvent>()
            .expect("manifest dread action event");
        let graveyard_snapshots = action
            .object_tags
            .get(&TagKey::from(crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG))
            .expect("manifest dread should tag the card put into the graveyard");
        assert_eq!(graveyard_snapshots.len(), 1);
        assert_eq!(graveyard_snapshots[0].name, "Unchosen Dread Card");
        assert_eq!(graveyard_snapshots[0].zone, Zone::Graveyard);

        let graveyard_object_id = graveyard_snapshots[0].object_id;
        let graveyard_stable_id = graveyard_snapshots[0].stable_id;
        drop(ctx);
        let tag = TagKey::from(crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG);
        let mut resolution_ctx =
            ExecutionContext::new_default(source, alice).with_triggering_event(action_event);
        crate::effects::TagTriggeringObjectEffect::new(tag.clone())
            .execute(&mut game, &mut resolution_ctx)
            .expect("observer prelude should recover the post-zone-change graveyard object");
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(tag), Zone::Hand, false)
            .execute(&mut game, &mut resolution_ctx)
            .expect("observer should move the tagged graveyard card to hand");

        assert!(game.object(graveyard_object_id).is_none());
        assert!(game.player(alice).is_some_and(|player| {
            player.hand.iter().any(|id| {
                game.object(*id).is_some_and(|card| {
                    card.name == "Unchosen Dread Card" && card.stable_id == graveyard_stable_id
                })
            })
        }));
    }

    #[test]
    fn manifest_dread_observer_excludes_a_card_replaced_out_of_the_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card_to_manifest = create_library_card(
            &mut game,
            alice,
            189,
            "Chosen Replacement Dread Card",
            vec![CardType::Creature],
            None,
            Some(2),
            Some(2),
        );
        create_library_card(
            &mut game,
            alice,
            190,
            "Exiled Replacement Dread Card",
            vec![CardType::Instant],
            None,
            None,
            None,
        );
        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![card_to_manifest]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        crate::effects::ExileInsteadOfGraveyardEffect::you()
            .execute(&mut game, &mut ctx)
            .expect("graveyard replacement should register");

        let outcome = ManifestDreadEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("manifest dread should honor the replacement");
        let action_event = outcome
            .events
            .iter()
            .find(|event| {
                event
                    .downcast::<KeywordActionEvent>()
                    .is_some_and(|action| action.action == KeywordActionKind::ManifestDread)
            })
            .cloned()
            .expect("manifest dread action event");
        let action = action_event
            .downcast::<KeywordActionEvent>()
            .expect("manifest dread action payload");
        assert!(
            action
                .object_tags
                .get(&TagKey::from(crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG))
                .is_some_and(Vec::is_empty),
            "a card replaced into exile must not be tagged as put into the graveyard"
        );
        assert!(game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|card| card.name == "Exiled Replacement Dread Card")
        }));

        drop(ctx);
        let tag = TagKey::from(crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG);
        let mut resolution_ctx =
            ExecutionContext::new_default(source, alice).with_triggering_event(action_event);
        let tagged = crate::effects::TagTriggeringObjectEffect::new(tag.clone())
            .execute(&mut game, &mut resolution_ctx)
            .expect("observer prelude should handle an empty event tag");
        assert_eq!(tagged.value.as_count(), Some(0));
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(tag), Zone::Hand, false)
            .execute(&mut game, &mut resolution_ctx)
            .expect("empty observer selection should resolve without moving a card");

        assert!(
            game.player(alice)
                .is_some_and(|player| player.hand.is_empty())
        );
        assert!(game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|card| card.name == "Exiled Replacement Dread Card")
        }));
    }

    #[test]
    fn manifest_dread_with_one_library_card_manifests_it_without_a_choice() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let only_card = create_library_card(
            &mut game,
            alice,
            197,
            "Only Dread Card",
            vec![CardType::Creature],
            None,
            Some(1),
            Some(1),
        );
        let only_card_stable_id = game.object(only_card).expect("only library card").stable_id;
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ManifestDreadEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("one-card manifest dread should execute");

        let manifested_id = outcome
            .value
            .objects()
            .and_then(|ids| ids.first().copied())
            .expect("the only card should be manifested");
        assert_eq!(
            game.object(manifested_id).map(|card| card.stable_id),
            Some(only_card_stable_id)
        );
        assert!(
            game.player(alice)
                .is_some_and(|player| player.library.is_empty())
        );
        assert!(
            game.player(alice)
                .is_some_and(|player| player.graveyard.is_empty())
        );
    }

    #[test]
    fn manifest_dread_waiting_for_a_private_choice_does_not_move_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        create_library_card(
            &mut game,
            alice,
            195,
            "Dread Choice One",
            vec![CardType::Creature],
            None,
            None,
            None,
        );
        create_library_card(
            &mut game,
            alice,
            196,
            "Dread Choice Two",
            vec![CardType::Instant],
            None,
            None,
            None,
        );
        let original_library = game.player(alice).expect("alice").library.to_vec();
        let mut dm = PromptingDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        ManifestDreadEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("prompt-only manifest dread should suspend cleanly");

        assert_eq!(game.player(alice).expect("alice").library, original_library);
        assert!(game.player(alice).expect("alice").graveyard.is_empty());
        assert!(game.battlefield.is_empty());
    }

    #[test]
    fn manifest_dread_with_an_empty_library_still_emits_the_completed_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ManifestDreadEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("empty-library manifest dread should execute");

        assert_eq!(outcome.value.as_count(), Some(0));
        assert!(game.battlefield.is_empty());
        assert!(game.player(alice).expect("alice").graveyard.is_empty());
        let action_events = outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<KeywordActionEvent>())
            .filter(|event| event.action == KeywordActionKind::ManifestDread)
            .collect::<Vec<_>>();
        assert_eq!(action_events.len(), 1, "CR 701.62b requires one event");
        assert_eq!(
            action_events[0]
                .object_tags
                .get(&TagKey::from(crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG))
                .map(Vec::len),
            Some(0),
            "the completed action should expose an empty this-way graveyard set"
        );
    }

    #[test]
    fn repeated_manifest_dread_emits_once_per_instruction_after_the_library_is_empty() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = crate::effects::RepeatEffectsEffect::new(
            crate::effect::Value::Fixed(2),
            vec![Effect::manifest_dread()],
        )
        .execute(&mut game, &mut ctx)
        .expect("repeated empty-library manifest dread should execute");

        assert_eq!(
            outcome
                .events
                .iter()
                .filter_map(|event| event.downcast::<KeywordActionEvent>())
                .filter(|event| event.action == KeywordActionKind::ManifestDread)
                .count(),
            2,
            "each impossible manifest-dread instruction completes independently"
        );
    }

    #[test]
    fn manifest_dread_twice_uses_the_next_two_cards_for_the_second_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_library_card(
            &mut game,
            alice,
            191,
            "First Dread Card",
            vec![CardType::Creature],
            None,
            None,
            None,
        );
        let second = create_library_card(
            &mut game,
            alice,
            192,
            "Second Dread Card",
            vec![CardType::Instant],
            None,
            None,
            None,
        );
        let third = create_library_card(
            &mut game,
            alice,
            193,
            "Third Dread Card",
            vec![CardType::Creature],
            None,
            None,
            None,
        );
        let fourth = create_library_card(
            &mut game,
            alice,
            194,
            "Fourth Dread Card",
            vec![CardType::Instant],
            None,
            None,
            None,
        );
        let mut dm = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![fourth], vec![second]]),
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        ManifestDreadEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("first manifest dread should execute");
        ManifestDreadEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("second manifest dread should execute");

        assert!(game.player(alice).expect("alice").library.is_empty());
        assert_eq!(game.battlefield.len(), 2);
        assert_eq!(game.player(alice).expect("alice").graveyard.len(), 2);
        assert!(game.object(first).is_none());
        assert!(game.object(second).is_none());
        assert!(game.object(third).is_none());
        assert!(game.object(fourth).is_none());
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
    fn cloak_top_card_enters_face_down_with_ward_two_and_emits_cloak() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let _card = create_library_card(
            &mut game,
            alice,
            202,
            "Cloak Test Creature",
            vec![CardType::Creature],
            None,
            Some(3),
            Some(3),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ManifestTopCardOfLibraryEffect::cloak(PlayerFilter::You)
            .execute(&mut game, &mut ctx)
            .expect("cloak should execute");
        let cloaked_id = outcome
            .value
            .objects()
            .and_then(|ids| ids.first().copied())
            .expect("cloak should create one permanent");
        let cloaked = game
            .object(cloaked_id)
            .expect("cloaked permanent should exist");

        assert!(game.is_face_down(cloaked_id));
        assert!(game.is_cloaked(cloaked_id));
        assert!(!game.is_manifested(cloaked_id));
        assert_eq!(game.calculated_power(cloaked_id), Some(2));
        assert_eq!(game.calculated_toughness(cloaked_id), Some(2));
        assert!(cloaked.abilities.iter().any(|ability| matches!(
            &ability.kind,
            crate::ability::AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::Ward
                    && static_ability.display() == "Ward {2}"
        )));
        let keyword = outcome
            .events
            .iter()
            .find(|event| event.kind() == EventKind::KeywordAction)
            .expect("expected keyword action event")
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(keyword.action, KeywordActionKind::Cloak);
    }

    #[test]
    fn cloak_objects_moves_the_collection_simultaneously_tapped_and_emits_one_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature = create_library_card(
            &mut game,
            alice,
            203,
            "Pile Creature",
            vec![CardType::Creature],
            None,
            Some(4),
            Some(4),
        );
        let instant = create_library_card(
            &mut game,
            alice,
            204,
            "Pile Instant",
            vec![CardType::Instant],
            None,
            None,
            None,
        );
        let creature = game
            .move_object_by_effect(creature, Zone::Exile)
            .expect("creature should move to exile");
        let instant = game
            .move_object_by_effect(instant, Zone::Exile)
            .expect("instant should move to exile");
        game.set_face_down(creature);
        game.set_face_down(instant);
        let pile = [creature, instant]
            .into_iter()
            .map(|id| ObjectSnapshot::from_object(game.object(id).expect("pile card"), &game))
            .collect();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects("pile", pile);
        let random_before = game.irreversible_random_count();

        let outcome =
            ManifestObjectsEffect::new(ChooseSpec::Tagged(TagKey::from("pile")), PlayerFilter::You)
                .cloak()
                .tapped()
                .shuffled()
                .execute(&mut game, &mut ctx)
                .expect("cloaking the collection should execute");
        let cloaked = outcome
            .value
            .objects()
            .expect("cloaking should return the moved permanents");

        assert_eq!(cloaked.len(), 2);
        assert_eq!(game.irreversible_random_count(), random_before + 1);
        for object_id in cloaked {
            let object = game
                .object(*object_id)
                .expect("cloaked permanent should exist");
            assert_eq!(object.zone, Zone::Battlefield);
            assert_eq!(game.controller_of(object), alice);
            assert!(game.is_tapped(*object_id));
            assert!(game.is_face_down(*object_id));
            assert!(game.is_cloaked(*object_id));
            assert!(!game.is_manifested(*object_id));
            assert_eq!(game.calculated_power(*object_id), Some(2));
            assert_eq!(game.calculated_toughness(*object_id), Some(2));
            assert!(object.abilities.iter().any(|ability| matches!(
                &ability.kind,
                crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::Ward
                        && static_ability.display() == "Ward {2}"
            )));
        }
        let keyword_actions = outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<KeywordActionEvent>())
            .collect::<Vec<_>>();
        assert_eq!(keyword_actions.len(), 1);
        assert_eq!(keyword_actions[0].action, KeywordActionKind::Cloak);
        assert_eq!(keyword_actions[0].player, alice);
        assert_eq!(keyword_actions[0].amount, 2);
    }

    #[test]
    fn manifest_objects_does_not_grant_cloak_ward() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card = create_library_card(
            &mut game,
            alice,
            205,
            "Manifest Collection Card",
            vec![CardType::Creature],
            None,
            Some(3),
            Some(3),
        );
        game.object_mut(card)
            .expect("card should exist in the library")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::disguise(
                crate::cost::TotalCost::mana(crate::mana::ManaCost::from_pips(vec![vec![
                    crate::mana::ManaSymbol::Generic(3),
                ]])),
            )));
        let card = game
            .move_object_by_effect(card, Zone::Exile)
            .expect("card should move to exile");
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome =
            ManifestObjectsEffect::new(ChooseSpec::SpecificObject(card), PlayerFilter::You)
                .execute(&mut game, &mut ctx)
                .expect("manifesting the card should execute");
        let manifested = outcome
            .value
            .objects()
            .and_then(|ids| ids.first().copied())
            .expect("manifest should return the moved permanent");
        let object = game.object(manifested).expect("manifested permanent");

        assert!(game.is_face_down(manifested));
        assert!(game.is_manifested(manifested));
        assert!(!object.abilities.iter().any(|ability| matches!(
            &ability.kind,
            crate::ability::AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::Ward
        )));
        let keyword = outcome
            .events
            .iter()
            .find_map(|event| event.downcast::<KeywordActionEvent>())
            .expect("manifest should emit one keyword action");
        assert_eq!(keyword.action, KeywordActionKind::Manifest);
        assert_eq!(keyword.amount, 1);
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
            .map(|obj| obj.name.to_string())
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
        game.turn.phase = crate::game_state::Phase::Combat;

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
    fn dynamic_bolster_uses_one_quantity_and_rejects_malformed_tie_answers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_creature(&mut game, alice, 1, "First", 1, 1);
        let second = create_creature(&mut game, alice, 2, "Second", 1, 1);
        let _largest = create_creature(&mut game, alice, 3, "Largest", 4, 4);
        let effect = BolsterEffect::with_value(crate::effect::Value::Count(
            crate::target::ObjectFilter::creature().you_control()));
        let mut malformed = SelectIdsDecisionMaker {
            choices: VecDeque::from([vec![first, second]]),
        };
        let result = effect.execute(&mut game,
            &mut ExecutionContext::new_default(source, alice).with_decision_maker(&mut malformed));
        assert!(matches!(result, Err(ExecutionError::InvalidTarget)));
        for id in [first, second] {
            assert_eq!(game.counter_count(id, CounterType::PlusOnePlusOne), 0);
        }
        let mut dm = SelectIdsDecisionMaker { choices: VecDeque::from([vec![second]]) };
        effect.execute(&mut game,
            &mut ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm)).unwrap();
        assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 3);
        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 0);
    }

    #[test]
    fn dynamic_bolster_zero_pending_and_unsigned_overflow_preserve_the_attempt() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_creature(&mut game, alice, 1, "First", 1, 1);
        let second = create_creature(&mut game, alice, 2, "Second", 1, 1);
        let mut dm = PromptingDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let pending = BolsterEffect::with_value(crate::effect::Value::Fixed(3))
            .execute(&mut game, &mut ctx).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(pending.events.is_empty());
        let mut dm = SelectIdsDecisionMaker { choices: VecDeque::from([vec![second]]) };
        let zero = BolsterEffect::with_value(crate::effect::Value::Fixed(0)).execute(&mut game,
            &mut ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm)).unwrap();
        assert_eq!(zero.count_or_zero(), 0);
        let too_large = crate::effect::Value::Add(
            Box::new(crate::effect::Value::from(u32::MAX)),
            Box::new(crate::effect::Value::Fixed(1)));
        assert!(BolsterEffect::with_value(too_large)
            .execute(&mut game, &mut ExecutionContext::new_default(source, alice)).is_err());
        for id in [first, second] {
            assert_eq!(game.counter_count(id, CounterType::PlusOnePlusOne), 0);
        }
        // Preserve main's unsigned event domain instead of imposing an i32 cap.
        assert_eq!(crate::effects::helpers::resolve_nonnegative_u32(&game,
            &crate::effect::Value::from(u32::MAX),
            &ExecutionContext::new_default(source, alice)).unwrap(), u32::MAX);
    }

    #[test]
    fn dynamic_bolster_counter_additions_share_original_receipts_and_pending_rollback() {
        use crate::effect::{EffectId, Value};
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        struct Answers { pause: bool, pending: bool, target: ObjectId, calls: usize }
        impl DecisionMaker for Answers {
            fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
                self.calls += 1;
                assert_eq!(game.counter_count(self.target, CounterType::PlusOnePlusOne), 3);
                self.pending = self.pause;
                !self.pending
            }
            fn awaiting_choice(&self) -> bool { self.pending }
        }
        for mode in 0..3 {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let source = create_creature(&mut game, alice, 1, "Bolster recipient", 1, 1);
            let replacement_source = create_creature(&mut game, bob, 2, "Counter addition owner", 4, 4);
            let continuation = if mode == 1 { Effect::lose_life(Value::X) }
                else { Effect::may(vec![Effect::gain_life(4)]) };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(replacement_source, bob,
                    crate::events::counters::matchers::WouldPutCountersMatcher::new(
                        crate::target::ObjectFilter::specific(source), Some(CounterType::PlusOnePlusOne)),
                    ReplacementAction::Additionally(vec![Effect::gain_life(3), continuation])));
            game.take_pending_trigger_events();
            let next_id = game.next_object_id_counter();
            let mut dm = Answers { pause: mode == 2, pending: false, target: source, calls: 0 };
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            ctx.set_tagged_players("retained", vec![alice]);
            ctx.store_outcome(EffectId(57), EffectOutcome::count(9));
            let effect = Effect::with_id(57, Effect::bolster_value(Value::Fixed(3)));
            let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
            if mode == 0 {
                let outcome = result.unwrap();
                assert_eq!(outcome.as_count(), Some(3));
                assert_eq!(ctx.get_outcome(EffectId(57)).unwrap().as_count(), Some(3));
                assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 3);
                assert_eq!(game.player(bob).unwrap().life, 27);
                assert_eq!(outcome.events_of_type::<KeywordActionEvent>().count(), 1);
                assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            } else {
                if mode == 1 {
                    assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
                } else {
                    assert!(ctx.decision_maker.awaiting_choice());
                    assert!(result.unwrap().events.is_empty());
                }
                assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
                assert_eq!(game.player(bob).unwrap().life, 20);
                assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
                assert!(game.take_pending_trigger_events().is_empty());
                assert_eq!(game.next_object_id_counter(), next_id);
                assert_eq!(ctx.get_outcome(EffectId(57)).unwrap().as_count(), Some(9));
            }
            assert_eq!(ctx.source, source);
            assert_eq!(ctx.controller, alice);
            assert_eq!(ctx.get_tagged_players("retained"), Some(&vec![alice]));
            if mode == 2 {
                let checkpoint = crate::effects::ExecutionContextCheckpoint::capture(&ctx);
                drop(ctx);
                dm.pause = false;
                dm.pending = false;
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                checkpoint.restore(&mut ctx);
                let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
                assert_eq!(outcome.as_count(), Some(3));
                assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 3);
                assert_eq!(game.player(bob).unwrap().life, 27);
                assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
                assert!(!ctx.decision_maker.awaiting_choice());
            }
        }
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

#[cfg(test)]
mod replacement_manifest_owner_contract_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::Value;
    use crate::ids::CardId;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    struct Answers { first: ObjectId, second: ObjectId, stable: StableId, sequential: bool, pause: bool, pending: bool, calls: usize, binding: bool }
    impl DecisionMaker for Answers {
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.calls += 1;
            assert!(game.object(self.first).is_none());
            let arrival = game.objects_in_deterministic_order().into_iter().find(|object| object.stable_id == self.stable).unwrap();
            assert_eq!(arrival.zone, Zone::Battlefield); assert!(game.is_manifested(arrival.id)); assert!(game.is_face_down(arrival.id));
            if self.sequential { assert_eq!(game.object(self.second).unwrap().zone, Zone::Library); assert!(!game.is_face_down(self.second)); }
            else { assert!(game.object(self.second).is_none(), "simultaneous entries precede additions"); }
            if self.binding { assert_eq!(game.counter_count(arrival.id, CounterType::PlusOnePlusOne), 1); }
            self.pending = self.pause; !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn card(game: &mut GameState, owner: PlayerId, zone: Zone) -> ObjectId {
        game.create_object_from_card(&CardBuilder::new(CardId::new(), "Manifest owner fixture")
            .card_types(vec![if zone == Zone::Battlefield { CardType::Artifact } else { CardType::Instant }]).build(), owner, zone)
    }
    fn check(sequential: bool, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game(); let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let source = card(&mut game, alice, Zone::Battlefield); let replacement_source = card(&mut game, bob, Zone::Battlefield); let sentinel = card(&mut game, alice, Zone::Battlefield);
        let origin = if sequential { Zone::Library } else { Zone::Hand };
        let first = card(&mut game, alice, origin); let second = card(&mut game, alice, origin); let stable = game.object(first).unwrap().stable_id;
        let originals = vec![first, second]; let snapshots = originals.iter().map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game)).collect::<Vec<_>>();
        let sentinel_snapshot = ObjectSnapshot::from_object(game.object(sentinel).unwrap(), &game);
        let actions = match mode { 1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![Effect::new(crate::effects::PutCountersEffect::new(CounterType::PlusOnePlusOne, 1, ChooseSpec::tagged("it"))), Effect::may(vec![Effect::gain_life(0)])],
            _ => vec![Effect::gain_life(3), Effect::may(vec![Effect::gain_life(4)])] };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(replacement_source, bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(first), Some(origin), Some(Zone::Battlefield)), ReplacementAction::Additionally(actions)));
        let mut manifest = ManifestObjectsEffect::new(ChooseSpec::tagged("victims"), PlayerFilter::You);
        if sequential { manifest = manifest.cloak(); }
        let effect = Effect::new(manifest); game.take_pending_trigger_events(); let ids = game.next_object_id_counter(); let objects = game.objects_in_deterministic_order().len();
        let mut dm = Answers { first, second, stable, sequential, pause: mode == 2, pending: false, calls: 0, binding: mode == 3 };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm); ctx.set_tagged_objects("victims", snapshots.clone()); ctx.set_tagged_objects("it", vec![sentinel_snapshot.clone()]);
        let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        else if mode == 2 { assert!(ctx.decision_maker.awaiting_choice()); assert!(result.unwrap().events.is_empty()); }
        else {
            let outcome = result.unwrap(); assert_eq!(outcome.value.objects().unwrap().len(), 2);
            for id in outcome.value.objects().unwrap() { assert_eq!(game.object(*id).unwrap().zone, Zone::Battlefield); assert!(game.is_manifested(*id)); assert!(game.is_face_down(*id)); }
            assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<KeywordActionEvent>()).map(|event| event.amount).sum::<u32>(), 2);
            assert_eq!(game.player(alice).unwrap().life, 20); assert_eq!(game.player(bob).unwrap().life, if mode == 3 { 20 } else { 27 });
            if mode == 3 { let arrival = game.objects_in_deterministic_order().into_iter().find(|object| object.stable_id == stable).unwrap(); assert_eq!(game.counter_count(arrival.id, CounterType::PlusOnePlusOne), 1); }
            else { assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>()).map(|event| (event.player, event.amount)).collect::<Vec<_>>(), vec![(bob, 3), (bob, 4)]); }
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
        assert_eq!(ctx.source, source); assert_eq!(ctx.controller, alice); assert!(ctx.targets.is_empty()); assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id, sentinel_snapshot.object_id); assert_eq!(game.counter_count(sentinel, CounterType::PlusOnePlusOne), 0);
        if mode == 1 || mode == 2 {
            for id in originals { assert_eq!(game.object(id).unwrap().zone, origin); assert!(!game.is_manifested(id)); assert!(!game.is_face_down(id)); }
            assert_eq!(game.player(bob).unwrap().life, 20); assert_eq!(game.next_object_id_counter(), ids); assert_eq!(game.objects_in_deterministic_order().len(), objects); assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx); if mode == 0 || mode == 3 { assert_eq!(dm.calls, 1); }
        if mode == 2 { assert_eq!(dm.calls, 1); dm.pause = false; dm.pending = false;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm); ctx.set_tagged_objects("victims", snapshots);
            let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap(); assert_eq!(outcome.value.objects().unwrap().len(), 2); assert_eq!(game.player(bob).unwrap().life, 27); assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx); assert_eq!(dm.calls, 2);
        }
    }
    #[test] fn simultaneous_additions_follow_manifest_annotations() { check(false, 0); }
    #[test] fn simultaneous_error_restores_overlays_and_entries() { check(false, 1); }
    #[test] fn simultaneous_pending_replays_once() { check(false, 2); }
    #[test] fn simultaneous_addition_binds_arrival() { check(false, 3); }
    #[test] fn sequential_cloak_additions_precede_next_entry() { check(true, 0); }
    #[test] fn sequential_cloak_error_restores_overlays_and_entries() { check(true, 1); }
    #[test] fn sequential_cloak_pending_replays_once() { check(true, 2); }
    #[test] fn sequential_cloak_addition_binds_arrival() { check(true, 3); }
}

#[cfg(test)]
mod explore_zone_replacement_owner_contract_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;

    struct Answers { explorer: ObjectId, land: bool, pause: bool, pending: bool, calls: usize }
    impl DecisionMaker for Answers {
        fn decide_boolean(&mut self, game: &GameState, choice: &crate::decisions::context::BooleanContext) -> bool {
            if choice.player == PlayerId::from_index(0) { return true; }
            self.calls += 1;
            assert!(game.player(PlayerId::from_index(0)).unwrap().library.is_empty(), "the original Explore movement must finish before its additions");
            assert_eq!(game.counter_count(self.explorer, CounterType::PlusOnePlusOne), u32::from(!self.land));
            self.pending = self.pause;
            !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn check(land: bool, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let explorer = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Explore owner subject")
            .card_types(vec![crate::types::CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2)).build(), alice, Zone::Battlefield);
        let source = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Explore owner source")
            .card_types(vec![crate::types::CardType::Artifact]).build(), bob, Zone::Battlefield);
        let revealed = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Explore owner card")
            .card_types(vec![if land { crate::types::CardType::Land } else { crate::types::CardType::Sorcery }]).build(), alice, Zone::Library);
        let stable = game.object(revealed).unwrap().stable_id;
        let destination = if land { Zone::Hand } else { Zone::Graveyard };
        let actions = if mode == 1 { vec![Effect::gain_life(3), Effect::lose_life(Value::X)] }
            else { vec![Effect::gain_life(3), Effect::new(crate::effects::MayEffect::new_for_player(vec![Effect::gain_life(4)], crate::target::PlayerFilter::You))] };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(revealed), Some(Zone::Library), Some(destination)), ReplacementAction::Additionally(actions)));
        game.take_pending_trigger_events(); let ids = game.next_object_id_counter(); let objects = game.objects_in_deterministic_order().len();
        let mut dm = Answers { explorer, land, pause: mode == 2, pending: false, calls: 0 };
        let effect = ExploreEffect::new(ChooseSpec::SpecificObject(explorer));
        let mut ctx = ExecutionContext::new(explorer, alice, &mut dm);
        let result = effect.execute(&mut game, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))), "Explore must propagate a movement addition's error"); }
        else if mode == 2 { assert!(ctx.decision_maker.awaiting_choice(), "Explore must expose the added instruction's pending choice"); assert!(result.unwrap().events.is_empty()); }
        else {
            let outcome = result.unwrap(); assert_eq!(game.player(bob).unwrap().life, 27, "Explore must execute movement additions");
            assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, destination);
            assert_eq!(game.counter_count(explorer, CounterType::PlusOnePlusOne), u32::from(!land));
            assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>()).map(|event| (event.player,event.amount)).collect::<Vec<_>>(), vec![(bob,3),(bob,4)]);
            assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<KeywordActionEvent>()).filter(|event| event.action==KeywordActionKind::Explore).count(),1);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
        assert_eq!(ctx.source, explorer); assert_eq!(ctx.controller, alice); assert_eq!(game.player(alice).unwrap().life,20);
        if mode == 1 || mode == 2 {
            assert_eq!(game.player(bob).unwrap().life,20); assert_eq!(game.object(revealed).unwrap().zone,Zone::Library);
            assert_eq!(game.player(alice).unwrap().library.to_vec(),vec![revealed]); assert_eq!(game.counter_count(explorer,CounterType::PlusOnePlusOne),0);
            assert_eq!(game.next_object_id_counter(),ids); assert_eq!(game.objects_in_deterministic_order().len(),objects);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx);
        if mode==0 { assert_eq!(dm.calls,1); }
        if mode==2 {
            assert_eq!(dm.calls,1); dm.pause=false; dm.pending=false;
            let mut ctx=ExecutionContext::new(explorer,alice,&mut dm);
            let outcome=effect.execute(&mut game,&mut ctx).unwrap(); assert!(!ctx.decision_maker.awaiting_choice());
            assert_eq!(game.player(bob).unwrap().life,27); assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,destination);
            assert_eq!(game.counter_count(explorer,CounterType::PlusOnePlusOne),u32::from(!land));
            assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>()).count(),2);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none()); drop(ctx); assert_eq!(dm.calls,2);
        }
    }
    #[test] fn land_additions_execute() { check(true,0); }
    #[test] fn land_addition_error_restores_explore() { check(true,1); }
    #[test] fn land_addition_pending_replays_once() { check(true,2); }
    #[test] fn nonland_additions_execute() { check(false,0); }
    #[test] fn nonland_addition_error_restores_explore() { check(false,1); }
    #[test] fn nonland_addition_pending_replays_once() { check(false,2); }
}

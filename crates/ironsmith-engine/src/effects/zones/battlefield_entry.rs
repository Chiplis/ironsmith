use crate::effects::ExecutionContext;
use crate::effects::ExecutionError;
use crate::effects::helpers::resolve_value;
use crate::events::EnterBattlefieldEvent;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

/// Controller policy when an object enters the battlefield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlefieldEntryController {
    Preserve,
    Owner,
    Specific(PlayerId),
}

/// Original entry verdict and all deferred instructions. The enclosing effect
/// finishes authored links/combat state and the whole original batch before
/// executing these programs through `finish_battlefield_entry_receipts`.
#[derive(Debug, Clone)]
#[must_use = "retain and finish every entry receipt after the original batch"]
pub(crate) struct BattlefieldEntryReceipt {
    pub outcome: BattlefieldEntryOutcome,
    original_object: ObjectId,
    zone_receipt: crate::events::processing::PreparedEventOutcome<super::AppliedZoneChange>,
}

impl BattlefieldEntryReceipt {
    pub(crate) fn into_zone_receipt(self) -> (ObjectId, crate::events::processing::PreparedEventOutcome<super::AppliedZoneChange>) {
        (self.original_object, self.zone_receipt)
    }
}

pub(crate) fn finish_battlefield_entry_receipts(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: crate::effect::EffectOutcome,
    receipts: Vec<BattlefieldEntryReceipt>,
) -> Result<crate::effect::EffectOutcome, ExecutionError> {
    super::finish_zone_change_receipts(game, ctx, original,
        receipts.into_iter().map(BattlefieldEntryReceipt::into_zone_receipt).collect())
}

/// Config for moving an object to the battlefield through ETB processing.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BattlefieldEntryOptions {
    pub controller: BattlefieldEntryController,
    pub tapped: bool,
    pub transformed: bool,
    /// Authored prospective characteristics, independent of the physical cards.
    pub entry_definition: Option<crate::cards::CardDefinition>,
    pub linked_face_mana_cost: Option<crate::mana::ManaCost>,
    /// Original physical cards that become one entering permanent.
    pub physical_components: Vec<ObjectId>,
    pub initial_counters: Vec<(crate::object::CounterType, u32)>,
    /// One-shot continuous modifications that define how this object enters.
    pub entry_modifications: Vec<crate::continuous::Modification>,
    /// The object or player the effect says this enters attached to ("...
    /// onto the battlefield attached to X"). An Aura then doesn't choose what
    /// to enchant (CR 303.4f).
    pub entry_attachment: Option<crate::object::AttachmentTarget>,
    /// An Aura spell becomes unattached if replacement makes it a non-Aura.
    pub entry_attachment_requires_aura: bool,
}

impl BattlefieldEntryOptions {
    pub(crate) fn preserve(tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Preserve,
            tapped,
            transformed: false,
            entry_definition: None,
            linked_face_mana_cost: None,
            physical_components: Vec::new(),
            initial_counters: Vec::new(),
            entry_modifications: Vec::new(),
            entry_attachment: None,
            entry_attachment_requires_aura: false,
        }
    }

    pub(crate) fn owner(tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Owner,
            tapped,
            transformed: false,
            entry_definition: None,
            linked_face_mana_cost: None,
            physical_components: Vec::new(),
            initial_counters: Vec::new(),
            entry_modifications: Vec::new(),
            entry_attachment: None,
            entry_attachment_requires_aura: false,
        }
    }

    pub(crate) fn specific(controller: PlayerId, tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Specific(controller),
            tapped,
            transformed: false,
            entry_definition: None,
            linked_face_mana_cost: None,
            physical_components: Vec::new(),
            initial_counters: Vec::new(),
            entry_modifications: Vec::new(),
            entry_attachment: None,
            entry_attachment_requires_aura: false,
        }
    }

    pub(crate) fn with_initial_counters(
        mut self,
        counters: Vec<(crate::object::CounterType, u32)>,
    ) -> Self {
        self.initial_counters = counters;
        self
    }

    pub(crate) fn with_entry_modifications(
        mut self,
        modifications: Vec<crate::continuous::Modification>,
    ) -> Self {
        self.entry_modifications = modifications;
        self
    }

    pub(crate) fn with_entry_attachment(
        mut self,
        target: Option<crate::object::AttachmentTarget>,
    ) -> Self {
        self.entry_attachment = target;
        self
    }

    pub(crate) fn with_aura_entry_attachment(mut self, target: Option<crate::object::AttachmentTarget>) -> Self {
        self.entry_attachment = target;
        self.entry_attachment_requires_aura = true;
        self
    }

    pub(crate) fn with_linked_face_mana_cost(mut self, cost: crate::mana::ManaCost) -> Self {
        self.linked_face_mana_cost = Some(cost);
        self
    }

    pub(crate) fn with_composite_entry(
        mut self, definition: crate::cards::CardDefinition, components: Vec<ObjectId>,
    ) -> Self {
        self.entry_definition = Some(definition);
        self.physical_components = components;
        self
    }

    pub(crate) fn transformed(mut self, transformed: bool) -> Self {
        self.transformed = transformed;
        self
    }
}

/// Resolve authored one-shot entry-counter metadata before the object changes
/// zones. The resulting concrete counters are passed into ETB replacement
/// processing as part of the original enter event.
pub(crate) fn resolve_battlefield_entry_counters(
    game: &GameState,
    ctx: &ExecutionContext,
    object_id: ObjectId,
    specs: &[ironsmith_core::BattlefieldEntryCounterSpec],
) -> Result<Vec<(crate::object::CounterType, u32)>, ExecutionError> {
    let mut counters = Vec::new();
    for spec in specs {
        if let Some(condition) = &spec.condition
            && !crate::condition_eval::evaluate_condition_resolution(game, condition, ctx)?
        {
            continue;
        }
        if let Some(filter) = &spec.object_filter {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            let filter_ctx = ctx.filter_context(game);
            if !filter.matches(object, &filter_ctx, game) {
                continue;
            }
        }
        let amount = resolve_value(game, &spec.amount, ctx)?.max(0) as u32;
        if amount > 0 {
            counters.push((spec.counter_type, amount));
        }
    }
    Ok(counters)
}

/// Result for a move-to-battlefield attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BattlefieldEntryOutcome {
    Moved(ObjectId),
    /// The zone movement committed to a non-battlefield destination.
    Redirected(super::AppliedZoneChange),
    Prevented,
}

fn battlefield_entry_controller(
    game: &GameState,
    object: ObjectId,
    options: &BattlefieldEntryOptions,
) -> Option<PlayerId> {
    let object = game.object(object)?;
    Some(match options.controller {
        BattlefieldEntryController::Specific(controller) => controller,
        BattlefieldEntryController::Owner => object.owner,
        BattlefieldEntryController::Preserve => game.controller_of(object),
    })
}

/// Resolve the back-face definition used by an instruction that puts a card
/// onto the battlefield transformed. This is not a transform action, so
/// "can't transform" restrictions do not apply; an object without a
/// permanent transforming back face simply cannot enter this way.
fn transformed_entry_definition(
    game: &GameState,
    object_id: ObjectId,
) -> Option<crate::cards::CardDefinition> {
    let object = game.object(object_id)?;
    if object.linked_face_layout != crate::card::LinkedFaceLayout::TransformLike {
        return None;
    }
    let current = game.linked_face_definition_by_name_or_id(Some(&object.name), object.card)?;
    let other = game.linked_face_definition_by_name_or_id(
        object.other_face_name.as_deref(),
        object.other_face,
    )?;
    if current.card.linked_face_layout != crate::card::LinkedFaceLayout::TransformLike
        || other.card.linked_face_layout != crate::card::LinkedFaceLayout::TransformLike
    {
        return None;
    }

    // Transform-like card families use the lower card id for the front face,
    // matching the existing default-face restoration used by zone changes.
    let transformed = if current.card.id.0 > other.card.id.0 {
        current
    } else {
        other
    };
    (!transformed.card.card_types.iter().any(|card_type| {
        matches!(
            card_type,
            crate::types::CardType::Instant | crate::types::CardType::Sorcery
        )
    }))
    .then_some(transformed)
}

fn apply_entry_definition(
    game: &mut GameState,
    object_id: ObjectId,
    definition: &crate::cards::CardDefinition,
) -> bool {
    let Some(object) = game.object_mut(object_id) else {
        return false;
    };
    object.apply_definition_face(definition);
    true
}

fn apnap_position(game: &GameState, player: PlayerId) -> usize {
    game.team_apnap_player_order()
        .iter()
        .position(|candidate| *candidate == player)
        .unwrap_or(usize::MAX)
}

fn apply_entry_modifications(
    game: &mut GameState,
    ctx: &ExecutionContext,
    object: ObjectId,
    options: &BattlefieldEntryOptions,
) -> Result<Vec<crate::continuous::ContinuousEffectId>, ExecutionError> {
    if options.entry_modifications.is_empty() {
        return Ok(Vec::new());
    }
    let group = game.effect_store.continuous_effects.next_effect_group_id();
    let ids = options
        .entry_modifications
        .iter()
        .map(|modification| {
            game.effect_store.continuous_effects.add_effect(
                crate::continuous::ContinuousEffect::new(
                    ctx.source,
                    ctx.controller,
                    crate::continuous::EffectTarget::Specific(object),
                    modification.clone(),
                )
                .with_group(group),
            )
        })
        .collect();
    game.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
    Ok(ids)
}

fn finish_battlefield_entry(
    game: &mut GameState,
    ctx: &ExecutionContext,
    old_object: ObjectId,
    old_zone: Zone,
    result: crate::game_state::EntersResult,
) -> Result<BattlefieldEntryOutcome, ExecutionError> {
    let new_id = result.new_id;
    let destination = game.object(new_id)
        .ok_or(ExecutionError::ObjectNotFound(new_id))?.zone;
    // A failed attachment choice can return the original object without any
    // movement. It is not a completed redirection and must not acquire links.
    if new_id == old_object && destination == old_zone {
        return Ok(BattlefieldEntryOutcome::Prevented);
    }
    if destination != Zone::Battlefield {
        let mut new_ids = game.take_zone_change_results(old_object);
        if new_ids.is_empty() { new_ids.push(new_id); }
        // Preserve the identity continuation for other consumers of this move.
        game.record_zone_change_results(old_object, new_ids.clone());
        return Ok(BattlefieldEntryOutcome::Redirected(super::AppliedZoneChange {
            final_zone: destination, new_object_id: Some(new_id), new_object_ids: new_ids,
        }));
    }

    game.add_battlefield_put_with_source_link(ctx.source, new_id);
    let enters_tapped = result.enters_tapped;

    // "This creature enters prepared." The permanent is on the battlefield by
    // now, which is what the prepare spell copy's existence is tied to.
    let event = if enters_tapped {
        TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::tapped(new_id, old_zone),
            ProvNodeId::default(),
        )
    } else {
        TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::new(new_id, old_zone),
            ProvNodeId::default(),
        )
    };
    game.queue_trigger_event(ctx.provenance, event);
    Ok(BattlefieldEntryOutcome::Moved(new_id))
}

/// Prepare and commit a simultaneous group of battlefield entries.
///
/// Replacement choices are gathered in APNAP order against the pre-entry
/// battlefield. The explicit reserved-ID set excludes pending entrants from
/// entry replacement choices without removing real cards from zone indexes.
/// Replacement programs can therefore execute real movements against a valid
/// game state, while combined costs remain visible to later choices. The fully prepared entries are then committed on a
/// clone and published as one state transition.
pub(crate) fn move_to_battlefield_batch_with_options(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    requests: Vec<(ObjectId, BattlefieldEntryOptions)>,
) -> Result<Vec<BattlefieldEntryReceipt>, ExecutionError> {
    move_to_battlefield_batch_with_options_and_zone_proposals(
        game, ctx, requests, std::collections::HashMap::new(),
    )
}

pub(crate) fn move_to_battlefield_batch_with_options_and_zone_proposals(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    requests: Vec<(ObjectId, BattlefieldEntryOptions)>,
    zone_proposals: std::collections::HashMap<ObjectId, crate::events::processing::PreparedBattlefieldZoneChange>,
) -> Result<Vec<BattlefieldEntryReceipt>, ExecutionError> {
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = move_to_battlefield_batch_with_options_inner(game, ctx, requests, zone_proposals);
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        *game = checkpoint;
        context_checkpoint.restore(ctx);
    }
    result
}

fn move_to_battlefield_batch_with_options_inner(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    requests: Vec<(ObjectId, BattlefieldEntryOptions)>,
    mut zone_proposals: std::collections::HashMap<ObjectId, crate::events::processing::PreparedBattlefieldZoneChange>,
) -> Result<Vec<BattlefieldEntryReceipt>, ExecutionError> {
    if requests.is_empty() {
        return Ok(Vec::new());
    }

    // Original zone snapshots and observer lookback precede provisional faces,
    // continuous modifications and every entry program in this batch.
    let pre_event_lookback = game.trigger_source_lookback_snapshots();
    let original_snapshots = requests.iter().filter_map(|(id, _)| {
        game.object(*id).map(|object| (*id,
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, game)))
    }).collect::<std::collections::HashMap<_, _>>();
    let mut physical_entries = std::collections::HashMap::new();
    for (index, (primary, options)) in requests.iter().enumerate() {
        if options.physical_components.is_empty() { continue; }
        let unique = options.physical_components.iter().copied().collect::<std::collections::HashSet<_>>();
        let origin = game.object(*primary).ok_or(ExecutionError::ObjectNotFound(*primary))?.zone;
        if options.physical_components.first() != Some(primary)
            || unique.len() != options.physical_components.len() || options.entry_definition.is_none()
            || origin == Zone::Battlefield
        {
            return Err(ExecutionError::InternalError("invalid composite entry representation".into()));
        }
        let mut components = Vec::new();
        for &id in &options.physical_components {
            let object = game.object(id).ok_or(ExecutionError::ObjectNotFound(id))?;
            if object.zone != origin || object.kind != crate::object::ObjectKind::Card {
                return Err(ExecutionError::InternalError("composite entry cards have incompatible origins".into()));
            }
            let definition = game.linked_face_definition_by_name_or_id(Some(object.name.as_str()), object.card)
                .ok_or_else(|| ExecutionError::InternalError("composite entry source definition missing".into()))?;
            components.push(crate::game_state::PreparedEntryComponent {
                snapshot: crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, game),
                original_definition: definition,
            });
        }
        physical_entries.insert(index, components);
    }
    let additional_effects = ctx.additional_replacement_effects_snapshot();
    let mut working = game.clone();
    let mut transformed_entry_states = std::collections::HashMap::new();
    for (index, (object_id, options)) in requests.iter().enumerate() {
        if working.object(*object_id).is_some_and(|object| object.zone == Zone::Battlefield)
            || (!options.transformed && options.entry_definition.is_none()) {
            continue;
        }
        let Some(original) = working.object(*object_id).cloned() else {
            continue;
        };
        let Some(definition) = options.entry_definition.clone().or_else(|| transformed_entry_definition(&working, *object_id)) else {
            continue;
        };
        if apply_entry_definition(&mut working, *object_id, &definition) {
            transformed_entry_states.insert(index, (original, definition));
        }
    }
    // The replacement proposal must see the characteristics the returning
    // effect gives the entering object, before any entry replacements match.
    let mut provisional_effects = Vec::new();
    let mut provisional_entry_effects = std::collections::HashMap::new();
    for (object, options) in &requests {
        if working.object(*object).is_some_and(|object| object.zone == Zone::Battlefield) {
            continue;
        }
        let effects = apply_entry_modifications(&mut working, ctx, *object, options)?;
        provisional_effects.extend(effects.iter().copied());
        provisional_entry_effects.insert(*object, effects);
    }
    let eligible_indices = requests
        .iter()
        .enumerate()
        .filter_map(|(index, (object, options))| {
            if working.object(*object).is_some_and(|object| object.zone == Zone::Battlefield) {
                return None;
            }
            if options.transformed && !transformed_entry_states.contains_key(&index) {
                return None;
            }
            if working.card_cannot_enter_battlefield(*object) {
                return None;
            }
            battlefield_entry_controller(&working, *object, options)
                .filter(|controller| {
                    working
                        .player(*controller)
                        .is_some_and(|player| player.is_in_game())
                })
                .map(|_| index)
        })
        .collect::<std::collections::HashSet<_>>();
    let mut reserved_objects = requests
        .iter()
        .enumerate()
        .filter(|(index, _)| eligible_indices.contains(index))
        .map(|(_, (object, _))| *object)
        .collect::<std::collections::HashSet<_>>();
    for &index in &eligible_indices {
        if let Some(components) = physical_entries.get(&index) {
            reserved_objects.extend(components.iter().map(|component| component.snapshot.object_id));
        }
    }

    let mut proposal_order = eligible_indices.iter().copied().collect::<Vec<_>>();
    proposal_order.sort_by_key(|index| {
        let (object, options) = &requests[*index];
        let controller =
            battlefield_entry_controller(&working, *object, options).unwrap_or(ctx.controller);
        (apnap_position(&working, controller), *index)
    });

    let preparation_order = proposal_order.clone();
    let mut proposals = vec![None; requests.len()];
    let mut proposal_lookbacks = std::collections::HashMap::new();
    // Retain instructions even for proposals rejected by prospective entry
    // restrictions. The original verdict does not erase previously applied
    // zone replacement instructions.
    let mut deferred_programs = requests.iter().map(|(object, _)| {
        zone_proposals.get_mut(object).map(|proposal| proposal.take_programs()).unwrap_or_default()
    }).collect::<Vec<_>>();
    let mut replaced_entries = std::collections::HashSet::new();
    for index in proposal_order {
        let (object, options) = &requests[index];
        let Some(old_zone) = working.object(*object).map(|object| object.zone) else {
            continue;
        };
        let entering_controller = match options.controller {
            BattlefieldEntryController::Specific(controller) => Some(controller),
            BattlefieldEntryController::Preserve | BattlefieldEntryController::Owner => None,
        };
        let (mut scope, scoped_additional, lookback, programs) = match zone_proposals.remove(object) {
            Some(proposal) => proposal.into_entry_scope(),
            None => (
                crate::events::processing::ReplacementEventContext::with_scope(&working, crate::events::Event::zone_change(*object, old_zone, Zone::Battlefield,
                ctx.cause.clone(), original_snapshots.get(object).cloned())
                .with_provenance(ctx.provenance), &ctx.replacement),
                additional_effects.clone(), pre_event_lookback.clone(), Vec::new(),
            ),
        };
        if let Some(components) = physical_entries.get(&index) {
            let snapshots = components.iter().map(|component| component.snapshot.clone()).collect::<Vec<_>>();
            let zone = crate::events::zones::ZoneChangeEvent::batch_with_snapshots(
                snapshots.iter().map(|snapshot| snapshot.object_id).collect(), old_zone, Zone::Battlefield,
                ctx.cause.clone(), snapshots,
            );
            scope.event = crate::events::Event::new_with_provenance(zone.clone(), ctx.provenance);
            scope.zone_change_context = Some(zone);
        }
        deferred_programs[index].extend(programs);
        proposal_lookbacks.insert(index, lookback);
        // Store only source presentations overlaid for this proposal. Instead
        // programs operate on the real originals after entry has been replaced.
        let original_source_faces = if let Some(components) = physical_entries.get(&index) {
            components.iter().filter_map(|component| game.object(component.snapshot.object_id).cloned()).collect::<Vec<_>>()
        } else {
            transformed_entry_states.get(&index).map(|(original, _)| vec![original.clone()]).unwrap_or_default()
        };
        let mut result = crate::events::processing::process_etb_batch_proposal_with_scope(
            &mut working,
            *object,
            old_zone,
            &mut ctx.decision_maker,
            options.initial_counters.clone(),
            options.tapped,
            entering_controller,
            &reserved_objects,
            scope,
            &scoped_additional,
            &original_source_faces,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        deferred_programs[index].append(&mut result.additional_programs);
        if result.replaced { replaced_entries.insert(index); }
        reserved_objects.extend(result.linked_exile_with_entering.iter().copied());
        proposals[index] = Some((old_zone, result));
    }

    let mut prepared_entries = vec![None; requests.len()];
    for index in preparation_order {
        let Some((old_zone, proposal)) = proposals[index].take() else {
            continue;
        };
        let (object, options) = &requests[index];
        let entering_controller = match options.controller {
            BattlefieldEntryController::Specific(controller) => Some(controller),
            BattlefieldEntryController::Preserve | BattlefieldEntryController::Owner => None,
        };
        let Some(mut prepared) = working.prepare_etb_entry_with_controller_and_dm(
            *object,
            proposal,
            entering_controller,
            &mut ctx.decision_maker,
        )? else {
            return Ok(Vec::new());
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        prepared.zone_entry_lookback = Some(proposal_lookbacks.remove(&index).unwrap_or_else(|| pre_event_lookback.clone()));
        prepared.entry_definition = transformed_entry_states.get(&index).map(|(_, definition)| definition.clone());
        prepared.physical_components = physical_entries.remove(&index).unwrap_or_default();
        prepared.linked_face_mana_cost = options.linked_face_mana_cost.clone();
        // Retarget authored characteristic changes with the actual entrant,
        // before attachment legality and other commit-time checks inspect it.
        if let Some(effects) = provisional_entry_effects.get(object) {
            prepared.choices.as_enters_continuous_effects.extend(effects.iter().copied());
        }
        prepared.entry_attachment = options.entry_attachment;
        prepared.entry_attachment_requires_aura = options.entry_attachment_requires_aura;
        prepared_entries[index] = Some((old_zone, prepared));
    }

    // Every proposal in this simultaneous event has now been prepared against
    // the same batch-scoped one-shot replacements. Consume the replacements
    // that matched at least one member before committing the prepared entries,
    // so a later independent ETB event cannot reuse them.
    working
        .effect_store
        .replacement_effects
        .consume_pending_batch_one_shot_effects();

    // CR 603.2c: the entries are one simultaneous event, so "whenever one or
    // more creatures enter" sees them together.
    let opened_batch = working.open_simultaneous_action();
    let mut outcomes = vec![BattlefieldEntryOutcome::Prevented; requests.len()];
    for (index, (object, options)) in requests.iter().enumerate() {
        let Some((old_zone, prepared_entry)) = prepared_entries[index].take() else {
            continue;
        };
        let entering_controller = match options.controller {
            BattlefieldEntryController::Specific(controller) => Some(controller),
            BattlefieldEntryController::Preserve | BattlefieldEntryController::Owner => None,
        };
        let committed = working.commit_prepared_etb_with_cause_and_options_and_dm(
            *object,
            prepared_entry,
            entering_controller,
            ctx.cause.clone(),
            options.entry_attachment.is_none(),
            &mut ctx.decision_maker,
        );
        let mut committed = committed?;
        if committed.pending || ctx.decision_maker.awaiting_choice() { return Ok(Vec::new()); }
        deferred_programs[index].append(&mut committed.programs);
        let result = match committed.original {
            crate::events::processing::EventOutcome::Proceed(result) => result,
            crate::events::processing::EventOutcome::Replaced => {
                replaced_entries.insert(index);
                continue;
            }
            crate::events::processing::EventOutcome::Prevented
                | crate::events::processing::EventOutcome::NotApplicable => continue,
        };
        // Provisional modifications participate in replacement matching above;
        // lasting entry effects belong only to an actual battlefield entrant.
        if working.object(result.new_id).is_some_and(|object| object.zone == Zone::Battlefield) {
            // These effects now describe the committed entrant. Keep them;
            // redirected and prevented proposals still lose provisional effects.
            if let Some(effects) = provisional_entry_effects.get(object) {
                provisional_effects.retain(|id| !effects.contains(id));
            }
        }
        outcomes[index] = finish_battlefield_entry(&mut working, ctx, *object, old_zone, result)?;
    }

    // CR 613.7j: objects that receive timestamps simultaneously get them in an
    // order the active player chooses. Only entrants whose static abilities
    // generate continuous effects can make that order observable, so the
    // choice is offered for those; everything else keeps commit order.
    let relevant_entrants: Vec<ObjectId> = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            BattlefieldEntryOutcome::Moved(id) => Some(*id),
            BattlefieldEntryOutcome::Redirected(_) | BattlefieldEntryOutcome::Prevented => None,
        })
        .filter(|id| entrant_generates_continuous_effects(&working, *id))
        .collect();
    if relevant_entrants.len() >= 2 {
        let Some(ordered) =
            choose_simultaneous_timestamp_order(&working, ctx.decision_maker, &relevant_entrants)
        else {
            return Ok(Vec::new());
        };
        for id in ordered {
            working.effect_store.continuous_effects.record_entry(id);
        }
    }

    for (index, (object_id, _)) in requests.iter().enumerate() {
        if outcomes[index] != BattlefieldEntryOutcome::Prevented {
            continue;
        }
        let Some((original, _)) = transformed_entry_states.get(&index) else {
            continue;
        };
        if let Some(object) = working.object_mut(*object_id) {
            *object = original.clone();
        }
    }

    for id in provisional_effects {
        working.effect_store.continuous_effects.remove_effect(id);
    }
    working.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
    working.close_simultaneous_action(opened_batch);
    *game = working;
    Ok(outcomes.into_iter().enumerate().map(|(index, outcome)| {
        use crate::events::processing::{EventOutcome, PreparedEventOutcome};
        let original = match &outcome {
            BattlefieldEntryOutcome::Moved(id) => EventOutcome::Proceed(super::AppliedZoneChange {
                final_zone: Zone::Battlefield, new_object_id: Some(*id), new_object_ids: vec![*id],
            }),
            BattlefieldEntryOutcome::Redirected(change) => EventOutcome::Proceed(change.clone()),
            BattlefieldEntryOutcome::Prevented if replaced_entries.contains(&index) => EventOutcome::Replaced,
            BattlefieldEntryOutcome::Prevented => EventOutcome::Prevented,
        };
        BattlefieldEntryReceipt {
            outcome, original_object: requests[index].0,
            zone_receipt: PreparedEventOutcome { original, programs: std::mem::take(&mut deferred_programs[index]) },
        }
    }).collect())
}

/// True when a permanent's own static abilities generate continuous effects,
/// so its timestamp relative to other simultaneous entrants can matter.
fn entrant_generates_continuous_effects(game: &GameState, id: ObjectId) -> bool {
    let Some(object) = game.object(id) else {
        return false;
    };
    let controller = game.controller_of(object);
    object.abilities.iter().any(|ability| {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        ability.functions_in(&object.zone)
            && !static_ability
                .generate_effects(id, controller, game)
                .is_empty()
    })
}

/// Ask the active player for the timestamp order of simultaneous entrants
/// (CR 613.7j). The leftmost item receives the oldest timestamp. Returns
/// `None` while the decision is still pending.
fn choose_simultaneous_timestamp_order(
    game: &GameState,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
    entrants: &[ObjectId],
) -> Option<Vec<ObjectId>> {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let items: Vec<(ObjectId, String)> = entrants
        .iter()
        .map(|id| {
            let name = game
                .object(*id)
                .map(|object| object.name.to_string())
                .unwrap_or_else(|| "Permanent".to_string());
            let ordinal = seen.entry(name.clone()).or_insert(0);
            *ordinal += 1;
            let label = if *ordinal > 1 {
                format!("{name} ({ordinal})")
            } else {
                name
            };
            (*id, label)
        })
        .collect();
    let context = crate::decisions::context::enrich_display_hints(
        game,
        crate::decisions::context::DecisionContext::Order(
            crate::decisions::context::OrderContext::new(
                game.turn.active_player,
                None,
                "Choose the timestamp order for permanents entering at the same time. \
                 The leftmost item is treated as having entered first.",
                items,
            ),
        ),
    )
    .into_order();
    let response = decision_maker.decide_order(game, &context);
    if decision_maker.awaiting_choice() {
        return None;
    }

    let mut remaining: Vec<ObjectId> = entrants.to_vec();
    let mut ordered = Vec::with_capacity(remaining.len());
    for id in response {
        if let Some(position) = remaining.iter().position(|candidate| *candidate == id) {
            ordered.push(remaining.remove(position));
        }
    }
    ordered.extend(remaining);
    Some(ordered)
}

/// Move an object to the battlefield with ETB replacement processing and policy hooks.
pub(crate) fn move_to_battlefield_with_options(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    object_id: ObjectId,
    options: BattlefieldEntryOptions,
) -> Result<Option<BattlefieldEntryReceipt>, ExecutionError> {
    Ok(move_to_battlefield_batch_with_options(game, ctx, vec![(object_id, options)])?
        .into_iter()
        .next())
}

#[cfg(test)]
impl BattlefieldEntryReceipt {
    pub(crate) fn assert_without_additions(&self) -> &BattlefieldEntryOutcome {
        assert!(self.zone_receipt.programs.is_empty(), "fixture must finish its added entry programs");
        &self.outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::color::Color;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::{ColorsContext, SelectObjectsContext};
    use crate::ids::CardId;
    use crate::object::{AttachmentTarget, AuraAttachmentFilter, Object};
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;
    use crate::types::{CardType, Subtype};

    struct EntryOrderDm {
        untapper: ObjectId,
        untapped_last: bool,
        choices: usize,
    }

    impl DecisionMaker for EntryOrderDm {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.choices += 1;
            let option = ctx
                .options
                .iter()
                .find(|option| {
                    option.legal && (option.object_id == Some(self.untapper)) != self.untapped_last
                })
                .expect("both entry replacements should be offered");
            vec![option.index]
        }
    }

    fn check_tapped_instruction(has_untapper: bool, intrinsic_tapped: bool, untapped_last: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::new(), "Entry Untapper")
            .card_types(vec![CardType::Enchantment])
            .build();
        let mut object = Object::from_card(source, &card, alice, Zone::Battlefield);
        if has_untapper {
            object.abilities_mut().push(Ability::static_ability(
                StaticAbility::enters_untapped_for_filter(ObjectFilter::land().you_control()),
            ));
        }
        game.add_object(object);
        let requests = (0..3)
            .map(|_| {
                let id = game.new_object_id();
                let card = CardBuilder::new(CardId::new(), "Entering Land")
                    .card_types(vec![CardType::Land])
                    .build();
                let mut object = Object::from_card(id, &card, alice, Zone::Library);
                if intrinsic_tapped {
                    object.abilities_mut().push(Ability::static_ability(
                        StaticAbility::enters_tapped_ability(),
                    ));
                }
                game.add_object(object);
                (id, BattlefieldEntryOptions::specific(alice, true))
            })
            .collect();
        let mut dm = EntryOrderDm {
            untapper: source,
            untapped_last,
            choices: 0,
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcomes = move_to_battlefield_batch_with_options(&mut game, &mut ctx, requests).expect("replacement operation must execute successfully in this scenario");
        let expected_tapped = !has_untapper || (intrinsic_tapped && !untapped_last);
        for outcome in outcomes {
            let BattlefieldEntryOutcome::Moved(id) = outcome.assert_without_additions() else {
                panic!("all three lands should enter");
            };
            assert_eq!(game.is_tapped(*id), expected_tapped);
        }
        let events = game.take_pending_trigger_events();
        let entries: Vec<_> = events
            .iter()
            .filter_map(|event| {
                crate::events::downcast_event::<EnterBattlefieldEvent>(event.inner())
            })
            .collect();
        assert_eq!(entries.len(), 3);
        assert!(
            entries
                .iter()
                .all(|event| event.enters_tapped == expected_tapped)
        );
        assert_eq!(
            dm.choices,
            if has_untapper && intrinsic_tapped {
                3
            } else {
                0
            }
        );
    }

    #[test]
    fn tapped_instruction_allows_untapped_replacement_last() {
        check_tapped_instruction(true, true, true);
    }

    #[test]
    fn tapped_instruction_allows_tapped_replacement_last() {
        check_tapped_instruction(true, true, false);
    }

    #[test]
    fn tapped_instruction_is_replaced_for_plain_lands() {
        check_tapped_instruction(true, false, true);
    }

    #[test]
    fn tapped_instruction_without_replacement_stays_tapped() {
        check_tapped_instruction(false, false, false);
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.add_object(Object::from_card(id, &card, owner, Zone::Battlefield));
        id
    }

    fn create_hand_aura(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        filter: ObjectFilter,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        let mut object = Object::from_card(id, &card, owner, Zone::Hand);
        object.aura_attach_filter = Some(AuraAttachmentFilter::from(filter).into());
        game.add_object(object);
        id
    }

    struct ChooseObjectDm {
        desired: ObjectId,
        chooser: Option<PlayerId>,
    }

    impl DecisionMaker for ChooseObjectDm {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.chooser = Some(ctx.player);
            ctx.candidates
                .iter()
                .any(|candidate| candidate.id == self.desired)
                .then_some(vec![self.desired])
                .unwrap_or_default()
        }
    }

    fn create_color_chooser(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        zone: Zone,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Artifact])
            .build();
        let mut object = Object::from_card(id, &card, owner, zone);
        object.abilities_mut().push(Ability::static_ability(
            StaticAbility::choose_color_as_enters(None, "As this enters, choose a color.".into()),
        ));
        game.add_object(object);
        id
    }

    struct InspectColorChoiceDm {
        color: Color,
        awaiting: bool,
        suspend_on_color_choice: bool,
        sources: Vec<Option<ObjectId>>,
        source_zones: Vec<Option<Zone>>,
        battlefield_sizes: Vec<usize>,
    }

    impl InspectColorChoiceDm {
        fn synchronous(color: Color) -> Self {
            Self {
                color,
                awaiting: false,
                suspend_on_color_choice: false,
                sources: Vec::new(),
                source_zones: Vec::new(),
                battlefield_sizes: Vec::new(),
            }
        }
    }

    impl DecisionMaker for InspectColorChoiceDm {
        fn awaiting_choice(&self) -> bool {
            self.awaiting
        }

        fn decide_colors(&mut self, game: &GameState, ctx: &ColorsContext) -> Vec<Color> {
            self.sources.push(ctx.source);
            self.source_zones.push(
                ctx.source
                    .and_then(|source| game.object(source).map(|object| object.zone)),
            );
            self.battlefield_sizes.push(game.battlefield.len());
            if self.suspend_on_color_choice {
                self.awaiting = true;
            }
            vec![self.color]
        }
    }

    #[test]
    fn as_enters_choice_is_requested_before_the_destination_object_is_committed() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant = create_color_chooser(&mut game, "Chromatic Reliquary", alice, Zone::Hand);
        let mut dm = InspectColorChoiceDm::synchronous(Color::Blue);

        let result = game
            .move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut dm).expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions().expect("the permanent should enter after its choice is complete");

        assert_eq!(dm.sources, vec![Some(entrant)]);
        assert_eq!(dm.source_zones, vec![Some(Zone::Hand)]);
        assert_eq!(dm.battlefield_sizes, vec![0]);
        assert_eq!(game.chosen_color(result.new_id), Some(Color::Blue));
    }

    #[test]
    fn instant_card_is_rejected_before_etb_replacements_or_choices_are_proposed() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(entrant.0 as u32), "Impossible Instant")
            .card_types(vec![CardType::Instant])
            .build();
        let mut object = Object::from_card(entrant, &card, alice, Zone::Hand);
        object.abilities_mut().push(Ability::static_ability(
            StaticAbility::choose_color_as_enters(None, "As this enters, choose a color.".into()),
        ));
        game.add_object(object);
        let mut dm = InspectColorChoiceDm::synchronous(Color::Blue);

        let result =
            game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut dm).expect("replacement operation must execute successfully in this scenario");

        assert!(!result.pending);
        assert!(result.programs.is_empty());
        assert!(matches!(result.original, crate::events::processing::EventOutcome::NotApplicable));
        assert!(dm.sources.is_empty(), "no ETB choice should be proposed");
        assert_eq!(
            game.object(entrant).expect("instant remains").zone,
            Zone::Hand
        );
        assert!(game.battlefield.is_empty());
    }

    #[test]
    fn suspended_as_enters_choice_keeps_the_whole_entry_uncommitted() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant = create_color_chooser(&mut game, "Pending Reliquary", alice, Zone::Hand);
        let mut dm = InspectColorChoiceDm {
            suspend_on_color_choice: true,
            ..InspectColorChoiceDm::synchronous(Color::Red)
        };

        let result =
            game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut dm).expect("replacement operation must execute successfully in this scenario");

        assert!(result.pending);
        assert!(result.programs.is_empty());
        assert_eq!(dm.source_zones, vec![Some(Zone::Hand)]);
        assert_eq!(
            game.object(entrant).map(|object| object.zone),
            Some(Zone::Hand)
        );
        assert!(game.battlefield.is_empty());
        assert!(game.chosen_color(entrant).is_none());
    }

    #[test]
    fn copied_as_enters_abilities_are_used_by_the_prospective_choice_record() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let copy_source =
            create_color_chooser(&mut game, "Chromatic Blueprint", alice, Zone::Battlefield);
        let entrant_id = game.new_object_id();
        let entrant_card =
            CardBuilder::new(CardId::from_raw(entrant_id.0 as u32), "Unfinished Replica")
                .card_types(vec![CardType::Artifact])
                .build();
        game.add_object(Object::from_card(
            entrant_id,
            &entrant_card,
            alice,
            Zone::Hand,
        ));
        let mut dm = InspectColorChoiceDm::synchronous(Color::Black);
        let proposal = crate::events::processing::EtbEventResult {
            enters_as_copy_of: Some(copy_source),
            ..Default::default()
        };

        let prepared = game
            .prepare_etb_entry_with_controller_and_dm(entrant_id, proposal, None, &mut dm).expect("replacement operation must execute successfully in this scenario")
            .expect("copy-derived entry choice should be prepared");
        assert_eq!(dm.source_zones, vec![Some(Zone::Hand)]);
        let result = game
            .commit_prepared_etb_with_controller_and_dm(entrant_id, prepared, None, &mut dm).expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions().expect("the prepared copy should enter");

        assert_eq!(game.chosen_color(result.new_id), Some(Color::Black));
        assert_eq!(
            game.object(result.new_id)
                .map(|object| object.name.as_ref()),
            Some("Chromatic Blueprint")
        );
    }

    #[test]
    fn simultaneous_as_enters_choices_are_all_collected_before_batch_commit() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let first = create_color_chooser(&mut game, "First Prism", alice, Zone::Hand);
        let second = create_color_chooser(&mut game, "Second Prism", alice, Zone::Hand);
        let mut dm = InspectColorChoiceDm::synchronous(Color::Green);
        let mut ctx = ExecutionContext::new(ObjectId::from_raw(9_031), alice, &mut dm);

        let outcomes = move_to_battlefield_batch_with_options(
            &mut game,
            &mut ctx,
            vec![
                (first, BattlefieldEntryOptions::preserve(false)),
                (second, BattlefieldEntryOptions::preserve(false)),
            ],
        ).expect("replacement operation must execute successfully in this scenario");

        assert!(
            outcomes
                .iter()
                .all(|outcome| matches!(outcome.assert_without_additions(), BattlefieldEntryOutcome::Moved(_)))
        );
        assert_eq!(dm.source_zones, vec![Some(Zone::Hand), Some(Zone::Hand)]);
        assert_eq!(dm.battlefield_sizes, vec![0, 0]);
    }

    #[test]
    fn simultaneous_entry_batch_skips_instant_without_blocking_legal_permanent() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let instant_id = game.new_object_id();
        let instant_card = CardBuilder::new(
            CardId::from_raw(instant_id.0 as u32),
            "Batch Impossible Instant",
        )
        .card_types(vec![CardType::Instant])
        .build();
        game.add_object(Object::from_card(
            instant_id,
            &instant_card,
            alice,
            Zone::Hand,
        ));
        let permanent_id = game.new_object_id();
        let permanent_card =
            CardBuilder::new(CardId::from_raw(permanent_id.0 as u32), "Batch Bear")
                .card_types(vec![CardType::Creature])
                .build();
        game.add_object(Object::from_card(
            permanent_id,
            &permanent_card,
            alice,
            Zone::Hand,
        ));
        let mut dm = InspectColorChoiceDm::synchronous(Color::Green);
        let mut ctx = ExecutionContext::new(ObjectId::from_raw(9_032), alice, &mut dm);

        let outcomes = move_to_battlefield_batch_with_options(
            &mut game,
            &mut ctx,
            vec![
                (instant_id, BattlefieldEntryOptions::preserve(false)),
                (permanent_id, BattlefieldEntryOptions::preserve(false)),
            ],
        ).expect("replacement operation must execute successfully in this scenario");

        assert!(matches!(outcomes[0].assert_without_additions(), BattlefieldEntryOutcome::Prevented));
        assert!(matches!(outcomes[1].assert_without_additions(), BattlefieldEntryOutcome::Moved(_)));
        assert_eq!(
            game.object(instant_id).expect("instant remains").zone,
            Zone::Hand
        );
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Batch Bear")
        }));
    }

    #[test]
    fn discard_hand_as_enters_finishes_before_the_battlefield_zone_change() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant_id = game.new_object_id();
        let entrant_card = CardBuilder::new(CardId::from_raw(entrant_id.0 as u32), "Memory Eraser")
            .card_types(vec![CardType::Artifact])
            .build();
        let mut entrant = Object::from_card(entrant_id, &entrant_card, alice, Zone::Hand);
        entrant.abilities_mut().push(Ability::static_ability(
            StaticAbility::discard_hand_as_enters("As this enters, discard your hand.".into()),
        ));
        game.add_object(entrant);

        let discarded_id = game.new_object_id();
        let discarded_card =
            CardBuilder::new(CardId::from_raw(discarded_id.0 as u32), "Forgotten Card").build();
        game.add_object(Object::from_card(
            discarded_id,
            &discarded_card,
            alice,
            Zone::Hand,
        ));
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let entered = game
            .move_object_with_etb_processing_with_dm(entrant_id, Zone::Battlefield, &mut dm).expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions().expect("the permanent should enter after discarding the rest of the hand");
        let discarded_new_id = game
            .player(alice)
            .and_then(|player| player.graveyard.first().copied())
            .expect("the other hand card should be discarded");
        let zone_change_objects = game
            .take_pending_trigger_events()
            .into_iter()
            .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
            .filter_map(|event| event.object_id())
            .collect::<Vec<_>>();

        assert_eq!(zone_change_objects, vec![discarded_new_id, entered.new_id]);
    }

    struct RevealFromHandDm {
        desired: ObjectId,
        choice_source_zone: Option<Zone>,
        view_source_zones: Vec<Option<Zone>>,
        viewed_cards: Vec<Vec<ObjectId>>,
    }

    impl DecisionMaker for RevealFromHandDm {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.choice_source_zone = ctx
                .source
                .and_then(|source| game.object(source).map(|object| object.zone));
            vec![self.desired]
        }

        fn view_cards(
            &mut self,
            game: &GameState,
            _viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.view_source_zones.push(
                ctx.source
                    .and_then(|source| game.object(source).map(|object| object.zone)),
            );
            self.viewed_cards.push(cards.to_vec());
        }
    }

    #[test]
    fn reveal_from_hand_as_enters_is_selected_and_published_before_commit() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let entrant_id = game.new_object_id();
        let entrant_card =
            CardBuilder::new(CardId::from_raw(entrant_id.0 as u32), "Revealing Reliquary")
                .card_types(vec![CardType::Artifact])
                .build();
        let mut entrant = Object::from_card(entrant_id, &entrant_card, alice, Zone::Hand);
        entrant.abilities_mut().push(Ability::static_ability(
            StaticAbility::reveal_from_hand_as_enters(
                ObjectFilter::creature().in_zone(Zone::Hand),
                crate::ChoiceCount::any_number(),
                true,
                "As this enters, you may reveal any number of creature cards from your hand."
                    .into(),
            ),
        ));
        game.add_object(entrant);

        let creature_id = game.new_object_id();
        let creature_card =
            CardBuilder::new(CardId::from_raw(creature_id.0 as u32), "Revealed Bear")
                .card_types(vec![CardType::Creature])
                .build();
        game.add_object(Object::from_card(
            creature_id,
            &creature_card,
            alice,
            Zone::Hand,
        ));
        let mut dm = RevealFromHandDm {
            desired: creature_id,
            choice_source_zone: None,
            view_source_zones: Vec::new(),
            viewed_cards: Vec::new(),
        };

        game.move_object_with_etb_processing_with_dm(entrant_id, Zone::Battlefield, &mut dm).expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions().expect("the revealing permanent should enter");

        assert_eq!(dm.choice_source_zone, Some(Zone::Hand));
        assert_eq!(
            dm.view_source_zones,
            vec![Some(Zone::Hand), Some(Zone::Hand)]
        );
        assert_eq!(dm.viewed_cards, vec![vec![creature_id], vec![creature_id]]);
    }

    #[test]
    fn nonspell_aura_entry_uses_the_entering_controller_to_choose_attachment() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let _alice_creature = create_creature(&mut game, "Alice Bear", alice);
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);
        let aura = create_hand_aura(
            &mut game,
            "Borrowed Blessing",
            alice,
            ObjectFilter::creature().you_control(),
        );
        let mut dm = ChooseObjectDm {
            desired: bob_creature,
            chooser: None,
        };

        let outcome = {
            let mut ctx = ExecutionContext::new(ObjectId::from_raw(9000), bob, &mut dm);
            move_to_battlefield_with_options(
                &mut game,
                &mut ctx,
                aura,
                BattlefieldEntryOptions::specific(bob, false),
            ).expect("replacement operation must execute successfully in this scenario")
        };

        let BattlefieldEntryOutcome::Moved(new_id) = outcome.as_ref().expect("completed Aura entry receipt").assert_without_additions().clone() else {
            panic!("Aura should enter attached");
        };
        assert_eq!(dm.chooser, Some(bob));
        assert_eq!(game.current_controller(new_id), Some(bob));
        assert_eq!(
            game.object(new_id).and_then(|object| object.attached_to),
            Some(AttachmentTarget::Object(bob_creature))
        );
    }

    #[test]
    fn nonspell_aura_with_no_legal_attachment_remains_in_its_current_zone() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let aura = create_hand_aura(
            &mut game,
            "Lonely Blessing",
            alice,
            ObjectFilter::creature(),
        );
        let mut ctx = ExecutionContext::new_default(ObjectId::from_raw(9001), alice);

        let outcome = move_to_battlefield_with_options(
            &mut game,
            &mut ctx,
            aura,
            BattlefieldEntryOptions::specific(alice, false),
        ).expect("replacement operation must execute successfully in this scenario");

        assert_eq!(outcome.as_ref().expect("completed prevented entry receipt").assert_without_additions(), &BattlefieldEntryOutcome::Prevented);
        assert_eq!(
            game.object(aura).map(|object| object.zone),
            Some(Zone::Hand)
        );
        assert!(
            game.player(alice)
                .is_some_and(|player| player.hand.contains(&aura))
        );
        assert!(game.battlefield.iter().all(|id| *id != aura));
        assert!(
            game.player(alice)
                .is_some_and(|player| player.graveyard.is_empty())
        );
    }

    #[test]
    fn simultaneous_entrants_are_reserved_from_as_enters_zone_change_choices() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let mox_id = game.new_object_id();
        let mox = CardBuilder::new(CardId::from_raw(mox_id.0 as u32), "Batch Mox")
            .card_types(vec![CardType::Artifact])
            .build();
        let mut mox = Object::from_card(mox_id, &mox, alice, Zone::Hand);
        mox.abilities_mut().push(Ability::static_ability(
            StaticAbility::discard_or_redirect_replacement(
                ObjectFilter::land().in_zone(Zone::Hand),
                Zone::Graveyard,
            ),
        ));
        game.add_object(mox);

        let land_id = game.new_object_id();
        let land = CardBuilder::new(CardId::from_raw(land_id.0 as u32), "Entering Land")
            .card_types(vec![CardType::Land])
            .build();
        game.add_object(Object::from_card(land_id, &land, alice, Zone::Hand));

        let mut ctx = ExecutionContext::new_default(ObjectId::from_raw(9002), alice);
        let outcomes = move_to_battlefield_batch_with_options(
            &mut game,
            &mut ctx,
            vec![
                (mox_id, BattlefieldEntryOptions::preserve(false)),
                (land_id, BattlefieldEntryOptions::preserve(false)),
            ],
        ).expect("replacement operation must execute successfully in this scenario");

        assert!(matches!(outcomes[0].assert_without_additions(), BattlefieldEntryOutcome::Redirected(receipt) if receipt.final_zone == Zone::Graveyard));
        assert!(matches!(outcomes[1].assert_without_additions(), BattlefieldEntryOutcome::Moved(_)));
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Entering Land")
        }));
        assert!(game.player(alice).is_some_and(|player| {
            player.graveyard.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name == "Batch Mox")
            })
        }));
    }

    #[test]
    fn multiplayer_800_4b_keeps_object_in_zone_when_entering_controller_left() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card_id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(90_003), "Stranded Permanent")
            .card_types(vec![CardType::Artifact])
            .build();
        game.add_object(Object::from_card(card_id, &card, bob, Zone::Graveyard));
        game.player_mut(alice).expect("Alice").has_left_game = true;
        let mut ctx = ExecutionContext::new_default(ObjectId::from_raw(9003), bob);

        let outcome = move_to_battlefield_with_options(
            &mut game,
            &mut ctx,
            card_id,
            BattlefieldEntryOptions::specific(alice, false),
        ).expect("replacement operation must execute successfully in this scenario");

        assert_eq!(outcome.as_ref().expect("completed prevented entry receipt").assert_without_additions(), &BattlefieldEntryOutcome::Prevented);
        assert_eq!(
            game.object(card_id).map(|object| object.zone),
            Some(Zone::Graveyard)
        );
    }

    struct ReverseOrderDm {
        prompts: Vec<String>,
    }

    impl DecisionMaker for ReverseOrderDm {
        fn decide_order(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::OrderContext,
        ) -> Vec<ObjectId> {
            self.prompts.push(ctx.description.clone());
            ctx.items.iter().rev().map(|(id, _)| *id).collect()
        }
    }

    fn create_hand_creature_with_static_effect(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        let mut object = Object::from_card(id, &card, owner, Zone::Hand);
        object
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::make_colorless(
                ObjectFilter::source(),
            )));
        game.add_object(object);
        id
    }

    fn entry_timestamp(game: &GameState, id: ObjectId) -> u64 {
        game.effect_store
            .continuous_effects
            .get_entry_timestamp(id)
            .expect("battlefield permanent should have an entry timestamp")
    }

    #[test]
    fn simultaneous_entrants_receive_timestamps_in_the_active_players_order() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let first = create_hand_creature_with_static_effect(&mut game, "First Drone", alice);
        let second = create_hand_creature_with_static_effect(&mut game, "Second Drone", alice);
        let mut dm = ReverseOrderDm {
            prompts: Vec::new(),
        };
        let mut ctx = ExecutionContext::new(ObjectId::from_raw(9_041), alice, &mut dm);

        let outcomes = move_to_battlefield_batch_with_options(
            &mut game,
            &mut ctx,
            vec![
                (first, BattlefieldEntryOptions::preserve(false)),
                (second, BattlefieldEntryOptions::preserve(false)),
            ],
        ).expect("replacement operation must execute successfully in this scenario");
        let ids: Vec<ObjectId> = outcomes
            .iter()
            .map(|outcome| match outcome.assert_without_additions() {
                BattlefieldEntryOutcome::Moved(id) => *id,
                BattlefieldEntryOutcome::Redirected(_) | BattlefieldEntryOutcome::Prevented => panic!("both permanents should enter"),
            })
            .collect();

        assert_eq!(
            dm.prompts.len(),
            1,
            "the active player is asked once per batch"
        );
        // The player put the second entrant first, so it holds the older timestamp.
        assert!(
            entry_timestamp(&game, ids[1]) < entry_timestamp(&game, ids[0]),
            "CR 613.7j: the chosen order decides the relative timestamps"
        );
    }

    #[test]
    fn simultaneous_entry_without_two_relevant_entrants_asks_nothing() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let relevant = create_hand_creature_with_static_effect(&mut game, "Lone Drone", alice);
        let plain_id = game.new_object_id();
        let plain_card = CardBuilder::new(CardId::from_raw(plain_id.0 as u32), "Plain Bear")
            .card_types(vec![CardType::Creature])
            .build();
        game.add_object(Object::from_card(plain_id, &plain_card, alice, Zone::Hand));
        let mut dm = ReverseOrderDm {
            prompts: Vec::new(),
        };
        let mut ctx = ExecutionContext::new(ObjectId::from_raw(9_042), alice, &mut dm);

        let outcomes = move_to_battlefield_batch_with_options(
            &mut game,
            &mut ctx,
            vec![
                (relevant, BattlefieldEntryOptions::preserve(false)),
                (plain_id, BattlefieldEntryOptions::preserve(false)),
            ],
        ).expect("replacement operation must execute successfully in this scenario");
        assert!(
            outcomes
                .iter()
                .all(|outcome| matches!(outcome.assert_without_additions(), BattlefieldEntryOutcome::Moved(_)))
        );
        assert!(dm.prompts.is_empty());
    }

    #[test]
    fn pending_batch_entry_stops_before_later_prompts_and_rolls_back_payments() {
        struct Answers { calls: usize, pause_second: bool, pending: bool }
        impl DecisionMaker for Answers {
            fn decide_boolean(&mut self, _: &GameState,
                _: &crate::decisions::context::BooleanContext) -> bool {
                self.calls += 1;
                self.pending = self.pause_second && self.calls == 2;
                !self.pending
            }
            fn awaiting_choice(&self) -> bool { self.pending }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Batch land")
            .card_types(vec![crate::types::CardType::Land]).build();
        let mut requests = Vec::new();
        let mut shields = Vec::new();
        for _ in 0..3 {
            let object = game.create_object_from_card(&card, alice, Zone::Hand);
            shields.push(game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(object, alice,
                    crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                        crate::target::ObjectFilter::specific(object)),
                    crate::replacement::ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost: 2 }),
            ));
            requests.push((object, BattlefieldEntryOptions::preserve(false)));
        }
        let mut dm = Answers { calls: 0, pause_second: true, pending: false };
        let mut ctx = ExecutionContext::new(requests[0].0, alice, &mut dm);
        let pending = move_to_battlefield_batch_with_options(&mut game, &mut ctx, requests.clone()).expect("replacement operation must execute successfully in this scenario");
        drop(ctx);
        assert_eq!(dm.calls, 2, "a pending prompt must stop the batch immediately");
        assert!(pending.iter().all(|result| matches!(result.assert_without_additions(), BattlefieldEntryOutcome::Prevented)));
        assert_eq!(game.player(alice).unwrap().life, 20);
        for (object, _) in &requests { assert_eq!(game.object(*object).unwrap().zone, Zone::Hand); }
        for effect in &shields { assert!(game.effect_store.replacement_effects.get_effect(*effect).is_some()); }
        assert!(game.take_pending_trigger_events().is_empty());
        dm.calls = 0;
        dm.pause_second = false;
        dm.pending = false;
        let mut ctx = ExecutionContext::new(requests[0].0, alice, &mut dm);
        let completed = move_to_battlefield_batch_with_options(&mut game, &mut ctx, requests).expect("replacement operation must execute successfully in this scenario");
        assert!(completed.iter().all(|result| matches!(result.assert_without_additions(), BattlefieldEntryOutcome::Moved(_))));
        assert_eq!(game.player(alice).unwrap().life, 14);
        for effect in &shields { assert!(game.effect_store.replacement_effects.get_effect(*effect).is_none()); }
    }

}

#[cfg(test)]
#[path = "entry_tapped_tests.rs"]
mod entry_tapped_tests;

#[cfg(test)]
mod authored_attachment_entry_contract_tests {
    use super::*;
    use crate::ability::Ability;
    use crate::ids::{CardId,PlayerId};
    use crate::object::{AttachmentTarget,AuraAttachmentFilter};
    use crate::static_abilities::StaticAbility;
    use crate::types::{CardType,Subtype};
    fn definitions() -> (crate::cards::CardDefinition,crate::cards::CardDefinition) {
        let aura=crate::CardDefinitionBuilder::new(CardId::new(),"Requested aura").card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura])
            .with_ability(Ability::static_ability(StaticAbility::enchant(AuraAttachmentFilter::Object(crate::target::ObjectFilter::creature())))).build();
        let source=crate::CardDefinitionBuilder::new(CardId::new(),"Entry owner").card_types(vec![CardType::Artifact]).build();
        (aura,source)
    }
    #[test]
    fn illegal_authored_aura_attachment_does_not_commit_entry() {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let alice=PlayerId::from_index(0);let (aura,source)=definitions();
        let parent=game.create_object_from_definition(&source,alice,Zone::Battlefield);
        let target=game.create_object_from_definition(&source,alice,Zone::Battlefield);
        let original=game.create_object_from_definition(&aura,alice,Zone::Graveyard);
        game.take_pending_trigger_events();let ids=game.next_object_id_counter();let objects=game.objects_in_deterministic_order().len();
        let mut ctx=ExecutionContext::new_default(parent,alice);
        let _receipt=move_to_battlefield_with_options(&mut game,&mut ctx,original,BattlefieldEntryOptions::preserve(false).with_entry_attachment(Some(AttachmentTarget::Object(target)))).unwrap();
        assert!(game.object(original).is_some_and(|object|object.zone==Zone::Graveyard),"an Aura with an illegal specified attachment stays in its original zone");
        assert_eq!(game.next_object_id_counter(),ids);assert_eq!(game.objects_in_deterministic_order().len(),objects);
        assert_eq!(game.battlefield.len(),2);assert!(game.object(target).unwrap().attachments.is_empty());
        assert!(game.take_pending_trigger_events().is_empty(),"a rejected entry publishes no completed movement or ETB");
    }
    #[test]
    fn legal_authored_aura_attachment_is_present_when_entry_is_published() {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let alice=PlayerId::from_index(0);let (aura,source)=definitions();
        let parent=game.create_object_from_definition(&source,alice,Zone::Battlefield);
        let creature=crate::CardDefinitionBuilder::new(CardId::new(),"Attachment target").card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2,2)).build();
        let target=game.create_object_from_definition(&creature,alice,Zone::Battlefield);
        let original=game.create_object_from_definition(&aura,alice,Zone::Graveyard);game.take_pending_trigger_events();
        let mut ctx=ExecutionContext::new_default(parent,alice);
        let _receipt=move_to_battlefield_with_options(&mut game,&mut ctx,original,BattlefieldEntryOptions::preserve(false).with_entry_attachment(Some(AttachmentTarget::Object(target)))).unwrap();
        assert!(game.object(original).is_none());let attached=game.object(target).unwrap().attachments.clone();assert_eq!(attached.len(),1);
        assert_eq!(game.object(attached[0]).unwrap().attached_to,Some(AttachmentTarget::Object(target)));
        let events=game.take_pending_trigger_events();assert_eq!(events.iter().filter(|event|event.kind()==crate::events::EventKind::EnterBattlefield).count(),1);
    }
}

#[cfg(test)]
mod replacement_invalid_entry_answer_contract_tests {
    use super::*;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    struct Answer(Vec<usize>);
    impl crate::decision::DecisionMaker for Answer {
        fn decide_options(&mut self, _: &GameState, _: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> { self.0.clone() }
    }
    fn invalid(indices: Vec<usize>) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20); let alice = PlayerId::from_index(0);
        let source = game.create_object_from_card(&crate::card::CardBuilder::new(crate::ids::CardId::new(), "Entry choice source").card_types(vec![crate::types::CardType::Enchantment]).build(), alice, Zone::Battlefield);
        let entrant = game.create_object_from_card(&crate::card::CardBuilder::new(crate::ids::CardId::new(), "Entry choice land").card_types(vec![crate::types::CardType::Land]).build(), alice, Zone::Hand);
        let mut shields = Vec::new(); for action in [ReplacementAction::EnterTapped, ReplacementAction::EnterUntapped] {
            shields.push(game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, alice, crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(crate::target::ObjectFilter::specific(entrant)), action)));
        }
        game.take_pending_trigger_events(); let ids = game.next_object_id_counter(); let mut dm = Answer(indices);
        let result = game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut dm);
        assert!(matches!(result, Err(crate::effects::ExecutionError::InternalError(_))), "entry must not treat malformed replacement input as an authorized default");
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand); assert_eq!(game.next_object_id_counter(), ids);
        for shield in shields { assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); } assert!(game.take_pending_trigger_events().is_empty());
    }
    #[test] fn entry_empty_answer_is_invalid() { invalid(vec![]); }
    #[test] fn entry_out_of_range_answer_is_invalid() { invalid(vec![usize::MAX]); }
    #[test] fn entry_multiple_answers_are_invalid() { invalid(vec![0, 1]); }
}

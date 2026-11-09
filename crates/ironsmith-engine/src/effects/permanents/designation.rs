//! Permanent designations, independent of the actions that precede them.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::triggers::TriggerEvent;

#[derive(Debug, Clone, Copy)]
pub(super) enum PermanentDesignation {
    Monstrous,
    Renowned,
}

impl PermanentDesignation {
    pub(super) fn is_present(self, game: &GameState, object: ObjectId) -> bool {
        match self {
            Self::Monstrous => game.is_monstrous(object),
            Self::Renowned => game.is_renowned(object),
        }
    }
}

#[derive(Debug, Clone)]
struct ApplyDesignation {
    object: ObjectId,
    designation: PermanentDesignation,
    amount: u32,
}

impl EffectExecutor for ApplyDesignation {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            if ctx.decision_maker.awaiting_choice()
                || !game
                    .object(self.object)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                || game.is_phased_out(self.object)
                || self.designation.is_present(game, self.object)
            {
                return Ok(EffectOutcome::count(0));
            }
            let (label, outcome) = match self.designation {
                PermanentDesignation::Monstrous => {
                    game.set_monstrous(self.object);
                    let completion = crate::effects::observe_action_completion(
                        game,
                        TriggerEvent::new_with_provenance(
                            crate::events::BecameMonstrousEvent::new(
                                self.object,
                                ctx.controller,
                                self.amount,
                            ),
                            ctx.provenance,
                        ),
                        Some(ctx.provenance),
                    )?;
                    (
                        "monstrous",
                        EffectOutcome::monstrosity_applied(self.object, self.amount)
                            .with_event(completion),
                    )
                }
                PermanentDesignation::Renowned => {
                    game.set_renowned(self.object);
                    (
                        "renown",
                        crate::effects::composition::complete_keyword_action_with_result(
                            game,
                            ctx,
                            EffectOutcome::count(1),
                            KeywordActionEvent::new(
                                KeywordActionKind::Renown,
                                ctx.controller,
                                self.object,
                                self.amount,
                            ),
                        )?,
                    )
                }
            };
            if let Some(stable_id) = game.object(self.object).map(|object| object.stable_id) {
                game.record_ui_effect_event(
                    "level_up",
                    Some(ctx.controller),
                    None,
                    vec![stable_id],
                    Some(i64::from(self.amount)),
                    Some(label.into()),
                );
            }
            Ok(outcome)
        })
    }
}

pub(super) fn apply_designation(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    object: ObjectId,
    designation: PermanentDesignation,
    amount: u32,
) -> Result<EffectOutcome, ExecutionError> {
    ApplyDesignation {
        object,
        designation,
        amount,
    }
    .execute_child(game, ctx)
}

/// Counter placement followed by a one-time designation. The requested amount
/// is frozen before replacements run; a departed incarnation cannot acquire the
/// designation when counter replacement finishes.
#[derive(Debug, Clone)]
pub(super) struct DesignationProgram {
    pub designation: PermanentDesignation,
    pub amount: crate::effect::Value,
    pub missing_source_is_invalid: bool,
}

impl EffectExecutor for DesignationProgram {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Leaf
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        visitor(&crate::effect::Effect::new(
            crate::effects::PutCountersEffect::on_source(
                crate::object::CounterType::PlusOnePlusOne,
                self.amount.clone(),
            ),
        ));
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
        use crate::effects::CompletedEffectOutputs;
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let source = ctx.source;
                if self.missing_source_is_invalid && game.object(source).is_none() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                }
                if !game
                    .object(source)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                    || game.is_phased_out(source)
                    || self.designation.is_present(game, source)
                {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let amount =
                    crate::effects::helpers::resolve_nonnegative_u32(game, &self.amount, ctx)?;
                let placement = crate::effects::PutCountersEffect::on_source(
                    crate::object::CounterType::PlusOnePlusOne,
                    amount,
                )
                .execute_child_with_outputs(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let designation = apply_designation(game, ctx, source, self.designation, amount)?;
                let outcome = EffectOutcome::aggregate_with_primary_result(
                    designation.summary_projection(),
                    [placement.outcome.clone(), designation.clone()],
                );
                let mut outputs = placement;
                outputs
                    .retain_batch_children([CompletedEffectOutputs::aggregate_only(designation)]);
                Ok(outputs.project_aggregate(outcome))
            },
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
    }
}

impl EffectExecutor for ironsmith_core::SetPermanentDesignationEffect {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            let targets = crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)?;
            let mut changed = Vec::new();
            let mut events = Vec::new();
            for id in targets {
                if ctx.replacement.entry_counter_source == Some(id) {
                    // Entry programs apply permanent facts to the resulting
                    // incarnation after its zone change commits. Source-zone
                    // cards cannot acquire battlefield designations.
                    ctx.replacement.entry_designations.push(self.designation.clone());
                    changed.push(id);
                    continue;
                }
                if game.is_phased_out(id) { continue; }
                match &self.designation {
                    ironsmith_core::PermanentDesignation::Harnessed => if game.harness(id) { changed.push(id); },
                    ironsmith_core::PermanentDesignation::CustomStatusInZone { name, zone, present, carry_to_stack } => {
                        if game.set_custom_status_in_zone_with_cast_continuation(id, name, *zone, *present, *carry_to_stack) { changed.push(id); }
                    },
                    ironsmith_core::PermanentDesignation::CustomStatus { name, .. }
                    | ironsmith_core::PermanentDesignation::CustomStatusUntilEvent { name, .. } => {
                        let fresh = match &self.designation {
                            ironsmith_core::PermanentDesignation::CustomStatus { until_end_of_turn, .. } => game.apply_custom_status(id, name, *until_end_of_turn),
                            ironsmith_core::PermanentDesignation::CustomStatusUntilEvent { event, .. } => game.apply_custom_status_until_event(id, name, *event),
                            _ => unreachable!(),
                        };
                        if fresh {
                            changed.push(id);
                            let controller = game.current_controller(id).unwrap_or(ctx.controller);
                            let mut event = KeywordActionEvent::new(KeywordActionKind::CustomStatusGained, controller, id, 1);
                            event.custom_status = Some(name.clone());
                            event.snapshot = game.object(id).map(|object| crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)).transpose()?;
                            events.push(crate::effects::observe_action_completion(game,
                                TriggerEvent::new_with_provenance(event, ctx.provenance), Some(ctx.provenance))?);
                        }
                    }
                }
            }
            let mut outcome = EffectOutcome::with_objects(changed);
            for event in events { outcome = outcome.with_event(event); }
            Ok(outcome)
        })
    }
    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> { Some(&self.target) }
}

impl EffectExecutor for ironsmith_core::RecordSourceCollectionEffect {
    fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
        let sources=match &self.source_tag {Some(tag)=>ctx.get_tagged_all(tag.as_str()).into_iter().flatten().map(|snapshot|snapshot.object_id).collect::<Vec<_>>(),None=>vec![ctx.source]};
        // A bookkeeping relation records the exact selected incarnation.
        // Ordinary effect reference resolution may follow a tagged card across
        // zones; that would incorrectly attach a later reincarnation here.
        let targets=if !self.target.is_target() {
            if let crate::target::ChooseSpec::Tagged(tag)=self.target.base() {
                ctx.get_tagged_all(tag.as_str()).into_iter().flatten().map(|snapshot|snapshot.object_id).collect()
            } else {crate::effects::helpers::resolve_objects_for_effect(game,ctx,&self.target)?}
        } else {crate::effects::helpers::resolve_objects_for_effect(game,ctx,&self.target)?};
        let mut changed=Vec::new();
        for source in sources {
            let members=targets.iter().copied().filter(|id|game.object(*id).is_some_and(|object|object.zone==self.member_zone && game.find_object_by_stable_id(object.stable_id)==Some(*id))).collect::<Vec<_>>();
            if game.note_source_collection(source,self.source_zone,&self.name,self.member_zone,&members) {changed.extend(members);}
        }
        changed.sort();changed.dedup();Ok(EffectOutcome::with_objects(changed))
    }
    fn get_target_spec(&self)->Option<&crate::target::ChooseSpec>{Some(&self.target)}
}

impl EffectExecutor for ironsmith_core::GrantExileControllerInspectionEffect {
    fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
        let cards=crate::effects::helpers::resolve_objects_for_effect(game,ctx,&self.target)?;
        let sources=crate::effects::helpers::resolve_objects_for_effect(game,ctx,&self.sources)?;
        let mut count=0;
        for card in cards {
            if !game.object(card).is_some_and(|object|object.zone==crate::zone::Zone::Exile){continue;}
            for source in &sources {
                game.grant_face_down_exile_source_controller_view(card,*source);count+=1;
                // The created inspector is the card's linked source: its
                // "cards exiled with this" abilities read this card (CR 607.2a).
                if !game.get_exiled_with_source_links(*source).contains(&card) {
                    game.add_exiled_with_source_link(*source,card);
                }
            }
        }
        Ok(EffectOutcome::count(count))
    }
    fn get_target_spec(&self)->Option<&crate::target::ChooseSpec>{Some(&self.target)}
}

impl EffectExecutor for ironsmith_core::effect::RememberOriginalMovementArrivalsEffect {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        let outcome = ctx.get_outcome(self.from).ok_or_else(|| ExecutionError::IncompleteEvidence("remembered movement has no exact producer receipt".into()))?;
        let mut known = false;
        let mut snapshots = Vec::new();
        for fact in &outcome.instruction_result().execution_facts {
            if let crate::effect::ExecutionFact::OriginalZoneMoveObjects(moves) = fact {
                known = true;
                snapshots.extend(moves.iter().filter(|(_, _, arrival)| arrival.zone == self.zone).map(|(_, _, arrival)| arrival.clone()));
            }
            if let crate::effect::ExecutionFact::OriginalZoneMoveCards(cards) = fact {
                known = true;
                snapshots.extend(cards.iter().filter(|arrival| arrival.zone == self.zone).cloned());
            }
        }
        if !known { return Err(ExecutionError::IncompleteEvidence("remembered movement lacks original arrival evidence".into())); }
        let mut seen = std::collections::HashSet::new();
        snapshots.retain(|snapshot| seen.insert(snapshot.object_id) && game.object(snapshot.object_id).is_some_and(|object| object.zone == self.zone && object.stable_id == snapshot.stable_id && game.find_object_by_stable_id(snapshot.stable_id) == Some(snapshot.object_id)));
        let ids = snapshots.iter().map(|snapshot| snapshot.object_id).collect();
        ctx.set_tagged_objects(self.tag.clone(), snapshots);
        ctx.exact_object_reference_tags.insert(self.tag.clone());
        Ok(EffectOutcome::with_objects(ids))
    }
}

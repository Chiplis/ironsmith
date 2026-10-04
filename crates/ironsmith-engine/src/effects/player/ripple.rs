//! CR702.60: one optional reveal, an ordered sequence of real casts, and the
//! exact uncast remainder. Pending input or typed execution failure restores
//! the whole instruction, including queued cast triggers and library order.
use crate::decisions::context::{BooleanContext, SelectOptionsContext, SelectableOption};
use crate::effect::{Effect, EffectOutcome};
use crate::effects::consult_helpers::{execute_library_consult, LibraryConsultMode, LibraryConsultStopRule};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;
pub use ironsmith_core::RippleEffect;
use super::runtime_helpers::{effect_driven_cast_options_for_card, with_spell_cast_event};

impl EffectExecutor for RippleEffect {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_atomically(game,ctx,|game,ctx| {
            if !game.player(ctx.controller).is_some_and(|player|player.is_in_game()) {return Ok(EffectOutcome::resolved());}
            let reveal=BooleanContext::new(ctx.controller,Some(ctx.source),format!("Reveal up to {} cards for ripple?",self.amount));
            let yes=ctx.decision_maker.decide_boolean(game,&reveal);
            if ctx.decision_maker.awaiting_choice() || !yes {return Ok(EffectOutcome::count(0));}
            let all=TagKey::from("__ripple_revealed");
            let kept=TagKey::from("__ripple_cast");
            let consult=execute_library_consult(game,ctx,ctx.controller,LibraryConsultMode::Reveal,
                LibraryConsultStopRule::MatchCount(self.amount),Some(&all),None,|_,_|true)?;
            if ctx.decision_maker.awaiting_choice() {return Ok(EffectOutcome::count(0));}
            let exposed=consult.exposed_object_ids.clone();
            let mut reveal_outcome=consult.attach_to_outcome(EffectOutcome::resolved());
            crate::effects::capture_triggers_before_added_program(game,ctx,None,reveal_outcome.events.iter_mut())?;
            if exposed.is_empty() { return Ok(reveal_outcome); }
            let mut outcomes=vec![reveal_outcome];
            let mut failed_methods=Vec::new();
            let mut kept_snapshots=Vec::new();
            let mut cast_ids=Vec::new();
            loop {
                let checked=game.continuous_query_snapshot().map_err(ExecutionError::ContinuousDiscovery)?;
                let name=checked.current_name(ctx.source).or_else(||game.turn_store.turn_history.source_last_known_snapshot(ctx.source).map(|s|s.name.to_string()))
                    .or_else(||ctx.source_snapshot.as_ref().map(|s|s.name.to_string()))
                    .ok_or_else(||ExecutionError::UnresolvableValue("ripple requires its exact source name".into()))?;
                let filter=ObjectFilter::nonland();
                let mut options=Vec::new();
                for id in &exposed {
                    // A previously revealed card must still be that exact library
                    // incarnation. A later copy/blink is never a new candidate.
                    if checked.object(*id).is_some_and(|o|o.zone==Zone::Library&&o.owner==ctx.controller
                        && crate::filter::names_share(&checked.current_name(*id).unwrap_or_else(||o.name.to_string()),o.split_other_half_name(),&name,None)) {
                        // The revealed card qualifies before selecting its cast
                        // face. A split card matching one half may cast either
                        // half (Double Masters 2022 Thrumming Stone ruling).
                        options.extend(effect_driven_cast_options_for_card(&checked,ctx.controller,ctx.source,*id,Zone::Library,&filter));
                    }
                }
                options.retain(|option| !failed_methods.iter().any(|(id,method)|*id==option.object_id && *method==option.casting_method));
                if options.is_empty() {break;}
                let menu=options.iter().enumerate().map(|(i,o)|SelectableOption::new(i,o.label.clone())).collect();
                let prompt=SelectOptionsContext::new(ctx.controller,Some(ctx.source),"Cast a revealed card with ripple, or stop",menu,0,1);
                let selected=ctx.decision_maker.decide_options(game,&prompt);
                if ctx.decision_maker.awaiting_choice() {return Ok(EffectOutcome::count(0));}
                let index=match selected.as_slice() {
                    []=>break,
                    [index] if *index<options.len()=>*index,
                    _=>return Err(ExecutionError::Impossible("invalid ripple cast selection".into())),
                };
                let option=&options[index];
                failed_methods.push((option.object_id,option.casting_method.clone()));
                let snapshot=game.object(option.object_id).map(|object|crate::snapshot::ObjectSnapshot::from_object(object,game))
                    .ok_or(ExecutionError::ObjectNotFound(option.object_id))?;
                let result=match crate::game_loop::cast_spell_from_resolving_effect(
                    game,option.object_id,option.from_zone,ctx.controller,&option.casting_method,
                    true,None,ctx.provenance,&mut ctx.decision_maker,
                ) {
                    Ok(result)=>result,
                    Err(crate::game_loop::GameLoopError::ActionCancelled(_))=>None,
                    // In particular, an ExecutionFailed(Impossible) raised by
                    // a real payment/replacement program is NOT an unavailable
                    // cast. Preserve that failure instead of suppressing it.
                    Err(error)=>return Err(super::runtime_helpers::effect_driven_cast_error(error)),
                };
                if ctx.decision_maker.awaiting_choice() {return Ok(EffectOutcome::count(0));}
                if let Some(new_id)=result {
                    // A failed face does not exhaust another face's permission.
                    // A successful cast may also change the state via its costs,
                    // so previously unavailable proposals can be reconsidered.
                    failed_methods.clear();
                    kept_snapshots.push(snapshot);cast_ids.push(new_id);
                    let provenance=game.alloc_child_event_provenance(ctx.provenance,crate::events::EventKind::SpellCast);
                    let mut outcome=with_spell_cast_event(EffectOutcome::with_objects(vec![new_id]),game,new_id,ctx.controller,option.from_zone,provenance);
                    // Later cast costs can remove a watcher or alter the spell.
                    // Retain this completed cast's matches and history now.
                    crate::effects::capture_triggers_before_added_program(game,ctx,None,outcome.events.iter_mut())?;
                    outcomes.push(outcome);
                }
            }
            ctx.set_tagged_objects(kept.clone(),kept_snapshots);
            outcomes.push(crate::effects::execute_effect(game,&Effect::put_tagged_remainder_on_library_bottom(
                all,Some(kept),ironsmith_core::LibraryBottomOrder::ChooserChooses,PlayerFilter::You),ctx)?);
            if ctx.decision_maker.awaiting_choice() {return Ok(EffectOutcome::count(0));}
            let mut outcome=EffectOutcome::aggregate(outcomes);
            let result=EffectOutcome::with_objects(cast_ids);outcome.status=result.status;outcome.value=result.value;
            Ok(outcome)
        })
    }
}

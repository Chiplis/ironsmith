//! Note names using normal current/LKI object-reference timing.
use crate::{GameState,ObjectId};
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor,ExecutionContext,ExecutionError};
use crate::target::ChooseSpec;
use crate::snapshot::ObjectSnapshot;
pub use ironsmith_core::NoteObjectNamesEffect;

fn current_or_lki(game:&GameState,snapshot:&ObjectSnapshot)->Result<ObjectSnapshot,ExecutionError>{
    if let Some(object)=game.object(snapshot.object_id).filter(|object|object.zone==snapshot.zone && !game.is_phased_out(object.id)){
        return ObjectSnapshot::try_from_object_with_calculated_characteristics(object,game);
    }
    Ok(game.source_last_known_snapshot(snapshot.object_id).filter(|last|last.zone==snapshot.zone).unwrap_or(snapshot).clone())
}
impl EffectExecutor for NoteObjectNamesEffect {
    fn get_target_spec(&self)->Option<&ChooseSpec>{self.object.is_target().then_some(&self.object)}
    fn decision_related_object_specs(&self)->Vec<ChooseSpec>{vec![self.object.clone(),self.for_object.clone()]}
    fn scalar_target_resolution_specs(&self)->Vec<ChooseSpec>{vec![self.object.clone()]}
    fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
        let owners:Vec<ObjectId>=match self.for_object.base(){
            ChooseSpec::Source=>vec![ctx.source],
            ChooseSpec::SpecificObject(id)=>vec![*id],
            ChooseSpec::Tagged(tag)=>ctx.get_tagged_all(tag.as_str()).ok_or_else(||ExecutionError::IncompleteEvidence("name note has no bound memory owner".into()))?.iter().map(|snapshot|snapshot.object_id).collect(),
            _=>return Err(ExecutionError::IncompleteEvidence("name note owner must be an exact existing reference".into())),
        };
        let snapshots=match self.object.base(){
            ChooseSpec::Tagged(tag)=>{
                if tag.as_str()=="triggering" && let Some(event)=ctx.triggering_event.as_ref(){
                    vec![crate::condition_eval::capture_triggering_object_at_resolution(game,event)?]
                }else{
                let snapshots=ctx.get_tagged_all(tag.as_str()).cloned()
                    .ok_or_else(||ExecutionError::IncompleteEvidence("name note has no bound object evidence".into()))?;
                snapshots.iter().map(|snapshot|current_or_lki(game,snapshot)).collect::<Result<Vec<_>,_>>()?
                }
            },
            ChooseSpec::Source|ChooseSpec::SpecificObject(_)=>{
                let id=match self.object.base(){ChooseSpec::Source=>ctx.source,ChooseSpec::SpecificObject(id)=>*id,_=>unreachable!()};
                if let Some(object)=game.object(id).filter(|object|!game.is_phased_out(object.id)){
                    vec![ObjectSnapshot::try_from_object_with_calculated_characteristics(object,game)?]
                }else{
                    let retained=game.source_last_known_snapshot(id)
                        .or_else(||ctx.source_snapshot.as_ref().filter(|snapshot|snapshot.object_id==id))
                        .or_else(||ctx.target_snapshots.get(&id))
                        .ok_or_else(||ExecutionError::IncompleteEvidence("name note has no exact object identity".into()))?;
                    vec![retained.clone()]
                }
            },
            _=>{
                let ids=crate::effects::helpers::resolve_objects_for_effect(game,ctx,&self.object)?;
                if ctx.decision_maker.awaiting_choice(){return Ok(EffectOutcome::count(0));}
                ids.into_iter().map(|id|{
                    if let Some(object)=game.object(id){return ObjectSnapshot::try_from_object_with_calculated_characteristics(object,game);}
                    game.source_last_known_snapshot(id).cloned().ok_or_else(||ExecutionError::IncompleteEvidence("name note object has no current or retained identity".into()))
                }).collect::<Result<Vec<_>,_>>()?
            },
        };
        let mut names = std::collections::BTreeSet::new();
        for snapshot in &snapshots {
            names.extend(crate::filter::ObjectNames::snapshot(snapshot).literals().map(str::to_owned));
        }
        if snapshots.iter().any(|snapshot| snapshot.all_nonlegendary_creature_names) {
            names.extend(crate::filter::all_nonlegendary_creature_card_names(game).map(str::to_owned));
        }
        let owners=owners.into_iter().map(|source|crate::source_numbers::resolve_owner(game,source,self.pair,
            ctx.source_number_owner.as_ref(),ctx.source_snapshot.as_ref().filter(|snapshot|snapshot.object_id==source)))
            .collect::<Result<Vec<_>,_>>()?;
        let mut count=0;
        for owner in owners {
            count += game.note_card_names_for_owner(owner, names.iter().map(String::as_str));
        }
        Ok(EffectOutcome::count(i32::try_from(count).unwrap_or(i32::MAX)))
    }
}

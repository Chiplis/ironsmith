//! Unregistered proof obligations for unapplied contextual-duration plumbing.
use ironsmith::{Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
fn card(game:&mut GameState,name:&str,body:&str,zone:Zone)->ObjectId {
    let definition=ironsmith_compiler_runtime::compile_to_runtime_definition(name,body,false).unwrap();
    game.create_object_from_definition(&definition,A,zone)
}
fn moved(game:&mut GameState,object:ObjectId,zone:Zone)->ObjectId {
    let stable=game.object(object).unwrap().stable_id;
    execute_effect(game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ChooseSpec::SpecificObject(object),zone,false)),&mut EffectContext::new_default(object,A)).unwrap();
    game.find_object_by_stable_id(stable).unwrap()
}
#[test]
fn source_zone_reference_materializes_only_current_source_or_validated_own_exile_arrival() {
    use ironsmith_core::{ContinuousDurationObject as Object,ContinuousDurationPredicate as Predicate};
    for mode in 0..6 {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=card(&mut game,"Same printed name","Type: Sorcery",if mode==0 {Zone::Exile}else{Zone::Hand});
        let host=card(&mut game,"Recipient","Type: Creature\nPower/Toughness: 2/2",Zone::Battlefield);
        let other=card(&mut game,"Same printed name","Type: Sorcery",Zone::Exile);
        let mut ctx=EffectContext::new_default(source,A);
        ctx.source_snapshot=Some(ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(),&game));
        if mode==1 || mode==3 || mode==5 {
            let spec=if mode==5 {
                ctx.set_tagged_objects("selected_exact_source",vec![ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(),&game)]);
                ChooseSpec::tagged("selected_exact_source")
            } else {ChooseSpec::Source};
            execute_effect(&mut game,&Effect::new(ironsmith::effects::ExileEffect::with_spec(spec)),&mut ctx).unwrap();
            if mode==5 {assert_eq!(ctx.source,source,"publishing tagged self-arrival does not rebase context source");}
            assert!(ctx.get_tagged_all(ironsmith_core::tag::SOURCE_EXILED_SELF_TAG).is_some());
        }
        if mode==2 {
            ctx.set_tagged_objects(ironsmith_core::tag::SOURCE_EXILED_SELF_TAG,vec![ironsmith::snapshot::ObjectSnapshot::from_object(game.object(other).unwrap(),&game)]);
        }
        if mode==3 {
            let old=ctx.get_tagged_all(ironsmith_core::tag::SOURCE_EXILED_SELF_TAG).unwrap()[0].object_id;
            let hand=moved(&mut game,old,Zone::Hand);moved(&mut game,hand,Zone::Exile);
        }
        // 0 already exiled;1 exact own arrival;2 unrelated same-name arrival;
        // 3 stale arrival left/reentered;4 no arrival;5 tagged exact self.
        let until=ironsmith::effect::Until::ForAsLongAs(Predicate::all([
            Predicate::ObjectInZone{object:Object::SourceInZone(Zone::Exile),zone:Zone::Exile},
            Predicate::ObjectInZone{object:Object::Specific(host),zone:Zone::Battlefield},
        ]));
        let encoded=serde_json::to_string(&until).unwrap();
        let decoded:ironsmith::effect::Until=serde_json::from_str(&encoded).unwrap();assert_eq!(decoded,until);
        let grant=ironsmith::effects::ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(host),ironsmith::continuous::Modification::AddSubtypes(vec![ironsmith::types::Subtype::Elf]),decoded);
        execute_effect(&mut game,&Effect::new(grant),&mut ctx).unwrap();
        assert_eq!(game.current_has_subtype(host,ironsmith::types::Subtype::Elf),mode<2||mode==5,"mode{mode}");
        if mode<2 || mode==5 {
            let bound=if mode==0 {source}else{ctx.get_tagged_all(ironsmith_core::tag::SOURCE_EXILED_SELF_TAG).unwrap()[0].object_id};
            let hand=moved(&mut game,bound,Zone::Hand);assert!(!game.current_has_subtype(host,ironsmith::types::Subtype::Elf));
            moved(&mut game,hand,Zone::Exile);assert!(!game.current_has_subtype(host,ironsmith::types::Subtype::Elf),"bound Specific incarnation never revives");
        }
    }
}
struct ChooseReferenced(ObjectId);
impl DecisionMaker for ChooseReferenced {
    fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId> {
        if ctx.candidates.iter().any(|candidate|candidate.id==self.0&&candidate.legal) {vec![self.0]}else{SelectFirstDecisionMaker.decide_objects(game,ctx)}
    }
}
#[test]
fn prior_chosen_creature_duration_never_rebinds_to_distinct_grant_recipient() {
    let text="Type: Sorcery\nChoose a creature you control. Target artifact creature gains \"Flying\" for as long as that creature remains on the battlefield.";
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Separate referenced object and recipient",text,false)));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();
    let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
    for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {for leave_reference in [false,true] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Stack);
        let referenced=card(&mut game,"Chosen creature","Type: Creature\nPower/Toughness: 2/2",Zone::Battlefield);
        let recipient=card(&mut game,"Distinct recipient","Type: Artifact Creature\nPower/Toughness: 2/2",Zone::Battlefield);
        let mut dm=ChooseReferenced(referenced);
        let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
        execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(recipient)]).with_decision_maker(&mut dm)).unwrap();
        assert!(game.current_has_static_ability_id(recipient,ironsmith_core::StaticAbilityId::Flying));
        assert!(!game.current_has_static_ability_id(referenced,ironsmith_core::StaticAbilityId::Flying));
        let moved_id=if leave_reference {referenced}else{recipient};
        let hand=moved(&mut game,moved_id,Zone::Hand);
        if leave_reference {
            assert_eq!(game.object(recipient).unwrap().zone,Zone::Battlefield);
            assert!(!game.current_has_static_ability_id(recipient,ironsmith_core::StaticAbilityId::Flying),"referenced creature departure ends grant on other recipient");
            moved(&mut game,hand,Zone::Battlefield);
            assert!(!game.current_has_static_ability_id(recipient,ironsmith_core::StaticAbilityId::Flying));
        }
    }}
}

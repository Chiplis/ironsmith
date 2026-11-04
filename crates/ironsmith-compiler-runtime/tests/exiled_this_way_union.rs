//! Source-authored and deliberately unrun (cf8 p04): Crabomination's one
//! instruction exiles three groups (library top, random graveyard card,
//! random hand card); "cards exiled this way" names all three, so the free
//! cast chooses among the union, not only the last exile.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/exiled_this_way_union.json.fixture"
    ))
    .unwrap();
    let row = &rows[0];
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct.unwrap(), materialize_artifact(&restored).unwrap()]
}

#[test]
fn crabomination_casts_from_the_union_of_its_three_exiles() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        for selected in 0..3 { crabomination_actual_cast_pool(&definition,selected); }
    }
}

/// Two separate exile sentences are two instructions: "the card exiled this
/// way" keeps naming the most recent exile only (no union).
#[test]
fn separate_exile_sentences_bind_this_way_to_the_latest_exile() {
    const TEXT:&str="Mana cost: {3}{R}\nType: Sorcery\nExile target creature an opponent controls. Exile the top card of your library. Until end of turn, you may play the card exiled this way.";
    let direct=compile_to_runtime_definition("Two Exiles",TEXT,false).unwrap();
    let (artifact,_)=compile_to_artifact("Two Exiles",TEXT,false).unwrap();
    let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();restored.validate().unwrap();
    for definition in [direct,materialize_artifact(&restored).unwrap()] {
        separate_sentences_actual_permissions(&definition);
    }
}

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::ids::StableId;
use ironsmith::{CardId,CardType,Effect,EffectContext,GameState,ObjectId,PlayerId,PowerToughness,Zone};
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext,SelectObjectsContext,SelectOptionsContext,TargetsContext};
use ironsmith::effects::execute_effect;
use ironsmith::effects::ResolvedTarget;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct PickPool { preferred:StableId, observed:std::collections::HashSet<StableId> }
impl DecisionMaker for PickPool {
 fn decide_boolean(&mut self,_:&GameState,_:&BooleanContext)->bool{true}
 fn decide_options(&mut self,g:&GameState,c:&SelectOptionsContext)->Vec<usize>{SelectFirstDecisionMaker.decide_options(g,c)}
 fn decide_targets(&mut self,g:&GameState,c:&TargetsContext)->Vec<ironsmith::Target>{SelectFirstDecisionMaker.decide_targets(g,c)}
 fn decide_objects(&mut self,g:&GameState,c:&SelectObjectsContext)->Vec<ObjectId>{
  if !c.candidates.is_empty() && c.candidates.iter().all(|candidate|g.object(candidate.id).is_some_and(|object|object.zone==Zone::Exile)){
   self.observed.extend(c.candidates.iter().map(|candidate|g.object(candidate.id).unwrap().stable_id));
   return c.candidates.iter().find(|candidate|g.object(candidate.id).unwrap().stable_id==self.preferred).map(|candidate|vec![candidate.id]).unwrap_or_default();
  }
  SelectFirstDecisionMaker.decide_objects(g,c)
 }
}
fn game()->GameState{let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g.turn.step=None;g}
fn spell(name:&str,life:i32)->CardDefinition{
 CardDefinitionBuilder::new(CardId::new(),name).mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(5)]))
  .card_types(vec![CardType::Sorcery]).with_spell_effect(vec![Effect::gain_life(life)]).build()
}
fn settle(g:&mut GameState,dm:&mut dyn DecisionMaker){
 for _ in 0..32{
  let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(g,&mut queue,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(g,&mut queue,dm).unwrap();
  if g.stack.is_empty(){return;}ironsmith::game_loop::resolve_stack_entry_with(g,dm).unwrap();
 }
 panic!("actual exile/cast trigger did not settle")
}
fn crabomination_actual_cast_pool(definition:&CardDefinition,selected:usize){
 let mut g=game();let mut cards=Vec::new();
 for (index,zone)in [Zone::Library,Zone::Graveyard,Zone::Hand].into_iter().enumerate(){
  let card=spell(&format!("Independent union member {index}"),[2,3,5][index]);let id=g.create_object_from_definition(&card,B,zone);cards.push(g.object(id).unwrap().stable_id);
 }
 let source=g.create_object_from_definition(definition,A,Zone::Hand);
 let mut dm=PickPool{preferred:cards[selected],observed:Default::default()};
 g.move_object_by_effect(source,Zone::Battlefield).unwrap();settle(&mut g,&mut dm);
 assert_eq!(dm.observed,cards.iter().copied().collect(),"the real cast chooser must offer all three distinct original exile receipts");
 assert_eq!(g.player(A).unwrap().life,20+[2,3,5][selected],"the library, graveyard and hand receipt can each supply the actual free cast");assert_eq!(g.player(B).unwrap().life,20);
 assert_eq!(g.player(A).unwrap().mana_pool.total(),0,"the source permission pays no mana");
 for (index,stable) in cards.into_iter().enumerate(){let id=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(id).unwrap().zone,if index==selected{Zone::Graveyard}else{Zone::Exile});}
}
fn separate_sentences_actual_permissions(definition:&CardDefinition){
 let mut g=game();let creature=CardDefinitionBuilder::new(CardId::new(),"Earlier exile witness").mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(5)])).card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1,5)).build();
 let first=g.create_object_from_definition(&creature,B,Zone::Battlefield);let first_stable=g.object(first).unwrap().stable_id;
 let latest=g.create_object_from_definition(&spell("Latest exile witness",1),A,Zone::Library);let latest_stable=g.object(latest).unwrap().stable_id;
 let source=g.create_object_from_definition(definition,A,Zone::Stack);let mut dm=SelectFirstDecisionMaker;let mut context=EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(first)]).with_decision_maker(&mut dm);
 execute_effect(&mut g,&Effect::new(ironsmith::effects::SequenceEffect::new(definition.spell_effect.as_ref().unwrap().all_effects_owned())),&mut context).unwrap();
 let first=g.find_object_by_stable_id(first_stable).unwrap();let latest=g.find_object_by_stable_id(latest_stable).unwrap();assert_eq!(g.object(first).unwrap().zone,Zone::Exile);assert_eq!(g.object(latest).unwrap().zone,Zone::Exile);
 g.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless,10);
 let actions=compute_legal_actions(&g,A).unwrap();let allowed=|id|actions.iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==id));
 assert!(allowed(latest),"the most recent independent exile supplies the play permission");assert!(!allowed(first),"separate authored exile instructions cannot inherit the earlier result as a union");
}

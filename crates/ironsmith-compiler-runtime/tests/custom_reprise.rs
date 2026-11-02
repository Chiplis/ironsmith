//! Graveyard text additions reuse splice's transactional casting program.
use ironsmith::{GameState, ObjectId, PlayerId, Zone, GameProgress};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
use ironsmith::mana::ManaSymbol;
const A: PlayerId=PlayerId::from_index(0);
fn definition(name:&str,text:&str)->ironsmith::CardDefinition {
    ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false)).unwrap_or_else(|e|panic!("{name}: {e}"))
}
struct Choices(Vec<ObjectId>);
impl DecisionMaker for Choices {
    fn decide_objects(&mut self,game:&GameState,ctx:&SelectObjectsContext)->Vec<ObjectId>{
        if ctx.description.starts_with("Reveal cards to splice") {self.0.clone()} else {SelectFirstDecisionMaker.decide_objects(game,ctx)}
    }
}
fn cast(game:&mut GameState,spell:ObjectId,choices:Vec<ObjectId>)->Result<(),String>{
    let receiving_stable=game.object(spell).ok_or("receiving spell disappeared")?.stable_id;
    let action=compute_legal_actions(game,A).map_err(|e|format!("{e:?}"))?.into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==spell)).ok_or("no cast action")?;
    let mut queue=ironsmith::triggers::TriggerQueue::new();
    let mut state=PriorityLoopState::new(2);
    let mut dm=Choices(choices);
    let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).map_err(|e|format!("{e:?}"))?;
    for _ in 0..64 {
        if !state.has_pending_action(){
            // A normal priority cast can be canceled during mana payment and
            // return ordinary priority progress after rolling back its proposal.
            // Only a committed stack incarnation counts as a successful cast.
            return if game.stack.iter().any(|entry| game.object(entry.object_id).is_some_and(|object|object.stable_id==receiving_stable)) {
                Ok(())
            } else {Err("receiving spell proposal was canceled".into())};
        }
        let GameProgress::NeedsDecisionCtx(ctx)=progress else{return Err(format!("{progress:?}"));};
        progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).map_err(|e|format!("{e:?}"))?;
    }
    Err("casting did not terminate".into())
}
fn game()->GameState {
    let mut g=GameState::new(vec!["A".into(),"B".into()],20);
    g.turn.phase=ironsmith::game_state::Phase::FirstMain;g.turn.step=None;g.turn.active_player=A;g.turn.priority_player=Some(A);g
}
const ADDITION:&str="Mana cost: {1}\nType: Instant\nYou gain 4 life.\nAs you cast an instant or sorcery spell, you may exile this card from your graveyard and pay {1}. If you do, add this card's effects to that spell. You may exile only one card this way per spell.";
#[test]
fn graveyard_text_source_is_exiled_as_cost_and_adds_its_program_once(){
    for choose in [false,true] {
        let mut g=game();
        let main=definition("Receiving spell","Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");
        let extra=definition("Graveyard text",ADDITION);
        let spell=g.create_object_from_definition(&main,A,Zone::Hand);
        let source=g.create_object_from_definition(&extra,A,Zone::Graveyard);
        let stable=g.object(source).unwrap().stable_id;
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);
        cast(&mut g,spell,if choose {vec![source]} else {vec![]}).unwrap();
        let moved=g.find_object_by_stable_id(stable).unwrap();
        assert_eq!(g.object(moved).unwrap().zone,if choose {Zone::Exile} else {Zone::Graveyard});
        ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(g.player(A).unwrap().life,if choose {25} else {21});
    }
}
#[test]
fn graveyard_text_program_rejects_multiple_sources_and_rolls_back(){
    let mut g=game();let main=definition("Receiving spell","Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");let extra=definition("Graveyard text",ADDITION);
    let spell=g.create_object_from_definition(&main,A,Zone::Hand);
    let one=g.create_object_from_definition(&extra,A,Zone::Graveyard);let two=g.create_object_from_definition(&extra,A,Zone::Graveyard);
    g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,3);
    assert!(cast(&mut g,spell,vec![one,two]).is_err());
    assert_eq!(g.object(one).unwrap().zone,Zone::Graveyard);assert_eq!(g.object(two).unwrap().zone,Zone::Graveyard);assert!(g.stack.is_empty());
}
#[test]
fn text_source_in_hand_is_not_eligible_for_graveyard_addition(){
    let mut g=game();let main=definition("Receiving spell","Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");let extra=definition("Graveyard text",ADDITION);
    let spell=g.create_object_from_definition(&main,A,Zone::Hand);let source=g.create_object_from_definition(&extra,A,Zone::Hand);
    g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);
    cast(&mut g,spell,vec![source]).unwrap();
    assert_eq!(g.object(source).unwrap().zone,Zone::Hand);
    ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(g.player(A).unwrap().life,21);
}
#[test]
fn required_targets_from_added_text_must_be_available_before_costs_are_paid(){
    let mut g=game();let main=definition("Receiving spell","Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");
    let extra=definition("Targeted graveyard text",&ADDITION.replace("You gain 4 life.","Destroy target creature."));
    let spell=g.create_object_from_definition(&main,A,Zone::Hand);let source=g.create_object_from_definition(&extra,A,Zone::Graveyard);
    let stable=g.object(source).unwrap().stable_id;
    g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);
    assert!(cast(&mut g,spell,vec![source]).is_err());
    let retained=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(retained).unwrap().zone,Zone::Graveyard);assert!(g.stack.is_empty());
}
#[test]
fn inability_to_pay_additional_mana_does_not_exile_selected_source(){
    let mut g=game();let main=definition("Receiving spell","Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");let extra=definition("Graveyard text",ADDITION);
    let spell=g.create_object_from_definition(&main,A,Zone::Hand);let source=g.create_object_from_definition(&extra,A,Zone::Graveyard);
    let stable=g.object(source).unwrap().stable_id;
    g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,1);
    assert!(cast(&mut g,spell,vec![source]).is_err());
    let retained=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(retained).unwrap().zone,Zone::Graveyard);assert!(g.stack.is_empty());
}

#[test]
fn selected_text_addition_retains_attributed_custom_payment_receipt() {
    let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),"Attributed text source").with_custom_keyword("reprise");
    let mut compiled=ironsmith_compiler::CompilerFacade::new().compile_definition(builder,ADDITION,ironsmith_compiler::CompilePolicy{allow_unsupported:false}).unwrap();
    assert!(compiled.definition.attribute_custom_keyword_payment("reprise"));
    let wire=ironsmith_compiled_artifact::wire_definition_from_serializable(&compiled.definition).unwrap();
    let direct=ironsmith_compiler_runtime::into_runtime_definition(compiled.definition.clone()).unwrap();
    let artifact=ironsmith_compiled_artifact::CompiledCardArtifact::new(ironsmith_compiled_artifact::ArtifactCardIdentity{local_id:ironsmith_compiled_artifact::ArtifactCardId(compiled.definition.card.id.0),name:compiled.definition.card.name.clone(),face_name:None,other_face:None,linked_face_layout:None},ironsmith_compiled_artifact::CompiledCardPayload{definition:wire,canonical_text:ironsmith_text::compiled_text_lines(&direct).join("\n"),ability_labels:ironsmith_text::ability_surface_texts(&direct)},"text-addition-receipt-test",ADDITION.as_bytes());
    let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for extra in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()] {for selected in [false,true]{
        let mut g=game();let main=definition("Receiving spell","Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");
        let spell=g.create_object_from_definition(&main,A,Zone::Hand);let source=g.create_object_from_definition(&extra,A,Zone::Graveyard);g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);
        cast(&mut g,spell,if selected{vec![source]}else{vec![]}).unwrap();
        let stack=g.stack.last().unwrap();let receiving=g.object(stack.object_id).unwrap();
        assert_eq!(stack.optional_costs_paid.paid_custom_keyword("reprise"),selected,"completed stack receipt owns the selected donor payment");
        assert_eq!(receiving.optional_costs_paid.paid_custom_keyword("reprise"),selected,"spell-object filters observe the same completed payment as the stack");
        assert!(!receiving.optional_costs_paid.paid_custom_keyword("replicate"));
    }}
}
#[test]
fn explicit_text_addition_label_is_local_and_reads_the_complete_multiword_name() {
    let extra=definition("Explicit labeled text",&format!("{ADDITION} This text-adding cost is named moon reprise."));
    let mut g=game();let main=definition("Receiving spell","Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");let spell=g.create_object_from_definition(&main,A,Zone::Hand);let source=g.create_object_from_definition(&extra,A,Zone::Graveyard);g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);cast(&mut g,spell,vec![source]).unwrap();
    let stack=g.stack.last().unwrap();assert!(stack.optional_costs_paid.paid_custom_keyword("moon reprise"));let receiving=g.object(stack.object_id).unwrap();assert!(receiving.optional_costs_paid.paid_custom_keyword("moon reprise"));assert!(!receiving.optional_costs_paid.paid_custom_keyword("reprise"));
}
#[test]
fn runtime_attribution_uses_the_same_single_unattributed_text_price_rule() {
    let mut extra=definition("Runtime attributed text",ADDITION);assert!(extra.attribute_custom_keyword_payment("reprise"));assert!(!extra.attribute_custom_keyword_payment("another mechanic"));
}

#[test]
fn text_addition_lineage_is_exact_copied_and_retained_in_last_known_snapshots(){
    use ironsmith::filter::FilterContext;
    let mut g=game();let main=definition("Receiving instant","Mana cost: {1}\nType: Instant\nYou gain 1 life.");
    let donor=definition("Originating text",&format!("{ADDITION}\nDamage dealt by sources with text added from this card can't be prevented."));
    let first=g.create_object_from_definition(&main,A,Zone::Hand);let second=g.create_object_from_definition(&main,A,Zone::Hand);
    let one=g.create_object_from_definition(&donor,A,Zone::Graveyard);let two=g.create_object_from_definition(&donor,A,Zone::Graveyard);let origin_one=g.object(one).unwrap().stable_id;let origin_two=g.object(two).unwrap().stable_id;
    g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,4);cast(&mut g,first,vec![one]).unwrap();let first_stack=g.stack.last().unwrap().object_id;cast(&mut g,second,vec![two]).unwrap();let second_stack=g.stack.last().unwrap().object_id;
    let mut filter=ironsmith::target::ObjectFilter::default();filter.text_added_from_stable=Some(origin_one);let context=FilterContext::new(A).with_source(first_stack);
    assert!(object_matches(&g,&filter,first_stack,&context));assert!(!object_matches(&g,&filter,second_stack,&context),"same definition is not the same original source receipt");
    let mut other=filter.clone();other.text_added_from_stable=Some(origin_two);assert!(!object_matches(&g,&other,first_stack,&context));assert!(object_matches(&g,&other,second_stack,&context));
    let copy=ironsmith::Effect::new(ironsmith::effects::CopySpellEffect::single(ironsmith::target::ChooseSpec::SpecificObject(first_stack)));
    ironsmith::effects::execute_effect(&mut g,&copy,&mut ironsmith::effects::EffectContext::new_default(second_stack,A)).unwrap();let copied=g.stack.last().unwrap().object_id;assert_ne!(copied,first_stack);assert!(object_matches(&g,&filter,copied,&context));
    let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(g.object(first_stack).unwrap(),&g);let moved=g.move_object_by_effect(first_stack,Zone::Graveyard).unwrap();assert!(!object_matches(&g,&filter,moved,&context));assert!(snapshot_matches(&g,&filter,&snapshot,&context),"departure LKI retains the exact lineage");
}
#[test]
fn only_sources_carrying_the_original_text_bypass_damage_prevention(){
    let mut g=game();let main=definition("Receiving instant","Mana cost: {1}\nType: Instant\nYou gain 1 life.");let donor=definition("Originating text",&format!("{ADDITION}\nDamage dealt by sources with text added from this card can't be prevented."));
    let spell=g.create_object_from_definition(&main,A,Zone::Hand);let source=g.create_object_from_definition(&donor,A,Zone::Graveyard);g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);cast(&mut g,spell,vec![source]).unwrap();let receiving=g.stack.last().unwrap().object_id;
    let shield=ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(),"Prevention shield").card_types(vec![ironsmith::types::CardType::Enchantment]).with_ability(ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::prevent_all_damage_to_you())).build();g.create_object_from_definition(&shield,PlayerId::from_index(1),Zone::Battlefield);
    let ordinary=g.create_object_from_definition(&main,A,Zone::Stack);
    let effect=ironsmith::Effect::deal_damage(2,ironsmith::target::ChooseSpec::SpecificPlayer(PlayerId::from_index(1)));
    ironsmith::effects::execute_effect(&mut g,&effect,&mut ironsmith::effects::EffectContext::new_default(ordinary,A)).unwrap();assert_eq!(g.player(PlayerId::from_index(1)).unwrap().life,20);
    ironsmith::effects::execute_effect(&mut g,&effect,&mut ironsmith::effects::EffectContext::new_default(receiving,A)).unwrap();assert_eq!(g.player(PlayerId::from_index(1)).unwrap().life,18);
}

#[test]
fn source_relative_lineage_survives_artifact_serialization_before_it_is_bound() {
    use ironsmith::filter::FilterContext;
    let text=format!("{ADDITION}\nDamage dealt by sources with text added from this card can't be prevented.");
    let (artifact,direct)=ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Artifact text origin",&text,false)).unwrap();
    let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for donor in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()] {
        assert!(donor.abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(ability) if ability.canonical_model().is_some_and(|model|matches!(model.payload,ironsmith_core::StaticAbilityPayload::RuleRestriction{restriction:ironsmith::effect::Restriction::PreventDamageFrom{..},..})))),"durationless donor rule must be classified as a static ability before lineage binding; donor={donor:#?}");
        let mut g=game();let main=definition("Receiving instant","Mana cost: {1}\nType: Instant\nYou gain 1 life.");let spell=g.create_object_from_definition(&main,A,Zone::Hand);let source=g.create_object_from_definition(&donor,A,Zone::Graveyard);let origin=g.object(source).unwrap().stable_id;
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);cast(&mut g,spell,vec![source]).unwrap();let receiving=g.stack.last().unwrap().object_id;
        let mut filter=ironsmith::target::ObjectFilter::default();filter.text_added_from_stable=Some(origin);assert!(object_matches(&g,&filter,receiving,&FilterContext::new(A).with_source(receiving)));
        let bindings=g.current_abilities(receiving).unwrap().into_iter().filter_map(|ability|match ability.kind{ironsmith::ability::AbilityKind::Static(ability)=>ability.canonical_model(),_=>None}).collect::<Vec<_>>();
        assert!(bindings.iter().any(|model|matches!(&model.payload,ironsmith_core::StaticAbilityPayload::RuleRestriction {restriction:ironsmith::effect::Restriction::PreventDamageFrom {sources,..},..} if sources.text_added_from_stable==Some(origin))),"copied rule retains originating card, not receiving spell as its anchor; origin={origin:?}; receiving={receiving:?}; donor={donor:#?}; bindings={bindings:#?}");
    }
}

#[test]
fn selected_source_dynamic_price_preserves_colored_pips_and_never_uses_receiving_spell_cost(){
 for hybrid in [false,true]{for enough in [false,true]{
  let main=definition("Receiving spell","Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");
  let text=format!("Mana cost: {}\nType: Instant\nYou gain 4 life.\nAs you cast an instant or sorcery spell, you may exile this card from your graveyard and pay this card's mana cost. If you do, add this card's effects to that spell. You may exile only one card this way per spell.",if hybrid{"{W/U}"}else{"{G}{G}"});
  let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact("Dynamic text source",&text,false).unwrap();let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
  for extra in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]{
   let mut g=game();let spell=g.create_object_from_definition(&main,A,Zone::Hand);let source=g.create_object_from_definition(&extra,A,Zone::Graveyard);let stable=g.object(source).unwrap().stable_id;
   g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,3);
   if enough{g.player_mut(A).unwrap().mana_pool.add(if hybrid{ManaSymbol::White}else{ManaSymbol::Green},if hybrid{1}else{2});}
   let result=cast(&mut g,spell,vec![source]);let moved=g.find_object_by_stable_id(stable).unwrap();assert_eq!(result.is_ok(),enough,"colored source price cannot be paid with only receiving generic spell mana");assert_eq!(g.object(moved).unwrap().zone,if enough{Zone::Exile}else{Zone::Graveyard});
   if enough{ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(g.player(A).unwrap().life,25);}
  }
 }}
}
#[test]
fn source_with_no_mana_cost_is_not_a_zero_mana_price_and_failed_selection_rolls_back(){
 for has_cost in [false,true]{
  let mut g=game();let main=definition("Receiving spell","Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");let body=ADDITION.replace("Mana cost: {1}\n",if has_cost{"Mana cost: {0}\n"}else{""}).replace("pay {1}.","pay this card's mana cost.");let extra=definition("Optional zero or missing cost",&body);let spell=g.create_object_from_definition(&main,A,Zone::Hand);let source=g.create_object_from_definition(&extra,A,Zone::Graveyard);let stable=g.object(source).unwrap().stable_id;g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,1);let result=cast(&mut g,spell,vec![source]);assert_eq!(result.is_ok(),has_cost);let moved=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(moved).unwrap().zone,if has_cost{Zone::Exile}else{Zone::Graveyard});if !has_cost{assert!(g.stack.is_empty());assert_eq!(g.player(A).unwrap().mana_pool.total(),1);}
 }
}

// Public selection and frozen-condition APIs exercise the same predicates
// without importing the engine's private ObjectFilterExt implementation.
fn object_matches(game:&GameState,filter:&ironsmith::ObjectFilter,id:ObjectId,context:&ironsmith::filter::FilterContext)->bool{
    let object=game.object(id).unwrap();let mut filter=filter.clone();filter.zone=Some(object.zone);
    let ctx=ironsmith::effects::EffectContext::new_default(context.source.unwrap_or(id),context.you.unwrap_or(A));
    ironsmith::effects::helpers::resolve_objects_from_spec(game,&ironsmith::target::ChooseSpec::All(filter),&ctx).unwrap().contains(&id)
}
fn snapshot_matches(game:&GameState,filter:&ironsmith::ObjectFilter,snapshot:&ironsmith::snapshot::ObjectSnapshot,context:&ironsmith::filter::FilterContext)->bool{
    let mut ctx=ironsmith::effects::EffectContext::new_default(context.source.unwrap_or(snapshot.object_id),context.you.unwrap_or(A));
    ctx.tagged_objects.insert("test_departure".into(),vec![snapshot.clone()]);
    ironsmith::condition_eval::evaluate_condition_resolution(game,&ironsmith::effect::Condition::TaggedObjectMatchedLastKnown("test_departure".into(),filter.clone()),&ctx).unwrap()
}

#[test]
fn durationless_passive_restriction_is_not_a_spell_resolution_effect_on_nonpermanent_source(){
    for (text,expected_static) in [
        ("Damage dealt by sources with text added from this card can't be prevented.",true),
        ("Damage dealt by sources with text added from this card can't be prevented this turn.",false),
    ] {
        let source=definition("Independent damage rule",&format!("Mana cost: {{1}}\nType: Instant\nYou gain 4 life.\n{text}"));
        let static_rule=source.abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(ability) if ability.canonical_model().is_some_and(|model|matches!(model.payload,ironsmith_core::StaticAbilityPayload::RuleRestriction{restriction:ironsmith::effect::Restriction::PreventDamageFrom{..},..}))));
        assert_eq!(static_rule,expected_static,"{text}: {source:#?}");
    }
}

#[test]
fn filtered_passive_damage_rules_remain_static_after_spell_line_lowering() {
    for rule in [
        "Damage dealt by creatures you control can't be prevented.",
        "As long as it's your turn, damage dealt by creatures you control can't be prevented.",
    ] {
        let text=format!("Type: Instant\nYou gain 4 life.\n{rule}");
        let direct=definition("Independent filtered rule",&text);
        let (artifact,_)=ironsmith_compiler_runtime::compile_to_artifact("Independent filtered rule",text,false).unwrap();
        let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
        for def in [direct,restored] {
            assert!(def.abilities.iter().any(|ability| matches!(&ability.kind,
                ironsmith::ability::AbilityKind::Static(ability) if format!("{:?}",ability.canonical_model()).contains("PreventDamageFrom"))),"{def:#?}");
            assert!(!format!("{:?}",def.spell_effect).contains("CantEffect"),"{def:#?}");
        }
    }
}

// Artifact-boundary counterparts of engine native/private-state splice tests.
#[cfg(test)]
mod splice_reveal_artifact_boundary {
use ironsmith::{GameState,ObjectId,PlayerId,Zone,GameProgress,Effect};
use ironsmith::decision::{DecisionMaker,LegalAction};
use ironsmith::decisions::context::{DecisionContext,BooleanContext};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{Ability,CardDefinition,CardDefinitionBuilder,CardId,CardType,Subtype};
use ironsmith::decision::{SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::static_abilities::{CompiledStaticAbility,StaticAbility,SpliceSpec,SpliceQuality};
use ironsmith::game_state::Phase;
const A:PlayerId=PlayerId::from_index(0);
#[derive(Default)]
struct Choices {selected:Vec<ObjectId>,answer:bool,views:usize,splice_prompts:usize}
impl DecisionMaker for Choices {
 fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{
  if ctx.description.starts_with("Reveal cards to splice"){self.splice_prompts+=1;self.selected.clone()}else{SelectFirstDecisionMaker.decide_objects(game,ctx)}
 }
 fn decide_boolean(&mut self,_:&GameState,_:&BooleanContext)->bool{self.answer}
 fn view_cards(&mut self,_:&GameState,_:PlayerId,_:&[ObjectId],ctx:&ironsmith::decisions::context::ViewCardsContext){if ctx.public&&ctx.description.contains("spliced"){self.views+=1;}}
}
fn route(def:CardDefinition,wire:bool)->CardDefinition{
 if !wire{return def}
 let model=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(def).unwrap();
 let bytes=serde_json::to_string(&model).unwrap();
 let decoded:ironsmith_compiled_artifact::WireCardDefinition=serde_json::from_str(&bytes).unwrap();
 ironsmith_runtime_catalog::artifact_materializer::materialize_definition(decoded).unwrap()
}
fn setup(wire:bool,count:usize,nested:bool,reprise:bool)->(GameState,TriggerQueue,PriorityLoopState,Choices,ObjectId,Vec<ironsmith::ids::StableId>){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.phase=Phase::FirstMain;game.turn.step=None;game.turn.active_player=A;game.turn.priority_player=Some(A);
 let main=CardDefinitionBuilder::new(CardId::new(),"Main Arcane Spell").mana_cost(ironsmith::mana::ManaCost::new()).card_types(vec![CardType::Sorcery]).subtypes(vec![Subtype::Arcane]).with_spell_effect(vec![Effect::gain_life(1)]).build();
 let main=game.create_object_from_definition(&route(main,wire),A,Zone::Hand);
 let mut selected=Vec::new();let mut stable=Vec::new();
 for _ in 0..count{
  let mut effects=vec![Effect::new(ironsmith::effects::ExileEffect::with_spec(ironsmith::target::ChooseSpec::Source))];
  if nested{effects.push(Effect::may(vec![Effect::gain_life(4)]));}
  let offer=StaticAbility::from_model(CompiledStaticAbility::reveal_action_offer("revealed splice offer",vec![Zone::Hand],Some(1),Zone::Exile,effects,"Exile this revealed card"));
  let splice=StaticAbility::splice(SpliceSpec{custom_keyword:reprise.then(||"Reprise".into()),source_zone:reprise.then_some(Zone::Graveyard),source_destination:reprise.then_some(Zone::Exile),max_per_spell:reprise.then_some(1),descriptive:false,quality:SpliceQuality::Arcane,cost:ironsmith::cost::TotalCost::mana(ironsmith::mana::ManaCost::new()),cost_surface:None});
  let card=CardDefinitionBuilder::new(CardId::new(),"Revealed splice donor").mana_cost(ironsmith::mana::ManaCost::new()).card_types(vec![CardType::Instant]).with_ability(Ability::static_ability(splice).in_zones(vec![Zone::Hand,Zone::Graveyard])).with_ability(Ability::static_ability(offer)).with_spell_effect(vec![Effect::gain_life(4)]).build();
  let card=game.create_object_from_definition(&route(card,wire),A,if reprise{Zone::Graveyard}else{Zone::Hand});
  selected.push(card);stable.push(game.object(card).unwrap().stable_id);
 }
 game.take_pending_trigger_events();
 (game,TriggerQueue::new(),PriorityLoopState::new(2),Choices{selected,..Default::default()},main,stable)
}
fn start(game:&mut GameState,queue:&mut TriggerQueue,state:&mut PriorityLoopState,dm:&mut Choices,main:ObjectId)->GameProgress{
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==main)).unwrap();
 let progress=apply_priority_response_with_dm(game,queue,state,&PriorityResponse::PriorityAction(action),dm).unwrap();
 let GameProgress::NeedsDecisionCtx(ctx@DecisionContext::SelectObjects(_))=progress else{panic!("expected actual splice selection: {progress:?}")};
 apply_decision_context_with_dm(game,queue,state,&ctx,dm).unwrap()
}
fn answer(game:&mut GameState,queue:&mut TriggerQueue,state:&mut PriorityLoopState,dm:&mut Choices,progress:GameProgress,value:bool)->GameProgress{
 let GameProgress::NeedsDecisionCtx(ctx@DecisionContext::Boolean(_))=progress else{panic!("expected actual Boolean response channel: {progress:?}")};
 dm.answer=value;apply_decision_context_with_dm(game,queue,state,&ctx,dm).unwrap()
}
fn finish(game:&mut GameState,queue:&mut TriggerQueue,state:&mut PriorityLoopState,dm:&mut Choices,mut progress:GameProgress){
 for _ in 0..64{
  if !state.has_pending_action(){return}
  let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("pending cast lost its response context: {progress:?}")};
  progress=apply_decision_context_with_dm(game,queue,state,&ctx,dm).unwrap();
 }
 panic!("splice continuation did not finish")
}
fn completed_offers(game:&GameState)->usize{game.turn_store.turn_history.projected_records().filter(|record|record.event.downcast::<ironsmith::events::NamedActionEvent>().is_some_and(|event|event.label=="revealed splice offer")).count()}
fn reveal_count(game:&GameState)->usize{game.turn_store.turn_history.projected_records().filter(|record|record.event.downcast::<ironsmith::events::CardRevealedEvent>().is_some()).count()}
#[test]
fn decline_resumes_original_splice_once_through_real_boolean_channel(){
 for wire in [true]{
  let(mut game,mut queue,mut state,mut dm,main,stable)=setup(wire,1,false,false);
  let progress=start(&mut game,&mut queue,&mut state,&mut dm,main);
  assert_eq!(dm.splice_prompts,1);assert_eq!(dm.views,2);assert!(state.has_pending_action());
  let progress=answer(&mut game,&mut queue,&mut state,&mut dm,progress,false);finish(&mut game,&mut queue,&mut state,&mut dm,progress);
  assert_eq!(dm.splice_prompts,1);assert_eq!(dm.views,2);assert_eq!(completed_offers(&game),0);assert_eq!(reveal_count(&game),1);
  assert_eq!(game.object(game.find_object_by_stable_id(stable[0]).unwrap()).unwrap().zone,Zone::Hand);
  assert_eq!(game.stack.len(),1);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,25);
 }
}
#[test]
fn accepted_offer_and_nested_decision_replay_only_the_private_child(){
 for wire in [true]{
  let(mut game,mut queue,mut state,mut dm,main,stable)=setup(wire,1,true,false);
  let progress=start(&mut game,&mut queue,&mut state,&mut dm,main);
  let GameProgress::NeedsDecisionCtx(original_prompt)=&progress else{panic!("expected outer offer")};
  let original_prompt=original_prompt.clone();
  let progress=answer(&mut game,&mut queue,&mut state,&mut dm,progress,true);
  assert!(matches!(progress,GameProgress::NeedsDecisionCtx(DecisionContext::Boolean(_))));
  assert!(apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&original_prompt,&mut dm).is_err(),"an old outer Boolean cannot answer the child decision");
  assert_eq!(game.object(game.find_object_by_stable_id(stable[0]).unwrap()).unwrap().zone,Zone::Hand);
  assert_eq!(game.player(A).unwrap().life,20);assert_eq!(completed_offers(&game),0);assert_eq!(dm.views,2);
  let progress=answer(&mut game,&mut queue,&mut state,&mut dm,progress,true);finish(&mut game,&mut queue,&mut state,&mut dm,progress);
  assert_eq!(game.object(game.find_object_by_stable_id(stable[0]).unwrap()).unwrap().zone,Zone::Exile);
  assert_eq!(game.player(A).unwrap().life,24);assert_eq!(completed_offers(&game),1);assert_eq!(reveal_count(&game),1);assert_eq!(dm.views,2);assert_eq!(dm.splice_prompts,1);
  assert_eq!(game.stack.len(),1);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,29);
 }
}
#[test]
fn quota_rechecks_next_frozen_donor_without_revealing_or_splicing_again(){
 for wire in [true]{
  let(mut game,mut queue,mut state,mut dm,main,stable)=setup(wire,2,false,false);
  let progress=start(&mut game,&mut queue,&mut state,&mut dm,main);
  assert!(state.has_pending_action()); // Native unit counterpart also inspects the private frozen donor vector.
  let progress=answer(&mut game,&mut queue,&mut state,&mut dm,progress,true);finish(&mut game,&mut queue,&mut state,&mut dm,progress);
  assert_eq!(completed_offers(&game),1);assert_eq!(reveal_count(&game),2);assert_eq!(dm.views,2);assert_eq!(dm.splice_prompts,1);
  assert_eq!(game.object(game.find_object_by_stable_id(stable[0]).unwrap()).unwrap().zone,Zone::Exile);
  assert_eq!(game.object(game.find_object_by_stable_id(stable[1]).unwrap()).unwrap().zone,Zone::Hand);
  assert_eq!(game.stack.len(),1);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,29);
 }
}
#[test]
fn reprise_moving_source_cost_is_not_reinterpreted_as_a_hand_reveal_offer(){
 for wire in [true]{
  let(mut game,mut queue,mut state,mut dm,main,stable)=setup(wire,1,false,true);
  let progress=start(&mut game,&mut queue,&mut state,&mut dm,main);finish(&mut game,&mut queue,&mut state,&mut dm,progress);
  assert_eq!(completed_offers(&game),0);assert_eq!(reveal_count(&game),0);assert_eq!(dm.splice_prompts,1);
  assert_eq!(game.object(game.find_object_by_stable_id(stable[0]).unwrap()).unwrap().zone,Zone::Exile);
  assert_eq!(game.stack.len(),1);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,25);
 }
}

}

//! Coordinated membership survives owner departure without copying onto new objects.
use ironsmith::{GameState,ObjectId,PlayerId,Zone,Effect};
use ironsmith::combat_state::{CombatState,AttackTarget};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::triggers::TriggerQueue;
use ironsmith::effects::{execute_effect,EffectContext};
use ironsmith::target::ChooseSpec;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);const C:PlayerId=PlayerId(2);
fn setup()->(GameState,[ObjectId;3]){let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Group participant","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let ids=std::array::from_fn(|_|{let id=game.create_object_from_definition(&body,A,Zone::Battlefield);game.remove_summoning_sickness(id);id});(game,ids)}
fn attack(game:&mut GameState,ids:&[ObjectId])->bool{let mut combat=CombatState::default();let mut queue=TriggerQueue::new();let declarations:Vec<_>=ids.iter().enumerate().map(|(i,id)|AttackerDeclaration{creature:*id,target:AttackTarget::Player(if i%2==0{B}else{C})}).collect();ironsmith::game_loop::apply_attacker_declarations(game,&mut combat,&mut queue,&declarations).is_ok()}
#[test]
fn owned_and_ownerless_groups_require_every_able_actor_without_sharing_defender(){for ownerless in [false,true]{for tapped in [false,true]{let(mut game,ids)=setup();game.link_object_group(if ownerless{None}else{Some(ids[0])},if ownerless{&ids[..]}else{&ids[1..]},true);if tapped{game.tap(ids[2]);}assert!(!attack(&mut game,&ids[..1]));let allowed=if tapped{&ids[..2]}else{&ids[..]};assert!(attack(&mut game,allowed));}}}
#[test]
fn group_survives_owner_departure_and_membership_is_exclusive(){let(mut game,ids)=setup();game.link_object_group(Some(ids[0]),&ids[1..],true);execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::SpecificObject(ids[0]),Zone::Graveyard,false),&mut EffectContext::new_default(ids[0],A)).unwrap();assert!(!attack(&mut game,&ids[1..2]));assert!(attack(&mut game,&ids[1..]));let(mut game,ids)=setup();game.link_object_group(Some(ids[0]),&ids[..],true);assert!(!game.object_group_members_owned_by(ids[0]).contains(&ids[0]));game.link_object_group(None,&[ids[1]],true);assert!(!game.object_group_members_owned_by(ids[0]).contains(&ids[1]));assert!(attack(&mut game,&[ids[1]]));}
#[test]
fn additions_redirect_into_the_owners_existing_membership_group(){let(mut game,ids)=setup();let extra=game.create_object_from_definition(&ironsmith_compiler_runtime::compile_to_runtime_definition("New participant","Type: Creature\nPower/Toughness: 1/1",false).unwrap(),A,Zone::Battlefield);game.link_object_group(Some(ids[0]),&[ids[1]],true);game.link_object_group(Some(ids[1]),&[extra],true);assert!(game.object_group_members_owned_by(ids[0]).contains(&extra));assert!(game.object_group_members_owned_by(ids[1]).is_empty());assert_eq!(game.object_group_members_with(ids[1]),game.object_group_members_with(extra));}
#[test]
fn blocking_together_allows_different_attackers_and_waives_unable_members(){for tapped in [false,true]{let(mut game,ids)=setup();game.link_object_group(Some(ids[0]),&ids[1..],true);if tapped{game.tap(ids[2]);}let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Incoming attacker","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let incoming:[ObjectId;2]=std::array::from_fn(|_|{let id=game.create_object_from_definition(&body,B,Zone::Battlefield);game.remove_summoning_sickness(id);id});game.turn.active_player=B;game.mark_combat_phase_started();let mut combat=CombatState::default();let mut queue=TriggerQueue::new();let declarations:Vec<_>=incoming.iter().map(|id|AttackerDeclaration{creature:*id,target:AttackTarget::Player(A)}).collect();ironsmith::game_loop::apply_attacker_declarations(&mut game,&mut combat,&mut queue,&declarations).unwrap();assert!(ironsmith::combat_state::declare_blockers(&game,&mut combat.clone(),vec![(ids[0],incoming[0])]).is_err());let mut blocks=vec![(ids[0],incoming[0]),(ids[1],incoming[1])];if !tapped{blocks.push((ids[2],incoming[0]));}assert!(ironsmith::combat_state::declare_blockers(&game,&mut combat,blocks).is_ok());}}

#[test]
fn member_attack_does_not_require_the_owner_to_attack(){let(mut game,ids)=setup();game.link_object_group(Some(ids[0]),&ids[1..],true);assert!(attack(&mut game,&ids[1..]));}

#[test]
fn full_akvani_source_creates_red_army_and_retains_formal_group_obligations(){
 const ORIGINAL:&str="Retinue — 1/1 (When this enters, create the listed red Army tokens. They attack and block together and with this if able.)\nAttacking tokens you control have deathtouch.";
 assert!(ORIGINAL.contains("with this if able")); // Preserve source discrepancy as evidence.
 let text="Mana cost: {4}{G}\nType: Creature — Ogre Warrior\nPower/Toughness: 5/5\nWhen this permanent enters, create a 1/1 red Army creature token. Group the tokens created this way under this permanent so all members attack and block if any member or the owner does, if able.\nAttacking tokens you control have deathtouch.";
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Akvani Warbrute",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let encoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&encoded).unwrap()]{
  let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
  let outcome=execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::Source,Zone::Battlefield,false),&mut EffectContext::new_default(source,A)).unwrap();
  for event in outcome.events {game.queue_trigger_event(event.provenance(),event);}
  let mut queue=TriggerQueue::new();let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
  ironsmith::game_loop::drain_pending_trigger_events(&mut game,&mut queue);
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();assert_eq!(game.stack.len(),1);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  let source=game.find_object_by_stable_id(stable).unwrap();let members=game.object_group_members_owned_by(source);assert_eq!(members.len(),1);let army=members[0];assert!(game.object(army).unwrap().subtypes.contains(&ironsmith::types::Subtype::Army));assert!(game.object(army).unwrap().colors().contains(ironsmith::color::Color::Red));assert_eq!(game.current_power(army),Some(1));assert_eq!(game.current_toughness(army),Some(1));
  game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();game.remove_summoning_sickness(source);game.remove_summoning_sickness(army);
  assert!(!attack(&mut game,&[source]));assert!(attack(&mut game,&[army]));assert!(game.object(army).unwrap().has_static_ability_id(ironsmith::static_abilities::StaticAbilityId::Deathtouch));
 }
}

struct ChooseSecondGroup;
impl ironsmith::decision::DecisionMaker for ChooseSecondGroup {
 fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{assert_eq!(ctx.player,B,"the member's controller chooses, not the effect's controller");vec![ctx.options[1].index]}
}
#[test]
fn simultaneous_group_insertions_use_members_controller_choice(){
 let(mut game,ids)=setup();let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Contested member","Type: Creature\nPower/Toughness: 1/1",false).unwrap();let member=game.create_object_from_definition(&body,B,Zone::Battlefield);
 let effect=ironsmith_core::LinkObjectsToGroupEffect{groups:vec![
  ironsmith_core::ObjectGroupAssignment{subjects:ChooseSpec::SpecificObject(member),owner:Some(ChooseSpec::SpecificObject(ids[0])),combat_together:true},
  ironsmith_core::ObjectGroupAssignment{subjects:ChooseSpec::SpecificObject(member),owner:Some(ChooseSpec::SpecificObject(ids[1])),combat_together:true},
 ]};
 execute_effect(&mut game,&Effect::new(effect),&mut EffectContext::new(ids[0],A,&mut ChooseSecondGroup)).unwrap();
 assert!(!game.object_group_members_owned_by(ids[0]).contains(&member));assert!(game.object_group_members_owned_by(ids[1]).contains(&member));
}
#[test]
fn group_filters_use_exact_source_membership_through_public_selector(){
 let(mut game,ids)=setup();game.link_object_group(Some(ids[0]),&[ids[1]],true);
 let mut filter=ironsmith::target::ObjectFilter::creature();filter.group_member_of=Some(Box::new(ChooseSpec::Source));
 let ctx=EffectContext::new_default(ids[0],A);
 let selected=ironsmith::effects::helpers::resolve_objects_from_spec(&game,&ChooseSpec::All(filter),&ctx).unwrap();
 assert_eq!(selected,vec![ids[1]],"only the exact source's member is selected; neither outsider nor owner qualifies");
}

#[test]
fn every_retinue_family_source_compiles_through_direct_and_artifact_routes(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-retinue.json")).unwrap();
 for row in fixture["mechanics"].as_array().unwrap(){for card in row["representative_cards"].as_array().unwrap(){
  let name=card["name"].as_str().unwrap();let text=card["executable_text"].as_str().unwrap_or_else(||panic!("{name} requires parent source materialization"));let mut builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),name);
  if let Some(identities)=card["custom_keyword_identities"].as_array(){for identity in identities{builder=builder.with_custom_keyword(identity.as_str().unwrap());}}
  let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_builder_to_artifact(builder,text,false)));
  assert!(!loss.is_lossy(),"{name}: {}",loss.reasons_text());let(artifact,_)=result.unwrap_or_else(|error|panic!("{name}: {error:?}"));let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
 }}
}

struct StoryChoices(Vec<ObjectId>);
impl ironsmith::decision::DecisionMaker for StoryChoices {
 fn decide_number(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::NumberContext)->u32{assert!(ctx.min<=1&&ctx.max>=1);1}
 fn decide_targets(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{self.0.iter().map(|id|{let target=ironsmith::Target::Object(*id);assert!(ctx.requirements[0].legal_targets.contains(&target));target}).collect()}
}
#[test]
fn full_stories_source_places_all_for_each_copy_results_into_one_ownerless_group(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-retinue.json")).unwrap();let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]=="msem-943").unwrap();let card=&row["representative_cards"][0];let(artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap(),false).unwrap();let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]{
  let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
  let originals=[("Earlier scout",1),("Earlier soldier",2)].into_iter().map(|(name,stat)|{let body=ironsmith_compiler_runtime::compile_to_runtime_definition(name,&format!("Mana cost: {{1}}\nType: Creature\nPower/Toughness: {stat}/{stat}"),false).unwrap();game.create_object_from_definition(&body,A,Zone::Graveyard)}).collect::<Vec<_>>();let original_stable=originals.iter().map(|id|game.object(*id).unwrap().stable_id).collect::<Vec<_>>();
  let spell=game.create_object_from_definition(&definition,A,Zone::Hand);game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::White,2);game.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless,1);
  let action=ironsmith::decision::compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,ironsmith::decision::LegalAction::CastSpell{spell_id,..}if *spell_id==spell)).unwrap();let mut queue=TriggerQueue::new();let mut state=ironsmith::game_loop::PriorityLoopState::new(3);let mut dm=StoryChoices(originals);let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut dm).unwrap();
  for _ in 0..64{if !state.has_pending_action(){break}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=ironsmith::game_loop::apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}assert!(!state.has_pending_action());ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  for stable in original_stable{assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile)}let tokens=game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().kind==ironsmith::ObjectKind::Token).collect::<Vec<_>>();assert_eq!(tokens.len(),2);let group=game.object_group_members_with(tokens[0]);assert!(tokens.iter().all(|id|group.contains(id)));assert!(game.object_group_members_owned_by(tokens[0]).is_empty());
  game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();for token in &tokens{game.remove_summoning_sickness(*token)}assert!(!attack(&mut game,&tokens[..1]));assert!(attack(&mut game,&tokens));
 }
}

#[test]
fn ordinary_copy_and_zone_reentry_do_not_copy_group_membership(){
 let(mut game,ids)=setup();game.link_object_group(Some(ids[0]),&[ids[1]],true);let stable=game.object(ids[1]).unwrap().stable_id;
 execute_effect(&mut game,&Effect::create_token_copy(ChooseSpec::SpecificObject(ids[1])),&mut EffectContext::new_default(ids[0],A)).unwrap();let copy=game.battlefield.iter().copied().find(|id|game.object(*id).unwrap().kind==ironsmith::ObjectKind::Token).unwrap();assert!(game.object_group_members_with(copy).is_empty());
 execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::SpecificObject(ids[1]),Zone::Graveyard,false),&mut EffectContext::new_default(ids[0],A)).unwrap();let graveyard=game.find_object_by_stable_id(stable).unwrap();execute_effect(&mut game,&Effect::move_to_zone(ChooseSpec::SpecificObject(graveyard),Zone::Battlefield,false),&mut EffectContext::new_default(ids[0],A)).unwrap();let returned=game.find_object_by_stable_id(stable).unwrap();assert_ne!(returned,ids[1]);assert!(game.object_group_members_with(returned).is_empty());assert!(game.object_group_members_with(ids[1]).contains(&ids[1]),"historical identity is distinct from its returned card");
}

#[path = "custom_final_next_wave_support.rs"]
mod token_addition_source_support;

#[test]
fn actual_djinn_adds_once_to_first_matching_creation_and_preserves_retinue_membership() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-retinue.json"
    )).unwrap();
    let source = fixture["mechanics"].as_array().unwrap().iter()
        .flat_map(|row| row["representative_cards"].as_array().unwrap())
        .find(|card| card["name"] == "Djinn of the Measured Road").unwrap();
    let producer = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Independent grouped creation", "Type: Creature\nPower/Toughness: 2/2\n{0}: Create two 1/1 red Army creature tokens. Group the tokens created this way under this permanent so all members attack and block if any member or the owner does, if able.", false,
    ).unwrap();
    let effects = producer.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability.effects.all_effects()),
        _ => None,
    }).unwrap();
    for definition in token_addition_source_support::definitions(source).unwrap() {
        for (prior_noncreature, recipient) in [(false, A), (true, A), (true, B)] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let owner = game.create_object_from_definition(&producer, recipient, Zone::Battlefield);
            if prior_noncreature {
                let food = ironsmith::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Unrelated token")
                    .token().card_types(vec![ironsmith::CardType::Artifact]).build();
                execute_effect(&mut game, &Effect::new(ironsmith::effects::CreateTokenEffect::you(food, 1)),
                    &mut EffectContext::new_default(owner, recipient)).unwrap();
            }
            for invocation in 0..2 {
                let mut ctx = EffectContext::new_default(owner, recipient);
                for effect in &effects { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
                let soldiers: Vec<_> = game.battlefield.iter().copied().filter(|id|
                    game.object(*id).is_some_and(|object| object.kind == ironsmith::ObjectKind::Token)
                        && game.current_subtypes(*id).is_some_and(|types| types.contains(&ironsmith::Subtype::Soldier))
                ).collect();
                assert_eq!(soldiers.len(), usize::from(recipient == A),
                    "unrelated earlier tokens and later matching creations do not consume/reopen the first matching window");
                let group = game.object_group_members_owned_by(owner);
                assert_eq!(group.len(), (invocation + 1) * 2 + usize::from(recipient == A));
                for soldier in soldiers {
                    assert!(group.contains(&soldier), "replacement-added token belongs to the original creation's group");
                    assert_eq!(game.controller_of_id(soldier), Some(A));
                    assert_eq!(game.calculated_power(soldier), Some(1));
                    assert_eq!(game.calculated_toughness(soldier), Some(1));
                    assert_eq!(game.current_colors(soldier), Some(ironsmith::color::ColorSet::WHITE));
                }
            }
        }
    }
}

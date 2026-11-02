use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::effect::{Effect,Value};
use crate::target::{ChooseSpec,PlayerFilter};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn life(amount:i32)->crate::cost::TotalCost{crate::cost::TotalCost::from_cost(crate::costs::Cost::from_model(ironsmith_core::Cost::Life(Value::Fixed(amount))).unwrap())}
fn definition(cost:crate::cost::TotalCost)->crate::cards::CardDefinition{
 let rule=crate::static_abilities::StaticAbility::from_model(ironsmith_core::StaticAbility{
  id:Some(ironsmith_core::StaticAbilityId::IntrinsicImmediateAction),label:"Private paid entry".into(),
  payload:ironsmith_core::StaticAbilityPayload::IntrinsicImmediateAction{
   cost,timing:ironsmith_core::static_ability_model::ImmediateActionTiming::Sorcery,source_zone:Zone::Hand,
   player:PlayerFilter::You,condition:None,shared_turn_group:None,once:false,
   effects:vec![Effect::move_to_zone(ChooseSpec::Source,Zone::Exile,false)],
  },
 });
 crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Private source")
  .card_types(vec![crate::types::CardType::Artifact])
  .with_ability(crate::ability::Ability::static_ability(rule).in_zones(vec![Zone::Hand])).build()
}
fn setup(known:bool,cost:crate::cost::TotalCost)->(GameState,ObjectId,crate::cards::CardDefinition){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=Phase::FirstMain;game.turn.step=None;
 let source=game.create_hidden_card_placeholder(A,Zone::Hand,7,"ziffle:private-admission:7".into());let definition=definition(cost);
 if known{game.reveal_hidden_card_with_definition(source,&definition);}
 (game,source,definition)
}
fn reserve(game:&GameState,source:ObjectId,cost:crate::cost::TotalCost)->PrivateCostReservation{
 reserve_private_action(game,A,source,PrivateActionOccurrence::Intrinsic{ability_index:0},cost).unwrap()
}
#[test]
fn placeholder_cost_reservation_is_not_payment_authority(){
 let(mut game,source,definition)=setup(false,life(3));let reservation=reserve(&game,source,life(3));
 assert!(matches!(stage_private_action(&game,&reservation,&mut SelectFirstDecisionMaker).unwrap(),PrivateActionStage::AwaitingAuthenticatedOpening));
 assert_eq!(game.player(A).unwrap().life,20);assert_eq!(game.object(source).unwrap().zone,Zone::Hand);assert!(game.is_hidden_card_placeholder(source));assert!(!game.is_publicly_revealed_hidden_card(source));
 assert_eq!(commit_private_action(&mut game,&reservation,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::AuthenticatedOpeningRequired);
 game.reveal_hidden_card_with_definition(source,&definition);
 let PrivateActionStage::CompletedPreview(preview)=stage_private_action(&game,&reservation,&mut SelectFirstDecisionMaker).unwrap()else{panic!("known private owner can preview full paid action")};
 assert_eq!(preview.player(A).unwrap().life,17);assert!(preview.exile.iter().any(|id|preview.object(*id).unwrap().name=="Private source"));
 assert_eq!(game.player(A).unwrap().life,20);assert_eq!(game.object(source).unwrap().zone,Zone::Hand);assert!(!game.is_publicly_revealed_hidden_card(source));
 assert_eq!(commit_private_action(&mut game,&reservation,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::AuthenticatedOpeningRequired,"private knowledge is not a public authenticated opening");
 // The transport's verified-opening boundary marks this exact source public.
 game.mark_hidden_cards_publicly_revealed(&[source]);assert!(commit_private_action(&mut game,&reservation,&mut SelectFirstDecisionMaker).unwrap());assert_eq!(game.player(A).unwrap().life,17);
 assert_eq!(commit_private_action(&mut game,&reservation,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::WrongSource,"completed action cannot be charged twice");
}
#[test]
fn final_opening_rejects_cheap_or_wrong_ordinal_reservations_before_payment(){
 let(mut game,source,definition)=setup(false,life(3));let cheap=reserve(&game,source,life(0));let ordinal=reserve_private_action(&game,A,source,PrivateActionOccurrence::Intrinsic{ability_index:999},life(3)).unwrap();
 game.reveal_hidden_card_with_definition(source,&definition);game.mark_hidden_cards_publicly_revealed(&[source]);
 assert_eq!(commit_private_action(&mut game,&cheap,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::CostMismatch);
 assert_eq!(commit_private_action(&mut game,&ordinal,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::InvalidOccurrence);
 assert_eq!(game.player(A).unwrap().life,20);assert_eq!(game.object(source).unwrap().zone,Zone::Hand);
}
#[test]
fn actor_turn_and_exact_incarnation_are_bound_to_the_reservation(){
 let(mut game,source,definition)=setup(true,life(3));assert_eq!(reserve_private_action(&game,B,source,PrivateActionOccurrence::Intrinsic{ability_index:0},life(3)).unwrap_err(),PrivateCostAdmissionError::WrongActor);
 let reservation=reserve(&game,source,life(3));game.mark_hidden_cards_publicly_revealed(&[source]);game.turn.turn_number+=1;assert_eq!(commit_private_action(&mut game,&reservation,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::StaleTurn);game.turn.turn_number-=1;
 let old_stable=game.object(source).unwrap().stable_id;game.move_object(source,Zone::Graveyard,crate::events::EventCause::effect()).unwrap();let current=game.find_object_by_stable_id(old_stable).unwrap();game.move_object(current,Zone::Hand,crate::events::EventCause::effect()).unwrap();assert_eq!(commit_private_action(&mut game,&reservation,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::WrongSource);assert_eq!(game.player(A).unwrap().life,20);let _=definition;
}
#[test]
fn canonical_comparison_retains_alternative_structure_and_all_nonmana_components(){
 let required=crate::cost::TotalCost::one_of(vec![life(3),life(5)]);let(mut game,source,definition)=setup(false,required.clone());
 let incomplete=reserve(&game,source,life(3));let correct=reserve(&game,source,required);
 game.reveal_hidden_card_with_definition(source,&definition);game.mark_hidden_cards_publicly_revealed(&[source]);
 assert_eq!(commit_private_action(&mut game,&incomplete,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::CostMismatch);
 assert_eq!(game.player(A).unwrap().life,20);assert!(commit_private_action(&mut game,&correct,&mut SelectFirstDecisionMaker).unwrap());assert_eq!(game.player(A).unwrap().life,17);
}
#[test]
fn paused_cost_choices_publish_no_resources_source_opening_or_original_action(){
 struct Pending{pending:bool}
 impl DecisionMaker for Pending{
  fn decide_options(&mut self,_:&GameState,_:&crate::decisions::context::SelectOptionsContext)->Vec<usize>{self.pending=true;vec![]}
  fn awaiting_choice(&self)->bool{self.pending}
 }
 let cost=crate::cost::TotalCost::one_of(vec![life(3),life(5)]);let(mut game,source,definition)=setup(false,cost.clone());let reservation=reserve(&game,source,cost);
 assert!(matches!(stage_private_action(&game,&reservation,&mut Pending{pending:false}).unwrap(),PrivateActionStage::AwaitingDecision));assert!(game.is_hidden_card_placeholder(source));assert_eq!(game.player(A).unwrap().life,20);assert!(game.exile.is_empty());assert!(!game.is_publicly_revealed_hidden_card(source));
 game.reveal_hidden_card_with_definition(source,&definition);game.mark_hidden_cards_publicly_revealed(&[source]);assert!(!commit_private_action(&mut game,&reservation,&mut Pending{pending:false}).unwrap());assert_eq!(game.player(A).unwrap().life,20);assert_eq!(game.object(source).unwrap().zone,Zone::Hand);
 assert!(commit_private_action(&mut game,&reservation,&mut SelectFirstDecisionMaker).unwrap());assert_eq!(game.player(A).unwrap().life,17);
}
#[test]
fn native_callback_without_canonical_cost_cannot_be_authenticated_by_its_display(){
 // Replacing a real payer invalidates its retained model by pointer identity.
 let(game,source,_)=setup(false,life(3));let mut cost=crate::costs::Cost::from_model(ironsmith_core::Cost::Life(Value::Fixed(3))).unwrap();let replacement=crate::costs::Cost::life(0);cost.0=replacement.0;
 assert_eq!(reserve_private_action(&game,A,source,PrivateActionOccurrence::Intrinsic{ability_index:0},crate::cost::TotalCost::from_cost(cost)).unwrap_err(),PrivateCostAdmissionError::MissingCanonicalCost);
}

#[test]
fn reservation_binds_authenticated_hidden_generation_and_immutable_ciphertext_anchor(){
 let(mut game,source,definition)=setup(false,life(3));let generation=game.hidden_card_info(source).unwrap().incarnation;
 assert_eq!(reserve_private_action_with_incarnation(&game,A,source,None,PrivateActionOccurrence::Intrinsic{ability_index:0},life(3)).unwrap_err(),PrivateCostAdmissionError::WrongSource);
 assert_eq!(reserve_private_action_with_incarnation(&game,A,source,generation.map(|n|n+1),PrivateActionOccurrence::Intrinsic{ability_index:0},life(3)).unwrap_err(),PrivateCostAdmissionError::WrongSource);
 let reservation=reserve_private_action_with_incarnation(&game,A,source,generation,PrivateActionOccurrence::Intrinsic{ability_index:0},life(3)).unwrap();
 let original=game.hidden_card_info(source).unwrap().clone();
 let mut info=original.clone();info.origin_commitment=Some("forged-other-ciphertext".into());game.set_hidden_card_info(source,info);
 assert_eq!(game.hidden_card_info(source).unwrap().origin_commitment,original.origin_commitment,"metadata hydration cannot replace the authenticated ciphertext anchor");
 game.reveal_hidden_card_with_definition(source,&definition);game.mark_hidden_cards_publicly_revealed(&[source]);
 let mut forged=reservation.clone();forged.hidden_source.as_mut().unwrap().3="forged-other-ciphertext".into();
 assert_eq!(commit_private_action(&mut game,&forged,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::WrongSource);assert_eq!(game.player(A).unwrap().life,20);assert_eq!(game.object(source).unwrap().zone,Zone::Hand);
 assert!(commit_private_action(&mut game,&reservation,&mut SelectFirstDecisionMaker).unwrap());assert_eq!(game.player(A).unwrap().life,17);
}
#[test]
fn completion_cannot_invent_an_opening_for_an_action_that_keeps_its_source_private(){
 let(mut game,source,_)=setup(false,life(3));let mut definition=definition(life(3));
 let crate::ability::AbilityKind::Static(rule)=&mut definition.abilities[0].kind else{panic!("static action")};
 let mut model=rule.canonical_model().unwrap();let ironsmith_core::StaticAbilityPayload::IntrinsicImmediateAction{effects,..}=&mut model.payload else{panic!("intrinsic")};*effects=vec![Effect::gain_life(1)];*rule=crate::static_abilities::StaticAbility::from_model(model);
 game.reveal_hidden_card_with_definition(source,&definition);let reservation=reserve(&game,source,life(3));
 assert!(matches!(stage_private_action(&game,&reservation,&mut SelectFirstDecisionMaker),Err(PrivateCostAdmissionError::SourceRemainsPrivate)));
 assert_eq!(game.player(A).unwrap().life,20);assert_eq!(game.object(source).unwrap().zone,Zone::Hand);assert!(!game.is_publicly_revealed_hidden_card(source));assert!(!game.effect_store.has_pending_trigger_work());
 game.mark_hidden_cards_publicly_revealed(&[source]);
 assert_eq!(commit_private_action(&mut game,&reservation,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::SourceRemainsPrivate,"an imported opening flag cannot impersonate an action-authored public source transition");
 assert_eq!(game.player(A).unwrap().life,20);assert_eq!(game.object(source).unwrap().zone,Zone::Hand);
}
#[test]
fn preparation_selected_method_full_cost_is_authenticated_before_mana_or_sacrifice_commits(){
 let(mut game,source,_)=setup(false,life(3));
 let green=crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Green]);
 let full=crate::cost::TotalCost::from_costs(vec![crate::costs::Cost::mana(green.clone()),crate::costs::Cost::from_model(ironsmith_core::Cost::Sacrifice(crate::filter::ObjectFilter::land())).unwrap()]);
 let preparation=crate::alternative_cast::ExilePreparation::FaceUpSorcery{price:green.clone(),on_next_spell_resolution:true,optional_cast:false};
 let method=crate::alternative_cast::AlternativeCastingMethod::cast_from_zone_with_total_cost("Independent preparation",Zone::Exile,crate::cost::TotalCost::free(),None,false)
  .with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions{exile_preparation:Some(preparation.clone()),..Default::default()})
  .with_preparation_total_cost(full.clone());
 let definition=crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Prepared source").card_types(vec![crate::types::CardType::Sorcery]).alternative_cast(method).build();
 let occurrence=PrivateActionOccurrence::Preparation{method_index:0,preparation:preparation.clone()};let reservation=reserve_private_action(&game,A,source,occurrence.clone(),full).unwrap();let mana_only=reserve_private_action(&game,A,source,occurrence,crate::cost::TotalCost::mana(green)).unwrap();
 let land_definition=crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Sacrifice material").card_types(vec![crate::types::CardType::Land]).build();let land=game.create_object_from_definition(&land_definition,A,Zone::Battlefield);game.player_mut(A).unwrap().mana_pool.add(crate::mana::ManaSymbol::Green,1);
 game.reveal_hidden_card_with_definition(source,&definition);
 let PrivateActionStage::CompletedPreview(preview)=stage_private_action(&game,&reservation,&mut SelectFirstDecisionMaker).unwrap()else{panic!("complete real preparation preview")};assert!(preview.object(land).is_none());assert!(game.object(land).is_some());assert!(!game.is_publicly_revealed_hidden_card(source));
 game.mark_hidden_cards_publicly_revealed(&[source]);assert_eq!(commit_private_action(&mut game,&mana_only,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::CostMismatch);assert!(game.object(land).is_some());
 assert!(commit_private_action(&mut game,&reservation,&mut SelectFirstDecisionMaker).unwrap());assert!(game.object(land).is_none());assert_eq!(game.effect_store.delayed_triggers.len(),1);assert!(game.stack.is_empty());
}
#[test]
fn known_same_price_ability_replacement_cannot_substitute_a_new_body_at_the_same_ordinal(){
 let(mut game,source,_)=setup(true,life(3));let reservation=reserve(&game,source,life(3));
 let mut changed=definition(life(3));let crate::ability::AbilityKind::Static(rule)=&mut changed.abilities[0].kind else{panic!("static action")};let mut model=rule.canonical_model().unwrap();let ironsmith_core::StaticAbilityPayload::IntrinsicImmediateAction{effects,..}=&mut model.payload else{panic!("intrinsic")};effects.push(Effect::gain_life(9));*rule=crate::static_abilities::StaticAbility::from_model(model);
 std::sync::Arc::make_mut(&mut game.object_mut(source).unwrap().abilities)[0]=changed.abilities[0].clone();game.mark_hidden_cards_publicly_revealed(&[source]);
 assert_eq!(commit_private_action(&mut game,&reservation,&mut SelectFirstDecisionMaker).unwrap_err(),PrivateCostAdmissionError::InvalidOccurrence);assert_eq!(game.player(A).unwrap().life,20);assert_eq!(game.object(source).unwrap().zone,Zone::Hand);
}
#[test]
fn native_cost_completion_phase_distinguishes_body_pause_from_partial_payment(){
 struct Capture{pending:bool,snapshot:Option<GameState>}
 impl DecisionMaker for Capture{
  fn decide_boolean(&mut self,game:&GameState,_:&crate::decisions::context::BooleanContext)->bool{self.snapshot=Some(game.clone());self.pending=true;false}
  fn awaiting_choice(&self)->bool{self.pending}
 }
 let(mut game,source,_)=setup(false,life(3));let mut definition=definition(life(3));let crate::ability::AbilityKind::Static(rule)=&mut definition.abilities[0].kind else{panic!("action")};let mut model=rule.canonical_model().unwrap();let ironsmith_core::StaticAbilityPayload::IntrinsicImmediateAction{effects,..}=&mut model.payload else{panic!("action")};effects.push(Effect::may(vec![Effect::gain_life(1)]));*rule=crate::static_abilities::StaticAbility::from_model(model);game.reveal_hidden_card_with_definition(source,&definition);let reservation=reserve(&game,source,life(3));let mut dm=Capture{pending:false,snapshot:None};
 assert!(matches!(stage_private_action(&game,&reservation,&mut dm).unwrap(),PrivateActionStage::AwaitingPostPaymentDecision));let snapshot=dm.snapshot.as_ref().unwrap();assert_eq!(snapshot.player(A).unwrap().life,17);assert!(verify_public_source_completion(snapshot,&reservation).is_ok());assert_eq!(game.player(A).unwrap().life,20);assert_eq!(game.object(source).unwrap().zone,Zone::Hand);assert!(!game.is_publicly_revealed_hidden_card(source));
 game.mark_hidden_cards_publicly_revealed(&[source]);let mut dm=Capture{pending:false,snapshot:None};assert!(!commit_private_action(&mut game,&reservation,&mut dm).unwrap());assert_eq!(game.player(A).unwrap().life,20);assert_eq!(game.object(source).unwrap().zone,Zone::Hand);
}
#[test]
fn public_permission_reference_resolves_native_origin_without_serializing_local_ids(){
 let(mut game,_,_)=setup(false,life(3));let provider=crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Public provider").card_types(vec![crate::types::CardType::Creature]).with_ability(crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::haste())).build();let source=game.create_object_from_definition(&provider,A,Zone::Battlefield);
 let reference=PrivateGrantIdentityReference::SourceAbility{ability_index:0};let key=resolve_private_grant_selection(&game,source,3,&reference).unwrap();assert_eq!(encode_private_grant_identity(&game,&key).unwrap(),reference);assert_eq!(key.index,3);assert_eq!(key.source,source);let crate::grant_registry::GrantPermissionIdentity::Static{source:actual,printed_face,..}=key.identity else{panic!("native static identity")};assert_eq!(actual,source);assert_eq!(printed_face,Some(provider.card.id));
 assert_eq!(resolve_private_grant_selection(&game,source,3,&PrivateGrantIdentityReference::SourceAbility{ability_index:999}).unwrap_err(),PrivateCostAdmissionError::InvalidOccurrence);
 let stored=crate::alternative_cast::GrantSelection{source,index:4,identity:crate::grant_registry::GrantPermissionIdentity::Stored(87)};let reference=encode_private_grant_identity(&game,&stored).unwrap();assert_eq!(reference,PrivateGrantIdentityReference::Stored{id:87});assert_eq!(resolve_private_grant_selection(&game,source,4,&reference).unwrap(),stored);
}
#[test]
fn permission_identity_reference_does_not_open_or_inspect_a_private_provider(){
 let(game,source,_)=setup(false,life(3));assert_eq!(resolve_private_grant_selection(&game,source,0,&PrivateGrantIdentityReference::SourceAbility{ability_index:0}).unwrap_err(),PrivateCostAdmissionError::InvalidOccurrence);assert!(game.is_hidden_card_placeholder(source));assert!(!game.is_publicly_revealed_hidden_card(source));
}

//! Chosen outside-game cards use original printed deck text, not current abilities.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::pregame_setup::{PregameSetupSession,PrintedStartingDeckCard,PregameSetupError};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions()->[CardDefinition;2]{
 let text="Type: Legendary Creature — Human Bard\nPower/Toughness: 4/4\nChosen outside game card \"first mate\": Each nonland card in your starting deck has \"+1/+1 counter\" or \"harmonize\" in its printed rules text. You may exchange this card with a card in your opening hand.";
 let (artifact,direct)=ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Lilia of the Soul",text,false)).unwrap();
 assert!(direct.spell_effect.is_none(),"deck choice belongs to printed static abilities");
 let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn index(def:&CardDefinition)->usize{def.abilities.iter().position(|a|matches!(&a.kind,ironsmith::AbilityKind::Static(s) if matches!(s.pregame_action_kind(),Some(ironsmith::static_abilities::PregameActionKind::ChosenOutsideGame(_))))).unwrap()}
fn make_deck(g:&mut GameState,valid:bool)->Vec<PrintedStartingDeckCard>{
 let mut evidence=Vec::new();
 for (kind,text) in [("Land",""),("Artifact","Put a +1/+1 counter on target creature."),("Artifact",if valid{"Harmonize."}else{"Draw a card."})]{
  let def=ironsmith_compiler_runtime::compile_to_runtime_definition("Starting deck memory",&format!("Type: {kind}"),false).unwrap();
  let object=g.create_object_from_definition(&def,A,Zone::Library);
  evidence.push(PrintedStartingDeckCard{object,original_printed_rules_text:text.into()});
 }
 evidence
}
#[test]
fn source_printed_text_condition_and_optional_opening_exchange_survive_artifacts(){
 for def in definitions(){for exchange in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let deck=make_deck(&mut game,true);
  let source=game.create_object_from_definition(&def,A,Zone::OutsideGame);
  let stable=game.object(source).unwrap().stable_id;
  let other=game.create_object_from_definition(&def,A,Zone::OutsideGame);
  let mut setup=PregameSetupSession::new(&game,vec![(A,deck),(B,vec![])]).unwrap();
  setup.choose(&game,A,source,index(&def)).unwrap();
  assert_eq!(setup.choose(&game,A,other,index(&def)),Err(PregameSetupError::AlreadyChosen));
  game.draw_cards(A,3);
  setup.begin_opening_hand_actions().unwrap();
  let outgoing=game.player(A).unwrap().hand[0];let outgoing_stable=game.object(outgoing).unwrap().stable_id;
  if exchange{
   let (incoming,removed)=setup.exchange_with_opening_hand(&mut game,A,"first mate",outgoing).unwrap();
   assert_eq!(game.find_object_by_stable_id(stable),Some(incoming));
   assert_eq!(game.find_object_by_stable_id(outgoing_stable),Some(removed));
   assert_eq!(game.object(incoming).unwrap().zone,Zone::Hand);
   assert_eq!(game.object(removed).unwrap().zone,Zone::OutsideGame);
   let another=game.player(A).unwrap().hand[0];
   assert_eq!(setup.exchange_with_opening_hand(&mut game,A,"first mate",another),Err(PregameSetupError::AlreadyUsed));
  }else{assert_eq!(game.object(source).unwrap().zone,Zone::OutsideGame);}
  assert_eq!(game.player(A).unwrap().hand.len(),3);
  assert!(game.player(A).unwrap().companion.is_none());
  setup.finish();
  assert_eq!(setup.exchange_with_opening_hand(&mut game,A,"first mate",outgoing),Err(PregameSetupError::WrongStage));
 }}
}
#[test]
fn invalid_printed_deck_and_missing_complete_evidence_never_designate_or_move(){
 for def in definitions(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let deck=make_deck(&mut game,false);
  let source=game.create_object_from_definition(&def,A,Zone::OutsideGame);
  let mut incomplete=deck.clone();incomplete.pop();
  assert!(matches!(PregameSetupSession::new(&game,vec![(A,incomplete),(B,vec![])]),Err(PregameSetupError::IncompleteStartingDeck)));
  let mut setup=PregameSetupSession::new(&game,vec![(A,deck),(B,vec![])]).unwrap();
  assert_eq!(setup.choose(&game,A,source,index(&def)),Err(PregameSetupError::ConditionNotFulfilled));
  assert_eq!(setup.choose(&game,B,source,index(&def)),Err(PregameSetupError::InvalidSource));
  assert_eq!(game.object(source).unwrap().zone,Zone::OutsideGame);
 }
}
#[test]
fn chosen_identity_does_not_follow_an_outside_card_that_left_and_returned(){
 for def in definitions(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let deck=make_deck(&mut game,true);
  let source=game.create_object_from_definition(&def,A,Zone::OutsideGame);
  let mut setup=PregameSetupSession::new(&game,vec![(A,deck),(B,vec![])]).unwrap();
  setup.choose(&game,A,source,index(&def)).unwrap();
  let exile=game.move_object_by_effect(source,Zone::Exile).unwrap();
  game.move_object_by_effect(exile,Zone::OutsideGame).unwrap();
  game.draw_cards(A,3);setup.begin_opening_hand_actions().unwrap();
  let outgoing=game.player(A).unwrap().hand[0];
  assert_eq!(setup.exchange_with_opening_hand(&mut game,A,"first mate",outgoing),Err(PregameSetupError::InvalidSource));
  assert_eq!(game.object(outgoing).unwrap().zone,Zone::Hand);
 }
}

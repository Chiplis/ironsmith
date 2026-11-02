//! Whole-cost reduction choices survive legality, announcement and payment.
#[path="custom_final_next_wave_support.rs"] mod routes;
#[path="cf8_p08/play.rs"] mod play;
use ironsmith::{CardDefinition, GameState, GameProgress, ObjectId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, LegalAction, compute_legal_actions};
use ironsmith::mana::ManaSymbol;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
const REDUCER:&str="Type: Enchantment\nSpells you cast cost one mana of any color less to cast.";
fn definitions(name:&str,text:&str)->[CardDefinition;2] {
    routes::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap()
}
#[derive(Default)]
struct ChooseCost { wanted:String, offered:Vec<String> }
impl DecisionMaker for ChooseCost {
 fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::SelectOptionsContext)->Vec<usize> {
  if ctx.description=="Choose the total mana cost" {
   self.offered=ctx.options.iter().filter(|option|option.legal).map(|option|option.description.clone()).collect();
   return vec![ctx.options.iter().find(|option|option.legal&&option.description==format!("Pay {}",self.wanted)).expect("desired complete total is offered").index];
  }
  SelectFirstDecisionMaker.decide_options(game,ctx)
 }
}
fn cast(game:&mut GameState,id:ObjectId,dm:&mut ChooseCost) {
 let action=LegalAction::CastSpell {spell_id:id,from_zone:Zone::Hand,casting_method:ironsmith::alternative_cast::CastingMethod::Normal};
 assert!(compute_legal_actions(game,play::A).unwrap().contains(&action),"some complete reduced total is payable");
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(3);
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
 for _ in 0..64 {
  if !state.has_pending_action(){return;}
  let GameProgress::NeedsDecisionCtx(ctx)=progress else {panic!("{progress:?}")};
  progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).unwrap();
 }
 panic!("cast failed to finish");
}
#[test]
fn every_two_and_three_color_reduction_branch_is_legal_and_paid_as_chosen() {
 for route in 0..2 {for (cost,pairs) in [
  ("{W}{U}",vec![("{W}",vec![ManaSymbol::White]),("{U}",vec![ManaSymbol::Blue])]),
  ("{W}{U}{B}",vec![("{W}{U}",vec![ManaSymbol::White,ManaSymbol::Blue]),("{W}{B}",vec![ManaSymbol::White,ManaSymbol::Black]),("{U}{B}",vec![ManaSymbol::Blue,ManaSymbol::Black])]),
 ] {for (wanted,pool) in pairs {
  let mut game=play::game();let reducer=definitions("Independent colored reduction",REDUCER);
  game.create_object_from_definition(&reducer[route],play::A,Zone::Battlefield);
  let spell=definitions("Independent multicolor spell",&format!("Mana cost: {cost}\nType: Instant\nYou gain 1 life."));
  let id=game.create_object_from_definition(&spell[route],play::A,Zone::Hand);
  for color in pool {play::give_mana(&mut game,play::A,color,1);}
  cast(&mut game,id,&mut ChooseCost{wanted:wanted.into(),..Default::default()});
  assert_eq!(game.player(play::A).unwrap().mana_pool.total(),0);
 }}}
}
#[test]
fn choice_menu_preserves_all_three_pairs_and_the_player_can_choose_each() {
 for route in 0..2 {for wanted in ["{W}{U}","{W}{B}","{U}{B}"] {
  let mut game=play::game();let reducer=definitions("Independent reduction menu",REDUCER);
  game.create_object_from_definition(&reducer[route],play::A,Zone::Battlefield);
  let spell=definitions("Independent three-color menu","Mana cost: {W}{U}{B}\nType: Instant\nYou gain 1 life.");
  let id=game.create_object_from_definition(&spell[route],play::A,Zone::Hand);
  for color in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black] {play::give_mana(&mut game,play::A,color,1);}
  let mut dm=ChooseCost{wanted:wanted.into(),..Default::default()};cast(&mut game,id,&mut dm);
  assert_eq!(dm.offered.len(),3,"no absent-color no-op may preserve the unreduced WUB total");
  for pair in ["Pay {W}{U}","Pay {W}{B}","Pay {U}{B}"] {assert!(dm.offered.iter().any(|option|option==pair),"{pair}: {:?}",dm.offered);}
  for invented in ["Pay {W}{W}","Pay {U}{U}","Pay {B}{B}"] {assert!(!dm.offered.iter().any(|option|option==invented));}
  assert_eq!(game.player(play::A).unwrap().mana_pool.total(),1);
 }}
}
#[test]
fn announced_hybrid_and_tax_are_applied_once_before_the_cost_branch_is_locked() {
 for route in 0..2 {for taxed in [false,true] {
  let mut game=play::game();let reducer=definitions("Independent hybrid reduction",REDUCER);
  game.create_object_from_definition(&reducer[route],play::A,Zone::Battlefield);
  if taxed {let tax=definitions("Independent one-mana tax","Type: Enchantment\nSpells cost {1} more to cast.");game.create_object_from_definition(&tax[route],play::A,Zone::Battlefield);}
  let spell=definitions("Independent hybrid spell","Mana cost: {W/U}{B}\nType: Instant\nYou gain 1 life.");
  let id=game.create_object_from_definition(&spell[route],play::A,Zone::Hand);
  play::give_mana(&mut game,play::A,ManaSymbol::Black,1);if taxed {play::give_mana(&mut game,play::A,ManaSymbol::Red,1);}
  cast(&mut game,id,&mut ChooseCost{wanted:if taxed{"{1}{B}"}else{"{B}"}.into(),..Default::default()});
  assert_eq!(game.player(play::A).unwrap().mana_pool.total(),0,"tax is charged exactly once");
 }}
}
#[test]
fn reduction_options_do_not_launder_restricted_mana_into_an_illegal_spell_payment() {
 use ironsmith::ability::{ManaPaymentPredicate as P,ManaPaymentPurpose,ManaUsageRestriction,RestrictedManaUnit};
 for route in 0..2 {
  let mut game=play::game();let reducer=definitions("Independent restricted-mana reducer",REDUCER);
  let provider=game.create_object_from_definition(&reducer[route],play::A,Zone::Battlefield);
  let spell=definitions("Independent restricted-mana instant","Mana cost: {W}{U}{B}\nType: Instant\nYou gain 1 life.");
  let id=game.create_object_from_definition(&spell[route],play::A,Zone::Hand);
  play::give_mana(&mut game,play::A,ManaSymbol::White,1);
  game.player_mut(play::A).unwrap().add_restricted_mana(RestrictedManaUnit {
   symbol:ManaSymbol::Blue,source:provider,source_controller:Some(play::A),source_chosen_creature_type:None,
   restrictions:vec![ManaUsageRestriction::PaymentTransaction {restriction:Some(P::All(vec![P::Purpose(ManaPaymentPurpose::CastSpell),P::SourceMatches(ironsmith::ObjectFilter::default().with_type(ironsmith::CardType::Creature))])),on_spend:vec![]}],
  });
  assert!(!compute_legal_actions(&game,play::A).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==id)));
  play::give_mana(&mut game,play::A,ManaSymbol::Blue,1);
  cast(&mut game,id,&mut ChooseCost{wanted:"{W}{U}".into(),..Default::default()});
  assert_eq!(game.player(play::A).unwrap().restricted_mana.len(),1,"the forbidden blue unit remains unspent");
 }
}

#[test]
fn any_color_reduction_uses_generic_only_when_no_colored_requirement_exists() {
 for route in 0..2 {
  let mut game=play::game();let reducer=definitions("Independent exact any-color policy",REDUCER);
  game.create_object_from_definition(&reducer[route],play::A,Zone::Battlefield);
  let spell=definitions("Independent colored-and-generic spell","Mana cost: {1}{W}{U}{B}\nType: Instant\nYou gain 1 life.");
  let id=game.create_object_from_definition(&spell[route],play::A,Zone::Hand);
  for color in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black] {play::give_mana(&mut game,play::A,color,1);}
  let mut dm=ChooseCost{wanted:"{1}{W}{U}".into(),..Default::default()};cast(&mut game,id,&mut dm);
  assert!(!dm.offered.iter().any(|cost|cost=="Pay {W}{U}{B}"),"generic component cannot be reduced while colored requirements remain");
  assert_eq!(dm.offered.len(),3);
  let mut game=play::game();game.create_object_from_definition(&reducer[route],play::A,Zone::Battlefield);
  let spell=definitions("Independent colorless fallback","Mana cost: {2}{C}\nType: Instant\nYou gain 1 life.");
  let id=game.create_object_from_definition(&spell[route],play::A,Zone::Hand);
  play::give_mana(&mut game,play::A,ManaSymbol::Colorless,2);
  cast(&mut game,id,&mut ChooseCost{wanted:"{1}{C}".into(),..Default::default()});
  assert_eq!(game.player(play::A).unwrap().mana_pool.total(),0,"fallback reduces generic, not the C requirement");
 }
}

//! Contextual name-history predicates; requires the separate named-object-name filter package.
#[path="custom_player_named_action_history_support.rs"] mod history;
use history::*;
use ironsmith::{Effect,GameState,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::ChooseSpec;
#[path="custom_final_next_wave_support.rs"] mod support;
#[test]
fn contextual_it_and_that_spell_match_completed_object_names_without_restricting_to_cards() {
 for subject in ["it is an object","that spell is a spell"] {
  let text=format!("Type: Enchantment\nWhenever you cast a spell, if {subject} sharing a name with a card from your sample forge action this turn, you gain 1 life.");
  for observer in support::definitions(&serde_json::json!({"name":"Contextual witness","executable_text":text})).unwrap() {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&observer,A,Zone::Battlefield);
   let maker=game.create_object_from_definition(&definition("Creator","Type: Artifact"),A,Zone::Battlefield);
   let twin=definition("Shared name","Type: Sorcery\nYou gain 1 life.");let historical=game.create_object_from_definition(&twin,A,Zone::Exile);
   complete(&mut game,maker,A,"sample forge",&[historical]);game.move_object_by_effect(historical,Zone::Graveyard).unwrap();game.move_object_by_effect(maker,Zone::Graveyard).unwrap();
   for same in [false,true] {
    let spell=game.create_object_from_definition(&if same{twin.clone()}else{definition("Different name","Type: Sorcery\nYou gain 1 life.")},A,Zone::Stack);
    game.refresh_continuous_state().unwrap();
    let event=ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::SpellCastEvent::from_completed_cast(spell,A,Zone::Hand,&game),Default::default());
    assert_eq!(ironsmith::triggers::check_triggers(&game,&event).len(),usize::from(same),"{subject}: implicit reference must bind the triggering spell, never the observer");
    if same {
     let effect=Effect::new(ironsmith::effects::CopySpellEffect::single(ChooseSpec::SpecificObject(spell)));
     execute_effect(&mut game,&effect,&mut EffectContext::new_default(spell,A)).unwrap();
     let copy=game.stack.last().unwrap().object_id;assert_ne!(copy,spell);
     let event=ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::SpellCastEvent::from_completed_cast(copy,A,Zone::Exile,&game),Default::default());
     assert_eq!(ironsmith::triggers::check_triggers(&game,&event).len(),1,"a same-name spell copy is still an object; adding card-only qualification would change the source condition");
    }
   }
  }
 }
}

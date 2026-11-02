//! Uses the actual private replay owner, not a fabricated pending state.
use super::{WasmGame,ReplayRoot,ReplayOutcome,ReplayDecisionAnswer};
use ironsmith::{GameState,PlayerId,Zone,CardId};
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::ability::Ability;
use ironsmith::effect::Effect;
use ironsmith::static_abilities::{StaticAbility,CompiledStaticAbility};
use ironsmith::target::PlayerFilter;
use ironsmith::decision::LegalAction;
use ironsmith::GameProgress;
use ironsmith::decisions::context::DecisionContext;
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn provider(name:&str,identity:&str)->ironsmith::CardDefinition{
 let definition=CardDefinitionBuilder::new(CardId::new(),identity).card_types(vec![ironsmith::CardType::Sorcery])
     .with_spell_effect(vec![Effect::draw(1)]).build();
 let carrier=Effect::new(ironsmith::effects::CopyFromDefinitionEffect{definition,count:ironsmith::effect::Value::Fixed(1),player:PlayerFilter::You});
 CardDefinitionBuilder::new(CardId::new(),name).card_types(vec![ironsmith::CardType::Enchantment])
     .with_ability(Ability::static_ability(StaticAbility::from_model(CompiledStaticAbility::copy_source_selection_replacement(PlayerFilter::Any,carrier,"Source replacement")))).build()
}
#[test]
fn cast_copy_source_choice_uses_actual_replay_and_preserves_preproposal_atomicity(){
 let mut wasm=WasmGame::new();wasm.game=GameState::new(vec!["A".into(),"B".into()],20);wasm.priority_state=PriorityLoopState::new(2);wasm.trigger_queue=ironsmith::triggers::TriggerQueue::new();
 wasm.game.turn.active_player=A;wasm.game.turn.priority_player=Some(A);wasm.game.turn.phase=ironsmith::game_state::Phase::FirstMain;wasm.game.turn.step=None;
 for (name,identity) in [("First provider","First identity"),("Second provider","Second identity")]{wasm.game.create_object_from_definition(&provider(name,identity),B,Zone::Battlefield);}
 let mut donor=CardDefinitionBuilder::new(CardId::new(),"Original donor").card_types(vec![ironsmith::CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(8,8)).build();
 let mana=ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(1),ironsmith::mana::ManaSymbol::Blue]);
 let method=ironsmith::alternative_cast::AlternativeCastingMethod::cast_from_zone_with_total_cost("Independent price",Zone::Graveyard,ironsmith::cost::TotalCost::from_cost(ironsmith::costs::Cost::mana(mana)),None,false)
     .with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions{cast_copy:true,source_destination_as_cost:Some(Zone::Exile),..Default::default()});
 donor.alternative_casts.push(method);
 let original=wasm.game.create_object_from_definition(&donor,A,Zone::Graveyard);let stable=wasm.game.object(original).unwrap().stable_id;
 wasm.game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);wasm.game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
 let action=ironsmith::decision::compute_legal_actions(&wasm.game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,from_zone:Zone::Graveyard,..} if *spell_id==original)).unwrap();
 let checkpoint=wasm.capture_replay_checkpoint();let root=ReplayRoot::Response(PriorityResponse::PriorityAction(action));
 let result=wasm.execute_with_replay(&checkpoint,&root,&[]).unwrap();
 let ReplayOutcome::NeedsDecision(DecisionContext::SelectOptions(question))=result else{panic!("replacement-order decision: {result:?}")};
 assert_eq!(question.player,A);assert_eq!(question.options.len(),2);assert!(wasm.game.stack.is_empty());assert_eq!(wasm.game.object(original).unwrap().zone,Zone::Graveyard);assert_eq!(wasm.game.player(A).unwrap().mana_pool.total(),2);assert!(!wasm.priority_state.has_pending_action());
 let result=wasm.execute_with_replay(&checkpoint,&root,&[ReplayDecisionAnswer::Options(vec![question.options[0].index])]).unwrap();
 assert!(matches!(result,ReplayOutcome::Complete(GameProgress::NeedsDecisionCtx(_))),"normal payment announcement follows selection: {result:?}");
 let copy=wasm.priority_state.pending_cast.as_ref().expect("normal cast owner").spell_id;
 assert_ne!(copy,original);let proposed=wasm.game.object(copy).unwrap();assert!(matches!(proposed.name.as_str(),"First identity"|"Second identity"));assert_eq!(proposed.kind,ironsmith::object::ObjectKind::SpellCopy);
 assert_eq!(wasm.game.cast_origin_snapshot(copy).unwrap().object_id,original);assert_eq!(wasm.game.object(original).unwrap().zone,Zone::Graveyard);assert_eq!(wasm.game.player(A).unwrap().mana_pool.total(),2);
 assert!(wasm.priority_state.rollback_action(&mut wasm.game));assert_eq!(wasm.game.find_object_by_stable_id(stable),Some(original));assert_eq!(wasm.game.object(original).unwrap().zone,Zone::Graveyard);assert_eq!(wasm.game.player(A).unwrap().mana_pool.total(),2);
}

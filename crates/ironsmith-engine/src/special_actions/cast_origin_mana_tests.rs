use super::*;
#[test]
fn explicitly_retained_mana_operand_survives_original_departure_without_following_it(){
 let player=crate::PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 let original_card=crate::card::CardBuilder::new(crate::ids::CardId::new(),"Original price identity")
     .mana_cost(crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(3),crate::mana::ManaSymbol::Blue]))
     .card_types(vec![crate::types::CardType::Sorcery]).build();
 let proposed_card=crate::card::CardBuilder::new(crate::ids::CardId::new(),"Proposed copy identity")
     .mana_cost(crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(2)]))
     .card_types(vec![crate::types::CardType::Sorcery]).build();
 let original=game.create_object_from_card(&original_card,player,Zone::Graveyard);
 let proposal=game.create_object_from_card(&proposed_card,player,Zone::Stack);
 let snapshot=crate::snapshot::ObjectSnapshot::from_object_id(&game,original).unwrap();
 game.set_cast_origin_snapshot(proposal,snapshot);
 let mut cost=ironsmith_core::DynamicManaCost::from_object_mana_cost(crate::target::ChooseSpec::SpecificObject(original));
 cost.mana_cost_of_cast_origin=true;
 let mut ctx=crate::effects::ExecutionContext::new_default(proposal,player);
 assert_eq!(resolve_dynamic_mana_cost(&game,&cost,&mut ctx).unwrap(),original_card.mana_cost.clone().unwrap());
 let later=game.move_object_by_effect(original,Zone::Exile).unwrap();assert_ne!(later,original);
 assert_eq!(resolve_dynamic_mana_cost(&game,&cost,&mut ctx).unwrap(),original_card.mana_cost.clone().unwrap());
 cost.mana_cost_of_cast_origin=false;
 assert!(resolve_dynamic_mana_cost(&game,&cost,&mut ctx).is_err(),"ordinary object operands do not gain blanket historical lookup");
 cost.mana_cost_of_cast_origin=true;cost.mana_cost_of=Some(Box::new(crate::target::ChooseSpec::SpecificObject(later)));
 assert!(resolve_dynamic_mana_cost(&game,&cost,&mut ctx).is_err(),"a later stable-identity incarnation cannot replace the retained operand");
}

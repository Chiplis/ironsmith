//! Native tests are authored but intentionally unrun during source preparation.
use super::*;
use crate::card::CardBuilder;
use crate::decision::DecisionMaker;
use crate::decisions::context::NumberContext;
use crate::effect::{Effect, EffectId, ExecutionFact};
use crate::effects::{ChooseNumberEffect, EffectExecutor, SequenceEffect, execute_effect};
use crate::ids::CardId;

struct Numeric { value: u32, pending: bool }
impl DecisionMaker for Numeric {
    fn decide_number(&mut self, _: &GameState, _: &NumberContext) -> u32 { self.value }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn source(game: &mut GameState) -> ObjectId {
    let card=CardBuilder::new(CardId::new(),"Numeric source").card_types(vec![CardType::Artifact]).build();
    game.create_object_from_card(&card,PlayerId::from_index(0),Zone::Battlefield)
}
fn owner(source:ObjectId)->crate::source_numbers::NumberChoiceOwner{
    crate::linked_exile::LinkedExileOwner{host:source,
        pair:ironsmith_core::LinkedExilePair{definition:ironsmith_core::LinkedExileDefinition([77;32]),pair:0},
        acquisition:crate::linked_exile::LinkedExileAcquisition::Printed}
}
fn chosen(game:&GameState,source:ObjectId)->Option<u32>{
    game.numeric_choice_memory(source).get(&owner(source)).map(|record|record.number)
}
fn reveal_query(color_id: EffectId) -> PriorEffectMetricQuery {
    let mut query=PriorEffectMetricQuery::new(EffectMetricSource::AffectedObjects,EffectMetric::Count)
        .with_action(ironsmith_core::PriorEffectAction::Revealed)
        .with_filter(crate::target::ObjectFilter::default().of_chosen_color());
    query.color_choice=Some(ironsmith_core::ColorChoiceReference::Effect(color_id));query
}
#[test]
fn persistent_and_local_numeric_receipts_are_disjoint_and_pending_choices_do_not_publish() {
    let mut game=crate::tests::test_helpers::setup_two_player_game();let a=PlayerId::from_index(0);let source=source(&mut game);
    for number in [0,7,u32::MAX] {
        let mut dm=Numeric{value:number,pending:false};
        let out=ChooseNumberEffect::unbounded(PlayerFilter::You).with_source_retention().execute(&mut game,&mut ExecutionContext::new(source,a,&mut dm).with_source_number_owner(Some(owner(source)))).unwrap();
        assert_eq!(out.as_count(),Some(i64::from(number)));assert_eq!(chosen(&game,source),Some(number));
    }
    let mut local=Numeric{value:2,pending:false};
    ChooseNumberEffect::new(PlayerFilter::You,0,7).execute(&mut game,&mut ExecutionContext::new(source,a,&mut local)).unwrap();
    assert_eq!(chosen(&game,source),Some(u32::MAX));
    let mut pending=Numeric{value:5,pending:true};
    let out=ChooseNumberEffect::new(PlayerFilter::You,0,7).with_source_retention().execute(&mut game,&mut ExecutionContext::new(source,a,&mut pending).with_source_number_owner(Some(owner(source)))).unwrap();
    assert!(out.execution_facts.is_empty());assert_eq!(chosen(&game,source),Some(u32::MAX));
}
#[test]
fn invalid_source_choice_rolls_back_earlier_effect_and_preserves_the_last_actual_choice() {
    let mut game=crate::tests::test_helpers::setup_two_player_game();let a=PlayerId::from_index(0);let source=source(&mut game);
    game.set_number_for_acquisition(owner(source),3).unwrap();let before=game.player(a).unwrap().life;
    let mut invalid=Numeric{value:8,pending:false};
    let sequence=Effect::new(SequenceEffect::new(vec![Effect::gain_life(5),Effect::new(ChooseNumberEffect::new(PlayerFilter::You,0,7).with_source_retention())]));
    assert!(execute_effect(&mut game,&sequence,&mut ExecutionContext::new(source,a,&mut invalid).with_source_number_owner(Some(owner(source)))).is_err());
    assert_eq!(game.player(a).unwrap().life,before);assert_eq!(chosen(&game,source),Some(3));
    let positive=ChooseNumberEffect{chooser:PlayerFilter::You,min:1,max:None,source_owned:false};
    invalid.value=0;assert!(positive.execute(&mut game,&mut ExecutionContext::new(source,a,&mut invalid).with_source_number_owner(Some(owner(source)))).is_err());
}
#[test]
fn exact_color_and_reveal_receipts_survive_unrelated_choices_mutations_and_departure() {
    let mut game=crate::tests::test_helpers::setup_two_player_game();let a=PlayerId::from_index(0);let source=source(&mut game);
    let blue=crate::color::Color::Blue;let red=crate::color::Color::Red;
    let card=CardBuilder::new(CardId::new(),"Revealed blue").card_types(vec![CardType::Instant])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Blue]])).build();
    let card_id=game.create_object_from_card(&card,a,Zone::Hand);
    let memory=crate::effect::OutcomeObjectMemory::from_object_id(&game,card_id).unwrap();
    let color_id=EffectId(2);let reveal_id=EffectId(7);let query=reveal_query(color_id);
    let mut ctx=ExecutionContext::new_default(source,a);
    ctx.store_outcome(color_id,EffectOutcome::count(1).with_execution_fact(ExecutionFact::ChosenColor(blue)));
    ctx.store_outcome(reveal_id,EffectOutcome::count(1).with_execution_fact(ExecutionFact::RevealedCards(vec![memory])));
    ctx.store_outcome(EffectId(9),EffectOutcome::count(1).with_execution_fact(ExecutionFact::ChosenColor(red)));
    ctx.store_outcome(EffectId(10),EffectOutcome::count(0).with_execution_fact(ExecutionFact::RevealedCards(vec![])));
    ctx.store_outcome(EffectId(11),EffectOutcome::count(700).with_execution_fact(ExecutionFact::ChosenNumber(700)));
    game.set_chosen_color(source,red);game.move_object_by_effect(card_id,Zone::Graveyard).unwrap();game.move_object_by_effect(source,Zone::Graveyard).unwrap();
    assert_eq!(resolve_prior_effect_metric(&game,&ctx,reveal_id,&query).unwrap(),1);
    assert_eq!(resolve_prior_effect_metric(&game,&ctx,EffectId(10),&query).unwrap(),0,"completed empty reveal is known zero");
}
#[test]
fn absent_or_ambiguous_color_and_reveal_receipts_never_become_zero_or_source_color() {
    let mut game=crate::tests::test_helpers::setup_two_player_game();let a=PlayerId::from_index(0);let source=source(&mut game);
    game.set_chosen_color(source,crate::color::Color::Blue);
    let mut ctx=ExecutionContext::new_default(source,a);let color=EffectId(2);let reveal=EffectId(3);let query=reveal_query(color);
    for color_outcome in [None,Some(EffectOutcome::count(1)),Some(EffectOutcome::count(1)
        .with_execution_fact(ExecutionFact::ChosenColor(crate::color::Color::Blue))
        .with_execution_fact(ExecutionFact::ChosenColor(crate::color::Color::Red)))] {
        ctx.effect_outcomes.clear();if let Some(out)=color_outcome {ctx.store_outcome(color,out);}
        ctx.store_outcome(reveal,EffectOutcome::count(0).with_execution_fact(ExecutionFact::RevealedCards(vec![])));
        assert!(matches!(resolve_prior_effect_metric(&game,&ctx,reveal,&query),Err(ExecutionError::IncompleteEvidence(_))));
    }
    ctx.store_outcome(color,EffectOutcome::count(1).with_execution_fact(ExecutionFact::ChosenColor(crate::color::Color::Blue)));
    ctx.store_outcome(reveal,EffectOutcome::count(0));
    assert!(matches!(resolve_prior_effect_metric(&game,&ctx,reveal,&query),Err(ExecutionError::IncompleteEvidence(_))));
}
#[cfg(feature="serialization")]
#[test]
fn numeric_snapshot_codec_does_not_turn_unavailable_acquisition_memory_into_never_chosen() {
    let mut game=crate::tests::test_helpers::setup_two_player_game();let source=source(&mut game);let owner=owner(source);
    let fresh=ObjectSnapshot::from_object(game.object(source).unwrap(),&game);
    assert!(fresh.numeric_choice_memory.as_ref().unwrap().is_empty());
    for number in [0,u32::MAX] {
        game.set_number_for_acquisition(owner.clone(),number).unwrap();
        let snapshot=ObjectSnapshot::from_object(game.object(source).unwrap(),&game);
        assert_eq!(snapshot.numeric_choice_memory.as_ref().unwrap()[&owner].number,number);
        let restored:ObjectSnapshot=serde_json::from_value(serde_json::to_value(snapshot).unwrap()).unwrap();
        assert!(restored.numeric_choice_memory.is_none());
    }
}

#[test]
fn pending_and_failed_new_acquisitions_do_not_consume_public_group_ordinals() {
    let mut game=crate::tests::test_helpers::setup_two_player_game();let a=PlayerId::from_index(0);let source=source(&mut game);
    let first=owner(source);game.set_number_for_acquisition(first.clone(),2).unwrap();
    let mut second=first.clone();second.pair.pair=1;
    let before=game.numeric_choice_memory(source);
    let mut pending=Numeric{value:4,pending:true};
    ChooseNumberEffect::new(PlayerFilter::You,0,7).with_source_retention().execute(&mut game,
        &mut ExecutionContext::new(source,a,&mut pending).with_source_number_owner(Some(second.clone()))).unwrap();
    assert_eq!(game.numeric_choice_memory(source),before);
    let mut invalid=Numeric{value:9,pending:false};
    let transaction=Effect::new(SequenceEffect::new(vec![Effect::gain_life(1),Effect::new(ChooseNumberEffect::new(PlayerFilter::You,0,7).with_source_retention())]));
    assert!(execute_effect(&mut game,&transaction,&mut ExecutionContext::new(source,a,&mut invalid)
        .with_source_number_owner(Some(second.clone()))).is_err());
    assert_eq!(game.numeric_choice_memory(source),before);
    // Allocate the second ordinal successfully, then fail a later instruction.
    // Rollback must undo the completed record, not merely reject a bad prompt.
    invalid.value=4;
    let later_failure=Effect::new(SequenceEffect::new(vec![
        Effect::new(ChooseNumberEffect::new(PlayerFilter::You,0,7).with_source_retention()),
        Effect::gain_life(Value::SourceChosenNumber{if_unset:Some(0),pair:None}),
    ]));
    assert!(execute_effect(&mut game,&later_failure,&mut ExecutionContext::new(source,a,&mut invalid)
        .with_source_number_owner(Some(second.clone()))).is_err());
    assert_eq!(game.numeric_choice_memory(source),before);
    execute_effect(&mut game,&Effect::new(ChooseNumberEffect::new(PlayerFilter::You,0,7).with_source_retention()),
        &mut ExecutionContext::new(source,a,&mut invalid).with_source_number_owner(Some(second.clone()))).unwrap();
    let memory=game.numeric_choice_memory(source);assert_eq!(memory[&first].public_group,0);assert_eq!(memory[&second].public_group,1);
    let saved=game.clone();game.set_number_for_acquisition(second.clone(),6).unwrap();
    assert_eq!(game.numeric_choice_memory(source)[&second].public_group,1);
    game=saved;assert_eq!(game.numeric_choice_memory(source)[&second].number,4);
}

#[cfg(feature="serialization")]
#[test]
fn public_snapshot_cannot_restore_native_acquisition_or_turn_missing_history_into_zero() {
    let mut game=crate::tests::test_helpers::setup_two_player_game();let a=PlayerId::from_index(0);let source=source(&mut game);
    let owner=owner(source);game.set_number_for_acquisition(owner.clone(),0).unwrap();
    let native=ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(),&game);
    let public:ObjectSnapshot=serde_json::from_value(serde_json::to_value(&native).unwrap()).unwrap();
    assert!(public.numeric_choice_memory.is_none());assert!(public.ability_origins.is_none());
    game.move_object_by_effect(source,Zone::Graveyard).unwrap();game.turn_store.turn_history.clear_for_new_turn();
    let value=Value::SourceChosenNumber{if_unset:Some(0),pair:Some(owner.pair)};
    let context=ExecutionContext::new_default(source,a).with_source_number_owner(Some(owner.clone())).with_source_snapshot(native);
    assert_eq!(resolve_value_wide(&game,&value,&context).unwrap(),0);
    let context=ExecutionContext::new_default(source,a).with_source_number_owner(Some(owner)).with_source_snapshot(public);
    assert!(matches!(resolve_value_wide(&game,&value,&context),Err(ExecutionError::IncompleteEvidence(_))));
}

#[test]
fn public_proof_requires_pairs_for_source_owned_entry_upkeep_and_activation_producers() {
    for kind in 0..3 {
        for source_owned in [false,true] {
            for paired in [false,true] {
                let mut game=crate::tests::test_helpers::setup_two_player_game();let source=source(&mut game);
                let choose=Effect::new(ChooseNumberEffect{chooser:PlayerFilter::You,min:0,max:Some(7),source_owned});
                let mut program=crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::new(SequenceEffect::new(vec![choose])),
                ]);
                if paired {program.source_number_pair=Some(owner(source).pair);}
                let ability=match kind {
                    0=>crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::from_model(
                        crate::static_abilities::CompiledStaticAbility::as_enters_effect_program(program,"this artifact",false,false,None))),
                    1=>crate::ability::Ability::triggered(crate::triggers::Trigger::beginning_of_upkeep(PlayerFilter::You),program),
                    _=>crate::ability::Ability::activated(crate::cost::TotalCost::free(),program),
                };
                game.object_mut(source).unwrap().abilities_mut().push(ability);
                let result=crate::source_numbers::public_proof(&game,source,true);
                if source_owned && !paired {
                    assert!(matches!(result,Err(ExecutionError::IncompleteEvidence(_))),"producer kind {kind} needs its own pair");
                } else {
                    let proof=result.unwrap();assert!(proof.records.is_empty());
                    assert_eq!(proof.bindings.len(),usize::from(paired));
                    assert!(proof.bindings.iter().all(|binding|binding.group.is_none()));
                }
                assert_eq!(crate::source_numbers::public_proof(&game,source,false).unwrap(),Default::default(),
                    "hidden producer identity remains redacted even when public evidence is unavailable");
            }
        }
    }
}

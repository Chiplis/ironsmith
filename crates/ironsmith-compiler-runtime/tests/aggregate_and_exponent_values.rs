//! cf8 p01: values that were silently simplified — 2ˣ read as 2, "card type
//! among spells" read as a spell count, and a set-wide "card types among
//! them" read per card. Source-authored, deliberately unrun.
use ironsmith::effect::Value;

#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn mathemagics_draws_two_to_the_x() {
    for definition in support::definitions("Mathemagics") {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("PowerOfTwo(X)"), "{debug}");
        let text = support::rendered(&definition);
        assert!(text.contains("2ˣ"), "{text}");
    }
    let _ = Value::PowerOfTwo(Box::new(Value::X));
}

#[test]
fn april_oneil_counts_card_types_among_spells_cast() {
    for definition in support::definitions("April O'Neil, Hacktivist") {
        fn draw_values(effect: &ironsmith::Effect, values: &mut Vec<Value>) {
            if let Some(draw)=effect.downcast_ref::<ironsmith::effects::DrawCardsEffect>() {
                assert_eq!(draw.player,ironsmith::target::PlayerFilter::You);
                values.push(draw.count.clone());
            }
            effect.visit_child_effects(&mut |child|draw_values(child,values));
        }
        let program=definition.abilities.iter().find_map(|ability|match &ability.kind {
            ironsmith::AbilityKind::Triggered(trigger)=>Some(trigger.effects.clone()),_=>None,
        }).expect("the complete original end-step ability is retained");
        let mut values=Vec::new();for effect in program.all_effects(){draw_values(effect,&mut values);}
        assert_eq!(values.len(),1);
        assert!(matches!(values[0].unhinted(),Value::CardTypesAmongSpellsCastThisTurn {player:ironsmith::target::PlayerFilter::You,..}));
        let text = support::rendered(&definition);
        assert!(text.contains("card type") && text.contains("among spells you've cast this turn"),"{text}");
        support::assert_no_internal_markers("April O'Neil, Hacktivist",&text);
        april_actual_cast_history_draw(&definition,&program);
    }
}

#[test]
fn winter_constrains_the_exiled_set_not_each_card() {
    for definition in support::definitions("Winter, Cynical Opportunist") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("DistinctCardTypes"), "{debug}");
        assert!(!debug.contains("card_type_count: Some"), "{debug}");
    }
}

/// Populate history through ordinary casting/finalization; a repeated type is
/// counted once, multitype spells contribute every type, and an opponent's
/// additional type does not belong to "you". Resolved spells still contribute.
fn april_actual_cast_history_draw(definition:&ironsmith::CardDefinition,program:&ironsmith::resolution::ResolutionProgram){
 use ironsmith::cards::builders::CardDefinitionBuilder;
 use ironsmith::{CardId,CardType,Effect,EffectContext,GameState,PlayerId,PowerToughness,Zone};
 use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
 const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
 let source=game.create_object_from_definition(definition,A,Zone::Battlefield);
 let zero=ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(0)]);
 for (i,(actor,types)) in [
  (A,vec![CardType::Artifact,CardType::Creature]),(A,vec![CardType::Sorcery]),
  (A,vec![CardType::Sorcery]),(A,vec![CardType::Sorcery]),(A,vec![CardType::Instant]),
  (B,vec![CardType::Kindred,CardType::Instant]),
 ].into_iter().enumerate(){
  let creature=types.contains(&CardType::Creature);let mut builder=CardDefinitionBuilder::new(CardId::new(),format!("Actual cast history witness {i}")).mana_cost(zero.clone()).card_types(types);
  if creature{builder=builder.power_toughness(PowerToughness::fixed(1,3));}else{builder=builder.with_spell_effect(vec![Effect::gain_life(1)]);}
  let spell=game.create_object_from_definition(&builder.build(),actor,Zone::Hand);game.turn.priority_player=Some(actor);
  let action=compute_legal_actions(&game,actor).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==spell)).expect("ordinary zero-mana witness castable");
  let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
  let mut progress=apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
  for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}
  assert!(!state.has_pending_action());while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
 }
 let library=CardDefinitionBuilder::new(CardId::new(),"Draw supply").card_types(vec![CardType::Land]).build();for _ in 0..10{game.create_object_from_definition(&library,A,Zone::Library);}
 let before=game.player(A).unwrap().hand.len();let effect=Effect::new(ironsmith::effects::SequenceEffect::new(program.all_effects_owned()));
 ironsmith::effects::execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A)).unwrap();assert_eq!(game.player(A).unwrap().hand.len()-before,4,"five own casts have four distinct types; opponent's Kindred is excluded");
 game.next_turn();let before=game.player(A).unwrap().hand.len();ironsmith::effects::execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A)).unwrap();assert_eq!(game.player(A).unwrap().hand.len(),before,"the value is current-turn history, not lifetime or resolved cards on the battlefield");
}

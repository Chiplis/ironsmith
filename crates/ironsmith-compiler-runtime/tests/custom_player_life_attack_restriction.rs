//! Actual attack-target class and current player life remain independent.
#[path = "custom_final_next_wave_support.rs"] mod source;
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::combat_state::{AttackTarget,CombatState};
use ironsmith::decision::{AttackerDeclaration,compute_legal_attackers};
use ironsmith::triggers::TriggerQueue;
const A:PlayerId=PlayerId(0);
const B:PlayerId=PlayerId(1);
const C:PlayerId=PlayerId(2);
fn exercise_targets(definition:&CardDefinition,threshold:i32) {
    let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);
    let attacker=game.create_object_from_definition(definition,A,Zone::Battlefield);
    exercise_existing_targets(game,attacker,threshold);
}
fn exercise_existing_targets(mut game:GameState,attacker:ironsmith::ObjectId,threshold:i32) {
    game.turn.active_player=A;game.turn.priority_player=Some(A);
    game.turn.phase=ironsmith::Phase::Combat;game.turn.step=Some(ironsmith::Step::DeclareAttackers);
    game.player_mut(C).unwrap().life=1;
    game.remove_summoning_sickness(attacker);
    let walker_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Attack-target control","Type: Planeswalker\nLoyalty: 3",false).unwrap();
    let walker=game.create_object_from_definition(&walker_definition,B,Zone::Battlefield);
    let battle_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Battle target control","Type: Battle — Siege\nDefense: 3",false).unwrap();
    let battle=game.create_object_from_definition(&battle_definition,A,Zone::Battlefield);
    assert!(game.set_battle_protector(battle,B));
    for life in [threshold-1,threshold,threshold+1,threshold-1] {
        game.player_mut(B).unwrap().life=life;
        game.refresh_continuous_state().unwrap();
        let legal=compute_legal_attackers(&game,&CombatState::default());
        let option=legal.iter().find(|option|option.creature==attacker).unwrap();
        assert_eq!(option.valid_targets.contains(&AttackTarget::Player(B)),life<threshold);
        assert!(option.valid_targets.contains(&AttackTarget::Player(C)));
        assert!(option.valid_targets.contains(&AttackTarget::Planeswalker(walker)));
        assert!(option.valid_targets.contains(&AttackTarget::Battle(battle)));
        for target in [AttackTarget::Player(B),AttackTarget::Planeswalker(walker),AttackTarget::Battle(battle)] {
            let should_allow=life<threshold || !matches!(&target,AttackTarget::Player(_));
            let mut attempted=game.clone();let mut combat=CombatState::default();
            let result=ironsmith::game_loop::apply_attacker_declarations(&mut attempted,&mut combat,&mut TriggerQueue::new(),&[
                AttackerDeclaration {creature:attacker,target:target.clone()},
            ]);
            assert_eq!(result.is_ok(),should_allow,"life {life}, target {target:?}: {result:?}");
        }
    }
}
#[test]
fn direct_player_threshold_does_not_ban_their_planeswalkers_or_battles() {
    for threshold in [10,17] {
        let input=serde_json::json!({"name":"Player-life restriction control","executable_text":format!(
            "Type: Creature\nPower/Toughness: 3/3\nThis creature can't attack players with {threshold} or more life.")});
        for definition in source::definitions(&input).unwrap() {exercise_targets(&definition,threshold);}
    }
}
#[test]
fn complete_actual_source_keeps_the_typed_rule_on_its_real_token_blueprint() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-glitch-fragments.json")).unwrap();
    let card=fixture["mechanics"].as_array().unwrap().iter()
        .flat_map(|row|row["representative_cards"].as_array().unwrap())
        .find(|card|card["source_card_key"]=="Manifest Corruption_OTH").unwrap();
    assert!(card["original_text"].as_str().unwrap().contains("can't attack players with 10 or more life"));
    let faces=source::linked_definitions(card).unwrap();assert_eq!(faces.len(),1);
    for definition in &faces[0] {
        let program=definition.spell_effect.as_ref().unwrap();
        fn visit(effect:&ironsmith::Effect,tokens:&mut Vec<CardDefinition>) {
            if let Some(create)=effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>() {tokens.push(create.token.clone());}
            effect.visit_child_effects(&mut |child|visit(child,tokens));
        }
        let mut blueprints=Vec::new();for effect in program.all_effects(){visit(effect,&mut blueprints);}
        assert_eq!(blueprints.len(),1,"transparent result tags preserve exactly one token creation");
        assert_eq!(blueprints[0].card.card_types,vec![ironsmith::CardType::Artifact,ironsmith::CardType::Creature]);
        assert!(blueprints[0].card.subtypes.contains(&ironsmith::Subtype::Demon));
        exercise_targets(&blueprints[0],10);
        // Resolve the complete source program with zero optional payments.
        // The produced token must retain the restriction, not only its blueprint.
        let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);
        let spell=game.create_object_from_definition(definition,A,Zone::Stack);
        let sequence=ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(program.all_effects_owned()));
        ironsmith::effects::execute_effect(&mut game,&sequence,&mut ironsmith::effects::EffectContext::new_default(spell,A)).unwrap();
        let tokens:Vec<_>=game.battlefield.iter().copied().filter(|id|game.object(*id).is_some_and(|o|o.kind==ironsmith::ObjectKind::Token)).collect();
        assert_eq!(tokens.len(),1,"complete source must create exactly one actual token");
        let token=tokens[0];let object=game.object(token).unwrap();
        assert_eq!(object.card_types.as_slice(),&[ironsmith::CardType::Artifact,ironsmith::CardType::Creature]);
        assert!(object.subtypes.contains(&ironsmith::Subtype::Demon));
        assert_eq!(game.calculated_power(token),Some(6));assert_eq!(game.calculated_toughness(token),Some(6));
        exercise_existing_targets(game,token,10);
    }
}

//! Complete operative Coalesce family, independent of reminder-seed associations.
#[path = "custom_final_next_wave_support.rs"] mod source;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{compute_legal_actions,LegalAction,SelectFirstDecisionMaker};
const A:PlayerId=PlayerId(0);
fn fixture()->serde_json::Value {
    serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/coalesce-complete-source-fulfillment.json")).unwrap()
}
fn actual_card(key:&str)->serde_json::Value {
    fixture()["mechanics"].as_array().unwrap().iter()
        .flat_map(|row|row["representative_cards"].as_array().unwrap())
        .find(|card|card["source_card_key"]==key).unwrap().clone()
}
#[test]
fn every_operative_physical_source_keeps_its_complete_body_and_selected_price_on_both_routes() {
    let fixture=fixture();let mut failures=Vec::new();let mut keys=std::collections::BTreeSet::new();
    let mut counts=std::collections::BTreeMap::new();let mut faces=0;
    for row in fixture["mechanics"].as_array().unwrap() {
        let scope=if row["scope"]=="revolution" {"rev"} else {"msem"};
        for card in row["representative_cards"].as_array().unwrap() {
            let key=format!("{scope}:{}",card["source_card_key"].as_str().unwrap());
            assert!(keys.insert(key.clone()),"duplicate physical source {key}");
            *counts.entry(scope).or_insert(0usize)+=1;
            if card["rewrite_status"]!="frontend_materialized_unverified" {
                failures.push(format!("{key}: requires actual frontend materialization, status {}",card["rewrite_status"]));continue;
            }
            match source::linked_definitions(card) {
                Err(error)=>failures.push(format!("{key}: {error}")),
                Ok(definitions)=> {
                    if definitions.len()!=card["source_faces"].as_array().unwrap().len() {
                        failures.push(format!("{key}: source face count changed"));continue;
                    }
                    for [native,artifact] in definitions {
                        faces+=1;
                        // Runtime Effect equality is executor allocation identity.
                        // Compare complete typed payloads instead: preserve every
                        // program/ID/label/source field, not only rendered text.
                        let typed=|definition:&CardDefinition|->Result<serde_json::Value,String>{
                            let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition.clone())
                                .map_err(|error|format!("typed source encoding: {error:?}"))?;
                            serde_json::to_value(wire).map_err(|error|error.to_string())
                        };
                        match (typed(&native),typed(&artifact)) {
                            (Ok(native),Ok(artifact)) if native!=artifact=>failures.push(format!("{key}: complete typed native/artifact source payload differs")),
                            (Err(error),_)=>failures.push(format!("{key}: native {error}")),
                            (_,Err(error))=>failures.push(format!("{key}: artifact {error}")),
                            _=>{},
                        }
                        for definition in [native,artifact] {
                            let coalesce:Vec<_>=definition.alternative_casts.iter().filter(|method| {
                                method.cast_from_zone()==Zone::Graveyard
                                    && method.battlefield_instructions().resolution_entry_creature_subtypes
                                        .as_ref().is_some_and(|types|types.contains(&ironsmith::Subtype::Horror))
                            }).collect();
                            if coalesce.len()!=1 {failures.push(format!("{key}: expected exactly one sourced Coalesce permission"));continue;}
                            let instructions=coalesce[0].battlefield_instructions();
                            if !instructions.spell_copies_cannot_enter || !instructions.keep_kindred_creature_types_on_entry {
                                failures.push(format!("{key}: missing copied-spell or Kindred entry instruction"));
                            }
                            if !instructions.departure_replacements.iter().any(|redirect|redirect.from_zone==Zone::Stack
                                && redirect.destination==Zone::Exile && redirect.to_zone.is_none()
                                && redirect.excluded_to_zones.contains(&Zone::Battlefield)) {
                                failures.push(format!("{key}: selected spell needs destination-aware stack-exit replacement"));
                            }
                            if !instructions.departure_replacements.iter().any(|redirect|redirect.from_zone==Zone::Battlefield
                                && redirect.destination==Zone::Exile && redirect.to_zone.is_none()) {
                                failures.push(format!("{key}: selected permanent needs incarnation-scoped departure replacement"));
                            }
                            if let Some(expected)=card["source_coalesce_cost"].as_str() {
                                if coalesce[0].mana_cost().map(|cost|cost.to_oracle()).as_deref()!=Some(expected) {
                                    failures.push(format!("{key}: selected price differs from original {expected}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let expected:std::collections::BTreeSet<_>=fixture["physical_source_keys"].as_array().unwrap().iter().map(|key|key.as_str().unwrap().to_string()).collect();
    assert_eq!(keys,expected);assert_eq!(keys.len(),73);assert_eq!(counts["msem"],56);assert_eq!(counts["rev"],17);
    assert!(failures.is_empty(),"All complete-source failures:\n{}",failures.join("\n"));assert_eq!(faces,73);
}
fn cast(game:&mut GameState,source:ObjectId) {
    use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
    let action=compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source)).unwrap();
    let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
    let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {break;}
        let ironsmith::GameProgress::NeedsDecisionCtx(context)=progress else {panic!("{progress:?}");};
        progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&context,&mut dm).unwrap();
    }
    assert!(!state.has_pending_action());
}
fn setup(definition:&CardDefinition,green:u32)->(GameState,ObjectId) {
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
    game.register_linked_face_definition(definition);
    let source=game.create_object_from_definition(definition,A,Zone::Graveyard);
    game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green,green);
    (game,source)
}
fn basic_land()->CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition("Source search control","Type: Basic Land — Forest",false).unwrap()
}
#[test]
fn additional_kindred_source_runs_its_land_search_before_entering_as_eldrazi_horror() {
    let card=actual_card("Thrive in Exotic Chaos_GQC");
    for definition in source::linked_definitions(&card).unwrap().pop().unwrap() {
        let(mut game,source)=setup(&definition,7);
        let land=game.create_object_from_definition(&basic_land(),A,Zone::Library);
        let land_stable=game.object(land).unwrap().stable_id;
        cast(&mut game,source);let stable=game.object(game.stack[0].object_id).unwrap().stable_id;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
        let land=game.find_object_by_stable_id(land_stable).unwrap();assert_eq!(game.object(land).unwrap().zone,Zone::Battlefield);assert!(game.is_tapped(land));
        let permanent=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.current_power(permanent),Some(4));assert_eq!(game.current_toughness(permanent),Some(5));
        assert!(game.current_has_card_type(permanent,ironsmith::CardType::Creature));
        let types=game.current_subtypes(permanent).unwrap();assert!(types.contains(&ironsmith::Subtype::Eldrazi));assert!(types.contains(&ironsmith::Subtype::Horror));
        assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
        ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(permanent),Zone::Graveyard,false)),&mut ironsmith::effects::EffectContext::new_default(permanent,A)).unwrap();
        let departed=game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap();assert_eq!(departed.zone,Zone::Exile);
        assert!(departed.card_types.contains(&ironsmith::CardType::Kindred));assert!(departed.card_types.contains(&ironsmith::CardType::Sorcery));assert!(!departed.card_types.contains(&ironsmith::CardType::Creature));
    }
}
#[test]
fn additional_search_partition_source_puts_one_found_land_tapped_and_the_other_in_hand() {
    let card=actual_card("Clear the Way Home_RWR");
    for definition in source::linked_definitions(&card).unwrap().pop().unwrap() {
        let(mut game,source)=setup(&definition,8);
        let lands:Vec<_>=(0..2).map(|_| {
            let land=game.create_object_from_definition(&basic_land(),A,Zone::Library);game.object(land).unwrap().stable_id
        }).collect();
        cast(&mut game,source);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
        let zones:Vec<_>=lands.iter().map(|stable|game.object(game.find_object_by_stable_id(*stable).unwrap()).unwrap().zone).collect();
        assert_eq!(zones.iter().filter(|zone|**zone==Zone::Hand).count(),1);
        assert_eq!(zones.iter().filter(|zone|**zone==Zone::Battlefield).count(),1);
        let land=lands.iter().map(|stable|game.find_object_by_stable_id(*stable).unwrap()).find(|id|game.object(*id).unwrap().zone==Zone::Battlefield).unwrap();
        assert!(game.is_tapped(land));assert!(game.player(A).unwrap().library.is_empty());
    }
}
#[test]
fn additional_repeated_exile_source_collects_every_iteration_but_not_unrelated_exiled_cards() {
    struct RepeatTwice { remaining:usize, prompts:usize }
    impl ironsmith::decision::DecisionMaker for RepeatTwice {
        fn decide_boolean(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::BooleanContext)->bool {
            if ctx.description==ironsmith_core::RepeatProcessPromptKind::MayRepeatAnyNumberOfTimes.prompt_text() {
                self.prompts+=1;
                if self.remaining==0 {return false;}
                self.remaining-=1;return ctx.can_accept;
            }
            ctx.can_accept
        }
        fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId> {
            ironsmith::decision::DecisionMaker::decide_objects(&mut SelectFirstDecisionMaker,game,ctx)
        }
        fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize> {
            ironsmith::decision::DecisionMaker::decide_options(&mut SelectFirstDecisionMaker,game,ctx)
        }
    }
    let card=actual_card("Black Hole Musings_BYK");
    for definition in source::linked_definitions(&card).unwrap().pop().unwrap() {
        let(mut game,source)=setup(&definition,0);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black,7);
        let filler=ironsmith_compiler_runtime::compile_to_runtime_definition("Iteration receipt control","Type: Artifact",false).unwrap();
        let prior:Vec<_>=[A,PlayerId(1)].into_iter().map(|owner| {
            let id=game.create_object_from_definition(&filler,owner,Zone::Exile);game.object(id).unwrap().stable_id
        }).collect();
        let library:Vec<_>=(0..5).map(|_| {
            let id=game.create_object_from_definition(&filler,A,Zone::Library);game.object(id).unwrap().stable_id
        }).collect();
        cast(&mut game,source);let stable=game.object(game.stack[0].object_id).unwrap().stable_id;
        let mut dm=RepeatTwice {remaining:2,prompts:0};
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert_eq!(dm.prompts,3);assert_eq!(dm.remaining,0);
        assert_eq!(game.player(A).unwrap().life,17,"loss belongs to all three process iterations");
        let zone_of=|stable|game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone;
        assert_eq!(library.iter().filter(|stable|zone_of(**stable)==Zone::Hand).count(),2,"hand choice ranges over every actual exile receipt");
        assert_eq!(library.iter().filter(|stable|zone_of(**stable)==Zone::Exile).count(),1);
        assert_eq!(library.iter().filter(|stable|zone_of(**stable)==Zone::Library).count(),2);
        assert!(prior.iter().all(|stable|zone_of(*stable)==Zone::Exile),"this-way collection excludes unrelated old exile cards");
        let permanent=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.current_power(permanent),Some(3),"CDA counts all cards currently exiled, including other players' cards");
        assert_eq!(game.current_toughness(permanent),Some(7));
    }
}
#[test]
fn additional_entry_cast_source_mills_every_player_and_keeps_optional_cast_actor_after_departure() {
    struct Accept(bool);
    impl ironsmith::decision::DecisionMaker for Accept {
        fn decide_boolean(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::BooleanContext)->bool {self.0&&ctx.can_accept}
        fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId> {
            ironsmith::decision::DecisionMaker::decide_objects(&mut SelectFirstDecisionMaker,game,ctx)
        }
        fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target> {
            ironsmith::decision::DecisionMaker::decide_targets(&mut SelectFirstDecisionMaker,game,ctx)
        }
        fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize> {
            ironsmith::decision::DecisionMaker::decide_options(&mut SelectFirstDecisionMaker,game,ctx)
        }
    }
    let card=actual_card("A Vision Too Vast_BYK");
    for definition in source::linked_definitions(&card).unwrap().pop().unwrap() {
        for accept in [false,true] {
            let(mut game,source)=setup(&definition,0);let b=PlayerId(1);
            game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,7);
            let gift=ironsmith_compiler_runtime::compile_to_runtime_definition("Foreign graveyard cast control","Mana cost: {3}{R}\nType: Sorcery\nYou gain 5 life.",false).unwrap();
            let gift=game.create_object_from_definition(&gift,b,Zone::Graveyard);let gift_stable=game.object(gift).unwrap().stable_id;
            for owner in [A,b] {for _ in 0..4 {game.create_object_from_definition(&basic_land(),owner,Zone::Library);}}
            cast(&mut game,source);let stable=game.object(game.stack[0].object_id).unwrap().stable_id;
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
            assert!(game.player(A).unwrap().library.is_empty());assert!(game.player(b).unwrap().library.is_empty());
            assert_eq!(game.player(A).unwrap().graveyard.len(),4);assert_eq!(game.player(b).unwrap().graveyard.len(),5);
            let mut queue=ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.stack.len(),1);assert!(game.stack[0].is_ability);
            let permanent=game.find_object_by_stable_id(stable).unwrap();
            ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(permanent),Zone::Hand,false)),&mut ironsmith::effects::EffectContext::new_default(permanent,A)).unwrap();
            assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut Accept(accept)).unwrap();
            let gift=game.find_object_by_stable_id(gift_stable).unwrap();
            if accept {
                assert_eq!(game.stack.len(),1);assert!(!game.stack[0].is_ability);
                assert_eq!(game.object(gift).unwrap().zone,Zone::Stack);assert_eq!(game.object(gift).unwrap().owner,b);
                assert_eq!(game.current_controller(gift),Some(A));
                assert_eq!(game.player(A).unwrap().mana_pool.total(),0,"the foreign spell is cast without its mana cost");
                ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
                assert_eq!(game.player(A).unwrap().life,25);assert_eq!(game.player(b).unwrap().life,20);
            } else {
                assert!(game.stack.is_empty());assert_eq!(game.object(gift).unwrap().zone,Zone::Graveyard);
                assert_eq!(game.player(A).unwrap().life,20);
            }
        }
    }
}
#[test]
fn additional_instant_source_casts_on_opponents_turn_returns_each_owners_creature_and_keeps_cant_block() {
    struct TargetOpponent { discard:ObjectId }
    impl ironsmith::decision::DecisionMaker for TargetOpponent {
        fn decide_boolean(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::BooleanContext)->bool {ctx.can_accept}
        fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target> {
            let mut targets=ironsmith::decision::DecisionMaker::decide_targets(&mut SelectFirstDecisionMaker,game,ctx);
            for target in &mut targets {if matches!(target,ironsmith::Target::Player(_)) {*target=ironsmith::Target::Player(PlayerId(1));}}
            targets
        }
        fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId> {
            if ctx.player==PlayerId(1) && ctx.candidates.iter().any(|candidate|candidate.id==self.discard&&candidate.legal) {return vec![self.discard];}
            ironsmith::decision::DecisionMaker::decide_objects(&mut SelectFirstDecisionMaker,game,ctx)
        }
        fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize> {
            ironsmith::decision::DecisionMaker::decide_options(&mut SelectFirstDecisionMaker,game,ctx)
        }
    }
    let card=actual_card("Terrors of Cuxuri Trench_OTH");
    for definition in source::linked_definitions(&card).unwrap().pop().unwrap() {
        let(mut game,source)=setup(&definition,0);let b=PlayerId(1);
        game.turn.active_player=b;game.turn.priority_player=Some(A);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,3);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black,3);
        let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Owned return control","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
        let returned:Vec<_>=[A,b].into_iter().map(|owner| {
            let id=game.create_object_from_definition(&creature,owner,Zone::Battlefield);(owner,game.object(id).unwrap().stable_id)
        }).collect();
        let discard=ironsmith_compiler_runtime::compile_to_runtime_definition("Preexisting discard control","Type: Artifact",false).unwrap();
        let discard=game.create_object_from_definition(&discard,b,Zone::Hand);let discard_stable=game.object(discard).unwrap().stable_id;
        use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
        let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source)).expect("the actual Instant retains its ordinary timing from the graveyard");
        let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=TargetOpponent {discard};
        let mut progress=apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() {break;}
            let ironsmith::GameProgress::NeedsDecisionCtx(context)=progress else {panic!("{progress:?}");};
            progress=apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&context,&mut dm).unwrap();
        }
        assert!(!state.has_pending_action());let stable=game.object(game.stack[0].object_id).unwrap().stable_id;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        for (owner,stable) in returned {
            let id=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(id).unwrap().zone,Zone::Hand);
            assert!(game.player(owner).unwrap().hand.contains(&id));
        }
        assert_eq!(game.object(game.find_object_by_stable_id(discard_stable).unwrap()).unwrap().zone,Zone::Graveyard);
        let permanent=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(permanent).unwrap().zone,Zone::Battlefield);
        game.refresh_continuous_state().unwrap();assert!(!game.can_block(permanent));
    }
}

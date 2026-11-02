//! Unregistered Inscribe candidate: ordinary hand activation, exact exile link,
//! and noncopiable gained attack trigger. Parent validates before certification.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions() -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-inscribe.json")).unwrap();
    let reminder = fixture["mechanics"][0]["executable_reminder"].as_str().unwrap().replace("{Cost}", "{U}");
    let text = format!("Mana cost: {{U}}\nType: Sorcery\nYou gain 1 life.\n{reminder}");
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), "Hand inscription witness").with_custom_keyword("Inscribe");
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_builder_to_artifact(builder, &text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
struct Choices {host: ObjectId, cast: bool, public_views: usize}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {self.cast}
    fn decide_objects(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        if ctx.candidates.iter().any(|candidate| candidate.id == self.host && candidate.legal) {vec![self.host]} else {SelectFirstDecisionMaker.decide_objects(game, ctx)}
    }
    fn view_cards(&mut self, _: &GameState, _: PlayerId, _: &[ObjectId], ctx: &ironsmith::decisions::context::ViewCardsContext) {if ctx.public {self.public_views += 1;}}
}
fn activate(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility {source: id, ..} if *id == source)).expect("hand activation is offered");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {return;}
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("activation did not finish");
}
fn attack(game: &mut GameState, host: ObjectId, dm: &mut Choices) {
    let defender = if game.controller_of(game.object(host).unwrap()) == A {B} else {A};
    let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::new(host, ironsmith::events::AttackEventTarget::Player(defender)), Default::default());
    game.queue_trigger_event(Default::default(), event);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, dm).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn move_to(game: &mut GameState, source: ObjectId, zone: Zone) -> ObjectId {
    let stable = game.object(source).unwrap().stable_id;
    execute_effect(game, &Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source), zone, false)), &mut EffectContext::new_default(source, A)).unwrap();
    game.find_object_by_stable_id(stable).unwrap()
}
#[test]
fn hand_cost_exile_link_and_optional_attack_copy_follow_recipient_controller() {
    for definition in definitions() {for take in [false, true] {for controller in [A, B] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A; game.turn.priority_player = Some(A); game.turn.phase = ironsmith::game_state::Phase::FirstMain; game.turn.step = None;
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Chosen host", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        let host = game.create_object_from_definition(&body, A, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(source).unwrap().stable_id;
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|a| matches!(a, LegalAction::ActivateAbility {source: id, ..} if *id == source)), "activation price is required");
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 1);
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|a| matches!(a, LegalAction::ActivateAbility {source: id, ..} if *id == source)), "activation remains sorcery-only");
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None}).unwrap();
        assert_eq!(activated.keyword, None, "generic operations cannot acquire Cipher identity");
        let mut dm = Choices {host, cast: take, public_views: 0};
        activate(&mut game, source, &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(dm.public_views > 0, "authoritative reveal cost is paid before ability resolution");
        assert_eq!(game.object(source).unwrap().zone, Zone::Hand, "exile is the resolving effect, not an extra cost");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        let has_trigger=|game:&GameState,id:ObjectId|game.current_abilities(id).unwrap_or_default().iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Triggered(_)));
        let host_trigger=has_trigger(&game,host);let exile_trigger=has_trigger(&game,exiled);
        let installed=game.effect_store.continuous_effects.effects().iter().filter(|effect|matches!(&effect.modification,ironsmith::continuous::Modification::AddAbilityGeneric(_))).map(|effect|format!("source={:?} target={:?} duration={:?}",effect.source,effect.applies_to,effect.duration)).collect::<Vec<_>>();
        assert!(host_trigger,"gained trigger missing before attack: host={host:?} exiled={exiled:?} exiled_has_trigger={exile_trigger} installed={installed:?}");
        assert!(!exile_trigger,"gained attack trigger belongs to chosen recipient, never exiled source");
        game.set_current_controller(host, controller).unwrap();
        attack(&mut game, host, &mut dm);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].controller, controller);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.stack.len(), usize::from(take));
        if take {
            assert_eq!(game.stack[0].controller, controller);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        }
        assert_eq!(game.player(controller).unwrap().life, if take {21} else {20});
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile, "copy cast never consumes original");
    }}}
}
#[test]
fn leaving_either_linked_incarnation_ends_grant_without_revival_on_reentry() {
    for definition in definitions() {for departure_of_card in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player=A; game.turn.priority_player=Some(A); game.turn.phase=ironsmith::game_state::Phase::FirstMain; game.turn.step=None;
        let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Host", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        let host=game.create_object_from_definition(&body,A,Zone::Battlefield);
        let source=game.create_object_from_definition(&definition,A,Zone::Hand);
        let stable=game.object(source).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);
        let mut dm=Choices {host,cast:false,public_views:0}; activate(&mut game,source,&mut dm);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let host=if departure_of_card {let exiled=game.find_object_by_stable_id(stable).unwrap();let hand=move_to(&mut game,exiled,Zone::Hand);move_to(&mut game,hand,Zone::Exile);host} else {let hand=move_to(&mut game,host,Zone::Hand);move_to(&mut game,hand,Zone::Battlefield)};
        attack(&mut game,host,&mut dm);assert!(game.stack.is_empty(), "new incarnation cannot revive expired inscription relationship");
    }}
}

fn pending_activation(definition: &CardDefinition, host_owner: PlayerId) -> (GameState, ObjectId, ObjectId, Choices) {
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
    let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Eligible host","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
    let host=game.create_object_from_definition(&body,A,Zone::Battlefield);
    let source=game.create_object_from_definition(definition,A,Zone::Hand);
    game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);
    let mut dm=Choices{host,cast:false,public_views:0};activate(&mut game,source,&mut dm);
    game.set_current_controller(host,host_owner).unwrap();
    (game,source,host,dm)
}
#[test]
fn copied_or_stolen_activation_checks_current_controller_hand_and_cannot_exile_twice() {
    for definition in definitions() {for controller in [A,B] {for copied in [false,true] {
        let (mut game,source,host,mut dm)=pending_activation(&definition,controller);
        let stable=game.object(source).unwrap().stable_id;
        if copied {
            let ability=game.stack[0].ability_id.expect("actual activated stack identity");
            let effect=ironsmith::effects::CopySpellEffect::new(ironsmith::target::ChooseSpec::SpecificObject(ability),1).with_target_reference_kind(ironsmith::filter::StackObjectKind::Ability);
            execute_effect(&mut game,&Effect::new(effect),&mut EffectContext::new_default(source,controller)).unwrap();
            assert_eq!(game.stack.len(),2,"real copy operation duplicates the activated ability");
        } else {
            // The property under test is resolution after control changes,
            // not the unrelated spell that performs the control change.
            game.stack[0].controller=controller;
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let arrival=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(arrival).unwrap().zone,if controller==A {Zone::Exile}else{Zone::Hand},"foreign-controlled ability cannot exile a card from someone else's hand");
        if copied {
            game.set_current_controller(host,A).unwrap();
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
            let arrival=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(arrival).unwrap().zone,Zone::Exile);
            attack(&mut game,host,&mut dm);assert_eq!(game.stack.len(),1,"only successful exact hand exile creates one link");
        } else if controller==B {
            attack(&mut game,host,&mut dm);assert!(game.stack.is_empty());
        }
    }}}
}
#[test]
fn missing_source_or_empty_creature_choice_does_not_exile_or_create_a_link() {
    for definition in definitions() {for source_left in [false,true] {
        let (mut game,source,host,mut dm)=pending_activation(&definition,A);
        let stable=game.object(source).unwrap().stable_id;
        if source_left {move_to(&mut game,source,Zone::Graveyard);} else {move_to(&mut game,host,Zone::Graveyard);}
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let arrival=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(arrival).unwrap().zone,if source_left {Zone::Graveyard}else{Zone::Hand});
        if source_left {attack(&mut game,host,&mut dm);assert!(game.stack.is_empty());}
    }}
}
#[test]
fn prevented_or_redirected_exile_cannot_create_a_granted_attack_trigger() {
    for definition in definitions() {for prevent in [false,true] {
        let (mut game,source,host,mut dm)=pending_activation(&definition,A);
        let stable=game.object(source).unwrap().stable_id;
        let mut replacement=ironsmith::replacement::ZoneReplacementSpec::new(ironsmith::target::ObjectFilter::specific(source),Zone::Graveyard).from_zone(Zone::Hand).to_zone(Zone::Exile).build(source,A);
        if prevent {replacement.replacement=ironsmith::replacement::ReplacementAction::Prevent;}
        game.effect_store.replacement_effects.add_one_shot_effect(replacement);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let arrival=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(arrival).unwrap().zone,if prevent {Zone::Hand}else{Zone::Graveyard});
        attack(&mut game,host,&mut dm);assert!(game.stack.is_empty(),"no successful exile arrival means no inscription link");
    }}
}

#[test]
fn exact_inner_thoughts_source_activates_from_hand_and_attack_casts_draw_copy() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-inscribe.json")).unwrap();
    let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]=="msem-421").unwrap();let card=&row["representative_cards"][0];
    let text=card["executable_text"].as_str().unwrap();let name=card["name"].as_str().unwrap();assert!(text.contains("Type: Instant — Inscription"));assert!(text.contains("Draw a card."));
    let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),name).with_custom_keyword("Inscribe");
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_builder_to_artifact(builder,text,false)));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);
    for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
        let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Inner Thoughts host","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
        let host=game.create_object_from_definition(&body,A,Zone::Battlefield);
        let library=ironsmith_compiler_runtime::compile_to_runtime_definition("Actual source draw","Type: Land",false).unwrap();let drawn=game.create_object_from_definition(&library,A,Zone::Library);let drawn_stable=game.object(drawn).unwrap().stable_id;
        let source=game.create_object_from_definition(&definition,A,Zone::Hand);let source_stable=game.object(source).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);
        let mut dm=Choices{host,cast:true,public_views:0};activate(&mut game,source,&mut dm);assert!(dm.public_views>0);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();let exiled=game.find_object_by_stable_id(source_stable).unwrap();assert_eq!(game.object(exiled).unwrap().zone,Zone::Exile);
        attack(&mut game,host,&mut dm);assert_eq!(game.stack.len(),1);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.stack.len(),1,"source spell is copied and cast, never consumed");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();let drawn=game.find_object_by_stable_id(drawn_stable).unwrap();assert!(game.player(A).unwrap().hand.contains(&drawn));assert_eq!(game.object(exiled).unwrap().zone,Zone::Exile);
    }
}
#[test]
fn copying_recipient_does_not_copy_resolution_granted_attack_ability() {
    for definition in definitions() {
        let (mut game,source,host,mut dm)=pending_activation(&definition,A);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let outcome=execute_effect(&mut game,&Effect::new(ironsmith::effects::CreateTokenCopyEffect::one(ironsmith::target::ChooseSpec::SpecificObject(host))),&mut EffectContext::new_default(source,A)).unwrap();
        let ironsmith::effect::OutcomeValue::Objects(copies)=outcome.value else {panic!("ordinary recipient token copy expected")};assert_eq!(copies.len(),1);
        attack(&mut game,copies[0],&mut dm);assert!(game.stack.is_empty(),"gained attack trigger is not part of recipient copiable values");
        attack(&mut game,host,&mut dm);assert_eq!(game.stack.len(),1,"original creature retains its independent gained trigger");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert!(game.stack.is_empty(),"declining copied-card cast remains optional");
    }
}

#[test]
fn independent_inscriptions_on_one_recipient_capture_distinct_exact_card_ids() {
    for definition in definitions() {for departing in [None,Some(0usize),Some(1usize)] {
        let (mut game,source,host,mut dm)=pending_activation(&definition,A);
        let first_stable=game.object(source).unwrap().stable_id;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let first=game.find_object_by_stable_id(first_stable).unwrap();
        let second=game.create_object_from_definition(&definition,A,Zone::Hand);let second_stable=game.object(second).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);
        activate(&mut game,second,&mut dm);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let second=game.find_object_by_stable_id(second_stable).unwrap();
        let references=game.effect_store.continuous_effects.effects().iter().filter_map(|effect| {
            let ironsmith::continuous::Modification::AddAbilityGeneric(ability)=&effect.modification else{return None;};
            let ironsmith::ability::AbilityKind::Triggered(triggered)=&ability.kind else{return None;};
            fn cast_tags(effect:&Effect,tags:&mut Vec<ironsmith::tag::TagKey>){
                if let Some(cast)=effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>(){tags.push(cast.tag.clone());}
                effect.0.visit_child_effects(&mut |child|cast_tags(child,tags));
            }
            let mut consumed=Vec::new();for effect in triggered.effects.all_effects(){cast_tags(effect,&mut consumed);}
            assert!(triggered.effects.captured_object_references.iter().all(|(tag,_)|consumed.contains(tag)),"acquired capture keys must equal the actual lowered CastTagged operand keys");
            Some(triggered.effects.captured_object_references.iter().flat_map(|(_,ids)|ids.iter().copied()).collect::<Vec<_>>())
        }).collect::<Vec<_>>();
        assert!(references.contains(&vec![first]),"first inscription captures its own exile ID: {references:?}");
        assert!(references.contains(&vec![second]),"second inscription cannot overwrite the first: {references:?}");
        if let Some(index)=departing {let id=[first,second][index];let hand=move_to(&mut game,id,Zone::Hand);move_to(&mut game,hand,Zone::Exile);}
        dm.cast=true;attack(&mut game,host,&mut dm);
        let count=if departing.is_some(){1}else{2};assert_eq!(game.stack.len(),count);
        for _ in 0..count {
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
            // The newly cast copy is above any remaining attack trigger.
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        }
        assert!(game.stack.is_empty());assert_eq!(game.player(A).unwrap().life,20+count as i32);
    }}
}
#[test]
fn departing_or_reexiled_card_after_trigger_announcement_cannot_satisfy_captured_reference() {
    for definition in definitions() {for reexile in [false,true] {
        let (mut game,source,host,mut dm)=pending_activation(&definition,A);let stable=game.object(source).unwrap().stable_id;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();let exiled=game.find_object_by_stable_id(stable).unwrap();
        dm.cast=true;attack(&mut game,host,&mut dm);assert_eq!(game.stack.len(),1);
        let hand=move_to(&mut game,exiled,Zone::Hand);if reexile {move_to(&mut game,hand,Zone::Exile);}
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert!(game.stack.is_empty(),"a new incarnation never revives the exact exiled-card reference");assert_eq!(game.player(A).unwrap().life,20);
    }}
}
#[test]
fn gained_body_local_exile_shadows_the_granting_abilitys_parent_exile_receipt() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-inscribe.json")).unwrap();
    let reminder=fixture["mechanics"][0]["executable_reminder"].as_str().unwrap().replace("{Cost}","{U}")
        .replace("Whenever this creature attacks, you may cast a copy of the exiled card", "Whenever this creature attacks, exile the top card of your library, then you may cast a copy of the exiled card");
    let text=format!("Mana cost: {{U}}\nType: Sorcery\nYou gain 1 life.\n{reminder}");
    let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),"Local exile scope witness").with_custom_keyword("Inscribe");
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_builder_to_artifact(builder,&text,false)));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);
    for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()] {
        let (mut game,source,host,mut dm)=pending_activation(&definition,A);let original_stable=game.object(source).unwrap().stable_id;
        let local=ironsmith_compiler_runtime::compile_to_runtime_definition("Local result reward","Type: Sorcery\nYou gain 3 life.",false).unwrap();let local=game.create_object_from_definition(&local,A,Zone::Library);let local_stable=game.object(local).unwrap().stable_id;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();dm.cast=true;attack(&mut game,host,&mut dm);assert_eq!(game.stack.len(),1);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.stack.len(),1,"locally exiled card is copied");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(A).unwrap().life,23,"local result wins over the identically named parent reference");
        assert_eq!(game.object(game.find_object_by_stable_id(original_stable).unwrap()).unwrap().zone,Zone::Exile);
        assert_eq!(game.object(game.find_object_by_stable_id(local_stable).unwrap()).unwrap().zone,Zone::Exile);
    }
}

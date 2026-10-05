//! RECOVERED / UNRUN. Full frozen bodies, direct/artifact materialization and
//! actual declaration/damage/activation paths. These are deferred source gates.
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState, DefendingPlayerReference};
use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration, DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effects::{EffectContext, ExecutionError, execute_effect};
use ironsmith::game_loop::{apply_attacker_declarations, apply_multiplayer_blocker_declarations,
    put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::replacement::{RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, Effect, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
const D: PlayerId = PlayerId::from_index(3);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/combat_defending_actor_references.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row=rows().into_iter().find(|row|row["name"]==name).unwrap();
    let mut text=format!("Mana cost: {}\nType: {}\n",row["mana_cost"].as_str().unwrap(),row["type_line"].as_str().unwrap());
    if let (Some(p),Some(t))=(row["power"].as_str(),row["toughness"].as_str()) {text.push_str(&format!("Power/Toughness: {p}/{t}\n"));}
    text.push_str(row["oracle_text"].as_str().unwrap());
    // d51's compile_to_artifact second return is already materialized from
    // its artifact. Exercise the independent direct compiler entry separately.
    let (direct_result,direct_loss)=ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name,&text,false));
    let direct=direct_result.unwrap_or_else(|error|panic!("{name} direct: {error}"));
    assert!(!direct_loss.is_lossy(),"{name} direct: {}",direct_loss.reasons_text());
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||compile_to_artifact(name,&text,false));
    let (artifact,_)=result.unwrap_or_else(|error|panic!("{name}: {error}"));
    assert!(!loss.is_lossy(),"{name}: {}",loss.reasons_text());artifact.validate().unwrap();
    let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game()->GameState {
    let mut game=GameState::new(vec!["Alice".into(),"Bob".into(),"Charlie".into(),"Dana".into()],20);
    game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=Phase::FirstMain;game.turn.step=None;game
}
fn printed(game:&mut GameState,player:PlayerId,zone:Zone,name:&str,text:&str)->ObjectId {
    game.create_object_from_definition(&compile_to_runtime_definition(name,text,false).unwrap(),player,zone)
}
fn creature(game:&mut GameState,player:PlayerId,name:&str)->ObjectId {
    printed(game,player,Zone::Battlefield,name,"Type: Creature — Human\nPower/Toughness: 2/6")
}
fn artifact(game:&mut GameState,player:PlayerId)->ObjectId {printed(game,player,Zone::Battlefield,"Counted artifact","Type: Artifact")}
fn library(game:&mut GameState,player:PlayerId,count:usize) {
    for _ in 0..count {printed(game,player,Zone::Library,"Library spell","Mana cost: {1}\nType: Sorcery\nYou gain 1 life.");}
}
fn evidence(game:&mut GameState,player:PlayerId)->Vec<ObjectId> {
    (0..3).map(|_|printed(game,player,Zone::Graveyard,"Evidence","Mana cost: {3}\nType: Artifact")).collect()
}
#[derive(Default)]
struct Choices {
    targets:Vec<Target>,objects:Vec<ObjectId>,accept:bool,forbidden_targets:Vec<Target>,copy_on_entry:bool,
    options:std::collections::VecDeque<usize>,option_choosers:Vec<PlayerId>,pause_options:bool,pending:bool,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self)->bool {true}
    fn awaiting_choice(&self)->bool {self.pending}
    fn decide_boolean(&mut self,_:&GameState,ctx:&BooleanContext)->bool {self.accept&&ctx.can_accept}
    fn decide_options(&mut self,game:&GameState,ctx:&SelectOptionsContext)->Vec<usize> {
        if self.copy_on_entry&&let Some(option)=ctx.options.iter().find(|option|option.legal&&option.description.starts_with("Enter as a copy of")) {return vec![option.index];}
        let defending_choice=!ctx.options.is_empty()&&ctx.options.iter().all(|option|game.players.iter().any(|player|player.name.as_str()==option.description));
        if defending_choice {self.option_choosers.push(ctx.player);}
        if defending_choice&&self.pause_options {self.pending=true;return Vec::new();}
        if defending_choice&&let Some(index)=self.options.pop_front() {
            assert!(ctx.options.iter().any(|option|option.index==index&&option.legal));vec![index]
        } else {SelectFirstDecisionMaker.decide_options(game,ctx)}
    }
    fn decide_targets(&mut self,game:&GameState,ctx:&TargetsContext)->Vec<Target> {
        for target in &self.forbidden_targets {assert!(!ctx.requirements.iter().any(|requirement|requirement.legal_targets.contains(target)),"{ctx:?}");}
        if self.targets.is_empty() {return SelectFirstDecisionMaker.decide_targets(game,ctx);}
        for target in &self.targets {assert!(ctx.requirements.iter().any(|requirement|requirement.legal_targets.contains(target)),"{ctx:?}");}
        self.targets.clone()
    }
    fn decide_objects(&mut self,game:&GameState,ctx:&SelectObjectsContext)->Vec<ObjectId> {
        if !self.objects.is_empty()&&self.objects.iter().all(|id|ctx.candidates.iter().any(|candidate|candidate.id==*id&&candidate.legal)) {self.objects.clone()}
        else {SelectFirstDecisionMaker.decide_objects(game,ctx)}
    }
}
fn attack(game:&mut GameState,attacker:ObjectId,target:AttackTarget)->TriggerQueue {
    game.remove_summoning_sickness(attacker);game.turn.phase=Phase::Combat;game.turn.step=Some(Step::DeclareAttackers);
    let mut combat=CombatState::default();let mut queue=TriggerQueue::new();
    apply_attacker_declarations(game,&mut combat,&mut queue,&[AttackerDeclaration{creature:attacker,target}]).unwrap();
    game.combat=Some(combat);queue
}
fn reselect(game:&mut GameState,attacker:ObjectId,target:AttackTarget) {
    // CR 508.7 destination mutation, not another attack declaration.
    game.combat.as_mut().unwrap().attackers.iter_mut().find(|info|info.creature==attacker).unwrap().target=target;
}
fn admit(game:&mut GameState,queue:&mut TriggerQueue,choices:&mut Choices) {put_triggers_on_stack_with_dm(game,queue,choices).unwrap();}
fn resolve(game:&mut GameState,choices:&mut Choices) {resolve_stack_entry_with(game,choices).unwrap();}
fn blocks(game:&mut GameState,declarations:&[BlockerDeclaration])->TriggerQueue {
    game.turn.step=Some(Step::DeclareBlockers);let mut combat=game.combat.clone().unwrap();let mut queue=TriggerQueue::new();
    apply_multiplayer_blocker_declarations(game,&mut combat,&mut queue,declarations).unwrap();game.combat=Some(combat);queue
}
fn combat_damage(game:&mut GameState)->TriggerQueue {
    game.turn.step=Some(Step::CombatDamage);let combat=game.combat.clone().unwrap();
    let events=ironsmith::game_loop::execute_combat_damage_step_with_dm(game,&combat,false,&mut SelectFirstDecisionMaker);
    let mut queue=TriggerQueue::new();ironsmith::game_loop::queue_combat_damage_triggers(game,&events,&mut queue);queue
}
fn activate(game:&mut GameState,source:ObjectId,ability_index:usize,choices:&mut Choices) {
    use ironsmith::decision::{LegalAction,compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
    let action=LegalAction::ActivateAbility{source,ability_index};assert!(compute_legal_actions(game,A).unwrap().contains(&action));
    let mut queue=TriggerQueue::new();let mut state=PriorityLoopState::new(4);
    let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),choices).unwrap();
    for _ in 0..32 {if state.pending_activation.is_none(){break;} let ironsmith::GameProgress::NeedsDecisionCtx(context)=progress else {panic!("{progress:?}");};
        progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&context,choices).unwrap();}
    assert!(state.pending_activation.is_none());assert_eq!(game.stack.len(),1);
}
#[test]
fn nine_complete_frozen_bodies_round_trip_without_parse_loss() {
    assert_eq!(rows().len(),9);for row in rows(){assert_eq!(row["oracle_id"].as_str().unwrap().len(),36);
        for definition in definitions(row["name"].as_str().unwrap()){assert_eq!(definition.card.name.as_str(),row["name"].as_str().unwrap());}}
}
#[test]
fn perforator_keeps_the_current_then_last_reselected_defender_across_removal() {
    for definition in definitions("Falkenrath Perforator"){for removal in 0..5 {
        let mut game=game();let attacker=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let stable=game.object(attacker).unwrap().stable_id;let mut queue=attack(&mut game,attacker,AttackTarget::Player(B));
        let mut choices=Choices::default();admit(&mut game,&mut queue,&mut choices);reselect(&mut game,attacker,AttackTarget::Player(C));
        match removal {
            0=>{},1=>{game.set_current_controller(attacker,D).unwrap();},
            2=>{game.phase_out(attacker);game.phase_in(attacker);},
            3=>{game.object_mut(attacker).unwrap().card_types=vec![CardType::Artifact].into();game.refresh_continuous_state().unwrap();},
            _=>{let graveyard=game.move_object_by_effect(attacker,Zone::Graveyard).unwrap();let returned=game.move_object_by_effect(graveyard,Zone::Battlefield).unwrap();
                assert_ne!(returned,attacker);assert_eq!(game.object(returned).unwrap().stable_id,stable);
                game.combat.as_mut().unwrap().attackers.push(ironsmith::combat_state::AttackerInfo{creature:returned,target:AttackTarget::Player(D)});}
        }
        resolve(&mut game,&mut choices);assert_eq!(game.player(B).unwrap().life,20);assert_eq!(game.player(C).unwrap().life,19);assert_eq!(game.player(D).unwrap().life,20);
    }}
}
#[test]
fn departed_planeswalker_and_battle_destinations_retain_controller_or_protector_before_admission() {
    for definition in definitions("Falkenrath Perforator"){for battle in [false,true]{for change in 0..3 {
        let mut game=game();let attacker=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let destination=if battle{let id=printed(&mut game,A,Zone::Battlefield,"Siege","Type: Battle — Siege\nDefense: 8");assert!(game.set_battle_protector(id,B));id}
            else{printed(&mut game,B,Zone::Battlefield,"Planeswalker","Type: Planeswalker\nLoyalty: 8")};
        let mut queue=attack(&mut game,attacker,if battle{AttackTarget::Battle(destination)}else{AttackTarget::Planeswalker(destination)});
        let expected=if battle&&change==0{assert!(game.set_battle_protector(destination,C));C}else{B};
        match change{0 if !battle=>{game.set_current_controller(destination,C).unwrap();},1=>{game.move_object_by_effect(destination,Zone::Graveyard).unwrap();},2=>{game.phase_out(destination);},_=>{}}
        let mut choices=Choices::default();admit(&mut game,&mut queue,&mut choices);resolve(&mut game,&mut choices);assert_eq!(game.player(expected).unwrap().life,19);
    }}}
}
#[test]
fn plunderer_uses_current_artifacts_and_retains_its_optional_reflexive_treasures() {
    for definition in definitions("Generous Plunderer"){
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        artifact(&mut game,B);for _ in 0..3{artifact(&mut game,C);}let mut queue=attack(&mut game,source,AttackTarget::Player(B));
        reselect(&mut game,source,AttackTarget::Player(C));let mut choices=Choices::default();admit(&mut game,&mut queue,&mut choices);resolve(&mut game,&mut choices);
        assert_eq!(game.player(B).unwrap().life,20);assert_eq!(game.player(C).unwrap().life,17);assert!(game.object_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Menace));
        for accept in [false,true]{game.turn.phase=Phase::Beginning;game.turn.step=Some(Step::Upkeep);let before=game.battlefield.len();
            ironsmith::game_loop::generate_and_queue_step_triggers(&mut game,&mut queue);choices.accept=accept;choices.targets=vec![Target::Player(D)];
            admit(&mut game,&mut queue,&mut choices);resolve(&mut game,&mut choices);admit(&mut game,&mut queue,&mut choices);
            if accept{assert_eq!(game.stack.len(),1);resolve(&mut game,&mut choices);}assert_eq!(game.battlefield.len(),before+if accept{2}else{0});
            if accept{let treasures:Vec<_>=game.battlefield.iter().copied().filter(|id|game.object(*id).is_some_and(|object|object.name.as_str()=="Treasure")).collect();assert_eq!(treasures.len(),2);
                for id in treasures{let owner=game.current_controller(id).unwrap();assert!(owner==A||owner==D);assert_eq!(game.is_tapped(id),owner==D);}}
        }
    }
}
#[test]
fn sling_and_helm_use_the_exact_blocked_attacker_after_equipment_or_host_departure() {
    for name in ["Simian Sling","Tormentor's Helm"]{for definition in definitions(name){for remove_equipment in [false,true]{
        let mut game=game();let equipment=game.create_object_from_definition(&definition,A,Zone::Battlefield);let attacker=creature(&mut game,A,"Equipped attacker");
        assert!(game.attach_object_to_target(equipment,ironsmith::object::AttachmentTarget::Object(attacker)));assert_eq!(game.current_power(attacker),Some(3));assert_eq!(game.current_toughness(attacker),Some(7));
        if name=="Simian Sling"{assert!(!game.current_has_card_type(equipment,CardType::Creature));}
        let blocker=creature(&mut game,B,"Blocker");assert!(attack(&mut game,attacker,AttackTarget::Player(B)).is_empty());
        let mut queue=blocks(&mut game,&[BlockerDeclaration{blocker,blocking:attacker}]);assert_eq!(queue.entries.len(),1);
        reselect(&mut game,attacker,AttackTarget::Player(C));game.move_object_by_effect(if remove_equipment{equipment}else{attacker},Zone::Graveyard).unwrap();
        let mut choices=Choices::default();admit(&mut game,&mut queue,&mut choices);resolve(&mut game,&mut choices);
        assert_eq!(game.player(C).unwrap().life,19);assert_eq!(game.player(B).unwrap().life,20);
        let damage:Vec<_>=game.action_history_for_player(C).filter_map(|record|record.event.downcast::<ironsmith::events::DamageEvent>()).filter(|damage|!damage.is_combat&&damage.amount==1).collect();
        assert!(damage.iter().any(|damage|damage.source==attacker&&damage.target==ironsmith::events::DamageTarget::Player(C)));assert!(!damage.iter().any(|damage|damage.source==equipment));
    }}}
}
#[test]
fn unattached_sling_has_its_own_actual_blocked_trigger() {
    for definition in definitions("Simian Sling"){let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let blocker=creature(&mut game,B,"Blocker");
        attack(&mut game,source,AttackTarget::Player(B));let mut queue=blocks(&mut game,&[BlockerDeclaration{blocker,blocking:source}]);
        let mut choices=Choices::default();admit(&mut game,&mut queue,&mut choices);resolve(&mut game,&mut choices);assert_eq!(game.player(B).unwrap().life,19);}
}
#[test]
fn sling_reconfigure_and_helm_equip_use_native_activations_and_sorcery_timing() {
    for name in ["Simian Sling","Tormentor's Helm"]{for definition in definitions(name){
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let host=creature(&mut game,A,"Legal host");let enemy=creature(&mut game,B,"Illegal host");
        game.player_mut(A).unwrap().mana_pool.colorless=10;
        let indices:Vec<_>=definition.abilities.iter().enumerate().filter_map(|(index,ability)|matches!(ability.kind,ironsmith::ability::AbilityKind::Activated(_)).then_some(index)).collect();
        assert_eq!(indices.len(),if name=="Simian Sling"{2}else{1});
        let mut choices=Choices{targets:vec![Target::Object(host)],forbidden_targets:vec![Target::Object(enemy)],..Default::default()};
        activate(&mut game,source,indices[0],&mut choices);resolve(&mut game,&mut choices);assert_eq!(game.current_power(host),Some(3));assert_eq!(game.current_toughness(host),Some(7));
        if name=="Simian Sling"{assert!(!game.current_has_card_type(source,CardType::Creature));choices.targets.clear();choices.forbidden_targets.clear();
            activate(&mut game,source,indices[1],&mut choices);resolve(&mut game,&mut choices);assert!(game.object(source).unwrap().attached_to.is_none());assert!(game.current_has_card_type(source,CardType::Creature));
            assert_eq!(game.current_power(host),Some(2));assert_eq!(game.current_toughness(host),Some(6));}
        game.turn.phase=Phase::Combat;game.turn.step=Some(Step::DeclareAttackers);let legal=ironsmith::decision::compute_legal_actions(&game,A).unwrap();
        assert!(!legal.iter().any(|action|matches!(action,ironsmith::decision::LegalAction::ActivateAbility{source:id,ability_index}if *id==source&&indices.contains(ability_index))));
    }}
}
#[test]
fn memory_redirection_keeps_the_attack_actor_through_evidence_and_reflexive_cast() {
    for definition in definitions("Memory Vampire"){
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);library(&mut game,D,7);let paid=evidence(&mut game,A);
        let spell=printed(&mut game,B,Zone::Graveyard,"Defenders spell","Mana cost: {6}\nType: Sorcery\nYou gain 3 life.");let stable=game.object(spell).unwrap().stable_id;
        attack(&mut game,source,AttackTarget::Player(B));blocks(&mut game,&[]);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source,A,
            ironsmith::events::damage::matchers::DamageToPlayerMatcher::new(PlayerFilter::Specific(B)),
            ReplacementAction::Redirect{target:RedirectTarget::ToPlayer(C),which:RedirectWhich::First}));
        let mut queue=combat_damage(&mut game);assert_eq!(game.player(C).unwrap().life,16);assert_eq!(game.player(B).unwrap().life,20);
        let mut choices=Choices{accept:true,targets:vec![Target::Player(D)],objects:paid.clone(),..Default::default()};
        admit(&mut game,&mut queue,&mut choices);resolve(&mut game,&mut choices);assert_eq!(game.player(D).unwrap().library.len(),3);assert!(paid.iter().all(|id|game.object(*id).is_none()));
        reselect(&mut game,source,AttackTarget::Player(D));choices.targets=vec![Target::Object(spell)];choices.objects.clear();
        admit(&mut game,&mut queue,&mut choices);assert_eq!(game.stack.len(),1);resolve(&mut game,&mut choices);assert_eq!(game.stack.len(),1);
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Stack);resolve(&mut game,&mut choices);assert_eq!(game.player(A).unwrap().life,23);
        assert!(choices.option_choosers.is_empty());
    }
}
#[test]
fn blocking_memory_chooses_a_defender_other_than_recipient_or_controller_and_pauses_atomically() {
    for definition in definitions("Memory Vampire"){for accept in [false,true]{
        let mut game=game();let source=game.create_object_from_definition(&definition,B,Zone::Battlefield);let attacker=creature(&mut game,A,"Attacker");
        let paid=evidence(&mut game,B);library(&mut game,C,7);let spell=printed(&mut game,D,Zone::Graveyard,"Chosen defenders spell","Mana cost: {6}\nType: Instant\nYou gain 3 life.");
        attack(&mut game,attacker,AttackTarget::Player(B));blocks(&mut game,&[BlockerDeclaration{blocker:source,blocking:attacker}]);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source,B,
            ironsmith::events::damage::matchers::DamageToObjectMatcher::new(ObjectFilter::specific(attacker)),
            ReplacementAction::Redirect{target:RedirectTarget::ToPlayer(C),which:RedirectWhich::First}));
        let mut queue=combat_damage(&mut game);let mut choices=Choices{accept,targets:vec![Target::Player(C)],objects:paid,pause_options:accept,..Default::default()};
        admit(&mut game,&mut queue,&mut choices);let checkpoint=game.clone();resolve(&mut game,&mut choices);
        if !accept{assert!(choices.option_choosers.is_empty());assert_eq!(game.player(C).unwrap().library.len(),3);continue;}
        assert!(choices.pending);assert_eq!(choices.option_choosers,vec![B]);assert_eq!(game.player(C).unwrap().library.len(),checkpoint.player(C).unwrap().library.len());
        assert_eq!(game.player(B).unwrap().graveyard.len(),checkpoint.player(B).unwrap().graveyard.len());assert_eq!(game.stack.len(),checkpoint.stack.len());
        choices.pending=false;choices.pause_options=false;choices.options.push_back(2);resolve(&mut game,&mut choices);
        choices.targets=vec![Target::Object(spell)];choices.objects.clear();admit(&mut game,&mut queue,&mut choices);assert_eq!(game.stack.last().unwrap().defending_player,Some(D));
        resolve(&mut game,&mut choices);resolve(&mut game,&mut choices);assert_eq!(game.player(B).unwrap().life,23);
    }}
}
#[test]
fn cane_announces_current_defender_targets_and_revalidates_reselection_departure_and_source_lki() {
    for definition in definitions("Blue Mage's Cane"){for change in 0..4{
        let mut game=game();let equipment=game.create_object_from_definition(&definition,A,Zone::Battlefield);let host=creature(&mut game,A,"Cane bearer");
        assert!(game.attach_object_to_target(equipment,ironsmith::object::AttachmentTarget::Object(host)));
        let b_spell=printed(&mut game,B,Zone::Graveyard,"B spell","Mana cost: {8}\nType: Instant\nYou gain 3 life.");
        let c_spell=printed(&mut game,C,Zone::Graveyard,"C spell","Mana cost: {8}\nType: Instant\nYou gain 5 life.");
        let mut queue=attack(&mut game,host,AttackTarget::Player(B));let target=if change==0{reselect(&mut game,host,AttackTarget::Player(C));c_spell}else{b_spell};
        let mut choices=Choices{targets:vec![Target::Object(target)],forbidden_targets:vec![Target::Object(if change==0{b_spell}else{c_spell})],accept:true,..Default::default()};
        admit(&mut game,&mut queue,&mut choices);match change{1=>reselect(&mut game,host,AttackTarget::Player(C)),2=>{game.move_object_by_effect(target,Zone::Exile).unwrap();},3=>{game.move_object_by_effect(host,Zone::Graveyard).unwrap();},_=>{}}
        game.player_mut(A).unwrap().mana_pool.colorless=3;resolve(&mut game,&mut choices);
        if change==1||change==2{assert!(game.stack_is_empty());assert_eq!(game.player(A).unwrap().mana_pool.total(),3);if change==1{assert_eq!(game.object(b_spell).unwrap().zone,Zone::Graveyard);}}
        else{assert_eq!(game.player(A).unwrap().mana_pool.total(),0);assert_eq!(game.stack.len(),1);resolve(&mut game,&mut choices);assert_eq!(game.player(A).unwrap().life,if change==0{25}else{23});}
    }}
}
#[test]
fn cane_shared_team_targets_follow_the_exact_attacker_before_and_after_announcement() {
    for definition in definitions("Blue Mage's Cane") {
        for reselect_before_announcement in [false,true] {
            let mut game=game();
            game.set_teams(vec![vec![A,B],vec![C,D]]).unwrap();
            game.enable_shared_team_turns().unwrap();
            let equipment=game.create_object_from_definition(&definition,A,Zone::Battlefield);
            let host=creature(&mut game,A,"Team Cane bearer");
            assert!(game.attach_object_to_target(equipment,ironsmith::object::AttachmentTarget::Object(host)));
            let c_spell=printed(&mut game,C,Zone::Graveyard,"Charlie's spell","Mana cost: {8}\nType: Instant\nYou gain 3 life.");
            let d_spell=printed(&mut game,D,Zone::Graveyard,"Dana's spell","Mana cost: {8}\nType: Instant\nYou gain 5 life.");
            let d_stable=game.object(d_spell).unwrap().stable_id;
            let mut queue=attack(&mut game,host,AttackTarget::Player(C));
            if reselect_before_announcement {reselect(&mut game,host,AttackTarget::Player(D));}
            let (target,forbidden)=if reselect_before_announcement {(d_spell,c_spell)}else{(c_spell,d_spell)};
            let mut choices=Choices{targets:vec![Target::Object(target)],forbidden_targets:vec![Target::Object(forbidden)],accept:true,..Default::default()};
            admit(&mut game,&mut queue,&mut choices);
            assert!(choices.option_choosers.is_empty(),"an exact defender never offers its teammate");
            let entry=game.stack.last().unwrap();
            assert_eq!(entry.targets,vec![Target::Object(target)]);
            assert_eq!(entry.defending_player,None,"announcement must not freeze an attacker reference");
            let reference=entry.defending_player_reference.unwrap();
            assert!(matches!(reference,DefendingPlayerReference::Attacker{attacker,..} if attacker==host));
            if !reselect_before_announcement {reselect(&mut game,host,AttackTarget::Player(D));}
            assert_eq!(game.defending_player_candidates(reference).unwrap(),vec![D]);
            game.player_mut(A).unwrap().mana_pool.colorless=3;
            resolve(&mut game,&mut choices);
            if reselect_before_announcement {
                assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
                assert_eq!(game.stack.len(),1,"the exact new defender's spell was copied and cast");
                resolve(&mut game,&mut choices);
                assert!(game.object(d_spell).is_none());
                assert_eq!(game.object(game.find_object_by_stable_id(d_stable).unwrap()).unwrap().zone,Zone::Exile);
            } else {
                assert!(game.stack_is_empty(),"the old teammate's card is no longer a legal target");
                assert_eq!(game.player(A).unwrap().mana_pool.total(),3);
                assert_eq!(game.object(d_spell).unwrap().zone,Zone::Graveyard);
            }
            assert_eq!(game.object(c_spell).unwrap().zone,Zone::Graveyard);
            assert!(choices.option_choosers.is_empty());
        }
    }
}
#[test]
fn barret_and_xander_keep_the_last_reselected_defender_after_the_runner_ends_combat() {
    for name in ["Barret Wallace","Lord Xander, the Collector"]{for definition in definitions(name){
        let mut game=game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let host=creature(&mut game,A,"Equipped ally");
        let equipment=printed(&mut game,A,Zone::Battlefield,"Equipment","Type: Artifact — Equipment");game.attach_object_to_target(equipment,ironsmith::object::AttachmentTarget::Object(host));
        library(&mut game,B,7);library(&mut game,C,9);let mut queue=attack(&mut game,source,AttackTarget::Player(B));let mut choices=Choices::default();admit(&mut game,&mut queue,&mut choices);
        let mut runner=ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::EndCombatPriority);*runner.combat_mut()=game.combat.clone().unwrap();
        reselect(&mut game,source,AttackTarget::Player(C));runner.advance(&mut game,&mut TriggerQueue::new()).unwrap();assert!(game.combat.as_ref().unwrap().attackers.is_empty());
        resolve(&mut game,&mut choices);assert_eq!(game.player(B).unwrap().life,20);assert_eq!(game.player(B).unwrap().library.len(),7);
        if name=="Barret Wallace"{assert_eq!(game.player(C).unwrap().life,19);}else{assert_eq!(game.player(C).unwrap().library.len(),5);}
    }}
}
#[test]
fn auton_myriad_excludes_current_defender_and_preserves_copy_exceptions_and_delayed_exile() {
    for definition in definitions("Auton Soldier"){
        let mut game=game();printed(&mut game,B,Zone::Battlefield,"Borrowed original","Type: Legendary Creature — Human\nPower/Toughness: 2/6\nVigilance");
        let card=game.create_object_from_definition(&definition,A,Zone::Hand);let mut choices=Choices{accept:true,copy_on_entry:true,..Default::default()};
        let receipt=game.move_object_with_etb_processing_with_dm(card,Zone::Battlefield,&mut choices).unwrap();assert!(!receipt.pending);assert!(receipt.programs.is_empty());let source=receipt.original.into_result().unwrap().new_id;
        assert!(game.current_has_card_type(source,CardType::Artifact));assert!(!game.object(source).unwrap().supertypes.contains(&ironsmith::Supertype::Legendary));
        let mut queue=attack(&mut game,source,AttackTarget::Player(B));admit(&mut game,&mut queue,&mut choices);reselect(&mut game,source,AttackTarget::Player(C));resolve(&mut game,&mut choices);
        let tokens:Vec<_>=game.battlefield.iter().copied().filter(|id|game.object(*id).is_some_and(|object|matches!(object.kind,ironsmith::object::ObjectKind::Token))).collect();assert_eq!(tokens.len(),2);
        let targets:Vec<_>=game.combat.as_ref().unwrap().attackers.iter().filter(|info|tokens.contains(&info.creature)).map(|info|info.target.clone()).collect();
        assert!(targets.contains(&AttackTarget::Player(B)));assert!(targets.contains(&AttackTarget::Player(D)));assert!(!targets.contains(&AttackTarget::Player(C)));
        for id in &tokens{assert!(game.is_tapped(*id));assert!(game.current_has_card_type(*id,CardType::Artifact));assert!(!game.object(*id).unwrap().supertypes.contains(&ironsmith::Supertype::Legendary));}
        let mut runner=ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::EndCombat);*runner.combat_mut()=game.combat.clone().unwrap();
        runner.advance(&mut game,&mut queue).unwrap();admit(&mut game,&mut queue,&mut choices);while !game.stack_is_empty(){resolve(&mut game,&mut choices);}
        assert!(tokens.iter().all(|id|!game.battlefield.contains(id)));assert!(game.battlefield.contains(&source));
    }
}
#[test]
fn missing_evidence_fails_atomically_and_known_absence_produces_no_recipient() {
    let mut game=game();let source=creature(&mut game,A,"Source");let damage=Effect::new(ironsmith::effects::DealDamageEffect::new(1,ChooseSpec::Player(PlayerFilter::Defending)));
    let mut dm=Choices::default();let mut context=EffectContext::new(source,A,&mut dm);context.combat.defending_player_reference=Some(DefendingPlayerReference::Missing);
    assert!(matches!(execute_effect(&mut game,&damage,&mut context),Err(ExecutionError::IncompleteEvidence(_))));assert!(game.players.iter().all(|player|player.life==20));
    context.combat.defending_player_reference=Some(DefendingPlayerReference::KnownAbsent);execute_effect(&mut game,&damage,&mut context).unwrap();assert!(game.players.iter().all(|player|player.life==20));
}
#[test]
fn missing_target_actor_evidence_preserves_the_trigger_queue_and_stack() {
    for definition in definitions("Blue Mage's Cane"){
        let mut game=game();let equipment=game.create_object_from_definition(&definition,A,Zone::Battlefield);let host=creature(&mut game,A,"Cane bearer");game.attach_object_to_target(equipment,ironsmith::object::AttachmentTarget::Object(host));
        printed(&mut game,B,Zone::Graveyard,"Potential target","Type: Instant\nYou gain 1 life.");let mut queue=attack(&mut game,host,AttackTarget::Player(B));assert_eq!(queue.entries.len(),1);
        queue.entries[0].triggering_event=queue.entries[0].triggering_event.clone().with_defending_player_reference(DefendingPlayerReference::Missing);let before=game.next_object_id_counter();
        let error=put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut Choices::default()).unwrap_err();assert!(matches!(error,ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::IncompleteEvidence(_))));
        assert_eq!(queue.entries.len(),1);assert!(game.stack_is_empty());assert_eq!(game.next_object_id_counter(),before);
    }
}

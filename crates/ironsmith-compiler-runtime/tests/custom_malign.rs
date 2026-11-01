//! Unregistered source-backed Malign checks through ordinary entry processing.
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::combat_state::{AttackTarget,CombatState};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::object::CounterType;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(name:&str,text:&str)->[CardDefinition;2] {
    let builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),name).with_custom_keyword("Malign");
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_builder_to_artifact(builder,text,false)));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();
    let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(decoded,artifact);
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn attack(game:&mut GameState,player:PlayerId) {
    let d=ironsmith_compiler_runtime::compile_to_runtime_definition("Attack history witness","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
    let attacker=game.create_object_from_definition(&d,player,Zone::Battlefield);game.remove_summoning_sickness(attacker);
    game.turn.active_player=player;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();
    let mut combat=CombatState::default();let mut queue=ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(game,&mut combat,&mut queue,&[AttackerDeclaration{creature:attacker,target:AttackTarget::Player(if player==A {B}else{A})}]).unwrap();
    assert!(game.turn_store.turn_history.players_attacked_this_turn.contains(&player));
    // The historical fact outlives every creature which declared the attack.
    execute_effect(game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(attacker),Zone::Graveyard,false)),&mut EffectContext::new_default(attacker,player)).unwrap();
}
fn enter(game:&mut GameState,definition:&CardDefinition,controller:PlayerId)->ObjectId {
    let source=game.create_object_from_definition(definition,controller,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
    execute_effect(game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Battlefield,false)),&mut EffectContext::new_default(source,controller)).unwrap();
    let arrived=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(arrived).unwrap().zone,Zone::Battlefield);arrived
}
#[test]
fn exact_materialized_source_entries_read_only_their_controllers_attack_history() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-malign.json")).unwrap();
    for (id,n) in [("msem-968",1),("msem-969",2)] {
        let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]==id).unwrap();let card=&row["representative_cards"][0];
        for definition in definitions(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap()) {for who in [None,Some(A),Some(B)] {
            let mut game=GameState::new(vec!["A".into(),"B".into()],20);if let Some(player)=who {attack(&mut game,player);}
            let arrived=enter(&mut game,&definition,A);
            assert_eq!(game.counter_count(arrived,CounterType::PlusOnePlusOne),if who==Some(A){n}else{0});
        }}
    }
}
#[test]
fn attack_history_resets_each_turn_and_each_malign_instance_applies_separately() {
    let text="Type: Creature\nPower/Toughness: 1/1\nIf you attacked this turn, this creature enters with one +1/+1 counter on it.\nIf you attacked this turn, this creature enters with two +1/+1 counters on it.";
    for definition in definitions("Independent entry instances",text) {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);attack(&mut game,A);
        let arrived=enter(&mut game,&definition,A);assert_eq!(game.counter_count(arrived,CounterType::PlusOnePlusOne),3);
        game.next_turn();assert!(!game.turn_store.turn_history.players_attacked_this_turn.contains(&A));
        let next=enter(&mut game,&definition,A);assert_eq!(game.counter_count(next,CounterType::PlusOnePlusOne),0);
        assert_eq!(game.counter_count(arrived,CounterType::PlusOnePlusOne),3,"existing counters do not reset with attack history");
    }
}
#[test]
fn entry_counters_use_existing_counter_replacements_and_prohibitions() {
    let text="Type: Creature\nPower/Toughness: 1/1\nIf you attacked this turn, this creature enters with two +1/+1 counters on it.";
    for definition in definitions("Entry counter replacement witness",text) {for mode in 0..3 {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);attack(&mut game,A);
        let mut helper=ironsmith_compiler_runtime::compile_to_runtime_definition("Counter modification source","Type: Artifact",false).unwrap();
        if mode==2 {helper.abilities.push(ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::restriction(ironsmith::effect::Restriction::HaveCounterTypePlaced(ironsmith::target::ObjectFilter::creature().you_control(),CounterType::PlusOnePlusOne),"No plus-one counters".into())));}
        let helper=game.create_object_from_definition(&helper,A,Zone::Battlefield);
        if mode==1 {let replacement=ironsmith::static_abilities::StaticAbility::double_counters_replacement(ironsmith::target::ObjectFilter::creature().you_control(),Some(CounterType::PlusOnePlusOne),"Double entry counters".into()).generate_replacement_effect(helper,A).unwrap();game.effect_store.replacement_effects.add_resolution_effect(replacement);}
        let arrived=enter(&mut game,&definition,A);assert_eq!(game.counter_count(arrived,CounterType::PlusOnePlusOne),[2,4,0][mode]);
    }}
}

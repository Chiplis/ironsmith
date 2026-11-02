//! Complete Revolution Waste characteristic replacement through generic object templates.
//! Activate-cost and action-receipt readers are separate from this body operation.
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith::ability::AbilityKind;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
use ironsmith::object::CounterType;
use ironsmith::types::{CardType, Subtype, Supertype};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const TEMPLATE: &str = "a basic land named Wastes with \"{T}: Add {C}\" and loses all other supertypes, card types, subtypes, and abilities";
fn definitions(name: &str, text: &str) -> [ironsmith::cards::CardDefinition; 2] {
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),name).with_custom_keyword("Waste");
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true,true,|| ironsmith_compiler_runtime::compile_builder_to_artifact(builder,text,false)));
    assert!(!loss.is_lossy(),"{name}: {}",loss.reasons_text());
    let (artifact,direct)=result.unwrap_or_else(|e|panic!("{name}: {text}\n{e:?}"));
    let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded,artifact);
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn card(name:&str,text:&str)->ironsmith::cards::CardDefinition { definitions(name,text).into_iter().next().unwrap() }
fn assert_wastes(game:&GameState,id:ObjectId) {
    let view=game.current_characteristics(id).unwrap();
    assert_eq!(view.name.as_ref(),"Wastes");
    assert_eq!(view.card_types.as_slice(),&[CardType::Land]);
    assert_eq!(view.supertypes.as_slice(),&[Supertype::Basic]);
    assert!(view.subtypes.is_empty(),"all former subtype families must be erased: {:?}",view.subtypes);
    let abilities=game.current_abilities(id).unwrap();
    assert_eq!(abilities.len(),1,"old printed/granted abilities must be erased");
    assert!(abilities[0].is_mana_ability());
}
#[test]
fn waste_replaces_all_characteristic_families_and_preserves_unmentioned_state() {
    for spell in definitions("Overwrite",&format!("Type: Sorcery\nTarget permanent becomes {TEMPLATE}.")) {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let original=card("Ancient engine","Mana cost: {R}\nType: Legendary Snow Artifact Creature — Equipment Human\nPower/Toughness: 4/5\nFlying, haste\n{T}: Add {G}.");
        let source=game.create_object_from_definition(&spell,A,Zone::Stack);
        let changed=game.create_object_from_definition(&original,B,Zone::Battlefield);
        let untouched=game.create_object_from_definition(&original,A,Zone::Battlefield);
        game.add_counters(changed,CounterType::PlusOnePlusOne,2);
        game.tap(changed);
        let stable=game.object(changed).unwrap().stable_id;
        let mut dm=SelectFirstDecisionMaker;
        let mut ctx=EffectContext::new(source,A,&mut dm).with_targets(vec![ResolvedTarget::Object(changed)]);
        for effect in spell.spell_effect.as_ref().unwrap().all_effects() { execute_effect(&mut game,effect,&mut ctx).unwrap(); }
        assert_wastes(&game,changed);
        let mana=game.current_abilities(changed).unwrap().remove(0);
        let AbilityKind::Activated(mana)=mana.kind else {panic!("expected colorless mana activation")};
        let mut mana_ctx=EffectContext::new_default(changed,B);
        for effect in mana.effects.all_effects() {execute_effect(&mut game,effect,&mut mana_ctx).unwrap();}
        assert_eq!(game.player(B).unwrap().mana_pool.colorless,1);
        assert_eq!(game.player(B).unwrap().mana_pool.green,0);
        assert_eq!(game.current_characteristics(changed).unwrap().colors,ironsmith::ColorSet::RED);
        assert_eq!(game.current_controller(changed),Some(B));
        assert_eq!(game.object(changed).unwrap().stable_id,stable);
        assert!(game.is_tapped(changed));
        assert_eq!(game.counter_count(changed,CounterType::PlusOnePlusOne),2);
        let observer=game.current_characteristics(untouched).unwrap();
        assert_eq!(observer.name.as_ref(),"Ancient engine");
        assert!(observer.card_types.contains(&CardType::Creature));
        assert!(observer.supertypes.contains(&Supertype::Legendary));
        assert!(observer.subtypes.contains(&Subtype::Equipment));
        assert!(observer.subtypes.contains(&Subtype::Human));
        // The action has no duration. Cleanup must not restore any former
        // characteristics; a later zone transition does create a new object.
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_wastes(&game,changed);
        let mut ctx=EffectContext::new(source,A,&mut dm);
        let movement=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(changed),Zone::Hand,false));
        execute_effect(&mut game,&movement,&mut ctx).unwrap();
        let moved=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.current_characteristics(moved).unwrap().name.as_ref(),"Ancient engine");
    }
}
struct Choice { accept:bool, land:Option<ObjectId> }
impl DecisionMaker for Choice {
    fn decide_boolean(&mut self,_:&GameState,_:&BooleanContext)->bool {self.accept}
    fn decide_objects(&mut self,game:&GameState,ctx:&SelectObjectsContext)->Vec<ObjectId> {
        if let Some(land)=self.land { if ctx.candidates.iter().any(|c|c.id==land && c.legal) {return vec![land];} }
        SelectFirstDecisionMaker.decide_objects(game,ctx)
    }
}
#[test]
fn desperate_animist_fullsource_equivalent_keeps_optional_choice_and_reward_scope() {
    // Original exact source: fixtures/custom_mechanics/revolution.json,
    // rev:Desperate Animist_VST. The declaration's nonwastes selector is an
    // explicit name exclusion, and both operations share the paid action arm.
    let text=format!("Mana cost: {{G}}\nType: Creature — Human Shaman\nPower/Toughness: 1/1\nWhenever Desperate Animist attacks, you may choose a land you control not named Wastes. If you do, that land becomes {TEMPLATE}, then Desperate Animist gets +2/+2 until end of turn.");
    for definition in definitions("Desperate Animist",&text) { for accept in [false,true] { for available in [false,true] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let land=card("Ancient grove","Type: Legendary Snow Land — Forest\n{T}: Add {G}.");
        let own=available.then(||game.create_object_from_definition(&land,A,Zone::Battlefield));
        let opponent=game.create_object_from_definition(&land,B,Zone::Battlefield);
        let wastes=game.create_object_from_definition(&card("Wastes","Type: Basic Land\n{T}: Add {C}."),A,Zone::Battlefield);
        let trigger=definition.abilities.iter().find_map(|ability|match &ability.kind {AbilityKind::Triggered(trigger)=>Some(trigger),_=>None}).expect("expected attack trigger");
        let mut dm=Choice {accept,land:own};
        let mut ctx=EffectContext::new(source,A,&mut dm);
        for effect in trigger.effects.all_effects() {execute_effect(&mut game,effect,&mut ctx).unwrap();}
        assert_eq!(game.calculated_power(source),Some(if accept&&available {3}else{1}));
        if let Some(id)=own {if accept {assert_wastes(&game,id);}else{assert_eq!(game.current_characteristics(id).unwrap().name.as_ref(),"Ancient grove");}}
        assert_eq!(game.current_characteristics(opponent).unwrap().name.as_ref(),"Ancient grove");
        assert_wastes(&game,wastes);
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.calculated_power(source),Some(1));
        if accept {if let Some(id)=own {assert_wastes(&game,id);}}
    } } }
}
#[test]
fn clearing_one_characteristic_family_does_not_clear_other_families() {
    for spell in definitions("Subtype reset","Type: Sorcery\nTarget permanent becomes a land named Oasis and loses all other subtypes.") {
        let original=card("Snowy relic","Type: Legendary Snow Artifact Creature — Equipment Human\nPower/Toughness: 4/5\nFlying\n{T}: Add {G}.");
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&spell,A,Zone::Stack);
        let target=game.create_object_from_definition(&original,A,Zone::Battlefield);
        let mut ctx=EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(target)]);
        for effect in spell.spell_effect.as_ref().unwrap().all_effects() {execute_effect(&mut game,effect,&mut ctx).unwrap();}
        let view=game.current_characteristics(target).unwrap();
        assert_eq!(view.name.as_ref(),"Oasis");
        assert_eq!(view.card_types.as_slice(),&[CardType::Land]);
        assert!(view.subtypes.is_empty());
        assert!(view.supertypes.contains(&Supertype::Snow));
        assert!(view.supertypes.contains(&Supertype::Legendary));
        assert_eq!(game.current_abilities(target).unwrap().len(),2,"abilities were not among the cleared families");
    }
}

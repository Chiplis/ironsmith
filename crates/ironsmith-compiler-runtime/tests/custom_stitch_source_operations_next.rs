//! Exact full sources plus atomic resolving prices and exact token-owned collections.
use ironsmith::{CardDefinition, CardType, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::BooleanContext;
use ironsmith::effects::{EffectContext, execute_effect};
#[path = "custom_final_next_wave_support.rs"] mod source_support;
const A: PlayerId = PlayerId(0);
#[derive(Default)] struct Decisions { accept: bool }
impl DecisionMaker for Decisions { fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.accept } }
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    source_support::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap_or_else(|error|panic!("{name}: {error}"))
}
fn raw(name: &str, types: Vec<CardType>) -> CardDefinition {
    ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(),name).card_types(types).build()
}
fn canonical(size: i32) -> String {
    format!("Type: Sorcery\nPerform the assemble action with tokens created this way if any are created: [You may pay the cost \"Exile a creature card from your graveyard; Exile an instant or sorcery card from your graveyard\". If you do, do the following: [Remember the original cards exiled this way. Create a {size}/{size} blue Zombie creature token. Record the remembered cards in the assembled collection of the created tokens while those members remain in exile.]].")
}
fn resolve(game: &mut GameState, definition: &CardDefinition, accept: bool) {
    let source=game.create_object_from_definition(definition,A,Zone::Stack);
    let mut dm=Decisions {accept};let mut ctx=EffectContext::new_default(source,A).with_decision_maker(&mut dm);
    for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects() {execute_effect(game,effect,&mut ctx).unwrap();}
}
fn tokens(game:&GameState)->Vec<ObjectId> {game.battlefield.iter().copied().filter(|id|game.object(*id).is_some_and(|object|matches!(object.kind,ironsmith::object::ObjectKind::Token))).collect()}
#[test]
fn every_formal_source_face_is_strict_and_materialized_on_both_routes() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-stitch-fullsource.json")).unwrap();
    let cards:Vec<_>=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).collect();
    assert_eq!(cards.len(),15);
    for card in cards { for definition in source_support::definitions(card).unwrap_or_else(|error|panic!("{}: {error}",card["name"])) {assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));} }
}
fn source_routes(name:&str)->[CardDefinition;2] {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-stitch-fullsource.json")).unwrap();
    let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap();
    source_support::definitions(card).unwrap_or_else(|error|panic!("{name}: {error}"))
}
#[test]
fn actual_overfueled_fallback_distinguishes_no_token_from_accepted_creation() {
    for definition in source_routes("Overfueled Abomination") {
        for accepted in [false,true] {
            let mut game=GameState::new(vec!["A".into(),"B".into()],20);
            let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
            game.create_object_from_definition(&raw("Creature material",vec![CardType::Creature]),A,Zone::Graveyard);
            game.create_object_from_definition(&raw("Spell material",vec![CardType::Instant]),A,Zone::Graveyard);
            let trigger=definition.abilities.iter().find_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Triggered(trigger)=>Some(trigger),_=>None}).unwrap();
            let mut dm=Decisions {accept:accepted};let mut ctx=EffectContext::new_default(source,A).with_decision_maker(&mut dm);
            for effect in trigger.effects.all_effects() {execute_effect(&mut game,effect,&mut ctx).unwrap();}
            assert_eq!(tokens(&game).len(),usize::from(accepted));
            assert_eq!(game.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne),if accepted {0}else{2});
        }
    }
}
#[test]
fn actual_return_then_stitch_binds_resolution_power_instead_of_outer_x() {
    for definition in source_routes("It's Alive!") {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Returned witness","Type: Creature\nPower/Toughness: 3/2",false).unwrap();
        let returned=game.create_object_from_definition(&creature,A,Zone::Graveyard);
        game.create_object_from_definition(&raw("Creature material",vec![CardType::Creature]),A,Zone::Graveyard);
        game.create_object_from_definition(&raw("Spell material",vec![CardType::Instant]),A,Zone::Graveyard);
        let source=game.create_object_from_definition(&definition,A,Zone::Stack);
        let mut dm=Decisions {accept:true};let mut ctx=EffectContext::new_default(source,A).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(returned)]).with_x(9).with_decision_maker(&mut dm);
        for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects() {execute_effect(&mut game,effect,&mut ctx).unwrap();}
        let token=tokens(&game)[0];assert_eq!(game.calculated_power(token),Some(3));assert_eq!(game.calculated_toughness(token),Some(3));
        assert_eq!(game.source_collection_objects(token,"stitched").len(),2);
    }
}
#[test]
fn actual_variable_activation_creates_tapped_before_entry() {
    for definition in source_routes("Nephalia Heartsewer") {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        game.create_object_from_definition(&raw("Creature material",vec![CardType::Creature]),A,Zone::Graveyard);
        game.create_object_from_definition(&raw("Spell material",vec![CardType::Instant]),A,Zone::Graveyard);
        let activation=definition.abilities.iter().filter_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Activated(activation)=>Some(activation),_=>None}).nth(1).unwrap();
        let mut dm=Decisions {accept:true};let mut ctx=EffectContext::new_default(source,A).with_x(4).with_decision_maker(&mut dm);
        for effect in activation.effects.all_effects() {execute_effect(&mut game,effect,&mut ctx).unwrap();}
        let token=tokens(&game)[0];assert!(game.is_tapped(token));assert_eq!(game.calculated_power(token),Some(4));
        assert_eq!(game.source_collection_objects(token,"stitched").len(),2);
    }
}
#[test]
fn whole_price_is_optional_atomic_and_never_reuses_one_dual_type_card() {
    for definition in definitions("Ordinary assembly operation",&canonical(3)) {
        for (accept,second,dual) in [(false,true,false),(true,false,false),(true,false,true),(true,true,false)] {
            let mut game=GameState::new(vec!["A".into(),"B".into()],20);
            let first=game.create_object_from_definition(&raw("Creature material",if dual {vec![CardType::Creature,CardType::Sorcery]}else{vec![CardType::Creature]}),A,Zone::Graveyard);
            let first_stable=game.object(first).unwrap().stable_id;
            if second {game.create_object_from_definition(&raw("Spell material",vec![CardType::Instant]),A,Zone::Graveyard);}
            resolve(&mut game,&definition,accept);
            let produced=tokens(&game);
            assert_eq!(produced.len(),usize::from(accept&&second));
            assert_eq!(game.object(game.find_object_by_stable_id(first_stable).unwrap()).unwrap().zone,if accept&&second {Zone::Exile}else{Zone::Graveyard});
            if let Some(token)=produced.first() {assert_eq!(game.calculated_power(*token),Some(3));assert_eq!(game.calculated_toughness(*token),Some(3));assert_eq!(game.source_collection_objects(*token,"assembled").len(),2);}
        }
    }
}
#[test]
fn source_scoped_alternative_price_works_without_the_default_and_creates_once() {
    let alternative="Type: Creature — Zombie\nPower/Toughness: 1/1\nYou may pay the cost \"Exile a Zombie card from your graveyard\" rather than the cost of performing the assemble action.";
    let operation=definitions("Assembly operation",&canonical(4));let providers=definitions("Price provider",alternative);
    for (definition,provider) in operation.iter().zip(providers.iter()) {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        game.create_object_from_definition(provider,A,Zone::Battlefield);
        let zombie=ironsmith_compiler_runtime::compile_to_runtime_definition("Zombie material","Type: Creature — Zombie\nPower/Toughness: 2/2",false).unwrap();
        game.create_object_from_definition(&zombie,A,Zone::Graveyard);
        resolve(&mut game,definition,true);
        let produced=tokens(&game);assert_eq!(produced.len(),1);
        assert_eq!(game.source_collection_objects(produced[0],"assembled").len(),1);
        assert_eq!(game.calculated_power(produced[0]),Some(4));
    }
}
#[test]
fn linked_members_and_owner_do_not_follow_later_incarnations() {
    for definition in definitions("Assembly operation",&canonical(2)) {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        game.create_object_from_definition(&raw("Creature",vec![CardType::Creature]),A,Zone::Graveyard);
        game.create_object_from_definition(&raw("Instant",vec![CardType::Instant]),A,Zone::Graveyard);
        resolve(&mut game,&definition,true);let token=tokens(&game)[0];
        let members=game.source_collection_objects(token,"assembled");assert_eq!(members.len(),2);
        let grave=game.move_object_by_effect(members[0],Zone::Graveyard).unwrap();
        let reexiled=game.move_object_by_effect(grave,Zone::Exile).unwrap();
        assert!(!game.source_collection_objects(token,"assembled").contains(&reexiled));
        assert_eq!(game.source_collection_objects(token,"assembled").len(),1);
        let left=game.move_object_by_effect(token,Zone::Graveyard).unwrap();
        assert_eq!(game.source_collection_objects(token,"assembled").len(),1,"queued old-source abilities retain exact old-owner membership");
        // CR 111.8: a token that left the battlefield can't come back, so no
        // later incarnation exists to inherit the old token's members.
        assert!(game.move_object_by_effect(left,Zone::Battlefield).is_none());
    }
}
#[test]
fn original_arrival_binding_excludes_independent_added_exile_and_reexiled_ids() {
    use ironsmith::effect::{EffectId, EffectOutcome, ExecutionFact};
    use ironsmith::effects::EffectExecutor;
    use ironsmith::snapshot::ObjectSnapshot;
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    let source=game.create_object_from_definition(&raw("Source",vec![CardType::Artifact]),A,Zone::Battlefield);
    let original=game.create_object_from_definition(&raw("Original",vec![CardType::Creature]),A,Zone::Exile);
    let added=game.create_object_from_definition(&raw("Addition",vec![CardType::Instant]),A,Zone::Exile);
    let capture=ironsmith_core::effect::RememberOriginalMovementArrivalsEffect {from:EffectId(77),zone:Zone::Exile,tag:"remembered".into()};
    let mut ctx=EffectContext::new_default(source,A);
    let receipt=EffectOutcome::count(1).with_execution_fact(ExecutionFact::OriginalZoneMoveCards(vec![ObjectSnapshot::from_object(game.object(original).unwrap(),&game)]));
    let addition=EffectOutcome::count(1).with_execution_fact(ExecutionFact::OriginalZoneMoveCards(vec![ObjectSnapshot::from_object(game.object(added).unwrap(),&game)]));
    ctx.store_outcome(EffectId(77),EffectOutcome::aggregate_replacement_outcomes(receipt,[addition]));
    capture.execute(&mut game,&mut ctx).unwrap();
    assert_eq!(ctx.get_tagged_all("remembered").unwrap().iter().map(|snapshot|snapshot.object_id).collect::<Vec<_>>(),vec![original]);
    let moved=game.move_object_by_effect(original,Zone::Graveyard).unwrap();game.move_object_by_effect(moved,Zone::Exile).unwrap();
    let record=ironsmith_core::RecordSourceCollectionEffect {target:ironsmith::target::ChooseSpec::Tagged("remembered".into()),source_tag:None,name:"exact remembered".into(),source_zone:None,member_zone:Zone::Exile};
    record.execute(&mut game,&mut ctx).unwrap();
    assert!(game.source_collection_objects(source,"exact remembered").is_empty(),"relation bookkeeping cannot follow an old tag into a later exile incarnation");
    capture.execute(&mut game,&mut ctx).unwrap();assert!(ctx.get_tagged_all("remembered").unwrap().is_empty());
}

#[test]
fn actual_master_granted_copy_cast_reads_old_token_collection_after_departure() {
    for definition in source_routes("Master Skaabstitcher") {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        game.create_object_from_definition(&raw("Creature material",vec![CardType::Creature]),A,Zone::Graveyard);
        let instant=ironsmith_compiler_runtime::compile_to_runtime_definition("Castable witness","Mana cost: {3}\nType: Instant\nYou gain 1 life.",false).unwrap();
        game.create_object_from_definition(&instant,A,Zone::Graveyard);
        let entering=definition.abilities.iter().find_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Triggered(trigger)=>Some(trigger),_=>None}).unwrap();
        let mut dm=Decisions {accept:true};
        {let mut ctx=EffectContext::new_default(source,A).with_decision_maker(&mut dm);for effect in entering.effects.all_effects() {execute_effect(&mut game,effect,&mut ctx).unwrap();}}
        let token=tokens(&game)[0];
        let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(token).unwrap(),&game);
        let trigger=game.object(token).unwrap().abilities.iter().find_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Triggered(trigger)=>Some(trigger.clone()),_=>None}).expect("creation-time copiable grant");
        let original=game.source_collection_objects(token,"stitched").into_iter().find(|id|game.object(*id).unwrap().name=="Castable witness").unwrap();
        game.move_object_by_effect(token,Zone::Graveyard).unwrap();
        let mut ctx=EffectContext::new_default(token,A).with_source_snapshot(snapshot).with_decision_maker(&mut dm);
        for effect in trigger.effects.all_effects() {execute_effect(&mut game,effect,&mut ctx).unwrap();}
        assert_eq!(game.object(original).unwrap().zone,Zone::Exile,"the linked original is copied, never cast or moved");
        assert_eq!(game.stack.len(),1);
        assert!(matches!(game.object(game.stack[0].object_id).unwrap().kind,ironsmith::object::ObjectKind::SpellCopy));
    }
}
#[test]
fn copying_a_created_token_does_not_copy_its_linked_material_collection() {
    for definition in definitions("Assembly operation",&canonical(2)) {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        game.create_object_from_definition(&raw("Creature",vec![CardType::Creature]),A,Zone::Graveyard);
        game.create_object_from_definition(&raw("Instant",vec![CardType::Instant]),A,Zone::Graveyard);
        resolve(&mut game,&definition,true);let original=tokens(&game)[0];
        let copy=Effect::new(ironsmith::effects::CreateTokenCopyEffect::new(ironsmith::target::ChooseSpec::SpecificObject(original),1,ironsmith::target::PlayerFilter::You));
        execute_effect(&mut game,&copy,&mut EffectContext::new_default(original,A).with_decision_maker(&mut SelectFirstDecisionMaker)).unwrap();
        let copied=tokens(&game).into_iter().find(|id|*id!=original).unwrap();
        assert_eq!(game.source_collection_objects(original,"assembled").len(),2);
        assert!(game.source_collection_objects(copied,"assembled").is_empty());
    }
}

#[test]
fn actual_overfueled_paid_prevented_creation_uses_original_receipt_even_with_an_added_token() {
    use ironsmith::replacement::{ReplacementAction,ReplacementEffect};
    use ironsmith::target::{ObjectFilter,PlayerFilter};
    for definition in source_routes("Overfueled Abomination") {
        for independent_addition in [false,true] {
            let mut game=GameState::new(vec!["A".into(),"B".into()],20);
            let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
            let materials=[game.create_object_from_definition(&raw("Creature material",vec![CardType::Creature]),A,Zone::Graveyard),
                game.create_object_from_definition(&raw("Spell material",vec![CardType::Instant]),A,Zone::Graveyard)];
            let material_stables=materials.map(|id|game.object(id).unwrap().stable_id);
            // Prevent the original creation only; this price is still paid.
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                source,A,ironsmith::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(PlayerFilter::You),ReplacementAction::Prevent));
            if independent_addition {
                let witness=ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(),"Independent added token")
                    .token().card_types(vec![CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(1,1)).build();
                game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                    source,A,ironsmith::events::other::WouldNamedActionMatcher{label:"stitch".into(),source_filter:ObjectFilter::specific(source),performer_filter:Some(PlayerFilter::You),object_requirement:None},
                    ReplacementAction::Additionally(vec![Effect::new(ironsmith::effects::CreateTokenEffect::new(witness,1,PlayerFilter::You))])));
            }
            let trigger=definition.abilities.iter().find_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Triggered(trigger)=>Some(trigger),_=>None}).unwrap();
            let mut dm=Decisions{accept:true};let mut ctx=EffectContext::new_default(source,A).with_decision_maker(&mut dm);
            for effect in trigger.effects.all_effects(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
            for stable in material_stables {assert!(game.exile.iter().any(|id|game.object(*id).unwrap().stable_id==stable),"the real two-card price completed");}
            assert_eq!(tokens(&game).len(),usize::from(independent_addition));
            assert_eq!(game.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne),2,"accepted payment and independent addition cannot satisfy original creation");
            if independent_addition {assert_eq!(game.object(tokens(&game)[0]).unwrap().name.as_ref(),"Independent added token");}
        }
    }
}

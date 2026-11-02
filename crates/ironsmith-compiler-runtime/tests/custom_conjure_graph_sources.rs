//! Complete source-input gates; execution belongs to the parent integration wave.
mod custom_final_next_wave_support;
use custom_final_next_wave_support as source;

#[test]
fn remaining_161_sources_compile_and_exact_user_boundary_stays_unresolved() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-conjure-whole-family-source-inputs.json")).unwrap();
    let rows = fixture["mechanics"][0]["representative_cards"].as_array().unwrap();
    assert_eq!(rows.len(), 162);
    let boundaries=fixture["unresolved_source_boundaries"].as_array().unwrap();
    assert_eq!(boundaries.len(),1);
    assert_eq!(boundaries[0]["source_card_key"],"Tunu, Ancient Shaper_MSC");
    let mut failures = Vec::new();
    let mut checked=0;
    let mut unresolved=0;
    for card in rows {
        if card["source_card_key"]==boundaries[0]["source_card_key"] {
            assert!(card["original_text"].as_str().unwrap().contains(boundaries[0]["ability_text"].as_str().unwrap()));
            assert_eq!(card["unresolved_source_boundary"]["status"],"explicit_user_boundary");
            unresolved+=1;
            continue; // No full-body or remaining-ability support certification.
        }
        checked+=1;
        if let Err(error) = source::definitions(card) {
            failures.push(format!("{}: {error}",card["source_card_key"].as_str().unwrap()));
        }
    }
    assert_eq!(checked,161);assert_eq!(unresolved,1);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn generated_self_references_survive_standalone_export_and_fresh_game() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::effects::{EffectContext, execute_effect};
    let id=ironsmith_core::CardId::new();
    let root_text="Type: Sorcery\nCreate a real card named \"Returning design\" in your hand as part of the named making action.";
    let bundle=ironsmith_compiler::compile_supplied_card_definition_bundle(id,"Returning design",root_text,true,&[],|| {
        ironsmith_compiler::CompilerFacade::new().compile_definition(
            ironsmith_compiler::CardDefinitionBuilder::new(id,"Returning design"),
            "Type: Sorcery\nCreate a real card named \"Returning design\" in your hand as part of the named making action.",
            ironsmith_compiler::CompilePolicy{allow_unsupported:false},
        ).map(|compiled|compiled.definition).map_err(|error|error.to_string())
    }).unwrap();
    let root=ironsmith_compiler_runtime::into_runtime_definition_bundle(bundle).unwrap();
    let actor=PlayerId(0);
    fn run(game:&mut GameState,actor:PlayerId,definition:&ironsmith::CardDefinition) {
        let source=game.create_object_from_definition(definition,actor,Zone::Stack);
        let mut context=EffectContext::new_default(source,actor);
        for effect in definition.spell_effect.as_ref().unwrap().flattened_default_effects() {
            execute_effect(game,effect,&mut context).unwrap();
        }
    }
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    run(&mut game,actor,&root);
    let created=*game.player(actor).unwrap().hand.first().unwrap();
    let definition_id=game.object(created).unwrap().card.unwrap();
    let generated=game.retained_card_definition(definition_id).unwrap().clone();
    let wire=ironsmith::artifact_materializer::encode_runtime_definition(generated).unwrap();
    let json=serde_json::to_string(&wire).unwrap();
    let restored=ironsmith::artifact_materializer::materialize_definition(serde_json::from_str(&json).unwrap()).unwrap();
    // No old GameState, compiler scope, or process registry supplies this generation.
    let mut fresh=GameState::new(vec!["A".into(),"B".into()],20);
    run(&mut fresh,actor,&restored);
    let hand=&fresh.player(actor).unwrap().hand;
    assert_eq!(hand.len(),1);
    assert_eq!(fresh.object(hand[0]).unwrap().name,"Returning design");
    assert_eq!(fresh.object(hand[0]).unwrap().kind,ironsmith::object::ObjectKind::Card);
}

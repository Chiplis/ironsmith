//! Source metadata must reach native interpretation and serialized transport.
#[path = "custom_final_next_wave_support.rs"]
mod source;
use serde_json::json;
// Compiler result tags/other transparent composition are allowed. Inspect the
// complete typed program, then execute that same program through its real owner.
fn token_blueprints(definition: &ironsmith::CardDefinition) -> Vec<ironsmith::CardDefinition> {
    fn visit(effect: &ironsmith::Effect, found: &mut Vec<ironsmith::CardDefinition>) {
        if let Some(create) = effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>() {
            found.push(create.token.clone());
        }
        effect.visit_child_effects(&mut |child| visit(child, found));
    }
    let mut found = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() { visit(effect, &mut found); }
    assert_eq!(found.len(), 1, "one source instruction must keep exactly one token creation, including inside result tags");
    found
}
fn resolve_token(definition: &ironsmith::CardDefinition, name: &str) -> (ironsmith::GameState, ironsmith::ObjectId) {
    let a = ironsmith::PlayerId(0);
    let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into()], 20);
    let spell = game.create_object_from_definition(definition, a, ironsmith::Zone::Stack);
    let mut ctx = ironsmith::effects::EffectContext::new_default(spell, a);
    let program = ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(definition.spell_effect.as_ref().unwrap().all_effects_owned()));
    ironsmith::effects::execute_effect(&mut game, &program, &mut ctx).unwrap();
    let created: Vec<_> = game.battlefield.iter().copied().filter(|id| game.object(*id).is_some_and(|object| object.name == name) && game.current_controller(*id) == Some(a)).collect();
    assert_eq!(created.len(), 1, "the actual native/artifact program must create its supplied token exactly once");
    (game, created[0])
}
#[test]
fn declared_counter_programs_are_catalog_entries_not_intrinsic_abilities() {
    let input = json!({"name":"Counter vocabulary host","executable_text":"Type: Creature — LocalPerformer\nPower/Toughness: 1/1",
        "custom_subtypes":[{"singular":"LocalPerformer","plural":"LocalPerformers","family":"Creature"}],
        "custom_keyword_counter_definitions":[{"name":"witness","reminder":"Flying"}]});
    for definition in source::definitions(&input).unwrap() {
        assert_eq!(definition.card.subtypes.len(), 1);
        assert!(
            definition
                .ability_counter_definitions
                .contains_key("witness")
        );
        assert!(!definition.abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::AbilityKind::Static(rule) if rule.id()==ironsmith::static_abilities::StaticAbilityId::Flying)));
    }
    let mut missing = input;
    missing.as_object_mut().unwrap().remove("custom_subtypes");
    assert!(
        source::definitions(&missing).is_err(),
        "a previous source cannot admit a later undeclared subtype"
    );
}
#[test]
fn supplied_token_keeps_real_abilities_and_its_own_counter_catalog() {
    let input = json!({"name":"Named token maker","executable_text":"Type: Sorcery\nCreate a colorless artifact token named Keepsake.",
        "custom_token_definitions":[{"name":"Keepsake","token":true,"executable_text":"Type: Artifact\n{T}: Draw a card.",
            "custom_keyword_counter_definitions":[{"name":"witness","reminder":"Flying"}]}]});
    for definition in source::definitions(&input).unwrap() {
        let blueprints = token_blueprints(&definition);
        let create = &blueprints[0];
        assert_eq!(create.card.name, "Keepsake");
        assert!(
            create
                .abilities
                .iter()
                .any(|ability| matches!(&ability.kind, ironsmith::AbilityKind::Activated(_)))
        );
        assert!(
            create
                .ability_counter_definitions
                .contains_key("witness")
        );
        let (mut game, token) = resolve_token(&definition, "Keepsake");
        assert!(game.object(token).unwrap().abilities.iter().any(|ability| matches!(&ability.kind, ironsmith::AbilityKind::Activated(_))));
        let flying = ironsmith::static_abilities::StaticAbility::flying();
        assert!(!game.object_has_ability(token, &flying), "catalog entries are not intrinsic abilities");
        game.add_counters(token, ironsmith::CounterType::Named("witness".into()), 1);
        assert!(game.object_has_ability(token, &flying), "created token must retain its own ability-counter catalog at runtime");
    }
}
#[test]
fn physical_linked_faces_keep_matching_ids_layout_and_independent_face_programs() {
    for layout in ["transform", "modal_dfc"] {
        let card = json!({"name":"Front witness","source_layout":layout,
            "source_faces":[{"name":"Front witness"},{"name":"Back witness"}],
            "executable_faces":[
                {"name":"Front witness","executable_text":"Mana cost: {1}\nType: Creature\nPower/Toughness: 1/1"},
                {"name":"Back witness","executable_text":"Mana cost: {2}\nType: Creature\nPower/Toughness: 2/2"}]});
        assert!(
            source::definitions(&card).is_err(),
            "single-face adapter cannot discard a supplied sibling"
        );
        let faces = source::linked_definitions(&card).unwrap();
        assert_eq!(faces.len(), 2);
        for route in 0..2 {
            assert_eq!(
                faces[0][route].card.other_face,
                Some(faces[1][route].card.id)
            );
            assert_eq!(
                faces[1][route].card.other_face,
                Some(faces[0][route].card.id)
            );
            assert_eq!(
                faces[0][route].card.other_face_name.as_deref(),
                Some("Back witness")
            );
            assert_eq!(
                faces[0][route].card.linked_face_layout,
                ironsmith::card::LinkedFaceLayout::TransformLike
            );
            assert_eq!(faces[0][route].card.transforming_dfc, layout == "transform");
        }
    }
}
#[test]
fn malformed_or_conflicting_source_catalogs_fail_before_either_route() {
    let base = json!({"name":"Invalid catalog probe","executable_text":"Type: Creature\nPower/Toughness: 1/1"});
    let mut counter = base.clone();
    counter["custom_keyword_counter_definitions"] =
        json!([{"name":"witness","reminder":"Draw a card."}]);
    assert!(source::definitions(&counter).is_err());
    counter["custom_keyword_counter_definitions"] =
        json!([{"name":"witness","reminder":"Flying"},{"name":"WITNESS","reminder":"Haste"}]);
    assert!(source::definitions(&counter).is_err());
    let mut token = base.clone();
    token["custom_token_definitions"] =
        json!([{"name":"Keepsake","executable_text":"Type: Artifact"}]);
    assert!(
        source::definitions(&token).is_err(),
        "unmarked card is not a token blueprint"
    );
    token["custom_token_definitions"] = json!([{"name":"Keepsake","token":true,"executable_text":"Mana cost: {1}\nType: Artifact"}]);
    assert!(source::definitions(&token).is_err());
    let mut malformed = base;
    malformed["custom_subtypes"] = json!({"name":"Ignored"});
    assert!(source::definitions(&malformed).is_err());
}

#[test]
fn explicit_expected_token_dependencies_require_supplied_blueprints() {
    let dependency=json!({"face":{"name":"Record"},"input":{"name":"Record","cardTypes":["Enchantment"],"oracleText":"Attacking creatures you control get +1/+0."}});
    let blueprint=json!({"name":"Record","token":true,"executable_text":"Type: Enchantment\nAttacking creatures you control get +1/+0."});
    let input=json!({"name":"Supplied dependency host","executable_text":"Type: Sorcery\nCreate a colorless artifact token named Record.",
        "custom_tokens":[dependency],"custom_token_definitions":[blueprint]});
    for definition in source::definitions(&input).unwrap() {
        let blueprints=token_blueprints(&definition);let token=&blueprints[0];
        assert_eq!(token.card.card_types,vec![ironsmith::CardType::Enchantment]);
        assert!(!token.abilities.is_empty());
        let (game,created)=resolve_token(&definition,"Record");
        assert_eq!(game.object(created).unwrap().card_types.as_slice(),&[ironsmith::CardType::Enchantment]);
        assert!(!game.object(created).unwrap().abilities.is_empty(),"supplied anthem survives actual token creation");
    }
    let mut missing=input.clone();missing.as_object_mut().unwrap().remove("custom_token_definitions");
    let error=source::definitions(&missing).unwrap_err();
    assert!(error.contains("missing supplied token definition"),"{error}");
    missing["custom_token_definitions"]=json!([]);
    assert!(source::definitions(&missing).is_err());
    for malformed in [json!("Record"),json!([{}]),json!([{"input":{"name":""}}]),
        json!([{"face":{"name":"Other"},"input":{"name":"Record"}}])] {
        let mut bad=input.clone();bad["custom_tokens"]=malformed;
        assert!(source::definitions(&bad).is_err());
    }
    // An ordinary named artifact with no explicit source dependency remains
    // a valid creation instruction, without requiring any global registry.
    let ordinary=json!({"name":"Independent named artifact host","executable_text":"Type: Sorcery\nCreate a colorless artifact token named Record."});
    for definition in source::definitions(&ordinary).unwrap() {
        let blueprints=token_blueprints(&definition);
        assert_eq!(blueprints[0].card.card_types,vec![ironsmith::CardType::Artifact]);
        let (game,created)=resolve_token(&definition,"Record");
        assert_eq!(game.object(created).unwrap().card_types.as_slice(),&[ironsmith::CardType::Artifact]);
    }
}
#[test]
fn nested_expected_token_dependencies_require_prior_catalog_entries() {
    let child=json!({"name":"Child record","token":true,"executable_text":"Type: Artifact"});
    let parent=json!({"name":"Parent record","token":true,"executable_text":"Type: Artifact\n{T}: Create a colorless artifact token named Child record.",
        "custom_tokens":[{"input":{"name":"Child record"}}]});
    let mut input=json!({"name":"Ordered supplied catalog host","executable_text":"Type: Sorcery\nCreate a colorless artifact token named Parent record.",
        "custom_tokens":[{"input":{"name":"Parent record"}}],"custom_token_definitions":[child,parent]});
    assert!(source::definitions(&input).is_ok());
    input["custom_token_definitions"].as_array_mut().unwrap().reverse();
    assert!(source::definitions(&input).is_err(),"forward dependencies must not fall back to blank named tokens");
    input["custom_token_definitions"]=json!([{"name":"Parent record","token":true,"executable_text":"Type: Artifact","custom_tokens":[{"input":{"name":"Parent record"}}]}]);
    assert!(source::definitions(&input).is_err(),"a token cannot satisfy its own required blueprint dependency");
}

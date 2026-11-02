//! Complete physical sources; independent native/artifact routes preserve every sibling.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
// Native Effect::PartialEq deliberately compares executor Arc identity, not
// independently constructed executable programs. Compare complete typed wire
// definitions instead: every numeric identity, program, cost, target and source
// metadata field remains present; no ID or semantic field is discarded.
fn canonical_source(definition: ironsmith::CardDefinition) -> Result<serde_json::Value, String> {
    let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(definition)
        .map_err(|error|format!("canonical runtime encoding: {error:?}"))?;
    serde_json::to_value(wire).map_err(|error|error.to_string())
}
fn source_deltas(left:&serde_json::Value,right:&serde_json::Value)->Vec<String>{
    fn walk(path:&str,left:&serde_json::Value,right:&serde_json::Value,out:&mut Vec<String>){
        if left==right || out.len()>=8{return;}
        match(left,right){
            (serde_json::Value::Object(a),serde_json::Value::Object(b))=>{
                let keys:std::collections::BTreeSet<_>=a.keys().chain(b.keys()).collect();
                for key in keys{match(a.get(key),b.get(key)){
                    (Some(a),Some(b))=>walk(&format!("{path}/{key}"),a,b,out),
                    _=>out.push(format!("{path}/{key}: field presence differs")),
                }if out.len()>=8{break;}}
            },
            (serde_json::Value::Array(a),serde_json::Value::Array(b))=>{
                if a.len()!=b.len(){out.push(format!("{path}: array length {} != {}",a.len(),b.len()));}
                for (index,(a,b)) in a.iter().zip(b).enumerate(){walk(&format!("{path}/{index}"),a,b,out);if out.len()>=8{break;}}
            },
            _=>out.push(format!("{path}: direct={} artifact={}",left.to_string().chars().take(240).collect::<String>(),right.to_string().chars().take(240).collect::<String>())),
        }
    }
    let mut out=Vec::new();walk("$",left,right,&mut out);out
}
#[test]
fn canonical_parity_erases_only_executor_pointer_identity_and_detects_semantic_deltas(){
    let input=serde_json::json!({"name":"Parity witness","executable_text":"Type: Sorcery\nDraw a card."});
    let [direct,artifact]=source_support::definitions(&input).unwrap();
    assert_ne!(direct.spell_effect,artifact.spell_effect,"independent runtime executors intentionally have different allocation identity");
    let left=canonical_source(direct).unwrap();let right=canonical_source(artifact).unwrap();
    assert_eq!(left,right,"same complete typed source program must survive independent routes");
    let mut changed=input;changed["executable_text"]=serde_json::json!("Type: Sorcery\nDraw two cards.");
    let [changed,_]=source_support::definitions(&changed).unwrap();
    let mut changed=canonical_source(changed).unwrap();
    // Reuse the original printed-card record only for this deliberate negative
    // witness: its effect must still differ after allocation identity is gone.
    changed["card"]=left["card"].clone();
    let deltas=source_deltas(&left,&changed);
    assert!(!deltas.is_empty());
    assert!(deltas.iter().any(|delta|delta.contains("spell_effect")),"draw quantity must remain observable: {deltas:?}");
    let mut identity=left.clone();identity["card"]["id"]=serde_json::json!(999999);
    assert!(!source_deltas(&left,&identity).is_empty(),"numeric card identity must never be normalized away");
}
#[test]
fn every_owned_actual_source_face_compiles_and_survives_artifact_transport_without_loss() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for row in fixture["mechanics"].as_array().unwrap() {
        for card in row["representative_cards"].as_array().unwrap() {
            let faces = card["executable_faces"]
                .as_array()
                .expect("complete physical source must retain executable faces");
            count += faces.len();
            let key = format!("{} / {}", row["id"], card["id"]);
            match source_support::linked_definitions(card) {
                Err(error) => failures.push(format!("{key}: {error}")),
                Ok(routes) => {
                    if routes.len() != faces.len() {
                        failures.push(format!("{key}: native/artifact face count changed"));
                        continue;
                    }
                    for (index, [direct, materialized]) in routes.into_iter().enumerate() {
                        match (canonical_source(direct),canonical_source(materialized)) {
                            (Ok(direct),Ok(materialized))=>{
                                let deltas=source_deltas(&direct,&materialized);
                                if !deltas.is_empty(){failures.push(format!("{key} / {}: native/artifact typed source deltas:\n{}",faces[index]["name"],deltas.join("\n")));}
                            },
                            (Err(error),_)=>failures.push(format!("{key} / {}: direct {error}",faces[index]["name"])),
                            (_,Err(error))=>failures.push(format!("{key} / {}: artifact {error}",faces[index]["name"])),
                        }
                    }
                }
            }
        }
    }
    assert_eq!(
        count, 126,
        "all101 complete physical sources and their sibling faces remain in the matrix"
    );
    assert!(
        failures.is_empty(),
        "{} source failures:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}


#[test]
fn dormant_memory_complete_source_retains_future_second_cast_phase_hold_both_routes() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json")).unwrap();
    let card = fixture["mechanics"].as_array().unwrap().iter()
        .flat_map(|row| row["representative_cards"].as_array().unwrap())
        .find(|card| card["source_card_key"] == "Dormant Memory_EOW").unwrap();
    for definition in source_support::definitions(card).unwrap() {
        let encoded = canonical_source(definition).unwrap().to_string();
        assert!(encoded.contains("UntilControllerCastsNthSpellInTurn"), "{encoded}");
    }
}


#[test]
fn branded_usurper_complete_source_retains_any_opponent_next_turn_boundary_both_routes() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json")).unwrap();
    let card = fixture["mechanics"].as_array().unwrap().iter()
        .flat_map(|row| row["representative_cards"].as_array().unwrap())
        .find(|card| card["source_card_key"] == "The Marquess Devonport//The Branded Usurper_ETF").unwrap();
    let definitions = source_support::linked_definitions(card).unwrap();
    assert_eq!(definitions.len(), 2);
    for definition in definitions.into_iter().last().unwrap() {
        let encoded = canonical_source(definition).unwrap().to_string();
        assert!(encoded.contains("OpponentsNextTurn"), "{encoded}");
    }
}

#[test]
fn active_counter_removal_and_future_phase_ordinal_are_strict_generic_readings() {
    for (name, body, retained) in [
        ("Counter removal witness", "Whenever you remove a time counter from this creature, draw a card.", "CounterRemovedFrom"),
        ("Another removal witness", "Whenever you remove a stun counter from this creature, draw a card.", "CounterRemovedFrom"),
        ("Future second cast witness", "Whenever you cast your second spell each turn, you may have this creature phase out until you cast your second spell in a turn.", "UntilControllerCastsNthSpellInTurn(2)"),
        ("Future third cast witness", "Whenever you cast your third spell each turn, you may have this creature phase out until you cast your third spell in a turn.", "UntilControllerCastsNthSpellInTurn(3)"),
    ] {
        let source=serde_json::json!({"name":name,"executable_text":format!("Type: Creature — Spirit\nPower/Toughness: 2/2\n{body}")});
        let [direct,artifact]=source_support::definitions(&source).unwrap_or_else(|error|panic!("{name}: {error}"));
        for definition in [&direct,&artifact] {
            let debug=format!("{:?}",definition.abilities);
            assert!(debug.contains(retained),"{name}: {debug}");
        }
        assert_eq!(canonical_source(direct).unwrap(),canonical_source(artifact).unwrap(),"{name}");
    }
}


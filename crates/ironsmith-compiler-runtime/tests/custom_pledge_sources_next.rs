use ironsmith_compiler_runtime::{compile_to_artifact,compile_to_runtime_definition};
#[path="custom_final_next_wave_support.rs"] mod source_routes;
fn definitions(name:&str,text:&str)->[ironsmith::cards::CardDefinition;2] {
    definitions_with_token(name,text,false)
}
fn definitions_with_token(name:&str,text:&str,token:bool)->[ironsmith::cards::CardDefinition;2] {
    let vocabulary:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/source-subtypes.json")).unwrap();
    let vocabulary:Vec<_>=vocabulary["definitions"].as_array().unwrap().iter()
        .filter(|definition|definition["scope"]=="rev").cloned().collect();
    source_routes::definitions(&serde_json::json!({
        "name":name,"executable_text":text,"token":token,"custom_subtypes":vocabulary,
    })).unwrap_or_else(|error|panic!("{name}: {error}"))
}
// A catalog's printed color annotation is metadata, not an effect. Admit
// its exact actual color through the typed header; preserve every ability.
fn catalog_token_text(token:&serde_json::Value)->(String,ironsmith::color::ColorSet){
 use ironsmith::color::{Color,ColorSet};
 let parse_colors=|text:&str|->Option<ColorSet>{
  let mut colors=ColorSet::COLORLESS;
  for word in text.split(|ch:char|!ch.is_alphanumeric()).filter(|word|!word.is_empty()&&!word.eq_ignore_ascii_case("and")){
   colors=colors.with(Color::from_mana_code_or_name(&word.to_ascii_lowercase())?);
  }Some(colors)
 };
 let raw=token["color"].as_str().expect("actual token color metadata");
 let colors=parse_colors(raw).expect("recognized actual token colors");
 let color_header=raw.trim().trim_matches(|ch|ch=='{'||ch=='}').replace('/',", ");
 let rules=token["rulesText"].as_str().unwrap().lines().filter(|line|{
  let Some(annotation)=line.trim().strip_prefix("*(This token is ").and_then(|line|line.strip_suffix(")*")) else{return true};
  assert_eq!(parse_colors(annotation.trim_end_matches('.')),Some(colors),"printed color annotation must agree with source metadata");false
 }).collect::<Vec<_>>().join("\n");
 let types=token["typeLine"].as_str().unwrap().strip_prefix("Token ").expect("source token metadata");
 (format!("Type: {types}\nColor indicator: {color_header}\n{rules}"),colors)
}
#[test]
fn designated_instruction_and_explicit_source_timing_exception_survive_both_routes() {
    let text="Type: Enchantment — Clique\nPledge — {4}: Surveil 2. Activate only as a sorcery unless you're making a pledge.\nWhen this enchantment enters, activate an activated ability with designation pledge of this permanent without paying its costs during making-a-pledge actions.";
    for definition in definitions("Labeled timing probe",text) {
        let activated=definition.abilities.iter().find_map(|a|if let ironsmith::ability::AbilityKind::Activated(a)=&a.kind {Some(a)}else{None}).unwrap();
        assert!(activated.effects.activation_designations.iter().any(|s|s=="pledge"));
        assert_eq!(activated.effects.activation_sorcery_exceptions,["making-a-pledge"]);
        assert_eq!(activated.timing,ironsmith::ability::ActivationTiming::SorcerySpeed);
        let debug=format!("{definition:?}");assert!(debug.contains("ActivateAbilityWithoutCostsEffect"));assert!(debug.contains("designation: Some(\"pledge\")"));
    }
}
#[test]
fn all_eighteen_exact_source_variants_include_every_associated_actual_card() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-pledge-exact-source-next.json")).unwrap();
    let rows=fixture["mechanics"].as_array().unwrap();assert_eq!(rows.len(),18);
    let mut count=0;
    for row in rows {
        for card in row["representative_cards"].as_array().unwrap() {
            let name=card["name"].as_str().unwrap();let input=card["executable_text"].as_str().unwrap();
            for definition in definitions(name,input) {
                let debug=format!("{definition:?}");assert!(debug.contains("ActivateAbilityWithoutCostsEffect"),"{name}: {debug}");
                assert!(debug.contains("making-a-pledge"),"{name}: {debug}");
            }
            count+=1;
        }
    }
    assert_eq!(count,35);
}
#[test]
fn all_five_exact_source_token_abilities_keep_their_authored_exception() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-pledge-exact-source-next.json")).unwrap();
    let tokens=fixture["token_source_catalog"].as_array().unwrap();assert_eq!(tokens.len(),5);
    for token in tokens {
        // The source catalog's leading Token denotes object metadata; it is
        // not a card type. Preserve the original catalog and admit the token
        // through the same production builder flag used by other fixtures.
        let (text,colors)=catalog_token_text(token);
        for definition in definitions_with_token(token["cardName"].as_str().unwrap(),&text,true) {
            assert_eq!(definition.card.color_indicator,Some(colors),"the token keeps its actual printed colors on each independent route");
            let activated=definition.abilities.iter().find_map(|a|if let ironsmith::ability::AbilityKind::Activated(a)=&a.kind {Some(a)}else{None}).unwrap();
            assert_eq!(activated.effects.activation_sorcery_exceptions,["making-a-pledge"]);
        }
    }
}
#[test]
fn repeated_action_creates_once_then_announces_existing_clique_ability_on_both_routes() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-pledge-exact-source-next.json")).unwrap();
    let row=&fixture["mechanics"][0];
    let text=format!("Type: Sorcery\n{}",row["executable_reminder"].as_str().unwrap());
    for definition in definitions("Repeated action probe",&text) {
        let a=ironsmith::PlayerId(0);
        let mut game=ironsmith::GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,a,ironsmith::Zone::Stack);
        for pass in 0..2 {
            let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
            let mut ctx=ironsmith::effects::EffectContext::new(source,a,&mut dm);
            for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
                ironsmith::effects::execute_effect(&mut game,effect,&mut ctx).unwrap();
            }
            assert_eq!(game.battlefield.len(),1,"pass {pass} must not create a second Clique");
            assert_eq!(game.stack.len(),pass,"only the second pass announces a real activated ability");
            assert_eq!(game.player(a).unwrap().mana_pool.total(),0);
        }
        assert!(game.stack[0].is_ability);
        assert_eq!(game.player(a).unwrap().life,20);
    }
}

#[test]
fn incomplete_instruction_menu_is_not_admitted_as_its_unconditional_child() {
    let text="Type: Sorcery\nChoose one instruction — Draw a card.";
    assert!(compile_to_runtime_definition("Incomplete menu",text,false).is_err());
    assert!(compile_to_artifact("Incomplete menu",text,false).is_err());
}

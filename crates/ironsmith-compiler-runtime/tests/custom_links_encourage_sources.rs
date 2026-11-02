//! Every exact physical source uses independent native/artifact routes and full typed parity.
#[path="custom_final_next_wave_support.rs"] mod source;
#[path="custom_typed_source_parity_support.rs"] mod parity;
#[test]
fn complete_source_programs_round_trip() {
    let mut count=0;let mut unavailable=Vec::new();let mut failures=Vec::new();
    for fixture in [
        include_str!("../../../fixtures/custom_mechanics/msem-trigger-effects.json")
    ] {
        let fixture:serde_json::Value=serde_json::from_str(fixture).unwrap();
        for row in fixture["mechanics"].as_array().unwrap() {
            if row["id"]!="msem-685"{continue;}
            for card in row["representative_cards"].as_array().unwrap() {
                if card["executable_text"].as_str().is_none() {
                    unavailable.push(format!("{} / {}",row["id"],card["name"]));continue;
                }
                // Pass the complete physical card to the strict source adapter:
                // source metadata and linked siblings cannot be discarded.
                match source::linked_definitions(card) {
                    Err(error)=>failures.push(format!("{} / {}: {error}",row["id"],card["name"])),
                    Ok(faces)=>for [native,artifact] in faces {
                        if let Err(error)=parity::compare(&native,&artifact) {
                            failures.push(format!("{} / {}: {error}",row["id"],native.card.name));
                        }
                    },
                }
                count+=1;
            }
        }
    }
    assert!(count>0);
    assert!(unavailable.is_empty(),"strict compile requires every source program: {unavailable:?}");
    assert!(failures.is_empty(),"{} source failures:\n{}",failures.len(),failures.join("\n\n"));
}

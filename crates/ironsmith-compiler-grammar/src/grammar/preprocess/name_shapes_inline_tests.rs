use super::*;

#[test]
fn titled_personal_names_keep_title_and_surname_in_short_references() {
    assert_eq!(parse_short_self_reference_name("Dr. Ada Example"), "Dr. Example");
    assert_eq!(parse_short_self_reference_name("Captain Ada B. Example"), "Captain Example");
    assert_eq!(parse_short_self_reference_name("Doctor Example"), "Doctor Example");
    assert_eq!(parse_short_self_reference_name("Captain Example the Bold"), "Captain Example");
    assert_ne!(parse_short_self_reference_name("Captain of the Watch"), "Captain Watch");
}

#[test]
fn parses_keyword_and_short_name_surfaces() {
    assert!(parse_single_keyword_verb("Mill").is_some());
    assert!(parse_keyword_ability_name("double strike").is_some());
    assert_eq!(
        parse_short_self_reference_name("Brago, King Eternal"),
        "Brago"
    );
    assert_eq!(
        parse_short_self_reference_name("Draw the Line"),
        "Draw the Line"
    );
    assert_eq!(
        parse_short_self_reference_name("Skeleton Crew"),
        "Skeleton Crew"
    );
    assert_eq!(
        parse_short_self_reference_name("Attached Count Anthem Variant"),
        "Attached Count Anthem Variant"
    );
    assert_eq!(
        parse_short_self_reference_name("Each Player Sacrifice Variant"),
        "Each Player Sacrifice Variant"
    );
    assert_eq!(
        parse_short_self_reference_name("Black Scarab"),
        "Black Scarab"
    );
    assert_eq!(
        parse_short_self_reference_name("Exiled Flashback Return Variant"),
        "Exiled Flashback Return Variant"
    );
    assert_eq!(
        parse_short_self_reference_name("Turn Static Boundary Variant"),
        "Turn Static Boundary Variant"
    );
    for name in ["And They Shall Know No Fear", "Then Draw Probe", "Or Choose Probe"] {
        assert_eq!(parse_short_self_reference_name(name), name);
    }
    assert_eq!(parse_short_self_reference_name("Ajani Vengeant"), "Ajani");
    assert_eq!(
        parse_short_self_reference_name("Enchanted River's Grasp"),
        "Enchanted River's Grasp",
        "an attached-object adjective must not become a source alias"
    );

    for name in [
        "Craft Variant",
        "Prototype Probe",
        "Escalate Probe",
        "Rampage Variant",
        "Learn Test",
        "Echo Variant",
        "Morph Variant",
        "Adapt Variant",
        "Vanishing Parse Test",
        "Sunburst Parse Test",
        "Removed Counter Mana Variant",
        "Destroyed Draw Variant",
        "Cast intervening-if probe",
        "Play with Fire",
        "Protection context grant",
        "Roll and mana",
        "Tapped Damage Variant",
        "Bottom Library Exile",
        "Target Opponent Put",
        "Villainous Choice Variant",
        "Same Name Search Probe",
        "Double Counter Probe",
        "Prevent Combat Probe",
        "Blocked Variant",
        "Chosen Copy Probe",
        "Snow Untap Probe",
        "Additional Cost Probe",
        "Then If Probe",
        "Nonhistoric Probe",
        "Out of Time",
    ] {
        assert_eq!(
            parse_short_self_reference_name(name),
            name,
            "mechanic heads must not become abbreviated source names"
        );
    }
}

#[test]
fn regnal_number_suffix_keeps_the_complete_multiword_alias() {
    assert_eq!(parse_short_self_reference_name("King Darien XLVIII"), "King Darien");
    assert_eq!(parse_short_self_reference_name("Queen Example XVII"), "Queen Example");
    assert_eq!(parse_short_self_reference_name("Brago, King Eternal"), "Brago");
    assert_eq!(parse_short_self_reference_name("Ajani Vengeant"), "Ajani");
}

#[test]
fn multiword_name_before_an_epithet_stays_intact() {
    assert_eq!(parse_short_self_reference_name("Vial Smasher the Fierce"), "Vial Smasher");
    assert_eq!(parse_short_self_reference_name("Captain Example the Bold"), "Captain Example");
    assert_eq!(parse_short_self_reference_name("Brago, King Eternal"), "Brago");
    assert_ne!(parse_short_self_reference_name("Oath of the Gatewatch"), "Oath of");
    assert_ne!(parse_short_self_reference_name("Journey to the Unknown"), "Journey to");
}

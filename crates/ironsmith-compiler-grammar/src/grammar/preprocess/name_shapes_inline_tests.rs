use super::*;

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

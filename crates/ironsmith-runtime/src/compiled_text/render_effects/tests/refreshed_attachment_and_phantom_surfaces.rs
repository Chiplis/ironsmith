use super::*;

fn render_card(
    name: &str,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    text: &str,
) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(card_types)
        .subtypes(subtypes)
        .parse_text(text)
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"));
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

fn assert_round_trip(name: &str, card_types: Vec<CardType>, subtypes: Vec<Subtype>, text: &str) {
    assert_eq!(
        render_card(name, card_types, subtypes, text),
        text,
        "{name}"
    );
}

#[test]
fn targeted_attachment_spells_keep_both_targets_in_the_attach_instruction() {
    assert_round_trip(
        "Magnetic Theft",
        vec![CardType::Instant],
        vec![],
        "Attach target Equipment to target creature.",
    );
    assert_round_trip(
        "Aura Finesse",
        vec![CardType::Instant],
        vec![],
        "Attach target Aura you control to target creature.\nDraw a card.",
    );
}

#[test]
fn attachment_fold_requires_the_destination_tag_consumed_by_attach() {
    let object = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default().with_subtype(Subtype::Equipment),
    ));
    let destination = ChooseSpec::target_creature();
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(object.clone())),
        Effect::new(crate::effects::TargetOnlyEffect::new(destination)).tag("attachment_target_a"),
        Effect::new(crate::effects::AttachObjectsEffect::new(
            object,
            ChooseSpec::Tagged(TagKey::from("attachment_target_b")),
        )),
    ];

    assert_ne!(
        describe_effect_list(&effects),
        "Attach target Equipment to target creature"
    );
}

#[test]
fn searched_attachment_and_equipment_entry_targets_stay_inline() {
    assert_round_trip(
        "Arachnus Spinner",
        vec![CardType::Creature],
        vec![Subtype::Spider],
        "Reach\nTap an untapped Spider you control: Search your graveyard and/or library for a card named Arachnus Web and put it onto the battlefield attached to target creature. If you search your library this way, shuffle.",
    );
    assert_round_trip(
        "Hunter's Bow",
        vec![CardType::Artifact],
        vec![Subtype::Equipment],
        "When this Equipment enters, attach it to target creature you control. That creature deals damage equal to its power to up to one target creature you don't control.\nEquipped creature has reach and ward {2}.\nEquip {1}",
    );
}

#[test]
fn flash_equipment_entry_family_keeps_the_attachment_target_inline() {
    let cases = [
        (
            "Barbed Bloodletter",
            "That creature gains wither until end of turn.",
            "Equipped creature gets +1/+2.",
            "{2}",
        ),
        (
            "Bladed Battle-Fan",
            "That creature gains indestructible until end of turn.",
            "Equipped creature gets +1/+0.",
            "{1}",
        ),
        (
            "Celestial Armor",
            "That creature gains hexproof and indestructible until end of turn.",
            "Equipped creature gets +2/+0 and has flying.",
            "{3}{W}",
        ),
        (
            "Coral Sword",
            "That creature gains first strike until end of turn.",
            "Equipped creature gets +1/+0.",
            "{1}",
        ),
        (
            "Galadhrim Bow",
            "Untap that creature.",
            "Equipped creature gets +1/+2 and has reach.",
            "{2}",
        ),
        (
            "Hidden Blade",
            "If that creature is an Assassin, it gains deathtouch until end of turn.",
            "Equipped creature gets +1/+0 and has first strike.",
            "{2}",
        ),
        (
            "Hidden Footblade",
            "That creature gains first strike until end of turn.",
            "Equipped creature gets +1/+0 and has haste.",
            "{2}",
        ),
        (
            "Illvoi Light Jammer",
            "That creature gains hexproof until end of turn.",
            "Equipped creature gets +1/+2.",
            "{3}",
        ),
        (
            "Quick-Draw Dagger",
            "That creature gains first strike until end of turn.",
            "Equipped creature gets +1/+1.",
            "{1}",
        ),
        (
            "Silver Shroud Costume",
            "That creature gains shroud until end of turn.",
            "Equipped creature can't be blocked.",
            "{3}",
        ),
        (
            "Squire's Lightblade",
            "That creature gains first strike until end of turn.",
            "Equipped creature gets +1/+0.",
            "{3}",
        ),
        (
            "Stolen Stark Tech",
            "That creature gains indestructible until end of turn.",
            "Equipped creature gets +1/+0.",
            "{1}",
        ),
        (
            "Super Suit",
            "Untap that creature.",
            "Equipped creature gets +1/+2.",
            "{2}",
        ),
        (
            "Twin Blades",
            "That creature gains double strike until end of turn.",
            "Equipped creature gets +1/+1.",
            "{2}",
        ),
    ];
    for (name, entry_follow_up, equipped, equip_cost) in cases {
        let text = format!(
            "Flash\nWhen this Equipment enters, attach it to target creature you control. {entry_follow_up}\n{equipped}\nEquip {equip_cost}"
        );
        assert_round_trip(
            name,
            vec![CardType::Artifact],
            vec![Subtype::Equipment],
            &text,
        );
    }
}

#[test]
fn phantom_family_preserves_the_separate_counter_removal_sentence() {
    let cases = [
        ("Phantom Nomad", "two", ""),
        ("Phantom Tiger", "two", ""),
        ("Phantom Wurm", "four", ""),
        ("Phantom Centaur", "three", "Protection from black\n"),
        ("Phantom Flock", "three", "Flying\n"),
        (
            "Phantom Nishoba",
            "seven",
            "Trample\nWhenever this creature deals damage, you gain that much life.\n",
        ),
        ("Phantom Nantuko", "two", "Trample\n"),
    ];
    for (name, count, prefix) in cases {
        let suffix = if name == "Phantom Nantuko" {
            "\n{T}: Put a +1/+1 counter on this creature."
        } else {
            ""
        };
        let text = format!(
            "{prefix}This creature enters with {count} +1/+1 counters on it.\nIf damage would be dealt to this creature, prevent that damage. Remove a +1/+1 counter from this creature.{suffix}"
        );
        assert_round_trip(name, vec![CardType::Creature], vec![], &text);
    }
}

#[test]
fn conjoined_counter_removal_surface_is_not_split() {
    assert_round_trip(
        "Conjoined Counter Prevention Probe",
        vec![CardType::Creature],
        vec![],
        "If damage would be dealt to this creature, prevent that damage and remove one shield counter from it.",
    );
}

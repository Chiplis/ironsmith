use super::*;

const MINE: (&str, &str, &str) = (
    "Urza's Mine",
    "Land — Urza's Mine",
    "{T}: Add {C}. If you control an Urza's Power-Plant and an Urza's Tower, add {C}{C} instead.",
);
const POWER_PLANT: (&str, &str, &str) = (
    "Urza's Power Plant",
    "Land — Urza's Power-Plant",
    "{T}: Add {C}. If you control an Urza's Mine and an Urza's Tower, add {C}{C} instead.",
);
const TOWER: (&str, &str, &str) = (
    "Urza's Tower",
    "Land — Urza's Tower",
    "{T}: Add {C}. If you control an Urza's Mine and an Urza's Power-Plant, add {C}{C}{C} instead.",
);

/// Compile the printed card, type line included, through the real document
/// path: the type line is what gives each Tron land its compound subtypes.
fn compile_printed((name, type_line, text): (&str, &str, &str)) -> crate::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .parse_text(format!("Type: {type_line}\n{text}"))
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"))
}

/// Whether `land`'s "add more instead" branch applies with the given Tron
/// companions on the battlefield, as the engine selects it for both mana
/// planning and resolution.
fn tron_bonus_applies(land: (&str, &str, &str), companions: &[(&str, &str, &str)]) -> bool {
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let definition = compile_printed(land);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for companion in companions {
        game.create_object_from_definition(&compile_printed(*companion), alice, Zone::Battlefield);
    }
    let program = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Activated(activated) => Some(&activated.effects),
            _ => None,
        })
        .expect("Tron land should have a mana ability");
    let [segment] = program.segments.as_slice() else {
        panic!("{}: expected one mana segment", land.0);
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("{}: expected one 'instead' branch", land.0);
    };
    let selected =
        crate::ability::selected_resolution_effects_for_current_state(program, &game, source, alice);
    let is_selected = |effects: &[crate::effect::Effect]| {
        selected.len() == effects.len()
            && selected
                .iter()
                .zip(effects)
                .all(|(chosen, effect)| std::ptr::eq(*chosen, effect))
    };
    if is_selected(&branch.replacement_effects) {
        true
    } else {
        assert!(is_selected(&segment.default_effects), "{}", land.0);
        false
    }
}

#[test]
fn printed_tron_lands_carry_both_halves_of_their_land_type() {
    for (land, expected) in [
        (MINE, Subtype::Mine),
        (POWER_PLANT, Subtype::PowerPlant),
        (TOWER, Subtype::Tower),
    ] {
        let definition = compile_printed(land);
        assert_eq!(
            definition.card.subtypes,
            vec![Subtype::Urzas, expected],
            "{}",
            land.0
        );
    }
}

#[test]
fn printed_tron_lands_render_each_full_compound_land_type() {
    for (land, expected) in [
        (
            MINE,
            "{T}: Add {C}. If you control an Urza's Power-Plant and you control an Urza's Tower, add {C}{C} instead.",
        ),
        (
            POWER_PLANT,
            "{T}: Add {C}. If you control an Urza's Mine and you control an Urza's Tower, add {C}{C} instead.",
        ),
        (
            TOWER,
            "{T}: Add {C}. If you control an Urza's Mine and you control an Urza's Power-Plant, add {C}{C}{C} instead.",
        ),
    ] {
        let definition = compile_printed(land);
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [expected],
            "{}",
            land.0
        );
    }
}

#[test]
fn assembled_tron_selects_the_bonus_and_partial_tron_does_not() {
    assert!(tron_bonus_applies(TOWER, &[MINE, POWER_PLANT]));
    assert!(tron_bonus_applies(MINE, &[POWER_PLANT, TOWER]));
    assert!(tron_bonus_applies(POWER_PLANT, &[MINE, TOWER]));

    assert!(!tron_bonus_applies(TOWER, &[MINE]), "Tower needs a Power-Plant too");
    assert!(!tron_bonus_applies(TOWER, &[MINE, MINE]), "two Mines are not a Power-Plant");
    assert!(!tron_bonus_applies(MINE, &[TOWER]), "Mine needs a Power-Plant too");
    assert!(!tron_bonus_applies(POWER_PLANT, &[MINE]), "Power Plant needs a Tower too");
    assert!(!tron_bonus_applies(TOWER, &[]));
}

#[test]
fn urzas_factory_token_keeps_its_hyphenated_assembly_worker_type() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Urza's Factory")
        .parse_text(
            "Type: Land — Urza's\n{T}: Add {C}.\n{7}, {T}: Create a 2/2 colorless Assembly-Worker artifact creature token.",
        )
        .expect("Urza's Factory should compile");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [
            "{T}: Add {C}.",
            "{7}, {T}: Create a 2/2 colorless Assembly-Worker artifact creature token."
        ]
    );
}

#[test]
fn metalcraft_mana_restriction_renders_once() {
    for (name, type_line, text) in [
        (
            "Mox Opal",
            "Legendary Artifact",
            "Metalcraft — {T}: Add one mana of any color. Activate only if you control three or more artifacts.",
        ),
        (
            "Urza's Workshop",
            "Land — Urza's",
            "Metalcraft — {T}: Add {C} for each Urza's land you control. Activate only if you control three or more artifacts.",
        ),
    ] {
        let definition = compile_printed((name, type_line, text));
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [text],
            "{name}"
        );
    }
}

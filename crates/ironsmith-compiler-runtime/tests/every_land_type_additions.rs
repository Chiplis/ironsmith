//! "Each land with an everything counter on it is every land type in addition
//! to its other types." (Omo, Queen of Vesuva): CR 205.3i/305.7 additive
//! land-type family grant with the explicit "in addition" tail.
//! Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p02_line_families/compile.rs"]
mod compile;

const OMO: &str = "Mana cost: {2}{G/U}\nType: Legendary Creature — Shapeshifter Noble\nPower/Toughness: 1/5\nWhenever Omo enters or attacks, put an everything counter on each of up to one target land and up to one target creature.\nEach land with an everything counter on it is every land type in addition to its other types.\nEach nonland creature with an everything counter on it is every creature type.";

#[test]
fn omo_adds_every_land_type_and_every_creature_type() {
    for definition in compile::compile_both("Omo, Queen of Vesuva", OMO) {
        let statics: Vec<String> = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) if ability.id() != ironsmith::static_abilities::StaticAbilityId::SourceLineStaticGroup => Some(format!("{ability:?}")),
                _ => None,
            })
            .collect();
        assert!(statics.iter().any(|debug| debug.contains("Land") && debug.to_ascii_lowercase().contains("everything")), "{statics:?}");
        assert!(statics.iter().any(|debug| debug.contains("Creature") && debug.to_ascii_lowercase().contains("everything")), "{statics:?}");
        assert_eq!(statics.len(), 2, "{statics:?}");

        // Source-line group markers preserve source ownership and are not
        // additional rules. Verify both actual subtype rules through gameplay.
        use ironsmith::{GameState, PlayerId, Zone, CounterType};
        use ironsmith::types::Subtype;
        let player = PlayerId(0);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, player, Zone::Battlefield);
        let land = ironsmith_compiler_runtime::compile_to_runtime_definition("Subtype land witness", "Type: Land — Forest", false).unwrap();
        let creature = ironsmith_compiler_runtime::compile_to_runtime_definition("Subtype creature witness", "Type: Creature — Elf\nPower/Toughness: 2/2", false).unwrap();
        let marked_land = game.create_object_from_definition(&land, player, Zone::Battlefield);
        let unmarked_land = game.create_object_from_definition(&land, player, Zone::Battlefield);
        let marked_creature = game.create_object_from_definition(&creature, player, Zone::Battlefield);
        let unmarked_creature = game.create_object_from_definition(&creature, player, Zone::Battlefield);
        for id in [marked_land, marked_creature] {
            game.add_counters(id, CounterType::Named("everything".into()), 1).unwrap();
        }
        // CR 205.3i: "every land type" is the official list; legacy custom
        // vocabulary (Satrapy) is only admitted per game, never implied.
        for subtype in Subtype::all_land_types().iter().filter(|subtype| !subtype.is_custom_vocabulary()) {
            assert!(game.current_has_subtype(marked_land, *subtype), "missing land subtype {subtype:?}");
        }
        for subtype in Subtype::all_creature_types().iter().filter(|subtype| !subtype.is_custom_vocabulary()) {
            assert!(game.current_has_subtype(marked_creature, *subtype), "missing creature subtype {subtype:?}");
        }
        assert_eq!(game.current_subtypes(unmarked_land).unwrap(), vec![Subtype::Forest]);
        assert_eq!(game.current_subtypes(unmarked_creature).unwrap(), vec![Subtype::Elf]);
        assert!(!game.current_has_subtype(marked_creature, Subtype::Plains), "the nonland creature clause grants only creature subtypes");
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(game.current_subtypes(marked_land).unwrap(), vec![Subtype::Forest], "the land grant is additive and ends with its source");
        assert_eq!(game.current_subtypes(marked_creature).unwrap(), vec![Subtype::Elf], "printed creature subtype survives after the source leaves");
    }
}

//! cf8 p01 round 3: "as you activate this ability" is kept as an
//! activation-time sampling hint on where-X values. Unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn activation_time_where_x_values_keep_their_clause() {
    for name in ["Agility Bobblehead", "Endurance Bobblehead", "Lukka, Bound to Ruin"] {
        for definition in support::definitions(name) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("AsYouActivateThisAbility"), "{name}: {debug}");
            let text = support::rendered(&definition);
            assert!(text.contains("as you activate this ability"), "{name}: {text}");
        }
    }
}

/// Keeper of the Beasts: the target opponent must control more creatures than
/// you as you activate; on resolution only the opponent relation is rechecked
/// (CR 601.2c via 602.2b, 608.2b).
#[test]
fn keeper_of_the_beasts_targets_an_opponent_with_more_creatures_as_you_activate() {
    for definition in support::definitions("Keeper of the Beasts") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("OpponentWithMoreControlledObjectsThan"), "{debug}");
        assert!(debug.contains("as_you_activate: true"), "{debug}");
        assert!(debug.contains("CreateToken"), "{debug}");
        let text = support::rendered(&definition);
        assert!(
            text.contains("target opponent who controls more creatures than you do as you activate this ability"),
            "{text}"
        );
        assert!(!text.contains("target creature"), "{text}");
    }
}

#[path = "cf8_p08/play.rs"]
mod play;

#[test]
fn announced_value_survives_costs_responses_and_checkpoint_clone() {
    let text = "Type: Creature\nPower/Toughness: 2/2\nSacrifice this creature: Draw X cards, where X is the number of creatures you control as you activate this ability.";
    for definition in support::definitions_for_text("Activation sample probe", text) {
        let mut game = play::game();
        let source = game.create_object_from_definition(&definition, play::A, ironsmith::Zone::Battlefield);
        let other = play::vanilla("Other", "{1}", "Bear", 2, 2);
        game.create_object_from_definition(&other, play::A, ironsmith::Zone::Battlefield);
        for _ in 0..4 { game.create_object_from_definition(&other, play::A, ironsmith::Zone::Library); }
        let mut dm = play::Script::default();
        play::activate(&mut game, source, play::activated_at(&definition, 0), &mut dm);
        let entry = game.stack.last().unwrap();
        assert_eq!(entry.ability_effects.as_ref().unwrap().activation_values.len(), 1);
        assert_eq!(entry.ability_effects.as_ref().unwrap().activation_values[0].1, Some(2));
        // Costs already removed the source; a response adds another creature.
        for _ in 0..3 { game.create_object_from_definition(&other, play::A, ironsmith::Zone::Battlefield); }
        let mut restored = game.clone();
        ironsmith::game_loop::resolve_stack_entry_with(&mut restored, &mut dm).unwrap();
        assert_eq!(restored.player(play::A).unwrap().hand.len(), 2);
    }
}

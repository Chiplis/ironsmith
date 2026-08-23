use super::shard_16::parse_oracle_card_definition;
use super::*;

fn triggered_modal_choice(definition: &CardDefinition) -> &ChooseModeEffect {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>()),
            _ => None,
        })
        .expect("card should contain a triggered modal choice")
}

#[test]
pub(super) fn distinct_player_modal_cards_preserve_rule_counts_and_mode_effects() {
    let cases: &[(&str, &[&str])] = &[
        (
            "Balor",
            &[
                "Target opponent draws three cards, then discards three cards at random.",
                "Target opponent sacrifices a nontoken artifact of their choice.",
                "This creature deals damage to target opponent equal to the number of cards in their hand.",
            ],
        ),
        (
            "Donnie & April, Adorkable Duo",
            &[
                "Target player draws two cards.",
                "Target player returns an artifact, instant, or sorcery card from their graveyard to their hand.",
            ],
        ),
        (
            "Mikey & Mona, Mutant Sitters",
            &[
                "Target player chooses a creature they control and puts two +1/+1 counters on it.",
                "Target player returns a creature or land card from their graveyard to their hand.",
            ],
        ),
        (
            "Splinter & Leo, Father & Son",
            &[
                "Target player creates a 2/2 red Mutant creature token.",
                "Put a +1/+1 counter on each other creature target player controls.",
            ],
        ),
        (
            "Vindictive Lich",
            &[
                "Target opponent sacrifices a creature of their choice.",
                "Target opponent discards two cards.",
                "Target opponent loses 5 life.",
            ],
        ),
    ];

    for (name, expected_modes) in cases {
        let definition = parse_oracle_card_definition(name);
        let modal = triggered_modal_choice(&definition);
        let rendered = unprocessed_compiled_lines(&definition).join("\n");
        let actual_modes = modal
            .modes
            .iter()
            .map(|mode| mode.source_text.as_str())
            .collect::<Vec<_>>();

        assert!(
            rendered.contains("Each mode must target a different player."),
            "{name} should render the modal target constraint: {rendered}"
        );
        assert!(modal.distinct_player_targets_per_mode, "{name}: {modal:#?}");
        assert_eq!(modal.min_choose_count, Value::Fixed(1), "{name}");
        assert_eq!(
            modal.choose_count,
            Value::Fixed(expected_modes.len() as i32),
            "{name}"
        );
        assert_eq!(actual_modes, *expected_modes, "{name}");
    }
}

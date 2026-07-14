use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
pub(super) fn opening_hand_reveal_cards_compile_to_typed_pregame_actions() {
    let cases = [
        (
            "Chancellor of the Annex",
            "You may reveal this card from your opening hand. If you do, when each opponent casts their first spell of the game, counter that spell unless that player pays {1}.",
        ),
        (
            "Chancellor of the Forge",
            "You may reveal this card from your opening hand. If you do, at the beginning of the first upkeep, create a 1/1 red Phyrexian Goblin creature token with haste.",
        ),
        (
            "Chancellor of the Spires",
            "You may reveal this card from your opening hand. If you do, at the beginning of the first upkeep, each opponent mills seven cards.",
        ),
        (
            "Chancellor of the Tangle",
            "You may reveal this card from your opening hand. If you do, at the beginning of your first main phase of the game, add {G}.",
        ),
        (
            "Sphinx of Foresight",
            "You may reveal this card from your opening hand. If you do, scry 3 at the beginning of your first upkeep.",
        ),
    ];

    for (name, expected_opening_line) in cases {
        let definition = parse_oracle_card_definition(name);
        let pregame = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => Some(ability),
                _ => None,
            })
            .find(|ability| {
                matches!(
                    ability.pregame_action_kind(),
                    Some(crate::static_abilities::PregameActionKind::RevealFromOpeningHand(_))
                )
            })
            .unwrap_or_else(|| panic!("{name} should have a typed opening-hand reveal action"));
        let effects = pregame
            .pregame_action_effects()
            .expect("typed pregame consequence effects");
        assert_eq!(effects.len(), 1, "unexpected {name} pregame program");
        assert!(
            effects[0]
                .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
                .is_some(),
            "{name} should schedule its consequence as a delayed trigger: {effects:#?}"
        );

        let rendered = canonical_compiled_lines(&definition);
        assert!(
            rendered.iter().any(|line| line == expected_opening_line),
            "{name} opening action did not round-trip structurally; expected {expected_opening_line:?}, got {rendered:#?}"
        );
    }
}

#[test]
pub(super) fn chancellor_annex_uses_game_scoped_first_spell_trigger() {
    let definition = parse_oracle_card_definition("Chancellor of the Annex");
    let schedule = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.pregame_action_effects(),
            _ => None,
        })
        .flatten()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>())
        .expect("Annex opening-hand delayed trigger");
    let trigger = schedule
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()
        .expect("Annex spell-cast trigger");

    assert!(trigger.first_spell_of_game);
    assert_eq!(trigger.caster, PlayerFilter::Opponent);
    assert!(!schedule.one_shot, "Annex must watch every opponent");
}

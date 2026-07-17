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

#[test]
pub(super) fn eumidian_wastewaker_preserves_both_players_choice_and_shared_graveyard_count() {
    let definition = parse_oracle_card_definition("Eumidian Wastewaker");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Eumidian Wastewaker attack trigger");
    let effects = triggered.effects.flattened_default_effects();
    let [tagged_choices, draw] = effects else {
        panic!("expected joint choice followed by a draw, got {effects:#?}");
    };

    let (choices_tag, choices_effect) =
        if let Some(tagged) = tagged_choices.downcast_ref::<crate::effects::TaggedEffect>() {
            (&tagged.tag, tagged.effect.as_ref())
        } else if let Some(tagged) = tagged_choices.downcast_ref::<crate::effects::TagAllEffect>() {
            (&tagged.tag, tagged.effect.as_ref())
        } else {
            panic!("joint choices should share an affected-object tag: {tagged_choices:#?}");
        };
    assert_eq!(choices_tag.as_str(), "joint_discard_or_sacrifice");
    let choices = choices_effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the two printed player choices should remain coordinated");
    assert_eq!(choices.effects.len(), 2, "{choices:#?}");
    let choice_players = choices
        .effects
        .iter()
        .map(|effect| {
            effect
                .downcast_ref::<crate::effects::UnlessActionEffect>()
                .expect("each player should choose discard or sacrifice")
                .player
                .clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        choice_players,
        vec![PlayerFilter::You, PlayerFilter::Defending]
    );

    let draw = draw
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .expect("land cards moved this way should determine the draw count");
    let crate::effect::Value::Count(filter) = draw.count.unhinted() else {
        panic!("expected a typed tagged-land count, got {draw:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.card_types, [crate::types::CardType::Land]);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == "joint_discard_or_sacrifice"
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }));

    let rendered = canonical_compiled_lines(&definition).join("\n");
    assert!(
        rendered.contains(
            "Whenever this creature attacks, you and defending player each discard a card or sacrifice a permanent. You draw a card for each land card put into a graveyard this way."
        ),
        "{rendered}"
    );
}

#[test]
pub(super) fn creation_of_avacyn_keeps_source_exiled_card_through_all_chapters() {
    let definition = parse_oracle_card_definition("The Creation of Avacyn");
    let debug = format!("{definition:#?}");

    assert_eq!(definition.abilities.len(), 3, "{debug}");
    assert!(debug.contains("searched_face_down"), "{debug}");
    assert!(debug.contains("TurnFaceUpEffect"), "{debug}");
    assert!(debug.contains("__source_exiled__"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(debug.contains("Declined"), "{debug}");
    assert!(
        !debug.contains("TagTriggeringObjectEffect"),
        "the saga chapter trigger must not replace the exiled-card antecedent: {debug}"
    );

    let rendered = canonical_compiled_lines(&definition).join("\n");
    assert!(rendered.contains("the exiled card"), "{rendered}");
    assert!(rendered.contains("its owner's hand"), "{rendered}");
}

#[test]
pub(super) fn attack_or_block_end_of_combat_cards_keep_typed_delayed_payloads() {
    let cases = [
        (
            "Clockwork Beetle",
            "Whenever this creature attacks or blocks, remove a +1/+1 counter from it at end of combat.",
            "RemoveCountersEffect",
        ),
        (
            "Saprazzan Outrigger",
            "When this creature attacks or blocks, put it on top of its owner's library at end of combat.",
            "MoveToZoneEffect",
        ),
        (
            "Wicker Warcrawler",
            "Whenever this creature attacks or blocks, put a -1/-1 counter on it at end of combat.",
            "PutCountersEffect",
        ),
    ];

    for (name, expected_line, expected_payload) in cases {
        let definition = parse_oracle_card_definition(name);
        let triggered = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{name} should have an attack-or-block trigger"));
        assert_eq!(
            triggered.trigger.display(),
            expected_line
                .split_once(',')
                .expect("triggered oracle line")
                .0,
            "{name} should compact the shared source subject"
        );

        let immediate = triggered.effects.flattened_default_effects();
        assert_eq!(
            immediate.len(),
            2,
            "{name} should capture the triggering creature and schedule one delayed payload: {immediate:#?}"
        );
        assert!(
            immediate[0]
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some(),
            "{name} should snapshot the triggering creature before scheduling: {immediate:#?}"
        );
        let schedule = immediate[1]
            .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            .unwrap_or_else(|| panic!("{name} should schedule its payload: {immediate:#?}"));
        assert!(schedule.one_shot, "{name} delay should fire once");
        assert!(
            schedule
                .trigger
                .downcast_ref::<crate::triggers::EndOfCombatTrigger>()
                .is_some(),
            "{name} should wait for end of combat: {schedule:#?}"
        );
        let payload_debug = format!("{:#?}", schedule.effects);
        assert!(
            payload_debug.contains(expected_payload),
            "{name} should keep its typed delayed payload {expected_payload}: {payload_debug}"
        );

        let rendered = canonical_compiled_lines(&definition);
        assert!(
            rendered.iter().any(|line| line == expected_line),
            "{name} delayed trigger should round-trip exactly; expected {expected_line:?}, got {rendered:#?}"
        );
    }
}

#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn faerie_tauntings_keeps_the_chooser_distinct_from_each_affected_opponent() {
    let definition = parse_oracle_card_definition("Faerie Tauntings");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Whenever you cast a spell during an opponent's turn, you may have each opponent lose 1 life."
                .to_string(),
        ]
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Faerie Tauntings should have one triggered ability");
    let debug = format!("{:#?}", triggered.effects);
    assert!(debug.contains("MayEffect"), "{debug}");
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("Opponent"), "{debug}");
    assert!(debug.contains("LoseLifeEffect"), "{debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");
}

#[test]
fn entropic_battlecruiser_public_route_uses_station_rows_and_per_player_failure() {
    let definition = parse_oracle_card_definition("Entropic Battlecruiser");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Station".to_string(),
            "1+ | Whenever an opponent discards a card, they lose 3 life.".to_string(),
            "8+ | Flying, deathtouch".to_string(),
            "Whenever this Spacecraft attacks, each opponent discards a card. Each opponent who can't loses 3 life."
                .to_string(),
        ]
    );

    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("CountersOnSource"), "{debug}");
    assert!(debug.contains("Charge"), "{debug}");
    assert!(debug.contains("DidNotHappen"), "{debug}");
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
}

#[test]
fn pizza_face_keeps_the_anaphoric_animation_inside_its_trigger() {
    let definition = parse_oracle_card_definition("Pizza Face, Gastromancer");
    let rendered = canonical_compiled_lines(&definition);
    assert_eq!(
        rendered.get(1).map(String::as_str),
        Some(
            "Disappear — At the beginning of your end step, if a permanent left the battlefield under your control this turn, put three +1/+1 counters on up to one other target artifact or creature. If it isn't a creature, it becomes a 0/0 Mutant creature in addition to its other types."
        ),
        "{rendered:#?}"
    );

    assert!(
        definition.abilities.iter().all(|ability| {
            !matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if format!("{static_ability:#?}").contains("Mutant")
            )
        }),
        "the target-relative animation must not become a global static ability: {:#?}",
        definition.abilities
    );
    let disappear = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .presentation_label
                    .as_ref()
                    .and_then(crate::ability::PresentationLabel::display_prefix)
                    .is_some_and(|label| label == "Disappear") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Pizza Face should retain its labeled end-step trigger");
    let debug = format!("{:#?}", disappear.effects);
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(debug.contains("ConditionalEffect"), "{debug}");
    assert!(debug.contains("AddSubtypes"), "{debug}");
    assert!(debug.contains("SetBasePowerToughness"), "{debug}");
}

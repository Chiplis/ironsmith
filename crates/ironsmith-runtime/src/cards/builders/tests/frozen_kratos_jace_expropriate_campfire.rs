#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn kratos_keeps_both_repeated_intro_trigger_events_and_partner_label_case() {
    let definition = parse_oracle_card_definition("Kratos, Stoic Father");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Whenever you attack with one or more Gods and whenever a God dies, you get an experience counter.".to_string(),
            "At the beginning of your end step, put a number of +1/+1 counters on target creature equal to the number of experience counters you have.".to_string(),
            "Partner—Father & son".to_string(),
        ]
    );
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .display()
                    .contains("attack with one or more Gods") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Kratos should keep the group-attack branch");
    let debug = format!("{:#?}", triggered.trigger);
    assert!(debug.contains("AttacksTrigger"), "{debug}");
    assert!(debug.contains("ZoneChangeTrigger"), "{debug}");
}

#[test]
fn jaces_cast_choice_is_scoped_to_the_cards_milled_by_the_trigger() {
    let definition = parse_oracle_card_definition("Jace's Mindseeker");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Flying".to_string(),
            "When this creature enters, target opponent mills five cards. You may cast an instant or sorcery spell from among them without paying its mana cost.".to_string(),
        ]
    );
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Jace should have an enters trigger");
    let debug = format!("{:#?}", triggered.effects);
    assert!(debug.contains("MillEffect"), "{debug}");
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("CastTaggedEffect"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
}

#[test]
fn expropriate_keeps_voter_relative_money_choice_after_the_time_clause() {
    let definition = parse_oracle_card_definition("Expropriate");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Council's dilemma — Starting with you, each player votes for time or money. For each time vote, take an extra turn after this one. For each money vote, choose a permanent owned by the voter and gain control of it. Exile Expropriate.".to_string(),
        ]
    );
    let spell = definition
        .spell_effect
        .as_ref()
        .expect("Expropriate should have a spell program");
    let debug = format!("{spell:#?}");
    assert!(debug.contains("VoteEffect"), "{debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");
    assert!(debug.contains("VoteCount"), "{debug}");
    assert!(debug.contains("\"time\""), "{debug}");
}

#[test]
fn campfire_moves_owned_commanders_from_both_zones_while_source_stays_battlefield_active() {
    let definition = parse_oracle_card_definition("Campfire");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "{1}, {T}: You gain 2 life.".to_string(),
            "{2}, {T}, Exile this artifact: Put all commanders you own from the command zone and from your graveyard into your hand. Then shuffle your graveyard into your library.".to_string(),
        ]
    );
    let activated = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some((ability, activated)),
            _ => None,
        })
        .find(|(_, activated)| format!("{:#?}", activated.effects).contains("is_commander: true"))
        .expect("Campfire should keep its two-zone commander activation");
    assert_eq!(activated.0.functional_zones, vec![Zone::Battlefield]);
    let debug = format!("{:#?}", activated.1.effects);
    assert!(debug.contains("Command"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert_eq!(debug.matches("is_commander: true").count(), 2, "{debug}");
}

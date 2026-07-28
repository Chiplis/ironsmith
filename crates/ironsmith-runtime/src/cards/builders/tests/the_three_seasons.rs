#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const THIRD_CHAPTER: &str = "III — Choose three cards in each graveyard. Their owners shuffle those cards into their libraries.";

fn third_chapter() -> crate::ability::TriggeredAbility {
    parse_oracle_card_definition("The Three Seasons")
        .abilities
        .into_iter()
        .filter_map(|ability| match ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .nth(2)
        .expect("The Three Seasons should have a third Saga chapter")
}

#[test]
fn the_three_seasons_compiles_to_exact_oracle_text() {
    let definition = parse_oracle_card_definition("The Three Seasons");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "I — Mill three cards.",
            "II — Return up to two target snow permanent cards from your graveyard to your hand.",
            THIRD_CHAPTER,
        ]
    );
}

#[test]
fn the_three_seasons_partitions_selection_by_graveyard_and_shuffles_the_chosen_set() {
    let chapter = third_chapter();
    assert!(
        chapter.choices.is_empty(),
        "the owner shuffle must not invent a target-player choice: {chapter:#?}"
    );
    let for_players = chapter.effects.segments[0]
        .default_effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ForPlayersEffect>())
        .expect("the choice should run once for each graveyard owner");
    assert_eq!(for_players.filter, PlayerFilter::Any);

    let [choose_effect, shuffle_effect] = for_players.effects.as_slice() else {
        panic!("expected one choice and one owner shuffle: {for_players:#?}");
    };
    let choose = choose_effect
        .downcast_ref::<ChooseObjectsEffect>()
        .expect("each player iteration should choose from that player's graveyard");
    assert_eq!(choose.chooser, PlayerFilter::You);
    assert_eq!(choose.count, crate::effect::ChoiceCount::exactly(3));
    assert_eq!(choose.zone, Some(Zone::Graveyard));
    assert_eq!(choose.filter.zone, Some(Zone::Graveyard));
    assert_eq!(choose.filter.owner, Some(PlayerFilter::IteratedPlayer));

    let shuffle_effect = shuffle_effect
        .downcast_ref::<WithIdEffect>()
        .map_or(shuffle_effect, |with_id| &with_id.effect);
    let shuffle = shuffle_effect
        .downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()
        .expect("the selected collection should move and shuffle as one typed effect");
    assert!(
        matches!(
            shuffle.target.base(),
            ChooseSpec::Tagged(tag) if tag == &choose.tag
        ),
        "shuffle should consume the per-graveyard chosen set: {shuffle:#?}"
    );
    assert!(
        matches!(
            &shuffle.player,
            PlayerFilter::OwnerOf(ObjectRef::Tagged(tag)) if tag == &choose.tag
        ),
        "each selected card's owner should own the shuffled library: {shuffle:#?}"
    );
}

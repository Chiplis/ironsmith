#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn find_draw_cards(effect: &Effect) -> Option<DrawCardsEffect> {
    if let Some(draw) = effect.downcast_ref::<DrawCardsEffect>() {
        return Some(draw.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_draw_cards(child);
        }
    });
    found
}

fn assert_revealed_hand_union_count(card_name: &str, expected_line: &str) {
    let definition = parse_oracle_card_definition(card_name);
    let program = definition
        .spell_effect
        .as_ref()
        .expect("the sorcery should have a resolution program");
    let draw = program
        .flattened_default_effects()
        .iter()
        .find_map(|effect| find_draw_cards(effect))
        .expect("the second sentence should remain a typed draw action");
    assert_eq!(draw.player, PlayerFilter::You);

    let Value::Count(filter) = draw.count.unhinted() else {
        panic!("the draw amount must count the revealed hand union: {draw:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(
        filter.owner,
        Some(PlayerFilter::AliasedTarget(Box::new(
            PlayerFilter::Opponent,
        )))
    );
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [expected_line],
        "{definition:#?}"
    );
}

#[test]
fn withering_gaze_keeps_the_shared_forest_or_green_hand_count() {
    assert_revealed_hand_union_count(
        "Withering Gaze",
        "Target opponent reveals their hand. You draw a card for each Forest and green card in it.",
    );
}

#[test]
fn baleful_stare_keeps_the_shared_mountain_or_red_hand_count() {
    assert_revealed_hand_union_count(
        "Baleful Stare",
        "Target opponent reveals their hand. You draw a card for each Mountain and red card in it.",
    );
}

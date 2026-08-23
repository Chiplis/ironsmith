#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn tempting_offer_draw_and_token_round_trips_to_oracle() {
    let definition = parse_oracle_card_definition("Tempt with Bunnies");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Tempting Offer — Draw a card and create a 1/1 white Rabbit creature token. Then each opponent may draw a card and create a 1/1 white Rabbit creature token. For each opponent who does, you draw a card and you create a 1/1 white Rabbit creature token."
        ]
    );

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Tempt with Bunnies has a spell program");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one resolution segment: {program:#?}");
    };
    let [_, opponents_effect] = segment.default_effects.as_slice() else {
        panic!("expected initial action and opponent loop: {segment:#?}");
    };
    let opponents = opponents_effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("second effect is the opponent loop");
    assert_eq!(opponents.filter, PlayerFilter::Opponent);
    let [offer_effect, reward_effect] = opponents.effects.as_slice() else {
        panic!("expected linked offer and reward: {opponents:#?}");
    };
    let offer = offer_effect
        .downcast_ref::<WithIdEffect>()
        .expect("offer records its result ID");
    let may = offer
        .effect
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("each opponent receives an optional action");
    assert_eq!(may.decider, Some(PlayerFilter::IteratedPlayer));
    let reward = reward_effect
        .downcast_ref::<IfEffect>()
        .expect("controller reward is conditional on the offer result");
    assert_eq!(reward.condition, offer.id);
    assert_eq!(reward.predicate, crate::effect::EffectPredicate::Chosen);
}

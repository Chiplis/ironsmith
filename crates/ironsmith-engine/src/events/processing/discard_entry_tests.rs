//! A discard whose destination a replacement rewrote to the battlefield is
//! still an entry: entry replacements apply (CR 614.1c, 614.12, 616.1).

use super::*;
use crate::ability::Ability;
use crate::card::{CardBuilder, PowerToughness};
use crate::events::cause::EventCause;
use crate::ids::CardId;
use crate::object::CounterType;
use crate::static_abilities::StaticAbility;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

fn smiter_in_hand(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Loxodon Smiter")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let id = game.create_object_from_card(&card, owner, Zone::Hand);
    game.object_mut(id)
        .expect("smiter exists")
        .abilities_mut()
        .push(
            Ability::static_ability(
                StaticAbility::opponent_effect_discard_this_to_battlefield_replacement(),
            )
            .in_zones(vec![Zone::Hand]),
        );
    id
}

fn opponent_permanent(game: &mut GameState, controller: PlayerId, ability: StaticAbility) {
    let card = CardBuilder::new(CardId::new(), "Entry Replacement Source")
        .card_types(vec![CardType::Enchantment])
        .build();
    let id = game.create_object_from_card(&card, controller, Zone::Battlefield);
    game.object_mut(id)
        .expect("source exists")
        .abilities_mut()
        .push(Ability::static_ability(ability));
}

fn opponent_discard(game: &mut GameState, card: ObjectId) -> DiscardResult {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Opponent Discard Spell")
            .card_types(vec![CardType::Sorcery])
            .build(),
        bob,
        Zone::Stack,
    );
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    execute_discard(
        game,
        card,
        alice,
        EventCause::from_effect(spell, bob),
        false,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("discard should execute")
    .expect("discard should finish without a pending choice")
}

#[test]
fn discard_redirected_to_battlefield_applies_enters_tapped() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let smiter = smiter_in_hand(&mut game, alice);
    // Blind Obedience: "Artifacts and creatures your opponents control enter tapped."
    opponent_permanent(
        &mut game,
        bob,
        StaticAbility::enters_tapped_for_filter(
            ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        ),
    );

    let result = opponent_discard(&mut game, smiter);

    assert_eq!(result.final_zone, Zone::Battlefield);
    let entered = result.new_id.expect("smiter should enter");
    assert_eq!(game.object(entered).map(|o| o.zone), Some(Zone::Battlefield));
    assert!(
        game.is_tapped(entered),
        "the discard-to-battlefield entry must see Blind Obedience's replacement"
    );
}

#[test]
fn discard_redirected_to_battlefield_applies_enters_with_counters() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let smiter = smiter_in_hand(&mut game, alice);
    opponent_permanent(
        &mut game,
        bob,
        StaticAbility::enters_with_counters_for_filter(
            ObjectFilter::creature(),
            CounterType::PlusOnePlusOne,
            2,
        ),
    );

    let result = opponent_discard(&mut game, smiter);

    let entered = result.new_id.expect("smiter should enter");
    assert_eq!(result.final_zone, Zone::Battlefield);
    assert_eq!(game.counter_count(entered, CounterType::PlusOnePlusOne), 2);
}

#[test]
fn discard_redirected_to_battlefield_obeys_containment_priest() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let smiter = smiter_in_hand(&mut game, alice);
    // Containment Priest: "If a nontoken creature would enter and it wasn't
    // cast, exile it instead."
    opponent_permanent(
        &mut game,
        bob,
        StaticAbility::redirect_would_enter(
            ObjectFilter::creature(),
            true,
            Zone::Exile,
            "If a nontoken creature would enter and it wasn't cast, exile it instead.",
        ),
    );

    let result = opponent_discard(&mut game, smiter);

    assert!(
        !game.battlefield.iter().any(|&id| game
            .object(id)
            .is_some_and(|o| o.name == "Loxodon Smiter")),
        "Containment Priest must stop the discarded card from entering"
    );
    assert!(
        game.exile.iter().any(|&id| game
            .object(id)
            .is_some_and(|o| o.name == "Loxodon Smiter")),
        "the card is exiled instead"
    );
    assert_eq!(result.final_zone, Zone::Exile);
}

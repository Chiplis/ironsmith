//! "Creatures your opponents control have base toughness 1." (Maha, Its
//! Feathers Night): CR 613.4b base toughness setting that leaves base power
//! alone. Source-authored, deliberately unrun.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};

#[path = "p02_line_families/compile.rs"]
mod compile;

const MAHA: &str = "Mana cost: {3}{B}{B}\nType: Legendary Creature — Elemental Bird\nPower/Toughness: 6/5\nFlying, trample\nWard—Discard a card.\nCreatures your opponents control have base toughness 1.";

#[test]
fn maha_sets_only_opposing_base_toughness() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in compile::compile_both("Maha, Its Feathers Night", MAHA) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let maha = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let card = CardBuilder::new(CardId::new(), "Opposing creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let opposing = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let own = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let opposing = game.calculated_characteristics(opposing).unwrap();
        assert_eq!((opposing.power, opposing.toughness), (Some(4), Some(1)));
        let own = game.calculated_characteristics(own).unwrap();
        assert_eq!((own.power, own.toughness), (Some(4), Some(4)));
        let maha = game.calculated_characteristics(maha).unwrap();
        assert_eq!((maha.power, maha.toughness), (Some(6), Some(5)));
    }
}

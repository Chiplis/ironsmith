//! "Colorless creatures you control enter with two additional +1/+1 counters
//! on them." (Curator Beastie): the filtered entry-counter replacement
//! (CR 614.1c) was unreachable from color/supertype adjective heads.
//! Source-authored, deliberately unrun.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::color::{Color, ColorSet};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::object::CounterType;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};

#[path = "p02_line_families/compile.rs"]
mod compile;

const A: PlayerId = PlayerId::from_index(0);
const CURATOR_BEASTIE: &str = "Mana cost: {4}{G}{G}\nType: Creature — Beast\nPower/Toughness: 6/6\nReach\nColorless creatures you control enter with two additional +1/+1 counters on them.\nWhenever this creature enters or attacks, manifest dread. (Look at the top two cards of your library. Put one onto the battlefield face down as a 2/2 creature and the other into your graveyard. Turn it face up any time for its mana cost if it's a creature card.)";

fn enter_creature(game: &mut GameState, colors: Option<ColorSet>, types: Vec<CardType>) -> ObjectId {
    let mut builder = CardBuilder::new(CardId::new(), "Entering creature")
        .card_types(types)
        .power_toughness(PowerToughness::fixed(1, 1));
    if let Some(colors) = colors {
        builder = builder.color_indicator(colors);
    }
    let card = game.create_object_from_card(&builder.build(), A, Zone::Hand);
    let receipt = game
        .move_object_with_etb_processing_with_dm(card, Zone::Battlefield, &mut SelectFirstDecisionMaker)
        .unwrap();
    receipt.original.into_result().unwrap().new_id
}

#[test]
fn colorless_creatures_enter_with_two_additional_counters() {
    for definition in compile::compile_both("Curator Beastie", CURATOR_BEASTIE) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let colorless = enter_creature(&mut game, None, vec![CardType::Artifact, CardType::Creature]);
        assert_eq!(game.counter_count(colorless, CounterType::PlusOnePlusOne), 2);
        let green = enter_creature(
            &mut game,
            Some(ColorSet::from_color(Color::Green)),
            vec![CardType::Creature],
        );
        assert_eq!(game.counter_count(green, CounterType::PlusOnePlusOne), 0);
    }
}

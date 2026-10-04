//! Reproducible wide-board mana payment benchmark. Run with --release.
use ironsmith::costs::{Cost, PaymentReason};
use ironsmith::ids::CardId;
use ironsmith::mana::ManaCost;
use ironsmith::mana_payment::{
    ManaPaymentRequest, mana_payment_activation_inventory, plan_first_mana_payment,
};
use ironsmith::types::CardType;
use ironsmith::{Ability, CardBuilder, GameState, ManaSymbol, PlayerId, TotalCost, Zone};
use std::time::Instant;
fn main() {
    let sources: usize = std::env::args()
        .nth(1)
        .unwrap_or("400".into())
        .parse()
        .unwrap();
    let repeats: usize = std::env::args()
        .nth(2)
        .unwrap_or("3".into())
        .parse()
        .unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let payer = PlayerId::from_index(0);
    let source_card = CardBuilder::new(CardId::new(), "Mana source")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..sources {
        let id = game.create_object_from_card(&source_card, payer, Zone::Battlefield);
        game.object_mut(id)
            .unwrap()
            .abilities_mut()
            .push(Ability::mana(
                TotalCost::from_cost(Cost::tap()),
                vec![ManaSymbol::Green],
            ));
    }
    let filler = CardBuilder::new(CardId::new(), "Board object")
        .card_types(vec![CardType::Creature])
        .build();
    for _ in 0..sources {
        game.create_object_from_card(&filler, PlayerId::from_index(1), Zone::Battlefield);
    }
    game.refresh_continuous_state().unwrap();
    let request = ManaPaymentRequest::new(
        payer,
        game.new_object_id(),
        PaymentReason::Effect,
        ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)], vec![ManaSymbol::Green]]),
    );
    for iteration in 0..repeats {
        let started = Instant::now();
        let options = std::hint::black_box(mana_payment_activation_inventory(&game, &request));
        println!(
            "{{\"case\":\"wide_board_inventory\",\"sources\":{sources},\"battlefield\":{},\"iteration\":{iteration},\"ms\":{},\"options\":{}}}",
            game.battlefield.len(),
            started.elapsed().as_secs_f64() * 1000.,
            options.len()
        );
        assert_eq!(options.len(), sources);
        assert!(
            options
                .iter()
                .all(|o| o.expected_mana.green == 1 && !o.repeatable)
        );
        let started = Instant::now();
        let plan = std::hint::black_box(plan_first_mana_payment(&game, &request).unwrap());
        println!(
            "{{\"case\":\"wide_board_first_plan\",\"sources\":{sources},\"battlefield\":{},\"iteration\":{iteration},\"ms\":{},\"activations\":{}}}",
            game.battlefield.len(),
            started.elapsed().as_secs_f64() * 1000.,
            plan.mana_ability_steps.len()
        );
        assert!(plan.payable);
        assert_eq!(plan.mana_ability_steps.len(), 6);
        assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
    }
}

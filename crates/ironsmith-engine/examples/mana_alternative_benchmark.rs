//! Alternative-payment stress cases with authoritative execution assertions.
//! Run in release mode; the optional argument is the repetition count.
use ironsmith::costs::{Cost, PaymentReason};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::ids::CardId;
use ironsmith::mana::ManaCost;
use ironsmith::mana_payment::{
    ManaPaymentExecution, ManaPaymentRequest, PlannedPipPayment, execute_mana_payment_plan,
    last_mana_payment_perf, plan_first_mana_payment,
};
use ironsmith::static_abilities::StaticAbility;
use ironsmith::types::CardType;
use ironsmith::{Ability, CardBuilder, GameState, ManaSymbol, PlayerId, TotalCost, Zone};
use std::time::Instant;

fn main() {
    let repeats = std::env::args()
        .nth(1)
        .unwrap_or("5".into())
        .parse::<usize>()
        .unwrap();
    for case in ["convoke", "improvise", "delve"] {
        for iteration in 0..repeats {
            let mut game = GameState::new(vec!["Alice".into()], 20);
            let payer = PlayerId::from_index(0);
            let spell_card = CardBuilder::new(CardId::new(), "Alternative payment probe")
                .card_types(vec![CardType::Sorcery])
                .build();
            let spell = game.create_object_from_card(&spell_card, payer, Zone::Stack);
            let keyword = match case {
                "convoke" => StaticAbility::convoke(),
                "improvise" => StaticAbility::improvise(),
                _ => StaticAbility::delve(),
            };
            game.object_mut(spell)
                .unwrap()
                .abilities_mut()
                .push(Ability::static_ability(keyword));
            let resource = CardBuilder::new(CardId::new(), "Alternative resource")
                .card_types(vec![CardType::Artifact, CardType::Creature])
                .power_toughness(ironsmith::card::PowerToughness::fixed(1, 1))
                .build();
            let zone = if case == "delve" {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            };
            let resources = (0..30)
                .map(|_| game.create_object_from_card(&resource, payer, zone))
                .collect::<Vec<_>>();
            let land_card = CardBuilder::new(CardId::new(), "Blue mana source")
                .card_types(vec![CardType::Land])
                .build();
            for _ in 0..2 {
                let land = game.create_object_from_card(&land_card, payer, Zone::Battlefield);
                game.object_mut(land)
                    .unwrap()
                    .abilities_mut()
                    .push(Ability::mana(
                        TotalCost::from_cost(Cost::tap()),
                        vec![ManaSymbol::Blue],
                    ));
            }
            game.refresh_continuous_state().unwrap();
            let request = ManaPaymentRequest::new(
                payer,
                spell,
                PaymentReason::CastSpell,
                ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(12)],
                    vec![ManaSymbol::Blue],
                    vec![ManaSymbol::Blue],
                ]),
            );
            let started = Instant::now();
            let plan = plan_first_mana_payment(&game, &request)
                .expect("keyword resources plus two blue sources must pay");
            let plan_ms = started.elapsed().as_secs_f64() * 1000.;
            let metrics = last_mana_payment_perf();
            let alternative_count = plan
                .allocations
                .iter()
                .filter(|allocation| {
                    matches!(
                        allocation.payment,
                        PlannedPipPayment::Convoke(_)
                            | PlannedPipPayment::Improvise(_)
                            | PlannedPipPayment::Delve(_)
                    )
                })
                .count();
            assert_eq!(alternative_count, 12);
            assert_eq!(plan.mana_ability_steps.len(), 2);
            assert!(
                resources
                    .iter()
                    .all(|id| game.object(*id).unwrap().zone == zone && !game.is_tapped(*id))
            );
            let started = Instant::now();
            assert_eq!(
                execute_mana_payment_plan(
                    &mut game,
                    &request,
                    &plan,
                    &mut SelectFirstDecisionMaker
                ),
                Ok(ManaPaymentExecution::Paid)
            );
            let commit_ms = started.elapsed().as_secs_f64() * 1000.;
            let consumed = resources
                .iter()
                .filter(|id| {
                    if case == "delve" {
                        game.object(**id)
                            .is_none_or(|object| object.zone != Zone::Graveyard)
                    } else {
                        game.is_tapped(**id)
                    }
                })
                .count();
            assert_eq!(
                consumed, 12,
                "commit must consume exactly the selected resources"
            );
            assert_eq!(game.player(payer).unwrap().mana_pool.total(), 0);
            println!(
                "{{\"case\":\"{case}\",\"iteration\":{iteration},\"resources\":30,\"alternative_pips\":{alternative_count},\"activations\":2,\"plan_ms\":{plan_ms},\"commit_ms\":{commit_ms},\"analytic_selections\":{},\"searched_selections\":{},\"visited_nodes\":{}}}",
                metrics.analytic_selections, metrics.searched_selections, metrics.visited_nodes
            );
        }
    }
}

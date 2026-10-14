//! Counter-amount modifiers on permanents entering with counters
//! (CR 122.6, 616.1e-f; Vorinclex, Monstrous Raider rulings).

use crate::card::{CardBuilder, PowerToughness};
use crate::game_state::GameState;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::object::CounterType;
use crate::static_abilities::StaticAbility;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

/// Picks the replacement whose source comes first in `order`, and records
/// whether any choice offered more than one legal replacement.
struct PreferSources {
    order: Vec<ObjectId>,
    contested: bool,
}

impl crate::decision::DecisionMaker for PreferSources {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        let legal = ctx.options.iter().filter(|option| option.legal).count();
        self.contested |= legal > 1;
        let choice = self
            .order
            .iter()
            .find_map(|source| {
                ctx.options
                    .iter()
                    .find(|option| option.legal && option.object_id == Some(*source))
            })
            .or_else(|| ctx.options.iter().find(|option| option.legal))
            .expect("a legal option");
        vec![choice.index]
    }
}

fn permanent_with(
    game: &mut GameState,
    name: &str,
    controller: PlayerId,
    abilities: Vec<StaticAbility>,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Enchantment])
        .build();
    let id = game.create_object_from_card(&card, controller, Zone::Battlefield);
    for ability in abilities {
        game.object_mut(id)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(ability));
    }
    id
}

/// Vorinclex, Monstrous Raider: you double, opponents halve (rounded down).
fn vorinclex(game: &mut GameState, controller: PlayerId) -> ObjectId {
    permanent_with(
        game,
        "Vorinclex",
        controller,
        vec![
            StaticAbility::actor_counter_multiplier_replacement(
                PlayerFilter::You,
                false,
                "If you would put one or more counters on a permanent or player, put twice that many instead.".into(),
            ),
            StaticAbility::actor_counter_multiplier_replacement(
                PlayerFilter::Opponent,
                true,
                "If an opponent would put one or more counters on a permanent or player, they put half that many instead, rounded down.".into(),
            ),
        ],
    )
}

/// A 0/0 that enters with one +1/+1 counter (modular 1).
fn modular_one(game: &mut GameState, owner: PlayerId, zone: Zone) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Modular Entrant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .build();
    let id = game.create_object_from_card(&card, owner, zone);
    game.object_mut(id)
        .unwrap()
        .abilities_mut()
        .push(crate::ability::Ability::static_ability(
            StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, 1),
        ));
    game.take_pending_trigger_events();
    id
}

fn enter(
    game: &mut GameState,
    entrant: ObjectId,
    controller: Option<PlayerId>,
    dm: &mut PreferSources,
) -> ObjectId {
    game.move_object_with_etb_processing_with_cause_and_entry_options_and_controller(
        entrant,
        Zone::Battlefield,
        crate::events::cause::EventCause::effect(),
        dm,
        controller,
        false,
        true,
        None,
    )
    .expect("entry must process")
    .assert_completed_without_additions()
    .expect("the permanent should enter")
    .new_id
}

// CR 616.1e: the opponent's halving and my Hardened Scales are both "other"
// replacements on the counters the entrant enters with; I choose the order.
#[test]
fn entering_counters_order_halving_against_hardened_scales() {
    for (scales_first, expected) in [(true, 1), (false, 0)] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let raider = vorinclex(&mut game, bob);
        let scales = permanent_with(
            &mut game,
            "Hardened Scales",
            alice,
            vec![StaticAbility::add_counters_placement_replacement(
                ObjectFilter::creature().you_control(),
                Some(CounterType::PlusOnePlusOne),
                1,
                "If one or more +1/+1 counters would be put on a creature you control, that many plus one +1/+1 counters are put on it instead.".into(),
            )],
        );
        let worker = modular_one(&mut game, alice, Zone::Hand);
        let order = if scales_first { vec![scales, raider] } else { vec![raider, scales] };
        let mut dm = PreferSources { order, contested: false };
        let entered = enter(&mut game, worker, None, &mut dm);
        assert!(dm.contested, "the halving and Scales must be offered together");
        assert_eq!(
            game.counter_count(entered, CounterType::PlusOnePlusOne),
            expected,
            "scales first: {scales_first}"
        );
    }
}

#[test]
fn entering_counters_order_halving_against_doubling_season() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let raider = vorinclex(&mut game, bob);
    let season = permanent_with(
        &mut game,
        "Doubling Season",
        alice,
        vec![StaticAbility::double_effect_counters_replacement(
            ObjectFilter::permanent().you_control(),
            None,
            "If an effect would put one or more counters on a permanent you control, it puts twice that many of those counters on that permanent instead.".into(),
        )],
    );
    let ballista = modular_one(&mut game, alice, Zone::Hand);
    let mut dm = PreferSources { order: vec![season, raider], contested: false };
    let entered = enter(&mut game, ballista, None, &mut dm);
    assert!(dm.contested);
    assert_eq!(game.counter_count(entered, CounterType::PlusOnePlusOne), 1);
}

// CR 122.6 / Vorinclex ruling: the player who controls the permanent as it
// enters puts its entry counters, not its owner.
#[test]
fn reanimated_opponent_card_uses_entering_controller_for_vorinclex() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    vorinclex(&mut game, alice);
    let ravager = modular_one(&mut game, bob, Zone::Graveyard);
    let mut dm = PreferSources { order: Vec::new(), contested: false };
    let entered = enter(&mut game, ravager, Some(alice), &mut dm);
    assert_eq!(game.current_controller(entered), Some(alice));
    assert_eq!(game.counter_count(entered, CounterType::PlusOnePlusOne), 2);
}

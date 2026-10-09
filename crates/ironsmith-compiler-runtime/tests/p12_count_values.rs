//! Count values: players in the game, colors of the antecedent, permanents
//! returned this way, one mill per damage point (p12-other). Unrun.
#[path = "p12_other/support.rs"]
mod support;

use ironsmith::effect::Value;
use ironsmith::effects::GainLifeEffect;

fn gain(definition: &ironsmith::cards::CardDefinition) -> String {
    let effects = support::effects(definition);
    let gain = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<GainLifeEffect>())
        .expect("life gain");
    format!("{:?}", gain.amount)
}

#[test]
fn benediction_counts_players_and_breathe_counts_colors() {
    for definition in support::definitions("Benediction of Moons") {
        assert!(gain(&definition).contains("CountPlayers(Any)"));
    }
    for definition in support::definitions("Breathe Your Last") {
        let amount = gain(&definition);
        assert!(amount.contains("ColorsOf"), "{amount}");
    }
}

#[test]
fn wanderwine_counts_permanents_returned_this_way() {
    for definition in support::definitions("Wanderwine Farewell") {
        let create = support::effects(&definition)
            .into_iter()
            .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>().cloned())
            .expect("merfolk creation");
        let count = format!("{:?}", create.count);
        assert!(count.contains("PendingPriorEffectMetric") && count.contains("Returned"), "{count}");
    }
}

#[test]
fn anowon_mills_one_card_per_damage_point() {
    for definition in support::definitions("Anowon, the Ruin Thief") {
        let debug = support::debug(&definition);
        assert!(debug.contains("Mill"), "{debug}");
        assert!(debug.contains(&format!("{:?}", Value::EventValue(ironsmith::effect::EventValueSpec::Amount))));
    }
}

//! Descent into Avernus card definition.

use crate::ability::Ability;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cards::tokens::treasure_token_definition;
use crate::effect::{Effect, Value};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::Trigger;
use crate::types::CardType;

/// Descent into Avernus - {2}{R}
/// Enchantment
/// At the beginning of your upkeep, put two descent counters on this
/// enchantment. Then each player creates X Treasure tokens and this enchantment
/// deals X damage to each player, where X is the number of descent counters on
/// this enchantment.
pub fn descent_into_avernus() -> CardDefinition {
    let descent = CounterType::Named("descent");
    let x = Value::CountersOnSource(descent);
    let treasure = treasure_token_definition();

    CardDefinitionBuilder::new(CardId::new(), "Descent into Avernus")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Enchantment])
        .with_ability(
            Ability::triggered(
                Trigger::beginning_of_upkeep(PlayerFilter::You),
                vec![
                    Effect::put_counters_on_source(descent, 2),
                    Effect::for_players(
                        PlayerFilter::Any,
                        vec![Effect::create_tokens_player(
                            treasure,
                            x.clone(),
                            PlayerFilter::IteratedPlayer,
                        )],
                    ),
                    Effect::for_players(
                        PlayerFilter::Any,
                        vec![Effect::deal_damage(
                            x,
                            ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                        )],
                    ),
                ],
            )
            .with_text(
                "At the beginning of your upkeep, put two descent counters on this enchantment. Then each player creates X Treasure tokens and this enchantment deals X damage to each player, where X is the number of descent counters on this enchantment.",
            ),
        )
        .build()
}

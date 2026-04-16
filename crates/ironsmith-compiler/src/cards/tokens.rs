use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

fn token(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .token()
        .build()
}

pub fn treasure_token_definition() -> CardDefinition {
    let mana_ability = crate::ability::Ability {
        kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![Cost::tap(), Cost::sacrifice_self()]),
            effects: vec![Effect::add_mana_of_any_color(1)].into(),
            choices: vec![],
            timing: crate::ability::ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: Some(vec![]),
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![crate::zone::Zone::Battlefield],
        text: Some("{T}, Sacrifice this artifact: Add one mana of any color.".to_string()),
    };

    CardDefinitionBuilder::new(CardId::new(), "Treasure")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Treasure])
        .with_ability(mana_ability)
        .build()
}
pub fn clue_token_definition() -> CardDefinition {
    let draw_ability = crate::ability::Ability::activated_with_timing(
        TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
            Cost::sacrifice_self(),
        ]),
        vec![Effect::draw(1)],
        crate::ability::ActivationTiming::AnyTime,
    )
    .with_text("{2}, Sacrifice this artifact: Draw a card.");

    CardDefinitionBuilder::new(CardId::new(), "Clue")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Clue])
        .with_ability(draw_ability)
        .build()
}
pub fn map_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Map")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Map])
        .build()
}
pub fn lander_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Lander")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Lander])
        .build()
}
pub fn junk_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Junk")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Junk])
        .build()
}
pub fn gold_token_definition() -> CardDefinition {
    let mana_ability = crate::ability::Ability {
        kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![Cost::sacrifice_self()]),
            effects: vec![Effect::add_mana_of_any_color(1)].into(),
            choices: vec![],
            timing: crate::ability::ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: Some(vec![]),
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![crate::zone::Zone::Battlefield],
        text: Some("Sacrifice this token: Add one mana of any color.".to_string()),
    };

    CardDefinitionBuilder::new(CardId::new(), "Gold")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Gold])
        .with_ability(mana_ability)
        .build()
}
pub fn shard_token_definition() -> CardDefinition {
    let scry_and_draw_ability = crate::ability::Ability::activated_with_timing(
        TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
            Cost::sacrifice_self(),
        ]),
        vec![Effect::scry(1), Effect::draw(1)],
        crate::ability::ActivationTiming::AnyTime,
    )
    .with_text("{2}, Sacrifice this token: Scry 1, then draw a card.");

    CardDefinitionBuilder::new(CardId::new(), "Shard")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Shard])
        .with_ability(scry_and_draw_ability)
        .build()
}
pub fn walker_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Walker")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build()
}
pub fn wicked_role_token_definition() -> CardDefinition {
    token("Wicked Role")
}
pub fn young_hero_role_token_definition() -> CardDefinition {
    token("Young Hero Role")
}
pub fn monster_role_token_definition() -> CardDefinition {
    token("Monster Role")
}
pub fn sorcerer_role_token_definition() -> CardDefinition {
    token("Sorcerer Role")
}
pub fn royal_role_token_definition() -> CardDefinition {
    token("Royal Role")
}
pub fn cursed_role_token_definition() -> CardDefinition {
    token("Cursed Role")
}

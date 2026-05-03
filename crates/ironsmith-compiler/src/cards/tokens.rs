use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::{Anthem, GrantAbility, StaticAbility};
use crate::tag::TagKey;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

fn role_token(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .enchants(ObjectFilter::creature().into())
        .build()
}

fn enchanted_creature_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::creature();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("enchanted"),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    filter
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
            is_loyalty_ability: false,
        }),
        functional_zones: vec![crate::zone::Zone::Battlefield],
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
    );

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
    let exile_tag = crate::tag::TagKey::from("junk_exiled_card");
    let impulse_draw_ability = crate::ability::Ability {
        kind: crate::ability::AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![Cost::tap(), Cost::sacrifice_self()]),
            effects: vec![
                Effect::new(
                    crate::effects::ChooseObjectsEffect::new(
                        crate::filter::ObjectFilter::default()
                            .in_zone(Zone::Library)
                            .owned_by(PlayerFilter::You),
                        1,
                        PlayerFilter::You,
                        exile_tag.clone(),
                    )
                    .in_zone(Zone::Library)
                    .top_only(),
                ),
                Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(
                    exile_tag.clone(),
                ))),
                Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                    exile_tag,
                    PlayerFilter::You,
                    crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
                    true,
                    false,
                )),
            ]
            .into(),
            choices: vec![],
            timing: crate::ability::ActivationTiming::SorcerySpeed,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    CardDefinitionBuilder::new(CardId::new(), "Junk")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Junk])
        .with_ability(impulse_draw_ability)
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
            is_loyalty_ability: false,
        }),
        functional_zones: vec![crate::zone::Zone::Battlefield],
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
    );

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
    role_token("Wicked Role")
}
pub fn young_hero_role_token_definition() -> CardDefinition {
    role_token("Young Hero Role")
}
pub fn monster_role_token_definition() -> CardDefinition {
    let enchanted = enchanted_creature_filter();
    CardDefinitionBuilder::new(CardId::new(), "Monster Role")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .oracle_text("Enchant creature\nEnchanted creature gets +1/+1 and has trample.")
        .enchants(ObjectFilter::creature().into())
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            Anthem::new(enchanted.clone(), 1, 1),
        )))
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            GrantAbility::new(enchanted, StaticAbility::trample().into()),
        )))
        .build()
}
pub fn sorcerer_role_token_definition() -> CardDefinition {
    role_token("Sorcerer Role")
}
pub fn royal_role_token_definition() -> CardDefinition {
    role_token("Royal Role")
}
pub fn cursed_role_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Cursed Role")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .oracle_text("Enchant creature\nEnchanted creature has base power and toughness 1/1.")
        .enchants(ObjectFilter::creature().into())
        .with_ability(crate::ability::Ability::static_ability(
            StaticAbility::set_base_power_toughness(enchanted_creature_filter(), 1, 1),
        ))
        .build()
}

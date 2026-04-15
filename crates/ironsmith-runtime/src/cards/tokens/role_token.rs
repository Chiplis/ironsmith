//! Role token definitions.

use crate::ability::{Ability, AbilityKind, TriggeredAbility};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::effect::{Condition, Effect};
use crate::filter::{Comparison, ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::static_abilities::StaticAbility;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::triggers::Trigger;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

fn enchanted_creature_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::creature();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("enchanted"),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    filter
}

fn role_token_builder(name: &str, oracle_text: &str) -> CardDefinitionBuilder {
    CardDefinitionBuilder::new(CardId::new(), name)
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Role])
        .oracle_text(oracle_text)
        .enchants(ObjectFilter::creature())
}

pub fn wicked_role_token_definition() -> CardDefinition {
    let enchanted = enchanted_creature_filter();
    role_token_builder(
        "Wicked Role",
        "Enchant creature\nEnchanted creature gets +1/+1.\nWhen this token is put into a graveyard from the battlefield, each opponent loses 1 life.",
    )
    .with_ability(Ability::static_ability(StaticAbility::anthem(
        enchanted,
        1,
        1,
    )))
    .with_ability(
        Ability::triggered(
            Trigger::this_dies(),
            vec![Effect::for_each_opponent(vec![Effect::lose_life(1)])],
        )
        .with_text("When this token is put into a graveyard from the battlefield, each opponent loses 1 life."),
    )
    .build()
}

pub fn young_hero_role_token_definition() -> CardDefinition {
    let triggering_tag = TagKey::from("triggering");
    let granted_trigger = TriggeredAbility {
        trigger: Trigger::this_attacks(),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::tag_triggering_object(triggering_tag.clone()),
            Effect::put_counters(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::Tagged(triggering_tag.clone()),
            ),
        ]),
        choices: vec![],
        intervening_if: Some(Condition::TaggedObjectMatches(
            triggering_tag,
            ObjectFilter::creature().with_toughness(Comparison::LessThanOrEqual(3)),
        )),
    };

    role_token_builder(
        "Young Hero Role",
        "Enchant creature\nEnchanted creature has \"Whenever this creature attacks, if its toughness is 3 or less, put a +1/+1 counter on it.\"",
    )
    .with_ability(
        Ability::static_ability(StaticAbility::attached_ability_grant(
            Ability {
                kind: AbilityKind::Triggered(granted_trigger),
                functional_zones: vec![Zone::Battlefield],
                text: None,
            },
            "enchanted creature has whenever this creature attacks if its toughness is 3 or less put a +1/+1 counter on it".to_string(),
        ))
        .with_text(
            "Enchanted creature has \"Whenever this creature attacks, if its toughness is 3 or less, put a +1/+1 counter on it.\"",
        ),
    )
    .build()
}

pub fn monster_role_token_definition() -> CardDefinition {
    let enchanted = enchanted_creature_filter();
    role_token_builder(
        "Monster Role",
        "Enchant creature\nEnchanted creature gets +1/+1 and has trample.",
    )
    .with_ability(Ability::static_ability(StaticAbility::anthem(
        enchanted.clone(),
        1,
        1,
    )))
    .with_ability(Ability::static_ability(StaticAbility::grant_ability(
        enchanted,
        StaticAbility::trample(),
    )))
    .build()
}

pub fn sorcerer_role_token_definition() -> CardDefinition {
    let granted_trigger = Ability::triggered(Trigger::this_attacks(), vec![Effect::scry(1)]);

    role_token_builder(
        "Sorcerer Role",
        "Enchant creature\nEnchanted creature gets +1/+1 and has \"Whenever this creature attacks, scry 1.\"",
    )
    .with_ability(Ability::static_ability(StaticAbility::anthem(
        enchanted_creature_filter(),
        1,
        1,
    )))
    .with_ability(
        Ability::static_ability(StaticAbility::attached_ability_grant(
            granted_trigger,
            "enchanted creature has whenever this creature attacks scry 1".to_string(),
        ))
        .with_text("Enchanted creature has \"Whenever this creature attacks, scry 1.\""),
    )
    .build()
}

pub fn royal_role_token_definition() -> CardDefinition {
    let enchanted = enchanted_creature_filter();
    let ward_cost = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]);
    role_token_builder(
        "Royal Role",
        "Enchant creature\nEnchanted creature gets +1/+1 and has ward {1}.",
    )
    .with_ability(Ability::static_ability(StaticAbility::anthem(
        enchanted.clone(),
        1,
        1,
    )))
    .with_ability(Ability::static_ability(StaticAbility::grant_ability(
        enchanted,
        StaticAbility::ward(crate::cost::TotalCost::mana(ward_cost)),
    )))
    .build()
}

pub fn cursed_role_token_definition() -> CardDefinition {
    role_token_builder(
        "Cursed Role",
        "Enchant creature\nEnchanted creature has base power and toughness 1/1.",
    )
    .with_ability(Ability::static_ability(
        StaticAbility::set_base_power_toughness(enchanted_creature_filter(), 1, 1),
    ))
    .build()
}

#[cfg(test)]
mod tests {
    use super::{
        cursed_role_token_definition, wicked_role_token_definition,
        young_hero_role_token_definition,
    };
    use crate::ability::AbilityKind;
    use crate::effect::Condition;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn wicked_role_keeps_attached_buff_and_dies_trigger() {
        let token = wicked_role_token_definition();
        let static_ids = token
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.id()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(static_ids.contains(&StaticAbilityId::Anthem));
        assert!(
            token
                .abilities
                .iter()
                .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_)))
        );
    }

    #[test]
    fn young_hero_role_grants_toughness_checked_attack_trigger() {
        let token = young_hero_role_token_definition();
        let granted = token
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => static_ability.granted_inline_ability(),
                _ => None,
            })
            .expect("expected attached granted trigger");

        let AbilityKind::Triggered(triggered) = &granted.kind else {
            panic!("expected granted triggered ability");
        };
        assert!(matches!(
            triggered.intervening_if,
            Some(Condition::TaggedObjectMatches(_, _))
        ));
    }

    #[test]
    fn cursed_role_sets_base_power_toughness() {
        let token = cursed_role_token_definition();
        let static_ids = token
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.id()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(static_ids.contains(&StaticAbilityId::SetBasePowerToughnessForFilter));
    }
}

#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn unwrap_with_id_and_tag(effect: &crate::effect::Effect) -> &crate::effect::Effect {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return unwrap_with_id_and_tag(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_with_id_and_tag(&tagged.effect);
    }
    effect
}

#[test]
fn jace_copy_and_zero_ability_keep_dynamic_loyalty_semantics() {
    let definition = parse_oracle_card_definition("Jace, Mirror Mage");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Kicker {2}\nWhen Jace enters, if Jace was kicked, create a token that's a copy of Jace, except it isn't legendary and its starting loyalty is 1.\n+1: Scry 2.\n0: Draw a card and reveal it. Remove a number of loyalty counters equal to that card's mana value from Jace."
    );

    let copy = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .flat_map(|triggered| triggered.effects.flattened_default_effects())
        .find_map(|effect| {
            unwrap_with_id_and_tag(effect).downcast_ref::<crate::effects::CreateTokenCopyEffect>()
        })
        .expect("Jace's kicked entry trigger should create a typed token copy");
    assert_eq!(copy.starting_loyalty, Some(1));
    assert_eq!(copy.removed_supertypes, [Supertype::Legendary]);

    let zero = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_loyalty_ability => Some(activated),
            _ => None,
        })
        .find(|activated| {
            activated
                .effects
                .flattened_default_effects()
                .iter()
                .any(|effect| {
                    unwrap_with_id_and_tag(effect)
                        .downcast_ref::<crate::effects::RemoveCountersEffect>()
                        .is_some()
                })
        })
        .expect("Jace should retain the zero-loyalty ability");
    let remove = zero
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| {
            unwrap_with_id_and_tag(effect).downcast_ref::<crate::effects::RemoveCountersEffect>()
        })
        .expect("the zero ability should remove loyalty counters");
    assert_eq!(remove.counter_type, crate::object::CounterType::Loyalty);
    assert!(matches!(remove.target.base(), ChooseSpec::Source));
    assert!(matches!(
        remove.count.unhinted(),
        Value::ManaValueOf(spec)
            if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str().contains("revealed"))
    ));
}

#[test]
fn pollywog_candidate_mutate_filter_is_shared_by_cost_and_trigger_condition() {
    let definition = parse_oracle_card_definition("Pollywog Symbiote");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Each creature spell you cast costs {1} less to cast if it has mutate.\nWhenever you cast a creature spell, if it has mutate, draw a card, then discard a card."
    );

    let reduction = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.cost_reduction(),
            _ => None,
        })
        .expect("Pollywog should retain a typed spell-cost reduction");
    assert_eq!(reduction.filter.card_types, [CardType::Creature]);
    assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
    assert_eq!(reduction.filter.ability_markers, ["mutate"]);
    assert!(
        reduction
            .filter
            .has_trailing_candidate_ability_condition_surface()
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Pollywog should retain its cast trigger");
    let crate::ConditionExpr::TaggedObjectMatches(tag, trigger_filter) = triggered
        .intervening_if
        .as_ref()
        .expect("the mutate condition should remain intervening-if")
    else {
        panic!(
            "the mutate condition must inspect the triggering spell: {:#?}",
            triggered.intervening_if
        );
    };
    assert_eq!(tag.as_str(), "triggering");
    assert_eq!(trigger_filter.ability_markers, ["mutate"]);
    assert!(
        triggered
            .effects
            .flattened_default_effects()
            .iter()
            .any(|effect| {
                effect
                    .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                    .is_some_and(|tagged| tagged.tag.as_str() == "triggering")
            })
    );

    let mutate_definition = CardDefinitionBuilder::new(CardId::new(), "Mutating Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::keyword_marker("mutate"),
        ))
        .build();
    let ordinary_definition = CardDefinitionBuilder::new(CardId::new(), "Ordinary Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let mutate_spell = game.create_object_from_definition(&mutate_definition, alice, Zone::Stack);
    let ordinary_spell =
        game.create_object_from_definition(&ordinary_definition, alice, Zone::Stack);
    let ctx = crate::filter::FilterContext::new(alice);
    assert!(trigger_filter.matches(game.object(mutate_spell).unwrap(), &ctx, &game));
    assert!(!trigger_filter.matches(game.object(ordinary_spell).unwrap(), &ctx, &game));
}

#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn flowstone_sculpture_preserves_both_outer_actions_and_the_nested_ability_choice() {
    let oracle = "{2}, Discard a card: Put a +1/+1 counter on this creature or this creature gains flying, first strike, or trample.";
    let definition = parse_oracle_card_definition("Flowstone Sculpture");

    assert_eq!(canonical_compiled_lines(&definition).join("\n"), oracle);

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("UnlessActionEffect")
            && debug.contains("PutCountersEffect")
            && debug.contains("ChooseModeEffect")
            && debug.contains("Flying")
            && debug.contains("FirstStrike")
            && debug.contains("Trample"),
        "both outer branches and every nested ability option must remain typed: {debug}"
    );
}

#[derive(Default)]
struct AcceptAlternativeDecisionMaker;

impl crate::decision::DecisionMaker for AcceptAlternativeDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }
}

#[test]
fn flowstone_sculpture_executes_either_the_counter_or_one_permanent_keyword_grant() {
    let definition = parse_oracle_card_definition("Flowstone Sculpture");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Flowstone Sculpture should have an activated ability");
    let [choice_effect] = activated.effects.flattened_default_effects() else {
        panic!(
            "activation should contain one outer action choice: {:#?}",
            activated.effects
        );
    };

    let alice = PlayerId::from_index(0);
    let mut counter_game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let counter_source =
        counter_game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut decline = crate::decision::AutoPassDecisionMaker;
    let mut counter_ctx =
        crate::effects::ExecutionContext::new(counter_source, alice, &mut decline);
    crate::effects::execute_effect(&mut counter_game, choice_effect, &mut counter_ctx)
        .expect("declining the alternative should put a counter on the source");
    assert_eq!(
        counter_game.counter_count(counter_source, crate::object::CounterType::PlusOnePlusOne),
        1
    );
    assert!(!counter_game.object_has_static_ability_id(
        counter_source,
        crate::static_abilities::StaticAbilityId::Trample,
    ));

    let mut ability_game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let ability_source =
        ability_game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut accept = AcceptAlternativeDecisionMaker;
    let mut ability_ctx = crate::effects::ExecutionContext::new(ability_source, alice, &mut accept)
        .with_chosen_modes(Some(vec![2]));
    crate::effects::execute_effect(&mut ability_game, choice_effect, &mut ability_ctx)
        .expect("accepting the alternative should grant the chosen keyword");
    assert_eq!(
        ability_game.counter_count(ability_source, crate::object::CounterType::PlusOnePlusOne),
        0
    );
    assert!(ability_game.object_has_static_ability_id(
        ability_source,
        crate::static_abilities::StaticAbilityId::Trample,
    ));
    assert!(!ability_game.object_has_static_ability_id(
        ability_source,
        crate::static_abilities::StaticAbilityId::Flying,
    ));
}

#[test]
fn natures_blessing_reuses_its_creature_target_across_the_outer_choice() {
    let oracle = "{G}{W}, Discard a card: Put a +1/+1 counter on target creature or that creature gains banding, first strike, or trample.";
    let definition = parse_oracle_card_definition("Nature's Blessing");

    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Nature's Blessing should have an activated ability");
    let [choice_effect] = activated.effects.flattened_default_effects() else {
        panic!(
            "activation should contain one typed outer action choice: {:#?}",
            activated.effects
        );
    };
    let debug = format!("{choice_effect:#?}");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle,
        "unexpected typed outer action shape: {debug}"
    );
    assert!(
        debug.contains("UnlessActionEffect")
            && debug.contains("PutCountersEffect")
            && debug.contains("ChooseModeEffect")
            && !debug.contains("discarded_cost"),
        "the alternative must share the creature target rather than a discarded cost object: {debug}"
    );

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let target_definition =
        CardDefinitionBuilder::new(CardId::from_raw(99_402), "Blessed Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
    let target =
        game.create_object_from_definition(&target_definition, alice, Zone::Battlefield);

    let mut accept = AcceptAlternativeDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut accept)
        .with_chosen_modes(Some(vec![2]))
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    crate::effects::execute_effect(&mut game, choice_effect, &mut ctx)
        .expect("accepting the alternative should grant the selected keyword to the target");

    assert!(game.object_has_static_ability_id(
        target,
        crate::static_abilities::StaticAbilityId::Trample,
    ));
    assert!(!game.object_has_static_ability_id(
        source,
        crate::static_abilities::StaticAbilityId::Trample,
    ));
    assert_eq!(
        game.counter_count(target, crate::object::CounterType::PlusOnePlusOne),
        0,
        "choosing the keyword branch must skip the counter branch"
    );
}

//! Spirited invokes a current activated occurrence through the real activation
//! pipeline, with its complete cost waived and its restrictions retained.
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const SPIRITED: &str = "When this permanent enters, you may activate one of its activated abilities without paying that ability's cost.";
#[path="custom_final_next_wave_support.rs"] mod source_routes;
fn routes(text: &str) -> [CardDefinition; 2] {
    source_routes::definitions(&serde_json::json!({"name":"Spirited source","executable_text":text})).unwrap()
}
struct Choices {
    yes: bool,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{
        SelectFirstDecisionMaker.decide_targets(game,ctx)
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.yes
    }
}
fn drain(game: &mut GameState, dm: &mut impl DecisionMaker) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, dm).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn enter(
    game: &mut GameState,
    definition: &CardDefinition,
    dm: &mut impl DecisionMaker,
) -> ObjectId {
    let hand = game.create_object_from_definition(definition, A, Zone::Hand);
    let stable = game.object(hand).unwrap().stable_id;
    execute_effect(
        game,
        &Effect::new(ironsmith::effects::MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(hand),
            Zone::Battlefield,
            false,
        )),
        &mut EffectContext::new_default(hand, A).with_decision_maker(dm),
    )
    .unwrap();
    let source = game.find_object_by_stable_id(stable).unwrap();
    drain(game, dm);
    assert_eq!(game.stack.len(), 1);
    source
}
fn finish(game: &mut GameState, dm: &mut impl DecisionMaker) {
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
        drain(game, dm);
    }
}
#[test]
fn spirited_waives_mana_tap_sacrifice_and_uses_a_real_activation_stack_entry() {
    let text = format!(
        "Type: Creature — Warrior\nPower/Toughness: 4/4\n{SPIRITED}\n{{3}}, {{T}}, Sacrifice this creature: You gain 3 life.\nWhenever you activate an ability of this creature, you gain 1 life."
    );
    for definition in routes(&text) {
        for yes in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let mut dm = Choices { yes };
            let source = enter(&mut game, &definition, &mut dm);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(game.object(source).is_some());
            assert!(!game.is_tapped(source));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(
                game.player(A).unwrap().life,
                20,
                "activation must not resolve inside Spirited"
            );
            drain(&mut game, &mut dm);
            assert_eq!(
                game.stack.len(),
                if yes { 2 } else { 0 },
                "one real activation plus activation watcher"
            );
            finish(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().life, if yes { 24 } else { 20 });
            assert!(game.object(source).is_some());
        }
    }
}
#[test]
fn spirited_mana_ability_resolves_immediately_without_paying_tap_cost() {
    let text =
        format!("Type: Creature — Druid\nPower/Toughness: 1/1\n{SPIRITED}\n{{T}}: Add {{G}}.");
    for definition in routes(&text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let mut dm = Choices { yes: true };
        let source = enter(&mut game, &definition, &mut dm);
        finish(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
        assert!(!game.is_tapped(source));
        assert!(game.stack.is_empty());
    }
}
#[test]
fn spirited_preserves_target_legality_and_explicit_sorcery_timing_restrictions() {
    for sorcery in [false, true] {
        let text = format!(
            "Type: Creature\nPower/Toughness: 2/2\n{SPIRITED}\n{{8}}, {{T}}: Put two +1/+1 counters on another target creature.{}",
            if sorcery {
                " Activate only as a sorcery."
            } else {
                ""
            }
        );
        for definition in routes(&text) {
            for available in [false, true] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let target = if available {
                    let target = ironsmith_compiler_runtime::compile_to_runtime_definition(
                        "Target",
                        "Type: Creature\nPower/Toughness: 1/1",
                        false,
                    )
                    .unwrap();
                    Some(game.create_object_from_definition(&target, A, Zone::Battlefield))
                } else {
                    None
                };
                let mut dm = Choices { yes: true };
                let source = enter(&mut game, &definition, &mut dm);
                finish(&mut game, &mut dm);
                assert!(!game.is_tapped(source));
                if let Some(target) = target {
                    assert_eq!(
                        game.counter_count(target, CounterType::PlusOnePlusOne),
                        if sorcery { 0 } else { 2 }
                    );
                }
            }
        }
    }
}
#[test]
fn spirited_chooses_current_abilities_at_resolution() {
    let text = format!("Type: Creature\nPower/Toughness: 2/2\n{SPIRITED}\n{{T}}: You gain 3 life.");
    for definition in routes(&text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let mut dm = Choices { yes: true };
        let source = enter(&mut game, &definition, &mut dm);
        let changed = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Current ability",
            "Type: Creature\nPower/Toughness: 2/2\n{8}, {T}: You gain 5 life.",
            false,
        )
        .unwrap();
        *game.object_mut(source).unwrap().abilities_mut() = changed.abilities.to_vec();
        game.refresh_continuous_state().unwrap();
        finish(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 25);
        assert!(!game.is_tapped(source));
    }
}
#[test]
fn spirited_instances_are_independent_and_departed_source_does_not_use_old_abilities() {
    for departed in [false, true] {
        let text = format!(
            "Type: Creature\nPower/Toughness: 2/2\n{SPIRITED}\n{SPIRITED}\n{{T}}: You gain 3 life."
        );
        for definition in routes(&text) {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let mut dm = Choices { yes: true };
            let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = game.object(hand).unwrap().stable_id;
            execute_effect(
                &mut game,
                &Effect::new(ironsmith::effects::MoveToZoneEffect::new(
                    ChooseSpec::SpecificObject(hand),
                    Zone::Battlefield,
                    false,
                )),
                &mut EffectContext::new_default(hand, A).with_decision_maker(&mut dm),
            )
            .unwrap();
            let source = game.find_object_by_stable_id(stable).unwrap();
            drain(&mut game, &mut dm);
            assert_eq!(game.stack.len(), 2);
            if departed {
                execute_effect(
                    &mut game,
                    &Effect::new(ironsmith::effects::MoveToZoneEffect::new(
                        ChooseSpec::SpecificObject(source),
                        Zone::Hand,
                        false,
                    )),
                    &mut EffectContext::new_default(source, A).with_decision_maker(&mut dm),
                )
                .unwrap();
            }
            finish(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().life, if departed { 20 } else { 26 });
        }
    }
}
#[test]
fn complete_spirited_source_variants_compile_without_loss() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-middle-keywords.json"
    ))
    .unwrap();
    let rows = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["keyword"] == "Spirited")
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 4);
    for row in rows {
        for card in row["representative_cards"].as_array().unwrap() {
            source_routes::definitions(card).unwrap_or_else(|error|panic!("{}: {error}",card["name"]));
        }
    }
}

#[test]
fn cancelling_native_target_announcement_does_not_publish_an_activation_receipt(){
 struct Cancel;
 impl DecisionMaker for Cancel{
  fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{true}
  fn decide_targets(&mut self,_:&GameState,_:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{vec![]}
 }
 let text=format!("Type: Creature\nPower/Toughness: 2/2\n{SPIRITED}\n{{8}}: Put two +1/+1 counters on another target creature.\nWhenever you activate an ability of this creature, you gain 1 life.");
 for definition in routes(&text){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Target","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
  let target=game.create_object_from_definition(&body,A,Zone::Battlefield);
  let mut dm=Cancel;let source=enter(&mut game,&definition,&mut dm);
  finish(&mut game,&mut dm);
  assert!(game.stack.is_empty());assert!(game.object(source).is_some());
  assert_eq!(game.counter_count(target,CounterType::PlusOnePlusOne),0);
  assert_eq!(game.player(A).unwrap().life,20,"cancelled target selection is not a completed activation");
 }
}

#[test]
fn actual_erduk_chosen_card_type_is_added_by_its_free_real_activation() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-middle-keywords.json")).unwrap();
    let card = fixture["mechanics"].as_array().unwrap().iter()
        .flat_map(|row| row["representative_cards"].as_array().unwrap())
        .find(|card| card["source_card_key"] == "Erduk, Settled Nomad_OTH").unwrap();
    for definition in source_routes::definitions(card).unwrap() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let mut dm = Choices { yes: true };
        let source = enter(&mut game, &definition, &mut dm);
        finish(&mut game, &mut dm);
        assert_eq!(game.chosen_card_type(source), Some(ironsmith::types::CardType::Artifact));
        let types = game.current_card_types(source).unwrap();
        assert!(types.contains(&ironsmith::types::CardType::Creature));
        assert!(types.contains(&ironsmith::types::CardType::Artifact));
    }
}

#[test]
fn chosen_type_does_not_reuse_an_obsolete_domain_across_an_optional_choice() {
    let prefix = "Type: Instant\nChoose artifact or enchantment. You may choose a creature type. ";
    let ambiguous = format!("{prefix}Target nonland permanent becomes the chosen type in addition to its other types until end of turn.");
    assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Ambiguous choice", &ambiguous, false).is_err());
    let explicit = format!("{prefix}Target creature becomes the chosen creature type in addition to its other types until end of turn.");
    source_routes::definitions(&serde_json::json!({"name":"Explicit domain","executable_text":explicit})).unwrap();
}

#[test]
fn explicit_chosen_card_type_text_retains_its_typed_domain_on_both_routes() {
    let text = "Type: Instant\nChoose artifact or enchantment. Target nonland permanent becomes the chosen card type in addition to its other types until end of turn.";
    for definition in source_routes::definitions(&serde_json::json!({"name":"Explicit card domain","executable_text":text})).unwrap() {
        fn has_card_domain(effect: &Effect) -> bool {
            if let Some(change) = effect.downcast_ref::<ironsmith::effects::BecomeCreatureTypeChoiceEffect>() {
                return change.use_source_chosen_type && change.use_source_chosen_card_type;
            }
            let mut found = false;
            effect.visit_child_effects(&mut |child| found |= has_card_domain(child));
            found
        }
        assert!(definition.spell_effect.as_ref().unwrap().all_effects_owned().iter().any(has_card_domain));
    }
}

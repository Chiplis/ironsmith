//! Exact-source Resilience and reusable ability-group designation metadata.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
fn definitions() -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-resilience-next-wave.json"
    ))
    .unwrap();
    let card = &fixture["mechanics"][0]["representative_cards"][0];
    assert_eq!(card["id"], "Nitz, Globetrotting Knight_MSL");
    source_support::definitions(card).unwrap()
}
fn settle(game: &mut GameState) {
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    for _ in 0..16 {
        let mut q = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut q, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut q, &mut dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
        }
    }
    panic!("unexpected recursive trigger");
}
fn destroy(game: &mut GameState, id: ObjectId) {
    execute_effect(
        game,
        &Effect::new(ironsmith::effects::DestroyEffect::with_spec(
            ChooseSpec::SpecificObject(id),
        )),
        &mut EffectContext::new_default(id, A),
    )
    .unwrap();
    settle(game);
}
#[test]
fn exact_resilience_exiles_original_and_copies_other_abilities_without_repeating_on_token_death() {
    for definition in definitions() {
        assert_eq!(
            definition
                .abilities
                .iter()
                .filter(|ability| ability.has_designation("resilience"))
                .count(),
            2,
            "identity and its complete death-trigger group carry ownership metadata"
        );
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        destroy(&mut game, source);
        let original = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(original).unwrap().zone, Zone::Exile);
        assert_eq!(game.battlefield.len(), 1);
        let token = game.battlefield[0];
        assert_eq!(
            game.object(token).unwrap().kind,
            ironsmith::object::ObjectKind::Token
        );
        assert_eq!(game.calculated_power(token), Some(1));
        assert_eq!(game.calculated_toughness(token), Some(1));
        assert!(game.object_has_static_ability_id(
            token,
            ironsmith::static_abilities::StaticAbilityId::Vigilance
        ));
        let abilities = &game.object(token).unwrap().abilities;
        assert_eq!(
            abilities.len(),
            2,
            "retain vigilance and the source's shared blocking restriction"
        );
        assert!(
            abilities
                .iter()
                .all(|ability| !ability.has_designation("resilience"))
        );
        assert!(!abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(static_ability) if matches!(static_ability.compiled_model().map(|model|&model.payload),Some(ironsmith_core::StaticAbilityPayload::CustomKeywordIdentity{name})if name.eq_ignore_ascii_case("resilience")))));
        destroy(&mut game, token);
        assert!(
            game.battlefield.is_empty(),
            "the copied token cannot start another resilience loop"
        );
        assert_eq!(game.object(original).unwrap().zone, Zone::Exile);
    }
}
#[test]
fn source_labels_designate_only_their_own_complete_ability_groups() {
    for definition in source_support::definitions(&serde_json::json!({"name":"Designation witness","executable_text":"Type: Creature — Wizard\nPower/Toughness: 2/2\nRecursive — {1}: You gain 1 life.\n{2}: Draw a card.\nKeyword ability \"echoing\" — Whenever this creature attacks, you gain 1 life."})).unwrap(){assert_eq!(definition.abilities.len(),3);assert!(definition.abilities[0].has_designation("recursive"));assert!(definition.abilities[1].designations.is_empty());assert!(definition.abilities[2].has_designation("echoing"));}
}
#[test]
fn spell_ability_designations_survive_artifact_mapping_without_spreading_to_other_source_lines() {
    for definition in source_support::definitions(&serde_json::json!({"name":"Spell designation witness","executable_text":"Type: Instant\nRecursive — Draw a card.\nYou gain 1 life."})).unwrap(){let program=definition.spell_effect.unwrap();assert_eq!(program.segments.len(),2);assert_eq!(program.segments[0].ability_designations,vec!["recursive".to_string()]);assert!(program.segments[1].ability_designations.is_empty());}
}

#[test]
fn generic_static_grant_retains_the_complete_ability_wrapper() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut granted = ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::flying(),
        );
        granted.designations = vec!["borrowed group".into()];
        granted.functional_zones = vec![Zone::Battlefield, Zone::Graveyard];
        let effect = Effect::new(ironsmith::effects::ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(source),
            ironsmith::continuous::Modification::AddAbilityGeneric(granted.clone()),
            ironsmith::effect::Until::EndOfTurn,
        ));
        execute_effect(
            &mut game,
            &effect,
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        let chars = game.calculated_characteristics(source).unwrap();
        let retained = chars
            .abilities
            .iter()
            .find(|ability| ability.has_designation("borrowed group"))
            .expect("static grants preserve wrapper designation metadata");
        assert_eq!(
            retained, &granted,
            "static grant must retain full wrapper, including functional zones"
        );
        assert!(chars.static_abilities.iter().any(|ability|ability.id()==ironsmith::static_abilities::StaticAbilityId::Flying));
    }
}

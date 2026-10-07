//! Full frozen body, independently compiled routes, native prevention. All UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::ChooseSpec;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Supertype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn definitions() -> [CardDefinition; 2] {
    let row: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/chomanno_full_body.json.fixture")).unwrap();
    assert_eq!(row["oracle_id"], "91af5e35-b3b8-43ce-b1ea-997ed74e4ad2");
    assert_eq!(row["name"], "Cho-Manno, Revolutionary");
    assert_eq!(row["oracle_text"], "Prevent all damage that would be dealt to Cho-Manno.");
    assert_eq!(row["mana_cost"], "{2}{W}{W}");
    assert_eq!(row["type_line"], "Legendary Creature — Human Rebel");
    assert_eq!(row["power"], "2");
    assert_eq!(row["toughness"], "2");
    let name = row["name"].as_str().unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
        row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "direct: {}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!loss.is_lossy(), "artifact: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, artifact);
    let definitions = [direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()];
    for definition in &definitions {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
        assert_eq!(definition.card.name, name);
        assert_eq!(definition.card.mana_cost, Some(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(2), ManaSymbol::White, ManaSymbol::White])));
        assert_eq!(definition.card.supertypes, vec![Supertype::Legendary]);
        assert_eq!(definition.card.card_types, vec![CardType::Creature]);
        assert_eq!(definition.card.subtypes, vec![Subtype::Human, Subtype::Rebel]);
        assert_eq!(definition.card.power_toughness, Some(PowerToughness::fixed(2, 2)));
        assert!(definition.spell_effect.is_none());
        assert_eq!(definition.abilities.len(), 1, "complete body is one static ability");
        let AbilityKind::Static(ability) = &definition.abilities[0].kind else {
            panic!("prevention is native static behavior, not an activation or placeholder");
        };
        assert_eq!(ability.id(), StaticAbilityId::PreventAllDamageToSelf);
    }
    definitions
}

fn source(game: &mut GameState, owner: PlayerId, zone: Zone) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), "Independent damage source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 8)).build(), owner, zone)
}

fn damage(game: &mut GameState, source: ObjectId, target: DamageTarget,
    combat: bool, unpreventable: bool) -> (u32, Vec<(u32, ObjectId, PlayerId)>) {
    game.take_pending_trigger_events();
    let result = process_damage_assignments_with_event_with_source_snapshot_opts(
        game, source, target, 3, combat, unpreventable, EventCause::effect(), None).unwrap();
    let amount = result.assignments.iter().map(|assignment| assignment.amount).sum();
    let events = game.take_pending_trigger_events().into_iter().filter_map(|event|
        event.downcast::<DamagePreventedEvent>().map(|event| {
            assert_eq!(event.damage_source, source);
            assert_eq!(event.target, target);
            assert_eq!(event.is_combat, combat);
            assert!(event.prevention_shield.is_none(), "printed static prevention is not a created shield");
            assert_eq!(event.applications.len(), 1);
            let application = &event.applications[0];
            assert_eq!(application.damage_source, source);
            assert_eq!(application.target, target);
            assert_eq!(application.amount, event.amount);
            assert_eq!(application.is_combat, combat);
            (event.amount, event.prevention_source, event.prevention_controller)
        })).collect();
    (amount, events)
}

#[test]
fn complete_body_prevents_all_repeatable_damage_to_exact_self_without_activation() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        // Same name without the ability must not receive another object's shield.
        let mut blank = definition.clone();
        blank.abilities.clear();
        let other = game.create_object_from_definition(&blank, B, Zone::Battlefield);
        for owner in [A, B] {
            for zone in [Zone::Battlefield, Zone::Stack, Zone::Graveyard] {
                let source = source(&mut game, owner, zone);
                for combat in [false, true, false] {
                    assert_eq!(damage(&mut game, source, DamageTarget::Object(host), combat, false),
                        (0, vec![(3, host, A)]));
                    assert_eq!(damage(&mut game, source, DamageTarget::Object(other), combat, false),
                        (3, vec![]));
                    for player in [A, B] {
                        assert_eq!(damage(&mut game, source, DamageTarget::Player(player), combat, false),
                            (3, vec![]));
                    }
                    assert_eq!(damage(&mut game, source, DamageTarget::Object(host), combat, true),
                        (3, vec![]), "unpreventable damage is not silently stopped");
                }
            }
        }
        assert!(game.stack.is_empty(), "static prevention never needs an activation");
    }
}

#[test]
fn live_controller_ability_and_zone_changes_preserve_exact_recipient_identity() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = source(&mut game, B, Zone::Battlefield);
        game.set_current_controller(host, B).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false),
            (0, vec![(3, host, B)]));
        game.phase_out(host);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false), (3, vec![]));
        game.phase_in(host);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false),
            (0, vec![(3, host, B)]));
        ApplyContinuousEffect::new(EffectTarget::Specific(host),
            Modification::SetCardTypes(vec![CardType::Artifact]), Until::Forever)
            .execute(&mut game, &mut EffectContext::new_default(host, B)).unwrap();
        assert_eq!(game.calculated_characteristics(host).unwrap().card_types.as_slice(),
            &[CardType::Artifact], "the live type change must actually apply");
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false).0, 0);
        ApplyContinuousEffect::new(EffectTarget::Specific(host),
            Modification::RemoveAllAbilities, Until::Forever)
            .execute(&mut game, &mut EffectContext::new_default(host, B)).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false), (3, vec![]));
        let graveyard = game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(graveyard), false, false), (3, vec![]));
        let returned = game.move_object_by_effect(graveyard, Zone::Battlefield).unwrap();
        assert_ne!(returned, host);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false), (3, vec![]));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(returned), false, false),
            (0, vec![(3, returned, A)]), "new incarnation regains its printed native ability");
    }
}

#[test]
fn native_damage_effect_records_no_damage_until_the_printed_ability_is_removed() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = source(&mut game, B, Zone::Battlefield);
        let effect = ironsmith::Effect::deal_damage(1, ChooseSpec::SpecificObject(host));
        for _ in 0..3 {
            ironsmith::effects::execute_effect(&mut game, &effect,
                &mut EffectContext::new_default(source, B)).unwrap();
            assert_eq!(game.damage_on(host), 0);
        }
        ApplyContinuousEffect::new(EffectTarget::Specific(host),
            Modification::RemoveAllAbilities, Until::Forever)
            .execute(&mut game, &mut EffectContext::new_default(host, A)).unwrap();
        ironsmith::effects::execute_effect(&mut game, &effect,
            &mut EffectContext::new_default(source, B)).unwrap();
        assert_eq!(game.damage_on(host), 1);
    }
}

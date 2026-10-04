//! Authored only. No test/build/compilation is permitted until the campaign gate.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::{Color, ColorSet};
use ironsmith::effects::EffectContext;
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::game_loop::{extract_target_requirements_from_program_with_modes, resolve_stack_entry};
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::targeting::{can_target_object, compute_legal_targets_with_execution_context};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/source_filtered_targeting.json.fixture")).unwrap()
}
fn fixture_definitions(name: &str) -> [CardDefinition; 2] {
    let cards = fixtures(); let card = cards.iter().find(|card| card["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", card["mana_cost"].as_str().unwrap(), card["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str()) { text.push_str(&format!("Power/Toughness: {power}/{toughness}\n")); }
    text.push_str(card["oracle_text"].as_str().unwrap()); definitions(name, &text)
}
fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into()], 20) }
fn a() -> PlayerId { PlayerId::from_index(0) }
fn b() -> PlayerId { PlayerId::from_index(1) }
fn object(game: &mut GameState, owner: PlayerId, colors: ColorSet, zone: Zone) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Targeting subject")
        .card_types(vec![if zone == Zone::Stack { CardType::Instant } else { CardType::Creature }])
        .power_toughness(PowerToughness::fixed(2, 2)).color_indicator(colors).build();
    game.create_object_from_card(&card, owner, zone)
}
fn legal(game: &mut GameState, source: ObjectId, caster: PlayerId, target: ObjectId) -> bool {
    game.refresh_continuous_state().unwrap(); can_target_object(game, target, source, caster).is_legal()
}
fn ability_legal(game: &mut GameState, source: ObjectId, caster: PlayerId, snapshot: ObjectSnapshot, target: ObjectId) -> bool {
    game.refresh_continuous_state().unwrap();
    let ctx = EffectContext::new_default(source, caster).with_source_snapshot(snapshot);
    compute_legal_targets_with_execution_context(game, &ChooseSpec::Object(ObjectFilter::specific(target)), &ctx).contains(&Target::Object(target))
}
fn snapshot(game: &GameState, source: ObjectId) -> ObjectSnapshot {
    ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), game)
}

#[test]
fn eight_complete_frozen_cards_keep_strict_direct_and_restored_artifacts() {
    for card in fixtures() {
        let name = card["name"].as_str().unwrap();
        for definition in fixture_definitions(name) { assert_eq!(definition.card.name, name); }
    }
}

#[test]
fn spell_only_global_restriction_is_symmetric_and_tracks_host_lifetime() {
    for definition in fixture_definitions("Dense Foliage") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let target = object(&mut game, a(), ColorSet::GREEN, Zone::Battlefield);
        let own_spell = object(&mut game, a(), ColorSet::BLUE, Zone::Stack);
        let foreign_spell = object(&mut game, b(), ColorSet::BLACK, Zone::Stack);
        assert!(!legal(&mut game, own_spell, a(), target));
        assert!(!legal(&mut game, foreign_spell, b(), target));
        let source = object(&mut game, b(), ColorSet::BLACK, Zone::Battlefield);
        let retained = snapshot(&game, source);
        assert!(ability_legal(&mut game, source, b(), retained, target));
        // A cast trigger is an ability even while its source is on the stack.
        let retained = snapshot(&game, foreign_spell);
        assert!(ability_legal(&mut game, foreign_spell, b(), retained, target));
        game.phase_out(host);
        assert!(legal(&mut game, foreign_spell, b(), target));
        game.phase_in(host);
        assert!(!legal(&mut game, foreign_spell, b(), target));
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert!(legal(&mut game, foreign_spell, b(), target));
    }
}

#[test]
fn colored_spell_restriction_preserves_color_kind_and_controller() {
    for definition in fixture_definitions("Fiendslayer Paladin") {
        let mut game = game();
        let target = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        for (color, expected) in [(ColorSet::BLACK, false), (ColorSet::RED, false), (ColorSet::BLUE, true), (ColorSet::COLORLESS, true)] {
            let spell = object(&mut game, b(), color, Zone::Stack);
            assert_eq!(legal(&mut game, spell, b(), target), expected);
            let own = object(&mut game, a(), color, Zone::Stack);
            assert!(legal(&mut game, own, a(), target));
            let source = object(&mut game, b(), color, Zone::Battlefield);
            let retained = snapshot(&game, source);
            assert!(ability_legal(&mut game, source, b(), retained, target));
        }
    }
}

#[test]
fn ability_controller_stays_distinct_from_live_or_departed_source_controller() {
    for definition in fixture_definitions("Shanna, Sisay's Legacy") {
        let mut game = game();
        let target = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let source = object(&mut game, b(), ColorSet::RED, Zone::Battlefield);
        let retained = snapshot(&game, source);
        assert!(!ability_legal(&mut game, source, b(), retained.clone(), target));
        game.set_current_controller(source, a()).unwrap();
        assert!(!ability_legal(&mut game, source, b(), retained.clone(), target), "the opponent still controls the ability");
        assert!(ability_legal(&mut game, source, a(), retained.clone(), target));
        game.phase_out(source);
        assert!(!ability_legal(&mut game, source, b(), retained.clone(), target));
        game.phase_in(source);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert!(!ability_legal(&mut game, source, b(), retained, target));
        let spell = object(&mut game, b(), ColorSet::BLUE, Zone::Stack);
        assert!(legal(&mut game, spell, b(), target));
        let cast_trigger_snapshot = snapshot(&game, spell);
        assert!(!ability_legal(&mut game, spell, b(), cast_trigger_snapshot, target));
    }
}

#[test]
fn ability_from_quality_checks_current_source_then_last_known_source() {
    for definition in fixture_definitions("Thrun, Breaker of Silence") {
        let mut game = game();
        let target = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let source = object(&mut game, b(), ColorSet::BLACK, Zone::Battlefield);
        let retained = snapshot(&game, source);
        assert!(!ability_legal(&mut game, source, b(), retained.clone(), target));
        game.set_current_controller(source, a()).unwrap();
        assert!(ability_legal(&mut game, source, b(), retained.clone(), target), "source is no longer opponent controlled");
        game.set_current_controller(source, b()).unwrap();
        game.object_mut(source).unwrap().color_override = Some(ColorSet::BLACK.with(Color::Green));
        assert!(ability_legal(&mut game, source, b(), retained.clone(), target), "green multicolor source is not nongreen");
        game.object_mut(source).unwrap().color_override = Some(ColorSet::BLACK);
        let departed = snapshot(&game, source);
        game.phase_out(source);
        assert!(!ability_legal(&mut game, source, b(), departed.clone(), target));
        game.phase_in(source);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert!(!ability_legal(&mut game, source, b(), departed, target));
    }
}

#[test]
fn graveyard_prohibition_covers_both_players_and_current_incarnations_only() {
    for name in ["Ground Seal", "Silent Gravestone", "Underworld Cerberus"] {
        for definition in fixture_definitions(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let source = object(&mut game, b(), ColorSet::BLUE, Zone::Stack);
            let own_source = object(&mut game, a(), ColorSet::GREEN, Zone::Stack);
            for owner in [a(), b()] {
                let old = object(&mut game, owner, ColorSet::GREEN, Zone::Graveyard);
                assert!(!legal(&mut game, source, b(), old), "{name}");
                assert!(!legal(&mut game, own_source, a(), old), "{name}: neither player exempt");
                let ability_source = object(&mut game, owner, ColorSet::GREEN, Zone::Battlefield);
                let retained = snapshot(&game, ability_source);
                assert!(!ability_legal(&mut game, ability_source, owner, retained, old));
                let current = game.move_object_by_effect(old, Zone::Battlefield).unwrap();
                assert_ne!(old, current);
                assert!(legal(&mut game, source, b(), current));
                let returned = game.move_object_by_effect(current, Zone::Graveyard).unwrap();
                assert_ne!(current, returned);
                assert!(!legal(&mut game, source, b(), returned));
                game.phase_out(host);
                assert!(legal(&mut game, source, b(), returned));
                game.phase_in(host);
                assert!(!legal(&mut game, source, b(), returned));
            }
            game.move_object_by_effect(host, Zone::Exile).unwrap();
            let later = object(&mut game, a(), ColorSet::GREEN, Zone::Graveyard);
            assert!(legal(&mut game, source, b(), later));
        }
    }
}

fn cast_display(game: &mut GameState, definition: &CardDefinition) {
    let spell = game.create_object_from_definition(definition, a(), Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, a()).with_chosen_modes(Some(vec![1])));
    resolve_stack_entry(game).unwrap();
}

#[test]
fn temporary_rule_tracks_later_permanents_controller_changes_and_cleanup() {
    for definition in fixture_definitions("Display of Dominance") {
        let mut game = game();
        cast_display(&mut game, &definition);
        let late = object(&mut game, a(), ColorSet::GREEN, Zone::Battlefield);
        let spell = object(&mut game, b(), ColorSet::BLUE, Zone::Stack);
        assert!(!legal(&mut game, spell, b(), late), "rule includes permanents gained later");
        let own_spell = object(&mut game, a(), ColorSet::BLUE, Zone::Stack);
        assert!(legal(&mut game, own_spell, a(), late));
        let source = object(&mut game, b(), ColorSet::BLACK, Zone::Battlefield);
        let retained = snapshot(&game, source);
        assert!(ability_legal(&mut game, source, b(), retained, late));
        game.set_current_controller(late, b()).unwrap();
        assert!(legal(&mut game, spell, b(), late));
        game.set_current_controller(late, a()).unwrap();
        assert!(!legal(&mut game, spell, b(), late));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(legal(&mut game, spell, b(), late));
    }
}

#[test]
fn resolution_recheck_uses_spell_role_even_though_stack_entry_has_snapshot() {
    for display in fixture_definitions("Display of Dominance") {
        for (mana, protected) in [("{U}", true), ("{G}", false)] {
            for removal in definitions("Unlisted Removal", &format!("Mana cost: {mana}\nType: Instant\nDestroy target creature.")) {
                let mut game = game();
                let target = object(&mut game, a(), ColorSet::GREEN, Zone::Battlefield);
                let spell = game.create_object_from_definition(&removal, b(), Zone::Stack);
                game.refresh_continuous_state().unwrap();
                let requirements = extract_target_requirements_from_program_with_modes(&game, removal.spell_effect.as_ref().unwrap(), b(), Some(spell), None);
                assert_eq!(requirements.len(), 1);
                assert!(requirements[0].legal_targets.contains(&Target::Object(target)));
                game.push_to_stack(StackEntry::new(spell, b()).with_targets(vec![Target::Object(target)])
                    .with_target_assignments(vec![TargetAssignment { spec: requirements[0].spec.clone(), range: 0..1 }]));
                cast_display(&mut game, &display);
                resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.object(target).is_some(), protected, "blue spell loses its target at resolution; green does not");
            }
        }
    }
}

#[test]
fn activated_and_triggered_programs_recheck_ability_control_on_real_stack_resolution() {
    use ironsmith::ability::AbilityKind;
    for protected in fixture_definitions("Shanna, Sisay's Legacy") {
        for source_definition in definitions("Unlisted Ability Source", "Mana cost: {2}{R}\nType: Creature — Human\nPower/Toughness: 2/2\nWhen this creature enters, destroy target creature.\n{T}: Destroy target creature.") {
            for (kind, program) in source_definition.abilities.iter().filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(ability) => Some(("activated", ability.effects.clone())),
                AbilityKind::Triggered(ability) => Some(("triggered", ability.effects.clone())),
                _ => None,
            }) {
                for departure in [0, 1, 2] {
                    let mut game = game();
                    let target = game.create_object_from_definition(&protected, b(), Zone::Battlefield);
                    let source = game.create_object_from_definition(&source_definition, b(), Zone::Battlefield);
                    game.refresh_continuous_state().unwrap();
                    let requirements = extract_target_requirements_from_program_with_modes(&game, &program, b(), Some(source), None);
                    assert!(requirements[0].legal_targets.contains(&Target::Object(target)), "{kind}: initially own target");
                    game.push_to_stack(StackEntry::ability(source, b(), program.clone())
                        .with_targets(vec![Target::Object(target)])
                        .with_target_assignments(vec![TargetAssignment { spec: requirements[0].spec.clone(), range: 0..1 }]));
                    game.set_current_controller(target, a()).unwrap();
                    if departure == 1 { game.phase_out(source); }
                    if departure == 2 { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
                    resolve_stack_entry(&mut game).unwrap();
                    assert!(game.object(target).is_some(), "{kind}: opponent-controlled ability loses its target, departure={departure}");
                }
            }
        }
    }
}

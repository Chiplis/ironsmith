//! Complete frozen card bodies and source-authored scenarios. Execution deferred.
use ironsmith::ability::Ability;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::object::AttachmentTarget;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const NAMES: &[&str] = &[
    "Argothian Pixies", "Argothian Treefolk", "Dawn Elemental", "Desert Nomads",
    "Tresserhorn Skyknight", "Wall of Putrid Flesh",
];

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/permanent_self_damage_prevention.json.fixture")).unwrap();
    assert_eq!(rows.len(), NAMES.len());
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
        row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded, artifact);
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = result.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    [direct, materialize_artifact(&decoded).unwrap()]
}

fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into()], 20) }

fn object(game: &mut GameState, owner: PlayerId, zone: Zone, types: Vec<CardType>,
    subtypes: Vec<Subtype>, abilities: Vec<StaticAbility>) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Unlisted source probe")
        .card_types(types).subtypes(subtypes).power_toughness(PowerToughness::fixed(2, 8)).build();
    let mut definition = CardDefinition::new(card);
    definition.abilities = abilities.into_iter().map(Ability::static_ability).collect();
    game.create_object_from_definition(&definition, owner, zone)
}
fn creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
    object(game, owner, Zone::Battlefield, vec![CardType::Creature], vec![], vec![])
}
fn attach_aura(game: &mut GameState, target: ObjectId) -> ObjectId {
    let aura = object(game, B, Zone::Battlefield, vec![CardType::Enchantment], vec![Subtype::Aura], vec![]);
    assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(target)));
    aura
}
fn change(game: &mut GameState, target: ObjectId, modification: Modification) {
    ApplyContinuousEffect::new(EffectTarget::Specific(target), modification, Until::Forever)
        .execute(game, &mut EffectContext::new_default(target, B)).unwrap();
}
fn snapshot(game: &GameState, source: ObjectId) -> ObjectSnapshot {
    ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), game)
}
fn damage(game: &mut GameState, source: ObjectId, target: DamageTarget, combat: bool,
    unpreventable: bool, lki: Option<&ObjectSnapshot>) -> (u32, Vec<(u32, ObjectId, PlayerId)>) {
    game.take_pending_trigger_events();
    let result = process_damage_assignments_with_event_with_source_snapshot_opts(
        game, source, target, 3, combat, unpreventable, EventCause::effect(), lki).unwrap();
    let amount = result.assignments.iter().map(|assignment| assignment.amount).sum();
    let prevented = game.take_pending_trigger_events().into_iter().filter_map(|event|
        event.downcast::<DamagePreventedEvent>().map(|event|
            (event.amount, event.prevention_source, event.prevention_controller))).collect();
    (amount, prevented)
}
fn source_for(game: &mut GameState, name: &str) -> ObjectId {
    match name {
        "Argothian Pixies" => object(game, B, Zone::Battlefield,
            vec![CardType::Artifact, CardType::Creature], vec![], vec![]),
        "Argothian Treefolk" => object(game, B, Zone::Battlefield,
            vec![CardType::Artifact], vec![], vec![]),
        "Desert Nomads" => object(game, B, Zone::Battlefield,
            vec![CardType::Land], vec![Subtype::Desert], vec![]),
        "Tresserhorn Skyknight" => object(game, B, Zone::Battlefield,
            vec![CardType::Creature], vec![], vec![StaticAbility::first_strike()]),
        "Wall of Putrid Flesh" => {
            let source = creature(game, B);
            attach_aura(game, source);
            source
        }
        "Dawn Elemental" => creature(game, B),
        _ => panic!("unexpected frozen identity"),
    }
}

#[test]
fn all_six_complete_bodies_prevent_repeatable_combat_and_noncombat_damage_to_exact_self() {
    for name in NAMES { for definition in definitions(name) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = source_for(&mut game, name);
        let other = creature(&mut game, A);
        for combat in [false, true, false] {
            assert_eq!(damage(&mut game, source, DamageTarget::Object(host), combat, false, None),
                (0, vec![(3, host, A)]), "{name}");
            assert_eq!(damage(&mut game, source, DamageTarget::Object(other), combat, false, None), (3, vec![]));
            assert_eq!(damage(&mut game, source, DamageTarget::Player(A), combat, false, None), (3, vec![]));
            assert_eq!(damage(&mut game, source, DamageTarget::Object(host), combat, true, None), (3, vec![]));
        }
        game.set_current_controller(host, B).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, None), (0, vec![(3, host, B)]));
        // The rule follows the exact permanent even after a type change.
        change(&mut game, host, Modification::SetCardTypes(vec![CardType::Artifact]));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, None).0, 0);
        change(&mut game, host, Modification::RemoveAllAbilities);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, None), (3, vec![]));
    } }
}

#[test]
fn artifact_sources_and_artifact_creatures_have_different_domains_and_conjunctive_types() {
    for name in ["Argothian Pixies", "Argothian Treefolk"] { for definition in definitions(name) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for (types, zone, matches_pixies, matches_treefolk) in [
            (vec![CardType::Artifact, CardType::Creature], Zone::Battlefield, true, true),
            (vec![CardType::Artifact], Zone::Battlefield, false, true),
            (vec![CardType::Creature], Zone::Battlefield, false, false),
            (vec![CardType::Artifact, CardType::Creature], Zone::Stack, false, true),
            (vec![CardType::Artifact], Zone::Graveyard, false, true),
            (vec![CardType::Sorcery], Zone::Stack, false, false),
        ] {
            let source = object(&mut game, B, zone, types, vec![], vec![]);
            let expected = if name == "Argothian Pixies" { matches_pixies } else { matches_treefolk };
            assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, None).0,
                if expected { 0 } else { 3 }, "{name}: {zone:?}");
        }
        let source = source_for(&mut game, name);
        let old = snapshot(&game, source);
        change(&mut game, source, Modification::RemoveCardTypes(vec![CardType::Artifact]));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, Some(&old)), (3, vec![]),
            "a live source's changed type wins over an older matching snapshot");
        change(&mut game, source, Modification::AddCardTypes(vec![CardType::Artifact]));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, None).0, 0);
    } }
}

#[test]
fn first_strike_and_desert_are_current_source_qualities_not_damage_step_or_card_names() {
    for definition in definitions("Tresserhorn Skyknight") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = source_for(&mut game, "Tresserhorn Skyknight");
        let double = object(&mut game, B, Zone::Battlefield, vec![CardType::Creature], vec![], vec![StaticAbility::double_strike()]);
        assert_eq!(damage(&mut game, double, DamageTarget::Object(host), true, false, None).0, 3,
            "double strike does not confer first strike");
        let old = snapshot(&game, first);
        change(&mut game, first, Modification::RemoveAllAbilities);
        assert_eq!(damage(&mut game, first, DamageTarget::Object(host), false, false, Some(&old)).0, 3);
        change(&mut game, double, Modification::AddAbility(StaticAbility::first_strike()));
        assert_eq!(damage(&mut game, double, DamageTarget::Object(host), false, false, None).0, 0);
    }
    for definition in definitions("Desert Nomads") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let desert = source_for(&mut game, "Desert Nomads");
        for zone in [Zone::Graveyard, Zone::Stack] {
            let card = object(&mut game, B, zone, vec![CardType::Land], vec![Subtype::Desert], vec![]);
            assert_eq!(damage(&mut game, card, DamageTarget::Object(host), false, false, None).0, 3,
                "a Desert card in another zone is not a Desert permanent");
        }
        let old = snapshot(&game, desert);
        change(&mut game, desert, Modification::SetSubtypes(vec![Subtype::Mountain]));
        assert_eq!(damage(&mut game, desert, DamageTarget::Object(host), false, false, Some(&old)).0, 3);
        change(&mut game, desert, Modification::AddSubtypes(vec![Subtype::Desert]));
        assert_eq!(damage(&mut game, desert, DamageTarget::Object(host), false, false, None).0, 0);
    }
}

#[test]
fn enchanted_creatures_track_active_auras_and_exact_attachment_lki() {
    for definition in definitions("Wall of Putrid Flesh") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = creature(&mut game, B);
        let equipment = object(&mut game, B, Zone::Battlefield, vec![CardType::Artifact], vec![Subtype::Equipment], vec![]);
        assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(source)));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, None).0, 3);
        let aura = attach_aura(&mut game, source);
        let enchanted = snapshot(&game, source);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, None).0, 0);
        game.phase_out(aura);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, Some(&enchanted)).0, 3,
            "a phased-out Aura does not make a live creature enchanted");
        let unenchanted = snapshot(&game, source);
        assert!(!unenchanted.was_enchanted);
        assert!(!unenchanted.attachment_snapshots.iter().any(|attachment| attachment.object_id == aura));
        game.phase_in(aura);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, Some(&unenchanted)).0, 0);
        let other = creature(&mut game, B);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(other)));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, Some(&enchanted)).0, 3);
        assert_eq!(damage(&mut game, other, DamageTarget::Object(host), false, false, None).0, 0);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, Some(&enchanted)).0, 0,
            "departed source uses attachment characteristics from its supplied exact LKI");
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, Some(&unenchanted)).0, 3,
            "LKI captured while the Aura was phased out must remain unenchanted");
    }
}

#[test]
fn exact_source_lki_survives_departure_and_phasing_without_using_a_new_incarnation() {
    for name in NAMES { for definition in definitions(name) { for phased in [false, true] {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = source_for(&mut game, name);
        let lki = snapshot(&game, source);
        if phased { game.phase_out(source); }
        else {
            let graveyard = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            // A returned card has a different identity, and cannot provide
            // current characteristics for the earlier object's damage.
            let returned = game.move_object_by_effect(graveyard, Zone::Battlefield).unwrap();
            assert_ne!(returned, source);
            change(&mut game, returned, Modification::SetCardTypes(vec![CardType::Artifact]));
            change(&mut game, returned, Modification::RemoveAllAbilities);
        }
        game.turn_store.turn_history.clear_for_new_turn();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, Some(&lki)).0, 0, "{name}: phased={phased}");
    } } }
}

#[test]
fn missing_or_wrong_source_lki_does_not_claim_a_filtered_damage_result() {
    for definition in definitions("Argothian Treefolk") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = source_for(&mut game, "Argothian Treefolk");
        let unrelated = creature(&mut game, B);
        let wrong = snapshot(&game, unrelated);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        game.turn_store.turn_history.clear_for_new_turn();
        for lki in [None, Some(&wrong)] {
            let result = process_damage_assignments_with_event_with_source_snapshot_opts(
                &mut game, source, DamageTarget::Object(host), 3, false, false, EventCause::effect(), lki);
            assert!(result.is_err(), "filtered damage needs the exact missing source's evidence");
        }
    }
}

#[test]
fn prevention_source_is_live_and_disappears_on_phasing_departure_or_ability_loss() {
    for name in NAMES { for definition in definitions(name) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = source_for(&mut game, name);
        game.phase_out(host);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, None).0, 3);
        game.phase_in(host);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, None).0, 0);
        let graveyard = game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        let returned = game.move_object_by_effect(graveyard, Zone::Battlefield).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, None).0, 3);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(returned), false, false, None).0, 0);
    } }
}

#[test]
fn companion_keywords_and_blocking_rules_complete_the_frozen_bodies() {
    for definition in definitions("Argothian Pixies") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let artifact = source_for(&mut game, "Argothian Pixies");
        let ordinary = creature(&mut game, B);
        assert!(!ironsmith::rules::combat::can_block(game.object(host).unwrap(), game.object(artifact).unwrap(), &game));
        assert!(ironsmith::rules::combat::can_block(game.object(host).unwrap(), game.object(ordinary).unwrap(), &game));
    }
    for name in ["Dawn Elemental", "Tresserhorn Skyknight"] { for definition in definitions(name) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ordinary = creature(&mut game, B);
        let flyer = object(&mut game, B, Zone::Battlefield, vec![CardType::Creature], vec![], vec![StaticAbility::flying()]);
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Flying));
        assert!(!ironsmith::rules::combat::can_block(game.object(host).unwrap(), game.object(ordinary).unwrap(), &game));
        assert!(ironsmith::rules::combat::can_block(game.object(host).unwrap(), game.object(flyer).unwrap(), &game));
    } }
    for definition in definitions("Desert Nomads") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let blocker = creature(&mut game, B);
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Landwalk));
        assert!(ironsmith::rules::combat::can_block(game.object(host).unwrap(), game.object(blocker).unwrap(), &game));
        let desert = source_for(&mut game, "Desert Nomads");
        assert!(!ironsmith::rules::combat::can_block(game.object(host).unwrap(), game.object(blocker).unwrap(), &game));
        game.phase_out(desert);
        assert!(ironsmith::rules::combat::can_block(game.object(host).unwrap(), game.object(blocker).unwrap(), &game));
    }
    for definition in definitions("Wall of Putrid Flesh") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Defender));
        change(&mut game, host, Modification::AddAbility(StaticAbility::haste()));
        assert!(!ironsmith::rules::combat::can_attack(game.object(host).unwrap(), &game));
        let white = creature(&mut game, B);
        change(&mut game, white, Modification::SetColors(ironsmith::color::ColorSet::WHITE));
        assert_eq!(damage(&mut game, white, DamageTarget::Object(host), false, false, None).0, 0,
            "protection from white is independent of enchanted-creature prevention");
        assert_eq!(damage(&mut game, white, DamageTarget::Object(host), false, true, None).0, 3);
    }
}


#[test]
fn unrelated_names_use_the_same_typed_complete_body() {
    let text = "Mana cost: {2}{G}\nType: Creature — Treefolk\nPower/Toughness: 2/4\nPrevent all damage that would be dealt to this creature by artifact sources.";
    let (artifact, direct) = compile_to_artifact("An Unlisted Prevention Creature", text, false).unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct, materialize_artifact(&decoded).unwrap()] {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = source_for(&mut game, "Argothian Treefolk");
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), false, false, None), (0, vec![(3, host, A)]));
    }
}

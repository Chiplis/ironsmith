use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::{CompiledCardArtifact, WireAbility};
use ironsmith_compiler_runtime::compile_builder_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::{
    encode_runtime_static_ability, materialize_artifact, restore_runtime_ability,
};

const SOURCES: &[(&str, &str)] = &[
    (
        "Crevasse",
        "Mana cost: {2}{R}\nType: Enchantment\nCreatures with mountainwalk can be blocked as though they didn't have mountainwalk.",
    ),
    (
        "Deadfall",
        "Mana cost: {2}{G}\nType: Enchantment\nCreatures with forestwalk can be blocked as though they didn't have forestwalk.",
    ),
    (
        "Gosta Dirk",
        "Mana cost: {3}{W}{W}{U}{U}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 4/4\nFirst strike\nCreatures with islandwalk can be blocked as though they didn't have islandwalk.",
    ),
    (
        "Great Wall",
        "Mana cost: {2}{W}\nType: Enchantment\nCreatures with plainswalk can be blocked as though they didn't have plainswalk.",
    ),
    (
        "Lord Magnus",
        "Mana cost: {3}{G}{W}{W}\nType: Legendary Creature — Human Druid\nPower/Toughness: 4/3\nFirst strike\nCreatures with plainswalk can be blocked as though they didn't have plainswalk.\nCreatures with forestwalk can be blocked as though they didn't have forestwalk.",
    ),
    (
        "Quagmire",
        "Mana cost: {2}{B}\nType: Enchantment\nCreatures with swampwalk can be blocked as though they didn't have swampwalk.",
    ),
    (
        "Staff of the Ages",
        "Mana cost: {3}\nType: Artifact\nCreatures with landwalk abilities can be blocked as though they didn't have those abilities.",
    ),
    (
        "Undertow",
        "Mana cost: {2}{U}\nType: Enchantment\nCreatures with islandwalk can be blocked as though they didn't have islandwalk.",
    ),
    (
        "Ur-Drago",
        "Mana cost: {3}{U}{U}{B}{B}\nType: Legendary Creature — Elemental\nPower/Toughness: 4/4\nFirst strike\nCreatures with swampwalk can be blocked as though they didn't have swampwalk.",
    ),
];

fn compile(name: &str, text: &str) -> CardDefinition {
    let (artifact, _) = compile_builder_to_artifact(
        ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
        text,
        false,
    )
    .expect("landwalk permission must compile without a fallback");
    let encoded = artifact.to_json().expect("current artifact must encode");
    let decoded = CompiledCardArtifact::from_json(&encoded).expect("current artifact must decode");
    assert_eq!(decoded, artifact);
    materialize_artifact(&decoded).expect("decoded permission must materialize")
}

fn creature(game: &mut GameState, owner: PlayerId, abilities: Vec<StaticAbility>) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Block legality probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut definition = CardDefinition::new(card);
    for ability in abilities {
        definition.abilities.push(Ability::static_ability(ability));
    }
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

fn land(game: &mut GameState, owner: PlayerId, subtype: Subtype) {
    let card = CardBuilder::new(CardId::new(), "Defending land")
        .card_types(vec![CardType::Land])
        .subtypes(vec![subtype])
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield);
}

fn can_block(game: &GameState, attacker: ObjectId, blocker: ObjectId) -> bool {
    ironsmith::rules::combat::can_block(
        game.object(attacker).unwrap(),
        game.object(blocker).unwrap(),
        game,
    )
}

fn has_landwalk(game: &GameState, attacker: ObjectId) -> bool {
    game.calculated_characteristics(attacker)
        .unwrap()
        .static_abilities
        .iter()
        .any(|ability| ability.id() == StaticAbilityId::Landwalk)
}

#[test]
fn landwalk_override_all_source_cards_round_trip_with_typed_permission() {
    for &(name, text) in SOURCES {
        let definition = compile(name, text);
        let permissions = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability)
                    if ability.id() == StaticAbilityId::BlockingAsThoughNoLandwalk =>
                {
                    Some(ability)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            permissions.len(),
            if name == "Lord Magnus" { 2 } else { 1 },
            "{name}"
        );
        for permission in permissions {
            let wire = encode_runtime_static_ability(permission.clone())
                .expect("typed permission encodes");
            let restored = restore_runtime_ability(WireAbility::static_ability(wire.clone()))
                .expect("typed permission restores");
            let AbilityKind::Static(restored) = restored.kind else {
                panic!("static permission")
            };
            assert_eq!(encode_runtime_static_ability(restored).unwrap(), wire);
        }
        // A renamed input must preserve behavior without any card-name dispatch.
        let renamed = compile("Synthetic Evasion Permission", text);
        assert!(renamed.abilities.iter().any(|ability| matches!(&ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::BlockingAsThoughNoLandwalk)));
    }
}

#[test]
fn landwalk_override_all_source_cards_preserve_abilities_and_only_override_matching_walk() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for &(name, text) in SOURCES {
        let definition = compile(name, text);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let blocker = creature(&mut game, bob, vec![]);
        let types = [
            Subtype::Plains,
            Subtype::Island,
            Subtype::Swamp,
            Subtype::Mountain,
            Subtype::Forest,
        ];
        for subtype in types {
            land(&mut game, bob, subtype);
        }
        for subtype in types {
            let attacker = creature(&mut game, alice, vec![StaticAbility::landwalk(subtype)]);
            game.update_cant_effects();
            let word = format!("{}walk", subtype.to_string().to_ascii_lowercase());
            let expected = name == "Staff of the Ages" || text.contains(&word);
            assert_eq!(
                can_block(&game, attacker, blocker),
                expected,
                "{name}: {word}"
            );
            assert!(
                has_landwalk(&game, attacker),
                "{name} must never remove {word}"
            );
            let flyer = creature(
                &mut game,
                alice,
                vec![StaticAbility::landwalk(subtype), StaticAbility::flying()],
            );
            game.update_cant_effects();
            assert!(
                !can_block(&game, flyer, blocker),
                "{name} must not ignore flying"
            );
        }
        let islandwalker = creature(
            &mut game,
            alice,
            vec![StaticAbility::landwalk(Subtype::Island)],
        );
        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("source should leave");
        game.update_cant_effects();
        assert!(
            !can_block(&game, islandwalker, blocker),
            "source departure must end permission"
        );
        assert!(has_landwalk(&game, islandwalker));
    }
}

#[test]
fn landwalk_override_source_phasing_and_ability_loss_end_permission() {
    let text = SOURCES
        .iter()
        .find(|(name, _)| *name == "Gosta Dirk")
        .unwrap()
        .1;
    let definition = compile("Synthetic Evasion Permission", text);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let attacker = creature(
        &mut game,
        alice,
        vec![StaticAbility::landwalk(Subtype::Island)],
    );
    let blocker = creature(&mut game, bob, vec![]);
    land(&mut game, bob, Subtype::Island);
    game.update_cant_effects();
    assert!(can_block(&game, attacker, blocker));
    game.phase_out(source);
    game.update_cant_effects();
    assert!(!can_block(&game, attacker, blocker));
    game.phase_in(source);
    game.update_cant_effects();
    assert!(can_block(&game, attacker, blocker));
    game.object_mut(source).unwrap().abilities_mut().clear();
    game.update_cant_effects();
    assert!(!can_block(&game, attacker, blocker));
    assert!(has_landwalk(&game, attacker));
}

#[test]
fn landwalk_override_blanket_permission_covers_special_walk_without_erasing_it() {
    let text = SOURCES
        .iter()
        .find(|(name, _)| *name == "Staff of the Ages")
        .unwrap()
        .1;
    let definition = compile("Synthetic Evasion Permission", text);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let blocker = creature(&mut game, bob, vec![]);
    let special_land = CardBuilder::new(CardId::new(), "Snow artifact island")
        .card_types(vec![CardType::Land, CardType::Artifact])
        .supertypes(vec![ironsmith::Supertype::Snow])
        .subtypes(vec![Subtype::Island])
        .build();
    game.create_object_from_card(&special_land, bob, Zone::Battlefield);
    for ability in [
        StaticAbility::any_landwalk(),
        StaticAbility::nonbasic_landwalk(),
        StaticAbility::snow_landwalk(Subtype::Island),
        StaticAbility::artifact_landwalk(),
    ] {
        let attacker = creature(&mut game, alice, vec![ability]);
        game.update_cant_effects();
        assert!(can_block(&game, attacker, blocker));
        assert!(has_landwalk(&game, attacker));
    }
}

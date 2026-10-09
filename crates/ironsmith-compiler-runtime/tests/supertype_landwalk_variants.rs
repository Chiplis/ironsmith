//! CR 702.14c legendary/snow landwalk. Source-authored, deliberately unrun.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::static_abilities::{LandwalkKind, StaticAbility, StaticAbilityId};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Supertype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const CARDS: &[(&str, &str, &str, LandwalkKind)] = &[
    (
        "Livonya Silone",
        "57f2aa02-f5f3-42a5-939d-5cc94200951e",
        "Mana cost: {2}{R}{R}{G}{G}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 4/4\nFirst strike; legendary landwalk (This creature can't be blocked as long as defending player controls a legendary land.)",
        LandwalkKind::LegendaryLand,
    ),
    (
        "Ayumi, the Last Visitor",
        "12acd466-deb9-4e68-b75b-aed86680b917",
        "Mana cost: {3}{G}{G}\nType: Legendary Creature — Spirit\nPower/Toughness: 7/3\nLegendary landwalk (This creature can't be blocked as long as defending player controls a legendary land.)",
        LandwalkKind::LegendaryLand,
    ),
    (
        "Zombie Musher",
        "89f87341-c18f-4dc7-ba2e-81305383231e",
        "Mana cost: {3}{B}\nType: Snow Creature — Zombie\nPower/Toughness: 2/3\nSnow landwalk (This creature can't be blocked as long as defending player controls a snow land.)\n{S}: Regenerate this creature. ({S} can be paid with one mana from a snow source.)",
        LandwalkKind::SnowLand,
    ),
];

fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

fn landwalk_kinds(definition: &CardDefinition) -> Vec<LandwalkKind> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::Landwalk => {
                ability.landwalk_kind()
            }
            _ => None,
        })
        .collect()
}

#[test]
fn supertype_landwalk_full_bodies_compile_on_both_routes() {
    for &(name, _oracle_id, text, kind) in CARDS {
        for definition in routes(name, text) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert_eq!(landwalk_kinds(&definition), vec![kind], "{name}");
            if name == "Livonya Silone" {
                assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
                    AbilityKind::Static(ability) if ability.id() == StaticAbilityId::FirstStrike)));
            }
            if name == "Zombie Musher" {
                assert!(definition
                    .abilities
                    .iter()
                    .any(|ability| matches!(&ability.kind, AbilityKind::Activated(_))));
            }
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            let word = match kind {
                LandwalkKind::LegendaryLand => "legendary landwalk",
                _ => "snow landwalk",
            };
            assert!(rendered.to_ascii_lowercase().contains(word), "{rendered}");
        }
    }
}

fn creature(game: &mut GameState, owner: PlayerId, abilities: Vec<StaticAbility>) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Landwalk probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut definition = CardDefinition::new(card);
    for ability in abilities {
        definition.abilities.push(Ability::static_ability(ability));
    }
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

fn land(game: &mut GameState, owner: PlayerId, supertypes: Vec<Supertype>) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Defending land")
        .card_types(vec![CardType::Land])
        .supertypes(supertypes)
        .subtypes(vec![Subtype::Forest])
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

fn can_block(game: &GameState, attacker: ObjectId, blocker: ObjectId) -> bool {
    ironsmith::rules::combat::can_block(
        game.object(attacker).unwrap(),
        game.object(blocker).unwrap(),
        game,
    )
}

#[test]
fn supertype_landwalk_checks_only_the_defending_players_matching_lands() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for &(name, _oracle_id, text, kind) in CARDS {
        for definition in routes(name, text) {
            let required = match kind {
                LandwalkKind::LegendaryLand => Supertype::Legendary,
                _ => Supertype::Snow,
            };
            let other = match kind {
                LandwalkKind::LegendaryLand => Supertype::Snow,
                _ => Supertype::Legendary,
            };
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let attacker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let blocker = creature(&mut game, bob, vec![]);
            // Plain and wrong-supertype lands controlled by the defender do not
            // enable the evasion; neither does a matching land the attacker
            // controls.
            land(&mut game, bob, vec![]);
            land(&mut game, bob, vec![Supertype::Basic, other]);
            land(&mut game, alice, vec![required]);
            game.update_cant_effects();
            assert!(can_block(&game, attacker, blocker), "{name}: no matching defending land");
            let matching = land(&mut game, bob, vec![required]);
            game.update_cant_effects();
            assert!(!can_block(&game, attacker, blocker), "{name}: matching defending land");
            game.move_object_by_effect(matching, Zone::Graveyard).unwrap();
            game.update_cant_effects();
            assert!(can_block(&game, attacker, blocker), "{name}: land left");
        }
    }
}

#[test]
fn bare_unqualified_landwalk_phrases_keep_their_existing_kinds() {
    for (text, kind) in [
        ("Landwalk", LandwalkKind::AnyLand),
        ("Nonbasic landwalk", LandwalkKind::NonbasicLand),
        ("Artifact landwalk", LandwalkKind::ArtifactLand),
        ("Snow swampwalk", LandwalkKind::Subtype { subtype: Subtype::Swamp, snow: true }),
    ] {
        let source = format!("Mana cost: {{2}}\nType: Creature — Bear\nPower/Toughness: 2/2\n{text}");
        for definition in routes("Landwalk control", &source) {
            assert_eq!(landwalk_kinds(&definition), vec![kind], "{text}");
        }
    }
}

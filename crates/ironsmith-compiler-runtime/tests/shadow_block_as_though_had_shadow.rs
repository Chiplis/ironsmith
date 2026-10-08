//! CR 702.28b shadow-blocking permission worded from the blocker's side
//! ("as though it had shadow"). Source-authored, deliberately unrun.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const CARDS: &[(&str, &str, &str)] = &[
    (
        "Heartwood Dryad",
        "976968c5-7ef4-4451-86ff-03a011bbcca7",
        "Mana cost: {1}{G}\nType: Creature — Dryad\nPower/Toughness: 2/1\nThis creature can block creatures with shadow as though it had shadow.",
    ),
    (
        "Wall of Diffusion",
        "ae42f803-f4ac-45f3-9ce7-227a59096f19",
        "Mana cost: {1}{R}\nType: Creature — Wall\nPower/Toughness: 0/5\nDefender (This creature can't attack.)\nThis creature can block creatures with shadow as though it had shadow.",
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

fn has(definition: &CardDefinition, id: StaticAbilityId) -> bool {
    definition
        .abilities
        .iter()
        .any(|ability| matches!(&ability.kind, AbilityKind::Static(ability) if ability.id() == id))
}

fn creature(game: &mut GameState, owner: PlayerId, abilities: Vec<StaticAbility>) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Shadow probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut definition = CardDefinition::new(card);
    for ability in abilities {
        definition.abilities.push(Ability::static_ability(ability));
    }
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

fn can_block(game: &GameState, attacker: ObjectId, blocker: ObjectId) -> bool {
    ironsmith::rules::combat::can_block(
        game.object(attacker).unwrap(),
        game.object(blocker).unwrap(),
        game,
    )
}

#[test]
fn blocker_side_shadow_permission_compiles_on_both_routes() {
    for &(name, _oracle_id, text) in CARDS {
        for definition in routes(name, text) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert!(has(&definition, StaticAbilityId::CanBlockAsThoughNoShadow), "{name}");
            assert!(!has(&definition, StaticAbilityId::Shadow), "{name} must not gain shadow");
            assert_eq!(has(&definition, StaticAbilityId::Defender), name == "Wall of Diffusion");
        }
    }
}

#[test]
fn blocker_side_shadow_permission_blocks_shadow_and_ordinary_attackers() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for &(name, _oracle_id, text) in CARDS {
        for definition in routes(name, text) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let shadow = creature(&mut game, alice, vec![StaticAbility::shadow()]);
            let plain = creature(&mut game, alice, vec![]);
            let blocker = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
            let control = creature(&mut game, bob, vec![]);
            game.update_cant_effects();
            assert!(can_block(&game, shadow, blocker), "{name} blocks shadow");
            assert!(can_block(&game, plain, blocker), "{name} still blocks non-shadow");
            assert!(!can_block(&game, shadow, control), "control creature cannot block shadow");
        }
    }
}

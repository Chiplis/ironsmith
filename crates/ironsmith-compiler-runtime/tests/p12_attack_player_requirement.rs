//! "Target creature attacks <player> this turn if able" (CR 508.1d).
//! Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, MustAttackPlayerThisTurnEffect, execute_effect};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::{GameState, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const CARDS: [(&str, &str); 3] = [
    ("Alluring Siren", "Mana cost: {1}{U}\nType: Creature — Siren\nPower/Toughness: 1/1\n{T}: Target creature an opponent controls attacks you this turn if able."),
    ("Dulcet Sirens", "Mana cost: {2}{U}\nType: Creature — Siren\nPower/Toughness: 1/3\n{U}, {T}: Target creature attacks target opponent this turn if able.\nMorph {U}"),
    ("Ravener", "Mana cost: {X}{G}{U}\nType: Creature — Tyranid\nPower/Toughness: 0/0\nFlash\nRavenous\nWhen this creature enters, target creature attacks target opponent this turn if able."),
];

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

fn requirement(definition: &CardDefinition) -> MustAttackPlayerThisTurnEffect {
    fn collect(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| collect(child, out));
    }
    let mut all = Vec::new();
    for ability in &definition.abilities {
        let program = match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => &activated.effects,
            ironsmith::ability::AbilityKind::Triggered(triggered) => &triggered.effects,
            _ => continue,
        };
        for effect in program.all_effects() {
            collect(effect, &mut all);
        }
    }
    all.iter()
        .find_map(|effect| effect.downcast_ref::<MustAttackPlayerThisTurnEffect>().cloned())
        .expect("typed attack requirement")
}

#[test]
fn sirens_and_ravener_require_attacking_the_named_player() {
    for (name, text) in CARDS {
        for definition in definitions(name, text) {
            let effect = requirement(&definition);
            let player = format!("{:?}", effect.player);
            if name == "Alluring Siren" {
                assert!(player.contains("You"), "{player}");
                assert!(format!("{:?}", effect.target).contains("Opponent"));
            } else {
                assert!(player.contains("Opponent") && player.contains("Target"), "{player}");
            }
        }
    }
}

#[test]
fn requirement_is_recorded_for_this_turn_only() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = B;
    game.turn.priority_player = Some(B);
    game.turn.phase = Phase::FirstMain;
    let bear = compile_to_runtime_definition("Bear", "Type: Creature — Bear\nPower/Toughness: 2/2", false).unwrap();
    let source = game.create_object_from_definition(&bear, A, Zone::Battlefield);
    let attacker = game.create_object_from_definition(&bear, B, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    execute_effect(
        &mut game,
        &Effect::new(MustAttackPlayerThisTurnEffect::new(
            ChooseSpec::SpecificObject(attacker),
            ChooseSpec::Player(PlayerFilter::You),
        )),
        &mut EffectContext::new(source, A, &mut dm),
    )
    .unwrap();
    assert_eq!(game.required_attack_players_this_turn(attacker).collect::<Vec<_>>(), vec![A]);
    game.turn.turn_number += 1;
    assert!(game.required_attack_players_this_turn(attacker).next().is_none());
}

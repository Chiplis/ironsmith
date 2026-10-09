//! "Players skip their untap steps." (Stasis; CR 502, 614.10). New typed
//! static `PlayersSkipUntapStep`. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const STASIS: &str = "Mana cost: {1}{U}\nType: Enchantment\nPlayers skip their untap steps.\nAt the beginning of your upkeep, sacrifice this enchantment unless you pay {U}.";

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

#[test]
fn stasis_makes_every_player_skip_untap_steps_while_it_remains() {
    for definition in routes("Stasis", STASIS) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::PlayersSkipUntapStep)));
        assert!(definition
            .abilities
            .iter()
            .any(|ability| matches!(&ability.kind, AbilityKind::Triggered(_))));

        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let stasis = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bear = CardDefinitionBuilder::new(CardId::new(), "Tapped bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let mine = game.create_object_from_definition(&bear, A, Zone::Battlefield);
        let theirs = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        game.tap(mine);
        game.tap(theirs);
        assert!(game.player_skips_untap_step(A) && game.player_skips_untap_step(B));

        game.turn.active_player = A;
        ironsmith::turn::execute_untap_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.is_tapped(mine), "the untap step was skipped");

        game.move_object_by_effect(stasis, Zone::Graveyard).unwrap();
        assert!(!game.player_skips_untap_step(A));
        ironsmith::turn::execute_untap_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(!game.is_tapped(mine), "untaps normally once Stasis is gone");
        assert!(game.is_tapped(theirs), "only the active player's permanents untap");
    }
}

#[test]
fn skip_your_untap_step_applies_only_to_its_controller() {
    let text = "Mana cost: {2}\nType: Artifact\nSkip your untap step.";
    for definition in routes("Untap skip probe", text) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(game.player_skips_untap_step(A));
        assert!(!game.player_skips_untap_step(B));
    }
}

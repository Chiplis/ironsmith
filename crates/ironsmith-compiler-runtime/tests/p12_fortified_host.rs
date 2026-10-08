//! "Fortified land" names the Fortification's host (CR 301.6) and Fortify
//! is an attach-to-land activation (CR 702.67a). Collateral of the silent
//! "Fortified land has X" -> "All lands have X" miscompilation. Unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::object::AttachmentTarget;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const GARRISON: &str = "Mana cost: {2}\nType: Artifact — Fortification\nFortified land has indestructible.\nWhenever fortified land becomes tapped, target creature gets +1/+1 until end of turn.\nFortify {3} ({3}: Attach to target land you control. Fortify only as a sorcery.)";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

fn indestructible(game: &GameState, id: ObjectId) -> bool {
    game.calculated_characteristics(id)
        .is_some_and(|chars| chars.static_abilities.iter().any(|ability| ability.id() == StaticAbilityId::Indestructible))
}

#[test]
fn darksteel_garrison_grants_only_its_fortified_land() {
    for definition in definitions("Darksteel Garrison", GARRISON) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("\"fortified\""), "the host tag, not every land");
        let fortify = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(format!("{activated:?}")),
                _ => None,
            })
            .expect("Fortify activation");
        assert!(fortify.contains("SorcerySpeed") && fortify.contains("Land"), "{fortify}");

        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = Phase::FirstMain;
        let land = compile_to_runtime_definition("Plains", "Type: Basic Land — Plains", false).unwrap();
        let host = game.create_object_from_definition(&land, A, Zone::Battlefield);
        let other = game.create_object_from_definition(&land, A, Zone::Battlefield);
        let garrison = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(game.attach_object_to_target(garrison, AttachmentTarget::Object(host)));
        assert!(indestructible(&game, host));
        assert!(!indestructible(&game, other), "CR 301.6: only the fortified land");
    }
}

#[test]
fn camp_tapped_for_mana_trigger_still_fails_closed() {
    let text = "Mana cost: {2}\nType: Artifact — Fortification\nWhenever fortified land is tapped for mana, put a +1/+1 counter on target creature you control. If that creature shares a color with the mana that land produced, create a Junk token.\nFortify {3}";
    let direct = compile_to_runtime_definition("C.A.M.P.", text, false);
    if let Ok(definition) = direct {
        assert!(format!("{definition:?}").contains("\"fortified\""), "never all lands");
    }
}

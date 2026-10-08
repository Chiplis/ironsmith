//! cf8/p06 generic "instead" replacements and "this way" result predicates:
//! full frozen bodies on the direct and artifact routes, plus a gameplay
//! check of an instead-program reading the replaced event.
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext as ExecutionContext, execute_effect};
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/p06_predicate_fallbacks.json.fixture"))
        .unwrap()
}

fn text(row: &serde_json::Value) -> String {
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().to_string());
    lines.join("\n")
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let text = text(&row);
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, &text, false)
    });
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&direct));
    [direct, decoded]
}

fn assert_cluster(cases: &[(&str, &[&str])]) {
    for (name, fragments) in cases {
        for definition in definitions(name) {
            let debug = format!("{definition:?}");
            for fragment in *fragments {
                assert!(debug.contains(fragment), "{name}: missing `{fragment}`");
            }
        }
    }
}

#[test]
fn instead_programs_replace_damage_and_life_gain() {
    assert_cluster(&[
        ("Tainted Remedy", &["EventReplacementWithEffects", "LifeGain", "Opponent"]),
        ("Plague Drone", &["EventReplacementWithEffects", "LifeGain"]),
        ("Crumbling Sanctuary", &["EventReplacementWithEffects", "DamageToPlayer", "IteratedPlayer"]),
        ("Dralnu, Lich Lord", &["EventReplacementWithEffects", "DamageToObject"]),
        ("Lichenthrope", &["EventReplacementWithEffects", "DamageToObject", "MinusOneMinusOne"]),
        ("Szadek, Lord of Secrets", &["EventReplacementWithEffects", "combat_only: true"]),
        ("Undead Alchemist", &["EventReplacementWithEffects", "combat_only: true", "Zombie"]),
        ("Force Bubble", &["EventReplacementWithEffects", "DamageToPlayer"]),
    ]);
}

#[test]
fn prevention_follow_up_programs_keep_the_proposed_amount() {
    assert_cluster(&[
        ("Gloom Surgeon", &["PreventMatchingDamageWithFollowUp", "Proposed"]),
        ("Nine Lives", &["PreventMatchingDamageWithFollowUp"]),
    ]);
}

#[test]
fn this_way_results_read_the_producing_instruction() {
    assert_cluster(&[
        ("Long Rest", &["PriorEffectMetric", "Returned", "Fixed(8)"]),
        ("Flood of Tears", &["PriorEffectMetric", "Returned", "Fixed(4)"]),
        ("Vengeful Rebirth", &["PriorEffectMetric", "Returned"]),
        ("Transcendent Archaic", &["PriorEffectMetric", "Drawn"]),
        ("Mr. Foxglove", &["Not(", "PriorEffectMetric", "Drawn"]),
        ("Blitzwing, Cruel Tormentor // Blitzwing, Adaptive Assailant", &["PriorEffectMetric", "LifeLost"]),
        ("Rulik Mons, Warren Chief", &["Not(", "PlayerTaggedObjectMatches"]),
        ("Break Out", &["Not(", "PlayerTaggedObjectMatches"]),
    ]);
}

#[test]
fn history_and_source_link_readings() {
    assert_cluster(&[
        ("Case of the Gateway Express", &["CreaturesAttackedWith", "Fixed(3)"]),
        ("Smirking Spelljacker", &["__source_exiled__"]),
        ("Archangel of Wrath", &["KickCount", "Fixed(2)"]),
    ]);
}

#[test]
fn modification_and_choice_bodies_stay_with_their_own_readers() {
    for text in [
        // Amount modifications are not replacement programs.
        "If a source would deal damage to you, it deals that much damage plus 1 to you instead.",
        // Optional replacements need their own chooser semantics.
        "If you would gain life, you may draw that many cards instead.",
        // Two "instead" markers are malformed.
        "If an opponent would gain life, instead that player loses that much life instead.",
    ] {
        let source = format!("Type: Enchantment\n{text}");
        let direct = compile_to_runtime_definition("Neighbor", &source, false);
        assert!(
            direct.map_or(true, |definition| !format!("{definition:?}").contains("EventReplacementWithEffects")),
            "{text}"
        );
    }
}

#[test]
fn tainted_remedy_turns_an_opponents_life_gain_into_life_loss() {
    for definition in definitions("Tainted Remedy") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let gainer = game.create_object_from_definition(&definition, B, Zone::Hand);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(gainer, B, &mut dm);
        execute_effect(&mut game, &Effect::gain_life(3), &mut ctx).unwrap();
        assert_eq!(game.player(B).unwrap().life, 17, "B loses that much life instead (CR 614.1a)");
        assert_eq!(game.player(A).unwrap().life, 20);
        // The controller's own gain is not an opponent's and is unaffected.
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut ctx = ExecutionContext::new(source, A, &mut dm);
        execute_effect(&mut game, &Effect::gain_life(2), &mut ctx).unwrap();
        assert_eq!(game.player(A).unwrap().life, 22);
    }
}

#[test]
fn would_die_instead_programs_move_the_dying_object() {
    assert_cluster(&[
        ("Gravebane Zombie", &["EventReplacementWithEffects", "ZoneChange", "Some(Battlefield)", "Some(Graveyard)"]),
        ("Nissa's Chosen", &["EventReplacementWithEffects", "ZoneChange"]),
        ("Necromancer's Magemark", &["EventReplacementWithEffects", "ZoneChange"]),
        ("Wumpus Aberration", &["Not("]),
    ]);
}

#[test]
fn gravebane_zombie_goes_to_its_owners_library_instead_of_dying() {
    for definition in definitions("Gravebane Zombie") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let zombie = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let library_before = game.player(A).unwrap().library.len();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(zombie, B, &mut dm);
        execute_effect(
            &mut game,
            &Effect::destroy(ironsmith::target::ChooseSpec::SpecificObject(zombie)),
            &mut ctx,
        )
        .unwrap();
        // CR 614.6: the creature never dies; it is put into its owner's
        // library instead.
        assert!(game.player(A).unwrap().graveyard.is_empty());
        assert_eq!(game.player(A).unwrap().library.len(), library_before + 1);
    }
}

//! cf8/p06 predicate readings: full frozen bodies on the direct and artifact
//! routes, plus the runtime gate the prepared designation adds.
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::condition_eval::{ExternalEvaluationContext, evaluate_condition_external};
use ironsmith::effect::Condition;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);

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

/// Every card of a cluster compiles strictly on both routes, and each
/// definition's lowered structure carries every expected typed fragment.
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
fn negated_copulas_deny_the_positive_reading() {
    assert_cluster(&[
        ("Twinned Vision", &["Not(", "ThisSpellWasCastFromZone(Hand)"]),
        ("Sphinx of Lost Truths", &["Not(", "Kicked"]),
        ("Court of Vantress", &["Not(", "PlayerIsMonarch"]),
        ("Dose of Dawnglow", &["Not("]),
        ("Luminarch Ascension", &["Not("]),
    ]);
}

#[test]
fn this_spell_cast_from_your_hand_is_the_hand_origin() {
    assert_cluster(&[
        ("Apex of Power", &["ThisSpellWasCastFromZone(Hand)"]),
        ("Transpose", &["ThisSpellWasCastFromZone(Hand)"]),
    ]);
}

#[test]
fn zone_quantity_thresholds_are_typed_counts() {
    assert_cluster(&[
        ("Visions of Beyond", &["CountPlayersWithCardsInGraveyardAtLeast(Any, 20)"]),
        ("Jace, the Perfected Mind", &["CountPlayersWithCardsInGraveyardAtLeast(Any, 20)"]),
        ("Nightmares and Daydreams", &["CountPlayersWithCardsInGraveyardAtLeast(Any, 20)"]),
        ("Sanguine Spy", &["DistinctManaValues("]),
        ("Tainted Indulgence", &["DistinctManaValues("]),
        ("Negative Zone Portal", &["__source_exiled__", "Fixed(4)"]),
        ("Profane Procession // Tomb of the Dusk Rose", &["__source_exiled__", "Fixed(3)"]),
    ]);
}

#[test]
fn player_turn_facts_use_existing_conditions() {
    assert_cluster(&[
        ("Timely Reinforcements", &["PlayerHasMoreLifeThanYou"]),
        ("Servant of the Stinger", &["PlayerCommittedCrimeThisTurn"]),
        ("Oko, the Ringleader", &["PlayerCommittedCrimeThisTurn"]),
        ("The Raven Man", &["CardsDiscardedThisTurn(Any)"]),
        ("River of Tears", &["PlayerPlayedLandThisTurn"]),
        ("Kiora of Salt and Sand", &["PlayerActivatedLoyaltyAbilityThisTurn"]),
        ("Lunar Convocation", &["And("]),
    ]);
}

#[test]
fn object_state_predicates_read_source_and_referents() {
    assert_cluster(&[
        ("Polis Crusher", &["SourceIsMonstrous"]),
        ("Arachnus Web", &["Fixed(4)"]),
        ("Domestication", &["Fixed(4)"]),
        ("Anax, Hardened in the Forge", &["Fixed(4)"]),
        ("Burn the Impure", &["Infect"]),
        ("Hotshot Investigators", &["MatchedLastKnown"]),
        ("Unyielding Gatekeeper", &["MatchedLastKnown"]),
        ("Gleeful Demolition", &["MatchedLastKnown"]),
        ("Paradox Shaper // Omit Variables", &["Not(", "SourceIsPrepared"]),
        ("Stingerquill Voxmancer // Vicious Verse", &["Not(", "SourceIsPrepared"]),
        ("Woodwork Prodigy // Soul Tether", &["Not(", "SourceIsPrepared"]),
    ]);
}

#[test]
fn die_results_at_most_a_bound_compare_the_completed_roll() {
    assert_cluster(&[
        ("Dissatisfied Customer", &["Rolled", "LessThanOrEqual", "Fixed(3)"]),
        ("Non-Human Cannonball", &["Rolled", "LessThanOrEqual", "Fixed(4)"]),
    ]);
}

#[test]
fn another_creature_would_die_excludes_the_source() {
    assert_cluster(&[("Void Maw", &["other: true"])]);
}

#[test]
fn unsupported_neighbors_still_fail_closed() {
    for text in [
        // Negation inside a noun phrase is not the clause's main verb.
        "If a creature that isn't a Wolf would die, draw a card.",
        // An unknown positive reading stays unknown when negated.
        "Draw a card. If this spell wasn't frobnicated, draw two cards instead.",
        "Draw a card. If a graveyard has twenty or more frobs in it, draw three cards instead.",
    ] {
        let source = format!("Mana cost: {{U}}\nType: Instant\n{text}");
        assert!(compile_to_artifact("Neighbor", &source, false).is_err(), "{text}");
        assert!(compile_to_runtime_definition("Neighbor", &source, false).is_err(), "{text}");
    }
}

#[test]
fn prepared_condition_reads_the_live_designation() {
    let [definition, _] = definitions("Paradox Shaper // Omit Variables");
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let id = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    let ctx = ExternalEvaluationContext {
        controller: A,
        source: id,
        defending_player: None,
        attacking_player: None,
        filter_source: None,
        iterated_player: None,
        triggering_event: None,
        trigger_identity: None,
        ability_index: None,
        options: Default::default(),
    };
    let prepared = Condition::SourceIsPrepared;
    let not_prepared = Condition::Not(Box::new(Condition::SourceIsPrepared));
    assert!(!evaluate_condition_external(&game, &prepared, &ctx));
    assert!(evaluate_condition_external(&game, &not_prepared, &ctx));
    game.set_prepared(id);
    assert!(evaluate_condition_external(&game, &prepared, &ctx));
    assert!(!evaluate_condition_external(&game, &not_prepared, &ctx));
}

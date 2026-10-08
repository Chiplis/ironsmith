//! Source-authored player-counter anthem expectations. UNVALIDATED / UNRUN.
//! Full-card compilation is deliberately required; no isolated-line substitute.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::{CardType, CounterType, GameState, ObjectId, PlayerFilter, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{AnthemCountExpression, AnthemValue, StaticAbilityPayload};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixture(name: &str) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/player_counter_anthems.json.fixture"
    )).unwrap();
    rows.into_iter().find(|row| row["name"] == name).unwrap()
}

fn source(row: &serde_json::Value, text: &str) -> String {
    format!(
        "Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{text}",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(),
        row["toughness"].as_str().unwrap(),
    )
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixture(name);
    let text = source(&row, row["oracle_text"].as_str().unwrap());
    let (direct, loss) = parse_loss::capture(|| {
        compile_to_runtime_definition(name, &text, false)
    });
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    // The artifact compiler's companion is itself artifact-materialized;
    // independent direct compilation above must remain a separate route.
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(artifact, decoded);
    let restored =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    [direct, restored]
}

fn assert_anthem(
    definition: &CardDefinition,
    player: PlayerFilter,
    counter_type: CounterType,
    toughness: i32,
    global: bool,
) {
    let anthems: Vec<_> = definition.abilities.iter().filter_map(|ability| {
        let AbilityKind::Static(ability) = &ability.kind else { return None; };
        let model = ability.compiled_model()?;
        let StaticAbilityPayload::Anthem(anthem) = &model.payload else { return None; };
        Some(anthem.clone())
    }).collect();
    let [anthem] = anthems.as_slice() else {
        panic!("expected exactly one independently typed anthem: {anthems:#?}");
    };
    let count = AnthemCountExpression::PlayerCounters(player, counter_type);
    assert_eq!(anthem.power, AnthemValue::scaled(1, count.clone()));
    assert_eq!(anthem.toughness, AnthemValue::scaled(toughness, count));
    assert!(anthem.condition.is_none());
    assert!(!anthem.count_uses_where_x);
    if global {
        let filter = anthem.filter.as_ref().expect("Minthara has a recipient filter");
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.card_types, vec![CardType::Creature]);
    } else {
        assert!(anthem.filter.is_none(), "self scaling must not affect other creatures");
    }
}

fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20)
}

fn add(game: &mut GameState, player: PlayerId, kind: CounterType, amount: u32) {
    game.add_player_counters_with_source(player, kind, amount, None, None).unwrap();
}

fn pt(game: &GameState, id: ObjectId) -> (i32, i32) {
    (game.current_power(id).unwrap(), game.current_toughness(id).unwrap())
}

fn vanilla(name: &str) -> CardDefinition {
    compile_to_runtime_definition(name, "Type: Creature — Human\nPower/Toughness: 1/1", false).unwrap()
}

#[test]
fn all_five_exact_cards_preserve_typed_counts_metadata_artifacts_and_rendering() {
    for (name, player, kind, toughness, global) in [
        ("Kalemne, Disciple of Iroas", PlayerFilter::You, CounterType::Experience, 1, false),
        ("Kelsien, the Plague", PlayerFilter::You, CounterType::Experience, 1, false),
        ("Minthara, Merciless Soul", PlayerFilter::You, CounterType::Experience, 0, true),
        ("Mycosynth Fiend", PlayerFilter::Opponent, CounterType::Poison, 1, false),
        ("Vishgraz, the Doomhive", PlayerFilter::Opponent, CounterType::Poison, 1, false),
    ] {
        let row = fixture(name);
        for definition in definitions(name) {
            assert_anthem(&definition, player.clone(), kind, toughness, global);
            assert_eq!(definition.card.mana_cost.as_ref().unwrap().to_oracle(), row["mana_cost"].as_str().unwrap());
            let stats = definition.card.power_toughness.unwrap();
            assert_eq!(stats.power.to_string(), row["power"].as_str().unwrap());
            assert_eq!(stats.toughness.to_string(), row["toughness"].as_str().unwrap());
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            let expected = if kind == CounterType::Poison {
                "+1/+1 for each poison counter your opponents have"
            } else if global {
                "+1/+0 for each experience counter you have"
            } else {
                "+1/+1 for each experience counter you have"
            };
            assert!(rendered.contains(expected), "{name}: {rendered}");
            let (reparsed, loss) = parse_loss::capture(|| {
                compile_to_runtime_definition(name, source(&row, &rendered), false)
            });
            let reparsed = reparsed.unwrap_or_else(|error| panic!("{name}: {error}\n{rendered}"));
            assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
            assert_anthem(&reparsed, player.clone(), kind, toughness, global);
        }
    }
}

#[test]
fn kalemne_and_kelsien_follow_live_experience_and_new_controller() {
    for (name, base) in [("Kalemne, Disciple of Iroas", 3), ("Kelsien, the Plague", 2)] {
        for definition in definitions(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let peer = game.create_object_from_definition(&vanilla("Peer"), A, Zone::Battlefield);
            assert_eq!(pt(&game, host), (base, base));
            add(&mut game, A, CounterType::Experience, 2);
            add(&mut game, A, CounterType::Poison, 4);
            add(&mut game, B, CounterType::Experience, 5);
            assert_eq!(pt(&game, host), (base + 2, base + 2));
            assert_eq!(pt(&game, peer), (1, 1));
            assert_eq!(game.remove_player_counters_with_source(A, CounterType::Experience, 1, None, None).unwrap().0, 1);
            assert_eq!(pt(&game, host), (base + 1, base + 1));
            game.set_current_controller(host, B).unwrap();
            assert_eq!(pt(&game, host), (base + 5, base + 5));
            add(&mut game, A, CounterType::Experience, 3);
            assert_eq!(pt(&game, host), (base + 5, base + 5));
        }
    }
}

#[test]
fn minthara_only_buffs_currently_controlled_creatures_and_only_their_power() {
    for definition in definitions("Minthara, Merciless Soul") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ally = game.create_object_from_definition(&vanilla("Ally"), A, Zone::Battlefield);
        let opponent = game.create_object_from_definition(&vanilla("Opponent"), B, Zone::Battlefield);
        add(&mut game, A, CounterType::Experience, 3);
        add(&mut game, B, CounterType::Experience, 5);
        assert_eq!(pt(&game, host), (5, 2));
        assert_eq!(pt(&game, ally), (4, 1));
        assert_eq!(pt(&game, opponent), (1, 1));
        game.set_current_controller(host, B).unwrap();
        assert_eq!(pt(&game, host), (7, 2));
        assert_eq!(pt(&game, ally), (1, 1));
        assert_eq!(pt(&game, opponent), (6, 1));
    }
}

#[test]
fn fiend_and_vishgraz_sum_opponents_poison_and_rebind_after_control_change() {
    for (name, base) in [("Mycosynth Fiend", 2), ("Vishgraz, the Doomhive", 3)] {
        for definition in definitions(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let peer = game.create_object_from_definition(&vanilla("Peer"), A, Zone::Battlefield);
            assert_eq!(pt(&game, host), (base, base));
            add(&mut game, A, CounterType::Poison, 7);
            add(&mut game, B, CounterType::Poison, 2);
            add(&mut game, C, CounterType::Poison, 4);
            add(&mut game, B, CounterType::Experience, 8);
            assert_eq!(pt(&game, host), (base + 6, base + 6));
            assert_eq!(pt(&game, peer), (1, 1));
            assert_eq!(game.remove_player_counters_with_source(C, CounterType::Poison, 1, None, None).unwrap().0, 1);
            assert_eq!(pt(&game, host), (base + 5, base + 5));
            game.set_current_controller(host, B).unwrap();
            assert_eq!(pt(&game, host), (base + 10, base + 10));
            add(&mut game, B, CounterType::Poison, 1);
            assert_eq!(pt(&game, host), (base + 10, base + 10));
        }
    }
}

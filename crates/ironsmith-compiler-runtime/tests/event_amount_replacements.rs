//! cf8/p06 amount-modifying replacements (CR 614.1a, 616.1): mill, scry and
//! surveil numbers, energy additions, and damage set/halve forms, plus the
//! scry instead-program. Full frozen bodies on the direct and artifact
//! routes, and gameplay checks that the modified amount is what happens.
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext as ExecutionContext, execute_effect};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/p06_round4.json.fixture")).unwrap()
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

fn add_library_cards(game: &mut GameState, owner: PlayerId, count: usize) -> Vec<ObjectId> {
    (0..count)
        .map(|index| {
            let definition = CardDefinitionBuilder::new(CardId::new(), format!("Filler {index}"))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_definition(&definition, owner, Zone::Library)
        })
        .collect()
}

#[test]
fn keyword_action_numbers_are_modified_not_replaced() {
    assert_cluster(&[
        (
            "Bruvac the Grandiloquent",
            &["EventAmountReplacement", "Mill", "Opponent", "Multiply(2)"],
        ),
        (
            "Kenessos, Priest of Thassa",
            &["EventAmountReplacement", "Scry", "Add(1)"],
        ),
        (
            "Enhanced Surveillance",
            &["EventAmountReplacement", "Surveil", "Add(2)", "optional: true"],
        ),
    ]);
}

#[test]
fn scry_instead_program_draws_that_many() {
    assert_cluster(&[(
        "Eligeth, Crossroads Augur",
        &["KeywordActionReplacement", "Scry", "EventValue(Amount)"],
    )]);
}

#[test]
fn damage_amounts_are_set_or_halved_with_their_gates() {
    assert_cluster(&[
        (
            "Divine Presence",
            &["EventAmountReplacement", "minimum: Some(4)", "SetTo(3)"],
        ),
        (
            "Forethought Amulet",
            &["EventAmountReplacement", "minimum: Some(3)", "SetTo(2)", "Instant", "Sorcery"],
        ),
        (
            "Ghosts of the Innocent",
            &["EventAmountReplacement", "Half { round_up: false }"],
        ),
    ]);
}

#[test]
fn energy_additions_use_the_player_counter_replacement() {
    assert_cluster(&[(
        "Izzet Generatorium",
        &["AddCountersPlacementReplacement", "Energy", "additional: 1"],
    )]);
}

#[test]
fn bruvac_doubles_an_opponents_mill() {
    for definition in definitions("Bruvac the Grandiloquent") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        add_library_cards(&mut game, B, 10);
        add_library_cards(&mut game, A, 10);
        let source = game.create_object_from_definition(&definition, B, Zone::Hand);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, B, &mut dm);
        execute_effect(&mut game, &Effect::mill(3), &mut ctx).unwrap();
        // CR 616.1: the opponent's mill 3 becomes mill 6.
        assert_eq!(game.player(B).unwrap().graveyard.len(), 6);
        // Bruvac's controller is not an opponent; their mill is unchanged.
        let mut ctx = ExecutionContext::new(source, A, &mut dm);
        execute_effect(&mut game, &Effect::mill(3), &mut ctx).unwrap();
        assert_eq!(game.player(A).unwrap().graveyard.len(), 3);
    }
}

#[test]
fn kenessos_scries_one_more() {
    for definition in definitions("Kenessos, Priest of Thassa") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        add_library_cards(&mut game, A, 5);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, A, &mut dm);
        let outcome = execute_effect(&mut game, &Effect::scry(2), &mut ctx).unwrap();
        assert_eq!(outcome.count_or_zero(), 3, "scry 2 becomes scry 3");
    }
}

#[test]
fn eligeth_draws_instead_of_scrying() {
    for definition in definitions("Eligeth, Crossroads Augur") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        add_library_cards(&mut game, A, 5);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let hand_before = game.player(A).unwrap().hand.len();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, A, &mut dm);
        execute_effect(&mut game, &Effect::scry(2), &mut ctx).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), hand_before + 2);
        assert_eq!(game.player(A).unwrap().library.len(), 3);
    }
}

#[test]
fn divine_presence_caps_large_damage_at_three() {
    for definition in definitions("Divine Presence") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, A, &mut dm);
        let to_b = || ChooseSpec::Player(PlayerFilter::Specific(B));
        execute_effect(&mut game, &Effect::deal_damage(5, to_b()), &mut ctx).unwrap();
        assert_eq!(game.player(B).unwrap().life, 17, "5 damage becomes 3");
        execute_effect(&mut game, &Effect::deal_damage(2, to_b()), &mut ctx).unwrap();
        assert_eq!(game.player(B).unwrap().life, 15, "2 damage is below the gate");
    }
}

#[test]
fn ghosts_of_the_innocent_halves_damage_rounded_down() {
    for definition in definitions("Ghosts of the Innocent") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, A, &mut dm);
        execute_effect(
            &mut game,
            &Effect::deal_damage(5, ChooseSpec::Player(PlayerFilter::Specific(B))),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(game.player(B).unwrap().life, 18, "half of 5, rounded down");
    }
}

#[test]
fn izzet_generatorium_adds_one_energy() {
    for definition in definitions("Izzet Generatorium") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, A, &mut dm);
        execute_effect(&mut game, &Effect::energy_counters(2), &mut ctx).unwrap();
        assert_eq!(game.player(A).unwrap().energy_counters, 3);
    }
}

#[test]
fn until_your_next_turn_multipliers_fix_their_referents() {
    assert_cluster(&[
        (
            "Lightning, Army of One",
            &["RegisterDamageMultiplierEffect", "UntilYourNextTurn", "factor: 2"],
        ),
        (
            "Jeska, Thrice Reborn",
            &["RegisterDamageMultiplierEffect", "UntilYourNextTurn", "factor: 3", "combat_only: true"],
        ),
    ]);
}

#[test]
fn this_turn_multipliers_read_sources_you_control_and_late_recipients() {
    assert_cluster(&[(
        "Isengard Unleashed",
        &["RegisterDamageMultiplierEffect", "UntilEndOfTurn", "factor: 3", "Opponent"],
    )]);
}

#[test]
fn equal_treatment_registers_a_set_amount_for_the_turn() {
    assert_cluster(&[(
        "Equal Treatment",
        &[
            "RegisterDamageMultiplierEffect",
            "UntilEndOfTurn",
            "amount_override: Some(SetTo(2))",
            "minimum: Some(1)",
        ],
    )]);
}

#[test]
fn alms_collector_replaces_a_whole_multi_card_draw() {
    for definition in definitions("Alms Collector") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("DrawInstruction"), "{debug}");
        assert!(debug.contains("minimum: 2"), "{debug}");
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        add_library_cards(&mut game, A, 5);
        add_library_cards(&mut game, B, 5);
        let source = game.create_object_from_definition(&definition, B, Zone::Hand);
        let (a_hand, b_hand) = (
            game.player(A).unwrap().hand.len(),
            game.player(B).unwrap().hand.len(),
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, B, &mut dm);
        execute_effect(&mut game, &Effect::draw(3), &mut ctx).unwrap();
        // CR 614.1a: instead of drawing three, each of them draws one.
        assert_eq!(game.player(A).unwrap().hand.len(), a_hand + 1);
        assert_eq!(game.player(B).unwrap().hand.len(), b_hand + 1);
        // A single draw is not "two or more cards".
        let mut ctx = ExecutionContext::new(source, B, &mut dm);
        execute_effect(&mut game, &Effect::draw(1), &mut ctx).unwrap();
        assert_eq!(game.player(B).unwrap().hand.len(), b_hand + 2);
    }
}

#[test]
fn twinning_staff_adds_one_spell_copy() {
    assert_cluster(&[(
        "Twinning Staff",
        &["EventAmountReplacement", "CopySpell", "Add(1)", "CopySpellEffect"],
    )]);
}

#[test]
fn worship_floors_damage_life_loss_while_you_control_a_creature() {
    assert_cluster(&[("Worship", &["DamageReduceLifeBelowOne", "Creature"])]);
}

#[test]
fn overblaze_doubles_its_targets_damage_this_turn() {
    assert_cluster(&[(
        "Overblaze",
        &["TargetOnlyEffect", "RegisterDamageMultiplierEffect", "UntilEndOfTurn", "factor: 2"],
    )]);
}

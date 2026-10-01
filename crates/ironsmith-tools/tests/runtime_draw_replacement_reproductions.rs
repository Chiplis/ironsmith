//! Opt-in legal-cast probes for draw/replacement chains. Rust success only means report emission.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::DecisionContext;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;

fn hash(path: &std::path::Path) -> Value {
    let mut file = std::fs::File::open(path).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    let sha256: String = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    json!({"path": path.display().to_string(), "sha256": sha256})
}

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice();
    game.turn.priority_player = Some(alice());
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    for color in [ManaSymbol::Black, ManaSymbol::Blue, ManaSymbol::Colorless] {
        game.player_mut(alice()).unwrap().mana_pool.add(color, 30);
    }
    game
}

fn cast_or_activate(
    game: &mut GameState,
    id: ObjectId,
    cast: bool,
) -> Result<(ObjectId, u32), String> {
    let actor = game.object(id).ok_or("action source missing")?.owner;
    let stable = game.object(id).ok_or("action source missing")?.stable_id;
    game.turn.priority_player = Some(actor);
    let action = compute_legal_actions(game, actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|action| match action {
            LegalAction::CastSpell { spell_id, .. } => cast && *spell_id == id,
            LegalAction::ActivateAbility { source, .. } => !cast && *source == id,
            _ => false,
        })
        .ok_or("fixture has no matching legal action")?;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .map_err(|error| error.to_string())?;
    for _ in 0..24 {
        if let Some(result) = game
            .stack
            .iter()
            .find(|entry| {
                game.object(entry.object_id)
                    .is_some_and(|object| object.stable_id == stable)
            })
            .map(|entry| (entry.object_id, entry.mana_spent_on_activation.total()))
        {
            drain_pending_trigger_events(game, &mut queue);
            put_triggers_on_stack_with_dm(game, &mut queue, &mut dm)
                .map_err(|error| error.to_string())?;
            return Ok(result);
        }
        progress = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref context))
                if context.description.starts_with("Choose optional costs") =>
            {
                apply_priority_response_with_dm(
                    game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::OptionalCosts(vec![]),
                    &mut dm,
                )
                .map_err(|error| error.to_string())?
            }
            GameProgress::NeedsDecisionCtx(ref context)
                if !matches!(context, DecisionContext::Priority(_)) =>
            {
                apply_decision_context_with_dm(game, &mut queue, &mut state, context, &mut dm)
                    .map_err(|error| error.to_string())?
            }
            ref other => return Err(format!("fixture action did not reach the stack: {other:?}")),
        };
    }
    Err("fixture exceeded action decision bound".into())
}

fn resolve_all(game: &mut GameState) -> Result<usize, String> {
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut resolved = 0;
    for _ in 0..24 {
        drain_pending_trigger_events(game, &mut queue);
        put_triggers_on_stack_with_dm(game, &mut queue, &mut dm)
            .map_err(|error| error.to_string())?;
        if game.stack.is_empty() {
            return Ok(resolved);
        }
        resolve_stack_entry_with(game, &mut dm).map_err(|error| error.to_string())?;
        resolved += 1;
    }
    Err("fixture exceeded resolution bound".into())
}

fn bob() -> PlayerId {
    PlayerId::from_index(1)
}

fn filler(card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(
        CardId::new(),
        format!("Simultaneous audit filler {card_type:?}"),
    )
    .card_types(vec![card_type])
    .build()
}

fn count_named(game: &GameState, name: &str, zone: Zone, owner: PlayerId) -> usize {
    game.objects_in_deterministic_order()
        .into_iter()
        .filter(|object| object.name == name && object.zone == zone && object.owner == owner)
        .count()
}

fn cast_paid(game: &mut GameState, definition: &CardDefinition, mana: u32) -> Result<(), String> {
    let id = game.create_object_from_definition(definition, alice(), Zone::Hand);
    let before = game.player(alice()).unwrap().mana_pool.total();
    cast_or_activate(game, id, true)?;
    let paid = before - game.player(alice()).unwrap().mana_pool.total();
    if paid != mana {
        return Err(format!("fixture cast paid {paid}, expected {mana}"));
    }
    Ok(())
}

fn draw_scenario(
    definitions: &HashMap<String, CardDefinition>,
    spell: &str,
    replacement: Option<&str>,
    starting_hand: usize,
) -> Result<Value, String> {
    let mut game = setup();
    let land = filler(CardType::Land);
    for player in [alice(), bob()] {
        for _ in 0..20 {
            game.create_object_from_definition(&land, player, Zone::Library);
        }
    }
    for _ in 0..starting_hand {
        game.create_object_from_definition(&land, alice(), Zone::Hand);
    }
    if let Some(name) = replacement {
        let owner = if name == "Notion Thief" {
            bob()
        } else {
            alice()
        };
        game.turn.active_player = owner;
        game.turn.priority_player = Some(owner);
        if owner == bob() {
            for color in [ManaSymbol::Black, ManaSymbol::Blue, ManaSymbol::Colorless] {
                game.player_mut(owner).unwrap().mana_pool.add(color, 30);
            }
        }
        let id = game.create_object_from_definition(&definitions[name], owner, Zone::Hand);
        let before = game.player(owner).unwrap().mana_pool.total();
        cast_or_activate(&mut game, id, true)?;
        let paid = before - game.player(owner).unwrap().mana_pool.total();
        let expected = match name {
            "Thought Reflection" => 7,
            "Notion Thief" => 4,
            "Blood Scrivener" => 2,
            "Asmodeus the Archfiend" => 6,
            _ => unreachable!(),
        };
        if paid != expected {
            return Err(format!(
                "replacement permanent {name} paid {paid}, expected {expected}"
            ));
        }
        resolve_all(&mut game)?;
        if count_named(&game, name, Zone::Battlefield, owner) != 1 {
            return Err(format!("replacement permanent {name} did not enter"));
        }
        game.turn.active_player = alice();
        game.turn.priority_player = Some(alice());
    }
    cast_paid(
        &mut game,
        &definitions[spell],
        if spell == "Ancient Excavation" { 4 } else { 3 },
    )?;
    let error = resolve_all(&mut game).err();
    Ok(json!({"resolution_error": error,
        "hands": game.players.iter().map(|player| player.hand.len()).collect::<Vec<_>>(),
        "libraries": game.players.iter().map(|player| player.library.len()).collect::<Vec<_>>(),
        "alice_life": game.player(alice()).unwrap().life,
        "alice_exiled": game.objects_in_deterministic_order().into_iter().filter(|object|object.zone==Zone::Exile&&object.owner==alice()).count(),
        "discarded_filler":count_named(&game, land.name(), Zone::Graveyard, alice())}))
}

#[test]
#[ignore = "manual expected-result audit; inspect JSON row statuses"]
fn report_draw_replacement_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_draw_replacement_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|path| hash(path)).collect();
    let names = [
        "Ancient Excavation",
        "Thought Reflection",
        "Divination",
        "Blood Scrivener",
        "Asmodeus the Archfiend",
        "Notion Thief",
    ]
    .map(str::to_owned)
    .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut definitions = HashMap::new();
    let mut compilation = Vec::new();
    for payload in payloads.into_values().flatten() {
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            payload.parse_name.as_deref().unwrap_or(&payload.name),
        );
        match ironsmith_registry::compile_builder_to_artifact(builder,&payload.parse_input,false) {
            Ok((artifact,definition))=>{
                compilation.push(json!({"card":payload.name,"status":"strict_artifact_materialized","artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
                definitions.insert(definition.name().to_owned(),definition);
            },
            Err(error)=>compilation.push(json!({"card":payload.name,"status":"compile_or_materialization_failed","error":error.to_string()})),
        }
    }
    let mut rows = Vec::new();
    for (
        spell,
        replacement,
        hand,
        alice_hand,
        bob_hand,
        alice_library,
        bob_library,
        life,
        exiled,
        discarded,
    ) in [
        ("Divination", None, 0, 2, 0, 18, 20, 20, 0, 0),
        (
            "Divination",
            Some("Thought Reflection"),
            0,
            4,
            0,
            16,
            20,
            20,
            0,
            0,
        ),
        (
            "Divination",
            Some("Notion Thief"),
            0,
            0,
            2,
            20,
            18,
            20,
            0,
            0,
        ),
        ("Ancient Excavation", None, 0, 0, 0, 20, 20, 20, 0, 0),
        ("Ancient Excavation", None, 1, 1, 0, 19, 20, 20, 0, 1),
        ("Ancient Excavation", None, 3, 3, 0, 17, 20, 20, 0, 3),
        (
            "Ancient Excavation",
            Some("Thought Reflection"),
            1,
            1,
            0,
            18,
            20,
            20,
            0,
            2,
        ),
        (
            "Ancient Excavation",
            Some("Notion Thief"),
            1,
            1,
            1,
            20,
            19,
            20,
            0,
            0,
        ),
        (
            "Divination",
            Some("Blood Scrivener"),
            0,
            3,
            0,
            17,
            20,
            19,
            0,
            0,
        ),
        (
            "Divination",
            Some("Blood Scrivener"),
            1,
            3,
            0,
            18,
            20,
            20,
            0,
            0,
        ),
        (
            "Divination",
            Some("Asmodeus the Archfiend"),
            0,
            0,
            0,
            18,
            20,
            20,
            2,
            0,
        ),
    ] {
        let expected = json!({"resolution_error":null,"hands":[alice_hand,bob_hand],"libraries":[alice_library,bob_library],"alice_life":life,"alice_exiled":exiled,"discarded_filler":discarded});
        let result = if !definitions.contains_key(spell)
            || replacement.is_some_and(|name| !definitions.contains_key(name))
        {
            Err("required canonical definition did not strictly compile/materialize".to_string())
        } else {
            draw_scenario(&definitions, spell, replacement, hand)
        };
        let (status, actual) = match result {
            Ok(actual) if actual == expected => ("passed", actual),
            Ok(actual) if !actual["resolution_error"].is_null() => {
                ("confirmed_resolution_failure", actual)
            }
            Ok(actual) => ("semantic_mismatch", actual),
            Err(error) => ("execution_or_fixture_error", json!({"error":error})),
        };
        rows.push(json!({"card":spell,"related_card":replacement,"scenario":{"starting_hand_after_cast":hand},"status":status,"expected":expected,"actual":actual}));
    }
    let after: Vec<_> = paths.iter().map(|path| hash(path)).collect();
    let report = json!({"scope":"Canonical payloads compiled through strict artifact encoding/materialization without oracle-only fallback; replacement permanents and draw spells legally cast with verified paid mana and normal trigger/resolution pipeline. Draw/discard/library/life outcomes measured without state repairs.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    let output = root.join("reports/runtime-audit/draw-replacement-execution.json");
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    std::fs::write(
        &output,
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!("{}", serde_json::to_string(&report).unwrap());
}

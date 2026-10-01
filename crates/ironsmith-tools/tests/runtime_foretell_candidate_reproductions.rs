//! Opt-in canonical runtime evidence. Passing means reports were generated.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, PartitionContext, SelectObjectsContext, SelectOptionsContext, ViewCardsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, Phase, PlayerId, PowerToughness, Step,
    Subtype, Zone,
};
use serde_json::{Value, json};

use std::collections::HashMap;

const SEED: u64 = 0x49524f4e534d4954;
fn alice() -> PlayerId {
    PlayerId(0)
}
fn bob() -> PlayerId {
    PlayerId(1)
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.set_random_seed(SEED);
    game.turn.turn_number = 3;
    game.turn.active_player = alice();
    game.turn.priority_player = Some(alice());
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    for player in [alice(), bob()] {
        for symbol in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 12);
        }
    }
    game
}
fn creature(name: &str, blue: bool, flying: bool) -> CardDefinition {
    let mut b = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 6));
    if blue {
        b = b.color_indicator(ironsmith::color::ColorSet::BLUE);
    }
    if flying {
        b = b.flying();
    }
    b.build()
}
struct Choices {
    target: Option<ObjectId>,
    accept: bool,
    trace: Vec<Value>,
    planar_viewed: usize,
    mode: Option<usize>,
}
impl DecisionMaker for Choices {
    fn decide_colors(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::ColorsContext,
    ) -> Vec<ironsmith::color::Color> {
        let color = if self.mode == Some(1) {
            ironsmith::color::Color::Green
        } else {
            ironsmith::color::Color::Black
        };
        self.trace.push(json!({"choice":"mana_color","context":format!("{ctx:?}"),"selected":format!("{color:?}")}));
        vec![color; ctx.count as usize]
    }

    fn decide_targets(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        let selected = self
            .target
            .map(ironsmith::game_state::Target::Object)
            .into_iter()
            .filter(|target| {
                ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target))
            })
            .collect::<Vec<_>>();
        self.trace.push(json!({"choice":"targets","context":format!("{ctx:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"answer":self.accept}));
        self.accept
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = if let Some(x) = c
            .candidates
            .iter()
            .find(|x| x.legal && x.name == "Lightning Bolt")
        {
            vec![x.id]
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = if c.description.to_ascii_lowercase().contains("mode") {
            self.mode
                .map(|m| vec![m])
                .unwrap_or_else(|| SelectFirstDecisionMaker.decide_options(g, c))
        } else {
            SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_partition(&mut self, _: &GameState, c: &PartitionContext) -> Vec<ObjectId> {
        self.trace
            .push(json!({"choice":"surveil_keep_all","context":format!("{c:?}")}));
        Vec::new()
    }
    fn view_cards(
        &mut self,
        _: &GameState,
        _: PlayerId,
        cards: &[ObjectId],
        ctx: &ViewCardsContext,
    ) {
        if ctx.description.contains("planar") {
            self.planar_viewed += cards.len();
        }
        self.trace.push(json!({"choice":"view_cards","context":format!("{ctx:?}"),"cards":format!("{cards:?}")}));
    }
}
fn announce(
    game: &mut GameState,
    action: LegalAction,
    dm: &mut impl DecisionMaker,
) -> Result<TriggerQueue, String> {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..24 {
        if !game.stack.is_empty()
            && state.pending_activation.is_none()
            && state.pending_cast.is_none()
        {
            progress.map_err(|e| e.to_string())?;
            return Ok(queue);
        }
        let context = match progress.map_err(|e| e.to_string())? {
            GameProgress::NeedsDecisionCtx(c) => c,
            p => return Err(format!("announcement stopped: {p:?}")),
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm);
    }
    Err("announcement exceeded24 decisions".into())
}
fn cast(
    game: &mut GameState,
    def: &CardDefinition,
    dm: &mut impl DecisionMaker,
) -> Result<TriggerQueue, String> {
    game.turn.priority_player = Some(alice());
    let id = game.create_object_from_definition(def, alice(), Zone::Hand);
    let action = compute_legal_actions(game, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or_else(|| format!("no legal cast of {}", def.name()))?;
    announce(game, action, dm)
}
// Use production priority-pass resolution, including dedicated ETB event emission.
fn finish(
    game: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut impl DecisionMaker,
) -> Result<(), String> {
    let mut state = PriorityLoopState::new(game.players_in_game());
    for step in 0..32 {
        eprintln!("FORETELL_STAGE finish {step} stack={}", game.stack.len());
        advance_priority_with_dm(game, q, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(());
        }
        state.reset_for_new_priority_window(game);
        for _ in 0..game.players_in_game() {
            apply_priority_response_with_dm(
                game,
                q,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                dm,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Err("resolution exceeded32 priority windows".into())
}
fn compile(names: &[String]) -> HashMap<String, (CardDefinition, String)> {
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reports/runtime-audit/foretell-candidate-inputs.json");
    let document: Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let mut compilation = Vec::new();
    let definitions = names
        .iter()
        .map(|name| {
            let p = &document["cards"][name];
            let builder = ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(name),
            );
            let (a, d) = ironsmith_registry::compile_builder_to_artifact(
                builder,
                p["parse_input"].as_str().unwrap(),
                false,
            )
            .unwrap();
            compilation.push(json!({"card":name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
            (name.clone(), (d, a.payload_checksum))
        })
        .collect();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reports/runtime-audit/foretell-candidate-artifacts.json");
    std::fs::write(out, serde_json::to_string_pretty(&compilation).unwrap()).unwrap();
    definitions
}

fn record(
    rows: &mut Vec<Value>,
    card: &str,
    scenario: Value,
    expected: Value,
    result: Result<Value, String>,
    checksum: &str,
    trace: Vec<Value>,
) {
    let (status, actual) = match result {
        Ok(a) if a == expected => ("expected_result_observed", a),
        Ok(a) => ("semantic_mismatch", a),
        Err(e) if e.starts_with("Resolution failed:") => ("resolution_failed", json!({"error":e})),
        Err(e) if e.starts_with("Invalid state: Failed special action:") => {
            ("action_or_choice_failed", json!({"error":e}))
        }
        Err(e) => ("execution_or_fixture_error", json!({"error":e})),
    };
    rows.push(json!({"card":card,"scenario":scenario,"expected":expected,"actual":actual,"status":status,"artifact_checksum":checksum,"seed":SEED,"execution_trace":trace}));
    if status == "semantic_mismatch" {
        rows.last_mut().unwrap()["outcome_category"] = json!("silent_wrong_result");
    }
    if status == "resolution_failed" || status == "action_or_choice_failed" {
        rows.last_mut().unwrap()["outcome_category"] = json!("runtime_exception");
    }
}
fn report(name: &str, scope: &str, limitations: &str, rows: Vec<Value>) {
    let path = std::env::current_exe().unwrap();
    let binary_hash = std::env::var("AUDIT_BINARY_SHA256")
        .expect("isolating driver supplies independently verified executable hash");
    let report = json!({"scope":scope,"limitations":limitations,"provenance":{"binary":path,"binary_sha256":binary_hash,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","seed":SEED,"unique_card_ids":true,"reporter_thread_stack_bytes":67108864},"rows":rows});
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reports/runtime-audit")
        .join(name);
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

fn choices() -> Choices {
    Choices {
        target: None,
        accept: true,
        trace: vec![],
        planar_viewed: 0,
        mode: None,
    }
}
fn count(g: &GameState, name: &str, zone: Zone) -> usize {
    g.objects_in_deterministic_order()
        .into_iter()
        .filter(|o| o.name == name && o.zone == zone)
        .count()
}
fn perform_special(g: &mut GameState, action: LegalAction, dm: &mut Choices) -> Result<(), String> {
    let mut q = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":"special_action_selected","action":format!("{action:?}")}));
    let mut result = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        match result {
            GameProgress::NeedsDecisionCtx(ref c)
                if !matches!(
                    c,
                    ironsmith::decisions::context::DecisionContext::Priority(_)
                ) =>
            {
                result = apply_decision_context_with_dm(g, &mut q, &mut state, c, dm)
                    .map_err(|e| e.to_string())?;
            }
            _ => return finish(g, &mut q, dm),
        }
    }
    Err("special action decision budget".into())
}
fn foreteller(g: &GameState, id: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::SpecialAction(ironsmith::special_actions::SpecialAction::Foretell{card_id})if *card_id==id))
}
fn cast_offered(g: &GameState, id: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(g, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
}
fn seed_library(g: &mut GameState) {
    for _ in 0..6 {
        let d = CardDefinitionBuilder::new(CardId::new(), "Foretell library witness")
            .card_types(vec![CardType::Land])
            .build();
        g.create_object_from_definition(&d, alice(), Zone::Library);
    }
}
fn run_card(
    d: &CardDefinition,
    foretell: bool,
    opponent_turn: bool,
    dm: &mut Choices,
) -> Result<Value, String> {
    let mut g = game();
    seed_library(&mut g);
    let hand = g.create_object_from_definition(d, alice(), Zone::Hand);
    let stable = g.object(hand).unwrap().stable_id;
    let before = g.player(alice()).unwrap().mana_pool.total();
    let mut foretell_paid = 0;
    let mut same_turn_cast = false;
    let mut marked = false;
    if foretell {
        let foretell_action = foreteller(&g, hand).ok_or("legal foretell unavailable")?;
        perform_special(&mut g, foretell_action, dm)?;
        foretell_paid = before - g.player(alice()).unwrap().mana_pool.total();
        let exile = g
            .find_object_by_stable_id(stable)
            .ok_or("foretell lost stable identity")?;
        marked = g.object(exile).is_some_and(|o| o.zone == Zone::Exile)
            && g.is_face_down(exile)
            && g.is_foretold(exile);
        same_turn_cast = cast_offered(&g, exile).is_some();
        dm.trace.push(json!({"stage":"actual_foretell_completed","mana_paid":foretell_paid,"old_object":hand.0,"new_object":exile.0,"old_object_exists":g.object(hand).is_some(),"marked_foretold_facedown_exile":marked,"same_turn_cast_offered":same_turn_cast}));
        g.turn.turn_number += if opponent_turn { 1 } else { 3 };
        g.turn.active_player = if opponent_turn { bob() } else { alice() };
        g.turn.priority_player = Some(alice());
    }
    let object = g
        .find_object_by_stable_id(stable)
        .ok_or("missing cast object")?;
    let action = cast_offered(&g, object);
    if opponent_turn && d.name() == "Alrund's Epiphany" {
        return Ok(
            json!({"later_opponent_turn_cast_offered":action.is_some(),"foretell_paid":foretell_paid,"same_turn_cast_offered":same_turn_cast,"foretold_facedown_exile":marked}),
        );
    }
    let action = action.ok_or("legal cast unavailable")?;
    let before = g.player(alice()).unwrap().mana_pool.total();
    let mut q = announce(&mut g, action, dm)?;
    let cast_paid = before - g.player(alice()).unwrap().mana_pool.total();
    dm.trace.push(json!({"stage":"actual_cast_completed","mana_paid":cast_paid,"origin":if foretell{"Exile"}else{"Hand"}}));
    finish(&mut g, &mut q, dm)?;
    let birds = g
        .battlefield
        .iter()
        .copied()
        .filter(|id| g.current_has_subtype(*id, Subtype::Bird))
        .collect::<Vec<_>>();
    let birds_correct = birds.iter().all(|id| {
        g.calculated_power(*id) == Some(1)
            && g.calculated_toughness(*id) == Some(1)
            && g.current_has_static_ability_id(
                *id,
                ironsmith::static_abilities::StaticAbilityId::Flying,
            )
    });
    Ok(
        json!({"foretell_paid":foretell_paid,"cast_paid":cast_paid,"same_turn_cast_offered":same_turn_cast,"foretold_facedown_exile":marked,"source_exile":count(&g,d.name(),Zone::Exile),"source_graveyard":count(&g,d.name(),Zone::Graveyard),"birds":birds.len(),"birds_correct":birds_correct,"extra_turns":g.turn_store.extra_turns.len(),"cards_drawn":g.player(alice()).unwrap().hand.len()}),
    )
}
fn run_mage(
    defs: &HashMap<String, (CardDefinition, String)>,
    bloom: bool,
    stop_after_bloom: bool,
    dm: &mut Choices,
) -> Result<Value, String> {
    let mut g = game();
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::BeginCombat);
    g.player_mut(alice()).unwrap().mana_pool = Default::default();
    let alrund = g.create_object_from_definition(
        &defs["Alrund, God of the Cosmos"].0,
        alice(),
        Zone::Battlefield,
    );
    let effigy = g.create_object_from_definition(&defs["Scorn Effigy"].0, alice(), Zone::Hand);
    if bloom {
        let source = g.create_object_from_definition(
            &defs["Cadaverous Bloom"].0,
            alice(),
            Zone::Battlefield,
        );
        g.create_object_from_definition(&defs["Lightning Bolt"].0, alice(), Zone::Hand);
        let action = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateManaAbility{source:id,..}if *id==source))
            .ok_or("Bloom mana action unavailable")?;
        perform_special(&mut g, action, dm)?;
        dm.trace.push(json!({"stage":"bloom_completed","mana":g.player(alice()).unwrap().mana_pool.total(),"bolt_exile":count(&g,"Lightning Bolt",Zone::Exile),"effigy_hand":count(&g,"Scorn Effigy",Zone::Hand)}));
    } else {
        g.player_mut(alice())
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 2);
    }
    if stop_after_bloom {
        return Ok(
            json!({"mana_total":g.player(alice()).unwrap().mana_pool.total(),"bolt_exile":count(&g,"Lightning Bolt",Zone::Exile),"effigy_hand":count(&g,"Scorn Effigy",Zone::Hand)}),
        );
    }
    let action = foreteller(&g, effigy).ok_or("Effigy foretell unavailable")?;
    let result = perform_special(&mut g, action, dm);
    let effigy_in_exile = g
        .exile
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.name == "Scorn Effigy"));
    dm.trace.push(json!({"stage":"after_foretell_attempt","error":result.as_ref().err(),"effigy_hand":count(&g,"Scorn Effigy",Zone::Hand),"effigy_exile":count(&g,"Scorn Effigy",Zone::Exile),"effigy_marked_foretold":effigy_in_exile.is_some_and(|id|g.is_foretold(id)),"mana_remaining":g.player(alice()).unwrap().mana_pool.total()}));
    result?;
    Ok(
        json!({"hand":g.player(alice()).unwrap().hand.len(),"effigy_exile":count(&g,"Scorn Effigy",Zone::Exile),"bolt_exile":usize::from(bloom).min(count(&g,"Lightning Bolt",Zone::Exile)),"alrund_power":g.calculated_power(alrund),"alrund_toughness":g.calculated_toughness(alrund),"mana_remaining":g.player(alice()).unwrap().mana_pool.total()}),
    )
}
fn generate_report() {
    let names = [
        "Alrund's Epiphany",
        "Behold the Multiverse",
        "Alrund, God of the Cosmos",
        "Scorn Effigy",
        "Cadaverous Bloom",
        "Lightning Bolt",
    ]
    .map(str::to_owned);
    let defs = compile(&names);
    let mut rows = vec![];
    for name in ["Alrund's Epiphany", "Behold the Multiverse"] {
        for (foretell, opponent) in [(false, false), (true, false), (true, true)] {
            let (d, c) = &defs[name];
            let epiphany = name == "Alrund's Epiphany";
            let expected = if epiphany && opponent {
                json!({"later_opponent_turn_cast_offered":false,"foretell_paid":2,"same_turn_cast_offered":false,"foretold_facedown_exile":true})
            } else {
                json!({"foretell_paid":if foretell{2}else{0},"cast_paid":if epiphany{if foretell{6}else{7}}else{if foretell{2}else{4}},"same_turn_cast_offered":false,"foretold_facedown_exile":foretell,"source_exile":usize::from(epiphany),"source_graveyard":usize::from(!epiphany),"birds":if epiphany{2}else{0},"birds_correct":true,"extra_turns":usize::from(epiphany),"cards_drawn":if epiphany{0}else{2}})
            };
            let mut dm = choices();
            eprintln!("FORETELL_CASE {name} foretold={foretell} opponent={opponent}");
            let result = run_card(d, foretell, opponent, &mut dm);
            record(
                &mut rows,
                name,
                json!({"foretell":foretell,"later_opponent_turn":opponent}),
                expected,
                result,
                c,
                dm.trace,
            );
        }
    }
    for mode in [0, 1] {
        let mut dm = choices();
        dm.mode = Some(mode);
        let result = run_mage(&defs, true, true, &mut dm);
        record(
            &mut rows,
            "Cadaverous Bloom",
            json!({"mana_color":if mode==0{"Black"}else{"Green"},"stop_after_paid_activation":true}),
            json!({"mana_total":2,"bolt_exile":1,"effigy_hand":1}),
            result,
            &defs["Cadaverous Bloom"].1,
            dm.trace,
        );
    }
    for bloom in [false, true] {
        let mut dm = choices();
        let result = run_mage(&defs, bloom, false, &mut dm);
        record(
            &mut rows,
            if bloom {
                "Cadaverous Bloom"
            } else {
                "Alrund, God of the Cosmos"
            },
            json!({"original_mage_bloom_producer":bloom}),
            json!({"hand":0,"effigy_exile":1,"bolt_exile":usize::from(bloom),"alrund_power":2,"alrund_toughness":2,"mana_remaining":0}),
            result,
            &defs[if bloom {
                "Cadaverous Bloom"
            } else {
                "Alrund, God of the Cosmos"
            }]
            .1,
            dm.trace,
        );
    }
    report(
        "foretell-candidate-reproductions.json",
        "Actual foretell special action with paid costs, zone identity change, same-turn legality, later-turn paid casting; exact MAGE Alrund/Bloom/Effigy producer and mana-pool control.",
        "Turns are positioned explicitly; unrelated intervening steps are not replayed. Generic initial mana and neutral library resources. The MAGE lead concerns Alrund, God of the Cosmos, not Alrund's Epiphany. No engine changes; report success is not a semantic pass.",
        rows,
    );
}
#[test]
#[ignore = "opt-in expected-result audit reporter"]
fn report_foretell_candidate_cases() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate_report)
        .unwrap()
        .join()
        .unwrap();
}

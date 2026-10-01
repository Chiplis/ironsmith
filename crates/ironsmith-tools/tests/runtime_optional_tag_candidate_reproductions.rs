//! Opt-in canonical runtime evidence. Passing means reports were generated.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{
    AttackerDeclaration, DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker,
    compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, PartitionContext, SelectObjectsContext, SelectOptionsContext, ViewCardsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_attacker_declarations_with_dm, apply_blocker_declarations,
    apply_decision_context_with_dm, apply_priority_response_with_dm,
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
    player_target: Option<PlayerId>,
}
impl DecisionMaker for Choices {
    fn decide_targets(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        let selected = self
            .target
            .map(ironsmith::game_state::Target::Object)
            .or_else(|| {
                self.player_target
                    .map(ironsmith::game_state::Target::Player)
            })
            .into_iter()
            .filter(|t| ctx.requirements.iter().any(|r| r.legal_targets.contains(t)))
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
        let selected = if c.aggregate_constraint.is_some() {
            c.candidates
                .iter()
                .filter(|x| x.legal)
                .map(|x| x.id)
                .collect()
        } else if let Some(x) = c
            .candidates
            .iter()
            .find(|x| x.legal && x.name == "Sacrifice witness")
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
        eprintln!(
            "TRIGGER_CHOICE_STAGE finish {step} stack={}",
            game.stack.len()
        );
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
fn attack(
    game: &mut GameState,
    attackers: &[ObjectId],
    defender: PlayerId,
) -> Result<TriggerQueue, String> {
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    for id in attackers {
        game.remove_summoning_sickness(*id);
    }
    let declarations = attackers
        .iter()
        .map(|id| AttackerDeclaration {
            creature: *id,
            target: AttackTarget::Player(defender),
        })
        .collect::<Vec<_>>();
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations_with_dm(
        game,
        &mut combat,
        &mut queue,
        &declarations,
        &mut SelectFirstDecisionMaker,
    )
    .map_err(|e| e.to_string())?;
    game.combat = Some(combat);
    Ok(queue)
}
fn compile(names: &[String]) -> HashMap<String, (CardDefinition, String)> {
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reports/runtime-audit/optional-tag-candidate-inputs.json");
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
        .join("../../reports/runtime-audit/optional-tag-candidate-artifacts.json");
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
        Err(e) => ("execution_or_fixture_error", json!({"error":e})),
    };
    rows.push(json!({"card":card,"scenario":scenario,"expected":expected,"actual":actual,"status":status,"artifact_checksum":checksum,"seed":SEED,"execution_trace":trace}));
    if status == "semantic_mismatch" {
        rows.last_mut().unwrap()["outcome_category"] = json!("silent_wrong_result");
    }
    if status == "resolution_failed" {
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

fn choices(accept: bool) -> Choices {
    Choices {
        target: None,
        accept,
        trace: vec![],
        planar_viewed: 0,
        mode: None,
        player_target: None,
    }
}
fn count(g: &GameState, name: &str, zone: Zone) -> usize {
    g.objects_in_deterministic_order()
        .into_iter()
        .filter(|o| o.name == name && o.zone == zone)
        .count()
}
fn find(g: &GameState, name: &str) -> Result<ObjectId, String> {
    g.battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.name == name))
        .ok_or(format!("missing battlefield {name}"))
}
fn trample(g: &GameState, id: ObjectId) -> bool {
    g.current_has_static_ability_id(id, ironsmith::static_abilities::StaticAbilityId::Trample)
}
fn cast_paid(
    g: &mut GameState,
    d: &CardDefinition,
    dm: &mut Choices,
    cost: u32,
) -> Result<(), String> {
    let before = g.player(alice()).unwrap().mana_pool.total();
    let mut q = cast(g, d, dm)?;
    let paid = before - g.player(alice()).unwrap().mana_pool.total();
    dm.trace
        .push(json!({"stage":"paid_cast","card":d.name(),"mana_paid":paid}));
    if paid != cost {
        return Err(format!("fixture payment expected{cost} actual{paid}"));
    }
    finish(g, &mut q, dm)
}
fn upkeep(g: &mut GameState, player: PlayerId, dm: &mut Choices) -> Result<(), String> {
    g.turn.turn_number += 1;
    g.turn.active_player = player;
    g.turn.priority_player = Some(player);
    g.turn.phase = Phase::Beginning;
    g.turn.step = Some(Step::Upkeep);
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::generate_and_queue_step_triggers(g, &mut q);
    dm.trace
        .push(json!({"stage":"real_upkeep_event","player":player.0}));
    finish(g, &mut q, dm)
}
fn forest(g: &mut GameState, snow: bool) {
    let b = CardDefinitionBuilder::new(CardId::new(), "Audit Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest]);
    let d = if snow {
        b.supertypes(vec![ironsmith::Supertype::Snow]).build()
    } else {
        b.build()
    };
    g.create_object_from_definition(&d, alice(), Zone::Battlefield);
}
fn food(g: &mut GameState) {
    let d = CardDefinitionBuilder::new(CardId::new(), "Audit Food")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Food])
        .build();
    g.create_object_from_definition(&d, alice(), Zone::Battlefield);
}
fn attack_next_turn(g: &mut GameState, ids: &[ObjectId], dm: &mut Choices) -> Result<(), String> {
    g.turn.turn_number += 1;
    let mut q = attack(g, ids, bob())?;
    dm.trace.push(json!({"stage":"actual_attack_declaration","attackers":format!("{ids:?}"),"timing_fixture":"next own turn established; skipped intervening steps"}));
    finish(g, &mut q, dm)
}
fn add_row(
    rows: &mut Vec<Value>,
    name: &str,
    scenario: Value,
    expected: Value,
    checksum: &str,
    mut dm: Choices,
    run: impl FnOnce(&mut Choices) -> Result<Value, String>,
) {
    eprintln!("TRIGGER_CHOICE_CASE {name} {scenario}");
    let result = run(&mut dm);
    record(rows, name, scenario, expected, result, checksum, dm.trace);
}
fn one(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
    let mut state = PriorityLoopState::new(g.players_in_game());
    state.reset_for_new_priority_window(g);
    for _ in 0..g.players_in_game() {
        apply_priority_response_with_dm(
            g,
            q,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            dm,
        )
        .map_err(|e| e.to_string())?;
    }
    advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
    Ok(())
}
fn cast_for(
    g: &mut GameState,
    d: &CardDefinition,
    player: PlayerId,
    dm: &mut Choices,
    cost: u32,
) -> Result<(), String> {
    g.turn.priority_player = Some(player);
    let id = g.create_object_from_definition(d, player, Zone::Hand);
    let action = compute_legal_actions(g, player).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or("witnesscastmissing")?;
    let before = g.player(player).unwrap().mana_pool.total();
    let mut q = announce(g, action, dm)?;
    let paid = before - g.player(player).unwrap().mana_pool.total();
    dm.trace
        .push(json!({"stage":"paid_witness_cast","card":d.name(),"player":player.0,"paid":paid}));
    if paid != cost {
        return Err("witnesspaymentincorrect".into());
    }
    finish(g, &mut q, dm)
}
fn instant_card(name: &str, mv: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
            ManaSymbol::Generic(mv),
        ]))
        .build()
}
fn enchantment(name: &str, mv: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Enchantment])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
            ManaSymbol::Generic(mv),
        ]))
        .build()
}
fn endstep(g: &mut GameState, own: bool, q: &mut TriggerQueue) {
    g.turn.active_player = if own { alice() } else { bob() };
    g.turn.priority_player = Some(g.turn.active_player);
    g.turn.phase = Phase::Ending;
    g.turn.step = Some(Step::End);
    ironsmith::game_loop::generate_and_queue_step_triggers(g, q);
}
fn subtype_count(g: &GameState, kind: Subtype, owner: PlayerId) -> usize {
    g.battlefield
        .iter()
        .filter(|id| g.current_controller(**id) == Some(owner) && g.current_has_subtype(**id, kind))
        .count()
}
fn generate_report() {
    let names = [
        "Elven Passage",
        "Lamplight Phoenix",
        "Enchanter's Bane",
        "Severance Priest",
        "Skyclave Apparition",
        "Murder",
        "Unsummon",
        "Disenchant",
    ]
    .map(str::to_owned);
    let defs = compile(&names);
    let mut rows = vec![];
    let name = "Elven Passage";
    let (d, c) = &defs[name];
    for (elf, accept, land) in [
        ("none", false, true),
        ("hand", false, true),
        ("hand", true, true),
        ("battlefield", true, true),
        ("hand", true, false),
    ] {
        add_row(
            &mut rows,
            name,
            json!({"elf":elf,"accept":accept,"basic_land_in_library":land}),
            json!({"life":19,"source_graveyard":1,"lands":usize::from(land),"land_tapped":land&&!(accept&&elf!="none"),"elf_hand":usize::from(elf=="hand"),"elf_battlefield":usize::from(elf=="battlefield")}),
            c,
            choices(accept),
            |dm| {
                let mut g = game();
                let sourcehand = g.create_object_from_definition(d, alice(), Zone::Hand);
                let action = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
                    .into_iter()
                    .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==sourcehand))
                    .ok_or("Passage landplaymissing")?;
                let mut q = TriggerQueue::new();
                let mut state = PriorityLoopState::new(g.players_in_game());
                apply_priority_response_with_dm(
                    &mut g,
                    &mut q,
                    &mut state,
                    &PriorityResponse::PriorityAction(action),
                    dm,
                )
                .map_err(|e| e.to_string())?;
                let source = find(&g, name)?;
                if land {
                    let l = CardDefinitionBuilder::new(CardId::new(), "Passage Forest")
                        .card_types(vec![CardType::Land])
                        .supertypes(vec![ironsmith::Supertype::Basic])
                        .subtypes(vec![Subtype::Forest])
                        .build();
                    g.create_object_from_definition(&l, alice(), Zone::Library);
                }
                if elf != "none" {
                    let e = CardDefinitionBuilder::new(CardId::new(), "Passage Elf")
                        .card_types(vec![CardType::Creature])
                        .subtypes(vec![Subtype::Elf])
                        .power_toughness(PowerToughness::fixed(1, 1))
                        .build();
                    g.create_object_from_definition(
                        &e,
                        alice(),
                        if elf == "hand" {
                            Zone::Hand
                        } else {
                            Zone::Battlefield
                        },
                    );
                }
                let action = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
                    .into_iter()
                    .find(|a| matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source))
                    .ok_or("Passageactivationmissing")?;
                let mut q = announce(&mut g, action, dm)?;
                dm.trace.push(json!({"stage":"actual_passage_costs","life":g.player(alice()).unwrap().life,"source_graveyard":count(&g,name,Zone::Graveyard)}));
                finish(&mut g, &mut q, dm)?;
                let lands = g
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|id| g.object(*id).is_some_and(|o| o.name == "Passage Forest"))
                    .collect::<Vec<_>>();
                Ok(
                    json!({"life":g.player(alice()).unwrap().life,"source_graveyard":count(&g,name,Zone::Graveyard),"lands":lands.len(),"land_tapped":lands.first().is_some_and(|id|g.is_tapped(*id)),"elf_hand":count(&g,"Passage Elf",Zone::Hand),"elf_battlefield":count(&g,"Passage Elf",Zone::Battlefield)}),
                )
            },
        );
    }
    let name = "Lamplight Phoenix";
    let (d, c) = &defs[name];
    for (evidence, accept) in [
        (vec![], false),
        (vec![3], false),
        (vec![4], false),
        (vec![4], true),
        (vec![2, 2], true),
    ] {
        let revive = accept && evidence.iter().sum::<u8>() >= 4;
        add_row(
            &mut rows,
            name,
            json!({"evidence_mana_values":evidence,"accept":accept,"kill_spell_owner":"Bob"}),
            json!({"source_battlefield":usize::from(revive),"source_graveyard":usize::from(!revive),"source_tapped":revive,"evidence_exile":if revive{evidence.len()}else{0},"evidence_graveyard":if revive{0}else{evidence.len()}}),
            c,
            choices(accept),
            |dm| {
                let mut g = game();
                for mv in &evidence {
                    g.create_object_from_definition(
                        &instant_card("Phoenix evidence", *mv),
                        alice(),
                        Zone::Graveyard,
                    );
                }
                cast_paid(&mut g, d, dm, 3)?;
                dm.target = Some(find(&g, name)?);
                cast_for(&mut g, &defs["Murder"].0, bob(), dm, 3)?;
                let id = find(&g, name).ok();
                Ok(
                    json!({"source_battlefield":count(&g,name,Zone::Battlefield),"source_graveyard":count(&g,name,Zone::Graveyard),"source_tapped":id.is_some_and(|id|g.is_tapped(id)),"evidence_exile":count(&g,"Phoenix evidence",Zone::Exile),"evidence_graveyard":count(&g,"Phoenix evidence",Zone::Graveyard)}),
                )
            },
        );
    }
    let name = "Enchanter's Bane";
    let (d, c) = &defs[name];
    for (own, accept, removal) in [
        (true, false, "none"),
        (true, true, "none"),
        (false, false, "none"),
        (true, false, "target"),
        (true, false, "source"),
    ] {
        let hit = own && !accept && removal != "target";
        add_row(
            &mut rows,
            name,
            json!({"own_end_step":own,"sacrifice":accept,"removed_in_response":removal}),
            json!({"bob_life":if hit{16}else{20},"target_battlefield":usize::from(removal!="target"&&!(own&&accept)),"target_graveyard":usize::from(removal=="target"||own&&accept)}),
            c,
            choices(accept),
            |dm| {
                let mut g = game();
                let target = g.create_object_from_definition(
                    &enchantment("Bane target", 4),
                    bob(),
                    Zone::Battlefield,
                );
                cast_paid(&mut g, d, dm, 2)?;
                let source = find(&g, name)?;
                dm.target = Some(target);
                let mut q = TriggerQueue::new();
                endstep(&mut g, own, &mut q);
                advance_priority_with_dm(&mut g, &mut q, dm).map_err(|e| e.to_string())?;
                dm.trace.push(json!({"stage":"actual_end_step_trigger_queued","stack":g.stack.len(),"target":target.0}));
                if removal != "none" {
                    dm.target = Some(if removal == "target" { target } else { source });
                    cast_for(&mut g, &defs["Disenchant"].0, alice(), dm, 2)?;
                } else {
                    finish(&mut g, &mut q, dm)?;
                }
                Ok(
                    json!({"bob_life":g.player(bob()).unwrap().life,"target_battlefield":count(&g,"Bane target",Zone::Battlefield),"target_graveyard":count(&g,"Bane target",Zone::Graveyard)}),
                )
            },
        );
    }
    for name in ["Severance Priest", "Skyclave Apparition"] {
        let (d, c) = &defs[name];
        for scenario in [
            "no_eligible",
            "decline",
            "accept",
            "target_removed_before_etb",
            "source_removed_before_etb",
        ] {
            if name == "Severance Priest" && scenario == "target_removed_before_etb" {
                continue;
            }
            let eligible = scenario != "no_eligible";
            let chosen = eligible && scenario != "decline";
            let early = scenario.contains("before_etb");
            let should_exile = chosen && scenario != "target_removed_before_etb";
            let token = chosen && !early;
            add_row(
                &mut rows,
                name,
                json!({"case":scenario}),
                json!({"source_hand":1,"victim_exile":usize::from(should_exile),"compensation_tokens":usize::from(token),"token_power":if token{Some(3)}else{None},"token_toughness":if token{Some(3)}else{None}}),
                c,
                choices(scenario != "decline"),
                |dm| {
                    let mut g = game();
                    let victim = if eligible {
                        let victim = if name == "Severance Priest" {
                            instant_card("Exile victim", 3)
                        } else {
                            CardDefinitionBuilder::new(CardId::new(), "Exile victim")
                                .card_types(vec![CardType::Creature])
                                .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
                                    ManaSymbol::Generic(3),
                                ]))
                                .power_toughness(PowerToughness::fixed(2, 6))
                                .build()
                        };
                        Some(g.create_object_from_definition(
                            &victim,
                            bob(),
                            if name == "Severance Priest" {
                                Zone::Hand
                            } else {
                                Zone::Battlefield
                            },
                        ))
                    } else {
                        None
                    };
                    if name == "Severance Priest" {
                        dm.player_target = Some(bob());
                        dm.target = None;
                    } else {
                        dm.target = if chosen { victim } else { None };
                    }
                    if early {
                        let mut q = cast(&mut g, d, dm)?;
                        one(&mut g, &mut q, dm)?;
                        dm.trace.push(json!({"stage":"source_entered_trigger_pending","source_battlefield":count(&g,name,Zone::Battlefield),"stack":g.stack.len()}));
                        let source = find(&g, name)?;
                        dm.target = Some(if scenario == "target_removed_before_etb" {
                            victim.unwrap()
                        } else {
                            source
                        });
                        cast_for(&mut g, &defs["Unsummon"].0, alice(), dm, 1)?;
                        if scenario == "target_removed_before_etb" {
                            dm.target = Some(source);
                            cast_for(&mut g, &defs["Unsummon"].0, alice(), dm, 1)?;
                        }
                    } else {
                        cast_paid(&mut g, d, dm, 3)?;
                        dm.trace.push(json!({"stage":"entry_trigger_completed","victim_exile":count(&g,"Exile victim",Zone::Exile)}));
                        dm.target = Some(find(&g, name)?);
                        cast_for(&mut g, &defs["Unsummon"].0, alice(), dm, 1)?;
                    }
                    let kind = if name == "Severance Priest" {
                        Subtype::Spirit
                    } else {
                        Subtype::Illusion
                    };
                    let token_id = g.battlefield.iter().copied().find(|id| {
                        g.current_controller(*id) == Some(bob()) && g.current_has_subtype(*id, kind)
                    });
                    Ok(
                        json!({"source_hand":count(&g,name,Zone::Hand),"victim_exile":count(&g,"Exile victim",Zone::Exile),"compensation_tokens":subtype_count(&g,kind,bob()),"token_power":token_id.and_then(|id|g.calculated_power(id)),"token_toughness":token_id.and_then(|id|g.calculated_toughness(id))}),
                    )
                },
            );
        }
    }
    report(
        "optional-tag-candidate-reproductions.json",
        "Canonical actual land play/activation and paid casts, production priority resolution, actual death/end-step/entry/exit and source/target response controls. Optional costs use feasible aggregate selections and explicitly decline unavailable resources.",
        "Neutral resource objects are seeded; Bob owns Phoenix's Murder so it cannot contaminate Alice's evidence resources. Related steps are positioned explicitly. Returning source before ETB can fail the exit trigger before original entry resolves; downstream expected state is not claimed reached. No engine changes.",
        rows,
    );
}
#[test]
#[ignore = "opt-in expected-result audit reporter"]
fn report_optional_tag_candidate_cases() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate_report)
        .unwrap()
        .join()
        .unwrap();
}

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
    x: u32,
}
impl DecisionMaker for Choices {
    fn decide_number(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let n = if ctx.is_x_value {
            self.x.clamp(ctx.min, ctx.max)
        } else {
            ctx.min
        };
        self.trace
            .push(json!({"choice":"number","context":format!("{ctx:?}"),"selected":n}));
        n
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
        .join("../../reports/runtime-audit/second-trigger-choice-inputs.json");
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
        .join("../../reports/runtime-audit/second-trigger-choice-artifacts.json");
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
        x: 0,
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
fn beginning_combat(g: &mut GameState, owner: bool, dm: &mut Choices) -> Result<(), String> {
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::BeginCombat);
    g.turn.active_player = if owner { alice() } else { bob() };
    g.turn.priority_player = Some(g.turn.active_player);
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::generate_and_queue_step_triggers(g, &mut q);
    dm.trace
        .push(json!({"stage":"actual_begin_combat_event","active":g.turn.active_player.0}));
    finish(g, &mut q, dm)
}
fn equip(
    g: &mut GameState,
    source: ObjectId,
    wearer: ObjectId,
    dm: &mut Choices,
) -> Result<(), String> {
    dm.target = Some(wearer);
    let action = compute_legal_actions(g, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source))
        .ok_or("equip unavailable")?;
    let mana = g.player(alice()).unwrap().mana_pool.total();
    let mut q = announce(g, action, dm)?;
    finish(g, &mut q, dm)?;
    let paid = mana - g.player(alice()).unwrap().mana_pool.total();
    if paid != 3 || g.object(source).unwrap().attached_to.is_none() {
        return Err("equip fixture payment or attachment failed".into());
    }
    dm.trace.push(json!({"stage":"actual_equip_completed","mana_paid":paid,"attachment":format!("{:?}",g.object(source).unwrap().attached_to)}));
    Ok(())
}
fn visit(g: &mut GameState, lit: bool, dm: &mut Choices) -> Result<(), String> {
    g.turn.turn_number += 3;
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    let mut chosen = None;
    for seed in 0..100_u64 {
        let mut probe = g.clone();
        probe.set_random_seed(seed);
        let roll = ironsmith::game_loop::roll_to_visit_attractions_with_dm(
            &mut probe,
            &mut TriggerQueue::new(),
            &mut SelectFirstDecisionMaker,
        )
        .map_err(|e| e.to_string())?;
        if roll.is_some_and(|r| (r == 5 || r == 6) == lit) {
            chosen = Some((seed, roll));
            break;
        }
    }
    let (seed, wanted) = chosen.ok_or("no requested deterministic attraction roll seed")?;
    g.set_random_seed(seed);
    let mut q = TriggerQueue::new();
    let roll = ironsmith::game_loop::roll_to_visit_attractions_with_dm(g, &mut q, dm)
        .map_err(|e| e.to_string())?;
    if roll != wanted {
        return Err("roll seed drift".into());
    }
    dm.trace.push(
        json!({"stage":"actual_attraction_roll","seed":seed,"roll":roll,"printed_lights":[5,6]}),
    );
    finish(g, &mut q, dm)
}
fn generate_report() {
    let names = [
        "Balloon Stand",
        "Devouring Sugarmaw",
        "Devouring Sugarmaw // Have for Dinner",
        "Palani's Hatcher",
        "Spiked Ripsaw",
        "The First Eruption",
        "The Goose Mother",
        "Unsummon",
        "Crush",
        "Deadbeat Attendant",
    ]
    .map(str::to_owned);
    let defs = compile(&names);
    let mut rows = vec![];
    let name = "Devouring Sugarmaw";
    let (d, c) = &defs[name];
    for (resource, accept, own) in [
        ("none", false, true),
        ("artifact", false, true),
        ("artifact", true, true),
        ("enchantment", true, true),
        ("token", true, true),
        ("artifact", true, false),
    ] {
        let sacrifice = own && accept && resource != "none";
        add_row(
            &mut rows,
            name,
            json!({"resource":resource,"accept":accept,"own_upkeep":own}),
            json!({"source_tapped":own&&!sacrifice,"resources":usize::from(resource!="none"&&!sacrifice)}),
            c,
            choices(accept),
            |dm| {
                let mut g = game();
                if resource != "none" {
                    let b = CardDefinitionBuilder::new(CardId::new(), "Sugarmaw resource");
                    let r = match resource {
                        "artifact" => b.card_types(vec![CardType::Artifact]).build(),
                        "enchantment" => b.card_types(vec![CardType::Enchantment]).build(),
                        _ => b
                            .card_types(vec![CardType::Creature])
                            .power_toughness(PowerToughness::fixed(1, 1))
                            .token()
                            .build(),
                    };
                    g.create_object_from_definition(&r, alice(), Zone::Battlefield);
                }
                cast_paid(&mut g, d, dm, 4)?;
                let source = find(&g, name)?;
                upkeep(&mut g, if own { alice() } else { bob() }, dm)?;
                Ok(
                    json!({"source_tapped":g.is_tapped(source),"resources":count(&g,"Sugarmaw resource",Zone::Battlefield)}),
                )
            },
        );
    }
    let name = "Palani's Hatcher";
    let (d, c) = &defs[name];
    for (eggs, own) in [(true, true), (false, true), (true, false)] {
        add_row(
            &mut rows,
            name,
            json!({"retain_actual_etb_eggs":eggs,"own_combat":own}),
            json!({"eggs":if eggs{if own{1}else{2}}else{0},"dinosaurs":usize::from(eggs&&own)}),
            c,
            choices(true),
            |dm| {
                let mut g = game();
                cast_paid(&mut g, d, dm, 5)?;
                if count(&g, "Dinosaur Egg", Zone::Battlefield) != 2 {
                    return Err("Hatcher ETB failed to create2Eggs".into());
                }
                dm.trace
                    .push(json!({"stage":"actual_etb_verified","eggs":2}));
                if !eggs {
                    for _ in 0..2 {
                        dm.target = Some(find(&g, "Dinosaur Egg")?);
                        cast_paid(&mut g, &defs["Unsummon"].0, dm, 1)?;
                    }
                }
                beginning_combat(&mut g, own, dm)?;
                Ok(
                    json!({"eggs":count(&g,"Dinosaur Egg",Zone::Battlefield),"dinosaurs":count(&g,"Dinosaur",Zone::Battlefield)}),
                )
            },
        );
    }
    let name = "Spiked Ripsaw";
    let (d, c) = &defs[name];
    for (attached, attacking, has_forest, accept) in [
        (false, true, true, true),
        (true, false, true, true),
        (true, true, true, false),
        (true, true, true, true),
        (true, true, false, false),
    ] {
        let sac = attached && attacking && has_forest && accept;
        add_row(
            &mut rows,
            name,
            json!({"equipped":attached,"attack":attacking,"forest":has_forest,"accept":accept}),
            json!({"wearer_power":if attached{5}else{2},"wearer_toughness":if attached{9}else{6},"wearer_trample":sac,"forests":usize::from(has_forest&&!sac)}),
            c,
            choices(accept),
            |dm| {
                let mut g = game();
                if has_forest {
                    forest(&mut g, false)
                }
                let wearer = g.create_object_from_definition(
                    &creature("Ripsaw wearer", false, false),
                    alice(),
                    Zone::Battlefield,
                );
                cast_paid(&mut g, d, dm, 3)?;
                let source = find(&g, name)?;
                if attached {
                    equip(&mut g, source, wearer, dm)?;
                }
                if attacking {
                    attack_next_turn(&mut g, &[wearer], dm)?;
                }
                Ok(
                    json!({"wearer_power":g.calculated_power(wearer),"wearer_toughness":g.calculated_toughness(wearer),"wearer_trample":trample(&g,wearer),"forests":count(&g,"Audit Forest",Zone::Battlefield)}),
                )
            },
        );
    }
    let name = "The First Eruption";
    let (d, c) = &defs[name];
    for (chapter, mountain) in [(1, false), (2, false), (3, false), (3, true)] {
        add_row(
            &mut rows,
            name,
            json!({"through_chapter":chapter,"mountain":mountain}),
            json!({"ground_damage":if chapter==1{1}else if chapter==3&&mountain{3}else{0},"flying_damage":if chapter==3&&mountain{3}else{0},"saga_battlefield":usize::from(chapter<3),"mountains":usize::from(mountain&&chapter<3),"red_mana_after_chapter_two":if chapter>=2{2}else{0}}),
            c,
            choices(true),
            |dm| {
                let mut g = game();
                if mountain {
                    let m = CardDefinitionBuilder::new(CardId::new(), "Eruption Mountain")
                        .card_types(vec![CardType::Land])
                        .subtypes(vec![Subtype::Mountain])
                        .build();
                    g.create_object_from_definition(&m, alice(), Zone::Battlefield);
                }
                let ground = g.create_object_from_definition(
                    &creature("Eruption ground", false, false),
                    bob(),
                    Zone::Battlefield,
                );
                let flying = g.create_object_from_definition(
                    &creature("Eruption flyer", false, true),
                    bob(),
                    Zone::Battlefield,
                );
                cast_paid(&mut g, d, dm, 3)?;
                let source = find(&g, name)?;
                if g.counter_count(source, ironsmith::object::CounterType::Lore) != 1
                    || g.damage_on(ground) != 1
                    || g.damage_on(flying) != 0
                {
                    return Err("chapterI fixture failed".into());
                }
                let mut red2 = 0;
                dm.trace.push(json!({"stage":"chapter_one_completed","ground_damage":1,"flying_damage":0,"lore":1}));
                for ch in 2..=chapter {
                    ironsmith::turn::execute_cleanup_step(&mut g);
                    g.turn.turn_number += 3;
                    g.turn.active_player = alice();
                    g.turn.priority_player = Some(alice());
                    g.turn.phase = Phase::FirstMain;
                    g.turn.step = None;
                    let mut q = TriggerQueue::new();
                    ironsmith::game_loop::add_saga_lore_counters_with_dm(&mut g, &mut q, dm).unwrap();
                    dm.trace.push(json!({"stage":"normal_lore_counter_step_action","chapter":ch,"lore":g.counter_count(source,ironsmith::object::CounterType::Lore)}));
                    finish(&mut g, &mut q, dm)?;
                    if ch == 2 {
                        red2 = g.player(alice()).unwrap().mana_pool.red;
                    }
                }
                Ok(
                    json!({"ground_damage":g.damage_on(ground),"flying_damage":g.damage_on(flying),"saga_battlefield":count(&g,name,Zone::Battlefield),"mountains":count(&g,"Eruption Mountain",Zone::Battlefield),"red_mana_after_chapter_two":red2}),
                )
            },
        );
    }
    let name = "The Goose Mother";
    let (d, c) = &defs[name];
    for x in [0, 1, 2, 3, 4] {
        let mut dm = choices(true);
        dm.x = x;
        add_row(
            &mut rows,
            name,
            json!({"x":x,"stage":"entry_only"}),
            json!({"food":(x+1)/2,"counters":x,"power":2+x}),
            c,
            dm,
            |dm| {
                let mut g = game();
                cast_paid(&mut g, d, dm, x + 2)?;
                let source = find(&g, name)?;
                Ok(
                    json!({"food":count(&g,"Food",Zone::Battlefield),"counters":g.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne),"power":g.calculated_power(source)}),
                )
            },
        );
    }
    for (retain, accept) in [(true, false), (true, true), (false, false)] {
        let mut dm = choices(accept);
        dm.x = 2;
        add_row(
            &mut rows,
            name,
            json!({"x":2,"attack":true,"retain_etb_food":retain,"accept":accept}),
            json!({"food":usize::from(retain&&!accept),"cards_drawn":usize::from(retain&&accept)}),
            c,
            dm,
            |dm| {
                let mut g = game();
                for _ in 0..4 {
                    let l = CardDefinitionBuilder::new(CardId::new(), "Goose library card")
                        .card_types(vec![CardType::Land])
                        .build();
                    g.create_object_from_definition(&l, alice(), Zone::Library);
                }
                cast_paid(&mut g, d, dm, 4)?;
                let source = find(&g, name)?;
                if count(&g, "Food", Zone::Battlefield) != 1 {
                    return Err("X2GooseETB expected1Food".into());
                }
                if !retain {
                    dm.target = Some(find(&g, "Food")?);
                    cast_paid(&mut g, &defs["Crush"].0, dm, 1)?;
                }
                attack_next_turn(&mut g, &[source], dm)?;
                Ok(
                    json!({"food":count(&g,"Food",Zone::Battlefield),"cards_drawn":g.player(alice()).unwrap().hand.len()}),
                )
            },
        );
    }
    let name = "Balloon Stand";
    let (d, c) = &defs[name];
    for (resource, mode, lit) in [
        (false, 0, true),
        (false, 1, true),
        (true, 1, true),
        (false, 0, false),
    ] {
        add_row(
            &mut rows,
            name,
            json!({"prior_visit_created_balloon":resource,"mode":mode,"lit_roll":lit}),
            json!({"balloons":if lit&&mode==0{1}else{0},"target_flying":lit&&mode==1&&resource}),
            c,
            choices(true),
            |dm| {
                let mut g = game();
                g.enable_attractions(vec![(
                    alice(),
                    ironsmith::game_state::AttractionDeckFormat::Limited,
                    vec![d.clone(), d.clone(), d.clone()],
                )])?;
                cast_paid(&mut g, &defs["Deadbeat Attendant"].0, dm, 2)?;
                if g.face_up_attractions().len() != 1 {
                    return Err("Deadbeat did not open exactly1Attraction".into());
                }
                let target = g.create_object_from_definition(
                    &creature("Balloon visit target", false, false),
                    alice(),
                    Zone::Battlefield,
                );
                dm.target = Some(target);
                if resource {
                    dm.mode = Some(0);
                    visit(&mut g, true, dm)?;
                    if count(&g, "Balloon", Zone::Battlefield) != 1 {
                        return Err("first actual visit did not create1Balloon".into());
                    }
                }
                dm.mode = Some(mode);
                visit(&mut g, lit, dm)?;
                Ok(
                    json!({"balloons":count(&g,"Balloon",Zone::Battlefield),"target_flying":g.current_has_static_ability_id(target,ironsmith::static_abilities::StaticAbilityId::Flying)}),
                )
            },
        );
    }
    report(
        "second-trigger-choice-reproductions.json",
        "Canonical paid casts and production priority-pass resolution followed by real upkeep/combat/attack/chapter/Attraction-visit producers; positive resource, decline, zero and nonmatching controls. Goose X entry oracle checked separately.",
        "Unrelated turn steps positioned explicitly; Saga uses actual cleanup before normal lore step action. Equipment is attached through paid equip. Limited Attraction deck with3physical copies; actual Deadbeat opener and deterministic die-roll seeds. Sugarmaw parent alias compiled for equivalence and not counted as a separate card. No engine code changes; reporter completion is not a semantics pass.",
        rows,
    );
}
#[test]
#[ignore = "opt-in expected-result audit reporter"]
fn report_second_trigger_choice_cases() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate_report)
        .unwrap()
        .join()
        .unwrap();
}

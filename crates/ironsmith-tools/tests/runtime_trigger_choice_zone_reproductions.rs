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
        .join("../../reports/runtime-audit/trigger-choice-zone-inputs.json");
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
        .join("../../reports/runtime-audit/trigger-choice-zone-artifacts.json");
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
fn generate_trigger_choice_zone_report() {
    let names = [
        "Gargantuan Gorilla",
        "Horned Stoneseeker",
        "Nimble Hobbit",
        "Provisions Merchant",
        "Puppet Conjurer",
        "Wanderwine Prophets",
        "Unsummon",
        "Crush",
    ]
    .map(str::to_owned);
    let defs = compile(&names);
    let mut rows = vec![];
    let name = "Gargantuan Gorilla";
    let (d, checksum) = &defs[name];
    for (resource, accept, own) in [
        ("none", false, true),
        ("forest", false, true),
        ("forest", true, true),
        ("snow", true, true),
        ("forest", true, false),
    ] {
        let survives = !own || accept && resource != "none";
        add_row(
            &mut rows,
            name,
            json!({"forest":resource,"accept":accept,"own_upkeep":own}),
            json!({"source_battlefield":usize::from(survives),"source_graveyard":usize::from(!survives),"forest_battlefield":usize::from(resource!="none"&&(!own||!accept)),"controller_life":if survives{20}else{13},"trample":own&&accept&&resource=="snow"}),
            checksum,
            choices(accept),
            |dm| {
                let mut g = game();
                if resource != "none" {
                    forest(&mut g, resource == "snow")
                }
                cast_paid(&mut g, d, dm, 7)?;
                let source = find(&g, name)?;
                upkeep(&mut g, if own { alice() } else { bob() }, dm)?;
                Ok(
                    json!({"source_battlefield":count(&g,name,Zone::Battlefield),"source_graveyard":count(&g,name,Zone::Graveyard),"forest_battlefield":count(&g,"Audit Forest",Zone::Battlefield),"controller_life":g.player(alice()).unwrap().life,"trample":g.object(source).is_some()&&trample(&g,source)}),
                )
            },
        );
    }
    let name = "Horned Stoneseeker";
    let (d, checksum) = &defs[name];
    for removal in ["none", "keep_powerstone", "destroy_powerstone"] {
        add_row(
            &mut rows,
            name,
            json!({"leave_battlefield":removal!="none","resource":removal}),
            json!({"powerstones":usize::from(removal=="none"),"source_battlefield":usize::from(removal=="none"),"source_hand":usize::from(removal!="none")}),
            checksum,
            choices(true),
            |dm| {
                let mut g = game();
                cast_paid(&mut g, d, dm, 2)?;
                let source = find(&g, name)?;
                let powerstone = find(&g, "Powerstone")?;
                if !g.is_tapped(powerstone) {
                    return Err("entry fixture expected tapped Powerstone".into());
                }
                dm.trace
                    .push(json!({"stage":"actual_etb_verified","powerstone_tapped":true}));
                if removal == "destroy_powerstone" {
                    dm.target = Some(powerstone);
                    cast_paid(&mut g, &defs["Crush"].0, dm, 1)?;
                }
                if removal != "none" {
                    dm.target = Some(source);
                    cast_paid(&mut g, &defs["Unsummon"].0, dm, 1)?;
                }
                Ok(
                    json!({"powerstones":count(&g,"Powerstone",Zone::Battlefield),"source_battlefield":count(&g,name,Zone::Battlefield),"source_hand":count(&g,name,Zone::Hand)}),
                )
            },
        );
    }
    let name = "Nimble Hobbit";
    let (d, checksum) = &defs[name];
    for (resource, accept, mode) in [
        (false, false, 0),
        (true, false, 0),
        (true, true, 0),
        (false, true, 1),
    ] {
        add_row(
            &mut rows,
            name,
            json!({"food":resource,"accept":accept,"mode":mode}),
            json!({"target_tapped":accept,"food_remaining":usize::from(resource&&!(accept&&mode==0)),"trigger_mana_paid":if accept&&mode==1{3}else{0},"source_battlefield":1}),
            checksum,
            choices(accept),
            |dm| {
                let mut g = game();
                if resource {
                    food(&mut g)
                }
                let target = g.create_object_from_definition(
                    &creature("Nimble target", false, false),
                    bob(),
                    Zone::Battlefield,
                );
                dm.target = Some(target);
                dm.mode = Some(mode);
                cast_paid(&mut g, d, dm, 2)?;
                let source = find(&g, name)?;
                let mana = g.player(alice()).unwrap().mana_pool.total();
                attack_next_turn(&mut g, &[source], dm)?;
                Ok(
                    json!({"target_tapped":g.is_tapped(target),"food_remaining":count(&g,"Audit Food",Zone::Battlefield),"trigger_mana_paid":mana-g.player(alice()).unwrap().mana_pool.total(),"source_battlefield":count(&g,name,Zone::Battlefield)}),
                )
            },
        );
    }
    let name = "Provisions Merchant";
    let (d, checksum) = &defs[name];
    for (resource, accept, attacking) in [
        (true, false, true),
        (true, true, true),
        (false, false, true),
        (true, true, false),
    ] {
        let boosted = attacking && accept && resource;
        add_row(
            &mut rows,
            name,
            json!({"etb_food_retained":resource,"accept":accept,"attacking":attacking}),
            json!({"food_remaining":usize::from(resource&&!boosted),"source_power":if boosted{4}else{3},"source_toughness":if boosted{4}else{3},"source_trample":boosted,"other_attacker_power":if boosted{3}else{2},"other_attacker_trample":boosted}),
            checksum,
            choices(accept),
            |dm| {
                let mut g = game();
                cast_paid(&mut g, d, dm, 4)?;
                let source = find(&g, name)?;
                let token = find(&g, "Food")?;
                if !resource {
                    dm.target = Some(token);
                    cast_paid(&mut g, &defs["Crush"].0, dm, 1)?;
                }
                let witness = g.create_object_from_definition(
                    &creature("Coattacker", false, false),
                    alice(),
                    Zone::Battlefield,
                );
                if attacking {
                    attack_next_turn(&mut g, &[source, witness], dm)?;
                }
                Ok(
                    json!({"food_remaining":count(&g,"Food",Zone::Battlefield),"source_power":g.calculated_power(source),"source_toughness":g.calculated_toughness(source),"source_trample":trample(&g,source),"other_attacker_power":g.calculated_power(witness),"other_attacker_trample":trample(&g,witness)}),
                )
            },
        );
    }
    let name = "Puppet Conjurer";
    let (d, checksum) = &defs[name];
    for (resource, own) in [(false, true), (true, true), (true, false), (false, false)] {
        add_row(
            &mut rows,
            name,
            json!({"activate_token_first":resource,"own_upkeep":own}),
            json!({"homunculi":usize::from(resource&&!own),"source_battlefield":1}),
            checksum,
            choices(true),
            |dm| {
                let mut g = game();
                cast_paid(&mut g, d, dm, 2)?;
                let source = find(&g, name)?;
                if resource {
                    g.turn.turn_number += 1;
                    g.remove_summoning_sickness(source);
                    let action=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source)).ok_or("Puppet activation missing")?;
                    let before = g.player(alice()).unwrap().mana_pool.total();
                    let mut q = announce(&mut g, action, dm)?;
                    finish(&mut g, &mut q, dm)?;
                    let paid = before - g.player(alice()).unwrap().mana_pool.total();
                    if paid != 1
                        || !g.is_tapped(source)
                        || count(&g, "Homunculus", Zone::Battlefield) != 1
                    {
                        return Err("Puppet token activation fixture failed".into());
                    }
                    dm.trace.push(json!({"stage":"actual_token_activation_verified","mana_paid":paid,"source_tapped":true}));
                }
                upkeep(&mut g, if own { alice() } else { bob() }, dm)?;
                Ok(
                    json!({"homunculi":count(&g,"Homunculus",Zone::Battlefield),"source_battlefield":count(&g,name,Zone::Battlefield)}),
                )
            },
        );
    }
    let name = "Wanderwine Prophets";
    let (d, checksum) = &defs[name];
    for (accept, resource, damage) in [
        (false, false, true),
        (true, false, true),
        (true, true, true),
        (true, true, false),
    ] {
        add_row(
            &mut rows,
            name,
            json!({"accept_sacrifice":accept,"additional_merfolk":resource,"combat_damage":damage}),
            json!({"extra_turns":usize::from(accept&&damage),"bob_life":if damage{16}else{20},"source_battlefield":usize::from(!damage||!accept||resource),"championed_merfolk_exile":usize::from(!damage||!accept||resource),"additional_merfolk_battlefield":usize::from(resource&&!(accept&&damage))}),
            checksum,
            choices(true),
            |dm| {
                let mut g = game();
                let merfolk = CardDefinitionBuilder::new(CardId::new(), "Champion witness")
                    .card_types(vec![CardType::Creature])
                    .subtypes(vec![Subtype::Merfolk])
                    .power_toughness(PowerToughness::fixed(1, 1))
                    .build();
                g.create_object_from_definition(&merfolk, alice(), Zone::Battlefield);
                cast_paid(&mut g, d, dm, 6)?;
                let source = find(&g, name)?;
                if count(&g, "Champion witness", Zone::Exile) != 1 {
                    return Err("champion fixture did not exile one Merfolk".into());
                }
                dm.trace
                    .push(json!({"stage":"champion_completed","exiled_merfolk":1}));
                if resource {
                    let m = CardDefinitionBuilder::new(CardId::new(), "Sacrifice witness")
                        .card_types(vec![CardType::Creature])
                        .subtypes(vec![Subtype::Merfolk])
                        .power_toughness(PowerToughness::fixed(1, 1))
                        .build();
                    g.create_object_from_definition(&m, alice(), Zone::Battlefield);
                }
                dm.accept = accept;
                attack_next_turn(&mut g, &[source], dm)?;
                if damage {
                    let mut q = TriggerQueue::new();
                    g.turn.step = Some(Step::DeclareBlockers);
                    let mut combat = g.combat.take().unwrap();
                    apply_blocker_declarations(&mut g, &mut combat, &mut q, &[], bob())
                        .map_err(|e| e.to_string())?;
                    g.turn.step = Some(Step::CombatDamage);
                    let events =
                        ironsmith::game_loop::execute_combat_damage_step(&mut g, &combat, false);
                    g.combat = Some(combat);
                    dm.trace.push(json!({"stage":"actual_unblocked_combat_damage","events":format!("{events:?}")}));
                    ironsmith::game_loop::queue_combat_damage_triggers(&mut g, &events, &mut q);
                    finish(&mut g, &mut q, dm)?;
                }
                Ok(
                    json!({"extra_turns":g.turn_store.extra_turns.len(),"bob_life":g.player(bob()).unwrap().life,"source_battlefield":count(&g,name,Zone::Battlefield),"championed_merfolk_exile":count(&g,"Champion witness",Zone::Exile),"additional_merfolk_battlefield":count(&g,"Sacrifice witness",Zone::Battlefield)}),
                )
            },
        );
    }
    report(
        "trigger-choice-zone-reproductions.json",
        "Canonical paid casts, production priority-pass resolution, actual attack and combat damage events, and controller/noncontroller upkeep events; resource and decline controls.",
        "Initial resources are established fixture objects. Next-turn setup increments turn and clears summoning sickness without replaying unrelated turns; upkeep phase is positioned explicitly then normal step trigger producer runs. Counterexamples name the first reachable failing stage. No engine edits. Success means report generated, not semantic certification.",
        rows,
    );
}

#[test]
#[ignore = "opt-in expected-result audit reporter; test success is not a semantics pass"]
fn report_trigger_choice_zone_cases() {
    // Compiled card data and scenario fixtures are large in unoptimized test builds.
    // Keep the reporter stack budget explicit; startup overflow is not card evidence.
    std::thread::Builder::new()
        .name("trigger-choice-zone-audit".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(generate_trigger_choice_zone_report)
        .unwrap()
        .join()
        .unwrap();
}
